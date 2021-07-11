//! Game master console

use tokio::sync::watch;

use crate::error;
use crate::game;


/// Game settings
///
#[derive(Clone, Default, Debug)]
pub struct Settings {
    pub accept_players: bool,
    pub max_players: u8,
    pub virus_count: u8,
    pub tick_duration: std::time::Duration,
}

impl Settings {
    /// Create a LobbyControl message reflecting the relevant settings
    fn as_lobby_control(&self) -> game::LobbyControl {
        game::LobbyControl::Settings{
            registration_acceptance: self.accept_players,
            max_players: self.max_players,
        }
    }

    /// Create a GameControl message reflecting the relevant settings
    fn as_game_control(&self) -> game::GameControl {
        game::GameControl::Settings{viruses: self.virus_count, tick: self.tick_duration}
    }
}


/// Common sender for both lobby and "regular" control
///
enum ControlSender {
    Lobby(watch::Sender<game::LobbyControl>),
    Regular(watch::Sender<game::GameControl>),
}

impl ControlSender {
    /// Send a regular control message, switching if necessary
    ///
    /// If the sender is a `Lobby`, this function will issue a game start
    /// message and re-initialize the sender as a `Regular` with an appropriate
    /// sender. Otherwise, the given contol message will be just sent over the
    /// existing channel.
    ///
    pub async fn send_regular(
        &mut self,
        message: game::GameControl,
    ) -> Result<(), error::WrappedErr<Box<dyn std::error::Error>>> {
        type E = error::WrappedErr<Box<dyn std::error::Error>>;

        match self {
            Self::Lobby(old) => {
                let (sender, receiver) = watch::channel(message);
                old.send(game::LobbyControl::GameStart(receiver))
                    .map_err(|e| E::new("Could not send game start message", Box::new(e)))?;
                old.closed().await;
                *self = Self::Regular(sender);
                Ok(())
            },
            Self::Regular(sender) => sender
                .send(message)
                .map_err(|e| E::new("Could not send control message", Box::new(e))),
        }
    }

    /// Retrieve a reference of the inner lobby control sender, if any
    ///
    pub fn as_lobby_sender(&self) -> Option<&watch::Sender<game::LobbyControl>> {
        if let Self::Lobby(sender) = self {
            Some(sender)
        } else {
            None
        }
    }

    /// Retrieve a reference of the inner regular game control sender, if any
    ///
    pub fn as_regular_sender(&self) -> Option<&watch::Sender<game::GameControl>> {
        if let Self::Regular(sender) = self {
            Some(sender)
        } else {
            None
        }
    }
}

impl From<watch::Sender<game::LobbyControl>> for ControlSender {
    fn from(sender: watch::Sender<game::LobbyControl>) -> Self {
        Self::Lobby(sender)
    }
}

