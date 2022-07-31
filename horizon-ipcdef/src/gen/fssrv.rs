#![allow(unused_qualifications)]
ij_core_workaround!();
use bitflags::bitflags;
use core::mem::MaybeUninit;
use horizon_error::{ErrorCode, Result};
use horizon_ipc::RawHandle;
use horizon_ipc::buffer::get_ipc_buffer_ptr;
use horizon_ipc::cmif::CommandType;
use horizon_ipc::handle_storage::{HandleStorage, OwnedHandle, RefHandle, SharedHandle};
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

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u8)]
pub enum DirectoryEntryType {
    #[default]
    Directory = 0,
    File = 1,
}
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
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
pub struct IFileSystemProxy<S: HandleStorage = OwnedHandle> {
    pub(crate) handle: S,
}
impl<S: HandleStorage> IFileSystemProxy<S> {
    pub fn new(handle: S) -> Self {
        Self { handle }
    }
    pub fn into_inner(self) -> S {
        self.handle
    }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook(
                "fssrv::IFileSystemProxy::OpenSdCardFileSystem",
                *handle,
            );
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook(
                "fssrv::IFileSystemProxy::OpenSdCardFileSystem",
                *handle,
            );
        }
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
            handle: OwnedHandle::new(out),
        };
        Ok(out)
    }
}
impl IFileSystemProxy<OwnedHandle> {
    pub fn as_ref(&self) -> IFileSystemProxy<RefHandle<'_>> {
        IFileSystemProxy {
            handle: self.handle.as_ref(),
        }
    }
    pub fn into_shared(self) -> IFileSystemProxy<SharedHandle> {
        IFileSystemProxy {
            handle: SharedHandle::new(self.handle.leak()),
        }
    }
}
impl ::core::fmt::Debug for IFileSystemProxy {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "IFileSystemProxy({})", self.handle)
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

pub struct IFileSystemProxyForLoader<S: HandleStorage = OwnedHandle> {
    pub(crate) handle: S,
}
impl<S: HandleStorage> IFileSystemProxyForLoader<S> {
    pub fn new(handle: S) -> Self {
        Self { handle }
    }
    pub fn into_inner(self) -> S {
        self.handle
    }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        0,
                        0,
                        0,
                        10,
                        3,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook(
                "fssrv::IFileSystemProxyForLoader::OpenCodeFileSystem",
                *handle,
            );
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook(
                "fssrv::IFileSystemProxyForLoader::OpenCodeFileSystem",
                *handle,
            );
        }
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
            handle: OwnedHandle::new(out_fs),
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        0,
                        0,
                        10,
                        0,
                        0,
                        false,
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook(
                "fssrv::IFileSystemProxyForLoader::IsArchivedProgram",
                *handle,
            );
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook(
                "fssrv::IFileSystemProxyForLoader::IsArchivedProgram",
                *handle,
            );
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        0,
                        0,
                        10,
                        0,
                        0,
                        true,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook(
                "fssrv::IFileSystemProxyForLoader::SetCurrentProcess",
                *handle,
            );
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook(
                "fssrv::IFileSystemProxyForLoader::SetCurrentProcess",
                *handle,
            );
        }
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
impl IFileSystemProxyForLoader<OwnedHandle> {
    pub fn as_ref(&self) -> IFileSystemProxyForLoader<RefHandle<'_>> {
        IFileSystemProxyForLoader {
            handle: self.handle.as_ref(),
        }
    }
    pub fn into_shared(self) -> IFileSystemProxyForLoader<SharedHandle> {
        IFileSystemProxyForLoader {
            handle: SharedHandle::new(self.handle.leak()),
        }
    }
}
impl ::core::fmt::Debug for IFileSystemProxyForLoader {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "IFileSystemProxyForLoader({})", self.handle)
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
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
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
bitflags! {
    #[derive(Default)] pub struct OpenFileMode : u32 { const Read = 0x1; const Write =
    0x2; const Append = 0x4; }
}
pub struct IFileSystem<S: HandleStorage = OwnedHandle> {
    pub(crate) handle: S,
}
impl<S: HandleStorage> IFileSystem<S> {
    pub fn new(handle: S) -> Self {
        Self { handle }
    }
    pub fn into_inner(self) -> S {
        self.handle
    }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        0,
                        0,
                        0,
                        12,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::CreateFile", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::CreateFile", *handle);
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::DeleteFile", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::DeleteFile", *handle);
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::CreateDirectory", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::CreateDirectory", *handle);
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::DeleteDirectory", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::DeleteDirectory", *handle);
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook(
                "fssrv::IFileSystem::DeleteDirectoryRecursively",
                *handle,
            );
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook(
                "fssrv::IFileSystem::DeleteDirectoryRecursively",
                *handle,
            );
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        2,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::RenameFile", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::RenameFile", *handle);
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        2,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::RenameDirectory", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::RenameDirectory", *handle);
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::GetEntryType", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::GetEntryType", *handle);
        }
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

    pub fn open_file(&self, path: &Path, mode: OpenFileMode) -> Result<IFile> {
        let data_in = mode;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: OpenFileMode,
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        0,
                        0,
                        0,
                        9,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::OpenFile", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::OpenFile", *handle);
        }
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
            handle: OwnedHandle::new(out),
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        0,
                        0,
                        0,
                        9,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::OpenDirectory", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::OpenDirectory", *handle);
        }
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
            handle: OwnedHandle::new(out),
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::Commit", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::Commit", *handle);
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::GetFreeSpaceSize", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::GetFreeSpaceSize", *handle);
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::GetTotalSpaceSize", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::GetTotalSpaceSize", *handle);
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook(
                "fssrv::IFileSystem::CleanDirectoryRecursively",
                *handle,
            );
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook(
                "fssrv::IFileSystem::CleanDirectoryRecursively",
                *handle,
            );
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::GetFileTimeStampRaw", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::GetFileTimeStampRaw", *handle);
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        1,
                        1,
                        1,
                        0,
                        9,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFileSystem::QueryEntry", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFileSystem::QueryEntry", *handle);
        }
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
impl IFileSystem<OwnedHandle> {
    pub fn as_ref(&self) -> IFileSystem<RefHandle<'_>> {
        IFileSystem {
            handle: self.handle.as_ref(),
        }
    }
    pub fn into_shared(self) -> IFileSystem<SharedHandle> {
        IFileSystem {
            handle: SharedHandle::new(self.handle.leak()),
        }
    }
}
impl ::core::fmt::Debug for IFileSystem {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "IFileSystem({})", self.handle)
    }
}

bitflags! {
    #[derive(Default)] pub struct ReadOption : u32 {}
}
bitflags! {
    #[derive(Default)] pub struct WriteOption : u32 { const Flush = 0x1; }
}
#[derive(Debug, Clone, Copy)]
#[repr(C)]
pub struct FileQueryRangeInfo {
    pub aes_ctr_key_type: u32,
    pub speed_emulation_type: u32,
    pub reserved: [u8; 56],
}
// Static size check for FileQueryRangeInfo (expect 64 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<FileQueryRangeInfo, [u8; 64]>;
};
impl Default for FileQueryRangeInfo {
    fn default() -> Self {
        Self {
            aes_ctr_key_type: 0,
            speed_emulation_type: 0,
            reserved: [0; 56],
        }
    }
}

#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Hash)]
#[repr(u32)]
pub enum OperationId {
    #[default]
    Clear = 0,
    ClearSignature = 1,
    InvalidateCache = 2,
    QueryRange = 3,
}
pub struct IFile<S: HandleStorage = OwnedHandle> {
    pub(crate) handle: S,
}
impl<S: HandleStorage> IFile<S> {
    pub fn new(handle: S) -> Self {
        Self { handle }
    }
    pub fn into_inner(self) -> S {
        self.handle
    }
    pub fn read(
        &self,
        offset: i64,
        buffer: &mut [u8],
        size: i64,
        option: ReadOption,
    ) -> Result<i64> {
        #[repr(C, packed)]
        struct In {
            pub option: ReadOption,
            pub _padding_0: [u8; 4],
            pub offset: i64,
            pub size: i64,
        }
        let _ = ::core::mem::transmute::<In, [u8; 24]>;
        let data_in: In = In {
            option,
            offset,
            size,
            _padding_0: Default::default(),
        };
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            out_map_alias_desc_0: HipcMapAliasBufferDescriptor,
            pre_padding: [u8; 12],
            cmif: CmifInHeader,
            raw_data: In,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 4],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 76]>;
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        1,
                        0,
                        14,
                        0,
                        0,
                        false,
                    ),
                    out_map_alias_desc_0: HipcMapAliasBufferDescriptor::new(
                        MapAliasBufferMode::NonSecure,
                        buffer.as_ptr() as usize,
                        ::core::mem::size_of_val(buffer),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFile::Read", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFile::Read", *handle);
        }
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

    pub fn write(
        &self,
        offset: i64,
        buffer: &[u8],
        size: i64,
        option: WriteOption,
    ) -> Result<()> {
        #[repr(C, packed)]
        struct In {
            pub option: WriteOption,
            pub _padding_0: [u8; 4],
            pub offset: i64,
            pub size: i64,
        }
        let _ = ::core::mem::transmute::<In, [u8; 24]>;
        let data_in: In = In {
            option,
            offset,
            size,
            _padding_0: Default::default(),
        };
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_map_alias_desc_0: HipcMapAliasBufferDescriptor,
            pre_padding: [u8; 12],
            cmif: CmifInHeader,
            raw_data: In,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 4],
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        1,
                        0,
                        0,
                        14,
                        0,
                        0,
                        false,
                    ),
                    in_map_alias_desc_0: HipcMapAliasBufferDescriptor::new(
                        MapAliasBufferMode::NonSecure,
                        buffer.as_ptr() as usize,
                        ::core::mem::size_of_val(buffer),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFile::Write", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFile::Write", *handle);
        }
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

    pub fn flush(&self) -> Result<()> {
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFile::Flush", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFile::Flush", *handle);
        }
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

    pub fn set_size(&self, size: i64) -> Result<()> {
        let data_in = size;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: i64,
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        0,
                        0,
                        10,
                        0,
                        0,
                        false,
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFile::SetSize", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFile::SetSize", *handle);
        }
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

    pub fn get_size(&self) -> Result<i64> {
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFile::GetSize", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFile::GetSize", *handle);
        }
        let Response { hipc, cmif, raw_data: size, .. } = unsafe {
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
        Ok(size)
    }

    pub fn operate_range(
        &self,
        op_id: OperationId,
        offset: i64,
        size: i64,
    ) -> Result<FileQueryRangeInfo> {
        #[repr(C, packed)]
        struct In {
            pub op_id: OperationId,
            pub _padding_0: [u8; 4],
            pub offset: i64,
            pub size: i64,
        }
        let _ = ::core::mem::transmute::<In, [u8; 24]>;
        let data_in: In = In {
            op_id,
            offset,
            size,
            _padding_0: Default::default(),
        };
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: In,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 64]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: FileQueryRangeInfo,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 104]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        0,
                        0,
                        14,
                        0,
                        0,
                        false,
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFile::OperateRange", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFile::OperateRange", *handle);
        }
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

    pub fn operate_range_with_buffer(
        &self,
        out_buf: &mut [u8],
        in_buf: &[u8],
        op_id: OperationId,
        offset: i64,
        size: i64,
    ) -> Result<()> {
        #[repr(C, packed)]
        struct In {
            pub op_id: OperationId,
            pub _padding_0: [u8; 4],
            pub offset: i64,
            pub size: i64,
        }
        let _ = ::core::mem::transmute::<In, [u8; 24]>;
        let data_in: In = In {
            op_id,
            offset,
            size,
            _padding_0: Default::default(),
        };
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            in_map_alias_desc_0: HipcMapAliasBufferDescriptor,
            out_map_alias_desc_0: HipcMapAliasBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: In,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 88]>;
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        1,
                        1,
                        0,
                        14,
                        0,
                        0,
                        false,
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
                        command_id: 6,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IFile::OperateRangeWithBuffer", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IFile::OperateRangeWithBuffer", *handle);
        }
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
impl IFile<OwnedHandle> {
    pub fn as_ref(&self) -> IFile<RefHandle<'_>> {
        IFile {
            handle: self.handle.as_ref(),
        }
    }
    pub fn into_shared(self) -> IFile<SharedHandle> {
        IFile {
            handle: SharedHandle::new(self.handle.leak()),
        }
    }
}
impl ::core::fmt::Debug for IFile {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "IFile({})", self.handle)
    }
}

pub struct IDirectory<S: HandleStorage = OwnedHandle> {
    pub(crate) handle: S,
}
impl<S: HandleStorage> IDirectory<S> {
    pub fn new(handle: S) -> Self {
        Self { handle }
    }
    pub fn into_inner(self) -> S {
        self.handle
    }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        1,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IDirectory::Read", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IDirectory::Read", *handle);
        }
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
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        0,
                        0,
                        8,
                        0,
                        0,
                        false,
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
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("fssrv::IDirectory::GetEntryCount", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("fssrv::IDirectory::GetEntryCount", *handle);
        }
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
impl IDirectory<OwnedHandle> {
    pub fn as_ref(&self) -> IDirectory<RefHandle<'_>> {
        IDirectory {
            handle: self.handle.as_ref(),
        }
    }
    pub fn into_shared(self) -> IDirectory<SharedHandle> {
        IDirectory {
            handle: SharedHandle::new(self.handle.leak()),
        }
    }
}
impl ::core::fmt::Debug for IDirectory {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "IDirectory({})", self.handle)
    }
}

