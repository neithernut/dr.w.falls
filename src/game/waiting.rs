//! Implementation of the waiting phase

use tokio::io;
use tokio::sync::{watch, mpsc};

use crate::display;
use crate::player;


/// Waiting phase function
///
/// This function implements the connection task part of the game logic for the
/// waiting phase.
///
async fn serve<P>(
    control: Ports,
    display: &mut display::Display<impl io::AsyncWrite + Unpin>,
    mut input: impl futures::stream::Stream<Item = Result<char, io::Error>> + Unpin,
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

    left.place_center(display::StaticText::from(super::INSTRUCTIONS.iter().cloned())).await?;

    let max_scores = area.rows().saturating_sub(2);
    let mut score_board = area.place_center(display::ScoreBoard::new(max_scores).show_scores(false)).await?;
    score_board.update(&mut display.handle().await?, scores.borrow().iter(), |_| false).await?;


    num_display.update_single(&mut display.handle().await?, *countdown.borrow()).await?;
    inst.update_single(&mut display.handle().await?, "Press any key when ready.").await?;

    // Actual waiting display logic
    loop {
        tokio::select! {
            res = input.next() => match res {
                Some(Ok(_)) => {
                    ready.send(me.tag()).await.map_err(ConnTaskError::other)?;
                    inst.update_single(&mut display.handle().await?, "Wait to the round to start.").await?;
                },
                Some(Err(e)) if e.kind() != io::ErrorKind::WouldBlock => return Err(e.into()),
                None => return Err(ConnTaskError::Terminated),
                _ => (),
            },
            _ = scores.changed() => score_board
                .update(&mut display.handle().await?, scores.borrow().iter(), |_| false)
                .await?,
            _ = countdown.changed() => num_display
                .update_single(&mut display.handle().await?, *countdown.borrow())
                .await?,
            t = phase.transition() => break t,
        }
    }
}


/// Create ports for communication between connection and control task
///
/// This function returns a pair of ports specific to the waiting phase, one for
/// the connection task and one for the control task.
///
pub fn ports() -> (Ports, ControlPorts) {
    let (score_sender, score_receiver) = watch::channel(Default::default());
    let (countdown_sender, countdown_receiver) = watch::channel(Default::default());
    let (readiness_sender, readiness_receiver) = mpsc::channel(20); // TODO: replace hard-coded value?

    let ports = Ports {scores: score_receiver, countdown: countdown_receiver, ready: readiness_sender};
    let control = ControlPorts {scores: score_sender, countdown: countdown_sender, ready: readiness_receiver};

    (ports, control)
}


/// Connection task side of communication ports for the lobby phase
///
#[derive(Clone, Debug)]
pub struct Ports {
    scores: watch::Receiver<Vec<ScoreBoardEntry>>,
    countdown: watch::Receiver<u8>,
    ready: mpsc::Sender<player::Tag>,
}


/// Control task side of communication ports for the lobby phase
///
#[derive(Debug)]
pub struct ControlPorts {
    scores: watch::Sender<Vec<ScoreBoardEntry>>,
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

