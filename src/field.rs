//! Gameplay related types, functions and utilities

mod items;
mod movement;
mod moving_field;
mod preparation;
mod row;
mod static_field;
mod tick;

#[cfg(test)]
pub mod tests;


pub use items::Update;
pub use static_field::{StaticField, defeated};
pub use moving_field::{MovingField, MovingRowIndex};
pub use tick::{settle_elements, eliminate_elements, unsettle_elements};
pub use movement::{Movement, ControlledCapsule};
pub use preparation::prepare_field;

