//! Field tests

use quickcheck::{Arbitrary, Gen};

use crate::util;

use super::*;


#[quickcheck]
fn row_of_four_len(row: items::RowOfFour) -> bool {
    row.len() == row.count()
}


/// Field initialized with only two colours
///
#[derive(Clone, Debug)]
struct TwoColouredField {
    data: [row::Row<Option<util::Colour>>; util::FIELD_HEIGHT as usize],
    pub omitted: util::Colour,
}

impl std::ops::IndexMut<util::Position> for TwoColouredField {
    fn index_mut(&mut self, (row, col): util::Position) -> &mut Self::Output {
        &mut self.data[usize::from(row)][col]
    }
}

impl std::ops::Index<util::Position> for TwoColouredField {
    type Output = Option<util::Colour>;

    fn index(&self, (row, col): util::Position) -> &Self::Output {
        &self.data[usize::from(row)][col]
    }
}

impl Arbitrary for TwoColouredField {
    fn arbitrary(g: &mut Gen) -> Self {
        let mut data: [_; util::FIELD_HEIGHT as usize] = Default::default();
        let omitted = util::Colour::arbitrary(g);
        let opts = [Some(omitted.rotate(true)), Some(omitted.rotate(false)), None];
        data.fill_with(|| {
            let mut row: row::Row<_> = Default::default();
            util::COLUMNS.for_each(|c| row[c] = *g.choose(&opts).unwrap());
            row
        });
        Self {data, omitted}
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let field = self.clone();
        let res = util::ROWS.flat_map(util::complete_row).filter_map(move |p| {
            let mut field = field.clone();
            std::mem::take(&mut field[p]).map(|_| field)
        });
        Box::new(res)
    }
}

