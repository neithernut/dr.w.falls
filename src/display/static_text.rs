//! Static text entity

use super::area;
use super::commands::{DrawCommand as DC};


/// Representation of static text to display
///
/// An instance of this type itself is useless unless it is placed in an `Area`.
///
pub struct StaticText<'a, I: IntoIterator<Item = &'a str> + Clone> {
    lines: I,
}

impl<'a, I: IntoIterator<Item = &'a str> + Clone> StaticText<'a, I> {
    /// Retrieve an iterator over the lines to display
    ///
    pub fn lines(&self) -> I::IntoIter {
        self.lines.clone().into_iter()
    }
}

impl<'a> From<&'a str> for StaticText<'a, std::str::Lines<'a>> {
    fn from(string: &'a str) -> Self {
        string.lines().into()
    }
}

impl<'a, I: IntoIterator<Item = &'a str> + Clone> From<I> for StaticText<'a, I> {
    fn from(lines: I) -> Self {
        Self {lines}
    }
}

impl<'a, I: IntoIterator<Item = &'a str> + Clone> area::Entity for StaticText<'a, I> {
    type PlacedEntity = ();

    fn rows(&self) -> u16 {
        self.lines().count() as u16
    }

    fn cols(&self) -> u16 {
        self.lines().map(|l| l.len()).max().unwrap_or_default() as u16
    }

    fn init(&self, (base_row, base_col): (u16, u16)) -> area::PlacedInit {
        use std::iter::once;

        self.lines()
            .enumerate()
            .flat_map(|(n, l)| once(DC::SetPos(base_row + n as u16, base_col)).chain(once((*l).into())))
            .collect::<Vec<_>>()
            .into()
    }

    fn place(self, _: (u16, u16)) -> Self::PlacedEntity {}
}

