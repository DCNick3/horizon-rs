//! Implements a storage for mount point list

use core::mem::{ManuallyDrop, MaybeUninit};

use horizon_ipc::handle_storage::{HandleStorage, OwnedHandle, RefHandle};
use horizon_ipc::RawHandle;
use horizon_sync::mutex::Mutex;

/// Represents a device that can be mounted
#[derive(Copy, Clone, Debug)]
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
type Name = [u8; 32];

struct Mount {
    name: Name,
    pub device: ManuallyDrop<MountDevice<OwnedHandle>>,
}

impl Mount {
    #[inline]
    const fn empty() -> Self {
        Mount {
            name: [0; NAME_MAX_LEN],
            device: ManuallyDrop::new(MountDevice::IFileSystem(OwnedHandle::new(RawHandle(0)))),
        }
    }

    #[inline]
    fn new(name: &str, contents: MountDevice<OwnedHandle>) -> Option<Self> {
        if name.len() > NAME_MAX_LEN {
            None
        } else {
            let mut name_buf: Name = [0; NAME_MAX_LEN];
            name_buf[..name.len()].copy_from_slice(name.as_bytes());

            Some(Mount {
                name: name_buf,
                device: ManuallyDrop::new(contents),
            })
        }
    }

    #[inline]
    fn name_len(&self) -> usize {
        self.name
            .iter()
            .enumerate()
            .find(|&(_, c)| *c == 0)
            .map(|(p, _)| p)
            .unwrap_or(NAME_MAX_LEN)
    }

    #[inline]
    fn name(&self) -> &str {
        // SAFETY: we don't create mount with non-utf8 names
        unsafe { core::str::from_utf8_unchecked(&self.name[..self.name_len()]) }
    }

    #[inline]
    fn name_matches(&self, name: &str) -> bool {
        self.name_len() == name.len() && self.name() == name
    }

    #[inline]
    fn is_empty(&self) -> bool {
        self.name[0] == 0
    }
}

impl Default for Mount {
    fn default() -> Self {
        Mount::empty()
    }
}

impl Drop for Mount {
    fn drop(&mut self) {
        if !self.is_empty() {
            unsafe { ManuallyDrop::drop(&mut self.device) }
        }
    }
}

type Mounts = [Mount; MOUNTPOINT_COUNT];

/// Maximum number of mount points that you can have
pub const MOUNTPOINT_COUNT: usize = 32;
static mut MOUNT_POINTS: MaybeUninit<Mutex<Mounts>> = MaybeUninit::uninit();

/// Initialize the mount point storage
///
/// # Safety
///
/// Must be called only once
/// Must be called before any calls to [read] and [write()]
/// It's usually called by horizon-rt in early process initialization, so usually you don't call this
pub unsafe fn init() {
    let mounts: [Mount; 32] = Default::default();

    MOUNT_POINTS.write(Mutex::new(mounts));
}

fn find_empty_mount(mounts: &Mounts) -> Option<usize> {
    mounts
        .iter()
        .enumerate()
        .find(|&(_, m)| m.is_empty())
        .map(|(p, _)| p)
}

fn find_impl<'a>(mounts: &'a Mounts, name: &str) -> Option<(usize, &'a Mount)> {
    mounts
        .iter()
        .enumerate()
        .find(|&(_, m)| m.name_matches(name))
}

/// Represents an error that occurred while adding a mountpoint
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

fn add_impl(
    mounts: &mut Mounts,
    name: &str,
    contents: MountDevice<OwnedHandle>,
) -> Result<(), AddError> {
    // we can't have an empty mountpoint name because it's used as a marker for "no mountpoint here"
    if name.is_empty() {
        return Err(AddError::EmptyName);
    }

    let mount: Mount = if let Some(m) = Mount::new(name, contents) {
        m
    } else {
        return Err(AddError::NameTooLong);
    };

    if find_impl(&mounts, name).is_some() {
        return Err(AddError::DuplicateMount);
    }

    if let Some(i) = find_empty_mount(&mounts) {
        mounts[i] = mount
    } else {
        return Err(AddError::TooManyMounts);
    }

    Ok(())
}
fn remove_impl(mounts: &mut Mounts, name: &str) -> Option<()> {
    if let Some((p, _)) = find_impl(&mounts, name) {
        mounts[p] = Mount::empty();

        Some(())
    } else {
        None
    }
}

fn find_contents_impl<'a>(mounts: &'a Mounts, name: &str) -> Option<MountDevice<RefHandle<'a>>> {
    if let Some((p, _)) = find_impl(&mounts, name) {
        let mount: &Mount = &mounts[p];
        let contents: &MountDevice<OwnedHandle> = &mount.device;

        Some(contents.as_ref())
    } else {
        None
    }
}

/// Immutable mountpoint list iterator
pub struct Iter<'a> {
    mounts: core::slice::Iter<'a, Mount>,
}

impl<'a> Iterator for Iter<'a> {
    type Item = (&'a str, MountDevice<RefHandle<'a>>);

    fn next(&mut self) -> Option<Self::Item> {
        if let Some(m) = self.mounts.next() {
            Some((m.name(), m.device.as_ref()))
        } else {
            None
        }
    }
}

// TODO: use an actual RwLock here
/// RAII structure used to guard the read access to mountpoint list
pub struct ReadGuard {
    guard: horizon_sync::mutex::MutexGuard<'static, Mounts>,
}

impl ReadGuard {
    /// Find a mount point with a specified name
    ///
    /// # Errors
    /// * Returns None if mountpoint with a specified name does not exist
    pub fn find(&self, name: &str) -> Option<MountDevice<RefHandle<'_>>> {
        find_contents_impl(&*self.guard, name)
    }

    /// Iterate over the mount points
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            mounts: self.guard.iter(),
        }
    }
}

// TODO: use an actual RwLock here
/// RAII structure used to guard the write access to mountpoint list
pub struct WriteGuard {
    guard: horizon_sync::mutex::MutexGuard<'static, Mounts>,
}

impl WriteGuard {
    /// Find a mount point with a specified name
    ///
    /// # Errors
    /// * Returns None if mountpoint with a specified name does not exist
    pub fn find(&self, name: &str) -> Option<MountDevice<RefHandle<'_>>> {
        find_contents_impl(&*self.guard, name)
    }

    /// Iterate over the mount points
    pub fn iter(&self) -> Iter<'_> {
        Iter {
            mounts: self.guard.iter(),
        }
    }

    /// Find a mount point with a specified name and contents
    ///
    /// # Errors
    /// See [AddError]
    pub fn add(&mut self, name: &str, contents: MountDevice<OwnedHandle>) -> Result<(), AddError> {
        add_impl(&mut *self.guard, name, contents)
    }

    /// Remove a mount point with a specified name
    ///
    /// # Errors
    /// * Returns None if mountpoint with a specified name does not exist
    pub fn remove(&mut self, name: &str) -> Option<()> {
        remove_impl(&mut *self.guard, name)
    }
}

/// Lock mountpoints allowing read-only access
///
/// (This should allow multiple threads to access this in read-only fashion)
pub fn read() -> ReadGuard {
    ReadGuard {
        guard: unsafe { MOUNT_POINTS.assume_init_ref() }.lock(),
    }
}

/// Lock mountpoints allowing read-write access
///
/// (This ensures that only one thread accesses the mount point list)
pub fn write() -> WriteGuard {
    WriteGuard {
        guard: unsafe { MOUNT_POINTS.assume_init_ref() }.lock(),
    }
}
