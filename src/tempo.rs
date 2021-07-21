use crate::clock::Time;

/// Simple tempo detection.
///
/// Saves measured beat (clock pulse) intervals into an array. The assumption is that
/// a potential swing would be on every other beat (offbeat), which means we can make linear
/// regression either over the odd or the even positions in the array to predict the next
/// interval.
#[derive(Default, Debug, Clone, PartialEq, Eq)]
pub struct Tempo<const CLK: u32> {
    intervals: [Option<Time<CLK>>; 6],
    next: usize,
    predicted: i64,
}

impl<const CLK: u32> Tempo<CLK> {
    pub fn new() -> Self {
        Tempo {
            ..Default::default()
        }
    }

    /// Maybe offsets the pointer for the next update depending on where it is now.
    /// Generally we keep the even/odd ticks together and it doesn't matter which is
    /// the _real_ even or odd.
    ///
    /// However if a reset hits us when the next is supposed to be odd, it means
    /// the clock signal is deliberately "cutting short" full patterns. In this case
    /// we shift +1 to stay stable on the even/odd.
    pub fn reset(&mut self) {
        if self.next % 2 == 1 {
            self.next += 1;
        }
    }

    /// Update with a new interval, and get back the predicted next interval.
    pub fn predict(&mut self, interval: Time<CLK>) -> Time<CLK> {
        if self.next >= self.intervals.len() {
            self.next = self.next % 2;
        }

        if self.predicted > 0 && interval.count() > 10 * self.predicted {
            // The interval is much longer than predicted. Assume there was a stop
            // in the clock pulse, which happens when the user stops the sequencer
            // or similar.

            // We do a little switch-a-roo hoping we get back to the predicted speed.
            // If this is indeed the new speed, we will accept it on next clock tick.
            let guessed = Time {
                count: self.predicted,
            };

            // Use the one we guess instead of incoming.
            self.intervals[self.next] = Some(guessed);
            self.next += 1;

            // By saving incoming here, we will accept the next incoming if it has a similar interval.
            self.predicted = interval.count();

            return guessed;
        }

        self.intervals[self.next] = Some(interval);
        self.next += 1;

        // Typical swing is every other offbeat. That means we get two series
        // of numbers to predict from, either the even or the odd where the
        // intervals should be equidistant.
        //
        // --*---o-*---o-*---o-*--->
        //   ^     ^     ^     ^   // equidistant
        //       ^     ^     ^     // equidistant

        let (b0, b1) = self.linear_regress();

        let predicted = b0 + b1 * (self.next as i64);
        self.predicted = predicted;

        Time {
            count: if predicted == 0 {
                interval.count()
            } else {
                predicted
            },
        }
    }

    fn series<'a>(&'a self) -> IntervalIterator<'a, CLK> {
        IntervalIterator {
            tempo: &self,
            next: self.next % 2,
        }
    }

    fn linear_regress(&self) -> (i64, i64) {
        let mut count = 0;
        let mut sum: i64 = 0;

        for y in self.series() {
            count += 1;
            sum += y;
        }

        if sum == 0 {
            return (0, 0);
        }

        let mean_y = sum / count;
        let mean_x: i64 = ((count * (count + 1)) / 2) / sum;

        let mut variance = 0;
        let mut covariance = 0;

        for (x, y) in self.series().enumerate() {
            let x = x as i64;
            variance += (x - mean_x).pow(2);
            covariance += (x - mean_x) * (y - mean_y);
        }

        if variance == 0 {
            return (0, 0);
        }

        let b1 = covariance / variance;
        let b0 = mean_y - b1 * mean_x;

        (b0, b1)
    }
}

struct IntervalIterator<'a, const CLK: u32> {
    tempo: &'a Tempo<CLK>,
    next: usize,
}

impl<'a, const CLK: u32> Iterator for IntervalIterator<'a, CLK> {
    type Item = i64;

    fn next(&mut self) -> Option<Self::Item> {
        if self.next < self.tempo.intervals.len() {
            let item = &self.tempo.intervals[self.next];
            self.next += 2;
            item.map(|t| t.count())
        } else {
            None
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_predict_straight() {
        let mut t = Tempo::<1000>::new();

        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
        assert_eq!(t.predict(Time::from_secs(1)).count(), 1000);
    }

    #[test]
    fn test_predict_swing() {
        let mut t = Tempo::<1000>::new();

        // same as update with
        assert_eq!(t.predict(Time::from_secs(3)).count(), 3000);
        assert_eq!(t.predict(Time::from_secs(2)).count(), 2000);
        assert_eq!(t.predict(Time::from_secs(3)).count(), 3000);

        // now the prediction kicks in
        assert_eq!(t.predict(Time::from_secs(2)).count(), 3000);
        assert_eq!(t.predict(Time::from_secs(3)).count(), 2000);
        assert_eq!(t.predict(Time::from_secs(2)).count(), 3000);
        assert_eq!(t.predict(Time::from_secs(3)).count(), 2000);
        assert_eq!(t.predict(Time::from_secs(2)).count(), 3000);
        assert_eq!(t.predict(Time::from_secs(3)).count(), 2000);
        assert_eq!(t.predict(Time::from_secs(2)).count(), 3000);
        assert_eq!(t.predict(Time::from_secs(3)).count(), 2000);
        assert_eq!(t.predict(Time::from_secs(2)).count(), 3000);
    }
}
