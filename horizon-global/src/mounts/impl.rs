ij_core_workaround!();

use crate::core::mem::ManuallyDrop;
use crate::mounts::{AddError, MountDevice, NAME_MAX_LEN};
use horizon_ipc::handle_storage::{OwnedHandle, RefHandle};
use horizon_ipc::RawHandle;
use horizon_sync::raw_rw_lock::RawRwLock;

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

    #[inline]
    fn into_inner(mut self) -> MountDevice<OwnedHandle> {
        // we can't just move out of `self` because it implements Drop
        // so we do it in a roundabout way
        // SAFETY: we do not use the `ManuallyDrop` again because we immediately forget the struct it is contained in
        let device = unsafe { ManuallyDrop::take(&mut self.device) };
        core::mem::forget(self);

        device
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
const MOUNTPOINT_COUNT: usize = 32;

static MOUNT_POINT_LOCK: RawRwLock = RawRwLock::new();
static mut MOUNT_POINTS: Mounts = [
    // This is horrible, but I don't think it's possible to do this in const context in other way
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
    Mount::empty(),
];

#[no_mangle]
pub fn __horizon_global_mounts_lock_read() {
    unsafe { MOUNT_POINT_LOCK.read() };
}

#[no_mangle]
pub fn __horizon_global_mounts_unlock_read() {
    unsafe { MOUNT_POINT_LOCK.read_unlock() };
}

#[no_mangle]
pub fn __horizon_global_mounts_lock_write() {
    unsafe { MOUNT_POINT_LOCK.write() };
}

#[no_mangle]
pub fn __horizon_global_mounts_unlock_write() {
    unsafe { MOUNT_POINT_LOCK.write_unlock() };
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

#[no_mangle]
pub unsafe fn __horizon_global_mounts_add(
    name: &str,
    contents: MountDevice<OwnedHandle>,
) -> Result<(), AddError> {
    let mounts = &mut MOUNT_POINTS;

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

#[no_mangle]
pub unsafe fn __horizon_global_mounts_remove(name: &str) -> Option<MountDevice<OwnedHandle>> {
    let mounts = &mut MOUNT_POINTS;

    if let Some((p, _)) = find_impl(&mounts, name) {
        let old: Mount = core::mem::replace(&mut mounts[p], Mount::empty());

        Some(old.into_inner())
    } else {
        None
    }
}

#[no_mangle]
pub unsafe fn __horizon_global_mounts_find(name: &str) -> Option<MountDevice<RefHandle<'static>>> {
    let mounts = &mut MOUNT_POINTS;

    if let Some((p, _)) = find_impl(&mounts, name) {
        let mount: &Mount = &mounts[p];
        let contents: &MountDevice<OwnedHandle> = &mount.device;

        Some(contents.as_ref())
    } else {
        None
    }
}

#[no_mangle]
pub unsafe fn __horizon_global_mounts_get(
    index: usize,
) -> Option<(&'static str, MountDevice<RefHandle<'static>>)> {
    let mounts = &mut MOUNT_POINTS;

    let mount = mounts.get(index)?;
    if mount.is_empty() {
        None
    } else {
        Some((mount.name(), mount.device.as_ref()))
    }
}
