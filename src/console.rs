//! Game master console

use std::sync::Arc;
use std::time::Duration;

use tokio::io;
use tokio::net;
use tokio::sync::{RwLock, watch};
use tokio_util::codec;

use crate::error;
use crate::game;
use crate::player;

use error::WrappedErr;


/// Implementation of the game master logic
///
/// This function starts the game if a SIGUSR1 is received. If a `listener` is
/// passed, the function will accept connections from the associated socket and
/// serve game master consoles over them.
///
pub async fn game_master(
    control: watch::Sender<game::LobbyControl>,
    settings: Settings,
    mut phase: watch::Receiver<game::GamePhase<impl rand::Rng + Send + Sync + 'static>>,
    roster: Arc<RwLock<player::Roster>>,
    mut listener: Option<net::UnixListener>,
) -> Result<(), WrappedErr> {
    use tokio::signal::unix;

    use error::{TryExt, WrappedErr as E};

    let central = Arc::new(RwLock::new(Central {control: control.into(), settings}));
    let mut sigusr1 = unix::signal(unix::SignalKind::user_defined1())
        .map_err(|e| E::new("Could not create SIGUSR1 listener", e))?;

    loop {
        tokio::select!{
            c = accept(listener.as_mut()) => if let Some(conn) = c.or_warn("Could not accept GM conn") {
                let (reader, writer) = conn.into_split();
                let central = central.clone();
                let phase = phase.clone();
                let roster = roster.clone();
                tokio::spawn(async move { serve(reader, writer, central, phase, roster).await });
            },
            r = phase.changed() => r.map_err(|e| E::new("Phase channel closed", e))?,
            s = sigusr1.recv() => if s.is_some() {
                let mut central = central.write().await;
                let msg = central.settings.as_game_control();
                central.control.send_regular(msg).await.or_err("Could not start game");
            },
        }
    }
}


/// Accept a connection from a UnixListener
///
async fn accept(
    listener: Option<&mut net::UnixListener>
) -> io::Result<net::UnixStream> {
    if let Some(listener) = listener {
        listener.accept().await.map(|(s, _)| s)
    } else {
        futures::future::pending().await
    }
}


/// Serve a game master console via the given reader and writer
///
async fn serve(
    reader: impl io::AsyncRead + Unpin,
    writer: impl io::AsyncWrite + Unpin,
    central: Arc<RwLock<Central>>,
    mut phase: watch::Receiver<game::GamePhase<impl rand::Rng>>,
    roster: Arc<RwLock<player::Roster>>,
) {
    use futures::SinkExt;
    use io::AsyncBufReadExt;

    use error::TryExt;

    let mut commands = io::BufReader::new(reader).lines();
    let mut out = codec::FramedWrite::new(writer, codec::LinesCodec::new());

    while !phase.borrow().is_end_of_game() {
        tokio::select!{
            line = commands.next_line() => if let Some(line) = line.or_err("Could not get line").flatten() {
                if match process_line(line.as_ref(), &mut out, &central, &phase, &roster).await {
                    Ok(()) => out.send("OK").await.or_err("Could not send msg to GM"),
                    Err(e) => {
                        let msg = e.to_string();
                        out.send(msg).await.or_err("Could not report error")
                    },
                }.is_none() {
                    break
                }
            },
            r = phase.changed() => if r.or_warn("Phase channel closed").is_none() {
                break
            },
        }
    }
}


/// Process a single command line
///
async fn process_line(
    command: &str,
    out: &mut codec::FramedWrite<impl io::AsyncWrite + Unpin, codec::LinesCodec>,
    central: &Arc<RwLock<Central>>,
    phase: &watch::Receiver<game::GamePhase<impl rand::Rng>>,
    roster: &Arc<RwLock<player::Roster>>,
) -> Result<(), WrappedErr> {
    use std::ops::Deref;

    use futures::{SinkExt, stream::iter};

    use error::{NoneError as N, WrappedErr as E};

    fn parse_bool(input: &str) -> Option<bool> {
        match input {
            "true"  | "t" => Some(true),
            "false" | "f" => Some(false),
            _ => None,
        }
    }

    let mut words = command.split_whitespace();
    match words.next() {
        Some("players") => {
            let entries: Vec<_> = roster
                .read()
                .await
                .iter()
                .enumerate()
                .map(|(n, p)| Ok(format!("{} {} {} {}", n, p.name(), p.is_connected(), p.addr())))
                .collect();
            out.send_all(&mut iter(entries)).await.map_err(|e| E::new("Could not report result", e))
        },
        Some("accept") => {
            let v = words.next().and_then(parse_bool).ok_or_else(|| E::new("Expected 'true' or 'false'", N))?;
            central.write().await.accept_players(v)
        },
        Some("restrict") => {
            let num = words.next().and_then(|s| s.parse().ok()).ok_or_else(|| E::new("Expected number", N))?;
            central.write().await.set_max_players(num)
        },
        Some("kick") => {
            let num: usize = words
                .next()
                .and_then(|s| s.parse().ok())
                .ok_or_else(|| E::new("Expected number", N))?;
            roster.read().await.get(num).map(|p| p.kick()); // TODO: check return value?
            Ok(())
        },
        Some("status") => {
            let status = match phase.borrow().deref() {
                game::GamePhase::Lobby{..}      => "lobby".to_string(),
                game::GamePhase::Waiting{..}    => "waiting".to_string(),
                game::GamePhase::Round{num, ..} => format!("round {}", num),
                game::GamePhase::End            => "end".to_string(),
            };
            out.send(status).await.map_err(|e| E::new("Could not report result", e))
        },
        Some("start") => {
            let mut central = central.write().await;
            let msg = central.settings.as_game_control();
            central.control.send_regular(msg).await
        },
        Some("end") => central.write().await.control.send_regular(game::GameControl::EndOfGame).await,
        Some("set") => {
            let updated = match words.next() {
                Some("virs") => {
                    let num = words
                        .next()
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| E::new("Expected number", N))?;
                    central.write().await.set_virus_count(num)
                },
                Some("ticks") => {
                    let num = words
                        .next()
                        .and_then(|s| s.parse().ok())
                        .ok_or_else(|| E::new("Expected number", N))?;
                    central.write().await.set_tick_duration(Duration::from_millis(num))
                },
                _ => Err(E::new("No such value", N)),
            }?;
            if updated {
                Ok(())
            } else {
                out.send("Value will be sent when game starts")
                    .await
                    .map_err(|e| E::new("Could not report", e))
            }
        },
        Some("get") => match words.next() {
            Some("virs") => out
                .send(central.read().await.settings.virus_count.to_string())
                .await
                .map_err(|e| E::new("Could not report result", e)),
            Some("ticks") => out
                .send(central.read().await.settings.tick_duration.as_millis().to_string())
                .await
                .map_err(|e| E::new("Could not report result", e)),
            _ => Err(E::new("No such value", N)),
        },
        None => Ok(()),
        _ => Err(E::new("No such command", N)),
    }
}


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
    pub fn as_lobby_control(&self) -> game::LobbyControl {
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

