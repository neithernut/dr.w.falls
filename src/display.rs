//! Display rendering utilities

use std::future::Future;

use tokio::io;
use tokio_util::codec;

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


/// Representation of an area on the display
///
#[derive(Clone)]
struct Area {
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
    pub fn topleft_in<E: Element>(self) -> E {
        E::new(self.base_row, self.base_col)
    }

    /// Place an Element at the left of the area
    ///
    /// The `Element` will be centered vertically. Both the element and the
    /// remaining area to the right will be returned.
    ///
    pub fn left_in<E: Element>(self) -> (E, Self) {
        let (l, r) = self.split_vertically(E::width());
        (E::new(l.base_row + (l.height - E::height()) / 2, l.base_col), r)
    }

    /// Place an Element at the top of the area
    ///
    /// The `Element` will be centered horizontally. Both the element and the
    /// remaining area below will be returned.
    ///
    pub fn top_in<E: Element>(self) -> (E, Self) {
        let (t, b) = self.split_horizontally(E::height());
        (E::new(t.base_row, t.base_col + (t.width - E::width()) / 2), b)
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


/// Display element
///
trait Element {
    /// Create a new display element at the given position
    ///
    fn new(row: u16, col: u16) -> Self;

    /// Retrieve the width of a display element
    ///
    fn width() -> u16;

    /// Retrieve the height of a display element
    ///
    fn height() -> u16;
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
#[derive(Copy, Clone)]
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
    Text(&'s str),
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

