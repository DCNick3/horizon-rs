ij_core_workaround!();

use crate::core::sync::atomic::{AtomicI32, AtomicU32};
use crate::core::time::Duration;
use horizon_error::KernelErrorCode;
use horizon_svc::{ArbitrationType, SignalType};

fn futex_addr(futex: &AtomicU32) -> *const AtomicI32 {
    // Syscall uses i32 but it's irrelevant as it only checks for equality
    unsafe { core::mem::transmute::<_, *const AtomicI32>(futex as *const AtomicU32) }
}

pub fn futex_wait(futex: &AtomicU32, expected: u32, timeout: Option<Duration>) -> bool {
    match unsafe {
        horizon_svc::wait_for_address(
            futex_addr(futex),
            ArbitrationType::WaitIfEqual,
            expected as i32,
            timeout,
        )
    }
    .map_err(|e| unsafe { e.try_as::<KernelErrorCode>().unwrap_unchecked() })
    {
        Ok(_) => true,
        Err(e) => match e {
            // the futex did not have the expected value
            KernelErrorCode::InvalidState => true,

            KernelErrorCode::TimedOut => false,

            // some unknown error, let's panic
            e => panic!("futex_wait: {:?}", e),
        },
    }
}

pub fn futex_wake(futex: &AtomicU32) -> bool {
    unsafe {
        horizon_svc::signal_to_address(futex_addr(futex), SignalType::Signal, 0, 1).unwrap();
    }
    // same behavior as FreeBSD, DragonFlyBSD; always telling "false"
    false
}

pub fn futex_wake_all(futex: &AtomicU32) -> bool {
    unsafe {
        horizon_svc::signal_to_address(futex_addr(futex), SignalType::Signal, 0, i32::MAX).unwrap();
    }
    // same behavior as FreeBSD, DragonFlyBSD; always telling "false"
    false
}
