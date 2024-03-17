#[allow(unused_imports)]
use micromath::F32Ext;

use crate::rnd::Rnd;

use super::delay::Delay;
use super::diffusion::Diffuser;
use super::feedback::MixedFeedback;
use super::AudioNode;

pub struct BasicReverb<D, const C: usize, const S: usize> {
    diffuser: Diffuser<D, C, S>,
    feedback: MixedFeedback<D, C>,
    dry: f32,
    wet: f32,
}

impl<D: Delay, const C: usize, const S: usize> BasicReverb<D, C, S> {
    pub fn new(sample_rate: usize, room_size_secs: f32, rt60: f32, dry: f32, wet: f32) -> Self {
        let mut rnd = Rnd::new(82734);

        let diffuser = Diffuser::new(sample_rate, room_size_secs, &mut rnd);

        // How long does our signal take to go around the feedback loop?
        let typical_loop_secs = room_size_secs * 1.5;
        // How many times will it do that during an RT60 period?
        let loops_per_rt_60 = rt60 / typical_loop_secs;
        // How many dB to reduce per loop
        let db_per_cycle = -60. / loops_per_rt_60;

        let decay = 10.0_f32.powf(db_per_cycle * 0.05);

        let feedback = MixedFeedback::new(sample_rate, room_size_secs, decay);

        Self {
            dry,
            wet,
            diffuser,
            feedback,
        }
    }
}

impl<D: Delay, const C: usize, const S: usize> AudioNode<C> for BasicReverb<D, C, S> {
    fn process(&mut self, input: [f32; C]) -> [f32; C] {
        let diffuse = self.diffuser.process(input);
        let mut mixed = self.feedback.process(diffuse);

        for i in 0..C {
            mixed[i] = input[i] * self.dry + mixed[i] * self.wet;
        }

        mixed
    }
}
