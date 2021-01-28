//! Display rendering utilities

use tokio_util::codec;

use crate::util;


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

