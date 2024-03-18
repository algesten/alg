//!
//!
//! * C - channels
//! * N - buffer length
//! * S - sample rate
//! * M - milliseconds
//!

mod delay;
mod diffusion;
mod feedback;
mod hadamard;
mod householder;
mod reverb;

pub use reverb::Reverb;

pub trait AudioNode<const C: usize> {
    fn process(&mut self, input: [f32; C]) -> [f32; C];
}

#[cfg(test)]
mod test {
    use crate::drums::Drums;
    use rodio::buffer::SamplesBuffer;
    use rodio::{OutputStream, Sink, Source};

    use super::delay::MemoryDelay;
    use super::{AudioNode, Reverb};

    #[test]
    fn play_reverb() {
        let kick = Drums::Clap.as_source();

        let mut input = vec![];
        input.extend(kick);
        input.resize(input.len() + 44100 * 2 * 2, 0.0);

        let mut input = SamplesBuffer::new(2, 44100, input);

        let mut reverb: Reverb<MemoryDelay<440_000>> =
            Reverb::new(input.sample_rate() as usize, 3.0, 0.05, 0.0, 1.0);

        let mut samples = vec![];

        while let Some(sample) = input.next() {
            let input = [sample, sample];
            let output = reverb.process(input);
            samples.push(output[0]);
            samples.push(output[1]);
        }

        let samples = SamplesBuffer::new(2, 44100, samples);
        let (_stream, handle) = OutputStream::try_default().unwrap();

        let sink = Sink::try_new(&handle).unwrap();

        sink.append(samples);

        sink.play();
        sink.sleep_until_end();
    }
}
