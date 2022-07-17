#![allow(unused_qualifications)]
use bitflags::bitflags;
use core::mem::MaybeUninit;
use horizon_error::{ErrorCode, Result};
use horizon_ipc::RawHandle;
use horizon_ipc::buffer::get_ipc_buffer_ptr;
use horizon_ipc::cmif::SessionHandle;
use horizon_ipc::hipc::MapAliasBufferMode;
use horizon_ipc::raw::cmif::{CmifInHeader, CmifOutHeader};
use horizon_ipc::raw::hipc::{
    HipcHeader, HipcInPointerBufferDescriptor, HipcMapAliasBufferDescriptor,
    HipcOutPointerBufferDescriptor, HipcSpecialHeader,
};
use super::account::Uid;
use super::ncm::ProgramId;
#[derive(Debug, Clone, Copy, Default)]
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

#[derive(Debug, Clone, Copy, Default)]
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

#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct DirectoryEntry {
    pub path: Path,
    pub padding: [u8; 3],
    pub typ: DirectoryEntryType,
    pub _padding_0: [u8; 3],
    pub filesize: u64,
}
// Static size check for DirectoryEntry (expect 784 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<DirectoryEntry, [u8; 784]>;
};

#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[repr(u8)]
pub enum DirectoryEntryType {
    #[default]
    Directory = 0,
    File = 1,
}
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[repr(u32)]
pub enum Partition {
    #[default]
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
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[repr(u32)]
pub enum FileSystemType {
    #[default]
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
    pub(crate) handle: SessionHandle,
}
impl IFileSystemProxy {
    pub fn open_sd_card_file_system(&self) -> Result<IFileSystem> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 40]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            handle_out: RawHandle,
            pre_padding: [u8; 0],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 48]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 8, 0, 0, false),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 18,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook(
            "fssrv::IFileSystemProxy::OpenSdCardFileSystem",
            self.handle.0,
        );
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook(
            "fssrv::IFileSystemProxy::OpenSdCardFileSystem",
            self.handle.0,
        );
        let Response { hipc, special_header, handle_out: out, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if hipc.has_special_header() != 0 {
            if cmif.result.is_failure() {
                return Err(cmif.result);
            }
        } else {
            return Err(unsafe {
                ::core::ptr::read(ipc_buffer_ptr.offset(24) as *const ErrorCode)
            })
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 1);
        debug_assert_eq!(special_header.send_pid(), 0);
        debug_assert_eq!(special_header.num_copy_handles(), 0);
        debug_assert_eq!(special_header.num_move_handles(), 1);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        let out = IFileSystem {
            handle: SessionHandle(out),
        };
        Ok(out)
    }
}
impl From<RawHandle> for IFileSystemProxy {
    fn from(h: RawHandle) -> Self {
        Self { handle: SessionHandle(h) }
    }
}

/// This struct is marked with sf::LargeData
#[derive(Debug, Clone, Copy)]
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
impl Default for CodeVerificationData {
    fn default() -> Self {
        Self {
            signature: [0; 256],
            target_hash: [0; 32],
            has_data: false,
            reserved: [0; 3],
        }
    }
}

pub struct IFileSystemProxyForLoader {
    pub(crate) handle: SessionHandle,
}
impl IFileSystemProxyForLoader {
    pub fn open_code_file_system(
        &self,
        path: &Path,
        program_id: ProgramId,
    ) -> Result<(IFileSystem, CodeVerificationData)> {
        let data_in = program_id;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: ProgramId,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
            out_pointer_desc_0: HipcOutPointerBufferDescriptor,
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 64]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            handle_out_fs: RawHandle,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 56]>;
        let out_verif = MaybeUninit::<CodeVerificationData>::uninit();
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 0, 0, 0, 10, 3, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 0,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                    out_pointer_desc_0: HipcOutPointerBufferDescriptor::new(
                        out_verif.as_ptr() as usize,
                        ::core::mem::size_of_val(&out_verif),
                    ),
                },
            )
        };
        crate::pre_ipc_hook(
            "fssrv::IFileSystemProxyForLoader::OpenCodeFileSystem",
            self.handle.0,
        );
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook(
            "fssrv::IFileSystemProxyForLoader::OpenCodeFileSystem",
            self.handle.0,
        );
        let Response {
            hipc,
            special_header,
            handle_out_fs: out_fs,
            cmif,
            raw_data: (),
            ..
        } = unsafe { ::core::ptr::read(ipc_buffer_ptr as *const _) };
        if hipc.has_special_header() != 0 {
            if cmif.result.is_failure() {
                return Err(cmif.result);
            }
        } else {
            return Err(unsafe {
                ::core::ptr::read(ipc_buffer_ptr.offset(24) as *const ErrorCode)
            })
        }
        debug_assert_eq!(hipc.num_in_pointers(), 1);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 1);
        debug_assert_eq!(special_header.send_pid(), 0);
        debug_assert_eq!(special_header.num_copy_handles(), 0);
        debug_assert_eq!(special_header.num_move_handles(), 1);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        let out_verif = unsafe { out_verif.assume_init() };
        let out_fs = IFileSystem {
            handle: SessionHandle(out_fs),
        };
        Ok((out_fs, out_verif))
    }

    pub fn is_archived_program(&self, process_id: u64) -> Result<bool> {
        let data_in = process_id;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: u64,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 48]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: bool,
            raw_data_word_padding: [u8; 3],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 44]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 10, 0, 0, false),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 1,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook(
            "fssrv::IFileSystemProxyForLoader::IsArchivedProgram",
            self.handle.0,
        );
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook(
            "fssrv::IFileSystemProxyForLoader::IsArchivedProgram",
            self.handle.0,
        );
        let Response { hipc, cmif, raw_data: out, .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(out)
    }

    pub fn set_current_process(&self) -> Result<()> {
        let data_in = 0u64;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            pid_placeholder: u64,
            pre_padding: [u8; 12],
            cmif: CmifInHeader,
            raw_data: u64,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 4],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 60]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 40]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 10, 0, 0, true),
                    special_header: HipcSpecialHeader::new(true, 0, 0),
                    pid_placeholder: 0,
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 2,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook(
            "fssrv::IFileSystemProxyForLoader::SetCurrentProcess",
            self.handle.0,
        );
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook(
            "fssrv::IFileSystemProxyForLoader::SetCurrentProcess",
            self.handle.0,
        );
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(())
    }
}
impl From<RawHandle> for IFileSystemProxyForLoader {
    fn from(h: RawHandle) -> Self {
        Self { handle: SessionHandle(h) }
    }
}

/// This struct is marked with sf::LargeData
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct Path {
    pub str: [u8; 769],
}
// Static size check for Path (expect 769 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<Path, [u8; 769]>;
};
impl Default for Path {
    fn default() -> Self {
        Self { str: [0; 769] }
    }
}

#[derive(Debug, Clone, Copy, Default)]
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
    #[derive(Default)] pub struct CreateOption : u32 { const BigFile = 0x1; }
}
#[derive(Debug, Clone, Copy, Default, PartialEq)]
#[repr(u32)]
pub enum QueryId {
    #[default]
    SetConcatenationFileAttribute = 0,
    UpdateMac = 1,
    IsSignedSystemPartitionOnSdCardValid = 2,
    QueryUnpreparedFileInformation = 3,
}
bitflags! {
    #[derive(Default)] pub struct OpenDirectoryMode : u32 { const ReadDirs = 0x1; const
    ReadFiles = 0x2; const NoFileSize = 0x8000000; }
}
pub struct IFileSystem {
    pub(crate) handle: SessionHandle,
}
impl IFileSystem {
    pub fn create_file(
        &self,
        path: &Path,
        size: i64,
        option: CreateOption,
    ) -> Result<()> {
        #[repr(C, packed)]
        struct In {
            pub option: CreateOption,
            pub _padding_0: [u8; 4],
            pub size: i64,
        }
        let _ = ::core::mem::transmute::<In, [u8; 16]>;
        let data_in: In = In {
            option,
            size,
            _padding_0: Default::default(),
        };
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: In,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 64]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 40]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 0, 0, 0, 12, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 0,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::CreateFile", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::CreateFile", self.handle.0);
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(())
    }

    pub fn delete_file(&self, path: &Path) -> Result<()> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 48]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 40]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 0, 0, 0, 8, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 1,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::DeleteFile", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::DeleteFile", self.handle.0);
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(())
    }

    pub fn create_directory(&self, path: &Path) -> Result<()> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 48]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 40]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 0, 0, 0, 8, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 2,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::CreateDirectory", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::CreateDirectory", self.handle.0);
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(())
    }

    pub fn delete_directory(&self, path: &Path) -> Result<()> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 48]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 40]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 0, 0, 0, 8, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 3,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::DeleteDirectory", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::DeleteDirectory", self.handle.0);
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(())
    }

    pub fn delete_directory_recursively(&self, path: &Path) -> Result<()> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 48]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 40]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 0, 0, 0, 8, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 4,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook(
            "fssrv::IFileSystem::DeleteDirectoryRecursively",
            self.handle.0,
        );
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook(
            "fssrv::IFileSystem::DeleteDirectoryRecursively",
            self.handle.0,
        );
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(())
    }

    pub fn rename_file(&self, old_path: &Path, new_path: &Path) -> Result<()> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            in_pointer_desc_1: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 56]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 40]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 2, 0, 0, 0, 8, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        old_path as *const _ as usize,
                        ::core::mem::size_of_val(old_path),
                    ),
                    in_pointer_desc_1: HipcInPointerBufferDescriptor::new(
                        1,
                        new_path as *const _ as usize,
                        ::core::mem::size_of_val(new_path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 5,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::RenameFile", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::RenameFile", self.handle.0);
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(())
    }

    pub fn rename_directory(&self, old_path: &Path, new_path: &Path) -> Result<()> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            in_pointer_desc_1: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 56]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 40]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 2, 0, 0, 0, 8, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        old_path as *const _ as usize,
                        ::core::mem::size_of_val(old_path),
                    ),
                    in_pointer_desc_1: HipcInPointerBufferDescriptor::new(
                        1,
                        new_path as *const _ as usize,
                        ::core::mem::size_of_val(new_path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 6,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::RenameDirectory", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::RenameDirectory", self.handle.0);
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(())
    }

    pub fn get_entry_type(&self, path: &Path) -> Result<u32> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 48]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: u32,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 44]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 0, 0, 0, 8, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 7,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::GetEntryType", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::GetEntryType", self.handle.0);
        let Response { hipc, cmif, raw_data: out, .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(out)
    }

    pub fn open_file(&self, path: &Path, mode: u32) -> Result<IFile> {
        let data_in = mode;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: u32,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 52]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            handle_out: RawHandle,
            pre_padding: [u8; 0],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 48]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 0, 0, 0, 9, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 8,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::OpenFile", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::OpenFile", self.handle.0);
        let Response { hipc, special_header, handle_out: out, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if hipc.has_special_header() != 0 {
            if cmif.result.is_failure() {
                return Err(cmif.result);
            }
        } else {
            return Err(unsafe {
                ::core::ptr::read(ipc_buffer_ptr.offset(24) as *const ErrorCode)
            })
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 1);
        debug_assert_eq!(special_header.send_pid(), 0);
        debug_assert_eq!(special_header.num_copy_handles(), 0);
        debug_assert_eq!(special_header.num_move_handles(), 1);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        let out = IFile {
            handle: SessionHandle(out),
        };
        Ok(out)
    }

    pub fn open_directory(
        &self,
        path: &Path,
        mode: OpenDirectoryMode,
    ) -> Result<IDirectory> {
        let data_in = mode;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: OpenDirectoryMode,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 52]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            handle_out: RawHandle,
            pre_padding: [u8; 0],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 48]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 0, 0, 0, 9, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 9,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::OpenDirectory", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::OpenDirectory", self.handle.0);
        let Response { hipc, special_header, handle_out: out, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if hipc.has_special_header() != 0 {
            if cmif.result.is_failure() {
                return Err(cmif.result);
            }
        } else {
            return Err(unsafe {
                ::core::ptr::read(ipc_buffer_ptr.offset(24) as *const ErrorCode)
            })
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 1);
        debug_assert_eq!(special_header.send_pid(), 0);
        debug_assert_eq!(special_header.num_copy_handles(), 0);
        debug_assert_eq!(special_header.num_move_handles(), 1);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        let out = IDirectory {
            handle: SessionHandle(out),
        };
        Ok(out)
    }

    pub fn commit(&self) -> Result<()> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 40]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 40]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 8, 0, 0, false),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 10,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::Commit", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::Commit", self.handle.0);
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(())
    }

    pub fn get_free_space_size(&self, path: &Path) -> Result<i64> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 48]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: i64,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 48]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 0, 0, 0, 8, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 11,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::GetFreeSpaceSize", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::GetFreeSpaceSize", self.handle.0);
        let Response { hipc, cmif, raw_data: out, .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(out)
    }

    pub fn get_total_space_size(&self, path: &Path) -> Result<i64> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 48]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: i64,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 48]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 0, 0, 0, 8, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 12,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::GetTotalSpaceSize", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::GetTotalSpaceSize", self.handle.0);
        let Response { hipc, cmif, raw_data: out, .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(out)
    }

    pub fn clean_directory_recursively(&self, path: &Path) -> Result<()> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 48]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 40]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 0, 0, 0, 8, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 13,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook(
            "fssrv::IFileSystem::CleanDirectoryRecursively",
            self.handle.0,
        );
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook(
            "fssrv::IFileSystem::CleanDirectoryRecursively",
            self.handle.0,
        );
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(())
    }

    pub fn get_file_time_stamp_raw(&self, path: &Path) -> Result<FileTimeStampRaw> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 48]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: FileTimeStampRaw,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 72]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 0, 0, 0, 8, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 14,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::GetFileTimeStampRaw", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::GetFileTimeStampRaw", self.handle.0);
        let Response { hipc, cmif, raw_data: out, .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(out)
    }

    pub fn query_entry(
        &self,
        out_buf: &mut [u8],
        in_buf: &[u8],
        query_id: QueryId,
        path: &Path,
    ) -> Result<()> {
        let data_in = query_id;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            in_map_alias_desc_0: HipcMapAliasBufferDescriptor,
            out_map_alias_desc_0: HipcMapAliasBufferDescriptor,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: QueryId,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 76]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 40]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 1, 1, 1, 0, 9, 0, 0, false),
                    in_pointer_desc_0: HipcInPointerBufferDescriptor::new(
                        0,
                        path as *const _ as usize,
                        ::core::mem::size_of_val(path),
                    ),
                    in_map_alias_desc_0: HipcMapAliasBufferDescriptor::new(
                        MapAliasBufferMode::NonSecure,
                        in_buf.as_ptr() as usize,
                        ::core::mem::size_of_val(in_buf),
                    ),
                    out_map_alias_desc_0: HipcMapAliasBufferDescriptor::new(
                        MapAliasBufferMode::NonSecure,
                        out_buf.as_ptr() as usize,
                        ::core::mem::size_of_val(out_buf),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 15,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IFileSystem::QueryEntry", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IFileSystem::QueryEntry", self.handle.0);
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(())
    }
}
impl From<RawHandle> for IFileSystem {
    fn from(h: RawHandle) -> Self {
        Self { handle: SessionHandle(h) }
    }
}

pub struct IFile {
    pub(crate) handle: SessionHandle,
}
impl IFile {}
impl From<RawHandle> for IFile {
    fn from(h: RawHandle) -> Self {
        Self { handle: SessionHandle(h) }
    }
}

pub struct IDirectory {
    pub(crate) handle: SessionHandle,
}
impl IDirectory {
    pub fn read(&self, out_entries: &mut [DirectoryEntry]) -> Result<i64> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            out_map_alias_desc_0: HipcMapAliasBufferDescriptor,
            pre_padding: [u8; 12],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 4],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 52]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: i64,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 48]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 1, 0, 8, 0, 0, false),
                    out_map_alias_desc_0: HipcMapAliasBufferDescriptor::new(
                        MapAliasBufferMode::Normal,
                        out_entries.as_ptr() as usize,
                        ::core::mem::size_of_val(out_entries),
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 0,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IDirectory::Read", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IDirectory::Read", self.handle.0);
        let Response { hipc, cmif, raw_data: out, .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(out)
    }

    pub fn get_entry_count(&self) -> Result<i64> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 40]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: i64,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 48]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 8, 0, 0, false),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 1,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        crate::pre_ipc_hook("fssrv::IDirectory::GetEntryCount", self.handle.0);
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook("fssrv::IDirectory::GetEntryCount", self.handle.0);
        let Response { hipc, cmif, raw_data: out, .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(out)
    }
}
impl From<RawHandle> for IDirectory {
    fn from(h: RawHandle) -> Self {
        Self { handle: SessionHandle(h) }
    }
}

