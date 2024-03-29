//! Pre-tick functions and transfer types

use std::collections::HashSet;

use crate::util;

use super::items::RowOfFour;
use super::moving_field::MovingField;
use super::static_field::StaticField;


/// Settle elements
///
/// This function settles all capsules with at least one element which would be
/// moved to an occupied tile with the next tick. The function will only settle
/// elements from the top row to the provided lowest row, inclusive.
///
/// This function returns a list the settled capsule elements' positions as well
/// as the new lowest row containing unsettled elements or `None` If there are
/// none left.
///
pub fn settle_elements(
    moving_field: &mut MovingField,
    static_field: &mut StaticField,
    lowest: util::RowIndex,
) -> (Settled, Option<util::RowIndex>) {
    use util::Direction as Dir;

    // Settle elements, collecting their position
    let mut settled: Vec<_> = Default::default();
    util::RangeInclusive::new(util::RowIndex::TOP_ROW, lowest)
        .rev()
        .flat_map(util::complete_row)
        .for_each(|pos| if (pos + Dir::Below).map(|p| static_field[p].is_occupied()).unwrap_or(true) {
            // The tile below is occupied. Hence, we must move elements in the
            // current tile. However, we must not free the tile in the static
            // field but only transfer elements.
            if let Some(element) = moving_field[pos].take() {
                let partner = element
                    .partner
                    .and_then(|d| pos + d)
                    .and_then(|p| moving_field[p].take().map(|e| (p, e)));

                settled.push(pos);
                static_field[pos] = element.into();

                if let Some((pos, element)) = partner {
                        settled.push(pos);
                        static_field[pos] = element.into();
                }
            }
        });

    // Determine the new lowest row with unsettled elements
    let lowest = util::RangeInclusive::new(util::RowIndex::TOP_ROW, lowest)
        .rev()
        .find(|r| util::complete_row(*r).any(|p| moving_field[p].is_some()));

    (Settled {elements: settled}, lowest)
}


/// Settled elements' positions
///
pub struct Settled {
    elements: Vec<util::Position>,
}

impl std::ops::Deref for Settled {
    type Target = [util::Position];

    fn deref(&self) -> &Self::Target {
        self.elements.deref()
    }
}

impl IntoIterator for Settled {
    type Item = util::Position;
    type IntoIter = <Vec<Self::Item> as IntoIterator>::IntoIter;

    fn into_iter(self) -> Self::IntoIter {
        self.elements.into_iter()
    }
}

#[cfg(test)]
impl From<Vec<util::Position>> for Settled {
    fn from(elements: Vec<util::Position>) -> Self {
        Self {elements}
    }
}


/// Eliminate elements
///
/// This function eliminates rows of four from the field of settled elements.
/// These rows of four are detected based on hints provided in the form of
/// settled elements. The function will return a type encapsulating the
/// individual rows.
///
pub fn eliminate_elements(
    field: &mut StaticField,
    settled: &Settled
) -> Eliminated {
    use super::items::row_of_four;

    let rows: HashSet<_> = settled.iter().filter_map(|p| row_of_four(field, *p)).collect();
    let exes: HashSet<_> = rows
        .iter()
        .flat_map(|(_, p)| p.clone())
        .filter_map(|p| field[p].take().into_element().and_then(|e| e.partner).and_then(|d| p + d))
        .collect();
    exes.iter().for_each(|p| if let Some(e) = field[*p].as_element_mut() {
        e.partner = None
    });
    Eliminated {rows, exes}
}


/// Eliminated rows
///
pub struct Eliminated {
    // We use a hashset in order to prevent registering the same row twice.
    rows: HashSet<(util::Colour, RowOfFour)>,
    exes: HashSet<util::Position>,
}

impl Eliminated {
    /// Retrieve the colour and position of eliminated rows
    ///
    pub fn rows_of_four(&self) -> impl Iterator<Item = &(util::Colour, RowOfFour)> {
        self.rows.iter()
    }

    /// Retrieve the number of rows eliminated
    ///
    pub fn row_count(&self) -> usize {
        self.rows.len()
    }

    /// Retrieve the positions of eliminated elements
    ///
    pub fn positions(&self) -> impl Iterator<Item = util::Position> + '_ {
        self.rows_of_four().flat_map(|(_, p)| p.clone())
    }
}

#[cfg(test)]
impl Eliminated {
    pub fn new(
        rows: HashSet<(util::Colour, RowOfFour)>,
        exes: HashSet<util::Position>,
    ) -> Self {
        Self {rows, exes}
    }
}


/// Unsettle elements
///
/// This function unsettled all elements which are no longer supported by
/// another capsule element or virus. Unsettled elements are determined through
/// the list of eliminated elements as well as unsettling of elements during
/// processing.
///
/// This function returns the index of the lowest row in which an element was
/// unsettled. If no element was unsettled, it will return `None`.
///
pub fn unsettle_elements(
    moving_field: &mut MovingField,
    static_field: &mut StaticField,
    eliminated: &Eliminated
) -> Option<util::RowIndex> {
    use util::Direction as Dir;

    let mut lowest_unsettled = None;

    let mut worklist: std::collections::BinaryHeap<_> = eliminated
        .exes
        .iter()
        .cloned()
        .filter(|p| !(*p + Dir::Below).map(|p| static_field[p].is_occupied()).unwrap_or(true))
        .chain(eliminated.positions().filter_map(|p| p + Dir::Above))
        .collect();

    while let Some(pos) = worklist.pop() {
        if let Some(element) = static_field[pos].as_element() {
            let partner = element.partner.and_then(|d| pos + d);
            let partner_supported = partner
                .and_then(|p| p + Dir::Below)
                .filter(|p| *p != pos)
                .map(|p| static_field[p].is_occupied())
                .unwrap_or(false);
            if !partner_supported {
                let to_move = std::iter::once(pos)
                    .chain(partner)
                    .inspect(|p| moving_field[*p] = static_field[*p].take().into_element())
                    .inspect(|p| { lowest_unsettled.get_or_insert(p.0); });
                worklist.extend(to_move.filter_map(|p| p + Dir::Above));
            }
        }
    }

    lowest_unsettled
}

