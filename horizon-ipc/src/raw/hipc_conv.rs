use super::hipc::{
    HipcHeader, HipcInPointerBufferDescriptor, HipcMapAliasBufferDescriptor,
    HipcOutPointerBufferDescriptor, HipcSpecialHeader,
};
use crate::conv_traits::{as_bytes_impl_transmute, from_bytes_impl_transmute};
use crate::hipc::MapAliasBufferMode;

as_bytes_impl_transmute!(HipcHeader);
as_bytes_impl_transmute!(HipcSpecialHeader);
as_bytes_impl_transmute!(HipcInPointerBufferDescriptor);
as_bytes_impl_transmute!(HipcOutPointerBufferDescriptor);
as_bytes_impl_transmute!(HipcMapAliasBufferDescriptor);

from_bytes_impl_transmute!(HipcHeader);
from_bytes_impl_transmute!(HipcSpecialHeader);
from_bytes_impl_transmute!(HipcInPointerBufferDescriptor);
from_bytes_impl_transmute!(HipcOutPointerBufferDescriptor);
from_bytes_impl_transmute!(HipcMapAliasBufferDescriptor);

impl HipcHeader {
    #[inline]
    pub fn new(
        type_: u16,
        num_in_pointers: u32,
        num_in_map_aliases: u32,
        num_out_map_aliases: u32,
        num_inout_map_aliases: u32,
        num_data_words: u32,
        out_pointer_mode: u32,
        recv_list_offset: u32,
        has_special_header: bool,
    ) -> Self {
        // TODO: make bitfield construction const
        Self {
            _bitfield_1: HipcHeader::new_bitfield_1(
                type_,
                num_in_pointers,
                num_in_map_aliases,
                num_out_map_aliases,
                num_inout_map_aliases,
                num_data_words,
                out_pointer_mode,
                0,
                recv_list_offset,
                has_special_header as _,
            ),
        }
    }
}

impl HipcSpecialHeader {
    #[inline]
    pub fn new(send_pid: bool, num_copy_handles: u32, num_move_handles: u32) -> Self {
        // TODO: make bitfield construction const
        Self {
            _bitfield_1: HipcSpecialHeader::new_bitfield_1(
                send_pid as _,
                num_copy_handles,
                num_move_handles,
                0,
            ),
        }
    }
}

impl HipcInPointerBufferDescriptor {
    #[inline]
    pub fn new(index: usize, address: usize, size: usize) -> Self {
        debug_assert_eq!(index >> 6, 0, "Invalid buffer index");
        debug_assert_eq!(address >> 39, 0, "Invalid buffer address");
        debug_assert_eq!(size >> 16, 0, "Invalid buffer size");

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
    #[inline]
    pub fn new(address: usize, size: usize) -> Self {
        debug_assert_eq!(address >> 39, 0, "Invalid buffer address");
        debug_assert_eq!(size >> 16, 0, "Invalid buffer size");

        let address_low = address as u32;
        let address_high = ((address >> 32) & 0b1111111) as u32;

        Self {
            _bitfield_1: Self::new_bitfield_1(address_high, size as _),
            address_low,
        }
    }
}

impl HipcMapAliasBufferDescriptor {
    #[inline]
    pub fn new(mode: MapAliasBufferMode, address: usize, size: usize) -> Self {
        debug_assert_eq!(address >> 39, 0, "Invalid buffer address");
        debug_assert_eq!(size >> 16, 0, "Invalid buffer size");

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
