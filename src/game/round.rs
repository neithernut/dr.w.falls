//! Implementation of the round phase

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use tokio::sync::{Mutex, mpsc, watch};

use crate::display;
use crate::player;
use crate::util;


/// Create ports for communication between connection and control task
///
/// This function returns a pair of ports specific to the round phase, one for
/// the connection task and one for the control task.
///
pub fn ports(scores: impl IntoIterator<Item = player::Tag>) -> (Ports, ControlPorts) {
    let (capsules, scores): (HashMap<_, _>, Vec<_>) = scores
        .into_iter()
        .map(|t| ((t.clone(), Default::default()), t.into()))
        .unzip();
    let player_num = scores.len();

    let (score_sender, score_receiver) = watch::channel(scores);
    let (event_sender, event_receiver) = mpsc::channel(player_num);

    let ports = Ports {scores: score_receiver, events: event_sender, capsules: Arc::new(capsules.clone())};
    let control = ControlPorts {scores: score_sender, events: event_receiver, capsules};

    (ports, control)
}


/// Connection task side of communication ports for the lobby phase
///
#[derive(Clone, Debug)]
pub struct Ports {
    scores: watch::Receiver<Vec<ScoreBoardEntry>>,
    events: mpsc::Sender<(player::Tag, Event)>,
    capsules: Arc<HashMap<player::Tag, CapsulesQueue>>,
}


/// Control task side of communication ports for the lobby phase
///
#[derive(Debug)]
pub struct ControlPorts {
    scores: watch::Sender<Vec<ScoreBoardEntry>>,
    events: mpsc::Receiver<(player::Tag, Event)>,
    capsules: HashMap<player::Tag, CapsulesQueue>,
}


/// Message type for events associated with a particular player
///
#[derive(Clone, Debug)]
enum Event {
    /// Capsules to be sent to ther players
    Capsules(Vec<util::Colour>),
    /// The player's score has changed
    Score(u32),
    /// The player was defeated
    Defeat,
}


/// Queue for distribution of capsules
///
type CapsulesQueue = Arc<Mutex<VecDeque<Capsules>>>;


/// Convenience type for a batch of capsules
///
type Capsules = Vec<(util::ColumnIndex, util::Colour)>;


/// Score board entry for the waiting phase
///
#[derive(Clone, Debug)]
struct ScoreBoardEntry {
    tag: player::Tag,
    round_score: u32,
    state: PlayerState,
}

impl ScoreBoardEntry {
    /// Set the player's round score
    ///
    pub fn set_score(&mut self, score: u32) {
        self.round_score = score
    }

    /// Retrieve the player's state
    pub fn state(&self) -> PlayerState {
        self.state
    }

    /// Set the player's state
    pub fn set_state(&mut self, state: PlayerState) {
        self.state = state
    }
}

impl From<player::Tag> for ScoreBoardEntry {
    fn from(tag: player::Tag) -> Self {
        ScoreBoardEntry {tag, round_score: 0, state: Default::default()}
    }
}

impl display::ScoreBoardEntry for ScoreBoardEntry {
    fn tag(&self) -> &player::Tag {
        &self.tag
    }

    fn active(&self) -> bool {
        self.state != PlayerState::Defeated
    }
}


#[derive(Copy, Clone, Debug, PartialEq)]
enum PlayerState {Playing, Suceeded, Defeated}

impl Default for PlayerState {
    fn default() -> Self {
        Self::Playing
    }
}

