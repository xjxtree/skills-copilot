use std::{
    fs::{self, File},
    io,
    path::{Component, Path, PathBuf},
};

use fs4::FileExt;

use crate::CommandError;

/// Cross-sidecar owner lock for every app-coordinated mutation.
///
/// The owner directory must already exist. Acquiring this lock never creates a
/// lock file, target directory, or other stale-preview artifact.
pub struct AppMutationLock {
    file: File,
    owner_path: PathBuf,
}

impl AppMutationLock {
    /// Clone the already-opened no-follow app-data owner descriptor.
    ///
    /// Callers use this capability for descriptor-relative child I/O. The
    /// clone names the same directory inode even if its display path is later
    /// renamed or replaced.
    pub fn try_clone_owner_directory(&self) -> Result<File, CommandError> {
        self.file.try_clone().map_err(Into::into)
    }

    pub fn owner_directory(&self) -> &File {
        &self.file
    }

    #[cfg(not(unix))]
    pub(crate) fn owner_path(&self) -> &Path {
        &self.owner_path
    }

    pub(crate) fn owner_fs(&self) -> crate::app_data_owner_fs::AppDataOwnerFs<'_> {
        crate::app_data_owner_fs::AppDataOwnerFs::new(self)
    }

    pub(crate) fn validate_owner_path_binding(&self) -> Result<(), CommandError> {
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
        let _ = self.file.unlock();
    }
}

pub fn lock_app_mutations(app_data_dir: &Path) -> Result<AppMutationLock, CommandError> {
    let file = open_existing_app_mutation_owner(app_data_dir)?;
    file.lock_exclusive()?;
    Ok(AppMutationLock {
        file,
        owner_path: app_data_dir.to_path_buf(),
    })
}

/// Create and lock the private app-data owner for an already-confirmed action.
///
/// Creation is deliberately limited to one missing leaf below an existing
/// non-symlink parent. Callers must finish typed confirmation and non-creating
/// preflight validation before invoking this function, so every rejection
/// known before coordination bootstrap remains zero-write.
pub(crate) fn lock_or_create_app_mutations(
    app_data_dir: &Path,
) -> Result<AppMutationLock, CommandError> {
    let file = open_or_create_app_mutation_owner(app_data_dir)?;
    file.lock_exclusive()?;
    Ok(AppMutationLock {
        file,
        owner_path: app_data_dir.to_path_buf(),
    })
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
    let file = open_app_mutation_directory_tree(app_data_dir, true)?;
    file.lock_exclusive()?;
    Ok(AppMutationLock {
        file,
        owner_path: app_data_dir.to_path_buf(),
    })
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
    open_app_mutation_directory_tree(path, false)
}

#[cfg(not(unix))]
fn open_existing_app_mutation_owner(path: &Path) -> Result<File, CommandError> {
    validate_existing_owner(path)?;
    let canonical_owner = path.canonicalize()?;
    let file = File::open(canonical_owner)?;
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
    use rustix::fs::{fchmod, mkdirat, Mode};

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
    let owner = open_app_mutation_child(&parent_file, name).map_err(|error| {
        if is_unsafe_directory_error(&error) {
            unsafe_owner()
        } else {
            error.into()
        }
    })?;
    if created {
        fchmod(&owner, private_mode).map_err(io::Error::from)?;
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
) -> Result<File, CommandError> {
    use rustix::fs::{fchmod, mkdirat, open, openat, Mode, OFlags};
    use rustix::io::Errno;

    let path = normalize_trusted_root_alias(path)?;
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut current = if path.is_absolute() {
        open("/", flags, Mode::empty()).map_err(io::Error::from)?
    } else {
        open(".", flags, Mode::empty()).map_err(io::Error::from)?
    };
    let mut saw_name = false;
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
                match mkdirat(&current, name, mode) {
                    Ok(()) | Err(Errno::EXIST) => {}
                    Err(error) => return Err(io::Error::from(error).into()),
                }
                let next = openat(&current, name, flags, Mode::empty()).map_err(|error| {
                    let error = io::Error::from(error);
                    if is_unsafe_directory_error(&error) {
                        unsafe_owner()
                    } else {
                        error.into()
                    }
                })?;
                fchmod(&next, mode).map_err(io::Error::from)?;
                current = next;
            }
            Err(error) if error == Errno::LOOP || error == Errno::NOTDIR => {
                return Err(unsafe_owner())
            }
            Err(error) => return Err(io::Error::from(error).into()),
        }
    }
    if !saw_name {
        return Err(unsafe_owner());
    }
    let file = File::from(current);
    if !file.metadata()?.is_dir() {
        return Err(unsafe_owner());
    }
    Ok(file)
}

#[cfg(not(unix))]
fn open_app_mutation_directory_tree(
    path: &Path,
    create_missing: bool,
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
                        fs::create_dir(&current)?;
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
    let file = File::open(canonical_owner)?;
    if !file.metadata()?.is_dir() {
        return Err(unsafe_owner());
    }
    Ok(file)
}

#[cfg(unix)]
fn open_app_mutation_parent(path: &Path) -> Result<(File, &std::ffi::OsStr), CommandError> {
    let parent = path.parent().ok_or_else(unsafe_owner)?;
    let name = path.file_name().ok_or_else(unsafe_owner)?;
    let file = open_app_mutation_directory_tree(parent, false)?;
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
fn normalize_trusted_root_alias(path: &Path) -> Result<PathBuf, CommandError> {
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
    fs::create_dir(path)?;
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
    fn cloned_owner_descriptor_remains_on_the_locked_inode_after_path_replacement() {
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
        let capability = guard
            .try_clone_owner_directory()
            .expect("clone owner descriptor");
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
}
