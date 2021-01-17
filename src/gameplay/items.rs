//! Types representing items occupying individual tiles

use crate::util::{Colour, Direction};


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

