use crate::sm::{IUserInterface, ServiceName};
use crate::spl::IRandomInterface;
use horizon_error::Result;
use horizon_global::services;

ij_core_workaround!();

impl IRandomInterface {
    pub fn get() -> Result<IRandomInterface<services::csrng::Guard>> {
        Ok(IRandomInterface::new(services::csrng::get_or_connect(
            || {
                let sm = IUserInterface::get()?;
                sm.get_service(ServiceName::try_new("csrng").unwrap())
            },
        )?))
    }
}
