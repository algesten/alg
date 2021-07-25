use crate::euclid::euclid;
use crate::pat::Pattern;
use crate::rnd::Rnd;

const DEFAULT_PATTERN_LEN: u8 = 64;
const DEFAULT_TRACK_LEN: u8 = 64;

/// Base for seed since starting at 0 is so boring.
pub const SEED_BASE: i32 = 0x4144c47;

pub const STOKAST_PARAMS: Params<4> = Params {
    seed: SEED_BASE as u32,
    pattern_length: 64,
    tracks: [
        TrackParams {
            steps: 0,
            length: 16,
            offset: 0,
            offset_double: 0,
            density: 35,
            subdiv: 3,
            rare: &[3, 5],
        },
        TrackParams {
            steps: 0,
            length: 32,
            offset: 4,
            offset_double: 3,
            density: 30,
            subdiv: 3,
            rare: &[3, 5, 7],
        },
        TrackParams {
            steps: 0,
            length: 24,
            offset: 2,
            offset_double: 5,
            density: 80,
            subdiv: 4,
            rare: &[],
        },
        TrackParams {
            steps: 0,
            length: 32,
            offset: 2,
            offset_double: 10,
            density: 50,
            subdiv: 4,
            rare: &[],
        },
    ],
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct Params<const X: usize> {
    /// Random seed to generate all randomness from. */
    pub seed: u32,
    /// Length of the entire pattern. */
    pub pattern_length: u8,
    /// Parameters per track */
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
    /// Length of track. In clock-ticks.
    pub length: u8,
    /// Number of euclidean steps. 0 to use a random number.
    pub steps: u8,
    /// Offset of euclidean steps.
    pub offset: u8,
    /// Chance of doubling the offset. 4 means 1/4.
    pub offset_double: u8,
    /// density/127 multiplication factor for random steps.
    pub density: u8,
    /// Chance to do subdivision. 3 would mean 1/3.
    pub subdiv: u32,
    /// Steps that we don't want much of.
    pub rare: &'static [u8],
}

impl Default for TrackParams {
    fn default() -> Self {
        TrackParams {
            length: DEFAULT_TRACK_LEN,
            steps: 0,
            offset: 0,
            offset_double: 0,
            density: 0,
            subdiv: 0,
            rare: &[],
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

impl<const X: usize> Generated<X> {
    pub fn new(params: Params<X>) -> Self {
        assert!(params.pattern_length > 0);

        // Root randomizer.
        let mut rnd = Rnd::new(params.seed);

        let mut patterns: [Pattern; X] = [Pattern::default(); X];

        for i in 0..X {
            let mut seed = rnd.next() + (i as u32);

            'redo: loop {
                // generate pattern for this index.
                patterns[i] = generate(
                    seed,
                    &params.tracks[i],
                    params.pattern_length as usize,
                    true,
                    true,
                );

                // ensure we didn't end up with something that is exactly like
                // what we already have.
                let p = &patterns[i];
                for j in 0..i {
                    if j == i {
                        continue;
                    }
                    if p == &patterns[j] {
                        // pattern is exactly same as something else. redo it.
                        seed += 1;
                        continue 'redo;
                    }
                }

                break;
            }
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

fn generate(
    seed: u32,
    params: &TrackParams,
    pattern_length: usize,
    allow_subdivision: bool,
    enforce_rare: bool,
) -> Pattern {
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
        let x = rnd.next();
        let y = rnd.next();
        let a = rnd.next();
        let b = rnd.next();

        let do_subdivide = allow_subdivision // if it is allowed
            && params.steps == 0                  // the steps are indeed set to random
            && params.subdiv > 0                  // the params indicate there can be subdivision
            && x < u32::MAX / params.subdiv; // and the chance is on our side

        if do_subdivide {
            // The subdivisions we will attempt.
            const SUBDIVIDE: &[usize] = &[32, 24, 16, 8, 6, 4];

            for length in SUBDIVIDE {
                // Must divide evenly, and actually divide (not == 1)
                let l_u32 = *length as u32;
                if range % l_u32 != 0 || range / l_u32 <= 1 {
                    continue;
                }

                // let's do it!
                let mut new_params = TrackParams {
                    length: *length as u8,
                    offset: 0,
                    ..*params
                };

                let mut p1 = generate(x, &new_params, *length, false, true);

                new_params.density = params.density.wrapping_mul(2);
                let mut p2 = generate(x + 1, &new_params, *length, false, false);

                // Occassionally we will do:
                // p1-p2-p1-p2
                // and sometimes:
                // p1-p2-p1-p3
                let mut p3 = generate(x + 2, &new_params, *length, false, false);

                // Sometimes we add extra beats.
                if a < u32::MAX / 3 {
                    // 8 positions:
                    // - p1 -  .... - p2 - .... - p3 -
                    // 0 1 2        3 4  5      6 7  8
                    let mut pos = (b / (u32::MAX / 8)) as isize;

                    if pos <= 2 {
                        if pos == 1 {
                            pos -= 1;
                        }
                        p1.set(pos - 1, 70);
                    } else if pos <= 5 {
                        if pos == 4 {
                            pos -= 1;
                        }
                        p2.set(pos - 4, 70);
                    } else {
                        if pos == 7 {
                            pos -= 1;
                        }
                        p3.set(pos - 7, 70);
                    }
                }

                // Use the one with most density as last.
                let (p2, p3) = if p2.density() > p3.density() {
                    (p3, p2)
                } else {
                    (p2, p3)
                };

                // This is the whole point.
                let combined = if y < u32::MAX / 3 {
                    p1 + p2 + p1 + p3
                } else {
                    // Most of the time, we just alternate two patterns.
                    p1 + p2
                };

                return combined.offset(params.offset).repeat_to(pattern_length);
            }
        }
    }

    // Offset randomization. This is sowe don't always get snare on the second beat.
    let mut offset = params.offset;
    {
        let x = rnd.next();

        // if we are randomizing
        if params.steps == 0
        // and there is a setting for this track
            && params.offset_double > 0
            // sometimes double the offset.
            && x < u32::MAX / (params.offset_double as u32)
        {
            offset *= 2;
        }
    }

    // important to generate this also when it's not used since we need rnd.next() every time.
    let random_steps = loop {
        let r = rnd.next();
        let unweighted = r / (u32::MAX / range);

        let weighted = if params.density == 0 {
            unweighted
        } else {
            (unweighted * params.density as u32) / 127
        };

        let proposed = (weighted as u8) + 1;

        // The rare functionality is a mechanism to not have too much  3 and 5 as steps
        // of the bassdrum or snare.
        let is_rare = r < u32::MAX / 10;
        if enforce_rare && params.rare.contains(&proposed) && !is_rare {
            continue;
        }

        break proposed;
    };

    let steps = if params.steps == 0 {
        random_steps as u8
    } else {
        params.steps
    };

    euclid(steps, length)
        .offset(offset)
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
