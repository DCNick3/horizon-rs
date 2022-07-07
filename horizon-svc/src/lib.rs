#![no_std]
// hard to specify when dealing with syscalls..
#![allow(clippy::missing_safety_doc)]

//! Defines wrappers around horizon kernel system calls and related types

mod raw;

use bitflags::bitflags;
use core::hint::unreachable_unchecked;
use core::sync::atomic::AtomicI32;
use core::time::Duration;
use horizon_error::Result;

pub type Address = *const u8;
pub type Size = usize;
pub type ThreadEntrypointFn = unsafe extern "C" fn(*mut u8) -> !;
pub type AddressRange = (Address, Size);

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash)]
#[repr(transparent)]
pub struct RawHandle(pub u32);

impl core::fmt::Debug for RawHandle {
    fn fmt(&self, fmt: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(fmt, "RawHandle({:08x})", self.0)
    }
}

pub const CURRENT_PROCESS_PSEUDO_HANDLE: RawHandle = RawHandle(0xFFFF8001);
pub const CURRENT_THREAD_PSEUDO_HANDLE: RawHandle = RawHandle(0xFFFF8000);

#[cfg(not(target_pointer_width = "64"))]
compile_error!("Only 64-bit mode is supported");

bitflags! {
    pub struct MemoryPermission: u32 {
        const READ      = 1 << 0;
        const WRITE     = 1 << 1;
        const EXECUTE   = 1 << 2;
        const DONT_CARE  = 1 << 28;
    }
}

bitflags! {
    pub struct BreakReason: u64 {
        const PANIC                  = 0;
        const ASSERT                 = 1;
        const USER                   = 2;
        const PRE_LOAD_DLL           = 3;
        const POST_LOAD_DLL          = 4;
        const PRE_UNLOAD_DLL         = 5;
        const POST_UNLOAD_DLL        = 6;
        const CPP_EXCEPTION          = 7;
        const NOTIFICATION_ONLY_FLAG = 0x80000000;
    }
}

/// Used in [get_info] svc
/// https://switchbrew.org/wiki/SVC#InfoType
pub enum InfoType {
    CoreMask,
    PriorityMask,
    AliasRegionAddress,
    AliasRegionSize,
    HeapRegionAddress,
    HeapRegionSize,
    /// Total memory available(free+used).
    TotalMemorySize,
    /// Total used size of codebin memory + main-thread stack + allocated heap.
    UsedMemorySize,
    DebuggerAttached,
    ResourceLimit,
    IdleTickCount(Option<u64>),
    /// Used to seed usermode PRNGs.
    RandomEntropy(u64),
    /// [2.0.0+]
    AslrRegionAddress,
    /// [2.0.0+]
    AslrRegionSize,
    /// [2.0.0+]
    StackRegionAddress,
    /// [2.0.0+]
    StackRegionSize,
    /// [3.0.0+]
    SystemResourceSizeTotal,
    /// [3.0.0+]
    SystemResourceSizeUsed,
    /// [3.0.0+]
    ProgramId,
    // InitialProcessIdRange not included, as it was supported only by[4.0.0-4.1.0]
    /// [5.0.0+]
    UserExceptionContextAddress,
    /// [6.0.0+]
    TotalNonSystemMemorySize,
    /// [6.0.0+]
    UsedNonSystemMemorySize,
    /// [9.0.0+]
    IsApplication,
    /// [11.0.0+]
    FreeThreadCount,
    /// When 0-3 are passed, gets specific core CPU ticks spent on thread. When None is passed, gets total CPU ticks spent on thread.
    ///
    /// Used to have a different id prior to 12.1.0, so wouldn't work w/o mesosphere before 12.1.0?
    ThreadTickCount(Option<u64>),
    /// [14.0.0+]
    IsSvcPermitted,

    // mesosphere extensions
    MesosphereMetaKernelVersion,
    MesosphereMetaIsKTraceEnabled,
    MesosphereMetaIsSingleStepEnabled,
    MesosphereCurrentProcess,
}

impl InfoType {
    #[rustfmt::skip]
    pub fn into_type_and_subtype(self) -> (u32, u64) {
        match self {
            InfoType::CoreMask =>                           (0, 0),
            InfoType::PriorityMask =>                       (1, 0),
            InfoType::AliasRegionAddress =>                 (2, 0),
            InfoType::AliasRegionSize =>                    (3, 0),
            InfoType::HeapRegionAddress =>                  (4, 0),
            InfoType::HeapRegionSize =>                     (5, 0),
            InfoType::TotalMemorySize =>                    (6, 0),
            InfoType::UsedMemorySize =>                     (7, 0),
            InfoType::DebuggerAttached =>                   (8, 0),
            InfoType::ResourceLimit =>                      (9, 0),
            InfoType::IdleTickCount(core_id) =>             (10, core_id.unwrap_or(-1i64 as u64)),
            InfoType::RandomEntropy(rnd_id) =>         (11, rnd_id),
            InfoType::AslrRegionAddress =>                  (12, 0),
            InfoType::AslrRegionSize =>                     (13, 0),
            InfoType::StackRegionAddress =>                 (14, 0),
            InfoType::StackRegionSize =>                    (15, 0),
            InfoType::SystemResourceSizeTotal =>            (16, 0),
            InfoType::SystemResourceSizeUsed =>             (17, 0),
            InfoType::ProgramId =>                          (18, 0),
            // 19 skipped for             // 19 skipped for 
            InfoType::UserExceptionContextAddress =>        (20, 0),
            InfoType::TotalNonSystemMemorySize =>           (21, 0),
            InfoType::UsedNonSystemMemorySize =>            (22, 0),
            InfoType::IsApplication =>                      (23, 0),
            InfoType::FreeThreadCount =>                    (24, 0),
            InfoType::ThreadTickCount(core_id) =>           (25, core_id.unwrap_or(-1i64 as u64)),
            InfoType::IsSvcPermitted =>                     (26, 0),
            InfoType::MesosphereMetaKernelVersion =>        (65000, 0),
            InfoType::MesosphereMetaIsKTraceEnabled =>      (65000, 1),
            InfoType::MesosphereMetaIsSingleStepEnabled =>  (65000, 2),
            InfoType::MesosphereCurrentProcess =>           (65001, 0),
        }
    }
}

#[repr(u32)]
pub enum ArbitrationType {
    WaitIfLessThan = 0,
    DecrementAndWaitIfLessThan = 1,
    WaitIfEqual = 2,
}

#[repr(u32)]
pub enum SignalType {
    Signal = 0,
    SignalAndIncrementIfEqual = 1,
    SignalAndModifyByWaitingCountIfEqual = 2,
}

pub unsafe fn set_heap_size(size: Size) -> Result<Address> {
    let res = raw::set_heap_size(size as _); // usize -> u64

    res.result.into_result(res.heap_address)
}

pub unsafe fn set_memory_permission(
    (address, size): AddressRange,
    permission: MemoryPermission,
) -> Result<()> {
    raw::set_memory_permission(address, size as _, permission.bits)
        .result
        .into_result(())
}

pub unsafe fn exit_process() -> ! {
    let _ = raw::exit_process();

    unreachable_unchecked()
}

pub fn close_handle(handle: RawHandle) -> Result<()> {
    unsafe { raw::close_handle(handle.0).result.into_result(()) }
}

/// SAFETY: port_name should be zero-terminated
pub unsafe fn connect_to_named_port(port_name: &[u8]) -> Result<RawHandle> {
    debug_assert_eq!(
        port_name[port_name.len() - 1],
        0,
        "port_name should be zero-terminated"
    );

    let r = raw::connect_to_named_port(port_name.as_ptr());

    r.result.into_result(RawHandle(r.session_handle))
}

pub fn send_sync_request(session_handle: RawHandle) -> Result<()> {
    unsafe { raw::send_sync_request(session_handle.0) }
        .result
        .into_result(())
}

/// [buffer] must be 0x1000-aligned
/// NOTICE: yuzu does not support this svc yet =(
pub fn send_sync_request_with_user_buffer(buffer: &[u8], session_handle: RawHandle) -> Result<()> {
    unsafe {
        raw::send_sync_request_with_user_buffer(
            buffer.as_ptr(),
            buffer.len() as u64,
            session_handle.0,
        )
    }
    .result
    .into_result(())
}

pub unsafe fn r#break(reason: BreakReason, buffer_ptr: *const u8, size: usize) -> Result<()> {
    raw::r#break(reason.bits, buffer_ptr as usize as _, size as _)
        .result
        .into_result(())
}

pub fn output_debug_string(message: &[u8]) {
    // this svc has a return type, but it can be ignored I think
    let _ = unsafe { raw::output_debug_string(message.as_ptr(), message.len() as u64) };
}

pub fn get_info(info_type: InfoType, handle: Option<RawHandle>) -> Result<u64> {
    let (info_type, info_sub_type) = info_type.into_type_and_subtype();

    // SAFETY: this syscall should not modify anything, so it's safe??
    let res = unsafe { raw::get_info(info_type, handle.unwrap_or(RawHandle(0)).0, info_sub_type) };

    res.result.into_result(res.info)
}

pub unsafe fn map_physical_memory((address, size): AddressRange) -> Result<()> {
    raw::map_physical_memory(address, size as _)
        .result
        .into_result(())
}

pub unsafe fn unmap_physical_memory((address, size): AddressRange) -> Result<()> {
    raw::unmap_physical_memory(address, size as _)
        .result
        .into_result(())
}

pub unsafe fn wait_for_address(
    address: *const AtomicI32,
    arbitration_type: ArbitrationType,
    expected_value: i32,
    timeout: Option<Duration>,
) -> Result<()> {
    // horizon treats any negative timeout as infinite, so transform None -> -1
    let timeout_ns = timeout
        .and_then(|timeout| {
            // eh, we have to do a lossy conversion from Duration to nanoseconds
            // it's fine though, only VERY long duration (100s of years) can hit the i64 limit
            // treat those cases as "basically infinite" (return None which is "no limit")
            let sub_nanos = timeout.subsec_nanos() as i64;
            let full_secs: Option<i64> = timeout.as_secs().try_into().ok();

            full_secs
                .and_then(|v| v.checked_mul(1_000_000_000))
                .and_then(|v| v.checked_add(sub_nanos))
        })
        .unwrap_or(-1);

    raw::wait_for_address(
        address as *const u8,
        arbitration_type as u32,
        expected_value as u32,
        timeout_ns as u64,
    )
    .result
    .into_result(())
}

pub unsafe fn signal_to_address(
    address: *const AtomicI32,
    signal_type: SignalType,
    value: i32,
    count: i32,
) -> Result<()> {
    raw::signal_to_address(
        address as *const u8,
        signal_type as u32,
        value as u32,
        count as u32,
    )
    .result
    .into_result(())
}
