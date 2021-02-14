//! Display rendering utilities

use std::fmt;
use std::future::Future;
use std::sync::Arc;

use tokio::io;
use tokio_util::codec;

use crate::gameplay::Update;
use crate::util;


/// Display handle
///
/// An instance of this type wraps a writer conntected to the player's ANSII
/// terminal. It is used for displaying various game states.
///
pub struct Display<W> {
    writer: codec::FramedWrite<W, ANSIEncoder>,
    width: u16,
    height: u16,
}

impl<W> Display<W>
    where W: io::AsyncWrite + Unpin
{
    /// Create a new Display
    ///
    /// Create a new display with the specified `width` and `height` from the
    /// given `writer`
    ///
    pub fn new(
        writer: W,
        width: u16,
        height: u16,
    ) -> Self {
        Self {
            writer: codec::FramedWrite::new(writer, ANSIEncoder::new(height - 1, 0)),
            width,
            height,
        }
    }

    /// Retrieve the area
    pub fn area(&self) -> Area {
        Area::new(self.width, self.height - 2)
    }

    /// Send a sequence of DrawCommands
    ///
    /// The function returns a Future which will complete once all the commands
    /// are sent.
    ///
    fn send<'a>(
        &'a mut self,
        cmds: impl IntoIterator<Item = DrawCommand<'a>> + 'a
    ) -> impl Future<Output = io::Result<()>> + 'a {
        use futures::sink::SinkExt;

        self.writer.send(cmds)
    }
}


/// Representation of a play field
///
pub struct PlayField {
    base_row: u16,
    base_col: u16,
}

impl PlayField {
    /// Draw the outlines of the field
    ///
    pub fn draw_outlines<'a>(
        &self,
        display: &'a mut Display<impl io::AsyncWrite + Unpin>,
    ) -> impl Future<Output = std::io::Result<()>> + 'a {
        use std::iter::once;

        use DrawCommand as DC;

        let inlet = "/    \\";
        let inlet_col = util::FIELD_WIDTH as u16 - (inlet.len() as u16 / 2);

        let left_wall = self.base_col;
        let right_wall = self.base_col + 1 + 2*(util::FIELD_WIDTH as u16);

        let element_base_row = self.base_row + 2;

        let cmds = std::iter::empty()
            // Upper part of inlet
            .chain(once(DC::SetPos(self.base_row, self.base_col + 1 + inlet_col)))
            .chain(once("\\    /".into()))
            // Bottle's ceiling with inlet
            .chain(once(DC::SetPos(self.base_row + 1, self.base_col + 1)))
            .chain((0..inlet_col).map(|_| "_".into()))
            .chain(once(inlet.into()))
            .chain(((inlet_col + inlet.len() as u16)..(2 * util::FIELD_WIDTH as u16)).map(|_| "_".into()))
            // Left and right walls
            .chain(once(DC::SetPos(element_base_row, self.base_col)))
            .chain(once("/".into()))
            .chain(once(DC::SetPos(element_base_row, right_wall)))
            .chain(once("\\".into()))
            .chain((1..util::FIELD_HEIGHT.into())
                .map(move |row| row + element_base_row)
                .flat_map(move |row| once(DC::SetPos(row, left_wall))
                    .chain(once("|".into()))
                    .chain(once(DC::SetPos(row, right_wall)))
                    .chain(once("|".into()))
                )
            )
            // Bottle floor
            .chain(once(DC::SetPos(self.base_row + 2 + util::FIELD_HEIGHT as u16, self.base_col)))
            .chain(once("\\".into()))
            .chain((0..util::FIELD_WIDTH).map(|_| "__".into()))
            .chain(once("/".into()));
        display.send(cmds)
    }

    /// Place viruses in the field
    ///
    /// For each of the items in `viruses`, one virus will be placed in the
    /// field, at the given position and with the given colour.
    ///
    pub fn place_viruses<'a>(
        &self,
        display: &'a mut Display<impl io::AsyncWrite + Unpin>,
        viruses: impl IntoIterator<Item=(util::Position, util::Colour)> + 'a,
    ) -> impl Future<Output = std::io::Result<()>> + 'a {
        use std::iter::once;

        let trans = self.transformer();
        let viruses = viruses
            .into_iter()
            .flat_map(move |(pos, col)| once(trans(pos))
                .chain(once(DrawCommand::Format(SGR::FGColour(col.into()))))
                .chain(once("><".into()))
            );
        display.send(viruses)
    }

    /// Place the next capsule elements in the appropriate position
    ///
    pub fn place_next_elements<'a>(
        &self,
        display: &'a mut Display<impl io::AsyncWrite + Unpin>,
        left_element: util::Colour,
        right_element: util::Colour,
    ) -> impl Future<Output = std::io::Result<()>> + 'a {
        let row = self.base_row + 1;
        let col = self.base_col + 1 + util::FIELD_WIDTH as u16 - 2;

        let cmds = vec![
            DrawCommand::SetPos(row, col),
            DrawCommand::Format(SGR::FGColour(left_element.into())),
            "()".into(),
            DrawCommand::Format(SGR::FGColour(right_element.into())),
            "()".into(),
        ];
        display.send(cmds)
    }

    /// Process field updates
    ///
    /// Each item in `updates` will be processed in order: if the update carries
    /// a colour, a capsule element of the given colour will be placed at the
    /// given position. Otherwise, any element at the given position will be
    /// erased.
    ///
    pub fn update<'a>(
        &self,
        display: &'a mut Display<impl io::AsyncWrite + Unpin>,
        updates: impl IntoIterator<Item=Update> + 'a,
    ) -> impl Future<Output = std::io::Result<()>> + 'a {
        use std::iter::once;

        let trans = self.transformer();
        let updates = updates
            .into_iter()
            .flat_map(move |(pos, col)| {
                let sym = if col.is_some() {
                    "()"
                } else {
                    "  "
                };
                once(trans(pos))
                    .chain(col.map(|c| DrawCommand::Format(SGR::FGColour(c.into()))))
                    .chain(once(sym.into()))
            });

        display.send(updates)
    }

    /// Return a function transforming field positions to display positions
    ///
    fn transformer<'t>(&self) -> impl Fn(util::Position) -> DrawCommand<'static> {
        let base_row = self.base_row + 2;
        let base_col = self.base_col + 1;

        move |(row, col)| DrawCommand::SetPos(
            base_row + usize::from(row) as u16,
            base_col + 2 * usize::from(col) as u16
        )
    }
}


/// Factory for `PlayField`s
///
#[derive(Default)]
struct PlayFieldFactory {}

impl ElementFactory for PlayFieldFactory {
    type Element = PlayField;

    fn create_element(self, row: u16, col: u16) -> Self::Element {
        Self::Element {base_row: row, base_col: col}
    }

    fn width(&self) -> u16 {
        2 * util::FIELD_WIDTH as u16 + 2
    }

    fn height(&self) -> u16 {
        util::FIELD_HEIGHT as u16 + 3
    }
}


/// Representation of a score board
///
pub struct ScoreBoard<E> {
    old: Arc<Vec<E>>,
    base_row: u16,
    base_col: u16,
}

impl<E> ScoreBoard<E> {
    /// Render the headings of a score board
    ///
    pub fn render_heading<'a>(
        &self,
        display: &'a mut Display<impl io::AsyncWrite + Unpin>,
        extra_heading: &'a str
    ) -> impl Future<Output = std::io::Result<()>> + 'a {
        let cmds = vec![
            DrawCommand::SetPos(self.base_row, self.base_col + Self::NAME_COL),
            "Player".into(),
            DrawCommand::SetPos(self.base_row, self.base_col + Self::SCORE_COL),
            "Total".into(),
            DrawCommand::SetPos(self.base_row, self.base_col + Self::EXTRA_COL),
            extra_heading.into(),
        ];
        display.send(cmds)
    }

    const ENUM_COL: u16 = 4;
    const NAME_COL: u16 = 4;
    const SCORE_COL: u16 = 24;
    const EXTRA_COL: u16 = 32;
    const WIDTH: u16 = 40;
    const ROW_LIMIT: u16 = 20;
}

impl<E> ScoreBoard<E>
    where E: ScoreBoardEntry + PartialEq
{
    /// Update the score board
    ///
    pub fn update<'a>(
        &mut self,
        display: &'a mut Display<impl io::AsyncWrite + Unpin>,
        new: Arc<Vec<E>>,
        highlight: impl AsRef<E::Tag>,
    ) -> impl Future<Output = std::io::Result<()>> + 'a {
        use DrawCommand as DC;

        let mut cmds: Vec<_> = new
            .iter()
            .zip(self.old.iter().map(Some).chain(std::iter::repeat(None)))
            .enumerate()
            .filter(|(_, (n, o))| Some(*n) == *o)
            .map(|(r, (n, _))| (r as u16 + 1, n))
            .flat_map(|(r, d)| vec![
                DC::Format(SGR::Intensity(if d.tag() == *(highlight.as_ref()) {
                    Some(Intensity::Bold)
                } else if !d.active() {
                    Some(Intensity::Faint)
                }else {
                    None
                })),
                DC::Format(SGR::Strike(!d.active())),
                DC::SetPos(self.base_row + r, self.base_col + Self::ENUM_COL),
                format!("{:2}", r).into(),
                DC::SetPos(self.base_row + r, self.base_col + Self::NAME_COL),
                format!("{:1$}", d.name(), (Self::SCORE_COL - Self::NAME_COL) as usize).into(),
                DC::SetPos(self.base_row + r, self.base_col + Self::SCORE_COL),
                format!("{:>1$}", d.score(), (Self::EXTRA_COL - Self::SCORE_COL) as usize).into(),
                DC::SetPos(self.base_row + r, self.base_col + Self::EXTRA_COL),
                format!("{:>1$}", d.extra(), (Self::WIDTH - Self::EXTRA_COL) as usize).into(),
            ])
            .collect();

        // Remove any superfluous lines
        let remainder = (new.len()..=self.old.len())
            .flat_map(|r| std::iter::once(DC::SetPos(self.base_row + r as u16, self.base_col + Self::ENUM_COL))
                .chain((0..Self::WIDTH).map(|_| " ".into())));
        cmds.extend(remainder);

        self.old = new;
        display.send(cmds)
    }
}


/// Factory for `ScoreBoard`s
///
#[derive(Default)]
struct ScoreBoardFactory<E> {
    phantom: std::marker::PhantomData<E>,
}

impl<E> ElementFactory for ScoreBoardFactory<E> {
    type Element = ScoreBoard<E>;

    fn create_element(self, row: u16, col: u16) -> Self::Element {
        Self::Element {old: Default::default(), base_row: row, base_col: col}
    }

    fn width(&self) -> u16 {
        Self::Element::WIDTH
    }

    fn height(&self) -> u16 {
        Self::Element::ROW_LIMIT + 1
    }
}


/// Scoreboard entry
///
pub trait ScoreBoardEntry {
    /// Player tag type
    ///
    type Tag: PartialEq;

    /// Type of the extra column data
    ///
    type Extra: fmt::Display;

    /// Name of the player
    ///
    fn name(&self) -> &str;

    /// Player tag
    ///
    fn tag(&self) -> Self::Tag;

    /// The player's global score
    ///
    fn score(&self) -> u32;

    /// Extra data associated with the player
    ///
    fn extra(&self) -> Self::Extra;

    /// Indication of the player's status
    ///
    fn active(&self) -> bool;
}


/// Representation of a text element
///
pub struct TextElement<'s> {
    text: &'s str,
    base_row: u16,
    base_col: u16,
}

impl TextElement<'_> {
    /// Draw the text element
    ///
    pub fn draw<'a>(
        &'a mut self,
        display: &'a mut Display<impl io::AsyncWrite + Unpin>,
    ) -> impl Future<Output = std::io::Result<()>> + 'a {
        use std::iter::once;

        let base_row = self.base_row;
        let base_col = self.base_col;

        let cmds = self
            .text
            .lines()
            .enumerate()
            .flat_map(move |(r, l)| once(DrawCommand::SetPos(base_row + r as u16, base_col))
                .chain(once((*l).into())));
        display.send(cmds)
    }

    /// Erase the text element
    ///
    pub fn erase<'a>(
        &'a mut self,
        display: &'a mut Display<impl io::AsyncWrite + Unpin>,
    ) -> impl Future<Output = std::io::Result<()>> + 'a {
        use std::iter::once;

        let base_row = self.base_row;
        let base_col = self.base_col;

        let cmds = self
            .text
            .lines()
            .enumerate()
            .flat_map(move |(r, l)| once(DrawCommand::SetPos(base_row + r as u16, base_col))
                .chain((0..l.len()).map(|_| " ".into())));
        display.send(cmds)
    }
}


impl<'s> ElementFactory for &'s str {
    type Element = TextElement<'s>;

    fn create_element(self, row: u16, col: u16) -> Self::Element {
        Self::Element {text: self, base_row: row, base_col: col}
    }

    fn width(&self) -> u16 {
        self.lines().map(|s| s.chars().count()).max().unwrap_or(0) as u16
    }

    fn height(&self) -> u16 {
        self.lines().count() as u16
    }
}


/// Representation of a line input element
///
pub struct LineInput {
    width: u16,
    base_row: u16,
    base_col: u16,
}

impl LineInput {
    /// Update the line input's contents
    ///
    pub fn update<'a>(
        &mut self,
        display: &'a mut Display<impl io::AsyncWrite + Unpin>,
        data: &'a str,
    ) -> impl Future<Output = std::io::Result<()>> + 'a {
        use std::iter::once;

        let len = data.chars().count();
        let cmds = once(DrawCommand::SetPos(self.base_row, self.base_col))
            .chain(once(data.into()))
            .chain(once(DrawCommand::Format(SGR::Blink(true))))
            .chain(if len < self.width as usize { Some("_".into()) } else { None })
            .chain(once(DrawCommand::Format(SGR::Blink(false))))
            .chain((len..self.width as usize).map(|_| " ".into()));
        display.send(cmds)
    }
}


/// Factory for `LineInput`s
///
pub struct LineInputFactory {
    width: u16,
}

impl LineInputFactory {
    /// Create a factory for a `LineInput` with the given width
    ///
    pub fn new(width: u16) -> Self {
        Self {width}
    }
}

impl ElementFactory for LineInputFactory {
    type Element = LineInput;

    fn create_element(self, row: u16, col: u16) -> Self::Element {
        Self::Element {width: self.width, base_row: row, base_col: col}
    }

    fn width(&self) -> u16 {
        self.width
    }

    fn height(&self) -> u16 {
        1
    }
}


/// Representation of an area on the display
///
#[derive(Clone)]
pub struct Area {
    base_row: u16,
    base_col: u16,
    width: u16,
    height: u16,
}

impl Area {
    /// Create a new area
    ///
    fn new(width: u16, height: u16) -> Self {
        Self {base_row: 0, base_col: 0, width, height}
    }

    /// Split the area vertically at the given column
    ///
    /// The function will return the left and right sub areas. The right area
    /// will include the column at which the original one was split.
    ///
    pub fn split_vertically(mut self, col: u16) -> (Self, Self) {
        let mut right = self.clone();
        self.width = col;
        right.base_col += col;
        right.width -= col;
        (self, right)
    }

    /// Split the area horizontally at the given row
    ///
    /// The function will return the top and bottom sub areas. The bottom area
    /// will include the row at which the original one was split.
    ///
    pub fn split_horizontally(mut self, row: u16) -> (Self, Self) {
        let mut bottom = self.clone();
        self.height = row;
        bottom.base_row += row;
        bottom.height -= row;
        (self, bottom)
    }

    /// Add padding at the top
    ///
    /// This function removes rows from the top of the area.
    ///
    pub fn top_padded(mut self, padding: u16) -> Self {
        self.base_row += padding;
        self.height -= padding;
        self
    }

    /// Add padding at the bottom
    ///
    /// This function removes rows from the bottom of the area.
    ///
    pub fn bottom_padded(mut self, padding: u16) -> Self {
        self.height -= padding;
        self
    }

    /// Add padding at the left
    ///
    /// This function removes rows from the left of the area.
    ///
    pub fn left_padded(mut self, padding: u16) -> Self {
        self.base_col += padding;
        self.width -= padding;
        self
    }

    /// Add padding at the right
    ///
    /// This function removes rows from the right of the area.
    ///
    pub fn right_padded(mut self, padding: u16) -> Self {
        self.width -= padding;
        self
    }

    /// Place an Element at the top left of the area
    ///
    /// The resulting `Element` will be returned.
    ///
    pub fn topleft_in<F: ElementFactory>(self, element_factory: F) -> F::Element {
        element_factory.create_element(self.base_row, self.base_col)
    }

    /// Place an Element at the left of the area
    ///
    /// The `Element` will be centered vertically. Both the element and the
    /// remaining area to the right will be returned.
    ///
    pub fn left_in<F: ElementFactory>(self, element_factory: F) -> (F::Element, Self) {
        let (l, r) = self.split_vertically(element_factory.width());
        let base_row = l.base_row + (l.height - element_factory.height()) / 2;
        (element_factory.create_element(base_row, l.base_col), r)
    }

    /// Place an Element at the top of the area
    ///
    /// The `Element` will be centered horizontally. Both the element and the
    /// remaining area below will be returned.
    ///
    pub fn top_in<F: ElementFactory>(self, element_factory: F) -> (F::Element, Self) {
        let (t, b) = self.split_horizontally(element_factory.height());
        let base_col = t.base_col + (t.width - element_factory.width()) / 2;
        (element_factory.create_element(t.base_row, base_col), b)
    }

    /// Retrieve the area's width
    ///
    pub fn width(&self) -> u16 {
        self.width
    }

    /// Retrieve the area's height
    ///
    pub fn height(&self) -> u16 {
        self.height
    }
}


/// Factory for display element
///
pub trait ElementFactory {
    /// Type of the element constructed by this factory
    ///
    type Element;

    /// Create a new display element at the given position
    ///
    fn create_element(self, row: u16, col: u16) -> Self::Element;

    /// Retrieve the width of the created display element
    ///
    fn width(&self) -> u16;

    /// Retrieve the height of the created display element
    ///
    fn height(&self) -> u16;
}


/// Encoder for sequences of `DrawCommand`s
///
/// This encoder will encode `DrawCommand`s as ANSI escape sequenes. Each
/// `DrawCommand` sequence will be enclosed in sequences for hiding the
/// cursor during issuing those draw commands. In addition, after each
/// sequence, the cursor will be moved to a designated resting position
/// and the default formatting will be restored.
///
struct ANSIEncoder {
    resting_row: u16,
    resting_col: u16,
}

impl ANSIEncoder {
    /// Create a new encoder
    ///
    /// This function creates a new encoder with the resting position provided
    /// via `resting_row` and `resting_col`.
    ///
    pub fn new(resting_row: u16, resting_col: u16) -> Self {
        Self {resting_row, resting_col}
    }
}

impl<'c, I> codec::Encoder<I> for ANSIEncoder
    where I: IntoIterator<Item = DrawCommand<'c>>
{
    type Error = std::io::Error;

    fn encode(
        &mut self,
        items: I,
        dst: &mut bytes::BytesMut
    ) -> Result<(), Self::Error> {
        use bytes::BufMut;

        dst.put_slice(b"\x1b[?25l");
        items.into_iter().for_each(|i| i.write_as_ansi(dst));
        DrawCommand::Format(SGR::Reset).write_as_ansi(dst);
        DrawCommand::SetPos(self.resting_row, self.resting_col).write_as_ansi(dst);
        dst.put_slice(b"\x1b[?25h");
        Ok(())
    }
}


/// Representation of a draw command
///
#[derive(Clone)]
enum DrawCommand<'s> {
    /// Clear the entire screen
    ClearScreen,
    /// Set the cursor's position
    ///
    /// The first component denotes the row, the second one the column. Both are
    /// zero-based, meaning that `0` refers to the first row or column.
    ///
    SetPos(u16, u16),
    /// Select Graphic Rendition
    Format(SGR),
    /// Put text on the screen at the current cursor position
    Text(std::borrow::Cow<'s, str>),
}

impl DrawCommand<'_> {
    /// Write the draw commands as an ASNI escape sequence
    ///
    fn write_as_ansi(&self, out: &mut impl bytes::BufMut) {
        match self {
            DrawCommand::ClearScreen    => out.put_slice(b"\x1b[2J"),
            DrawCommand::SetPos(r, c)   => out.put_slice(format!("\x1b[{};{}H", r + 1, c + 1).as_bytes()),
            DrawCommand::Format(param)  => out.put_slice(format!("\x1b[{}m", param.code()).as_bytes()),
            DrawCommand::Text(s)        => out.put_slice(s.as_bytes()),
        }
    }
}

impl<'s> From<&'s str> for DrawCommand<'s> {
    fn from(text: &'s str) -> Self {
        Self::Text(text.into())
    }
}

impl From<String> for DrawCommand<'_> {
    fn from(text: String) -> Self {
        Self::Text(text.into())
    }
}


/// Representation of some selected "Select Graphic Rendition" parameters
///
#[derive(Copy, Clone)]
enum SGR {
    /// Reset to default formatting
    Reset,
    /// Change intensity
    ///
    /// A value of `None` will reset the intensity to the default.
    ///
    Intensity(Option<Intensity>),
    /// Control underline
    Underline(bool),
    /// Control blink
    Blink(bool),
    /// Control strike-through/cross-out
    Strike(bool),
    /// Set the foreground colour
    ///
    /// A value of `None` will reset the colour to the default.
    ///
    FGColour(Option<(Colour, Brightness)>),
    /// Set the background colour
    ///
    /// A value of `None` will reset the colour to the default.
    ///
    BGColour(Option<(Colour, Brightness)>),
}

impl SGR {
    /// Determine the code number for the SGR parameter
    ///
    fn code(&self) -> u8 {
        use Intensity as Int;

        match self {
            Self::Reset                       =>  0,
            Self::Intensity(Some(Int::Bold))  =>  1,
            Self::Intensity(Some(Int::Faint)) =>  2,
            Self::Intensity(None)             => 22,
            Self::Underline(true)             =>  4,
            Self::Underline(false)            => 24,
            Self::Blink(true)                 =>  5,
            Self::Blink(false)                => 25,
            Self::Strike(true)                =>  9,
            Self::Strike(false)               => 29,
            Self::FGColour(Some((col, br)))   => 30 + col.code_off() + br.code_off(),
            Self::FGColour(None)              => 39,
            Self::BGColour(Some((col, br)))   => 40 + col.code_off() + br.code_off(),
            Self::BGColour(None)              => 49,
        }
    }
}


/// Representation of intensity
///
#[derive(Copy, Clone)]
enum Intensity {
    Bold,
    Faint,
}


/// Representation of the basic colour supported by terminals
///
#[derive(Copy, Clone)]
enum Colour {
    Black,
    Red,
    Green,
    Yellow,
    Blue,
    Magenta,
    Cyan,
    White,
}

impl Colour {
    /// Determin the code offset corresponding to the colour
    ///
    fn code_off(&self) -> u8 {
        match self {
            Self::Black   => 0,
            Self::Red     => 1,
            Self::Green   => 2,
            Self::Yellow  => 3,
            Self::Blue    => 4,
            Self::Magenta => 5,
            Self::Cyan    => 6,
            Self::White   => 7,
        }
    }
}

impl From<util::Colour> for Colour {
    fn from(colour: util::Colour) -> Self {
        match colour {
            util::Colour::Red    => Self::Red,
            util::Colour::Yellow => Self::Yellow,
            util::Colour::Blue   => Self::Blue,
        }
    }
}

impl From<util::Colour> for (Colour, Brightness) {
    fn from(colour: util::Colour) -> Self {
        (colour.into(), Default::default())
    }
}

impl From<util::Colour> for Option<(Colour, Brightness)> {
    fn from(colour: util::Colour) -> Self {
        Some(colour.into())
    }
}


/// Representation of brightness
///
#[derive(Copy, Clone)]
enum Brightness {
    Dark,
    Light,
}

impl Brightness {
    /// Determin the code offset corresponding to the brightness
    ///
    fn code_off(&self) -> u8 {
        match self {
            Self::Dark  =>  0,
            Self::Light => 60,
        }
    }
}

impl Default for Brightness {
    fn default() -> Self {
        Self::Dark
    }
}

