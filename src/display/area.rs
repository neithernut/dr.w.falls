//! Screen area and related utilities

use super::commands::DrawCommand;


/// Displayable entity
///
pub trait Entity {
    /// Type of the entity after placing
    ///
    /// In the most simple case, e.g. if only an initial state of the entity has
    /// to be drawn, this can be `()`.
    ///
    type PlacedEntity;

    /// Retriev ethe number of rows the entity covers
    ///
    fn rows(&self) -> u16;

    /// Retriev ethe number of columns the entity covers
    ///
    fn cols(&self) -> u16;

    /// Create an initialization of the placed entity
    ///
    /// The returned initialization contains instructions for drawing the
    /// entity's initial state.
    ///
    fn init(&self, pos: (u16, u16)) -> PlacedInit;

    /// Place the entity
    ///
    /// This function places the entity on the given position and returns a
    /// value representing the entity after being placed.
    ///
    fn place(self, pos: (u16, u16)) -> Self::PlacedEntity;
}


/// Instructions for drawing the entity's initial state
///
#[derive(Debug)]
pub struct PlacedInit<'a> {
    cmds: Vec<DrawCommand<'a>>,
}

impl<'a> From<Vec<DrawCommand<'a>>> for PlacedInit<'a> {
    fn from(cmds: Vec<DrawCommand<'a>>) -> Self {
        Self {cmds}
    }
}

