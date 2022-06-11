//! Single threaded executor

use core::task::Poll;
use core::task::{Context, Waker};
use core::{future::Future, task::RawWaker};
use core::{pin::Pin, task::RawWakerVTable};
use slab::Slab;

/// Run an executor over the "root future" given. Any additional
/// futures must be added as children using [`zip`] etc.
pub fn executor<F: Future>(mut future: F) -> F::Output {
    // These tasks are allocated _on the stack_, and mustn't move for the
    // duration of running  this executor to finish. Wakers created from
    // these tasks have pointers to this stack position.
    let mut tasks = Tasks::new(1); // NB size 1 until we can do allocation

    let waker = tasks.next_raw_waker();
    let waker = unsafe { Waker::from_raw(waker) };
    let mut cx = Context::from_waker(&waker);

    loop {
        // Unsafe: We "own" this instance of impl Future, and will not move it
        // while running it to completion.
        match unsafe { Pin::new_unchecked(&mut future) }.poll(&mut cx) {
            Poll::Pending => continue,
            Poll::Ready(v) => return v,
        }
    }
}

/// Controls all wakers. Each entry is a waker refcount.
struct Tasks {
    /// Waker instances at fixed places in memory. The whole Tasks instance
    /// must be fixed in place to keep the validity of the wakers issued.
    wakers: Slab<Wok>,
}

impl Tasks {
    fn new(size: usize) -> Self {
        Tasks {
            wakers: Slab::with_capacity(size),
        }
    }

    /// Issue a new waker. Panics if we have run out.
    fn next_raw_waker(&mut self) -> RawWaker {
        if self.wakers.len() == self.wakers.capacity() - 1 {
            panic!("Too many wakers");
        }

        let ptr = self as *mut Tasks;

        let entry = self.wakers.vacant_entry();
        let key = entry.key();
        let w = Wok { ptr, key, count: 1 };
        entry.insert(w);

        self.wakers.get(key).unwrap().as_raw_waker()
    }

    fn remove_waker(&mut self, key: usize) {
        self.wakers.remove(key);
    }
}

struct Wok {
    ptr: *mut Tasks,
    key: usize,
    count: usize,
}

impl Wok {
    fn as_raw_waker(&self) -> RawWaker {
        RawWaker::new(self as *const Wok as *const (), vtable())
    }
}

fn vtable() -> &'static RawWakerVTable {
    &RawWakerVTable::new(vt_clone, vt_wake, vt_wake_by_ref, vt_drop)
}

/// Unsafe: We expect the Wok pointer to exist for the lifetime of the RawWaker.
/// This requires the instance of Tasks to not move.
unsafe fn vt_clone(p: *const ()) -> RawWaker {
    let wok = &mut *(p as *mut Wok);
    wok.count += 1;
    wok.as_raw_waker()
}

/// Unsafe: See vt_clone.
unsafe fn vt_wake(_p: *const ()) {
    //
}

/// Unsafe: See vt_clone.
unsafe fn vt_wake_by_ref(_p: *const ()) {
    //
}

/// Unsafe: See vt_clone.
unsafe fn vt_drop(p: *const ()) {
    let wok = &mut *(p as *mut Wok);
    wok.count -= 1;

    if wok.count == 0 {
        let wakers = &mut (*wok.ptr);
        wakers.remove_waker(wok.key);
    }
}

/// "Zip" two futures together and poll them one after another. The first future is
/// always polled first. The resulting future only exits when both contained futured
/// are ready.
pub fn zip(future1: impl Future + Unpin, future2: impl Future + Unpin) -> impl Future {
    ZipFuture::new(future1, future2)
}

struct ZipFuture<F1, F2>(Option<F1>, Option<F2>);

impl<F1: Future + Unpin, F2: Future + Unpin> ZipFuture<F1, F2> {
    fn new(future1: F1, future2: F2) -> Self {
        ZipFuture(Some(future1), Some(future2))
    }
}

impl<F1: Future + Unpin, F2: Future + Unpin> Future for ZipFuture<F1, F2> {
    type Output = ();

    fn poll(self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<Self::Output> {
        let this = self.get_mut();

        if let Some(future1) = this.0.as_mut() {
            if let Poll::Ready(_) = Pin::new(future1).poll(cx) {
                this.0.take(); // end polling future 1
            }
        }

        if let Some(future2) = this.1.as_mut() {
            if let Poll::Ready(_) = Pin::new(future2).poll(cx) {
                this.1.take(); // end polling future 2
            }
        }

        // Any still pending?
        if this.0.is_some() || this.1.is_some() {
            Poll::Pending
        } else {
            Poll::Ready(())
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_simple_task() {
        async fn xtest() -> usize {
            42
        }

        {
            let x = executor(xtest());
            assert_eq!(x, 42);
        }

        {
            let x = executor(async { xtest().await });
            assert_eq!(x, 42);
        }
    }
}
