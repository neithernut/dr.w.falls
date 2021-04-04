//! Implementation of the waiting phase


use std::sync::Arc;

use crate::display;
use crate::util;


/// Local type for game updates
///
type GameUpdate<S, R> = super::GameUpdate<(Arc<Vec<ScoreBoardEntry>>, u8), EndData<S, R>>;


/// Local type for phase end
///
type PhaseEnd<S, R> = super::PhaseEnd<EndData<S, R>>;


/// Phase end data
///
pub struct EndData<S, R> {
    sender: S,
    receiver: R,
    field: Vec<(util::Position, util::Colour)>,
    tick: std::time::Duration,
    rng: rand_pcg::Pcg32,
}

impl<S, R> EndData<S, R> {
    pub fn new(
        sender: S,
        receiver: R,
        field: Vec<(util::Position, util::Colour)>,
        tick: std::time::Duration,
        rng: rand_pcg::Pcg32,
    ) -> Self {
        Self {sender, receiver, field, tick, rng}
    }
}


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

