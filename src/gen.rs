use crate::{euclid, Pattern, Rnd};

pub struct Params<const X: usize> {
    /** Random seed to generate all randomness from. */
    pub seed: u32,
    /** Length of the entire pattern. */
    pub pattern_length: u8,
    /** Parameters per track */
    pub tracks: [TrackParams; X],
}

#[derive(Clone, Copy, Default)]
pub struct TrackParams {
    /** Length of track. In clock-ticks. */
    pub length: u8,
    /** Number of euclidean steps. 0 to use a random number. */
    pub steps: u8,
    /** Offset of euclidean steps. */
    pub offset: u8,
}

pub struct Generated<const X: usize> {
    pattern_length: u8,
    tracks: [TrackGenerator; X],
}

#[derive(Clone, Copy, Default)]
struct TrackGenerator {
    /** Seed per track to not change depending on global seed. */
    seed: u32,
    params: TrackParams,
}

impl<const X: usize> Generated<X> {
    pub fn new(params: Params<X>) -> Self {
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

        let basis = {
            let mut i = 16;
            loop {
                if pattern_length % i == 0 {
                    break i;
                }
                i -= 1;
            }
        };

        if self.params.length == 0 {
            // disabled
            return Pattern::new_with(0, pattern_length);
        }

        // Ensure length is at least the number of steps.
        let length = self.params.length.max(self.params.steps);

        // important to generate this also when it's not used.
        let random_steps = {
            // basis here is a denominator from 16..1 depending on pattern length.
            let v = rnd.next() / (u32::max_value() / (basis as u32 - 1));

            (v % (length as u32)) + 1
        };

        let steps = if self.params.steps == 0 {
            println!("random: {}/{}", random_steps, length);
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
    use crate::drums::Drums;

    use super::*;

    #[test]
    fn generate_test() {
        let mut drums = Drums::new();

        let g: Generated<4> = Generated::new(Params {
            seed: 15,
            pattern_length: 32,
            tracks: [
                TrackParams {
                    steps: 1,
                    length: 4,
                    offset: 0,
                },
                TrackParams {
                    steps: 1,
                    length: 8,
                    offset: 4,
                },
                TrackParams {
                    steps: 0,
                    length: 24,
                    offset: 2,
                },
                TrackParams {
                    steps: 0,
                    length: 32,
                    offset: 2,
                },
            ],
        });

        for i in 0..g.len() {
            drums.add_pattern(g.get_pattern(i));
        }

        println!("{:?}", drums);

        drums.play(2);
    }
}
