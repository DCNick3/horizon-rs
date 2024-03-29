//! This a vendored copy of <https://github.com/DrChat/buddyalloc/blob/4fd906fd901da8161c9257942782e6bf899bc0eb/src/heap.rs>
//!
//! A simple heap based on a buddy allocator.  For the theory of buddy
//! allocators, see <https://en.wikipedia.org/wiki/Buddy_memory_allocation>
//!
//! The basic idea is that our heap size is a power of two, and the heap
//! starts out as one giant free block.  When a memory allocation request
//! is received, we round the requested size up to a power of two, and find
//! the smallest available block we can use.  If the smallest free block is
//! too big (more than twice as big as the memory we want to allocate), we
//! split the smallest free block in half recursively until it's the right
//! size.  This simplifies a lot of bookkeeping, because all our block
//! sizes are a power of 2, which makes it easy to have one free list per
//! block size.
ij_core_workaround!();

use core::alloc::Layout;
use core::cmp::{max, min};
use core::mem::size_of;
use core::ptr::{self, NonNull};
use core::result::Result;

pub const fn log2(n: usize) -> u8 {
    let mut temp = n;
    let mut result = 0;
    temp >>= 1;
    while temp != 0 {
        result += 1;
        temp >>= 1;
    }
    result
}

const MIN_HEAP_ALIGN: usize = 4096;

/// Represents an error for an allocation's size.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllocationSizeError {
    BadAlignment,
    TooLarge,
}

/// Represents the reason for an allocation error.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum AllocationError {
    HeapExhausted,
    InvalidSize(AllocationSizeError),
}

/// An error in the creation of the heap.
#[derive(Clone, Copy, PartialEq, Eq, Debug)]
pub enum HeapError {
    BadBaseAlignment,
    BadSizeAlignment,
    BadHeapSize,
    MinBlockTooSmall,
}

/// A free block in our heap.  This is actually a header that we store at
/// the start of the block.  We don't store any size information in the
/// header, because we allocate a separate free block list for each block
/// size.
struct FreeBlock {
    /// The next block in the free list, or NULL if this is the final
    /// block.
    next: *mut FreeBlock,
}

impl FreeBlock {
    /// Construct a `FreeBlock` header pointing at `next`.
    const fn new(next: *mut FreeBlock) -> FreeBlock {
        FreeBlock { next }
    }
}

/// The interface to a heap.  This data structure is stored _outside_ the
/// heap somewhere, typically in a static variable, because every single
/// byte of our heap is potentially available for allocation.
///
/// The generic parameter N specifies the number of steps to divide the
/// available heap size by two. This will be the minimum allocable block size.
///
/// # Usage
/// ```no_run
/// # use buddyalloc::Heap;
/// # use core::{alloc::Layout, ptr::NonNull};
/// // This can be a block of free system memory on your microcontroller.
/// const HEAP_MEM: usize  = 0xFFF0_0000;
/// const HEAP_SIZE: usize = 0x0008_0000;
///
/// let mut heap: Heap<16> = unsafe {
///     Heap::new(NonNull::new(HEAP_MEM as *mut u8).unwrap(), HEAP_SIZE).unwrap()
/// };
/// let mem = heap.allocate(Layout::from_size_align(16, 16).unwrap()).unwrap();
///
/// // Yay! We have a 16-byte block of memory from the heap.
/// ```
///
/// # Usage (static initialization)
/// ```no_run
/// # use buddyalloc::Heap;
/// # use core::{alloc::Layout, ptr::NonNull};
/// const HEAP_MEM: usize  = 0xFFF0_0000;
/// const HEAP_SIZE: usize = 0x0008_0000;
///
/// // You'll want to wrap this heap in a lock abstraction for real-world use.
/// static mut ALLOCATOR: Heap<16> = unsafe {
///     Heap::new_unchecked(HEAP_MEM as *mut u8, HEAP_SIZE)
/// };
///
/// pub fn some_func() {
///   let mem = unsafe {
///     ALLOCATOR.allocate(Layout::from_size_align(16, 16).unwrap()).unwrap()
///   };
///
///   // Yay! We now have a 16-byte block from the heap without initializing it!
/// }
/// ```
#[derive(Debug)]
pub struct Heap<const N: usize> {
    /// The base address of our heap.  This must be aligned on a
    /// `MIN_HEAP_ALIGN` boundary.
    heap_base: *mut u8,

    /// The space available in our heap.  This must be a power of 2.
    heap_size: usize,

    /// The free lists for our heap.  The list at `free_lists[0]` contains
    /// the smallest block size we can allocate, and the list at the end
    /// can only contain a single free block the size of our entire heap,
    /// and only when no memory is allocated.
    free_lists: [*mut FreeBlock; N],

    /// Our minimum block size.  This is calculated based on `heap_size`
    /// and the generic parameter N, and it must be
    /// big enough to contain a `FreeBlock` header object.
    min_block_size: usize,

    /// The log base 2 of our block size.  Cached here so we don't have to
    /// recompute it on every allocation (but we haven't benchmarked the
    /// performance gain).
    min_block_size_log2: u8,
}

// This structure can safely be sent between threads.
unsafe impl<const N: usize> Send for Heap<N> {}

impl<const N: usize> Heap<N> {
    /// Create a new heap. If any parameter is invalid, this will return a [HeapError].
    pub unsafe fn new(heap_base: NonNull<u8>, heap_size: usize) -> Result<Self, HeapError> {
        // Calculate our minimum block size based on the number of free
        // lists we have available.
        let min_block_size = heap_size >> (N - 1);

        // The heap must be aligned on a 4K boundary.
        if heap_base.as_ptr() as usize & (MIN_HEAP_ALIGN - 1) != 0 {
            return Err(HeapError::BadBaseAlignment);
        }

        // The heap must be big enough to contain at least one block.
        if heap_size < min_block_size {
            return Err(HeapError::BadHeapSize);
        }

        // The smallest possible heap block must be big enough to contain
        // the block header.
        if min_block_size < size_of::<FreeBlock>() {
            return Err(HeapError::MinBlockTooSmall);
        }

        // The heap size must be a power of 2.
        if !heap_size.is_power_of_two() {
            return Err(HeapError::BadSizeAlignment);
        }

        // We must have one free list per possible heap block size.
        // FIXME: Can this assertion even be hit?
        // assert_eq!(
        //     min_block_size * (2u32.pow(N as u32 - 1)) as usize,
        //     heap_size
        // );

        // assert!(N > 0);
        Ok(Self::new_unchecked(heap_base.as_ptr(), heap_size))
    }

    /// Create a new heap without checking for parameter validity.
    /// Useful for const heap creation.
    ///
    /// # Safety
    /// `heap_base` must be aligned on a
    /// `MIN_HEAP_ALIGN` boundary, `heap_size` must be a power of 2, and
    /// `heap_size / 2.pow(free_lists.len()-1)` must be greater than or
    /// equal to `size_of::<FreeBlock>()`.  Passing in invalid parameters
    /// may do horrible things.
    pub const unsafe fn new_unchecked(heap_base: *mut u8, heap_size: usize) -> Self {
        // Calculate our minimum block size based on the number of free
        // lists we have available.
        let min_block_size = heap_size >> (N - 1);
        let mut free_lists: [*mut FreeBlock; N] = [ptr::null_mut(); N];

        // Insert the entire heap into the last free list.
        // See the documentation for `free_lists` - the last entry contains
        // the entire heap iff no memory is allocated.
        free_lists[N - 1] = heap_base as *mut FreeBlock;

        // Store all the info about our heap in our struct.
        Self {
            heap_base: heap_base,
            heap_size,
            free_lists,
            min_block_size,
            min_block_size_log2: log2(min_block_size),
        }
    }

    /// Figure out what size block we'll need to fulfill an allocation
    /// request.  This is deterministic, and it does not depend on what
    /// we've already allocated.  In particular, it's important to be able
    /// to calculate the same `allocation_size` when freeing memory as we
    /// did when allocating it, or everything will break horribly.
    fn allocation_size(&self, mut size: usize, align: usize) -> Result<usize, AllocationSizeError> {
        // Sorry, we don't support weird alignments.
        if !align.is_power_of_two() {
            return Err(AllocationSizeError::BadAlignment);
        }

        // We can't align any more precisely than our heap base alignment
        // without getting much too clever, so don't bother.
        if align > MIN_HEAP_ALIGN {
            return Err(AllocationSizeError::BadAlignment);
        }

        // We're automatically aligned to `size` because of how our heap is
        // sub-divided, but if we need a larger alignment, we can only do
        // it be allocating more memory.
        if align > size {
            size = align;
        }

        // We can't allocate blocks smaller than `min_block_size`.
        size = max(size, self.min_block_size);

        // Round up to the next power of two.
        size = size.next_power_of_two();

        // We can't allocate a block bigger than our heap.
        if size > self.heap_size {
            return Err(AllocationSizeError::TooLarge);
        }

        Ok(size)
    }

    /// The "order" of an allocation is how many times we need to double
    /// `min_block_size` in order to get a large enough block, as well as
    /// the index we use into `free_lists`.
    fn allocation_order(&self, size: usize, align: usize) -> Result<usize, AllocationSizeError> {
        self.allocation_size(size, align)
            .map(|s| (log2(s) - self.min_block_size_log2) as usize)
    }

    /// The size of the blocks we allocate for a given order.
    const fn order_size(&self, order: usize) -> usize {
        1 << (self.min_block_size_log2 as usize + order)
    }

    /// Pop a block off the appropriate free list.
    fn free_list_pop(&mut self, order: usize) -> Option<*mut u8> {
        let candidate = self.free_lists[order];
        if !candidate.is_null() {
            // N.B: If this is the entry corresponding to the entire heap,
            // the next entry is always going to be NULL. Special-case it here
            // to allow for uninitialized initial data.
            if order != self.free_lists.len() - 1 {
                self.free_lists[order] = unsafe { (*candidate).next };
            } else {
                self.free_lists[order] = ptr::null_mut();
            }

            Some(candidate as *mut u8)
        } else {
            None
        }
    }

    /// Insert `block` of order `order` onto the appropriate free list.
    unsafe fn free_list_insert(&mut self, order: usize, block: *mut u8) {
        let free_block_ptr = block as *mut FreeBlock;
        *free_block_ptr = FreeBlock::new(self.free_lists[order]);
        self.free_lists[order] = free_block_ptr;
    }

    /// Attempt to remove a block from our free list, returning true
    /// success, and false if the block wasn't on our free list.  This is
    /// the slowest part of a primitive buddy allocator, because it runs in
    /// O(log N) time where N is the number of blocks of a given size.
    ///
    /// We could perhaps improve this by keeping our free lists sorted,
    /// because then "nursery generation" allocations would probably tend
    /// to occur at lower addresses and then be faster to find / rule out
    /// finding.
    fn free_list_remove(&mut self, order: usize, block: *mut u8) -> bool {
        let block_ptr = block as *mut FreeBlock;

        // Yuck, list traversals are gross without recursion.  Here,
        // `*checking` is the pointer we want to check, and `checking` is
        // the memory location we found it at, which we'll need if we want
        // to replace the value `*checking` with a new value.
        let mut checking: &mut *mut FreeBlock = &mut self.free_lists[order];

        // Loop until we run out of free blocks.
        while !(*checking).is_null() {
            // Is this the pointer we want to remove from the free list?
            if *checking == block_ptr {
                // Yup, this is the one, so overwrite the value we used to
                // get here with the next one in the sequence.
                *checking = unsafe { (*(*checking)).next };
                return true;
            }

            // Haven't found it yet, so point `checking` at the address
            // containing our `next` field.  (Once again, this is so we'll
            // be able to reach back and overwrite it later if necessary.)
            checking = unsafe { &mut ((*(*checking)).next) };
        }
        false
    }

    /// Split a `block` of order `order` down into a block of order
    /// `order_needed`, placing any unused chunks on the free list.
    ///
    /// # Safety
    /// The block must be owned by this heap, otherwise bad things
    /// will happen.
    unsafe fn split_free_block(&mut self, block: *mut u8, mut order: usize, order_needed: usize) {
        // Get the size of our starting block.
        let mut size_to_split = self.order_size(order);

        // Progressively cut our block down to size.
        while order > order_needed {
            // Update our loop counters to describe a block half the size.
            size_to_split >>= 1;
            order -= 1;

            // Insert the "upper half" of the block into the free list.
            let split = block.add(size_to_split);
            self.free_list_insert(order, split);
        }
    }

    /// Given a `block` with the specified `order`, find the "buddy" block,
    /// that is, the other half of the block we originally split it from,
    /// and also the block we could potentially merge it with.
    fn buddy(&self, order: usize, block: *mut u8) -> Option<*mut u8> {
        assert!(block >= self.heap_base);

        let relative = unsafe { block.offset_from(self.heap_base) } as usize;
        let size = self.order_size(order);
        if size >= self.heap_size {
            // The main heap itself does not have a budy.
            None
        } else {
            // Fun: We can find our buddy by xoring the right bit in our
            // offset from the base of the heap.
            Some(unsafe { self.heap_base.add(relative ^ size) })
        }
    }

    /// Allocate a block of memory large enough to contain `layout`,
    /// and aligned to `layout`.  This will return an [`AllocationError`]
    /// if the alignment is greater than `MIN_HEAP_ALIGN`, or if
    /// we can't find enough memory.
    ///
    /// All allocated memory must be passed to `deallocate` with the same
    /// `layout` parameter, or else horrible things will happen.
    pub fn allocate(&mut self, layout: Layout) -> Result<*mut u8, AllocationError> {
        // Figure out which order block we need.
        match self.allocation_order(layout.size(), layout.align()) {
            Ok(order_needed) => {
                // Start with the smallest acceptable block size, and search
                // upwards until we reach blocks the size of the entire heap.
                for order in order_needed..self.free_lists.len() {
                    // Do we have a block of this size?
                    if let Some(block) = self.free_list_pop(order) {
                        // If the block is too big, break it up.  This leaves
                        // the address unchanged, because we always allocate at
                        // the head of a block.
                        if order > order_needed {
                            // SAFETY: The block came from the heap.
                            unsafe { self.split_free_block(block, order, order_needed) };
                        }

                        // We have an allocation, so quit now.
                        return Ok(block);
                    }
                }

                // We couldn't find a large enough block for this allocation.
                Err(AllocationError::HeapExhausted)
            }

            // We can't allocate a block with the specified size and
            // alignment.
            Err(e) => Err(AllocationError::InvalidSize(e)),
        }
    }

    /// Deallocate a block allocated using `allocate`.
    ///
    /// # Safety
    /// `ptr` and `layout` must match what was passed to / returned from `allocate`,
    /// or our heap will be corrupted.
    pub unsafe fn deallocate(&mut self, ptr: *mut u8, layout: Layout) {
        let initial_order = self
            .allocation_order(layout.size(), layout.align())
            .expect("Tried to dispose of invalid block");

        // The fun part: When deallocating a block, we also want to check
        // to see if its "buddy" is on the free list.  If the buddy block
        // is also free, we merge them and continue walking up.
        //
        // `block` is the biggest merged block we have so far.
        let mut block = ptr;
        for order in initial_order..self.free_lists.len() {
            // Would this block have a buddy?
            if let Some(buddy) = self.buddy(order, block) {
                // Is this block's buddy free?
                if self.free_list_remove(order, buddy) {
                    // Merge them!  The lower address of the two is the
                    // newly-merged block.  Then we want to try again.
                    block = min(block, buddy);
                    continue;
                }
            }

            // If we reach here, we didn't find a buddy block of this size,
            // so take what we've got and mark it as free.
            self.free_list_insert(order, block);
            return;
        }
    }
}
