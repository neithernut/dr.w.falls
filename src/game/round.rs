//! Implementation of the round phase

use std::collections::{HashMap, VecDeque};
use std::sync::Arc;

use tokio::io;
use tokio::sync::{Mutex, RwLock, mpsc, watch};
use tokio::time;

use crate::display;
use crate::error;
use crate::field;
use crate::player;
use crate::util;


/// Round phase function
///
/// This function implements the connection task part of the game logic for the
/// round phase.
///
pub async fn serve<P>(
    control: Ports,
    display: &mut display::Display<impl io::AsyncWrite + Unpin>,
    mut input: impl futures::stream::Stream<Item = Result<char, super::ConnTaskError>> + Unpin,
    mut phase: super::TransitionWatcher<P, impl Fn(&P) -> bool>,
    me: &player::Handle,
    viruses: HashMap<util::Position, util::Colour>,
    tick_diration: std::time::Duration,
    mut rng: impl rand::Rng,
) -> Result<(), super::ConnTaskError> {
    use std::convert::TryInto;

    use futures::stream::StreamExt;

    use super::ConnTaskError;

    let mut scores = control.scores;
    let events = control.events;
    let capsules = control
        .capsules
        .get(&me.tag())
        .ok_or_else(|| ConnTaskError::other(error::NoneError))?
        .clone();

    // Set up display
    let mut area = display.area().await?.pad_top(1);
    let mut left = area.split_left(super::COLUMN_SPLIT);

    let field = left.place_top(display::PlayField::default()).await?;
    left = left.pad_top(1);
    let indicator = left.place_center(
        display::DynamicText::new((super::COLUMN_SPLIT - 2).try_into().unwrap(), 4u16.try_into().unwrap())
    ).await?;

    let max_scores = area.rows().saturating_sub(2);
    let mut score_board = area.place_center(display::ScoreBoard::new(max_scores)).await?;
    let highlight = {
        let tag = me.tag();
        move |t: &player::Tag| *t == tag
    };
    {
        let scores = scores.borrow().clone();
        score_board.update(&mut display.handle().await?, scores.iter(), &highlight).await?
    }


    let next_colours = rng.gen();
    let mut virus_sym = Default::default();
    field.place_viruses(&mut display.handle().await?, viruses.clone().into_iter(), virus_sym).await?;
    field.place_next_elements(&mut display.handle().await?, &next_colours).await?;
    let mut actor = Actor::new(events, capsules, me.tag(), viruses, next_colours);

    // Let the player grasp the field for a bit before the game starts
    time::sleep(GRACE_PERIOD).await;

    // Kick off the actual game
    let mut tick_timer = Timer::new(tick_diration);
    let mut virs_timer = time::interval(time::Duration::from_secs(1));
    while !actor.is_defeated() && actor.virus_count() > 0{
        use field::Movement as M;

        tokio::select! {
            res = input.next() => match res {
                Some(Ok('p')) | Some(Ok('P')) | Some(Ok('\x1b')) if !tick_timer.is_paused() => {
                    tick_timer.pause();
                    indicator.update_single(&mut display.handle().await?, "Game paused").await?
                },
                Some(Ok('s')) | Some(Ok('S')) if !tick_timer.is_paused() =>
                    actor.r#move(&mut display.handle().await?, &field, M::Left).await?,
                Some(Ok('d')) | Some(Ok('D')) if !tick_timer.is_paused() =>
                    actor.r#move(&mut display.handle().await?, &field, M::Right).await?,
                Some(Ok('k')) | Some(Ok('K')) if !tick_timer.is_paused() =>
                    actor.r#move(&mut display.handle().await?, &field, M::RotateCCW).await?,
                Some(Ok('l')) | Some(Ok('L')) if !tick_timer.is_paused() =>
                    actor.r#move(&mut display.handle().await?, &field, M::RotateCW).await?,
                Some(Ok(' ')) if !tick_timer.is_paused() => if actor.is_controlled() {
                    actor.tick(&mut display.handle().await?, &field, &mut rng).await?
                },
                Some(Ok(c)) => if tick_timer.is_paused() && !c.is_ascii_control() {
                    tick_timer.resume();
                    indicator.clear(&mut display.handle().await?).await?
                },
                Some(Err(e)) if !e.is_would_block() => return Err(e.into()),
                None => return Err(ConnTaskError::Terminated),
                _ => (),
            },
            _ = tick_timer.tick() => actor.tick(&mut display.handle().await?, &field, &mut rng).await?,
            _ = virs_timer.tick() => {
                virus_sym = virus_sym.flipped();
                field.place_viruses(
                    &mut display.handle().await?,
                    actor.remaining_viruses(),
                    virus_sym,
                ).await?
            },
            _ = scores.changed() => {
                let scores = scores.borrow().clone();
                score_board.update(&mut display.handle().await?, scores.iter(), &highlight) .await?
            },
            t = phase.transition() => {
                t?;
                break
            },
        }
    }

    if actor.is_defeated() {
        let msg = [
            "Game over!",
            "Please wait for the others.",
        ];
        indicator.update(&mut display.handle().await?, msg.iter()).await?
    } else if actor.virus_count() == 0 {
        indicator.update_single(&mut display.handle().await?, "You won!").await?
    }

    // Make sure the player isn't thrown into the next waiting phase directly
    time::sleep(GRACE_PERIOD).await;

    // Let the defeated player do nothing until the round ended
    while !phase.transitioned() {
        tokio::select! {
            res = input.next() => match res {
                Some(Err(e)) if !e.is_would_block() => return Err(e.into()),
                None => return Err(ConnTaskError::Terminated),
                _ => (),
            },
            _ = virs_timer.tick() => {
                virus_sym = virus_sym.flipped();
                field.place_viruses(
                    &mut display.handle().await?,
                    actor.remaining_viruses(),
                    virus_sym,
                ).await?
            },
            _ = scores.changed() => {
                let scores = scores.borrow().clone();
                score_board.update(&mut display.handle().await?, scores.iter(), &highlight) .await?
            },
            t = phase.transition() => {
                t?;
                break
            },
        }
    }

    Ok(())
}


/// Round control function
///
/// This function implements the central control logic for the round phase.
///
pub async fn control(
    ports: ControlPorts,
    roster: Arc<RwLock<player::Roster>>,
    rng: &mut impl rand::Rng,
) -> Result<(), error::WrappedErr> {
    use display::ScoreBoardEntry as _;
    use error::TryExt;
    use error::WrappedErr as E;

    let scores_sender = ports.scores;
    let mut events = ports.events;
    let mut active = ports.capsules;

    let mut scores: Vec<ScoreBoardEntry> = roster.read().await.clone().into_iter().map(Into::into).collect();

    while !active.is_empty() {

        scores.sort_by_key(|p| p.round_score());
        scores_sender.send(scores.clone()).or_warn("Could not send updates");

        let (player, event) = events
            .recv()
            .await
            .ok_or_else(|| E::new("could not receive events", error::NoneError))?;
        match event {
            Event::Capsules(elements) => {
                use std::convert::TryInto;

                let max = scores.first().ok_or_else(|| E::new("no players", error::NoneError))?.round_score();
                let targets: Vec<_> = scores
                    .iter()
                    .take_while(|p| p.round_score() >= max)
                    .filter_map(|p| active.get(&p.tag()))
                    .collect();
                let with_colidx = |e: &[_]| e
                    .iter()
                    .cloned()
                    .map(|e| (
                        (rng.next_u32() as usize % util::FIELD_WIDTH as usize)
                            .try_into()
                            .expect("Could not convert to field index"),
                        e
                    )).collect();
                let sends = elements
                    .chunks((elements.len() / targets.len()).clamp(1, MAX_CAPSULE_RECEIVE))
                    .map(with_colidx)
                    .zip(targets);
                for (elements, target) in sends {
                    target.lock().await.push_back(elements)
                }
            },
            Event::Score(score) => {
                if let Some(entry) = scores.iter_mut().find(|e| *e.tag() == player) {
                    entry.set_score(score);
                } else {
                    log::warn!("Could not find entry for player tag");
                }
                if score == 0 {
                    active
                        .remove(&player)
                        .ok_or_else(|| E::new("winning player not active", error::NoneError))?;
                    break;
                }
            },
            Event::Defeat => { active.remove(&player).or_warn("Defeated player not active"); },
        }
    }

    // Preserve the round scores by adding them to the overall scores
    scores.into_iter().for_each(|e| {e.tag().add_score(e.round_score()); });

    Ok(())
}


/// Game logic encapsulation
///
/// This data type provides the core logic for a round, exposed as functions.
/// These include functions for performing both controlled moves and ticks.
///
struct Actor {
    event_sender: mpsc::Sender<(player::Tag, Event)>,
    capsule_receiver: CapsulesQueue,
    player_tag: player::Tag,
    moving: field::MovingField,
    r#static: field::StaticField,
    viruses: HashMap<util::Position, util::Colour>,
    active: ActiveElements,
    next_colours: [util::Colour; 2],
}

impl Actor {
    /// Create a new actor
    ///
    pub fn new(
        event_sender: mpsc::Sender<(player::Tag, Event)>,
        capsule_receiver: CapsulesQueue,
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
    /// The given `field` is updated via the given `display_handle` accordingly.
    ///
    pub async fn tick(
        &mut self,
        display_handle: &mut display::DrawHandle<'_, impl io::AsyncWrite + Unpin>,
        field: &display::FieldUpdater,
        rng: &mut impl rand::Rng,
    ) -> Result<(), super::ConnTaskError> {
        let lowest = self.moving.row_index_from_moving(self.active.lowest_row());

        let (settled, mut lowest) = field::settle_elements(&mut self.moving, &mut self.r#static, lowest);

        if !settled.is_empty() {
            // A lot of interesting stuff only happens if elements settled
            let eliminated = field::eliminate_elements(&mut self.r#static, &settled);
            lowest = lower_row(
                field::unsettle_elements(&mut self.moving, &mut self.r#static, &eliminated),
                lowest
            );

            if eliminated.positions().fold(false, |c, p| c || self.viruses.remove(&p).is_some()) {
                self.send_event(Event::Score(self.viruses.len() as u32)).await?;
            }
            if eliminated.row_count() > MIN_CAPSULES_SEND {
                let capsules = eliminated.rows_of_four().map(|(c, _)| *c).collect();
                self.send_event(Event::Capsules(capsules)).await?;
            }
            if field::defeated(&self.r#static) {
                self.send_event(Event::Defeat).await?;
            }

            field.update(display_handle, eliminated.positions().map(|p| (p, None))).await?;

            if let Some(lowest) = lowest {
                self.active = self.moving.moving_row_index(lowest).into();
            }
        }

        if lowest.is_some() {
            // We still have moving elements.
            field.update(display_handle, self.moving.tick()).await.map_err(Into::into)
        } else {
            // There are no moving element left. We need to respawn something.
            use util::RowIndex;
            if let Some(capsules) = self.capsule_receiver.lock().await.pop_front() {
                self.active = self.moving.moving_row_index(RowIndex::TOP_ROW).into();
                return field
                    .update(display_handle, self.moving.spawn_single_capsules(capsules))
                    .await
                    .map_err(Into::into)
            }

            // We didn't receive any unbound capsules, spawn a controlled capsule.
            let (capsule, updates) = field::ControlledCapsule::spawn_capsule(
                &mut self.moving,
                &self.next_colours
            );
            self.next_colours = rng.gen();

            self.active = capsule.into();
            field.update(display_handle, updates.iter().cloned()).await?;
            field.place_next_elements(display_handle, &self.next_colours).await.map_err(Into::into)
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
        field::defeated(&self.r#static)
    }

    /// Retrieve the number of remaining viruses
    ///
    pub fn virus_count(&self) -> usize {
        self.viruses.len()
    }

    /// Retrieve the remaining viruses
    ///
    pub fn remaining_viruses(&self) -> impl Iterator<Item = (util::Position, util::Colour)> {
        self.viruses.clone().into_iter()
    }

    /// Send the given event
    ///
    async fn send_event(&self, event: Event) -> Result<(), super::ConnTaskError> {
        self.event_sender.send((self.player_tag.clone(), event)).await.map_err(super::ConnTaskError::other)
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


/// The maximum number of capsules sent to a single player
///
const MAX_CAPSULE_RECEIVE: usize = 4;


/// Grace period before the first tick
///
const GRACE_PERIOD: time::Duration = time::Duration::from_secs(2);

