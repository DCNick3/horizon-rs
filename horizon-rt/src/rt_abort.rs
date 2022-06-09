use core::arch::asm;
// use horizon_error::ErrorCode;
// use horizon_svc::BreakReason;

// TODO: make it into its own error code module
#[repr(u16)]
pub enum RtAbortReason {
    NotImplemented = 1,

    // relocation code
    DuplicatedDtEntry,
    MissingDtEntry,
    RelaSizeMismatch,
    UnsupportedRelocationType,
}

// const MODULE_CODE: u32 = 390; // TODO: need to to talk to people how to select this number

/// This is a very low-level abort function
#[inline(never)]
pub fn rt_abort(reason: RtAbortReason) {
    // let code = ErrorCode::from_parts(MODULE_CODE, reason as u16 as u32);

    let reason_raw = reason as u16 as u32;

    unsafe {
        asm!("brk #1", in("x0") reason_raw);
    }

    // let _res = unsafe {
    //     horizon_svc::r#break(
    //         BreakReason::PANIC,
    //         &code as *const _ as *const u8,
    //         core::mem::size_of::<ErrorCode>(),
    //     )
    // };
    //
    // I don't think we should survive beyond the abort
    unsafe { horizon_svc::exit_process() }
}
