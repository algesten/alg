use crate::clock::Time;

pub struct WaveTableBuffer<W1: WaveTable, W2: WaveTable, const LEN: usize, const FQ: u32> {
    /// One wavetable.
    wt1: W1,

    /// The other wavetable.
    wt2: W2,

    /// Buffered output starting at `time`.
    buffer: [f32; LEN],

    /// Used when morphing between two sets of parameters.
    buffer_morph: [f32; LEN],

    /// Time at buffer[0], in sample rate denomination.
    time: Time<FQ>,

    /// Previous set of parameters used to create current buffer.
    params: WaveTableParams,

    /// Next set of parameters when doing advance_buffer().
    params_next: Option<WaveTableParams>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveTableParams {
    /// Offset between two wavetables.
    offset: f32,

    /// Frequency to play in Hz.
    freq: f32,
}

impl<W1: WaveTable, W2: WaveTable, const LEN: usize, const FQ: u32>
    WaveTableBuffer<W1, W2, LEN, FQ>
{
    pub fn new(wt1: W1, wt2: W2) -> Self {
        let buffer = [0_f32; LEN];
        let buffer_morph = [0_f32; LEN];

        // first call to advance_time below will move to 0.
        let time = Time::new(-1 * LEN as i64);

        let params = WaveTableParams {
            offset: 1.0,
            freq: 440.0,
        };

        let mut m = WaveTableBuffer {
            wt1,
            wt2,
            buffer,
            buffer_morph,
            time,
            params: params,
            params_next: None,
        };

        // fill first buffer (and move time to 0)
        m.advance_time();

        m
    }

    pub fn time(&self) -> &Time<FQ> {
        &self.time
    }

    pub fn buffer(&self) -> &[f32] {
        &self.buffer
    }

    pub fn set_params(&mut self, params: WaveTableParams) {
        self.params_next = Some(params);
    }

    pub fn advance_time(&mut self) {
        // Move time forwards.
        self.time.count += self.buffer.len() as i64;

        // Update current buffer with current parameters.
        fill_buf(
            &mut self.buffer,
            &self.wt1,
            &self.wt2,
            self.time,
            self.params,
        );

        if let Some(params_next) = self.params_next.take() {
            // Make morph buffer and update current towards it.
            fill_buf(
                &mut self.buffer_morph,
                &self.wt1,
                &self.wt2,
                self.time,
                params_next,
            );

            // weight between buffers moving from 0.0..1.0 over LEN
            let mut w = 0.0;

            // delta to move for each step.
            let dw = 1.0 / LEN as f32;

            for (b1, b2) in self.buffer.iter_mut().zip(self.buffer_morph.iter()) {
                *b1 = *b1 + (*b1 - *b2) * w;
                w += dw;
            }

            // update current set of parameters.
            self.params = params_next;
        }
    }
}

/// Fill buffer from set of parameters.
fn fill_buf<W1: WaveTable, W2: WaveTable, const FQ: u32>(
    buf: &mut [f32],
    wt1: &W1,
    wt2: &W2,
    mut sample_time: Time<FQ>,
    params: WaveTableParams,
) {
    for b in buf {
        let v1 = wt1.value_at(sample_time, params.freq);
        let v2 = wt2.value_at(sample_time, params.freq);

        // The weighted offset between the two tables.
        let n = v1 + (v2 - v1) * params.offset;

        // Set this in buffer.
        *b = n;

        // Move time
        sample_time.count += 1;
    }
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

        // fractional offset into the array.
        // we want the array period to be back-to-back and not interpolate
        // between the last element of the array and the first. hence
        // we subtract 1 from the len here.
        let offset_el = fract * (LEN - 1) as f32;

        // index into the array
        let n = offset_el as usize;

        // weight between two adjacent elements in the array.
        let w = offset_el - (n as f32);

        // n+1 is always ok, since offset_el is always (LEN - 1).
        let (el1, el2) = (self.elements[n], self.elements[n + 1]);

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
        assert_eq!(wt.value_at::<44_000>(Time::new(1), 440.0), 0.01);
        assert_eq!(wt.value_at::<44_000>(Time::new(2), 440.0), 0.02);
        assert_eq!(wt.value_at::<44_000>(Time::new(100), 440.0), 0.0);
        assert_eq!(wt.value_at::<44_000>(Time::new(101), 440.0), 0.01);
        assert_eq!(wt.value_at::<44_000>(Time::new(43_999), 440.0), 0.99);
        assert_eq!(wt.value_at::<44_000>(Time::new(44_000), 440.0), 0.0);
    }
}
