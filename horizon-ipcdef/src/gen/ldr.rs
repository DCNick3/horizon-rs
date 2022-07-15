use core::mem::MaybeUninit;
use horizon_error::Result;
use horizon_ipc::RawHandle;
use horizon_ipc::buffer::{IpcBufferRepr, get_ipc_buffer_for};
use horizon_ipc::cmif::SessionHandle;
use horizon_ipc::raw::cmif::CmifInHeader;
use horizon_ipc::raw::hipc::{
    HipcHeader, HipcOutPointerBufferDescriptor, HipcSpecialHeader,
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
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 64]>;
        unsafe impl IpcBufferRepr for Request {}
        unsafe {
            ::core::ptr::write(
                get_ipc_buffer_for(),
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 0, 0, 0, true),
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
                    post_padding: Default::default(),
                },
            )
        };
        horizon_svc::send_sync_request(self.handle.0)?;
        todo!("Command codegen")
    }
    pub fn get_program_info(&self, loc: ProgramLocation) -> Result<ProgramInfo> {
        let data_in = loc;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            pre_padding: [u8; 4],
            cmif: CmifInHeader,
            raw_data: ProgramLocation,
            post_padding: [u8; 12],
            out_pointer_desc_0: HipcOutPointerBufferDescriptor,
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 68]>;
        unsafe impl IpcBufferRepr for Request {}
        let out_program_info = MaybeUninit::<ProgramInfo>::uninit();
        unsafe {
            ::core::ptr::write(
                get_ipc_buffer_for(),
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 0, 3, 0, true),
                    special_header: HipcSpecialHeader::new(false, 0, 0),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 1,
                        token: 0,
                    },
                    raw_data: data_in,
                    post_padding: Default::default(),
                    out_pointer_desc_0: todo!(),
                },
            )
        };
        horizon_svc::send_sync_request(self.handle.0)?;
        todo!("Command codegen")
    }
    pub fn pin_program(&self, loc: ProgramLocation) -> Result<PinId> {
        let data_in = loc;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            pre_padding: [u8; 4],
            cmif: CmifInHeader,
            raw_data: ProgramLocation,
            post_padding: [u8; 12],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 60]>;
        unsafe impl IpcBufferRepr for Request {}
        unsafe {
            ::core::ptr::write(
                get_ipc_buffer_for(),
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 0, 0, 0, true),
                    special_header: HipcSpecialHeader::new(false, 0, 0),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 2,
                        token: 0,
                    },
                    raw_data: data_in,
                    post_padding: Default::default(),
                },
            )
        };
        horizon_svc::send_sync_request(self.handle.0)?;
        todo!("Command codegen")
    }
    pub fn unpin_program(&self, id: PinId) -> Result<()> {
        let data_in = id;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            pre_padding: [u8; 4],
            cmif: CmifInHeader,
            raw_data: PinId,
            post_padding: [u8; 12],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 52]>;
        unsafe impl IpcBufferRepr for Request {}
        unsafe {
            ::core::ptr::write(
                get_ipc_buffer_for(),
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 0, 0, 0, true),
                    special_header: HipcSpecialHeader::new(false, 0, 0),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 3,
                        token: 0,
                    },
                    raw_data: data_in,
                    post_padding: Default::default(),
                },
            )
        };
        horizon_svc::send_sync_request(self.handle.0)?;
        todo!("Command codegen")
    }
    pub fn set_enabled_program_verification(&self, enabled: bool) -> Result<()> {
        let data_in = enabled;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            pre_padding: [u8; 4],
            cmif: CmifInHeader,
            raw_data: bool,
            post_padding: [u8; 12],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 45]>;
        unsafe impl IpcBufferRepr for Request {}
        unsafe {
            ::core::ptr::write(
                get_ipc_buffer_for(),
                Request {
                    hipc: HipcHeader::new(4, 0, 0, 0, 0, 0, 0, 0, true),
                    special_header: HipcSpecialHeader::new(false, 0, 0),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 4,
                        token: 0,
                    },
                    raw_data: data_in,
                    post_padding: Default::default(),
                },
            )
        };
        horizon_svc::send_sync_request(self.handle.0)?;
        todo!("Command codegen")
    }
}
