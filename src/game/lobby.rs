//! Implementation of the lobby phase

use std::sync::Arc;
use std::sync::RwLock;

use tokio::io;
use tokio::net;
use tokio::sync::{watch, mpsc, oneshot};

use crate::display;
use crate::Roster;


/// Lobby phase function
///
/// This function provides the connection-task side of the lobby phase logic.
///
pub async fn lobby<E: Clone>(
    input: &mut super::ASCIIStream<impl io::AsyncRead + Unpin>,
    display: &mut display::Display<impl io::AsyncWrite + Unpin>,
    mut updates: watch::Receiver<GameUpdate<E>>,
    reg_channel: mpsc::Sender<Registration>,
    token: ConnectionToken,
) -> io::Result<(super::PlayerHandle, super::PhaseEnd<E>)> {
    use futures::stream::StreamExt;

    // Set up display
    let (left, right) = super::columns(display);
    let (mut text, left) = left.top_in("Enter name:");
    let (mut name_display, left) = left
        .top_padded(1)
        .top_in(display::LineInputFactory::new(NAME_MAX_LEN));
    let mut scoreboard: display::ScoreBoard<ScoreBoardEntry> = right
        .topleft_in(display::ScoreBoardFactory::default());

    text.draw(display).await?;
    scoreboard.render_heading(display, "").await?;

    // Get the player to register
    let mut name: String = Default::default();
    let player: super::PlayerHandle = loop {
        tokio::select! {
            res = input.next() => match res {
                Some(Ok('\x03')) | Some(Ok('\x04')) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Some(Ok('\x0A')) | Some(Ok('\x0D')) => {
                    // "Enter": try to perform registration
                    let (response_send, response) = oneshot::channel();
                    reg_channel
                        .send(Registration::new(name.clone(), token.clone(), response_send))
                        .await
                        .map_err(|_| io::Error::from(io::ErrorKind::Other))?;
                    match response.await.map_err(|_| io::Error::from(io::ErrorKind::Other))? {
                        RegistrationReply::Accepted(handle) => break handle,
                        RegistrationReply::Denied(_) => (), // TODO: handle reason
                    }
                }
                Some(Ok('\x08')) => {
                    // Backspace: remove the last char from the name
                    name.pop();
                    name_display.update(display, name.as_ref()).await?;
                }
                Some(Ok(c)) => if !c.is_ascii_control() && name.len() < NAME_MAX_LEN as usize {
                    name.push(c);
                    name_display.update(display, name.as_ref()).await?;
                },
                Some(Err(e)) if e.kind() == io::ErrorKind::WouldBlock => (),
                Some(Err(e)) => return Err(e),
                None => (),
            },
            _ = updates.changed() => {
                let players = match &*updates.borrow() {
                    GameUpdate::Update(players) => players.clone(),
                    GameUpdate::PhaseEnd(e) => return Err(io::ErrorKind::Other.into()),
                };
                scoreboard.update(display, players, &super::PlayerHandle::default().tag()).await?;
            },
        }
    };

    // Wait for game to start
    left.top_padded(1).top_in("You are registred.\nAwaiting game.").0.draw(display).await?;
    loop {
        tokio::select! {
            res = input.next() => match res {
                Some(Ok('\x03')) | Some(Ok('\x04')) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Some(Err(e)) if e.kind() != io::ErrorKind::WouldBlock => return Err(e),
                _ => (),
            },
            _ = updates.changed() => {
                let players = match &*updates.borrow() {
                    GameUpdate::Update(players) => players.clone(),
                    GameUpdate::PhaseEnd(e) => break Ok((player, e.clone())),
                };
                scoreboard.update(display, players, &super::PlayerHandle::default().tag()).await?;
            },
        }
    }
}


/// Lobby control function
///
/// This function implements the central control logic for the lobby phase.
///
pub async fn control_lobby<F, R, O>(
    listener: net::TcpListener,
    serve_conn: F,
    update_sender: &mut watch::Sender<GameUpdate<R>>,
    update_receiver: watch::Receiver<GameUpdate<R>>,
    mut control: watch::Receiver<LobbyControl>,
    roster: Arc<RwLock<Roster>>,
) -> io::Result<watch::Receiver<super::GameControl>>
where F: Fn(
        net::TcpStream,
        watch::Receiver<GameUpdate<R>>,
        mpsc::Sender<Registration>,
        ConnectionToken,
      ) -> O + 'static + Send + Sync + Copy,
      R: 'static + Send + Sync + std::fmt::Debug,
      O: std::future::Future<Output = io::Result<()>> + Send,
{
    use crate::util::TryExt;

    let mut accept = true;
    let mut max_players: u8 = 20;

    let (registrator, mut registrations) = mpsc::channel::<Registration>(1);

    let mut tokens: std::collections::HashMap<ConnectionToken, super::ConnTaskHandle> = Default::default();

    loop {
        tokio::select! {
            stream = listener.accept(), if accept => {
                let (stream, peer) = stream?;
                log::info!("Accepting connection from {}", peer);
                let updates = update_receiver.clone();
                let reg_chan = registrator.clone();
                let token = ConnectionToken {data: Arc::new(peer)};

                let conn_task = tokio::spawn({
                    let token = token.clone();
                    async move { serve_conn(stream, updates, reg_chan, token).await }
                });
                tokens.insert(token, conn_task);
            },
            _ = control.changed() => match &*control.borrow() {
                LobbyControl::Settings{registration_acceptance: a, max_players: m} => {
                    accept = *a;
                    max_players = *m;
                },
                LobbyControl::GameStart(c) => break Ok(c.clone()),
            },
            registration = registrations.recv() => if let Some(r) = registration {
                let res = if let Ok(mut roster) = roster.write() {
                    if !accept {
                        DenialReason::AcceptanceClosed.into()
                    } else if roster.len() >= max_players as usize {
                        DenialReason::MaxPlayers.into()
                    } else if roster.iter().any(|p| p.name == r.name) {
                        DenialReason::NameTaken.into()
                    } else if let Some(conn_handle) = tokens.remove(&r.token) {
                        let player_handle: super::PlayerHandle = Default::default();
                        roster.push(
                            crate::Player::new(r.name, player_handle.tag(), *r.token.data, conn_handle)
                        );
                        let scores: Vec<_> = roster
                            .iter()
                            .map(|p| ScoreBoardEntry::new(p.name.clone(), p.tag.clone()))
                            .collect();
                        update_sender
                            .send(GameUpdate::Update(Arc::new(scores)))
                            .or_warn("Could not send updates");
                        RegistrationReply::Accepted(player_handle)
                    } else {
                        log::warn!("No connection token found for {}", r.token.data);
                        DenialReason::Other.into()
                    }
                } else {
                    log::warn!("Could not access roster");
                    DenialReason::RosterAccess.into()
                };
                r.response.send(res).ok().or_warn("Failed to send reply");
            },
        }
    }
}


/// Local type for game updates
///
pub type GameUpdate<E> = super::GameUpdate<Arc<Vec<ScoreBoardEntry>>, E>;


/// Control message specific to the lobby phase
///
pub enum LobbyControl {
    Settings{registration_acceptance: bool, max_players: u8},
    GameStart(watch::Receiver<super::GameControl>),
}


/// Registration request
///
pub struct Registration {
    name: String,
    token: ConnectionToken,
    response: oneshot::Sender<RegistrationReply>
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
pub enum RegistrationReply {
    Accepted(super::PlayerHandle),
    Denied(DenialReason)
}

impl From<DenialReason> for RegistrationReply {
    fn from(reason: DenialReason) -> Self {
        Self::Denied(reason)
    }
}


/// Reason for denial of a registration
///
pub enum DenialReason {
    AcceptanceClosed,
    MaxPlayers,
    NameTaken,
    RosterAccess,
    Other,
}


/// Connection token
///
#[derive(Clone)]
pub struct ConnectionToken {
    data: Arc<std::net::SocketAddr>,
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


/// Score board entry for the lobby phase
///
#[derive(Debug, PartialEq)]
pub struct ScoreBoardEntry {
    name: String,
    tag: super::PlayerTag,
}

impl ScoreBoardEntry {
    /// Create a new score board entry
    ///
    pub fn new(name: String, tag: super::PlayerTag) -> Self {
        Self {name, tag}
    }
}

impl display::ScoreBoardEntry for ScoreBoardEntry {
    type Tag = super::PlayerTag;

    type Extra = &'static str;

    fn name(&self) -> &str {
        self.name.as_ref()
    }

    fn tag(&self) -> Self::Tag {
        self.tag.clone()
    }

    fn score(&self) -> u32 {
        0
    }

    fn extra(&self) -> Self::Extra {
        ""
    }

    fn active(&self) -> bool {
        self.tag.is_alive()
    }
}


/// Maximum allowed length for a name
///
const NAME_MAX_LEN: u16 = 16;

