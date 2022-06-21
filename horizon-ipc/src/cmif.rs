use crate::conv_traits::{WriteAsBytes, Writer};
use crate::hipc::HipcPayload;
use crate::raw::cmif::{CmifDomainInHeader, CmifInHeader};
use alloc::sync::Arc;
use core::borrow::Borrow;
use core::marker::PhantomData;
use core::ops::Deref;
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

/// A non-type-safe handle to some IPC object
pub struct ObjectHandle(RawHandle);

impl ObjectHandle {
    pub fn raw(&self) -> RawHandle {
        self.0
    }
}

impl Drop for ObjectHandle {
    fn drop(&mut self) {
        horizon_svc::close_handle(self.0).unwrap()
    }
}

/// A handle to an IPC object that must be a domain object
pub struct DomainHandle(ObjectHandle);

impl Deref for DomainHandle {
    type Target = ObjectHandle;

    fn deref(&self) -> &Self::Target {
        &self.0
    }
}

pub struct DomainObject<'a, T: Borrow<DomainHandle> + 'a> {
    domain: T,
    object_id: u32,
    phantom: PhantomData<&'a ()>,
}

impl<'a, T: Borrow<DomainHandle>> DomainObject<'a, T> {
    pub fn get_domain(&self) -> &DomainHandle {
        self.domain.borrow()
    }
}

impl<'a, T: Borrow<DomainHandle>> Drop for DomainObject<'a, T> {
    fn drop(&mut self) {
        todo!("Implement when there will be a CMIF IPC definitions for sending the close requests")
    }
}

/// A way to refer to an IPC object
pub enum ObjectReference<'a> {
    /// Direct reference to an object (a session handle)
    DirectObject(&'a ObjectHandle),
    /// Reference to an object inside a domain (a domain handle and an object id)
    DomainObject(&'a DomainHandle, u32),
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
    normal_request: Request<'a, T>,
}

impl<'a, T: WriteAsBytes> WriteAsBytes for DomainRequest<'a, T> {
    fn write_as_bytes(&self, dest: &mut (impl Writer + ?Sized)) {
        let normal_request_size = self.normal_request.size();

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

impl<'a, T: WriteAsBytes> HipcPayload for DomainRequest<'a, T> {
    fn get_type(&self) -> u16 {
        self.normal_request.ty as u16
    }
}

pub struct Request<'a, T: WriteAsBytes> {
    pub ty: CommandType,
    pub command_id: u32,
    pub input_parameters: &'a T,
}

impl<'a, T: WriteAsBytes> WriteAsBytes for Request<'a, T> {
    fn write_as_bytes(&self, dest: &mut (impl Writer + ?Sized)) {
        dest.write(&CmifInHeader {
            magic: CmifInHeader::MAGIC,
            version: 0,
            command_id: self.command_id,
            token: 0,
        });

        dest.write(self.input_parameters);
    }
}

impl<'a, T: WriteAsBytes> HipcPayload for Request<'a, T> {
    fn get_type(&self) -> u16 {
        self.ty as u16
    }
}
