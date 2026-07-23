use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Read, Write},
    marker::PhantomData,
    path::{Component, Path, PathBuf},
};

use crate::{mutation_lock::AppMutationLock, CommandError};

mod tree;

#[cfg(unix)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct OwnerRegularFileStamp {
    device: u64,
    inode: u64,
    mode: u32,
    links: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
impl OwnerRegularFileStamp {
    fn from_metadata(metadata: &std::fs::Metadata) -> Self {
        use std::os::unix::fs::MetadataExt;

        Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            mode: metadata.mode(),
            links: metadata.nlink(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        }
    }

    fn revision(self) -> String {
        format!(
            "{}:{}:{:o}:{}:{}:{}:{}:{}:{}",
            self.device,
            self.inode,
            self.mode,
            self.links,
            self.length,
            self.modified_seconds,
            self.modified_nanoseconds,
            self.changed_seconds,
            self.changed_nanoseconds
        )
    }
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
            .read_bounded_regular_file_with_stamp(relative, max_bytes, label)?
            .map(|(bytes, _)| bytes))
    }

    pub(crate) fn read_bounded_regular_file_with_stamp(
        &self,
        relative: &Path,
        max_bytes: u64,
        label: &str,
    ) -> Result<Option<(Vec<u8>, String)>, CommandError> {
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
        #[cfg(unix)]
        let before = OwnerRegularFileStamp::from_metadata(&metadata);
        #[cfg(unix)]
        {
            use std::os::unix::fs::MetadataExt;

            let owner_metadata = self.lock.owner_directory().metadata()?;
            if metadata.uid() != owner_metadata.uid() || before.links != 1 {
                return Err(unsafe_relative_file(label));
            }
        }
        run_owner_read_test_hook(relative);
        let mut bytes = Vec::with_capacity(metadata.len() as usize);
        Read::by_ref(&mut file)
            .take(max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)?;
        if bytes.len() as u64 > max_bytes || bytes.len() as u64 != metadata.len() {
            return Err(unsafe_relative_file(label));
        }
        #[cfg(unix)]
        {
            let after = OwnerRegularFileStamp::from_metadata(&file.metadata()?);
            if after != before {
                return Err(CommandError::StaleActionReference);
            }
            let rebound = self
                .open_regular_file(relative, false)
                .map_err(|_| CommandError::StaleActionReference)?;
            let rebound_metadata = rebound
                .metadata()
                .map_err(|_| CommandError::StaleActionReference)?;
            if !rebound_metadata.is_file()
                || OwnerRegularFileStamp::from_metadata(&rebound_metadata) != before
            {
                return Err(CommandError::StaleActionReference);
            }
            Ok(Some((bytes, before.revision())))
        }
        #[cfg(not(unix))]
        {
            Ok(Some((bytes, format!("length:{}", metadata.len()))))
        }
    }

    pub fn atomic_replace_private_file(
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
            for _ in 0..32_u32 {
                let temp_name = secure_owner_temp_name(temp_stem)?;
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
                let mut installed = false;
                let result = (|| {
                    fchmod(&file, mode).map_err(io::Error::from)?;
                    file.write_all(bytes)?;
                    file.sync_all()?;
                    renameat(&parent, &temp_name, &parent, name).map_err(io::Error::from)?;
                    installed = true;
                    fsync(&parent).map_err(|error| {
                        owner_file_effect_unverified(format!(
                            "the app-owned file was replaced, but parent-directory durability could not be verified: {}",
                            io::Error::from(error)
                        ))
                    })?;
                    Ok::<(), CommandError>(())
                })();
                if let Err(error) = result {
                    if installed {
                        return Err(error);
                    }
                    match unlinkat(&parent, &temp_name, AtFlags::empty()) {
                        Ok(()) => {
                            fsync(&parent).map_err(|cleanup| {
                                owner_file_effect_unknown(format!(
                                    "app-owned file preparation failed ({error}); temporary cleanup durability could not be verified: {}",
                                    io::Error::from(cleanup)
                                ))
                            })?;
                        }
                        Err(rustix::io::Errno::NOENT) => {}
                        Err(cleanup) => {
                            return Err(owner_file_effect_unknown(format!(
                                "app-owned file preparation failed ({error}); private temporary cleanup failed: {}",
                                io::Error::from(cleanup)
                            )))
                        }
                    }
                    return Err(error);
                }
                return Ok(());
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
            let temp = parent.join(secure_owner_temp_name(temp_stem)?);
            let mut installed = false;
            let result = (|| {
                let mut file = std::fs::OpenOptions::new()
                    .create_new(true)
                    .write(true)
                    .open(&temp)?;
                file.write_all(bytes)?;
                file.sync_all()?;
                std::fs::rename(&temp, &target)?;
                installed = true;
                File::open(parent)
                    .and_then(|directory| directory.sync_all())
                    .map_err(|error| {
                        owner_file_effect_unverified(format!(
                            "the app-owned file was replaced, but parent-directory durability could not be verified: {error}"
                        ))
                    })?;
                Ok::<(), CommandError>(())
            })();
            if let Err(error) = result {
                if installed {
                    return Err(error);
                }
                match std::fs::remove_file(&temp) {
                    Ok(()) => {
                        File::open(parent)
                            .and_then(|directory| directory.sync_all())
                            .map_err(|cleanup| {
                                owner_file_effect_unknown(format!(
                                    "app-owned file preparation failed ({error}); temporary cleanup durability could not be verified: {cleanup}"
                                ))
                            })?;
                    }
                    Err(cleanup) if cleanup.kind() == io::ErrorKind::NotFound => {}
                    Err(cleanup) => {
                        return Err(owner_file_effect_unknown(format!(
                            "app-owned file preparation failed ({error}); private temporary cleanup failed: {cleanup}"
                        )))
                    }
                }
                return Err(error);
            }
            Ok(())
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

            use rustix::fs::{fchmod, fsync, openat, Mode, OFlags};

            let (parent, name) = self.open_parent(relative, false)?;
            let mode = Mode::from_bits_truncate(0o600);
            let (descriptor, created) = match openat(
                &parent,
                name,
                OFlags::WRONLY | OFlags::APPEND | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                Mode::empty(),
            ) {
                Ok(descriptor) => (descriptor, false),
                Err(rustix::io::Errno::NOENT) => (
                    openat(
                        &parent,
                        name,
                        OFlags::WRONLY
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
            let metadata = file.metadata()?;
            let owner_metadata = self.lock.owner_directory().metadata()?;
            if !metadata.is_file()
                || metadata.uid() != owner_metadata.uid()
                || metadata.nlink() != 1
                || metadata.mode() & 0o077 != 0
            {
                return Err(unsafe_relative_file(label));
            }
            if created {
                fchmod(&file, mode).map_err(io::Error::from)?;
            }
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
            fsync(&parent).map_err(io::Error::from)?;
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

    pub(crate) fn remove_root_regular_files_matching(
        &self,
        prefix: &str,
        suffix: &str,
        max_matches: usize,
    ) -> Result<(), CommandError> {
        self.remove_regular_files_matching_in_directory(None, prefix, suffix, max_matches)
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

            use rustix::fs::{fsync, statat, unlinkat, AtFlags, Dir, FileType};

            let directory = match relative_directory {
                Some(relative) => self.open_directory(relative)?,
                None => self.lock.open_owner_directory()?,
            };
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

    pub fn ensure_directory_all(&self, relative: &Path) -> Result<Vec<PathBuf>, CommandError> {
        validate_relative_path(relative)?;
        #[cfg(unix)]
        {
            use rustix::fs::{fchmod, fsync, mkdirat, openat, Mode};

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
                            Ok(()) => {
                                fsync(&current).map_err(io::Error::from)?;
                                created.push(accumulated.clone());
                            }
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
            self.lock.open_owner_directory()?
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
    match error {
        CommandError::Io(error) => error,
        other => io::Error::new(io::ErrorKind::PermissionDenied, other.to_string()),
    }
}

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

fn owner_file_effect_unknown(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "app-owned file replace".to_string(),
        state: "outcome_unknown",
        cleanup_required: true,
        detail,
    }
}

fn owner_file_effect_unverified(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "app-owned file replace".to_string(),
        state: "applied_unverified",
        cleanup_required: false,
        detail,
    }
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
    fn nested_cleanup_stays_on_the_locked_inode_after_path_replacement() {
        use std::os::unix::fs::symlink;

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
        use std::os::unix::fs::symlink;

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
        std::fs::write(target.join("SKILL.md"), b"accepted").expect("accepted file");
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
}
