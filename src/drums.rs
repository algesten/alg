use crate::pat::trim_pattern;
use crate::Pattern;
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

const CLOCK: f32 = 1.0 / (4.0 * 135.0 / 60.0);

fn wav(bytes: &'static [u8], volume: f32, duration: f32) -> impl Source<Item = i16> {
    let buf = BufReader::new(Cursor::new(bytes));

    let silence: Empty<i16> = Empty::new();

    Decoder::new(buf)
        .unwrap()
        .amplify(volume)
        .take_crossfade_with(silence, Duration::from_secs_f32(duration))
}

#[derive(Default)]
pub struct Drums {
    tracks: Vec<String>,
}

impl Drums {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn add_track(&mut self, pat: &str) {
        assert!(self.tracks.len() < SAMPLES.len());
        self.tracks.push(trim_pattern(pat).to_string());
    }

    pub fn add_pattern(&mut self, pat: Pattern) {
        self.add_track(&format!("{:?}", pat));
    }

    pub fn play(&self, loop_count: usize) {
        let (_stream, handle) = OutputStream::try_default().unwrap();
        let sinks = [
            Sink::try_new(&handle).unwrap(),
            Sink::try_new(&handle).unwrap(),
            Sink::try_new(&handle).unwrap(),
            Sink::try_new(&handle).unwrap(),
        ];

        let max_track = self.tracks.iter().map(|t| t.len()).max().unwrap_or(0);
        // The entire play length is max length of all tracks * loop_count
        let len = max_track * loop_count;

        for (i, pattern) in self.tracks.iter().enumerate() {
            let sample = SAMPLES[i];
            let sink = &sinks[i];

            let mut chars = pattern.chars();

            for _ in 0..len {
                let step = if let Some(n) = chars.next() {
                    n
                } else {
                    if pattern.is_empty() {
                        chars = "----------------".chars();
                    } else {
                        // Every time this pattern ends before max_track, loop over chars again.
                        chars = pattern.chars();
                    }
                    chars.next().unwrap()
                };

                let volume = if step == '-' {
                    0.0
                } else if step.is_ascii_lowercase() {
                    0.5
                } else {
                    1.0
                };

                sink.append(wav(sample, volume, CLOCK));
            }
        }

        sinks.iter().for_each(Sink::play);
        sinks.iter().for_each(Sink::sleep_until_end);
    }
}

// #[cfg(test)]
// mod test {
//     use super::*;

//     #[test]
//     fn drum_test() {
//         let mut drums = Drums::new();

//         drums.add_track("|x---x---x---x---|"); // kick
//         drums.add_track("|----x-------x---|"); // clap
//         drums.add_track("|x---x-x-x---x-x-|"); // hh cl
//         drums.add_track("|--x-------x-----|"); // hh op

//         drums.play(1);
//     }
// }
