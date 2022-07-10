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
    fn Initialize() -> Result<()> {
        todo!("Command codegen")
    }
    fn GetService(name: ServiceName, session_handle: &mut RawHandle) -> Result<()> {
        todo!("Command codegen")
    }
    fn RegisterService(
        name: ServiceName,
        max_sessions: u32,
        is_light: bool,
        port_handle: &mut RawHandle,
    ) -> Result<()> {
        todo!("Command codegen")
    }
    fn UnregisterService(name: ServiceName) -> Result<()> {
        todo!("Command codegen")
    }
}
