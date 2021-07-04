//! Implementation of the waiting phase

use tokio::sync::{mpsc, watch};

use crate::display;
use crate::player;


/// Create ports for communication between connection and control task
///
/// This function returns a pair of ports specific to the waiting phase, one for
/// the connection task and one for the control task.
///
pub fn ports() -> (Ports, ControlPorts) {
    let (score_sender, score_receiver) = watch::channel(Default::default());
    let (countdown_sender, countdown_receiver) = watch::channel(Default::default());
    let (readiness_sender, readiness_receiver) = mpsc::channel(20); // TODO: replace hard-coded value?

    let ports = Ports {scores: score_receiver, countdown: countdown_receiver, ready: readiness_sender};
    let control = ControlPorts {scores: score_sender, countdown: countdown_sender, ready: readiness_receiver};

    (ports, control)
}


/// Connection task side of communication ports for the lobby phase
///
#[derive(Clone, Debug)]
pub struct Ports {
    scores: watch::Receiver<Vec<ScoreBoardEntry>>,
    countdown: watch::Receiver<u8>,
    ready: mpsc::Sender<player::Tag>,
}


/// Control task side of communication ports for the lobby phase
///
#[derive(Debug)]
pub struct ControlPorts {
    scores: watch::Sender<Vec<ScoreBoardEntry>>,
    countdown: watch::Sender<u8>,
    ready: mpsc::Receiver<player::Tag>,
}


/// Score board entry for the waiting phase
///
#[derive(Clone, Debug)]
struct ScoreBoardEntry {
    tag: player::Tag,
    ready: bool,
}

impl ScoreBoardEntry {
    /// Mark the player as ready
    ///
    fn set_ready(&mut self) {
        self.ready = true;
    }

    /// Retrieve the player's readyness
    ///
    fn ready(&self) -> bool {
        self.ready
    }
}

impl From<player::Tag> for ScoreBoardEntry {
    fn from(tag: player::Tag) -> Self {
        Self {tag, ready: false}
    }
}

impl display::ScoreBoardEntry for ScoreBoardEntry {
    fn tag(&self) -> &player::Tag {
        &self.tag
    }

    fn active(&self) -> bool {
        self.ready
    }
}

