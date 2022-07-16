#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
mod gen;

pub use gen::*;
pub use gen::*;

#[cfg(feature = "log-ipc-buffers")]
fn hex_dump(buffer: &[u8]) {
    assert_eq!(buffer.len() % 4, 0);
    for w in buffer.chunks(4) {
        let w: [u8; 4] = w.try_into().unwrap();

        eprint!("{:02x}{:02x}{:02x}{:02x} ", w[0], w[1], w[2], w[3]);
    }
}

#[cfg(feature = "log-ipc-buffers")]
fn pre_ipc_hook() {
    let buffer = unsafe { horizon_ipc::buffer::get_ipc_buffer() };
    eprint!("IPC CALL   = ");
    hex_dump(buffer);
    eprintln!();
}

#[cfg(feature = "log-ipc-buffers")]
fn post_ipc_hook() {
    let buffer = unsafe { horizon_ipc::buffer::get_ipc_buffer() };
    eprint!("IPC RESULT = ");
    hex_dump(buffer);
    eprintln!();
}

#[cfg(not(feature = "log-ipc-buffers"))]
fn pre_ipc_hook() {}

#[cfg(not(feature = "log-ipc-buffers"))]
fn post_ipc_hook() {}
