//! Draw commands and related types

use std::borrow::Cow;

use tokio::io::AsyncWrite;
use tokio_util::codec;

use crate::util;


/// Handle for drawing
///
/// An instance of this type will allow performing drawing operations as well
/// performing predefined operations when dropped. It is intended to be opaque
/// to code outside the `display` module.
///
pub struct DrawHandle<'a, W: AsyncWrite + Unpin> {
    write: codec::FramedWrite<W, ANSIEncoder>,
    termination_seq: &'a [DrawCommand<'a>],
}

impl<'a, W: AsyncWrite + Unpin> Drop for DrawHandle<'a, W> {
    fn drop(&mut self) {
        use futures::SinkExt;
        use futures::stream::iter;

        use crate::error::TryExt;

        let cmds = self.termination_seq.iter().cloned().map(Ok);
        tokio::runtime::Runtime::new()
            .and_then(|r| r.block_on(self.write.send_all(&mut iter(cmds))))
            .or_warn("Failed to send termination sequence")
            .unwrap_or_default()
    }
}


/// Create a draw handle
///
/// The handle will write encoded commands via the given `write`. When dropped,
/// it will issue the given termination sequence.
///
pub fn draw_handle<'a, W: AsyncWrite + Unpin>(
    write: W,
    termination_seq: &'a [DrawCommand<'a>],
) -> DrawHandle<'a, W> {
    DrawHandle {write: codec::FramedWrite::new(write, ANSIEncoder::new()), termination_seq}
}


/// A proxy for sinks
///
/// This trait will allow keeping the exact type of `futures::Sink`s internal to the
/// `display` module. It therefore not intended to be exported by `display` at all.
///
pub trait SinkProxy {
    /// Type of the underlying sink
    ///
    type Sink;

    /// Retrieve a reference to the underlying sink
    ///
    fn as_sink(&mut self) -> &mut Self::Sink;
}

impl<'a, W: AsyncWrite + Unpin> SinkProxy for DrawHandle<'a, W> {
    type Sink = codec::FramedWrite<W, ANSIEncoder>;

    fn as_sink(&mut self) -> &mut Self::Sink {
        &mut self.write
    }
}


/// Encoder for `DrawCommand`s
///
/// This encoder will encode `DrawCommand`s as ANSI escape sequenes.
///
pub struct ANSIEncoder;

impl ANSIEncoder {
    /// Create a new encoder
    ///
    pub fn new() -> Self {
        Self{}
    }
}

impl codec::Encoder<DrawCommand<'_>> for ANSIEncoder {
    type Error = std::io::Error;

    fn encode(&mut self, cmd: DrawCommand, dst: &mut bytes::BytesMut) -> Result<(), Self::Error> {
        use bytes::BufMut;

        use DrawCommand as DC;

        match cmd {
            DC::ClearScreen    => dst.put_slice(b"\x1b[2J"),
            DC::SetPos(r, c)   => dst.put_slice(format!("\x1b[{};{}H", r + 1, c + 1).as_bytes()),
            DC::Format(param)  => dst.put_slice(format!("\x1b[{}m", param.code()).as_bytes()),
            DC::Text(s)        => dst.put_slice(s.as_bytes()),
            DC::ShowCursor(true)    => dst.put_slice(b"\x1b[?25h"),
            DC::ShowCursor(false)   => dst.put_slice(b"\x1b[?25l"),
        }
        Ok(())
    }
}


/// Representation of a draw command
///
#[allow(dead_code)]
#[derive(Clone, Debug)]
pub enum DrawCommand<'s> {
    /// Clear the entire screen
    ClearScreen,
    /// Set the cursor's position
    ///
    /// The first component denotes the row, the second one the column. Both are
    /// zero-based, meaning that `0` refers to the first row or column.
    SetPos(u16, u16),
    /// Select Graphic Rendition
    Format(SGR),
    /// Put text on the screen at the current cursor position
    Text(Cow<'s, str>),
    /// Show (or hide) the cursor
    ///
    /// The flag indicates whether the cursor is shown or not.
    ShowCursor(bool),
}

impl<'s> From<(u16, u16)> for DrawCommand<'s> {
    fn from((r, c): (u16, u16)) -> Self {
        Self::SetPos(r, c)
    }
}

impl<'s, F: Into<SGR>> From<F> for DrawCommand<'s> {
    fn from(fmt: F) -> Self {
        Self::Format(fmt.into())
    }
}

impl<'s> From<&'s str> for DrawCommand<'s> {
    fn from(text: &'s str) -> Self {
        Self::Text(text.into())
    }
}

impl<'s> From<String> for DrawCommand<'s> {
    fn from(text: String) -> Self {
        Self::Text(text.into())
    }
}

impl<'s> From<Cow<'s, str>> for DrawCommand<'s> {
    fn from(text: Cow<'s, str>) -> Self {
        Self::Text(text)
    }
}


/// Representation of some selected "Select Graphic Rendition" parameters
///
#[allow(dead_code)]
#[derive(Copy, Clone, Debug)]
pub enum SGR {
    /// Reset to default formatting
    Reset,
    /// Change intensity
    ///
    /// A value of `None` will reset the intensity to the default.
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
    FGColour(Option<(Colour, Brightness)>),
    /// Set the background colour
    ///
    /// A value of `None` will reset the colour to the default.
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

impl From<Intensity> for SGR {
    fn from(int: Intensity) -> Self {
        Some(int).into()
    }
}

impl From<Option<Intensity>> for SGR {
    fn from(int: Option<Intensity>) -> Self {
        SGR::Intensity(int)
    }
}

impl From<Colour> for SGR {
    fn from(colour: Colour) -> Self {
        (colour, Default::default()).into()
    }
}

impl From<(Colour, Brightness)> for SGR {
    fn from((colour, brightness): (Colour, Brightness)) -> Self {
        (colour, brightness).into()
    }
}

impl From<Option<(Colour, Brightness)>> for SGR {
    fn from(param: Option<(Colour, Brightness)>) -> Self {
        Self::FGColour(param)
    }
}


/// Representation of intensity
///
#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub enum Intensity {
    Bold,
    Faint,
}


/// Representation of the basic colour supported by terminals
///
#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub enum Colour {
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


/// Representation of brightness
///
#[derive(Copy, Clone, Debug)]
#[allow(dead_code)]
pub enum Brightness {
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

