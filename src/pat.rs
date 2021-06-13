use core::ops::Range;

const MAX_LEN: usize = 64;

pub type Pattern = Pat<bool>;
pub type PatternGroup = Pat<Pattern>;

#[derive(Clone, Copy, PartialEq, Eq)]
pub struct Pat<T>([T; MAX_LEN], usize);

impl<T> Pat<T>
where
    T: Copy + Default,
{
    pub fn new(val: T, length: usize) -> Self {
        assert!(length <= MAX_LEN);

        Pat([val; MAX_LEN], length)
    }

    pub fn len(&self) -> usize {
        self.1
    }

    pub fn push(&mut self, val: T) {
        assert!(self.1 + 1 <= MAX_LEN);

        self.0[self.1] = val;
        self.1 += 1;
    }

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

    pub fn get(&self, index: usize) -> Option<T> {
        if index < self.1 {
            Some(self.0[index])
        } else {
            None
        }
    }
}

impl<T> Pat<Pat<T>> {
    pub fn flatten(&self) -> Pat<T>
    where
        T: Copy + Default,
    {
        let mut p = Pat::new(T::default(), 0);

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

impl PartialEq<&str> for Pat<bool> {
    fn eq(&self, other: &&str) -> bool {
        let trim = trim_pattern(other);

        if trim.len() != self.len() {
            return false;
        }

        if self
            .0
            .iter()
            .zip(trim.chars())
            .any(|(x, y)| *x != (y != '-'))
        {
            return false;
        }

        true
    }
}

impl core::fmt::Debug for Pattern {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "|")?;
        for i in 0..self.1 {
            write!(f, "{}", if self.0[i] { 'x' } else { '-' })?;
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

pub fn trim_pattern(pattern: &str) -> &str {
    let mut trim = pattern.as_bytes();

    if trim[0] == b'|' {
        trim = &trim[1..];
    }
    if trim[trim.len() - 1] == b'|' {
        trim = &trim[..trim.len() - 1];
    }

    core::str::from_utf8(trim).expect("trim pattern")
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn pattern_add() {
        let p1 = Pattern::new(true, 2);
        let p2 = Pattern::new(false, 1);

        let x = p1 + p2;

        assert_eq!(x.len(), 3);
        assert_eq!(x[0], true);
        assert_eq!(x[1], true);
        assert_eq!(x[2], false);

        assert_eq!(x, "xx-");
        assert_eq!(x, "|xx-|");
    }

    #[test]
    fn pattern_add_0() {
        let p1 = Pattern::new(true, 2);
        let p2 = Pattern::new(false, 0);

        let x = p1 + p2;

        assert_eq!(x.len(), 2);
        assert_eq!(x[0], true);
        assert_eq!(x[1], true);
    }

    #[test]
    fn pattern_sub() {
        let mut p1 = Pattern::new(false, 0);

        p1.push(true);
        p1.push(true);
        p1.push(false);
        p1.push(false);

        assert_eq!(p1.len(), 4);
        let p2 = p1.sub(1..3);

        assert_eq!(p2.len(), 2);
        assert_eq!(p2[0], true);
        assert_eq!(p2[1], false);
    }
}
