//! Provides definitions & client implementations for various horizon sysmodules

#![cfg_attr(not(feature = "std"), no_std)]
#![deny(rust_2018_idioms)]
#![cfg_attr(feature = "rustc-dep-of-std", feature(no_core), no_core)]

#[cfg(feature = "rustc-dep-of-std")]
#[allow(unused_imports)]
#[macro_use]
extern crate rustc_std_workspace_core as core;

#[cfg(not(feature = "rustc-dep-of-std"))]
#[allow(unused_extern_crates)]
extern crate core;

// See <https://github.com/intellij-rust/intellij-rust/issues/8954>
#[doc(hidden)]
#[macro_export]
macro_rules! ij_core_workaround {
    () => {
        #[cfg(not(feature = "rustc-dep-of-std"))]
        #[allow(unused_extern_crates)]
        extern crate core;

        #[cfg(feature = "rustc-dep-of-std")]
        use core::prelude::rust_2021::*;
    };
}

mod ext;
mod gen;

#[cfg(feature = "log-ipc-buffers")]
mod log;

pub use gen::*;
pub use gen::*;

#[cfg(feature = "log-ipc-buffers")]
use log::{post_ipc_hook, pre_ipc_hook};

#[cfg(not(feature = "log-ipc-buffers"))]
#[inline]
fn pre_ipc_hook(_name: &str, _handle: horizon_svc::RawHandle) {}

#[cfg(not(feature = "log-ipc-buffers"))]
#[inline]
fn post_ipc_hook(_name: &str, _handle: horizon_svc::RawHandle) {}
