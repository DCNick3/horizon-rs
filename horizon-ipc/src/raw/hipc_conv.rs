use super::hipc::{
    HipcHeader, HipcInPointerBufferDescriptor, HipcMapAliasBufferDescriptor,
    HipcOutPointerBufferDescriptor, HipcSpecialHeader,
};
use crate::conv_traits::as_bytes_impl_transmute;
use crate::hipc::MapAliasBufferMode;

as_bytes_impl_transmute!(HipcHeader);
as_bytes_impl_transmute!(HipcSpecialHeader);
as_bytes_impl_transmute!(HipcInPointerBufferDescriptor);
as_bytes_impl_transmute!(HipcOutPointerBufferDescriptor);
as_bytes_impl_transmute!(HipcMapAliasBufferDescriptor);

impl HipcInPointerBufferDescriptor {
    pub fn new(index: usize, address: usize, size: usize) -> Self {
        assert_eq!(index >> 6, 0, "Invalid buffer index");
        assert_eq!(address >> 39, 0, "Invalid buffer address");
        assert_eq!(size >> 16, 0, "Invalid buffer size");

        let address_low = address as u32;
        let address_mid = ((address >> 32) & 0b1111) as u32;
        let address_high = ((address >> 36) & 0b111) as u32;

        Self {
            _bitfield_1: Self::new_bitfield_1(index as u32, address_high, address_mid, size as _),
            address_low,
        }
    }
}

impl HipcOutPointerBufferDescriptor {
    pub fn new(address: usize, size: usize) -> Self {
        assert_eq!(address >> 39, 0, "Invalid buffer address");
        assert_eq!(size >> 16, 0, "Invalid buffer size");

        let address_low = address as u32;
        let address_high = ((address >> 32) & 0b1111111) as u32;

        Self {
            _bitfield_1: Self::new_bitfield_1(address_high, size as _),
            address_low,
        }
    }
}

impl HipcMapAliasBufferDescriptor {
    pub fn new(mode: MapAliasBufferMode, address: usize, size: usize) -> Self {
        assert_eq!(address >> 39, 0, "Invalid buffer address");
        assert_eq!(size >> 16, 0, "Invalid buffer size");

        let address_low = address as u32;
        let address_mid = ((address >> 32) & 0b1111) as u32;
        let address_high = ((address >> 36) & 0b111) as u32;

        let size_low = size as u32;
        let size_high = ((size >> 32) & 0b1111) as u32;
        Self {
            size_low,
            address_low,
            _bitfield_1: Self::new_bitfield_1(mode as u32, address_high, size_high, address_mid),
        }
    }
}
