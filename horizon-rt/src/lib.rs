//! This crate implements the low-level startup code for horizon OS processes'
//!
//! It is the first thing that would run in your process
//!
//! You probably don't want to directly depend on it, libstd already should do this for you

#![no_std]
#![allow(clippy::missing_safety_doc)]
#![deny(rust_2018_idioms)]
#![cfg_attr(feature = "rustc-dep-of-std", feature(no_core), no_core)]

#[cfg(feature = "rustc-dep-of-std")]
#[allow(unused_imports)]
#[macro_use]
extern crate rustc_std_workspace_core as core;

#[cfg(not(feature = "rustc-dep-of-std"))]
#[allow(unused_extern_crates)]
extern crate core;

// See <https://github.com/intellij-rust/intellij-rust/issues/8954>
#[doc(hidden)]
#[macro_export]
macro_rules! ij_core_workaround {
    () => {
        #[cfg(not(feature = "rustc-dep-of-std"))]
        #[allow(unused_extern_crates)]
        extern crate core;

        #[cfg(feature = "rustc-dep-of-std")]
        use core::prelude::rust_2021::*;
    };
}

mod hbl;
mod init;
mod relocate;
mod rt_abort;
mod tls;

use crate::hbl::AbiConfigEntry;
use crate::relocate::{relocate_with_dyn, Dyn};
use crate::rt_abort::{rt_abort, RtAbortReason};
use core::arch::global_asm;
use horizon_global::environment::EnvironmentType;

// define _start
// what happens at the start of a file
// we actually a fairly limited here, must follow this structure https://switchbrew.org/wiki/NRO#Start
global_asm! {
    // put the function to the .text.rrt0 section & mark it as a function in debug info (using .cfi_* crud)
    ".cfi_sections .debug_frame
     .section .text.rrt0, \"ax\", %progbits
     .align 2   
     .global _start
     .type _start,%function
     .cfi_startproc
     _start:",

    // branch over the remaining part to the entrypoint
    "b __horizon_rt_entry",

    ".cfi_endproc
     .size _start, . - _start",

    // mod0 header offset
    ".word __horizon_rt_mod0 - _start",
    // we fill the padding with "HOMEBREW" because why not
    ".ascii \"HOMEBREW\"",
    // restore the original .section value as per https://doc.rust-lang.org/nightly/reference/inline-assembly.html#template-string-arguments
    ".section .text",
}

// define __horizon_rt_entry
// the actually useful entrypoint code
// mostly borrowed from libnx https://github.com/switchbrew/libnx/blob/bb42eab289e3c801ed2338da9bed907b656f4c3a/nx/source/runtime/switch_crt0.s
global_asm! {
    ".cfi_sections .debug_frame
     .global __horizon_rt_entry
     .type __horizon_rt_entry,%function
     .cfi_startproc
     __horizon_rt_entry:",

    // Arguments on NSO entry:
    //   x0=zero                  | x1=main thread handle
    // Arguments on NRO entry (homebrew ABI):
    //   x0=ptr to env context    | x1=UINT64_MAX (-1 aka 0xFFFFFFFFFFFFFFFF)
    // Arguments on user-mode exception entry:
    //   x0=excpt type (non-zero) | x1=ptr to excpt context

    // NRO environment requires us to return to the linker when we are done,
    //              as it uses the same host process for hbmenu and friends
    // NSO environment doesn't, we just call svc::exit_process to kill our process

    // Detect and handle user-mode exceptions first:
    // if (x0 != 0 && x1 != UINT64_MAX) __libnx_exception_entry(<inargs>);
    "cmp  x0, #0
     ccmn x1, #1, #4, ne // #4 = Z
     beq  .Lcrt0_main_entry
     b    __horizon_rt_exception_entry",

    // now handle the usual entry
    ".Lcrt0_main_entry:",

    // Preserve registers across function calls
    "mov x25, x0  // entrypoint argument 0
     mov x26, x1  // entrypoint argument 1
     mov x27, lr // loader return address",

    // set lr to 0xffffffff so that gdb would happily provide a backtrace
    "mov x24, #0
     mvn x24, x24
     mov lr, x24",


    // Get pointer to MOD0 struct (contains offsets to important places)
    "adr x28, __horizon_rt_mod0",

    // so, a summary:
    // x24 -> 0xffff_ffff_ffff_ffff for clearing the lr
    // x25 -> entrypoint argument 0
    // x26 -> entrypoint argument 1
    // x27 -> saved lr (return to loader addr or ??? in case of NSO)
    // x28 -> MOD0 offset

    // Calculate BSS address/size
    "ldp  w8, w9, [x28, #8] // load BSS start/end offset from MOD0
     sub  w9, w9, w8        // calculate BSS size
     add  w9, w9, #7        // round up to 8
     bic  w9, w9, #7        // ^
     add  x8, x28, x8       // fixup the start pointer",

    // Clear the BSS in 8-byte units
    ".Lclear_bss:
     subs w9, w9, #8
     str  xzr, [x8], #8
     bne  .Lclear_bss",

    // Save initial stack pointer to later return to the linker (in case of NRO)
    "mov  x8, sp
     adrp x9, __HORIZON_RT_STACK_TOP
     str  x8, [x9, #:lo12:__HORIZON_RT_STACK_TOP]",

    // same for the lr
    "mov x8, x29
     adrp x9, __HORIZON_RT_SAVED_LR
     str  x8, [x9, #:lo12:__HORIZON_RT_SAVED_LR]",

    // Parse ELF .dynamic section (which applies relocations to our module)
    "adr  x0, _start    // get aslr base
     ldr  w1, [x28, #4] // pointer to .dynamic section from MOD0
     add  x1, x28, x1
     bl   __horizon_rt_relocate",

    // restore LR to be ~0
    "mov lr, x24",

    // Perform system initialization
    "mov  x0, x25 // entrypoint argument 0
     mov  x1, x26 // entrypoint argument 1
     mov  x2, x27 // saved lr
     bl   __horizon_rt_init",

    // restore LR to be ~0
    "mov lr, x24",

    // load addr of TLS storage for the main thread
    "adrp x0, __main_thread_tls_start
     add  x0, x0, #:lo12:__main_thread_tls_start",

    // init TLS for main thread
    "bl __horizon_rt_init_tls",

    // now we are gonna jump to the main function

    // make it return to __horizon_rt_exit
    "adrp lr, __horizon_rt_exit
     add  lr, lr, #:lo12:__horizon_rt_exit",

    // and... go!
    "b main",

    ".cfi_endproc
     .size __horizon_rt_entry, . - __horizon_rt_entry",
}

/// Stores the stack top address when we started the runtime
/// Used to be able to correctly return to homebrew loader on exit
#[no_mangle]
static mut __HORIZON_RT_STACK_TOP: u64 = 0;

#[no_mangle]
static mut __HORIZON_RT_SAVED_LR: u64 = 0;

// called when HOS calls our entrypoint with an exception
#[no_mangle]
pub unsafe extern "C" fn __horizon_rt_exception_entry() {
    // TODO: implement this
    rt_abort(RtAbortReason::NotImplemented)
}

/// called to parse the .dynamic section and perform relocations
/// it's very important that this function does not need any relocations applied to succeed, otherwise we are in trouble :P
/// _hopefully_ it doesn't touch any globals, so it should  be fine
#[no_mangle]
pub unsafe extern "C" fn __horizon_rt_relocate(aslr_base: u64, dynamic_section: u64) {
    relocate_with_dyn(
        aslr_base as usize as *const u8,
        dynamic_section as usize as *const Dyn,
    )
}

/// Initialize TLS for current thread
#[no_mangle]
pub unsafe extern "C" fn __horizon_rt_init_tls(tls_storage_addr: *mut u8) {
    tls::init(tls_storage_addr);
}

/// Perform most of initialization for horizon-global
#[no_mangle]
pub unsafe extern "C" fn __horizon_rt_init(x0: usize, x1: usize, saved_lr: usize) {
    init::init(x0 as *const AbiConfigEntry, x1, saved_lr)
}

/// Clean up the process & return to loader/exit process (depending on the env)
#[no_mangle]
pub unsafe extern "C" fn __horizon_rt_exit(_exit_code: u32) -> ! {
    if horizon_global::environment::get().environment_type == EnvironmentType::Nro {
        // TODO: return to the loader
        rt_abort(RtAbortReason::NotImplemented)
    } else {
        horizon_svc::exit_process()
    }
}

// define the MOD0 header
global_asm! {
    // put it into the .text.mod0 section
    ".section .text.mod0, \"ax\", %progbits
     .align 2",

    ".global __horizon_rt_mod0
     __horizon_rt_mod0:
     .ascii \"MOD0\"
     .word  __dynamic_start      - __horizon_rt_mod0
     .word  __bss_start          - __horizon_rt_mod0
     .word  __bss_end            - __horizon_rt_mod0
     .word  __eh_frame_hdr_start - __horizon_rt_mod0
     .word  __eh_frame_hdr_end   - __horizon_rt_mod0
     .word  0 // \"offset to runtime-generated module object\" (neither needed, used nor supported in homebrew)",

    // restore the original .section value as per https://doc.rust-lang.org/nightly/reference/inline-assembly.html#template-string-arguments
    ".section .text",
}
