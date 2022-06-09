#![no_std]
// hard to specify when dealing with syscalls..
#![allow(clippy::missing_safety_doc)]

//! Defines wrappers around horizon kernel system calls and related types

mod raw;

use bitflags::bitflags;
use core::hint::unreachable_unchecked;
use horizon_error::Result;

pub type Address = *mut u8;
pub type Size = usize;
pub type ThreadEntrypointFn = unsafe extern "C" fn(*mut u8) -> !;
pub type AddressRange = (Address, Size);

#[cfg(not(target_pointer_width = "64"))]
compile_error!("Only 64-bit mode is supported");

bitflags! {
    pub struct MemoryPermission: u32 {
        const READ      = 1 << 0;
        const WRITE     = 1 << 1;
        const EXECUTE   = 1 << 2;
        const DONT_CARE  = 1 << 28;
    }
}

pub unsafe fn set_heap_size(size: Size) -> Result<Address> {
    let res = raw::set_heap_size(size as _); // usize -> u64

    res.result.into_result(res.heap_address)
}

pub unsafe fn set_memory_permission(
    (address, size): AddressRange,
    permission: MemoryPermission,
) -> Result<()> {
    raw::set_memory_permission(address, size as _, permission.bits)
        .result
        .into_result(())
}

pub unsafe fn exit_process() -> ! {
    let _ = raw::exit_process();

    unreachable_unchecked()
}

pub unsafe fn map_physical_memory((address, size): AddressRange) -> Result<()> {
    raw::map_physical_memory(address, size as _)
        .result
        .into_result(())
}

pub unsafe fn unmap_physical_memory((address, size): AddressRange) -> Result<()> {
    raw::unmap_physical_memory(address, size as _)
        .result
        .into_result(())
}
