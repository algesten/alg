/// A delay line for a single channel.
///
/// The actual implementation might require RAM, which means we do this
/// as a trait that will be fulfilled in the implementation.
pub trait Delay: Sized + Debug + Default {
    /// The amount of samples in the delay
    fn set_sample_count(&mut self, cutoff: usize);

    /// Read the current index
    fn read(&self) -> f32;

    /// Write the current index and move to next sample
    fn write(&mut self, v: f32);
}

use core::fmt::Debug;

#[cfg(test)]
pub use mem::MemoryDelay;

#[cfg(test)]
mod mem {
    use core::fmt::Debug;

    use super::Delay;

    /// An in-memory version of the [`Delay`] trait
    pub struct MemoryDelay<const N: usize> {
        buffer: Vec<f32>,
        sample_count: usize,
        index: usize,
    }

    impl<const N: usize> Default for MemoryDelay<N> {
        fn default() -> Self {
            Self {
                buffer: vec![0.0; N],
                sample_count: N,
                index: 0,
            }
        }
    }

    impl<const N: usize> Debug for MemoryDelay<N> {
        fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
            let mut x = [0.0; 100];
            x.copy_from_slice(&self.buffer[2000..2100]);
            f.debug_struct("MemoryDelay")
                .field("sample_count", &self.sample_count)
                .field("index", &self.index)
                // .field("buffer", &x)
                .finish()
        }
    }

    impl<const N: usize> Delay for MemoryDelay<N> {
        fn set_sample_count(&mut self, sample_count: usize) {
            if sample_count > N {
                panic!("Sample count for Delay must be < N");
            }

            self.sample_count = sample_count;
        }

        fn read(&self) -> f32 {
            self.buffer[self.index]
        }

        fn write(&mut self, v: f32) {
            self.buffer[self.index] = v;

            self.index += 1;
            if self.index >= self.sample_count {
                self.index = 0;
            }
        }
    }
}
