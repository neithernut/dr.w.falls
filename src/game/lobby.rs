//! Implementation of the lobby phase

use std::sync::Arc;

use tokio::io;
use tokio::sync::{watch, mpsc, oneshot};

use crate::display;


/// Lobby phase function
///
/// This function provides the connection-task side of the lobby phase logic.
///
async fn lobby<E>(
    input: &mut super::ASCIIStream<'_>,
    display: &mut super::Display<'_>,
    updates: &mut watch::Receiver<GameUpdate<E>>,
    reg_channel: mpsc::Sender<Registration>,
    token: ConnectionToken,
) -> io::Result<super::PlayerHandle> {
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
            _ = updates.changed() => match &*updates.borrow() {
                GameUpdate::Update(players) => scoreboard
                    .update(display, players.clone(), &super::PlayerHandle::default().tag())
                    .await?,
                GameUpdate::PhaseEnd(_) => return Err(io::ErrorKind::Other.into()),
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
            _ = updates.changed() => match &*updates.borrow() {
                GameUpdate::Update(players) => scoreboard
                    .update(display, players.clone(), &super::PlayerHandle::default().tag())
                    .await?,
                GameUpdate::PhaseEnd(_) => break,
            },
        }
    }

    Ok(player)
}


/// Local type for game updates
///
pub type GameUpdate<E> = super::GameUpdate<Arc<Vec<ScoreBoardEntry>>, E>;


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
}


/// Connection token
///
#[derive(Clone)]
pub struct ConnectionToken {
    data: Arc<()>,
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
#[derive(PartialEq)]
pub struct ScoreBoardEntry {
    name: String,
    tag: super::PlayerTag,
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

