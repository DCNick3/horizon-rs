//! This uses a buddy memory allocator to coarsely distribute memory between dlmalloc (or any other heap) and some other alloc that may be used
//!
//! uwin use-case: get some memory pages to feed it to svc::map_process_code_memory to remap them to arbitrary address inside alias region

ij_core_workaround!();

use core::alloc::Layout;

#[cfg(feature = "impl")]
mod r#impl;

#[cfg(feature = "impl")]
pub use r#impl::init;

extern "Rust" {
    fn __horizon_global_heap_allocate(size: usize, alignment: usize) -> *mut u8;
    fn __horizon_global_heap_deallocate(ptr: *mut u8, size: usize, alignment: usize);
}

/// Allocate a block of memory large enough to contain `layout.size`,
/// and aligned to `layout.alignment`.  This will return an [`AllocationError`]
/// if the alignment is greater than `MIN_HEAP_ALIGN`, or if
/// we can't find enough memory.
///
/// All allocated memory must be passed to `deallocate` with the same
/// `layout` parameter, or else horrible things will happen.
pub fn allocate(layout: Layout) -> Result<*mut u8, ()> {
    // SAFETY: TODO
    let res = unsafe { __horizon_global_heap_allocate(layout.size(), layout.align()) };
    if res == core::ptr::null_mut() {
        Err(())
    } else {
        Ok(res)
    }
}

/// Deallocate a block allocated using `allocate`.
///
/// This is safe only when [init] was called
///
/// # Safety
/// `ptr` and `layout` must match what was passed to / returned from `allocate`,
/// or our heap will be corrupted.
pub unsafe fn deallocate(ptr: *mut u8, layout: Layout) {
    __horizon_global_heap_deallocate(ptr, layout.size(), layout.align())
}
