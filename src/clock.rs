//! CPU cycle based clock.

use gcd::Gcd;

/// Clock based on cpu cycles. This clock provides 64 bits of time using a sample function that
/// provides a 32 bit clock cycle number.
///
/// The `tick()` must be called often enough to guarantee the the cycle count doesnt loop twice.
///
/// Example: If the CPU speed is 600Mhz, we get a 32-bit cycle of 2.pow(32) / 600E6 ≈ 7.16 seconds.
/// That means we must call `tick` more often than every 7.16 seconds.
#[derive(Debug)]
pub struct Clock<S, const FQ: u32> {
    sample_fn: S,
    upper: u32,
    lower: u32,
}

impl<S, const FQ: u32> Clock<S, FQ>
where
    S: Fn() -> u32,
{
    /// Creates a new clock instance providing the sample function needed to read the current
    /// cpu clock cycle and the frequency of the CPU.
    pub fn new(sample_fn: S) -> Self {
        let start = sample_fn();

        Clock {
            sample_fn,
            upper: 0,
            lower: start,
        }
    }

    /// Sample the current CPU clock and update the internal clock state. This must be done often
    /// enough that `sample_fn` doesn't risk looping twice.
    pub fn tick(&mut self) {
        const HALF: u32 = 2_u32.pow(31);

        let cur = (self.sample_fn)();

        if cur < HALF && self.lower > HALF {
            // we have looped around.
            self.upper += 1;
        }

        self.lower = cur;
    }

    /// Given a u32 cycle count, get Time using the "knowledge" of where the
    /// time is in this clock. I.e. if the time wraps around,
    /// we get a correct time for that.
    pub fn time_relative(&self, cycle_count: u32) -> Time<FQ> {
        const HALF: u32 = 2_u32.pow(31);

        let upper = if cycle_count < HALF && self.lower > HALF {
            // looped around.
            self.upper + 1
        } else {
            self.upper
        };

        Time {
            count: ((upper as i64) << 32) | (cycle_count as i64),
        }
    }

    /// Get the current time. This is reasonably called _after_ `tick()`.
    pub fn now(&self) -> Time<FQ> {
        Time {
            count: ((self.upper as i64) << 32) | (self.lower as i64),
        }
    }

    pub fn delay_nanos(&mut self, ns: u64) {
        self.tick();
        let start = self.now();
        let until = Time::from_nanos(ns as i64);
        loop {
            self.tick();
            if self.now() - start >= until {
                break;
            }
        }
    }
}

/// A time representation as produced by `Clock::now()`.
#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub struct Time<const FQ: u32> {
    pub count: i64,
}

impl<const FQ: u32> Time<FQ> {
    pub const ZERO: Time<FQ> = Time::new(0);

    /// Create a new instance of Time setting the count.
    ///
    /// This is probably not what you want to do.
    pub const fn new(count: i64) -> Self {
        Time { count }
    }

    /// Create a new instance converted from a number of seconds.
    pub const fn from_secs(secs: i64) -> Self {
        Time {
            count: secs * (FQ as i64),
        }
    }

    // Create a new instance converted from a number of milliseconds.
    pub const fn from_millis(millis: i64) -> Self {
        Time {
            count: (millis * FQ as i64) / 1000,
        }
    }

    // Create a new instance converted from a number of microseconds.
    pub const fn from_micros(micros: i64) -> Self {
        Time {
            count: (micros * FQ as i64) / 1000_000,
        }
    }

    // Create a new instance converted from a number of milliseconds.
    pub const fn from_nanos(nanos: i64) -> Self {
        Time {
            count: (nanos * FQ as i64) / 1000_000_000,
        }
    }

    /// Time rounded off to full seconds. This is calculated using `FQ` clock frequency,
    /// and probably isn't very exact because the oscillating crystals driving a CPU are
    /// never exactly what they say they are.
    pub fn seconds(&self) -> i64 {
        self.count / (FQ as i64)
    }

    /// Fractional seconds in milliseconds. I.e. if time is 500E6 and clock frequency is 600E6,
    /// this function returns 833.
    pub fn subsec_millis(&self) -> i64 {
        let rest = self.count % FQ as i64;
        rest / ((FQ as i64) / 1000)
    }

    /// Fractional seconds in microseconds. I.e. if time is 500E6 and clock frequency is 600E6,
    /// this function returns 833_333.
    pub fn subsec_micros(&self) -> i64 {
        let rest = self.count % FQ as i64;

        let g = (FQ as u64).gcd(1_000_000) as i64;
        let nom = (FQ as i64) / g;
        let denom = 1_000_000 / g;

        (rest * denom) / nom
    }

    /// Fractional seconds in nanoseconds. I.e. if time is 500E6 and clock frequency is 600E6,
    /// this function returns 833_333_333.
    pub fn subsec_nanos(&self) -> i64 {
        let rest = self.count % FQ as i64;

        let g = (FQ as u64).gcd(1_000_000_000) as i64;
        let nom = (FQ as i64) / g;
        let denom = 1_000_000_000 / g;

        (rest * denom) / nom
    }

    /// The number of clock cycles in this time instance.
    pub fn count(&self) -> i64 {
        self.count
    }
}

impl<const FQ: u32> core::fmt::Display for Time<FQ> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{:03}s", self.seconds(), self.subsec_millis())
    }
}

impl<const FQ: u32> core::fmt::Debug for Time<FQ> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "Time {{ {:08x} }}", self.count)
    }
}

impl<const FQ: u32> core::ops::Add for Time<FQ> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Time {
            count: self.count + rhs.count,
        }
    }
}

impl<const FQ: u32> core::ops::Sub for Time<FQ> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        Time {
            count: self.count - rhs.count,
        }
    }
}

impl<const FQ: u32> core::ops::AddAssign for Time<FQ> {
    fn add_assign(&mut self, rhs: Self) {
        self.count += rhs.count;
    }
}

impl<const FQ: u32> core::ops::SubAssign for Time<FQ> {
    fn sub_assign(&mut self, rhs: Self) {
        self.count -= rhs.count;
    }
}

impl<const FQ: u32> core::cmp::PartialOrd for Time<FQ> {
    fn partial_cmp(&self, other: &Self) -> Option<core::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl<const FQ: u32> core::cmp::Ord for Time<FQ> {
    fn cmp(&self, other: &Self) -> core::cmp::Ordering {
        self.count.cmp(&other.count)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn time_from_secs() {
        let t = Time::<600_000_000>::from_secs(10);
        assert_eq!(format!("{}", t), "10.000s");
        let t = Time::<600_000_000>::from_secs(6);
        assert_eq!(format!("{}", t), "6.000s");
        let t = Time::<600_000_000>::from_secs(0);
        assert_eq!(format!("{}", t), "0.000s");
        let t = Time::<600_000_000>::from_secs(49);
        assert_eq!(format!("{}", t), "49.000s");
    }

    #[test]
    fn time_seconds_millis_nanos() {
        let t: Time<600_000_000> = Time::new(500_000_000);
        assert_eq!(t.seconds(), 0);
        assert_eq!(t.subsec_millis(), 833);
        assert_eq!(t.subsec_nanos(), 833_333_333);

        let t: Time<600_000_000> = Time::new(700_000_000);
        assert_eq!(t.seconds(), 1);
        assert_eq!(t.subsec_millis(), 166);
        assert_eq!(t.subsec_nanos(), 166666666);
    }

    #[test]
    fn time_add() {
        let t1: Time<600_000_000> = Time::new(u32::MAX as i64);
        let t2: Time<600_000_000> = Time::new(1);

        let t3 = t1 + t2;

        assert_eq!(t3.count, 0x1_0000_0000);

        let mut t4 = Time::new(1);
        t4 += Time::<600_000_000>::new(1);

        assert_eq!(t4.count, 2);
    }

    #[test]
    fn time_sub() {
        let t1: Time<600_000_000> = Time::new(0);
        let t2: Time<600_000_000> = Time::new(1);

        let t3 = t1 - t2;

        assert_eq!(t3.count, -1);

        let mut t4 = Time::new(1);
        t4 -= Time::<600_000_000>::new(1);

        assert_eq!(t4.count, 0);
    }
}
