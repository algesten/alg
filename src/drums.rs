//! This is a simple drum player with 4 samples for testing.
//! It takes up to 4 patterns and the order they are added matters.
//! kick, clap, closed and open hihat.

use crate::pat::Pattern;
use rodio::source::Empty;
use rodio::Sink;
use rodio::{source::Source, Decoder, OutputStream};
use std::io::BufReader;
use std::io::Cursor;
use std::time::Duration;

const SAMPLES: &[&[u8]] = &[
    include_bytes!("../kit/kick.wav"),
    include_bytes!("../kit/clap.wav"),
    include_bytes!("../kit/hhcl.wav"),
    include_bytes!("../kit/hhop.wav"),
];

/// 135 BPM.
const CLOCK: f32 = 1.0 / (4.0 * 135.0 / 60.0);

fn wav(bytes: &'static [u8], volume: f32, duration: f32) -> impl Source<Item = i16> {
    let buf = BufReader::new(Cursor::new(bytes));

    let silence: Empty<i16> = Empty::new();

    Decoder::new(buf)
        .unwrap()
        .amplify(volume)
        // To avoid clicks, we crossfade each sound to silence.
        .take_crossfade_with(silence, Duration::from_secs_f32(duration))
}

#[derive(Default)]
pub struct Drums {
    tracks: Vec<Pattern>,
}

impl std::fmt::Debug for Drums {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        writeln!(f)?;

        for t in &self.tracks {
            writeln!(f, "{:?}", t)?;
        }

        Ok(())
    }
}

impl Drums {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_pattern<P: Into<Pattern>>(&mut self, pat: P) {
        assert!(self.tracks.len() < SAMPLES.len());

        let mut pat: Pattern = pat.into();

        let cur_max = self.tracks.iter().map(|t| t.len()).max().unwrap_or(0);

        if pat.len() > cur_max {
            // resize all existing.
            self.tracks
                .iter_mut()
                .for_each(|t| *t = t.repeat_to(pat.len()))
        } else {
            // resize pat
            pat = pat.repeat_to(cur_max);
        }

        self.tracks.push(pat);
    }

    pub fn play(&self, loop_count: usize) {
        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sinks = [
            Sink::try_new(&handle).unwrap(),
            Sink::try_new(&handle).unwrap(),
            Sink::try_new(&handle).unwrap(),
            Sink::try_new(&handle).unwrap(),
        ];

        for _ in 0..loop_count {
            for (i, pattern) in self.tracks.iter().enumerate() {
                let sample = SAMPLES[i];
                let sink = &sinks[i];

                for i in 0..pattern.len() {
                    let step = &pattern[i];

                    let volume = *step as f32 / 255.0;

                    sink.append(wav(sample, volume, CLOCK));
                }
            }
        }

        sinks.iter().for_each(Sink::play);
        sinks.iter().for_each(Sink::sleep_until_end);
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn drum_test() {
        let mut drums = Drums::new();

        drums.add_pattern("|x---x---x---x---|"); // kick
        drums.add_pattern("|----x-------x---|"); // clap
        drums.add_pattern("|x---x-x-x---x-x-|"); // hh cl
        drums.add_pattern("|--x-------x-----|"); // hh op

        drums.play(1);
    }
}
