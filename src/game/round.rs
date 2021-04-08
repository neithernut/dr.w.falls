//! Implementation of the round phase

use std::sync;

use tokio::sync::mpsc;

use crate::display;
use crate::gameplay;
use crate::util;


/// Categorization of currently active capsule elements
///
enum ActiveElements {
    /// A controlled capsule exists
    Controlled(gameplay::ControlledCapsule),
    /// Some uncontrolled elements exist including and above this row
    Uncontrolled(gameplay::MovingRowIndex),
}

impl ActiveElements {
    /// Retrieve the lowest row containing active capsule elements
    ///
    pub fn lowest_row(&self) -> gameplay::MovingRowIndex {
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

impl From<gameplay::ControlledCapsule> for ActiveElements {
    fn from(capsule: gameplay::ControlledCapsule) -> Self {
        Self::Controlled(capsule)
    }
}

impl From<gameplay::MovingRowIndex> for ActiveElements {
    fn from(row: gameplay::MovingRowIndex) -> Self {
        Self::Uncontrolled(row)
    }
}


/// Local type for game updates
///
type GameUpdate<E> = super::GameUpdate<sync::Arc<Vec<ScoreBoardEntry>>, E>;


/// Message type for events associated with a particular player
///
enum PlayerEvent {
    /// Capsules to be sent to ther players
    Capsules(Vec<util::Colour>),
    /// The player's score has changed
    Score(u32),
    /// The player was defeated
    Defeat,
}


/// Score board entry for the waiting phase
///
#[derive(PartialEq)]
struct ScoreBoardEntry {
    name: String,
    total_score: u32,
    round_score: u32,
    tag: super::PlayerTag,
    capsule_receiver: CapsuleReceiver,
}

impl ScoreBoardEntry {
    /// Get the capsule receiver associated with this entry
    ///
    pub fn capsule_receiver(&self) -> CapsuleReceiver {
        self.capsule_receiver.clone()
    }
}

impl display::ScoreBoardEntry for ScoreBoardEntry {
    type Tag = super::PlayerTag;

    type Extra = u32;

    fn name(&self) -> &str {
        self.name.as_ref()
    }

    fn tag(&self) -> Self::Tag {
        self.tag.clone()
    }

    fn score(&self) -> u32 {
        self.total_score
    }

    fn extra(&self) -> Self::Extra {
        self.round_score
    }

    fn active(&self) -> bool {
        self.tag.is_alive()
    }
}


/// Wrapper for capsule receivers
///
#[derive(Clone)]
struct CapsuleReceiver {
    inner: sync::Arc<sync::Mutex<mpsc::Receiver<Capsules>>>
}

impl CapsuleReceiver {
    /// Get a locked inner receiver
    ///
    pub fn lock(&self) -> sync::LockResult<sync::MutexGuard<'_, mpsc::Receiver<Capsules>>> {
        self.inner.lock()
    }
}

impl From<mpsc::Receiver<Capsules>> for CapsuleReceiver {
    fn from(inner: mpsc::Receiver<Capsules>) -> Self {
        Self {inner: sync::Arc::new(sync::Mutex::new(inner))}
    }
}

impl PartialEq for CapsuleReceiver {
    fn eq(&self, other: &Self) -> bool {
        sync::Arc::ptr_eq(&self.inner, &other.inner)
    }
}


/// Generate two random colour items
///
fn random_colours(rng: &mut impl rand_core::RngCore) -> [util::Colour; 2] {
    let gen = |i| match i % 3 {
        0 => util::Colour::Red,
        1 => util::Colour::Yellow,
        _ => util::Colour::Blue,
    };

    let mut bytes: [u8; 2] = Default::default();
    rng.fill_bytes(&mut bytes);
    [gen(bytes[0]), gen(bytes[1])]
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


/// Unbound capsules
///
type Capsules = Vec<(util::ColumnIndex, util::Colour)>;


/// The minimum number of capsules which would be sent to other players
///
const MIN_CAPSULES_SEND: usize = 2;

