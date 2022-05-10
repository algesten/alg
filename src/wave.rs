use crate::clock::Time;

pub struct WaveTableBuffer<W1: WaveTable, W2: WaveTable, const LEN: usize, const FQ: u32> {
    /// One wavetable.
    wt1: W1,

    /// The other wavetable.
    wt2: W2,

    /// Current offset in wt1,
    acc1: Accumulator,

    /// Current offset in wt2,
    acc2: Accumulator,

    /// Buffered output starting at `time`.
    buffer: [f32; LEN],

    /// Used when morphing between two sets of parameters.
    buffer_morph: [f32; LEN],

    /// Previous set of parameters used to create current buffer.
    params: WaveTableParams,

    /// Next set of parameters when doing advance_buffer().
    params_next: Option<WaveTableParams>,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct WaveTableParams {
    /// Offset between two wavetables.
    pub offset: f32,

    /// Frequency to play in Hz.
    pub freq: f32,
}

impl<W1: WaveTable, W2: WaveTable, const LEN: usize, const FQ: u32>
    WaveTableBuffer<W1, W2, LEN, FQ>
{
    pub fn new(wt1: W1, wt2: W2) -> Self {
        let buffer = [0_f32; LEN];
        let buffer_morph = [0_f32; LEN];

        let params = WaveTableParams {
            offset: 0.0,
            freq: 440.0,
        };

        WaveTableBuffer {
            wt1,
            wt2,
            acc1: Accumulator(0.0),
            acc2: Accumulator(0.0),
            buffer,
            buffer_morph,
            params: params,
            params_next: None,
        }
    }

    pub fn buffer(&self) -> &[f32] {
        &self.buffer
    }

    pub fn params(&self) -> &WaveTableParams {
        &self.params
    }

    pub fn set_params(&mut self, params: WaveTableParams) {
        self.params_next = Some(params);
    }

    pub fn advance_time(&mut self) {
        let dt: Time<FQ> = Time::new(1);

        // Prefer next parameter frequency
        let freq = self.params_next.unwrap_or(self.params).freq;

        let (acc1, acc2) = fill_buf(
            &self.wt1,
            &self.wt2,
            self.acc1,
            self.acc2,
            dt,
            freq,
            &mut self.buffer,
            self.params.offset,
        );

        if let Some(next) = self.params_next {
            let _ = fill_buf(
                &self.wt1,
                &self.wt2,
                self.acc1,
                self.acc2,
                dt,
                freq,
                &mut self.buffer_morph,
                next.offset,
            );

            // weight between buffers moving from 0.0..1.0 over LEN
            let mut w = 0.0;

            // delta to move for each step.
            let dw = 1.0 / LEN as f32;

            for (b1, b2) in self.buffer.iter_mut().zip(self.buffer_morph.iter()) {
                *b1 = *b1 + (*b2 - *b1) * w;
                w += dw;
            }

            // update current set of parameters.
            self.params = next;
        }

        // remember accumulator for next advance_time() call
        self.acc1 = acc1;
        self.acc2 = acc2;
    }
}

#[inline(always)]
fn fill_buf<W1: WaveTable, W2: WaveTable, const FQ: u32>(
    wt1: &W1,
    wt2: &W2,
    acc1: Accumulator,
    acc2: Accumulator,
    dt: Time<FQ>,
    freq: f32,
    buf: &mut [f32],
    offset: f32,
) -> (Accumulator, Accumulator) {
    // First fill buffer with value of wt1, offset 0.0
    let acc1 = wt1.fill_buf(acc1, dt, freq, buf, 0.0);

    // Second add to buffer the offset from wt2
    let acc2 = wt2.fill_buf(acc2, dt, freq, buf, offset);

    (acc1, acc2)
}

/// Abstraction over a wavetable.
pub trait WaveTable {
    fn fill_buf<const FQ: u32>(
        &self,
        acc: Accumulator,
        dt: Time<FQ>,
        freq: f32,
        buf: &mut [f32],
        offset: f32,
    ) -> Accumulator;
}

#[derive(Debug, Clone, Copy)]
pub struct Accumulator(pub f32);

pub struct ArrayWaveTable<const LEN: usize> {
    elements: [f32; LEN],
}

impl<const LEN: usize> ArrayWaveTable<LEN> {
    pub fn new(elements: [f32; LEN]) -> Self {
        ArrayWaveTable { elements }
    }
}

impl<const LEN: usize> WaveTable for ArrayWaveTable<LEN> {
    #[inline(always)]
    fn fill_buf<const FQ: u32>(
        &self,
        acc: Accumulator,
        dt: Time<FQ>,
        freq: f32,
        buf: &mut [f32],
        offset: f32,
    ) -> Accumulator {
        // LEN - 1, because we don't want to "overshoot" the last element.
        let len = (LEN - 1) as f32;

        // "distance" in fractional index that dt represents.
        // NB. dt.count is typically 1, so "as f32" is fine despite it being an i64
        let dp = (dt.count as f32 * freq * len) / FQ as f32;

        // Value we want at dt "distance" from start.
        let mut offset_el = acc.0;

        for b in buf {
            offset_el += dp;

            // Wrap around the end
            while offset_el > len {
                offset_el -= len;
            }

            // Index into array
            let n = offset_el as usize;

            // weight between two adjacent elements in the array.
            let w = offset_el - (n as f32);

            // n+1 is always ok, since offset_el is always (LEN - 1).
            let (el1, el2) = (self.elements[n], self.elements[n + 1]);

            // weighted value between elements
            let value = el1 + (el2 - el1) * w;

            if offset == 0.0 {
                *b = value;
            } else {
                // weight between existing and incoming value.
                *b = *b + (value - *b) * offset;
            }
        }

        Accumulator(offset_el)
    }
}

pub enum BasicWavetable {
    Saw,
    Square,
    Sine,
    Triangle,
}

impl WaveTable for BasicWavetable {
    #[inline(always)]
    fn fill_buf<const FQ: u32>(
        &self,
        acc: Accumulator,
        dt: Time<FQ>,
        freq: f32,
        buf: &mut [f32],
        offset: f32,
    ) -> Accumulator {
        // NB. dt.count is typically 1, so "as f32" is fine despite it being an i64
        let dp = (dt.count as f32 * freq) / FQ as f32;

        // Fractional offset for the value wanted
        let mut fract = acc.0;

        for b in buf {
            fract += dp;

            while fract > 1.0 {
                fract -= 1.0;
            }

            let value = match self {
                BasicWavetable::Saw => {
                    //   /|
                    //  / |
                    // -  |
                    //    | /
                    //    |/

                    let min_step = 1.0 / FQ as f32;

                    // start at 0.0
                    if fract < min_step {
                        0.0
                    } else if fract <= 0.5 {
                        fract * 2.0
                    } else {
                        -1.0 + (fract - 0.5) * 2.0
                    }
                }
                BasicWavetable::Square => {
                    // ---|
                    //    |
                    //    |
                    //    |
                    //    |---
                    if fract <= 0.5 {
                        1.0
                    } else {
                        -1.0
                    }
                }
                BasicWavetable::Sine => {
                    let deg = fract * u32::MAX as f32;
                    crate::geom::sin(deg as u32) as f32 / 32768.0
                }
                BasicWavetable::Triangle => {
                    let deg = fract * u32::MAX as f32;
                    crate::geom::tri(deg as u32) as f32 / 32768.0
                }
            };

            if offset == 0.0 {
                *b = value;
            } else {
                // weight between existing and incoming value.
                *b = *b + (value - *b) * offset;
            }
        }

        Accumulator(fract)
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_wt_saw() {
        let mut buf = [0.0; 16];

        let wt = BasicWavetable::Saw;

        wt.fill_buf(Accumulator(0.0), Time::<64>::new(1), 440.0, &mut buf, 0.0);

        assert_eq!(
            buf,
            [
                -0.25, -0.5, -0.75, 1.0, 0.75, 0.5, 0.25, 0.0, -0.25, -0.5, -0.75, 1.0, 0.75, 0.5,
                0.25, 0.0
            ]
        );
    }

    #[test]
    fn test_wt_square() {
        let mut buf = [0.0; 16];

        let wt = BasicWavetable::Square;

        wt.fill_buf(Accumulator(0.0), Time::<64>::new(1), 440.0, &mut buf, 0.0);

        assert_eq!(
            buf,
            [
                -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0, -1.0, -1.0, -1.0, -1.0, 1.0, 1.0, 1.0, 1.0,
                -1.0
            ]
        );
    }

    #[test]
    fn test_wt_sine() {
        let mut buf = [0.0; 16];

        let wt = BasicWavetable::Sine;

        wt.fill_buf(Accumulator(0.0), Time::<64>::new(1), 440.0, &mut buf, 0.0);

        assert_eq!(
            buf,
            [
                -0.70706177,
                -0.9999695,
                -0.7070923,
                0.0,
                0.70706177,
                0.9999695,
                0.7070923,
                0.0,
                -0.70706177,
                -0.9999695,
                -0.7070923,
                0.0,
                0.70706177,
                0.9999695,
                0.7070923,
                0.0
            ]
        );
    }

    #[test]
    fn test_wt_tri() {
        let mut buf = [0.0; 16];

        let wt = BasicWavetable::Triangle;

        wt.fill_buf(Accumulator(0.0), Time::<64>::new(1), 440.0, &mut buf, 0.0);

        assert_eq!(
            buf,
            [
                -0.5,
                -1.0,
                -0.5,
                0.0,
                0.5,
                0.9999695,
                0.5,
                -3.0517578e-5,
                -0.5,
                -1.0,
                -0.5,
                0.0,
                0.5,
                0.9999695,
                0.5,
                -3.0517578e-5
            ]
        );
    }

    // #[test]
    // fn test_wt_buf() {
    //     let wt1 = BasicWavetable::Saw;
    //     let wt2 = BasicWavetable::Square;

    //     let mut wt_buf = WaveTableBuffer::<_, _, 256, 44_000>::new(wt1, wt2);

    //     let mut params = *wt_buf.params();
    //     params.freq = 439.0;

    //     wt_buf.set_params(params);

    //     wt_buf.advance_time();

    //     assert_eq!(wt_buf.buffer(), &[]);
    // }
}
