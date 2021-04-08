//! Implementation of the round phase

use std::collections::HashMap;
use std::sync;

use tokio::io;
use tokio::sync::mpsc;
use tokio::time;

use crate::display;
use crate::gameplay;
use crate::util;


/// Game logic encapsulation
///
/// This data type provides the core logic for a round, exposed as functions.
/// These include functions for performing both controlled moves and ticks.
///
struct Actor<'a> {
    event_sender: mpsc::Sender<(super::PlayerTag, PlayerEvent)>,
    capsule_receiver: &'a mut mpsc::Receiver<Capsules>,
    player_tag: super::PlayerTag,
    moving: gameplay::MovingField,
    r#static: gameplay::StaticField,
    viruses: HashMap<util::Position, util::Colour>,
    active: ActiveElements,
    next_colours: [util::Colour; 2],
}

impl<'a> Actor<'a> {
    /// Create a new actor
    ///
    pub fn new(
        event_sender: mpsc::Sender<(super::PlayerTag, PlayerEvent)>,
        capsule_receiver: &'a mut mpsc::Receiver<Capsules>,
        player_tag: super::PlayerTag,
        viruses: HashMap<util::Position, util::Colour>,
        next_colours: [util::Colour; 2],
    ) -> Self {
        let moving: gameplay::MovingField = Default::default();
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
        display: &mut super::Display<'_>,
        field: &display::PlayField,
        movement: gameplay::Movement,
    ) -> io::Result<()> {
        let moving = &mut self.moving;
        let r#static = &mut self.r#static;

        match &mut self.active {
            ActiveElements::Controlled(c) => {
                let updates = c
                    .apply_move(moving, r#static, movement)
                    .map(|u| u.to_vec())
                    .unwrap_or_default();
                field.update(display, updates).await
            }
            ActiveElements::Uncontrolled(_) => Ok(()),
        }
    }

    /// Perform a tick
    ///
    /// This function performs the settling, elimination and unsettling
    /// operation and communicates all the events emerging through the
    /// event sender encapsulated in the actor. After the unsettling step,
    /// all unsettled elements are moves one rows downwards.
    ///
    /// Furthermore, this function spawns either unbound capsule elements
    /// received via the encapsulated receiver or a new controlled capsule if
    /// necessary.
    ///
    /// The given `field` is updated via the given `display` accordingly.
    ///
    pub async fn tick(
        &mut self,
        display: &mut super::Display<'_>,
        field: &display::PlayField,
        rng: &mut impl rand_core::RngCore,
    ) -> io::Result<()> {
        let lowest = self.moving.row_index_from_moving(self.active.lowest_row());

        let (settled, mut lowest) = gameplay::settle_elements(&mut self.moving, &mut self.r#static, lowest);

        if !settled.is_empty() {
            // A lot of interesting stuff only happens if elements settled
            let eliminated = gameplay::eliminate_elements(&mut self.r#static, &settled);
            lowest = lower_row(
                gameplay::unsettle_elements(&mut self.moving, &mut self.r#static, &eliminated),
                lowest
            );

            if eliminated.positions().fold(false, |c, p| c || self.viruses.remove(&p).is_some()) {
                self.send_event(PlayerEvent::Score(self.viruses.len() as u32)).await?;
            }
            if eliminated.row_count() > MIN_CAPSULES_SEND {
                let capsules = eliminated.rows_of_four().map(|(c, _)| *c).collect();
                self.send_event(PlayerEvent::Capsules(capsules)).await?;
            }
            if gameplay::defeated(&self.r#static) {
                self.send_event(PlayerEvent::Defeat).await?;
            }

            field.update(display, eliminated.positions().map(|p| (p, None))).await?;

            if let Some(lowest) = lowest {
                self.active = self.moving.moving_row_index(lowest).into();
            }
        }

        if lowest.is_some() {
            // We still have moving elements.
            field.update(display, self.moving.tick()).await
        } else {
            // There are no moving element left. We need to respawn something.
            use util::RowIndex;
            tokio::select! {
                biased;
                capsules = self.capsule_receiver.recv() => if let Some(capsules) = capsules {
                    self.active = self.moving.moving_row_index(RowIndex::TOP_ROW).into();
                    return field.update(display, self.moving.spawn_single_capsules(capsules)).await;
                },
                _ = std::future::ready(()) => (),
            };

            // We didn't receive any unbound capsules, spawn a controlled capsule.
            let (capsule, updates) = gameplay::ControlledCapsule::spawn_capsule(
                &mut self.moving,
                &self.next_colours
            );
            self.next_colours = random_colours(rng);

            self.active = capsule.into();
            field.update(display, updates.iter().cloned()).await?;
            field.place_next_elements(display, self.next_colours[0], self.next_colours[1]).await
        }
    }

    /// Check whether there is a controlled capsule
    ///
    pub fn is_controlled(&self) -> bool {
        self.active.is_controlled()
    }

    /// Check whether we are defeated
    ///
    pub fn is_defeated(&self) -> bool {
        gameplay::defeated(&self.r#static)
    }

    /// Send the given event
    ///
    async fn send_event(&self, event: PlayerEvent) -> io::Result<()> {
        self.event_sender
            .send((self.player_tag.clone(), event))
            .await
            .map_err(|_| io::Error::from(io::ErrorKind::Other))
    }
}


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


/// A paubable/resumable repetition timer
///
struct Timer {
    inner: ResumableInterval,
    duration: time::Duration,
}

impl Timer {
    /// Create a new timer
    ///
    /// The timer will trigger each time the given duration elapsed.
    ///
    pub fn new(duration: time::Duration) -> Self {
        Self {inner: ResumableInterval::Interval(time::interval(duration), time::Instant::now()), duration}
    }

    /// Completes on the next tick
    ///
    pub async fn tick(&mut self) -> time::Instant {
        match &mut self.inner {
            ResumableInterval::Interval(i, t) => {
                *t = i.tick().await;
                *t
            },
            ResumableInterval::Remaining(_) => std::future::pending().await,
        }
    }

    /// Pause the timer
    ///
    /// This function halts the timer and stored the amount of time remaining
    /// until the next tick. If the timer is already paused, this function
    /// doesn't have any effect.
    ///
    pub fn pause(&mut self) {
        match self.inner {
            ResumableInterval::Interval(_, t) => self.inner = ResumableInterval::Remaining(t.elapsed()),
            ResumableInterval::Remaining(_) => (),
        }
    }

    /// Resume the timer
    ///
    /// This function restarts the timer. The next tick will be scheduled
    /// according to the duration previously stored by `pause`. This function
    /// only has any effect if the timer is paused.
    ///
    pub fn resume(&mut self) {
        match self.inner {
            ResumableInterval::Interval(..) => (),
            ResumableInterval::Remaining(d) => {
                let start = time::Instant::now() + d;
                self.inner = ResumableInterval::Interval(time::interval_at(start, self.duration), start)
            },
        }
    }

    /// Check whether the timer is paused
    ///
    pub fn is_paused(&self) -> bool {
        match self.inner {
            ResumableInterval::Interval(..) => false,
            ResumableInterval::Remaining(..) => true,
        }
    }
}


/// Enum representing the possible states of a `Timer`
///
enum ResumableInterval {
    Interval(time::Interval, time::Instant),
    Remaining(time::Duration),
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


/// Grace period before the first tick
///
const GRACE_PERIOD: time::Duration = time::Duration::from_secs(2);

