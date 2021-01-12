//! Gameplay related types, functions and utilities

use std::ops;


const FIELD_WIDTH: usize = 8;
const FIELD_HEIGHT: usize = 16;


/// Representation of a field of settled/non-moving elements
///
/// The game field has 16 rows and 8 columns. The top-level index refers to a
/// row, with `0` referring to the top row. The second-level index refers to the
/// column, with `0` referring to the left-most column.
///
/// Only rows `1` through `15` are accessible on this field. Any attempt to
/// access the top row will result in a panic.
///
#[derive(Default)]
pub struct StaticField {
    rows: [[Tile; FIELD_WIDTH]; FIELD_HEIGHT - 1],
}

impl ops::IndexMut<usize> for StaticField {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.rows[index + 1]
    }
}

impl ops::Index<usize> for StaticField {
    type Output = [Tile; FIELD_WIDTH];

    fn index(&self, index: usize) -> &Self::Output {
        &self.rows[index + 1]
    }
}


/// Representation of a tile
///
pub enum Tile {
    None,
    CapsuleElement(CapsuleElement),
    Virus(Virus),
}

impl Default for Tile {
    fn default() -> Self {
        Self::None
    }
}


/// Representation of a capsule
///
pub struct CapsuleElement {
    colour: Colour,
    pub binding: Binding,
}

impl CapsuleElement {
    /// Create a new capsule
    ///
    pub fn new(colour: Colour) -> Self {
        Self {colour: colour, binding: Default::default()}
    }

    /// Retrieve the colour of a capsule
    ///
    pub fn colour(&self) -> Colour {
        self.colour
    }
}


/// Binding of a capsule element
///
/// A capsule may be bound to a direct neighbor.
///
pub enum Binding {
    None,
    Left,
    Right,
    Above,
    Below,
}

impl Default for Binding {
    fn default() -> Self {
        Self::None
    }
}


/// Representation of a virus
///
pub struct Virus {
    colour: Colour,
}

impl Virus {
    pub fn new(colour: Colour) -> Self {
        Self {colour: colour}
    }

    pub fn colour(&self) -> Colour {
        self.colour
    }
}


/// Representation of a `Capsule`'s or `Virus`' colour
///
#[derive(Copy, Clone, PartialEq)]
pub enum Colour {
    Red,
    Yellow,
    Blue,
}

