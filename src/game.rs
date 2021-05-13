//! Game implementation

mod lobby;
mod waiting;
mod round;


use tokio::io;
use tokio::net;
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
                prep.tick_duration,
                prep.rng,
            ).await?;

            match phase_end {
                PhaseEnd::Transition(t) => t,
                PhaseEnd::EndOfGame => return Ok(()),
            }
        };
    }
}


/// Run the game
///
/// This function implements the the overall game phase logic. During the lobby
/// phase, connections will be accepted via the given `listener` and new players
/// are added to the `roster`. Commands will be `control` accepted via the given
/// `control` receiver. Game phases are broadcasted via `game_phase`.
///
pub async fn run_game(
    listener: net::TcpListener,
    control: watch::Receiver<lobby::LobbyControl>,
    game_phase: watch::Sender<GamePhase>,
    roster: std::sync::Arc<std::sync::RwLock<crate::Roster>>,
) -> io::Result<()> {
    use rand_core::SeedableRng;

    use util::Step;

    let (mut lobby_updates, receiver) = watch::channel(GameUpdate::Update(Default::default()));

    game_phase.send(GamePhase::Lobby).map_err(|_| io::ErrorKind::Other)?;
    let control = lobby::control_lobby(
        listener,
        serve_connection,
        &mut lobby_updates,
        receiver,
        control,
        roster.clone()
    ).await?;

    let (mut waiting_updates, mut waiting_receiver) = watch::channel(GameUpdate::Update(Default::default()));
    let (mut ready_sender, mut ready_receiver) = mpsc::channel(10);
    lobby_updates
        .send(WaitingPhasePreq {updates: waiting_receiver, ready_channel: ready_sender}.into())
        .map_err(|_| io::ErrorKind::Other)?;

    let mut round_num = 0usize;

    loop {
        game_phase.send(GamePhase::Waiting).map_err(|_| io::ErrorKind::Other)?;
        waiting::control_waiting(
            &mut waiting_updates,
            ready_receiver,
            control.clone(),
            roster.clone()
        ).await?;

        let (virus_count, tick_duration) = match *control.borrow() {
            GameControl::Settings{viruses, tick} => (viruses, tick),
            GameControl::EndOfGame => break,
        };

        let mut rng = rand_pcg::Pcg64Mcg::from_entropy();
        let viruses = crate::gameplay::prepare_field(
            &mut rng,
            util::RowIndex::TOP_ROW.forward_checked(FREE_ROWS).unwrap(),
            virus_count
        ).collect();

        let (mut round_updates, receiver) = watch::channel(GameUpdate::Update(Default::default()));
        let (event_sender, event_receiver) = mpsc::channel(10);
        waiting_updates
            .send(RoundPhasePreq {
                updates: receiver,
                event_sender,
                viruses,
                tick_duration,
                rng: rng.clone()
            }.into())
            .map_err(|_| io::ErrorKind::Other)?;
        waiting_updates.closed().await;

        round_num = round_num + 1;
        game_phase.send(GamePhase::Round(round_num)).map_err(|_| io::ErrorKind::Other)?;
        round::control_round(
            &mut round_updates,
            event_receiver,
            roster.clone(),
            virus_count,
            rng
        ).await?;

        let updates = watch::channel(GameUpdate::Update(Default::default()));
        waiting_updates = updates.0;
        waiting_receiver = updates.1;
        let ready = mpsc::channel(10);
        ready_sender = ready.0;
        ready_receiver = ready.1;
        round_updates
            .send(WaitingPhasePreq {updates: waiting_receiver, ready_channel: ready_sender}.into())
            .map_err(|_| io::ErrorKind::Other)?;
        round_updates.closed().await;
    }

    Ok(())
}


/// Prequisites for the waiting phase
///
#[derive(Clone, Debug)]
pub struct WaitingPhasePreq<R: rand_core::RngCore> {
    updates: watch::Receiver<waiting::GameUpdate<RoundPhasePreq<R>>>,
    ready_channel: mpsc::Sender<PlayerTag>,
}

impl<U, R: rand_core::RngCore> From<WaitingPhasePreq<R>> for GameUpdate<U, WaitingPhasePreq<R>> {
    fn from(preq: WaitingPhasePreq<R>) -> Self {
        Self::PhaseEnd(PhaseEnd::Transition(preq))
    }
}


/// Prequisites for the round phase
///
#[derive(Clone, Debug)]
pub struct RoundPhasePreq<R: rand_core::RngCore> {
    updates: watch::Receiver<round::GameUpdate<WaitingPhasePreq<R>>>,
    event_sender: mpsc::Sender<(PlayerTag, round::PlayerEvent)>,
    viruses: std::collections::HashMap<util::Position, util::Colour>,
    tick_duration: std::time::Duration,
    rng: R,
}

impl<U, R: rand_core::RngCore> From<RoundPhasePreq<R>> for GameUpdate<U, RoundPhasePreq<R>> {
    fn from(preq: RoundPhasePreq<R>) -> Self {
        Self::PhaseEnd(PhaseEnd::Transition(preq))
    }
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
pub enum GameControl {
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
pub enum GamePhase {
    Lobby,
    Waiting,
    Round(usize),
    EndOfGame,
}

impl Default for GamePhase {
    fn default() -> Self {
        Self::Lobby
    }
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

impl Eq for PlayerTag {}

impl PartialEq<PlayerTag> for PlayerTag {
    fn eq(&self, other: &PlayerTag) -> bool {
        self.data.ptr_eq(&other.data)
    }
}

impl std::hash::Hash for PlayerTag {
    fn hash<H>(&self, state: &mut H)
        where H: std::hash::Hasher
    {
        self.data.as_ptr().hash(state)
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


/// Number of rows at the top to keep free when placing viruses
///
const FREE_ROWS: usize = 4;

