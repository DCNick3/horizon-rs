use horizon_error::Result;
use horizon_ipc::RawHandle;
use horizon_ipc::cmif::SessionHandle;
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
    fn initialize() -> Result<()> {
        todo!("Command codegen")
    }
    fn get_service(name: ServiceName) -> Result<RawHandle> {
        todo!("Command codegen")
    }
    fn register_service(
        name: ServiceName,
        max_sessions: u32,
        is_light: bool,
    ) -> Result<RawHandle> {
        todo!("Command codegen")
    }
    fn unregister_service(name: ServiceName) -> Result<()> {
        todo!("Command codegen")
    }
}
