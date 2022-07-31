//! Provides definitions & client implementations for various horizon sysmodules

#![cfg_attr(not(feature = "std"), no_std)]

mod ext;
mod gen;

#[cfg(feature = "log-ipc-buffers")]
mod log;

pub use gen::*;
pub use gen::*;

#[cfg(feature = "log-ipc-buffers")]
use log::{post_ipc_hook, pre_ipc_hook};

#[cfg(not(feature = "log-ipc-buffers"))]
fn pre_ipc_hook(_name: &str, _handle: horizon_svc::RawHandle) {}

#[cfg(not(feature = "log-ipc-buffers"))]
fn post_ipc_hook(_name: &str, _handle: horizon_svc::RawHandle) {}
