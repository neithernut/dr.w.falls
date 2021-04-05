//! Implementation of the waiting phase


use std::sync::Arc;

use tokio::io;
use tokio::sync::{watch, mpsc};

use crate::display;


/// Waiting phase function
///
/// This function provides the interface for the waiting phase.
///
async fn waiting<E>(
    input: &mut super::ASCIIStream<'_>,
    display: &mut super::Display<'_>,
    updates: &mut watch::Receiver<GameUpdate<E>>,
    ready_channel: mpsc::Sender<super::PlayerTag>,
    me: &super::PlayerHandle,
) -> io::Result<()> {
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
            _ = updates.changed() => match &*updates.borrow() {
                GameUpdate::Update((scores, count)) => {
                    num.update(display, (*count).into()).await?;
                    scoreboard.update(display, scores.clone(), &me.tag()).await?;
                },
                GameUpdate::PhaseEnd(_) => break,
            },
        }
    }

    Ok(())
}


/// Local type for game updates
///
type GameUpdate<E> = super::GameUpdate<(Arc<Vec<ScoreBoardEntry>>, u8), E>;


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

