//! Types representing items occupying individual tiles

use crate::util::Colour;


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

