pub struct RingBuf<T, const X: usize> {
    data: [Option<T>; X],
    insert: usize,
    remove: usize,
}

impl<T, const X: usize> RingBuf<T, X> {
    pub fn new() -> Self {
        assert!(X > 0);
        let data = unsafe { core::mem::MaybeUninit::<[Option<T>; X]>::zeroed().assume_init() };

        RingBuf {
            data,
            insert: 0,
            remove: 0,
        }
    }

    #[inline(always)]
    pub fn len(&self) -> usize {
        if self.remove <= self.insert {
            self.insert - self.remove
        } else {
            X - self.remove + self.insert
        }
    }

    pub fn push(&mut self, el: T) {
        if self.len() == X - 1 {
            panic!("RingBuf push overflow");
        }
        self.data[self.insert] = Some(el);
        self.insert += 1;
        self.insert %= X;
    }

    pub fn pop(&mut self) -> Option<T> {
        let x = self.data[self.remove].take();
        if x.is_some() {
            self.remove += 1;
            self.remove %= X;
        }
        x
    }
}
