#![allow(unused_qualifications)]
use core::mem::MaybeUninit;
use horizon_error::Result;
use horizon_ipc::RawHandle;
use horizon_ipc::buffer::{IpcBufferRepr, get_ipc_buffer_for};
use horizon_ipc::cmif::SessionHandle;
use horizon_ipc::raw::cmif::{CmifInHeader, CmifOutHeader};
use horizon_ipc::raw::hipc::{
    HipcHeader, HipcInPointerBufferDescriptor, HipcOutPointerBufferDescriptor,
    HipcSpecialHeader,
};
use super::ncm::{ProgramId, ProgramLocation};
/// This struct is marked with sf::LargeData
#[repr(C, packed)]
pub struct ProgramInfo {
    pub main_thread_priority: u8,
    pub default_cpu_id: u8,
    pub flags: u16,
    pub main_thread_stack_size: u32,
    pub program_id: ProgramId,
    pub acid_sac_size: u32,
    pub aci_sac_size: u32,
    pub acid_fac_size: u32,
    pub aci_fah_size: u32,
    pub ac_buffer: [u8; 992],
}
// Static size check for ProgramInfo (expect 1024 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<ProgramInfo, [u8; 1024]>;
};

#[repr(C, packed)]
pub struct PinId {
    pub value: u64,
}
// Static size check for PinId (expect 8 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<PinId, [u8; 8]>;
};

pub struct IProcessManagerInterface {
    pub(crate) handle: SessionHandle,
}
impl IProcessManagerInterface {
    pub fn create_process(
        &self,
        id: PinId,
        flags: u32,
        reslimit_h: RawHandle,
    ) -> Result<RawHandle> {
        #[repr(C, packed)]
        struct In {
            pub flags: u32,
            pub _padding_0: [u8; 4],
            pub id: PinId,
        }
        let _ = ::core::mem::transmute::<In, [u8; 16]>;
        let data_in: In = In {
            flags,
            id,
            _padding_0: Default::default(),
        };
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            handle_reslimit_h: RawHandle,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: In,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 64]>;
        unsafe impl IpcBufferRepr for Request {}
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            handle_proc_h: RawHandle,
            pre_padding: [u8; 0],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 48]>;
        unsafe impl IpcBufferRepr for Response {}
        unsafe {
            ::core::ptr::write(
                get_ipc_buffer_for(),
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 12, 0, 0, true),
                    special_header: HipcSpecialHeader::new(false, 1, 0),
                    handle_reslimit_h: reslimit_h,
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
        crate::pre_ipc_hook();
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook();
        let Response {
            hipc,
            special_header,
            handle_proc_h: proc_h,
            cmif,
            raw_data: (),
            ..
        } = unsafe { ::core::ptr::read(get_ipc_buffer_for()) };
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
        cmif.result.into_result_with(|| proc_h)
    }
    pub fn get_program_info(&self, loc: ProgramLocation) -> Result<ProgramInfo> {
        let data_in = loc;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: ProgramLocation,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
            out_pointer_desc_0: HipcOutPointerBufferDescriptor,
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 64]>;
        unsafe impl IpcBufferRepr for Request {}
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            in_pointer_desc_0: HipcInPointerBufferDescriptor,
            pre_padding: [u8; 0],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 48]>;
        unsafe impl IpcBufferRepr for Response {}
        let out_program_info = MaybeUninit::<ProgramInfo>::uninit();
        unsafe {
            ::core::ptr::write(
                get_ipc_buffer_for(),
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 12, 3, 0, false),
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
                    out_pointer_desc_0: todo!(),
                },
            )
        };
        crate::pre_ipc_hook();
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook();
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(get_ipc_buffer_for())
        };
        debug_assert_eq!(hipc.num_in_pointers(), 1);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        let out_program_info = unsafe { out_program_info.assume_init() };
        cmif.result.into_result_with(|| out_program_info)
    }
    pub fn pin_program(&self, loc: ProgramLocation) -> Result<PinId> {
        let data_in = loc;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: ProgramLocation,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 56]>;
        unsafe impl IpcBufferRepr for Request {}
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: PinId,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 48]>;
        unsafe impl IpcBufferRepr for Response {}
        unsafe {
            ::core::ptr::write(
                get_ipc_buffer_for(),
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 12, 0, 0, false),
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
        crate::pre_ipc_hook();
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook();
        let Response { hipc, cmif, raw_data: out_id, .. } = unsafe {
            ::core::ptr::read(get_ipc_buffer_for())
        };
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        cmif.result.into_result_with(|| out_id)
    }
    pub fn unpin_program(&self, id: PinId) -> Result<()> {
        let data_in = id;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: PinId,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 48]>;
        unsafe impl IpcBufferRepr for Request {}
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
        unsafe impl IpcBufferRepr for Response {}
        unsafe {
            ::core::ptr::write(
                get_ipc_buffer_for(),
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 10, 0, 0, false),
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
        crate::pre_ipc_hook();
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook();
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(get_ipc_buffer_for())
        };
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        cmif.result.into_result_with(|| ())
    }
    pub fn set_enabled_program_verification(&self, enabled: bool) -> Result<()> {
        let data_in = enabled;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 0],
            cmif: CmifInHeader,
            raw_data: bool,
            raw_data_word_padding: [u8; 3],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 44]>;
        unsafe impl IpcBufferRepr for Request {}
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
        unsafe impl IpcBufferRepr for Response {}
        unsafe {
            ::core::ptr::write(
                get_ipc_buffer_for(),
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 9, 0, 0, false),
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
        crate::pre_ipc_hook();
        horizon_svc::send_sync_request(self.handle.0)?;
        crate::post_ipc_hook();
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(get_ipc_buffer_for())
        };
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        cmif.result.into_result_with(|| ())
    }
}
