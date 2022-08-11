ij_core_workaround!();

use crate::environment::Environment;

static mut ENVIRONMENT: core::mem::MaybeUninit<Environment> = core::mem::MaybeUninit::uninit();

/// Initialize the environment
///
/// # Safety
///
/// Must be called exactly once (you HAVE to call it before using get)
/// Must be called before any calls to [get]
/// It's usually called by horizon-rt in early process initialization, so usually you don't call this
pub unsafe fn init(environment: Environment) {
    ENVIRONMENT.write(environment);
}

/// This is safe only when [init] was called
#[no_mangle]
pub fn __horizon_global_environment_get() -> Environment {
    // return a copy so that an std shim would actually work
    // SAFETY: the [ENVIRONMENT] var should've been initialized via [init] and not modified otherwise
    unsafe { ENVIRONMENT.assume_init_ref() }.clone()
}
