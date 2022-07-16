use core::arch::asm;

#[inline]
pub unsafe fn get_ipc_buffer_ptr() -> *mut u8 {
    let buffer_ptr: *mut u8;
    asm! {
        "mrs {}, TPIDRRO_EL0",
        out(reg) buffer_ptr
    };
    buffer_ptr
}

/// Get a (mutable) reference to thread-local IPC buffer
///  
/// # Safety
///
/// Do not use it to get two mutable references to the IPC buffer
/// Do not store it's result across IPC calls
#[inline]
pub unsafe fn get_ipc_buffer_mut() -> &'static mut [u8] {
    const BUFFER_SIZE: usize = 0x100;

    let buffer_ptr = get_ipc_buffer_ptr();
    core::slice::from_raw_parts_mut(buffer_ptr, BUFFER_SIZE)
}

/// Get a read-only reference to thread-local IPC buffer
///
/// Note that it may be unsafe to do while a slice returned by [get_ipc_buffer_mut] is slill alive
///
/// # Safety
///
/// Do not use it to get two mutable references to the IPC buffer
/// Do not store it's result across IPC calls
#[inline]
pub unsafe fn get_ipc_buffer() -> &'static [u8] {
    // SAFETY: we return a read-only reference, which is safe
    get_ipc_buffer_mut()
}
