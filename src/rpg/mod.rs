//! Port of ba-stick `@bellgrave/combat` — jobs, grades, XP, core combat math.
//!
//! Explicit formula source of truth: [`COMBAT_CALCULATIONS.md`](COMBAT_CALCULATIONS.md).
//! Runtime helpers below are provisional until they cite and match those sections.
//! Ability / WS names & blurbs: `assets/combat/kits.json` (from ba-stick kits).

#![allow(dead_code)] // Full combat surface kept for upcoming fights.

mod combat;
mod fight;
mod grades;
mod jobs;
mod kits;
mod progression;

pub use combat::*;
pub use fight::*;
pub use grades::*;
pub use jobs::*;
pub use kits::*;
pub use progression::*;
