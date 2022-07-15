use crate::gen::sm::IUserInterface;
use horizon_error::Result;
use horizon_ipc::cmif::SessionHandle;

impl IUserInterface {
    pub fn new() -> Result<Self> {
        let handle = unsafe { horizon_svc::connect_to_named_port(b"sm:\0") }?;
        Ok(Self {
            handle: SessionHandle(handle),
        })
    }
}
