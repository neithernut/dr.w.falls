//! Implementation of the round phase

use crate::display;
use crate::player;
use crate::util;


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

