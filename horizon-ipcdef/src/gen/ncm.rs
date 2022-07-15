pub type ProgramId = u64;
#[repr(u8)]
pub enum StorageId {
    None = 0,
    Host = 1,
    GameCard = 2,
    BuiltInSystem = 3,
    BuiltInUser = 4,
    SdCard = 5,
    Any = 6,
}
#[repr(C)]
pub struct ProgramLocation {
    pub program_id: ProgramId,
    pub storage_id: StorageId,
}
// Static size check for ProgramLocation (expect 16 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<ProgramLocation, [u8; 16]>;
};

