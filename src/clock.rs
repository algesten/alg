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

    /// Get the current time. This is reasonably called _after_ `tick()`.
    pub fn now(&self) -> Time<FQ> {
        Time {
            upper: self.upper,
            lower: self.lower,
        }
    }
}

/// A time representation as produced by `Clock::now()`.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Time<const FQ: u32> {
    upper: u32,
    lower: u32,
}

impl<const FQ: u32> Time<FQ> {
    pub fn new(upper: u32, lower: u32) -> Self {
        Time { upper, lower }
    }

    /// Time rounded off to full seconds. This is calculated using `FQ` clock frequency,
    /// and probably isn't very exact because the oscillating crystals driving a CPU are
    /// never exactly what they say they are.
    pub fn seconds(&self) -> u32 {
        let mut full_seconds = self.upper * (u32::MAX / FQ);

        // We want this which would overflow: (self.upper * u32::MAX) % FQ

        // Distributive properties of modulo
        // ( a + b ) % c = ( ( a % c ) + ( b % c ) ) % c
        // ( a * b ) % c = ( ( a % c ) * ( b % c ) ) % c
        // ( a – b ) % c = ( ( a % c ) – ( b % c ) ) % c
        // ( a / b ) % c = ( ( a % c ) / ( b % c ) ) % c

        let upper_rest = ((self.upper % FQ) * (u32::MAX % FQ)) % FQ;

        let (over, overflow) = upper_rest.overflowing_add(self.lower);

        if overflow {
            full_seconds += u32::MAX / FQ;
        }

        full_seconds += over / FQ;

        full_seconds
    }

    /// Fractional seconds in milliseconds. I.e. if time is 500E6 and clock frequency is 600E6,
    /// this function returns 833.
    ///
    /// NB panics if `FQ` is less than 1000.
    pub fn subsec_millis(&self) -> u32 {
        assert!(FQ >= 1, "subsec_millis requires FQ >= 1000");

        let upper_rest = ((self.upper % FQ) * (u32::MAX % FQ)) % FQ;

        let over = upper_rest.wrapping_add(self.lower);

        let lower_rest = over % FQ;

        lower_rest / (FQ / 1000)
    }

    /// Fractional seconds in nanoseconds. I.e. if time is 500E6 and clock frequency is 600E6,
    /// this function returns 833_333.
    pub fn subsec_nanos(&self) -> u32 {
        let upper_rest = ((self.upper % FQ) * (u32::MAX % FQ)) % FQ;

        let over = upper_rest.wrapping_add(self.lower);

        let lower_rest = over % FQ;

        let g = FQ.gcd(1000_000_000);
        let nom = FQ / g;
        let denom = 1000_000_000 / g;

        let (m, overflow) = lower_rest.overflowing_mul(denom);
        if overflow {
            // this will lose precision since we're scaling down then up.
            (lower_rest / nom) * denom
        } else {
            m / nom
        }
    }
}

impl<const FQ: u32> core::fmt::Debug for Time<FQ> {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{}.{}", self.seconds(), self.subsec_millis())
    }
}

impl<const FQ: u32> core::ops::Add for Time<FQ> {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        let mut upper = self.upper + rhs.upper;

        let (lower, over) = self.lower.overflowing_add(rhs.lower);

        if over {
            upper += 1;
        }

        Time { upper, lower }
    }
}

impl<const FQ: u32> core::ops::Sub for Time<FQ> {
    type Output = Self;

    fn sub(self, rhs: Self) -> Self::Output {
        let mut upper = self.upper - rhs.upper;

        let (lower, over) = self.lower.overflowing_sub(rhs.lower);

        if over {
            upper -= 1;
        }

        Time { upper, lower }
    }
}

impl<const FQ: u32> core::ops::AddAssign for Time<FQ> {
    fn add_assign(&mut self, rhs: Self) {
        let x = *self + rhs;
        self.upper = x.upper;
        self.lower = x.lower;
    }
}

impl<const FQ: u32> core::ops::SubAssign for Time<FQ> {
    fn sub_assign(&mut self, rhs: Self) {
        let x = *self - rhs;
        self.upper = x.upper;
        self.lower = x.lower;
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn time_seconds_millis_nanos() {
        let t: Time<600_000_000> = Time::new(0, 500_000_000);
        assert_eq!(t.seconds(), 0);
        assert_eq!(t.subsec_millis(), 833);
        assert_eq!(t.subsec_nanos(), 833_333_333);

        let t: Time<600_000_000> = Time::new(1, 700_000_000);
        assert_eq!(t.seconds(), 8);
        assert_eq!(t.subsec_millis(), 324);
        assert_eq!(t.subsec_nanos(), 324_945_491);
    }

    #[test]
    fn time_add() {
        let t1: Time<600_000_000> = Time::new(0, u32::MAX);
        let t2: Time<600_000_000> = Time::new(0, 1);

        let t3 = t1 + t2;

        assert_eq!(t3.upper, 1);
        assert_eq!(t3.lower, 0);

        let mut t4 = Time::new(0, 1);
        t4 += Time::<600_000_000>::new(0, 1);

        assert_eq!(t4.upper, 0);
        assert_eq!(t4.lower, 2);
    }

    #[test]
    fn time_sub() {
        let t1: Time<600_000_000> = Time::new(1, 0);
        let t2: Time<600_000_000> = Time::new(0, 1);

        let t3 = t1 - t2;

        assert_eq!(t3.upper, 0);
        assert_eq!(t3.lower, u32::MAX);

        let mut t4 = Time::new(0, 1);
        t4 -= Time::<600_000_000>::new(0, 1);

        assert_eq!(t4.upper, 0);
        assert_eq!(t4.lower, 0);
    }
}
