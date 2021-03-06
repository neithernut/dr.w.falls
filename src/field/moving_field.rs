//! Definition of the moving field and associated types

use crate::util;

use super::items;
use super::row::Row;


/// Field of unsettled/moving elements
///
#[derive(Default)]
pub struct MovingField {
    data: [Row<Option<items::CapsuleElement>>; util::FIELD_HEIGHT as usize],
    offset: usize,
}

impl MovingField {
    /// Move all elements down one position
    ///
    /// The function returns a list of `Update`s which have to be applied in
    /// order.
    ///
    pub fn tick(&mut self) -> impl Iterator<Item = items::Update> + '_ {
        use util::PotentiallyColoured;

        self.offset = self.offset.checked_sub(1).unwrap_or(self.data.len() - 1);

        util::ROWS
            .rev()
            .flat_map(util::complete_row)
            .filter_map(move |pos| if let Some(c) = self[pos].colour() {
                Some((pos, Some(c)))
            } else if (pos + util::Direction::Below).map(|p| self[p].is_some()).unwrap_or(false) {
                Some((pos, None))
            } else {
                None
            })
    }

    /// Spawn single capsules in the current top row
    ///
    /// For each item yielded by `capsules`, this function creates a single,
    /// unbound capsule with the given colour and place it in the top row at the
    /// given column. It returns a list of `Update`s reflecting the changes.
    ///
    pub fn spawn_single_capsules<'a>(
        &'a mut self,
        capsules: impl IntoIterator<Item = (util::ColumnIndex, util::Colour)> + 'a,
    ) -> impl Iterator<Item = items::Update> + 'a {
        let top_row = &mut self.data[self.transform(util::RowIndex::TOP_ROW)];
        capsules
            .into_iter()
            .inspect(move |(i, c)| top_row[*i] = Some(items::CapsuleElement::new_single(*c)))
            .map(|(i, c)| ((util::RowIndex::TOP_ROW, i), Some(c)))
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
    type Output = Option<items::CapsuleElement>;

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

