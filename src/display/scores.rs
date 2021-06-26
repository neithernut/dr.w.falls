//! Score board entity

use std::hash::Hash;

use tokio::io::AsyncWrite;

use crate::player;
use super::area;
use super::commands::{self, DrawCommand as DC, DrawHandle};


/// Representation of a score board
///
/// An instance of this type itself is useless unless it is placed in an `Area`.
///
pub struct ScoreBoard {
    max_rows: u16,
    show_scores: bool,
}

impl ScoreBoard {
    /// Create a new score board
    ///
    /// By default, the score board will include the scores.
    ///
    pub fn new(max_rows: u16) -> Self {
        Self {max_rows, show_scores: true}
    }

    /// Change whether scores are shown
    ///
    pub fn show_scores(self, show_scores: bool) -> Self {
        Self {show_scores, ..self}
    }

    const ENUM_COL: u16 = 0;
    const NAME_COL: u16 = 4;
    const TOTAL_SCORE_COL: u16 = 24;
    const ROUND_SCORE_COL: u16 = 32;
    const WIDTH: u16 = 40;
}

impl area::Entity for ScoreBoard {
    type PlacedEntity = BoardUpdater;

    fn rows(&self) -> u16 {
        self.max_rows + 1
    }

    fn cols(&self) -> u16 {
        Self::WIDTH
    }

    fn init(&self, (base_row, base_col): (u16, u16)) -> area::PlacedInit {
        let mut res = vec![DC::SetPos(base_row, base_col + Self::NAME_COL), "Player".into()];

        if self.show_scores {
            res.extend([
                DC::SetPos(base_row, base_col + Self::TOTAL_SCORE_COL),
                "Total".into(),
                DC::SetPos(base_row, base_col + Self::ROUND_SCORE_COL),
                "Round".into(),
            ].iter().cloned())
        }

        res.into()
    }

    fn place(self, (base_row, base_col): (u16, u16)) -> Self::PlacedEntity {
        BoardUpdater {
            row_hashes: vec![Default::default(); self.max_rows as usize].into(),
            base_row,
            base_col,
            show_scores: self.show_scores,
        }
    }
}


/// Handle for updating a score board entity
///
pub struct BoardUpdater {
    row_hashes: Box<[u64]>,
    base_row: u16,
    base_col: u16,
    show_scores: bool,
}

impl BoardUpdater {
    /// Update the score board
    ///
    /// The score board will be updated to reflect the given entries. Any entry
    /// matching the predicate provided by `highlight` will be highlightted
    /// visually.
    ///
    pub async fn update<'e, E: Entry + 'e>(
        &mut self,
        draw_handle: &mut DrawHandle<'_, impl AsyncWrite + Unpin>,
        entries: impl IntoIterator<Item = &'e E>,
        highlight: impl Fn (&player::Tag) -> bool,
    ) -> std::io::Result<()> {
        use std::collections::hash_map::DefaultHasher as Hasher;
        use std::hash::Hasher as _;

        use futures::stream::iter;
        use futures::SinkExt;

        use commands::{Intensity, SGR, SinkProxy};

        const NUM_WIDTH: usize = (ScoreBoard::NAME_COL - ScoreBoard::ENUM_COL) as usize;
        const NAME_WIDTH: usize = (ScoreBoard::TOTAL_SCORE_COL - ScoreBoard::NAME_COL) as usize;
        const TOTAL_SCORE_WIDTH: usize = (ScoreBoard::ROUND_SCORE_COL - ScoreBoard::TOTAL_SCORE_COL) as usize;
        const ROUND_SCORE_WIDTH: usize = (ScoreBoard::WIDTH - ScoreBoard::ROUND_SCORE_COL) as usize;

        let row_pos = {
            let base_row = self.base_row;
            let base_col = self.base_col;

            move |row| (base_row + row as u16, base_col).into()
        };
        let show_scores = self.show_scores;

        // We'll ultimately iterate over all rows in the table and each of those
        // will have a hash assoziated with it which we might need to modify.
        let mut hashes = self
            .row_hashes
            .iter_mut()
            .enumerate()
            .map(|(row, hash)| (row + 1, hash));

        // First, we update the entries which do not match the hash. Regardless
        // of what entries will end being updated, the `zip` will cause as many
        // enumerated hashes to be consumed as there are entries. Thus, `hashes`
        // will be advanced to the position where there shouldn't be any more
        // entries.
        let cmds = hashes
            .by_ref()
            .zip(entries.into_iter())
            .filter_map(|((row, old_hash), entry)| {
                // First, we need to prepare the details and decide whether or
                // not we need to draw an update for the entry.
                let details = entry.details();

                let mut hasher = Hasher::new();
                details.hash(&mut hasher);
                let new_hash = hasher.finish();

                if new_hash != *old_hash {
                    *old_hash = new_hash;
                    Some((row, details, highlight(entry.tag())))
                } else {
                    None
                }
            })
            .flat_map(|(row, entry, highlight)| {
                // We then translate the details for each entry needing an
                // update into a sequence of draw commands.
                let intensity = if highlight {
                    Some(Intensity::Bold)
                } else if entry.active && entry.connected {
                    None
                }else {
                    Some(Intensity::Faint)
                };

                let mut res = vec![
                    row_pos(row),
                    intensity.into(),
                    SGR::Strike(!entry.connected).into(),
                    format!("{0:1$} {2:3$}", row, NUM_WIDTH - 1, entry.name, NAME_WIDTH).into(),
                ];
                if show_scores {
                    res.push(format!(
                        "{0:>1$}{2:>3$}",
                        entry.total_score,
                        TOTAL_SCORE_WIDTH,
                        entry.round_score,
                        ROUND_SCORE_WIDTH,
                    ).into())
                }
                res
            })
            .map(Ok);
        draw_handle.as_sink().send_all(&mut iter(cmds)).await?;

        // We might have fewer entries than before. We thus need to clear all of
        // the remaining rows which were previously filled.
        let cmds = hashes
            .filter(|(_, hash)| **hash != Default::default())
            .flat_map(|(row, hash)| {
                *hash = Default::default();
                std::iter::once(row_pos(row)).chain((0..ScoreBoard::WIDTH).map(|_| " ".into()))
            })
            .map(Ok);
        draw_handle.as_sink().send_all(&mut iter(cmds)).await
    }
}


/// Scoreboard entry
///
pub trait Entry {
    /// Player tag
    ///
    fn tag(&self) -> &player::Tag;

    /// The player's round score
    ///
    fn round_score(&self) -> u32 {
        0
    }

    /// Indication of the player's activity status
    ///
    fn active(&self) -> bool {
        true
    }

    /// Generate a collection of all the details of this entry's current state
    ///
    fn details(&self) -> EntryDetails {
        EntryDetails {
            name: self.tag().name(),
            total_score: self.tag().score(),
            round_score: self.round_score(),
            connected: self.tag().is_connected(),
            active: self.active(),
        }
    }
}

impl Entry for player::Tag {
    fn tag(&self) -> &player::Tag {
        &self
    }
}


/// Representation of an entry's state at a given point in time
///
#[derive(Copy, Clone, Debug, Hash, Eq, PartialEq)]
pub struct EntryDetails<'a> {
    pub name: &'a str,
    pub total_score: u32,
    pub round_score: u32,
    pub connected: bool,
    pub active: bool,
}

