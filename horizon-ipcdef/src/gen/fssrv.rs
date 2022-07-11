use horizon_error::Result;
use horizon_ipc::cmif::SessionHandle;
use super::account::Uid;
#[repr(C)]
pub struct FsSaveDataCreationInfo {
    pub save_data_size: i64,
    pub journal_size: i64,
    pub available_size: u64,
    pub owner_id: u64,
    pub flags: u32,
    pub save_data_space_id: u8,
    pub unk: u8,
    pub padding: [u8; 26],
}
// Static size check for FsSaveDataCreationInfo (expect 64 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<FsSaveDataCreationInfo, [u8; 64]>;
};

#[repr(C)]
pub struct FsSaveDataAttribute {
    pub application_id: u64,
    pub uid: Uid,
    pub system_save_data_id: u64,
    pub save_data_type: u8,
    pub save_data_rank: u8,
    pub save_data_index: u16,
    pub pad_x_24: u32,
    pub unk_x_28: u64,
    pub unk_x_30: u64,
    pub unk_x_38: u64,
}
// Static size check for FsSaveDataAttribute (expect 64 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<FsSaveDataAttribute, [u8; 64]>;
};

#[repr(C)]
pub struct DirectoryEntry {
    pub path: [u8; 769],
    pub pad: [u8; 3],
    pub typ: i8,
    pub pad_2: [u8; 3],
    pub filesize: i64,
}
// Static size check for DirectoryEntry (expect 784 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<DirectoryEntry, [u8; 784]>;
};

#[repr(u32)]
pub enum Partition {
    BootPartition1Root = 0,
    BootPartition2Root = 10,
    UserDataRoot = 20,
    BootConfigAndPackage2Part1 = 21,
    BootConfigAndPackage2Part2 = 22,
    BootConfigAndPackage2Part3 = 23,
    BootConfigAndPackage2Part4 = 24,
    BootConfigAndPackage2Part5 = 25,
    BootConfigAndPackage2Part6 = 26,
    CalibrationBinary = 27,
    CalibrationFile = 28,
    SafeMode = 29,
    SystemProperEncryption = 30,
    User = 31,
}
#[repr(u8)]
pub enum DirectoryEntryType {
    Directory = 0,
    File = 1,
}
#[repr(u32)]
pub enum FileSystemType {
    Invalid = 0,
    Invalid2 = 1,
    Logo = 2,
    ContentControl = 3,
    ContentManual = 4,
    ContentMeta = 5,
    ContentData = 6,
    ApplicationPackage = 7,
}
pub struct IFileSystemProxy {
    handle: SessionHandle,
}
impl IFileSystemProxy {
    pub fn open_sd_card_file_system() -> Result<IFileSystem> {
        todo!("Command codegen")
    }
}
pub struct IFileSystem {
    handle: SessionHandle,
}
impl IFileSystem {}
