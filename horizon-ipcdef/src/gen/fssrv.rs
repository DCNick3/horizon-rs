use bitflags::bitflags;
use core::mem::MaybeUninit;
use horizon_error::Result;
use horizon_ipc::cmif::SessionHandle;
use super::account::Uid;
use super::ncm::ProgramId;
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
        let data_in = ();
        todo!("Command codegen")
    }
}
/// This struct is marked with sf::LargeData
#[repr(C)]
pub struct CodeVerificationData {
    pub signature: [u8; 256],
    pub target_hash: [u8; 32],
    pub has_data: bool,
    pub reserved: [u8; 3],
}
// Static size check for CodeVerificationData (expect 292 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<CodeVerificationData, [u8; 292]>;
};

pub struct IFileSystemProxyForLoader {
    handle: SessionHandle,
}
impl IFileSystemProxyForLoader {
    pub fn open_code_file_system(
        path: &Path,
        program_id: ProgramId,
    ) -> Result<(IFileSystem, CodeVerificationData)> {
        let data_in = program_id;
        let out_verif = MaybeUninit::<CodeVerificationData>::uninit();
        todo!("Command codegen")
    }
    pub fn is_archived_program(process_id: u64) -> Result<bool> {
        let data_in = process_id;
        #[repr(C)]
        struct Out {
            out: bool,
        }
        let _ = ::core::mem::transmute::<Out, [u8; 1]>;
        todo!("Command codegen")
    }
    pub fn set_current_process() -> Result<()> {
        let data_in = 0u64;
        todo!("Command codegen")
    }
}
/// This struct is marked with sf::LargeData
#[repr(C)]
pub struct Path {
    pub str: [u8; 769],
}
// Static size check for Path (expect 769 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<Path, [u8; 769]>;
};

#[repr(C)]
pub struct FileTimeStampRaw {
    pub create: i64,
    pub modify: i64,
    pub access: i64,
    pub is_local_time: bool,
    pub pad: [u8; 7],
}
// Static size check for FileTimeStampRaw (expect 32 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<FileTimeStampRaw, [u8; 32]>;
};

bitflags! {
    pub struct CreateOption : u32 { const BigFile = 0x1; }
}
#[repr(u32)]
pub enum QueryId {
    SetConcatenationFileAttribute = 0,
    UpdateMac = 1,
    IsSignedSystemPartitionOnSdCardValid = 2,
    QueryUnpreparedFileInformation = 3,
}
pub struct IFileSystem {
    handle: SessionHandle,
}
impl IFileSystem {
    pub fn create_file(path: &Path, size: i64, option: CreateOption) -> Result<()> {
        #[repr(C)]
        struct In {
            option: CreateOption,
            size: i64,
        }
        let _ = ::core::mem::transmute::<In, [u8; 16]>;
        let data_in: In = In { option, size };
        todo!("Command codegen")
    }
    pub fn delete_file(path: &Path) -> Result<()> {
        let data_in = ();
        todo!("Command codegen")
    }
    pub fn create_directory(path: &Path) -> Result<()> {
        let data_in = ();
        todo!("Command codegen")
    }
    pub fn delete_directory(path: &Path) -> Result<()> {
        let data_in = ();
        todo!("Command codegen")
    }
    pub fn delete_directory_recursively(path: &Path) -> Result<()> {
        let data_in = ();
        todo!("Command codegen")
    }
    pub fn rename_file(old_path: &Path, new_path: &Path) -> Result<()> {
        let data_in = ();
        todo!("Command codegen")
    }
    pub fn rename_directory(old_path: &Path, new_path: &Path) -> Result<()> {
        let data_in = ();
        todo!("Command codegen")
    }
    pub fn get_entry_type(path: &Path) -> Result<u32> {
        let data_in = ();
        #[repr(C)]
        struct Out {
            out: u32,
        }
        let _ = ::core::mem::transmute::<Out, [u8; 4]>;
        todo!("Command codegen")
    }
    pub fn open_file(path: &Path, mode: u32) -> Result<IFile> {
        let data_in = mode;
        todo!("Command codegen")
    }
    pub fn open_directory(path: &Path, mode: u32) -> Result<IDirectory> {
        let data_in = mode;
        todo!("Command codegen")
    }
    pub fn commit() -> Result<()> {
        let data_in = ();
        todo!("Command codegen")
    }
    pub fn get_free_space_size(path: &Path) -> Result<i64> {
        let data_in = ();
        #[repr(C)]
        struct Out {
            out: i64,
        }
        let _ = ::core::mem::transmute::<Out, [u8; 8]>;
        todo!("Command codegen")
    }
    pub fn get_total_space_size(path: &Path) -> Result<i64> {
        let data_in = ();
        #[repr(C)]
        struct Out {
            out: i64,
        }
        let _ = ::core::mem::transmute::<Out, [u8; 8]>;
        todo!("Command codegen")
    }
    pub fn clean_directory_recursively(path: &Path) -> Result<()> {
        let data_in = ();
        todo!("Command codegen")
    }
    pub fn get_file_time_stamp_raw(path: &Path) -> Result<FileTimeStampRaw> {
        let data_in = ();
        #[repr(C)]
        struct Out {
            out: FileTimeStampRaw,
        }
        let _ = ::core::mem::transmute::<Out, [u8; 32]>;
        todo!("Command codegen")
    }
    pub fn query_entry(
        out_buf: &mut [u8],
        in_buf: &[u8],
        query_id: QueryId,
        path: &Path,
    ) -> Result<()> {
        let data_in = query_id;
        todo!("Command codegen")
    }
}
pub struct IFile {
    handle: SessionHandle,
}
impl IFile {}
pub struct IDirectory {
    handle: SessionHandle,
}
impl IDirectory {}
