use core::arch::asm;
use core::mem::MaybeUninit;

/// Get a (mutable) reference to thread-local IPC buffer
///  
/// # Safety
///
/// Do not use it to get two mutable references to the IPC buffer
/// Do not store it's result across IPC calls
pub unsafe fn get_ipc_buffer_mut() -> &'static mut [u8] {
    const BUFFER_SIZE: usize = 0x100;

    let buffer_ptr: *mut u8;
    asm! {
        "mrs {}, TPIDRRO_EL0",
        out(reg) buffer_ptr
    };
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
pub unsafe fn get_ipc_buffer() -> &'static [u8] {
    // SAFETY: we return a read-only reference, which is safe
    unsafe { get_ipc_buffer_mut() }
}

/// A type that can be found in IpcBuffer
/// Used to mark IPC requests & response structures in generated code
/// SAFETY: size must be <= 0x100 to fit into the IPC buffer
pub unsafe trait IpcBufferRepr {}

/// Get a typed IPC buffer pointer for a particular repr
pub fn get_ipc_buffer_for<T: IpcBufferRepr>() -> *mut T {
    // SAFETY: it's ok to get a mutable __pointer__
    unsafe { get_ipc_buffer_mut().as_mut_ptr() as *mut T }
}
