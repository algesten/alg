use core::ops::BitAnd;

use crate::clock::Time;

/// An input that produces deltas. Think rotary encoder that will give us
/// -1, 0 or 1.
pub trait DeltaInput<const CLK: u32> {
    /// Polled when needed run. Returns 0 as long as there isn't a value.
    fn tick(&mut self, now: Time<CLK>) -> i8;
}

/// An input that is either high or low.
pub trait DigitalInput<const CLK: u32>: Sized {
    /// Polled when needed.
    fn tick(&mut self, now: Time<CLK>) -> HiLo<CLK>;

    /// Wrap this source input in a debouncer.
    fn debounce(self) -> DebounceDigitalInput<Self, CLK> {
        DebounceDigitalInput::new(self)
    }

    /// Turns this input into an edge sensing input.
    fn edge(self) -> DigitalEdgeInput<Self, CLK> {
        DigitalEdgeInput::new(self)
    }
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
#[derive(Debug, Clone, Copy)]
pub enum HiLo<const CLK: u32> {
    Hi(Time<CLK>),
    Lo(Time<CLK>),
}

impl<const CLK: u32> HiLo<CLK> {
    pub fn is_same_state(&self, other: &HiLo<CLK>) -> bool {
        match (self, other) {
            (HiLo::Hi(_), HiLo::Hi(_)) | (HiLo::Lo(_), HiLo::Lo(_)) => true,
            _ => false,
        }
    }

    pub fn time(&self) -> &Time<CLK> {
        match self {
            HiLo::Hi(v) => v,
            HiLo::Lo(v) => v,
        }
    }
}

impl<const CLK: u32> PartialEq for HiLo<CLK> {
    fn eq(&self, other: &Self) -> bool {
        match (self, other) {
            (HiLo::Hi(_), HiLo::Hi(_)) => true,
            (HiLo::Lo(_), HiLo::Lo(_)) => true,
            _ => false,
        }
    }
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
    /// Polled when needed. Returns None as long as there isn't a change in the input.
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
    pub fn new(mut input: I) -> Self {
        let value = input.tick(Time::ZERO);

        DigitalEdgeInput { input, value }
    }
}

impl<I, const CLK: u32> EdgeInput<CLK> for DigitalEdgeInput<I, CLK>
where
    I: DigitalInput<CLK>,
{
    fn tick(&mut self, now: Time<CLK>) -> Option<Edge<CLK>> {
        let x = self.input.tick(now);

        if x != self.value {
            trace!("EdgeInput trigger: {:?} -> {:?}", self.value, x);

            self.value = x;

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

/// Input that debounces the input to avoid unintentional double clicks.
pub struct DebounceDigitalInput<I, const CLK: u32> {
    input: I,
    value: HiLo<CLK>,
}
impl<I, const CLK: u32> DebounceDigitalInput<I, CLK>
where
    I: DigitalInput<CLK>,
{
    pub fn new(mut input: I) -> Self {
        let value = input.tick(Time::ZERO);
        DebounceDigitalInput { input, value }
    }
}

impl<I, const CLK: u32> DigitalInput<CLK> for DebounceDigitalInput<I, CLK>
where
    I: DigitalInput<CLK>,
{
    fn tick(&mut self, now: Time<CLK>) -> HiLo<CLK> {
        let value = self.input.tick(now);

        if !self.value.is_same_state(&value) {
            if now - *self.value.time() > Time::from_millis(1) {
                self.value = value;
            }
        }

        return self.value;
    }
}
