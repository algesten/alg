// For tests we use std.
#![cfg_attr(not(test), no_std)]

mod clock;
mod euclid;
mod gen;
mod pat;
mod rnd;

pub use clock::{Clock, Time};
pub use euclid::euclid;
pub use gen::Generated;
pub use pat::{Pattern, PatternGroup};
pub use rnd::Rnd;

#[cfg(test)]
mod drums;
