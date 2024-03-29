#![allow(unused_qualifications)]
ij_core_workaround!();
pub type ProgramId = u64;
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum StorageId {
    #[default]
    None = 0,
    Host = 1,
    GameCard = 2,
    BuiltInSystem = 3,
    BuiltInUser = 4,
    SdCard = 5,
    Any = 6,
}
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct ProgramLocation {
    pub program_id: ProgramId,
    pub storage_id: StorageId,
    pub _padding_0: [u8; 7],
}
// Static size check for ProgramLocation (expect 16 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<ProgramLocation, [u8; 16]>;
};

