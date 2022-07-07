use crate::conv_traits::{ReadFromBytes, Reader, WriteAsBytes, Writer};
use crate::hipc::{ConstBuffer, HipcPayloadIn, MapAliasBufferMode, MutBuffer};
use crate::raw::cmif::{CmifDomainInHeader, CmifInHeader, CmifOutHeader};
use arrayvec::ArrayVec;
use core::marker::PhantomData;
use core::ops::Deref;
use horizon_error::ErrorCode;
use horizon_svc::RawHandle;

#[repr(u16)]
#[derive(Copy, Clone)]
pub enum CommandType {
    Invalid = 0,
    LegacyRequest = 1,
    Close = 2,
    LegacyControl = 3,
    Request = 4,
    Control = 5,
    RequestWithContext = 6,
    ControlWithContext = 7,
}

pub trait AsRawSessionHandle {
    fn raw(&self) -> RawHandle;
}

/// A non-type-safe owning handle to some IPC session
pub struct SessionHandle(RawHandle);

impl SessionHandle {
    pub fn as_ref(&self) -> SessionHandleRef {
        SessionHandleRef::new(self)
    }
}

impl AsRawSessionHandle for SessionHandle {
    fn raw(&self) -> RawHandle {
        self.0
    }
}

impl Drop for SessionHandle {
    fn drop(&mut self) {
        horizon_svc::close_handle(self.0).unwrap()
    }
}

/// A non-type-safe non-owning handle to some IPC session
#[derive(Copy, Clone)]
pub struct SessionHandleRef<'a> {
    object: RawHandle,
    phantom: PhantomData<&'a ()>,
}

impl<'a> SessionHandleRef<'a> {
    pub fn new(obj_ref: &'a SessionHandle) -> Self {
        Self {
            object: obj_ref.raw(),
            phantom: PhantomData::default(),
        }
    }

    pub fn raw(&self) -> RawHandle {
        self.object
    }
}

impl<'a> AsRawSessionHandle for SessionHandleRef<'a> {
    fn raw(&self) -> RawHandle {
        self.object
    }
}

/// A handle to an IPC object that must be a domain object
pub struct DomainHandle(SessionHandle);

impl Deref for DomainHandle {
    type Target = SessionHandle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

#[derive(Copy, Clone)]
pub struct DomainHandleRef<'a>(SessionHandleRef<'a>);

impl<'a> Deref for DomainHandleRef<'a> {
    type Target = SessionHandleRef<'a>;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct DomainObject<'a> {
    domain: DomainHandleRef<'a>,
    object_id: u32,
}

impl<'a> DomainObject<'a> {
    pub fn get_domain(&self) -> DomainHandleRef<'a> {
        self.domain
    }
}

impl<'a> Drop for DomainObject<'a> {
    fn drop(&mut self) {
        todo!("Implement when there will be a CMIF IPC definitions for sending the close requests")
    }
}

pub struct DomainObjectRef<'a> {
    domain: DomainHandleRef<'a>,
    object_id: u32,
    phantom: PhantomData<&'a ()>,
}

// TODO: will we actually have functions that are agnostic to the kind of object we are using?
/// A way to refer to an IPC object
pub enum ObjectReference<'a> {
    /// Direct reference to an object (a session handle)
    SessionObject(SessionHandleRef<'a>),
    /// Reference to an object inside a domain (a domain session handle and an object id)
    DomainObject(DomainObjectRef<'a>),
}

#[repr(u8)]
#[derive(Copy, Clone)]
pub enum DomainRequestType {
    SendMessage = 1,
    CloseHandle = 2,
}

impl WriteAsBytes for DomainRequestType {
    fn write_as_bytes(&self, dest: &mut (impl Writer + ?Sized)) {
        dest.write(&(*self as u8));
    }
}

pub struct DomainRequest<'a, T: WriteAsBytes> {
    request_type: DomainRequestType,
    /// List of objects in the domain to pass to the function as parameters
    input_objects: &'a [u32],
    /// Id of the object in the domain on which to operate (use as this)
    object_id: u32,
    normal_request: NormalRequest<'a, T>,
}

impl<'a, T: WriteAsBytes> WriteAsBytes for DomainRequest<'a, T> {
    fn write_as_bytes(&self, dest: &mut (impl Writer + ?Sized)) {
        let normal_request_size = WriteAsBytes::size(&self.normal_request);

        dest.write(&CmifDomainInHeader {
            type_: self.request_type as u8,
            num_in_objects: self.input_objects.len().try_into().unwrap(),
            data_size: normal_request_size.try_into().unwrap(),
            object_id: self.object_id,
            padding: 0,
            token: 0,
        });

        dest.write(&self.normal_request);
    }
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub enum BufferMode {
    Pointer,
    MapAlias(MapAliasBufferMode),
    AutoSelect,
}

#[derive(Eq, PartialEq, Copy, Clone)]
pub struct BufferType {
    pub mode: BufferMode,
    pub is_fixed_size: bool,
}

pub enum BufferContent<'a> {
    In(ConstBuffer<'a>),
    Out(MutBuffer<'a>),
}

pub struct Buffer<'a> {
    pub contents: BufferContent<'a>,
    pub ty: BufferType,
}

pub struct NormalRequest<'a, T: WriteAsBytes> {
    pub ty: CommandType,
    pub command_id: u32,
    pub send_pid: Option<u64>,
    pub copy_handles: &'a [RawHandle],
    pub move_handles: &'a [RawHandle],
    pub buffers: &'a [Buffer<'a>],
    pub input_parameters: &'a T,
}

impl<'a, T: WriteAsBytes> WriteAsBytes for NormalRequest<'a, T> {
    fn write_as_bytes(&self, dest: &mut (impl Writer + ?Sized)) {
        dest.write(&CmifInHeader {
            magic: CmifInHeader::MAGIC,
            version: 0,
            command_id: self.command_id,
            token: 0,
        });

        // TODO: !!! Somewhere (?) we should save the lengths of some (?) buffers as an array of u16s

        dest.write(self.input_parameters);
    }
}

impl<'a, T: WriteAsBytes> HipcPayloadIn<'a> for NormalRequest<'a, T> {
    fn get_type(&self) -> u16 {
        self.ty as u16
    }

    fn get_pointer_in_buffers(&self) -> ArrayVec<&'a ConstBuffer<'a>, 8> {
        ArrayVec::from_iter(self.buffers.iter().filter_map(|b| match b {
            Buffer {
                contents: BufferContent::In(buf),
                ty:
                    BufferType {
                        mode: BufferMode::Pointer | BufferMode::AutoSelect,
                        ..
                    },
            } => {
                if b.ty.mode == BufferMode::Pointer {
                    Some(buf)
                } else {
                    // handle AutoSelect buffer here by putting a null descriptor
                    Some(ConstBuffer::null_ref())
                }
            }
            _ => None,
        }))
    }

    fn get_pointer_out_buffers(&self) -> ArrayVec<&'a MutBuffer<'a>, 8> {
        ArrayVec::from_iter(self.buffers.iter().filter_map(|b| match b {
            Buffer {
                contents: BufferContent::Out(buf),
                ty:
                    BufferType {
                        mode: BufferMode::Pointer | BufferMode::AutoSelect,
                        ..
                    },
            } => {
                if b.ty.mode == BufferMode::Pointer {
                    Some(buf)
                } else {
                    // handle AutoSelect buffer here by putting a null descriptor
                    Some(MutBuffer::null_ref())
                }
            }
            _ => None,
        }))
    }

    fn get_map_alias_in_buffers(&self) -> ArrayVec<(MapAliasBufferMode, &'a ConstBuffer<'a>), 8> {
        ArrayVec::from_iter(self.buffers.iter().filter_map(|b| match b {
            Buffer {
                contents: BufferContent::In(buf),
                ty:
                    BufferType {
                        mode: BufferMode::MapAlias(_) | BufferMode::AutoSelect,
                        ..
                    },
            } => {
                if let BufferMode::MapAlias(mode) = b.ty.mode {
                    Some((mode, buf))
                } else {
                    Some((MapAliasBufferMode::Normal, buf))
                }
            }
            _ => None,
        }))
    }

    fn get_map_alias_out_buffers(&self) -> ArrayVec<(MapAliasBufferMode, &'a MutBuffer<'a>), 8> {
        ArrayVec::from_iter(self.buffers.iter().filter_map(|b| match b {
            Buffer {
                contents: BufferContent::Out(buf),
                ty:
                    BufferType {
                        mode: BufferMode::MapAlias(_) | BufferMode::AutoSelect,
                        ..
                    },
            } => {
                if let BufferMode::MapAlias(mode) = b.ty.mode {
                    Some((mode, buf))
                } else {
                    Some((MapAliasBufferMode::Normal, buf))
                }
            }
            _ => None,
        }))
    }

    fn get_map_alias_in_out_buffers(&self) -> ArrayVec<(MapAliasBufferMode, &'a MutBuffer<'a>), 8> {
        // no in_out buffers in CMIF =)
        ArrayVec::new()
    }

    fn get_send_pid(&self) -> Option<u64> {
        self.send_pid
    }

    fn get_copy_handles(&self) -> &[RawHandle] {
        self.copy_handles
    }

    fn get_move_handles(&self) -> &[RawHandle] {
        self.move_handles
    }

    fn write_as_bytes(&self, dest: &mut (impl Writer + ?Sized)) {
        // we need to align stuff to 16 bytes because CMIF does it
        let aligned = dest.align(16);

        // payload
        dest.write(self);

        // for some reason we have to insert padding at two places: before and after the payload
        // they, in sum, should be 16 bytes
        // (WTF man)
        let zeroes = [0u8; 16];
        dest.write_bytes(&zeroes[aligned..]);

        // TODO: if (when) we had supported them, sizes of buffers that are HipcAutoSelect would go here
        // we don't implement them currently, but they are used somewhat heavily across the codebase
        // so we probably should implement them
    }
}

pub struct NormalResponse<'d, T: ReadFromBytes<'d>> {
    result: ErrorCode,
    payload: T,
    phantom: PhantomData<&'d ()>,
}

impl<'d, T: ReadFromBytes<'d>> NormalResponse<'d, T> {
    pub fn as_result(&self) -> Result<&T, ErrorCode> {
        self.result.into_result(&self.payload)
    }
}

impl<'d, T: ReadFromBytes<'d>> ReadFromBytes<'d> for NormalResponse<'d, T> {
    fn read_from_bytes(src: &mut (impl Reader<'d> + ?Sized)) -> Self {
        // we need to align stuff to 16 bytes because CMIF does it
        let aligned = src.align(16);

        let header = src.read::<CmifOutHeader>();

        debug_assert_eq!(header.magic, CmifOutHeader::MAGIC);
        debug_assert_eq!(header.version, 0);
        let result = header.result;

        let payload = src.read::<T>();

        // for some reason we have to insert padding at two places: before and after the payload
        // they, in sum, should be 16 bytes
        // (WTF man)
        let _ = src.read_bytes(16 - aligned);

        Self {
            result,
            payload,
            phantom: Default::default(),
        }
    }
}
