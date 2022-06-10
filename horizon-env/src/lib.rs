#![no_std]

use core::mem::MaybeUninit;

use horizon_svc::RawHandle;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub enum EnvironmentType {
    Nro,
    Nso,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct HorizonVersion {
    pub major: u8,
    pub minor: u8,
    pub micro: u8,
}

impl HorizonVersion {
    pub fn new(major: u8, minor: u8, micro: u8) -> Self {
        Self {
            major,
            minor,
            micro,
        }
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
pub struct Environment {
    pub environment_type: EnvironmentType,
    pub main_thread_handle: RawHandle,
    pub hos_version: HorizonVersion,
}

static mut ENVIRONMENT: MaybeUninit<Environment> = MaybeUninit::uninit();

/// # Safety
///
/// Must be called only once
/// Must be called before any calls to [get_environment]
/// It's usually called by horizon-rt in early process initialization, so usually you don't call this
pub unsafe fn init(environment: Environment) {
    ENVIRONMENT.write(environment);
}

pub fn get_environment() -> &'static Environment {
    // SAFETY: the ENVIRONMENT var is not mutated after calls to get_environment
    unsafe { ENVIRONMENT.assume_init_ref() }
}
