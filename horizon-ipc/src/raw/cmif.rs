use horizon_error::ErrorCode;

#[repr(C)]
pub struct CmifInHeader {
    pub magic: u32,
    pub version: u32,
    pub command_id: u32,
    pub token: u32,
}

#[repr(C)]
pub struct CmifOutHeader {
    pub magic: u32,
    pub version: u32,
    pub result: ErrorCode,
    pub token: u32,
}

#[repr(C)]
pub struct CmifDomainInHeader {
    pub type_: u8,
    pub num_in_objects: u8,
    pub data_size: u16,
    pub object_id: u32,
    pub padding: u32,
    pub token: u32,
}

#[repr(C)]
pub struct CmifDimainOutHeader {
    pub num_out_objects: u32,
    pub padding: [u32; 3],
}
