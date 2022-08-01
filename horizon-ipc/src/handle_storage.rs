use crate::cmif::control::{clone_object, close_object};
use alloc::boxed::Box;
use core::fmt::{Debug, Display, Formatter};
use core::marker::PhantomData;
use core::ops::Deref;
use core::ptr::NonNull;
use core::sync::atomic::{AtomicU32, AtomicUsize, Ordering};
use horizon_svc::RawHandle;

/// A type mostly used to represent a handle borrow
/// mostly needed for PooledHandle, other storage types do this as a no-op
///
/// You are not supposed to look inside
pub struct HandleRef<'a, T: HandleStorage> {
    pub handle: RawHandle,
    pub index: u32,
    pub storage: &'a T,
}

impl<'a, T: HandleStorage> Deref for HandleRef<'a, T> {
    type Target = RawHandle;

    #[inline]
    fn deref(&self) -> &Self::Target {
        &self.handle
    }
}

impl<'a, T: HandleStorage> Drop for HandleRef<'a, T> {
    #[inline]
    fn drop(&mut self) {
        self.storage.give_back(self)
    }
}

/// A type that encapsulates a handle, managing its lifetime
///
/// You are not supposed to implement it usually
pub trait HandleStorage: Sized + Display {
    fn get(&self) -> HandleRef<'_, Self>;
    fn give_back(&self, handle: &HandleRef<'_, Self>);
}

pub struct OwnedHandle {
    handle: RawHandle,
}

impl OwnedHandle {
    #[inline]
    pub const fn new(handle: RawHandle) -> Self {
        Self { handle }
    }
    #[inline]
    pub fn as_ref(&self) -> RefHandle<'_> {
        RefHandle {
            handle: self.handle,
            phantom: PhantomData::default(),
        }
    }
    #[inline]
    pub fn leak(self) -> RawHandle {
        self.handle
    }
}

impl Drop for OwnedHandle {
    fn drop(&mut self) {
        close_object(self.handle)
    }
}

impl Debug for OwnedHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "OwnedHandle({})", self)
    }
}

impl Display for OwnedHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:x}", self.handle.0)
    }
}

// NOTE: you can call get multiple times concurrently here, it's not an error
// it is just would be inefficient to use them for IPC calls
impl HandleStorage for OwnedHandle {
    #[inline]
    fn get(&self) -> HandleRef<'_, Self> {
        HandleRef {
            handle: self.handle,
            index: 0,
            storage: self,
        }
    }

    #[inline]
    fn give_back(&self, _: &HandleRef<'_, Self>) {}
}

#[derive(Copy, Clone)]
pub struct RefHandle<'a> {
    handle: RawHandle,
    phantom: PhantomData<&'a ()>,
}

impl RefHandle<'_> {
    /// Create a new RefHandle from a raw handle
    ///
    /// It's caller's responsibility to ensure that the lifetime of the handle is correct
    #[inline]
    pub const fn new(handle: RawHandle) -> Self {
        Self {
            handle,
            phantom: PhantomData {},
        }
    }

    #[inline]
    pub fn inner(&self) -> RawHandle {
        self.handle
    }
}

impl Debug for RefHandle<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "RefHandle({})", self)
    }
}

impl Display for RefHandle<'_> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:x}", self.handle.0)
    }
}

impl HandleStorage for RefHandle<'_> {
    #[inline]
    fn get(&self) -> HandleRef<'_, Self> {
        HandleRef {
            handle: self.handle,
            index: 0,
            storage: self,
        }
    }

    #[inline]
    fn give_back(&self, _: &HandleRef<'_, Self>) {}
}

struct SharedHandleInner {
    refcount: AtomicUsize,
}

/// A reference-counted handle
/// Stores pointer in the struct itself, so IPC access is as efficient as just a raw handle
pub struct SharedHandle {
    inner: NonNull<SharedHandleInner>,
    handle: RawHandle,
}

impl SharedHandle {
    pub fn new(handle: RawHandle) -> Self {
        let inner = Box::new(SharedHandleInner {
            refcount: AtomicUsize::new(1),
        });

        Self {
            inner: NonNull::new(Box::into_raw(inner)).unwrap(),
            handle,
        }
    }
}

unsafe impl Send for SharedHandle {}
unsafe impl Sync for SharedHandle {}

impl Debug for SharedHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "SharedHandle({})", self)
    }
}

impl Display for SharedHandle {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "0x{:x}", self.handle.0)
    }
}

impl HandleStorage for SharedHandle {
    #[inline]
    fn get(&self) -> HandleRef<'_, Self> {
        HandleRef {
            handle: self.handle,
            index: 0,
            storage: self,
        }
    }

    #[inline]
    fn give_back(&self, _: &HandleRef<'_, Self>) {}
}

impl Clone for SharedHandle {
    fn clone(&self) -> Self {
        let inner = unsafe { self.inner.as_ref() };
        inner.refcount.fetch_add(1, Ordering::SeqCst);

        Self {
            inner: self.inner,
            handle: self.handle,
        }
    }
}

impl Drop for SharedHandle {
    fn drop(&mut self) {
        let inner = unsafe { self.inner.as_ref() };
        if inner.refcount.fetch_sub(1, Ordering::SeqCst) != 1 {
            return;
        }
        core::sync::atomic::fence(Ordering::SeqCst);
        close_object(self.handle);
        unsafe { Box::from_raw(self.inner.as_ptr()) };
    }
}

struct PooledHandleInner<const POOL_SIZE: usize> {
    refcount: AtomicUsize,
    used_mask: AtomicU32,
    handles: [RawHandle; POOL_SIZE],
}

/// Implements handle pooling for session objects
/// Should not be used for other handle types obviously
pub struct PooledHandle<const POOL_SIZE: usize = 16> {
    inner: NonNull<PooledHandleInner<POOL_SIZE>>,
}

impl<const POOL_SIZE: usize> PooledHandle<POOL_SIZE> {
    pub fn new(handle: RawHandle) -> Self {
        let mut handles = [RawHandle(0); POOL_SIZE];
        handles[0] = handle;
        for i in 1..POOL_SIZE {
            handles[i] = clone_object(handle);
        }

        let inner = Box::new(PooledHandleInner {
            refcount: AtomicUsize::new(1),
            used_mask: AtomicU32::new(0),
            handles,
        });

        Self {
            inner: NonNull::new(Box::into_raw(inner)).unwrap(),
        }
    }
}

unsafe impl<const POOL_SIZE: usize> Send for PooledHandle<POOL_SIZE> {}
unsafe impl<const POOL_SIZE: usize> Sync for PooledHandle<POOL_SIZE> {}

impl<const POOL_SIZE: usize> Display for PooledHandle<POOL_SIZE> {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "POOL@0x{:x}", self.inner.as_ptr() as usize)
    }
}

impl<const POOL_SIZE: usize> HandleStorage for PooledHandle<POOL_SIZE> {
    // returns a handle from a pool that _most probably_ not used
    // in case of contention used handles may be returned
    // (it is optimized for "no contention" use case)
    fn get(&self) -> HandleRef<'_, Self> {
        assert!(POOL_SIZE <= 32);

        let inner = unsafe { self.inner.as_ref() };

        let found_index = loop {
            let mask = inner.used_mask.load(Ordering::SeqCst);
            let zero_index = mask.leading_ones() as usize;
            if zero_index >= POOL_SIZE {
                // if we have contention - it's okay to return an already used index
                // the IPC request will just block until the previous request finishes (usually quick? problematic if not)
                // it's even fine to have it being given back two times. at first it will be incorrectly marked as free
                //  and may cause blocking on IPC request again
                // but _eventually_ the contention will be resolved and it will be actually free
                break POOL_SIZE - 1;
            }

            let new_mask = mask | (1 << zero_index);
            if let Ok(_) =
                inner
                    .used_mask
                    .compare_exchange(mask, new_mask, Ordering::SeqCst, Ordering::SeqCst)
            {
                break zero_index;
            }
        };

        let handle = inner.handles[found_index];

        HandleRef {
            handle,
            index: found_index as u32,
            storage: &self,
        }
    }

    fn give_back(&self, handle: &HandleRef<'_, Self>) {
        let &HandleRef { index, .. } = handle;
        let index = index as usize;

        let inner = unsafe { self.inner.as_ref() };

        loop {
            let mask = inner.used_mask.load(Ordering::SeqCst);
            let new_mask = mask & !(1 << index);
            if let Ok(_) =
                inner
                    .used_mask
                    .compare_exchange(mask, new_mask, Ordering::SeqCst, Ordering::SeqCst)
            {
                break;
            }
        }
    }
}

impl<const POOL_SIZE: usize> Clone for PooledHandle<POOL_SIZE> {
    fn clone(&self) -> Self {
        let inner = unsafe { self.inner.as_ref() };
        inner.refcount.fetch_add(1, Ordering::SeqCst);

        Self { inner: self.inner }
    }
}

impl<const POOL_SIZE: usize> Drop for PooledHandle<POOL_SIZE> {
    fn drop(&mut self) {
        let inner = unsafe { self.inner.as_ref() };
        if inner.refcount.fetch_sub(1, Ordering::SeqCst) != 1 {
            return;
        }
        core::sync::atomic::fence(Ordering::SeqCst);
        for handle in inner.handles {
            close_object(handle);
        }
        unsafe { Box::from_raw(self.inner.as_ptr()) };
    }
}
