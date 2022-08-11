#![cfg_attr(not(feature = "std"), no_std)]
// we do very black magic here. Maybe there is a better way...
#![cfg_attr(feature = "std", feature(rustc_private, horizon_nx_platform))]
#![deny(rust_2018_idioms)]
#![cfg_attr(feature = "rustc-dep-of-std", feature(no_core), no_core)]

#[cfg(all(feature = "std", feature = "rustc-dep-of-std"))]
compile_error!("You can't have features \"std\" and \"rustc-dep-of-std\" enabled at the same time");

#[cfg(feature = "rustc-dep-of-std")]
#[allow(unused_imports)]
#[macro_use]
extern crate rustc_std_workspace_core as core;

#[cfg(not(feature = "rustc-dep-of-std"))]
#[allow(unused_extern_crates)]
extern crate core;

use cfg_if::cfg_if;

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

// Guard against horizon-global ABI breakage
// The version should be bumped on each ABI-breaking change in horizon-global
// If you are getting "Undefined symbol" errors with this, you should probably update your rust toolchain
macro_rules! abi_version {
    ($abi_version:literal) => {
        mod abi_guard {
            cfg_if::cfg_if! {
                if #[cfg(feature = "impl")] {
                    #[used]
                    #[no_mangle]
                    #[export_name = concat!("__horizon_global_abi_version_", stringify!($abi_version), "_guard")]
                    static __GUARD: u32 = $abi_version;
                } else {
                    extern "Rust" {
                        #[link_name = concat!("__horizon_global_abi_version_", stringify!($abi_version), "_guard")]
                        static __GUARD: u32;
                    }
                    #[no_mangle]
                    pub fn __horizon_global_abi_guard() -> u32 {
                        unsafe { __GUARD }
                    }
                }
            }
        }
    };
}
abi_version!(2);

pub mod environment;
pub mod heap;
pub mod mounts;
pub mod services;
cfg_if! {
    if #[cfg(feature = "impl")] {
        pub mod virtual_memory;
        // Guard against double implementation
        // TODO: guard against ABI-breaking stuff
        #[no_mangle]
        pub static __HORIZON_GLOBAL_IF_YOU_SEE_THIS_SYMBOL_IN_DUPLICATE_SYMBOL_LINKER_ERROR_YOU_HAVE_MULTIPLE_HORIZON_GLOBAL_CRATES_WHICH_IS_REALLY_BAD: u32 = 1;
    }
}
