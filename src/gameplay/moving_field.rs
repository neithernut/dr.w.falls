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

    /// Crate a MovingRowIndex for a given mapped row
    ///
    pub fn moving_row_index(&self, row: util::RowIndex) -> MovingRowIndex {
        MovingRowIndex {row: self.transform(row)}
    }

    /// Convert a MovingRowIndex back to a RowIndex
    ///
    pub fn row_index_from_moving(&self, index: MovingRowIndex) -> util::RowIndex {
        use std::convert::TryInto;

        ((index.row + self.data.len() - self.offset) % self.data.len())
            .try_into()
            .expect("Failed to tranfform MovingRowIndex to plain util::RowIndex")
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


/// Index for a moving row
///
/// Indexes of this kind refer to one row in the field of moving rows. The row
/// will be tracked across invocations of `MovingField::tick`, i.e. as it moves
/// down the field.
///
#[derive(Copy, Clone, PartialEq, Eq)]
pub struct MovingRowIndex {
    row: usize,
}

