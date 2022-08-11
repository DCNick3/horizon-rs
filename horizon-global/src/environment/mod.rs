ij_core_workaround!();

#[cfg(feature = "impl")]
mod r#impl;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(C)]
pub enum EnvironmentType {
    Nro,
    Nso,
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(C)]
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
#[repr(C)]
pub struct Environment {
    pub environment_type: EnvironmentType,
    pub main_thread_handle: u32,
    pub hos_version: HorizonVersion,
}

#[cfg(feature = "impl")]
pub use r#impl::init;

extern "Rust" {
    fn __horizon_global_environment_get() -> Environment;
}

pub fn get() -> Environment {
    unsafe { __horizon_global_environment_get() }
}
