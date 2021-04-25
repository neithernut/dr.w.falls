//! Game implementation

mod lobby;
mod waiting;
mod round;


use tokio::io;
use tokio::net::{self, tcp};
use tokio::sync::{watch, mpsc};

use crate::display;
use crate::util;


/// Serve the given connection
///
/// This function provides the game interface for players as well as the part
/// of the game-logic bound to a specific connection.
///
pub async fn serve_connection(
    mut stream: net::TcpStream,
    updates: watch::Receiver<lobby::GameUpdate<WaitingPhasePreq<impl rand_core::RngCore + Clone>>>,
    reg_channel: mpsc::Sender<lobby::Registration>,
    token: lobby::ConnectionToken,
) -> io::Result<()> {
    let (conn_in, conn_out) = stream.split();

    let mut input = ASCIIStream::new(conn_in, Default::default());
    let mut display = display::Display::new(conn_out, DISPLAY_WIDTH, DISPLAY_HEIGHT);
    display.clear().await?;

    let (handle, phase_end) = lobby::lobby(&mut input, &mut display, updates, reg_channel, token).await?;
    let mut prep = match phase_end {
        PhaseEnd::Transition(t) => t,
        PhaseEnd::EndOfGame => return Ok(()),
    };

    loop {
        prep = {
            display.clear().await?;
            let phase_end = waiting::waiting(
                &mut input,
                &mut display,
                prep.updates,
                prep.ready_channel,
                &handle
            ).await?;

            let prep = match phase_end {
                PhaseEnd::Transition(t) => t,
                PhaseEnd::EndOfGame => return Ok(()),
            };

            display.clear().await?;
            let phase_end = round::round(
                &mut input,
                &mut display,
                prep.updates,
                prep.event_sender,
                &handle,
                prep.viruses,
                prep.tick_diration,
                prep.rng,
            ).await?;

            match phase_end {
                PhaseEnd::Transition(t) => t,
                PhaseEnd::EndOfGame => return Ok(()),
            }
        };
    }
}


/// Prequisites for the waiting phase
///
#[derive(Clone)]
pub struct WaitingPhasePreq<R: rand_core::RngCore> {
    updates: watch::Receiver<waiting::GameUpdate<RoundPhasePreq<R>>>,
    ready_channel: mpsc::Sender<PlayerTag>,
}


/// Prequisites for the round phase
///
#[derive(Clone)]
pub struct RoundPhasePreq<R: rand_core::RngCore> {
    updates: watch::Receiver<round::GameUpdate<WaitingPhasePreq<R>>>,
    event_sender: mpsc::Sender<(PlayerTag, round::PlayerEvent)>,
    viruses: std::collections::HashMap<util::Position, util::Colour>,
    tick_diration: std::time::Duration,
    rng: R,
}


/// Item type for game update channels
///
#[derive(Debug)]
pub enum GameUpdate<U,T> {
    Update(U),
    PhaseEnd(PhaseEnd<T>),
}


/// Phase end messages
///
#[derive(Clone, Debug)]
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
#[derive(Clone, Debug)]
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


/// Convenience type for the JoinHandles of connection tasks
///
pub type ConnTaskHandle = tokio::task::JoinHandle<tokio::io::Result<()>>;


/// A stream of ASCII characters
///
type ASCIIStream<R> = tokio_util::codec::FramedRead<R, ASCIICharDecoder>;


/// Decoder for single ASCII characters
///
#[derive(Default)]
pub struct ASCIICharDecoder {}

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


/// Retrieve two columns for a Display
///
fn columns(display: &mut display::Display<impl io::AsyncWrite + Unpin>) -> (display::Area, display::Area) {
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


/// Assumed width of the player's terminal
///
const DISPLAY_WIDTH: u16 = 80;


/// Assumed height of the player's terminal
///
const DISPLAY_HEIGHT: u16 = 24;

