//! Movement function and input types

use crate::util;

use super::items;
use super::moving_field::{MovingField, MovingRowIndex};
use super::static_field::StaticField;


/// Handle for a player controlled capsule
///
/// A value of this type represents a player controlled capsule. While its
/// elements occupy tiles in the field of moving elements, this type provides
/// means to control the capsule's movements.
///
struct ControlledCapsule{
    row: MovingRowIndex,
    column: util::ColumnIndex,
}

impl ControlledCapsule {
    /// Spawn a new player controlled capsule
    ///
    /// Place a new player controlled capsule in the given colours. The new
    /// capsule will be placed in the (current) top row of the moving field.
    ///
    pub fn spawn_capsule(
        moving_field: &mut MovingField,
        colours: impl AsRef<[util::Colour; 2]>
    ) -> Self {
        use util::Step;

        let rmid = util::ColumnIndex::LEFTMOST_COLUMN.forward_checked((util::FIELD_WIDTH/2).into())
            .expect("Failed to compute right position for new capsule");
        let lmid = rmid.backward_checked(1)
            .expect("Failed to compute left position for new capsule");

        moving_field[(util::RowIndex::TOP_ROW, lmid)] = Some(
            items::CapsuleElement::new(colours.as_ref()[0], Some(util::Direction::Right))
        );
        moving_field[(util::RowIndex::TOP_ROW, rmid)] = Some(
            items::CapsuleElement::new(colours.as_ref()[1], Some(util::Direction::Left))
        );
        Self {row: moving_field.moving_row_index(util::RowIndex::TOP_ROW), column: lmid}
    }
}

