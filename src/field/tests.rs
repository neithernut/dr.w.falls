//! Field tests

use quickcheck::{Arbitrary, Gen, TestResult};

use crate::util;

use super::*;


#[quickcheck]
fn full_tick_consistency(static_field: StaticField, moving_field: MovingField) -> bool {
    let mut static_field: static_field::StaticField = static_field.into();
    let mut moving_field = moving_field.instantiate_for(&static_field);

    let (settled, _) = tick::settle_elements(
        &mut moving_field,
        &mut static_field,
        util::RowIndex::BOTTOM_ROW
    );
    let eliminated = tick::eliminate_elements(&mut static_field, &settled);
    tick::unsettle_elements(&mut moving_field, &mut static_field, &eliminated);
    moving_field.tick().fold((), |_, _| ());

    check_overlaps(&static_field, &moving_field) &&
        check_element_partnership(&static_field) &&
        check_element_partnership(&moving_field)
}


#[quickcheck]
fn settlement_settled_positions(
    static_field: StaticField,
    moving_field: MovingField,
    bottom: util::RowIndex,
) -> bool {
    let mut static_field: static_field::StaticField = static_field.into();
    let mut moving_field = moving_field.instantiate_for(&static_field);
    tick::settle_elements(&mut moving_field, &mut static_field, bottom)
        .0
        .into_iter()
        .all(|p| static_field[p].is_occupied() && moving_field[p].is_none())
}


#[quickcheck]
fn settlement_lowest_unsettled(
    static_field: StaticField,
    moving_field: MovingField,
    bottom: util::RowIndex,
) -> bool {
    use util::Step;

    let mut static_field: static_field::StaticField = static_field.into();
    let mut moving_field = moving_field.instantiate_for(&static_field);
    let (_, lowest) = tick::settle_elements(&mut moving_field, &mut static_field, bottom);

    let is_empty_to_bottom = |top| util::RangeInclusive::new(top, bottom)
        .flat_map(util::complete_row)
        .all(|p| moving_field[p].is_none());

    if let Some(lowest) = lowest {
        lowest.forward_checked(1).filter(|l| *l <= bottom).map(is_empty_to_bottom).unwrap_or(true)
    } else {
        is_empty_to_bottom(util::RowIndex::TOP_ROW)
    }
}


#[quickcheck]
fn settlement_element_partnership(static_field: StaticField, moving_field: MovingField) -> bool {
    let mut static_field: static_field::StaticField = static_field.into();
    let mut moving_field = moving_field.instantiate_for(&static_field);
    tick::settle_elements(&mut moving_field, &mut static_field, util::RowIndex::BOTTOM_ROW);
    check_element_partnership(&static_field) && check_element_partnership(&moving_field)
}


#[quickcheck]
fn settlement_tick(static_field: StaticField, moving_field: MovingField) -> bool {
    let mut static_field: static_field::StaticField = static_field.into();
    let mut moving_field = moving_field.instantiate_for(&static_field);
    tick::settle_elements(&mut moving_field, &mut static_field, util::RowIndex::BOTTOM_ROW);
    moving_field.tick().fold((), |_, _| ());
    check_overlaps(&static_field, &moving_field) && check_element_partnership(&moving_field)
}


#[quickcheck]
fn elimination_result(field: StaticField, settled: Vec<util::Position>) -> bool {
    let mut field: static_field::StaticField = field.into();
    tick::eliminate_elements(&mut field, &settled.into())
        .positions()
        .all(|p| !field[p].is_occupied())
}


#[quickcheck]
fn elimination_element_partnership(field: StaticField, settled: Vec<util::Position>) -> bool {
    let mut field: static_field::StaticField = field.into();
    tick::eliminate_elements(&mut field, &settled.into());
    check_element_partnership(&field)
}


#[quickcheck]
fn unsettlement_consistency(
    static_field: StaticField,
    rows: std::collections::HashSet<(util::Colour, items::RowOfFour)>,
) -> bool {
    let mut static_field: static_field::StaticField = static_field.into();
    let mut moving_field = Default::default();
    tick::unsettle_elements(&mut moving_field, &mut static_field, &rows.into());
    check_overlaps(&static_field, &moving_field) &&
        check_element_partnership(&static_field) &&
        check_element_partnership(&moving_field)
}


#[quickcheck]
fn unsettlement_tick(
    static_field: StaticField,
    rows: std::collections::HashSet<(util::Colour, items::RowOfFour)>,
) -> bool {
    let mut static_field: static_field::StaticField = static_field.into();
    let mut moving_field = Default::default();

    let eliminated: tick::Eliminated = rows.into();
    // Taken from `eliminate_elements`
    eliminated.positions().for_each(|p| static_field[p]
        .take()
        .into_element()
        .and_then(|e| e.partner)
        .and_then(|d| p + d)
        .and_then(|p| static_field[p].as_element_mut())
        .map(|e| e.partner = None)
        .unwrap_or_default()
    );

    tick::unsettle_elements(&mut moving_field, &mut static_field, &eliminated);
    moving_field.tick().fold((), |_, _| ());
    check_overlaps(&static_field, &moving_field) && check_element_partnership(&moving_field)
}


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
    static_field: StaticField,
) -> TestResult {
    use util::Step;

    let mut moving_field = moving_field::MovingField::default();
    let static_field: static_field::StaticField = static_field.into();

    {
        let rmid = util::ColumnIndex::LEFTMOST_COLUMN.forward_checked((util::FIELD_WIDTH/2).into())
            .expect("Failed to compute right target position for capsule");
        let lmid = rmid.backward_checked(1)
            .expect("Failed to compute left target position for capsule");
        if static_field[(row, lmid)].is_occupied() || static_field[(row, rmid)].is_occupied() {
            return TestResult::discard()
        }
    }

    let (mut capsule, _) = movement::ControlledCapsule::spawn_capsule(&mut moving_field, &[a, b]);
    let ticks = Step::steps_between(&util::RowIndex::TOP_ROW, &row).expect("Invalid row");
    (0..ticks).for_each(|_| moving_field.tick().fold((), |_, _| ()));

    moves.into_iter().for_each(|m| { capsule.apply_move(&mut moving_field, &static_field, m); });

    TestResult::from_bool(
        check_element_partnership(&moving_field) && check_overlaps(&static_field, &moving_field)
    )
}


#[quickcheck]
fn single_capsule_output(
    movement: movement::Movement,
    a: util::Colour,
    b: util::Colour,
    row: util::RowIndex,
    static_field: StaticField,
) -> TestResult {
    use util::{PotentiallyColoured, Step};

    let mut moving_field = moving_field::MovingField::default();
    let static_field: static_field::StaticField = static_field.into();

    let rmid = util::ColumnIndex::LEFTMOST_COLUMN.forward_checked((util::FIELD_WIDTH/2).into())
        .expect("Failed to compute right target position for capsule");
    let lmid = rmid.backward_checked(1)
        .expect("Failed to compute left target position for capsule");
    if static_field[(row, lmid)].is_occupied() || static_field[(row, rmid)].is_occupied() {
        return TestResult::discard()
    }

    let (mut capsule, _) = movement::ControlledCapsule::spawn_capsule(&mut moving_field, &[a, b]);
    let ticks = Step::steps_between(&util::RowIndex::TOP_ROW, &row).expect("Invalid row");
    (0..ticks).for_each(|_| moving_field.tick().fold((), |_, _| ()));

    let res = if let Some(updates) = capsule.apply_move(&mut moving_field, &static_field, movement) {
        // Later updates overwrite earlier ones. Here we assume that later
        // values will also overwrite earlier values for the same key when
        // collecting into a `HashMap`.
        let updates: std::collections::HashMap<_, _> = updates.iter().cloned().collect();
        updates.into_iter().all(|(p, c)| moving_field[p].colour() == c)
    } else {
        moving_field[(row, lmid)].colour() == Some(a) && moving_field[(row, rmid)].colour() == Some(b)
    };
    TestResult::from_bool(res)
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
fn tick_output(field: MovingField) -> bool {
    use util::PotentiallyColoured;

    let mut field = field.instantiate_for(&Default::default());
    let updates: Vec<_> = field.tick().collect();
    updates.into_iter().all(|(p, c)| field[p].colour() == c)
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


#[quickcheck]
fn find_no_row_of_four(mut field: TwoColouredField, pos: util::Position) -> bool {
    field[pos] = None;
    items::row_of_four(&field, pos).is_none()
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


/// Static field construction helper
///
#[derive(Clone, Debug)]
pub struct StaticField {
    viruses: std::collections::HashMap<util::Position, util::Colour>,
    capsules: Vec<RandomCapsule>,
}

impl From<StaticField> for static_field::StaticField {
    fn from(field: StaticField) -> Self {
        let mut res: Self = std::iter::FromIterator::from_iter(field.viruses);
        field.capsules.into_iter().for_each(|c| c.try_place_on(&mut res));
        res
    }
}

impl Arbitrary for StaticField {
    fn arbitrary(g: &mut Gen) -> Self {
        Self {viruses: Arbitrary::arbitrary(g), capsules: Arbitrary::arbitrary(g)}
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let res = (self.viruses.clone(), self.capsules.clone())
            .shrink()
            .map(|(viruses, capsules)| StaticField{viruses, capsules});
        Box::new(res)
    }
}


#[quickcheck]
fn random_capsule_placement(field: StaticField) -> bool {
    check_element_partnership(&static_field::StaticField::from(field))
}


/// Moving field construction helper
///
#[derive(Clone, Debug)]
pub struct MovingField {
    capsules: Vec<RandomCapsule>,
}

impl MovingField {
    /// Fill a moving field with capsules honouring occupied positions in a moving field
    ///
    pub fn instantiate_for(&self, field: &static_field::StaticField) -> moving_field::MovingField {
        use util::PotentiallyColoured;

        let mut res: moving_field::MovingField = Default::default();
        self.capsules
            .iter()
            .cloned()
            .for_each(|c| c.try_place_on_masked(&mut res, |p| field[p].colour().is_none()));
        res
    }
}

impl Arbitrary for MovingField {
    fn arbitrary(g: &mut Gen) -> Self {
        Self {capsules: Arbitrary::arbitrary(g)}
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        Box::new(self.capsules.shrink().map(|capsules| Self{capsules}))
    }
}


#[quickcheck]
fn random_moving_capsule_placement(static_field: StaticField, moving_field: MovingField) -> bool {
    let static_field: static_field::StaticField = static_field.into();
    let moving_field = moving_field.instantiate_for(&static_field);
    check_overlaps(&static_field, &moving_field) && check_element_partnership(&moving_field)
}


/// A random capsule or single capsule element
///
#[derive(Copy, Clone, Debug)]
struct RandomCapsule {
    pos: util::Position,
    colour: util::Colour,
    partner: Option<(util::Direction, util::Colour)>,
}

impl RandomCapsule {
    /// Try to place this capsule on the given field
    ///
    /// Capsule lements will only be placed if the respective positions are not
    /// already occupied (i.e. coloured).
    ///
    pub fn try_place_on<F>(&self, field: &mut F)
        where F: std::ops::IndexMut<util::Position>,
              F::Output: From<items::CapsuleElement> + util::PotentiallyColoured,
    {
        self.try_place_on_masked(field, |_| true)
    }

    /// Try to place this capsule on the given field
    ///
    /// Capsule lements will only be placed if the supplied `mask` returns
    /// `false` for the respective positions and they are not already occupied
    /// (i.e. coloured).
    ///
    pub fn try_place_on_masked<F>(&self, field: &mut F, mask: impl Fn(util::Position) -> bool)
        where F: std::ops::IndexMut<util::Position>,
              F::Output: From<items::CapsuleElement> + util::PotentiallyColoured,
    {
        use util::PotentiallyColoured;

        if field[self.pos].colour().is_some() || !mask(self.pos) {
            return
        }

        let partner = self
            .partner
            .and_then(|(d, c)| (self.pos + d).map(|p| (d, c, p)))
            .filter(|(.., p)| field[*p].colour().is_none() && mask(*p));

        field[self.pos] = items::CapsuleElement::new(self.colour, partner.map(|(d, ..)| d)).into();
        if let Some((d, c, p)) = partner {
            field[p] = items::CapsuleElement::new(c, Some(d.rotated_cw().rotated_cw())).into();
        }
    }

    /// Transform a set of capsules so that they are consistent
    ///
    pub fn consistent_capsules(
        capsules: impl IntoIterator<Item = Self>,
        occupied: impl IntoIterator<Item = util::Position>,
    ) -> impl Iterator<Item = Self> {
        let mut occupied: std::collections::HashSet<_> = std::iter::FromIterator::from_iter(occupied);

        capsules.into_iter().filter_map(move |mut c| if occupied.insert(c.pos) {
            if c.partner.and_then(|(d, _)| c.pos + d).map(|p| !occupied.insert(p)).unwrap_or(true) {
                c.partner = None;
            }
            Some(c)
        } else {
            None
        })
    }
}

impl Arbitrary for RandomCapsule {
    fn arbitrary(g: &mut Gen) -> Self {
        Self {
            pos: Arbitrary::arbitrary(g),
            colour: Arbitrary::arbitrary(g),
            partner: Arbitrary::arbitrary(g),
        }
    }

    fn shrink(&self) -> Box<dyn Iterator<Item = Self>> {
        let res = (self.pos, self.colour, self.partner)
            .shrink()
            .map(|(pos, colour, partner)| Self{pos, colour, partner});
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


/// Check that a position is only occupied in one of the fields
///
fn check_overlaps(
    static_field: &static_field::StaticField,
    moving_field: &moving_field::MovingField,
) -> bool {
    !util::ROWS
        .flat_map(util::complete_row)
        .any(|p| moving_field[p].is_some() && static_field[p].is_occupied())
}

