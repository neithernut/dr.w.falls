//! Implementation of the round phase

use std::sync;

use tokio::sync::mpsc;

use crate::display;
use crate::util;


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


/// Unbound capsules
///
type Capsules = Vec<(util::ColumnIndex, util::Colour)>;

