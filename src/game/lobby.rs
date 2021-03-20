//! Implementation of the lobby phase

use crate::display;


/// Score board entry for the lobby phase
///
struct ScoreBoardEntry {
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

