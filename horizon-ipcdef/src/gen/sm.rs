#![allow(unused_qualifications)]
ij_core_workaround!();
use horizon_error::{ErrorCode, Result};
use horizon_ipc::RawHandle;
use horizon_ipc::buffer::get_ipc_buffer_ptr;
use horizon_ipc::cmif::CommandType;
use horizon_ipc::handle_storage::{HandleStorage, OwnedHandle, RefHandle, SharedHandle};
use horizon_ipc::raw::cmif::{CmifInHeader, CmifOutHeader};
use horizon_ipc::raw::hipc::{HipcHeader, HipcSpecialHeader};
#[derive(Debug, Clone, Copy, Default)]
#[repr(C)]
pub struct ServiceName {
    pub name: [u8; 8],
}
// Static size check for ServiceName (expect 8 bytes)
const _: fn() = || {
    let _ = ::core::mem::transmute::<ServiceName, [u8; 8]>;
};

pub struct IUserInterface<S: HandleStorage = OwnedHandle> {
    pub(crate) handle: S,
}
impl<S: HandleStorage> IUserInterface<S> {
    pub fn new(handle: S) -> Self {
        Self { handle }
    }
    pub fn into_inner(self) -> S {
        self.handle
    }
    pub fn initialize(&self) -> Result<()> {
        let data_in = 0u64;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            pid_placeholder: u64,
            pre_padding: [u8; 12],
            cmif: CmifInHeader,
            raw_data: u64,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 4],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 60]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 40]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        0,
                        0,
                        10,
                        0,
                        0,
                        true,
                    ),
                    special_header: HipcSpecialHeader::new(true, 0, 0),
                    pid_placeholder: 0,
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 0,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("sm::IUserInterface::Initialize", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("sm::IUserInterface::Initialize", *handle);
        }
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(())
    }

    pub fn get_service(&self, name: ServiceName) -> Result<OwnedHandle> {
        let data_in = name;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: ServiceName,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 48]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            handle_session_handle: RawHandle,
            pre_padding: [u8; 0],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 48]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        0,
                        0,
                        10,
                        0,
                        0,
                        false,
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 1,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("sm::IUserInterface::GetService", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("sm::IUserInterface::GetService", *handle);
        }
        let Response {
            hipc,
            special_header,
            handle_session_handle: session_handle,
            cmif,
            raw_data: (),
            ..
        } = unsafe { ::core::ptr::read(ipc_buffer_ptr as *const _) };
        if hipc.has_special_header() != 0 {
            if cmif.result.is_failure() {
                return Err(cmif.result);
            }
        } else {
            return Err(unsafe {
                ::core::ptr::read(ipc_buffer_ptr.offset(24) as *const ErrorCode)
            })
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 1);
        debug_assert_eq!(special_header.send_pid(), 0);
        debug_assert_eq!(special_header.num_copy_handles(), 0);
        debug_assert_eq!(special_header.num_move_handles(), 1);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        let session_handle = OwnedHandle::new(session_handle);
        Ok(session_handle)
    }

    pub fn register_service(
        &self,
        name: ServiceName,
        max_sessions: u32,
        is_light: bool,
    ) -> Result<OwnedHandle> {
        #[repr(C, packed)]
        struct In {
            pub name: ServiceName,
            pub is_light: bool,
            pub _padding_0: [u8; 3],
            pub max_sessions: u32,
        }
        let _ = ::core::mem::transmute::<In, [u8; 16]>;
        let data_in: In = In {
            name,
            is_light,
            max_sessions,
            _padding_0: Default::default(),
        };
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: In,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 56]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            special_header: HipcSpecialHeader,
            handle_port_handle: RawHandle,
            pre_padding: [u8; 0],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 16],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 48]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        0,
                        0,
                        12,
                        0,
                        0,
                        false,
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 2,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("sm::IUserInterface::RegisterService", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("sm::IUserInterface::RegisterService", *handle);
        }
        let Response {
            hipc,
            special_header,
            handle_port_handle: port_handle,
            cmif,
            raw_data: (),
            ..
        } = unsafe { ::core::ptr::read(ipc_buffer_ptr as *const _) };
        if hipc.has_special_header() != 0 {
            if cmif.result.is_failure() {
                return Err(cmif.result);
            }
        } else {
            return Err(unsafe {
                ::core::ptr::read(ipc_buffer_ptr.offset(24) as *const ErrorCode)
            })
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 1);
        debug_assert_eq!(special_header.send_pid(), 0);
        debug_assert_eq!(special_header.num_copy_handles(), 0);
        debug_assert_eq!(special_header.num_move_handles(), 1);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        let port_handle = OwnedHandle::new(port_handle);
        Ok(port_handle)
    }

    pub fn unregister_service(&self, name: ServiceName) -> Result<()> {
        let data_in = name;
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifInHeader,
            raw_data: ServiceName,
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 48]>;
        #[repr(packed)]
        struct Response {
            hipc: HipcHeader,
            pre_padding: [u8; 8],
            cmif: CmifOutHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 8],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Response, [u8; 40]>;
        let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
        unsafe {
            ::core::ptr::write(
                ipc_buffer_ptr as *mut _,
                Request {
                    hipc: HipcHeader::new(
                        CommandType::Request,
                        0,
                        0,
                        0,
                        0,
                        10,
                        0,
                        0,
                        false,
                    ),
                    pre_padding: Default::default(),
                    cmif: CmifInHeader {
                        magic: CmifInHeader::MAGIC,
                        version: 1,
                        command_id: 3,
                        token: 0,
                    },
                    raw_data: data_in,
                    raw_data_word_padding: Default::default(),
                    post_padding: Default::default(),
                },
            )
        };
        {
            let handle = self.handle.get();
            crate::pre_ipc_hook("sm::IUserInterface::UnregisterService", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("sm::IUserInterface::UnregisterService", *handle);
        }
        let Response { hipc, cmif, raw_data: (), .. } = unsafe {
            ::core::ptr::read(ipc_buffer_ptr as *const _)
        };
        if cmif.result.is_failure() {
            return Err(cmif.result);
        }
        debug_assert_eq!(hipc.num_in_pointers(), 0);
        debug_assert_eq!(hipc.num_in_map_aliases(), 0);
        debug_assert_eq!(hipc.num_out_map_aliases(), 0);
        debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
        debug_assert_eq!(hipc.out_pointer_mode(), 0);
        debug_assert_eq!(hipc.has_special_header(), 0);
        debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
        Ok(())
    }
}
impl IUserInterface<OwnedHandle> {
    pub fn as_ref(&self) -> IUserInterface<RefHandle<'_>> {
        IUserInterface {
            handle: self.handle.as_ref(),
        }
    }
    pub fn into_shared(self) -> IUserInterface<SharedHandle> {
        IUserInterface {
            handle: SharedHandle::new(self.handle.leak()),
        }
    }
}
impl ::core::fmt::Debug for IUserInterface {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "IUserInterface({})", self.handle)
    }
}

