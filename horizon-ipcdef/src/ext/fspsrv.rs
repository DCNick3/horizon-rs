ij_core_workaround!();

use crate::fssrv::{IFileSystemProxy, Path};
use crate::sm::{IUserInterface, ServiceName};
use core::str::Utf8Error;
use horizon_error::Result;
use horizon_global::services;

impl IFileSystemProxy {
    pub fn get() -> Result<IFileSystemProxy<services::fs::Guard>> {
        Ok(IFileSystemProxy::new(services::fs::get_or_connect(|| {
            let sm = IUserInterface::get()?;
            sm.get_service(ServiceName::try_new("fsp-srv").unwrap())
        })?))
    }
}

const PATH_SIZE: usize = 0x300;

impl Path {
    pub fn new(s: impl AsRef<[u8]>) -> Self {
        Self::try_new(s).expect("Path was too big to fit into a buffer")
    }

    pub fn try_new(s: impl AsRef<[u8]>) -> Option<Self> {
        let s = s.as_ref();
        let mut r = Self { str: [0; 0x301] };

        if s.len() > PATH_SIZE {
            return None;
        }
        r.str[..s.len()].copy_from_slice(s);

        Some(r)
    }

    pub fn as_str(&self) -> core::result::Result<&str, Utf8Error> {
        core::str::from_utf8(self.as_ref())
    }
}

impl AsRef<[u8]> for Path {
    fn as_ref(&self) -> &[u8] {
        let (len, _) = self
            .str
            .iter()
            .cloned()
            .enumerate()
            .find(|&(_, p)| p == 0)
            .unwrap();
        &self.str[..len]
    }
}
