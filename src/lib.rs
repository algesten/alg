// For tests we use std.
#![cfg_attr(not(test), no_std)]

pub mod clock;
pub mod euclid;
pub mod gen;
pub mod pat;
pub mod rnd;

#[cfg(test)]
mod drums;
