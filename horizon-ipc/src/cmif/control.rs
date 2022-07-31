use crate::buffer::get_ipc_buffer_ptr;
use crate::cmif::CommandType;
use crate::raw::cmif::{CmifInHeader, CmifOutHeader};
use crate::raw::hipc::HipcHeader;
use horizon_error::{ErrorCode, Result};
use horizon_svc::RawHandle;

pub fn clone_object(_handle: RawHandle) -> RawHandle {
    todo!()
}

#[allow(dead_code)]
fn send_close_request(handle: RawHandle) -> Result<()> {
    #[repr(C, packed)]
    struct Request {
        hipc: HipcHeader,
        pre_padding: [u8; 8],
        cmif: CmifInHeader,
        post_padding: [u8; 8],
    }
    // Compiler time request size check
    let _ = ::core::mem::transmute::<Request, [u8; 40]>;
    #[repr(C, packed)]
    struct Response {
        hipc: HipcHeader,
        pre_padding: [u8; 8],
        cmif: CmifOutHeader,
        post_padding: [u8; 8],
    }
    // Compiler time request size check
    let _ = ::core::mem::transmute::<Response, [u8; 40]>;
    let ipc_buffer_ptr = unsafe { get_ipc_buffer_ptr() };
    unsafe {
        ::core::ptr::write(
            ipc_buffer_ptr as *mut _,
            Request {
                hipc: HipcHeader::new(CommandType::Close, 0, 0, 0, 0, 10, 0, 0, false),
                pre_padding: Default::default(),
                cmif: CmifInHeader {
                    magic: CmifInHeader::MAGIC,
                    version: 1,
                    command_id: 3,
                    token: 0,
                },
                post_padding: Default::default(),
            },
        )
    };
    // we ignore the result because the server returns a "port dead" error
    let _ = horizon_svc::send_sync_request(handle);
    let Response { hipc, cmif, .. } = unsafe { ::core::ptr::read(ipc_buffer_ptr as *const _) };
    debug_assert_eq!(hipc.num_in_pointers(), 0);
    debug_assert_eq!(hipc.num_in_map_aliases(), 0);
    debug_assert_eq!(hipc.num_out_map_aliases(), 0);
    debug_assert_eq!(hipc.num_inout_map_aliases(), 0);
    debug_assert_eq!(hipc.out_pointer_mode(), 0);
    debug_assert_eq!(hipc.has_special_header(), 0);
    debug_assert_eq!(cmif.magic, CmifOutHeader::MAGIC);
    debug_assert_eq!(cmif.result, ErrorCode::new(0));
    Ok(())
}

#[allow(unreachable_code)]
pub fn close_object(handle: RawHandle) {
    send_close_request(handle).unwrap();

    horizon_svc::close_handle(handle).unwrap();
}
