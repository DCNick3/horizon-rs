use crate::hbl::AbiConfigEntry;
use crate::{rt_abort, RtAbortReason};
use horizon_env::{Environment, EnvironmentType, HorizonVersion};
use horizon_svc::RawHandle;

// SAFETY: only call it once
pub unsafe fn init(
    maybe_abi_cfg_entries_ptr: *const AbiConfigEntry,
    maybe_main_thread_handle: usize,
    _saved_lr: usize,
) {
    let environment_type =
        match !maybe_abi_cfg_entries_ptr.is_null() && (maybe_main_thread_handle == usize::MAX) {
            true => EnvironmentType::Nro,
            false => EnvironmentType::Nso,
        };

    let environment = match environment_type {
        EnvironmentType::Nro => {
            // TODO: read the HBABI keys

            rt_abort(RtAbortReason::NotImplemented)
        }
        EnvironmentType::Nso => {
            if maybe_main_thread_handle == usize::MAX {
                rt_abort(RtAbortReason::NoMainThreadHandleInNsoEnv);
            }
            Environment {
                environment_type,
                main_thread_handle: RawHandle(maybe_main_thread_handle as u32),
                hos_version: HorizonVersion::new(12, 1, 0),
            }
        }
    };

    // TODO: store the _saver_lr somewhere (in case of Nro env) so that we can return to loader later

    horizon_env::init(environment)
}
