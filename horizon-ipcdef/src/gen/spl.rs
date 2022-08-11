#![allow(unused_qualifications)]
ij_core_workaround!();
use horizon_error::Result;
use horizon_ipc::buffer::get_ipc_buffer_ptr;
use horizon_ipc::cmif::CommandType;
use horizon_ipc::handle_storage::{HandleStorage, OwnedHandle, RefHandle, SharedHandle};
use horizon_ipc::hipc::MapAliasBufferMode;
use horizon_ipc::raw::cmif::{CmifInHeader, CmifOutHeader};
use horizon_ipc::raw::hipc::{HipcHeader, HipcMapAliasBufferDescriptor};
pub struct IRandomInterface<S: HandleStorage = OwnedHandle> {
    pub(crate) handle: S,
}
impl<S: HandleStorage> IRandomInterface<S> {
    pub fn new(handle: S) -> Self {
        Self { handle }
    }
    pub fn into_inner(self) -> S {
        self.handle
    }
    pub fn generate_random_bytes(&self, buffer: &mut [u8]) -> Result<()> {
        let data_in = ();
        #[repr(packed)]
        struct Request {
            hipc: HipcHeader,
            out_map_alias_desc_0: HipcMapAliasBufferDescriptor,
            pre_padding: [u8; 12],
            cmif: CmifInHeader,
            raw_data: (),
            raw_data_word_padding: [u8; 0],
            post_padding: [u8; 4],
        }
        // Compiler time request size check
        let _ = ::core::mem::transmute::<Request, [u8; 52]>;
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
                        1,
                        0,
                        8,
                        0,
                        0,
                        false,
                    ),
                    out_map_alias_desc_0: HipcMapAliasBufferDescriptor::new(
                        MapAliasBufferMode::Normal,
                        buffer.as_ptr() as usize,
                        ::core::mem::size_of_val(buffer),
                    ),
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
            crate::pre_ipc_hook("spl::IRandomInterface::GenerateRandomBytes", *handle);
            horizon_svc::send_sync_request(*handle)?;
            crate::post_ipc_hook("spl::IRandomInterface::GenerateRandomBytes", *handle);
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
impl IRandomInterface<OwnedHandle> {
    pub fn as_ref(&self) -> IRandomInterface<RefHandle<'_>> {
        IRandomInterface {
            handle: self.handle.as_ref(),
        }
    }
    pub fn into_shared(self) -> IRandomInterface<SharedHandle> {
        IRandomInterface {
            handle: SharedHandle::new(self.handle.leak()),
        }
    }
}
impl ::core::fmt::Debug for IRandomInterface {
    fn fmt(&self, f: &mut ::core::fmt::Formatter<'_>) -> ::core::fmt::Result {
        write!(f, "IRandomInterface({})", self.handle)
    }
}

