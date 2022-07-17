pub mod control;

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
