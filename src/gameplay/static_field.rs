//! Definition of the static field and associated types

use crate::util;

use super::items::{CapsuleElement, Virus};
use super::row::Row;


/// Field of settled/non-moving elements
///
#[derive(Default)]
pub struct StaticField {
    data: [Row<TileContents>; util::FIELD_HEIGHT as usize],
}

impl std::ops::IndexMut<util::Position> for StaticField {
    fn index_mut(&mut self, index: util::Position) -> &mut Self::Output {
        &mut self.data[usize::from(index.0)][index.1]
    }
}

impl std::ops::Index<util::Position> for StaticField {
    type Output = TileContents;

    fn index(&self, index: util::Position) -> &Self::Output {
        &self.data[usize::from(index.0)][index.1]
    }
}


/// Representation of a single tile's contents
///
pub enum TileContents {
    None,
    CapsuleElement(CapsuleElement),
    Virus(Virus),
}

impl TileContents {
    /// Check whether the tile is occupied
    ///
    pub fn is_occupied(&self) -> bool {
        match self {
            Self::None => false,
            _ => true,
        }
    }

    /// Retrieve a reference to any capsule elemnt held by the tile
    ///
    /// If the tile holds a capsule element, this function will return a
    /// reference to that element. Otherwise, this function returns `None`.
    ///
    pub fn as_element(&self) -> Option<&CapsuleElement> {
        match self {
            Self::CapsuleElement(e) => Some(e),
            _ => None
        }
    }

    /// Retrieve a mutable reference to any capsule elemnt held by the tile
    ///
    /// If the tile holds a capsule element, this function will return a mutable
    /// reference to that element. Otherwise, this function returns `None`.
    ///
    pub fn as_element_mut(&mut self) -> Option<&mut CapsuleElement> {
        match self {
            Self::CapsuleElement(e) => Some(e),
            _ => None
        }
    }

    /// Retrieve the capsule elemnt held by the tile, if any
    ///
    /// If the tile holds a capsule element, this function will return that
    /// element. Otherwise, this function returns `None`.
    ///
    pub fn into_element(self) -> Option<CapsuleElement> {
        match self {
            Self::CapsuleElement(e) => Some(e),
            _ => None
        }
    }

    /// Retrieve a reference to any virus held by the tile
    ///
    /// If the tile holds a virus, this function will return a reference to that
    /// virus. Otherwise, this function returns `None`.
    ///
    pub fn as_virus(&self) -> Option<&Virus> {
        match self {
            Self::Virus(e) => Some(e),
            _ => None
        }
    }

    /// Take the tile's contents, leaving it unoccupied
    ///
    pub fn take(&mut self) -> Self {
        std::mem::take(self)
    }
}

impl Default for TileContents {
    fn default() -> Self {
        Self::None
    }
}

impl From<CapsuleElement> for TileContents {
    fn from(e: CapsuleElement) -> Self {
        Self::CapsuleElement(e)
    }
}

impl From<Virus> for TileContents {
    fn from(v: Virus) -> Self {
        Self::Virus(v)
    }
}

impl util::PotentiallyColoured for TileContents {
    fn colour(&self) -> Option<util::Colour> {
        match self {
            Self::None => None,
            Self::CapsuleElement(e) => Some(e.colour()),
            Self::Virus(v) => Some(v.colour()),
        }
    }
}


/// Find rows of four or more tiles of the same colour
///
/// This function finds horizontal and vertical configurations of at least four
/// tiles with the same colour. Only configurations which include the given
/// position will be considered. If such a configuration is found, it is
/// returned alongside the colour of that row.
///
/// The list of positions is not sorted in any particular order. Furthermore,
/// its order may depend on the hint.
///
pub fn row_of_four(
    field: &StaticField,
    hint: util::Position
) -> Option<(util::Colour, Vec<util::Position>)> {
    use util::Direction as Dir;
    use util::PotentiallyColoured;

    const ROW_OF_FOUR_LEN: usize = 4;

    field[hint]
        .colour()
        .and_then(|col| {
            let positions_towards = |dir| std::iter::successors(hint + dir, move |p| *p + dir)
                .take_while(|p| field[*p].colour() == Some(col));

            let positions: Vec<_> = std::iter::once(hint)
                .chain(positions_towards(Dir::Left))
                .chain(positions_towards(Dir::Right))
                .collect();
            if positions.len() >= ROW_OF_FOUR_LEN {
                return Some((col, positions))
            }

            let positions: Vec<_> = std::iter::once(hint)
                .chain(positions_towards(Dir::Above))
                .chain(positions_towards(Dir::Below))
                .collect();
            if positions.len() >= ROW_OF_FOUR_LEN {
                Some((col, positions))
            } else {
                None
            }
        })
}

