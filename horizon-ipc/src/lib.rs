#![no_std]
#![deny(rust_2018_idioms)]

#[cfg(not(feature = "rustc-dep-of-std"))]
extern crate alloc;

pub use horizon_svc::RawHandle;

pub mod buffer;
pub mod cmif;
pub mod conv_traits;
pub mod handle_storage;
pub mod hipc;
pub mod raw;
