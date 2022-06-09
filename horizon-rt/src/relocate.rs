//! Horizon kernel is pretty dumb and does not perform any relocation for us
//! This is why we have to do it manually after being loaded
//! We also should make sure that the relocation routine does not require any relocations
//! This is fairly easy to accomplish on aarch64, but I don't think it's easy to enforce that
//! So things will silently break if it doesn't go according to plan (whoops)

use crate::rt_abort::{rt_abort, RtAbortReason};

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(i64)]
#[allow(unused)]
pub enum Tag {
    Invalid = 0,
    Needed = 1,
    PltRelSize = 2,
    Hash = 4,
    StrTab = 5,
    SymTab = 6,
    RelaOffset = 7,
    RelaSize = 8,
    RelaEntrySize = 9,
    SymEnt = 11,
    RelOffset = 17,
    RelSize = 18,
    RelEntrySize = 19,
    PltRel = 20,
    JmpRel = 23,
    InitArray = 25,
    FiniArray = 26,
    InitArraySize = 27,
    FiniArraySize = 28,
    RelaCount = 0x6FFFFFF9,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(u32)]
#[allow(unused)]
pub enum RelocationType {
    AArch64Abs64 = 257,
    AArch64GlobDat = 1025,
    AArch64JumpSlot = 1026,
    AArch64Relative = 1027,
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct Dyn {
    pub tag: Tag,
    pub val_ptr: u64,
}

impl Dyn {
    pub unsafe fn find_value(&self, tag: Tag) -> u64 {
        let mut found: *const u64 = core::ptr::null();
        let mut self_ptr = self as *const Self;

        while (*self_ptr).tag != Tag::Invalid {
            if (*self_ptr).tag == tag {
                if !found.is_null() {
                    rt_abort(RtAbortReason::DuplicatedDtEntry)
                }
                found = &(*self_ptr).val_ptr;
            }
            self_ptr = self_ptr.offset(1);
        }
        if found.is_null() {
            rt_abort(RtAbortReason::MissingDtEntry)
        }

        *found
    }
}

#[derive(Copy, Clone, PartialEq, Eq, Debug)]
#[repr(C)]
pub struct InfoSymbol {
    pub relocation_type: RelocationType,
    pub symbol: u32,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub union Info {
    pub value: u64,
    pub symbol: InfoSymbol,
}

#[derive(Copy, Clone)]
#[repr(C)]
pub struct Rela {
    pub offset: u64,
    pub info: Info,
    pub addend: i64,
}

pub unsafe fn relocate_with_dyn(base_address: *const u8, dynamic: *const Dyn) {
    let rela_offset = (*dynamic).find_value(Tag::RelaOffset);
    let rela_size = (*dynamic).find_value(Tag::RelaSize);
    let rela_entry_size = (*dynamic).find_value(Tag::RelaEntrySize);
    let rela_count = (*dynamic).find_value(Tag::RelaCount);
    if rela_size != rela_entry_size * rela_count {
        rt_abort(RtAbortReason::RelaSizeMismatch)
    }

    let rela_base = base_address.offset(rela_offset as isize) as *const Rela;
    for i in 0..rela_count {
        let rela = rela_base.offset(i as isize);
        match (*rela).info.symbol.relocation_type {
            RelocationType::AArch64Relative => {
                if (*rela).info.symbol.symbol == 0 {
                    let relocation_offset =
                        base_address.offset((*rela).offset as isize) as *mut *const u8;
                    *relocation_offset = base_address.offset((*rela).addend as isize);
                }
            }
            _ => rt_abort(RtAbortReason::UnsupportedRelocationType),
        }
    }
}
