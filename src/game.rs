//! Game implementation

mod lobby;
mod waiting;
mod round;

use tokio::io;
use tokio::sync::watch;


/// Utility for awaiting a phase transition
///
pub struct TransitionWatcher<P, F: Fn(&P) -> bool> {
    receiver: watch::Receiver<P>,
    predicate: F,
}

impl<P, F: Fn(&P) -> bool> TransitionWatcher<P, F> {
    /// Create a new transition watcher
    ///
    /// The watcher will receive phase updates via the `receiver`. It will
    /// observe a transition if `predicate` returns `true` for the phase.
    ///
    pub fn new(receiver: watch::Receiver<P>, predicate: F) -> Self {
        Self {receiver, predicate}
    }

    /// Wait for a transition
    ///
    /// This function will return only if either a transition was observed or an
    /// error occured.
    ///
    pub async fn transition(&mut self) -> Result<(), watch::error::RecvError> {
        while !self.transitioned() {
            self.receiver.changed().await?
        }
        Ok(())
    }

    /// Check wehther a transition occured
    ///
    pub fn transitioned(&self) -> bool {
        (self.predicate)(&self.receiver.borrow())
    }
}


/// A stream of ASCII characters
///
type ASCIIStream<R> = tokio_util::codec::FramedRead<R, ASCIICharDecoder>;


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


/// Number of columns to split from the left
///
/// The remeining columns are reserved for the score board.
const COLUMN_SPLIT: u16 = 32;


/// Number of rows to split from the top when displaying instructions
///
/// During some phases, we'll display instructions in the lower part of the left
/// column. This value specifies the number of rows of the left column to use
/// for other content during those phases.
///
const INSTRUCTION_SPLIT: u16 = 12;


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

