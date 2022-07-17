use horizon_svc::RawHandle;

pub fn clone_object(_handle: RawHandle) -> RawHandle {
    todo!()
}

#[allow(unreachable_code)]
pub fn close_object(_handle: RawHandle) {
    todo!("Send a close handle request");

    horizon_svc::close_handle(_handle).unwrap();
}
