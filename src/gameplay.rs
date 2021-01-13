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

impl Field for StaticField {
    type Tile = Tile;

    fn tile(&self, row: usize, col: usize) -> &Self::Tile {
        &self[row][col]
    }

    fn tile_mut(&mut self, row: usize, col: usize) -> &mut Self::Tile {
        &mut self[row][col]
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


/// Representation of a field of unsettled/moving elements
///
/// The game field has 16 rows and 8 columns. The top-level index refers to a
/// row, with `0` referring to the top row. The second-level index refers to the
/// column, with `0` referring to the left-most column.
///
/// The rows of this field can be cycled down via the `cycle` function.
///
#[derive(Default)]
pub struct MovingField {
    rows: [[Option<CapsuleElement>; FIELD_WIDTH]; FIELD_HEIGHT],
    offset: usize,
}

impl MovingField {
    /// Cycle the rows
    ///
    /// All rows are "moved" down one index, and the bottom row becomes the new
    /// top row.
    ///
    pub fn cycle(&mut self) {
        self.offset = self.offset.checked_sub(1).unwrap_or(self.rows.len() - 1);
    }

    /// Generate a handle for accessing one moving row
    ///
    /// This function creates a handle for accessing one moving row. Initially,
    /// accessing the row through the handle will be equivalent to accessing the
    /// row via the index directly. However, the handle will refer to the same
    /// row as it moves down the field with each call to `cycle`.
    ///
    pub fn row_handle(&self, index: usize) -> MovingRowHandle {
        MovingRowHandle {index: self.translate(index)}
    }

    /// Translate an unmapped index to a mapped index
    ///
    fn translate(&self, index: usize) -> usize {
        (index + self.offset) % self.rows.len()
    }
}

impl ops::IndexMut<usize> for MovingField {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        &mut self.rows[self.translate(index)]
    }
}

impl ops::Index<usize> for MovingField {
    type Output = [Option<CapsuleElement>; FIELD_WIDTH];

    fn index(&self, index: usize) -> &Self::Output {
        &self.rows[self.translate(index)]
    }
}

impl ops::IndexMut<MovingRowHandle> for MovingField {
    fn index_mut(&mut self, index: MovingRowHandle) -> &mut Self::Output {
        &mut self.rows[index.index]
    }
}

impl ops::Index<MovingRowHandle> for MovingField {
    type Output = [Option<CapsuleElement>; FIELD_WIDTH];

    fn index(&self, index: MovingRowHandle) -> &Self::Output {
        &self.rows[index.index]
    }
}

impl Field for MovingField {
    type Tile = Option<CapsuleElement>;

    fn tile(&self, row: usize, col: usize) -> &Self::Tile {
        &self[row][col]
    }

    fn tile_mut(&mut self, row: usize, col: usize) -> &mut Self::Tile {
        &mut self[row][col]
    }
}


/// Handle for one moving row within a MovingField
///
/// An instance of this type will refer to one row in the moving field. The
/// handle tracks the row as it moves downwards.
///
#[derive(Copy, Clone, PartialEq)]
pub struct MovingRowHandle {
    index: usize
}

impl ops::Add<usize> for MovingRowHandle {
    type Output = MovingRowHandle;

    fn add(mut self, rhs: usize) -> Self::Output {
        self.index = self.index + rhs;
        self
    }
}

impl ops::Add<usize> for &MovingRowHandle {
    type Output = MovingRowHandle;

    fn add(self, rhs: usize) -> Self::Output {
        (*self).add(rhs)
    }
}

impl ops::Sub<usize> for MovingRowHandle {
    type Output = MovingRowHandle;

    fn sub(mut self, rhs: usize) -> Self::Output {
        if rhs > self.index {
            self.index += FIELD_HEIGHT;
        }
        self.index = self.index - rhs;
        self
    }
}

impl ops::Sub<usize> for &MovingRowHandle {
    type Output = MovingRowHandle;

    fn sub(self, rhs: usize) -> Self::Output {
        (*self).sub(rhs)
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


/// Trait abstracting a field
///
pub trait Field {
    /// Type of a single tile
    ///
    type Tile;

    /// Access a tile by reference
    ///
    fn tile(&self, row: usize, col: usize) -> &Self::Tile;

    /// Access a tile by reference, mutable
    ///
    fn tile_mut(&mut self, row: usize, col: usize) -> &mut Self::Tile;

    /// Retrieve the width of a field
    ///
    fn width(&self) -> usize {
        FIELD_WIDTH
    }

    /// Retrieve the height of a field
    ///
    fn height(&self) -> usize {
        FIELD_HEIGHT
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

