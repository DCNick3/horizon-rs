use horizon_error::Result;
use horizon_ipc::RawHandle;
use horizon_ipc::cmif::SessionHandle;
use horizon_ipc::raw::cmif::CmifInHeader;
use horizon_ipc::raw::hipc::{HipcHeader, HipcSpecialHeader};
#[repr(C)]
pub struct ServiceName {
    pub name: [u8; 8],
}
// Static size check for ServiceName (expect 8 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<ServiceName, [u8; 8]>;
};

pub struct IUserInterface {
    handle: SessionHandle,
}
impl IUserInterface {
    pub fn initialize() -> Result<()> {
        let data_in = 0u64;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            pid_placeholder: u64,
            pre_padding: [u8; 12],
            cmif: CmifInHeader,
            raw_data: u64,
            post_padding: [u8; 4],
        }
        todo!("Command codegen")
    }
    pub fn get_service(name: ServiceName) -> Result<RawHandle> {
        let data_in = name;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: ServiceName,
            post_padding: [u8; 8],
        }
        todo!("Command codegen")
    }
    pub fn register_service(
        name: ServiceName,
        max_sessions: u32,
        is_light: bool,
    ) -> Result<RawHandle> {
        #[repr(C)]
        struct In {
            name: ServiceName,
            is_light: bool,
            max_sessions: u32,
        }
        let _ = ::core::mem::transmute::<In, [u8; 16]>;
        let data_in: In = In { name, is_light, max_sessions };
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: In,
            post_padding: [u8; 8],
        }
        todo!("Command codegen")
    }
    pub fn unregister_service(name: ServiceName) -> Result<()> {
        let data_in = name;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            pre_padding: [u8; 4],
            cmif: CmifInHeader,
            raw_data: ServiceName,
            post_padding: [u8; 12],
        }
        todo!("Command codegen")
    }
}
