use std::{fs::File, path::Path};

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
    let canonical_owner = app_data_dir.canonicalize()?;
    if !canonical_owner.is_dir() {
        return Err(CommandError::UnsafeConfigPath(
            "mutation lock owner is not the app data directory".to_string(),
        ));
    }
    let file = File::open(&canonical_owner)?;
    file.lock_exclusive()?;
    Ok(AppMutationLock { file })
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
}
