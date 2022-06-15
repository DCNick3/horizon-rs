#![no_std]

// TODO: add a feature that will use global values from std instead of defining them (when they will get exported from std)

pub mod environment;
pub mod virtual_memory;

#[no_mangle]
pub static __HORIZON_ENV_IF_YOU_SEE_THIS_SYMBOL_IN_DUPLICATE_SYMBOL_LINKER_ERROR_YOU_HAVE_MULTIPLE_HORIZON_GLOBAL_CRATES_WHICH_IS_REALLY_BAD: u32 = 1;
