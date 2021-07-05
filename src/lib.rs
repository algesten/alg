// For tests we use std.
#![cfg_attr(not(test), no_std)]

pub mod clock;
pub mod encoder;
pub mod euclid;
pub mod gen;
pub mod input;
pub mod pat;
pub mod rnd;

#[cfg(test)]
mod drums;

pub trait SetBit {
    fn set_bit(&mut self, bit: u8, on: bool);
}

impl SetBit for u16 {
    fn set_bit(&mut self, bit: u8, on: bool) {
        if on {
            *self = *self | 1 << bit;
        } else {
            *self = *self & !(1 << bit);
        }
    }
}
