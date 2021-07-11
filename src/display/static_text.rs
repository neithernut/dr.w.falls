//! Static text entity

use super::area;
use super::commands::{DrawCommand as DC};


/// Representation of static text to display
///
/// An instance of this type itself is useless unless it is placed in an `Area`.
///
pub struct StaticText {
    lines: Vec<&'static str>,
}

impl StaticText {
    /// Retrieve an iterator over the lines to display
    ///
    pub fn lines(&self) -> impl Iterator<Item = &'static str> {
        self.lines.clone().into_iter()
    }
}

impl From<&'static str> for StaticText {
    fn from(string: &'static str) -> Self {
        Self {lines: string.lines().collect()}
    }
}

impl From<&[&'static str]> for StaticText {
    fn from(lines: &[&'static str]) -> Self {
        Self {lines: lines.to_vec()}
    }
}

impl area::Entity for StaticText {
    type PlacedEntity = ();

    fn rows(&self) -> u16 {
        self.lines().count() as u16
    }

    fn cols(&self) -> u16 {
        self.lines().map(|l| l.len()).max().unwrap_or_default() as u16
    }

    fn init(&self, (base_row, base_col): (u16, u16)) -> area::PlacedInit {
        use std::iter::once;

        self.lines
            .iter()
            .enumerate()
            .flat_map(|(n, l)| once(DC::SetPos(base_row + n as u16, base_col)).chain(once((*l).into())))
            .collect::<Vec<_>>()
            .into()
    }

    fn place(self, _: (u16, u16)) -> Self::PlacedEntity {}
}

