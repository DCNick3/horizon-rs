use crate::conv_traits::{WriteAsBytes, Writer};
use crate::raw::hipc::{
    HipcHeader, HipcInPointerBufferDescriptor, HipcMapAliasBufferDescriptor,
    HipcOutPointerBufferDescriptor, HipcSpecialHeader,
};
use core::marker::PhantomData;
use horizon_svc::RawHandle;

/// Determines what MemoryState to use with the mapped memory in the sysmodule.
/// Used to enforce whether or not device mapping is allowed for src and dst buffers respectively.
#[repr(u32)]
#[derive(Copy, Clone)]
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

pub struct ConstBuffer<'a> {
    address: usize,
    size: usize,
    phantom: PhantomData<&'a ()>,
}

pub struct MutBuffer<'a> {
    address: usize,
    size: usize,
    phantom: PhantomData<&'a ()>,
}

pub enum Buffer<'a> {
    PointerIn(ConstBuffer<'a>),
    PointerOut(MutBuffer<'a>),

    MapAliasIn(MapAliasBufferMode, ConstBuffer<'a>),
    MapAliasOut(MapAliasBufferMode, MutBuffer<'a>),
    MapAliasInOut(MapAliasBufferMode, MutBuffer<'a>),
}

pub struct Request<'a, 'b, P: WriteAsBytes> {
    pub ty: u16,
    pub send_pid: bool,
    pub buffers: &'b [Buffer<'a>],
    pub copy_handles: &'b [RawHandle],
    pub move_handles: &'b [RawHandle],
    pub payload: &'b P,
}

impl<'a, 'b, P: WriteAsBytes> WriteAsBytes for Request<'a, 'b, P> {
    fn write_as_bytes(&self, dest: &mut (impl Writer + ?Sized)) {
        let mut in_pointers_count = 0;
        let mut out_pointers_count = 0;

        let mut in_map_aliases_count = 0;
        let mut out_map_aliases_count = 0;
        let mut in_out_map_aliases_count = 0;

        for buffer in self.buffers {
            match buffer {
                Buffer::PointerIn(_) => in_pointers_count += 1,
                Buffer::PointerOut(_) => out_pointers_count += 1,
                Buffer::MapAliasIn(_, _) => in_map_aliases_count += 1,
                Buffer::MapAliasOut(_, _) => out_map_aliases_count += 1,
                Buffer::MapAliasInOut(_, _) => in_out_map_aliases_count += 1,
            }
        }

        let has_special_header =
            !self.move_handles.is_empty() || !self.copy_handles.is_empty() || self.send_pid;

        let payload_size = self.payload.size();

        // compute as ceil(payload_size / 4)
        let payload_size_in_words = (payload_size + 3) / 4;

        // I do not assert because it will probably run out of TLS buffer space before it even reaches this size
        // TLS IPC buffer is 0x200 (512) bytes

        // assert!(
        //     payload_size_in_words < (1 << 10),
        //     "Payload size is too large to fit into size field"
        // );

        dest.write(&HipcHeader {
            _bitfield_1: HipcHeader::new_bitfield_1(
                self.ty,
                in_pointers_count,
                in_map_aliases_count,
                out_map_aliases_count,
                in_out_map_aliases_count,
                payload_size_in_words as _,
                if out_pointers_count == 0 {
                    // If it has value 0, the C descriptor functionality is disabled.
                    0
                } else {
                    // If it has value 1, there is an "inlined" C buffer after the raw data.
                    //    Received data is copied to ROUND_UP(cmdbuf+raw_size+index, 16)
                    // If it has value 2, there is a single C descriptor, which gets all the buffers that were sent
                    // Otherwise it has (flag-2) C descriptors.
                    //   In this case, index picks which C descriptor to copy
                    //   received data to [instead of picking the offset into the buffer].
                    2 + out_pointers_count
                },
                0,
                0,
                has_special_header as _,
            ),
        });

        assert!(self.copy_handles.len() < (1 << 4));
        assert!(self.move_handles.len() < (1 << 4));

        if has_special_header {
            dest.write(&HipcSpecialHeader {
                _bitfield_1: HipcSpecialHeader::new_bitfield_1(
                    self.send_pid as _,
                    self.copy_handles.len() as _,
                    self.move_handles.len() as _,
                    0,
                ),
            })
        }

        // descriptors go in order:
        // in_pointers
        // in_map_aliases
        // out_map_aliases
        // in_out_map_aliases
        // payload
        // out_pointers

        // in_pointers
        for (i, buffer) in self.buffers.iter().enumerate() {
            if let Buffer::PointerIn(buf) = buffer {
                dest.write(&HipcInPointerBufferDescriptor::new(
                    i,
                    buf.address,
                    buf.size,
                ))
            }
        }

        // in_map_aliases
        for buffer in self.buffers.iter() {
            if let Buffer::MapAliasIn(mode, buf) = buffer {
                dest.write(&HipcMapAliasBufferDescriptor::new(
                    *mode,
                    buf.address,
                    buf.size,
                ))
            }
        }

        // out_map_aliases
        for buffer in self.buffers.iter() {
            if let Buffer::MapAliasOut(mode, buf) = buffer {
                dest.write(&HipcMapAliasBufferDescriptor::new(
                    *mode,
                    buf.address,
                    buf.size,
                ))
            }
        }

        // in_out_map_aliases
        for buffer in self.buffers.iter() {
            if let Buffer::MapAliasInOut(mode, buf) = buffer {
                dest.write(&HipcMapAliasBufferDescriptor::new(
                    *mode,
                    buf.address,
                    buf.size,
                ))
            }
        }

        // payload
        dest.write(self.payload);

        // out_pointers
        for buffer in self.buffers.iter() {
            if let Buffer::PointerOut(buf) = buffer {
                dest.write(&HipcOutPointerBufferDescriptor::new(buf.address, buf.size))
            }
        }
    }
}
