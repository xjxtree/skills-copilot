use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Read, Seek, SeekFrom, Write},
    marker::PhantomData,
    path::{Component, Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::{mutation_lock::AppMutationLock, CommandError};

mod tree;

const PRIVATE_CLEANUP_LEAF_REVISION_DOMAIN: &str = "agent-copilot/app-data-private-cleanup-leaf/v1";

#[cfg(unix)]
pub(crate) fn unix_timestamp_nanoseconds(value: impl TryInto<i64>) -> i64 {
    value.try_into().unwrap_or(i64::MAX)
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum AppDataPrivateLeafKind {
    RegularFile,
    SymbolicLink,
}

impl AppDataPrivateLeafKind {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::RegularFile => "regular_file",
            Self::SymbolicLink => "symbolic_link",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AppDataPrivateLeafSnapshot {
    kind: AppDataPrivateLeafKind,
    revision: String,
    bytes: Option<Vec<u8>>,
    identity: AppDataPrivateLeafIdentity,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AppDataCreatedDirectory {
    relative: PathBuf,
    #[cfg(unix)]
    device: u64,
    #[cfg(unix)]
    inode: u64,
    #[cfg(unix)]
    uid: u32,
    #[cfg(unix)]
    mode: u32,
}

impl AppDataPrivateLeafSnapshot {
    pub fn kind(&self) -> AppDataPrivateLeafKind {
        self.kind
    }

    pub fn revision(&self) -> &str {
        &self.revision
    }

    pub fn regular_file_bytes(&self) -> Option<&[u8]> {
        self.bytes.as_deref()
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct AppDataPrivateLeafIdentity {
    device: u64,
    inode: u64,
    uid: u32,
    link_count: u64,
    mode: u32,
    size: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct PrivateRegularFileStamp {
    device: u64,
    inode: u64,
    uid: u32,
    link_count: u64,
    mode: u32,
    size: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
impl PrivateRegularFileStamp {
    fn binding_revision(self) -> String {
        format!(
            "{}:{}:{:o}:{}:{}:{}:{}:{}:{}",
            self.device,
            self.inode,
            self.mode,
            self.link_count,
            self.size,
            self.modified_seconds,
            self.modified_nanoseconds,
            self.changed_seconds,
            self.changed_nanoseconds
        )
    }

    fn private_identity(self) -> AppDataPrivateLeafIdentity {
        AppDataPrivateLeafIdentity {
            device: self.device,
            inode: self.inode,
            uid: self.uid,
            link_count: self.link_count,
            mode: self.mode,
            size: self.size,
            modified_seconds: self.modified_seconds,
            modified_nanoseconds: self.modified_nanoseconds,
            changed_seconds: self.changed_seconds,
            changed_nanoseconds: self.changed_nanoseconds,
        }
    }
}

#[cfg(unix)]
fn private_stamp_matches_after_rename(
    before: PrivateRegularFileStamp,
    moved: PrivateRegularFileStamp,
) -> bool {
    before.device == moved.device
        && before.inode == moved.inode
        && before.uid == moved.uid
        && before.link_count == moved.link_count
        && before.mode == moved.mode
        && before.size == moved.size
        && before.modified_seconds == moved.modified_seconds
        && before.modified_nanoseconds == moved.modified_nanoseconds
}

#[derive(Debug)]
struct BoundedRegularFileRead {
    bytes: Vec<u8>,
    binding_revision: String,
    identity: AppDataPrivateLeafIdentity,
}

#[cfg(test)]
thread_local! {
    static REMOVE_MATCHING_BEFORE_QUARANTINE_HOOK:
        std::cell::RefCell<Option<RemoveMatchingBeforeQuarantineHook>> =
            const { std::cell::RefCell::new(None) };
    static REMOVE_MATCHING_DIRECTORY_SYNC_FAULT: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
    static PRIVATE_CLEANUP_POST_QUARANTINE_HOOK:
        std::cell::RefCell<Option<PrivateCleanupPostQuarantineHook>> =
            const { std::cell::RefCell::new(None) };
    static PRIVATE_READ_BEFORE_FINAL_STAMP_HOOK:
        std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
            const { std::cell::RefCell::new(None) };
    static ENSURE_PRIVATE_DIRECTORY_SYNC_FAULT: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
    static PRIVATE_APPEND_POST_WRITE_HOOK:
        std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
            const { std::cell::RefCell::new(None) };
    static PRIVATE_WRITE_POST_ACTIVATION_HOOK:
        std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
            const { std::cell::RefCell::new(None) };
    static PRIVATE_CLEANUP_DIRECTORY_SYNC_FAULT: std::cell::Cell<bool> =
        const { std::cell::Cell::new(false) };
}

#[cfg(test)]
struct RemoveMatchingBeforeQuarantineHook {
    target: OsString,
    action: Box<dyn FnOnce()>,
}

#[cfg(test)]
struct PrivateCleanupPostQuarantineHook {
    target: OsString,
    action: Box<dyn FnOnce()>,
}

#[cfg(test)]
fn install_remove_matching_before_quarantine_hook(
    target: impl Into<OsString>,
    action: impl FnOnce() + 'static,
) {
    REMOVE_MATCHING_BEFORE_QUARANTINE_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(RemoveMatchingBeforeQuarantineHook {
            target: target.into(),
            action: Box::new(action),
        });
    });
}

#[cfg(test)]
fn run_remove_matching_before_quarantine_hook(name: &OsStr) {
    REMOVE_MATCHING_BEFORE_QUARANTINE_HOOK.with(|slot| {
        let action = {
            let mut hook = slot.borrow_mut();
            if hook.as_ref().is_some_and(|hook| hook.target == name) {
                hook.take().map(|hook| hook.action)
            } else {
                None
            }
        };
        if let Some(action) = action {
            action();
        }
    });
}

#[cfg(not(test))]
fn run_remove_matching_before_quarantine_hook(_name: &OsStr) {}

#[cfg(test)]
fn install_remove_matching_directory_sync_fault() {
    REMOVE_MATCHING_DIRECTORY_SYNC_FAULT.with(|fault| fault.set(true));
}

#[cfg(test)]
fn install_private_cleanup_post_quarantine_hook(
    target: impl Into<OsString>,
    action: impl FnOnce() + 'static,
) {
    PRIVATE_CLEANUP_POST_QUARANTINE_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(PrivateCleanupPostQuarantineHook {
            target: target.into(),
            action: Box::new(action),
        });
    });
}

#[cfg(test)]
fn run_private_cleanup_post_quarantine_hook(name: &OsStr) {
    PRIVATE_CLEANUP_POST_QUARANTINE_HOOK.with(|slot| {
        let action = {
            let mut hook = slot.borrow_mut();
            if hook.as_ref().is_some_and(|hook| hook.target == name) {
                hook.take().map(|hook| hook.action)
            } else {
                None
            }
        };
        if let Some(action) = action {
            action();
        }
    });
}

#[cfg(not(test))]
fn run_private_cleanup_post_quarantine_hook(_name: &OsStr) {}

#[cfg(test)]
fn install_private_read_before_final_stamp_hook(action: impl FnOnce() + 'static) {
    PRIVATE_READ_BEFORE_FINAL_STAMP_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(action));
    });
}

#[cfg(test)]
fn run_private_read_before_final_stamp_hook() {
    PRIVATE_READ_BEFORE_FINAL_STAMP_HOOK.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
fn run_private_read_before_final_stamp_hook() {}

#[cfg(test)]
fn install_ensure_private_directory_sync_fault() {
    ENSURE_PRIVATE_DIRECTORY_SYNC_FAULT.with(|fault| fault.set(true));
}

#[cfg(test)]
fn install_private_append_post_write_hook(action: impl FnOnce() + 'static) {
    PRIVATE_APPEND_POST_WRITE_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(action));
    });
}

#[cfg(test)]
fn run_private_append_post_write_hook() {
    PRIVATE_APPEND_POST_WRITE_HOOK.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
fn run_private_append_post_write_hook() {}

#[cfg(test)]
fn install_private_write_post_activation_hook(action: impl FnOnce() + 'static) {
    PRIVATE_WRITE_POST_ACTIVATION_HOOK.with(|slot| {
        *slot.borrow_mut() = Some(Box::new(action));
    });
}

#[cfg(test)]
fn run_private_write_post_activation_hook() {
    PRIVATE_WRITE_POST_ACTIVATION_HOOK.with(|slot| {
        if let Some(action) = slot.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
fn run_private_write_post_activation_hook() {}

#[cfg(test)]
fn install_private_cleanup_directory_sync_fault() {
    PRIVATE_CLEANUP_DIRECTORY_SYNC_FAULT.with(|fault| fault.set(true));
}
/// Filesystem capability rooted at the already-opened and locked app-data
/// owner directory.
///
/// The lifetime deliberately borrows the mutation lock so descriptor-relative
/// access cannot outlive the cross-process exclusion guard.
pub struct AppDataOwnerFs<'lock> {
    lock: &'lock AppMutationLock,
    _guard: PhantomData<&'lock AppMutationLock>,
}

impl<'lock> AppDataOwnerFs<'lock> {
    pub(crate) fn new(lock: &'lock AppMutationLock) -> Self {
        Self {
            lock,
            _guard: PhantomData,
        }
    }

    pub(crate) fn validate_owner_path_binding(&self) -> Result<(), CommandError> {
        self.lock.validate_owner_path_binding()
    }

    pub fn read_bounded_regular_file(
        &self,
        relative: &Path,
        max_bytes: u64,
        label: &str,
    ) -> Result<Option<Vec<u8>>, CommandError> {
        Ok(self
            .read_bounded_regular_file_snapshot(relative, max_bytes, label)?
            .map(|snapshot| {
                snapshot
                    .regular_file_bytes()
                    .expect("private regular-file snapshot has bytes")
                    .to_vec()
            }))
    }

    pub(crate) fn read_bounded_regular_file_with_stamp(
        &self,
        relative: &Path,
        max_bytes: u64,
        label: &str,
    ) -> Result<Option<(Vec<u8>, String)>, CommandError> {
        Ok(self
            .read_bounded_regular_file_state(relative, max_bytes, label, false)?
            .map(|read| (read.bytes, read.binding_revision)))
    }

    pub fn read_bounded_regular_file_snapshot(
        &self,
        relative: &Path,
        max_bytes: u64,
        label: &str,
    ) -> Result<Option<AppDataPrivateLeafSnapshot>, CommandError> {
        Ok(self
            .read_bounded_regular_file_state(relative, max_bytes, label, true)?
            .map(|read| {
                let revision = private_cleanup_leaf_revision(
                    AppDataPrivateLeafKind::RegularFile,
                    &read.identity,
                    Some(&read.bytes),
                );
                AppDataPrivateLeafSnapshot {
                    kind: AppDataPrivateLeafKind::RegularFile,
                    revision,
                    bytes: Some(read.bytes),
                    identity: read.identity,
                }
            }))
    }

    fn read_bounded_regular_file_state(
        &self,
        relative: &Path,
        max_bytes: u64,
        label: &str,
        require_private_mode: bool,
    ) -> Result<Option<BoundedRegularFileRead>, CommandError> {
        validate_relative_path(relative)?;
        let mut file = match self.open_regular_file(relative, false, require_private_mode) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(map_relative_io(error, label)),
        };
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() > max_bytes {
            return Err(unsafe_relative_file(label));
        }
        #[cfg(unix)]
        let before = {
            let before = private_regular_file_stamp_from_metadata(&metadata, label)?;
            let owner_uid = private_owner_uid(self.lock)?;
            if before.uid != owner_uid
                || before.link_count != 1
                || (require_private_mode && before.mode & 0o077 != 0)
            {
                return Err(unsafe_relative_file(label));
            }
            before
        };
        run_owner_read_test_hook(relative);
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        Read::by_ref(&mut file)
            .take(max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > max_bytes || bytes.len() as u64 != metadata.len() {
            return Err(unsafe_relative_file(label));
        }
        run_private_read_before_final_stamp_hook();
        #[cfg(unix)]
        {
            let after = private_regular_file_stamp_from_metadata(&file.metadata()?, label)?;
            if after != before {
                return Err(CommandError::StaleActionReference);
            }
            let rebound = self
                .open_regular_file(relative, false, require_private_mode)
                .map_err(|_| CommandError::StaleActionReference)?;
            let rebound_metadata = rebound
                .metadata()
                .map_err(|_| CommandError::StaleActionReference)?;
            if !rebound_metadata.is_file()
                || private_regular_file_stamp_from_metadata(&rebound_metadata, label)? != before
            {
                return Err(CommandError::StaleActionReference);
            }
            Ok(Some(BoundedRegularFileRead {
                bytes,
                binding_revision: before.binding_revision(),
                identity: before.private_identity(),
            }))
        }
        #[cfg(not(unix))]
        {
            if file.metadata()?.len() != metadata.len() {
                return Err(CommandError::StaleActionReference);
            }
            let identity = AppDataPrivateLeafIdentity {
                device: 0,
                inode: 0,
                uid: 0,
                link_count: 1,
                mode: 0,
                size: metadata.len(),
                modified_seconds: 0,
                modified_nanoseconds: 0,
                changed_seconds: 0,
                changed_nanoseconds: 0,
            };
            Ok(Some(BoundedRegularFileRead {
                bytes,
                binding_revision: String::new(),
                identity,
            }))
        }
    }

    pub fn append_private_file(
        &self,
        relative: &Path,
        bytes: &[u8],
        max_result_bytes: u64,
        label: &str,
    ) -> Result<(), CommandError> {
        validate_relative_path(relative)?;
        let appended_len = u64::try_from(bytes.len()).map_err(|_| {
            CommandError::UnsafeConfigPath(format!("{label} exceeds its append safety bound"))
        })?;
        if appended_len > max_result_bytes {
            return Err(CommandError::UnsafeConfigPath(format!(
                "{label} exceeds its append safety bound"
            )));
        }
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            use rustix::fs::{fchmod, fsync, openat, statat, AtFlags, Mode, OFlags};

            let (parent, name) = self.open_private_parent(relative)?;
            let mode = Mode::from_bits_truncate(0o600);
            let (descriptor, created) = match openat(
                &parent,
                name,
                OFlags::RDWR | OFlags::APPEND | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            ) {
                Ok(descriptor) => (descriptor, false),
                Err(rustix::io::Errno::NOENT) => (
                    openat(
                        &parent,
                        name,
                        OFlags::RDWR
                            | OFlags::APPEND
                            | OFlags::CREATE
                            | OFlags::EXCL
                            | OFlags::NOFOLLOW
                            | OFlags::CLOEXEC,
                        mode,
                    )
                    .map_err(io::Error::from)?,
                    true,
                ),
                Err(error) => return Err(io::Error::from(error).into()),
            };
            let mut file = File::from(descriptor);
            let mut metadata = match file.metadata() {
                Ok(metadata) => metadata,
                Err(error) if created => {
                    cleanup_created_private_append_file(
                        &parent,
                        name,
                        &file,
                        CommandError::from(error),
                    )?;
                    unreachable!("created append cleanup returns the original error")
                }
                Err(error) => return Err(error.into()),
            };
            let owner_uid = match private_owner_uid(self.lock) {
                Ok(uid) => uid,
                Err(error) if created => {
                    cleanup_created_private_append_file(&parent, name, &file, error)?;
                    unreachable!("created append cleanup returns the original error")
                }
                Err(error) => return Err(error),
            };
            if !metadata.is_file()
                || metadata.uid() != owner_uid
                || metadata.nlink() != 1
                || metadata.mode() & 0o077 != 0
            {
                let error = unsafe_relative_file(label);
                if created {
                    cleanup_created_private_append_file(&parent, name, &file, error)?;
                    unreachable!("created append cleanup returns the original error")
                }
                return Err(error);
            }
            if created {
                if let Err(error) = fchmod(&file, mode).map_err(io::Error::from) {
                    cleanup_created_private_append_file(
                        &parent,
                        name,
                        &file,
                        CommandError::from(error),
                    )?;
                    unreachable!("created append cleanup returns the original error")
                }
                metadata = match file.metadata() {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        cleanup_created_private_append_file(
                            &parent,
                            name,
                            &file,
                            CommandError::from(error),
                        )?;
                        unreachable!("created append cleanup returns the original error")
                    }
                };
            }
            let expected_len_result = metadata
                .len()
                .checked_add(appended_len)
                .filter(|len| *len <= max_result_bytes)
                .ok_or_else(|| {
                    CommandError::UnsafeConfigPath(format!(
                        "{label} exceeds its append safety bound"
                    ))
                });
            let expected_len = match expected_len_result {
                Ok(expected_len) => expected_len,
                Err(error) if created => {
                    cleanup_created_private_append_file(&parent, name, &file, error)?;
                    unreachable!("created append cleanup returns the original error")
                }
                Err(error) => return Err(error),
            };
            let before = private_regular_file_stamp_from_metadata(&metadata, label)?;
            let mut original = Vec::with_capacity(metadata.len() as usize);
            let original_read = (|| {
                file.seek(SeekFrom::Start(0))?;
                Read::by_ref(&mut file)
                    .take(max_result_bytes.saturating_add(1))
                    .read_to_end(&mut original)?;
                if original.len() as u64 != metadata.len()
                    || original.len() as u64 > max_result_bytes
                    || private_regular_file_stamp_from_metadata(&file.metadata()?, label)? != before
                {
                    return Err(CommandError::StaleActionReference);
                }
                Ok::<(), CommandError>(())
            })();
            if let Err(error) = original_read {
                if created {
                    cleanup_created_private_append_file(&parent, name, &file, error)?;
                    unreachable!("created append cleanup returns the original error")
                }
                return Err(error);
            }
            let mut expected_bytes = original;
            expected_bytes.extend_from_slice(bytes);
            if let Err(error) = file.write_all(bytes) {
                return Err(app_data_private_append_partial_error(format!(
                    "{label} append started but its bytes could not be completed: {error}"
                )));
            }
            run_private_append_post_write_hook();
            if let Err(error) = file.sync_all() {
                return Err(app_data_private_append_partial_error(format!(
                    "{label} append completed in memory but file durability could not be verified: {error}"
                )));
            }
            let observed_metadata = file.metadata().map_err(|error| {
                app_data_private_append_partial_error(format!(
                    "{label} append completed but its descriptor stamp could not be read: {error}"
                ))
            })?;
            let observed = private_regular_file_stamp_from_metadata(&observed_metadata, label)
                .map_err(|error| {
                    app_data_private_append_outcome_unknown(format!(
                        "{label} append completed but its descriptor stamp is unsafe: {error}"
                    ))
                })?;
            if observed.size != expected_len
                || observed.uid != owner_uid
                || observed.link_count != 1
                || observed.mode & 0o077 != 0
            {
                return Err(app_data_private_append_outcome_unknown(format!(
                    "{label} append result stamp changed concurrently"
                )));
            }
            let rebound = openat(
                &parent,
                name,
                OFlags::RDONLY
                    | OFlags::NOFOLLOW
                    | OFlags::NONBLOCK
                    | OFlags::NOCTTY
                    | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map(File::from)
            .map_err(|error| {
                app_data_private_append_outcome_unknown(format!(
                    "{label} append completed but its path entry could not be rebound: {}",
                    io::Error::from(error)
                ))
            })?;
            let rebound_metadata = rebound.metadata().map_err(|error| {
                app_data_private_append_outcome_unknown(format!(
                    "{label} append path-entry metadata could not be read: {error}"
                ))
            })?;
            let rebound_stamp = private_regular_file_stamp_from_metadata(&rebound_metadata, label)
                .map_err(|error| {
                    app_data_private_append_outcome_unknown(format!(
                        "{label} append path entry is unsafe after write: {error}"
                    ))
                })?;
            if rebound_stamp != observed {
                return Err(app_data_private_append_outcome_unknown(format!(
                    "{label} append path entry changed after write"
                )));
            }
            let mut rebound = rebound;
            let mut readback = Vec::with_capacity(expected_bytes.len());
            Read::by_ref(&mut rebound)
                .take(max_result_bytes.saturating_add(1))
                .read_to_end(&mut readback)
                .map_err(|error| {
                    app_data_private_append_partial_error(format!(
                        "{label} append result could not be read back: {error}"
                    ))
                })?;
            let final_descriptor_metadata = rebound.metadata().map_err(|error| {
                app_data_private_append_partial_error(format!(
                    "{label} append read-back metadata could not be read: {error}"
                ))
            })?;
            let final_descriptor_stamp =
                private_regular_file_stamp_from_metadata(&final_descriptor_metadata, label)
                    .map_err(|error| {
                        app_data_private_append_outcome_unknown(format!(
                            "{label} append read-back stamp is unsafe: {error}"
                        ))
                    })?;
            let final_path = statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                app_data_private_append_outcome_unknown(format!(
                    "{label} append final path entry could not be inspected: {}",
                    io::Error::from(error)
                ))
            })?;
            let final_path_stamp = private_regular_file_stamp_from_stat(&final_path, label)
                .map_err(|error| {
                    app_data_private_append_outcome_unknown(format!(
                        "{label} append final path entry is unsafe: {error}"
                    ))
                })?;
            if readback != expected_bytes || final_descriptor_stamp != observed {
                return Err(app_data_private_append_outcome_unknown(format!(
                    "{label} append semantic read-back or accepted descriptor stamp changed"
                )));
            }
            if final_path_stamp != observed {
                return Err(app_data_private_append_outcome_unknown(format!(
                    "{label} append final path binding changed"
                )));
            }
            fsync(&parent).map_err(|error| {
                app_data_private_append_partial_error(format!(
                    "{label} append file was synced but directory durability could not be verified: {}",
                    io::Error::from(error)
                ))
            })?;
            let durable_path =
                statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                    app_data_private_append_outcome_unknown(format!(
                        "{label} append was synced but its durable path binding could not be read: {}",
                        io::Error::from(error)
                    ))
                })?;
            if private_regular_file_stamp_from_stat(&durable_path, label).map_err(|error| {
                app_data_private_append_outcome_unknown(format!(
                    "{label} append durable path entry is unsafe: {error}"
                ))
            })? != observed
            {
                return Err(app_data_private_append_outcome_unknown(format!(
                    "{label} append path binding changed after directory sync"
                )));
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let target = guarded_fallback_path(self.lock.owner_path(), relative)?;
            let existed = match std::fs::symlink_metadata(&target) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                    return Err(unsafe_relative_file(label))
                }
                Ok(_) => true,
                Err(error) if error.kind() == io::ErrorKind::NotFound => false,
                Err(error) => return Err(error.into()),
            };
            let mut options = std::fs::OpenOptions::new();
            options.append(true);
            if existed {
                options.write(true);
            } else {
                options.create_new(true);
            }
            let mut file = options.open(&target)?;
            let metadata = file.metadata()?;
            let expected_len = metadata
                .len()
                .checked_add(appended_len)
                .filter(|len| *len <= max_result_bytes)
                .ok_or_else(|| {
                    CommandError::UnsafeConfigPath(format!(
                        "{label} exceeds its append safety bound"
                    ))
                })?;
            file.write_all(bytes)?;
            file.sync_all()?;
            if file.metadata()?.len() != expected_len {
                return Err(CommandError::StaleActionReference);
            }
            File::open(target.parent().ok_or_else(unsafe_relative_path)?)?.sync_all()?;
            Ok(())
        }
    }

    /// Inspect one legacy private-content leaf without following symbolic links.
    ///
    /// This cleanup-only projection deliberately accepts an owner-owned regular
    /// file whose historical mode is broader than the current private-file
    /// contract so the user can explicitly remove that unsafe legacy content.
    /// Hard links, directories, devices, sockets, and foreign-owned leaves are
    /// rejected. Regular-file content is read in full within `max_bytes` and is
    /// included in the opaque revision; it is never returned by service RPCs.
    pub fn inspect_private_cleanup_leaf(
        &self,
        relative: &Path,
        max_bytes: u64,
        label: &str,
    ) -> Result<Option<AppDataPrivateLeafSnapshot>, CommandError> {
        validate_relative_path(relative)?;
        #[cfg(unix)]
        {
            let (parent, name) = self.open_private_parent(relative)?;
            inspect_private_cleanup_leaf_at(self.lock, &parent, name, max_bytes, label)
        }
        #[cfg(not(unix))]
        {
            let target = guarded_fallback_path(self.lock.owner_path(), relative)?;
            inspect_private_cleanup_leaf_fallback(&target, max_bytes, label)
        }
    }

    fn create_private_cleanup_file_noreplace(
        &self,
        relative: &Path,
        bytes: &[u8],
        temp_stem: &str,
    ) -> Result<AppDataPrivateLeafSnapshot, CommandError> {
        validate_relative_path(relative)?;
        validate_single_component(OsStr::new(temp_stem))?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            use rustix::fs::{fchmod, fsync, openat, statat, AtFlags, FileType, Mode, OFlags};

            let (parent, name) = self.open_private_parent(relative)?;
            let owner_uid = private_owner_uid(self.lock)?;
            let mode = Mode::from_bits_truncate(0o600);
            for attempt in 0..32_u32 {
                let token = random_cleanup_token()?;
                let temp_name = OsString::from(format!(".{temp_stem}.{token}.{attempt}.tmp"));
                let descriptor = match openat(
                    &parent,
                    &temp_name,
                    OFlags::WRONLY
                        | OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    mode,
                ) {
                    Ok(descriptor) => descriptor,
                    Err(rustix::io::Errno::EXIST) => continue,
                    Err(error) => return Err(io::Error::from(error).into()),
                };
                let mut file = File::from(descriptor);
                let before_activation = (|| {
                    let metadata = file.metadata()?;
                    let path_metadata = statat(&parent, &temp_name, AtFlags::SYMLINK_NOFOLLOW)
                        .map_err(io::Error::from)?;
                    if !metadata.is_file()
                        || metadata.uid() != owner_uid
                        || metadata.nlink() != 1
                        || FileType::from_raw_mode(path_metadata.st_mode) != FileType::RegularFile
                        || path_metadata.st_dev as u64 != metadata.dev()
                        || path_metadata.st_ino != metadata.ino()
                        || path_metadata.st_uid != owner_uid
                        || path_metadata.st_nlink != 1
                    {
                        return Err(unsafe_relative_file("app-data private candidate"));
                    }
                    fchmod(&file, mode).map_err(io::Error::from)?;
                    file.write_all(bytes)?;
                    file.sync_all()?;
                    let descriptor_stamp = private_regular_file_stamp_from_metadata(
                        &file.metadata()?,
                        "app-data private candidate",
                    )?;
                    let path_stamp = private_regular_file_stamp_from_stat(
                        &statat(&parent, &temp_name, AtFlags::SYMLINK_NOFOLLOW)
                            .map_err(io::Error::from)?,
                        "app-data private candidate",
                    )?;
                    if descriptor_stamp != path_stamp
                        || descriptor_stamp.uid != owner_uid
                        || descriptor_stamp.link_count != 1
                        || descriptor_stamp.mode & 0o077 != 0
                        || descriptor_stamp.size != bytes.len() as u64
                    {
                        return Err(CommandError::StaleActionReference);
                    }
                    Ok::<(), CommandError>(())
                })();
                if let Err(error) = before_activation {
                    cleanup_unactivated_private_candidate(&parent, &temp_name, &file, error)?;
                    unreachable!("candidate cleanup returns the original error")
                }
                if let Err(error) = rename_noreplace(&parent, &temp_name, name) {
                    cleanup_unactivated_private_candidate(
                        &parent,
                        &temp_name,
                        &file,
                        CommandError::from(error),
                    )?;
                    unreachable!("candidate cleanup returns the original error")
                }
                if let Err(error) = fsync(&parent).map_err(io::Error::from) {
                    return Err(app_data_private_write_partial_error(format!(
                        "private candidate was activated but directory durability could not be verified: {error}"
                    )));
                }
                run_private_write_post_activation_hook();
                let installed_descriptor =
                    private_regular_file_stamp_from_metadata(
                        &file.metadata().map_err(|error| {
                            app_data_private_write_partial_error(format!(
                                "private candidate was activated, but its descriptor could not be inspected: {error}"
                            ))
                        })?,
                        "activated app-data private candidate",
                    )
                    .map_err(|error| {
                        app_data_private_write_outcome_unknown(format!(
                            "private candidate was activated, but its descriptor is unsafe: {error}"
                        ))
                    })?;
                let snapshot = self
                    .read_bounded_regular_file_snapshot(
                        relative,
                        bytes.len() as u64,
                        "activated app-data private candidate",
                    )
                    .map_err(classify_app_data_private_write_readback_error)?
                    .ok_or_else(|| {
                        app_data_private_write_outcome_unknown(
                            "private candidate disappeared after activation".to_string(),
                        )
                    })?;
                if snapshot.identity != installed_descriptor.private_identity()
                    || snapshot.regular_file_bytes() != Some(bytes)
                {
                    return Err(app_data_private_write_outcome_unknown(
                        "private candidate identity or bytes changed after activation".to_string(),
                    ));
                }
                return Ok(snapshot);
            }
            Err(CommandError::UnsafeConfigPath(
                "legacy private-content candidate allocation was exhausted".to_string(),
            ))
        }
        #[cfg(not(unix))]
        {
            let target = guarded_fallback_path(self.lock.owner_path(), relative)?;
            let parent = target.parent().ok_or_else(unsafe_relative_path)?;
            let temp = parent.join(format!(
                ".{temp_stem}.{}.{}.tmp",
                std::process::id(),
                owner_fs_timestamp_millis()
            ));
            let result = (|| {
                let mut file = std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&temp)?;
                file.write_all(bytes)?;
                file.sync_all()?;
                if target.exists() {
                    return Err(io::Error::new(
                        io::ErrorKind::AlreadyExists,
                        "legacy private-content target was recreated",
                    )
                    .into());
                }
                std::fs::rename(&temp, &target)?;
                File::open(parent)?.sync_all()?;
                Ok::<(), CommandError>(())
            })();
            if result.is_err() {
                let _ = std::fs::remove_file(temp);
            }
            result?;
            let snapshot = inspect_private_cleanup_leaf_fallback(
                &target,
                bytes.len() as u64,
                "activated app-data private candidate",
            )?
            .ok_or(CommandError::StaleActionReference)?;
            if snapshot.regular_file_bytes() != Some(bytes) {
                return Err(app_data_private_write_outcome_unknown(
                    "private candidate bytes changed after activation".to_string(),
                ));
            }
            Ok(snapshot)
        }
    }

    /// Replace a private file only if the accepted leaf is still current.
    ///
    /// A missing accepted leaf activates the candidate with `NOREPLACE`. A
    /// present accepted leaf is identity/content-bound, quarantined with
    /// `NOREPLACE`, and only then followed by a `NOREPLACE` candidate
    /// activation. Thus an uncoordinated same-UID writer is preserved instead
    /// of being overwritten.
    pub fn replace_private_file_if_current(
        &self,
        relative: &Path,
        expected: Option<&AppDataPrivateLeafSnapshot>,
        bytes: &[u8],
        temp_stem: &str,
        quarantine_prefix: &str,
    ) -> Result<(), CommandError> {
        self.replace_private_file_if_current_with_snapshot(
            relative,
            expected,
            bytes,
            temp_stem,
            quarantine_prefix,
        )
        .map(|_| ())
    }

    pub fn replace_private_file_if_current_with_snapshot(
        &self,
        relative: &Path,
        expected: Option<&AppDataPrivateLeafSnapshot>,
        bytes: &[u8],
        temp_stem: &str,
        quarantine_prefix: &str,
    ) -> Result<AppDataPrivateLeafSnapshot, CommandError> {
        match expected {
            Some(expected) => self.replace_private_cleanup_regular_leaf_bound(
                relative,
                expected,
                bytes,
                temp_stem,
                quarantine_prefix,
            ),
            None => self.create_private_cleanup_file_noreplace(relative, bytes, temp_stem),
        }
    }

    /// Enumerate cleanup quarantine leaves created by a prior interrupted
    /// privacy action. Every returned leaf is independently bounded and bound
    /// to its no-follow identity and complete regular-file bytes.
    pub fn list_root_private_cleanup_leaves_matching(
        &self,
        prefix: &str,
        suffix: &str,
        max_matches: usize,
        max_bytes: u64,
        label: &str,
    ) -> Result<Vec<(OsString, AppDataPrivateLeafSnapshot)>, CommandError> {
        validate_filename_fragment(prefix)?;
        validate_filename_fragment(suffix)?;
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;

            use rustix::fs::Dir;

            let directory = self.lock.open_owner_directory()?;
            let mut entries = Dir::read_from(&directory).map_err(io::Error::from)?;
            let mut matches = Vec::new();
            for entry in &mut entries {
                let entry = entry.map_err(io::Error::from)?;
                let bytes = entry.file_name().to_bytes();
                if matches!(bytes, b"." | b"..") {
                    continue;
                }
                let name = OsStr::from_bytes(bytes);
                let text = name.to_string_lossy();
                if !text.starts_with(prefix) || !text.ends_with(suffix) {
                    continue;
                }
                if matches.len() >= max_matches {
                    return Err(CommandError::UnsafeConfigPath(
                        "app-data private cleanup residue exceeds its enumeration safety bound"
                            .to_string(),
                    ));
                }
                let snapshot =
                    inspect_private_cleanup_leaf_at(self.lock, &directory, name, max_bytes, label)?
                        .ok_or(CommandError::StaleActionReference)?;
                matches.push((name.to_owned(), snapshot));
            }
            matches.sort_by(|left, right| left.0.cmp(&right.0));
            Ok(matches)
        }
        #[cfg(not(unix))]
        {
            let mut matches = Vec::new();
            for entry in std::fs::read_dir(self.lock.owner_path())? {
                let entry = entry?;
                let name = entry.file_name();
                let text = name.to_string_lossy();
                if !text.starts_with(prefix) || !text.ends_with(suffix) {
                    continue;
                }
                if matches.len() >= max_matches {
                    return Err(CommandError::UnsafeConfigPath(
                        "app-data private cleanup residue exceeds its enumeration safety bound"
                            .to_string(),
                    ));
                }
                let snapshot =
                    inspect_private_cleanup_leaf_fallback(&entry.path(), max_bytes, label)?
                        .ok_or(CommandError::StaleActionReference)?;
                matches.push((name, snapshot));
            }
            matches.sort_by(|left, right| left.0.cmp(&right.0));
            Ok(matches)
        }
    }

    /// Remove exactly the regular file or symbolic link represented by
    /// `expected`. The leaf is moved to a random no-replace quarantine name,
    /// re-verified there, then unlinked and directory-synced. A symbolic link is
    /// removed as a link and is never followed.
    pub fn remove_private_cleanup_leaf(
        &self,
        relative: &Path,
        expected: &AppDataPrivateLeafSnapshot,
        quarantine_prefix: &str,
    ) -> Result<(), CommandError> {
        validate_relative_path(relative)?;
        validate_filename_fragment(quarantine_prefix)?;
        #[cfg(unix)]
        {
            use rustix::fs::{fsync, statat, unlinkat, AtFlags};

            let (parent, name) = self.open_private_parent(relative)?;
            let (quarantine, quarantined_snapshot) = quarantine_bound_private_leaf(
                self.lock,
                &parent,
                name,
                expected,
                quarantine_prefix,
            )?;
            run_private_cleanup_post_quarantine_hook(name);
            match statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW) {
                Err(rustix::io::Errno::NOENT) => {}
                Ok(_) => {
                    return Err(private_cleanup_outcome_unknown(
                        "legacy private content was quarantined, but a third-party replacement now occupies its original name"
                            .to_string(),
                    ))
                }
                Err(error) => {
                    return Err(private_cleanup_outcome_unknown(format!(
                        "legacy private content was quarantined, but absence of its original name could not be verified: {}",
                        io::Error::from(error)
                    )))
                }
            }
            require_bound_private_leaf_at(
                self.lock,
                &parent,
                &quarantine,
                &quarantined_snapshot,
                "quarantined legacy private content before removal",
            )?;
            unlinkat(&parent, &quarantine, AtFlags::empty()).map_err(|error| {
                private_cleanup_partial_error(format!(
                    "cannot remove quarantined legacy private content: {}",
                    io::Error::from(error)
                ))
            })?;
            fsync(&parent).map_err(|error| {
                private_cleanup_partial_error(format!(
                    "legacy private-content removal could not be durably verified: {}",
                    io::Error::from(error)
                ))
            })?;
            match statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW) {
                Err(rustix::io::Errno::NOENT) => {}
                Ok(_) => {
                    return Err(private_cleanup_outcome_unknown(
                        "legacy private content was removed, but a third-party replacement appeared at its original name"
                            .to_string(),
                    ))
                }
                Err(error) => {
                    return Err(private_cleanup_outcome_unknown(format!(
                        "legacy private-content removal completed, but final target absence could not be verified: {}",
                        io::Error::from(error)
                    )))
                }
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let target = guarded_fallback_path(self.lock.owner_path(), relative)?;
            let current = inspect_private_cleanup_leaf_fallback(
                &target,
                expected.identity.size,
                "legacy private content",
            )?
            .ok_or(CommandError::StaleActionReference)?;
            if &current != expected {
                return Err(CommandError::StaleActionReference);
            }
            std::fs::remove_file(&target)?;
            File::open(target.parent().ok_or_else(unsafe_relative_path)?)?.sync_all()?;
            Ok(())
        }
    }

    /// Replace one identity-bound legacy regular file with canonical private
    /// bytes. The original is quarantined and fully re-verified before the
    /// replacement is created. Any uncertainty after the quarantine move is a
    /// typed partial effect and the original quarantine is retained for a
    /// later explicit cleanup.
    pub fn replace_private_cleanup_regular_leaf(
        &self,
        relative: &Path,
        expected: &AppDataPrivateLeafSnapshot,
        candidate: &[u8],
        temp_stem: &str,
        quarantine_prefix: &str,
    ) -> Result<(), CommandError> {
        self.replace_private_cleanup_regular_leaf_bound(
            relative,
            expected,
            candidate,
            temp_stem,
            quarantine_prefix,
        )
        .map(|_| ())
    }

    fn replace_private_cleanup_regular_leaf_bound(
        &self,
        relative: &Path,
        expected: &AppDataPrivateLeafSnapshot,
        candidate: &[u8],
        temp_stem: &str,
        quarantine_prefix: &str,
    ) -> Result<AppDataPrivateLeafSnapshot, CommandError> {
        validate_relative_path(relative)?;
        validate_single_component(OsStr::new(temp_stem))?;
        validate_filename_fragment(quarantine_prefix)?;
        if expected.kind != AppDataPrivateLeafKind::RegularFile {
            return Err(CommandError::StaleActionReference);
        }
        #[cfg(unix)]
        {
            use rustix::fs::{fsync, statat, unlinkat, AtFlags};

            let (parent, name) = self.open_private_parent(relative)?;
            let (quarantine, quarantined_snapshot) = quarantine_bound_private_leaf(
                self.lock,
                &parent,
                name,
                expected,
                quarantine_prefix,
            )?;
            run_private_cleanup_post_quarantine_hook(name);
            let candidate_snapshot = match self
                .create_private_cleanup_file_noreplace(relative, candidate, temp_stem)
            {
                Ok(snapshot) => snapshot,
                Err(error) => match statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW) {
                    Err(rustix::io::Errno::NOENT) => {
                        restore_quarantined_private_leaf_bound(
                            self.lock,
                            &parent,
                            &quarantine,
                            name,
                            &quarantined_snapshot,
                        )?;
                        return Err(error);
                    }
                    _ => {
                        return Err(private_cleanup_outcome_unknown(format!(
                            "legacy private content was quarantined but its canonical replacement could not be durably verified: {error}"
                        )));
                    }
                },
            };
            require_bound_private_leaf_at(
                self.lock,
                &parent,
                &quarantine,
                &quarantined_snapshot,
                "bound private-file original before cleanup",
            )?;
            unlinkat(&parent, &quarantine, AtFlags::empty()).map_err(|error| {
                private_cleanup_partial_error(format!(
                    "canonical legacy cleanup succeeded but its sensitive quarantine could not be removed: {}",
                    io::Error::from(error)
                ))
            })?;
            fsync(&parent).map_err(|error| {
                private_cleanup_partial_error(format!(
                    "canonical legacy cleanup could not be durably verified: {}",
                    io::Error::from(error)
                ))
            })?;
            Ok(candidate_snapshot)
        }
        #[cfg(not(unix))]
        {
            let target = guarded_fallback_path(self.lock.owner_path(), relative)?;
            let current = inspect_private_cleanup_leaf_fallback(
                &target,
                expected.identity.size,
                "legacy private content",
            )?
            .ok_or(CommandError::StaleActionReference)?;
            if &current != expected {
                return Err(CommandError::StaleActionReference);
            }
            let parent = target.parent().ok_or_else(unsafe_relative_path)?;
            let temp = parent.join(secure_owner_temp_name(temp_stem)?);
            let quarantine = parent.join(format!(
                "{quarantine_prefix}{}.quarantine",
                owner_fs_timestamp_millis()
            ));
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp)?;
            file.write_all(candidate)?;
            file.sync_all()?;
            std::fs::rename(&target, &quarantine)?;
            if target.exists() {
                return Err(app_data_private_write_outcome_unknown(
                    "the accepted private file was quarantined, but a third state appeared at its original name"
                        .to_string(),
                ));
            }
            std::fs::rename(&temp, &target).map_err(|error| {
                app_data_private_write_partial_error(format!(
                    "the accepted private file was quarantined, but candidate activation failed: {error}"
                ))
            })?;
            File::open(parent)?.sync_all().map_err(|error| {
                app_data_private_write_partial_error(format!(
                    "private replacement directory durability failed: {error}"
                ))
            })?;
            std::fs::remove_file(&quarantine).map_err(|error| {
                app_data_private_write_partial_error(format!(
                    "private replacement succeeded, but its original quarantine could not be removed: {error}"
                ))
            })?;
            File::open(parent)?.sync_all().map_err(|error| {
                app_data_private_write_partial_error(format!(
                    "private original cleanup durability failed: {error}"
                ))
            })?;
            inspect_private_cleanup_leaf_fallback(
                &target,
                candidate.len() as u64,
                "activated app-data private candidate",
            )?
            .ok_or(CommandError::StaleActionReference)
        }
    }

    pub fn remove_regular_files_matching(
        &self,
        relative_directory: &Path,
        prefix: &str,
        suffix: &str,
        max_matches: usize,
    ) -> Result<(), CommandError> {
        validate_relative_path(relative_directory)?;
        self.remove_regular_files_matching_in_directory(
            Some(relative_directory),
            prefix,
            suffix,
            max_matches,
        )
    }

    fn remove_regular_files_matching_in_directory(
        &self,
        relative_directory: Option<&Path>,
        prefix: &str,
        suffix: &str,
        max_matches: usize,
    ) -> Result<(), CommandError> {
        validate_filename_fragment(prefix)?;
        validate_filename_fragment(suffix)?;
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;

            use rustix::fs::{statat, unlinkat, AtFlags, Dir, FileType};

            let directory = match relative_directory {
                Some(relative) => self.open_private_directory(relative)?,
                None => self.lock.open_owner_directory()?,
            };
            let owner_uid = private_owner_uid(self.lock)?;
            let mut entries = Dir::read_from(&directory).map_err(io::Error::from)?;
            let mut matches = Vec::new();
            for entry in &mut entries {
                let entry = entry.map_err(io::Error::from)?;
                let bytes = entry.file_name().to_bytes();
                if matches!(bytes, b"." | b"..") {
                    continue;
                }
                let name = OsStr::from_bytes(bytes);
                let text = name.to_string_lossy();
                if !text.starts_with(prefix) || !text.ends_with(suffix) {
                    continue;
                }
                if matches.len() >= max_matches {
                    return Err(CommandError::UnsafeConfigPath(
                        "app-data private residue exceeds its cleanup safety bound".to_string(),
                    ));
                }
                let metadata =
                    statat(&directory, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
                if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile
                    || metadata.st_uid != owner_uid
                    || metadata.st_nlink != 1
                    || metadata.st_mode & 0o077 != 0
                {
                    return Err(CommandError::UnsafeConfigPath(
                        "app-data private residue is not an owner-only regular file".to_string(),
                    ));
                }
                matches.push((
                    name.to_owned(),
                    private_regular_file_stamp_from_stat(&metadata, "app-data private residue")?,
                ));
            }
            matches.sort_by(|left, right| left.0.cmp(&right.0));
            let mut removed_any = false;
            for (name, expected) in matches {
                let current =
                    statat(&directory, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                        cleanup_identity_error(removed_any, io::Error::from(error).to_string())
                    })?;
                if private_regular_file_stamp_from_stat(&current, "app-data private residue")
                    .map_err(|error| cleanup_identity_error(removed_any, error.to_string()))?
                    != expected
                {
                    return Err(cleanup_identity_error(
                        removed_any,
                        "app-data private residue changed before removal".to_string(),
                    ));
                }
                run_remove_matching_before_quarantine_hook(&name);
                let before_quarantine = statat(&directory, &name, AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(|error| {
                        cleanup_identity_error(removed_any, io::Error::from(error).to_string())
                    })?;
                if private_regular_file_stamp_from_stat(
                    &before_quarantine,
                    "app-data private residue",
                )
                .map_err(|error| cleanup_identity_error(removed_any, error.to_string()))?
                    != expected
                {
                    return Err(cleanup_identity_error(
                        removed_any,
                        "app-data private residue changed at the quarantine boundary".to_string(),
                    ));
                }
                let quarantine =
                    quarantine_residue(&directory, &name, prefix, suffix, removed_any)?;
                let quarantined = statat(&directory, &quarantine, AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(|error| {
                        cleanup_outcome_unknown(format!(
                            "cannot verify quarantined app-data private residue: {}",
                            io::Error::from(error)
                        ))
                    })?;
                let quarantined_stamp = private_regular_file_stamp_from_stat(
                    &quarantined,
                    "quarantined app-data private residue",
                )
                .map_err(|error| cleanup_outcome_unknown(error.to_string()))?;
                if !private_stamp_matches_after_rename(expected, quarantined_stamp) {
                    return Err(cleanup_outcome_unknown(
                        "app-data private residue changed before its identity-bound quarantine; the third state was retained"
                            .to_string(),
                    ));
                }
                sync_cleanup_directory(&directory).map_err(|error| {
                    cleanup_partial_error(format!(
                        "app-data private residue quarantine durability could not be verified: {error}"
                    ))
                })?;
                let before_unlink = statat(&directory, &quarantine, AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(|error| {
                        cleanup_outcome_unknown(format!(
                            "cannot revalidate quarantined app-data private residue: {}",
                            io::Error::from(error)
                        ))
                    })?;
                if private_regular_file_stamp_from_stat(
                    &before_unlink,
                    "quarantined app-data private residue",
                )
                .map_err(|error| cleanup_outcome_unknown(error.to_string()))?
                    != quarantined_stamp
                {
                    return Err(cleanup_outcome_unknown(
                        "quarantined app-data private residue changed before unlink; the replacement was retained"
                            .to_string(),
                    ));
                }
                unlinkat(&directory, &quarantine, AtFlags::empty()).map_err(|error| {
                    cleanup_partial_error(format!(
                        "cannot remove quarantined app-data private residue: {}",
                        io::Error::from(error)
                    ))
                })?;
                removed_any = true;
                sync_cleanup_directory(&directory).map_err(|error| {
                    cleanup_partial_error(format!(
                        "app-data private residue removal could not be durably verified: {error}"
                    ))
                })?;
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let root = match relative_directory {
                Some(relative) => guarded_fallback_path(self.lock.owner_path(), relative)?,
                None => self.lock.owner_path().to_path_buf(),
            };
            let metadata = std::fs::symlink_metadata(&root)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(unsafe_relative_path());
            }
            let mut matches = Vec::new();
            for entry in std::fs::read_dir(&root)? {
                let entry = entry?;
                let text = entry.file_name().to_string_lossy().to_string();
                if text.starts_with(prefix) && text.ends_with(suffix) {
                    if matches.len() >= max_matches {
                        return Err(CommandError::UnsafeConfigPath(
                            "app-data private residue exceeds its cleanup safety bound".to_string(),
                        ));
                    }
                    let metadata = std::fs::symlink_metadata(entry.path())?;
                    if metadata.file_type().is_symlink() || !metadata.is_file() {
                        return Err(CommandError::UnsafeConfigPath(
                            "app-data private residue is not a regular file".to_string(),
                        ));
                    }
                    matches.push(entry.path());
                }
            }
            for path in matches {
                std::fs::remove_file(path)?;
            }
            File::open(root)?.sync_all()?;
            Ok(())
        }
    }

    pub fn ensure_directory_all(
        &self,
        relative: &Path,
    ) -> Result<Vec<AppDataCreatedDirectory>, CommandError> {
        validate_relative_path(relative)?;
        #[cfg(unix)]
        {
            use rustix::fs::{fchmod, fsync, mkdirat, openat, statat, AtFlags, FileType, Mode};

            let flags = directory_flags();
            let mode = Mode::from_bits_truncate(0o700);
            let mut current = openat(self.lock.owner_directory(), ".", flags, Mode::empty())
                .map_err(io::Error::from)?;
            let owner_uid = private_owner_uid(self.lock)?;
            let mut created = Vec::new();
            let mut accumulated = PathBuf::new();
            for component in relative.components() {
                let Component::Normal(name) = component else {
                    return Err(classify_private_directory_creation_error(
                        !created.is_empty(),
                        unsafe_relative_path(),
                    ));
                };
                accumulated.push(name);
                match openat(&current, name, flags, Mode::empty()) {
                    Ok(next) => {
                        validate_private_directory(&next, owner_uid).map_err(|error| {
                            classify_private_directory_creation_error(!created.is_empty(), error)
                        })?;
                        current = next;
                    }
                    Err(rustix::io::Errno::NOENT) => {
                        let created_now = match mkdirat(&current, name, mode) {
                            Ok(()) => true,
                            Err(rustix::io::Errno::EXIST) => false,
                            Err(error) => {
                                return Err(classify_private_directory_creation_error(
                                    !created.is_empty(),
                                    io::Error::from(error).into(),
                                ))
                            }
                        };
                        let created_stat = if created_now {
                            let stat = statat(&current, name, AtFlags::SYMLINK_NOFOLLOW).map_err(
                                |error| {
                                    classify_private_directory_creation_error(
                                        true,
                                        io::Error::from(error).into(),
                                    )
                                },
                            )?;
                            if FileType::from_raw_mode(stat.st_mode) != FileType::Directory
                                || stat.st_uid != owner_uid
                            {
                                return Err(classify_private_directory_creation_error(
                                    true,
                                    CommandError::UnsafeConfigPath(
                                        "new app-data directory is not owned by the current effective user"
                                            .to_string(),
                                    ),
                                ));
                            }
                            Some(stat)
                        } else {
                            None
                        };
                        let next = openat(&current, name, flags, Mode::empty())
                            .map_err(map_unsafe_relative_errno)
                            .map_err(|error| {
                                classify_private_directory_creation_error(
                                    !created.is_empty(),
                                    error,
                                )
                            })?;
                        if let Some(created_stat) = created_stat {
                            let opened = rustix::fs::fstat(&next).map_err(|error| {
                                classify_private_directory_creation_error(
                                    true,
                                    io::Error::from(error).into(),
                                )
                            })?;
                            if opened.st_dev != created_stat.st_dev
                                || opened.st_ino != created_stat.st_ino
                                || opened.st_uid != owner_uid
                            {
                                return Err(classify_private_directory_creation_error(
                                    true,
                                    CommandError::StaleActionReference,
                                ));
                            }
                            fchmod(&next, mode)
                                .map_err(io::Error::from)
                                .map_err(CommandError::from)
                                .map_err(|error| {
                                    classify_private_directory_creation_error(true, error)
                                })?;
                            validate_private_directory(&next, owner_uid).map_err(|error| {
                                classify_private_directory_creation_error(true, error)
                            })?;
                            fsync(&next)
                                .map_err(io::Error::from)
                                .and_then(|()| sync_created_private_directory_parent(&current))
                                .map_err(CommandError::from)
                                .map_err(|error| {
                                    classify_private_directory_creation_error(true, error)
                                })?;
                            let rebound = statat(&current, name, AtFlags::SYMLINK_NOFOLLOW)
                                .map_err(|error| {
                                    classify_private_directory_creation_error(
                                        true,
                                        io::Error::from(error).into(),
                                    )
                                })?;
                            if rebound.st_dev != created_stat.st_dev
                                || rebound.st_ino != created_stat.st_ino
                                || rebound.st_uid != owner_uid
                                || FileType::from_raw_mode(rebound.st_mode) != FileType::Directory
                                || rebound.st_mode & 0o077 != 0
                            {
                                return Err(classify_private_directory_creation_error(
                                    true,
                                    CommandError::StaleActionReference,
                                ));
                            }
                            created.push(AppDataCreatedDirectory {
                                relative: accumulated.clone(),
                                device: rebound.st_dev as u64,
                                inode: rebound.st_ino,
                                uid: rebound.st_uid,
                                mode: rebound.st_mode as u32,
                            });
                        } else {
                            validate_private_directory(&next, owner_uid).map_err(|error| {
                                classify_private_directory_creation_error(
                                    !created.is_empty(),
                                    error,
                                )
                            })?;
                        }
                        current = next;
                    }
                    Err(error) => {
                        return Err(classify_private_directory_creation_error(
                            !created.is_empty(),
                            map_unsafe_relative_errno(error),
                        ))
                    }
                }
            }
            Ok(created)
        }
        #[cfg(not(unix))]
        {
            let mut created = Vec::new();
            let mut accumulated = PathBuf::new();
            for component in relative.components() {
                let Component::Normal(name) = component else {
                    return Err(unsafe_relative_path());
                };
                accumulated.push(name);
                let path = guarded_fallback_path(self.lock.owner_path(), &accumulated)?;
                match std::fs::symlink_metadata(&path) {
                    Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                        return Err(unsafe_relative_path())
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {
                        std::fs::create_dir(&path)?;
                        created.push(AppDataCreatedDirectory {
                            relative: accumulated.clone(),
                        });
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            Ok(created)
        }
    }

    #[cfg(unix)]
    fn open_regular_file(
        &self,
        relative: &Path,
        write: bool,
        require_private_parent: bool,
    ) -> io::Result<File> {
        use rustix::fs::{openat, Mode, OFlags};

        let (parent, name) = if require_private_parent {
            self.open_private_parent(relative)
        } else {
            self.open_parent(relative, false)
        }
        .map_err(command_error_to_io)?;
        let mut flags = OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC;
        flags |= if write {
            OFlags::WRONLY
        } else {
            OFlags::RDONLY
        };
        openat(&parent, name, flags, Mode::empty())
            .map(File::from)
            .map_err(io::Error::from)
    }

    #[cfg(not(unix))]
    fn open_regular_file(
        &self,
        relative: &Path,
        _write: bool,
        _require_private_parent: bool,
    ) -> io::Result<File> {
        let path =
            guarded_fallback_path(self.lock.owner_path(), relative).map_err(command_error_to_io)?;
        let metadata = std::fs::symlink_metadata(&path)?;
        if metadata.file_type().is_symlink() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "app-data relative file is unsafe",
            ));
        }
        File::open(path)
    }

    #[cfg(unix)]
    fn open_directory(&self, relative: &Path) -> Result<File, CommandError> {
        use rustix::fs::{openat, Mode};

        let mut current = openat(
            self.lock.owner_directory(),
            ".",
            directory_flags(),
            Mode::empty(),
        )
        .map_err(io::Error::from)?;
        for component in relative.components() {
            let Component::Normal(name) = component else {
                return Err(unsafe_relative_path());
            };
            current = openat(&current, name, directory_flags(), Mode::empty())
                .map_err(map_unsafe_relative_errno)?;
        }
        Ok(File::from(current))
    }

    #[cfg(unix)]
    fn open_private_directory(&self, relative: &Path) -> Result<File, CommandError> {
        use rustix::fs::{openat, Mode};

        let mut current = openat(
            self.lock.owner_directory(),
            ".",
            directory_flags(),
            Mode::empty(),
        )
        .map_err(io::Error::from)?;
        let owner_uid = private_owner_uid(self.lock)?;
        for component in relative.components() {
            let Component::Normal(name) = component else {
                return Err(unsafe_relative_path());
            };
            current = openat(&current, name, directory_flags(), Mode::empty())
                .map_err(map_unsafe_relative_errno)?;
            validate_private_directory(&current, owner_uid)?;
        }
        Ok(File::from(current))
    }

    #[cfg(unix)]
    fn open_parent<'path>(
        &self,
        relative: &'path Path,
        create: bool,
    ) -> Result<(File, &'path OsStr), CommandError> {
        let name = relative.file_name().ok_or_else(unsafe_relative_path)?;
        let parent = relative.parent().unwrap_or_else(|| Path::new(""));
        if create && !parent.as_os_str().is_empty() {
            self.ensure_directory_all(parent)?;
        }
        let directory = if parent.as_os_str().is_empty() {
            self.lock.open_owner_directory()?
        } else {
            self.open_directory(parent)?
        };
        Ok((directory, name))
    }

    #[cfg(unix)]
    fn open_private_parent<'path>(
        &self,
        relative: &'path Path,
    ) -> Result<(File, &'path OsStr), CommandError> {
        let name = relative.file_name().ok_or_else(unsafe_relative_path)?;
        let parent = relative.parent().unwrap_or_else(|| Path::new(""));
        let directory = if parent.as_os_str().is_empty() {
            self.lock.open_owner_directory()?
        } else {
            self.open_private_directory(parent)?
        };
        Ok((directory, name))
    }
}

#[cfg(unix)]
fn private_regular_file_stamp_from_metadata(
    metadata: &std::fs::Metadata,
    _label: &str,
) -> Result<PrivateRegularFileStamp, CommandError> {
    use std::os::unix::fs::MetadataExt;

    Ok(PrivateRegularFileStamp {
        device: metadata.dev(),
        inode: metadata.ino(),
        uid: metadata.uid(),
        link_count: metadata.nlink(),
        mode: metadata.mode(),
        size: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    })
}

#[cfg(unix)]
fn private_regular_file_stamp_from_stat(
    metadata: &rustix::fs::Stat,
    label: &str,
) -> Result<PrivateRegularFileStamp, CommandError> {
    Ok(PrivateRegularFileStamp {
        device: u64::try_from(metadata.st_dev).map_err(|_| unsafe_relative_file(label))?,
        inode: metadata.st_ino,
        uid: metadata.st_uid,
        link_count: u64::from(metadata.st_nlink),
        mode: u32::from(metadata.st_mode),
        size: u64::try_from(metadata.st_size).map_err(|_| unsafe_relative_file(label))?,
        modified_seconds: metadata.st_mtime,
        modified_nanoseconds: unix_timestamp_nanoseconds(metadata.st_mtime_nsec),
        changed_seconds: metadata.st_ctime,
        changed_nanoseconds: unix_timestamp_nanoseconds(metadata.st_ctime_nsec),
    })
}

#[cfg(unix)]
fn inspect_private_cleanup_leaf_at(
    lock: &AppMutationLock,
    directory: &File,
    name: &OsStr,
    max_bytes: u64,
    label: &str,
) -> Result<Option<AppDataPrivateLeafSnapshot>, CommandError> {
    use rustix::fs::{fstat, openat, statat, AtFlags, FileType, Mode, OFlags};

    let metadata = match statat(directory, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(metadata) => metadata,
        Err(rustix::io::Errno::NOENT) => return Ok(None),
        Err(error) => return Err(io::Error::from(error).into()),
    };
    let owner_uid = private_owner_uid(lock)?;
    let kind = match FileType::from_raw_mode(metadata.st_mode) {
        FileType::RegularFile => AppDataPrivateLeafKind::RegularFile,
        FileType::Symlink => AppDataPrivateLeafKind::SymbolicLink,
        _ => return Err(unsafe_relative_file(label)),
    };
    if metadata.st_uid != owner_uid || metadata.st_nlink != 1 {
        return Err(unsafe_relative_file(label));
    }
    let size = u64::try_from(metadata.st_size).map_err(|_| unsafe_relative_file(label))?;
    if kind == AppDataPrivateLeafKind::RegularFile && size > max_bytes {
        return Err(unsafe_relative_file(label));
    }
    let metadata_stamp = if kind == AppDataPrivateLeafKind::RegularFile {
        Some(private_regular_file_stamp_from_stat(&metadata, label)?)
    } else {
        None
    };
    let identity = AppDataPrivateLeafIdentity {
        device: u64::try_from(metadata.st_dev).map_err(|_| unsafe_relative_file(label))?,
        inode: metadata.st_ino,
        uid: metadata.st_uid,
        link_count: u64::from(metadata.st_nlink),
        mode: u32::from(metadata.st_mode),
        size,
        modified_seconds: metadata.st_mtime,
        modified_nanoseconds: unix_timestamp_nanoseconds(metadata.st_mtime_nsec),
        changed_seconds: metadata.st_ctime,
        changed_nanoseconds: unix_timestamp_nanoseconds(metadata.st_ctime_nsec),
    };
    let bytes = if kind == AppDataPrivateLeafKind::RegularFile {
        let descriptor = openat(
            directory,
            name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(|error| map_relative_io(io::Error::from(error), label))?;
        let mut file = File::from(descriptor);
        let opened = fstat(&file).map_err(io::Error::from)?;
        let opened_stamp = private_regular_file_stamp_from_stat(&opened, label)?;
        if Some(opened_stamp) != metadata_stamp {
            return Err(CommandError::StaleActionReference);
        }
        let mut bytes = Vec::with_capacity(size as usize);
        Read::by_ref(&mut file)
            .take(max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > max_bytes || bytes.len() as u64 != size {
            return Err(CommandError::StaleActionReference);
        }
        let after = fstat(&file).map_err(io::Error::from)?;
        if private_regular_file_stamp_from_stat(&after, label)? != opened_stamp {
            return Err(CommandError::StaleActionReference);
        }
        Some(bytes)
    } else {
        None
    };
    let revision = private_cleanup_leaf_revision(kind, &identity, bytes.as_deref());
    Ok(Some(AppDataPrivateLeafSnapshot {
        kind,
        revision,
        bytes,
        identity,
    }))
}

#[cfg(not(unix))]
fn inspect_private_cleanup_leaf_fallback(
    target: &Path,
    max_bytes: u64,
    label: &str,
) -> Result<Option<AppDataPrivateLeafSnapshot>, CommandError> {
    let metadata = match std::fs::symlink_metadata(target) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(error) => return Err(error.into()),
    };
    let kind = if metadata.file_type().is_symlink() {
        AppDataPrivateLeafKind::SymbolicLink
    } else if metadata.is_file() {
        AppDataPrivateLeafKind::RegularFile
    } else {
        return Err(unsafe_relative_file(label));
    };
    if kind == AppDataPrivateLeafKind::RegularFile && metadata.len() > max_bytes {
        return Err(unsafe_relative_file(label));
    }
    let bytes = if kind == AppDataPrivateLeafKind::RegularFile {
        Some(std::fs::read(target)?)
    } else {
        None
    };
    let identity = AppDataPrivateLeafIdentity {
        device: 0,
        inode: 0,
        uid: 0,
        link_count: 1,
        mode: 0,
        size: metadata.len(),
        modified_seconds: 0,
        modified_nanoseconds: 0,
        changed_seconds: 0,
        changed_nanoseconds: 0,
    };
    let revision = private_cleanup_leaf_revision(kind, &identity, bytes.as_deref());
    Ok(Some(AppDataPrivateLeafSnapshot {
        kind,
        revision,
        bytes,
        identity,
    }))
}

fn private_cleanup_leaf_revision(
    kind: AppDataPrivateLeafKind,
    identity: &AppDataPrivateLeafIdentity,
    bytes: Option<&[u8]>,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(PRIVATE_CLEANUP_LEAF_REVISION_DOMAIN.as_bytes());
    hasher.update(kind.as_str().as_bytes());
    hasher.update(identity.device.to_be_bytes());
    hasher.update(identity.inode.to_be_bytes());
    hasher.update(identity.uid.to_be_bytes());
    hasher.update(identity.link_count.to_be_bytes());
    hasher.update(identity.mode.to_be_bytes());
    hasher.update(identity.size.to_be_bytes());
    hasher.update(identity.modified_seconds.to_be_bytes());
    hasher.update(identity.modified_nanoseconds.to_be_bytes());
    hasher.update(identity.changed_seconds.to_be_bytes());
    hasher.update(identity.changed_nanoseconds.to_be_bytes());
    match bytes {
        Some(bytes) => {
            hasher.update(b"content");
            hasher.update((bytes.len() as u64).to_be_bytes());
            hasher.update(bytes);
        }
        None => hasher.update(b"no-content"),
    }
    format!("sha256:{:x}", hasher.finalize())
}

#[cfg(unix)]
fn quarantine_bound_private_leaf(
    lock: &AppMutationLock,
    directory: &File,
    source: &OsStr,
    expected: &AppDataPrivateLeafSnapshot,
    quarantine_prefix: &str,
) -> Result<(OsString, AppDataPrivateLeafSnapshot), CommandError> {
    let max_bytes = expected
        .regular_file_bytes()
        .map(|bytes| bytes.len() as u64)
        .unwrap_or_default();
    for _ in 0..32 {
        let before = inspect_private_cleanup_leaf_at(
            lock,
            directory,
            source,
            max_bytes,
            "legacy private content before quarantine",
        )?
        .ok_or(CommandError::StaleActionReference)?;
        if &before != expected {
            return Err(CommandError::StaleActionReference);
        }
        let token = random_cleanup_token().map_err(|error| {
            io::Error::other(format!(
                "cannot allocate a legacy private-content quarantine name: {error}"
            ))
        })?;
        let quarantine = OsString::from(format!("{quarantine_prefix}{token}.quarantine"));
        match rename_noreplace(directory, source, &quarantine) {
            Ok(()) => {
                let moved = inspect_private_cleanup_leaf_at(
                    lock,
                    directory,
                    &quarantine,
                    max_bytes,
                    "quarantined legacy private content",
                );
                match moved {
                    Ok(Some(moved)) if private_leaf_matches_after_rename(&before, &moved) => {
                        sync_private_cleanup_directory(directory).map_err(|error| {
                            private_cleanup_partial_error(format!(
                                "the private leaf was quarantined, but directory durability could not be verified: {error}"
                            ))
                        })?;
                        return Ok((quarantine, moved));
                    }
                    Ok(_) | Err(_) => {
                        return Err(private_cleanup_outcome_unknown(
                            "a raced legacy private-content leaf no longer matches the accepted identity after quarantine; the third state was retained"
                                .to_string(),
                        ));
                    }
                }
            }
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {
                return Err(CommandError::StaleActionReference)
            }
            Err(error) => return Err(error.into()),
        }
    }
    Err(CommandError::UnsafeConfigPath(
        "legacy private-content quarantine allocation was exhausted".to_string(),
    ))
}

#[cfg(unix)]
fn require_bound_private_leaf_at(
    lock: &AppMutationLock,
    directory: &File,
    name: &OsStr,
    expected: &AppDataPrivateLeafSnapshot,
    label: &str,
) -> Result<(), CommandError> {
    let max_bytes = expected
        .regular_file_bytes()
        .map(|bytes| bytes.len() as u64)
        .unwrap_or_default();
    let current = inspect_private_cleanup_leaf_at(lock, directory, name, max_bytes, label)
        .map_err(|error| {
            private_cleanup_outcome_unknown(format!(
                "{label} could not be identity-checked: {error}"
            ))
        })?
        .ok_or_else(|| {
            private_cleanup_outcome_unknown(format!("{label} disappeared before removal"))
        })?;
    if &current != expected {
        return Err(private_cleanup_outcome_unknown(format!(
            "{label} changed; the replacement was retained"
        )));
    }
    Ok(())
}

#[cfg(unix)]
fn private_leaf_matches_after_rename(
    before: &AppDataPrivateLeafSnapshot,
    moved: &AppDataPrivateLeafSnapshot,
) -> bool {
    before.kind == moved.kind
        && before.bytes == moved.bytes
        && before.identity.device == moved.identity.device
        && before.identity.inode == moved.identity.inode
        && before.identity.uid == moved.identity.uid
        && before.identity.link_count == moved.identity.link_count
        && before.identity.mode == moved.identity.mode
        && before.identity.size == moved.identity.size
        && before.identity.modified_seconds == moved.identity.modified_seconds
        && before.identity.modified_nanoseconds == moved.identity.modified_nanoseconds
}

#[cfg(unix)]
fn restore_quarantined_private_leaf(
    directory: &File,
    quarantine: &OsStr,
    original: &OsStr,
) -> Result<(), CommandError> {
    rename_noreplace(directory, quarantine, original).map_err(|error| {
        private_cleanup_outcome_unknown(format!(
            "legacy private content remains quarantined because its original name could not be safely restored: {error}"
        ))
    })?;
    sync_private_cleanup_directory(directory).map_err(|error| {
        private_cleanup_partial_error(format!(
            "restored legacy private content could not be durably verified: {error}"
        ))
    })?;
    Ok(())
}

#[cfg(unix)]
fn restore_quarantined_private_leaf_bound(
    lock: &AppMutationLock,
    directory: &File,
    quarantine: &OsStr,
    original: &OsStr,
    expected: &AppDataPrivateLeafSnapshot,
) -> Result<(), CommandError> {
    require_bound_private_leaf_at(
        lock,
        directory,
        quarantine,
        expected,
        "private-file quarantine before restoration",
    )?;
    restore_quarantined_private_leaf(directory, quarantine, original)
}

fn private_cleanup_partial_error(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "legacy AI/private-content cleanup".to_string(),
        state: "applied_unverified",
        cleanup_required: true,
        detail,
    }
}

fn private_cleanup_outcome_unknown(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "legacy AI/private-content cleanup".to_string(),
        state: "outcome_unknown",
        cleanup_required: true,
        detail,
    }
}

#[cfg(unix)]
fn sync_private_cleanup_directory(directory: &File) -> Result<(), io::Error> {
    #[cfg(test)]
    if PRIVATE_CLEANUP_DIRECTORY_SYNC_FAULT.with(|fault| fault.replace(false)) {
        return Err(io::Error::other(
            "injected private-cleanup directory sync failure",
        ));
    }
    rustix::fs::fsync(directory).map_err(io::Error::from)
}

#[cfg(unix)]
fn cleanup_unactivated_private_candidate(
    directory: &File,
    temp_name: &OsStr,
    opened: &File,
    original_error: CommandError,
) -> Result<(), CommandError> {
    use rustix::fs::{fstat, fsync, statat, unlinkat, AtFlags};

    let opened = fstat(opened).map_err(|error| {
        app_data_private_write_partial_error(format!(
            "private candidate activation failed and its descriptor identity could not be read: {}; original failure: {original_error}",
            io::Error::from(error)
        ))
    })?;
    let current =
        statat(directory, temp_name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
            app_data_private_write_outcome_unknown(format!(
                "private candidate activation failed and its path identity could not be read: {}; original failure: {original_error}",
                io::Error::from(error)
            ))
        })?;
    if opened.st_dev != current.st_dev
        || opened.st_ino != current.st_ino
        || opened.st_uid != current.st_uid
        || opened.st_nlink != current.st_nlink
        || opened.st_mode != current.st_mode
    {
        return Err(app_data_private_write_outcome_unknown(format!(
            "private candidate activation failed and its temporary path changed; the replacement was retained; original failure: {original_error}"
        )));
    }
    if let Err(error) = unlinkat(directory, temp_name, AtFlags::empty()) {
        return Err(app_data_private_write_partial_error(format!(
            "private candidate activation failed and its temporary file could not be removed: {}; original failure: {original_error}",
            io::Error::from(error)
        )));
    }
    if let Err(error) = fsync(directory).map_err(io::Error::from) {
        return Err(app_data_private_write_partial_error(format!(
            "private candidate activation failed and temporary cleanup durability could not be verified: {error}; original failure: {original_error}"
        )));
    }
    Err(original_error)
}

fn app_data_private_write_partial_error(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "app-data private file write".to_string(),
        state: "applied_unverified",
        cleanup_required: true,
        detail,
    }
}

fn app_data_private_write_outcome_unknown(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "app-data private file write".to_string(),
        state: "outcome_unknown",
        cleanup_required: true,
        detail,
    }
}

fn classify_app_data_private_write_readback_error(error: CommandError) -> CommandError {
    let detail =
        format!("private candidate was activated, but its bound read-back failed: {error}");
    match error {
        CommandError::StaleActionReference | CommandError::UnsafeConfigPath(_) => {
            app_data_private_write_outcome_unknown(detail)
        }
        CommandError::PartialEffect {
            state: "outcome_unknown",
            ..
        } => error,
        _ => app_data_private_write_partial_error(detail),
    }
}

#[cfg(unix)]
fn cleanup_created_private_append_file(
    directory: &File,
    name: &OsStr,
    opened: &File,
    original_error: CommandError,
) -> Result<(), CommandError> {
    use rustix::fs::{fstat, fsync, statat, unlinkat, AtFlags};

    let opened = fstat(opened).map_err(|error| {
        app_data_private_append_partial_error(format!(
            "a newly created append target could not be identity-checked for cleanup: {}; original failure: {original_error}",
            io::Error::from(error)
        ))
    })?;
    let current = statat(directory, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
        app_data_private_append_outcome_unknown(format!(
            "a newly created append target could not be rebound for cleanup: {}; original failure: {original_error}",
            io::Error::from(error)
        ))
    })?;
    if opened.st_dev != current.st_dev
        || opened.st_ino != current.st_ino
        || opened.st_uid != current.st_uid
        || opened.st_nlink != current.st_nlink
        || opened.st_mode != current.st_mode
    {
        return Err(app_data_private_append_outcome_unknown(format!(
            "a newly created append target changed identity before cleanup; original failure: {original_error}"
        )));
    }
    unlinkat(directory, name, AtFlags::empty()).map_err(|error| {
        app_data_private_append_partial_error(format!(
            "a newly created append target could not be removed safely: {}; original failure: {original_error}",
            io::Error::from(error)
        ))
    })?;
    fsync(directory).map_err(|error| {
        app_data_private_append_partial_error(format!(
            "new append-target cleanup durability could not be verified: {}; original failure: {original_error}",
            io::Error::from(error)
        ))
    })?;
    Err(original_error)
}

fn app_data_private_append_partial_error(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "app-data private append".to_string(),
        state: "applied_unverified",
        cleanup_required: true,
        detail,
    }
}

fn app_data_private_append_outcome_unknown(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "app-data private append".to_string(),
        state: "outcome_unknown",
        cleanup_required: true,
        detail,
    }
}

fn classify_private_directory_creation_error(
    created_any: bool,
    error: CommandError,
) -> CommandError {
    if !created_any || matches!(error, CommandError::PartialEffect { .. }) {
        return error;
    }
    CommandError::PartialEffect {
        operation: "app-data private directory creation".to_string(),
        state: "applied_unverified",
        cleanup_required: true,
        detail: format!(
            "one or more private directories were created before completion failed: {error}"
        ),
    }
}

fn validate_relative_path(path: &Path) -> Result<(), CommandError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(unsafe_relative_path());
    }
    Ok(())
}

fn validate_single_component(name: &OsStr) -> Result<(), CommandError> {
    let path = Path::new(name);
    if name.is_empty()
        || path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        return Err(unsafe_relative_path());
    }
    Ok(())
}

fn validate_filename_fragment(fragment: &str) -> Result<(), CommandError> {
    if fragment.is_empty()
        || fragment.chars().any(char::is_control)
        || fragment.contains('/')
        || fragment.contains('\\')
    {
        return Err(unsafe_relative_path());
    }
    Ok(())
}

fn unsafe_relative_path() -> CommandError {
    CommandError::UnsafeConfigPath(
        "app-data access requires a non-empty normalized relative path".to_string(),
    )
}

fn unsafe_relative_file(label: &str) -> CommandError {
    CommandError::UnsafeConfigPath(format!("{label} is not a bounded regular file"))
}

fn cleanup_identity_error(removed_any: bool, detail: String) -> CommandError {
    if removed_any {
        cleanup_partial_error(detail)
    } else {
        CommandError::UnsafeConfigPath(detail)
    }
}

fn cleanup_operation_error(removed_any: bool, detail: String) -> CommandError {
    if removed_any {
        cleanup_partial_error(detail)
    } else {
        io::Error::other(detail).into()
    }
}

fn cleanup_partial_error(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "app-data private residue cleanup".to_string(),
        state: "applied_unverified",
        cleanup_required: true,
        detail,
    }
}

fn cleanup_outcome_unknown(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "app-data private residue cleanup".to_string(),
        state: "outcome_unknown",
        cleanup_required: true,
        detail,
    }
}

#[cfg(unix)]
fn quarantine_residue(
    directory: &File,
    source: &OsStr,
    prefix: &str,
    suffix: &str,
    removed_any: bool,
) -> Result<OsString, CommandError> {
    for _ in 0..32 {
        let token = random_cleanup_token().map_err(|error| {
            cleanup_operation_error(
                removed_any,
                format!("cannot allocate an app-data residue quarantine name: {error}"),
            )
        })?;
        let quarantine = OsString::from(format!("{prefix}cleanup-{token}{suffix}"));
        match rename_noreplace(directory, source, &quarantine) {
            Ok(()) => return Ok(quarantine),
            Err(error) if error.kind() == io::ErrorKind::AlreadyExists => {}
            Err(error) => {
                return Err(cleanup_operation_error(
                    removed_any,
                    format!("cannot quarantine app-data private residue: {error}"),
                ));
            }
        }
    }
    Err(cleanup_operation_error(
        removed_any,
        "app-data private residue quarantine allocation was exhausted".to_string(),
    ))
}

#[cfg(unix)]
fn random_cleanup_token() -> io::Result<String> {
    let mut entropy = [0_u8; 32];
    File::open("/dev/urandom")?.read_exact(&mut entropy)?;
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = [0_u8; 64];
    for (index, byte) in entropy.into_iter().enumerate() {
        encoded[index * 2] = HEX[usize::from(byte >> 4)];
        encoded[index * 2 + 1] = HEX[usize::from(byte & 0x0f)];
    }
    String::from_utf8(encoded.to_vec())
        .map_err(|error| io::Error::new(io::ErrorKind::InvalidData, error))
}

#[cfg(all(
    unix,
    any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "linux",
        target_os = "redox"
    )
))]
fn rename_noreplace(directory: &File, from: &OsStr, to: &OsStr) -> io::Result<()> {
    use rustix::fs::{renameat_with, RenameFlags};

    renameat_with(directory, from, directory, to, RenameFlags::NOREPLACE).map_err(io::Error::from)
}

#[cfg(all(
    unix,
    not(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "linux",
        target_os = "redox"
    ))
))]
fn rename_noreplace(_directory: &File, _from: &OsStr, _to: &OsStr) -> io::Result<()> {
    Err(io::Error::new(
        io::ErrorKind::Unsupported,
        "atomic no-replace quarantine is unavailable on this platform",
    ))
}

#[cfg(unix)]
fn sync_cleanup_directory(directory: &File) -> io::Result<()> {
    #[cfg(test)]
    if REMOVE_MATCHING_DIRECTORY_SYNC_FAULT.with(|fault| fault.replace(false)) {
        return Err(io::Error::other(
            "injected app-data residue directory sync failure",
        ));
    }
    rustix::fs::fsync(directory).map_err(io::Error::from)
}

#[cfg(unix)]
fn sync_created_private_directory_parent(directory: &impl std::os::fd::AsFd) -> io::Result<()> {
    #[cfg(test)]
    if ENSURE_PRIVATE_DIRECTORY_SYNC_FAULT.with(|fault| fault.replace(false)) {
        return Err(io::Error::other(
            "injected private-directory parent sync failure",
        ));
    }
    rustix::fs::fsync(directory).map_err(io::Error::from)
}

#[cfg(unix)]
fn validate_private_directory(
    directory: &impl std::os::fd::AsFd,
    owner_uid: u32,
) -> Result<(), CommandError> {
    use rustix::fs::fstat;

    let metadata = fstat(directory).map_err(io::Error::from)?;
    if metadata.st_uid != owner_uid || metadata.st_mode & 0o077 != 0 {
        return Err(CommandError::UnsafeConfigPath(
            "app-data relative directory must be owner-owned with owner-only permissions"
                .to_string(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn private_owner_uid(lock: &AppMutationLock) -> Result<u32, CommandError> {
    use std::os::unix::fs::MetadataExt;

    let metadata = lock.owner_directory().metadata()?;
    let effective_uid = rustix::process::geteuid().as_raw();
    if !metadata.is_dir() || metadata.uid() != effective_uid {
        return Err(CommandError::UnsafeConfigPath(
            "app-data owner must be a directory owned by the current effective user".to_string(),
        ));
    }
    Ok(effective_uid)
}

fn map_relative_io(error: io::Error, label: &str) -> CommandError {
    if error.kind() == io::ErrorKind::PermissionDenied {
        return unsafe_relative_file(label);
    }
    #[cfg(unix)]
    if error.raw_os_error().is_some_and(|code| {
        code == rustix::io::Errno::LOOP.raw_os_error()
            || code == rustix::io::Errno::NOTDIR.raw_os_error()
    }) {
        return unsafe_relative_file(label);
    }
    error.into()
}

#[cfg(unix)]
fn map_unsafe_relative_errno(error: rustix::io::Errno) -> CommandError {
    if matches!(error, rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR) {
        unsafe_relative_path()
    } else {
        io::Error::from(error).into()
    }
}

#[cfg(unix)]
fn directory_flags() -> rustix::fs::OFlags {
    rustix::fs::OFlags::RDONLY
        | rustix::fs::OFlags::DIRECTORY
        | rustix::fs::OFlags::NOFOLLOW
        | rustix::fs::OFlags::CLOEXEC
}

#[cfg(not(unix))]
fn guarded_fallback_path(root: &Path, relative: &Path) -> Result<PathBuf, CommandError> {
    validate_relative_path(relative)?;
    let root_metadata = std::fs::symlink_metadata(root)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(unsafe_relative_path());
    }
    let mut current = root.to_path_buf();
    let components = relative.components().collect::<Vec<_>>();
    for (index, component) in components.iter().enumerate() {
        let Component::Normal(name) = component else {
            return Err(unsafe_relative_path());
        };
        current.push(name);
        if index + 1 < components.len() {
            let metadata = std::fs::symlink_metadata(&current)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(unsafe_relative_path());
            }
        }
    }
    Ok(current)
}

fn command_error_to_io(error: CommandError) -> io::Error {
    match error {
        CommandError::Io(error) => error,
        other => io::Error::new(io::ErrorKind::PermissionDenied, other.to_string()),
    }
}

#[cfg(not(unix))]
fn secure_owner_temp_name(stem: &str) -> Result<OsString, CommandError> {
    let mut nonce = [0_u8; 16];
    getrandom::getrandom(&mut nonce).map_err(|error| {
        io::Error::other(format!("secure random name generation failed: {error}"))
    })?;
    let encoded = nonce
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(OsString::from(format!(".{stem}.{encoded}.tmp")))
}

#[cfg(test)]
struct OwnerReadTestHook {
    relative: PathBuf,
    action: Box<dyn FnOnce() + Send>,
}

#[cfg(test)]
thread_local! {
    static OWNER_READ_TEST_HOOKS: std::cell::RefCell<Vec<OwnerReadTestHook>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(crate) fn install_owner_read_test_hook(
    relative: PathBuf,
    action: impl FnOnce() + Send + 'static,
) {
    OWNER_READ_TEST_HOOKS.with(|hooks| {
        hooks.borrow_mut().push(OwnerReadTestHook {
            relative,
            action: Box::new(action),
        })
    });
}

#[cfg(test)]
fn run_owner_read_test_hook(relative: &Path) {
    let action = OWNER_READ_TEST_HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        hooks
            .iter()
            .position(|hook| hook.relative == relative)
            .map(|index| hooks.remove(index).action)
    });
    if let Some(action) = action {
        action();
    }
}

#[cfg(not(test))]
fn run_owner_read_test_hook(_relative: &Path) {}

#[cfg(test)]
mod tests {
    use super::*;
    use std::time::{SystemTime, UNIX_EPOCH};

    #[test]
    #[cfg(unix)]
    fn owner_relative_private_replace_stays_on_the_locked_inode_after_path_replacement() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-anchor-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let moved_owner = root.join("locked-owner");
        let victim = root.join("victim");
        std::fs::create_dir_all(&owner_path).expect("owner");
        std::fs::create_dir_all(&victim).expect("victim");
        std::fs::write(victim.join("sentinel"), b"unchanged").expect("victim sentinel");
        let victim_mode = std::fs::metadata(&victim)
            .expect("victim metadata")
            .permissions()
            .mode()
            & 0o777;

        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let owner = lock.owner_fs();
        std::fs::rename(&owner_path, &moved_owner).expect("move locked owner");
        symlink(&victim, &owner_path).expect("replace owner path");

        owner
            .replace_private_file_if_current(
                Path::new("state.json"),
                None,
                br#"{"ok":true}"#,
                "state",
                ".state.bound.",
            )
            .expect("descriptor-relative replace");

        assert_eq!(
            std::fs::read(moved_owner.join("state.json")).expect("anchored state"),
            br#"{"ok":true}"#
        );
        assert!(
            !victim.join("state.json").exists(),
            "the replacement target must never receive app data"
        );
        assert_eq!(
            std::fs::read(victim.join("sentinel")).expect("victim sentinel"),
            b"unchanged"
        );
        assert_eq!(
            std::fs::metadata(&victim)
                .expect("victim metadata")
                .permissions()
                .mode()
                & 0o777,
            victim_mode,
            "owner-relative operations must not chmod the replacement target"
        );

        std::fs::remove_file(&owner_path).expect("remove link");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    fn owner_relative_paths_reject_empty_and_parent_components() {
        for path in [
            Path::new(""),
            Path::new("."),
            Path::new("../state"),
            Path::new("/state"),
        ] {
            assert!(matches!(
                validate_relative_path(path),
                Err(CommandError::UnsafeConfigPath(_))
            ));
        }
    }

    #[test]
    #[cfg(unix)]
    fn bounded_append_rejects_hard_links_without_changing_the_target() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-append-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let victim = root.join("victim");
        std::fs::create_dir_all(&owner_path).expect("owner");
        std::fs::write(&victim, b"unchanged").expect("victim");
        std::fs::hard_link(&victim, owner_path.join("activity.jsonl")).expect("hard link");

        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let owner = lock.owner_fs();
        let result =
            owner.append_private_file(Path::new("activity.jsonl"), b"\nchanged", 1024, "activity");

        assert!(matches!(result, Err(CommandError::UnsafeConfigPath(_))));
        assert_eq!(std::fs::read(&victim).expect("victim"), b"unchanged");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn bounded_read_rejects_hard_links() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-read-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let victim = root.join("victim");
        std::fs::create_dir_all(&owner_path).expect("owner");
        std::fs::write(&victim, b"private").expect("victim");
        std::fs::hard_link(&victim, owner_path.join("state.json")).expect("hard link");

        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let result =
            lock.owner_fs()
                .read_bounded_regular_file(Path::new("state.json"), 1024, "state");

        assert!(matches!(result, Err(CommandError::UnsafeConfigPath(_))));
        assert_eq!(std::fs::read(&victim).expect("victim"), b"private");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn bounded_read_rejects_group_or_other_file_permissions_but_accepts_owner_only_read() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-read-mode-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("owner");
        let path = root.join("state.json");
        std::fs::write(&path, b"private").expect("state");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o640))
            .expect("broad state mode");
        let lock = crate::mutation_lock::lock_app_mutations(&root).expect("lock owner");

        let broad =
            lock.owner_fs()
                .read_bounded_regular_file(Path::new("state.json"), 1024, "state");
        assert!(matches!(broad, Err(CommandError::UnsafeConfigPath(_))));

        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o400))
            .expect("private state mode");
        assert_eq!(
            lock.owner_fs()
                .read_bounded_regular_file(Path::new("state.json"), 1024, "state")
                .expect("private state"),
            Some(b"private".to_vec())
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn bounded_read_rejects_same_length_in_place_change_before_final_stamp() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-read-in-place-race-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("owner");
        let path = root.join("state.json");
        std::fs::write(&path, b"first-value").expect("state");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .expect("private state mode");
        let raced_path = path.clone();
        install_private_read_before_final_stamp_hook(move || {
            std::fs::write(&raced_path, b"other-value").expect("same-length raced write");
        });
        let lock = crate::mutation_lock::lock_app_mutations(&root).expect("lock owner");

        let result =
            lock.owner_fs()
                .read_bounded_regular_file(Path::new("state.json"), 1024, "state");

        assert!(matches!(result, Err(CommandError::StaleActionReference)));
        assert_eq!(std::fs::read(&path).expect("raced value"), b"other-value");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn bounded_read_rejects_group_or_other_permissions_on_nested_directories() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-directory-mode-{}-{unique}",
            std::process::id()
        ));
        let nested = root.join("llm");
        std::fs::create_dir_all(&nested).expect("nested owner");
        std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o750))
            .expect("broad directory mode");
        let path = nested.join("state.json");
        std::fs::write(&path, b"private").expect("state");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .expect("private state mode");
        let lock = crate::mutation_lock::lock_app_mutations(&root).expect("lock owner");

        let broad =
            lock.owner_fs()
                .read_bounded_regular_file(Path::new("llm/state.json"), 1024, "state");
        assert!(matches!(broad, Err(CommandError::UnsafeConfigPath(_))));

        std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o700))
            .expect("private directory mode");
        assert_eq!(
            lock.owner_fs()
                .read_bounded_regular_file(Path::new("llm/state.json"), 1024, "state")
                .expect("private nested state"),
            Some(b"private".to_vec())
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn bounded_read_treats_a_missing_nested_parent_as_absent() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-missing-parent-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("owner");
        let lock = crate::mutation_lock::lock_app_mutations(&root).expect("lock owner");

        assert_eq!(
            lock.owner_fs()
                .read_bounded_regular_file(Path::new("missing/state.json"), 1024, "missing state",)
                .expect("missing nested state"),
            None
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn private_directory_creation_sync_failure_after_mkdir_is_typed_partial() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-directory-sync-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("owner");
        let lock = crate::mutation_lock::lock_app_mutations(&root).expect("lock owner");
        install_ensure_private_directory_sync_fault();

        let result = lock.owner_fs().ensure_directory_all(Path::new("llm"));

        assert!(matches!(
            result,
            Err(CommandError::PartialEffect {
                state: "applied_unverified",
                cleanup_required: true,
                ..
            })
        ));
        assert!(
            root.join("llm").is_dir(),
            "mkdir crossed the effect boundary"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn nested_cleanup_stays_on_the_locked_inode_after_path_replacement() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-cleanup-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let moved_owner = root.join("locked-owner");
        let victim = root.join("victim");
        std::fs::create_dir_all(owner_path.join("llm")).expect("owner");
        std::fs::create_dir_all(victim.join("llm")).expect("victim");
        std::fs::write(owner_path.join("llm/.state.1.tmp"), b"owner").expect("owner residue");
        std::fs::write(victim.join("llm/.state.1.tmp"), b"victim").expect("victim residue");
        std::fs::set_permissions(
            owner_path.join("llm"),
            std::fs::Permissions::from_mode(0o700),
        )
        .expect("private owner directory");
        std::fs::set_permissions(
            owner_path.join("llm/.state.1.tmp"),
            std::fs::Permissions::from_mode(0o600),
        )
        .expect("private owner residue");

        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let owner = lock.owner_fs();
        std::fs::rename(&owner_path, &moved_owner).expect("move locked owner");
        symlink(&victim, &owner_path).expect("replace owner path");

        owner
            .remove_regular_files_matching(Path::new("llm"), ".state.", ".tmp", 8)
            .expect("descriptor-relative cleanup");

        assert!(!moved_owner.join("llm/.state.1.tmp").exists());
        assert_eq!(
            std::fs::read(victim.join("llm/.state.1.tmp")).expect("victim residue"),
            b"victim"
        );

        std::fs::remove_file(&owner_path).expect("remove link");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn matching_cleanup_quarantines_then_restores_a_replacement_raced_after_recheck() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-cleanup-race-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let directory = owner_path.join("llm");
        let residue = directory.join(".state.1.tmp");
        let original = directory.join("original-preserved.tmp");
        std::fs::create_dir_all(&directory).expect("owner");
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
            .expect("private directory");
        std::fs::write(&residue, b"original").expect("residue");
        std::fs::set_permissions(&residue, std::fs::Permissions::from_mode(0o600))
            .expect("private residue");
        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let raced_residue = residue.clone();
        let preserved_original = original.clone();
        install_remove_matching_before_quarantine_hook(OsString::from(".state.1.tmp"), move || {
            std::fs::rename(&raced_residue, &preserved_original).expect("move original residue");
            std::fs::write(&raced_residue, b"replacement").expect("raced replacement");
            std::fs::set_permissions(&raced_residue, std::fs::Permissions::from_mode(0o600))
                .expect("private raced replacement");
        });

        let result =
            lock.owner_fs()
                .remove_regular_files_matching(Path::new("llm"), ".state.", ".tmp", 8);

        assert!(matches!(result, Err(CommandError::UnsafeConfigPath(_))));
        assert_eq!(
            std::fs::read(&residue).expect("replacement preserved"),
            b"replacement"
        );
        assert_eq!(
            std::fs::read(&original).expect("original preserved"),
            b"original"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn matching_cleanup_directory_sync_failure_after_delete_is_typed_partial() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-cleanup-sync-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let directory = owner_path.join("llm");
        let residue = directory.join(".state.1.tmp");
        std::fs::create_dir_all(&directory).expect("owner");
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
            .expect("private directory");
        std::fs::write(&residue, b"original").expect("residue");
        std::fs::set_permissions(&residue, std::fs::Permissions::from_mode(0o600))
            .expect("private residue");
        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        install_remove_matching_directory_sync_fault();

        let result =
            lock.owner_fs()
                .remove_regular_files_matching(Path::new("llm"), ".state.", ".tmp", 8);

        assert!(matches!(
            result,
            Err(CommandError::PartialEffect {
                state: "applied_unverified",
                cleanup_required: true,
                ..
            })
        ));
        assert!(
            !residue.exists(),
            "the deletion occurred even though its directory sync was uncertain"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn matching_cleanup_later_identity_race_is_partial_after_an_earlier_delete() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-cleanup-multiple-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let directory = owner_path.join("llm");
        let first = directory.join(".state.1.tmp");
        let second = directory.join(".state.2.tmp");
        let preserved_second = directory.join("original-two-preserved.tmp");
        std::fs::create_dir_all(&directory).expect("owner");
        std::fs::set_permissions(&directory, std::fs::Permissions::from_mode(0o700))
            .expect("private directory");
        for (path, content) in [(&first, b"one".as_slice()), (&second, b"two".as_slice())] {
            std::fs::write(path, content).expect("residue");
            std::fs::set_permissions(path, std::fs::Permissions::from_mode(0o600))
                .expect("private residue");
        }
        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let raced_second = second.clone();
        let original_second = preserved_second.clone();
        install_remove_matching_before_quarantine_hook(OsString::from(".state.2.tmp"), move || {
            std::fs::rename(&raced_second, &original_second).expect("preserve second original");
            std::fs::write(&raced_second, b"replacement").expect("replacement");
            std::fs::set_permissions(&raced_second, std::fs::Permissions::from_mode(0o600))
                .expect("private replacement");
        });

        let result =
            lock.owner_fs()
                .remove_regular_files_matching(Path::new("llm"), ".state.", ".tmp", 8);

        assert!(matches!(
            result,
            Err(CommandError::PartialEffect {
                state: "applied_unverified",
                cleanup_required: true,
                ..
            })
        ));
        assert!(!first.exists(), "the first residue was already deleted");
        assert_eq!(
            std::fs::read(&second).expect("replacement restored"),
            b"replacement"
        );
        assert_eq!(
            std::fs::read(&preserved_second).expect("original preserved"),
            b"two"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn private_cleanup_replace_never_overwrites_a_post_quarantine_replacement() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-private-cleanup-replace-race-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let target = owner_path.join("prompt-runs.json");
        std::fs::create_dir_all(&owner_path).expect("owner");
        std::fs::write(&target, b"legacy-sensitive").expect("legacy target");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))
            .expect("private legacy target");
        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let owner = lock.owner_fs();
        let snapshot = owner
            .inspect_private_cleanup_leaf(Path::new("prompt-runs.json"), 1024, "legacy prompt runs")
            .expect("inspect")
            .expect("snapshot");
        let raced_target = target.clone();
        install_private_cleanup_post_quarantine_hook(
            OsString::from("prompt-runs.json"),
            move || {
                std::fs::write(&raced_target, b"third-party-replacement")
                    .expect("raced replacement");
                std::fs::set_permissions(&raced_target, std::fs::Permissions::from_mode(0o600))
                    .expect("private replacement");
            },
        );

        let result = owner.replace_private_cleanup_regular_leaf(
            Path::new("prompt-runs.json"),
            &snapshot,
            b"canonical-metadata",
            "prompt-cleanup",
            ".prompt-runs.json.legacy-private-cleanup-",
        );

        assert!(matches!(
            result,
            Err(CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            })
        ));
        assert_eq!(
            std::fs::read(&target).expect("replacement preserved"),
            b"third-party-replacement"
        );
        let quarantines = std::fs::read_dir(&owner_path)
            .expect("owner entries")
            .filter_map(Result::ok)
            .filter(|entry| {
                let name = entry.file_name().to_string_lossy().to_string();
                name.starts_with(".prompt-runs.json.legacy-private-cleanup-")
                    && name.ends_with(".quarantine")
            })
            .collect::<Vec<_>>();
        assert_eq!(quarantines.len(), 1);
        assert_eq!(
            std::fs::read(quarantines[0].path()).expect("legacy quarantine"),
            b"legacy-sensitive"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn private_cleanup_delete_does_not_report_success_when_original_name_is_recreated() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-private-cleanup-delete-race-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let target = owner_path.join("model-task-matches.json");
        std::fs::create_dir_all(&owner_path).expect("owner");
        std::fs::write(&target, b"legacy-sensitive").expect("legacy target");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))
            .expect("private legacy target");
        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let owner = lock.owner_fs();
        let snapshot = owner
            .inspect_private_cleanup_leaf(
                Path::new("model-task-matches.json"),
                1024,
                "legacy model-task history",
            )
            .expect("inspect")
            .expect("snapshot");
        let raced_target = target.clone();
        install_private_cleanup_post_quarantine_hook(
            OsString::from("model-task-matches.json"),
            move || {
                std::fs::write(&raced_target, b"third-party-replacement")
                    .expect("raced replacement");
                std::fs::set_permissions(&raced_target, std::fs::Permissions::from_mode(0o600))
                    .expect("private replacement");
            },
        );

        let result = owner.remove_private_cleanup_leaf(
            Path::new("model-task-matches.json"),
            &snapshot,
            ".model-task-matches.json.legacy-private-cleanup-",
        );

        assert!(matches!(
            result,
            Err(CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            })
        ));
        assert_eq!(
            std::fs::read(&target).expect("replacement preserved"),
            b"third-party-replacement"
        );
        assert_eq!(
            std::fs::read_dir(&owner_path)
                .expect("owner entries")
                .filter_map(Result::ok)
                .filter(|entry| {
                    let name = entry.file_name().to_string_lossy().to_string();
                    name.starts_with(".model-task-matches.json.legacy-private-cleanup-")
                        && name.ends_with(".quarantine")
                })
                .count(),
            1
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn private_cleanup_quarantine_sync_failure_is_applied_unverified() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-private-cleanup-sync-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let target = owner_path.join("model-task-matches.json");
        std::fs::create_dir_all(&owner_path).expect("owner");
        std::fs::write(&target, b"legacy-sensitive").expect("legacy target");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))
            .expect("private legacy target");
        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let owner = lock.owner_fs();
        let snapshot = owner
            .inspect_private_cleanup_leaf(
                Path::new("model-task-matches.json"),
                1024,
                "legacy model-task history",
            )
            .expect("inspect")
            .expect("snapshot");
        install_private_cleanup_directory_sync_fault();

        let result = owner.remove_private_cleanup_leaf(
            Path::new("model-task-matches.json"),
            &snapshot,
            ".model-task-matches.json.legacy-private-cleanup-",
        );

        assert!(matches!(
            result,
            Err(CommandError::PartialEffect {
                state: "applied_unverified",
                cleanup_required: true,
                ..
            })
        ));
        assert!(!target.exists(), "the accepted leaf was quarantined");
        assert_eq!(
            std::fs::read_dir(&owner_path)
                .expect("owner entries")
                .filter_map(Result::ok)
                .filter(|entry| {
                    let name = entry.file_name().to_string_lossy().to_string();
                    name.starts_with(".model-task-matches.json.legacy-private-cleanup-")
                        && name.ends_with(".quarantine")
                })
                .count(),
            1,
            "the identity-matching quarantine is retained"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn private_write_path_rebind_after_activation_is_outcome_unknown() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-private-write-rebind-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("owner");
        let target = root.join("state.json");
        let accepted = root.join("accepted-state.json");
        let raced_target = target.clone();
        let raced_accepted = accepted.clone();
        install_private_write_post_activation_hook(move || {
            std::fs::rename(&raced_target, &raced_accepted).expect("preserve accepted candidate");
            std::fs::write(&raced_target, b"replacement").expect("install replacement");
            std::fs::set_permissions(&raced_target, std::fs::Permissions::from_mode(0o600))
                .expect("private replacement");
        });
        let lock = crate::mutation_lock::lock_app_mutations(&root).expect("lock owner");

        let result = lock.owner_fs().replace_private_file_if_current(
            Path::new("state.json"),
            None,
            b"candidate",
            "state",
            ".state-cleanup-",
        );

        assert!(matches!(
            result,
            Err(CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            })
        ));
        assert_eq!(std::fs::read(&accepted).expect("candidate"), b"candidate");
        assert_eq!(std::fs::read(&target).expect("replacement"), b"replacement");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn bounded_append_creates_private_file_and_verifies_the_result_length() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-private-append-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        std::fs::create_dir_all(&owner_path).expect("owner");
        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let owner = lock.owner_fs();

        owner
            .append_private_file(Path::new("activity.jsonl"), b"one\n", 8, "activity")
            .expect("first append");
        owner
            .append_private_file(Path::new("activity.jsonl"), b"two\n", 8, "activity")
            .expect("second append");

        let path = owner_path.join("activity.jsonl");
        assert_eq!(std::fs::read(&path).expect("activity"), b"one\ntwo\n");
        assert_eq!(
            std::fs::metadata(&path)
                .expect("metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn bounded_append_concurrent_length_change_after_write_is_typed_partial() {
        use std::io::Write as _;
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-append-race-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("owner");
        let path = root.join("activity.jsonl");
        std::fs::write(&path, b"one\n").expect("activity");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .expect("private activity");
        let raced_path = path.clone();
        install_private_append_post_write_hook(move || {
            std::fs::OpenOptions::new()
                .append(true)
                .open(&raced_path)
                .expect("open raced append")
                .write_all(b"race\n")
                .expect("raced append");
        });
        let lock = crate::mutation_lock::lock_app_mutations(&root).expect("lock owner");

        let result = lock.owner_fs().append_private_file(
            Path::new("activity.jsonl"),
            b"two\n",
            64,
            "activity",
        );

        assert!(matches!(
            result,
            Err(CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            })
        ));
        assert_eq!(std::fs::read(&path).expect("activity"), b"one\ntwo\nrace\n");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn bounded_append_path_rebind_after_write_preserves_the_replacement() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-fs-append-rebind-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("owner");
        let path = root.join("activity.jsonl");
        let accepted = root.join("accepted-activity.jsonl");
        std::fs::write(&path, b"one\n").expect("activity");
        std::fs::set_permissions(&path, std::fs::Permissions::from_mode(0o600))
            .expect("private activity");
        let raced_path = path.clone();
        let accepted_path = accepted.clone();
        install_private_append_post_write_hook(move || {
            std::fs::rename(&raced_path, &accepted_path).expect("preserve accepted activity");
            std::fs::write(&raced_path, b"replacement\n").expect("install replacement");
            std::fs::set_permissions(&raced_path, std::fs::Permissions::from_mode(0o600))
                .expect("private replacement");
        });
        let lock = crate::mutation_lock::lock_app_mutations(&root).expect("lock owner");

        let result = lock.owner_fs().append_private_file(
            Path::new("activity.jsonl"),
            b"two\n",
            64,
            "activity",
        );

        assert!(matches!(
            result,
            Err(CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            })
        ));
        assert_eq!(
            std::fs::read(&accepted).expect("accepted activity"),
            b"one\ntwo\n"
        );
        assert_eq!(
            std::fs::read(&path).expect("replacement activity"),
            b"replacement\n"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn tree_snapshot_and_copy_reject_hard_linked_source_files_without_touching_victim() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-tree-hardlink-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let source = owner_path.join("source");
        let victim = root.join("victim");
        std::fs::create_dir_all(&source).expect("source");
        std::fs::write(&victim, b"unchanged").expect("victim");
        std::fs::hard_link(&victim, source.join("linked.txt")).expect("hard link");
        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let owner = lock.owner_fs();

        let snapshot =
            owner.snapshot_regular_tree(Path::new("source"), 16, 1024, 1024, "source tree");
        let copied = owner.copy_regular_tree(
            Path::new("source"),
            Path::new("copy"),
            16,
            1024,
            1024,
            "source tree",
        );

        assert!(matches!(snapshot, Err(CommandError::UnsafeConfigPath(_))));
        assert!(matches!(copied, Err(CommandError::UnsafeConfigPath(_))));
        assert!(!owner_path.join("copy").exists());
        assert_eq!(std::fs::read(&victim).expect("victim"), b"unchanged");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn owner_rename_atomically_refuses_a_target_created_at_the_last_boundary() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-rename-race-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let source = owner_path.join("source");
        let destination = owner_path.join("destination");
        std::fs::create_dir_all(&source).expect("source");
        std::fs::set_permissions(&source, std::fs::Permissions::from_mode(0o700))
            .expect("private source");
        std::fs::write(source.join("SKILL.md"), b"source").expect("source file");
        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let owner = lock.owner_fs();
        let raced_destination = destination.clone();
        tree::install_owner_rename_test_hook(PathBuf::from("destination"), move || {
            std::fs::create_dir(&raced_destination).expect("raced destination");
            std::fs::write(raced_destination.join("sentinel"), b"unchanged")
                .expect("destination sentinel");
        });

        let result = owner.rename(Path::new("source"), Path::new("destination"));

        assert!(matches!(result, Err(CommandError::StaleActionReference)));
        assert_eq!(
            std::fs::read(source.join("SKILL.md")).expect("source preserved"),
            b"source"
        );
        assert_eq!(
            std::fs::read(destination.join("sentinel")).expect("destination preserved"),
            b"unchanged"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn tree_cleanup_quarantines_a_raced_replacement_without_following_it() {
        use std::os::unix::fs::{symlink, PermissionsExt};

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-tree-cleanup-race-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let target = owner_path.join("target");
        let accepted = owner_path.join("accepted-target");
        let victim = root.join("victim");
        std::fs::create_dir_all(&target).expect("target");
        std::fs::create_dir_all(&victim).expect("victim");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o700))
            .expect("private target");
        std::fs::write(target.join("SKILL.md"), b"accepted").expect("accepted file");
        std::fs::set_permissions(
            target.join("SKILL.md"),
            std::fs::Permissions::from_mode(0o600),
        )
        .expect("private accepted file");
        std::fs::write(victim.join("sentinel"), b"unchanged").expect("victim sentinel");
        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let owner = lock.owner_fs();
        let raced_target = target.clone();
        let raced_accepted = accepted.clone();
        let raced_victim = victim.clone();
        tree::install_owner_remove_test_hook(PathBuf::from("target"), move || {
            std::fs::rename(&raced_target, &raced_accepted).expect("move accepted target");
            symlink(&raced_victim, &raced_target).expect("install raced replacement");
        });

        let error = owner
            .remove_tree_if_exists(Path::new("target"))
            .expect_err("raced replacement must be retained");

        assert!(matches!(
            error,
            CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            }
        ));
        assert_eq!(
            std::fs::read(accepted.join("SKILL.md")).expect("accepted target preserved"),
            b"accepted"
        );
        assert_eq!(
            std::fs::read(victim.join("sentinel")).expect("victim preserved"),
            b"unchanged"
        );
        assert_eq!(
            std::fs::read_dir(&victim).expect("victim entries").count(),
            1
        );
        assert!(
            std::fs::read_dir(&owner_path)
                .expect("owner entries")
                .filter_map(Result::ok)
                .any(|entry| entry.file_name().to_string_lossy().contains("quarantine")),
            "the raced replacement must remain in private quarantine"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn tree_cleanup_final_unlink_recheck_preserves_a_quarantine_replacement() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-tree-quarantine-race-{}-{unique}",
            std::process::id()
        ));
        let owner_path = root.join("app-data");
        let target = owner_path.join("target");
        let accepted = owner_path.join("accepted-target");
        std::fs::create_dir_all(&owner_path).expect("owner");
        std::fs::write(&target, b"accepted").expect("target");
        std::fs::set_permissions(&target, std::fs::Permissions::from_mode(0o600))
            .expect("private target");
        let lock = crate::mutation_lock::lock_app_mutations(&owner_path).expect("lock owner");
        let raced_owner = owner_path.clone();
        let accepted_path = accepted.clone();
        tree::install_owner_quarantine_unlink_test_hook(move || {
            let quarantine = std::fs::read_dir(&raced_owner)
                .expect("owner entries")
                .filter_map(Result::ok)
                .find(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".agent-copilot-tree-quarantine.")
                })
                .expect("tree quarantine")
                .path();
            std::fs::rename(&quarantine, &accepted_path).expect("preserve accepted quarantine");
            std::fs::write(&quarantine, b"replacement").expect("install quarantine replacement");
            std::fs::set_permissions(&quarantine, std::fs::Permissions::from_mode(0o600))
                .expect("private replacement");
        });

        let error = lock
            .owner_fs()
            .remove_tree_if_exists(Path::new("target"))
            .expect_err("a quarantine replacement must not be unlinked");

        assert!(matches!(
            error,
            CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            }
        ));
        assert_eq!(
            std::fs::read(&accepted).expect("accepted target"),
            b"accepted"
        );
        let replacement = std::fs::read_dir(&owner_path)
            .expect("owner entries")
            .filter_map(Result::ok)
            .find(|entry| {
                entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".agent-copilot-tree-quarantine.")
            })
            .expect("quarantine replacement");
        assert_eq!(
            std::fs::read(replacement.path()).expect("replacement"),
            b"replacement"
        );
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn created_directory_cleanup_preserves_a_same_mode_new_inode() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-created-directory-race-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("owner");
        let lock = crate::mutation_lock::lock_app_mutations(&root).expect("lock owner");
        let owner = lock.owner_fs();
        let created = owner
            .ensure_directory_all(Path::new("llm/nested"))
            .expect("create private chain");
        let nested = root.join("llm/nested");
        let accepted = root.join("llm/accepted-nested");
        std::fs::rename(&nested, &accepted).expect("preserve accepted directory");
        std::fs::create_dir(&nested).expect("install replacement directory");
        std::fs::set_permissions(&nested, std::fs::Permissions::from_mode(0o700))
            .expect("private replacement");

        let error = owner
            .remove_empty_directories(&created)
            .expect_err("new inode must not satisfy the cleanup capability");

        assert!(matches!(error, CommandError::StaleActionReference));
        assert!(accepted.is_dir(), "accepted directory is preserved");
        assert!(nested.is_dir(), "replacement directory is preserved");
        std::fs::remove_dir_all(root).ok();
    }

    #[test]
    #[cfg(unix)]
    fn created_directory_cleanup_never_restores_a_replaced_quarantine() {
        use std::os::unix::fs::PermissionsExt;

        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "agent-copilot-owner-created-directory-quarantine-race-{}-{unique}",
            std::process::id()
        ));
        std::fs::create_dir_all(&root).expect("owner");
        let lock = crate::mutation_lock::lock_app_mutations(&root).expect("lock owner");
        let owner = lock.owner_fs();
        let created = owner
            .ensure_directory_all(Path::new("nested"))
            .expect("create private directory");
        let accepted = root.join("accepted-nested");
        let raced_root = root.clone();
        let raced_accepted = accepted.clone();
        tree::install_owner_quarantine_unlink_test_hook(move || {
            let quarantine = std::fs::read_dir(&raced_root)
                .expect("owner entries")
                .filter_map(Result::ok)
                .find(|entry| {
                    entry
                        .file_name()
                        .to_string_lossy()
                        .starts_with(".agent-copilot-empty-directory-quarantine.")
                })
                .expect("empty-directory quarantine")
                .path();
            std::fs::rename(&quarantine, &raced_accepted).expect("preserve accepted directory");
            std::fs::create_dir(&quarantine).expect("install quarantine replacement");
            std::fs::set_permissions(&quarantine, std::fs::Permissions::from_mode(0o700))
                .expect("private replacement");
        });

        let error = owner
            .remove_empty_directories(&created)
            .expect_err("a replaced quarantine must not be restored");

        assert!(matches!(
            error,
            CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            }
        ));
        assert!(
            !root.join("nested").exists(),
            "the third state must not be moved to the original name"
        );
        assert!(accepted.is_dir(), "the accepted directory is preserved");
        assert!(
            std::fs::read_dir(&root)
                .expect("owner entries")
                .filter_map(Result::ok)
                .any(|entry| entry
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".agent-copilot-empty-directory-quarantine.")),
            "the third-state quarantine is retained"
        );
        std::fs::remove_dir_all(root).ok();
    }
}
