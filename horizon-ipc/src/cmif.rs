use core::marker::PhantomData;
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
pub struct SessionHandle(pub RawHandle);

impl SessionHandle {
    pub fn as_ref(&self) -> SessionHandleRef<'_> {
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

// /// A handle to an IPC object that must be a domain object
// pub struct DomainHandle(SessionHandle);
//
// impl Deref for DomainHandle {
//     type Target = SessionHandle;
//
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }
//
// #[derive(Copy, Clone)]
// pub struct DomainHandleRef<'a>(SessionHandleRef<'a>);
//
// impl<'a> Deref for DomainHandleRef<'a> {
//     type Target = SessionHandleRef<'a>;
//
//     fn deref(&self) -> &Self::Target {
//         &self.0
//     }
// }
//
// pub struct DomainObject<'a> {
//     domain: DomainHandleRef<'a>,
//     object_id: u32,
// }
//
// impl<'a> DomainObject<'a> {
//     pub fn get_domain(&self) -> DomainHandleRef<'a> {
//         self.domain
//     }
// }
//
// impl<'a> Drop for DomainObject<'a> {
//     fn drop(&mut self) {
//         todo!("Implement when there will be a CMIF IPC definitions for sending the close requests")
//     }
// }
//
// pub struct DomainObjectRef<'a> {
//     domain: DomainHandleRef<'a>,
//     object_id: u32,
//     phantom: PhantomData<&'a ()>,
// }
//
// // TODO: will we actually have functions that are agnostic to the kind of object we are using?
// /// A way to refer to an IPC object
// pub enum ObjectReference<'a> {
//     /// Direct reference to an object (a session handle)
//     SessionObject(SessionHandleRef<'a>),
//     /// Reference to an object inside a domain (a domain session handle and an object id)
//     DomainObject(DomainObjectRef<'a>),
// }
