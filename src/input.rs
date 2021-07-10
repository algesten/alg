use core::ops::BitAnd;

use crate::clock::Time;

/// An input that produces deltas. Think rotary encoder that will give us
/// -1, 0 or 1.
pub trait DeltaInput<const CLK: u32> {
    /// Polled every main-loop run. Returns 0 as long as there isn't a value.
    fn tick(&mut self, now: Time<CLK>) -> i8;
}

/// An input that is either high or low.
pub trait DigitalInput<const CLK: u32> {
    /// Polled every main-loop run. The time returned does not have to be `now`, it can be
    /// some time in the past when having debounce logic to ensure the state really is
    /// high or low.
    fn tick(&mut self, now: Time<CLK>) -> HiLo<CLK>;
}

/// Digital input over reading a pointer to a shared number.
pub struct BitmaskDigitalInput<W> {
    word: *const W,
    mask: W,
    zero: W,
}

impl<W> BitmaskDigitalInput<W>
where
    W: BitAnd<Output = W> + Default + Copy + PartialOrd,
{
    pub fn new(word: *const W, mask: W) -> Self {
        BitmaskDigitalInput {
            word,
            mask,
            zero: W::default(),
        }
    }
}

impl<W, const CLK: u32> DigitalInput<CLK> for BitmaskDigitalInput<W>
where
    W: BitAnd<Output = W> + Default + Copy + PartialOrd,
{
    fn tick(&mut self, now: Time<CLK>) -> HiLo<CLK> {
        let hi = unsafe { *self.word }.bitand(self.mask) > self.zero;
        if hi {
            HiLo::Hi(now)
        } else {
            HiLo::Lo(now)
        }
    }
}

/// Representation of a hi or a low (on/off) state. The state
/// is always tied to a time when the state flipped to high or low.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum HiLo<const CLK: u32> {
    Hi(Time<CLK>),
    Lo(Time<CLK>),
}

impl<const CLK: u32> HiLo<CLK> {
    /// Whether this state is hi (true) or low.
    pub fn is_set(&self) -> bool {
        if let Self::Hi(_) = self {
            true
        } else {
            false
        }
    }

    /// The time the state changed to whatever the value is right now.
    pub fn since(&self) -> &Time<CLK> {
        match self {
            HiLo::Hi(t) => t,
            HiLo::Lo(t) => t,
        }
    }
}

/// Trait for inputs that switches between low/high now and then. Used when we want to
/// detect the "edge" of switching from high to low or vice versa.
pub trait EdgeInput<const CLK: u32> {
    /// Polled every main-loop run. Returns None as long as there isn't a change in the input.
    fn tick(&mut self, now: Time<CLK>) -> Option<Edge<CLK>>;
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Edge<const CLK: u32> {
    Rising(Time<CLK>),
    Falling(Time<CLK>),
}

/// A deduping over DigitalInput that gives an indication of when something changes.
pub struct DigitalEdgeInput<I, const CLK: u32> {
    input: I,
    value: HiLo<CLK>,
}

impl<I, const CLK: u32> DigitalEdgeInput<I, CLK>
where
    I: DigitalInput<CLK>,
{
    pub fn new(input: I, start_high: bool) -> Self {
        DigitalEdgeInput {
            input,
            value: if start_high {
                HiLo::Hi(Time::ZERO)
            } else {
                HiLo::Lo(Time::ZERO)
            },
        }
    }
}

impl<I, const CLK: u32> EdgeInput<CLK> for DigitalEdgeInput<I, CLK>
where
    I: DigitalInput<CLK>,
{
    fn tick(&mut self, now: Time<CLK>) -> Option<Edge<CLK>> {
        let x = self.input.tick(now);
        if x != self.value {
            Some(match x {
                HiLo::Hi(t) => Edge::Rising(t),
                HiLo::Lo(t) => Edge::Falling(t),
            })
        } else {
            None
        }
    }
}

// Boiler plate impls for ()

impl<const CLK: u32> DeltaInput<CLK> for () {
    fn tick(&mut self, _now: Time<CLK>) -> i8 {
        0
    }
}

impl<const CLK: u32> DigitalInput<CLK> for () {
    fn tick(&mut self, _now: Time<CLK>) -> HiLo<CLK> {
        HiLo::Lo(Time::ZERO)
    }
}

impl<const CLK: u32> EdgeInput<CLK> for () {
    fn tick(&mut self, _now: Time<CLK>) -> Option<Edge<CLK>> {
        None
    }
}
