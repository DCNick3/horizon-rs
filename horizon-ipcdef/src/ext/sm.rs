use crate::gen::sm::IUserInterface;
use crate::sm::ServiceName;
use core::fmt::{Display, Formatter};
use horizon_error::Result;
use horizon_ipc::handle_storage::OwnedHandle;
use horizon_svc::RawHandle;

pub trait SmServiceType: From<RawHandle> {}

pub trait SmService {
    type Type: SmServiceType;
    fn name() -> ServiceName;
}

impl IUserInterface {
    pub fn open_named_port() -> Result<Self> {
        let handle = unsafe { horizon_svc::connect_to_named_port(b"sm:\0") }?;
        Ok(Self {
            handle: OwnedHandle::new(handle),
        })
    }
}

impl ServiceName {
    pub fn try_new(name: &str) -> Option<Self> {
        if name.bytes().len() >= 8 {
            return None;
        }

        let mut name_buffer = [0u8; 8];
        for (dst, name_byte) in name_buffer.iter_mut().zip(name.bytes()) {
            if name_byte >= 0x80 {
                return None;
            }

            *dst = name_byte;
        }

        Some(Self { name: name_buffer })
    }

    pub fn len(&self) -> usize {
        for res in 0..7 {
            if self.name[res] == 0 {
                return res;
            }
        }

        // SAFETY: we can't make a string with length > 7, so this should not be reachable
        unsafe { core::hint::unreachable_unchecked() }
    }

    pub fn as_str(&self) -> &str {
        let len = self.len();
        // SAFETY: service name is 0-7 bytes, so len can't return something out of bounds
        let bytes = unsafe { self.name.get_unchecked(..len) };
        // SAFETY: service names must be ASCII which is valid UTF8
        unsafe { core::str::from_utf8_unchecked(bytes) }
    }
}

impl Display for ServiceName {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("ServiceName").field(&self.as_str()).finish()
    }
}
