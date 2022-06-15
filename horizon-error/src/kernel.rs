use crate::ErrorCodeModule;

// TODO: refine and export as a macro for defining an error code module
macro_rules! back_to_enum {
    ($(#[$meta:meta])* $vis:vis enum $name:ident {
        $($(#[$vmeta:meta])* $vname:ident $(= $val:expr)?,)*
    }) => {
        $(#[$meta])*
        $vis enum $name {
            $($(#[$vmeta])* $vname $(= $val)?,)*
        }

        impl core::convert::TryFrom<u32> for $name {
            type Error = ();

            fn try_from(v: u32) -> Result<Self, Self::Error> {
                match v {
                    $(x if x == $name::$vname as u32 => Ok($name::$vname),)*
                    _ => Err(()),
                }
            }
        }
    }
}

back_to_enum! {
    #[derive(Debug)]
    #[repr(u32)]
    pub enum KernelErrorCode {
        OutOfSessions = 7,

        InvalidArgument = 14,

        NotImplemented = 33,

        StopProcessingException = 54,

        NoSynchronizationObject = 57,

        TerminationRequested = 59,

        NoEvent = 70,

        InvalidSize = 101,
        InvalidAddress = 102,
        OutOfResource = 103,
        OutOfMemory = 104,
        OutOfHandles = 105,
        InvalidCurrentMemory = 106,

        InvalidNewMemoryPermission = 108,

        InvalidMemoryRegion = 110,

        InvalidPriority = 112,
        InvalidCoreId = 113,
        InvalidHandle = 114,
        InvalidPointer = 115,
        InvalidCombination = 116,
        TimedOut = 117,
        Cancelled = 118,
        OutOfRange = 119,
        InvalidEnumValue = 120,
        NotFound = 121,
        Busy = 122,
        SessionClosed = 123,
        NotHandled = 124,
        InvalidState = 125,
        ReservedUsed = 126,
        NotSupported = 127,
        Debug = 128,
        NoThread = 129,
        UnknownThread = 130,
        PortClosed = 131,
        LimitReached = 132,
        InvalidMemoryPool = 133,

        ReceiveListBroken = 258,
        OutOfAddressSpace = 259,
        MessageTooLarge = 260,

        InvalidProcessId = 517,
        InvalidThreadId = 518,
        InvalidId = 519,
        ProcessTerminated = 520,
    }
}

impl ErrorCodeModule for KernelErrorCode {
    const MODULE: u32 = 1;

    fn from_desc(desc: u32) -> Self {
        KernelErrorCode::try_from(desc).expect("Unknown kernel error code")
    }
}
