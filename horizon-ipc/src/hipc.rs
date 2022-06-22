use crate::conv_traits::{ReadFromBytes, Reader, WriteAsBytes, Writer};
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

pub trait HipcPayloadIn: WriteAsBytes {
    fn get_type(&self) -> u16;
}

pub struct Request<'a, 'b, P: HipcPayloadIn> {
    pub send_pid: Option<u64>,
    pub buffers: &'b [Buffer<'a>],
    pub copy_handles: &'b [RawHandle],
    pub move_handles: &'b [RawHandle],
    pub payload: &'b P,
}

impl<'a, 'b, P: HipcPayloadIn> WriteAsBytes for Request<'a, 'b, P> {
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

        let has_special_header = !self.move_handles.is_empty()
            || !self.copy_handles.is_empty()
            || self.send_pid.is_some();

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
                self.payload.get_type(),
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
                    self.send_pid.is_some() as _,
                    self.copy_handles.len() as _,
                    self.move_handles.len() as _,
                    0,
                ),
            });

            if let Some(pid) = &self.send_pid {
                dest.write(pid)
            }

            // TODO: allow sending 0 as a handle
            for handle in self.copy_handles {
                dest.write(&handle.0)
            }
            for handle in self.move_handles {
                dest.write(&handle.0)
            }
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

pub struct Response<'d, P: ReadFromBytes<'d>> {
    // TODO: static/pointer recv descriptors
    pid: Option<u64>,
    move_handles: &'d [u8],
    copy_handles: &'d [u8],
    payload: P,
}

impl<'d, P: ReadFromBytes<'d>> Response<'d, P> {
    pub fn payload(&self) -> &P {
        &self.payload
    }
}

impl<'d, P: ReadFromBytes<'d>> ReadFromBytes<'d> for Response<'d, P> {
    fn read_from_bytes(src: &mut (impl Reader<'d> + ?Sized)) -> Self {
        let header = src.read::<HipcHeader>();

        debug_assert_eq!(header.type_(), 0);

        let num_pointer_desc = header.num_in_pointers();

        debug_assert_eq!(header.num_in_map_aliases(), 0);
        debug_assert_eq!(header.num_out_map_aliases(), 0);
        debug_assert_eq!(header.num_inout_map_aliases(), 0);

        let payload_size = header.num_data_words() * 4;

        debug_assert_eq!(header.out_pointer_mode(), 0);

        let has_special_header = header.has_special_header() != 0;

        let (pid, copy_handles, move_handles) = if has_special_header {
            let special_header = src.read::<HipcSpecialHeader>();

            let send_pid = special_header.send_pid() != 0;
            let num_copy_handles = special_header.num_copy_handles();
            let num_move_handles = special_header.num_move_handles();

            let pid = if send_pid {
                Some(src.read::<u64>())
            } else {
                None
            };

            let copy_handles = src.read_bytes((num_copy_handles * 4) as _);
            let move_handles = src.read_bytes((num_move_handles * 4) as _);

            (pid, copy_handles, move_handles)
        } else {
            (None, [].as_slice(), [].as_slice())
        };

        if num_pointer_desc != 0 {
            todo!("Reading pointer descriptors from the response")
        }

        let payload = src.read::<P>();

        Self {
            pid,
            move_handles,
            copy_handles,
            payload,
        }
    }
}
