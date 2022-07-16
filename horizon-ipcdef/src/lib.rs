#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
mod gen;

pub use gen::*;
pub use gen::*;

#[cfg(feature = "log-ipc-buffers")]
fn pre_ipc_hook() {
    let buffer = unsafe { horizon_ipc::buffer::get_ipc_buffer() };
    eprintln!("IPC CALL   = {:?}", buffer);
}

#[cfg(feature = "log-ipc-buffers")]
fn post_ipc_hook() {
    let buffer = unsafe { horizon_ipc::buffer::get_ipc_buffer() };
    eprintln!("IPC RESULT = {:?}", buffer);
}

#[cfg(not(feature = "log-ipc-buffers"))]
fn pre_ipc_hook() {}

#[cfg(not(feature = "log-ipc-buffers"))]
fn post_ipc_hook() {}
