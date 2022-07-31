ij_core_workaround!();

use core::arch::asm;

#[inline(always)]
unsafe fn set_tls_ptr(tls_storage_addr: *mut u8) {
    asm!("msr TPIDR_EL0, {}", in(reg) tls_storage_addr)
}

extern "C" {
    static __tls_image_start: u8;
    static __tls_image_end: u8;
}

pub unsafe fn init(tls_storage_addr: *mut u8) {
    let image_start = core::ptr::addr_of!(__tls_image_start);
    let image_end = core::ptr::addr_of!(__tls_image_end);
    let size = image_end as usize - image_start as usize;

    core::ptr::copy_nonoverlapping(image_start, tls_storage_addr, size);

    set_tls_ptr(tls_storage_addr);
}
