//! Implementation of the waiting phase

use std::sync::Arc;

use tokio::io;
use tokio::sync::{RwLock, watch, mpsc};
use tokio::time;

use crate::display;
use crate::player;


/// Waiting phase function
///
/// This function implements the connection task part of the game logic for the
/// waiting phase.
///
pub async fn serve<P>(
    control: Ports,
    display: &mut display::Display<impl io::AsyncWrite + Send + Unpin>,
    mut input: impl futures::stream::Stream<Item = Result<char, super::ConnTaskError>> + Unpin,
    mut phase: super::TransitionWatcher<P, impl Fn(&P) -> bool>,
    me: &player::Handle,
) -> Result<(), super::ConnTaskError> {
    use std::convert::TryInto;

    use futures::stream::StreamExt;

    use super::ConnTaskError;

    let mut scores = control.scores;
    let mut countdown = control.countdown;
    let ready = control.ready;

    // Set up the display
    let mut area = display.area().await?.pad_top(1);
    let mut left = area.split_left(super::COLUMN_SPLIT);
    let mut ct = left.split_top(super::INSTRUCTION_SPLIT);

    ct.place_top(display::StaticText::from("Round starts in:")).await?;
    ct = ct.pad_top(1);
    let num_display = ct.place_top(display::DynamicText::new_line(4u16.try_into().unwrap())).await?;
    ct = ct.pad_top(1);
    ct.place_top(display::StaticText::from("or when everybody's ready.")).await?;
    ct = ct.pad_top(1);
    let inst = ct.place_center(
        display::DynamicText::new_line((super::COLUMN_SPLIT - 2).try_into().unwrap())
    ).await?;

    left.place_center(display::StaticText::from(&super::INSTRUCTIONS as &[_])).await?;

    let max_scores = area.rows().saturating_sub(2);
    let mut score_board = area.place_center(display::ScoreBoard::new(max_scores).show_scores(false)).await?;
    let highlight = {
        let tag = me.tag();
        move |t: &player::Tag| *t == tag
    };
    {
        let scores = scores.borrow().clone();
        score_board.update(&mut display.handle().await?, scores.iter(), &highlight).await?
    }


    {
        let countdown = *countdown.borrow();
        num_display.update_single(&mut display.handle().await?, countdown).await?
    }
    inst.update_single(&mut display.handle().await?, "Press any key when ready.").await?;

    // Actual waiting display logic
    while !phase.transitioned() {
        tokio::select! {
            res = input.next() => match res {
                Some(Ok(_)) => {
                    ready.send(me.tag()).await.map_err(ConnTaskError::other)?;
                    inst.update_single(&mut display.handle().await?, "Wait for the round to start.").await?;
                },
                Some(Err(e)) if !e.is_would_block() => return Err(e.into()),
                None => return Err(ConnTaskError::Terminated),
                _ => (),
            },
            _ = scores.changed() => {
                let scores = scores.borrow().clone();
                score_board.update(&mut display.handle().await?, scores.iter(), &highlight).await?
            },
            _ = countdown.changed() => {
                let countdown = *countdown.borrow();
                num_display.update_single(&mut display.handle().await?, countdown).await?
            },
            t = phase.transition() => return t,
        }
    }

    Ok(())
}


/// Waiting phase control function
///
/// This function implements the central control logic for the waiting phase.
///
pub async fn control(
    ports: ControlPorts,
    mut game_control: watch::Receiver<super::GameControl>,
    roster: Arc<RwLock<player::Roster>>,
    disconnects: &mut mpsc::UnboundedReceiver<player::Tag>,
) -> () {
    use crate::error::TryExt;
    use display::ScoreBoardEntry as _;

    use ScoreBoardEntry as Entry;

    let scores = ports.scores;
    let countdown = ports.countdown;
    let mut ready = ports.ready;

    let mut value = WAITING_TIME;
    let mut timer = time::interval(std::time::Duration::from_secs(1));

    let mut roster: Vec<Entry> = roster.read().await.clone().into_iter().map(Into::into).collect();
    roster.sort_by_key(|p| p.tag().score());

    while value > 0 && roster.iter().any(Entry::is_blocking) && !game_control.borrow().is_end_of_game() {
        scores.send(roster.clone().into()).or_warn("Could not send scores");

        tokio::select! {
            _ = timer.tick() => {
                value = value.saturating_sub(1);
                countdown.send(value).or_warn("Could not send countdown value");
            },
            tag = ready.recv() => if let Some(tag) = tag {
                if let Some(entry) = roster.iter_mut().find(|e| *e.tag() == tag) {
                    entry.set_ready()
                } else {
                    log::warn!("Could not find entry for player tag");
                }
            } else {
                log::warn!("Could not receive readiness");
                break;
            },
            _ = disconnects.recv() => (),
            _ = game_control.changed() => (),
        }
    }
}


/// Create ports for communication between connection and control task
///
/// This function returns a pair of ports specific to the waiting phase, one for
/// the connection task and one for the control task.
///
pub fn ports(scores: impl IntoIterator<Item = player::Tag>) -> (Ports, ControlPorts) {
    let scores: Arc<_> = scores.into_iter().map(Into::into).collect();
    let player_num = scores.len();

    let (score_sender, score_receiver) = watch::channel(scores);
    let (countdown_sender, countdown_receiver) = watch::channel(Default::default());
    let (readiness_sender, readiness_receiver) = mpsc::channel(player_num);

    let ports = Ports {scores: score_receiver, countdown: countdown_receiver, ready: readiness_sender};
    let control = ControlPorts {scores: score_sender, countdown: countdown_sender, ready: readiness_receiver};

    (ports, control)
}


/// Connection task side of communication ports for the lobby phase
///
#[derive(Clone, Debug)]
pub struct Ports {
    scores: watch::Receiver<Arc<[ScoreBoardEntry]>>,
    countdown: watch::Receiver<u8>,
    ready: mpsc::Sender<player::Tag>,
}


/// Control task side of communication ports for the lobby phase
///
#[derive(Debug)]
pub struct ControlPorts {
    scores: watch::Sender<Arc<[ScoreBoardEntry]>>,
    countdown: watch::Sender<u8>,
    ready: mpsc::Receiver<player::Tag>,
}


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

    /// Check whether this player blocks overall readiness
    ///
    fn is_blocking(&self) -> bool {
        use display::ScoreBoardEntry;
        self.tag().is_connected() && !self.ready()
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


/// Waiting time in seconds
///
/// This constant is used as the initial value for the counter used for counting
/// down seconds.
///
const WAITING_TIME: u8 = 60;

