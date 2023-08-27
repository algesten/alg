// For tests we use std.
#![cfg_attr(not(test), no_std)]

#[macro_use]
extern crate log;

pub mod bitfield;
pub mod clock;
pub mod encoder;
pub mod euclid;
pub mod exec;
pub mod gen;
pub mod geom;
pub mod input;
pub mod pat;
pub mod ring_buf;
pub mod rnd;
pub mod tempo;

#[cfg(feature = "float")]
pub mod wave;

#[cfg(test)]
mod drums;

pub trait SetBit {
    fn set_bit(&mut self, bit: u8, on: bool);
    fn is_bit(&self, bit: u8) -> bool;
}

impl SetBit for u16 {
    fn set_bit(&mut self, bit: u8, on: bool) {
        if on {
            *self = *self | 1 << bit;
        } else {
            *self = *self & !(1 << bit);
        }
    }

    fn is_bit(&self, bit: u8) -> bool {
        *self & (1 << bit) > 0
    }
}
