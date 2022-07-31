ij_core_workaround!();

use core::mem::MaybeUninit;

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
    pub main_thread_handle: u32,
    pub hos_version: HorizonVersion,
}

static mut ENVIRONMENT: MaybeUninit<Environment> = MaybeUninit::uninit();

/// # Safety
///
/// Must be called exactly once (you HAVE to call it before using get)
/// Must be called before any calls to [get]
/// It's usually called by horizon-rt in early process initialization, so usually you don't call this
pub unsafe fn init(environment: Environment) {
    ENVIRONMENT.write(environment);
}

/// This is safe only when [init] was called
pub fn get() -> &'static Environment {
    // SAFETY: the [ENVIRONMENT] var should've been initialized via [init] and not modified otherwise
    unsafe { ENVIRONMENT.assume_init_ref() }
}
