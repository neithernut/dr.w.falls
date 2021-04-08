//! Implementation of the round phase

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use tokio::io;
use tokio::sync::{Mutex, mpsc, watch};

use crate::display;
use crate::field;
use crate::player;
use crate::util;


/// Game logic encapsulation
///
/// This data type provides the core logic for a round, exposed as functions.
/// These include functions for performing both controlled moves and ticks.
///
struct Actor<'a> {
    event_sender: mpsc::Sender<(player::Tag, Event)>,
    capsule_receiver: &'a mut CapsulesQueue,
    player_tag: player::Tag,
    moving: field::MovingField,
    r#static: field::StaticField,
    viruses: HashMap<util::Position, util::Colour>,
    active: ActiveElements,
    next_colours: [util::Colour; 2],
}

impl<'a> Actor<'a> {
    /// Create a new actor
    ///
    pub fn new(
        event_sender: mpsc::Sender<(player::Tag, Event)>,
        capsule_receiver: &'a mut CapsulesQueue,
        player_tag: player::Tag,
        viruses: HashMap<util::Position, util::Colour>,
        next_colours: [util::Colour; 2],
    ) -> Self {
        let moving: field::MovingField = Default::default();
        let r#static = viruses
            .iter()
            .map(|(p, c)| (p.clone(), c.clone()))
            .collect();
        // We'll start with an empty moving field. A capsule will be spawned on the first tick.
        let active = moving.moving_row_index(util::RowIndex::TOP_ROW).into();
        Self {event_sender, capsule_receiver, player_tag, moving, r#static, viruses, active, next_colours}
    }

    /// Perform a controlled move
    ///
    /// If there is a controlled capsule, this function performs the given move
    /// (if possible) and updates the given `field` on the given `display`
    /// accordingly. If there is no controlled capsule, this function does
    /// nothing.
    ///
    pub async fn r#move(
        &mut self,
        display_handle: &mut display::DrawHandle<'_, impl io::AsyncWrite + Unpin>,
        field: &display::FieldUpdater,
        movement: field::Movement,
    ) -> Result<(), super::ConnTaskError> {
        match &mut self.active {
            ActiveElements::Controlled(c) => {
                let updates = c
                    .apply_move(&mut self.moving, &mut self.r#static, movement)
                    .map(|u| u.to_vec())
                    .unwrap_or_default();
                field.update(display_handle, updates).await.map_err(Into::into)
            }
            ActiveElements::Uncontrolled(_) => Ok(()),
        }
    }

    /// Check whether there is a controlled capsule
    ///
    pub fn is_controlled(&self) -> bool {
        self.active.is_controlled()
    }
}


/// Categorization of currently active capsule elements
///
enum ActiveElements {
    /// A controlled capsule exists
    Controlled(field::ControlledCapsule),
    /// Some uncontrolled elements exist including and above this row
    Uncontrolled(field::MovingRowIndex),
}

impl ActiveElements {
    /// Retrieve the lowest row containing active capsule elements
    ///
    pub fn lowest_row(&self) -> field::MovingRowIndex {
        match self {
            Self::Controlled(c) => c.row(),
            Self::Uncontrolled(r) => *r,
        }
    }

    /// Check whether the active elements are controlled
    ///
    pub fn is_controlled(&self) -> bool {
        match self {
            Self::Controlled(_) => true,
            Self::Uncontrolled(_) => false,
        }
    }
}

impl From<field::ControlledCapsule> for ActiveElements {
    fn from(capsule: field::ControlledCapsule) -> Self {
        Self::Controlled(capsule)
    }
}

impl From<field::MovingRowIndex> for ActiveElements {
    fn from(row: field::MovingRowIndex) -> Self {
        Self::Uncontrolled(row)
    }
}


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


/// Determine the lower of two (optional) rows
///
/// If one row is `None`, the other is considered lower.
///
fn lower_row(a: Option<util::RowIndex>, b: Option<util::RowIndex>) -> Option<util::RowIndex> {
    match (a, b) {
        (Some(a), Some(b)) => Some(std::cmp::max(a, b)), // rows are enumerated from the top
        (Some(a), None   ) => Some(a),
        (None,    Some(b)) => Some(b),
        (None,    None   ) => None,
    }
}


/// The minimum number of capsules which would be sent to other players
///
const MIN_CAPSULES_SEND: usize = 2;

