//! Implementation of the waiting phase


use std::sync::{Arc, RwLock};

use tokio::io;
use tokio::sync::{watch, mpsc};
use tokio::time;

use crate::display;
use crate::Roster;


/// Waiting phase function
///
/// This function provides the interface for the waiting phase.
///
pub async fn waiting<E: Clone>(
    input: &mut super::ASCIIStream<impl io::AsyncRead + Unpin>,
    display: &mut display::Display<impl io::AsyncWrite + Unpin>,
    mut updates: watch::Receiver<GameUpdate<E>>,
    ready_channel: mpsc::Sender<super::PlayerTag>,
    me: &super::PlayerHandle,
) -> io::Result<super::PhaseEnd<E>> {
    use futures::stream::StreamExt;

    // Set up display
    let (left, right) = super::columns(display);
    let (mut text1, left)   = left.top_padded(1).top_in("Round starts in");
    let (mut num,   left)   = left.top_padded(1).top_in(display::NumFieldFactory::default());
    let (mut text2, left)   = left.top_padded(1).top_in("or when everybody's ready.");
    let (mut text3, left)   = left.top_padded(1).top_in(super::INSTRUCTIONS);
    let (mut ready, _)      = left.top_padded(1).top_in("Press any key when ready.");
    let mut scoreboard      = right.topleft_in(display::ScoreBoardFactory::<ScoreBoardEntry>::default());

    text1.draw(display).await?;
    text2.draw(display).await?;
    text3.draw(display).await?;
    ready.draw(display).await?;
    scoreboard.render_heading(display, "Ready?").await?;

    loop {
        tokio::select! {
            res = input.next() => match res {
                Some(Ok('\x03')) | Some(Ok('\x04')) => return Err(io::ErrorKind::UnexpectedEof.into()),
                Some(Ok(c)) => if !c.is_ascii_control() {
                    ready_channel.send(me.tag()).await.map_err(|_| io::Error::from(io::ErrorKind::Other))?;
                    ready.erase(display).await?;
                },
                Some(Err(e)) if e.kind() == io::ErrorKind::WouldBlock => (),
                Some(Err(e)) => return Err(e),
                None => (),
            },
            _ = updates.changed() => {
                let (scores, count) = match &*updates.borrow() {
                    GameUpdate::Update(u) => u.clone(),
                    GameUpdate::PhaseEnd(e) => break Ok(e.clone()),
                };
                num.update(display, count.into()).await?;
                scoreboard.update(display, scores, &me.tag()).await?;
            },
        }
    }
}


/// Waiting phase control function
///
/// This function implements the central control logic for the waiting phase.
///
pub async fn control_waiting<E: Clone + std::fmt::Debug>(
    update_sender: &mut watch::Sender<GameUpdate<E>>,
    mut ready: mpsc::Receiver<super::PlayerTag>,
    mut control: watch::Receiver<super::GameControl>,
    roster: Arc<RwLock<Roster>>,
) -> io::Result<()> {
    use crate::util::TryExt;

    let mut value = WAITING_TIME;
    let mut timer = time::interval(std::time::Duration::from_secs(1));

    let mut scores: Vec<_> = roster
        .read()
        .map_err(|_| io::ErrorKind::Other)?
        .iter()
        .map(|p| ScoreBoardEntry::new(p.name.clone(), p.tag.clone(), p.score))
        .collect();
    scores.sort_by_key(|p| p.score);

    while value > 0 && scores.iter().any(|e| !e.ready()) {
        use crate::display::ScoreBoardEntry;

        update_sender
            .send(GameUpdate::Update((Arc::new(scores.clone()), value)))
            .or_warn("Could not send updates");

        tokio::select! {
            _ = timer.tick() => value = value.saturating_sub(1),
            tag = ready.recv() => {
                let tag = tag.or_warn("Could not receive readiness").ok_or(io::ErrorKind::Other)?;
                if let Some(entry) = scores.iter_mut().find(|e| e.tag() == tag) {
                    entry.set_ready()
                } else {
                    log::warn!("Could not find entry for player tag");
                }
            },
            _ = control.changed() => match *control.borrow() {
                super::GameControl::EndOfGame => break,
                _ => (),
            },
        }
    }

    Ok(())
}


/// Local type for game updates
///
pub type GameUpdate<E> = super::GameUpdate<(Arc<Vec<ScoreBoardEntry>>, u8), E>;


/// Score board entry for the waiting phase
///
#[derive(Clone, Debug, PartialEq)]
pub struct ScoreBoardEntry {
    name: String,
    score: u32,
    ready: bool,
    tag: super::PlayerTag,
}

impl ScoreBoardEntry {
    /// Create a new score board entry
    ///
    fn new(name: String, tag: super::PlayerTag, score: u32) -> Self {
        Self {name, score, ready: false, tag}
    }

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


/// Waiting time in seconds
///
/// This constant is used as the initial value for the counter used for counting
/// down seconds.
///
const WAITING_TIME: u8 = 60;

