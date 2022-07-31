ij_core_workaround!();

use crate::hbl::AbiConfigEntry;
use crate::{rt_abort, RtAbortReason};
use horizon_error::Result;
use horizon_global::environment::{Environment, EnvironmentType, HorizonVersion};
use horizon_global::mounts::MountDevice;
use horizon_global::virtual_memory::{MemoryMap, MemoryRegion};
use horizon_ipcdef::sm::{IUserInterface, ServiceName};
use horizon_svc::{InfoType, CURRENT_PROCESS_PSEUDO_HANDLE};

use crate::rt_abort::rt_unwrap;
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

fn make_heap() -> Result<(*mut u8, usize)> {
    let total_memory = horizon_svc::get_info(
        InfoType::TotalMemorySize,
        Some(CURRENT_PROCESS_PSEUDO_HANDLE),
    )? as usize;
    let used_memory = horizon_svc::get_info(
        InfoType::UsedMemorySize,
        Some(CURRENT_PROCESS_PSEUDO_HANDLE),
    )? as usize;

    const HEAP_GRANULARITY: usize = 2 * 1024 * 1024; // 2 MiB

    // WTF (copied over from libnx)
    let size = if total_memory > used_memory + HEAP_GRANULARITY {
        // if we have enough memory - try to use all the free memory
        (total_memory - used_memory - HEAP_GRANULARITY) & !(HEAP_GRANULARITY - 1)
    } else {
        // else allocate 32 MiB ???
        16 * HEAP_GRANULARITY
    };

    let heap_addr = unsafe { horizon_svc::set_heap_size(size) }? as *mut u8;

    Ok((heap_addr, size))
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

    let (environment, heap) = match environment_type {
        EnvironmentType::Nro => {
            // TODO: read the HBABI keys

            rt_abort(RtAbortReason::NotImplemented)
        }
        EnvironmentType::Nso => {
            if maybe_main_thread_handle == usize::MAX {
                rt_abort(RtAbortReason::NoMainThreadHandleInNsoEnv);
            }
            let heap = match make_heap() {
                Ok(heap) => heap,
                Err(_) => rt_abort(RtAbortReason::MakeHeapFailed),
            };

            (
                Environment {
                    environment_type,
                    main_thread_handle: maybe_main_thread_handle as u32,
                    hos_version: HorizonVersion::new(12, 1, 0),
                },
                heap,
            )
        }
    };

    // TODO: store the _saver_lr somewhere (in case of Nro env) so that we can return to loader later

    let memory_map = rt_unwrap(get_memory_map(), RtAbortReason::MemoryMapReadFailed);

    // TODO: BAD BAD BAD
    // here we align the heap size to the nearest power of two, because this buddy allocator can't handle it
    // I __think__ though that the allocator can be adapted to support such sizes, it's just hard to implement
    // This usually means that we will be limited to 2 GiB of heap
    let heap = {
        let (addr, size): (*mut u8, usize) = heap;

        // min size is 4096 = 2 ** 12
        // but in the worst case the size_power > size will trigger immediately and the size_power will be divided by two
        // so the resulting size will always be >= 4096
        let mut size_power: usize = 1 << 13;
        let size = loop {
            if size_power > size {
                break size_power / 2;
            }
            size_power *= 2;
        };

        (addr, size)
    };

    let sm_session = rt_unwrap(
        IUserInterface::open_named_port(),
        RtAbortReason::SmOpenNamedPortFailed,
    );

    rt_unwrap(sm_session.initialize(), RtAbortReason::SmInitializeFailed);

    let fs_session = rt_unwrap(
        sm_session.get_service(ServiceName::try_new("fsp-srv").unwrap_unchecked()),
        RtAbortReason::FsOpenFailed,
    );
    let fs_session = horizon_ipcdef::fssrv::IFileSystemProxy::new(fs_session);

    let sd_fs = rt_unwrap(
        fs_session.open_sd_card_file_system(),
        RtAbortReason::SdFsOpenFailed,
    );

    horizon_global::environment::init(environment);
    horizon_global::virtual_memory::init(memory_map);
    horizon_global::heap::init(heap.0, heap.1);
    horizon_global::mounts::init();
    horizon_global::sm_session::init(sm_session.into_inner());

    rt_unwrap(
        horizon_global::mounts::write().add("sdmc", MountDevice::IFileSystem(sd_fs.into_inner())),
        RtAbortReason::SdFsMountFailed,
    );
}
