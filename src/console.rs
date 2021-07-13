//! Game master console

use std::time::Duration;

use tokio::sync::watch;

use crate::error;
use crate::game;

use error::WrappedErr;


/// Utility struct for central objects shared between all consoles
///
struct Central {
    pub control: ControlSender,
    pub settings: Settings,
}

impl Central {
    /// Set and send accept player setting
    ///
    /// This function returns an error if `control` is not a
    /// `ControlSender::Lobby`.
    ///
    pub fn accept_players(&mut self, accept: bool) -> Result<(), WrappedErr> {
        self.settings.accept_players = accept;
        self.send_lobby_settings()
    }

    /// Set and send max player setting
    ///
    /// This function returns an error if `control` is not a
    /// `ControlSender::Lobby`.
    ///
    pub fn set_max_players(&mut self, max_players: u8) -> Result<(), WrappedErr> {
        self.settings.max_players = max_players;
        self.send_lobby_settings()
    }

    /// Set and send virus count setting
    ///
    /// This function returns an error if `control` is not a
    /// `ControlSender::Regular`.
    ///
    pub fn set_virus_count(&mut self, virus_count: u8) -> Result<bool, WrappedErr> {
        self.settings.virus_count = virus_count;
        self.send_game_settings()
    }

    /// Set and send tock duration setting
    ///
    /// This function returns an error if `control` is not a
    /// `ControlSender::Regular`.
    ///
    pub fn set_tick_duration(&mut self, duration: Duration) -> Result<bool, WrappedErr> {
        self.settings.tick_duration = duration;
        self.send_game_settings()
    }

    /// Send the current lobby settings
    ///
    /// Send the current lobby settings via the control channel. This function
    /// returns an error if `control` is not a `ControlSender::Lobby`.
    ///
    pub fn send_lobby_settings(&mut self) -> Result<(), WrappedErr> {
        self.control
            .as_lobby_sender()
            .ok_or_else(|| error::WrappedErr::new("Not in lobby phase", error::NoneError))?
            .send(self.settings.as_lobby_control())
            .map_err(|e| error::WrappedErr::new("Could not send new settings", e))
    }

    /// Send the current game settings
    ///
    /// Send the current game settings via the control channel. This function
    /// returns an error if `control` is not a `ControlSender::Regular`.
    ///
    pub fn send_game_settings(&mut self) -> Result<bool, error::WrappedErr> {
        if let Some (sender) = self.control.as_regular_sender() {
            sender
                .send(self.settings.as_game_control())
                .map(|_| true)
                .map_err(|e| error::WrappedErr::new("Could not send new settings", e))
        } else {
            Ok(false)
        }
    }
}


/// Game settings
///
#[derive(Clone, Default, Debug)]
pub struct Settings {
    pub accept_players: bool,
    pub max_players: u8,
    pub virus_count: u8,
    pub tick_duration: Duration,
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
    ) -> Result<(), error::WrappedErr> {

        match self {
            Self::Lobby(old) => {
                let (sender, receiver) = watch::channel(message);
                old.send(game::LobbyControl::GameStart(receiver))
                    .map_err(|e| error::WrappedErr::new("Could not send game start message", e))?;
                old.closed().await;
                *self = Self::Regular(sender);
                Ok(())
            },
            Self::Regular(sender) => sender
                .send(message)
                .map_err(|e| error::WrappedErr::new("Could not send control message", e)),
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

