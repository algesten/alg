use crate::clock::Time;

/// Morph between two wavetables.
pub fn wavetable_morph<W1: WaveTable, W2: WaveTable, const FQ: u32>(
    wt1: W1,
    wt2: W2,
    sample_time: Time<FQ>,
    freq: f32,
    weight: f32,
) -> f32 {
    let v1 = wt1.value_at(sample_time, freq);
    let v2 = wt2.value_at(sample_time, freq);

    v1 + (v2 - v1) * weight
}

/// Abstraction over a wavetable.
pub trait WaveTable {
    /// Sample time is the current time in some sample rate. I.e. `Time<48_000>`
    /// or `Time<96_000>`. The wave table is considered one entire period,
    /// so a frequency of `440.0` means we should repeat the wave table 440
    /// times during one full sample time 0-FQ.
    fn value_at<const FQ: u32>(&self, sample_time: Time<FQ>, freq: f32) -> f32;
}

pub struct ArrayWaveTable<const LEN: usize> {
    elements: [f32; LEN],
}

impl<const LEN: usize> ArrayWaveTable<LEN> {
    pub fn new(elements: [f32; LEN]) -> Self {
        ArrayWaveTable { elements }
    }
}

impl<const LEN: usize> WaveTable for ArrayWaveTable<LEN> {
    fn value_at<const FQ: u32>(&self, time: Time<FQ>, freq: f32) -> f32 {
        let periods_fract = (time.count as f64 * freq as f64) / FQ as f64;

        // full periods done at time.count/FQ
        let periods_whole = periods_fract as usize as f64;

        // fractional part of current period.
        let fract = (periods_fract - periods_whole) as f32;

        // fractional offset into the array
        let offset_el = fract * LEN as f32;

        // index into the array
        let n = offset_el as usize;

        // weight between two adjacent elements in the array.
        let w = offset_el - (n as f32);

        let (el1, el2) = if n == LEN - 1 {
            (self.elements[LEN - 1], self.elements[0])
        } else {
            (self.elements[n], self.elements[n + 1])
        };

        // weighted value between elements
        el1 + (el2 - el1) * w
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn array_value_at_table2() {
        let wt = ArrayWaveTable::new([0.0, 1.0]);

        // time = tn/td
        // offset = (tn/td) * freq
        // e.g 0.5Hz at step 1 in 44kHz is: 1/44_000 * 0.5

        // weighted value:
        // 0.0 + 1/44_000 * 440 * (1.0 - 0.0)

        assert_eq!(wt.value_at::<44_000>(Time::new(0), 440.0), 0.0);
        assert_eq!(wt.value_at::<44_000>(Time::new(1), 440.0), 0.02);
        assert_eq!(wt.value_at::<44_000>(Time::new(2), 440.0), 0.04);
        assert_eq!(wt.value_at::<44_000>(Time::new(100), 440.0), 0.0);
        assert_eq!(wt.value_at::<44_000>(Time::new(101), 440.0), 0.02);
        assert_eq!(wt.value_at::<44_000>(Time::new(44_000), 440.0), 0.0);
    }

    #[test]
    fn array_value_at_table1() {
        let wt = ArrayWaveTable::new([0.0]);

        assert_eq!(wt.value_at::<44_000>(Time::new(0), 440.0), 0.0);
        assert_eq!(wt.value_at::<44_000>(Time::new(1), 440.0), 0.0);
        assert_eq!(wt.value_at::<44_000>(Time::new(2), 440.0), 0.0);
        assert_eq!(wt.value_at::<44_000>(Time::new(100), 440.0), 0.0);
        assert_eq!(wt.value_at::<44_000>(Time::new(101), 440.0), 0.0);
        assert_eq!(wt.value_at::<44_000>(Time::new(44_000), 440.0), 0.0);
    }

    #[test]
    fn array_value_at_triangle() {
        let wt = ArrayWaveTable::new([0.0, 1.0, 0.0, -1.0]);

        assert_eq!(wt.value_at::<44_000>(Time::new(0), 440.0), 0.0);
        assert_eq!(wt.value_at::<44_000>(Time::new(25), 440.0), 1.0);
        assert_eq!(wt.value_at::<44_000>(Time::new(50), 440.0), 0.0);
        assert_eq!(wt.value_at::<44_000>(Time::new(75), 440.0), -1.0);
        assert_eq!(wt.value_at::<44_000>(Time::new(44_000), 440.0), 0.0);
    }
}
