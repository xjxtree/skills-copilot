use super::*;

impl SkillManagerCommandOutput {
    pub(super) fn without_machine_stdout(mut self) -> Self {
        self.stdout.clear();
        self
    }
}

#[cfg(test)]
type PostCreateHook = Box<dyn FnOnce(&Path)>;

#[cfg(test)]
thread_local! {
    static POST_CREATE_HOOK: std::cell::RefCell<Option<PostCreateHook>> =
        const { std::cell::RefCell::new(None) };
    static PRE_UNLINK_HOOK: std::cell::RefCell<Option<PreUnlinkHook>> =
        const { std::cell::RefCell::new(None) };
    static PRE_RESTORE_HOOK: std::cell::RefCell<Option<Box<dyn FnOnce()>>> =
        const { std::cell::RefCell::new(None) };
    static SYNC_FAILURE_HOOK: std::cell::RefCell<Option<&'static str>> =
        const { std::cell::RefCell::new(None) };
}

#[cfg(test)]
struct PreUnlinkHook {
    expected_path: PathBuf,
    action: Box<dyn FnOnce(&Path)>,
}

#[cfg(test)]
pub(super) fn install_machine_capture_post_create_test_hook(action: impl FnOnce(&Path) + 'static) {
    POST_CREATE_HOOK.with(|hook| {
        let mut hook = hook.borrow_mut();
        assert!(hook.is_none(), "machine capture test hook already set");
        *hook = Some(Box::new(action));
    });
}

#[cfg(test)]
pub(super) fn install_machine_capture_pre_unlink_test_hook(
    expected_path: PathBuf,
    action: impl FnOnce(&Path) + 'static,
) {
    PRE_UNLINK_HOOK.with(|hook| {
        let mut hook = hook.borrow_mut();
        assert!(
            hook.is_none(),
            "machine capture cleanup test hook already set"
        );
        *hook = Some(PreUnlinkHook {
            expected_path,
            action: Box::new(action),
        });
    });
}

#[cfg(test)]
pub(super) fn install_machine_capture_pre_restore_test_hook(action: impl FnOnce() + 'static) {
    PRE_RESTORE_HOOK.with(|hook| {
        let mut hook = hook.borrow_mut();
        assert!(
            hook.is_none(),
            "machine capture restore test hook already set"
        );
        *hook = Some(Box::new(action));
    });
}

#[cfg(test)]
pub(super) fn inject_machine_capture_sync_failure_for_test(stage: &'static str) {
    SYNC_FAILURE_HOOK.with(|hook| {
        let mut hook = hook.borrow_mut();
        assert!(hook.is_none(), "machine capture sync hook already set");
        *hook = Some(stage);
    });
}

#[cfg(test)]
fn run_post_create_hook(path: &Path) {
    let action = POST_CREATE_HOOK.with(|hook| hook.borrow_mut().take());
    if let Some(action) = action {
        action(path);
    }
}

#[cfg(not(test))]
fn run_post_create_hook(_path: &Path) {}

#[cfg(test)]
fn run_pre_unlink_hook(path: &Path) {
    let action = PRE_UNLINK_HOOK.with(|hook| {
        let mut hook = hook.borrow_mut();
        if hook.as_ref().is_some_and(|hook| hook.expected_path == path) {
            hook.take().map(|hook| hook.action)
        } else {
            None
        }
    });
    if let Some(action) = action {
        action(path);
    }
}

#[cfg(not(test))]
fn run_pre_unlink_hook(_path: &Path) {}

#[cfg(test)]
fn run_pre_restore_hook() {
    let action = PRE_RESTORE_HOOK.with(|hook| hook.borrow_mut().take());
    if let Some(action) = action {
        action();
    }
}

#[cfg(not(test))]
fn run_pre_restore_hook() {}

pub(super) struct MachineStdoutCapture {
    pub(super) path: PathBuf,
    pub(super) file: File,
    #[cfg(unix)]
    parent: File,
    #[cfg(unix)]
    name: std::ffi::OsString,
    cleaned: bool,
}

#[cfg(unix)]
#[derive(Clone, Debug, Eq, PartialEq)]
struct CaptureIdentity {
    dev: u64,
    ino: u64,
    uid: u32,
    nlink: u64,
    mode: u32,
    size: u64,
    mtime: i64,
    mtime_nsec: i64,
}

#[cfg(unix)]
impl CaptureIdentity {
    fn from_file(file: &File) -> Result<Self, CommandError> {
        use std::os::unix::fs::MetadataExt;

        let metadata = file.metadata()?;
        if !metadata.is_file() {
            return Err(CommandError::UnsafeConfigPath(
                "manager output capture must remain a regular file".to_string(),
            ));
        }
        Ok(Self {
            dev: metadata.dev(),
            ino: metadata.ino(),
            uid: metadata.uid(),
            nlink: metadata.nlink(),
            mode: metadata.mode(),
            size: metadata.size(),
            mtime: metadata.mtime(),
            mtime_nsec: metadata.mtime_nsec(),
        })
    }

    fn is_private(&self) -> bool {
        self.uid == rustix::process::geteuid().as_raw()
            && self.nlink == 1
            && self.mode & 0o777 == 0o600
    }
}

impl MachineStdoutCapture {
    pub(super) fn create() -> Result<Self, CommandError> {
        let temp_dir = env::temp_dir();
        #[cfg(unix)]
        let parent = open_existing_manager_working_directory(&temp_dir)?;
        for _ in 0..32 {
            let name = std::ffi::OsString::from(format!(
                "agent-copilot-skill-manager-{}.json",
                random_token()?
            ));
            let path = temp_dir.join(&name);
            #[cfg(unix)]
            let opened = {
                use rustix::fs::{openat, Mode, OFlags};

                openat(
                    &parent,
                    &name,
                    OFlags::RDWR
                        | OFlags::CREATE
                        | OFlags::EXCL
                        | OFlags::NOFOLLOW
                        | OFlags::CLOEXEC,
                    Mode::from_bits_truncate(0o600),
                )
                .map(File::from)
                .map_err(std::io::Error::from)
            };
            #[cfg(not(unix))]
            let opened = fs::OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&path);
            match opened {
                Ok(file) => {
                    #[cfg(unix)]
                    {
                        use rustix::fs::{fchmod, Mode};

                        run_post_create_hook(&path);
                        if let Err(error) = fchmod(&file, Mode::from_bits_truncate(0o600)) {
                            return Err(create_error(
                                &parent,
                                &name,
                                &path,
                                &file,
                                std::io::Error::from(error).into(),
                            ));
                        }
                        let expected = match CaptureIdentity::from_file(&file) {
                            Ok(identity) => identity,
                            Err(error) => {
                                return Err(create_error(&parent, &name, &path, &file, error))
                            }
                        };
                        let current = match open_capture_entry(&parent, &name)
                            .and_then(|current| CaptureIdentity::from_file(&current))
                        {
                            Ok(identity) => identity,
                            Err(error) => {
                                return Err(create_error(&parent, &name, &path, &file, error))
                            }
                        };
                        if !expected.is_private() || !current.is_private() || expected != current {
                            return Err(create_error(
                                &parent,
                                &name,
                                &path,
                                &file,
                                CommandError::UnsafeConfigPath(
                                    "manager output capture lost its private regular-file binding"
                                        .to_string(),
                                ),
                            ));
                        }
                        return Ok(Self {
                            path,
                            file,
                            parent,
                            name,
                            cleaned: false,
                        });
                    }
                    #[cfg(not(unix))]
                    return Ok(Self {
                        path,
                        file,
                        cleaned: false,
                    });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(CommandError::SkillManagerCommandFailed(
            "could not allocate a private installed-inventory capture".to_string(),
        ))
    }

    pub(super) fn child_stdout(&self) -> Result<Stdio, CommandError> {
        Ok(Stdio::from(self.file.try_clone()?))
    }

    pub(super) fn read(&mut self) -> Result<Vec<u8>, CommandError> {
        let read_result = (|| {
            #[cfg(unix)]
            let before = CaptureIdentity::from_file(&self.file)?;
            #[cfg(unix)]
            if !before.is_private() {
                return Err(CommandError::UnsafeConfigPath(
                    "manager output capture lost its private single-link metadata before read"
                        .to_string(),
                ));
            }
            if self.file.metadata()?.len() > MAX_MACHINE_OUTPUT_BYTES as u64 {
                return Err(CommandError::SkillManagerCommandFailed(
                    "manager machine output exceeded the safe capture limit".to_string(),
                ));
            }
            self.file.seek(SeekFrom::Start(0))?;
            let mut output = Vec::new();
            self.file
                .by_ref()
                .take((MAX_MACHINE_OUTPUT_BYTES + 1) as u64)
                .read_to_end(&mut output)?;
            if output.len() > MAX_MACHINE_OUTPUT_BYTES {
                return Err(CommandError::SkillManagerCommandFailed(
                    "manager machine output exceeded the safe capture limit".to_string(),
                ));
            }
            #[cfg(unix)]
            {
                let after = CaptureIdentity::from_file(&self.file)?;
                if !after.is_private() || after != before {
                    return Err(CommandError::UnsafeConfigPath(
                        "manager output capture metadata changed while it was being read"
                            .to_string(),
                    ));
                }
            }
            Ok(output)
        })();
        let cleanup_result = self.cleanup();
        match (read_result, cleanup_result) {
            (Ok(output), Ok(())) => Ok(output),
            (Err(error), Ok(())) => Err(error),
            (_, Err(cleanup_error)) => Err(cleanup_error),
        }
    }

    pub(super) fn finalize(&mut self) -> Result<(), CommandError> {
        self.cleanup()
    }

    fn cleanup(&mut self) -> Result<(), CommandError> {
        if self.cleaned {
            return Ok(());
        }
        #[cfg(unix)]
        quarantine_and_unlink(&self.parent, &self.name, &self.path, &self.file)?;
        #[cfg(not(unix))]
        fs::remove_file(&self.path)?;
        self.cleaned = true;
        Ok(())
    }
}

impl Drop for MachineStdoutCapture {
    fn drop(&mut self) {
        let _ = self.cleanup();
    }
}

#[cfg(unix)]
fn create_error(
    parent: &File,
    name: &std::ffi::OsStr,
    path: &Path,
    file: &File,
    original: CommandError,
) -> CommandError {
    match quarantine_and_unlink(parent, name, path, file) {
        Ok(()) => original,
        Err(cleanup_error) => cleanup_error,
    }
}

#[cfg(unix)]
fn quarantine_and_unlink(
    parent: &File,
    name: &std::ffi::OsStr,
    path: &Path,
    file: &File,
) -> Result<(), CommandError> {
    use rustix::fs::{renameat_with, unlinkat, AtFlags, RenameFlags};

    let expected = CaptureIdentity::from_file(file)?;
    if !expected.is_private() {
        return Err(cleanup_partial(
            "applied_unverified",
            true,
            "manager output capture no longer has current-user, single-link, mode-0600 metadata",
        ));
    }
    let quarantine = (0..32)
        .find_map(|_| {
            let quarantine = quarantine_name().ok()?;
            match renameat_with(parent, name, parent, &quarantine, RenameFlags::NOREPLACE) {
                Ok(()) => Some(Ok(quarantine)),
                Err(rustix::io::Errno::EXIST) => None,
                Err(error) => Some(Err(error)),
            }
        })
        .transpose()
        .map_err(std::io::Error::from)?
        .ok_or_else(|| {
            CommandError::UnsafeConfigPath(
                "manager output capture could not allocate a private cleanup quarantine"
                    .to_string(),
            )
        })?;
    sync_parent(parent, "quarantine").map_err(|error| {
        cleanup_partial(
            "outcome_unknown",
            true,
            &format!("cleanup quarantine durability could not be verified: {error}"),
        )
    })?;

    let current = match open_capture_entry(parent, &quarantine)
        .and_then(|current| CaptureIdentity::from_file(&current))
    {
        Ok(identity) => identity,
        Err(error) => {
            restore(parent, &quarantine, name)?;
            return Err(cleanup_partial(
                "applied_unverified",
                false,
                &format!(
                    "cleanup quarantine could not be verified after durable restoration: {}",
                    error
                ),
            ));
        }
    };
    if !current.is_private() || current != expected {
        restore(parent, &quarantine, name)?;
        return Err(cleanup_partial(
            "applied_unverified",
            false,
            "cleanup found a replacement entry and durably restored it without deletion",
        ));
    }

    run_pre_unlink_hook(path);
    let descriptor_now = CaptureIdentity::from_file(file)?;
    let current = match open_capture_entry(parent, &quarantine)
        .and_then(|current| CaptureIdentity::from_file(&current))
    {
        Ok(identity) => identity,
        Err(error) => {
            restore(parent, &quarantine, name)?;
            return Err(cleanup_partial(
                "applied_unverified",
                false,
                &format!(
                    "cleanup quarantine changed after validation and was durably restored: {}",
                    error
                ),
            ));
        }
    };
    if !descriptor_now.is_private() || descriptor_now != expected {
        restore(parent, &quarantine, name)?;
        return Err(cleanup_partial(
            "applied_unverified",
            false,
            "manager output capture descriptor metadata drifted during cleanup and was durably restored",
        ));
    }
    if !current.is_private() || current != expected {
        restore(parent, &quarantine, name)?;
        return Err(cleanup_partial(
            "applied_unverified",
            false,
            "cleanup quarantine changed after validation and was durably restored",
        ));
    }
    unlinkat(parent, &quarantine, AtFlags::empty()).map_err(|error| {
        cleanup_partial(
            "outcome_unknown",
            true,
            &format!(
                "cleanup quarantine could not be removed: {}",
                std::io::Error::from(error)
            ),
        )
    })?;
    sync_parent(parent, "unlink").map_err(|error| {
        cleanup_partial(
            "outcome_unknown",
            true,
            &format!("cleanup removal durability could not be verified: {error}"),
        )
    })
}

#[cfg(unix)]
fn open_capture_entry(parent: &File, name: &std::ffi::OsStr) -> Result<File, CommandError> {
    use rustix::fs::{openat, Mode, OFlags};

    openat(
        parent,
        name,
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
        Mode::empty(),
    )
    .map(File::from)
    .map_err(std::io::Error::from)
    .map_err(Into::into)
}

#[cfg(unix)]
fn restore(
    parent: &File,
    quarantine: &std::ffi::OsStr,
    original: &std::ffi::OsStr,
) -> Result<(), CommandError> {
    use rustix::fs::{renameat_with, RenameFlags};

    run_pre_restore_hook();
    match renameat_with(
        parent,
        quarantine,
        parent,
        original,
        RenameFlags::NOREPLACE,
    ) {
        Ok(()) => sync_parent(parent, "restore").map_err(|error| {
            cleanup_partial(
                "outcome_unknown",
                true,
                &format!("cleanup restoration durability could not be verified: {error}"),
            )
        }),
        Err(rustix::io::Errno::EXIST) => Err(cleanup_partial(
            "outcome_unknown",
            true,
            "cleanup restoration found an original-path replacement; both it and the quarantine were preserved",
        )),
        Err(error) => Err(cleanup_partial(
            "outcome_unknown",
            true,
            &format!(
                "cleanup quarantine could not be restored and was retained: {}",
                std::io::Error::from(error)
            ),
        )),
    }
}

#[cfg(unix)]
fn quarantine_name() -> Result<std::ffi::OsString, CommandError> {
    Ok(format!(
        ".agent-copilot-skill-manager-quarantine-{}",
        random_token()?
    )
    .into())
}

fn random_token() -> Result<String, CommandError> {
    use std::fmt::Write as _;

    let mut random = [0_u8; 16];
    getrandom::getrandom(&mut random).map_err(|_| {
        CommandError::UnsafeConfigPath(
            "manager output capture could not obtain cleanup randomness".to_string(),
        )
    })?;
    let mut name = String::with_capacity(random.len() * 2);
    for byte in random {
        write!(&mut name, "{byte:02x}").expect("write to string");
    }
    Ok(name)
}

#[cfg(unix)]
fn sync_parent(parent: &File, stage: &'static str) -> Result<(), std::io::Error> {
    #[cfg(not(test))]
    let _ = stage;
    #[cfg(test)]
    {
        let fail = SYNC_FAILURE_HOOK.with(|hook| {
            let mut hook = hook.borrow_mut();
            if hook.as_ref().is_some_and(|expected| *expected == stage) {
                hook.take();
                true
            } else {
                false
            }
        });
        if fail {
            return Err(std::io::Error::other(format!(
                "injected {stage} directory sync failure"
            )));
        }
    }
    rustix::fs::fsync(parent).map_err(std::io::Error::from)
}

fn cleanup_partial(state: &'static str, cleanup_required: bool, detail: &str) -> CommandError {
    CommandError::PartialEffect {
        operation: "skillManager.machineCaptureCleanup".to_string(),
        state,
        cleanup_required,
        detail: detail.to_string(),
    }
}

pub(super) fn open_existing_manager_working_directory(path: &Path) -> Result<File, CommandError> {
    #[cfg(unix)]
    {
        use rustix::fs::{open, openat, Mode, OFlags};

        let normalized = crate::mutation_lock::normalize_trusted_root_alias(path)?;
        if !normalized.is_absolute() {
            return Err(CommandError::UnsafeConfigPath(
                "manager working directory must be an absolute existing directory".to_string(),
            ));
        }
        let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
        let mut current = open("/", flags, Mode::empty()).map_err(std::io::Error::from)?;
        let mut saw_name = false;
        for component in normalized.components() {
            let name = match component {
                std::path::Component::RootDir | std::path::Component::CurDir => continue,
                std::path::Component::Normal(name) => name,
                std::path::Component::ParentDir | std::path::Component::Prefix(_) => {
                    return Err(CommandError::UnsafeConfigPath(
                        "manager working directory contains an unsafe component".to_string(),
                    ))
                }
            };
            saw_name = true;
            current = openat(&current, name, flags, Mode::empty()).map_err(|error| {
                let error = std::io::Error::from(error);
                if matches!(
                    error.raw_os_error(),
                    Some(code)
                        if code == rustix::io::Errno::LOOP.raw_os_error()
                            || code == rustix::io::Errno::NOTDIR.raw_os_error()
                ) {
                    CommandError::UnsafeConfigPath(
                        "manager working directory must not contain symlinks".to_string(),
                    )
                } else if error.kind() == std::io::ErrorKind::NotFound {
                    CommandError::InvalidSkillManagerRequest(
                        "manager working directory must already exist".to_string(),
                    )
                } else {
                    error.into()
                }
            })?;
        }
        if !saw_name {
            return Err(CommandError::UnsafeConfigPath(
                "manager working directory cannot be the filesystem root".to_string(),
            ));
        }
        let directory = File::from(current);
        if !directory.metadata()?.is_dir() {
            return Err(CommandError::UnsafeConfigPath(
                "manager working directory must be a directory".to_string(),
            ));
        }
        Ok(directory)
    }
    #[cfg(not(unix))]
    {
        let metadata = fs::symlink_metadata(path).map_err(|error| {
            if error.kind() == std::io::ErrorKind::NotFound {
                CommandError::InvalidSkillManagerRequest(
                    "manager working directory must already exist".to_string(),
                )
            } else {
                error.into()
            }
        })?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(CommandError::UnsafeConfigPath(
                "manager working directory must be an existing non-symlink directory".to_string(),
            ));
        }
        File::open(path).map_err(Into::into)
    }
}

pub(super) fn validate_manager_working_directory_binding(
    path: &Path,
    accepted: &File,
) -> Result<(), CommandError> {
    let current = open_existing_manager_working_directory(path)?;
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
