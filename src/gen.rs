use crate::{euclid, Pattern, Rnd};

const DEFAULT_PATTERN_LEN: u8 = 64;
const DEFAULT_TRACK_LEN: u8 = 64;

pub const STOKAST_PARAMS: Params<4> = Params {
    seed: 0,
    pattern_length: 64,
    tracks: [
        TrackParams {
            steps: 0,
            length: 16,
            offset: 0,
            density: 30,
        },
        TrackParams {
            steps: 0,
            length: 32,
            offset: 4,
            density: 30,
        },
        TrackParams {
            steps: 0,
            length: 24,
            offset: 2,
            density: 80,
        },
        TrackParams {
            steps: 0,
            length: 32,
            offset: 2,
            density: 50,
        },
    ],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Params<const X: usize> {
    /** Random seed to generate all randomness from. */
    pub seed: u32,
    /** Length of the entire pattern. */
    pub pattern_length: u8,
    /** Parameters per track */
    pub tracks: [TrackParams; X],
}

impl<const X: usize> Default for Params<X> {
    fn default() -> Self {
        Params {
            seed: 0,
            pattern_length: DEFAULT_PATTERN_LEN,
            tracks: [TrackParams::default(); X],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct TrackParams {
    /** Length of track. In clock-ticks. */
    pub length: u8,
    /** Number of euclidean steps. 0 to use a random number. */
    pub steps: u8,
    /** Offset of euclidean steps. */
    pub offset: u8,
    /** density/127 multiplication factor for random steps. */
    pub density: u8,
}

impl Default for TrackParams {
    fn default() -> Self {
        TrackParams {
            length: DEFAULT_TRACK_LEN,
            steps: 0,
            offset: 0,
            density: 0,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Generated<const X: usize> {
    pattern_length: u8,
    tracks: [TrackGenerator; X],
}

impl<const X: usize> Default for Generated<X> {
    fn default() -> Self {
        Generated::new(Params {
            ..Default::default()
        })
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq)]
struct TrackGenerator {
    /** Seed per track to not change depending on global seed. */
    seed: u32,
    params: TrackParams,
}

impl<const X: usize> Generated<X> {
    pub fn new(params: Params<X>) -> Self {
        assert!(params.pattern_length > 0);

        let mut tracks = [TrackGenerator::default(); X];

        // Root randomizer.
        let mut rnd = Rnd::new(params.seed);

        for i in 0..X {
            tracks[i] = TrackGenerator {
                // mix in track index to not get the same patterns in all tracks
                seed: rnd.next() + (i as u32),
                params: params.tracks[i],
            };
        }

        Generated {
            pattern_length: params.pattern_length,
            tracks,
        }
    }

    pub fn len(&self) -> usize {
        X
    }

    pub fn get_pattern(&self, index: usize) -> Pattern {
        self.tracks[index].generate(self.pattern_length as usize)
    }
}

impl TrackGenerator {
    fn generate(&self, pattern_length: usize) -> Pattern {
        let mut rnd = Rnd::new(self.seed);

        if self.params.length == 0 {
            // disabled
            return Pattern::new_with(0, pattern_length);
        }

        // Ensure length is at least the number of steps.
        let length = self.params.length.max(self.params.steps);

        // This tries to find a subdivision of the pattern length. We use that as range
        // for randomizing the euclidean steps when the user parameter is set to 0 (random).
        // This is because I don't think  a large number of steps doesn't make for very
        // interesting rhythms (almost every 16th sounding with some euclidean distributed "holes").
        let range = {
            const DIV: &[u32] = &[61, 53, 41, 31, 23, 16, 15, 14, 13, 11, 8, 7, 6, 5, 4];

            let mut i = 0;

            loop {
                if length < 4 || i == DIV.len() {
                    break pattern_length as u32;
                }

                if length as u32 / DIV[i] > 0 {
                    break DIV[i];
                }

                i += 1;
            }
        };

        // important to generate this also when it's not used since we need rnd.next() every time.
        let random_steps = {
            let unweighted = rnd.next() / (u32::max_value() / range);

            let weighted = (unweighted * self.params.density as u32) / 127;

            (weighted as u8) + 1
        };

        let steps = if self.params.steps == 0 {
            random_steps as u8
        } else {
            self.params.steps
        };

        euclid(steps, length)
            .offset(self.params.offset)
            .repeat_to(pattern_length)
    }
}

#[cfg(test)]
mod test {
    use std::time::SystemTime;

    use super::*;
    use crate::drums::Drums;

    #[test]
    fn generate_test() {
        if true {
            return;
        }
        let x = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .unwrap()
            .as_secs_f32() as u32;
        for i in x..(x + 1000) {
            let mut drums = Drums::new();

            let g: Generated<4> = Generated::new(STOKAST_PARAMS);

            for i in 0..g.len() {
                drums.add_pattern(g.get_pattern(i));
            }

            println!("{:?}", drums);

            drums.play(1);
        }
    }
}
