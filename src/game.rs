//! Game implementation

mod lobby;
mod waiting;
mod round;

use std::fmt;

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
    pub async fn transition(&mut self) -> Result<(), ConnTaskError> {
        while !self.transitioned() {
            self.receiver.changed().await.map_err(ConnTaskError::other)?
        }
        Ok(())
    }

    /// Check wehther a transition occured
    ///
    pub fn transitioned(&self) -> bool {
        (self.predicate)(&self.receiver.borrow())
    }
}


/// Game control messages
///
#[derive(Clone, Debug)]
pub enum GameControl {
    Settings{
        /// Number of visuses a field is initialized with
        viruses: u8,
        /// Duration of a tick
        tick: std::time::Duration,
    },
    EndOfGame,
}

impl GameControl {
    /// Check whether the game control indicates an end of game condition
    ///
    pub fn is_end_of_game(&self) -> bool {
        match self {
            Self::EndOfGame => true,
            _ => false,
        }
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
    type Error = ConnTaskError;

    fn decode(
        &mut self,
        src: &mut bytes::BytesMut
    ) -> Result<Option<Self::Item>, Self::Error> {
        use bytes::Buf;

        if src.has_remaining() {
            match src.get_u8() {
                0x03 | 0x04         => Err(ConnTaskError::Terminated),
                c if c.is_ascii()   => Ok(Some(c as char)),
                _                   => Err(io::ErrorKind::InvalidData.into())
            }
        } else {
            src.reserve(1);
            Ok(None)
        }
    }
}


/// Error type for connection task functions
///
/// This error type is intended for functions and utilities used in connection
/// tasks.
///
#[derive(Debug)]
pub enum ConnTaskError {
    /// The connection was terminated in some way
    ///
    /// The conneciton was terminated, presumably actively by the user.
    Terminated,
    /// Some other error occured
    Other(Box<dyn std::error::Error>),
}

impl ConnTaskError {
    /// Create an "other" error
    ///
    pub fn other(e: impl std::error::Error + 'static) -> Self {
        Self::Other(Box::new(e))
    }
}

impl<E: Into<io::Error>> From<E> for ConnTaskError {
    fn from(e: E) -> Self {
        Self::other(e.into())
    }
}

impl std::error::Error for ConnTaskError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            Self::Terminated => None,
            Self::Other(err) => Some(err.as_ref()),
        }
    }
}

impl fmt::Display for ConnTaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Terminated    => write!(f, "Connection terminated"),
            Self::Other(_)      => write!(f, "Error in connection task"),
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

