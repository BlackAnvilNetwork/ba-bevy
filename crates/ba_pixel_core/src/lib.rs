//! Shared pixel-field types. Simulation and rendering both speak this, not ECS-per-pixel.

mod field;
mod material;
pub mod palette;

pub use field::*;
pub use material::*;
