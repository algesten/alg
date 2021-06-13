use core::num::Wrapping as w;

#[derive(Debug)]
pub struct Rnd(u32);

/**
* This is a 32-bit variant on the 63-bit Thrust PRNG
*/
impl Rnd {
    pub fn new(seed: u32) -> Self {
        Rnd(seed)
    }

    pub fn next(&mut self) -> u32 {
        let mut z = w(self.0) + w(0x6D2B79F5);
        self.0 = z.0;
        z = (z ^ (z >> 15)) * (z | w(1));
        z ^= z + (z ^ (z >> 7)) * (z | w(61));
        (z ^ (z >> 14)).0
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_seq() {
        let mut r = Rnd::new(12);
        assert_eq!(r.next(), 1237598750);
        assert_eq!(r.next(), 324989476);
        assert_eq!(r.next(), 2491772807);
    }
}
