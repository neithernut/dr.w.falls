//! Display rendering utilities

mod area;
mod commands;
mod display;
mod dynamic_text;
mod field;
mod input;
mod scores;
mod static_text;

#[cfg(test)]
pub mod tests;


pub use area::Area;
pub use commands::DrawHandle;
pub use display::Display;
pub use dynamic_text::DynamicText;
pub use field::{FieldUpdater, PlayField};
pub use input::LineInput;
pub use scores::{Entry as ScoreBoardEntry, ScoreBoard};
pub use static_text::StaticText;

