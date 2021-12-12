//! Display tests

use super::*;


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

