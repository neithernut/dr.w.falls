//! Types representing items occupying individual tiles

use crate::util;
use util::{Colour, Direction};


/// Representation of a virus
///
pub struct Virus {
    colour: Colour,
}

impl Virus {
    /// Create a new virus with the given colour
    ///
    pub fn new(colour: Colour) -> Self {
        Self {colour}
    }

    /// Retrieve the virus' colour
    ///
    pub fn colour(&self) -> Colour {
        self.colour
    }
}


/// Representation of a capsule element
///
pub struct CapsuleElement {
    colour: Colour,
    /// Direction of any capsule element bound to this one
    ///
    pub partner: Option<Direction>,
}

impl CapsuleElement {
    /// Create a new capsule element
    ///
    pub fn new(colour: Colour, partner: Option<Direction>) -> Self {
        Self {colour, partner}
    }

    /// Create a new unbound capsule element
    ///
    /// The capsule element will not have a partner.
    ///
    pub fn new_single(colour: Colour) -> Self {
        Self::new(colour, None)
    }

    /// Retrieve the capsule element's colour
    ///
    pub fn colour(&self) -> Colour {
        self.colour
    }
}

impl util::PotentiallyColoured for Option<CapsuleElement> {
    fn colour(&self) -> Option<Colour> {
        self.as_ref().map(|e| e.colour())
    }
}


/// Representation of a field update
///
/// A tuple of this kind may be used to convey updates in the play field. If it
/// contains a `Colour`, it represents a capsule elements emerging at the given
/// position. A `None` will represent the tile to become free.
///
pub type Update = (util::Position, Option<Colour>);


/// Find rows of four or more tiles of the same colour
///
/// This function finds horizontal and vertical configurations of at least four
/// tiles with the same colour. Only configurations which include the given
/// position will be considered. If such a configuration is found, it is
/// returned alongside the colour of that row.
///
pub fn row_of_four<F>(
    field: &F,
    hint: util::Position
) -> Option<(Colour, RowOfFour)>
    where F: std::ops::Index<util::Position>,
          F::Output: util::PotentiallyColoured
{
    use util::Direction as Dir;
    use util::PotentiallyColoured;

    const ROW_OF_FOUR_LEN: usize = 4;

    field[hint]
        .colour()
        .and_then(|col| {
            let positions_towards = |dir| std::iter::successors(Some(hint), move |p| *p + dir)
                .take_while(|p| field[*p].colour() == Some(col))
                .last()
                .expect("Position of tile with hint's colour");

            let columns = util::RangeInclusive::new(
                positions_towards(Dir::Left).1,
                positions_towards(Dir::Right).1
            );
            if columns.len() >= ROW_OF_FOUR_LEN {
                return Some((col, RowOfFour::Horizontal(hint.0, columns)))
            }

            let rows = util::RangeInclusive::new(
                positions_towards(Dir::Above).0,
                positions_towards(Dir::Below).0
            );
            if rows.len() >= ROW_OF_FOUR_LEN {
                Some((col, RowOfFour::Vertical(rows, hint.1)))
            } else {
                None
            }
        })
}


/// Representation of a vertical or horizontal configuration of elements
///
/// This type is intended as an output type for the `row_of_four` function.
///
#[derive(Copy, Clone, PartialEq, Eq, Hash)]
pub enum RowOfFour {
    Horizontal(util::RowIndex, util::RangeInclusive<util::ColumnIndex>),
    Vertical(util::RangeInclusive<util::RowIndex>, util::ColumnIndex),
}

impl Iterator for RowOfFour {
    type Item = util::Position;

    fn next(&mut self) -> Option<Self::Item> {
        match self {
            Self::Horizontal(row, columns)  => columns.next().map(|c| (*row, c)),
            Self::Vertical(rows, column)    => rows.next().map(|r| (r, *column)),
        }
    }

    fn size_hint(&self) -> (usize, Option<usize>) {
        match self {
            Self::Horizontal(_, range)  => range.size_hint(),
            Self::Vertical(range, _)    => range.size_hint(),
        }
    }
}

