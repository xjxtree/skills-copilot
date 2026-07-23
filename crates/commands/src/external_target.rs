use std::{
    collections::{BTreeMap, BTreeSet},
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Read, Write},
    marker::PhantomData,
    path::{Component, Path, PathBuf},
};

use sha2::{Digest, Sha256};

use crate::{mutation_lock::AppMutationLock, CommandError};

#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct ExternalFileState {
    pub(crate) exists: bool,
    pub(crate) content: String,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct EntryIdentity {
    device: u64,
    inode: u64,
    owner: u32,
}

#[cfg(unix)]
impl EntryIdentity {
    fn from_metadata(metadata: &std::fs::Metadata) -> Result<Self, CommandError> {
        use std::os::unix::fs::MetadataExt;

        ensure_current_user_owner(metadata.uid())?;
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            owner: metadata.uid(),
        })
    }

    fn from_stat(stat: &rustix::fs::Stat) -> Result<Self, CommandError> {
        ensure_current_user_owner(stat.st_uid)?;
        Ok(Self {
            device: stat.st_dev as u64,
            inode: stat.st_ino,
            owner: stat.st_uid,
        })
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct RegularFileStamp {
    device: u64,
    inode: u64,
    owner: u32,
    mode: u32,
    links: u64,
    length: u64,
    modified_seconds: i64,
    modified_nanoseconds: i64,
    changed_seconds: i64,
    changed_nanoseconds: i64,
}

#[cfg(unix)]
impl RegularFileStamp {
    fn from_metadata(metadata: &std::fs::Metadata) -> Result<Self, CommandError> {
        use std::os::unix::fs::MetadataExt;

        ensure_current_user_owner(metadata.uid())?;
        Ok(Self {
            device: metadata.dev(),
            inode: metadata.ino(),
            owner: metadata.uid(),
            mode: metadata.mode(),
            links: metadata.nlink(),
            length: metadata.len(),
            modified_seconds: metadata.mtime(),
            modified_nanoseconds: metadata.mtime_nsec(),
            changed_seconds: metadata.ctime(),
            changed_nanoseconds: metadata.ctime_nsec(),
        })
    }

    fn from_stat(stat: &rustix::fs::Stat) -> Result<Self, CommandError> {
        ensure_current_user_owner(stat.st_uid)?;
        Ok(Self {
            device: stat.st_dev as u64,
            inode: stat.st_ino,
            owner: stat.st_uid,
            mode: stat.st_mode as u32,
            links: stat.st_nlink as u64,
            length: u64::try_from(stat.st_size)
                .map_err(|_| unsafe_external_target("regular file has a negative size"))?,
            modified_seconds: stat.st_mtime,
            modified_nanoseconds: stat.st_mtime_nsec,
            changed_seconds: stat.st_ctime,
            changed_nanoseconds: stat.st_ctime_nsec,
        })
    }
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum TargetBinding {
    Missing,
    Present(EntryIdentity),
}

#[cfg(unix)]
struct DirectoryBinding {
    name: OsString,
    identity: EntryIdentity,
}

#[cfg(unix)]
struct CreatedDirectory {
    parent: File,
    name: OsString,
    identity: Option<EntryIdentity>,
}

#[cfg(unix)]
#[derive(Clone)]
struct DisplacedFile {
    name: OsString,
    identity: EntryIdentity,
}

#[cfg(unix)]
struct TemporaryFile {
    name: OsString,
    identity: Option<EntryIdentity>,
    file: File,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum TreeEntryKind {
    Directory,
    RegularFile,
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct TreeEntryBinding {
    kind: TreeEntryKind,
    identity: EntryIdentity,
    file_stamp: Option<RegularFileStamp>,
}

#[cfg(unix)]
type TreeManifest = BTreeMap<PathBuf, TreeEntryBinding>;

#[cfg(unix)]
struct TreeSnapshot {
    rows: Vec<String>,
    manifest: TreeManifest,
}

/// A descriptor-relative capability for one guarded agent config or skill
/// target. Its lifetime is tied to the shared app-data mutation lock so the
/// complete external write, semantic read-back, and exact compensation remain
/// inside one cross-sidecar lifecycle.
pub(crate) struct ExternalTargetCapability<'lock> {
    _lock: PhantomData<&'lock AppMutationLock>,
    display_path: PathBuf,
    anchored_path: PathBuf,
    #[cfg(unix)]
    root: File,
    #[cfg(unix)]
    root_display_path: PathBuf,
    #[cfg(unix)]
    directories: Vec<DirectoryBinding>,
    #[cfg(unix)]
    pending_directories: Vec<OsString>,
    #[cfg(unix)]
    parent: File,
    #[cfg(unix)]
    target_name: OsString,
    #[cfg(unix)]
    target_binding: TargetBinding,
    #[cfg(unix)]
    created_directories: Vec<CreatedDirectory>,
    #[cfg(unix)]
    displaced: Option<DisplacedFile>,
    #[cfg(unix)]
    temporary: Option<TemporaryFile>,
    #[cfg(not(unix))]
    root_path: PathBuf,
    #[cfg(not(unix))]
    created_directories: Vec<PathBuf>,
}

impl<'lock> ExternalTargetCapability<'lock> {
    pub(crate) fn prepare(
        lock: &'lock AppMutationLock,
        allowed_root: &Path,
        target: &Path,
    ) -> Result<Self, CommandError> {
        let relative = guarded_relative_target(allowed_root, target)?;
        #[cfg(unix)]
        {
            Self::prepare_unix(lock, allowed_root, target, &relative)
        }
        #[cfg(not(unix))]
        {
            prepare_fallback_root(allowed_root)?;
            let capability = Self {
                _lock: PhantomData,
                display_path: target.to_path_buf(),
                anchored_path: target.to_path_buf(),
                root_path: allowed_root.to_path_buf(),
                created_directories: Vec::new(),
            };
            capability.validate_fallback_path(false)?;
            Ok(capability)
        }
    }

    pub(crate) fn anchored_path(&self) -> &Path {
        &self.anchored_path
    }

    pub(crate) fn read_text_state(
        &self,
        max_bytes: u64,
    ) -> Result<ExternalFileState, CommandError> {
        #[cfg(unix)]
        {
            self.read_text_state_unix(true, max_bytes)
        }
        #[cfg(not(unix))]
        {
            self.validate_fallback_path(false)?;
            let metadata = match std::fs::symlink_metadata(&self.display_path) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
                    return Err(unsafe_external_target(
                        "guarded external target is not a regular file",
                    ))
                }
                Ok(metadata) if metadata.len() > max_bytes => {
                    return Err(unsafe_external_target(
                        "guarded external target exceeds its read bound",
                    ))
                }
                Ok(_) => Some(()),
                Err(error) if error.kind() == io::ErrorKind::NotFound => None,
                Err(error) => return Err(error.into()),
            };
            if metadata.is_none() {
                return Ok(ExternalFileState {
                    exists: false,
                    content: String::new(),
                });
            }
            match std::fs::read_to_string(&self.display_path) {
                Ok(content) => Ok(ExternalFileState {
                    exists: true,
                    content,
                }),
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(ExternalFileState {
                    exists: false,
                    content: String::new(),
                }),
                Err(error) => Err(error.into()),
            }
        }
    }

    pub(crate) fn read_text_state_anchored(
        &self,
        max_bytes: u64,
    ) -> Result<ExternalFileState, CommandError> {
        #[cfg(unix)]
        {
            self.read_text_state_unix(false, max_bytes)
        }
        #[cfg(not(unix))]
        {
            self.read_text_state(max_bytes)
        }
    }

    pub(crate) fn ensure_parent(&mut self) -> Result<(), CommandError> {
        #[cfg(unix)]
        {
            self.ensure_parent_unix()
        }
        #[cfg(not(unix))]
        {
            self.ensure_parent_fallback()
        }
    }

    /// Atomically replace the target only while it remains the exact entry
    /// accepted by this capability. Missing targets use NOREPLACE; present
    /// targets use an exchange-and-verify operation so a concurrent third
    /// state is restored rather than overwritten.
    pub(crate) fn atomic_replace(&mut self, bytes: &[u8]) -> Result<(), CommandError> {
        self.ensure_parent()?;
        run_test_hook(
            &self.display_path,
            ExternalTargetHookPoint::BeforeTempCreate,
        );
        #[cfg(unix)]
        {
            self.validate_display_binding()?;
            self.atomic_replace_unix(bytes, true)
        }
        #[cfg(not(unix))]
        {
            self.validate_fallback_path(true)?;
            self.atomic_replace_fallback(bytes)
        }
    }

    /// Restore bytes through the already-held parent descriptor. This is
    /// deliberately allowed after the display path itself became detached:
    /// exact compensation must repair the inode the action actually changed,
    /// while never following a replacement symlink into an attacker target.
    pub(crate) fn restore_present_anchored(&mut self, bytes: &[u8]) -> Result<(), CommandError> {
        #[cfg(unix)]
        {
            if self.displaced.is_some() {
                self.restore_displaced_original_unix()?;
            } else {
                self.atomic_replace_unix(bytes, false)?;
            }
            self.validate_display_binding()
                .map_err(|error| compensation_binding_unverified(error.to_string()))
        }
        #[cfg(not(unix))]
        {
            self.validate_fallback_path(true)?;
            self.atomic_replace_fallback(bytes)
        }
    }

    pub(crate) fn restore_missing_anchored(&mut self) -> Result<(), CommandError> {
        #[cfg(unix)]
        {
            self.remove_current_target_unix()?;
            self.remove_created_directories_unix()?;
            self.validate_display_binding()
                .map_err(|error| compensation_binding_unverified(error.to_string()))
        }
        #[cfg(not(unix))]
        {
            self.validate_fallback_path(true)?;
            match std::fs::remove_file(&self.display_path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
            if let Some(parent) = self.display_path.parent() {
                File::open(parent)
                    .and_then(|directory| directory.sync_all())
                    .map_err(|error| {
                        CommandError::PartialEffect {
                            operation: "external target compensation".to_string(),
                            state: "outcome_unknown",
                            cleanup_required: true,
                            detail: format!(
                                "the target was removed during compensation, but parent-directory durability could not be verified: {error}"
                            ),
                        }
                    })?;
            }
            for directory in self.created_directories.iter().rev() {
                match std::fs::remove_dir(directory) {
                    Ok(()) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                    Err(error) => return Err(error.into()),
                }
            }
            Ok(())
        }
    }

    pub(crate) fn run_before_compensation_hook(&self) {
        run_test_hook(
            &self.display_path,
            ExternalTargetHookPoint::BeforeCompensation,
        );
    }

    /// Remove a retained displaced original only after the catalog and
    /// semantic lifecycle committed. Keeping that inode until this point lets
    /// every post-rename failure restore the exact accepted entry.
    pub(crate) fn finish(&mut self) -> Result<(), CommandError> {
        #[cfg(unix)]
        {
            self.finish_displaced_unix().map_err(|error| match error {
                partial @ CommandError::PartialEffect { .. } => partial,
                other => CommandError::PartialEffect {
                    operation: "external target write".to_string(),
                    state: "applied_unverified",
                    cleanup_required: true,
                    detail: format!(
                        "the target was committed, but retained-original cleanup could not be verified: {other}"
                    ),
                },
            })
        }
        #[cfg(not(unix))]
        {
            Ok(())
        }
    }
}

#[cfg(unix)]
impl<'lock> ExternalTargetCapability<'lock> {
    fn prepare_unix(
        _lock: &'lock AppMutationLock,
        allowed_root: &Path,
        target: &Path,
        relative: &Path,
    ) -> Result<Self, CommandError> {
        use rustix::fs::{openat, statat, AtFlags, FileType, Mode};

        let root = crate::mutation_lock::open_existing_directory_nofollow(allowed_root)
            .map_err(|_| unsafe_external_target("guarded target root is not a safe directory"))?;
        EntryIdentity::from_metadata(&root.metadata()?)?;
        let mut current = open_directory_clone(&root)?;
        let mut directories = Vec::new();
        let mut pending_directories = Vec::new();
        let parent = relative.parent().unwrap_or_else(|| Path::new(""));
        let components = parent
            .components()
            .map(|component| match component {
                Component::Normal(name) => Ok(name.to_owned()),
                _ => Err(unsafe_external_target(
                    "guarded target contains a non-normal parent component",
                )),
            })
            .collect::<Result<Vec<_>, _>>()?;

        for (index, name) in components.iter().enumerate() {
            match openat(&current, name, directory_flags(), Mode::empty()) {
                Ok(next) => {
                    let next = File::from(next);
                    let identity = EntryIdentity::from_metadata(&next.metadata()?)?;
                    directories.push(DirectoryBinding {
                        name: name.clone(),
                        identity,
                    });
                    current = next;
                }
                Err(rustix::io::Errno::NOENT) => {
                    pending_directories.extend(components[index..].iter().cloned());
                    break;
                }
                Err(rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR) => {
                    return Err(unsafe_external_target(
                        "guarded target parent contains a symlink or non-directory",
                    ))
                }
                Err(error) => return Err(io::Error::from(error).into()),
            }
        }

        let target_name = relative
            .file_name()
            .ok_or_else(|| unsafe_external_target("guarded target has no file name"))?
            .to_owned();
        let target_binding = if pending_directories.is_empty() {
            match statat(&current, &target_name, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(stat) if FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile => {
                    TargetBinding::Present(EntryIdentity::from_stat(&stat)?)
                }
                Ok(_) => {
                    return Err(unsafe_external_target(
                        "guarded target is a symlink or non-regular file",
                    ))
                }
                Err(rustix::io::Errno::NOENT) => TargetBinding::Missing,
                Err(error) => return Err(io::Error::from(error).into()),
            }
        } else {
            TargetBinding::Missing
        };

        Ok(Self {
            _lock: PhantomData,
            display_path: target.to_path_buf(),
            anchored_path: descriptor_path(&root)
                .unwrap_or_else(|| allowed_root.to_path_buf())
                .join(relative),
            root,
            root_display_path: allowed_root.to_path_buf(),
            directories,
            pending_directories,
            parent: current,
            target_name,
            target_binding,
            created_directories: Vec::new(),
            displaced: None,
            temporary: None,
        })
    }

    fn validate_directory_chain(&self) -> Result<File, CommandError> {
        use rustix::fs::{openat, statat, AtFlags, Mode};

        let reopened_root =
            crate::mutation_lock::open_existing_directory_nofollow(&self.root_display_path)
                .map_err(|_| CommandError::StaleActionReference)?;
        if EntryIdentity::from_metadata(&reopened_root.metadata()?)?
            != EntryIdentity::from_metadata(&self.root.metadata()?)?
        {
            return Err(CommandError::StaleActionReference);
        }
        let mut current = reopened_root;
        for binding in &self.directories {
            let next = openat(&current, &binding.name, directory_flags(), Mode::empty())
                .map_err(|_| CommandError::StaleActionReference)?;
            let next = File::from(next);
            if EntryIdentity::from_metadata(&next.metadata()?)? != binding.identity {
                return Err(CommandError::StaleActionReference);
            }
            current = next;
        }
        if let Some(first_missing) = self.pending_directories.first() {
            match statat(&current, first_missing, AtFlags::SYMLINK_NOFOLLOW) {
                Err(rustix::io::Errno::NOENT) => {}
                Ok(_) => return Err(CommandError::StaleActionReference),
                Err(error) => return Err(io::Error::from(error).into()),
            }
        }
        Ok(current)
    }

    fn validate_target_binding_anchored(&self) -> Result<(), CommandError> {
        use rustix::fs::{statat, AtFlags, FileType};

        match (
            self.target_binding,
            statat(&self.parent, &self.target_name, AtFlags::SYMLINK_NOFOLLOW),
        ) {
            (TargetBinding::Missing, Err(rustix::io::Errno::NOENT)) => Ok(()),
            (TargetBinding::Missing, Ok(_))
            | (TargetBinding::Present(_), Err(rustix::io::Errno::NOENT)) => {
                Err(CommandError::StaleActionReference)
            }
            (TargetBinding::Present(expected), Ok(stat))
                if FileType::from_raw_mode(stat.st_mode) == FileType::RegularFile
                    && expected == EntryIdentity::from_stat(&stat)? =>
            {
                Ok(())
            }
            (TargetBinding::Present(_), Ok(_)) => Err(CommandError::StaleActionReference),
            (_, Err(error)) => Err(io::Error::from(error).into()),
        }
    }

    fn validate_display_binding(&self) -> Result<(), CommandError> {
        if !self.pending_directories.is_empty() {
            self.validate_directory_chain()?;
            return self.validate_target_binding_anchored();
        }
        let reopened_parent = self.validate_directory_chain()?;
        if EntryIdentity::from_metadata(&reopened_parent.metadata()?)?
            != EntryIdentity::from_metadata(&self.parent.metadata()?)?
        {
            return Err(CommandError::StaleActionReference);
        }
        self.validate_target_binding_anchored()
    }

    fn read_text_state_unix(
        &self,
        require_display_binding: bool,
        max_bytes: u64,
    ) -> Result<ExternalFileState, CommandError> {
        use rustix::fs::{openat, statat, AtFlags, FileType, Mode, OFlags};

        if require_display_binding {
            self.validate_display_binding()?;
        } else {
            self.validate_target_binding_anchored()?;
        }
        if self.target_binding == TargetBinding::Missing {
            return Ok(ExternalFileState {
                exists: false,
                content: String::new(),
            });
        }
        let before_path = statat(&self.parent, &self.target_name, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(map_unsafe_external_errno)?;
        if FileType::from_raw_mode(before_path.st_mode) != FileType::RegularFile {
            return Err(CommandError::StaleActionReference);
        }
        let before_path = RegularFileStamp::from_stat(&before_path)?;
        let descriptor = openat(
            &self.parent,
            &self.target_name,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
            Mode::empty(),
        )
        .map_err(map_unsafe_external_errno)?;
        let mut file = File::from(descriptor);
        let metadata = file.metadata()?;
        let before_read = RegularFileStamp::from_metadata(&metadata)?;
        let TargetBinding::Present(expected) = self.target_binding else {
            unreachable!();
        };
        if !metadata.is_file()
            || before_read.links != 1
            || before_read.length > max_bytes
            || EntryIdentity::from_metadata(&metadata)? != expected
            || before_path != before_read
        {
            return Err(CommandError::StaleActionReference);
        }
        run_test_hook(&self.display_path, ExternalTargetHookPoint::DuringRead);
        let mut bytes = Vec::new();
        Read::by_ref(&mut file)
            .take(max_bytes.saturating_add(1))
            .read_to_end(&mut bytes)?;
        let after_read = RegularFileStamp::from_metadata(&file.metadata()?)?;
        if bytes.len() as u64 > max_bytes
            || bytes.len() as u64 != before_read.length
            || after_read != before_read
        {
            return Err(CommandError::StaleActionReference);
        }
        let content = String::from_utf8(bytes).map_err(|_| {
            CommandError::UnsafeConfigPath(
                "guarded external target is not valid UTF-8 text".to_string(),
            )
        })?;
        Ok(ExternalFileState {
            exists: true,
            content,
        })
    }

    fn ensure_parent_unix(&mut self) -> Result<(), CommandError> {
        use rustix::fs::{fchmod, fsync, mkdirat, openat, statat, AtFlags, FileType, Mode};

        if self.pending_directories.is_empty() {
            self.validate_display_binding()?;
            return Ok(());
        }
        self.parent = self.validate_directory_chain()?;
        let mode = Mode::from_bits_truncate(0o700);
        while let Some(name) = self.pending_directories.first().cloned() {
            let created_parent = open_directory_clone(&self.parent)?;
            match statat(&self.parent, &name, AtFlags::SYMLINK_NOFOLLOW) {
                Err(rustix::io::Errno::NOENT) => {}
                Ok(_) => return Err(CommandError::StaleActionReference),
                Err(error) => return Err(io::Error::from(error).into()),
            }
            mkdirat(&self.parent, &name, mode).map_err(|error| match error {
                rustix::io::Errno::EXIST => CommandError::StaleActionReference,
                other => io::Error::from(other).into(),
            })?;
            self.created_directories.push(CreatedDirectory {
                parent: created_parent,
                name: name.clone(),
                identity: None,
            });
            run_external_target_fault(ExternalTargetFaultPoint::ParentMkdirStat).map_err(
                |error| {
                    directory_effect_unknown(format!(
                        "a target parent was created, but identity inspection was interrupted: {error}"
                    ))
                },
            )?;
            let created =
                statat(&self.parent, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                    directory_effect_unknown(format!(
                        "a target parent was created, but its identity could not be read: {}",
                        io::Error::from(error)
                    ))
                })?;
            if FileType::from_raw_mode(created.st_mode) != FileType::Directory {
                return Err(directory_effect_unknown(
                    "a target parent was created, but its entry is not a directory".to_string(),
                ));
            }
            let identity = EntryIdentity::from_stat(&created).map_err(|error| {
                directory_effect_unknown(format!(
                    "a target parent was created, but current-user ownership could not be verified: {error}"
                ))
            })?;
            self.created_directories
                .last_mut()
                .ok_or_else(|| {
                    directory_effect_unknown(
                        "created target parent recovery state was lost".to_string(),
                    )
                })?
                .identity = Some(identity);
            run_external_target_fault(ExternalTargetFaultPoint::ParentMkdirOpen).map_err(
                |error| {
                    directory_effect_unknown(format!(
                        "a target parent was created, but descriptor binding was interrupted: {error}"
                    ))
                },
            )?;
            let next =
                openat(&self.parent, &name, directory_flags(), Mode::empty()).map_err(|error| {
                    directory_effect_unknown(format!(
                        "a target parent was created, but its descriptor could not be opened: {}",
                        io::Error::from(error)
                    ))
                })?;
            let next = File::from(next);
            run_external_target_fault(ExternalTargetFaultPoint::ParentMkdirMetadata).map_err(
                |error| {
                    directory_effect_unknown(format!(
                        "a target parent was opened, but descriptor inspection was interrupted: {error}"
                    ))
                },
            )?;
            let next_metadata = next.metadata().map_err(|error| {
                directory_effect_unknown(format!(
                    "a target parent was opened, but descriptor identity could not be read: {error}"
                ))
            })?;
            if EntryIdentity::from_metadata(&next_metadata).map_err(|error| {
                directory_effect_unknown(format!(
                    "a target parent was opened, but current-user ownership could not be verified: {error}"
                ))
            })? != identity
            {
                return Err(directory_effect_unknown(
                    "a target parent changed before its descriptor was bound".to_string(),
                ));
            }
            self.directories.push(DirectoryBinding {
                name: name.clone(),
                identity,
            });
            self.pending_directories.remove(0);
            run_external_target_fault(ExternalTargetFaultPoint::ParentMkdirChmod).map_err(
                |error| {
                    directory_effect_unknown(format!(
                        "a target parent was created, but permission setup was interrupted: {error}"
                    ))
                },
            )?;
            fchmod(&next, mode).map_err(|error| {
                directory_effect_unknown(format!(
                    "a target parent was created, but private permissions could not be verified: {}",
                    io::Error::from(error)
                ))
            })?;
            run_external_target_fault(ExternalTargetFaultPoint::ParentMkdirSync)
                .map_err(directory_durability_unknown)?;
            fsync(&next)
                .and_then(|()| fsync(&self.parent))
                .map_err(|error| directory_durability_unknown(io::Error::from(error)))?;
            self.parent = next;
        }
        self.validate_target_binding_anchored()
    }

    fn atomic_replace_unix(
        &mut self,
        bytes: &[u8],
        require_display_binding: bool,
    ) -> Result<(), CommandError> {
        use rustix::fs::{fchmod, renameat_with, statat, AtFlags, Mode, RenameFlags};

        if require_display_binding {
            self.validate_display_binding()?;
        } else {
            self.validate_target_binding_anchored()?;
        }
        if self.displaced.is_some() {
            return Err(CommandError::StaleActionReference);
        }
        let mode = Mode::from_bits_truncate(0o600);
        if let Err(error) = self.create_temp_file_unix("agent-copilot-write", mode) {
            return Err(self.cleanup_temporary_after_error(error));
        }
        let (temp_name, temp_identity) = self
            .temporary
            .as_ref()
            .and_then(|temporary| {
                temporary
                    .identity
                    .map(|identity| (temporary.name.clone(), identity))
            })
            .ok_or_else(|| {
                file_effect_unknown(
                    "temporary-file recovery state was lost after creation".to_string(),
                )
            })?;
        #[derive(Clone, Copy, Eq, PartialEq)]
        enum TempRole {
            Candidate,
            Displaced,
            Gone,
        }
        let mut temp_role = TempRole::Candidate;
        let write_result = (|| {
            let temporary = self.temporary.as_mut().ok_or_else(|| {
                file_effect_unknown("temporary-file recovery state was lost".to_string())
            })?;
            run_external_target_fault(ExternalTargetFaultPoint::TempChmod)?;
            fchmod(&temporary.file, mode).map_err(io::Error::from)?;
            run_external_target_fault(ExternalTargetFaultPoint::TempWrite)?;
            temporary.file.write_all(bytes)?;
            run_external_target_fault(ExternalTargetFaultPoint::TempSync)?;
            temporary.file.sync_all()?;
            crate::run_atomic_pre_rename_failure_test_hook(&self.display_path)?;
            run_test_hook(&self.display_path, ExternalTargetHookPoint::BeforeRename);
            if require_display_binding {
                self.validate_display_binding()?;
            } else {
                self.validate_target_binding_anchored()?;
            }

            match self.target_binding {
                TargetBinding::Missing => {
                    renameat_with(
                        &self.parent,
                        &temp_name,
                        &self.parent,
                        &self.target_name,
                        RenameFlags::NOREPLACE,
                    )
                    .map_err(|error| match error {
                        rustix::io::Errno::EXIST => CommandError::StaleActionReference,
                        other => io::Error::from(other).into(),
                    })?;
                    self.target_binding = TargetBinding::Present(temp_identity);
                    temp_role = TempRole::Gone;
                    self.temporary = None;
                }
                TargetBinding::Present(expected) => {
                    renameat_with(
                        &self.parent,
                        &temp_name,
                        &self.parent,
                        &self.target_name,
                        RenameFlags::EXCHANGE,
                    )
                    .map_err(|error| match error {
                        rustix::io::Errno::NOENT => CommandError::StaleActionReference,
                        other => io::Error::from(other).into(),
                    })?;
                    temp_role = TempRole::Displaced;
                    self.target_binding = TargetBinding::Present(temp_identity);
                    self.temporary = None;
                    self.displaced = Some(DisplacedFile {
                        name: temp_name.clone(),
                        identity: expected,
                    });
                    run_external_target_fault(
                        ExternalTargetFaultPoint::ExchangeRetainedOriginalStat,
                    )
                    .map_err(|error| {
                        file_effect_unknown(format!(
                            "the target candidate was installed, but retained-original inspection was interrupted: {error}"
                        ))
                    })?;
                    let displaced = statat(&self.parent, &temp_name, AtFlags::SYMLINK_NOFOLLOW)
                        .map_err(|error| {
                            file_effect_unknown(format!(
                                "the target candidate was installed, but the retained original could not be inspected: {}",
                                io::Error::from(error)
                            ))
                        })?;
                    let displaced_identity =
                        EntryIdentity::from_stat(&displaced).map_err(|error| {
                            file_effect_unknown(format!(
                                "the retained original is not owned by the current user: {error}"
                            ))
                        })?;
                    if displaced_identity != expected {
                        return Err(file_effect_unknown(
                            "the retained external target original did not match the accepted inode"
                                .to_string(),
                        ));
                    }
                }
            }
            sync_external_parent_after_install(&self.parent, &self.display_path)?;
            run_external_target_fault(ExternalTargetFaultPoint::InstalledReadback).map_err(
                |error| {
                    file_effect_unknown(format!(
                        "the target candidate was installed, but read-back was interrupted: {error}"
                    ))
                },
            )?;
            crate::run_atomic_post_rename_test_hook(&self.display_path).map_err(|error| {
                file_effect_unknown(format!(
                    "the target candidate was installed, but post-rename verification failed: {error}"
                ))
            })?;
            run_test_hook(&self.display_path, ExternalTargetHookPoint::AfterRename);
            if require_display_binding {
                self.validate_display_binding()
                    .map_err(|error| binding_effect_unverified(error.to_string()))?;
            } else {
                self.validate_target_binding_anchored()
                    .map_err(|error| binding_effect_unverified(error.to_string()))?;
            }
            Ok(())
        })();
        if write_result.is_err() && temp_role == TempRole::Candidate {
            return Err(self
                .cleanup_temporary_after_error(write_result.expect_err("candidate write failed")));
        }
        write_result
    }

    fn create_temp_file_unix(
        &mut self,
        stem: &str,
        mode: rustix::fs::Mode,
    ) -> Result<(), CommandError> {
        use rustix::fs::{openat, OFlags};

        for _ in 0..32_u32 {
            let name = random_temp_name(stem)?;
            match openat(
                &self.parent,
                &name,
                OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                mode,
            ) {
                Ok(descriptor) => {
                    let file = File::from(descriptor);
                    self.temporary = Some(TemporaryFile {
                        name,
                        identity: None,
                        file,
                    });
                    run_external_target_fault(ExternalTargetFaultPoint::TempMetadata)?;
                    let identity = {
                        let temporary = self.temporary.as_ref().ok_or_else(|| {
                            file_effect_unknown(
                                "temporary-file recovery state was lost after creation".to_string(),
                            )
                        })?;
                        EntryIdentity::from_metadata(&temporary.file.metadata().map_err(
                            |error| {
                                file_effect_unknown(format!(
                                    "a temporary file was created, but its identity could not be read: {error}"
                                ))
                            },
                        )?)
                        .map_err(|error| {
                            file_effect_unknown(format!(
                                "a temporary file was created, but current-user ownership could not be verified: {error}"
                            ))
                        })?
                    };
                    self.temporary
                        .as_mut()
                        .ok_or_else(|| {
                            file_effect_unknown(
                                "temporary-file recovery state was lost after identity binding"
                                    .to_string(),
                            )
                        })?
                        .identity = Some(identity);
                    return Ok(());
                }
                Err(rustix::io::Errno::EXIST) => continue,
                Err(error) => return Err(io::Error::from(error).into()),
            }
        }
        Err(unsafe_external_target(
            "guarded target temporary-file allocation was exhausted",
        ))
    }

    fn cleanup_temporary_after_error(&mut self, original: CommandError) -> CommandError {
        match self.remove_temporary_unix() {
            Ok(()) => original,
            Err(cleanup) => match cleanup {
                partial @ CommandError::PartialEffect { .. } => partial,
                other => file_effect_unknown(format!(
                    "temporary-file cleanup failed after {original}: {other}"
                )),
            },
        }
    }

    fn remove_temporary_unix(&mut self) -> Result<(), CommandError> {
        use rustix::fs::{fsync, statat, unlinkat, AtFlags};

        let Some(temporary) = self.temporary.as_mut() else {
            return Ok(());
        };
        if temporary.identity.is_none() {
            temporary.identity = Some(
                EntryIdentity::from_metadata(&temporary.file.metadata().map_err(|error| {
                    file_effect_unknown(format!(
                        "a created temporary file could not be identity-bound for cleanup: {error}"
                    ))
                })?)
                .map_err(|error| {
                    file_effect_unknown(format!(
                        "a created temporary file is not owned by the current user: {error}"
                    ))
                })?,
            );
        }
        let expected = temporary.identity.ok_or_else(|| {
            file_effect_unknown("temporary-file identity is unavailable for cleanup".to_string())
        })?;
        let original_name = temporary.name.clone();
        let quarantine = allocate_file_quarantine(
            &self.parent,
            &original_name,
            "agent-copilot-temp-quarantine",
        )?;
        temporary.name = quarantine.clone();
        run_external_target_fault(ExternalTargetFaultPoint::TempCleanupStat).map_err(|error| {
            file_effect_unknown(format!(
                "a temporary file was quarantined, but cleanup inspection was interrupted: {error}"
            ))
        })?;
        let moved =
            statat(&self.parent, &quarantine, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                file_effect_unknown(format!(
                    "a temporary file was quarantined, but its identity could not be read: {}",
                    io::Error::from(error)
                ))
            })?;
        let moved_identity = EntryIdentity::from_stat(&moved).map_err(|error| {
            file_effect_unknown(format!(
                "a temporary-file quarantine is not owned by the current user: {error}"
            ))
        })?;
        if moved_identity != expected {
            return Err(file_effect_unknown(
                "a raced temporary-file replacement was retained in quarantine".to_string(),
            ));
        }
        unlinkat(&self.parent, &quarantine, AtFlags::empty()).map_err(|error| {
            file_effect_unknown(format!(
                "the verified temporary-file quarantine could not be removed: {}",
                io::Error::from(error)
            ))
        })?;
        self.temporary = None;
        run_external_target_fault(ExternalTargetFaultPoint::TempCleanupSync)
            .map_err(file_effect_unverified)?;
        fsync(&self.parent).map_err(|error| file_effect_unverified(io::Error::from(error)))
    }

    fn restore_displaced_original_unix(&mut self) -> Result<(), CommandError> {
        use rustix::fs::{renameat_with, statat, AtFlags, RenameFlags};

        let displaced = self
            .displaced
            .clone()
            .ok_or(CommandError::StaleActionReference)?;
        let TargetBinding::Present(candidate_identity) = self.target_binding else {
            return Err(CommandError::StaleActionReference);
        };
        self.validate_target_binding_anchored()?;
        let retained = statat(&self.parent, &displaced.name, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(io::Error::from)?;
        if EntryIdentity::from_stat(&retained)? != displaced.identity {
            return Err(CommandError::StaleActionReference);
        }
        renameat_with(
            &self.parent,
            &displaced.name,
            &self.parent,
            &self.target_name,
            RenameFlags::EXCHANGE,
        )
        .map_err(|error| {
            file_effect_unknown(format!(
                "the retained external target original could not be restored: {}",
                io::Error::from(error)
            ))
        })?;
        self.target_binding = TargetBinding::Present(displaced.identity);
        self.displaced = Some(DisplacedFile {
            name: displaced.name.clone(),
            identity: candidate_identity,
        });
        let restored = statat(&self.parent, &self.target_name, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|error| {
                file_effect_unknown(format!(
                    "the original target was exchanged back, but its identity could not be read: {}",
                    io::Error::from(error)
                ))
            })?;
        let quarantined = statat(&self.parent, &displaced.name, AtFlags::SYMLINK_NOFOLLOW)
            .map_err(|error| {
                file_effect_unknown(format!(
                    "the original target was exchanged back, but the candidate quarantine identity could not be read: {}",
                    io::Error::from(error)
                ))
            })?;
        if EntryIdentity::from_stat(&restored).map_err(|error| {
            file_effect_unknown(format!(
                "the restored original is not owned by the current user: {error}"
            ))
        })? != displaced.identity
            || EntryIdentity::from_stat(&quarantined).map_err(|error| {
                file_effect_unknown(format!(
                    "the candidate quarantine is not owned by the current user: {error}"
                ))
            })? != candidate_identity
        {
            return Err(file_effect_unknown(
                "external target restoration identities could not be verified".to_string(),
            ));
        }
        self.remove_displaced_unix(
            ExternalTargetFaultPoint::RestoreCandidateQuarantineStat,
            "the original target was restored, but candidate cleanup",
        )
    }

    fn finish_displaced_unix(&mut self) -> Result<(), CommandError> {
        let Some(displaced) = self.displaced.clone() else {
            return Ok(());
        };
        self.validate_target_binding_anchored()?;
        if displaced.identity
            != self
                .displaced
                .as_ref()
                .ok_or(CommandError::StaleActionReference)?
                .identity
        {
            return Err(CommandError::StaleActionReference);
        }
        self.remove_displaced_unix(
            ExternalTargetFaultPoint::FinishRetainedQuarantineStat,
            "the target was committed, but retained-original cleanup",
        )
    }

    fn remove_displaced_unix(
        &mut self,
        fault: ExternalTargetFaultPoint,
        context: &str,
    ) -> Result<(), CommandError> {
        use rustix::fs::{fsync, statat, unlinkat, AtFlags};

        let displaced = self
            .displaced
            .clone()
            .ok_or(CommandError::StaleActionReference)?;
        let quarantine = allocate_file_quarantine(
            &self.parent,
            &displaced.name,
            "agent-copilot-retained-quarantine",
        )
        .map_err(|error| file_effect_unknown(format!("{context} could not begin: {error}")))?;
        self.displaced = Some(DisplacedFile {
            name: quarantine.clone(),
            identity: displaced.identity,
        });
        run_external_target_fault(fault).map_err(|error| {
            file_effect_unknown(format!(
                "{context} was interrupted after atomic quarantine: {error}"
            ))
        })?;
        let retained =
            statat(&self.parent, &quarantine, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                file_effect_unknown(format!(
                    "{context} could not verify the private quarantine identity: {}",
                    io::Error::from(error)
                ))
            })?;
        if EntryIdentity::from_stat(&retained).map_err(|error| {
            file_effect_unknown(format!(
                "{context} found a quarantine not owned by the current user: {error}"
            ))
        })? != displaced.identity
        {
            return Err(file_effect_unknown(format!(
                "{context} retained a raced replacement without deleting it"
            )));
        }
        unlinkat(&self.parent, &quarantine, AtFlags::empty()).map_err(|error| {
            file_effect_unknown(format!(
                "{context} could not remove the verified private quarantine: {}",
                io::Error::from(error)
            ))
        })?;
        self.displaced = None;
        run_external_target_fault(ExternalTargetFaultPoint::DisplacedCleanupSync)
            .map_err(file_effect_unverified)?;
        fsync(&self.parent).map_err(|error| file_effect_unverified(io::Error::from(error)))
    }

    fn remove_current_target_unix(&mut self) -> Result<(), CommandError> {
        use rustix::fs::{fsync, renameat_with, statat, unlinkat, AtFlags, RenameFlags};

        let TargetBinding::Present(expected) = self.target_binding else {
            return Ok(());
        };
        self.validate_target_binding_anchored()?;
        let quarantine_name =
            allocate_file_quarantine(&self.parent, &self.target_name, "agent-copilot-remove")?;
        self.target_binding = TargetBinding::Missing;
        run_external_target_fault(ExternalTargetFaultPoint::RemoveTargetQuarantineStat).map_err(
            |error| {
                file_effect_unknown(format!(
                    "the candidate was quarantined, but inspection was interrupted: {error}"
                ))
            },
        )?;
        let moved =
            statat(&self.parent, &quarantine_name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                file_effect_unknown(format!(
                    "the candidate was quarantined, but its identity could not be read: {}",
                    io::Error::from(error)
                ))
            })?;
        let moved_identity = EntryIdentity::from_stat(&moved).map_err(|error| {
            file_effect_unknown(format!(
                "the candidate quarantine is not owned by the current user: {error}"
            ))
        })?;
        if moved_identity != expected {
            renameat_with(
                &self.parent,
                &quarantine_name,
                &self.parent,
                &self.target_name,
                RenameFlags::NOREPLACE,
            )
            .map_err(|error| {
                file_effect_unknown(format!(
                    "a raced target was quarantined and could not be restored: {}",
                    io::Error::from(error)
                ))
            })?;
            self.target_binding = TargetBinding::Present(moved_identity);
            fsync(&self.parent).map_err(|error| file_effect_unverified(io::Error::from(error)))?;
            return Err(file_effect_unknown(
                "a raced target entry was restored without deleting it".to_string(),
            ));
        }
        if let Err(unlink_error) = unlinkat(&self.parent, &quarantine_name, AtFlags::empty()) {
            renameat_with(
                &self.parent,
                &quarantine_name,
                &self.parent,
                &self.target_name,
                RenameFlags::NOREPLACE,
            )
            .map_err(|restore_error| CommandError::PartialEffect {
                operation: "external target compensation".to_string(),
                state: "outcome_unknown",
                cleanup_required: true,
                detail: format!(
                    "candidate quarantine removal failed ({}), and restoration also failed ({})",
                    io::Error::from(unlink_error),
                    io::Error::from(restore_error)
                ),
            })?;
            self.target_binding = TargetBinding::Present(expected);
            fsync(&self.parent).map_err(|error| file_effect_unverified(io::Error::from(error)))?;
            return Err(file_effect_unknown(format!(
                "candidate quarantine removal failed and the candidate was restored: {}",
                io::Error::from(unlink_error)
            )));
        }
        fsync(&self.parent).map_err(|error| file_effect_unverified(io::Error::from(error)))?;
        Ok(())
    }

    fn remove_created_directories_unix(&mut self) -> Result<(), CommandError> {
        use rustix::fs::{fsync, renameat_with, statat, unlinkat, AtFlags, RenameFlags};

        while let Some(mut created) = self.created_directories.pop() {
            let original_name = created.name.clone();
            let mut quarantined = None;
            let mut missing = false;
            for _ in 0..32_u32 {
                let quarantine = random_temp_name("agent-copilot-created-dir")?;
                match renameat_with(
                    &created.parent,
                    &original_name,
                    &created.parent,
                    &quarantine,
                    RenameFlags::NOREPLACE,
                ) {
                    Ok(()) => {
                        quarantined = Some(quarantine);
                        break;
                    }
                    Err(rustix::io::Errno::NOENT) => {
                        missing = true;
                        break;
                    }
                    Err(rustix::io::Errno::EXIST) => continue,
                    Err(error) => {
                        self.created_directories.push(created);
                        return Err(io::Error::from(error).into());
                    }
                }
            }
            let Some(quarantine) = quarantined else {
                if missing {
                    if let Some(identity) = created.identity {
                        self.mark_created_directory_removed(&original_name, identity)?;
                    }
                    continue;
                }
                self.created_directories.push(created);
                return Err(file_effect_unknown(
                    "created-directory quarantine allocation was exhausted".to_string(),
                ));
            };
            created.name = quarantine.clone();
            if let Err(error) =
                run_external_target_fault(ExternalTargetFaultPoint::CreatedDirectoryQuarantineStat)
            {
                self.created_directories.push(created);
                return Err(directory_effect_unknown(format!(
                    "a created target directory was quarantined, but inspection was interrupted: {error}"
                )));
            }
            let stat = match statat(&created.parent, &quarantine, AtFlags::SYMLINK_NOFOLLOW) {
                Ok(stat) => stat,
                Err(error) => {
                    self.created_directories.push(created);
                    return Err(file_effect_unknown(format!(
                        "a created target directory was quarantined, but its identity could not be read: {}",
                        io::Error::from(error)
                    )));
                }
            };
            let Some(expected_identity) = created.identity else {
                self.created_directories.push(created);
                return Err(directory_effect_unknown(
                    "a directory created before identity binding was retained in a private quarantine"
                        .to_string(),
                ));
            };
            if EntryIdentity::from_stat(&stat).map_err(|error| {
                directory_effect_unknown(format!(
                    "a created target directory quarantine is not owned by the current user: {error}"
                ))
            })? != expected_identity
            {
                match renameat_with(
                    &created.parent,
                    &quarantine,
                    &created.parent,
                    &original_name,
                    RenameFlags::NOREPLACE,
                ) {
                    Ok(()) => {
                        fsync(&created.parent)
                            .map_err(|error| file_effect_unverified(io::Error::from(error)))?;
                        return Err(file_effect_unknown(
                            "a raced replacement directory was restored without deleting it"
                                .to_string(),
                        ));
                    }
                    Err(error) => {
                        return Err(file_effect_unknown(format!(
                            "a raced replacement directory was quarantined but could not be restored to its display name: {}",
                            io::Error::from(error)
                        )));
                    }
                }
            }
            if let Err(error) = unlinkat(&created.parent, &quarantine, AtFlags::REMOVEDIR) {
                self.created_directories.push(created);
                return Err(file_effect_unknown(format!(
                    "an app-created target directory was quarantined but is no longer empty: {}",
                    io::Error::from(error)
                )));
            }
            self.mark_created_directory_removed(&original_name, expected_identity)
                .map_err(|error| {
                    file_effect_unknown(format!(
                        "a created target directory was removed, but capability state could not be updated: {error}"
                    ))
                })?;
            fsync(&created.parent)
                .map_err(|error| file_effect_unverified(io::Error::from(error)))?;
        }
        Ok(())
    }

    fn mark_created_directory_removed(
        &mut self,
        original_name: &OsStr,
        identity: EntryIdentity,
    ) -> Result<(), CommandError> {
        let removed = self
            .directories
            .pop()
            .ok_or(CommandError::StaleActionReference)?;
        if removed.name != original_name || removed.identity != identity {
            return Err(CommandError::StaleActionReference);
        }
        self.pending_directories.insert(0, original_name.to_owned());
        Ok(())
    }
}

#[cfg(not(unix))]
impl<'lock> ExternalTargetCapability<'lock> {
    fn validate_fallback_path(&self, require_parent: bool) -> Result<(), CommandError> {
        prepare_fallback_root(&self.root_path)?;
        let relative = guarded_relative_target(&self.root_path, &self.display_path)?;
        let mut current = self.root_path.clone();
        let components = relative.components().collect::<Vec<_>>();
        for (index, component) in components.iter().enumerate() {
            let Component::Normal(name) = component else {
                return Err(unsafe_external_target("guarded target path is unsafe"));
            };
            current.push(name);
            let is_file = index + 1 == components.len();
            match std::fs::symlink_metadata(&current) {
                Ok(metadata)
                    if metadata.file_type().is_symlink()
                        || (is_file && !metadata.is_file())
                        || (!is_file && !metadata.is_dir()) =>
                {
                    return Err(unsafe_external_target(
                        "guarded target path contains a symlink or special entry",
                    ))
                }
                Ok(_) => {}
                Err(error)
                    if error.kind() == io::ErrorKind::NotFound && (!require_parent || is_file) =>
                {
                    break
                }
                Err(error) => return Err(error.into()),
            }
        }
        Ok(())
    }

    fn ensure_parent_fallback(&mut self) -> Result<(), CommandError> {
        self.validate_fallback_path(false)?;
        let relative = guarded_relative_target(&self.root_path, &self.display_path)?;
        let mut current = self.root_path.clone();
        let parent = relative.parent().unwrap_or_else(|| Path::new(""));
        for component in parent.components() {
            let Component::Normal(name) = component else {
                return Err(unsafe_external_target("guarded target path is unsafe"));
            };
            current.push(name);
            match std::fs::symlink_metadata(&current) {
                Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                    return Err(unsafe_external_target(
                        "guarded target parent is not a safe directory",
                    ))
                }
                Ok(_) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {
                    std::fs::create_dir(&current)?;
                    File::open(&current)?.sync_all()?;
                    File::open(
                        current
                            .parent()
                            .ok_or_else(|| unsafe_external_target("target parent is missing"))?,
                    )?
                    .sync_all()?;
                    self.created_directories.push(current.clone());
                }
                Err(error) => return Err(error.into()),
            }
        }
        self.validate_fallback_path(true)
    }

    fn atomic_replace_fallback(&mut self, bytes: &[u8]) -> Result<(), CommandError> {
        let parent = self
            .display_path
            .parent()
            .ok_or_else(|| unsafe_external_target("guarded target has no parent"))?;
        let temp = parent.join(random_temp_name("agent-copilot-write")?);
        let result = (|| {
            let mut file = std::fs::OpenOptions::new()
                .create_new(true)
                .write(true)
                .open(&temp)?;
            file.write_all(bytes)?;
            file.sync_all()?;
            crate::run_atomic_pre_rename_failure_test_hook(&self.display_path)?;
            run_test_hook(&self.display_path, ExternalTargetHookPoint::BeforeRename);
            self.validate_fallback_path(true)?;
            std::fs::rename(&temp, &self.display_path)?;
            File::open(parent)?.sync_all()?;
            crate::run_atomic_post_rename_test_hook(&self.display_path)?;
            run_test_hook(&self.display_path, ExternalTargetHookPoint::AfterRename);
            self.validate_fallback_path(true)
        })();
        match result {
            Ok(()) => Ok(()),
            Err(original) => match std::fs::remove_file(&temp) {
                Ok(()) => {
                    File::open(parent)?.sync_all()?;
                    Err(original)
                }
                Err(error) if error.kind() == io::ErrorKind::NotFound => Err(original),
                Err(cleanup) => Err(file_effect_unknown(format!(
                    "non-Unix guarded target write failed ({original}); temporary-file cleanup failed ({cleanup})"
                ))),
            },
        }
    }
}

fn guarded_relative_target(allowed_root: &Path, target: &Path) -> Result<PathBuf, CommandError> {
    if !allowed_root.is_absolute() || !target.is_absolute() {
        return Err(unsafe_external_target(
            "guarded external roots and targets must be absolute",
        ));
    }
    let normalized_root = crate::normalize_path_lexically(allowed_root);
    let normalized_target = crate::normalize_path_lexically(target);
    let relative = normalized_target
        .strip_prefix(&normalized_root)
        .map_err(|_| unsafe_external_target("guarded target is outside its allowed root"))?;
    if relative.as_os_str().is_empty()
        || relative
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(unsafe_external_target(
            "guarded target is not a normalized descendant",
        ));
    }
    Ok(relative.to_path_buf())
}

#[cfg(not(unix))]
fn prepare_fallback_root(root: &Path) -> Result<(), CommandError> {
    let metadata = std::fs::symlink_metadata(root)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(unsafe_external_target(
            "guarded target root is not a safe directory",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn open_directory_clone(directory: &File) -> Result<File, CommandError> {
    use rustix::fs::{openat, Mode};

    openat(directory, ".", directory_flags(), Mode::empty())
        .map(File::from)
        .map_err(|error| io::Error::from(error).into())
}

#[cfg(all(unix, target_vendor = "apple"))]
fn descriptor_path(directory: &File) -> Option<PathBuf> {
    rustix::fs::getpath(directory)
        .ok()
        .map(|path| PathBuf::from(path.to_string_lossy().into_owned()))
}

#[cfg(all(unix, not(target_vendor = "apple")))]
fn descriptor_path(directory: &File) -> Option<PathBuf> {
    use std::os::fd::AsRawFd;

    std::fs::read_link(format!("/proc/self/fd/{}", directory.as_raw_fd())).ok()
}

#[cfg(unix)]
fn directory_flags() -> rustix::fs::OFlags {
    rustix::fs::OFlags::RDONLY
        | rustix::fs::OFlags::DIRECTORY
        | rustix::fs::OFlags::NOFOLLOW
        | rustix::fs::OFlags::CLOEXEC
}

#[cfg(unix)]
fn map_unsafe_external_errno(error: rustix::io::Errno) -> CommandError {
    if matches!(error, rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR) {
        unsafe_external_target("guarded target contains a symlink or non-directory")
    } else {
        io::Error::from(error).into()
    }
}

fn unsafe_external_target(detail: &str) -> CommandError {
    CommandError::UnsafeConfigPath(detail.to_string())
}

fn directory_durability_unknown(error: io::Error) -> CommandError {
    CommandError::PartialEffect {
        operation: "external target parent initialization".to_string(),
        state: "outcome_unknown",
        cleanup_required: true,
        detail: format!(
            "a private external target directory was created, but its durability could not be verified: {error}"
        ),
    }
}

fn directory_effect_unknown(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "external target parent initialization".to_string(),
        state: "outcome_unknown",
        cleanup_required: true,
        detail,
    }
}

fn file_effect_unverified(error: io::Error) -> CommandError {
    CommandError::PartialEffect {
        operation: "external target write".to_string(),
        state: "applied_unverified",
        cleanup_required: false,
        detail: format!(
            "the target candidate was installed, but parent-directory durability could not be verified: {error}"
        ),
    }
}

fn file_effect_unknown(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "external target write".to_string(),
        state: "outcome_unknown",
        cleanup_required: true,
        detail,
    }
}

fn binding_effect_unverified(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "external target write".to_string(),
        state: "applied_unverified",
        cleanup_required: true,
        detail: format!(
            "the target candidate was installed, but its guarded display binding changed before read-back: {detail}"
        ),
    }
}

fn compensation_binding_unverified(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "external target compensation".to_string(),
        state: "outcome_unknown",
        cleanup_required: true,
        detail: format!(
            "the accepted target inode was restored, but its display binding changed during compensation: {detail}"
        ),
    }
}

fn random_temp_name(stem: &str) -> Result<OsString, CommandError> {
    let mut nonce = [0_u8; 16];
    getrandom::getrandom(&mut nonce).map_err(|error| {
        io::Error::other(format!("secure random name generation failed: {error}"))
    })?;
    let mut encoded = String::with_capacity(nonce.len() * 2);
    for byte in nonce {
        use std::fmt::Write as _;

        write!(&mut encoded, "{byte:02x}")
            .map_err(|_| io::Error::other("random name encoding failed"))?;
    }
    Ok(OsString::from(format!(".{stem}.{encoded}.tmp")))
}

#[cfg(unix)]
pub(crate) fn random_external_entry_name(stem: &str) -> Result<OsString, CommandError> {
    random_temp_name(stem)
}

#[cfg(unix)]
fn allocate_file_quarantine(
    parent: &File,
    source: &OsStr,
    stem: &str,
) -> Result<OsString, CommandError> {
    use rustix::fs::{renameat_with, RenameFlags};

    for _ in 0..32_u32 {
        let name = random_temp_name(stem)?;
        match renameat_with(parent, source, parent, &name, RenameFlags::NOREPLACE) {
            Ok(()) => return Ok(name),
            Err(rustix::io::Errno::EXIST) => continue,
            Err(rustix::io::Errno::NOENT) => return Err(CommandError::StaleActionReference),
            Err(error) => return Err(io::Error::from(error).into()),
        }
    }
    Err(file_effect_unknown(
        "private file quarantine allocation was exhausted".to_string(),
    ))
}

#[cfg(unix)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ExternalTargetFaultPoint {
    ParentMkdirStat,
    ParentMkdirOpen,
    ParentMkdirMetadata,
    ParentMkdirChmod,
    ParentMkdirSync,
    TempMetadata,
    TempChmod,
    TempWrite,
    TempSync,
    TempCleanupStat,
    TempCleanupSync,
    ExchangeRetainedOriginalStat,
    InstalledReadback,
    RestoreCandidateQuarantineStat,
    FinishRetainedQuarantineStat,
    DisplacedCleanupSync,
    RemoveTargetQuarantineStat,
    CreatedDirectoryQuarantineStat,
    TreeStagingMkdirStat,
    TreeStagingMkdirOpen,
    TreeStagingMkdirMetadata,
    TreeStagingMkdirChmod,
    TreeStagingMkdirSync,
    TreeNestedMkdirStat,
    TreeNestedMkdirOpen,
    TreeNestedMkdirMetadata,
    TreeNestedMkdirChmod,
    TreeNestedMkdirSync,
    TreeActivationBackupStat,
    TreeActivationCandidateStat,
    TreeRestoreCandidateStat,
    TreeRestoreBackupStat,
    TreeQuarantineStat,
    TreeQuarantineSync,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum ExternalTargetHookPoint {
    BeforeTempCreate,
    BeforeRename,
    AfterRename,
    BeforeCompensation,
    DuringRead,
    BeforeTreeStaging,
    BeforeTreeActivation,
    BeforeTreeRestore,
    DuringTreeRead,
}

#[cfg(unix)]
pub(crate) struct ExternalTreeCapability<'lock> {
    _lock: PhantomData<&'lock AppMutationLock>,
    root: File,
    root_display_path: PathBuf,
    target_parent: File,
    target_name: OsString,
    target_identity: Option<EntryIdentity>,
    directory_chain: Vec<DirectoryBinding>,
    display_skill_path: PathBuf,
    anchored_skill_path: PathBuf,
    staging: Option<(OsString, EntryIdentity)>,
    unbound_staging_name: Option<OsString>,
    backup: Option<(OsString, EntryIdentity)>,
    staging_manifest: TreeManifest,
    unbound_staging_entries: BTreeSet<PathBuf>,
    original_manifest: Option<TreeManifest>,
    candidate_manifest: Option<TreeManifest>,
}

#[cfg(unix)]
impl<'lock> ExternalTreeCapability<'lock> {
    pub(crate) fn prepare(
        _lock: &'lock AppMutationLock,
        allowed_root: &Path,
        target_skill_path: &Path,
    ) -> Result<Self, CommandError> {
        use rustix::fs::{openat, Mode};

        let target_directory = target_skill_path
            .parent()
            .ok_or_else(|| unsafe_external_target("external tree target has no skill directory"))?;
        let target_relative = guarded_relative_target(allowed_root, target_directory)?;
        let target_name = target_relative
            .file_name()
            .ok_or_else(|| unsafe_external_target("external tree target has no directory name"))?
            .to_owned();
        let parent_relative = target_relative.parent().unwrap_or_else(|| Path::new(""));
        let root = crate::mutation_lock::open_existing_directory_nofollow(allowed_root)
            .map_err(|_| unsafe_external_target("external tree root is not a safe directory"))?;
        EntryIdentity::from_metadata(&root.metadata()?)?;
        let mut current = open_directory_clone(&root)?;
        let mut directory_chain = Vec::new();
        for component in parent_relative.components() {
            let Component::Normal(name) = component else {
                return Err(unsafe_external_target(
                    "external tree parent contains a non-normal component",
                ));
            };
            let next = openat(&current, name, directory_flags(), Mode::empty())
                .map_err(map_unsafe_external_errno)?;
            let next = File::from(next);
            directory_chain.push(DirectoryBinding {
                name: name.to_owned(),
                identity: EntryIdentity::from_metadata(&next.metadata()?)?,
            });
            current = next;
        }
        let target = openat(&current, &target_name, directory_flags(), Mode::empty())
            .map_err(map_unsafe_external_errno)?;
        let target = File::from(target);
        let target_identity = EntryIdentity::from_metadata(&target.metadata()?)?;
        let anchored_skill_path = descriptor_path(&root)
            .unwrap_or_else(|| allowed_root.to_path_buf())
            .join(&target_relative)
            .join("SKILL.md");
        let capability = Self {
            _lock: PhantomData,
            root,
            root_display_path: allowed_root.to_path_buf(),
            target_parent: current,
            target_name,
            target_identity: Some(target_identity),
            directory_chain,
            display_skill_path: target_skill_path.to_path_buf(),
            anchored_skill_path,
            staging: None,
            unbound_staging_name: None,
            backup: None,
            staging_manifest: TreeManifest::new(),
            unbound_staging_entries: BTreeSet::new(),
            original_manifest: None,
            candidate_manifest: None,
        };
        capability.validate_display_binding()?;
        Ok(capability)
    }

    pub(crate) fn anchored_skill_path(&self) -> &Path {
        &self.anchored_skill_path
    }

    pub(crate) fn validate_binding(&self) -> Result<(), CommandError> {
        self.validate_display_binding()
    }

    pub(crate) fn create_staging(&mut self, name: &OsStr) -> Result<(), CommandError> {
        use rustix::fs::{fchmod, fsync, mkdirat, openat, statat, AtFlags, FileType, Mode};

        validate_single_component(name)?;
        if self.staging.is_some() {
            return Err(CommandError::StaleActionReference);
        }
        run_test_hook(
            &self.display_skill_path,
            ExternalTargetHookPoint::BeforeTreeStaging,
        );
        self.validate_display_binding()?;
        let mode = Mode::from_bits_truncate(0o700);
        mkdirat(&self.root, name, mode).map_err(|error| match error {
            rustix::io::Errno::EXIST => CommandError::StaleActionReference,
            other => io::Error::from(other).into(),
        })?;
        self.unbound_staging_name = Some(name.to_owned());
        run_external_target_fault(ExternalTargetFaultPoint::TreeStagingMkdirStat).map_err(
            |error| {
                tree_effect_unknown(format!(
                    "archive staging was created, but identity inspection was interrupted: {error}"
                ))
            },
        )?;
        let created = statat(&self.root, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
            tree_effect_unknown(format!(
                "archive staging was created, but its identity could not be read: {}",
                io::Error::from(error)
            ))
        })?;
        if FileType::from_raw_mode(created.st_mode) != FileType::Directory {
            return Err(tree_effect_unknown(
                "archive staging was created, but its entry is not a directory".to_string(),
            ));
        }
        let identity = EntryIdentity::from_stat(&created).map_err(|error| {
            tree_effect_unknown(format!(
                "archive staging was created, but current-user ownership could not be verified: {error}"
            ))
        })?;
        self.staging = Some((name.to_owned(), identity));
        self.unbound_staging_name = None;
        self.staging_manifest.clear();
        self.unbound_staging_entries.clear();
        run_external_target_fault(ExternalTargetFaultPoint::TreeStagingMkdirOpen).map_err(
            |error| {
                tree_effect_unknown(format!(
                    "archive staging was created, but descriptor binding was interrupted: {error}"
                ))
            },
        )?;
        let directory =
            openat(&self.root, name, directory_flags(), Mode::empty()).map_err(|error| {
                tree_effect_unknown(format!(
                    "archive staging was created, but its descriptor could not be opened: {}",
                    io::Error::from(error)
                ))
            })?;
        let directory = File::from(directory);
        run_external_target_fault(ExternalTargetFaultPoint::TreeStagingMkdirMetadata).map_err(
            |error| {
                tree_effect_unknown(format!(
                    "archive staging was opened, but descriptor inspection was interrupted: {error}"
                ))
            },
        )?;
        let directory_metadata = directory.metadata().map_err(|error| {
            tree_effect_unknown(format!(
                "archive staging was opened, but descriptor identity could not be read: {error}"
            ))
        })?;
        if EntryIdentity::from_metadata(&directory_metadata).map_err(|error| {
            tree_effect_unknown(format!(
                "archive staging was opened, but current-user ownership could not be verified: {error}"
            ))
        })? != identity
        {
            return Err(tree_effect_unknown(
                "archive staging changed before its descriptor was bound".to_string(),
            ));
        }
        run_external_target_fault(ExternalTargetFaultPoint::TreeStagingMkdirChmod).map_err(
            |error| {
                tree_effect_unknown(format!(
                    "archive staging was created, but permission setup was interrupted: {error}"
                ))
            },
        )?;
        fchmod(&directory, mode).map_err(|error| {
            tree_effect_unknown(format!(
                "archive staging was created, but private permissions could not be verified: {}",
                io::Error::from(error)
            ))
        })?;
        run_external_target_fault(ExternalTargetFaultPoint::TreeStagingMkdirSync)
            .map_err(directory_durability_unknown)?;
        fsync(&directory)
            .and_then(|()| fsync(&self.root))
            .map_err(|error| directory_durability_unknown(io::Error::from(error)))?;
        Ok(())
    }

    pub(crate) fn ensure_staging_directory(&mut self, relative: &Path) -> Result<(), CommandError> {
        let staging = self.open_staging()?;
        ensure_directory_relative(
            &staging,
            relative,
            &mut self.staging_manifest,
            &mut self.unbound_staging_entries,
        )
    }

    pub(crate) fn create_staging_file(&mut self, relative: &Path) -> Result<File, CommandError> {
        use rustix::fs::{fchmod, openat, Mode, OFlags};

        validate_relative_components(relative)?;
        let name = relative
            .file_name()
            .ok_or_else(|| unsafe_external_target("staging file has no name"))?;
        let parent_relative = relative.parent().unwrap_or_else(|| Path::new(""));
        if !parent_relative.as_os_str().is_empty() {
            self.ensure_staging_directory(parent_relative)?;
        }
        let staging = self.open_staging()?;
        let parent = open_relative_directory(&staging, parent_relative)?;
        let mode = Mode::from_bits_truncate(0o600);
        let descriptor = openat(
            &parent,
            name,
            OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
            mode,
        )
        .map_err(|error| match error {
            rustix::io::Errno::EXIST => CommandError::StaleActionReference,
            other => io::Error::from(other).into(),
        })?;
        let file = File::from(descriptor);
        self.unbound_staging_entries.insert(relative.to_path_buf());
        let metadata = file.metadata().map_err(|error| {
            tree_effect_unknown(format!(
                "a staging file was created, but its identity could not be read: {error}"
            ))
        })?;
        let stamp = RegularFileStamp::from_metadata(&metadata).map_err(|error| {
            tree_effect_unknown(format!(
                "a staging file was created, but current-user ownership could not be verified: {error}"
            ))
        })?;
        if !metadata.is_file() || stamp.links != 1 {
            return Err(tree_effect_unknown(
                "a staging file was created, but it is not a private regular file".to_string(),
            ));
        }
        if self
            .staging_manifest
            .insert(
                relative.to_path_buf(),
                TreeEntryBinding {
                    kind: TreeEntryKind::RegularFile,
                    identity: EntryIdentity {
                        device: stamp.device,
                        inode: stamp.inode,
                        owner: stamp.owner,
                    },
                    file_stamp: Some(stamp),
                },
            )
            .is_some()
        {
            return Err(tree_effect_unknown(
                "staging file recovery state collided".to_string(),
            ));
        }
        self.unbound_staging_entries.remove(relative);
        fchmod(&file, mode).map_err(|error| {
            tree_effect_unknown(format!(
                "a staging file was created, but private permissions could not be verified: {}",
                io::Error::from(error)
            ))
        })?;
        Ok(file)
    }

    pub(crate) fn sync_staging(&mut self) -> Result<(), CommandError> {
        use rustix::fs::fsync;

        let staging = self.open_staging()?;
        let observed = capture_tree_manifest(&staging, 4_096).map_err(|error| {
            tree_effect_unknown(format!(
                "archive staging ownership could not be verified before sync: {error}"
            ))
        })?;
        ensure_owned_tree_unchanged(&self.staging_manifest, &observed).map_err(|error| {
            tree_effect_unknown(format!(
                "archive staging changed before durability sync: {error}"
            ))
        })?;
        self.staging_manifest = observed;
        sync_tree_directories(&staging).map_err(|error| {
            tree_effect_unknown(format!("archive staging durability sync failed: {error}"))
        })?;
        fsync(&staging)
            .and_then(|()| fsync(&self.root))
            .map_err(|error| {
                tree_effect_unknown(format!(
                    "archive staging directory durability could not be verified: {}",
                    io::Error::from(error)
                ))
            })?;
        Ok(())
    }

    pub(crate) fn read_staging_regular_file(
        &self,
        relative: &Path,
        max_bytes: u64,
    ) -> Result<String, CommandError> {
        let staging = self.open_staging()?;
        read_relative_regular_file(&staging, relative, max_bytes, None)
    }

    pub(crate) fn snapshot_staging(
        &self,
        max_entries: usize,
        max_total_bytes: u64,
        max_file_bytes: u64,
    ) -> Result<Vec<String>, CommandError> {
        let staging = self.open_staging()?;
        let snapshot = snapshot_directory(
            &staging,
            max_entries,
            max_total_bytes,
            max_file_bytes,
            "external archive staging",
            None,
        )?;
        if snapshot.manifest != self.staging_manifest {
            return Err(CommandError::StaleActionReference);
        }
        Ok(snapshot.rows)
    }

    pub(crate) fn snapshot_target(
        &mut self,
        max_entries: usize,
        max_total_bytes: u64,
        max_file_bytes: u64,
    ) -> Result<Vec<String>, CommandError> {
        let target = self.open_target()?;
        let snapshot = snapshot_directory(
            &target,
            max_entries,
            max_total_bytes,
            max_file_bytes,
            "external archive target",
            Some(&self.display_skill_path),
        )?;
        let expected = if self.backup.is_some() || self.candidate_manifest.is_some() {
            &mut self.candidate_manifest
        } else {
            &mut self.original_manifest
        };
        match expected {
            Some(expected) if *expected != snapshot.manifest => {
                return Err(CommandError::StaleActionReference)
            }
            Some(_) => {}
            None => *expected = Some(snapshot.manifest.clone()),
        }
        Ok(snapshot.rows)
    }

    pub(crate) fn read_target_regular_file(
        &self,
        relative: &Path,
        max_bytes: u64,
    ) -> Result<String, CommandError> {
        let target = self.open_target()?;
        let observed = capture_tree_manifest(&target, 4_096)?;
        let expected = if self.backup.is_some() || self.candidate_manifest.is_some() {
            self.candidate_manifest.as_ref()
        } else {
            self.original_manifest.as_ref()
        }
        .ok_or(CommandError::StaleActionReference)?;
        if &observed != expected {
            return Err(CommandError::StaleActionReference);
        }
        read_relative_regular_file(&target, relative, max_bytes, Some(&self.display_skill_path))
    }

    pub(crate) fn activate(&mut self, backup_name: &OsStr) -> Result<(), CommandError> {
        use rustix::fs::{fsync, renameat_with, statat, AtFlags, RenameFlags};

        validate_single_component(backup_name)?;
        run_test_hook(
            &self.display_skill_path,
            ExternalTargetHookPoint::BeforeTreeActivation,
        );
        self.validate_display_binding()?;
        let original = self
            .target_identity
            .ok_or(CommandError::StaleActionReference)?;
        let (staging_name, staging_identity) = self
            .staging
            .clone()
            .ok_or(CommandError::StaleActionReference)?;
        let original_manifest = self
            .original_manifest
            .clone()
            .ok_or(CommandError::StaleActionReference)?;
        let staging_manifest = self.staging_manifest.clone();
        if capture_tree_manifest(&self.open_target()?, 4_096)? != original_manifest
            || capture_tree_manifest(&self.open_staging()?, 4_096)? != staging_manifest
        {
            return Err(CommandError::StaleActionReference);
        }
        renameat_with(
            &self.target_parent,
            &self.target_name,
            &self.root,
            backup_name,
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| match error {
            rustix::io::Errno::NOENT | rustix::io::Errno::EXIST => {
                CommandError::StaleActionReference
            }
            other => io::Error::from(other).into(),
        })?;
        self.target_identity = None;
        self.backup = Some((backup_name.to_owned(), original));
        run_external_target_fault(ExternalTargetFaultPoint::TreeActivationBackupStat).map_err(
            |error| {
                tree_effect_unknown(format!(
                    "the external archive original was moved to backup, but inspection was interrupted: {error}"
                ))
            },
        )?;
        let moved = statat(&self.root, backup_name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
            tree_effect_unknown(format!(
                "the external archive original was moved to backup, but its identity could not be read: {}",
                io::Error::from(error)
            ))
        })?;
        let moved_identity = EntryIdentity::from_stat(&moved).map_err(|error| {
            tree_effect_unknown(format!(
                "the external archive backup is not owned by the current user: {error}"
            ))
        })?;
        if moved_identity != original {
            renameat_with(
                &self.root,
                backup_name,
                &self.target_parent,
                &self.target_name,
                RenameFlags::NOREPLACE,
            )
            .map_err(|error| tree_effect_unknown(format!(
                "a raced external target directory could not be restored after identity mismatch: {}",
                io::Error::from(error)
            )))?;
            self.backup = None;
            self.target_identity = Some(moved_identity);
            fsync(&self.target_parent)
                .and_then(|()| fsync(&self.root))
                .map_err(|error| tree_effect_unverified(io::Error::from(error)))?;
            return Err(tree_effect_unknown(
                "a raced external target directory was restored without activating the candidate"
                    .to_string(),
            ));
        }
        let backup = open_bound_directory(&self.root, backup_name, original).map_err(|error| {
            tree_effect_unknown(format!(
                "the external archive original was moved to backup, but its descriptor could not be bound: {error}"
            ))
        })?;
        if capture_tree_manifest(&backup, 4_096).map_err(|error| {
            tree_effect_unknown(format!(
                "the external archive backup manifest could not be verified: {error}"
            ))
        })? != original_manifest
        {
            return Err(tree_effect_unknown(
                "the external archive backup no longer matches the accepted original manifest"
                    .to_string(),
            ));
        }
        if let Err(error) = renameat_with(
            &self.root,
            &staging_name,
            &self.target_parent,
            &self.target_name,
            RenameFlags::NOREPLACE,
        ) {
            return Err(tree_effect_unknown(format!(
                "the original was retained in backup, but the archive candidate could not be activated: {}",
                io::Error::from(error)
            )));
        }
        self.staging = None;
        self.target_identity = Some(staging_identity);
        self.candidate_manifest = Some(staging_manifest.clone());
        run_external_target_fault(ExternalTargetFaultPoint::TreeActivationCandidateStat).map_err(
            |error| {
                tree_effect_unknown(format!(
                    "the external archive candidate was activated, but inspection was interrupted: {error}"
                ))
            },
        )?;
        let activated = statat(
            &self.target_parent,
            &self.target_name,
            AtFlags::SYMLINK_NOFOLLOW,
        )
        .map_err(|error| {
            tree_effect_unknown(format!(
                "the external archive candidate was activated, but its identity could not be read: {}",
                io::Error::from(error)
            ))
        })?;
        if EntryIdentity::from_stat(&activated).map_err(|error| {
            tree_effect_unknown(format!(
                "the activated archive candidate is not owned by the current user: {error}"
            ))
        })? != staging_identity
        {
            return Err(tree_effect_unknown(
                "activated external target identity could not be verified".to_string(),
            ));
        }
        let activated_directory =
            open_bound_directory(&self.target_parent, &self.target_name, staging_identity)
                .map_err(|error| {
                    tree_effect_unknown(format!(
                        "the activated external archive descriptor could not be bound: {error}"
                    ))
                })?;
        if capture_tree_manifest(&activated_directory, 4_096).map_err(|error| {
            tree_effect_unknown(format!(
                "the activated external archive manifest could not be verified: {error}"
            ))
        })? != staging_manifest
        {
            return Err(tree_effect_unknown(
                "activated external target tree no longer matches the staged capability"
                    .to_string(),
            ));
        }
        fsync(&self.target_parent)
            .and_then(|()| fsync(&self.root))
            .map_err(|error| tree_effect_unverified(io::Error::from(error)))?;
        self.validate_display_binding()
            .map_err(|error| tree_effect_unverified(io::Error::other(error.to_string())))
    }

    pub(crate) fn restore(&mut self) -> Result<(), CommandError> {
        use rustix::fs::{fsync, renameat_with, statat, AtFlags, RenameFlags};

        run_test_hook(
            &self.display_skill_path,
            ExternalTargetHookPoint::BeforeTreeRestore,
        );
        if self.backup.is_none() {
            return self.discard_staging();
        }
        let (backup_name, original_identity) = self
            .backup
            .clone()
            .ok_or(CommandError::StaleActionReference)?;
        let candidate_manifest = if self.target_identity.is_some() {
            self.candidate_manifest
                .clone()
                .ok_or(CommandError::StaleActionReference)?
        } else {
            self.staging_manifest.clone()
        };
        if let Some(candidate) = self.target_identity {
            let recovery_name = random_temp_name("archive-update-recovery").map_err(|error| {
                tree_effect_unknown(format!(
                    "the external archive original is retained in backup, but candidate quarantine naming failed: {error}"
                ))
            })?;
            renameat_with(
                &self.target_parent,
                &self.target_name,
                &self.root,
                &recovery_name,
                RenameFlags::NOREPLACE,
            )
            .map_err(|error| {
                tree_effect_unknown(format!(
                    "external archive candidate could not be quarantined for restoration: {}",
                    io::Error::from(error)
                ))
            })?;
            self.target_identity = None;
            self.staging = Some((recovery_name.clone(), candidate));
            run_external_target_fault(ExternalTargetFaultPoint::TreeRestoreCandidateStat).map_err(
                |error| {
                    tree_effect_unknown(format!(
                        "the archive candidate was quarantined for restoration, but inspection was interrupted: {error}"
                    ))
                },
            )?;
            let moved = statat(&self.root, &recovery_name, AtFlags::SYMLINK_NOFOLLOW)
                .map_err(|error| {
                    tree_effect_unknown(format!(
                        "the archive candidate was quarantined for restoration, but its identity could not be read: {}",
                        io::Error::from(error)
                    ))
                })?;
            let moved_identity = EntryIdentity::from_stat(&moved).map_err(|error| {
                tree_effect_unknown(format!(
                    "the archive candidate quarantine is not owned by the current user: {error}"
                ))
            })?;
            if moved_identity != candidate {
                return Err(tree_effect_unknown(
                    "external archive candidate changed during restoration".to_string(),
                ));
            }
        }
        if self.staging.is_none() {
            return Err(tree_effect_unknown(
                "archive restoration recovery state is incomplete".to_string(),
            ));
        }
        run_external_target_fault(ExternalTargetFaultPoint::TreeRestoreBackupStat).map_err(
            |error| {
                tree_effect_unknown(format!(
                    "archive restoration backup inspection was interrupted: {error}"
                ))
            },
        )?;
        let backup =
            statat(&self.root, &backup_name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                tree_effect_unknown(format!(
                    "archive restoration backup identity could not be read: {}",
                    io::Error::from(error)
                ))
            })?;
        if EntryIdentity::from_stat(&backup).map_err(|error| {
            tree_effect_unknown(format!(
                "the archive restoration backup is not owned by the current user: {error}"
            ))
        })? != original_identity
        {
            return Err(tree_effect_unknown(
                "archive restoration backup no longer matches the accepted original identity"
                    .to_string(),
            ));
        }
        renameat_with(
            &self.root,
            &backup_name,
            &self.target_parent,
            &self.target_name,
            RenameFlags::NOREPLACE,
        )
        .map_err(|error| {
            tree_effect_unknown(format!(
                "external archive original could not be restored: {}",
                io::Error::from(error)
            ))
        })?;
        self.backup = None;
        self.target_identity = Some(original_identity);
        let restored = self.open_target().map_err(|error| {
            tree_effect_unknown(format!(
                "the archive original was restored, but its descriptor could not be bound: {error}"
            ))
        })?;
        let expected_original = self.original_manifest.clone().ok_or_else(|| {
            tree_effect_unknown(
                "the archive original was restored, but its accepted manifest is unavailable"
                    .to_string(),
            )
        })?;
        if capture_tree_manifest(&restored, 4_096).map_err(|error| {
            tree_effect_unknown(format!(
                "the restored archive original manifest could not be verified: {error}"
            ))
        })? != expected_original
        {
            return Err(tree_effect_unknown(
                "restored external archive original no longer matches its accepted manifest"
                    .to_string(),
            ));
        }
        remove_tree_entry(
            &self.root,
            self.staging.as_mut().ok_or_else(|| {
                tree_effect_unknown(
                    "the archive original was restored, but candidate recovery state was lost"
                        .to_string(),
                )
            })?,
            &candidate_manifest,
            true,
        )?;
        self.staging = None;
        self.candidate_manifest = None;
        fsync(&self.target_parent)
            .and_then(|()| fsync(&self.root))
            .map_err(|error| tree_effect_unverified(io::Error::from(error)))?;
        self.validate_display_binding()
            .map_err(|error| tree_effect_unverified(io::Error::other(error.to_string())))
    }

    pub(crate) fn finish(&mut self) -> Result<(), CommandError> {
        let Some(backup) = self.backup.as_mut() else {
            return Ok(());
        };
        let manifest = self
            .original_manifest
            .as_ref()
            .ok_or(CommandError::StaleActionReference)?;
        remove_tree_entry(&self.root, backup, manifest, true)?;
        self.backup = None;
        Ok(())
    }

    pub(crate) fn discard_staging(&mut self) -> Result<(), CommandError> {
        if let Some(unbound_name) = self.unbound_staging_name.take() {
            let quarantine = allocate_tree_quarantine(&self.root, &unbound_name)?;
            self.unbound_staging_name = Some(quarantine);
            return Err(tree_effect_unknown(
                "an archive staging directory created before identity binding was retained in a private quarantine"
                    .to_string(),
            ));
        }
        let Some(staging) = self.staging.as_mut() else {
            return Ok(());
        };
        if !self.unbound_staging_entries.is_empty() {
            quarantine_bound_tree_entry(&self.root, staging)?;
            return Err(tree_effect_unknown(
                "archive staging contains an entry created before identity binding; the complete staging tree was retained in a private quarantine"
                    .to_string(),
            ));
        }
        remove_tree_entry(&self.root, staging, &self.staging_manifest, false)?;
        self.staging = None;
        self.staging_manifest.clear();
        Ok(())
    }

    fn open_staging(&self) -> Result<File, CommandError> {
        let (name, identity) = self
            .staging
            .as_ref()
            .ok_or(CommandError::StaleActionReference)?;
        open_bound_directory(&self.root, name, *identity)
    }

    fn open_target(&self) -> Result<File, CommandError> {
        let identity = self
            .target_identity
            .ok_or(CommandError::StaleActionReference)?;
        open_bound_directory(&self.target_parent, &self.target_name, identity)
    }

    fn validate_display_binding(&self) -> Result<(), CommandError> {
        use rustix::fs::{openat, Mode};

        let reopened_root =
            crate::mutation_lock::open_existing_directory_nofollow(&self.root_display_path)
                .map_err(|_| CommandError::StaleActionReference)?;
        if EntryIdentity::from_metadata(&reopened_root.metadata()?)?
            != EntryIdentity::from_metadata(&self.root.metadata()?)?
        {
            return Err(CommandError::StaleActionReference);
        }
        let mut current = reopened_root;
        for binding in &self.directory_chain {
            let next = openat(&current, &binding.name, directory_flags(), Mode::empty())
                .map_err(|_| CommandError::StaleActionReference)?;
            let next = File::from(next);
            if EntryIdentity::from_metadata(&next.metadata()?)? != binding.identity {
                return Err(CommandError::StaleActionReference);
            }
            current = next;
        }
        if EntryIdentity::from_metadata(&current.metadata()?)?
            != EntryIdentity::from_metadata(&self.target_parent.metadata()?)?
        {
            return Err(CommandError::StaleActionReference);
        }
        match self.target_identity {
            Some(identity) => {
                open_bound_directory(&self.target_parent, &self.target_name, identity)?;
            }
            None => {
                use rustix::fs::{statat, AtFlags};

                match statat(
                    &self.target_parent,
                    &self.target_name,
                    AtFlags::SYMLINK_NOFOLLOW,
                ) {
                    Err(rustix::io::Errno::NOENT) => {}
                    _ => return Err(CommandError::StaleActionReference),
                }
            }
        }
        Ok(())
    }
}

#[cfg(unix)]
fn open_bound_directory(
    parent: &File,
    name: &OsStr,
    identity: EntryIdentity,
) -> Result<File, CommandError> {
    use rustix::fs::{openat, Mode};

    let directory = openat(parent, name, directory_flags(), Mode::empty())
        .map_err(map_unsafe_external_errno)?;
    let directory = File::from(directory);
    if EntryIdentity::from_metadata(&directory.metadata()?)? != identity {
        return Err(CommandError::StaleActionReference);
    }
    Ok(directory)
}

#[cfg(unix)]
fn open_relative_directory(root: &File, relative: &Path) -> Result<File, CommandError> {
    use rustix::fs::{openat, Mode};

    EntryIdentity::from_metadata(&root.metadata()?)?;
    let mut current = open_directory_clone(root)?;
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(unsafe_external_target("tree path is not normalized"));
        };
        let next = openat(&current, name, directory_flags(), Mode::empty())
            .map_err(map_unsafe_external_errno)?;
        let next = File::from(next);
        EntryIdentity::from_metadata(&next.metadata()?)?;
        current = next;
    }
    Ok(current)
}

#[cfg(unix)]
fn ensure_directory_relative(
    root: &File,
    relative: &Path,
    manifest: &mut TreeManifest,
    unbound_entries: &mut BTreeSet<PathBuf>,
) -> Result<(), CommandError> {
    use rustix::fs::{fchmod, fsync, mkdirat, openat, statat, AtFlags, FileType, Mode};

    validate_relative_components(relative)?;
    let mut current = open_directory_clone(root)?;
    let mut current_relative = PathBuf::new();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(unsafe_external_target("tree path is not normalized"));
        };
        current_relative.push(name);
        match openat(&current, name, directory_flags(), Mode::empty()) {
            Ok(next) => {
                let next = File::from(next);
                let identity = EntryIdentity::from_metadata(&next.metadata()?)?;
                if manifest.get(&current_relative)
                    != Some(&TreeEntryBinding {
                        kind: TreeEntryKind::Directory,
                        identity,
                        file_stamp: None,
                    })
                {
                    return Err(CommandError::StaleActionReference);
                }
                current = next;
            }
            Err(rustix::io::Errno::NOENT) => {
                let mode = Mode::from_bits_truncate(0o700);
                mkdirat(&current, name, mode).map_err(io::Error::from)?;
                unbound_entries.insert(current_relative.clone());
                run_external_target_fault(ExternalTargetFaultPoint::TreeNestedMkdirStat).map_err(
                    |error| {
                        tree_effect_unknown(format!(
                            "a staging directory was created, but identity inspection was interrupted: {error}"
                        ))
                    },
                )?;
                let created =
                    statat(&current, name, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                        tree_effect_unknown(format!(
                        "a staging directory was created, but its identity could not be read: {}",
                        io::Error::from(error)
                    ))
                    })?;
                if FileType::from_raw_mode(created.st_mode) != FileType::Directory {
                    return Err(tree_effect_unknown(
                        "a staging directory was created, but its entry is not a directory"
                            .to_string(),
                    ));
                }
                let identity = EntryIdentity::from_stat(&created).map_err(|error| {
                    tree_effect_unknown(format!(
                        "a staging directory was created, but current-user ownership could not be verified: {error}"
                    ))
                })?;
                if manifest
                    .insert(
                        current_relative.clone(),
                        TreeEntryBinding {
                            kind: TreeEntryKind::Directory,
                            identity,
                            file_stamp: None,
                        },
                    )
                    .is_some()
                {
                    return Err(tree_effect_unknown(
                        "staging directory recovery state collided".to_string(),
                    ));
                }
                unbound_entries.remove(&current_relative);
                run_external_target_fault(ExternalTargetFaultPoint::TreeNestedMkdirOpen).map_err(
                    |error| {
                        tree_effect_unknown(format!(
                            "a staging directory was created, but descriptor binding was interrupted: {error}"
                        ))
                    },
                )?;
                let next = openat(&current, name, directory_flags(), Mode::empty())
                    .map_err(|error| {
                        tree_effect_unknown(format!(
                            "a staging directory was created, but its descriptor could not be opened: {}",
                            io::Error::from(error)
                        ))
                    })?;
                let next = File::from(next);
                run_external_target_fault(
                    ExternalTargetFaultPoint::TreeNestedMkdirMetadata,
                )
                .map_err(|error| {
                    tree_effect_unknown(format!(
                        "a staging directory was opened, but descriptor inspection was interrupted: {error}"
                    ))
                })?;
                let next_metadata = next.metadata().map_err(|error| {
                    tree_effect_unknown(format!(
                        "a staging directory was opened, but descriptor identity could not be read: {error}"
                    ))
                })?;
                if EntryIdentity::from_metadata(&next_metadata).map_err(|error| {
                    tree_effect_unknown(format!(
                        "a staging directory was opened, but current-user ownership could not be verified: {error}"
                    ))
                })? != identity
                {
                    return Err(tree_effect_unknown(
                        "a staging directory changed before its descriptor was bound".to_string(),
                    ));
                }
                run_external_target_fault(ExternalTargetFaultPoint::TreeNestedMkdirChmod).map_err(
                    |error| {
                        tree_effect_unknown(format!(
                            "a staging directory was created, but permission setup was interrupted: {error}"
                        ))
                    },
                )?;
                fchmod(&next, mode).map_err(|error| {
                    tree_effect_unknown(format!(
                        "a staging directory was created, but private permissions could not be verified: {}",
                        io::Error::from(error)
                    ))
                })?;
                run_external_target_fault(ExternalTargetFaultPoint::TreeNestedMkdirSync)
                    .map_err(directory_durability_unknown)?;
                fsync(&next)
                    .and_then(|()| fsync(&current))
                    .map_err(|error| directory_durability_unknown(io::Error::from(error)))?;
                current = next;
            }
            Err(error) => return Err(map_unsafe_external_errno(error)),
        }
    }
    Ok(())
}

#[cfg(unix)]
fn read_relative_regular_file(
    root: &File,
    relative: &Path,
    max_bytes: u64,
    hook_target: Option<&Path>,
) -> Result<String, CommandError> {
    use rustix::fs::{openat, statat, AtFlags, FileType, Mode, OFlags};

    validate_relative_components(relative)?;
    let parent = open_relative_directory(root, relative.parent().unwrap_or_else(|| Path::new("")))?;
    let name = relative
        .file_name()
        .ok_or_else(|| unsafe_external_target("tree file has no name"))?;
    let before_path =
        statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(map_unsafe_external_errno)?;
    if FileType::from_raw_mode(before_path.st_mode) != FileType::RegularFile {
        return Err(CommandError::StaleActionReference);
    }
    let before_path = RegularFileStamp::from_stat(&before_path)?;
    let descriptor = openat(
        &parent,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(map_unsafe_external_errno)?;
    let mut file = File::from(descriptor);
    let metadata = file.metadata()?;
    let before_read = RegularFileStamp::from_metadata(&metadata)?;
    if !metadata.is_file()
        || before_read.links != 1
        || before_read.length > max_bytes
        || before_path != before_read
    {
        return Err(unsafe_external_target(
            "tree file is not a bounded regular file",
        ));
    }
    if let Some(target) = hook_target {
        run_test_hook(target, ExternalTargetHookPoint::DuringTreeRead);
    }
    let mut bytes = Vec::with_capacity(before_read.length as usize);
    Read::by_ref(&mut file)
        .take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    let after_read = RegularFileStamp::from_metadata(&file.metadata()?)?;
    if bytes.len() as u64 != before_read.length
        || bytes.len() as u64 > max_bytes
        || after_read != before_read
    {
        return Err(CommandError::StaleActionReference);
    }
    String::from_utf8(bytes).map_err(|_| unsafe_external_target("tree file is not valid UTF-8"))
}

#[cfg(unix)]
fn snapshot_directory(
    root: &File,
    max_entries: usize,
    max_total_bytes: u64,
    max_file_bytes: u64,
    label: &str,
    hook_target: Option<&Path>,
) -> Result<TreeSnapshot, CommandError> {
    use rustix::fs::{openat, statat, AtFlags, Dir, FileType, Mode, OFlags};
    use std::os::unix::ffi::OsStrExt;

    EntryIdentity::from_metadata(&root.metadata()?)?;
    let mut pending = vec![(PathBuf::new(), open_directory_clone(root)?)];
    let mut rows = BTreeSet::new();
    let mut manifest = TreeManifest::new();
    let mut entry_count = 0usize;
    let mut total_bytes = 0u64;
    let mut read_hook_pending = hook_target;
    while let Some((relative_directory, directory)) = pending.pop() {
        let mut entries = Dir::read_from(&directory).map_err(io::Error::from)?;
        let mut names = Vec::new();
        for entry in &mut entries {
            let entry = entry.map_err(io::Error::from)?;
            let bytes = entry.file_name().to_bytes();
            if !matches!(bytes, b"." | b"..") {
                names.push(OsStr::from_bytes(bytes).to_owned());
            }
        }
        names.sort();
        for name in names {
            entry_count = entry_count.saturating_add(1);
            if entry_count > max_entries {
                return Err(CommandError::InvalidSkillManagerRequest(format!(
                    "{label} exceeds its entry safety budget"
                )));
            }
            let relative = relative_directory.join(&name);
            let stat =
                statat(&directory, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
            match FileType::from_raw_mode(stat.st_mode) {
                FileType::Directory => {
                    let child = openat(&directory, &name, directory_flags(), Mode::empty())
                        .map_err(map_unsafe_external_errno)?;
                    let child = File::from(child);
                    let identity = EntryIdentity::from_metadata(&child.metadata()?)?;
                    let path_identity = EntryIdentity::from_stat(&stat)?;
                    if identity != path_identity {
                        return Err(CommandError::StaleActionReference);
                    }
                    rows.insert(format!("dir:{}", relative.to_string_lossy()));
                    manifest.insert(
                        relative.clone(),
                        TreeEntryBinding {
                            kind: TreeEntryKind::Directory,
                            identity,
                            file_stamp: None,
                        },
                    );
                    pending.push((relative, child));
                }
                FileType::RegularFile => {
                    let before_path = RegularFileStamp::from_stat(&stat)?;
                    let descriptor = openat(
                        &directory,
                        &name,
                        OFlags::RDONLY
                            | OFlags::NOFOLLOW
                            | OFlags::NONBLOCK
                            | OFlags::NOCTTY
                            | OFlags::CLOEXEC,
                        Mode::empty(),
                    )
                    .map_err(map_unsafe_external_errno)?;
                    let mut file = File::from(descriptor);
                    let metadata = file.metadata()?;
                    let before_read = RegularFileStamp::from_metadata(&metadata)?;
                    if !metadata.is_file()
                        || before_read.links != 1
                        || before_read.length > max_file_bytes
                        || before_path != before_read
                    {
                        return Err(unsafe_external_target(
                            "tree contains an unsafe regular file",
                        ));
                    }
                    total_bytes = total_bytes
                        .checked_add(before_read.length)
                        .filter(|total| *total <= max_total_bytes)
                        .ok_or_else(|| {
                            CommandError::InvalidSkillManagerRequest(format!(
                                "{label} exceeds its byte safety budget"
                            ))
                        })?;
                    if let Some(target) = read_hook_pending.take() {
                        run_test_hook(target, ExternalTargetHookPoint::DuringTreeRead);
                    }
                    let mut bytes = Vec::with_capacity(before_read.length as usize);
                    Read::by_ref(&mut file)
                        .take(max_file_bytes.saturating_add(1))
                        .read_to_end(&mut bytes)?;
                    let after_read = RegularFileStamp::from_metadata(&file.metadata()?)?;
                    if bytes.len() as u64 != before_read.length
                        || bytes.len() as u64 > max_file_bytes
                        || after_read != before_read
                    {
                        return Err(CommandError::StaleActionReference);
                    }
                    rows.insert(format!(
                        "file:{}:{}:{:x}",
                        relative.to_string_lossy(),
                        before_read.length,
                        Sha256::digest(&bytes)
                    ));
                    manifest.insert(
                        relative,
                        TreeEntryBinding {
                            kind: TreeEntryKind::RegularFile,
                            identity: EntryIdentity {
                                device: before_read.device,
                                inode: before_read.inode,
                                owner: before_read.owner,
                            },
                            file_stamp: Some(before_read),
                        },
                    );
                }
                FileType::Symlink => return Err(unsafe_external_target("tree contains a symlink")),
                _ => return Err(unsafe_external_target("tree contains a special file")),
            }
        }
    }
    let observed = capture_tree_manifest(root, max_entries)?;
    if observed != manifest {
        return Err(CommandError::StaleActionReference);
    }
    Ok(TreeSnapshot {
        rows: rows.into_iter().collect(),
        manifest,
    })
}

#[cfg(unix)]
fn capture_tree_manifest(root: &File, max_entries: usize) -> Result<TreeManifest, CommandError> {
    use rustix::fs::{openat, statat, AtFlags, Dir, FileType, Mode, OFlags};
    use std::os::unix::ffi::OsStrExt;

    EntryIdentity::from_metadata(&root.metadata()?)?;
    let mut manifest = TreeManifest::new();
    let mut pending = vec![(PathBuf::new(), open_directory_clone(root)?)];
    while let Some((relative_directory, directory)) = pending.pop() {
        let mut entries = Dir::read_from(&directory).map_err(io::Error::from)?;
        let mut names = Vec::new();
        for entry in &mut entries {
            let entry = entry.map_err(io::Error::from)?;
            let bytes = entry.file_name().to_bytes();
            if !matches!(bytes, b"." | b"..") {
                names.push(OsStr::from_bytes(bytes).to_owned());
            }
        }
        names.sort();
        for name in names {
            if manifest.len() >= max_entries {
                return Err(unsafe_external_target(
                    "tree exceeds its identity-manifest entry budget",
                ));
            }
            let relative = relative_directory.join(&name);
            let stat =
                statat(&directory, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
            match FileType::from_raw_mode(stat.st_mode) {
                FileType::Directory => {
                    let descriptor = openat(&directory, &name, directory_flags(), Mode::empty())
                        .map_err(map_unsafe_external_errno)?;
                    let child = File::from(descriptor);
                    let identity = EntryIdentity::from_metadata(&child.metadata()?)?;
                    if identity != EntryIdentity::from_stat(&stat)? {
                        return Err(CommandError::StaleActionReference);
                    }
                    manifest.insert(
                        relative.clone(),
                        TreeEntryBinding {
                            kind: TreeEntryKind::Directory,
                            identity,
                            file_stamp: None,
                        },
                    );
                    pending.push((relative, child));
                }
                FileType::RegularFile => {
                    let path_stamp = RegularFileStamp::from_stat(&stat)?;
                    if path_stamp.links != 1 {
                        return Err(unsafe_external_target(
                            "tree identity manifest rejects hard-linked files",
                        ));
                    }
                    let descriptor = openat(
                        &directory,
                        &name,
                        OFlags::RDONLY
                            | OFlags::NOFOLLOW
                            | OFlags::NONBLOCK
                            | OFlags::NOCTTY
                            | OFlags::CLOEXEC,
                        Mode::empty(),
                    )
                    .map_err(map_unsafe_external_errno)?;
                    let file = File::from(descriptor);
                    let opened_stamp = RegularFileStamp::from_metadata(&file.metadata()?)?;
                    if path_stamp != opened_stamp
                        || RegularFileStamp::from_metadata(&file.metadata()?)? != opened_stamp
                    {
                        return Err(CommandError::StaleActionReference);
                    }
                    manifest.insert(
                        relative,
                        TreeEntryBinding {
                            kind: TreeEntryKind::RegularFile,
                            identity: EntryIdentity {
                                device: opened_stamp.device,
                                inode: opened_stamp.inode,
                                owner: opened_stamp.owner,
                            },
                            file_stamp: Some(opened_stamp),
                        },
                    );
                }
                FileType::Symlink => return Err(unsafe_external_target("tree contains a symlink")),
                _ => return Err(unsafe_external_target("tree contains a special file")),
            }
        }
    }
    Ok(manifest)
}

#[cfg(unix)]
fn ensure_owned_tree_unchanged(
    owned: &TreeManifest,
    observed: &TreeManifest,
) -> Result<(), CommandError> {
    if owned.len() != observed.len()
        || owned.iter().any(|(path, expected)| {
            observed.get(path).is_none_or(|actual| {
                expected.kind != actual.kind || expected.identity != actual.identity
            })
        })
    {
        return Err(CommandError::StaleActionReference);
    }
    Ok(())
}

#[cfg(unix)]
fn remove_tree_entry(
    parent: &File,
    entry: &mut (OsString, EntryIdentity),
    manifest: &TreeManifest,
    require_exact_stamps: bool,
) -> Result<(), CommandError> {
    use rustix::fs::{fsync, renameat_with, statat, unlinkat, AtFlags, RenameFlags};

    let original_name = entry.0.clone();
    let quarantine = allocate_tree_quarantine(parent, &original_name)?;
    entry.0 = quarantine.clone();
    run_external_target_fault(ExternalTargetFaultPoint::TreeQuarantineStat).map_err(|error| {
        tree_effect_unknown(format!(
            "a tree was atomically quarantined, but inspection was interrupted: {error}"
        ))
    })?;
    let moved = statat(parent, &quarantine, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
        tree_effect_unknown(format!(
            "a tree was atomically quarantined, but its identity could not be read: {}",
            io::Error::from(error)
        ))
    })?;
    let moved_identity = EntryIdentity::from_stat(&moved).map_err(|error| {
        tree_effect_unknown(format!(
            "the tree quarantine is not owned by the current user: {error}"
        ))
    })?;
    if moved_identity != entry.1 {
        match renameat_with(
            parent,
            &quarantine,
            parent,
            &original_name,
            RenameFlags::NOREPLACE,
        ) {
            Ok(()) => {
                entry.0 = original_name;
                fsync(parent).map_err(|error| tree_effect_unverified(io::Error::from(error)))?;
                return Err(tree_effect_unknown(
                    "a raced tree replacement was restored without deleting it".to_string(),
                ));
            }
            Err(error) => {
                return Err(tree_effect_unknown(format!(
                    "a raced tree replacement was quarantined but could not be restored: {}",
                    io::Error::from(error)
                )))
            }
        }
    }
    let directory = open_bound_directory(parent, &quarantine, entry.1).map_err(|error| {
        tree_effect_unknown(format!(
            "the quarantined tree descriptor could not be identity-bound: {error}"
        ))
    })?;
    let observed =
        capture_tree_manifest(&directory, manifest.len().saturating_add(1)).map_err(|error| {
            tree_effect_unknown(format!(
                "the quarantined tree manifest could not be verified: {error}"
            ))
        })?;
    let manifest_matches = if require_exact_stamps {
        observed == *manifest
    } else {
        ensure_owned_tree_unchanged(manifest, &observed).is_ok()
    };
    if !manifest_matches {
        return Err(tree_effect_unknown(
            "tree cleanup preserved a quarantined tree because its complete identity manifest changed"
                .to_string(),
        ));
    }
    remove_manifest_entries(&directory, manifest, require_exact_stamps).map_err(
        |error| match error {
            partial @ CommandError::PartialEffect { .. } => partial,
            other => tree_effect_unknown(format!(
                "the tree was quarantined, but manifest-bound cleanup could not complete: {other}"
            )),
        },
    )?;
    unlinkat(parent, &quarantine, AtFlags::REMOVEDIR).map_err(|error| {
        tree_effect_unknown(format!(
            "the verified private tree quarantine could not be removed: {}",
            io::Error::from(error)
        ))
    })?;
    run_external_target_fault(ExternalTargetFaultPoint::TreeQuarantineSync)
        .map_err(tree_effect_unverified)?;
    fsync(parent).map_err(|error| tree_effect_unverified(io::Error::from(error)))
}

#[cfg(unix)]
fn allocate_tree_quarantine(parent: &File, source: &OsStr) -> Result<OsString, CommandError> {
    use rustix::fs::{renameat_with, RenameFlags};

    for _ in 0..32_u32 {
        let name = random_temp_name("agent-copilot-tree-quarantine")?;
        match renameat_with(parent, source, parent, &name, RenameFlags::NOREPLACE) {
            Ok(()) => return Ok(name),
            Err(rustix::io::Errno::EXIST) => continue,
            Err(rustix::io::Errno::NOENT) => return Err(CommandError::StaleActionReference),
            Err(error) => return Err(io::Error::from(error).into()),
        }
    }
    Err(tree_effect_unknown(
        "tree quarantine allocation was exhausted".to_string(),
    ))
}

#[cfg(unix)]
fn quarantine_bound_tree_entry(
    parent: &File,
    entry: &mut (OsString, EntryIdentity),
) -> Result<(), CommandError> {
    use rustix::fs::{statat, AtFlags};

    let quarantine = allocate_tree_quarantine(parent, &entry.0)?;
    entry.0 = quarantine.clone();
    run_external_target_fault(ExternalTargetFaultPoint::TreeQuarantineStat).map_err(|error| {
        tree_effect_unknown(format!(
            "a bound tree was quarantined, but inspection was interrupted: {error}"
        ))
    })?;
    let moved = statat(parent, &quarantine, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
        tree_effect_unknown(format!(
            "a bound tree was quarantined, but its identity could not be read: {}",
            io::Error::from(error)
        ))
    })?;
    if EntryIdentity::from_stat(&moved).map_err(|error| {
        tree_effect_unknown(format!(
            "the bound tree quarantine is not owned by the current user: {error}"
        ))
    })? != entry.1
    {
        return Err(tree_effect_unknown(
            "a quarantined tree no longer matches its bound identity".to_string(),
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn remove_manifest_entries(
    root: &File,
    manifest: &TreeManifest,
    require_exact_stamps: bool,
) -> Result<(), CommandError> {
    use rustix::fs::{fsync, renameat_with, statat, unlinkat, AtFlags, Dir, RenameFlags};

    let mut paths = manifest.keys().cloned().collect::<Vec<_>>();
    paths.sort_by(|left, right| {
        right
            .components()
            .count()
            .cmp(&left.components().count())
            .then_with(|| right.cmp(left))
    });
    for relative in paths {
        let expected = *manifest
            .get(&relative)
            .ok_or(CommandError::StaleActionReference)?;
        let parent_relative = relative.parent().unwrap_or_else(|| Path::new(""));
        let parent = open_manifest_directory(root, parent_relative, manifest).map_err(|error| {
            tree_effect_unknown(format!(
                "a quarantined tree manifest parent could not be identity-bound: {error}"
            ))
        })?;
        let name = relative
            .file_name()
            .ok_or_else(|| unsafe_external_target("tree manifest entry has no name"))?;
        let before = statat(&parent, name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
        let before_matches = tree_binding_from_stat(&before)?.is_some_and(|binding| {
            if require_exact_stamps {
                binding == expected
            } else {
                binding.kind == expected.kind && binding.identity == expected.identity
            }
        });
        if !before_matches {
            return Err(CommandError::StaleActionReference);
        }
        let quarantine = allocate_tree_quarantine(&parent, name)?;
        run_external_target_fault(ExternalTargetFaultPoint::TreeQuarantineStat).map_err(
            |error| {
                tree_effect_unknown(format!(
                    "a manifest entry was quarantined, but inspection was interrupted: {error}"
                ))
            },
        )?;
        let observed =
            statat(&parent, &quarantine, AtFlags::SYMLINK_NOFOLLOW).map_err(|error| {
                tree_effect_unknown(format!(
                    "a manifest entry was quarantined, but its identity could not be read: {}",
                    io::Error::from(error)
                ))
            })?;
        let matches = tree_binding_from_stat(&observed)?.is_some_and(|binding| {
            binding.kind == expected.kind
                && binding.identity == expected.identity
                && (!require_exact_stamps
                    || tree_file_stamp_matches_after_rename(binding, expected))
        });
        if !matches {
            renameat_with(&parent, &quarantine, &parent, name, RenameFlags::NOREPLACE).map_err(
                |error| {
                    tree_effect_unknown(format!(
                        "a raced manifest entry was quarantined but could not be restored: {}",
                        io::Error::from(error)
                    ))
                },
            )?;
            fsync(&parent).map_err(|error| tree_effect_unverified(io::Error::from(error)))?;
            return Err(tree_effect_unknown(
                "a raced manifest entry was restored without deleting it".to_string(),
            ));
        }
        match expected.kind {
            TreeEntryKind::RegularFile => {
                unlinkat(&parent, &quarantine, AtFlags::empty()).map_err(|error| {
                    tree_effect_unknown(format!(
                        "verified file quarantine cleanup failed: {}",
                        io::Error::from(error)
                    ))
                })?;
            }
            TreeEntryKind::Directory => {
                let directory = open_bound_directory(&parent, &quarantine, expected.identity)
                    .map_err(|error| {
                        tree_effect_unknown(format!(
                            "a quarantined manifest directory could not be identity-bound: {error}"
                        ))
                    })?;
                let mut entries = Dir::read_from(&directory).map_err(|error| {
                    tree_effect_unknown(format!(
                        "a quarantined manifest directory could not be enumerated: {}",
                        io::Error::from(error)
                    ))
                })?;
                if entries.any(|entry| {
                    entry.is_err()
                        || !entry
                            .as_ref()
                            .is_ok_and(|entry| matches!(entry.file_name().to_bytes(), b"." | b".."))
                }) {
                    return Err(tree_effect_unknown(
                        "verified directory quarantine gained an unexpected entry".to_string(),
                    ));
                }
                unlinkat(&parent, &quarantine, AtFlags::REMOVEDIR).map_err(|error| {
                    tree_effect_unknown(format!(
                        "verified directory quarantine cleanup failed: {}",
                        io::Error::from(error)
                    ))
                })?;
            }
        }
        run_external_target_fault(ExternalTargetFaultPoint::TreeQuarantineSync)
            .map_err(tree_effect_unverified)?;
        fsync(&parent).map_err(|error| tree_effect_unverified(io::Error::from(error)))?;
    }
    Ok(())
}

#[cfg(unix)]
fn open_manifest_directory(
    root: &File,
    relative: &Path,
    manifest: &TreeManifest,
) -> Result<File, CommandError> {
    use rustix::fs::{openat, Mode};

    let mut current = open_directory_clone(root)?;
    let mut current_relative = PathBuf::new();
    for component in relative.components() {
        let Component::Normal(name) = component else {
            return Err(unsafe_external_target(
                "tree manifest path is not normalized",
            ));
        };
        current_relative.push(name);
        let binding = manifest
            .get(&current_relative)
            .filter(|binding| binding.kind == TreeEntryKind::Directory)
            .ok_or(CommandError::StaleActionReference)?;
        let next = openat(&current, name, directory_flags(), Mode::empty())
            .map_err(map_unsafe_external_errno)?;
        let next = File::from(next);
        if EntryIdentity::from_metadata(&next.metadata()?)? != binding.identity {
            return Err(CommandError::StaleActionReference);
        }
        current = next;
    }
    Ok(current)
}

#[cfg(unix)]
fn tree_binding_from_stat(
    stat: &rustix::fs::Stat,
) -> Result<Option<TreeEntryBinding>, CommandError> {
    use rustix::fs::FileType;

    let identity = EntryIdentity::from_stat(stat)?;
    match FileType::from_raw_mode(stat.st_mode) {
        FileType::Directory => Ok(Some(TreeEntryBinding {
            kind: TreeEntryKind::Directory,
            identity,
            file_stamp: None,
        })),
        FileType::RegularFile => {
            let stamp = RegularFileStamp::from_stat(stat)?;
            if stamp.links != 1 {
                return Ok(None);
            }
            Ok(Some(TreeEntryBinding {
                kind: TreeEntryKind::RegularFile,
                identity,
                file_stamp: Some(stamp),
            }))
        }
        _ => Ok(None),
    }
}

#[cfg(unix)]
fn tree_file_stamp_matches_after_rename(
    observed: TreeEntryBinding,
    expected: TreeEntryBinding,
) -> bool {
    match (observed.file_stamp, expected.file_stamp) {
        (None, None) => true,
        (Some(observed), Some(expected)) => {
            observed.device == expected.device
                && observed.inode == expected.inode
                && observed.owner == expected.owner
                && observed.mode == expected.mode
                && observed.links == 1
                && expected.links == 1
                && observed.length == expected.length
                && observed.modified_seconds == expected.modified_seconds
                && observed.modified_nanoseconds == expected.modified_nanoseconds
        }
        _ => false,
    }
}

#[cfg(unix)]
fn sync_tree_directories(root: &File) -> Result<(), CommandError> {
    use rustix::fs::{fsync, openat, statat, AtFlags, Dir, FileType, Mode};
    use std::os::unix::ffi::OsStrExt;

    EntryIdentity::from_metadata(&root.metadata()?)?;
    let mut pending = vec![open_directory_clone(root)?];
    while let Some(directory) = pending.pop() {
        EntryIdentity::from_metadata(&directory.metadata()?)?;
        let mut entries = Dir::read_from(&directory).map_err(io::Error::from)?;
        let mut names = Vec::new();
        for entry in &mut entries {
            let entry = entry.map_err(io::Error::from)?;
            let bytes = entry.file_name().to_bytes();
            if !matches!(bytes, b"." | b"..") {
                names.push(OsStr::from_bytes(bytes).to_owned());
            }
        }
        for name in names {
            let stat =
                statat(&directory, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
            match FileType::from_raw_mode(stat.st_mode) {
                FileType::Directory => {
                    let expected = EntryIdentity::from_stat(&stat)?;
                    let child = openat(&directory, &name, directory_flags(), Mode::empty())
                        .map_err(map_unsafe_external_errno)?;
                    let child = File::from(child);
                    if EntryIdentity::from_metadata(&child.metadata()?)? != expected {
                        return Err(CommandError::StaleActionReference);
                    }
                    pending.push(child);
                }
                FileType::RegularFile => {
                    if RegularFileStamp::from_stat(&stat)?.links != 1 {
                        return Err(unsafe_external_target(
                            "tree durability sync rejects hard-linked files",
                        ));
                    }
                }
                FileType::Symlink => {
                    return Err(unsafe_external_target(
                        "tree durability sync rejects symlinks",
                    ))
                }
                _ => {
                    return Err(unsafe_external_target(
                        "tree durability sync rejects special files",
                    ))
                }
            }
        }
        fsync(&directory).map_err(io::Error::from)?;
    }
    Ok(())
}

fn validate_relative_components(path: &Path) -> Result<(), CommandError> {
    if path.as_os_str().is_empty()
        || path.is_absolute()
        || path
            .components()
            .any(|component| !matches!(component, Component::Normal(_)))
    {
        return Err(unsafe_external_target("tree path is not normalized"));
    }
    Ok(())
}

fn validate_single_component(name: &OsStr) -> Result<(), CommandError> {
    let path = Path::new(name);
    if name.is_empty()
        || path.components().count() != 1
        || !matches!(path.components().next(), Some(Component::Normal(_)))
    {
        return Err(unsafe_external_target("tree entry name is unsafe"));
    }
    Ok(())
}

#[cfg(unix)]
fn ensure_current_user_owner(owner: u32) -> Result<(), CommandError> {
    if owner != rustix::process::geteuid().as_raw() {
        return Err(unsafe_external_target(
            "guarded external target contains an entry not owned by the current user",
        ));
    }
    Ok(())
}

fn tree_effect_unknown(detail: String) -> CommandError {
    CommandError::PartialEffect {
        operation: "external archive target".to_string(),
        state: "outcome_unknown",
        cleanup_required: true,
        detail,
    }
}

fn tree_effect_unverified(error: io::Error) -> CommandError {
    CommandError::PartialEffect {
        operation: "external archive target".to_string(),
        state: "applied_unverified",
        cleanup_required: true,
        detail: format!("external archive target durability or binding is unverified: {error}"),
    }
}

#[cfg(all(test, unix))]
mod ownership_tests {
    use super::*;

    #[test]
    fn current_user_owner_guard_rejects_foreign_entries() {
        let current = rustix::process::geteuid().as_raw();
        let foreign = if current == u32::MAX {
            current - 1
        } else {
            current + 1
        };

        assert!(ensure_current_user_owner(current).is_ok());
        assert!(matches!(
            ensure_current_user_owner(foreign),
            Err(CommandError::UnsafeConfigPath(_))
        ));
    }

    #[test]
    fn external_capability_rejects_a_foreign_owned_allowed_root() {
        use std::os::unix::fs::MetadataExt;

        let root_metadata = std::fs::metadata("/").expect("inspect filesystem root");
        if root_metadata.uid() == rustix::process::geteuid().as_raw() {
            return;
        }

        let test_root = std::env::temp_dir()
            .join(random_temp_name("agent-copilot-external-owner").expect("allocate test suffix"));
        let app_data = test_root.join("app-data");
        let target = test_root.join("settings.json");
        std::fs::create_dir_all(&test_root).expect("create test root");
        std::fs::write(&target, "{}\n").expect("write target");
        let lock = crate::mutation_lock::lock_or_create_app_mutations(&app_data)
            .expect("create mutation owner");

        assert!(matches!(
            ExternalTargetCapability::prepare(&lock, Path::new("/"), &target),
            Err(CommandError::UnsafeConfigPath(_))
        ));

        drop(lock);
        std::fs::remove_dir_all(test_root).expect("remove test root");
    }
}

#[cfg(all(test, unix))]
thread_local! {
    static EXTERNAL_TARGET_FAULTS: std::cell::RefCell<Vec<ExternalTargetFaultPoint>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(all(test, unix))]
pub(crate) fn install_external_target_fault(point: ExternalTargetFaultPoint) {
    EXTERNAL_TARGET_FAULTS.with(|faults| faults.borrow_mut().push(point));
}

#[cfg(all(test, unix))]
fn run_external_target_fault(point: ExternalTargetFaultPoint) -> Result<(), io::Error> {
    EXTERNAL_TARGET_FAULTS.with(|faults| {
        let mut faults = faults.borrow_mut();
        if let Some(index) = faults.iter().position(|candidate| *candidate == point) {
            faults.remove(index);
            return Err(io::Error::other(format!(
                "injected external target fault at {point:?}"
            )));
        }
        Ok(())
    })
}

#[cfg(all(not(test), unix))]
fn run_external_target_fault(_point: ExternalTargetFaultPoint) -> Result<(), io::Error> {
    Ok(())
}

#[cfg(test)]
thread_local! {
    static EXTERNAL_PARENT_SYNC_FAILURES: std::cell::RefCell<Vec<PathBuf>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(crate) fn install_external_parent_sync_failure(target: PathBuf) {
    EXTERNAL_PARENT_SYNC_FAILURES.with(|failures| failures.borrow_mut().push(target));
}

#[cfg(unix)]
fn sync_external_parent_after_install(
    parent: &File,
    _display_target: &Path,
) -> Result<(), CommandError> {
    use rustix::fs::fsync;

    #[cfg(test)]
    {
        let injected = EXTERNAL_PARENT_SYNC_FAILURES.with(|failures| {
            let mut failures = failures.borrow_mut();
            failures
                .iter()
                .position(|target| target == _display_target)
                .map(|index| failures.remove(index))
                .is_some()
        });
        if injected {
            return Err(file_effect_unverified(io::Error::other(
                "injected external target parent sync failure",
            )));
        }
    }
    fsync(parent).map_err(|error| file_effect_unverified(io::Error::from(error)))
}

#[cfg(test)]
struct ExternalTargetTestHook {
    target: PathBuf,
    point: ExternalTargetHookPoint,
    action: Box<dyn FnOnce() + Send>,
}

#[cfg(test)]
thread_local! {
    static EXTERNAL_TARGET_TEST_HOOKS: std::cell::RefCell<Vec<ExternalTargetTestHook>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(crate) fn install_external_target_test_hook(
    target: PathBuf,
    point: ExternalTargetHookPoint,
    action: impl FnOnce() + Send + 'static,
) {
    EXTERNAL_TARGET_TEST_HOOKS.with(|hooks| {
        hooks.borrow_mut().push(ExternalTargetTestHook {
            target,
            point,
            action: Box::new(action),
        })
    });
}

#[cfg(test)]
fn run_test_hook(target: &Path, point: ExternalTargetHookPoint) {
    let action = EXTERNAL_TARGET_TEST_HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        hooks
            .iter()
            .position(|hook| hook.target == target && hook.point == point)
            .map(|index| hooks.remove(index).action)
    });
    if let Some(action) = action {
        action();
    }
}

#[cfg(not(test))]
fn run_test_hook(_target: &Path, _point: ExternalTargetHookPoint) {}
