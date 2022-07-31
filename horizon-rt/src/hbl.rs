ij_core_workaround!();

use bitflags::bitflags;

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(u32)]
#[allow(unused)] // TODO: implement HBABI keys parsing
pub enum AbiConfigEntryKey {
    EndOfList = 0,
    MainThreadHandle = 1,
    NextLoadPath = 2,
    OverrideHeap = 3,
    OverrideService = 4,
    Argv = 5,
    SyscallAvailableHint = 6,
    AppletType = 7,
    AppletWorkaround = 8,
    Reserved9 = 9,
    ProcessHandle = 10,
    LastLoadResult = 11,
    RandomSeed = 14,
    UserIdStorage = 15,
    HosVersion = 16,
}

bitflags! {
    pub struct AbiConfigEntryFlags: u32 {
        const MANDATORY = 1;
    }
}

bitflags! {
    pub struct AbiConfigAppletFlags: u32 {
        const APPLICATION_OVERRIDE = 1;
    }
}

#[derive(Copy, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, Debug)]
#[repr(C)]
pub struct AbiConfigEntry {
    pub key: AbiConfigEntryKey,
    pub flags: AbiConfigEntryFlags,
    pub value: [u64; 2],
}
