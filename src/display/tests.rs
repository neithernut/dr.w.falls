//! Display tests

use std::pin::Pin;
use std::sync::Arc;
use std::task;

use quickcheck::{Arbitrary, Gen, TestResult};

use super::*;


#[quickcheck]
fn area_split_top(area: Area, split_rows: u16) -> std::io::Result<bool> {
    Ok(tokio::runtime::Runtime::new()?.block_on(async {
        let mut area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        let rows = area.rows();

        let sub = area.split_top(split_rows);
        let sub_rows = sub.rows();
        let sub_cols = sub.cols();

        sub_rows == std::cmp::min(rows, split_rows) &&
            area.rows() + sub_rows == rows &&
            area.cols() == sub_cols
    }))
}


#[quickcheck]
fn area_split_left(area: Area, split_cols: u16) -> std::io::Result<bool> {
    Ok(tokio::runtime::Runtime::new()?.block_on(async {
        let mut area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        let cols = area.cols();

        let sub = area.split_left(split_cols);
        let sub_rows = sub.rows();
        let sub_cols = sub.cols();

        sub_cols == std::cmp::min(cols, split_cols) &&
            area.cols() + sub_cols == cols &&
            area.rows() == sub_rows
    }))
}


#[quickcheck]
fn area_pad_top(area: Area, padding: u16) -> std::io::Result<bool> {
    Ok(tokio::runtime::Runtime::new()?.block_on(async {
        let area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        let rows = area.rows();
        let cols = area.cols();

        let area = area.pad_top(padding);

        area.rows() == rows.saturating_sub(padding) && cols == area.cols()
    }))
}


#[quickcheck]
fn area_pad_bottom(area: Area, padding: u16) -> std::io::Result<bool> {
    Ok(tokio::runtime::Runtime::new()?.block_on(async {
        let area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        let rows = area.rows();
        let cols = area.cols();

        let area = area.pad_bottom(padding);

        area.rows() == rows.saturating_sub(padding) && cols == area.cols()
    }))
}


#[quickcheck]
fn area_pad_left(area: Area, padding: u16) -> std::io::Result<bool> {
    Ok(tokio::runtime::Runtime::new()?.block_on(async {
        let area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        let rows = area.rows();
        let cols = area.cols();

        let area = area.pad_left(padding);

        area.cols() == cols.saturating_sub(padding) && rows == area.rows()
    }))
}


#[quickcheck]
fn area_pad_right(area: Area, padding: u16) -> std::io::Result<bool> {
    Ok(tokio::runtime::Runtime::new()?.block_on(async {
        let area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        let rows = area.rows();
        let cols = area.cols();

        let area = area.pad_right(padding);

        area.cols() == cols.saturating_sub(padding) && rows == area.rows()
    }))
}


#[quickcheck]
fn area_place_top(area: Area, entity: DummyEntity) -> std::io::Result<bool> {
    let res = tokio::runtime::Runtime::new()?.block_on(async {
        let mut area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        area.place_top(entity).await.map(|p| (p, area.rows(), area.cols()))
    });

    match res {
        Ok((placed, new_rows, new_cols)) => Ok(
            placed.base_row == area.row_a &&
            placed.base_col >= area.col_a &&
            placed.rows + new_rows == area.rows() &&
            placed.cols <= area.cols() &&
            new_cols == area.cols()
        ),
        Err(_) => Ok(entity.rows > area.rows() || entity.cols > area.cols()),
    }
}


#[quickcheck]
fn area_place_left(area: Area, entity: DummyEntity) -> std::io::Result<bool> {
    let res = tokio::runtime::Runtime::new()?.block_on(async {
        let mut area = area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await);
        area.place_left(entity).await.map(|p| (p, area.rows(), area.cols()))
    });

    match res {
        Ok((placed, new_rows, new_cols)) => Ok(
            placed.base_row >= area.row_a &&
            placed.base_col == area.col_a &&
            placed.rows <= area.rows() &&
            placed.cols + new_cols == area.cols() &&
            new_rows == area.rows()
        ),
        Err(_) => Ok(entity.rows > area.rows() || entity.cols > area.cols()),
    }
}


#[quickcheck]
fn area_place_center(area: Area, entity: DummyEntity) -> std::io::Result<bool> {
    let res = tokio::runtime::Runtime::new()?.block_on(async {
        area.instantiate(handle_from_bare(tokio::io::sink(), &[]).await).place_center(entity).await
    });

    match res {
        Ok(placed) => Ok(
            placed.base_row >= area.row_a &&
            placed.base_col >= area.col_a &&
            placed.rows <= area.rows() &&
            placed.cols <= area.cols()
        ),
        Err(_) => Ok(entity.rows > area.rows() || entity.cols > area.cols()),
    }
}


#[quickcheck]
fn draw_handle_drop(
    mut data: Vec<commands::DrawCommand<'static>>,
    term: Vec<commands::DrawCommand<'static>>,
) -> std::io::Result<TestResult> {
    use futures::SinkExt;

    use commands::{DrawCommand as DC, SinkProxy};

    let rt = tokio::runtime::Runtime::new()?;

    let inner: Arc<tokio::sync::Mutex<_>> = Arc::new(
        tokio_util::codec::FramedWrite::new(Vec::new(), commands::ANSIEncoder::new()).into()
    );

    rt.block_on(async {
        let mut handle = commands::draw_handle(inner.clone().lock_owned().await, term.as_ref());
        let res = handle.as_sink().send_all(&mut futures::stream::iter(data.iter().cloned().map(Ok))).await;
        drop(handle);
        res
    })?;

    data.extend(term);
    if data.windows(2).any(|w| if let [DC::Text(_), DC::Text(_)] = w { true } else { false }) {
        Ok(TestResult::discard())
    } else {
        let buf = inner.blocking_lock();
        let res = draw_commands_from(buf.get_ref().as_ref())
            .try_fold(Vec::new(), |mut a, c| { a.push(c?); Ok(a) })
            .map(|r| TestResult::from_bool(data == r));
        res
    }
}


#[quickcheck]
fn ansi_encode_decode(orig: Vec<commands::DrawCommand<'static>>) -> std::io::Result<TestResult> {
    use futures::SinkExt;

    use commands::DrawCommand as DC;

    if orig.windows(2).any(|w| if let [DC::Text(_), DC::Text(_)] = w { true } else { false }) {
        return Ok(TestResult::discard())
    }

    let rt = tokio::runtime::Runtime::new()?;

    let mut buf = Vec::new();

    let mut write = tokio_util::codec::FramedWrite::new(&mut buf, super::commands::ANSIEncoder::new());
    rt.block_on(write.send_all(&mut futures::stream::iter(orig.iter().cloned().map(Ok))))?;

    let res = draw_commands_from(buf.as_ref())
        .try_fold(Vec::new(), |mut a, c| { a.push(c?); Ok(a) })
        .map(|r| TestResult::from_bool(orig == r));
    res
}


/// Utility for generating random [area::Area]s
///
#[derive(Copy, Clone, Debug)]
struct Area {
    row_a: u16,
    col_a: u16,
    row_b: u16,
    col_b: u16,
}

impl Area {
    pub fn instantiate<W: tokio::io::AsyncWrite + Send + Unpin>(
        self,
        handle: DrawHandle<'static, W>,
    ) -> area::Area<'static, DrawHandle<'static, W>, W> {
        area::create_area_full(handle, self.row_a, self.col_a, self.row_b, self.col_b)
    }

    pub fn rows(&self) -> u16 {
        self.row_b - self.row_a
    }

    pub fn cols(&self) -> u16 {
        self.col_b - self.col_a
    }
}

impl Arbitrary for Area {
    fn arbitrary(g: &mut Gen) -> Self {
        let row_x = Arbitrary::arbitrary(g);
        let row_y = Arbitrary::arbitrary(g);
        let col_x = Arbitrary::arbitrary(g);
        let col_y = Arbitrary::arbitrary(g);

        Self {
            row_a: std::cmp::min(row_x, row_y),
            col_a: std::cmp::min(col_x, col_y),
            row_b: std::cmp::max(row_x, row_y),
            col_b: std::cmp::max(col_x, col_y),
        }
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let res = (self.row_a, self.col_a, self.row_b, self.col_b)
            .shrink()
            .filter(|(row_a, col_a, row_b, col_b)| row_a <= row_b && col_a <= col_b)
            .map(|(row_a, col_a, row_b, col_b)| Self{row_a, col_a, row_b, col_b});
        Box::new(res)
    }
}


/// Dummy [area::Entity] for testing entity placement
///
#[derive(Copy, Clone, Debug)]
struct DummyEntity {
    rows: u16,
    cols: u16,
}

impl area::Entity for DummyEntity {
    type PlacedEntity = DummyPlaced;

    fn rows(&self) -> u16 {
        self.rows
    }

    fn cols(&self) -> u16 {
        self.cols
    }

    fn init(&self, _pos: (u16, u16)) -> area::PlacedInit {
        Vec::new().into()
    }

    fn place(self, pos: (u16, u16)) -> Self::PlacedEntity {
        let (base_row, base_col) = pos;
        DummyPlaced {base_row, base_col, rows: self.rows, cols: self.cols}
    }
}

impl Arbitrary for DummyEntity {
    fn arbitrary(g: &mut Gen) -> Self {
        Self {rows: Arbitrary::arbitrary(g), cols: Arbitrary::arbitrary(g)}
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new((self.rows, self.cols).shrink().map(|(rows, cols)| Self{rows, cols}))
    }
}


struct DummyPlaced {
    pub base_row: u16,
    pub base_col: u16,
    pub rows: u16,
    pub cols: u16,
}


/// [AsyncWrite] modelling an VT
///
pub struct VTWriter(tokio::sync::watch::Sender<VT>);

impl From<tokio::sync::watch::Sender<VT>> for VTWriter {
    fn from(sender: tokio::sync::watch::Sender<VT>) -> Self {
        Self(sender)
    }
}

impl tokio::io::AsyncWrite for VTWriter {
    fn poll_write(
        self: Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
        buf: &[u8],
    ) -> task::Poll<std::io::Result<usize>> {
        let mut current: VT = self.0.borrow().clone();
        draw_commands_from(buf).try_for_each(|c| current.apply(c?))?;
        self.0.send(current).map_err(|_| std::io::Error::from(std::io::ErrorKind::Other))?;
        Ok(buf.len()).into()
    }

    fn poll_flush(
        self: Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
    ) -> task::Poll<std::io::Result<()>> {
        Ok(()).into()
    }

    fn poll_shutdown(
        self: Pin<&mut Self>,
        _cx: &mut task::Context<'_>,
    ) -> task::Poll<std::io::Result<()>> {
        Ok(()).into()
    }
}


/// Simplified model of a virtual terminal
///
#[derive(Clone, Debug, PartialEq)]
pub struct VT {
    cursor_row: u16,
    cursor_col: u16,
    rendition: GraphicRendition,
    show_cursor: bool,
    data: Vec<Vec<FormattedChar>>,
}

impl VT {
    /// Create a new VT with the given number of rows and columns
    ///
    pub fn new(rows: u16, cols: u16) -> Self {
        let mut row = Vec::new();
        row.resize(cols as usize, Default::default());

        let mut res: Self = Default::default();
        res.data.resize(rows as usize, row);
        res
    }

    /// Clear the "screen"
    ///
    pub fn clear(&mut self) {
        self.data.iter_mut().for_each(|r| r.fill(Default::default()))
    }

    /// Apply a [commands::DrawCommand] to the VT
    ///
    pub fn apply(&mut self, command: commands::DrawCommand) -> std::io::Result<()> {
        use commands::DrawCommand as DC;

        match command {
            DC::ClearScreen     => Ok(self.clear()),
            DC::SetPos(r, c)    => if (r as usize) < self.data.len() && (c as usize) < self.data[0].len() {
                self.cursor_row = r;
                self.cursor_col = c;
                Ok(())
            } else {
                Err(std::io::ErrorKind::Other.into())
            },
            DC::Format(sgr)     => Ok(self.rendition.apply(sgr)),
            DC::Text(txt)       => txt.chars().try_for_each(|c| {
                self.data
                    .get_mut(self.cursor_row as usize)
                    .ok_or(std::io::ErrorKind::Other)?
                    .get_mut(self.cursor_col as usize)
                    .ok_or(std::io::ErrorKind::Other)?
                    .set_from_char(c, self.rendition)?;
                self.cursor_col = self.cursor_col.checked_add(1).ok_or(std::io::ErrorKind::Other)?;
                Ok(())
            }),
            DC::ShowCursor(v)   => Ok(self.show_cursor = v),
        }
    }
}

impl Default for VT {
    fn default() -> Self {
        Self {
            cursor_row: 0,
            cursor_col: 0,
            rendition: Default::default(),
            show_cursor: true,
            data: Default::default(),
        }
    }
}


/// Representation of a formatted character on a [VT]
///
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct FormattedChar {
    pub data: u8,
    pub format: GraphicRendition,
}

impl FormattedChar {
    pub fn set_from_char(&mut self, data: char, format: GraphicRendition) -> std::io::Result<()> {
        if data.is_ascii_graphic() || data == '\x20' {
            self.data = data as u8;
            self.format = format;
            Ok(())
        } else {
            Err(std::io::ErrorKind::Other.into())
        }
    }
}

impl Default for FormattedChar {
    fn default() -> Self {
        Self {data: 0x20, format: Default::default()}
    }
}


/// Representation of a graphic rendition
///
#[derive(Copy, Clone, Debug, PartialEq)]
pub struct GraphicRendition {
    pub intensity: Option<commands::Intensity>,
    pub underline: bool,
    pub blink: bool,
    pub strike: bool,
    pub fg_colour: Option<(commands::Colour, commands::Brightness)>,
    pub bg_colour: Option<(commands::Colour, commands::Brightness)>,
}

impl GraphicRendition {
    /// Apply a change in the form of an SGR to this rendition
    ///
    pub fn apply(&mut self, sgr: commands::SGR) {
        use commands::SGR;

        match sgr {
            SGR::Reset          => *self = Default::default(),
            SGR::Intensity(v)   => self.intensity = v,
            SGR::Underline(v)   => self.underline = v,
            SGR::Blink(v)       => self.blink = v,
            SGR::Strike(v)      => self.strike = v,
            SGR::FGColour(v)    => self.fg_colour = v,
            SGR::BGColour(v)    => self.bg_colour = v,
        }
    }
}

impl Default for GraphicRendition {
    fn default() -> Self {
        Self {
            intensity: None,
            underline: false,
            blink: false,
            strike: false,
            fg_colour: None,
            bg_colour: None,
        }
    }
}


/// Create a DrawHandle from some bare parts
///
async fn handle_from_bare<'a, W: tokio::io::AsyncWrite + Send + Unpin + 'static>(
    write: W,
    termination_seq: &'a [commands::DrawCommand<'static>],
) -> DrawHandle<'a, W> {
    let inner: Arc<tokio::sync::Mutex<_>> = Arc::new(
        tokio_util::codec::FramedWrite::new(write, commands::ANSIEncoder::new()).into()
    );
    commands::draw_handle(inner.lock_owned().await, termination_seq)
}


/// Decode all `DrawCommand`s from a given input
///
fn draw_commands_from(mut src: &[u8]) -> impl Iterator<Item = std::io::Result<commands::DrawCommand<'static>>> + '_ {
    std::iter::from_fn(move || match decode_ansi(src) {
        Ok((res, rem))  => { src = rem; res.map(Ok) },
        Err(e)          => Some(Err(e))
    })
}


/// Decode a `DrawCommand`
///
/// Decode a single [commands::DrawCommand] from encoded ANSI provided as a
/// slice of bytes. The function resturns a tuple containing the decoded unit
/// and the remaining buffer. If the provided slice is empty, this function
/// returns `None` for the draw command. If an ANSI sequence could not be
/// decoded, an error will be returned.
///
fn decode_ansi(src: &[u8]) -> std::io::Result<(Option<commands::DrawCommand<'static>>, &[u8])> {
    use std::io::ErrorKind as EK;

    use commands::{Brightness, Colour, DrawCommand, Intensity, SGR};

    fn extract_num(s: &[u8]) -> Option<(&[u8], &[u8])> {
        s.iter().position(|c| !c.is_ascii_digit()).map(|p| s.split_at(p))
    }

    fn parse_u16(s: &[u8]) -> Option<u16> {
        std::str::from_utf8(s).ok().and_then(|s| s.parse().ok())
    }

    if src.is_empty() {
        Ok((None, src))
    } else if let Some(src) = src.strip_prefix(b"\x1b[") {
        let (n, rem) = extract_num(src).ok_or(EK::InvalidData)?;
        if !n.is_empty() {
            let n: u16 = parse_u16(n).ok_or(EK::InvalidData)?;
            let (com, rem) = rem.split_first().ok_or(EK::InvalidData)?;
            let data = match com {
                0x4a if n == 2  => DrawCommand::ClearScreen,
                0x3b            => {
                    let (m, rem) = extract_num(rem).ok_or(EK::InvalidData)?;
                    let m: u16 = parse_u16(m).ok_or(EK::InvalidData)?;
                    let (com, rem) = rem.split_first().ok_or(EK::InvalidData)?;
                    if *com == 0x48 {
                        let n = n.checked_sub(1).ok_or(EK::InvalidData)?;
                        let m = m.checked_sub(1).ok_or(EK::InvalidData)?;
                        return Ok((Some(DrawCommand::SetPos(n, m)), rem))
                    } else {
                        Err(EK::InvalidData)?
                    }
                },
                0x6d            => match n {
                      0 => SGR::Reset,
                      1 => SGR::Intensity(Some(Intensity::Bold)),
                      2 => SGR::Intensity(Some(Intensity::Faint)),
                      4 => SGR::Underline(true),
                      5 => SGR::Blink(true),
                      9 => SGR::Strike(true),
                     22 => SGR::Intensity(None),
                     24 => SGR::Underline(false),
                     25 => SGR::Blink(false),
                     29 => SGR::Strike(false),
                     30 => SGR::FGColour(Some((Colour::Black,   Brightness::Dark))),
                     31 => SGR::FGColour(Some((Colour::Red,     Brightness::Dark))),
                     32 => SGR::FGColour(Some((Colour::Green,   Brightness::Dark))),
                     33 => SGR::FGColour(Some((Colour::Yellow,  Brightness::Dark))),
                     34 => SGR::FGColour(Some((Colour::Blue,    Brightness::Dark))),
                     35 => SGR::FGColour(Some((Colour::Magenta, Brightness::Dark))),
                     36 => SGR::FGColour(Some((Colour::Cyan,    Brightness::Dark))),
                     37 => SGR::FGColour(Some((Colour::White,   Brightness::Dark))),
                     39 => SGR::FGColour(None),
                     40 => SGR::BGColour(Some((Colour::Black,   Brightness::Dark))),
                     41 => SGR::BGColour(Some((Colour::Red,     Brightness::Dark))),
                     42 => SGR::BGColour(Some((Colour::Green,   Brightness::Dark))),
                     43 => SGR::BGColour(Some((Colour::Yellow,  Brightness::Dark))),
                     44 => SGR::BGColour(Some((Colour::Blue,    Brightness::Dark))),
                     45 => SGR::BGColour(Some((Colour::Magenta, Brightness::Dark))),
                     46 => SGR::BGColour(Some((Colour::Cyan,    Brightness::Dark))),
                     47 => SGR::BGColour(Some((Colour::White,   Brightness::Dark))),
                     49 => SGR::BGColour(None),
                     90 => SGR::FGColour(Some((Colour::Black,   Brightness::Light))),
                     91 => SGR::FGColour(Some((Colour::Red,     Brightness::Light))),
                     92 => SGR::FGColour(Some((Colour::Green,   Brightness::Light))),
                     93 => SGR::FGColour(Some((Colour::Yellow,  Brightness::Light))),
                     94 => SGR::FGColour(Some((Colour::Blue,    Brightness::Light))),
                     95 => SGR::FGColour(Some((Colour::Magenta, Brightness::Light))),
                     96 => SGR::FGColour(Some((Colour::Cyan,    Brightness::Light))),
                     97 => SGR::FGColour(Some((Colour::White,   Brightness::Light))),
                    100 => SGR::BGColour(Some((Colour::Black,   Brightness::Light))),
                    101 => SGR::BGColour(Some((Colour::Red,     Brightness::Light))),
                    102 => SGR::BGColour(Some((Colour::Green,   Brightness::Light))),
                    103 => SGR::BGColour(Some((Colour::Yellow,  Brightness::Light))),
                    104 => SGR::BGColour(Some((Colour::Blue,    Brightness::Light))),
                    105 => SGR::BGColour(Some((Colour::Magenta, Brightness::Light))),
                    106 => SGR::BGColour(Some((Colour::Cyan,    Brightness::Light))),
                    107 => SGR::BGColour(Some((Colour::White,   Brightness::Light))),
                    _ => Err(EK::InvalidData)?
                }.into(),
                _ => Err(EK::InvalidData)?
            };
            Ok((Some(data), rem))
        } else {
            let (c, rem) = src.strip_prefix(b"?25").and_then(|s| s.split_first()).ok_or(EK::InvalidData)?;
            let show = match c {
                0x68    => true,
                0x6c    => false,
                _ => Err(EK::InvalidData)?
            };
            Ok((Some(DrawCommand::ShowCursor(show)), rem))
        }
    } else {
        let pos = src.iter().position(|c| *c == 0x1b).unwrap_or(src.len());
        let (data, rem) = src.split_at(pos);
        Ok((Some(String::from_utf8(data.to_vec()).map_err(|_| EK::InvalidData)?.into()), rem))
    }
}

