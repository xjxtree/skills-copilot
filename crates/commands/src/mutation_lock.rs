use std::{
    fs::{self, File},
    io,
    path::Path,
};

use fs4::FileExt;

use crate::CommandError;

/// Cross-sidecar owner lock for every app-coordinated mutation.
///
/// The owner directory must already exist. Acquiring this lock never creates a
/// lock file, target directory, or other stale-preview artifact.
pub struct AppMutationLock {
    file: File,
}

impl Drop for AppMutationLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

pub fn lock_app_mutations(app_data_dir: &Path) -> Result<AppMutationLock, CommandError> {
    let file = open_existing_app_mutation_owner(app_data_dir)?;
    file.lock_exclusive()?;
    Ok(AppMutationLock { file })
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
    Ok(AppMutationLock { file })
}

pub(crate) fn app_mutation_owner_is_missing(app_data_dir: &Path) -> Result<bool, CommandError> {
    match fs::symlink_metadata(app_data_dir) {
        Ok(_) => {
            validate_existing_owner(app_data_dir)?;
            Ok(false)
        }
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(true),
        Err(error) => Err(error.into()),
    }
}

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
    use rustix::fs::{open, Mode, OFlags};

    validate_existing_owner(path)?;
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file = File::from(open(path, flags, Mode::empty()).map_err(io::Error::from)?);
    if !file.metadata()?.is_dir() {
        return Err(CommandError::UnsafeConfigPath(
            "mutation lock owner is not the app data directory".to_string(),
        ));
    }
    Ok(file)
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
    use rustix::fs::{fchmod, mkdirat, open, openat, Mode, OFlags};

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
    let directory_flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let parent_file = open(parent, directory_flags, Mode::empty()).map_err(io::Error::from)?;
    let private_mode = Mode::from_bits_truncate(0o700);
    let created = match mkdirat(&parent_file, name, private_mode).map_err(io::Error::from) {
        Ok(()) => true,
        Err(error) if error.kind() == io::ErrorKind::AlreadyExists => false,
        Err(error) => return Err(error.into()),
    };
    let owner =
        openat(&parent_file, name, directory_flags, Mode::empty()).map_err(io::Error::from)?;
    if created {
        fchmod(&owner, private_mode).map_err(io::Error::from)?;
    }
    let file = File::from(owner);
    if !file.metadata()?.is_dir() {
        return Err(CommandError::UnsafeConfigPath(
            "mutation lock owner is not the app data directory".to_string(),
        ));
    }
    Ok(file)
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
}
