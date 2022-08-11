//! Implements a storage for mount point list

ij_core_workaround!();

#[cfg(feature = "impl")]
mod r#impl;

use crate::core::marker::PhantomData;
use horizon_ipc::handle_storage::{HandleStorage, OwnedHandle, RefHandle};

/// Represents a device that can be mounted
#[derive(Copy, Clone, Debug)]
#[repr(C)]
pub enum MountDevice<S: HandleStorage> {
    /// An IFileSystem object
    ///
    /// All the filesystem access code would be implemented in `fs` sysmodule
    IFileSystem(S),
    /// An IStorage object that contains a romfs
    ///
    /// Retail games would usually use this
    RomfsIStorage(S),
    /// An IFile object that contains a romfs at specified offset
    ///
    /// Homebrew would usually use this (having a romfs at some offset in the NRO file)
    RomfsIFile(S, u64),
}

impl MountDevice<OwnedHandle> {
    /// Convert a reference to an owned [`MountDevice`] to a reference [`MountDevice`]
    #[inline]
    pub fn as_ref(&self) -> MountDevice<RefHandle<'_>> {
        match self {
            MountDevice::IFileSystem(h) => MountDevice::IFileSystem(h.as_ref()),
            MountDevice::RomfsIStorage(h) => MountDevice::RomfsIStorage(h.as_ref()),
            MountDevice::RomfsIFile(h, offset) => MountDevice::RomfsIFile(h.as_ref(), *offset),
        }
    }
}

/// Maximum length of a mountpoint name in bytes
pub const NAME_MAX_LEN: usize = 32;

/// Represents an error that occurred while adding a mountpoint
#[repr(C)]
pub enum AddError {
    /// Mount name is an empty string
    EmptyName,
    /// Name is longer than [NAME_MAX_LEN]
    NameTooLong,
    /// There are more than [MOUNTPOINT_COUNT] mounts added
    TooManyMounts,
    /// A mount with the same name exists
    DuplicateMount,
}

extern "Rust" {
    fn __horizon_global_mounts_lock_read();
    fn __horizon_global_mounts_unlock_read();
    fn __horizon_global_mounts_lock_write();
    fn __horizon_global_mounts_unlock_write();

    // those functions need read lock
    fn __horizon_global_mounts_get(
        index: usize,
    ) -> Option<(&'static str, MountDevice<RefHandle<'static>>)>;
    fn __horizon_global_mounts_find(name: &str) -> Option<MountDevice<RefHandle<'static>>>;

    // those functions need write lock
    fn __horizon_global_mounts_add(
        name: &str,
        contents: MountDevice<OwnedHandle>,
    ) -> Result<(), AddError>;
    fn __horizon_global_mounts_remove(name: &str) -> Option<MountDevice<OwnedHandle>>;
}

/// Immutable mountpoint list iterator
pub struct Iter<'a> {
    index: usize,
    phantom: PhantomData<&'a ()>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a str, MountDevice<RefHandle<'a>>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(m) = unsafe { __horizon_global_mounts_get(self.index) } {
            self.index += 1;
            Some(m)
        } else {
            None
        }
    }
}

/// RAII structure used to guard the read access to mountpoint list
#[non_exhaustive]
pub struct ReadGuard {}

impl ReadGuard {
    /// Find a mount point with a specified name
    ///
    /// # Errors
    /// * Returns None if mountpoint with a specified name does not exist
    pub fn find<'a>(&'a self, name: &str) -> Option<MountDevice<RefHandle<'a>>> {
        unsafe { __horizon_global_mounts_find(name) }
    }

    // /// Iterate over the mount points
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            index: 0,
            phantom: Default::default(),
        }
    }
}

impl Drop for ReadGuard {
    fn drop(&mut self) {
        unsafe { __horizon_global_mounts_unlock_read() }
    }
}

/// RAII structure used to guard the write access to mountpoint list
#[non_exhaustive]
pub struct WriteGuard {}

impl WriteGuard {
    /// Find a mount point with a specified name
    ///
    /// # Errors
    /// * Returns None if mountpoint with a specified name does not exist
    pub fn find(&self, name: &str) -> Option<MountDevice<RefHandle<'_>>> {
        unsafe { __horizon_global_mounts_find(name) }
    }

    /// Iterate over the mount points
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            index: 0,
            phantom: Default::default(),
        }
    }

    /// Find a mount point with a specified name and contents
    ///
    /// # Errors
    /// See [AddError]
    pub fn add(&mut self, name: &str, contents: MountDevice<OwnedHandle>) -> Result<(), AddError> {
        unsafe { __horizon_global_mounts_add(name, contents) }
    }

    /// Remove a mount point with a specified name
    ///
    /// # Errors
    /// * Returns None if mountpoint with a specified name does not exist
    pub fn remove(&mut self, name: &str) -> Option<MountDevice<OwnedHandle>> {
        unsafe { __horizon_global_mounts_remove(name) }
    }
}

impl Drop for WriteGuard {
    fn drop(&mut self) {
        unsafe { __horizon_global_mounts_unlock_write() }
    }
}

/// Lock mountpoints allowing read-only access
///
/// (This should allow multiple threads to access this in read-only fashion)
pub fn read() -> ReadGuard {
    unsafe { __horizon_global_mounts_lock_read() }
    ReadGuard {}
}

/// Lock mountpoints allowing read-write access
///
/// (This ensures that only one thread accesses the mount point list)
pub fn write() -> WriteGuard {
    unsafe { __horizon_global_mounts_lock_write() }
    WriteGuard {}
}
