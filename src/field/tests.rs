//! Field tests

use quickcheck::{Arbitrary, Gen, TestResult};

use crate::util;

use super::*;


#[quickcheck]
fn preparation_vir_count(seed: u64, top_row: util::RowIndex, vir_count: u8) -> TestResult {
    use rand::SeedableRng;

    let area = util::RangeInclusive::new(top_row, util::RowIndex::BOTTOM_ROW).len() *
        (util::FIELD_WIDTH as usize);
    if area >= vir_count as usize {
        TestResult::from_bool(
            preparation::prepare_field(&mut rand_pcg::Pcg64Mcg::seed_from_u64(seed), top_row, vir_count)
                .count() <= vir_count.into()
        )
    } else {
        TestResult::discard()
    }
}


#[quickcheck]
fn preparation_unique_pos(seed: u64, top_row: util::RowIndex, vir_count: u8) -> TestResult {
    use rand::SeedableRng;

    let area = util::RangeInclusive::new(top_row, util::RowIndex::BOTTOM_ROW).len() *
        (util::FIELD_WIDTH as usize);
    if area >= vir_count as usize {
        let mut pos: Vec<_> = preparation::prepare_field(
            &mut rand_pcg::Pcg64Mcg::seed_from_u64(seed),
            top_row,
            vir_count,
        ).map(|(p, _)| p).collect();
        pos.sort();
        TestResult::from_bool(pos.windows(2).all(|p| p[0] != p[1]))
    } else {
        TestResult::discard()
    }
}


#[quickcheck]
fn preparation_empty_rows(seed: u64, top_row: util::RowIndex, vir_count: u8) -> TestResult {
    use rand::SeedableRng;

    let area = util::RangeInclusive::new(top_row, util::RowIndex::BOTTOM_ROW).len() *
        (util::FIELD_WIDTH as usize);
    if area >= vir_count as usize {
        TestResult::from_bool(
            preparation::prepare_field(&mut rand_pcg::Pcg64Mcg::seed_from_u64(seed), top_row, vir_count)
                .all(|((r, _), _)| r >= top_row)
        )
    } else {
        TestResult::discard()
    }
}


#[quickcheck]
fn single_capsule_consitency(
    moves: Vec<movement::Movement>,
    a: util::Colour,
    b: util::Colour,
    row: util::RowIndex,
    mut virs: std::collections::HashMap<util::Position, util::Colour>,
) -> bool {
    use std::iter::FromIterator;

    use util::Step;

    {
        let rmid = util::ColumnIndex::LEFTMOST_COLUMN.forward_checked((util::FIELD_WIDTH/2).into())
            .expect("Failed to compute right target position for capsule");
        let lmid = rmid.backward_checked(1)
            .expect("Failed to compute left target position for capsule");
        virs.remove(&(row, lmid));
        virs.remove(&(row, rmid));
    }

    let mut moving_field = moving_field::MovingField::default();
    let static_field = static_field::StaticField::from_iter(virs);

    let (mut capsule, _) = movement::ControlledCapsule::spawn_capsule(&mut moving_field, &[a, b]);
    let ticks = Step::steps_between(&util::RowIndex::TOP_ROW, &row).expect("Invalid row");
    (0..ticks).for_each(|_| moving_field.tick().fold((), |_, _| ()));

    moves.into_iter().for_each(|m| { capsule.apply_move(&mut moving_field, &static_field, m); });

    check_element_partnership(&moving_field) && !util::ROWS
        .flat_map(util::complete_row)
        .any(|p| moving_field[p].is_some() && static_field[p].is_occupied())
}


#[quickcheck]
fn moving_single_capsule(
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


/// Check consistency in capsule element partnership
///
/// Check that, in the given field, if a capsule element refers to a partner,
/// that element refers back to the original element.
///
fn check_element_partnership<F>(field: &F) -> bool
where F: std::ops::Index<util::Position>,
      F::Output: items::AsCapsuleElement
{
    use items::AsCapsuleElement;

    util::ROWS
        .flat_map(util::complete_row)
        .filter_map(|p| field[p].as_element().and_then(|c| c.partner).map(|d| (p, d)))
        .all(|(p, d)| (p + d)
            .and_then(|p| field[p].as_element())
            .and_then(|c| c.partner) == Some(d.rotated_cw().rotated_cw()))
}

