use super::{Pattern, PatternGroup};

const EUCLID_MAX: u8 = 64;

pub fn euclid(steps: u8, length: u8) -> Pattern {
    assert!(steps > 0);
    assert!(length > 0);
    assert!(steps <= EUCLID_MAX);
    assert!(length <= EUCLID_MAX);

    // length cannot be shorter than number of steps.
    let length = length.max(steps);

    // Algorithm is inspired by this graphic:
    // https://medium.com/code-music-noise/euclidean-rhythms-391d879494df
    let mut l = PatternGroup::new_with("x".into(), steps as usize);
    let mut r = PatternGroup::new_with("-".into(), (length - steps) as usize);

    while r.len() > 0 {
        let s = l.len().min(r.len());
        let t = l.len().max(r.len());

        let mut nl = PatternGroup::new();

        for i in 0..s {
            nl.push(l[i] + r[i]);
        }

        let nr = l.sub(s..t) + r.sub(s..t);

        l = nl;
        r = nr;

        // println!("{:?} {:?}", l, r);
    }

    let pattern = l.flatten() + r.flatten();
    assert_eq!(pattern.len(), length as usize);

    pattern
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    pub fn euclid_1_16() {
        assert_eq!(euclid(1, 16), "|x---------------|");
    }

    #[test]
    pub fn euclid_4_16() {
        assert_eq!(euclid(4, 16), "|x---x---x---x---|");
    }

    // This is to ensure stability in case we change
    // the algorithm. Odd number of beats could
    // potentially be distributed differently.
    #[test]
    pub fn euclid_prime() {
        assert_eq!(euclid(2, 7), "|x---x--|");
        assert_eq!(euclid(3, 7), "|x--x-x-|");
        assert_eq!(euclid(5, 7), "|x-xxx-x|");
        assert_eq!(euclid(3, 8), "|x--x-x--|");
        assert_eq!(euclid(5, 8), "|x-xx-x-x|");
        assert_eq!(euclid(7, 8), "|x-xxxxxx|");
        assert_eq!(euclid(3, 16), "|x-----x----x----|");
        assert_eq!(euclid(5, 16), "|x---x--x--x--x--|");
        assert_eq!(euclid(7, 16), "|x--x-x-x-x--x-x-|");
        assert_eq!(euclid(9, 16), "|x-xx-x-x-x-xx-x-|");
        assert_eq!(euclid(10, 16), "|x-xx-x-xx-xx-x-x|");
        assert_eq!(euclid(11, 16), "|x-xxx-xx-xx-xx-x|");
        assert_eq!(euclid(12, 16), "|x-xxx-xxx-xxx-xx|");
        assert_eq!(euclid(13, 16), "|x-xxxxx-xxxx-xxx|");
        assert_eq!(euclid(15, 16), "|x-xxxxxxxxxxxxxx|");
    }

    // Brute force try all euclid patterns.
    #[test]
    pub fn euclid_all() {
        for i in 1..=EUCLID_MAX {
            for j in i..=EUCLID_MAX {
                euclid(i, j);
            }
        }
    }

    #[test]
    pub fn euclid_offset() {
        assert_eq!(euclid(2, 5).offset(0), "|x--x-|");
        assert_eq!(euclid(2, 5).offset(1), "|--x-x|");
        assert_eq!(euclid(2, 5).offset(2), "|-x-x-|");
        assert_eq!(euclid(2, 5).offset(3), "|x-x--|");
        assert_eq!(euclid(2, 5).offset(4), "|-x--x|");
        assert_eq!(euclid(2, 5).offset(5), "|x--x-|");
        assert_eq!(euclid(2, 5).offset(6), "|--x-x|");
    }

    // use crate::drums::Drums;

    // #[test]
    // pub fn euclid_drums() {
    //     let mut drums = Drums::new();

    //     drums.add_pattern(euclid(3, 18).repeat_to(32).repeat_to(64));
    //     drums.add_pattern(euclid(2, 16).offset(4));
    //     drums.add_pattern(euclid(2, 11).offset(3).repeat_to(32));
    //     drums.add_pattern(euclid(4, 16).offset(2));

    //     println!("{:?}", drums);

    //     drums.play(1);
    // }
}
