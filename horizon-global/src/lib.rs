#![no_std]
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

// TODO: add a feature that will use global values from std instead of defining them (when they will get exported from std)
pub mod environment;
pub mod heap;
pub mod mounts;
pub mod services;
pub mod virtual_memory;

#[no_mangle]
pub static __HORIZON_ENV_IF_YOU_SEE_THIS_SYMBOL_IN_DUPLICATE_SYMBOL_LINKER_ERROR_YOU_HAVE_MULTIPLE_HORIZON_GLOBAL_CRATES_WHICH_IS_REALLY_BAD: u32 = 1;
