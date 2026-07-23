use std::{
    fs, io,
    path::{Component, Path, PathBuf},
};

use crate::{
    action_source_revision, normalize_path_lexically, CommandError, ExternalTargetCapability,
    MAX_DIRECT_SKILL_BYTES,
};

pub(super) struct SkillFileState {
    pub(super) revision: String,
    pub(super) binding_revision: String,
    pub(super) content: String,
    pub(super) exists: bool,
}

pub(super) fn read_skill_file_state(
    path: &Path,
    required: bool,
) -> Result<SkillFileState, CommandError> {
    #[cfg(unix)]
    let (exists, content, binding_revision) = {
        use rustix::fs::{open, Mode, OFlags};

        let descriptor = match open(
            path,
            OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC,
            Mode::empty(),
        ) {
            Ok(descriptor) => Some(descriptor),
            Err(rustix::io::Errno::NOENT) if !required => None,
            Err(error) => return Err(io::Error::from(error).into()),
        };
        match descriptor {
            None => (false, String::new(), String::new()),
            Some(descriptor) => {
                use std::io::Read as _;
                use std::os::unix::fs::MetadataExt;

                let mut file = fs::File::from(descriptor);
                let before = file.metadata()?;
                if !before.is_file() || before.nlink() != 1 || before.len() > MAX_DIRECT_SKILL_BYTES
                {
                    return Err(CommandError::UnsafeConfigPath(format!(
                        "skill file is not a bounded single-link regular file: {}",
                        path.display()
                    )));
                }
                let before_binding = unix_file_binding_revision(&before);
                let mut bytes = Vec::with_capacity(before.len() as usize);
                file.by_ref()
                    .take(MAX_DIRECT_SKILL_BYTES.saturating_add(1))
                    .read_to_end(&mut bytes)?;
                let after_binding = unix_file_binding_revision(&file.metadata()?);
                let rebound = open(
                    path,
                    OFlags::RDONLY
                        | OFlags::NOFOLLOW
                        | OFlags::NONBLOCK
                        | OFlags::NOCTTY
                        | OFlags::CLOEXEC,
                    Mode::empty(),
                )
                .map_err(|_| CommandError::StaleActionReference)?;
                let rebound_binding =
                    unix_file_binding_revision(&fs::File::from(rebound).metadata()?);
                if bytes.len() as u64 != before.len()
                    || bytes.len() as u64 > MAX_DIRECT_SKILL_BYTES
                    || after_binding != before_binding
                    || rebound_binding != before_binding
                {
                    return Err(CommandError::StaleActionReference);
                }
                let content = String::from_utf8(bytes).map_err(|_| {
                    CommandError::UnsafeConfigPath(format!(
                        "skill file is not valid UTF-8: {}",
                        path.display()
                    ))
                })?;
                (true, content, before_binding)
            }
        }
    };
    #[cfg(not(unix))]
    let (exists, content, binding_revision) = match fs::read_to_string(path) {
        Ok(content) => (true, content, String::new()),
        Err(error) if !required && error.kind() == io::ErrorKind::NotFound => {
            (false, String::new(), String::new())
        }
        Err(error) => return Err(error.into()),
    };
    let revision = skill_file_revision(path, exists, &content)?;
    let binding_revision = if exists {
        binding_revision_for(path, &revision, &binding_revision)?
    } else {
        revision.clone()
    };
    Ok(SkillFileState {
        revision,
        binding_revision,
        content,
        exists,
    })
}

pub(super) fn owner_locked_skill_file_state(
    path: &Path,
    content: String,
    file_stamp: &str,
) -> Result<SkillFileState, CommandError> {
    let revision = skill_file_revision(path, true, &content)?;
    Ok(SkillFileState {
        binding_revision: binding_revision_for(path, &revision, file_stamp)?,
        revision,
        content,
        exists: true,
    })
}

fn binding_revision_for(
    path: &Path,
    content_revision: &str,
    file_stamp: &str,
) -> Result<String, CommandError> {
    action_source_revision(
        "skill.file.binding",
        &[
            ("path", &path.to_string_lossy()),
            ("content_revision", content_revision),
            ("file_stamp", file_stamp),
        ],
    )
}

#[cfg(unix)]
fn unix_file_binding_revision(metadata: &fs::Metadata) -> String {
    use std::os::unix::fs::MetadataExt;

    format!(
        "{}:{}:{:o}:{}:{}:{}:{}:{}:{}",
        metadata.dev(),
        metadata.ino(),
        metadata.mode(),
        metadata.nlink(),
        metadata.len(),
        metadata.mtime(),
        metadata.mtime_nsec(),
        metadata.ctime(),
        metadata.ctime_nsec()
    )
}

pub(super) fn skill_file_revision(
    path: &Path,
    exists: bool,
    content: &str,
) -> Result<String, CommandError> {
    let path_text = path.to_string_lossy();
    action_source_revision(
        "skill.file",
        &[
            ("path", &path_text),
            ("exists", if exists { "true" } else { "false" }),
            ("content", content),
        ],
    )
}

pub(super) fn tool_global_source_relative(
    app_data_dir: &Path,
    source: &Path,
) -> Result<PathBuf, CommandError> {
    #[cfg(unix)]
    let app_data = normalize_path_lexically(&crate::mutation_lock::normalize_trusted_root_alias(
        app_data_dir,
    )?);
    #[cfg(not(unix))]
    let app_data = normalize_path_lexically(app_data_dir);
    #[cfg(unix)]
    let source =
        normalize_path_lexically(&crate::mutation_lock::normalize_trusted_root_alias(source)?);
    #[cfg(not(unix))]
    let source = normalize_path_lexically(source);
    let staging_root = app_data.join("tool-global").join("skills");
    let skill_relative = source.strip_prefix(&staging_root).map_err(|_| {
        CommandError::UnsafeConfigPath(format!(
            "tool-global install source {} is outside the app-owned tool-global/skills root",
            source.display()
        ))
    })?;
    let components = skill_relative.components().collect::<Vec<_>>();
    if components.len() != 2
        || !components
            .iter()
            .all(|component| matches!(component, Component::Normal(_)))
        || skill_relative.file_name().and_then(|name| name.to_str()) != Some("SKILL.md")
    {
        return Err(CommandError::UnsafeConfigPath(format!(
            "tool-global install source {} is not a direct app-owned skill entry",
            source.display()
        )));
    }
    Ok(PathBuf::from("tool-global")
        .join("skills")
        .join(skill_relative))
}

pub(super) fn read_external_skill_state(
    target: &ExternalTargetCapability<'_>,
    path: &Path,
) -> Result<SkillFileState, CommandError> {
    let state = target.read_text_state(MAX_DIRECT_SKILL_BYTES)?;
    Ok(SkillFileState {
        revision: skill_file_revision(path, state.exists, &state.content)?,
        binding_revision: String::new(),
        content: state.content,
        exists: state.exists,
    })
}

pub(super) fn read_external_skill_state_anchored(
    target: &ExternalTargetCapability<'_>,
    path: &Path,
) -> Result<SkillFileState, CommandError> {
    let state = target.read_text_state_anchored(MAX_DIRECT_SKILL_BYTES)?;
    Ok(SkillFileState {
        revision: skill_file_revision(path, state.exists, &state.content)?,
        binding_revision: String::new(),
        content: state.content,
        exists: state.exists,
    })
}

#[cfg(test)]
struct InstallCatalogScanTestHook {
    target: PathBuf,
    action: Box<dyn FnOnce() + Send>,
}

#[cfg(test)]
thread_local! {
    static INSTALL_CATALOG_SCAN_TEST_HOOKS: std::cell::RefCell<Vec<InstallCatalogScanTestHook>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
pub(super) fn install_catalog_scan_test_hook(
    target: PathBuf,
    action: impl FnOnce() + Send + 'static,
) {
    INSTALL_CATALOG_SCAN_TEST_HOOKS.with(|hooks| {
        hooks.borrow_mut().push(InstallCatalogScanTestHook {
            target,
            action: Box::new(action),
        })
    });
}

#[cfg(test)]
pub(super) fn run_install_catalog_scan_test_hook(target: &Path) {
    let action = INSTALL_CATALOG_SCAN_TEST_HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        hooks
            .iter()
            .position(|hook| hook.target == target)
            .map(|index| hooks.remove(index).action)
    });
    if let Some(action) = action {
        action();
    }
}

#[cfg(not(test))]
pub(super) fn run_install_catalog_scan_test_hook(_target: &Path) {}
