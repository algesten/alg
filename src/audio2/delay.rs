use core::array;
use core::marker::PhantomData;

use super::{Frame, Source};

pub trait Delay {
    fn with_sample_count(count: usize) -> Self;
}

pub struct DelayedSource<const C: usize, F: Frame<C>, S: Source<C, F>, D: Delay> {
    source: S,
    _frame: PhantomData<F>,
    delays: [D; C],
}
impl<D: Delay, const C: usize, F: Frame<C>, S: Source<C, F>> DelayedSource<C, F, S, D> {
    pub(crate) fn new(source: S, sample_counts: [usize; C]) -> Self {
        let delays = array::from_fn(|i| D::with_sample_count(sample_counts[i]));
        Self {
            source,
            delays,
            _frame: PhantomData,
        }
    }
}

impl<const C: usize, F: Frame<C>, S: Source<C, F>, D: Delay> Iterator
    for DelayedSource<C, F, S, D>
{
    type Item = F;

    fn next(&mut self) -> Option<Self::Item> {
        let frame = self.source.next()?;

        Some(frame)
    }
}

impl<const C: usize, F: Frame<C>, S: Source<C, F>, D: Delay> Source<C, F>
    for DelayedSource<C, F, S, D>
{
    fn sample_rate(&self) -> u32 {
        self.source.sample_rate()
    }
}
