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
    /**  If randomzing steps, whether the steps are targeting "dense" or "sparse" */
    pub dense: bool,
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

        let basis = {
            //  17, 19, 23, 29, 31, 37, 41, 43, 47, 53, 59, 61
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

        // important to generate this also when it's not used.
        let random_steps = {
            let (offset, range) = offset_range(basis, self.params.dense);

            println!("offset: {}, range: {}", offset, range);

            // basis here is a denominator from 16..1 depending on pattern length.
            let v = rnd.next() / (u32::max_value() / range);

            offset + v + 1
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

fn offset_range(basis: u32, dense: bool) -> (u32, u32) {
    // 1 => 0-0 0-0
    // 2 => 0-1 1-1
    // 3 => 0-1 1-2
    // 4 => 0-2 2-3
    let rest = (basis as u32 + 1) % 2;
    let half = (basis - rest) / 2;
    let off = if dense { half + rest } else { 0 };
    let range = half + if dense { 0 } else { rest };

    (off, range)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::drums::Drums;

    #[test]
    fn offset_range_test() {
        const Z: &[(u32, ((u32, u32), (u32, u32)))] = &[
            (1, ((0, 0), (0, 0))),
            (2, ((0, 1), (1, 0))),
            (3, ((0, 1), (1, 1))),
            (4, ((0, 2), (2, 1))),
            (5, ((0, 2), (2, 2))),
            (6, ((0, 3), (3, 2))),
            (7, ((0, 3), (3, 3))),
        ];

        for (basis, eq) in Z {
            assert_eq!(
                (offset_range(*basis, false), offset_range(*basis, true)),
                *eq,
                "for basis {}",
                basis
            );
        }
    }

    #[test]
    fn generate_test() {
        let mut drums = Drums::new();

        let g: Generated<4> = Generated::new(Params {
            seed: 121,
            pattern_length: 64,
            tracks: [
                TrackParams {
                    steps: 0,
                    length: 16,
                    offset: 0,
                    dense: false,
                },
                TrackParams {
                    steps: 4,
                    length: 32,
                    offset: 4,
                    dense: false,
                },
                TrackParams {
                    steps: 0,
                    length: 24,
                    offset: 2,
                    dense: false,
                },
                TrackParams {
                    steps: 0,
                    length: 32,
                    offset: 2,
                    dense: true,
                },
            ],
        });

        for i in 0..g.len() {
            drums.add_pattern(g.get_pattern(i));
        }

        println!("{:?}", drums);

        drums.play(4);
    }
}
