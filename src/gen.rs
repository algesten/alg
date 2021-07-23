use crate::euclid::euclid;
use crate::pat::Pattern;
use crate::rnd::Rnd;

const DEFAULT_PATTERN_LEN: u8 = 64;
const DEFAULT_TRACK_LEN: u8 = 64;

/// Base for seed since starting at 0 is so boring.
pub const SEED_BASE: i32 = 0x4164c47;

pub const STOKAST_PARAMS: Params<4> = Params {
    seed: SEED_BASE as u32,
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
    pub pattern_length: u8,
    pub patterns: [Pattern; X],
    pub rnd: Rnd,
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

        // Root randomizer.
        let mut rnd = Rnd::new(params.seed);

        let mut patterns: [Pattern; X] = [Pattern::default(); X];

        for (i, pat) in patterns.iter_mut().enumerate() {
            let gen = TrackGenerator {
                // mix in track index to not get the same patterns in all tracks
                seed: rnd.next() + (i as u32),
                params: params.tracks[i],
            };

            *pat = generate(gen.seed, &gen.params, params.pattern_length as usize);
        }

        // reserve 64 track specific rnd before letting it go.
        for _ in X..=64 {
            rnd.next();
        }

        Generated {
            pattern_length: params.pattern_length,
            patterns,
            rnd,
        }
    }

    pub fn len(&self) -> usize {
        X
    }
}

fn generate(seed: u32, params: &TrackParams, pattern_length: usize) -> Pattern {
    let mut rnd = Rnd::new(seed);

    if params.length == 0 {
        // disabled
        return Pattern::new_with(0, pattern_length);
    }

    // Ensure length is at least the number of steps.
    let length = params.length.max(params.steps);

    // This tries to find a subdivision of the pattern length. We use that as range
    // for randomizing the euclidean steps when the user parameter is set to 0 (random).
    // This is because I don't think a large number of steps makes for very interesting
    // rhythms (almost every 16th sounding with some euclidean distributed "holes").
    let range = {
        const DIV: &[u32] = &[
            61, 53, 41, 32, 31, 24, 23, 16, 15, 14, 13, 11, 8, 7, 6, 5, 4,
        ];

        let mut i = 0;

        loop {
            if length < 4 || i == DIV.len() {
                break pattern_length as u32;
            }

            if length as u32 % DIV[i] == 0 {
                break DIV[i];
            }

            i += 1;
        }
    };

    {
        // If the range is possible to subdivide by 4 or 2 to get something in the range of 6-8,
        // we _sometimes_ do that to create variation.
        const SUBDIVIDE: &[u32] = &[4, 2];

        let x = rnd.next();

        for sub in SUBDIVIDE {
            if range % *sub == 0 {
                // 33% of the time, if we are in random mode.
                if params.steps == 0 && x < u32::MAX / 3 {
                    let len = range / *sub;
                    if len == 8 || len == 6 {
                        // let's do it!
                        let mut new_params = TrackParams {
                            length: len as u8,
                            offset: 0,
                            ..*params
                        };

                        let p1 = generate(x, &new_params, len as usize);

                        new_params.density = params.density * 2;
                        let p2 = generate(x + 1, &new_params, len as usize);

                        // This is the whole point.
                        let combined = p1 + p2;

                        return combined.offset(params.offset).repeat_to(pattern_length);
                    }
                }
            }
        }
    }

    // important to generate this also when it's not used since we need rnd.next() every time.
    let random_steps = {
        let unweighted = rnd.next() / (u32::max_value() / range);

        let weighted = if params.density == 0 {
            unweighted
        } else {
            (unweighted * params.density as u32) / 127
        };

        (weighted as u8) + 1
    };

    let steps = if params.steps == 0 {
        random_steps as u8
    } else {
        params.steps
    };

    euclid(steps, length)
        .offset(params.offset)
        .repeat_to(pattern_length)
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::drums::Drums;

    #[test]
    fn generate_test() {
        if true {
            return;
        }
        let x = STOKAST_PARAMS.seed;
        for i in x..(x + 1000) {
            let mut drums = Drums::new();

            let mut params = STOKAST_PARAMS;
            params.seed = i;

            let g: Generated<4> = Generated::new(params);

            for i in 0..g.len() {
                drums.add_pattern(g.patterns[i]);
            }

            println!("{:?}", drums);

            drums.play(1);
        }
    }
}
