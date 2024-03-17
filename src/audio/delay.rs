/// A delay line for a single channel.
///
/// The actual implementation might require RAM, which means we do this
/// as a trait that will be fulfilled in the implementation.
pub trait Delay: Sized + Default {
    /// The amount of samples in the delay
    fn set_sample_count(&mut self, cutoff: usize);

    /// Read the current index
    fn read(&self) -> f32;

    /// Write the current index and move to next sample
    fn write(&mut self, v: f32);
}

/// An in-memory version of the [`Delay`] trait
pub struct MemoryDelay<const N: usize> {
    buffer: [f32; N],
    sample_count: usize,
    index: usize,
}

impl<const N: usize> Default for MemoryDelay<N> {
    fn default() -> Self {
        Self {
            buffer: [0.0; N],
            sample_count: N,
            index: 0,
        }
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
