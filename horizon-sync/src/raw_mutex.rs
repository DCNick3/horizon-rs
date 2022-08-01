ij_core_workaround!();

use core::sync::atomic::{
    AtomicU32,
    Ordering::{Acquire, Relaxed, Release},
};

use crate::futex::{futex_wait, futex_wake};

/// The implementation is taken from new rust stdlib mutex based on futex
///
/// svc::wait_for_address and svc::signal_to_address can be easily used to implement futex-like functionality
///
/// It makes this mutex `[4.0.0+]`, but it should be fine with mesosphere
///
/// Some details on how it works can be found in "Futexes Are Tricky" (<https://dept-info.labri.fr/~denis/Enseignement/2008-IR/Articles/01-futex.pdf>)
pub struct RawMutex {
    /// 0: unlocked
    /// 1: locked, no other threads waiting
    /// 2: locked, and other threads waiting (contended)
    pub value: AtomicU32,
}

impl RawMutex {
    #[inline]
    pub const fn new() -> Self {
        Self {
            value: AtomicU32::new(0),
        }
    }

    #[inline]
    pub unsafe fn init(&mut self) {}

    #[inline]
    pub unsafe fn destroy(&self) {}

    #[inline]
    pub unsafe fn try_lock(&self) -> bool {
        self.value.compare_exchange(0, 1, Acquire, Relaxed).is_ok()
    }

    #[inline]
    pub unsafe fn lock(&self) {
        if self.value.compare_exchange(0, 1, Acquire, Relaxed).is_err() {
            self.lock_contended();
        }
    }

    #[cold]
    fn lock_contended(&self) {
        // Spin first to speed things up if the lock is released quickly.
        let mut state = self.spin();

        // If it's unlocked now, attempt to take the lock
        // without marking it as contended.
        if state == 0 {
            match self.value.compare_exchange(0, 1, Acquire, Relaxed) {
                Ok(_) => return, // Locked!
                Err(s) => state = s,
            }
        }

        loop {
            // Put the lock in contended state.
            // We avoid an unnecessary write if it as already set to 2,
            // to be friendlier for the caches.
            if state != 2 && self.value.swap(2, Acquire) == 0 {
                // We changed it from 0 to 2, so we just succesfully locked it.
                return;
            }

            // Wait for the futex to change state, assuming it is still 2.
            futex_wait(&self.value, 2, None);

            // Spin again after waking up.
            state = self.spin();
        }
    }

    fn spin(&self) -> u32 {
        let mut spin = 100;
        loop {
            // We only use `load` (and not `swap` or `compare_exchange`)
            // while spinning, to be easier on the caches.
            let state = self.value.load(Relaxed);

            // We stop spinning when the mutex is unlocked (0),
            // but also when it's contended (2).
            if state != 1 || spin == 0 {
                return state;
            }

            core::hint::spin_loop();
            spin -= 1;
        }
    }

    #[inline]
    pub unsafe fn unlock(&self) {
        if self.value.swap(0, Release) == 2 {
            // We only wake up one thread. When that thread locks the mutex, it
            // will mark the mutex as contended (2) (see lock_contended above),
            // which makes sure that any other waiting threads will also be
            // woken up eventually.
            self.wake();
        }
    }

    #[cold]
    fn wake(&self) {
        futex_wake(&self.value);
    }
}
