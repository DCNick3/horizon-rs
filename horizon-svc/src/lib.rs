#![no_std]
// hard to specify when dealing with syscalls..
#![allow(clippy::missing_safety_doc)]

//! Defines wrappers around horizon kernel system calls and related types

mod raw;

use bitflags::bitflags;
use core::arch::asm;
use horizon_error::{ErrorCode, Result};

pub type Address = *const u8;
pub type Size = usize;
pub type ThreadEntrypointFn = unsafe extern "C" fn(*mut u8) -> !;
pub type AddressRange = (Address, Size);

bitflags! {
    pub struct MemoryPermission: u32 {
        const READ      = 1 << 0;
        const WRITE     = 1 << 1;
        const EXECUTE   = 1 << 2;
        const DONT_CARE  = 1 << 28;
    }
}

/// # Safety
/// `error_code` is a valid error code according to `ErrorCode::new_unchecked`
unsafe fn pack_result<T>(ok_result: T, error_code: u32) -> Result<T> {
    ErrorCode::new_unchecked(error_code).into_result(ok_result)
}

pub unsafe fn set_heap_size(size: Size) -> Result<Address> {
    let mut address: *mut u8;
    let mut error: u32;

    asm!("svc 0x1", in("x0") size, lateout("w0") error, lateout("x1") address);

    pack_result(address, error)
}

pub unsafe fn set_memory_permission(
    range: AddressRange,
    permission: MemoryPermission,
) -> Result<()> {
    let mut error: u32;

    asm!("svc 0x2", in("x0") range.0, in("x1") range.1, in("w2") permission.bits, lateout("w0") error);

    pack_result((), error)
}

pub unsafe fn map_physical_memory(range: AddressRange) -> Result<()> {
    let mut error: u32;

    asm!("svc 0x2C", in("x0") range.0, in("x1") range.1, lateout("w0") error);

    pack_result((), error)
}
