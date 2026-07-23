use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Read, Write},
    marker::PhantomData,
    path::{Component, Path, PathBuf},
};

use crate::{mutation_lock::AppMutationLock, CommandError};

/// Filesystem capability rooted at the already-opened and locked app-data
/// owner directory.
///
/// The lifetime deliberately borrows the mutation lock so descriptor-relative
/// access cannot outlive the cross-process exclusion guard.
pub(crate) struct AppDataOwnerFs<'lock> {
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

    pub(crate) fn read_bounded_regular_file(
        &self,
        relative: &Path,
        max_bytes: u64,
        label: &str,
    ) -> Result<Option<Vec<u8>>, CommandError> {
        validate_relative_path(relative)?;
        let mut file = match self.open_regular_file(relative, false) {
            Ok(file) => file,
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(error) => return Err(map_relative_io(error, label)),
        };
        let metadata = file.metadata()?;
        if !metadata.is_file() || metadata.len() > max_bytes {
            return Err(unsafe_relative_file(label));
        }
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        Read::by_ref(&mut file)
            .take(max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > max_bytes {
            return Err(unsafe_relative_file(label));
        }
        Ok(Some(bytes))
    }

    pub(crate) fn atomic_replace_private_file(
        &self,
        relative: &Path,
        bytes: &[u8],
        temp_stem: &str,
    ) -> Result<(), CommandError> {
        validate_relative_path(relative)?;
        validate_single_component(OsStr::new(temp_stem))?;
        #[cfg(unix)]
        {
            use rustix::fs::{fchmod, fsync, openat, renameat, unlinkat, AtFlags, Mode, OFlags};

            let (parent, name) = self.open_parent(relative, false)?;
            let mode = Mode::from_bits_truncate(0o600);
            let mut last_collision = false;
            for attempt in 0..32_u32 {
                let temp_name = OsString::from(format!(
                    ".{temp_stem}.{}.{}.{attempt}.tmp",
                    std::process::id(),
                    owner_fs_timestamp_millis()
                ));
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
                    Err(rustix::io::Errno::EXIST) => {
                        last_collision = true;
                        continue;
                    }
                    Err(error) => return Err(io::Error::from(error).into()),
                };
                let mut file = File::from(descriptor);
                let result = (|| {
                    fchmod(&file, mode).map_err(io::Error::from)?;
                    file.write_all(bytes)?;
                    file.sync_all()?;
                    renameat(&parent, &temp_name, &parent, name).map_err(io::Error::from)?;
                    fsync(&parent).map_err(io::Error::from)?;
                    Ok::<(), CommandError>(())
                })();
                if result.is_err() {
                    let _ = unlinkat(&parent, &temp_name, AtFlags::empty());
                    let _ = fsync(&parent);
                }
                return result;
            }
            if last_collision {
                return Err(CommandError::UnsafeConfigPath(
                    "app-data private temporary file allocation was exhausted".to_string(),
                ));
            }
            unreachable!();
        }
        #[cfg(not(unix))]
        {
            let root = self.lock.owner_path();
            let target = guarded_fallback_path(root, relative)?;
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
                std::fs::rename(&temp, &target)?;
                File::open(parent)?.sync_all()?;
                Ok::<(), CommandError>(())
            })();
            if result.is_err() {
                let _ = std::fs::remove_file(temp);
            }
            result
        }
    }

    pub(crate) fn remove_root_regular_files_matching(
        &self,
        prefix: &str,
        suffix: &str,
        max_matches: usize,
    ) -> Result<(), CommandError> {
        validate_filename_fragment(prefix)?;
        validate_filename_fragment(suffix)?;
        #[cfg(unix)]
        {
            use std::os::unix::ffi::OsStrExt;

            use rustix::fs::{fsync, statat, unlinkat, AtFlags, Dir, FileType};

            let directory = self.lock.try_clone_owner_directory()?;
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
                if FileType::from_raw_mode(metadata.st_mode) != FileType::RegularFile {
                    return Err(CommandError::UnsafeConfigPath(
                        "app-data private residue is not a regular file".to_string(),
                    ));
                }
                matches.push(name.to_owned());
            }
            for name in matches {
                unlinkat(&directory, &name, AtFlags::empty()).map_err(io::Error::from)?;
            }
            fsync(&directory).map_err(io::Error::from)?;
            Ok(())
        }
        #[cfg(not(unix))]
        {
            let root = self.lock.owner_path();
            let metadata = std::fs::symlink_metadata(root)?;
            if metadata.file_type().is_symlink() || !metadata.is_dir() {
                return Err(unsafe_relative_path());
            }
            let mut matches = Vec::new();
            for entry in std::fs::read_dir(root)? {
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

    pub(crate) fn ensure_directory_all(
        &self,
        relative: &Path,
    ) -> Result<Vec<PathBuf>, CommandError> {
        validate_relative_path(relative)?;
        #[cfg(unix)]
        {
            use rustix::fs::{fchmod, mkdirat, openat, Mode};

            let flags = directory_flags();
            let mode = Mode::from_bits_truncate(0o700);
            let mut current = openat(self.lock.owner_directory(), ".", flags, Mode::empty())
                .map_err(io::Error::from)?;
            let mut created = Vec::new();
            let mut accumulated = PathBuf::new();
            for component in relative.components() {
                let Component::Normal(name) = component else {
                    return Err(unsafe_relative_path());
                };
                accumulated.push(name);
                match openat(&current, name, flags, Mode::empty()) {
                    Ok(next) => current = next,
                    Err(rustix::io::Errno::NOENT) => {
                        match mkdirat(&current, name, mode) {
                            Ok(()) => created.push(accumulated.clone()),
                            Err(rustix::io::Errno::EXIST) => {}
                            Err(error) => return Err(io::Error::from(error).into()),
                        }
                        let next = openat(&current, name, flags, Mode::empty())
                            .map_err(map_unsafe_relative_errno)?;
                        if created.last() == Some(&accumulated) {
                            fchmod(&next, mode).map_err(io::Error::from)?;
                        }
                        current = next;
                    }
                    Err(error) => return Err(map_unsafe_relative_errno(error)),
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
                        created.push(accumulated.clone());
                    }
                    Err(error) => return Err(error.into()),
                }
            }
            Ok(created)
        }
    }

    #[cfg(unix)]
    fn open_regular_file(&self, relative: &Path, write: bool) -> io::Result<File> {
        use rustix::fs::{openat, Mode, OFlags};

        let (parent, name) = self.open_parent_io(relative)?;
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
    fn open_regular_file(&self, relative: &Path, _write: bool) -> io::Result<File> {
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
    fn open_directory(&self, relative: &Path) -> io::Result<File> {
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
                return Err(io::Error::new(
                    io::ErrorKind::InvalidInput,
                    "app-data relative path is unsafe",
                ));
            };
            current = openat(&current, name, directory_flags(), Mode::empty())
                .map_err(io::Error::from)?;
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
            self.lock.try_clone_owner_directory()?
        } else {
            self.open_directory(parent)?
        };
        Ok((directory, name))
    }

    #[cfg(unix)]
    fn open_parent_io<'path>(&self, relative: &'path Path) -> io::Result<(File, &'path OsStr)> {
        self.open_parent(relative, false)
            .map_err(command_error_to_io)
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

fn map_relative_io(error: io::Error, label: &str) -> CommandError {
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
    io::Error::new(io::ErrorKind::PermissionDenied, error.to_string())
}

fn owner_fs_timestamp_millis() -> i64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

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
            .atomic_replace_private_file(Path::new("state.json"), br#"{"ok":true}"#, "state")
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
}
