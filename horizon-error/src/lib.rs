#![no_std]

//! Defines types for handling horizon errors & error codes

mod kernel;

use core::fmt::{Debug, Formatter};

pub use kernel::KernelErrorCode;

const SUCCESS_VALUE: u32 = 0;
const MODULE_BITS: u32 = 9;
const DESCRIPTION_BITS: u32 = 13;

const MODULE_MASK: u32 = !(!0 << MODULE_BITS);

/// this mask is not shifted!
const DESCRIPTION_MASK: u32 = !(!0 << DESCRIPTION_BITS);

/// Represents a horizon error code, also known as Result Code in other parts of the ecosystem
#[derive(Copy, Clone, PartialEq, Eq, Default)]
#[repr(transparent)]
pub struct ErrorCode {
    value: u32,
}

// likely and unlikely from https://gitlab.com/okannen/likely/-/blob/master/src/lib.rs

#[inline(always)]
/// Brings [likely](core::intrinsics::likely) to stable rust.
pub const fn likely(b: bool) -> bool {
    #[allow(clippy::needless_bool)]
    if (1i32).checked_div(if b { 1 } else { 0 }).is_some() {
        true
    } else {
        false
    }
}

#[inline(always)]
/// Brings [unlikely](core::intrinsics::unlikely) to stable rust.
pub const fn unlikely(b: bool) -> bool {
    #[allow(clippy::needless_bool)]
    if (1i32).checked_div(if b { 0 } else { 1 }).is_none() {
        true
    } else {
        false
    }
}

impl ErrorCode {
    pub fn new(value: u32) -> Self {
        assert_eq!(value >> (MODULE_BITS + DESCRIPTION_BITS), 0);
        unsafe { Self::new_unchecked(value) }
    }

    /// # Safety
    /// The value is a valid result code, that is, it doesn't have bits set outside of the used lower 22 bits
    #[inline(always)]
    pub const unsafe fn new_unchecked(value: u32) -> Self {
        Self { value }
    }

    pub const fn from_parts(module: u32, desc: u32) -> Self {
        let value = (module & MODULE_MASK) | (desc & DESCRIPTION_MASK) << MODULE_BITS;

        unsafe { Self::new_unchecked(value) }
    }

    #[inline(always)]
    pub const fn is_success(&self) -> bool {
        self.value == SUCCESS_VALUE
    }

    #[inline(always)]
    pub const fn is_failure(&self) -> bool {
        !self.is_success()
    }

    #[inline(always)]
    pub const fn repr(&self) -> u32 {
        self.value
    }

    #[inline(always)]
    pub const fn get_module(&self) -> u32 {
        // extract MODULE_BITS lest significant bits
        self.value & MODULE_MASK
    }

    #[inline(always)]
    pub const fn get_description(&self) -> u32 {
        // extract DESCRIPTION_BITS bits after MODULE_BITS
        (self.value >> MODULE_BITS) & DESCRIPTION_MASK
    }

    #[inline(always)]
    pub fn try_as<T: ErrorCodeModule>(&self) -> Option<T> {
        if likely(self.get_module() == T::MODULE) {
            Some(T::from_desc(self.get_description()))
        } else {
            None
        }
    }

    #[inline(always)]
    pub fn into_result<T>(self, success: T) -> Result<T> {
        self.into_result_with(|| success)
    }

    #[inline(always)]
    pub fn into_result_with<T>(self, with_success: impl FnOnce() -> T) -> Result<T> {
        if likely(self.is_success()) {
            Ok(with_success())
        } else {
            Err(self)
        }
    }
}

impl Debug for ErrorCode {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        let module = self.get_module();
        let desc = self.get_description();

        write!(f, "{:04}-{:04}", module + 2000, desc)
    }
}

pub trait ErrorCodeModule: Debug {
    const MODULE: u32;

    // this is allowed to panic when the desc code is unknown
    fn from_desc(desc: u32) -> Self;
}

// TODO: macro for defining the ErrorCodeModule's

pub type Result<T> = core::result::Result<T, ErrorCode>;
