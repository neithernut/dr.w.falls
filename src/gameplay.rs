//! Gameplay related types, functions and utilities

const FIELD_WIDTH: usize = 8;
const FIELD_HEIGHT: usize = 16;


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

