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
pub struct ControlledCapsule {
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
        colours: &[util::Colour; 2]
    ) -> (Self, [items::Update; 2]) {
        use util::Step;

        let rmid = util::ColumnIndex::LEFTMOST_COLUMN.forward_checked((util::FIELD_WIDTH/2).into())
            .expect("Failed to compute right position for new capsule");
        let lmid = rmid.backward_checked(1)
            .expect("Failed to compute left position for new capsule");

        let lmid = (util::RowIndex::TOP_ROW, lmid);
        let rmid = (util::RowIndex::TOP_ROW, rmid);

        moving_field[lmid] = Some(items::CapsuleElement::new(colours[0], Some(util::Direction::Right)));
        moving_field[rmid] = Some(items::CapsuleElement::new(colours[1], Some(util::Direction::Left)));

        (
            Self {row: moving_field.moving_row_index(util::RowIndex::TOP_ROW), column: lmid.1},
            [(lmid, Some(colours[0])), (rmid, Some(colours[1]))]
        )
    }

    /// Retrieve the current "active" row
    ///
    /// This function will return the lowest row containing an element of the
    /// capsule.
    ///
    pub fn row(&self) -> MovingRowIndex {
        self.row
    }

    /// Apply a movement to the capsule
    ///
    /// The function returns a list of `Update`s which have to be applied in
    /// order. If the movement could not be performed (e.g. because a target
    /// tile is occupied), the function returns `None`.
    ///
    pub fn apply_move(
        &mut self,
        moving_field: &mut MovingField,
        static_field: &StaticField,
        movement: Movement,
    ) -> Option<[items::Update; 4]> {
        match movement {
            Movement::Left      => self.move_left(moving_field, static_field),
            Movement::Right     => self.move_right(moving_field, static_field),
            Movement::RotateCW  => self.rotate_cw(moving_field, static_field),
            Movement::RotateCCW => self.rotate_ccw(moving_field, static_field),
        }
    }

    /// Move the capsule to the left
    ///
    /// The function returns a list of `Update`s which have to be applied in
    /// order. If the movement could not be performed (e.g. because a target
    /// tile is occupied), the function returns `None`.
    ///
    pub fn move_left(
        &mut self,
        moving_field: &mut MovingField,
        static_field: &StaticField,
    ) -> Option<[items::Update; 4]> {
        use util::Direction as Dir;

        self.move_elements(
            moving_field,
            static_field,
            |pos| Some([(pos[0] + Dir::Left)?, (pos[1] + Dir::Left)?]),
            std::convert::identity
        )
    }

    /// Move the capsule to the right
    ///
    /// The function returns a list of `Update`s which have to be applied in
    /// order. If the movement could not be performed (e.g. because a target
    /// tile is occupied), the function returns `None`.
    ///
    pub fn move_right(
        &mut self,
        moving_field: &mut MovingField,
        static_field: &StaticField,
    ) -> Option<[items::Update; 4]> {
        use util::Direction as Dir;

        self.move_elements(
            moving_field,
            static_field,
            |pos| Some([(pos[0] + Dir::Right)?, (pos[1] + Dir::Right)?]),
            std::convert::identity
        )
    }

    /// Rotate the capsule clockwise
    ///
    /// The function returns a list of `Update`s which have to be applied in
    /// order. If the movement could not be performed (e.g. because a target
    /// tile is occupied), the function returns `None`.
    ///
    pub fn rotate_cw(
        &mut self,
        moving_field: &mut MovingField,
        static_field: &StaticField,
    ) -> Option<[items::Update; 4]> {
        use util::Direction as Dir;

        self.move_elements(
            moving_field,
            static_field,
            |pos| match direction(pos[0], pos[1]) {
                Dir::Left   => Some([pos[0], (pos[0] + Dir::Above)?]),
                Dir::Right  => Some([(pos[1] + Dir::Above)?, pos[1]]),
                Dir::Above  => Some([pos[0], (pos[0] + Dir::Right)?]),
                Dir::Below  => Some([(pos[1] + Dir::Right)?, pos[1]]),
            },
            |mut e| {e.partner = e.partner.map(Dir::rotated_cw); e}
        )
    }

    /// Rotate the capsule counterclockwise
    ///
    /// The function returns a list of `Update`s which have to be applied in
    /// order. If the movement could not be performed (e.g. because a target
    /// tile is occupied), the function returns `None`.
    ///
    pub fn rotate_ccw(
        &mut self,
        moving_field: &mut MovingField,
        static_field: &StaticField,
    ) -> Option<[items::Update; 4]> {
        use util::Direction as Dir;

        self.move_elements(
            moving_field,
            static_field,
            |pos| match direction(pos[0], pos[1]) {
                Dir::Left   => Some([(pos[1] + Dir::Above)?, pos[1]]),
                Dir::Right  => Some([pos[0], (pos[0] + Dir::Above)?]),
                Dir::Above  => Some([pos[0], (pos[0] + Dir::Left)?]),
                Dir::Below  => Some([(pos[1] + Dir::Left)?, pos[1]]),
            },
            |mut e| {e.partner = e.partner.map(Dir::rotated_ccw); e}
        )
    }

    /// Internal utility function for performing the move
    ///
    /// This function performs a move defined by `transform_pos`. That functor
    /// receives the positions of the capsule's two elements and is expected to
    /// return the positions after the move. While the elements are moved, they
    /// are subjected to the transformation given via `transform_element`.
    ///
    /// The function returns a list of `Update`s which have to be applied in
    /// order. If the movement could not be performed (e.g. because a target
    /// tile is occupied), the function returns `None`.
    ///
    fn move_elements(
        &mut self,
        moving_field: &mut MovingField,
        static_field: &StaticField,
        transform_pos: impl Fn([util::Position; 2]) -> Option<[util::Position; 2]>,
        transform_element: impl Fn(items::CapsuleElement) -> items::CapsuleElement + Copy,
    ) -> Option<[items::Update; 4]> {
        use util::PotentiallyColoured;

        let row = moving_field.row_index_from_moving(self.row);
        let opos = {
            let pos_a = (row, self.column);
            let pos_b = moving_field[pos_a]
                .as_ref()
                .and_then(|e| e.partner)
                .and_then(|d| pos_a + d)
                .expect("Incomplete controlled capsule");
            [pos_a, pos_b]
        };

        let tpos = transform_pos(opos)?;

        if !tpos.iter().any(|p| static_field[*p].is_occupied()) {
            let mut element = [moving_field[opos[0]].take(), moving_field[opos[1]].take()];
            let colour = [element[0].colour(), element[1].colour()];
            moving_field[tpos[0]] = element[0].take().map(transform_element);
            moving_field[tpos[1]] = element[1].take().map(transform_element);
            self.column = tpos
                .iter()
                .find(|p| p.0 == row)
                .expect("Controlled capsule left its row")
                .1;
            Some([(opos[0], None), (opos[1], None), (tpos[0], colour[0]), (tpos[1], colour[1])])
        } else {
            None
        }
    }
}


#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum Movement {
    Left,
    Right,
    RotateCW,
    RotateCCW,
}

#[cfg(test)]
impl quickcheck::Arbitrary for Movement {
    fn arbitrary(g: &mut quickcheck::Gen) -> Self {
        *g.choose(&[Self::Left, Self::Right, Self::RotateCW, Self::RotateCCW]).unwrap()
    }
}


/// Determine the direction a position lies from another
///
/// This function returns the direction that `dest` lies from `source` under the
/// assumption that the corresponding tiles are adjacent. The behaviour is
/// undefined if they are not.
///
fn direction(src: util::Position, dest: util::Position) -> util::Direction {
    use std::cmp::Ordering;
    use util::Direction;

    match src.0.cmp(&dest.0) {
        Ordering::Less      => Direction::Below,
        Ordering::Equal     => if src.1 > dest.1 { Direction::Left } else { Direction::Right },
        Ordering::Greater   => Direction::Above,
    }
}

