ij_core_workaround!();

use core::mem::MaybeUninit;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct MemoryRegion {
    pub start: *const u8,
    pub size: usize,
}

pub struct MemoryMap {
    /// This region should contain all other regions (I think?)
    /// TODO: doc
    pub aslr_region: MemoryRegion,
    /// This region has stacks mapped into it??
    /// TODO: doc
    pub stack_region: MemoryRegion,
    /// This region is also sometimes known as "Reserved"
    /// It may be used to map physical memory there, as well as map "aliases" - memory that is already mapped other place
    pub alias_region: MemoryRegion,
    /// This is a memory region that will be used to map heap when using svc::set_heap_size
    pub heap_region: MemoryRegion,
}

static mut MEMORY_MAP: MaybeUninit<MemoryMap> = MaybeUninit::uninit();

// TODO: store memory reservations. need locks and (maybe) allocation

/// # Safety
///
/// Must be called only once
/// Must be called before any calls to [get_memory_map]
/// It's usually called by horizon-rt in early process initialization, so usually you don't call this
pub unsafe fn init(map: MemoryMap) {
    MEMORY_MAP.write(map);
}

/// This is safe only when [init] was called
pub fn get_memory_map() -> &'static MemoryMap {
    // SAFETY: the [MEMORY_MAP] var should've been initialized via [init] and not modified otherwise
    unsafe { MEMORY_MAP.assume_init_ref() }
}
