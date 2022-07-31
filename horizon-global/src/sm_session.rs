use crate::core::mem::MaybeUninit;
use horizon_ipc::handle_storage::{OwnedHandle, RefHandle};

static mut SM_SESSION: MaybeUninit<OwnedHandle> = MaybeUninit::uninit();

/// Initialize the sm session
///
/// # Safety
///
/// Must be called exactly once (you HAVE to call it before using get)
/// Must be called before any calls to [get]
/// It's usually called by horizon-rt in early process initialization, so usually you don't call this
pub unsafe fn init(session: OwnedHandle) {
    SM_SESSION.write(session);
}

/// This is safe only when [init] was called
pub fn get() -> RefHandle<'static> {
    // SAFETY: the [SM_SESSION] var should've been initialized via [init] and not modified otherwise
    unsafe { SM_SESSION.assume_init_ref().as_ref() }
}
