use std::{
    collections::BTreeMap,
    fs::{self, File},
    io::Read,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skills_copilot_core::AdapterContext;

use crate::{parse_export_skill_file, CommandError};

use super::{
    command_preview_with_executable, redact_command_output, run_previewed_command,
    CommandPreviewDraft, SkillManagerCommandOutput, SkillManagerCommandPreview, SKILLS_CLI_BINARY,
};

const MAX_LOCAL_SOURCE_ENTRIES: usize = 2_000;
const MAX_LOCAL_SOURCE_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_LOCAL_SOURCE_BYTES: u64 = 64 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerInspectLocalSourceParams {
    pub source_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerLocalSourceSkillRecord {
    pub name: String,
    pub description: String,
    pub relative_path: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerLocalSourceInspectionRecord {
    pub preview: SkillManagerCommandPreview,
    pub output: SkillManagerCommandOutput,
    pub source_path: String,
    pub source_revision: String,
    pub skills: Vec<SkillManagerLocalSourceSkillRecord>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct LocalSourceTreeInspection {
    canonical_path: PathBuf,
    source_revision: String,
    skills: Vec<SkillManagerLocalSourceSkillRecord>,
}

pub fn inspect_local_source_with_manager(
    ctx: &AdapterContext,
    params: &SkillManagerInspectLocalSourceParams,
) -> Result<SkillManagerLocalSourceInspectionRecord, CommandError> {
    let executable = super::npx_executable()?;
    inspect_local_source_with_executable(ctx, params, &executable)
}

pub(super) fn inspect_local_source_with_executable(
    ctx: &AdapterContext,
    params: &SkillManagerInspectLocalSourceParams,
    executable: &Path,
) -> Result<SkillManagerLocalSourceInspectionRecord, CommandError> {
    let before = inspect_local_source_tree(Path::new(params.source_path.trim()))?;
    let preview = command_preview_with_executable(
        ctx,
        CommandPreviewDraft {
            operation: "inspectLocalSource",
            args: vec![
                SKILLS_CLI_BINARY.to_string(),
                "add".to_string(),
                before.canonical_path.to_string_lossy().to_string(),
                "--list".to_string(),
                "--full-depth".to_string(),
            ],
            cwd: before.canonical_path.clone(),
            network_required: false,
            network_allowed: true,
            confirmed: false,
            summary: format!(
                "Inspect {} local skill(s) through npx skills without installing them.",
                before.skills.len()
            ),
            risks: vec![
                "The selected directory is read and validated locally; packaged scripts are not executed."
                    .to_string(),
            ],
            source: Some(before.canonical_path.to_string_lossy().to_string()),
            skills: before
                .skills
                .iter()
                .map(|skill| skill.name.clone())
                .collect(),
        },
        executable,
    )?;
    let execution = run_previewed_command(ctx, &preview)?;
    let after = inspect_local_source_tree(&before.canonical_path)?;
    if before.source_revision != after.source_revision || before.skills != after.skills {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local skill source changed while it was being inspected; choose the folder again"
                .to_string(),
        ));
    }

    Ok(SkillManagerLocalSourceInspectionRecord {
        preview,
        output: execution.output,
        source_path: redact_command_output(ctx, &before.canonical_path.to_string_lossy()),
        source_revision: before.source_revision,
        skills: before.skills,
    })
}

pub(super) fn local_source_install_binding(
    path: &Path,
    selected_skills: &[String],
) -> Result<String, CommandError> {
    let inspection = inspect_local_source_tree(path)?;
    let available = inspection
        .skills
        .iter()
        .map(|skill| skill.name.to_lowercase())
        .collect::<std::collections::BTreeSet<_>>();
    for selected in selected_skills {
        if !available.contains(&selected.to_lowercase()) {
            return Err(CommandError::InvalidSkillManagerRequest(format!(
                "local skill source does not contain selected skill: {selected}"
            )));
        }
    }
    Ok(inspection.source_revision)
}

fn inspect_local_source_tree(path: &Path) -> Result<LocalSourceTreeInspection, CommandError> {
    if path.as_os_str().is_empty() || !path.is_absolute() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local skill source must be an absolute directory".to_string(),
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local skill source must be a regular directory, not a symlink".to_string(),
        ));
    }
    let canonical_path = path.canonicalize()?;
    let mut stack = vec![canonical_path.clone()];
    let mut entries_seen = 0usize;
    let mut total_bytes = 0u64;
    let mut hasher = Sha256::new();
    let mut skills_by_name = BTreeMap::<String, SkillManagerLocalSourceSkillRecord>::new();

    while let Some(directory) = stack.pop() {
        let mut entries = fs::read_dir(&directory)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let entry_path = entry.path();
            let relative_path = entry_path.strip_prefix(&canonical_path).map_err(|_| {
                CommandError::InvalidSkillManagerRequest(
                    "local skill source entry escaped the selected directory".to_string(),
                )
            })?;
            if should_ignore_local_source_entry(relative_path) {
                continue;
            }
            entries_seen += 1;
            if entries_seen > MAX_LOCAL_SOURCE_ENTRIES {
                return Err(CommandError::InvalidSkillManagerRequest(format!(
                    "local skill source exceeds the safe {MAX_LOCAL_SOURCE_ENTRIES}-entry limit"
                )));
            }
            let metadata = fs::symlink_metadata(&entry_path)?;
            let file_type = metadata.file_type();
            if file_type.is_symlink() {
                return Err(CommandError::InvalidSkillManagerRequest(
                    "local skill source symlinks are not allowed".to_string(),
                ));
            }
            if file_type.is_dir() {
                stack.push(entry_path);
                continue;
            }
            if !file_type.is_file() {
                return Err(CommandError::InvalidSkillManagerRequest(
                    "local skill source special files are not allowed".to_string(),
                ));
            }
            if metadata.len() > MAX_LOCAL_SOURCE_FILE_BYTES {
                return Err(CommandError::InvalidSkillManagerRequest(
                    "local skill source contains a file larger than 8 MiB".to_string(),
                ));
            }
            total_bytes = total_bytes.checked_add(metadata.len()).ok_or_else(|| {
                CommandError::InvalidSkillManagerRequest(
                    "local skill source size overflowed its safe limit".to_string(),
                )
            })?;
            if total_bytes > MAX_LOCAL_SOURCE_BYTES {
                return Err(CommandError::InvalidSkillManagerRequest(
                    "local skill source exceeds the safe 64 MiB expanded-size limit".to_string(),
                ));
            }

            let relative_display = relative_path.to_string_lossy().replace('\\', "/");
            hasher.update(relative_display.as_bytes());
            hasher.update(b"\0");
            let mut file = File::open(&entry_path)?;
            let mut buffer = [0u8; 64 * 1024];
            loop {
                let read = file.read(&mut buffer)?;
                if read == 0 {
                    break;
                }
                hasher.update(&buffer[..read]);
            }
            hasher.update(b"\0");

            if entry_path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md") {
                continue;
            }
            let Ok(parsed) = parse_export_skill_file(&entry_path) else {
                continue;
            };
            let key = parsed.name.to_lowercase();
            if skills_by_name.contains_key(&key) {
                return Err(CommandError::InvalidSkillManagerRequest(format!(
                    "local skill source contains duplicate skill name: {}",
                    parsed.name
                )));
            }
            skills_by_name.insert(
                key,
                SkillManagerLocalSourceSkillRecord {
                    name: parsed.name,
                    description: parsed.description,
                    relative_path: relative_display,
                },
            );
        }
    }

    if skills_by_name.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local skill source does not contain a valid SKILL.md".to_string(),
        ));
    }
    Ok(LocalSourceTreeInspection {
        canonical_path,
        source_revision: format!("sha256:{:x}", hasher.finalize()),
        skills: skills_by_name.into_values().collect(),
    })
}

fn should_ignore_local_source_entry(path: &Path) -> bool {
    path.components().any(|component| {
        matches!(
            component.as_os_str().to_str(),
            Some(".git" | ".svn" | "node_modules" | "__MACOSX")
        )
    }) || path.file_name().is_some_and(|name| name == ".DS_Store")
}
