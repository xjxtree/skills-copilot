#![cfg_attr(not(unix), allow(dead_code))]

use std::{
    fs::{self, File},
    io,
    path::{Component, Path, PathBuf},
};

#[cfg(not(windows))]
use fs4::FileExt;

use crate::CommandError;

#[cfg(all(test, unix))]
thread_local! {
    static INJECT_NEXT_OWNER_DIRECTORY_SYNC_FAILURE: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum OwnerCreationFaultPoint {
    Stat,
    Open,
    Chmod,
}

#[cfg(all(test, unix))]
thread_local! {
    static OWNER_CREATION_FAULTS: std::cell::RefCell<Vec<OwnerCreationFaultPoint>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(all(test, unix))]
pub(crate) fn install_owner_creation_fault(point: OwnerCreationFaultPoint) {
    OWNER_CREATION_FAULTS.with(|faults| faults.borrow_mut().push(point));
}

#[cfg(all(test, unix))]
fn run_owner_creation_fault(point: OwnerCreationFaultPoint) -> Result<(), io::Error> {
    OWNER_CREATION_FAULTS.with(|faults| {
        let mut faults = faults.borrow_mut();
        if let Some(index) = faults.iter().position(|candidate| *candidate == point) {
            faults.remove(index);
            return Err(io::Error::other(format!(
                "injected owner creation fault at {point:?}"
            )));
        }
        Ok(())
    })
}

#[cfg(all(not(test), unix))]
fn run_owner_creation_fault(_point: OwnerCreationFaultPoint) -> Result<(), io::Error> {
    Ok(())
}

/// Cross-sidecar owner lock for every app-coordinated mutation.
///
/// The owner directory must already exist. Acquiring this lock never creates a
/// lock file, target directory, or other stale-preview artifact.
pub struct AppMutationLock {
    file: File,
    owner_path: PathBuf,
    #[cfg(windows)]
    mutex: WindowsMutationMutex,
}

impl AppMutationLock {
    /// Clone the already-opened no-follow app-data owner descriptor.
    ///
    /// Callers use this capability for descriptor-relative child I/O. The
    /// returned descriptor names the same directory inode even if its display
    /// path is later renamed or replaced. On Unix it uses a fresh open-file
    /// description so retaining the descriptor does not retain this guard's
    /// advisory lock after the guard is dropped.
    pub fn open_owner_directory(&self) -> Result<File, CommandError> {
        #[cfg(unix)]
        {
            use rustix::fs::{openat, Mode, OFlags};

            let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
            openat(&self.file, ".", flags, Mode::empty())
                .map(File::from)
                .map_err(|error| io::Error::from(error).into())
        }
        #[cfg(not(unix))]
        {
            self.file.try_clone().map_err(Into::into)
        }
    }

    pub fn owner_directory(&self) -> &File {
        &self.file
    }

    #[cfg(not(unix))]
    pub(crate) fn owner_path(&self) -> &Path {
        &self.owner_path
    }

    pub fn owner_fs(&self) -> crate::AppDataOwnerFs<'_> {
        crate::app_data_owner_fs::AppDataOwnerFs::new(self)
    }

    pub fn validate_owner_path_binding(&self) -> Result<(), CommandError> {
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            let reopened = open_existing_app_mutation_owner(&self.owner_path)?;
            let locked = self.file.metadata()?;
            let current = reopened.metadata()?;
            if locked.dev() != current.dev() || locked.ino() != current.ino() {
                return Err(unsafe_owner());
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            validate_existing_owner(&self.owner_path)?;
            Ok(())
        }
    }
}

impl Drop for AppMutationLock {
    fn drop(&mut self) {
        #[cfg(not(windows))]
        {
            let _ = self.file.unlock();
        }
    }
}

pub fn lock_app_mutations(app_data_dir: &Path) -> Result<AppMutationLock, CommandError> {
    let file = open_existing_app_mutation_owner(app_data_dir)?;
    lock_open_owner(file, app_data_dir)
}

/// Create and lock the private app-data owner for an already-confirmed action.
///
/// Creation is deliberately limited to one missing leaf below an existing
/// non-symlink parent. Callers must finish typed confirmation and non-creating
/// preflight validation before invoking this function, so every rejection
/// known before coordination bootstrap remains zero-write.
pub fn lock_or_create_app_mutations(app_data_dir: &Path) -> Result<AppMutationLock, CommandError> {
    let file = open_or_create_app_mutation_owner(app_data_dir)?;
    lock_open_owner(file, app_data_dir)
}

/// Create missing app-data ancestors and lock the owner for a confirmed write.
///
/// This is the recursive initialization contract used by catalog refreshes,
/// project-context applies, and already-confirmed catalog-backed actions.
/// Skill Manager's fresh-filesystem search exception deliberately does not use
/// it: that path remains limited to [`lock_or_create_app_mutations`], which may
/// create only one missing owner leaf below an existing parent.
pub fn lock_or_create_app_mutations_with_parents(
    app_data_dir: &Path,
) -> Result<AppMutationLock, CommandError> {
    let file = open_app_mutation_directory_tree(app_data_dir, true, true)?;
    lock_open_owner(file, app_data_dir)
}

fn lock_open_owner(file: File, owner_path: &Path) -> Result<AppMutationLock, CommandError> {
    #[cfg(not(windows))]
    {
        file.lock_exclusive()?;
        Ok(AppMutationLock {
            file,
            owner_path: owner_path.to_path_buf(),
        })
    }
    #[cfg(windows)]
    {
        let mutex = WindowsMutationMutex::acquire(&file)?;
        Ok(AppMutationLock {
            file,
            owner_path: owner_path.to_path_buf(),
            mutex,
        })
    }
}

#[cfg(windows)]
struct WindowsMutationMutex {
    handle: usize,
}

#[cfg(windows)]
impl WindowsMutationMutex {
    fn acquire(owner: &File) -> Result<Self, CommandError> {
        use std::{mem::MaybeUninit, os::windows::io::AsRawHandle};
        use windows_sys::Win32::{
            Foundation::{CloseHandle, WAIT_ABANDONED, WAIT_FAILED, WAIT_OBJECT_0},
            Storage::FileSystem::{GetFileInformationByHandle, BY_HANDLE_FILE_INFORMATION},
            System::Threading::{CreateMutexW, WaitForSingleObject, INFINITE},
        };

        let raw_owner = owner.as_raw_handle().cast();
        let mut information = MaybeUninit::<BY_HANDLE_FILE_INFORMATION>::zeroed();
        if unsafe { GetFileInformationByHandle(raw_owner, information.as_mut_ptr()) } == 0 {
            return Err(io::Error::last_os_error().into());
        }
        let information = unsafe { information.assume_init() };
        let name = format!(
            "Local\\AgentCopilotMutation-{:08x}-{:08x}{:08x}",
            information.dwVolumeSerialNumber, information.nFileIndexHigh, information.nFileIndexLow
        );
        let wide_name = name.encode_utf16().chain(Some(0)).collect::<Vec<_>>();
        let handle = unsafe { CreateMutexW(std::ptr::null(), 0, wide_name.as_ptr()) };
        if handle.is_null() {
            return Err(io::Error::last_os_error().into());
        }
        match unsafe { WaitForSingleObject(handle, INFINITE) } {
            WAIT_OBJECT_0 | WAIT_ABANDONED => Ok(Self {
                handle: handle as usize,
            }),
            WAIT_FAILED => {
                let error = io::Error::last_os_error();
                unsafe {
                    CloseHandle(handle);
                }
                Err(error.into())
            }
            result => {
                unsafe {
                    CloseHandle(handle);
                }
                Err(io::Error::other(format!(
                    "unexpected Windows mutation mutex wait result: {result}"
                ))
                .into())
            }
        }
    }
}

#[cfg(windows)]
impl Drop for WindowsMutationMutex {
    fn drop(&mut self) {
        use windows_sys::Win32::{Foundation::CloseHandle, System::Threading::ReleaseMutex};

        let handle = self.handle as windows_sys::Win32::Foundation::HANDLE;
        unsafe {
            ReleaseMutex(handle);
            CloseHandle(handle);
        }
    }
}

pub(crate) fn app_mutation_owner_is_missing(app_data_dir: &Path) -> Result<bool, CommandError> {
    #[cfg(unix)]
    {
        let (parent, name) = open_app_mutation_parent(app_data_dir)?;
        match open_app_mutation_child(&parent, name) {
            Ok(_) => Ok(false),
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(true),
            Err(error) if is_unsafe_directory_error(&error) => Err(unsafe_owner()),
            Err(error) => Err(error.into()),
        }
    }
    #[cfg(not(unix))]
    {
        match fs::symlink_metadata(app_data_dir) {
            Ok(_) => {
                validate_existing_owner(app_data_dir)?;
                Ok(false)
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(true),
            Err(error) => Err(error.into()),
        }
    }
}

#[cfg(not(unix))]
fn validate_existing_owner(path: &Path) -> Result<(), CommandError> {
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(CommandError::UnsafeConfigPath(
            "mutation lock owner must be a non-symlink app data directory".to_string(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn open_existing_app_mutation_owner(path: &Path) -> Result<File, CommandError> {
    open_app_mutation_directory_tree(path, false, true)
}

/// Open an existing trusted directory with the same component-wise no-follow
/// walk used for the app-data mutation owner.
///
/// External agent config and skill mutations use this only after their typed
/// target has been revalidated while the app-data mutation lock is held. The
/// returned descriptor is the root capability for all later target-relative
/// I/O; callers never reopen the external target by pathname on Unix.
pub(crate) fn open_existing_directory_nofollow(path: &Path) -> Result<File, CommandError> {
    open_app_mutation_directory_tree(path, false, true)
}

#[cfg(not(unix))]
fn open_existing_app_mutation_owner(path: &Path) -> Result<File, CommandError> {
    validate_existing_owner(path)?;
    let canonical_owner = path.canonicalize()?;
    let file = open_nonunix_directory(&canonical_owner)?;
    if !file.metadata()?.is_dir() {
        return Err(CommandError::UnsafeConfigPath(
            "mutation lock owner is not the app data directory".to_string(),
        ));
    }
    Ok(file)
}

#[cfg(unix)]
fn open_or_create_app_mutation_owner(path: &Path) -> Result<File, CommandError> {
    open_or_create_app_mutation_owner_with_parent_open_hook(path, |_| {})
}

#[cfg(unix)]
fn open_or_create_app_mutation_owner_with_parent_open_hook(
    path: &Path,
    before_parent_open: impl FnOnce(&Path),
) -> Result<File, CommandError> {
    use rustix::fs::{fchmod, mkdirat, statat, AtFlags, FileType, Mode};

    if !app_mutation_owner_is_missing(path)? {
        return open_existing_app_mutation_owner(path);
    }
    if !path.is_absolute() {
        return Err(CommandError::UnsafeConfigPath(
            "a missing mutation lock owner must use an absolute app data path".to_string(),
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        CommandError::UnsafeConfigPath(
            "mutation lock owner has no existing parent directory".to_string(),
        )
    })?;
    let name = path.file_name().ok_or_else(|| {
        CommandError::UnsafeConfigPath(
            "mutation lock owner has no private directory name".to_string(),
        )
    })?;
    before_parent_open(parent);
    let (parent_file, opened_name) = open_app_mutation_parent(path)?;
    if opened_name != name {
        return Err(unsafe_owner());
    }
    let private_mode = Mode::from_bits_truncate(0o700);
    let created = match mkdirat(&parent_file, name, private_mode).map_err(io::Error::from) {
        Ok(()) => true,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => false,
        Err(error) => return Err(error.into()),
    };
    let created_identity = if created {
        run_owner_creation_fault(OwnerCreationFaultPoint::Stat)
            .map_err(|error| created_owner_effect_unknown(error.to_string()))?;
        let stat = statat(&parent_file, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
            created_owner_effect_unknown(format!(
                "the created app-data owner identity could not be read: {}",
                io::Error::from(error)
            ))
        })?;
        if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
            || stat.st_uid != rustix::process::geteuid().as_raw()
        {
            return Err(created_owner_effect_unknown(
                "the created app-data owner is no longer a directory".to_string(),
            ));
        }
        Some((stat.st_dev as u64, stat.st_ino))
    } else {
        None
    };
    if created {
        run_owner_creation_fault(OwnerCreationFaultPoint::Open)
            .map_err(|error| created_owner_effect_unknown(error.to_string()))?;
    }
    let owner = open_app_mutation_child(&parent_file, name).map_err(|error| {
        if created {
            created_owner_effect_unknown(format!(
                "the created app-data owner descriptor could not be opened: {error}"
            ))
        } else if is_unsafe_directory_error(&error) {
            unsafe_owner()
        } else {
            error.into()
        }
    })?;
    use std::os::unix::fs::MetadataExt;

    let owner_metadata = owner.metadata().map_err(|error| {
        if created {
            created_owner_effect_unknown(format!(
                "the created app-data owner descriptor identity could not be read: {error}"
            ))
        } else {
            CommandError::from(error)
        }
    })?;
    if owner_metadata.uid() != rustix::process::geteuid().as_raw() {
        return Err(if created {
            created_owner_effect_unknown(
                "the created app-data owner is not owned by the current effective user".to_string(),
            )
        } else {
            unsafe_owner()
        });
    }
    if created {
        if created_identity != Some((owner_metadata.dev(), owner_metadata.ino())) {
            return Err(created_owner_effect_unknown(
                "the created app-data owner changed before descriptor binding".to_string(),
            ));
        }
        run_owner_creation_fault(OwnerCreationFaultPoint::Chmod)
            .map_err(|error| created_owner_effect_unknown(error.to_string()))?;
        fchmod(&owner, private_mode).map_err(|error| {
            created_owner_effect_unknown(format!(
                "private app-data owner permissions could not be applied: {}",
                io::Error::from(error)
            ))
        })?;
        sync_created_app_mutation_directory(&parent_file, &owner)?;
    }
    let file = owner;
    if !file.metadata()?.is_dir() {
        return Err(CommandError::UnsafeConfigPath(
            "mutation lock owner is not the app data directory".to_string(),
        ));
    }
    Ok(file)
}

#[cfg(unix)]
fn open_app_mutation_directory_tree(
    path: &Path,
    create_missing: bool,
    require_effective_user_owner: bool,
) -> Result<File, CommandError> {
    use rustix::fs::{fchmod, mkdirat, open, openat, statat, AtFlags, FileType, Mode, OFlags};
    use rustix::io::Errno;
    use std::os::unix::fs::MetadataExt;

    let path = normalize_trusted_root_alias(path)?;
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut current = if path.is_absolute() {
        open("/", flags, Mode::empty()).map_err(io::Error::from)?
    } else {
        open(".", flags, Mode::empty()).map_err(io::Error::from)?
    };
    let mut saw_name = false;
    let mut created_any = false;
    for component in path.components() {
        let name = match component {
            Component::RootDir | Component::CurDir => continue,
            Component::Normal(name) => name,
            Component::ParentDir | Component::Prefix(_) => return Err(unsafe_owner()),
        };
        saw_name = true;
        match openat(&current, name, flags, Mode::empty()) {
            Ok(next) => current = next,
            Err(error) if error == Errno::NOENT && create_missing => {
                let mode = Mode::from_bits_truncate(0o700);
                let created = match mkdirat(&current, name, mode) {
                    Ok(()) => true,
                    Err(Errno::EXIST) => false,
                    Err(error) => return Err(io::Error::from(error).into()),
                };
                created_any |= created;
                let created_identity = if created {
                    run_owner_creation_fault(OwnerCreationFaultPoint::Stat)
                        .map_err(|error| created_owner_effect_unknown(error.to_string()))?;
                    let stat =
                        statat(&current, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                            created_owner_effect_unknown(format!(
                                "a created app-data ancestor identity could not be read: {}",
                                io::Error::from(error)
                            ))
                        })?;
                    if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
                        || stat.st_uid != rustix::process::geteuid().as_raw()
                    {
                        return Err(created_owner_effect_unknown(
                            "a created app-data ancestor is no longer a directory".to_string(),
                        ));
                    }
                    Some((stat.st_dev as u64, stat.st_ino))
                } else {
                    None
                };
                if created {
                    run_owner_creation_fault(OwnerCreationFaultPoint::Open)
                        .map_err(|error| created_owner_effect_unknown(error.to_string()))?;
                }
                let next = openat(&current, name, flags, Mode::empty()).map_err(|error| {
                    let error = io::Error::from(error);
                    if created_any {
                        created_owner_effect_unknown(format!(
                            "a created app-data directory chain could not be opened: {error}"
                        ))
                    } else if is_unsafe_directory_error(&error) {
                        unsafe_owner()
                    } else {
                        error.into()
                    }
                })?;
                if created {
                    let metadata = File::from(
                        rustix::fs::openat(&next, ".", flags, Mode::empty())
                            .map_err(|error| {
                                created_owner_effect_unknown(format!(
                                    "a created app-data ancestor descriptor could not be cloned: {}",
                                    io::Error::from(error)
                                ))
                            })?,
                    )
                    .metadata()
                    .map_err(|error| {
                        created_owner_effect_unknown(format!(
                            "a created app-data ancestor descriptor identity could not be read: {error}"
                        ))
                    })?;
                    if created_identity != Some((metadata.dev(), metadata.ino())) {
                        return Err(created_owner_effect_unknown(
                            "a created app-data ancestor changed before descriptor binding"
                                .to_string(),
                        ));
                    }
                    if metadata.uid() != rustix::process::geteuid().as_raw() {
                        return Err(created_owner_effect_unknown(
                            "a created app-data ancestor is not owned by the current effective user"
                                .to_string(),
                        ));
                    }
                    run_owner_creation_fault(OwnerCreationFaultPoint::Chmod)
                        .map_err(|error| created_owner_effect_unknown(error.to_string()))?;
                    fchmod(&next, mode).map_err(|error| {
                        created_owner_effect_unknown(format!(
                            "private app-data ancestor permissions could not be applied: {}",
                            io::Error::from(error)
                        ))
                    })?;
                    sync_created_app_mutation_directory(&current, &next)?;
                }
                current = next;
            }
            Err(error) if error == Errno::LOOP || error == Errno::NOTDIR => {
                return Err(if created_any {
                    created_owner_effect_unknown(
                        "a later app-data ancestor became a symlink or non-directory".to_string(),
                    )
                } else {
                    unsafe_owner()
                })
            }
            Err(error) => {
                return Err(if created_any {
                    created_owner_effect_unknown(format!(
                        "a later app-data ancestor could not be opened: {}",
                        io::Error::from(error)
                    ))
                } else {
                    io::Error::from(error).into()
                })
            }
        }
    }
    if !saw_name {
        return Err(unsafe_owner());
    }
    let file = File::from(current);
    let metadata = file.metadata().map_err(|error| {
        if created_any {
            created_owner_effect_unknown(format!(
                "the created app-data owner descriptor could not be inspected: {error}"
            ))
        } else {
            error.into()
        }
    })?;
    if !metadata.is_dir()
        || (require_effective_user_owner && metadata.uid() != rustix::process::geteuid().as_raw())
    {
        return Err(if created_any {
            created_owner_effect_unknown(
                "the created app-data owner is no longer a directory".to_string(),
            )
        } else {
            unsafe_owner()
        });
    }
    Ok(file)
}

#[cfg(unix)]
fn sync_created_app_mutation_directory(
    parent: impl std::os::fd::AsFd,
    directory: impl std::os::fd::AsFd,
) -> Result<(), CommandError> {
    use rustix::fs::fsync;

    #[cfg(test)]
    if INJECT_NEXT_OWNER_DIRECTORY_SYNC_FAILURE.with(|flag| flag.replace(false)) {
        return Err(created_owner_durability_unknown(io::Error::other(
            "injected app-data owner directory sync failure",
        )));
    }

    fsync(directory)
        .and_then(|()| fsync(parent))
        .map_err(|error| created_owner_durability_unknown(io::Error::from(error)))
}

fn created_owner_durability_unknown(error: io::Error) -> CommandError {
    CommandError::PartialEffect {
        operation: "app-data owner initialization".to_string(),
        state: "outcome_unknown",
        cleanup_required: false,
        detail: format!(
            "a private app-data directory was created, but its durability could not be verified: {error}"
        ),
    }
}

fn created_owner_effect_unknown(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "app-data owner initialization".to_string(),
        state: "outcome_unknown",
        cleanup_required: false,
        detail: format!(
            "a private app-data directory was created, but initialization could not be verified: {detail}"
        ),
    }
}

#[cfg(not(unix))]
fn open_app_mutation_directory_tree(
    path: &Path,
    create_missing: bool,
    _require_effective_user_owner: bool,
) -> Result<File, CommandError> {
    let mut current = PathBuf::new();
    let mut saw_name = false;
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            Component::RootDir => current.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => return Err(unsafe_owner()),
            Component::Normal(name) => {
                saw_name = true;
                current.push(name);
                match fs::symlink_metadata(&current) {
                    Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                        return Err(unsafe_owner())
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound && create_missing => {
                        let created = match fs::create_dir(&current) {
                            Ok(()) => true,
                            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => false,
                            Err(error) => return Err(error.into()),
                        };
                        validate_existing_owner(&current)?;
                        if created {
                            let directory = open_nonunix_directory(&current)?;
                            sync_nonunix_directory(&directory)?;
                            let parent = current.parent().ok_or_else(unsafe_owner)?;
                            let parent = open_nonunix_directory(parent)?;
                            sync_nonunix_directory(&parent)?;
                        }
                    }
                    Err(error) => return Err(error.into()),
                }
            }
        }
    }
    if !saw_name {
        return Err(unsafe_owner());
    }
    validate_existing_owner(path)?;
    let canonical_owner = path.canonicalize()?;
    let file = open_nonunix_directory(&canonical_owner)?;
    if !file.metadata()?.is_dir() {
        return Err(unsafe_owner());
    }
    Ok(file)
}

#[cfg(all(not(unix), not(windows)))]
fn open_nonunix_directory(path: &Path) -> Result<File, io::Error> {
    File::open(path)
}

#[cfg(windows)]
fn open_nonunix_directory(path: &Path) -> Result<File, io::Error> {
    use std::{fs::OpenOptions, os::windows::fs::OpenOptionsExt};
    use windows_sys::Win32::Storage::FileSystem::{
        FILE_FLAG_BACKUP_SEMANTICS, FILE_SHARE_DELETE, FILE_SHARE_READ, FILE_SHARE_WRITE,
    };

    OpenOptions::new()
        .read(true)
        .share_mode(FILE_SHARE_READ | FILE_SHARE_WRITE | FILE_SHARE_DELETE)
        .custom_flags(FILE_FLAG_BACKUP_SEMANTICS)
        .open(path)
}

#[cfg(all(not(unix), not(windows)))]
fn sync_nonunix_directory(directory: &File) -> Result<(), io::Error> {
    directory.sync_all()
}

#[cfg(windows)]
fn sync_nonunix_directory(_directory: &File) -> Result<(), io::Error> {
    // Windows does not support FlushFileBuffers for directory handles. The
    // directory creation call is already ordered before reopening the owner.
    Ok(())
}

#[cfg(unix)]
fn open_app_mutation_parent(path: &Path) -> Result<(File, &std::ffi::OsStr), CommandError> {
    let parent = path.parent().ok_or_else(unsafe_owner)?;
    let name = path.file_name().ok_or_else(unsafe_owner)?;
    // The parent is only a descriptor-relative creation capability. It may be
    // a trusted system-owned sticky directory such as `/tmp`; the new owner
    // leaf itself is still identity-bound and must be owned by this process.
    let file = open_app_mutation_directory_tree(parent, false, false)?;
    Ok((file, name))
}

#[cfg(unix)]
fn open_app_mutation_child(parent: &File, name: &std::ffi::OsStr) -> Result<File, io::Error> {
    use rustix::fs::{openat, Mode, OFlags};

    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    openat(parent, name, flags, Mode::empty())
        .map(File::from)
        .map_err(io::Error::from)
}

#[cfg(unix)]
pub(crate) fn normalize_trusted_root_alias(path: &Path) -> Result<PathBuf, CommandError> {
    use std::os::unix::fs::MetadataExt;

    if !path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let mut components = path.components();
    if components.next() != Some(Component::RootDir) {
        return Err(unsafe_owner());
    }
    let Some(Component::Normal(first)) = components.next() else {
        return Ok(path.to_path_buf());
    };
    let root_entry = Path::new("/").join(first);
    let metadata = match fs::symlink_metadata(&root_entry) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(path.to_path_buf()),
        Err(error) => return Err(error.into()),
    };
    if !metadata.file_type().is_symlink() {
        return Ok(path.to_path_buf());
    }
    if metadata.uid() != 0 {
        return Err(unsafe_owner());
    }
    let mut normalized = fs::canonicalize(&root_entry)?;
    for component in components {
        match component {
            Component::Normal(name) => normalized.push(name),
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => {
                return Err(unsafe_owner())
            }
        }
    }
    Ok(normalized)
}

fn unsafe_owner() -> CommandError {
    CommandError::UnsafeConfigPath(
        "mutation lock owner must be a non-symlink app data directory".to_string(),
    )
}

fn is_unsafe_directory_error(error: &io::Error) -> bool {
    #[cfg(unix)]
    {
        error.raw_os_error().is_some_and(|code| {
            code == rustix::io::Errno::LOOP.raw_os_error()
                || code == rustix::io::Errno::NOTDIR.raw_os_error()
        })
    }
    #[cfg(not(unix))]
    {
        error.kind() == io::ErrorKind::InvalidInput
    }
}

#[cfg(not(unix))]
fn open_or_create_app_mutation_owner(path: &Path) -> Result<File, CommandError> {
    if !app_mutation_owner_is_missing(path)? {
        return open_existing_app_mutation_owner(path);
    }
    if !path.is_absolute() {
        return Err(CommandError::UnsafeConfigPath(
            "a missing mutation lock owner must use an absolute app data path".to_string(),
        ));
    }
    let parent = path.parent().ok_or_else(|| {
        CommandError::UnsafeConfigPath(
            "mutation lock owner has no existing parent directory".to_string(),
        )
    })?;
    validate_existing_owner(parent)?;
    let created = match fs::create_dir(path) {
        Ok(()) => true,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => false,
        Err(error) => return Err(error.into()),
    };
    validate_existing_owner(path)?;
    if created {
        let directory = open_nonunix_directory(path)?;
        sync_nonunix_directory(&directory)?;
        let parent = open_nonunix_directory(parent)?;
        sync_nonunix_directory(&parent)?;
    }
    open_existing_app_mutation_owner(path)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::{
        process::Command,
        thread,
        time::{Duration, SystemTime, UNIX_EPOCH},
    };

    const CHILD_OWNER: &str = "SKILLS_COPILOT_LOCK_TEST_OWNER";
    const CHILD_MARKER: &str = "SKILLS_COPILOT_LOCK_TEST_MARKER";

    #[test]
    fn cross_process_lock_child() {
        let (Ok(owner), Ok(marker)) = (std::env::var(CHILD_OWNER), std::env::var(CHILD_MARKER))
        else {
            return;
        };
        let _lock = lock_app_mutations(Path::new(&owner)).expect("child acquires owner lock");
        std::fs::write(marker, "entered").expect("child writes entry marker");
    }

    #[test]
    fn app_mutation_lock_blocks_another_process_until_the_guard_is_released() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "skills-copilot-cross-process-lock-{}-{unique}",
            std::process::id()
        ));
        let owner = root.join("app-data");
        let marker = root.join("child-entered");
        std::fs::create_dir_all(&owner).expect("create owner");
        let lock = lock_app_mutations(&owner).expect("parent acquires owner lock");
        let mut child = Command::new(std::env::current_exe().expect("current test executable"))
            .args([
                "--exact",
                "mutation_lock::tests::cross_process_lock_child",
                "--nocapture",
            ])
            .env(CHILD_OWNER, &owner)
            .env(CHILD_MARKER, &marker)
            .spawn()
            .expect("spawn child test process");

        for _ in 0..10 {
            assert!(
                child.try_wait().expect("poll child").is_none(),
                "child must remain blocked while the parent guard is held"
            );
            assert!(
                !marker.exists(),
                "the second process must not enter the protected mutation/read-back section"
            );
            thread::sleep(Duration::from_millis(10));
        }

        drop(lock);
        let status = child.wait().expect("wait child");
        assert!(status.success());
        assert!(
            marker.exists(),
            "the second process enters only after the first guard is released"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn confirmed_owner_creation_is_private_and_rejects_symlinks() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "skills-copilot-create-mutation-owner-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create parent");
        let owner = root.join("app-data");

        let lock = lock_or_create_app_mutations(&owner).expect("create and lock owner");
        let mode = std::fs::metadata(&owner)
            .expect("owner metadata")
            .permissions()
            .mode()
            & 0o777;
        assert_eq!(mode, 0o700, "new app-data owner must be private");
        drop(lock);

        let outside = root.join("outside");
        let linked_owner = root.join("linked-app-data");
        std::fs::create_dir(&outside).expect("create outside directory");
        symlink(&outside, &linked_owner).expect("create owner symlink");
        assert!(
            matches!(
                lock_or_create_app_mutations(&linked_owner),
                Err(CommandError::UnsafeConfigPath(_))
            ),
            "a symlink must never become the mutation owner"
        );

        let linked_parent = root.join("linked-parent");
        symlink(&outside, &linked_parent).expect("create parent symlink");
        assert!(
            lock_or_create_app_mutations(&linked_parent.join("app-data")).is_err(),
            "a final-component parent symlink must fail closed instead of being canonicalized"
        );
        assert!(
            !outside.join("app-data").exists(),
            "rejected symlink parents must remain zero-write"
        );

        let replaced_parent = root.join("replaced-parent");
        let original_parent = root.join("replaced-parent-original");
        std::fs::create_dir(&replaced_parent).expect("create replaceable parent");
        let replaced_owner = replaced_parent.join("app-data");
        assert!(
            open_or_create_app_mutation_owner_with_parent_open_hook(&replaced_owner, |parent| {
                std::fs::rename(parent, &original_parent)
                    .expect("move validated parent out of the way");
                symlink(&outside, parent).expect("replace parent with symlink");
            })
            .is_err(),
            "a parent replaced by a symlink immediately before open must fail closed"
        );
        assert!(
            !outside.join("app-data").exists(),
            "a raced parent symlink must not redirect owner creation"
        );
        assert!(
            !original_parent.join("app-data").exists(),
            "the moved original parent must remain unchanged"
        );

        let missing_chain = root.join("missing-parent/app-data");
        assert!(
            lock_or_create_app_mutations(&missing_chain).is_err(),
            "owner creation must not manufacture a missing parent chain"
        );
        assert!(
            !root.join("missing-parent").exists(),
            "missing parent rejection must remain zero-write"
        );
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn confirmed_owner_creation_accepts_a_system_owned_existing_parent() {
        use std::os::unix::fs::{MetadataExt, PermissionsExt};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let owner = std::env::temp_dir().join(format!(
            "skills-copilot-system-parent-owner-{}-{unique}",
            std::process::id()
        ));

        let lock = lock_or_create_app_mutations(&owner)
            .expect("create an owned leaf below the platform temporary directory");
        let metadata = std::fs::metadata(&owner).expect("created owner metadata");
        assert!(metadata.is_dir());
        assert_eq!(metadata.uid(), rustix::process::geteuid().as_raw());
        assert_eq!(metadata.permissions().mode() & 0o777, 0o700);

        drop(lock);
        let _ = std::fs::remove_dir(owner);
    }

    #[test]
    #[cfg(unix)]
    fn confirmed_owner_creation_reports_unknown_outcome_when_directory_sync_fails() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "skills-copilot-create-mutation-owner-sync-failure-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create parent");
        let owner = root.join("app-data");

        INJECT_NEXT_OWNER_DIRECTORY_SYNC_FAILURE.with(|flag| flag.set(true));
        let error = match lock_or_create_app_mutations(&owner) {
            Ok(_) => panic!("directory sync failure must be partial"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            CommandError::PartialEffect {
                operation,
                state: "outcome_unknown",
                cleanup_required: false,
                ..
            } if operation == "app-data owner initialization"
        ));
        let metadata =
            std::fs::metadata(&owner).expect("created private owner remains inspectable");
        assert!(metadata.is_dir());
        assert_eq!(metadata.permissions().mode() & 0o777, 0o700);
        assert_eq!(
            std::fs::read_dir(&owner).expect("read owner").count(),
            0,
            "durability failure must happen before any child state is created"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn confirmed_owner_creation_faults_after_mkdir_are_typed_partial_effects() {
        for point in [
            OwnerCreationFaultPoint::Stat,
            OwnerCreationFaultPoint::Open,
            OwnerCreationFaultPoint::Chmod,
        ] {
            let unique = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("system clock")
                .as_nanos();
            let root = std::env::temp_dir().join(format!(
                "skills-copilot-owner-creation-fault-{point:?}-{}-{unique}",
                std::process::id()
            ));
            std::fs::create_dir_all(&root).expect("create parent");
            let owner = root.join("app-data");
            install_owner_creation_fault(point);

            let error = match lock_or_create_app_mutations(&owner) {
                Ok(_) => panic!("post-mkdir fault must not report success"),
                Err(error) => error,
            };

            assert!(matches!(
                error,
                CommandError::PartialEffect {
                    operation,
                    state: "outcome_unknown",
                    cleanup_required: false,
                    ..
                } if operation == "app-data owner initialization"
            ));
            assert!(
                std::fs::symlink_metadata(&owner)
                    .expect("created owner remains")
                    .is_dir(),
                "the namespace effect must be retained and reported"
            );
            let _ = std::fs::remove_dir_all(root);
        }
    }

    #[test]
    #[cfg(unix)]
    fn recursive_confirmed_owner_creation_rejects_intermediate_and_final_symlinks() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "skills-copilot-recursive-mutation-owner-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("create root");

        let recursive_owner = root.join("new-parent/new-child/app-data");
        let lock = lock_or_create_app_mutations_with_parents(&recursive_owner)
            .expect("confirmed catalog-style owner creation");
        assert_eq!(
            std::fs::metadata(&recursive_owner)
                .expect("owner metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        drop(lock);

        let victim = root.join("victim");
        std::fs::create_dir(&victim).expect("create victim");
        std::fs::write(victim.join("sentinel"), "unchanged").expect("seed victim");
        let linked_component = root.join("linked-component");
        symlink(&victim, &linked_component).expect("create intermediate link");
        let intermediate_result =
            lock_or_create_app_mutations_with_parents(&linked_component.join("nested/app-data"));
        assert!(matches!(
            intermediate_result,
            Err(CommandError::UnsafeConfigPath(_))
        ));
        assert_eq!(
            std::fs::read_to_string(victim.join("sentinel")).expect("victim sentinel"),
            "unchanged"
        );
        assert!(
            !victim.join("nested").exists(),
            "an intermediate symlink must not redirect recursive creation"
        );

        let linked_owner = root.join("linked-owner");
        symlink(&victim, &linked_owner).expect("create final owner link");
        let final_result = lock_or_create_app_mutations_with_parents(&linked_owner);
        assert!(matches!(
            final_result,
            Err(CommandError::UnsafeConfigPath(_))
        ));
        assert!(
            !victim.join("catalog.sqlite").exists(),
            "a final symlink target must remain untouched"
        );

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn opened_owner_descriptor_remains_on_the_locked_inode_after_path_replacement() {
        use std::os::unix::fs::{symlink, MetadataExt};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "skills-copilot-owner-capability-{}-{unique}",
            std::process::id()
        ));
        let owner = root.join("app-data");
        let moved_owner = root.join("app-data-opened");
        let victim = root.join("victim");
        std::fs::create_dir_all(&owner).expect("create owner");
        std::fs::create_dir(&victim).expect("create victim");
        let guard = lock_app_mutations(&owner).expect("lock owner");
        let capability = guard.open_owner_directory().expect("open owner descriptor");
        let accepted = capability.metadata().expect("accepted owner metadata");

        std::fs::rename(&owner, &moved_owner).expect("move accepted owner");
        symlink(&victim, &owner).expect("replace display path");

        let retained = capability.metadata().expect("retained owner metadata");
        let victim_metadata = std::fs::metadata(&victim).expect("victim metadata");
        assert_eq!(
            (retained.dev(), retained.ino()),
            (accepted.dev(), accepted.ino())
        );
        assert_ne!(
            (retained.dev(), retained.ino()),
            (victim_metadata.dev(), victim_metadata.ino()),
            "the descriptor capability must never retarget through the replaced path"
        );

        drop(capability);
        drop(guard);
        let _ = std::fs::remove_file(owner);
        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    #[cfg(unix)]
    fn opened_owner_descriptor_does_not_extend_the_mutation_guard_lock() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "skills-copilot-owner-capability-lock-{}-{unique}",
            std::process::id()
        ));
        let owner = root.join("app-data");
        std::fs::create_dir_all(&owner).expect("create owner");

        let guard = lock_app_mutations(&owner).expect("lock owner");
        let capability = guard
            .open_owner_directory()
            .expect("open independent owner descriptor");
        let competing = open_existing_app_mutation_owner(&owner).expect("open competing owner");
        assert!(
            competing.try_lock_exclusive().is_err(),
            "the mutation guard must exclude a separate owner descriptor"
        );

        drop(guard);
        competing
            .try_lock_exclusive()
            .expect("the retained capability must not retain the mutation lock");
        competing.unlock().expect("unlock competing owner");
        assert!(capability.metadata().expect("capability metadata").is_dir());

        drop(capability);
        let _ = std::fs::remove_dir_all(root);
    }
}
