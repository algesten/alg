use core::array;

use micromath::F32Ext;

use crate::audio::householder::transform_householder;

use super::delay::Delay;
use super::AudioNode;

pub struct MixedFeedback<D, const C: usize> {
    /// Delay per channel.
    ///
    /// Each delay is sized to max hold sample_rate * max_time_in_sec.
    delays: [D; C],

    /// The amount of gain decay for each feedback.
    decay: f32,
}

impl<D: Delay, const C: usize> MixedFeedback<D, C> {
    pub fn new(sample_rate: usize, delay_secs: f32, decay: f32) -> Self {
        let sample_count = sample_rate as f32 * delay_secs;

        let mut delays: [D; C] = array::from_fn(|_| D::default());

        for i in 0..C {
            let r = i as f32 / C as f32;
            let delay_size = (2.0.powf(r) * sample_count) as usize;
            delays[i].set_sample_count(delay_size + 1);
        }

        Self { delays, decay }
    }
}

impl<D: Delay, const C: usize> AudioNode<C> for MixedFeedback<D, C> {
    fn process(&mut self, input: [f32; C]) -> [f32; C] {
        let mut delayed: [_; C] = array::from_fn(|i| self.delays[i].read());

        // Mix a bit of all channels into all channels.
        let mixed = delayed.clone();
        transform_householder(&mut delayed);

        for i in 0..C {
            // Mix new value with old.
            let sum = input[i] + mixed[i] * self.decay;
            self.delays[i].write(sum);
        }

        mixed
    }
}
