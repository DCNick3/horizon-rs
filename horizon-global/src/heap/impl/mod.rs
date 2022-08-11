ij_core_workaround!();

use crate::core::mem::MaybeUninit;
use crate::core::ptr::NonNull;
use core::alloc::Layout;
use horizon_sync::mutex::Mutex;

mod buddy;

// this allows us to allocate 4 GiB with granularity of 4 KiB pages, which is exactly what we want
const BUDDY_LEVELS: usize = 21;

static mut BUDDY_ALLOCATOR: MaybeUninit<Mutex<buddy::Heap<BUDDY_LEVELS>>> = MaybeUninit::uninit();

/// Initialize the heap
///
/// # Safety
///
/// Must be called only once
/// Must be called before any calls to [allocate] and [deallocate]
/// It's usually called by horizon-rt in early process initialization, so usually you don't call this
pub unsafe fn init(heap_start: *mut u8, heap_size: usize) {
    let heap = buddy::Heap::new(NonNull::new_unchecked(heap_start), heap_size).unwrap();

    BUDDY_ALLOCATOR.write(Mutex::new(heap));
}

/// Allocate memory
///
/// Returns `null` on error
///
/// # Safety
///
/// This is safe only when [init] was called
///
/// size and alignment must be valid
#[no_mangle]
pub fn __horizon_global_heap_allocate(size: usize, alignment: usize) -> *mut u8 {
    let layout = unsafe { Layout::from_size_align_unchecked(size, alignment) };

    let mut allocator = unsafe { BUDDY_ALLOCATOR.assume_init_ref() }.lock();

    allocator.allocate(layout).unwrap_or_else(|_e| {
        // TODO: log or put a breakpoint here?
        core::ptr::null_mut()
    })
}

/// Deallocate memory
///
/// Ignores errors
///
/// # Safety
///
/// This is safe only when [init] was called
///
/// `ptr` must have been previously allocated with [__horizon_global_heap_allocate]
///
/// size and alignment must be valid
#[no_mangle]
pub fn __horizon_global_heap_deallocate(ptr: *mut u8, size: usize, alignment: usize) {
    let layout = unsafe { Layout::from_size_align_unchecked(size, alignment) };

    let mut allocator = unsafe { BUDDY_ALLOCATOR.assume_init_ref() }.lock();

    unsafe { allocator.deallocate(ptr, layout) }
}
