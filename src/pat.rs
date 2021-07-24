use core::ops::Range;

const MAX_LEN: usize = 64;

/// A rhythmical pattern. Each step is a `u8` representing the velocity of that step.
pub type Pattern = Pat<u8>;
/// A group of patterns (pattern of patterns).
pub type PatternGroup = Pat<Pattern>;

/// Generic pattern.
#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pat<T>([T; MAX_LEN], usize);

impl<T> Pat<T>
where
    T: Copy + Default,
{
    /// Construct a new pattern of length 0. Each value is `T::default()`.
    pub fn new() -> Self {
        Pat([T::default(); MAX_LEN], 0)
    }

    /// Construct a new pattern repeating `val` and setting the length.
    pub fn new_with(val: T, len: usize) -> Self {
        assert!(len <= MAX_LEN);

        Pat([val; MAX_LEN], len)
    }

    /// Length of the pattern.
    pub fn len(&self) -> usize {
        self.1
    }

    /// Append a value to the pattern. Increases length by 1.
    pub fn push(&mut self, val: T) {
        assert!(self.1 + 1 <= MAX_LEN);

        self.0[self.1] = val;
        self.1 += 1;
    }

    /// Clone a subrange of the pattern.
    ///
    /// TODO: Maybe do this as index operators?
    pub fn sub(&self, range: Range<usize>) -> Self {
        let mut p = [T::default(); MAX_LEN];

        let from = 0.max(range.start);
        let to = self.1.min(range.end);
        let len = to - from;
        for i in 0..len {
            p[i] = self.0[i + from];
        }

        Pat(p, len)
    }

    /// Get the value at `index`.
    pub fn get(&self, index: usize) -> Option<T> {
        if index < self.1 {
            Some(self.0[index])
        } else {
            None
        }
    }

    /// Set the value at `index`.
    pub fn set(&mut self, mut index: isize, v: T) {
        while index < 0 {
            index += self.1 as isize;
        }
        let index = index as usize % self.1;

        self.0[index] = v;
    }

    /// Make a copy of self where each value is `offset` to the right. Values pushed
    /// off the end to right wraps around and comes in at the beginning.
    ///
    /// ```ignore
    /// Start:    --A---B-CD-
    /// Offset 1: ---A---B-CD
    /// Offset 5: B-CD---A---
    /// ```
    pub fn offset(&self, offset: u8) -> Self {
        let m = (offset as usize) % self.1;
        let p = self.1 - m;
        self.sub(p..self.1) + self.sub(0..p)
    }

    /// Lengthen the pattern by repeating what is already there to an absolute `len`.
    ///
    /// ```ignore
    /// Start:     xA-B
    /// Repeat 7:  xA-BxA-
    /// Repeat 12: xA-BxA-BxA-B
    /// ```
    pub fn repeat_to(&self, len: usize) -> Self {
        assert!(len <= MAX_LEN);

        let mut x = Self::new();

        if self.1 == 0 {
            return x;
        }

        let n = len / self.1;
        let m = len % self.1;

        for _ in 0..n {
            x += *self;
        }
        x += self.sub(0..m);

        x
    }
}

impl<T> Pat<Pat<T>> {
    /// Concatenate the inner `Pat<T>` in `Pat<Pat<T>>` to a `Pat<T>`.
    /// Sometimes called "flatmap". `[[1,2],[3,4]]` becomes `[1,2,3,4]`
    pub fn flatten(&self) -> Pat<T>
    where
        T: Copy + Default,
    {
        let mut p = Pat::new();

        for i in 0..self.1 {
            p += self[i];
        }

        p
    }
}

impl<T> core::ops::Add for Pat<T>
where
    T: Copy + Default,
{
    type Output = Pat<T>;

    fn add(self, rhs: Self) -> Self::Output {
        assert!(self.1 + rhs.1 <= MAX_LEN);

        let mut p = [T::default(); MAX_LEN];

        for i in 0..self.1 {
            p[i] = self.0[i];
        }
        for i in 0..rhs.1 {
            p[i + self.1] = rhs.0[i];
        }

        Pat(p, self.1 + rhs.1)
    }
}

impl<T> core::ops::AddAssign for Pat<T>
where
    T: Copy + Default,
{
    fn add_assign(&mut self, rhs: Self) {
        assert!(self.1 + rhs.1 <= MAX_LEN);

        let off = self.1;

        for i in 0..rhs.1 {
            self.0[i + off] = rhs.0[i];
        }

        self.1 += rhs.1;
    }
}

impl<T> core::ops::Index<usize> for Pat<T> {
    type Output = T;

    fn index(&self, index: usize) -> &Self::Output {
        assert!(index < self.1, "Index out of bounds {} < {}", index, self.1);

        &self.0[index]
    }
}

impl<T> core::ops::IndexMut<usize> for Pat<T> {
    fn index_mut(&mut self, index: usize) -> &mut Self::Output {
        assert!(index < self.1, "Index out of bounds {} < {}", index, self.1);

        &mut self.0[index]
    }
}

impl<T> Default for Pat<T>
where
    T: Copy + Default,
{
    fn default() -> Self {
        Pat([T::default(); MAX_LEN], 0)
    }
}

impl Pat<u8> {
    pub fn density(&self) -> u8 {
        let x = self.repeat_to(64);
        x.0.iter().filter(|x| **x > 0).count() as u8 * 2
    }
}

impl PartialEq<&str> for Pat<u8> {
    fn eq(&self, other: &&str) -> bool {
        let trim = trim_pattern(other);

        if trim.len() != self.len() {
            return false;
        }

        if self.0.iter().zip(trim.chars()).any(|(x, y)| {
            if y == '-' {
                *x != 0
            } else if y.is_lowercase() {
                *x == 0 || *x > 127
            } else {
                *x <= 127
            }
        }) {
            return false;
        }

        true
    }
}

impl core::fmt::Debug for Pattern {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "|")?;
        for i in 0..self.1 {
            write!(
                f,
                "{}",
                match self.0[i] {
                    0 => '-',
                    x if x <= 127 => 'x',
                    _ => 'X',
                }
            )?;
        }
        write!(f, "|")?;
        Ok(())
    }
}

impl core::fmt::Debug for PatternGroup {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "[")?;
        for i in 0..self.1 {
            if i > 0 {
                write!(f, ", ")?;
            }
            write!(f, "{:?}", self.0[i])?;
        }
        write!(f, "]")?;
        Ok(())
    }
}

/// Quick way to make patterns from strings.
///
/// * `-` is off
/// * `x` is velocity 127
/// * `X` is velocity 255
///
/// Example:
/// A 4-on-the-floor bass drum with emphasis on 1: `X---x---x---x---`
impl From<&str> for Pattern {
    fn from(s: &str) -> Self {
        let trimmed = trim_pattern(s);

        let mut p = Pattern::new();

        for step in trimmed.chars() {
            let val = if step == '-' {
                0
            } else if step.is_lowercase() {
                127
            } else {
                255
            };

            p.push(val);
        }

        p
    }
}

pub fn trim_pattern(pattern: &str) -> &str {
    let mut trim = pattern.as_bytes();

    if !trim.is_empty() && trim[0] == b'|' {
        trim = &trim[1..];
    }
    if !trim.is_empty() && trim[trim.len() - 1] == b'|' {
        trim = &trim[..trim.len() - 1];
    }

    core::str::from_utf8(trim).expect("trim pattern")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pattern_add() {
        let p1: Pattern = "xX".into();
        let p2: Pattern = "-".into();

        let x = p1 + p2;

        assert_eq!(x.len(), 3);
        assert_eq!(x[0], 127);
        assert_eq!(x[1], 255);
        assert_eq!(x[2], 0);

        assert_eq!(x, "xX-");
        assert_eq!(x, "|xX-|");
    }

    #[test]
    fn pattern_add_0() {
        let p1: Pattern = "xX".into();
        let p2: Pattern = "".into();

        let x = p1 + p2;

        assert_eq!(x.len(), 2);
        assert_eq!(x[0], 127);
        assert_eq!(x[1], 255);
    }

    #[test]
    fn pattern_sub() {
        let p1: Pattern = "xX--".into();

        assert_eq!(p1.len(), 4);
        let p2 = p1.sub(1..3);

        assert_eq!(p2.len(), 2);
        assert_eq!(p2[0], 255);
        assert_eq!(p2[1], 0);
    }

    #[test]
    fn trim_test() {
        assert_eq!(trim_pattern(""), "");
        assert_eq!(trim_pattern("|"), "");
        assert_eq!(trim_pattern("||"), "");
        assert_eq!(trim_pattern("-|"), "-");
        assert_eq!(trim_pattern("|-"), "-");
        assert_eq!(trim_pattern("-"), "-");
    }
}
