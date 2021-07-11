//! Game implementation

mod lobby;
mod waiting;
mod round;

use std::collections::HashMap;
use std::fmt;
use std::sync::Arc;

use tokio::io;
use tokio::net;
use tokio::sync::{RwLock, watch};

use crate::error;
use crate::player;
use crate::util;


pub use lobby::LobbyControl;


/// Run the game
///
/// This function implements the the overall game phase logic. During the lobby
/// phase, connections will be accepted via the given `listener` and new players
/// are added to the `roster`.
///
pub async fn run_game<R>(
    listener: net::TcpListener,
    lobby_control: watch::Receiver<lobby::LobbyControl>,
    roster: Arc<RwLock<player::Roster>>,
    phase: watch::Sender<GamePhase<R>>,
    phase_receiver: watch::Receiver<GamePhase<R>>,
) -> Result<(), error::WrappedErr>
where R: rand::Rng + rand::SeedableRng + Clone + Send + Sync + fmt::Debug + 'static
{
    use crate::field::prepare_field;
    use error::WrappedErr as E;
    use util::Step;

    let (ports, control) = lobby::ports();
    phase.send(GamePhase::Lobby{ports}).map_err(|e| E::new("Could not send phase updates", e))?;
    let (game_control, _disconnects) = lobby::control(
        control,
        lobby_control,
        phase_receiver,
        listener,
        serve_connection,
        roster.clone(),
    ).await.unwrap();

    let mut num = 1;

    while !game_control.borrow().is_end_of_game() {
        let (ports, control) = waiting::ports(roster.read().await.clone());
        phase.send(GamePhase::Waiting{ports}).map_err(|e| E::new("Could not send phase updates", e))?;
        waiting::control(control, game_control.clone(), roster.clone()).await;

        let mut rng = R::from_entropy();
        let (viruses, tick_duration) = match game_control.borrow().clone() {
            GameControl::Settings{viruses, tick} => {
                let first_row = util::RowIndex::TOP_ROW.forward_checked(FREE_ROWS)
                    .expect("Not enough rows to keep free");
                (prepare_field(&mut rng, first_row, viruses).collect(), tick)
            },
            GameControl::EndOfGame => break,
        };

        let (ports, control) = round::ports(roster.read().await.clone());
        phase
            .send(GamePhase::Round{ports, viruses, tick_duration, rng: rng.clone(), num})
            .map_err(|e| E::new("Could not send phase updates", e))?;
        round::control(control, roster.clone(), &mut rng).await?;

        num = num + 1;
    }

    phase.send(GamePhase::End).map_err(|e| E::new("Could not send final phase updates", e))
}


/// Serve a given connection
///
async fn serve_connection(
    connection: net::TcpStream,
    phase: watch::Receiver<GamePhase<impl rand::Rng + Clone>>,
    token: lobby::ConnectionToken,
) {
    use crate::error::TryExt;

    match do_serve(connection, phase, token).await {
        Err(ConnTaskError::Terminated) => log::info!("Player disconnected"),
        e => { e.or_warn("Lost player"); },
    }
}


/// Actual connection logic
///
async fn do_serve(
    connection: net::TcpStream,
    phase: watch::Receiver<GamePhase<impl rand::Rng + Clone>>,
    token: lobby::ConnectionToken,
) -> Result<(), ConnTaskError> {
    use crate::display::Display;

    use {GamePhase as P, TransitionWatcher as W};

    connection.set_nodelay(true)?;
    let (conn_in, conn_out) = connection.into_split();
    let mut display = Display::new(conn_out, DISPLAY_HEIGHT, DISPLAY_WIDTH);
    let mut input = ASCIIStream::new(conn_in, Default::default());

    let mut me: Option<player::Handle> = Default::default();

    loop {
        let p = phase.borrow().clone();
        match p {
            P::Lobby{ports} => me = lobby::serve(
                ports,
                &mut display,
                &mut input,
                W::new(phase.clone(), |p| if let P::Lobby{..} = p { false } else { true }),
                token.clone(),
            ).await?,
            P::Waiting{ports} => waiting::serve(
                ports,
                &mut display,
                &mut input,
                W::new(phase.clone(), |p| if let P::Waiting{..} = p { false } else { true }),
                me.as_ref().ok_or_else(|| ConnTaskError::other(error::NoneError))?,
            ).await?,
            P::Round{ports, viruses, tick_duration, rng, ..} => round::serve(
                ports,
                &mut display,
                &mut input,
                W::new(phase.clone(), |p| if let P::Round{..} = p { false } else { true }),
                me.as_ref().ok_or_else(|| ConnTaskError::other(error::NoneError))?,
                viruses,
                tick_duration,
                rng,
            ).await?,
            P::End => break Ok(()),
        }
    }
}


/// Game phase updates
#[derive(Debug, Clone)]
pub enum GamePhase<R: rand::Rng> {
    Lobby{ports: lobby::Ports},
    Waiting{ports: waiting::Ports},
    Round{
        ports: round::Ports,
        viruses: HashMap<util::Position, util::Colour>,
        tick_duration: std::time::Duration,
        rng: R,
        num: u32,
    },
    End,
}

impl<R: rand::Rng> Default for GamePhase<R> {
    fn default() -> Self {
        GamePhase::End
    }
}


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
#[derive(Default, Debug)]
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
    /// The operation would block
    WouldBlock,
    /// Some other error occured
    Other(Box<dyn std::error::Error + Send>),
}

impl ConnTaskError {
    /// Create an "other" error
    ///
    pub fn other(e: impl std::error::Error + Send + 'static) -> Self {
        Self::Other(Box::new(e))
    }

    /// Check whether this error just indicates that an operation would block
    ///
    pub fn is_would_block(&self) -> bool {
        if let Self::WouldBlock = self {
            true
        } else {
            false
        }
    }
}

impl<E: Into<io::Error>> From<E> for ConnTaskError {
    fn from(e: E) -> Self {
        let err = e.into();
        if err.kind() == io::ErrorKind::WouldBlock {
            Self::WouldBlock
        } else {
            Self::other(err)
        }
    }
}

impl std::error::Error for ConnTaskError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        if let Self::Other(err) =  self {
            Some(err.as_ref())
        } else {
            None
        }
    }
}

impl fmt::Display for ConnTaskError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Terminated    => write!(f, "Connection terminated"),
            Self::WouldBlock    => write!(f, "Operation would block"),
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


/// Assumed width of the player's terminal
///
const DISPLAY_WIDTH: u16 = 80;


/// Assumed height of the player's terminal
///
const DISPLAY_HEIGHT: u16 = 24;


/// Number of rows at the top to keep free when placing viruses
///
const FREE_ROWS: usize = 4;

