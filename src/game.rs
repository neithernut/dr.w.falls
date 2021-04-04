//! Game implementation

mod lobby;
mod waiting;
mod round;

use tokio::io;


/// A stream of ASCII characters
///
type ASCIIStream<'a, R> = tokio_util::codec::FramedRead<R, ASCIICharDecoder>;


/// Decoder for single ASCII characters
///
/// This decoder yields (confirmed) ASCII characters. In addition, it emulates
/// an enf-of-file condition on ETX (`0x03`) and EOT (`0x04`) by issuing an
/// `UnexpectedEof` error.
///
struct ASCIICharDecoder {}

impl tokio_util::codec::Decoder for ASCIICharDecoder {
    type Item = char;
    type Error = io::Error;

    fn decode(
        &mut self,
        src: &mut bytes::BytesMut
    ) -> Result<Option<Self::Item>, Self::Error> {
        use bytes::Buf;

        if src.has_remaining() {
            match src.get_u8() {
                0x03 | 0x04         => Err(io::ErrorKind::UnexpectedEof.into()),
                c if c.is_ascii()   => Ok(Some(c as char)),
                _                   => Err(io::ErrorKind::InvalidData.into())
            }
        } else {
            src.reserve(1);
            Ok(None)
        }
    }
}


/// Game instructions
///
const INSTRUCTIONS: [&str; 8] = [
    "S:     move left",
    "D:     move right",
    "K:     rotate left",
    "L:     rotate right",
    "space: drop capsule",
    "",
    "P:     pause _your_ game",
    "any:   resume game",
];

