use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Read, Write},
    path::{Path, PathBuf},
};

use sha2::Digest;

use super::{
    directory_flags, map_unsafe_relative_errno, unsafe_relative_file, validate_relative_path,
    AppDataOwnerFs,
};
#[cfg(not(unix))]
use super::{guarded_fallback_path, unsafe_relative_path};
use crate::CommandError;

#[cfg(test)]
struct OwnerRenameTestHook {
    target: PathBuf,
    action: Box<dyn FnOnce() + Send>,
}

#[cfg(test)]
static OWNER_RENAME_TEST_HOOK: std::sync::Mutex<Option<OwnerRenameTestHook>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
pub(super) fn install_owner_rename_test_hook(
    target: PathBuf,
    action: impl FnOnce() + Send + 'static,
) {
    let mut hook = OWNER_RENAME_TEST_HOOK
        .lock()
        .expect("lock owner rename hook");
    assert!(hook.is_none(), "owner rename hook already set");
    *hook = Some(OwnerRenameTestHook {
        target,
        action: Box::new(action),
    });
}

#[cfg(test)]
fn run_owner_rename_test_hook(target: &Path) {
    let action = {
        let mut hook = OWNER_RENAME_TEST_HOOK
            .lock()
            .expect("lock owner rename hook");
        if hook.as_ref().is_some_and(|hook| hook.target == target) {
            hook.take().map(|hook| hook.action)
        } else {
            None
        }
    };
    if let Some(action) = action {
        action();
    }
}

#[cfg(not(test))]
fn run_owner_rename_test_hook(_target: &Path) {}

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
            self.open_directory(relative).map_err(Into::into)
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

    pub(crate) fn create_directory(&self, relative: &Path) -> Result<(), CommandError> {
        validate_relative_path(relative)?;
        #[cfg(unix)]
        {
            use rustix::fs::{fchmod, fsync, mkdirat, openat, Mode};

            let (parent, name) = self.open_parent(relative, false)?;
            let mode = Mode::from_bits_truncate(0o700);
            mkdirat(&parent, name, mode).map_err(io::Error::from)?;
            fsync(&parent).map_err(io::Error::from)?;
            let directory = openat(&parent, name, directory_flags(), Mode::empty())
                .map_err(map_unsafe_relative_errno)?;
            fchmod(&directory, mode).map_err(io::Error::from)?;
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let target = guarded_fallback_path(self.lock.owner_path(), relative)?;
            std::fs::create_dir(target)?;
            Ok(())
        }
    }

    pub(crate) fn create_private_file(&self, relative: &Path) -> Result<File, CommandError> {
        validate_relative_path(relative)?;
        #[cfg(unix)]
        {
            use rustix::fs::{fchmod, openat, Mode, OFlags};

            let (parent, name) = self.open_parent(relative, true)?;
            let mode = Mode::from_bits_truncate(0o600);
            let descriptor = openat(
                &parent,
                name,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                mode,
            )
            .map_err(io::Error::from)?;
            let file = File::from(descriptor);
            fchmod(&file, mode).map_err(io::Error::from)?;
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
            use rustix::fs::fsync;

            let (from_parent, from_name) = self.open_parent(from, false)?;
            let (to_parent, to_name) = self.open_parent(to, false)?;
            run_owner_rename_test_hook(to);
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
            fsync(&from_parent).map_err(io::Error::from)?;
            fsync(&to_parent).map_err(io::Error::from)?;
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
            use rustix::fs::{fsync, openat, statat, unlinkat, AtFlags, FileType, Mode};

            let (parent, name) = self.open_parent(relative, false)?;
            let metadata = match statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(metadata) => metadata,
                Err(rustix::io::Errno::NOENT) => return Ok(()),
                Err(error) => return Err(io::Error::from(error).into()),
            };
            match FileType::from_raw_mode(metadata.st_mode) {
                FileType::Directory => {
                    let directory = openat(&parent, name, directory_flags(), Mode::empty())
                        .map_err(map_unsafe_relative_errno)?;
                    remove_directory_contents(&File::from(directory))?;
                    unlinkat(&parent, name, AtFlags::REMOVEDIR).map_err(io::Error::from)?;
                }
                FileType::RegularFile | FileType::Symlink => {
                    unlinkat(&parent, name, AtFlags::empty()).map_err(io::Error::from)?;
                }
                _ => {
                    return Err(CommandError::UnsafeConfigPath(
                        "app-data removal refuses a special file".to_string(),
                    ))
                }
            }
            fsync(&parent).map_err(io::Error::from)?;
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
        relative_paths: &[PathBuf],
    ) -> Result<(), CommandError> {
        for relative in relative_paths.iter().rev() {
            validate_relative_path(relative)?;
            #[cfg(unix)]
            {
                use rustix::fs::{fsync, unlinkat, AtFlags};
                let (parent, name) = self.open_parent(relative, false)?;
                match unlinkat(&parent, name, AtFlags::REMOVEDIR) {
                    Ok(()) => fsync(&parent).map_err(io::Error::from)?,
                    Err(rustix::io::Errno::NOENT) => {}
                    Err(error) => return Err(io::Error::from(error).into()),
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
            use std::os::unix::fs::MetadataExt;

            let root = match self.open_directory(relative) {
                Ok(directory) => directory,
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    return Ok(AppDataTreeSnapshot {
                        present: false,
                        rows: Vec::new(),
                    })
                }
                Err(error) => return Err(error.into()),
            };
            let mut pending = vec![(PathBuf::new(), root)];
            let mut rows = Vec::new();
            let mut entry_count = 0usize;
            let mut total_bytes = 0u64;
            let owner_uid = self.lock.owner_directory().metadata()?.uid();
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
            use std::os::unix::fs::MetadataExt;

            use rustix::fs::{fchmod, fsync, mkdirat, openat, Mode};

            let source = self.open_directory(from)?;
            let (destination_parent, destination_name) = self.open_parent(to, true)?;
            let directory_mode = Mode::from_bits_truncate(0o700);
            mkdirat(&destination_parent, destination_name, directory_mode)
                .map_err(io::Error::from)?;
            fsync(&destination_parent).map_err(io::Error::from)?;
            let destination = openat(
                &destination_parent,
                destination_name,
                directory_flags(),
                Mode::empty(),
            )
            .map_err(map_unsafe_relative_errno)?;
            fchmod(&destination, directory_mode).map_err(io::Error::from)?;
            let mut entry_count = 0usize;
            let mut total_bytes = 0u64;
            let owner_uid = self.lock.owner_directory().metadata()?.uid();
            if let Err(error) = copy_directory_contents(
                &source,
                &File::from(destination),
                &mut entry_count,
                &mut total_bytes,
                max_entries,
                max_total_bytes,
                max_file_bytes,
                owner_uid,
                label,
            ) {
                let _ = self.remove_tree_if_exists(to);
                return Err(error);
            }
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
            if result.is_err() {
                let _ = std::fs::remove_dir_all(destination);
            }
            result
        }
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
            let child = openat(directory, name, directory_flags(), Mode::empty())
                .map_err(map_unsafe_relative_errno)?;
            rows.push(format!("dir:{relative_text}"));
            pending.push((child_relative, File::from(child)));
        }
        FileType::RegularFile => {
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
            if !file_metadata.is_file()
                || file_metadata.uid() != owner_uid
                || file_metadata.nlink() != 1
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
            if bytes.len() as u64 != file_metadata.len() || bytes.len() as u64 > max_file_bytes {
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
fn remove_directory_contents(directory: &File) -> Result<(), CommandError> {
    use rustix::fs::{openat, statat, unlinkat, AtFlags, FileType, Mode};

    for name in directory_entry_names(directory)? {
        let metadata =
            statat(directory, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
        match FileType::from_raw_mode(metadata.st_mode) {
            FileType::Directory => {
                let child = openat(directory, &name, directory_flags(), Mode::empty())
                    .map_err(map_unsafe_relative_errno)?;
                remove_directory_contents(&File::from(child))?;
                unlinkat(directory, &name, AtFlags::REMOVEDIR).map_err(io::Error::from)?;
            }
            FileType::RegularFile | FileType::Symlink => {
                unlinkat(directory, &name, AtFlags::empty()).map_err(io::Error::from)?;
            }
            _ => {
                return Err(CommandError::UnsafeConfigPath(
                    "app-data removal refuses a special file".to_string(),
                ))
            }
        }
    }
    rustix::fs::fsync(directory).map_err(io::Error::from)?;
    Ok(())
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
                mkdirat(destination, &name, directory_mode).map_err(io::Error::from)?;
                fsync(destination).map_err(io::Error::from)?;
                let source_child = openat(source, &name, directory_flags(), Mode::empty())
                    .map_err(map_unsafe_relative_errno)?;
                let destination_child =
                    openat(destination, &name, directory_flags(), Mode::empty())
                        .map_err(map_unsafe_relative_errno)?;
                fchmod(&destination_child, directory_mode).map_err(io::Error::from)?;
                copy_directory_contents(
                    &File::from(source_child),
                    &File::from(destination_child),
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
                if !source_metadata.is_file()
                    || source_metadata.uid() != owner_uid
                    || source_metadata.nlink() != 1
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
                fchmod(&destination_file, file_mode).map_err(io::Error::from)?;
                let copied = io::copy(
                    &mut Read::by_ref(&mut source_file).take(max_file_bytes.saturating_add(1)),
                    &mut destination_file,
                )?;
                if copied != source_metadata.len() || copied > max_file_bytes {
                    return Err(CommandError::StaleActionReference);
                }
                destination_file.flush()?;
                destination_file.sync_all()?;
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
                    sha2::Sha256::digest(std::fs::read(path)?)
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
