//! Stores sessions for various services that should be global per process and are used by libstd

// TODO: implement domain handles

macro_rules! normal_service {
    ($name:ident) => {
        pub mod $name {
            #![doc = concat!("This is a session storage for service ", stringify!($name))]

            ij_core_workaround!();

            use crate::core::fmt::{Display, Formatter};
            use horizon_ipc::RawHandle;
            use horizon_ipc::handle_storage::{HandleRef, HandleStorage, OwnedHandle, RefHandle};

            #[cfg(feature = "impl")]
            mod r#impl {
                ij_core_workaround!();

                use horizon_sync::raw_rw_lock::RawRwLock;
                use horizon_ipc::RawHandle;
                use horizon_ipc::handle_storage::{OwnedHandle};

                static SESSION_LOCK: RawRwLock = RawRwLock::new();
                static mut SESSION: Option<OwnedHandle> = None;

                #[export_name = concat!("__horizon_global_services_", stringify!($name), "_unlock_read")]
                unsafe fn __unlock_read()
                {
                    SESSION_LOCK.read_unlock();
                }

                // this function takes read lock and does not release it if the return value is not None
                #[export_name = concat!("__horizon_global_services_", stringify!($name), "_get")]
                unsafe fn __get() -> Option<RawHandle>
                {
                    SESSION_LOCK.read();
                    let res = SESSION.as_ref().map(|v| v.as_ref().inner());
                    SESSION_LOCK.read_unlock();
                    return res;
                }

                // this function takes a write lock and then downgrades it to a read lock
                #[export_name = concat!("__horizon_global_services_", stringify!($name), "_get_or_connect")]
                unsafe fn __get_or_connect(
                    connect_fn: fn(*mut ()) -> horizon_error::Result<OwnedHandle>,
                    arg: *mut (),
                ) -> horizon_error::Result<RawHandle>
                {
                    // this is really really scary unsafe...
                    // maybe if we allow to access the Raw lock of the wrapper type it would be less painful...
                    SESSION_LOCK.read();
                    if let Some(handle) = &SESSION {
                        return Ok(handle.as_ref().inner());
                    }
                    SESSION_LOCK.read_unlock();

                    SESSION_LOCK.write();
                    if let None = &SESSION {
                        match connect_fn(arg) {
                            Ok(handle) => {
                                SESSION.replace(handle);
                            },
                            Err(e) => {
                                 SESSION_LOCK.write_unlock();
                                 return Err(e);
                            }
                        }
                    }
                    SESSION_LOCK.write_downgrade();
                    // and downgrade our WRITE guard to a READ one
                    let handle = SESSION.as_ref().unwrap_unchecked().as_ref().inner();
                    return Ok(handle);
                }

                // no locking required (they take write lock internally and release it)
                #[export_name = concat!("__horizon_global_services_", stringify!($name), "_replace")]
                unsafe fn __replace(handle: OwnedHandle) -> Option<OwnedHandle>
                {
                    SESSION_LOCK.write();
                    let res = SESSION.take();
                    SESSION = Some(handle);
                    SESSION_LOCK.write_unlock();
                    return res;
                }

                #[export_name = concat!("__horizon_global_services_", stringify!($name), "_take")]
                unsafe fn __take() -> Option<OwnedHandle>
                {
                    SESSION_LOCK.write();
                    let res = SESSION.take();
                    SESSION_LOCK.write_unlock();
                    return res;
                }
            }

            /// Guard object that keeps the handle valid throughout its lifetime
            ///
            /// Note: it's unsafe to construct this with contents being `None`
            #[derive(Debug)]
            pub struct Guard {
                handle: RawHandle,
            }

            impl Guard {
                #[inline]
                pub fn inner(&self) -> RefHandle<'_> {
                    RefHandle::new(self.handle)
                }
            }

            impl Drop for Guard {
                fn drop(&mut self) {
                    unsafe { __unlock_read() }
                }
            }

            extern "Rust" {
                #[link_name = concat!("__horizon_global_services_", stringify!($name), "_unlock_read")]
                fn __unlock_read();

                // this function takes read lock and does not release it if the return value is not None
                #[link_name = concat!("__horizon_global_services_", stringify!($name), "_get")]
                fn __get() -> Option<RawHandle>;
                // this function takes a write lock and then downgrades it to a read lock
                #[link_name = concat!("__horizon_global_services_", stringify!($name), "_get_or_connect")]
                fn __get_or_connect(
                    connect_fn: fn(*mut ()) -> horizon_error::Result<OwnedHandle>,
                    arg: *mut (),
                ) -> horizon_error::Result<RawHandle>;

                // no locking required (they take write lock internally and release it)
                #[link_name = concat!("__horizon_global_services_", stringify!($name), "_replace")]
                fn __replace(handle: OwnedHandle) -> Option<OwnedHandle>;
                #[link_name = concat!("__horizon_global_services_", stringify!($name), "_take")]
                fn __take() -> Option<OwnedHandle>;
            }

            impl HandleStorage for Guard {
                #[inline]
                fn get(&self) -> HandleRef<'_, Self> {
                    HandleRef {
                        handle: self.handle,
                        index: 0,
                        storage: self,
                    }
                }

                #[inline]
                fn give_back(&self, _handle: &HandleRef<'_, Self>) {}
            }

            impl Display for Guard {
                fn fmt(&self, f: &mut Formatter<'_>) -> core::fmt::Result {
                    write!(f, "{}", self.inner())
                }
            }

            /// Replace the stored session, return the old one (if any)
            pub fn replace(handle: OwnedHandle) -> Option<OwnedHandle> {
                unsafe { __replace(handle) }
            }

            /// Remove the stored session (if any), return it
            pub fn take() -> Option<OwnedHandle> {
                unsafe { __take() }
            }

            /// Get currently stored session (if any)
            pub fn get() -> Option<Guard> {
                let res = unsafe { __get() };
                res.map(|handle| Guard { handle })
            }

            /// Get currently stored session or create a new one with a function (atomically)
            pub fn get_or_connect<F: FnOnce() -> horizon_error::Result<OwnedHandle>>(
                function: F,
            ) -> horizon_error::Result<Guard> {
                let mut function = Some(function);
                let userdata = &mut function as *mut _ as *mut ();

                fn shim<F: FnOnce() -> horizon_error::Result<OwnedHandle>>(userdata: *mut ())
                    -> horizon_error::Result<OwnedHandle>
                {
                    let function = userdata as *mut Option<F>;
                    let function = unsafe { (*function).take().unwrap_unchecked() };
                    function()
                }

                let handle = unsafe { __get_or_connect(shim::<F>, userdata) }?;

                Ok(Guard { handle })
            }
        }
    };
}

normal_service!(sm);
normal_service!(fs);
normal_service!(csrng);
