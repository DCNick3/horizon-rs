use crate::conv_traits::{CountingWriter, ReadFromBytes, Reader, WriteAsBytes, Writer};
use crate::raw::hipc::{
    HipcHeader, HipcInPointerBufferDescriptor, HipcMapAliasBufferDescriptor,
    HipcOutPointerBufferDescriptor, HipcSpecialHeader,
};
use arrayvec::ArrayVec;
use core::marker::PhantomData;
use horizon_svc::RawHandle;

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

pub struct ConstBuffer<'a> {
    address: usize,
    size: usize,
    phantom: PhantomData<&'a ()>,
}

impl<'a> ConstBuffer<'a> {
    pub const fn null() -> Self {
        Self {
            address: 0,
            size: 0,
            phantom: PhantomData {},
        }
    }

    pub fn null_ref() -> &'static Self {
        static NULL: ConstBuffer<'static> = ConstBuffer::null();

        &NULL
    }

    pub fn from_str(s: &'a str) -> Self {
        Self {
            // TODO: pointers cannot be cast to integers during const eval
            address: s.as_ptr() as usize,
            size: s.len(),
            phantom: PhantomData {},
        }
    }

    pub fn from_bytes(b: &'a [u8]) -> Self {
        Self {
            // TODO: pointers cannot be cast to integers during const eval
            address: b.as_ptr() as usize,
            size: b.len(),
            phantom: PhantomData {},
        }
    }

    // TODO: more constructors

    pub fn size(&self) -> usize {
        self.size
    }
}

pub struct MutBuffer<'a> {
    address: usize,
    size: usize,
    phantom: PhantomData<&'a ()>,
}

impl<'a> MutBuffer<'a> {
    pub const fn null() -> Self {
        Self {
            address: 0,
            size: 0,
            phantom: PhantomData {},
        }
    }

    pub fn null_ref() -> &'static Self {
        static NULL: MutBuffer<'static> = MutBuffer::null();

        &NULL
    }

    pub fn from_bytes(b: &'a mut [u8]) -> Self {
        Self {
            // TODO: pointers cannot be cast to integers during const eval
            address: b.as_ptr() as usize,
            size: b.len(),
            phantom: PhantomData {},
        }
    }

    pub unsafe fn from_slice_transmute<T>(b: &'a mut [T]) -> Self {
        Self {
            address: b.as_ptr() as usize,
            size: b.len() * core::mem::size_of::<T>(),
            phantom: PhantomData {},
        }
    }

    // TODO: more constructors

    pub fn size(&self) -> usize {
        self.size
    }
}

pub enum Buffer<'a> {
    PointerIn(ConstBuffer<'a>),
    PointerOut(MutBuffer<'a>),

    MapAliasIn(MapAliasBufferMode, ConstBuffer<'a>),
    MapAliasOut(MapAliasBufferMode, MutBuffer<'a>),
    MapAliasInOut(MapAliasBufferMode, MutBuffer<'a>),
}

impl<'a> Buffer<'a> {
    pub fn size(&self) -> usize {
        match self {
            Buffer::PointerIn(b) => b.size,
            Buffer::PointerOut(b) => b.size,
            Buffer::MapAliasIn(_, b) => b.size,
            Buffer::MapAliasOut(_, b) => b.size,
            Buffer::MapAliasInOut(_, b) => b.size,
        }
    }
}

pub trait HipcPayloadIn<'a> {
    fn get_type(&self) -> u16;

    fn get_pointer_in_buffers(&self) -> ArrayVec<&'a ConstBuffer<'a>, 8>;
    fn get_pointer_out_buffers(&self) -> ArrayVec<&'a MutBuffer<'a>, 8>;

    fn get_map_alias_in_buffers(&self) -> ArrayVec<(MapAliasBufferMode, &'a ConstBuffer<'a>), 8>;
    fn get_map_alias_out_buffers(&self) -> ArrayVec<(MapAliasBufferMode, &'a MutBuffer<'a>), 8>;
    fn get_map_alias_in_out_buffers(&self) -> ArrayVec<(MapAliasBufferMode, &'a MutBuffer<'a>), 8>;

    fn get_send_pid(&self) -> Option<u64>;

    fn get_copy_handles(&self) -> &[RawHandle];
    fn get_move_handles(&self) -> &[RawHandle];

    fn write_as_bytes(&self, dest: &mut (impl Writer + ?Sized));

    #[inline]
    fn size(&self) -> usize {
        let mut writer = CountingWriter::new();

        self.write_as_bytes(&mut writer);

        writer.count()
    }
}

pub struct Request<'p, P: HipcPayloadIn<'p>> {
    pub payload: &'p P,
}

impl<'p, P: HipcPayloadIn<'p>> WriteAsBytes for Request<'p, P> {
    fn write_as_bytes(&self, dest: &mut (impl Writer + ?Sized)) {
        let in_pointers = self.payload.get_pointer_in_buffers();
        let out_pointers = self.payload.get_pointer_out_buffers();

        let in_map_aliases = self.payload.get_map_alias_in_buffers();
        let out_map_aliases = self.payload.get_map_alias_out_buffers();
        let in_out_map_aliases = self.payload.get_map_alias_in_out_buffers();

        let send_pid = self.payload.get_send_pid();
        let move_handles = self.payload.get_move_handles();
        let copy_handles = self.payload.get_move_handles();

        let has_special_header =
            !move_handles.is_empty() || !copy_handles.is_empty() || send_pid.is_some();

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
                in_pointers.len() as _,
                in_map_aliases.len() as _,
                out_map_aliases.len() as _,
                in_out_map_aliases.len() as _,
                payload_size_in_words as _,
                if out_pointers.len() == 0 {
                    // If it has value 0, the C descriptor functionality is disabled.
                    0
                } else {
                    // If it has value 1, there is an "inlined" C buffer after the raw data.
                    //    Received data is copied to ROUND_UP(cmdbuf+raw_size+index, 16)
                    // If it has value 2, there is a single C descriptor, which gets all the buffers that were sent
                    // Otherwise it has (flag-2) C descriptors.
                    //   In this case, index picks which C descriptor to copy
                    //   received data to [instead of picking the offset into the buffer].
                    (2 + out_pointers.len()) as _
                },
                0,
                0,
                has_special_header as _,
            ),
        });

        assert!(copy_handles.len() < (1 << 4));
        assert!(move_handles.len() < (1 << 4));

        if has_special_header {
            dest.write(&HipcSpecialHeader {
                _bitfield_1: HipcSpecialHeader::new_bitfield_1(
                    send_pid.is_some() as _,
                    copy_handles.len() as _,
                    move_handles.len() as _,
                    0,
                ),
            });

            if let Some(pid) = &send_pid {
                dest.write(pid)
            }

            // TODO: allow sending 0 as a handle
            for handle in copy_handles {
                dest.write(&handle.0)
            }
            for handle in move_handles {
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
        for (i, buf) in in_pointers.iter().enumerate() {
            dest.write(&HipcInPointerBufferDescriptor::new(
                i,
                buf.address,
                buf.size,
            ))
        }

        // in_map_aliases
        for (mode, buf) in in_map_aliases {
            dest.write(&HipcMapAliasBufferDescriptor::new(
                mode,
                buf.address,
                buf.size,
            ))
        }

        // out_map_aliases
        for (mode, buf) in out_map_aliases {
            dest.write(&HipcMapAliasBufferDescriptor::new(
                mode,
                buf.address,
                buf.size,
            ))
        }

        // in_out_map_aliases
        for (mode, buf) in in_out_map_aliases {
            dest.write(&HipcMapAliasBufferDescriptor::new(
                mode,
                buf.address,
                buf.size,
            ))
        }

        // payload
        self.payload.write_as_bytes(dest);

        // out_pointers
        for buf in out_pointers {
            dest.write(&HipcOutPointerBufferDescriptor::new(buf.address, buf.size))
        }
    }
}

pub struct Response<'d, P: ReadFromBytes<'d>> {
    // TODO: static/pointer recv descriptors
    pub pid: Option<u64>,
    pub move_handles: Handles<'d>,
    pub copy_handles: Handles<'d>,
    pub payload: P,
}

impl<'d, P: ReadFromBytes<'d>> ReadFromBytes<'d> for Response<'d, P> {
    fn read_from_bytes(src: &mut (impl Reader<'d> + ?Sized)) -> Self {
        let header = src.read::<HipcHeader>();

        debug_assert_eq!(header.type_(), 0);

        let num_pointer_desc = header.num_in_pointers();

        debug_assert_eq!(header.num_in_map_aliases(), 0);
        debug_assert_eq!(header.num_out_map_aliases(), 0);
        debug_assert_eq!(header.num_inout_map_aliases(), 0);

        let _payload_size = header.num_data_words() * 4;

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

            (pid, Handles::new(copy_handles), Handles::new(move_handles))
        } else {
            (None, Handles::empty(), Handles::empty())
        };

        if num_pointer_desc != 0 {
            todo!("Reading pointer descriptors from the response")
        }

        // TODO: we may use the payload size to limit the bytes read to validate the payload read logic
        // but yuzu calculates the payload_size in a wrong way, so we can't use it (at least for now)

        // let payload_data = src.read_bytes()

        let payload = src.read::<P>();

        Self {
            pid,
            move_handles,
            copy_handles,
            payload,
        }
    }
}

pub struct Handles<'d> {
    handles: &'d [u8],
}

impl<'d> Handles<'d> {
    const HANDLE_SIZE: usize = 4;

    pub fn empty() -> Self {
        Self { handles: &[] }
    }

    pub fn new(handles: &'d [u8]) -> Self {
        assert_eq!(handles.len() % Self::HANDLE_SIZE, 0);
        Self { handles: handles }
    }

    pub fn len(&self) -> usize {
        self.handles.len() / Self::HANDLE_SIZE
    }

    /// # Safety
    /// Index should be in bounds
    unsafe fn get_unchecked(&self, index: usize) -> RawHandle {
        let v = u32::from_le_bytes(
            self.handles
                // SAFETY: index should be in bounds
                .get_unchecked(index * Self::HANDLE_SIZE..(index + 1) * Self::HANDLE_SIZE)
                .try_into()
                // SAFETY: the slice should be always the length of Self::HANDLE_SIZE (4) which is suitable for u32
                .unwrap_unchecked(),
        );

        RawHandle(v)
    }

    /// Converts the raw handles into an array of handles of fixed size  
    ///
    /// Panics if the size is mismatched
    pub fn into_array<const SIZE: usize>(self) -> [RawHandle; SIZE] {
        let len = self.len();
        assert_eq!(len, SIZE);
        let mut r = [RawHandle(0); SIZE];

        for i in 0..len {
            // SAFETY: i is in bounds
            r[i] = unsafe { self.get_unchecked(i) };
        }

        r
    }
}
