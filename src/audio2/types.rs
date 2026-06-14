mod private {
    pub trait Private {}
}
use core::array;

use private::Private;

use super::delay::{Delay, DelayedSource};

pub trait Frame<const C: usize = 2>: Private {
    type StackBuffer;
}

macro_rules! impl_frame {
    ($size:expr) => {
        impl<const C: usize> Private for &[&mut [f32; $size]; C] {}
        impl<const C: usize> Frame<C> for &[&mut [f32; $size]; C] {
            type StackBuffer = [f32; $size];
        }
    };
}

impl_frame!(32);
impl_frame!(64);
impl_frame!(128);
impl_frame!(256);
impl_frame!(512);
impl_frame!(1024);
impl_frame!(2048);

pub trait Source<const C: usize, F: Frame<C>>: Iterator<Item = F> + Sized {
    fn sample_rate(&self) -> u32;

    fn secs_to_sample_count(&self, secs: f32) -> usize {
        (secs * self.sample_rate() as f32) as usize
    }

    fn delay<D: Delay>(self, seconds: &[f32; C]) -> DelayedSource<C, F, Self, D> {
        let sample_counts = array::from_fn(|i| self.secs_to_sample_count(seconds[i]));

        DelayedSource::new(self, sample_counts)
    }
}
