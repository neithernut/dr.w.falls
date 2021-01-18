//! Pre-tick functions and transfer types

use crate::util;

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

