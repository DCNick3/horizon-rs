#[repr(C)]
pub struct Uid {
    pub uid_part_1: u64,
    pub uid_part_2: u64,
}
// Static size check for Uid (expect 16 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<Uid, [u8; 16]>;
};

