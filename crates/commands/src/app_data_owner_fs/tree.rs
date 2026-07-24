#![cfg_attr(not(unix), allow(dead_code))]

use std::{
    fs::File,
    io::{self, Write},
    path::Path,
};

#[cfg(any(test, unix))]
use std::path::PathBuf;

#[cfg(unix)]
use std::{
    ffi::{OsStr, OsString},
    io::Read,
};

#[cfg(unix)]
use sha2::Digest;

#[cfg(unix)]
use super::{
    directory_flags, map_unsafe_relative_errno, private_owner_uid, unix_device_id, unix_file_mode,
    unix_link_count, unix_timestamp_nanoseconds, validate_private_directory,
};
#[cfg(not(unix))]
use super::{guarded_fallback_path, unsafe_relative_path};
use super::{
    unsafe_relative_file, validate_relative_path, AppDataCreatedDirectory, AppDataOwnerFs,
};
use crate::CommandError;

#[cfg(test)]
struct OwnerRenameTestHook {
    target: PathBuf,
    action: Box<dyn FnOnce() + Send>,
}

#[cfg(test)]
thread_local! {
    static OWNER_RENAME_TEST_HOOK: std::cell::RefCell<Option<OwnerRenameTestHook>> =
        const { std::cell::RefCell::new(None) };
    static OWNER_REMOVE_TEST_HOOK: std::cell::RefCell<Option<OwnerRenameTestHook>> =
        const { std::cell::RefCell::new(None) };
    static OWNER_QUARANTINE_UNLINK_TEST_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce() + Send>>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
pub(super) fn install_owner_rename_test_hook(
    target: PathBuf,
    action: impl FnOnce() + Send + 'static,
) {
    OWNER_RENAME_TEST_HOOK.with(|hook| {
        let mut hook = hook.borrow_mut();
        assert!(hook.is_none(), "owner rename hook already set");
        *hook = Some(OwnerRenameTestHook {
            target,
            action: Box::new(action),
        });
    });
}

#[cfg(test)]
fn run_owner_rename_test_hook(target: &Path) {
    let action = OWNER_RENAME_TEST_HOOK.with(|hook| {
        let mut hook = hook.borrow_mut();
        if hook.as_ref().is_some_and(|hook| hook.target == target) {
            hook.take().map(|hook| hook.action)
        } else {
            None
        }
    });
    if let Some(action) = action {
        action();
    }
}

#[cfg(test)]
pub(super) fn install_owner_remove_test_hook(
    target: PathBuf,
    action: impl FnOnce() + Send + 'static,
) {
    OWNER_REMOVE_TEST_HOOK.with(|hook| {
        let mut hook = hook.borrow_mut();
        assert!(hook.is_none(), "owner remove hook already set");
        *hook = Some(OwnerRenameTestHook {
            target,
            action: Box::new(action),
        });
    });
}

#[cfg(test)]
pub(super) fn install_owner_quarantine_unlink_test_hook(action: impl FnOnce() + Send + 'static) {
    OWNER_QUARANTINE_UNLINK_TEST_HOOK.with(|hook| {
        let mut hook = hook.borrow_mut();
        assert!(hook.is_none(), "owner quarantine unlink hook already set");
        *hook = Some(Box::new(action));
    });
}

#[cfg(test)]
fn run_owner_remove_test_hook(target: &Path) {
    let action = OWNER_REMOVE_TEST_HOOK.with(|hook| {
        let mut hook = hook.borrow_mut();
        if hook.as_ref().is_some_and(|hook| hook.target == target) {
            hook.take().map(|hook| hook.action)
        } else {
            None
        }
    });
    if let Some(action) = action {
        action();
    }
}

#[cfg(test)]
fn run_owner_quarantine_unlink_test_hook() {
    OWNER_QUARANTINE_UNLINK_TEST_HOOK.with(|hook| {
        if let Some(action) = hook.borrow_mut().take() {
            action();
        }
    });
}

#[cfg(not(test))]
fn run_owner_rename_test_hook(_target: &Path) {}

#[cfg(not(test))]
fn run_owner_remove_test_hook(_target: &Path) {}

#[cfg(not(test))]
fn run_owner_quarantine_unlink_test_hook() {}

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct AppDataTreeSnapshot {
    pub(crate) present: bool,
    pub(crate) rows: Vec<String>,
}

impl<'lock> AppDataOwnerFs<'lock> {
    pub(crate) fn open_directory_clone(&self, relative: &Path) -> Result<File, CommandError> {
        validate_relative_path(relative)?;
        #[cfg(unix)]
        {
            self.open_directory(relative)
        }
        #[cfg(not(unix))]
        {
            let path = guarded_fallback_path(self.lock.owner_path(), relative)?;
            let file = File::open(path)?;
            if !file.metadata()?.is_dir() {
                return Err(unsafe_relative_path());
            }
            Ok(file)
        }
    }

    pub(crate) fn validate_directory_binding(
        &self,
        relative: &Path,
        accepted: &File,
    ) -> Result<(), CommandError> {
        self.validate_owner_path_binding()?;
        let current = self.open_directory_clone(relative)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            let accepted = accepted.metadata()?;
            let current = current.metadata()?;
            if accepted.dev() != current.dev() || accepted.ino() != current.ino() {
                return Err(CommandError::StaleActionReference);
            }
        }
        #[cfg(not(unix))]
        {
            let _ = (accepted, current);
        }
        Ok(())
    }

    pub(crate) fn create_directory(&self, relative: &Path) -> Result<(), CommandError> {
        validate_relative_path(relative)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            use rustix::fs::{fchmod, fsync, mkdirat, openat, statat, AtFlags, FileType, Mode};
            let (parent, name) = self.open_private_parent(relative)?;
            let owner_uid = private_owner_uid(self.lock)?;
            let mode = Mode::from_bits_truncate(0o700);
            mkdirat(&parent, name, mode).map_err(io::Error::from)?;
            let created = statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                owner_tree_effect_unknown(format!(
                    "an app-owned directory was created, but its identity could not be read: {}",
                    io::Error::from(error)
                ))
            })?;
            if FileType::from_raw_mode(created.st_mode) != FileType::Directory
                || created.st_uid != owner_uid
            {
                return Err(owner_tree_effect_unknown(
                    "an app-owned directory was created, but its entry type changed".to_string(),
                ));
            }
            let directory = openat(&parent, name, directory_flags(), Mode::empty())
                .map_err(|error| {
                    owner_tree_effect_unknown(format!(
                        "an app-owned directory was created, but its descriptor could not be opened: {}",
                        io::Error::from(error)
                    ))
                })?;
            let directory = File::from(directory);
            let descriptor_metadata = directory.metadata().map_err(|error| {
                owner_tree_effect_unknown(format!(
                    "an app-owned directory was created, but descriptor identity could not be read: {error}"
                ))
            })?;
            if descriptor_metadata.dev() != created.st_dev as u64
                || descriptor_metadata.ino() != created.st_ino
                || descriptor_metadata.uid() != owner_uid
            {
                return Err(owner_tree_effect_unknown(
                    "an app-owned directory changed before descriptor binding".to_string(),
                ));
            }
            fchmod(&directory, mode).map_err(|error| {
                owner_tree_effect_unknown(format!(
                    "an app-owned directory was created, but private permissions could not be applied: {}",
                    io::Error::from(error)
                ))
            })?;
            fsync(&directory)
                .and_then(|()| fsync(&parent))
                .map_err(|error| {
                    owner_tree_effect_unknown(format!(
                        "app-owned directory durability could not be verified: {}",
                        io::Error::from(error)
                    ))
                })?;
            let rebound = statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                owner_tree_effect_unknown(format!(
                    "app-owned directory was synced, but its path binding could not be read: {}",
                    io::Error::from(error)
                ))
            })?;
            if FileType::from_raw_mode(rebound.st_mode) != FileType::Directory
                || rebound.st_dev != created.st_dev
                || rebound.st_ino != created.st_ino
                || rebound.st_uid != owner_uid
                || rebound.st_mode & 0o077 != 0
            {
                return Err(owner_tree_effect_unknown(
                    "app-owned directory changed after creation; the replacement was retained"
                        .to_string(),
                ));
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let target = guarded_fallback_path(self.lock.owner_path(), relative)?;
            std::fs::create_dir(target)?;
            Ok(())
        }
    }

    fn create_private_file(&self, relative: &Path) -> Result<File, CommandError> {
        validate_relative_path(relative)?;
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            use rustix::fs::{fchmod, fsync, openat, statat, AtFlags, FileType, Mode, OFlags};

            let (parent, name) = self.open_private_parent(relative)?;
            let owner_uid = private_owner_uid(self.lock)?;
            let mode = Mode::from_bits_truncate(0o600);
            let descriptor = openat(
                &parent,
                name,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                mode,
            )
            .map_err(io::Error::from)?;
            let file = File::from(descriptor);
            let metadata = file.metadata().map_err(|error| {
                owner_tree_effect_unknown(format!(
                    "an app-owned file was created, but its identity could not be read: {error}"
                ))
            })?;
            if !metadata.is_file() || metadata.uid() != owner_uid || metadata.nlink() != 1 {
                return Err(owner_tree_effect_unknown(
                    "an app-owned file was created, but it is not a private regular file"
                        .to_string(),
                ));
            }
            fchmod(&file, mode).map_err(|error| {
                owner_tree_effect_unknown(format!(
                    "an app-owned file was created, but private permissions could not be applied: {}",
                    io::Error::from(error)
                ))
            })?;
            let rebound = statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                owner_tree_effect_unknown(format!(
                    "an app-owned file was created, but its path identity could not be read: {}",
                    io::Error::from(error)
                ))
            })?;
            if FileType::from_raw_mode(rebound.st_mode) != FileType::RegularFile
                || rebound.st_dev as u64 != metadata.dev()
                || rebound.st_ino != metadata.ino()
                || rebound.st_uid != owner_uid
                || rebound.st_nlink != 1
                || rebound.st_mode & 0o077 != 0
            {
                return Err(owner_tree_effect_unknown(
                    "an app-owned file changed before descriptor binding; the replacement was retained"
                        .to_string(),
                ));
            }
            fsync(&parent).map_err(|error| {
                owner_tree_effect_unknown(format!(
                    "app-owned file creation durability could not be verified: {}",
                    io::Error::from(error)
                ))
            })?;
            Ok(file)
        }
        #[cfg(not(unix))]
        {
            let target = guarded_fallback_path(self.lock.owner_path(), relative)?;
            Ok(std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(target)?)
        }
    }

    pub(crate) fn write_private_file_noreplace(
        &self,
        relative: &Path,
        bytes: &[u8],
        label: &str,
    ) -> Result<(), CommandError> {
        let mut file = self.create_private_file(relative)?;
        let partial = |detail: String| {
            owner_tree_effect_unknown(format!(
                "{label} was created before its content could be fully verified: {detail}"
            ))
        };
        file.write_all(bytes)
            .map_err(|error| partial(format!("write failed: {error}")))?;
        file.sync_all()
            .map_err(|error| partial(format!("file durability failed: {error}")))?;
        #[cfg(unix)]
        let expected = removal_binding_from_metadata(
            &file
                .metadata()
                .map_err(|error| partial(format!("descriptor metadata failed: {error}")))?,
        )
        .map_err(|error| partial(format!("descriptor identity is unsafe: {error}")))?;
        let readback = self
            .read_bounded_regular_file(relative, bytes.len() as u64, label)
            .map_err(|error| partial(format!("semantic read-back failed: {error}")))?
            .ok_or_else(|| partial("the created path disappeared".to_string()))?;
        if readback != bytes {
            return Err(partial(
                "the created path does not contain the exact candidate bytes".to_string(),
            ));
        }
        #[cfg(unix)]
        {
            let rebound = self
                .open_regular_file(relative, false, true)
                .map_err(|error| partial(format!("final path rebind failed: {error}")))?;
            let observed = removal_binding_from_metadata(
                &rebound
                    .metadata()
                    .map_err(|error| partial(format!("final path metadata failed: {error}")))?,
            )
            .map_err(|error| partial(format!("final path identity is unsafe: {error}")))?;
            if observed != expected {
                return Err(partial(
                    "the created path changed identity before completion".to_string(),
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn read_regular_file_to_string(
        &self,
        relative: &Path,
        max_bytes: u64,
        label: &str,
    ) -> Result<String, CommandError> {
        let bytes = self
            .read_bounded_regular_file(relative, max_bytes, label)?
            .ok_or_else(|| unsafe_relative_file(label))?;
        String::from_utf8(bytes)
            .map_err(|_| CommandError::UnsafeConfigPath(format!("{label} is not valid UTF-8 text")))
    }

    pub(crate) fn rename(&self, from: &Path, to: &Path) -> Result<(), CommandError> {
        validate_relative_path(from)?;
        validate_relative_path(to)?;
        #[cfg(unix)]
        {
            use rustix::fs::{fsync, statat, AtFlags};

            let (from_parent, from_name) = self.open_private_parent(from)?;
            let (to_parent, to_name) = self.open_private_parent(to)?;
            let owner_uid = private_owner_uid(self.lock)?;
            run_owner_rename_test_hook(to);
            let expected = statat(&from_parent, from_name, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(io::Error::from)?;
            let expected = removal_binding_from_stat(&expected)?;
            if expected.uid != owner_uid || expected.mode & 0o077 != 0 {
                return Err(CommandError::UnsafeConfigPath(
                    "app-owned rename requires an owner-only source owned by the current effective user"
                        .to_string(),
                ));
            }
            #[cfg(any(
                target_vendor = "apple",
                target_os = "android",
                target_os = "linux",
                target_os = "redox"
            ))]
            {
                use rustix::fs::{renameat_with, RenameFlags};

                match renameat_with(
                    &from_parent,
                    from_name,
                    &to_parent,
                    to_name,
                    RenameFlags::NOREPLACE,
                ) {
                    Ok(()) => {}
                    Err(rustix::io::Errno::EXIST) => {
                        return Err(CommandError::StaleActionReference)
                    }
                    Err(error) => return Err(io::Error::from(error).into()),
                }
            }
            #[cfg(not(any(
                target_vendor = "apple",
                target_os = "android",
                target_os = "linux",
                target_os = "redox"
            )))]
            {
                use rustix::fs::{renameat, statat, AtFlags};

                match statat(&to_parent, to_name, AtFlags::SYMLINK_NOFOLLOW) {
                    Err(rustix::io::Errno::NOENT) => {}
                    Ok(_) => return Err(CommandError::StaleActionReference),
                    Err(error) => return Err(io::Error::from(error).into()),
                }
                renameat(&from_parent, from_name, &to_parent, to_name).map_err(io::Error::from)?;
            }
            let moved =
                statat(&to_parent, to_name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                    owner_tree_effect_unknown(format!(
                        "an app-owned tree was renamed, but its target identity could not be read: {}",
                        io::Error::from(error)
                    ))
                })?;
            let moved = removal_binding_from_stat(&moved).map_err(|error| {
                owner_tree_effect_unknown(format!(
                    "an app-owned tree was renamed, but its target entry is unsafe: {error}"
                ))
            })?;
            if !removal_binding_matches_after_rename(expected, moved) {
                return Err(owner_tree_effect_unknown(
                    "an app-owned rename activated a target that does not match the accepted source; the third state was retained"
                        .to_string(),
                ));
            }
            fsync(&from_parent)
                .and_then(|()| fsync(&to_parent))
                .map_err(|error| {
                    owner_tree_effect_unknown(format!(
                        "an app-owned tree was renamed, but directory durability could not be verified: {}",
                        io::Error::from(error)
                    ))
                })?;
            let durable =
                statat(&to_parent, to_name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                    owner_tree_effect_unknown(format!(
                        "an app-owned tree rename was synced, but target identity could not be read: {}",
                        io::Error::from(error)
                    ))
                })?;
            if removal_binding_from_stat(&durable)
                .map_err(|error| owner_tree_effect_unknown(error.to_string()))?
                != moved
            {
                return Err(owner_tree_effect_unknown(
                    "an app-owned rename target changed after directory sync; the replacement was retained"
                        .to_string(),
                ));
            }
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let source = guarded_fallback_path(self.lock.owner_path(), from)?;
            let target = guarded_fallback_path(self.lock.owner_path(), to)?;
            if target.exists() {
                return Err(CommandError::StaleActionReference);
            }
            std::fs::rename(source, target)?;
            Ok(())
        }
    }

    pub(crate) fn remove_tree_if_exists(&self, relative: &Path) -> Result<(), CommandError> {
        validate_relative_path(relative)?;
        #[cfg(unix)]
        {
            use rustix::fs::{fsync, renameat_with, statat, AtFlags, RenameFlags};

            let (parent, name) = self.open_private_parent(relative)?;
            let expected = match statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(metadata) => removal_binding_from_stat(&metadata)?,
                Err(rustix::io::Errno::NOENT) => return Ok(()),
                Err(error) => return Err(io::Error::from(error).into()),
            };
            let owner_uid = private_owner_uid(self.lock)?;
            if expected.uid != owner_uid || expected.mode & 0o077 != 0 {
                return Err(CommandError::UnsafeConfigPath(
                    "app-data removal requires an owner-only entry owned by the current effective user"
                        .to_string(),
                ));
            }
            run_owner_remove_test_hook(relative);
            let quarantine = random_owner_quarantine_name("tree")?;
            renameat_with(&parent, name, &parent, &quarantine, RenameFlags::NOREPLACE).map_err(
                |error| match error {
                    rustix::io::Errno::NOENT | rustix::io::Errno::EXIST => {
                        CommandError::StaleActionReference
                    }
                    other => io::Error::from(other).into(),
                },
            )?;
            let moved =
                statat(&parent, &quarantine, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                    owner_tree_effect_unknown(format!(
                        "an app-owned tree was quarantined, but its identity could not be read: {}",
                        io::Error::from(error)
                    ))
                })?;
            let observed = removal_binding_from_stat(&moved).map_err(|error| {
                owner_tree_effect_unknown(format!(
                    "an app-owned tree was quarantined, but its entry type is unsafe: {error}"
                ))
            })?;
            if !removal_binding_matches_after_rename(expected, observed) {
                return Err(owner_tree_effect_unknown(
                    "an app-owned tree quarantine no longer matches the accepted root; the third state was retained"
                        .to_string(),
                ));
            }
            fsync(&parent).map_err(|error| {
                owner_tree_effect_unknown(format!(
                    "app-owned tree quarantine durability could not be verified: {}",
                    io::Error::from(error)
                ))
            })?;
            remove_quarantined_entry(&parent, &quarantine, observed, owner_uid).map_err(
                |error| match error {
                    partial @ CommandError::PartialEffect { .. } => partial,
                    other => owner_tree_effect_unknown(format!(
                        "the identity-bound app-owned tree quarantine could not be removed: {other}"
                    )),
                },
            )?;
            fsync(&parent).map_err(|error| {
                owner_tree_effect_unknown(format!(
                    "app-owned tree cleanup durability could not be verified: {}",
                    io::Error::from(error)
                ))
            })?;
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let target = guarded_fallback_path(self.lock.owner_path(), relative)?;
            match std::fs::symlink_metadata(&target) {
                Ok(metadata) if metadata.file_type().is_symlink() || metadata.is_file() => {
                    std::fs::remove_file(target)?;
                    Ok(())
                }
                Ok(metadata) if metadata.is_dir() => {
                    std::fs::remove_dir_all(target)?;
                    Ok(())
                }
                Ok(_) => Err(CommandError::UnsafeConfigPath(
                    "app-data removal refuses a special file".to_string(),
                )),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Err(error) => Err(error.into()),
            }
        }
    }

    pub(crate) fn remove_empty_directories(
        &self,
        created_directories: &[AppDataCreatedDirectory],
    ) -> Result<(), CommandError> {
        #[cfg(unix)]
        let mut removed_any = false;
        for created in created_directories.iter().rev() {
            let relative = &created.relative;
            validate_relative_path(relative)?;
            #[cfg(unix)]
            {
                let result = remove_bound_empty_directory(self, created);
                match result {
                    Ok(true) => removed_any = true,
                    Ok(false) => {}
                    Err(error) if removed_any => {
                        return Err(owner_tree_effect_unknown(format!(
                            "an earlier created directory was removed before later cleanup failed: {error}"
                        )))
                    }
                    Err(error) => return Err(error),
                }
            }
            #[cfg(not(unix))]
            {
                let path = guarded_fallback_path(self.lock.owner_path(), relative)?;
                match std::fs::remove_dir(path) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
        }
        Ok(())
    }

    pub(crate) fn snapshot_regular_tree(
        &self,
        relative: &Path,
        max_entries: usize,
        max_total_bytes: u64,
        max_file_bytes: u64,
        label: &str,
    ) -> Result<AppDataTreeSnapshot, CommandError> {
        validate_relative_path(relative)?;
        #[cfg(unix)]
        {
            let root = match self.open_directory(relative) {
                Ok(directory) => directory,
                Err(CommandError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
                    return Ok(AppDataTreeSnapshot {
                        present: false,
                        rows: Vec::new(),
                    })
                }
                Err(error) => return Err(error),
            };
            let owner_uid = private_owner_uid(self.lock)?;
            validate_private_directory(&root, owner_uid)?;
            let mut pending = vec![(PathBuf::new(), root)];
            let mut rows = Vec::new();
            let mut entry_count = 0usize;
            let mut total_bytes = 0u64;
            while let Some((directory_relative, directory)) = pending.pop() {
                let names = directory_entry_names(&directory)?;
                for name in names {
                    entry_count = entry_count.saturating_add(1);
                    if entry_count > max_entries {
                        return Err(CommandError::InvalidSkillManagerRequest(format!(
                            "{label} exceeds its entry safety budget"
                        )));
                    }
                    let child_relative = directory_relative.join(&name);
                    snapshot_child(
                        &directory,
                        &name,
                        child_relative,
                        &mut pending,
                        &mut rows,
                        &mut total_bytes,
                        max_total_bytes,
                        max_file_bytes,
                        owner_uid,
                        label,
                    )?;
                }
            }
            rows.sort();
            Ok(AppDataTreeSnapshot {
                present: true,
                rows,
            })
        }
        #[cfg(not(unix))]
        {
            snapshot_fallback_tree(
                self.lock.owner_path(),
                relative,
                max_entries,
                max_total_bytes,
                max_file_bytes,
                label,
            )
        }
    }

    pub(crate) fn copy_regular_tree(
        &self,
        from: &Path,
        to: &Path,
        max_entries: usize,
        max_total_bytes: u64,
        max_file_bytes: u64,
        label: &str,
    ) -> Result<(), CommandError> {
        validate_relative_path(from)?;
        validate_relative_path(to)?;
        #[cfg(unix)]
        {
            use rustix::fs::{fchmod, fsync, mkdirat, openat, Mode};

            let source = self.open_directory(from)?;
            let owner_uid = private_owner_uid(self.lock)?;
            validate_private_directory(&source, owner_uid)?;
            let (destination_parent, destination_name) = self.open_private_parent(to)?;
            let directory_mode = Mode::from_bits_truncate(0o700);
            mkdirat(&destination_parent, destination_name, directory_mode)
                .map_err(io::Error::from)?;
            let destination = openat(
                &destination_parent,
                destination_name,
                directory_flags(),
                Mode::empty(),
            )
            .map(File::from)
            .map_err(|error| {
                owner_tree_effect_unknown(format!(
                    "an app-owned copy destination was created, but its descriptor could not be opened: {}",
                    io::Error::from(error)
                ))
            })?;
            let copy_result = (|| {
                fchmod(&destination, directory_mode).map_err(io::Error::from)?;
                validate_private_directory(&destination, owner_uid)?;
                fsync(&destination).map_err(io::Error::from)?;
                fsync(&destination_parent).map_err(io::Error::from)?;
                let mut entry_count = 0usize;
                let mut total_bytes = 0u64;
                copy_directory_contents(
                    &source,
                    &destination,
                    &mut entry_count,
                    &mut total_bytes,
                    max_entries,
                    max_total_bytes,
                    max_file_bytes,
                    owner_uid,
                    label,
                )
            })();
            if let Err(error) = copy_result {
                return match self.remove_tree_if_bound(to, &destination) {
                    Ok(()) => Err(error),
                    Err(cleanup) => Err(owner_tree_effect_unknown(format!(
                        "app-owned tree copy failed ({error}); identity-bound destination cleanup failed ({cleanup})"
                    ))),
                };
            }
            self.validate_directory_binding(to, &destination)
                .map_err(|error| {
                    owner_tree_effect_unknown(format!(
                        "app-owned tree copy completed, but the destination path no longer binds its created directory: {error}"
                    ))
                })?;
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let source = guarded_fallback_path(self.lock.owner_path(), from)?;
            let destination = guarded_fallback_path(self.lock.owner_path(), to)?;
            std::fs::create_dir(&destination)?;
            let result = copy_fallback_directory(
                &source,
                &destination,
                max_entries,
                max_total_bytes,
                max_file_bytes,
                label,
            );
            match result {
                Ok(()) => Ok(()),
                Err(error) => match std::fs::remove_dir_all(destination) {
                    Ok(()) => Err(error),
                    Err(cleanup) if cleanup.kind() == io::ErrorKind::NotFound => Err(error),
                    Err(cleanup) => Err(owner_tree_effect_unknown(format!(
                        "app-owned tree copy failed ({error}); destination cleanup failed ({cleanup})"
                    ))),
                },
            }
        }
    }

    #[cfg(unix)]
    fn remove_tree_if_bound(&self, relative: &Path, accepted: &File) -> Result<(), CommandError> {
        use rustix::fs::{fsync, renameat_with, statat, AtFlags, RenameFlags};
        use std::os::unix::fs::MetadataExt;

        let accepted = accepted.metadata()?;
        let owner_uid = private_owner_uid(self.lock)?;
        let (parent, name) = self.open_private_parent(relative)?;
        let current = statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
        let current = removal_binding_from_stat(&current)?;
        if current.device != accepted.dev()
            || current.inode != accepted.ino()
            || current.uid != accepted.uid()
            || current.uid != owner_uid
            || current.mode & 0o077 != 0
        {
            return Err(CommandError::StaleActionReference);
        }
        let quarantine = random_owner_quarantine_name("bound-tree")?;
        renameat_with(&parent, name, &parent, &quarantine, RenameFlags::NOREPLACE).map_err(
            |error| match error {
                rustix::io::Errno::NOENT | rustix::io::Errno::EXIST => {
                    CommandError::StaleActionReference
                }
                other => io::Error::from(other).into(),
            },
        )?;
        let moved = statat(&parent, &quarantine, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
            owner_tree_effect_unknown(format!(
                "bound tree was quarantined, but its identity could not be read: {}",
                io::Error::from(error)
            ))
        })?;
        let moved = removal_binding_from_stat(&moved)
            .map_err(|error| owner_tree_effect_unknown(error.to_string()))?;
        if moved.device != accepted.dev()
            || moved.inode != accepted.ino()
            || moved.uid != accepted.uid()
            || moved.mode & 0o077 != 0
        {
            return Err(owner_tree_effect_unknown(
                "a bound tree quarantine no longer matches the retained descriptor; the third state was retained"
                    .to_string(),
            ));
        }
        fsync(&parent).map_err(|error| {
            owner_tree_effect_unknown(format!(
                "bound tree quarantine durability could not be verified: {}",
                io::Error::from(error)
            ))
        })?;
        remove_quarantined_entry(&parent, &quarantine, moved, owner_uid).map_err(|error| {
            owner_tree_effect_unknown(format!(
                "bound tree quarantine could not be removed: {error}"
            ))
        })
    }
}

#[cfg(unix)]
fn directory_entry_names(directory: &File) -> Result<Vec<OsString>, CommandError> {
    use std::os::unix::ffi::OsStrExt;

    use rustix::fs::Dir;

    let mut entries = Dir::read_from(directory).map_err(io::Error::from)?;
    let mut names = Vec::new();
    for entry in &mut entries {
        let entry = entry.map_err(io::Error::from)?;
        let bytes = entry.file_name().to_bytes();
        if matches!(bytes, b"." | b"..") {
            continue;
        }
        names.push(OsStr::from_bytes(bytes).to_owned());
    }
    names.sort();
    Ok(names)
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn snapshot_child(
    directory: &File,
    name: &OsStr,
    child_relative: PathBuf,
    pending: &mut Vec<(PathBuf, File)>,
    rows: &mut Vec<String>,
    total_bytes: &mut u64,
    max_total_bytes: u64,
    max_file_bytes: u64,
    owner_uid: u32,
    label: &str,
) -> Result<(), CommandError> {
    use std::os::unix::fs::MetadataExt;

    use rustix::fs::{openat, statat, AtFlags, FileType, Mode, OFlags};

    let metadata = statat(directory, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
    let relative_text = child_relative.to_string_lossy();
    match FileType::from_raw_mode(metadata.st_mode) {
        FileType::Directory => {
            if metadata.st_uid != owner_uid || metadata.st_mode & 0o077 != 0 {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "{label} contains a foreign-owned or non-private directory"
                )));
            }
            let child = openat(directory, name, directory_flags(), Mode::empty())
                .map_err(map_unsafe_relative_errno)?;
            let child = File::from(child);
            let opened = child.metadata()?;
            if opened.dev() != metadata.st_dev as u64 || opened.ino() != metadata.st_ino {
                return Err(CommandError::StaleActionReference);
            }
            validate_private_directory(&child, owner_uid)?;
            rows.push(format!("dir:{relative_text}"));
            pending.push((child_relative, child));
        }
        FileType::RegularFile => {
            let path_binding = removal_binding_from_stat(&metadata)?;
            let descriptor = openat(
                directory,
                name,
                OFlags::RDONLY
                    | OFlags::NOFOLLOW
                    | OFlags::NONBLOCK
                    | OFlags::NOCTTY
                    | OFlags::CLOEXEC,
                Mode::empty(),
            )
            .map_err(io::Error::from)?;
            let mut file = File::from(descriptor);
            let file_metadata = file.metadata()?;
            let opened_binding = removal_binding_from_metadata(&file_metadata)?;
            if !file_metadata.is_file()
                || file_metadata.uid() != owner_uid
                || file_metadata.nlink() != 1
                || file_metadata.mode() & 0o077 != 0
                || opened_binding != path_binding
            {
                return Err(unsafe_relative_file(label));
            }
            if file_metadata.len() > max_file_bytes {
                return Err(CommandError::InvalidSkillManagerRequest(format!(
                    "{label} contains a file beyond its safety budget"
                )));
            }
            *total_bytes = total_bytes
                .checked_add(file_metadata.len())
                .filter(|total| *total <= max_total_bytes)
                .ok_or_else(|| {
                    CommandError::InvalidSkillManagerRequest(format!(
                        "{label} exceeds its byte safety budget"
                    ))
                })?;
            let mut bytes = Vec::with_capacity(file_metadata.len() as usize);
            Read::by_ref(&mut file)
                .take(max_file_bytes.saturating_add(1))
                .read_to_end(&mut bytes)?;
            let after_binding = removal_binding_from_metadata(&file.metadata()?)?;
            let rebound =
                statat(directory, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
            let rebound_binding = removal_binding_from_stat(&rebound)?;
            if bytes.len() as u64 != file_metadata.len()
                || bytes.len() as u64 > max_file_bytes
                || after_binding != opened_binding
                || rebound_binding != opened_binding
            {
                return Err(CommandError::StaleActionReference);
            }
            rows.push(format!(
                "file:{relative_text}:{}:{:x}",
                file_metadata.len(),
                sha2::Sha256::digest(&bytes)
            ));
        }
        FileType::Symlink => {
            return Err(CommandError::UnsafeConfigPath(format!(
                "{label} refuses symlinked content"
            )))
        }
        _ => {
            return Err(CommandError::UnsafeConfigPath(format!(
                "{label} refuses special files"
            )))
        }
    }
    Ok(())
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RemovalEntryKind {
    Directory,
    RegularFile,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct RemovalBinding {
    kind: RemovalEntryKind,
    device: u64,
    inode: u64,
    uid: u32,
    mode: u32,
    links: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
fn removal_binding_from_stat(stat: &rustix::fs::Stat) -> Result<RemovalBinding, CommandError> {
    use rustix::fs::FileType;

    let kind = match FileType::from_raw_mode(stat.st_mode) {
        FileType::Directory => RemovalEntryKind::Directory,
        FileType::RegularFile if stat.st_nlink == 1 => RemovalEntryKind::RegularFile,
        FileType::RegularFile => {
            return Err(CommandError::UnsafeConfigPath(
                "app-data removal refuses a hard-linked file".to_string(),
            ))
        }
        FileType::Symlink => {
            return Err(CommandError::UnsafeConfigPath(
                "app-data removal refuses a symlink".to_string(),
            ))
        }
        _ => {
            return Err(CommandError::UnsafeConfigPath(
                "app-data removal refuses a special file".to_string(),
            ))
        }
    };
    Ok(RemovalBinding {
        kind,
        device: unix_device_id(stat.st_dev),
        inode: stat.st_ino,
        uid: stat.st_uid,
        mode: unix_file_mode(stat.st_mode),
        links: unix_link_count(stat.st_nlink),
        length: u64::try_from(stat.st_size)
            .map_err(|_| unsafe_relative_file("app-data removal"))?,
        modified_seconds: stat.st_mtime,
        modified_nanoseconds: unix_timestamp_nanoseconds(stat.st_mtime_nsec),
        changed_seconds: stat.st_ctime,
        changed_nanoseconds: unix_timestamp_nanoseconds(stat.st_ctime_nsec),
    })
}

#[cfg(unix)]
fn removal_binding_from_metadata(
    metadata: &std::fs::Metadata,
) -> Result<RemovalBinding, CommandError> {
    use std::os::unix::fs::MetadataExt;

    let kind = if metadata.is_dir() {
        RemovalEntryKind::Directory
    } else if metadata.is_file() && metadata.nlink() == 1 {
        RemovalEntryKind::RegularFile
    } else if metadata.is_file() {
        return Err(CommandError::UnsafeConfigPath(
            "app-data tree rejects a hard-linked file".to_string(),
        ));
    } else {
        return Err(CommandError::UnsafeConfigPath(
            "app-data tree rejects a symlink or special file".to_string(),
        ));
    };
    Ok(RemovalBinding {
        kind,
        device: metadata.dev(),
        inode: metadata.ino(),
        uid: metadata.uid(),
        mode: metadata.mode(),
        links: metadata.nlink(),
        length: metadata.len(),
        modified_seconds: metadata.mtime(),
        modified_nanoseconds: metadata.mtime_nsec(),
        changed_seconds: metadata.ctime(),
        changed_nanoseconds: metadata.ctime_nsec(),
    })
}

#[cfg(unix)]
fn removal_binding_matches_after_rename(
    expected: RemovalBinding,
    observed: RemovalBinding,
) -> bool {
    expected.kind == observed.kind
        && expected.device == observed.device
        && expected.inode == observed.inode
        && expected.uid == observed.uid
        && expected.mode == observed.mode
        && expected.links == observed.links
        && expected.length == observed.length
        && expected.modified_seconds == observed.modified_seconds
        && expected.modified_nanoseconds == observed.modified_nanoseconds
}

#[cfg(unix)]
fn random_owner_quarantine_name(stem: &str) -> Result<OsString, CommandError> {
    let mut nonce = [0_u8; 16];
    getrandom::getrandom(&mut nonce).map_err(|error| {
        io::Error::other(format!("secure random name generation failed: {error}"))
    })?;
    let encoded = nonce
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    Ok(OsString::from(format!(
        ".agent-copilot-{stem}-quarantine.{encoded}"
    )))
}

#[cfg(unix)]
fn remove_bound_empty_directory(
    owner: &AppDataOwnerFs<'_>,
    expected: &AppDataCreatedDirectory,
) -> Result<bool, CommandError> {
    use std::os::unix::fs::MetadataExt;

    use rustix::fs::{
        fsync, openat, renameat_with, statat, unlinkat, AtFlags, FileType, Mode, RenameFlags,
    };

    let (parent, name) = owner.open_private_parent(&expected.relative)?;
    let current = match statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW) {
        Ok(current) => current,
        Err(rustix::io::Errno::NOENT) => return Ok(false),
        Err(error) => return Err(io::Error::from(error).into()),
    };
    if FileType::from_raw_mode(current.st_mode) != FileType::Directory
        || current.st_dev as u64 != expected.device
        || current.st_ino != expected.inode
        || current.st_uid != expected.uid
        || current.st_mode as u32 != expected.mode
    {
        return Err(CommandError::StaleActionReference);
    }
    let quarantine = random_owner_quarantine_name("empty-directory")?;
    renameat_with(&parent, name, &parent, &quarantine, RenameFlags::NOREPLACE).map_err(
        |error| match error {
            rustix::io::Errno::NOENT | rustix::io::Errno::EXIST => {
                CommandError::StaleActionReference
            }
            other => io::Error::from(other).into(),
        },
    )?;
    let partial = |detail: String| {
        owner_tree_effect_unknown(format!(
            "a created app-data directory was quarantined before cleanup; {detail}"
        ))
    };
    let moved = statat(&parent, &quarantine, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
        partial(format!(
            "its identity could not be read: {}",
            io::Error::from(error)
        ))
    })?;
    if FileType::from_raw_mode(moved.st_mode) != FileType::Directory
        || moved.st_dev as u64 != expected.device
        || moved.st_ino != expected.inode
        || moved.st_uid != expected.uid
        || moved.st_mode as u32 != expected.mode
    {
        return Err(partial(
            "the quarantine no longer matches the created directory; the third state was retained"
                .to_string(),
        ));
    }
    fsync(&parent).map_err(|error| {
        partial(format!(
            "quarantine durability failed: {}",
            io::Error::from(error)
        ))
    })?;
    run_owner_quarantine_unlink_test_hook();
    let directory = openat(&parent, &quarantine, directory_flags(), Mode::empty())
        .map(File::from)
        .map_err(|error| {
            partial(format!(
                "the quarantine could not be opened: {}",
                io::Error::from(error)
            ))
        })?;
    let descriptor = directory.metadata().map_err(|error| {
        partial(format!(
            "the quarantine descriptor could not be inspected: {error}"
        ))
    })?;
    if descriptor.dev() != expected.device
        || descriptor.ino() != expected.inode
        || descriptor.uid() != expected.uid
        || !directory_entry_names(&directory)
            .map_err(|error| partial(format!("the quarantine could not be enumerated: {error}")))?
            .is_empty()
    {
        return Err(partial(
            "the created-directory quarantine changed or is no longer empty; it was retained"
                .to_string(),
        ));
    }
    let before_unlink =
        statat(&parent, &quarantine, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
            partial(format!(
                "final identity recheck failed: {}",
                io::Error::from(error)
            ))
        })?;
    if before_unlink.st_dev as u64 != expected.device
        || before_unlink.st_ino != expected.inode
        || before_unlink.st_uid != expected.uid
        || FileType::from_raw_mode(before_unlink.st_mode) != FileType::Directory
    {
        return Err(partial(
            "the quarantine changed before removal; the replacement was retained".to_string(),
        ));
    }
    unlinkat(&parent, &quarantine, AtFlags::REMOVEDIR).map_err(|error| {
        partial(format!(
            "the bound empty quarantine could not be removed: {}",
            io::Error::from(error)
        ))
    })?;
    fsync(&parent).map_err(|error| {
        partial(format!(
            "empty-directory removal durability failed: {}",
            io::Error::from(error)
        ))
    })?;
    Ok(true)
}

#[cfg(unix)]
fn remove_quarantined_entry(
    parent: &File,
    name: &OsStr,
    expected: RemovalBinding,
    owner_uid: u32,
) -> Result<(), CommandError> {
    use std::os::unix::fs::MetadataExt;

    use rustix::fs::{
        fsync, openat, renameat_with, statat, unlinkat, AtFlags, FileType, Mode, RenameFlags,
    };

    if expected.uid != owner_uid || expected.mode & 0o077 != 0 {
        return Err(CommandError::UnsafeConfigPath(
            "app-data removal requires owner-only entries owned by the current effective user"
                .to_string(),
        ));
    }
    match expected.kind {
        RemovalEntryKind::RegularFile => {
            let current =
                statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
            let current = removal_binding_from_stat(&current)?;
            if current != expected {
                return Err(CommandError::StaleActionReference);
            }
            run_owner_quarantine_unlink_test_hook();
            let final_current =
                statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
            if removal_binding_from_stat(&final_current)? != expected {
                return Err(CommandError::StaleActionReference);
            }
            unlinkat(parent, name, AtFlags::empty()).map_err(io::Error::from)?;
            fsync(parent).map_err(io::Error::from)?;
        }
        RemovalEntryKind::Directory => {
            let directory = openat(parent, name, directory_flags(), Mode::empty())
                .map_err(map_unsafe_relative_errno)?;
            let directory = File::from(directory);
            let metadata = directory.metadata()?;
            if metadata.dev() != expected.device
                || metadata.ino() != expected.inode
                || metadata.uid() != expected.uid
                || metadata.mode() & 0o077 != 0
            {
                return Err(CommandError::StaleActionReference);
            }
            for child_name in directory_entry_names(&directory)? {
                let before = statat(&directory, &child_name, AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(io::Error::from)?;
                let before = removal_binding_from_stat(&before)?;
                if before.uid != owner_uid || before.mode & 0o077 != 0 {
                    return Err(CommandError::UnsafeConfigPath(
                        "app-data removal requires owner-only entries owned by the current effective user"
                            .to_string(),
                    ));
                }
                let quarantine = random_owner_quarantine_name("entry")?;
                renameat_with(
                    &directory,
                    &child_name,
                    &directory,
                    &quarantine,
                    RenameFlags::NOREPLACE,
                )
                .map_err(|error| match error {
                    rustix::io::Errno::NOENT | rustix::io::Errno::EXIST => {
                        CommandError::StaleActionReference
                    }
                    other => io::Error::from(other).into(),
                })?;
                let moved = statat(&directory, &quarantine, AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(io::Error::from)?;
                let moved = removal_binding_from_stat(&moved)?;
                if !removal_binding_matches_after_rename(before, moved) {
                    return Err(CommandError::StaleActionReference);
                }
                fsync(&directory).map_err(io::Error::from)?;
                remove_quarantined_entry(&directory, &quarantine, moved, owner_uid)?;
            }
            if !directory_entry_names(&directory)?.is_empty() {
                return Err(CommandError::StaleActionReference);
            }
            let final_path =
                statat(parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
            if FileType::from_raw_mode(final_path.st_mode) != FileType::Directory
                || final_path.st_dev as u64 != expected.device
                || final_path.st_ino != expected.inode
                || final_path.st_uid != expected.uid
            {
                return Err(CommandError::StaleActionReference);
            }
            unlinkat(parent, name, AtFlags::REMOVEDIR).map_err(io::Error::from)?;
            fsync(parent).map_err(io::Error::from)?;
        }
    }
    Ok(())
}

fn owner_tree_effect_unknown(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "app-data tree cleanup".to_string(),
        state: "outcome_unknown",
        cleanup_required: true,
        detail,
    }
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn copy_directory_contents(
    source: &File,
    destination: &File,
    entry_count: &mut usize,
    total_bytes: &mut u64,
    max_entries: usize,
    max_total_bytes: u64,
    max_file_bytes: u64,
    owner_uid: u32,
    label: &str,
) -> Result<(), CommandError> {
    use std::os::unix::fs::MetadataExt;

    use rustix::fs::{fchmod, fsync, mkdirat, openat, statat, AtFlags, FileType, Mode, OFlags};

    let directory_mode = Mode::from_bits_truncate(0o700);
    let file_mode = Mode::from_bits_truncate(0o600);
    for name in directory_entry_names(source)? {
        *entry_count = entry_count.saturating_add(1);
        if *entry_count > max_entries {
            return Err(CommandError::InvalidSkillManagerRequest(format!(
                "{label} exceeds its entry safety budget"
            )));
        }
        let metadata = statat(source, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
        match FileType::from_raw_mode(metadata.st_mode) {
            FileType::Directory => {
                if metadata.st_uid != owner_uid || metadata.st_mode & 0o077 != 0 {
                    return Err(CommandError::UnsafeConfigPath(format!(
                        "{label} contains a foreign-owned or non-private directory"
                    )));
                }
                mkdirat(destination, &name, directory_mode).map_err(io::Error::from)?;
                let created = statat(destination, &name, AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(io::Error::from)?;
                if FileType::from_raw_mode(created.st_mode) != FileType::Directory
                    || created.st_uid != owner_uid
                {
                    return Err(CommandError::StaleActionReference);
                }
                let source_child = openat(source, &name, directory_flags(), Mode::empty())
                    .map_err(map_unsafe_relative_errno)?;
                let source_child = File::from(source_child);
                let source_child_metadata = source_child.metadata()?;
                if source_child_metadata.dev() != metadata.st_dev as u64
                    || source_child_metadata.ino() != metadata.st_ino
                    || source_child_metadata.uid() != owner_uid
                    || source_child_metadata.mode() & 0o077 != 0
                {
                    return Err(CommandError::StaleActionReference);
                }
                let destination_child =
                    openat(destination, &name, directory_flags(), Mode::empty())
                        .map_err(map_unsafe_relative_errno)?;
                let destination_child = File::from(destination_child);
                let destination_metadata = destination_child.metadata()?;
                if destination_metadata.dev() != created.st_dev as u64
                    || destination_metadata.ino() != created.st_ino
                    || destination_metadata.uid() != owner_uid
                {
                    return Err(CommandError::StaleActionReference);
                }
                fchmod(&destination_child, directory_mode).map_err(io::Error::from)?;
                validate_private_directory(&destination_child, owner_uid)?;
                fsync(&destination_child)
                    .and_then(|()| fsync(destination))
                    .map_err(io::Error::from)?;
                copy_directory_contents(
                    &source_child,
                    &destination_child,
                    entry_count,
                    total_bytes,
                    max_entries,
                    max_total_bytes,
                    max_file_bytes,
                    owner_uid,
                    label,
                )?;
            }
            FileType::RegularFile => {
                let path_binding = removal_binding_from_stat(&metadata)?;
                let source_descriptor = openat(
                    source,
                    &name,
                    OFlags::RDONLY
                        | OFlags::NOFOLLOW
                        | OFlags::NONBLOCK
                        | OFlags::NOCTTY
                        | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(io::Error::from)?;
                let mut source_file = File::from(source_descriptor);
                let source_metadata = source_file.metadata()?;
                let opened_binding = removal_binding_from_metadata(&source_metadata)?;
                if !source_metadata.is_file()
                    || source_metadata.uid() != owner_uid
                    || source_metadata.nlink() != 1
                    || source_metadata.mode() & 0o077 != 0
                    || opened_binding != path_binding
                {
                    return Err(unsafe_relative_file(label));
                }
                if source_metadata.len() > max_file_bytes {
                    return Err(CommandError::InvalidSkillManagerRequest(format!(
                        "{label} contains a file beyond its safety budget"
                    )));
                }
                *total_bytes = total_bytes
                    .checked_add(source_metadata.len())
                    .filter(|total| *total <= max_total_bytes)
                    .ok_or_else(|| {
                        CommandError::InvalidSkillManagerRequest(format!(
                            "{label} exceeds its byte safety budget"
                        ))
                    })?;
                let mut source_bytes = Vec::with_capacity(source_metadata.len() as usize);
                Read::by_ref(&mut source_file)
                    .take(max_file_bytes.saturating_add(1))
                    .read_to_end(&mut source_bytes)?;
                let after_binding = removal_binding_from_metadata(&source_file.metadata()?)?;
                let rebound =
                    statat(source, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
                if source_bytes.len() as u64 != source_metadata.len()
                    || source_bytes.len() as u64 > max_file_bytes
                    || after_binding != opened_binding
                    || removal_binding_from_stat(&rebound)? != opened_binding
                {
                    return Err(CommandError::StaleActionReference);
                }
                let destination_descriptor = openat(
                    destination,
                    &name,
                    OFlags::WRONLY
                        | OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    file_mode,
                )
                .map_err(io::Error::from)?;
                let mut destination_file = File::from(destination_descriptor);
                let created_metadata = destination_file.metadata()?;
                if !created_metadata.is_file()
                    || created_metadata.uid() != owner_uid
                    || created_metadata.nlink() != 1
                {
                    return Err(CommandError::StaleActionReference);
                }
                fchmod(&destination_file, file_mode).map_err(io::Error::from)?;
                let destination_binding =
                    removal_binding_from_metadata(&destination_file.metadata()?)?;
                let destination_path = statat(destination, &name, AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(io::Error::from)?;
                if removal_binding_from_stat(&destination_path)? != destination_binding {
                    return Err(CommandError::StaleActionReference);
                }
                destination_file.write_all(&source_bytes)?;
                destination_file.flush()?;
                destination_file.sync_all()?;
                let destination_read = openat(
                    destination,
                    &name,
                    OFlags::RDONLY
                        | OFlags::NOFOLLOW
                        | OFlags::NONBLOCK
                        | OFlags::NOCTTY
                        | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(io::Error::from)?;
                let mut destination_read = File::from(destination_read);
                let final_binding_before =
                    removal_binding_from_metadata(&destination_read.metadata()?)?;
                let mut copied_bytes = Vec::with_capacity(source_bytes.len());
                Read::by_ref(&mut destination_read)
                    .take(max_file_bytes.saturating_add(1))
                    .read_to_end(&mut copied_bytes)?;
                let final_binding_after =
                    removal_binding_from_metadata(&destination_read.metadata()?)?;
                let final_path = statat(destination, &name, AtFlags::SYMLINK_NOFOLLOW)
                    .map_err(io::Error::from)?;
                if copied_bytes != source_bytes
                    || final_binding_after != final_binding_before
                    || removal_binding_from_stat(&final_path)? != final_binding_before
                {
                    return Err(CommandError::StaleActionReference);
                }
                fsync(destination).map_err(io::Error::from)?;
            }
            FileType::Symlink => {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "{label} refuses symlinked content"
                )))
            }
            _ => {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "{label} refuses special files"
                )))
            }
        }
    }
    fsync(destination).map_err(io::Error::from)?;
    Ok(())
}

#[cfg(not(unix))]
fn snapshot_fallback_tree(
    owner_path: &Path,
    relative: &Path,
    max_entries: usize,
    max_total_bytes: u64,
    max_file_bytes: u64,
    label: &str,
) -> Result<AppDataTreeSnapshot, CommandError> {
    use sha2::Digest;

    let root = guarded_fallback_path(owner_path, relative)?;
    let metadata = match std::fs::symlink_metadata(&root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(AppDataTreeSnapshot {
                present: false,
                rows: Vec::new(),
            })
        }
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(unsafe_relative_path());
    }
    let mut pending = vec![root.clone()];
    let mut rows = Vec::new();
    let mut total_bytes = 0u64;
    while let Some(directory) = pending.pop() {
        let mut children = std::fs::read_dir(&directory)?.collect::<Result<Vec<_>, _>>()?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            if rows.len() >= max_entries {
                return Err(CommandError::InvalidSkillManagerRequest(format!(
                    "{label} exceeds its entry safety budget"
                )));
            }
            let path = child.path();
            let metadata = std::fs::symlink_metadata(&path)?;
            let relative = path
                .strip_prefix(&root)
                .map_err(|_| unsafe_relative_path())?
                .to_string_lossy();
            if metadata.file_type().is_symlink() {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "{label} refuses symlinked content"
                )));
            }
            if metadata.is_dir() {
                rows.push(format!("dir:{relative}"));
                pending.push(path);
            } else if metadata.is_file() {
                if metadata.len() > max_file_bytes {
                    return Err(CommandError::InvalidSkillManagerRequest(format!(
                        "{label} contains a file beyond its safety budget"
                    )));
                }
                total_bytes = total_bytes
                    .checked_add(metadata.len())
                    .filter(|total| *total <= max_total_bytes)
                    .ok_or_else(|| {
                        CommandError::InvalidSkillManagerRequest(format!(
                            "{label} exceeds its byte safety budget"
                        ))
                    })?;
                rows.push(format!(
                    "file:{relative}:{}:{:x}",
                    metadata.len(),
                    sha2::Sha256::digest(std::fs::read(&path)?)
                ));
            } else {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "{label} refuses special files"
                )));
            }
        }
    }
    rows.sort();
    Ok(AppDataTreeSnapshot {
        present: true,
        rows,
    })
}

#[cfg(not(unix))]
fn copy_fallback_directory(
    source: &Path,
    destination: &Path,
    max_entries: usize,
    max_total_bytes: u64,
    max_file_bytes: u64,
    label: &str,
) -> Result<(), CommandError> {
    let mut pending = vec![(source.to_path_buf(), destination.to_path_buf())];
    let mut entry_count = 0usize;
    let mut total_bytes = 0u64;
    while let Some((source_directory, destination_directory)) = pending.pop() {
        let mut children = std::fs::read_dir(&source_directory)?.collect::<Result<Vec<_>, _>>()?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            entry_count = entry_count.saturating_add(1);
            if entry_count > max_entries {
                return Err(CommandError::InvalidSkillManagerRequest(format!(
                    "{label} exceeds its entry safety budget"
                )));
            }
            let metadata = std::fs::symlink_metadata(child.path())?;
            if metadata.file_type().is_symlink() {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "{label} refuses symlinked content"
                )));
            }
            let destination_child = destination_directory.join(child.file_name());
            if metadata.is_dir() {
                std::fs::create_dir(&destination_child)?;
                pending.push((child.path(), destination_child));
            } else if metadata.is_file() {
                if metadata.len() > max_file_bytes {
                    return Err(CommandError::InvalidSkillManagerRequest(format!(
                        "{label} contains a file beyond its safety budget"
                    )));
                }
                total_bytes = total_bytes
                    .checked_add(metadata.len())
                    .filter(|total| *total <= max_total_bytes)
                    .ok_or_else(|| {
                        CommandError::InvalidSkillManagerRequest(format!(
                            "{label} exceeds its byte safety budget"
                        ))
                    })?;
                std::fs::copy(child.path(), destination_child)?;
            } else {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "{label} refuses special files"
                )));
            }
        }
    }
    Ok(())
}
