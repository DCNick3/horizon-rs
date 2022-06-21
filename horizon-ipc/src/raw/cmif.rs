use horizon_error::ErrorCode;

use crate::conv_traits::as_bytes_impl_transmute;

#[repr(C)]
pub struct CmifInHeader {
    pub magic: u32,
    pub version: u32,
    pub command_id: u32,
    pub token: u32,
}

impl CmifInHeader {
    pub const MAGIC: u32 = 0x49434653; // "SFCI"
}

as_bytes_impl_transmute!(CmifInHeader);

#[repr(C)]
pub struct CmifOutHeader {
    pub magic: u32,
    pub version: u32,
    pub result: ErrorCode,
    pub token: u32,
}

impl CmifOutHeader {
    pub const MAGIC: u32 = 0x4F434653; // "SFCO"
}

as_bytes_impl_transmute!(CmifOutHeader);

#[repr(C)]
pub struct CmifDomainInHeader {
    pub type_: u8,
    pub num_in_objects: u8,
    pub data_size: u16,
    pub object_id: u32,
    pub padding: u32,
    pub token: u32,
}

as_bytes_impl_transmute!(CmifDomainInHeader);

#[repr(C)]
pub struct CmifDomainOutHeader {
    pub num_out_objects: u32,
    pub padding: [u32; 3],
}

as_bytes_impl_transmute!(CmifDomainOutHeader);
