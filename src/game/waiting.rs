//! Implementation of the waiting phase

use crate::display;
use crate::player;


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

