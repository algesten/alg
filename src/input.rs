use crate::clock::Time;

/// An input that produces deltas. Think rotary encoder that will give us
/// -1, 0 or 1.
pub trait DeltaInput<const CLK: u32> {
    /// Polled every main-loop run. Returns 0 as long as there isn't a value.
    fn tick(&mut self, now: Time<CLK>) -> i8;
}

impl<const CLK: u32> DeltaInput<CLK> for () {
    fn tick(&mut self, _now: Time<CLK>) -> i8 {
        0
    }
}
