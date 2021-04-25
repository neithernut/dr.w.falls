//! Implementation of the lobby phase

use std::fmt;
use std::net::SocketAddr;
use std::sync::Arc;

use tokio::io;
use tokio::sync::{mpsc, oneshot, watch};

use crate::display;
use crate::player;


/// Connection function for the lobby phase
///
/// This function implements the connection task part of the game logic for the
/// lobby phase.
///
pub async fn serve<P>(
    control: Ports,
    mut display: display::Display<impl io::AsyncWrite + Unpin>,
    mut input: impl futures::stream::Stream<Item = Result<char, io::Error>> + Unpin,
    mut phase: super::TransitionWatcher<P, impl Fn(&P) -> bool>,
    token: ConnectionToken,
) -> Result<Option<player::Handle>, super::ConnTaskError> {
    use std::convert::TryInto;

    use futures::stream::StreamExt;

    use super::ConnTaskError;

    let mut scores = control.scores;
    let registration = control.registration;


    // Set up the display
    let mut area = display.area().await?.pad_top(1);
    let mut left = area.split_left(super::COLUMN_SPLIT);
    let mut reg = left.split_top(super::INSTRUCTION_SPLIT);

    reg.place_top(display::StaticText::from("Please enter your name:")).await?;
    reg = reg.pad_top(1);
    let mut name_input = reg.place_top(
        display::LineInput::new((player::MAX_PLAYER_NAME_LEN as u16).try_into().unwrap())
    ).await?;
    let reply_text = reg.place_center(
        display::DynamicText::new((super::COLUMN_SPLIT - 2).try_into().unwrap(), 4u16.try_into().unwrap())
    ).await?;

    left.place_center(display::StaticText::from(super::INSTRUCTIONS.iter().cloned())).await?;

    let max_scores = area.rows().saturating_sub(2);
    let mut score_board = area.place_center(display::ScoreBoard::new(max_scores).show_scores(false)).await?;
    score_board.update(&mut display.handle().await?, scores.borrow().iter(), |_| false).await?;


    // Get the player to register
    let handle = loop {
        tokio::select!{
            res = input.next() => match res {
                Some(Ok(c)) => {
                    let name = name_input
                        .update(&mut display.handle().await?, c)
                        .await?
                        .map(ToString::to_string);
                    if let Some(name) = name {
                        let (reply_sender, reply) = oneshot::channel();
                        registration
                            .send(Registration::new(name, token.clone(), reply_sender))
                            .await
                            .map_err(ConnTaskError::other)?;
                        match reply.await.map_err(|_| io::Error::from(io::ErrorKind::Other))? {
                            RegistrationReply::Accepted(handle) => break handle,
                            RegistrationReply::Denied(reason)   => reply_text
                                .update_single(&mut display.handle().await?, reason)
                                .await?,
                        }
                    }
                }
                Some(Err(e)) if e.kind() != io::ErrorKind::WouldBlock => return Err(e.into()),
                None => return Err(ConnTaskError::Terminated),
                _ => (),
            },
            _ = scores.changed() => score_board
                .update(&mut display.handle().await?, scores.borrow().iter(), |_| false)
                .await?,
            t = phase.transition() => {
                t?;
                reply_text
                    .update_single(&mut display.handle().await?, "The game started without you.")
                    .await?;
                return Ok(None)
            },
        }
    };

    let reg_msg = [
        "You are now registered.",
        "Please wait for the game",
        "to start.",
    ];
    reply_text.update(&mut display.handle().await?, reg_msg.iter()).await?;


    // Wait for the transition, updating scores
    while !phase.transitioned() {
        tokio::select!{
            res = input.next() => match res {
                Some(Err(e)) if e.kind() != io::ErrorKind::WouldBlock => return Err(e.into()),
                None => return Err(ConnTaskError::Terminated),
                _ => (),
            },
            _ = scores.changed() => score_board
                .update(&mut display.handle().await?, scores.borrow().iter(), |t| handle == *t)
                .await?,
            t = phase.transition() => {
                t?;
                break
            },
        }
    }

    Ok(Some(handle))
}


/// Create ports for communication between connection and control task
///
/// This function returns a pair of ports specific to the lobby phase, one for
/// the connection task and one for the control task.
///
pub fn ports() -> (Ports, ControlPorts) {
    let (score_sender, score_receiver) = watch::channel(Default::default());
    let (registration_sender, registration_receiver) = mpsc::channel(20); // TODO: replace hard-coded value?

    let ports = Ports {scores: score_receiver, registration: registration_sender};
    let control = ControlPorts {scores: score_sender, registration: registration_receiver};

    (ports, control)
}


/// Connection task side of communication ports for the lobby phase
///
#[derive(Clone, Debug)]
pub struct Ports {
    scores: watch::Receiver<Vec<player::Tag>>,
    registration: mpsc::Sender<Registration>,
}


/// Control task side of communication ports for the lobby phase
///
#[derive(Debug)]
pub struct ControlPorts {
    scores: watch::Sender<Vec<player::Tag>>,
    registration: mpsc::Receiver<Registration>,
}


/// Control message specific to the lobby phase
///
#[derive(Clone, Debug)]
pub enum LobbyControl {
    Settings{registration_acceptance: bool, max_players: u8},
    GameStart(watch::Receiver<super::GameControl>),
}


/// Registration request
///
#[derive(Debug)]
struct Registration {
    name: String,
    token: ConnectionToken,
    response: oneshot::Sender<RegistrationReply>,
}

impl Registration {
    /// Create a new Registration
    ///
    pub fn new(
        name: String,
        token: ConnectionToken,
        response: oneshot::Sender<RegistrationReply>
    ) -> Self {
        Self {name, token, response}
    }
}


/// Reply to a registration request
///
#[derive(Debug)]
enum RegistrationReply {
    Accepted(player::Handle),
    Denied(DenialReason),
}

impl From<player::Handle> for RegistrationReply {
    fn from(handle: player::Handle) -> Self {
        Self::Accepted(handle)
    }
}

impl From<DenialReason> for RegistrationReply {
    fn from(reason: DenialReason) -> Self {
        Self::Denied(reason)
    }
}


/// Reason for denial of a registration
///
#[derive(Copy, Clone, Debug)]
enum DenialReason {
    AcceptanceClosed,
    MaxPlayers,
    NameTaken,
    TemporaryFailure,
    PermanentFailure,
}

impl fmt::Display for DenialReason {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::AcceptanceClosed  => write!(f, "Registration is closed"),
            Self::MaxPlayers        => write!(f, "Max number of players reached"),
            Self::NameTaken         => write!(f, "Name is already taken"),
            Self::TemporaryFailure  => write!(f, "Temporary registration failure"),
            Self::PermanentFailure  => write!(f, "Permanent registration failure"),
        }
    }
}


/// Connection token
///
#[derive(Clone, Debug)]
pub struct ConnectionToken {
    data: Arc<SocketAddr>,
}

impl From<SocketAddr> for ConnectionToken {
    fn from(addr: SocketAddr) -> Self {
        Self {data: Arc::new(addr)}
    }
}

impl Eq for ConnectionToken {}

impl PartialEq for ConnectionToken {
    fn eq(&self, other: &ConnectionToken) -> bool {
        Arc::ptr_eq(&self.data, &other.data)
    }
}

impl std::hash::Hash for ConnectionToken {
    fn hash<H: std::hash::Hasher>(&self, state: &mut H) {
        Arc::as_ptr(&self.data).hash(state)
    }
}

impl AsRef<SocketAddr> for ConnectionToken {
    fn as_ref(&self) -> &SocketAddr {
        &self.data
    }
}


/// Maximum allowed length for a name
///
const NAME_MAX_LEN: u16 = 16;

