//! Definition of the static field and associated types

use crate::util;

use super::items::{self, CapsuleElement, Virus};
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

/// Initialize a field from an iterator
///
/// For each item in the source iterator, a virus with the given colour will be
/// placed in the tile on the given position.
///
impl std::iter::FromIterator<(util::Position, util::Colour)> for StaticField {
    fn from_iter<T>(iter: T) -> Self
        where T: IntoIterator<Item = (util::Position, util::Colour)>
    {
        iter.into_iter().fold(Default::default(), |mut field, (pos, colour)| {
            field[pos] = TileContents::Virus(Virus::new(colour));
            field
        })
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

impl items::AsCapsuleElement for TileContents {
    fn as_element(&self) -> Option<&CapsuleElement> {
        match self {
            Self::CapsuleElement(e) => Some(e),
            _ => None
        }
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


/// Check whether the player with the given field is defeated
///
/// This function returns true if any tile in the top row is occupied.
///
pub fn defeated(field: &StaticField) -> bool {
    util::COLUMNS.map(|c| (util::RowIndex::TOP_ROW, c)).any(|p| field[p].is_occupied())
}

