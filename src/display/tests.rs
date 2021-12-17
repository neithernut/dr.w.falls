//! Display tests

use quickcheck::{Arbitrary, Gen, TestResult};

use super::*;


#[quickcheck]
fn draw_handle_drop(
    mut data: Vec<commands::DrawCommand<'static>>,
    term: Vec<commands::DrawCommand<'static>>,
) -> std::io::Result<TestResult> {
    use futures::SinkExt;

    use commands::{DrawCommand as DC, SinkProxy};

    let rt = tokio::runtime::Runtime::new()?;

    let mut buf = Vec::new();

    let mut handle = commands::draw_handle(&mut buf, term.as_ref());
    rt.block_on(handle.as_sink().send_all(&mut futures::stream::iter(data.iter().cloned().map(Ok))))?;
    drop(handle);

    data.extend(term);
    if data.windows(2).any(|w| if let [DC::Text(_), DC::Text(_)] = w { true } else { false }) {
        Ok(TestResult::discard())
    } else {
        draw_commands_from(buf.as_ref())
            .try_fold(Vec::new(), |mut a, c| { a.push(c?); Ok(a) })
            .map(|r| TestResult::from_bool(data == r))
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
    pub fn instantiate<W: tokio::io::AsyncWrite + Unpin>(
        self,
        handle: DrawHandle<'static, W>,
    ) -> area::Area<'static, DrawHandle<'static, W>, W> {
        area::create_area_full(handle, self.row_a, self.col_a, self.row_b, self.col_b)
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

