use std::{
    ffi::{OsStr, OsString},
    fs::File,
    io::{self, Read, Write},
    path::{Component, Path, PathBuf},
    sync::atomic::{AtomicU64, Ordering},
    time::{SystemTime, UNIX_EPOCH},
};

use sha2::{Digest, Sha256};
use skills_copilot_commands::{lock_app_mutations, AppMutationLock, CommandError};

use crate::{ServiceError, DEFAULT_BUNDLE_ID, LEGACY_BUNDLE_ID};

const MIGRATION_MARKER_NAME: &str = "agent-copilot-app-data-migration.json";
const MIGRATION_ENTRY_LIMIT: usize = 100_000;
const MIGRATION_FILE_BYTE_LIMIT: u64 = 1024 * 1024 * 1024;
const MIGRATION_TOTAL_BYTE_LIMIT: u64 = 8 * 1024 * 1024 * 1024;
const MIGRATION_DEPTH_LIMIT: usize = 128;
const STAGING_ATTEMPT_LIMIT: u32 = 64;

static STAGING_SEQUENCE: AtomicU64 = AtomicU64::new(0);

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
struct FileIdentity {
    device: u64,
    inode: u64,
    uid: u32,
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum ManifestKind {
    Directory,
    File { size: u64, digest: [u8; 32] },
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ManifestEntry {
    relative: PathBuf,
    kind: ManifestKind,
}

#[derive(Debug, Default)]
struct CopyBudget {
    entries: usize,
    total_bytes: u64,
}

#[derive(Debug)]
struct StagingState {
    name: OsString,
    identity: FileIdentity,
    directory: File,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum MigrationPoint {
    BeforeFirstEffect,
    BeforeStagingCreate,
    AfterStagingCreated,
    BeforeActivation,
    AfterActivation,
}

#[derive(Debug)]
#[cfg_attr(not(test), allow(dead_code))]
struct MigrationContext<'a> {
    parent_path: &'a Path,
    source_path: &'a Path,
    target_path: &'a Path,
    staging_name: Option<&'a OsStr>,
}

pub(crate) fn migrate_legacy_app_data_dir(
    source: &Path,
    target: &Path,
) -> Result<(), ServiceError> {
    migrate_legacy_app_data_dir_with_hook(source, target, &mut |_, _| {})
}

fn migrate_legacy_app_data_dir_with_hook(
    source: &Path,
    target: &Path,
    hook: &mut impl FnMut(MigrationPoint, &MigrationContext<'_>),
) -> Result<(), ServiceError> {
    #[cfg(unix)]
    {
        migrate_legacy_app_data_dir_unix(source, target, hook)
    }
    #[cfg(not(unix))]
    {
        let _ = hook;
        migrate_legacy_app_data_dir_unsupported(source, target)
    }
}

#[cfg(unix)]
fn migrate_legacy_app_data_dir_unix(
    source: &Path,
    target: &Path,
    hook: &mut impl FnMut(MigrationPoint, &MigrationContext<'_>),
) -> Result<(), ServiceError> {
    use std::os::unix::fs::MetadataExt;

    let (parent_path, source_name, target_name) = migration_names(source, target)?;
    let parent_lock = match lock_app_mutations(parent_path) {
        Ok(lock) => lock,
        Err(CommandError::Io(error)) if error.kind() == io::ErrorKind::NotFound => return Ok(()),
        Err(error) => return Err(error.into()),
    };
    let parent_metadata = parent_lock.owner_directory().metadata()?;
    let owner_uid = parent_metadata.uid();
    if owner_uid != rustix::process::geteuid().as_raw() {
        return Err(invalid_migration(
            "app data migration parent is not owned by the current user",
        ));
    }

    if let Some(target_directory) =
        open_optional_owned_directory(parent_lock.owner_directory(), target_name, owner_uid)?
    {
        let target_identity = file_identity(&target_directory)?;
        parent_lock.validate_owner_path_binding()?;
        let current =
            open_optional_owned_directory(parent_lock.owner_directory(), target_name, owner_uid)?
                .ok_or_else(|| {
                invalid_migration("existing app data target changed during validation")
            })?;
        if file_identity(&current)? != target_identity {
            return Err(invalid_migration(
                "existing app data target changed during validation",
            ));
        }
        return Ok(());
    }

    let Some(source_directory) =
        open_optional_owned_directory(parent_lock.owner_directory(), source_name, owner_uid)?
    else {
        parent_lock.validate_owner_path_binding()?;
        return Ok(());
    };
    let source_identity = file_identity(&source_directory)?;

    let context = MigrationContext {
        parent_path,
        source_path: source,
        target_path: target,
        staging_name: None,
    };
    hook(MigrationPoint::BeforeFirstEffect, &context);
    validate_pre_activation_bindings(
        &parent_lock,
        source_name,
        source_identity,
        target_name,
        None,
        owner_uid,
    )?;

    let mut staging: Option<StagingState> = None;
    let mut activated = false;
    let attempt = (|| -> Result<(), ServiceError> {
        for attempt in 0..STAGING_ATTEMPT_LIMIT {
            let candidate = staging_name(target_name, attempt);
            let context = MigrationContext {
                parent_path,
                source_path: source,
                target_path: target,
                staging_name: Some(&candidate),
            };
            hook(MigrationPoint::BeforeStagingCreate, &context);
            validate_pre_activation_bindings(
                &parent_lock,
                source_name,
                source_identity,
                target_name,
                None,
                owner_uid,
            )?;
            match create_staging_directory(&parent_lock, &candidate, owner_uid)? {
                Some(created) => {
                    staging = Some(created);
                    break;
                }
                None => continue,
            }
        }
        let staging = staging.as_ref().ok_or_else(|| {
            invalid_migration("app data migration could not allocate private staging")
        })?;
        let context = MigrationContext {
            parent_path,
            source_path: source,
            target_path: target,
            staging_name: Some(&staging.name),
        };
        hook(MigrationPoint::AfterStagingCreated, &context);
        validate_pre_activation_bindings(
            &parent_lock,
            source_name,
            source_identity,
            target_name,
            Some(staging),
            owner_uid,
        )?;

        let mut copy_budget = CopyBudget::default();
        let mut copied_manifest = Vec::new();
        copy_directory_contents(
            &source_directory,
            &staging.directory,
            Path::new(""),
            0,
            owner_uid,
            source_identity.device,
            staging.identity.device,
            &mut copy_budget,
            &mut copied_manifest,
        )?;
        copied_manifest.sort_by(|left, right| left.relative.cmp(&right.relative));

        let mut verification_budget = CopyBudget::default();
        let mut source_manifest = Vec::new();
        snapshot_directory_contents(
            &source_directory,
            Path::new(""),
            0,
            owner_uid,
            source_identity.device,
            &mut verification_budget,
            &mut source_manifest,
        )?;
        source_manifest.sort_by(|left, right| left.relative.cmp(&right.relative));
        if copied_manifest != source_manifest {
            return Err(invalid_migration(
                "legacy app data changed while it was being migrated",
            ));
        }

        write_migration_marker(&staging.directory)?;
        rustix::fs::fsync(&staging.directory).map_err(io::Error::from)?;

        hook(MigrationPoint::BeforeActivation, &context);
        validate_pre_activation_bindings(
            &parent_lock,
            source_name,
            source_identity,
            target_name,
            Some(staging),
            owner_uid,
        )?;
        activate_staging(parent_lock.owner_directory(), &staging.name, target_name)?;
        activated = true;
        rustix::fs::fsync(parent_lock.owner_directory()).map_err(io::Error::from)?;

        hook(MigrationPoint::AfterActivation, &context);
        validate_post_activation_bindings(
            &parent_lock,
            source_name,
            source_identity,
            target_name,
            staging.identity,
            owner_uid,
        )?;
        Ok(())
    })();

    match attempt {
        Ok(()) => Ok(()),
        Err(error) if activated => Err(partial_migration(
            true,
            format!("migration activation completed but read-back failed: {error}"),
        )),
        Err(error) => {
            let Some(staging) = staging.as_ref() else {
                return Err(error);
            };
            match cleanup_staging_if_bound(
                parent_lock.owner_directory(),
                &staging.name,
                staging.identity,
            ) {
                Ok(()) => Err(error),
                Err(cleanup_error) => Err(partial_migration(
                    true,
                    format!(
                        "migration did not activate and private staging could not be proved clean: {cleanup_error}"
                    ),
                )),
            }
        }
    }
}

#[cfg(not(unix))]
fn migrate_legacy_app_data_dir_unsupported(
    source: &Path,
    target: &Path,
) -> Result<(), ServiceError> {
    match std::fs::symlink_metadata(target) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => Err(
            invalid_migration("app data migration target must be a non-symlink directory"),
        ),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            match std::fs::symlink_metadata(source) {
                Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
                Ok(_) => Err(invalid_migration(
                    "legacy app data migration requires descriptor-relative filesystem support",
                )),
                Err(error) => Err(error.into()),
            }
        }
        Err(error) => Err(error.into()),
    }
}

fn migration_names<'a>(
    source: &'a Path,
    target: &'a Path,
) -> Result<(&'a Path, &'a OsStr, &'a OsStr), ServiceError> {
    let source_parent = source
        .parent()
        .ok_or_else(|| invalid_migration("legacy app data source has no parent"))?;
    let target_parent = target
        .parent()
        .ok_or_else(|| invalid_migration("app data migration target has no parent"))?;
    if source_parent != target_parent {
        return Err(invalid_migration(
            "legacy and current app data must share one migration parent",
        ));
    }
    let source_name = single_leaf_name(source, "legacy app data source")?;
    let target_name = single_leaf_name(target, "app data migration target")?;
    if source_name == target_name {
        return Err(invalid_migration(
            "legacy and current app data names must be distinct",
        ));
    }
    Ok((target_parent, source_name, target_name))
}

fn single_leaf_name<'a>(path: &'a Path, label: &str) -> Result<&'a OsStr, ServiceError> {
    let name = path
        .file_name()
        .ok_or_else(|| invalid_migration(&format!("{label} has no leaf name")))?;
    if name.is_empty()
        || !matches!(
            Path::new(name).components().next(),
            Some(Component::Normal(_))
        )
        || Path::new(name).components().count() != 1
    {
        return Err(invalid_migration(&format!("{label} is not a safe leaf")));
    }
    Ok(name)
}

#[cfg(unix)]
fn open_optional_owned_directory(
    parent: &File,
    name: &OsStr,
    owner_uid: u32,
) -> Result<Option<File>, ServiceError> {
    use std::os::unix::fs::MetadataExt;

    use rustix::fs::{openat, Mode, OFlags};

    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let descriptor = match openat(parent, name, flags, Mode::empty()) {
        Ok(descriptor) => descriptor,
        Err(rustix::io::Errno::NOENT) => return Ok(None),
        Err(rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR) => {
            return Err(invalid_migration(
                "app data migration paths must be non-symlink directories",
            ))
        }
        Err(error) => return Err(io::Error::from(error).into()),
    };
    let directory = File::from(descriptor);
    let metadata = directory.metadata()?;
    if !metadata.is_dir() || metadata.uid() != owner_uid {
        return Err(invalid_migration(
            "app data migration directory ownership is unsafe",
        ));
    }
    Ok(Some(directory))
}

#[cfg(unix)]
fn validate_pre_activation_bindings(
    parent_lock: &AppMutationLock,
    source_name: &OsStr,
    source_identity: FileIdentity,
    target_name: &OsStr,
    staging: Option<&StagingState>,
    owner_uid: u32,
) -> Result<(), ServiceError> {
    parent_lock.validate_owner_path_binding()?;
    let source =
        open_optional_owned_directory(parent_lock.owner_directory(), source_name, owner_uid)?
            .ok_or_else(|| invalid_migration("legacy app data source changed during migration"))?;
    if file_identity(&source)? != source_identity {
        return Err(invalid_migration(
            "legacy app data source changed during migration",
        ));
    }
    if open_optional_owned_directory(parent_lock.owner_directory(), target_name, owner_uid)?
        .is_some()
    {
        return Err(invalid_migration(
            "app data migration target changed during migration",
        ));
    }
    if let Some(staging) = staging {
        let current =
            open_optional_owned_directory(parent_lock.owner_directory(), &staging.name, owner_uid)?
                .ok_or_else(|| {
                    invalid_migration("app data migration staging changed during migration")
                })?;
        if file_identity(&current)? != staging.identity {
            return Err(invalid_migration(
                "app data migration staging changed during migration",
            ));
        }
    }
    Ok(())
}

#[cfg(unix)]
fn validate_post_activation_bindings(
    parent_lock: &AppMutationLock,
    source_name: &OsStr,
    source_identity: FileIdentity,
    target_name: &OsStr,
    activated_identity: FileIdentity,
    owner_uid: u32,
) -> Result<(), ServiceError> {
    parent_lock.validate_owner_path_binding()?;
    let source =
        open_optional_owned_directory(parent_lock.owner_directory(), source_name, owner_uid)?
            .ok_or_else(|| invalid_migration("legacy app data source changed after activation"))?;
    if file_identity(&source)? != source_identity {
        return Err(invalid_migration(
            "legacy app data source changed after activation",
        ));
    }
    let target =
        open_optional_owned_directory(parent_lock.owner_directory(), target_name, owner_uid)?
            .ok_or_else(|| invalid_migration("migrated app data target is missing"))?;
    if file_identity(&target)? != activated_identity {
        return Err(invalid_migration(
            "migrated app data target changed after activation",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn create_staging_directory(
    parent_lock: &AppMutationLock,
    name: &OsStr,
    owner_uid: u32,
) -> Result<Option<StagingState>, ServiceError> {
    use std::os::unix::fs::MetadataExt;

    use rustix::fs::{fchmod, mkdirat, openat, Mode, OFlags};

    let private_mode = Mode::from_bits_truncate(0o700);
    match mkdirat(parent_lock.owner_directory(), name, private_mode) {
        Ok(()) => {}
        Err(rustix::io::Errno::EXIST) => return Ok(None),
        Err(error) => return Err(io::Error::from(error).into()),
    }
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let descriptor =
        openat(parent_lock.owner_directory(), name, flags, Mode::empty()).map_err(|error| {
            partial_migration(
                true,
                format!(
                    "private migration staging was created but could not be anchored: {}",
                    io::Error::from(error)
                ),
            )
        })?;
    let directory = File::from(descriptor);
    let metadata = directory.metadata().map_err(|error| {
        partial_migration(
            true,
            format!("private migration staging identity could not be read: {error}"),
        )
    })?;
    if !metadata.is_dir() || metadata.uid() != owner_uid {
        return Err(partial_migration(
            true,
            "private migration staging ownership could not be verified".to_string(),
        ));
    }
    let identity = identity_from_metadata(&metadata);
    let state = StagingState {
        name: name.to_owned(),
        identity,
        directory,
    };
    let finalize = (|| -> Result<(), ServiceError> {
        fchmod(&state.directory, private_mode).map_err(io::Error::from)?;
        rustix::fs::fsync(parent_lock.owner_directory()).map_err(io::Error::from)?;
        Ok(())
    })();
    match finalize {
        Ok(()) => Ok(Some(state)),
        Err(error) => match cleanup_staging_if_bound(
            parent_lock.owner_directory(),
            &state.name,
            state.identity,
        ) {
            Ok(()) => Err(error),
            Err(cleanup_error) => Err(partial_migration(
                true,
                format!(
                    "private migration staging initialization failed and cleanup could not be proved: {cleanup_error}"
                ),
            )),
        },
    }
}

#[cfg(unix)]
fn staging_name(target_name: &OsStr, attempt: u32) -> OsString {
    let sequence = STAGING_SEQUENCE.fetch_add(1, Ordering::Relaxed);
    let target = target_name.to_string_lossy();
    OsString::from(format!(
        ".{target}.migration.{}.{}.{}.{}",
        std::process::id(),
        now_unix_millis(),
        sequence,
        attempt
    ))
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn copy_directory_contents(
    source: &File,
    destination: &File,
    relative: &Path,
    depth: usize,
    owner_uid: u32,
    source_device: u64,
    destination_device: u64,
    budget: &mut CopyBudget,
    manifest: &mut Vec<ManifestEntry>,
) -> Result<(), ServiceError> {
    use rustix::fs::{fchmod, fsync, mkdirat, openat, statat, AtFlags, FileType, Mode, OFlags};

    if depth > MIGRATION_DEPTH_LIMIT {
        return Err(invalid_migration(
            "legacy app data exceeds the migration depth limit",
        ));
    }
    let directory_mode = Mode::from_bits_truncate(0o700);
    let file_mode = Mode::from_bits_truncate(0o600);
    for name in directory_entry_names(source)? {
        consume_entry_budget(budget)?;
        let child_relative = relative.join(&name);
        let stat = statat(source, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
        match FileType::from_raw_mode(stat.st_mode) {
            FileType::Directory => {
                let source_child =
                    open_verified_directory(source, &name, &stat, owner_uid, source_device)?;
                mkdirat(destination, &name, directory_mode).map_err(io::Error::from)?;
                let destination_child = open_verified_directory(
                    destination,
                    &name,
                    &stat_for_created(destination, &name)?,
                    owner_uid,
                    destination_device,
                )?;
                fchmod(&destination_child, directory_mode).map_err(io::Error::from)?;
                manifest.push(ManifestEntry {
                    relative: child_relative.clone(),
                    kind: ManifestKind::Directory,
                });
                copy_directory_contents(
                    &source_child,
                    &destination_child,
                    &child_relative,
                    depth + 1,
                    owner_uid,
                    source_device,
                    destination_device,
                    budget,
                    manifest,
                )?;
                fsync(&destination_child).map_err(io::Error::from)?;
            }
            FileType::RegularFile => {
                let mut source_file = open_verified_regular_file(source, &name, &stat, owner_uid)?;
                let source_metadata = source_file.metadata()?;
                let source_stamp = source_file_stamp(&source_metadata)?;
                consume_file_budget(budget, source_metadata.len())?;
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
                let (copied, digest) =
                    copy_and_hash_bounded(&mut source_file, &mut destination_file)?;
                if copied != source_metadata.len()
                    || source_file_stamp(&source_file.metadata()?)? != source_stamp
                {
                    return Err(invalid_migration(
                        "legacy app data file changed while it was being copied",
                    ));
                }
                destination_file.flush()?;
                destination_file.sync_all()?;
                manifest.push(ManifestEntry {
                    relative: child_relative,
                    kind: ManifestKind::File {
                        size: copied,
                        digest,
                    },
                });
            }
            FileType::Symlink => {
                return Err(invalid_migration(
                    "legacy app data migration refuses symlinked content",
                ))
            }
            _ => {
                return Err(invalid_migration(
                    "legacy app data migration refuses special files",
                ))
            }
        }
    }
    fsync(destination).map_err(io::Error::from)?;
    Ok(())
}

#[cfg(unix)]
#[allow(clippy::too_many_arguments)]
fn snapshot_directory_contents(
    source: &File,
    relative: &Path,
    depth: usize,
    owner_uid: u32,
    source_device: u64,
    budget: &mut CopyBudget,
    manifest: &mut Vec<ManifestEntry>,
) -> Result<(), ServiceError> {
    use rustix::fs::{statat, AtFlags, FileType};

    if depth > MIGRATION_DEPTH_LIMIT {
        return Err(invalid_migration(
            "legacy app data exceeds the migration depth limit",
        ));
    }
    for name in directory_entry_names(source)? {
        consume_entry_budget(budget)?;
        let child_relative = relative.join(&name);
        let stat = statat(source, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
        match FileType::from_raw_mode(stat.st_mode) {
            FileType::Directory => {
                let child =
                    open_verified_directory(source, &name, &stat, owner_uid, source_device)?;
                manifest.push(ManifestEntry {
                    relative: child_relative.clone(),
                    kind: ManifestKind::Directory,
                });
                snapshot_directory_contents(
                    &child,
                    &child_relative,
                    depth + 1,
                    owner_uid,
                    source_device,
                    budget,
                    manifest,
                )?;
            }
            FileType::RegularFile => {
                let mut file = open_verified_regular_file(source, &name, &stat, owner_uid)?;
                let metadata = file.metadata()?;
                let stamp = source_file_stamp(&metadata)?;
                consume_file_budget(budget, metadata.len())?;
                let (size, digest) = hash_bounded(&mut file)?;
                if size != metadata.len() || source_file_stamp(&file.metadata()?)? != stamp {
                    return Err(invalid_migration(
                        "legacy app data file changed during verification",
                    ));
                }
                manifest.push(ManifestEntry {
                    relative: child_relative,
                    kind: ManifestKind::File { size, digest },
                });
            }
            FileType::Symlink => {
                return Err(invalid_migration(
                    "legacy app data migration refuses symlinked content",
                ))
            }
            _ => {
                return Err(invalid_migration(
                    "legacy app data migration refuses special files",
                ))
            }
        }
    }
    Ok(())
}

#[cfg(unix)]
fn directory_entry_names(directory: &File) -> Result<Vec<OsString>, ServiceError> {
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
fn open_verified_directory(
    parent: &File,
    name: &OsStr,
    stat: &rustix::fs::Stat,
    owner_uid: u32,
    expected_device: u64,
) -> Result<File, ServiceError> {
    use std::os::unix::fs::MetadataExt;

    use rustix::fs::{openat, Mode, OFlags};

    let descriptor = openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| map_unsafe_migration_errno(error, "directory"))?;
    let directory = File::from(descriptor);
    let metadata = directory.metadata()?;
    if !metadata.is_dir()
        || metadata.uid() != owner_uid
        || metadata.dev() != expected_device
        || metadata.dev() != stat.st_dev as u64
        || metadata.ino() != stat.st_ino
    {
        return Err(invalid_migration(
            "legacy app data migration refuses cross-device directories or directory races",
        ));
    }
    Ok(directory)
}

#[cfg(unix)]
fn open_verified_regular_file(
    parent: &File,
    name: &OsStr,
    stat: &rustix::fs::Stat,
    owner_uid: u32,
) -> Result<File, ServiceError> {
    use std::os::unix::fs::MetadataExt;

    use rustix::fs::{openat, Mode, OFlags};

    let descriptor = openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map_err(|error| map_unsafe_migration_errno(error, "file"))?;
    let file = File::from(descriptor);
    let metadata = file.metadata()?;
    if !metadata.is_file()
        || metadata.uid() != owner_uid
        || metadata.nlink() != 1
        || metadata.dev() != stat.st_dev as u64
        || metadata.ino() != stat.st_ino
    {
        return Err(invalid_migration(
            "legacy app data file ownership or link count is unsafe",
        ));
    }
    Ok(file)
}

#[cfg(unix)]
fn stat_for_created(parent: &File, name: &OsStr) -> Result<rustix::fs::Stat, ServiceError> {
    rustix::fs::statat(parent, name, rustix::fs::AtFlags::SYMLINK_NOFOLLOW)
        .map_err(io::Error::from)
        .map_err(Into::into)
}

#[cfg(unix)]
fn map_unsafe_migration_errno(error: rustix::io::Errno, kind: &str) -> ServiceError {
    if matches!(error, rustix::io::Errno::LOOP | rustix::io::Errno::NOTDIR) {
        invalid_migration(&format!("legacy app data {kind} changed during migration"))
    } else {
        io::Error::from(error).into()
    }
}

#[cfg(unix)]
fn source_file_stamp(
    metadata: &std::fs::Metadata,
) -> Result<(FileIdentity, u64, i64, i64), ServiceError> {
    use std::os::unix::fs::MetadataExt;

    if !metadata.is_file() || metadata.nlink() != 1 {
        return Err(invalid_migration(
            "legacy app data file ownership or link count is unsafe",
        ));
    }
    Ok((
        FileIdentity {
            device: metadata.dev(),
            inode: metadata.ino(),
            uid: metadata.uid(),
        },
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
    ))
}

#[cfg(unix)]
fn consume_entry_budget(budget: &mut CopyBudget) -> Result<(), ServiceError> {
    budget.entries = budget.entries.saturating_add(1);
    if budget.entries > MIGRATION_ENTRY_LIMIT {
        return Err(invalid_migration(
            "legacy app data exceeds the migration entry limit",
        ));
    }
    Ok(())
}

#[cfg(unix)]
fn consume_file_budget(budget: &mut CopyBudget, size: u64) -> Result<(), ServiceError> {
    if size > MIGRATION_FILE_BYTE_LIMIT {
        return Err(invalid_migration(
            "legacy app data contains a file beyond the migration byte limit",
        ));
    }
    budget.total_bytes = budget
        .total_bytes
        .checked_add(size)
        .filter(|total| *total <= MIGRATION_TOTAL_BYTE_LIMIT)
        .ok_or_else(|| invalid_migration("legacy app data exceeds the migration byte limit"))?;
    Ok(())
}

#[cfg(unix)]
fn copy_and_hash_bounded(
    source: &mut File,
    destination: &mut File,
) -> Result<(u64, [u8; 32]), ServiceError> {
    let mut hasher = Sha256::new();
    let mut copied = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        copied = copied
            .checked_add(read as u64)
            .filter(|size| *size <= MIGRATION_FILE_BYTE_LIMIT)
            .ok_or_else(|| {
                invalid_migration("legacy app data contains a file beyond the migration byte limit")
            })?;
        destination.write_all(&buffer[..read])?;
        hasher.update(&buffer[..read]);
    }
    Ok((copied, hasher.finalize().into()))
}

#[cfg(unix)]
fn hash_bounded(source: &mut File) -> Result<(u64, [u8; 32]), ServiceError> {
    let mut hasher = Sha256::new();
    let mut read_total = 0u64;
    let mut buffer = [0u8; 64 * 1024];
    loop {
        let read = source.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        read_total = read_total
            .checked_add(read as u64)
            .filter(|size| *size <= MIGRATION_FILE_BYTE_LIMIT)
            .ok_or_else(|| {
                invalid_migration("legacy app data contains a file beyond the migration byte limit")
            })?;
        hasher.update(&buffer[..read]);
    }
    Ok((read_total, hasher.finalize().into()))
}

#[cfg(unix)]
fn write_migration_marker(staging: &File) -> Result<(), ServiceError> {
    use rustix::fs::{fchmod, openat, Mode, OFlags};

    let marker = serde_json::json!({
        "version": 1,
        "migration": "v2.90-agent-copilot-app-data",
        "source_bundle_id": LEGACY_BUNDLE_ID,
        "target_bundle_id": DEFAULT_BUNDLE_ID,
        "migrated_at_unix_ms": now_unix_millis(),
    });
    let bytes = serde_json::to_vec_pretty(&marker)?;
    let mode = Mode::from_bits_truncate(0o600);
    let descriptor = openat(
        staging,
        MIGRATION_MARKER_NAME,
        OFlags::WRONLY | OFlags::CREATE | OFlags::EXCL | OFlags::NOFOLLOW | OFlags::CLOEXEC,
        mode,
    )
    .map_err(io::Error::from)?;
    let mut file = File::from(descriptor);
    fchmod(&file, mode).map_err(io::Error::from)?;
    file.write_all(&bytes)?;
    file.write_all(b"\n")?;
    file.sync_all()?;
    Ok(())
}

#[cfg(unix)]
fn activate_staging(parent: &File, staging: &OsStr, target: &OsStr) -> Result<(), ServiceError> {
    #[cfg(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "linux",
        target_os = "redox"
    ))]
    {
        use rustix::fs::{renameat_with, RenameFlags};

        match renameat_with(parent, staging, parent, target, RenameFlags::NOREPLACE) {
            Ok(()) => Ok(()),
            Err(rustix::io::Errno::EXIST) => Err(invalid_migration(
                "app data migration target changed during activation",
            )),
            Err(error) => Err(io::Error::from(error).into()),
        }
    }
    #[cfg(not(any(
        target_vendor = "apple",
        target_os = "android",
        target_os = "linux",
        target_os = "redox"
    )))]
    {
        let _ = (parent, staging, target);
        Err(invalid_migration(
            "app data migration requires atomic no-replace rename support",
        ))
    }
}

#[cfg(unix)]
fn cleanup_staging_if_bound(
    parent: &File,
    staging_name: &OsStr,
    expected_identity: FileIdentity,
) -> Result<(), ServiceError> {
    let current = open_optional_owned_directory(parent, staging_name, expected_identity.uid)?
        .ok_or_else(|| {
            invalid_migration("private migration staging is no longer bound to its original inode")
        })?;
    if file_identity(&current)? != expected_identity {
        return Err(invalid_migration(
            "private migration staging is no longer bound to its original inode",
        ));
    }
    remove_directory_contents(&current, expected_identity.device)?;
    rustix::fs::unlinkat(parent, staging_name, rustix::fs::AtFlags::REMOVEDIR)
        .map_err(io::Error::from)?;
    rustix::fs::fsync(parent).map_err(io::Error::from)?;
    Ok(())
}

#[cfg(unix)]
fn remove_directory_contents(directory: &File, expected_device: u64) -> Result<(), ServiceError> {
    use rustix::fs::{openat, statat, unlinkat, AtFlags, FileType, Mode, OFlags};

    for name in directory_entry_names(directory)? {
        let stat = statat(directory, &name, AtFlags::SYMLINK_NOFOLLOW).map_err(io::Error::from)?;
        match FileType::from_raw_mode(stat.st_mode) {
            FileType::Directory => {
                let descriptor = openat(
                    directory,
                    &name,
                    OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|error| map_unsafe_migration_errno(error, "cleanup directory"))?;
                let child = File::from(descriptor);
                let metadata = child.metadata()?;
                #[cfg(unix)]
                {
                    use std::os::unix::fs::MetadataExt;
                    if metadata.dev() != expected_device
                        || metadata.dev() != stat.st_dev as u64
                        || metadata.ino() != stat.st_ino
                    {
                        return Err(invalid_migration(
                            "private migration staging changed or crossed a filesystem during cleanup",
                        ));
                    }
                }
                remove_directory_contents(&child, expected_device)?;
                unlinkat(directory, &name, AtFlags::REMOVEDIR).map_err(io::Error::from)?;
            }
            _ => {
                unlinkat(directory, &name, AtFlags::empty()).map_err(io::Error::from)?;
            }
        }
    }
    rustix::fs::fsync(directory).map_err(io::Error::from)?;
    Ok(())
}

#[cfg(unix)]
fn file_identity(file: &File) -> Result<FileIdentity, ServiceError> {
    let metadata = file.metadata()?;
    Ok(identity_from_metadata(&metadata))
}

#[cfg(unix)]
fn identity_from_metadata(metadata: &std::fs::Metadata) -> FileIdentity {
    use std::os::unix::fs::MetadataExt;

    FileIdentity {
        device: metadata.dev(),
        inode: metadata.ino(),
        uid: metadata.uid(),
    }
}

fn invalid_migration(detail: &str) -> ServiceError {
    ServiceError::InvalidRequest(detail.to_string())
}

fn partial_migration(cleanup_required: bool, detail: String) -> ServiceError {
    ServiceError::Command(CommandError::PartialEffect {
        operation: "legacy_app_data_migration".to_string(),
        state: "outcome_unknown",
        cleanup_required,
        detail,
    })
}

fn now_unix_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
