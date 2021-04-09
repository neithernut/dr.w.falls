//! Game implementation

mod lobby;
mod waiting;
mod round;


use tokio::io;
use tokio::net::tcp;

use crate::display;


/// Item type for game update channels
///
pub enum GameUpdate<U,T> {
    Update(U),
    PhaseEnd(PhaseEnd<T>),
}


/// Phase end messages
///
#[derive(Clone)]
pub enum PhaseEnd<T> {
    Transition(T),
    EndOfGame,
}


/// Game control messages
///
enum GameControl {
    Settings{
        /// Number of visuses a field is initialized with
        viruses: u8,
        /// Duration of a tick
        tick: std::time::Duration
    },
    EndOfGame,
}


/// Game phase indication
///
enum GamePhase {
    Lobby,
    Waiting,
    Round(usize),
    EndOfGame,
}


/// Player handle
///
#[derive(Default)]
pub struct PlayerHandle {
    data: std::sync::Arc<()>,
}

impl PlayerHandle {
    /// Generate a tag for this player handle
    ///
    fn tag(&self) -> PlayerTag {
        PlayerTag {data: std::sync::Arc::downgrade(&self.data)}
    }
}


/// Player tag
///
/// A value of this type allows identifying a player (via comparison)
///
#[derive(Clone)]
pub struct PlayerTag {
    data: std::sync::Weak<()>,
}

impl PlayerTag {
    pub fn is_alive(&self) -> bool {
        self.data.strong_count() > 0
    }
}

impl PartialEq<PlayerTag> for PlayerTag {
    fn eq(&self, other: &PlayerTag) -> bool {
        self.data.ptr_eq(&other.data)
    }
}


/// A stream of ASCII characters
///
type ASCIIStream<'a> = tokio_util::codec::FramedRead<tcp::ReadHalf<'a>, ASCIICharDecoder>;


/// Decoder for single ASCII characters
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
            let c = src.get_u8();
            if c.is_ascii() {
                Ok(Some(c as char))
            } else {
                Err(io::ErrorKind::InvalidData.into())
            }
        } else {
            src.reserve(1);
            Ok(None)
        }
    }
}


/// Local Display type
///
type Display<'a> = display::Display<tcp::WriteHalf<'a>>;


/// Retrieve two columns for a Display
///
fn columns(display: &mut Display) -> (display::Area, display::Area) {
    let area = display.area();
    let width = area.width();
    let (left, right) = area.split_vertically(width / 2);
    (
        left.top_padded(1).bottom_padded(1).left_padded(1).right_padded(1),
        right.top_padded(1).bottom_padded(1).left_padded(1).right_padded(1),
    )
}


/// Game instructions
///
const INSTRUCTIONS: &str = concat!(
    "S:     move left\n",
    "D:     move right\n",
    "K:     rotate left\n",
    "L:     rotate right\n",
    "space: drop capsule\n",
    "\n",
    "P:     pause _your_ game\n",
    "any:   resume game\n",
);

