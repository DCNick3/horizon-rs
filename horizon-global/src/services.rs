//! Stores sessions for various services that should be global per process and are used by libstd

// TODO: implement domain handles

macro_rules! normal_service {
    ($name:ident) => {
        pub mod $name {
            #![doc = concat!("This is a session storage for service ", stringify!($name))]

            use crate::core::fmt::{Display, Formatter};
            use horizon_ipc::handle_storage::{HandleRef, HandleStorage, OwnedHandle, RefHandle};
            use horizon_sync::rw_lock::{RwLock, RwLockReadGuard};

            static SESSION: RwLock<Option<OwnedHandle>> = RwLock::new(None);

            /// Guard object that keeps the handle valid throughout its lifetime
            ///
            /// Note: it's unsafe to construct this with contents being `None`
            #[derive(Debug)]
            pub struct Guard {
                guard: RwLockReadGuard<'static, Option<OwnedHandle>>,
            }

            impl Guard {
                fn inner(&self) -> RefHandle<'_> {
                    // SAFETY: we do not create guards in case there is no handle in the lock
                    unsafe { self.guard.as_ref().unwrap_unchecked() }.as_ref()
                }
            }

            impl HandleStorage for Guard {
                #[inline]
                fn get(&self) -> HandleRef<'_, Self> {
                    let handle = self.inner().inner();
                    HandleRef {
                        handle,
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
                let mut guard = SESSION.write();
                guard.replace(handle)
            }

            /// Remove the stored session (if any), return it
            pub fn take() -> Option<OwnedHandle> {
                let mut guard = SESSION.write();
                guard.take()
            }

            /// Get currently stored session (if any)
            pub fn get() -> Option<Guard> {
                let guard = SESSION.read();
                if guard.is_some() {
                    Some(Guard { guard })
                } else {
                    None
                }
            }

            /// Get currently stored session or create a new one with a function (atomically)
            pub fn get_or_connect<F: FnOnce() -> OwnedHandle>(function: F) -> Guard {
                loop {
                    {
                        let guard = SESSION.read();
                        if guard.is_some() {
                            return Guard { guard };
                        }
                    }

                    {
                        let mut guard = SESSION.write();
                        // somebody might have put the service while we were swapping READ lock for WRITE
                        if guard.is_none() {
                            // so open a new handle only if needed
                            guard.replace(function());
                        }

                        // and downgrade our WRITE guard to a READ one
                        return Guard {
                            guard: guard.downgrade(),
                        };
                    }
                }
            }
        }
    };
}

normal_service!(sm);
normal_service!(fs);
