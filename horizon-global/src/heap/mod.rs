//! This uses a buddy memory allocator to coarsely distribute memory between dlmalloc (or any other heap) and some other alloc that may be used
//!
//! uwin use-case: get some memory pages to feed it to svc::map_process_code_memory to remap them to arbitrary address inside alias region

ij_core_workaround!();

use core::alloc::Layout;
use core::mem::MaybeUninit;
use core::ptr::NonNull;
use horizon_sync::mutex::Mutex;

mod buddy;

pub use buddy::{AllocationError, AllocationSizeError};

// this allows us to allocate 4 GiB with granularity of 4 KiB pages, which is exactly what we want
const BUDDY_LEVELS: usize = 21;

static mut BUDDY_ALLOCATOR: MaybeUninit<Mutex<buddy::Heap<BUDDY_LEVELS>>> = MaybeUninit::uninit();

/// # Safety
///
/// Must be called only once
/// Must be called before any calls to [allocate] and [deallocate]
/// It's usually called by horizon-rt in early process initialization, so usually you don't call this
pub unsafe fn init(heap_start: *mut u8, heap_size: usize) {
    let heap = buddy::Heap::new(NonNull::new_unchecked(heap_start), heap_size).unwrap();

    BUDDY_ALLOCATOR.write(Mutex::new(heap));
}

/// Allocate a block of memory large enough to contain `layout`,
/// and aligned to `layout`.  This will return an [`AllocationError`]
/// if the alignment is greater than `MIN_HEAP_ALIGN`, or if
/// we can't find enough memory.
///
/// All allocated memory must be passed to `deallocate` with the same
/// `layout` parameter, or else horrible things will happen.
///
/// This is safe only when [init] was called
pub fn allocate(layout: Layout) -> Result<*mut u8, AllocationError> {
    // SAFETY:
    let mut allocator = unsafe { BUDDY_ALLOCATOR.assume_init_ref() }.lock();

    allocator.allocate(layout)
}

/// Deallocate a block allocated using `allocate`.
///
/// This is safe only when [init] was called
///
/// # Safety
/// `ptr` and `layout` must match what was passed to / returned from `allocate`,
/// or our heap will be corrupted.
pub unsafe fn deallocate(ptr: *mut u8, layout: Layout) {
    let mut allocator = BUDDY_ALLOCATOR.assume_init_ref().lock();

    allocator.deallocate(ptr, layout)
}
