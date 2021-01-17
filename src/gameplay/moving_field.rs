//! Definition of the moving field and associated types

use crate::util;

use super::items::CapsuleElement;
use super::row::Row;


/// Field of unsettled/moving elements
///
#[derive(Default)]
pub struct MovingField {
    data: [Row<Option<CapsuleElement>>; util::FIELD_HEIGHT as usize],
    offset: usize,
}

impl MovingField {
    /// Move all elements down one position
    ///
    pub fn tick(&mut self) {
        self.offset = self.offset.checked_sub(1).unwrap_or(self.data.len() - 1)
    }

    /// Transform a `RowIndex` to the row's index in the internal array
    ///
    fn transform(&self, row: util::RowIndex) -> usize {
        (usize::from(row) + self.offset) % self.data.len()
    }
}

impl std::ops::IndexMut<util::Position> for MovingField {
    fn index_mut(&mut self, index: util::Position) -> &mut Self::Output {
        &mut self.data[self.transform(index.0)][index.1]
    }
}

impl std::ops::Index<util::Position> for MovingField {
    type Output = Option<CapsuleElement>;

    fn index(&self, index: util::Position) -> &Self::Output {
        &self.data[self.transform(index.0)][index.1]
    }
}

