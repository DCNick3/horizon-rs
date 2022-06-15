use crate::hbl::AbiConfigEntry;
use crate::{rt_abort, RtAbortReason};
use horizon_error::Result;
use horizon_global::environment::{Environment, EnvironmentType, HorizonVersion};
use horizon_global::virtual_memory::{MemoryMap, MemoryRegion};
use horizon_svc::{InfoType, CURRENT_PROCESS_PSEUDO_HANDLE};

use horizon_svc as svc;

fn get_memory_region(start_info: InfoType, size_info: InfoType) -> Result<MemoryRegion> {
    let handle = Some(CURRENT_PROCESS_PSEUDO_HANDLE);

    let start = svc::get_info(start_info, handle)?;
    let size = svc::get_info(size_info, handle)?;

    Ok(MemoryRegion {
        start: start as *const u8,
        size: size as usize,
    })
}

fn get_memory_map() -> Result<MemoryMap> {
    // This is [2.0.0+]
    // not that we care to support lower kernel versions, lol
    let aslr_region = get_memory_region(InfoType::AslrRegionAddress, InfoType::AslrRegionSize)?;

    let stack_region = get_memory_region(InfoType::StackRegionAddress, InfoType::StackRegionSize)?;
    let alias_region = get_memory_region(InfoType::AliasRegionAddress, InfoType::AliasRegionSize)?;
    let heap_region = get_memory_region(InfoType::HeapRegionAddress, InfoType::HeapRegionSize)?;

    Ok(MemoryMap {
        aslr_region,
        stack_region,
        alias_region,
        heap_region,
    })
}

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
                main_thread_handle: maybe_main_thread_handle as u32,
                hos_version: HorizonVersion::new(12, 1, 0),
            }
        }
    };

    // TODO: store the _saver_lr somewhere (in case of Nro env) so that we can return to loader later

    let memory_map = match get_memory_map() {
        Ok(m) => m,
        Err(_) => rt_abort(RtAbortReason::MemoryMapReadFailed),
    };

    horizon_global::environment::init(environment);
    horizon_global::virtual_memory::init(memory_map);
}
