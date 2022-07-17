/// Determines what MemoryState to use with the mapped memory in the sysmodule.
/// Used to enforce whether or not device mapping is allowed for src and dst buffers respectively.
#[repr(u32)]
#[derive(Copy, Clone, Eq, PartialEq)]
pub enum MapAliasBufferMode {
    /// Device mapping *not* allowed for src or dst.
    Normal = 0,
    /// Device mapping allowed for src and dst.
    NonSecure = 1,
    /// This buffer mode is invalid
    Invalid = 2,
    // Device mapping allowed for src but not for dst.
    NonDevice = 3,
}
