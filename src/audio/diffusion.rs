use core::array;

use crate::rnd::Rnd;

use super::delay::Delay;
use super::hadamard::transform_hadamard;
use super::AudioNode;

/// A diffusion step
///
/// The step is made out of having multiple channels being mixed together in
/// a hadamard mixer. Each channel might also flip polarity.
struct DiffusionStep<D, const C: usize> {
    /// The delays for the channels
    delays: [D; C],

    /// Whether to flip polarity for the channel
    flip_polarity: [bool; C],
}

impl<D: Delay, const C: usize> DiffusionStep<D, C> {
    /// Creates a new diffusion step
    ///
    /// The delay range is spread among the channels, so that each channel gets
    /// a random delay from its own "bucket".
    ///
    /// ```text
    /// │    │    │    │    │
    /// └────┴────┴────┴────┘
    ///    a    b    c    d
    ///
    ///      delay_range
    /// ```
    pub fn new(sample_rate: usize, delay_range_secs: f32, rnd: &mut Rnd) -> Self {
        let delay_samples = delay_range_secs * sample_rate as f32;

        let delays: [D; C] = array::from_fn(|i| {
            let lo = (delay_samples * i as f32) / C as f32;
            let hi = (delay_samples * (i as f32 + 1.0)) / C as f32;

            let n = rnd.next();

            let range = hi - lo;
            let delay_size = (range * (n as f32 / u32::MAX as f32)) as usize;

            let mut d = D::default();
            d.set_sample_count(delay_size + 1);

            d
        });

        let flip_polarity: [bool; C] = array::from_fn(|_| rnd.next() > u32::MAX / 2);

        Self {
            delays,
            flip_polarity,
        }
    }
}

impl<D: Delay, const C: usize> AudioNode<C> for DiffusionStep<D, C> {
    fn process(&mut self, input: [f32; C]) -> [f32; C] {
        let mut mixed = array::from_fn(|i| {
            self.delays[i].write(input[i]);
            self.delays[i].read()
        });

        // Mix with hadamard matrix to retain energy but still
        // leak all channels into all channel
        transform_hadamard(&mut mixed);

        for i in 0..C {
            if self.flip_polarity[i] {
                mixed[i] *= -1.0;
            }
        }

        mixed
    }
}

pub struct Diffuser<D, const C: usize, const S: usize> {
    /// S number of steps over C channels.
    steps: [DiffusionStep<D, C>; S],
}

impl<D: Delay, const C: usize, const S: usize> Diffuser<D, C, S> {
    pub fn new(sample_rate: usize, mut seconds: f32, rnd: &mut Rnd) -> Self {
        Self {
            steps: array::from_fn(|_| {
                seconds *= 0.5;
                DiffusionStep::new(sample_rate, seconds, rnd)
            }),
        }
    }
}

impl<D: Delay, const C: usize, const S: usize> AudioNode<C> for Diffuser<D, C, S> {
    fn process(&mut self, mut input: [f32; C]) -> [f32; C] {
        for step in &mut self.steps {
            input = step.process(input);
        }
        input
    }
}
