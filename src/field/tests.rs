//! Field tests

use quickcheck::{Arbitrary, Gen};

use crate::util;

use super::*;


#[quickcheck]
fn moving_capsule(
    column: util::ColumnIndex,
    target_row: util::RowIndex,
    colour: util::Colour,
    pre_ticks: u8,
) -> bool {
    use util::Step;

    let mut field = moving_field::MovingField::default();
    (0..pre_ticks).for_each(|_| field.tick().fold((), |_, _| ()));
    field.spawn_single_capsules(std::iter::once((column, colour))).fold((), |_, _| ());

    let ticks = Step::steps_between(&util::RowIndex::TOP_ROW, &target_row).expect("Invalid target row");
    (0..ticks).for_each(|_| field.tick().fold((), |_, _| ()));

    field[(target_row, column)] == Some(items::CapsuleElement::new_single(colour))
}


#[quickcheck]
fn moving_index(column: util::ColumnIndex, colour: util::Colour, ticks: u8, pre_ticks: u8) -> bool {
    let mut field = moving_field::MovingField::default();
    (0..pre_ticks).for_each(|_| field.tick().fold((), |_, _| ()));
    field.spawn_single_capsules(std::iter::once((column, colour))).fold((), |_, _| ());

    let row = field.moving_row_index(util::RowIndex::TOP_ROW);
    (0..ticks).for_each(|_| field.tick().fold((), |_, _| ()));

    let row = field.row_index_from_moving(row);
    field[(row, column)] == Some(items::CapsuleElement::new_single(colour))
}


#[quickcheck]
fn row_of_four_len(row: items::RowOfFour) -> bool {
    row.len() == row.count()
}


#[quickcheck]
fn find_row_of_four(original: items::RowOfFour, mut field: TwoColouredField, pick: u8) -> bool {
    original.for_each(|p| field[p] = Some(field.omitted));
    let hint = original.cycle().nth(pick as usize).expect("Could not pick hint");

    let expected = if original.len() >= 4 {
        Some((field.omitted, original))
    } else {
        None
    };
    items::row_of_four(&field, hint) == expected
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

