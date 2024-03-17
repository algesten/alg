//!
//!
//! * C - channels
//! * N - buffer length
//! * S - sample rate
//! * M - milliseconds
//!

mod delay;
mod diffusion;
mod feedback;
mod hadamard;
mod householder;
mod reverb;

pub use reverb::BasicReverb;

pub trait AudioNode<const C: usize> {
    fn process(&mut self, input: [f32; C]) -> [f32; C];
}
