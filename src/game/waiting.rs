//! Implementation of the waiting phase


use crate::display;


/// Score board entry for the waiting phase
///
#[derive(PartialEq)]
struct ScoreBoardEntry {
    name: String,
    score: u32,
    ready: bool,
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
        self.score
    }

    fn extra(&self) -> Self::Extra {
        if self.ready {
            "yes"
        } else {
            "no"
        }
    }

    fn active(&self) -> bool {
        self.tag.is_alive()
    }
}

