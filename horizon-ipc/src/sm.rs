use core::fmt::{Debug, Display, Formatter};

/// Structure representing a service name
/// Null terminated if there is space for it, remaining characters set to zero
/// Must be ASCII (all bytes < 0x80)
#[repr(align(8))]
#[derive(PartialEq, Eq)]
pub struct SmServiceName {
    name: [u8; 8],
}

impl SmServiceName {
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

        // We can have
        8
    }

    pub fn as_str(&self) -> &str {
        let len = self.len();
        // SAFETY: service name is 0-8 bytes, so len can't return something out of bounds
        let bytes = unsafe { self.name.get_unchecked(..len) };
        // SAFETY: service names must be ASCII which is valid UTF8
        unsafe { core::str::from_utf8_unchecked(bytes) }
    }
}

impl Debug for SmServiceName {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        f.debug_tuple("SmServiceName")
            .field(&self.as_str())
            .finish()
    }
}

impl Display for SmServiceName {
    fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
        write!(f, "{:?}", self)
    }
}
