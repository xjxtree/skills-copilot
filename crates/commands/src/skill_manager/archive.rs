use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skills_copilot_catalog::Catalog;
use skills_copilot_core::{AdapterContext, AgentId, Scope};
use zip::ZipArchive;

use crate::{
    import_local_skill_to_tool_global, register_tool_global_staged_skill, scan_all_catalog_report,
    tool_global_skill_name_from_content, tool_global_staging_skills_root, CommandError,
    SkillRecord,
};

use super::{redact_command_output, unix_timestamp_millis};

const MAX_ARCHIVE_BYTES: u64 = 64 * 1024 * 1024;
const MAX_ARCHIVE_ENTRIES: usize = 2_000;
const MAX_ARCHIVE_FILE_BYTES: u64 = 8 * 1024 * 1024;
const MAX_ARCHIVE_UNCOMPRESSED_BYTES: u64 = 64 * 1024 * 1024;
const MAX_SKILL_MD_BYTES: u64 = 2 * 1024 * 1024;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerLocalArchiveUpdateParams {
    pub instance_id: String,
    pub archive_path: String,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub preview_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SkillManagerLocalArchiveUpdateRecord {
    pub instance_id: String,
    pub skill_name: String,
    pub archive_path: String,
    pub archive_sha256: String,
    pub file_count: usize,
    pub uncompressed_bytes: u64,
    pub preview_token: String,
    pub confirmed: bool,
    pub applied: bool,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub updated_skill: Option<SkillRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerLocalArchiveImportParams {
    pub archive_path: String,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub preview_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SkillManagerLocalArchiveImportRecord {
    pub skill_name: String,
    pub archive_path: String,
    pub archive_sha256: String,
    pub file_count: usize,
    pub uncompressed_bytes: u64,
    pub preview_token: String,
    pub confirmed: bool,
    pub applied: bool,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub imported_skill: Option<SkillRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub instance_id: Option<String>,
}

struct ArchiveInspection {
    archive_sha256: String,
    skill_name: String,
    skill_root: PathBuf,
    file_count: usize,
    uncompressed_bytes: u64,
}

enum LocalArchiveUpdateTargetKind {
    AppOwned,
    CatalogLocal { instance_id: String },
}

struct LocalArchiveUpdateTarget {
    skill_name: String,
    skill_path: PathBuf,
    canonical_root: PathBuf,
    kind: LocalArchiveUpdateTargetKind,
}

pub fn preview_local_archive_import(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveImportParams,
) -> Result<SkillManagerLocalArchiveImportRecord, CommandError> {
    build_local_archive_import_record(catalog, app_data_dir, ctx, params, false)
}

pub fn apply_local_archive_import(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveImportParams,
) -> Result<SkillManagerLocalArchiveImportRecord, CommandError> {
    if !params.confirmed {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local archive import requires confirmed=true".to_string(),
        ));
    }
    build_local_archive_import_record(catalog, app_data_dir, ctx, params, true)
}

fn build_local_archive_import_record(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveImportParams,
    apply: bool,
) -> Result<SkillManagerLocalArchiveImportRecord, CommandError> {
    let archive_path = validate_archive_path(Path::new(params.archive_path.trim()))?;
    let inspection = inspect_archive(&archive_path)?;
    ensure_local_import_name_available(catalog, &inspection.skill_name)?;
    let preview_token =
        archive_import_preview_token(&inspection.skill_name, &inspection.archive_sha256);
    if apply && params.preview_token.as_deref() != Some(preview_token.as_str()) {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local archive import requires a fresh preview_token for the same ZIP".to_string(),
        ));
    }

    let imported = if apply {
        Some(apply_archive_import(
            catalog,
            app_data_dir,
            ctx,
            &archive_path,
            &inspection,
        )?)
    } else {
        None
    };
    Ok(SkillManagerLocalArchiveImportRecord {
        skill_name: inspection.skill_name,
        archive_path: redact_command_output(ctx, &archive_path.to_string_lossy()),
        archive_sha256: inspection.archive_sha256,
        file_count: inspection.file_count,
        uncompressed_bytes: inspection.uncompressed_bytes,
        preview_token,
        confirmed: apply,
        applied: apply,
        summary: if apply {
            "Imported the validated ZIP into the app-owned local skill library; skill scripts were not executed."
                .to_string()
        } else {
            "The ZIP contains one local skill and can be imported into the app-owned library after confirmation."
                .to_string()
        },
        instance_id: imported.as_ref().map(|result| result.instance_id.clone()),
        imported_skill: imported.map(|result| result.imported),
    })
}

fn ensure_local_import_name_available(
    catalog: &Catalog,
    skill_name: &str,
) -> Result<(), CommandError> {
    let duplicate = catalog.list_skill_records()?.into_iter().any(|record| {
        record.name.eq_ignore_ascii_case(skill_name)
            && record.state != "missing"
            && (record.agent == AgentId::ToolGlobal.as_str()
                || is_shared_agents_skill_path(&record.path))
    });
    if duplicate {
        return Err(CommandError::InvalidSkillManagerRequest(
            "a local library or installed skill with the same name already exists; select it and use ZIP update"
                .to_string(),
        ));
    }
    Ok(())
}

fn is_shared_agents_skill_path(path: &Path) -> bool {
    path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md")
        && path
            .components()
            .zip(path.components().skip(1))
            .any(|(parent, child)| parent.as_os_str() == ".agents" && child.as_os_str() == "skills")
}

fn apply_archive_import(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    archive_path: &Path,
    inspection: &ArchiveInspection,
) -> Result<crate::ToolGlobalImportResult, CommandError> {
    let import_root = app_data_dir.join("local-archive-imports");
    fs::create_dir_all(&import_root)?;
    if fs::symlink_metadata(&import_root)?.file_type().is_symlink() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local archive import staging root must not be a symlink".to_string(),
        ));
    }
    let canonical_root = import_root.canonicalize()?;
    let temp_dir = canonical_root.join(format!(
        ".archive-import-{}-{}",
        unix_timestamp_millis(),
        std::process::id()
    ));
    ensure_child_path(&canonical_root, &temp_dir)?;
    fs::create_dir(&temp_dir)?;
    if let Err(error) = extract_skill_root(archive_path, inspection, &temp_dir) {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(error);
    }
    let imported = import_local_skill_to_tool_global(
        catalog,
        ctx,
        &app_data_dir.join("tool-global"),
        &temp_dir,
    );
    let _ = fs::remove_dir_all(&temp_dir);
    imported
}

pub fn preview_local_archive_update(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveUpdateParams,
) -> Result<SkillManagerLocalArchiveUpdateRecord, CommandError> {
    build_local_archive_update_record(catalog, app_data_dir, ctx, params, false)
}

pub fn apply_local_archive_update(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveUpdateParams,
) -> Result<SkillManagerLocalArchiveUpdateRecord, CommandError> {
    if !params.confirmed {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local archive update requires confirmed=true".to_string(),
        ));
    }
    build_local_archive_update_record(catalog, app_data_dir, ctx, params, true)
}

fn build_local_archive_update_record(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveUpdateParams,
    apply: bool,
) -> Result<SkillManagerLocalArchiveUpdateRecord, CommandError> {
    let target = validate_local_update_target(catalog, app_data_dir, ctx, &params.instance_id)?;
    let archive_path = validate_archive_path(Path::new(params.archive_path.trim()))?;
    let inspection = inspect_archive(&archive_path)?;
    if !inspection
        .skill_name
        .eq_ignore_ascii_case(&target.skill_name)
    {
        return Err(CommandError::InvalidSkillManagerRequest(
            "ZIP skill name must match the selected local skill".to_string(),
        ));
    }
    let target_digest = digest_bounded_file(&target.skill_path, MAX_SKILL_MD_BYTES)?;
    let preview_token = archive_preview_token(
        &params.instance_id,
        &target.skill_name,
        &inspection.archive_sha256,
        &target_digest,
    );
    if apply && params.preview_token.as_deref() != Some(preview_token.as_str()) {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local archive update requires a fresh preview_token for the same ZIP and source"
                .to_string(),
        ));
    }

    let updated_skill = if apply {
        Some(apply_archive_replacement(
            catalog,
            ctx,
            &archive_path,
            &inspection,
            &target,
        )?)
    } else {
        None
    };

    Ok(SkillManagerLocalArchiveUpdateRecord {
        instance_id: params.instance_id.clone(),
        skill_name: target.skill_name,
        archive_path: redact_command_output(ctx, &archive_path.to_string_lossy()),
        archive_sha256: inspection.archive_sha256,
        file_count: inspection.file_count,
        uncompressed_bytes: inspection.uncompressed_bytes,
        preview_token,
        confirmed: apply,
        applied: apply,
        summary: if apply {
            "Replaced the selected local skill source from the confirmed ZIP; skill scripts were not executed."
                .to_string()
        } else {
            "The ZIP contains one matching local skill and can replace the selected local source after confirmation."
                .to_string()
        },
        updated_skill,
    })
}

fn validate_local_update_target(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    instance_id: &str,
) -> Result<LocalArchiveUpdateTarget, CommandError> {
    let meta = catalog
        .get_skill_instance_meta(instance_id)?
        .ok_or_else(|| CommandError::InstanceNotFound(instance_id.to_string()))?;
    let file_type = fs::symlink_metadata(&meta.path)?.file_type();
    if file_type.is_symlink() || !file_type.is_file() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "ZIP replacement requires a regular local SKILL.md source".to_string(),
        ));
    }
    let canonical_path = meta.path.canonicalize()?;
    if canonical_path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md") {
        return Err(CommandError::InvalidSkillManagerRequest(
            "ZIP replacement target must be a SKILL.md file".to_string(),
        ));
    }

    let app_owned_root = tool_global_staging_skills_root(app_data_dir);
    if meta.agent == AgentId::ToolGlobal {
        let canonical_root = app_owned_root.canonicalize().map_err(|_| {
            CommandError::InvalidSkillManagerRequest(
                "app-owned local skill root is unavailable".to_string(),
            )
        })?;
        validate_direct_skill_child(&canonical_root, &canonical_path)?;
        return Ok(LocalArchiveUpdateTarget {
            skill_name: meta.name,
            skill_path: canonical_path,
            canonical_root,
            kind: LocalArchiveUpdateTargetKind::AppOwned,
        });
    }

    let supported_agent = matches!(
        meta.agent.as_str(),
        "claude-code" | "codex" | "opencode" | "pi" | "hermes" | "openclaw"
    );
    if !supported_agent {
        return Err(CommandError::InvalidSkillManagerRequest(
            "ZIP replacement is limited to supported local skill sources".to_string(),
        ));
    }
    let mut local_roots = match meta.scope {
        Scope::AgentProject => [ctx.project_root.as_ref(), ctx.project_cwd.as_ref()]
            .into_iter()
            .flatten()
            .map(|root| root.join(".agents/skills"))
            .collect::<Vec<_>>(),
        Scope::AgentGlobal => vec![ctx.user_home.join(".agents/skills")],
        _ => Vec::new(),
    };
    local_roots.dedup();
    if local_roots.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local skill scope has no guarded .agents/skills source root".to_string(),
        ));
    }
    let canonical_root = local_roots
        .into_iter()
        .filter_map(|root| root.canonicalize().ok())
        .find(|root| validate_skill_descendant(root, &canonical_path).is_ok())
        .ok_or_else(|| {
            CommandError::InvalidSkillManagerRequest(
                "local skill source is outside the active guarded .agents/skills roots".to_string(),
            )
        })?;
    Ok(LocalArchiveUpdateTarget {
        skill_name: meta.name,
        skill_path: canonical_path,
        canonical_root,
        kind: LocalArchiveUpdateTargetKind::CatalogLocal {
            instance_id: meta.id,
        },
    })
}

fn validate_direct_skill_child(root: &Path, skill_path: &Path) -> Result<(), CommandError> {
    let skill_dir = skill_path.parent().ok_or_else(|| {
        CommandError::InvalidSkillManagerRequest(
            "local skill source has no parent directory".to_string(),
        )
    })?;
    if skill_dir.parent() != Some(root) || !skill_path.starts_with(root) {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local skill source is outside the guarded direct-child root".to_string(),
        ));
    }
    Ok(())
}

fn validate_skill_descendant(root: &Path, skill_path: &Path) -> Result<(), CommandError> {
    let skill_dir = skill_path.parent().ok_or_else(|| {
        CommandError::InvalidSkillManagerRequest(
            "local skill source has no parent directory".to_string(),
        )
    })?;
    if skill_dir == root || !skill_path.starts_with(root) {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local skill source is outside the guarded .agents/skills root".to_string(),
        ));
    }
    Ok(())
}

fn validate_archive_path(path: &Path) -> Result<PathBuf, CommandError> {
    if !path.is_absolute() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local skill ZIP path must be absolute".to_string(),
        ));
    }
    let metadata = fs::symlink_metadata(path)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_ARCHIVE_BYTES
        || path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_none_or(|extension| !extension.eq_ignore_ascii_case("zip"))
    {
        return Err(CommandError::InvalidSkillManagerRequest(
            "select a regular ZIP file no larger than 64 MiB".to_string(),
        ));
    }
    Ok(path.canonicalize()?)
}

fn inspect_archive(path: &Path) -> Result<ArchiveInspection, CommandError> {
    let archive_sha256 = digest_bounded_file(path, MAX_ARCHIVE_BYTES)?;
    let mut archive = open_archive(path)?;
    if archive.is_empty() || archive.len() > MAX_ARCHIVE_ENTRIES {
        return Err(invalid_archive(
            "ZIP must contain between 1 and 2000 entries",
        ));
    }
    let mut skill_entries = Vec::new();
    let mut file_paths = Vec::new();
    let mut seen_file_paths = BTreeSet::new();
    let mut file_count = 0usize;
    let mut uncompressed_bytes = 0u64;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(zip_error)?;
        let path = safe_entry_path(&entry)?;
        if should_ignore_entry(&path) || entry.is_dir() {
            continue;
        }
        validate_entry_mode(&entry)?;
        if !seen_file_paths.insert(path.clone()) {
            return Err(invalid_archive("ZIP contains duplicate file paths"));
        }
        file_paths.push(path.clone());
        if entry.size() > MAX_ARCHIVE_FILE_BYTES {
            return Err(invalid_archive("ZIP contains a file larger than 8 MiB"));
        }
        uncompressed_bytes = uncompressed_bytes
            .checked_add(entry.size())
            .filter(|total| *total <= MAX_ARCHIVE_UNCOMPRESSED_BYTES)
            .ok_or_else(|| invalid_archive("ZIP expands beyond 64 MiB"))?;
        file_count += 1;
        if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md") {
            if entry.size() > MAX_SKILL_MD_BYTES {
                return Err(invalid_archive("SKILL.md exceeds 2 MiB"));
            }
            let mut content = String::new();
            entry
                .read_to_string(&mut content)
                .map_err(|_| invalid_archive("SKILL.md must be valid UTF-8 text"))?;
            let fallback = path
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                .unwrap_or("local-skill");
            skill_entries.push((
                path.parent().unwrap_or_else(|| Path::new("")).to_path_buf(),
                tool_global_skill_name_from_content(&content, fallback),
            ));
        }
    }
    if skill_entries.len() != 1 {
        return Err(invalid_archive("ZIP must contain exactly one SKILL.md"));
    }
    let (skill_root, skill_name) = skill_entries.remove(0);
    if !skill_root.as_os_str().is_empty()
        && file_paths.iter().any(|path| !path.starts_with(&skill_root))
    {
        return Err(invalid_archive(
            "ZIP contains files outside the single skill directory",
        ));
    }
    Ok(ArchiveInspection {
        archive_sha256,
        skill_name,
        skill_root,
        file_count,
        uncompressed_bytes,
    })
}

fn apply_archive_replacement(
    catalog: &Catalog,
    ctx: &AdapterContext,
    archive_path: &Path,
    inspection: &ArchiveInspection,
    target: &LocalArchiveUpdateTarget,
) -> Result<SkillRecord, CommandError> {
    let existing_skill_path = &target.skill_path;
    let canonical_root = &target.canonical_root;
    let nonce = format!("{}-{}", unix_timestamp_millis(), std::process::id());
    let temp_dir = canonical_root.join(format!(".archive-update-{nonce}"));
    let backup_dir = canonical_root.join(format!(".archive-backup-{nonce}"));
    ensure_child_path(canonical_root, &temp_dir)?;
    ensure_child_path(canonical_root, &backup_dir)?;
    fs::create_dir(&temp_dir)?;
    let extraction = extract_skill_root(archive_path, inspection, &temp_dir);
    if let Err(error) = extraction {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(error);
    }

    let replacement_skill_path = temp_dir.join("SKILL.md");
    let replacement_content = fs::read_to_string(&replacement_skill_path)?;
    let replacement_name = tool_global_skill_name_from_content(
        &replacement_content,
        existing_skill_path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("local-skill"),
    );
    if !replacement_name.eq_ignore_ascii_case(&inspection.skill_name) {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(invalid_archive(
            "ZIP skill identity changed during extraction",
        ));
    }

    let target_dir = existing_skill_path.parent().ok_or_else(|| {
        CommandError::InvalidSkillManagerRequest(
            "local skill source has no parent directory".to_string(),
        )
    })?;
    ensure_child_path(canonical_root, target_dir)?;
    fs::rename(target_dir, &backup_dir)?;
    if let Err(error) = fs::rename(&temp_dir, target_dir) {
        let _ = fs::rename(&backup_dir, target_dir);
        return Err(error.into());
    }

    let registered = (|| {
        let staged_skill_path = target_dir.join("SKILL.md").canonicalize()?;
        match &target.kind {
            LocalArchiveUpdateTargetKind::AppOwned => Ok(register_tool_global_staged_skill(
                catalog,
                ctx,
                archive_path,
                &staged_skill_path,
            )?
            .imported),
            LocalArchiveUpdateTargetKind::CatalogLocal { instance_id } => {
                scan_all_catalog_report(ctx, catalog)?;
                catalog
                    .get_skill_record(instance_id)?
                    .or_else(|| {
                        catalog
                            .list_skill_records()
                            .ok()?
                            .into_iter()
                            .find(|record| {
                                record.name.eq_ignore_ascii_case(&target.skill_name)
                                    && record.path == staged_skill_path
                            })
                    })
                    .ok_or_else(|| CommandError::InstanceNotFound(instance_id.clone()))
            }
        }
    })();
    match registered {
        Ok(updated_skill) => {
            fs::remove_dir_all(&backup_dir)?;
            Ok(updated_skill)
        }
        Err(error) => {
            let _ = fs::remove_dir_all(target_dir);
            let restored = fs::rename(&backup_dir, target_dir);
            if restored.is_ok() {
                match &target.kind {
                    LocalArchiveUpdateTargetKind::AppOwned => {
                        if let Ok(restored_path) = target_dir.join("SKILL.md").canonicalize() {
                            let _ = register_tool_global_staged_skill(
                                catalog,
                                ctx,
                                target_dir,
                                &restored_path,
                            );
                        }
                    }
                    LocalArchiveUpdateTargetKind::CatalogLocal { .. } => {
                        let _ = scan_all_catalog_report(ctx, catalog);
                    }
                }
            }
            Err(error)
        }
    }
}

fn extract_skill_root(
    archive_path: &Path,
    inspection: &ArchiveInspection,
    destination: &Path,
) -> Result<(), CommandError> {
    let mut archive = open_archive(archive_path)?;
    for index in 0..archive.len() {
        let mut entry = archive.by_index(index).map_err(zip_error)?;
        let path = safe_entry_path(&entry)?;
        if should_ignore_entry(&path) {
            continue;
        }
        validate_entry_mode(&entry)?;
        let relative = if inspection.skill_root.as_os_str().is_empty() {
            path.as_path()
        } else if let Ok(relative) = path.strip_prefix(&inspection.skill_root) {
            relative
        } else if entry.is_dir() {
            continue;
        } else {
            return Err(invalid_archive(
                "ZIP contains files outside the single skill directory",
            ));
        };
        if relative.as_os_str().is_empty() {
            continue;
        }
        let target = destination.join(relative);
        ensure_child_path(destination, &target)?;
        if entry.is_dir() {
            fs::create_dir_all(&target)?;
            continue;
        }
        if let Some(parent) = target.parent() {
            fs::create_dir_all(parent)?;
        }
        let mut output = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&target)?;
        let copied = std::io::copy(
            &mut entry.by_ref().take(MAX_ARCHIVE_FILE_BYTES + 1),
            &mut output,
        )?;
        output.flush()?;
        if copied > MAX_ARCHIVE_FILE_BYTES {
            return Err(invalid_archive("ZIP contains a file larger than 8 MiB"));
        }
    }
    if !destination.join("SKILL.md").is_file() {
        return Err(invalid_archive("ZIP extraction did not produce SKILL.md"));
    }
    Ok(())
}

fn open_archive(path: &Path) -> Result<ZipArchive<File>, CommandError> {
    ZipArchive::new(File::open(path)?).map_err(zip_error)
}

fn safe_entry_path(entry: &zip::read::ZipFile<'_>) -> Result<PathBuf, CommandError> {
    let path = entry
        .enclosed_name()
        .ok_or_else(|| invalid_archive("ZIP contains an unsafe path"))?;
    if path.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return Err(invalid_archive("ZIP contains an unsafe path"));
    }
    Ok(path)
}

fn validate_entry_mode(entry: &zip::read::ZipFile<'_>) -> Result<(), CommandError> {
    if let Some(mode) = entry.unix_mode() {
        let file_type = mode & 0o170000;
        if file_type == 0o120000 || !matches!(file_type, 0 | 0o040000 | 0o100000) {
            return Err(invalid_archive(
                "ZIP symlinks and special files are not allowed",
            ));
        }
    }
    Ok(())
}

fn should_ignore_entry(path: &Path) -> bool {
    path.components()
        .next()
        .is_some_and(|component| component.as_os_str() == "__MACOSX")
        || path.file_name().is_some_and(|name| name == ".DS_Store")
}

fn ensure_child_path(root: &Path, path: &Path) -> Result<(), CommandError> {
    if path == root || !path.starts_with(root) {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local archive path escaped the allowed local skill root".to_string(),
        ));
    }
    Ok(())
}

fn digest_bounded_file(path: &Path, max_bytes: u64) -> Result<String, CommandError> {
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() || metadata.len() > max_bytes {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local archive input exceeds its safe size limit".to_string(),
        ));
    }
    let mut file = File::open(path)?;
    let mut hasher = Sha256::new();
    let mut buffer = [0u8; 64 * 1024];
    let mut total = 0u64;
    loop {
        let read = file.read(&mut buffer)?;
        if read == 0 {
            break;
        }
        total += read as u64;
        if total > max_bytes {
            return Err(CommandError::InvalidSkillManagerRequest(
                "local archive input exceeds its safe size limit".to_string(),
            ));
        }
        hasher.update(&buffer[..read]);
    }
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn archive_preview_token(
    instance_id: &str,
    skill_name: &str,
    archive_digest: &str,
    target_digest: &str,
) -> String {
    let mut hasher = Sha256::new();
    for value in [instance_id, skill_name, archive_digest, target_digest] {
        hasher.update(value.len().to_le_bytes());
        hasher.update(value.as_bytes());
    }
    format!("skill-manager-local-archive:{:x}", hasher.finalize())
}

fn archive_import_preview_token(skill_name: &str, archive_digest: &str) -> String {
    let mut hasher = Sha256::new();
    for value in [skill_name, archive_digest] {
        hasher.update(value.len().to_le_bytes());
        hasher.update(value.as_bytes());
    }
    format!("skill-manager-local-archive-import:{:x}", hasher.finalize())
}

fn invalid_archive(detail: &str) -> CommandError {
    CommandError::InvalidSkillManagerRequest(detail.to_string())
}

fn zip_error(error: zip::result::ZipError) -> CommandError {
    CommandError::InvalidSkillManagerRequest(format!("invalid local skill ZIP: {error}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use zip::{write::SimpleFileOptions, ZipWriter};

    fn archive_path(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "skill-manager-archive-{label}-{}-{}.zip",
            std::process::id(),
            unix_timestamp_millis()
        ))
    }

    fn write_archive(label: &str, entries: &[(&str, &str)]) -> PathBuf {
        let path = archive_path(label);
        let file = File::create(&path).expect("archive file");
        let mut writer = ZipWriter::new(file);
        for (name, content) in entries {
            writer
                .start_file(*name, SimpleFileOptions::default())
                .expect("archive entry");
            writer
                .write_all(content.as_bytes())
                .expect("archive content");
        }
        writer.finish().expect("finish archive");
        path
    }

    #[test]
    fn inspection_accepts_one_bounded_skill_directory() {
        let path = write_archive(
            "valid",
            &[
                (
                    "review-helper/SKILL.md",
                    "---\nname: review-helper\n---\n# Review",
                ),
                ("review-helper/references/guide.md", "safe reference"),
            ],
        );

        let inspection = inspect_archive(&path).expect("valid archive");

        assert_eq!(inspection.skill_name, "review-helper");
        assert_eq!(inspection.file_count, 2);
        fs::remove_file(path).ok();
    }

    #[test]
    fn inspection_rejects_multiple_skills_and_outside_files() {
        let multiple = write_archive(
            "multiple",
            &[
                ("one/SKILL.md", "---\nname: one\n---"),
                ("two/SKILL.md", "---\nname: two\n---"),
            ],
        );
        assert!(inspect_archive(&multiple).is_err());
        fs::remove_file(multiple).ok();

        let outside = write_archive(
            "outside",
            &[
                ("one/SKILL.md", "---\nname: one\n---"),
                ("unrelated.txt", "not part of the skill"),
            ],
        );
        assert!(matches!(
            inspect_archive(&outside),
            Err(CommandError::InvalidSkillManagerRequest(detail))
                if detail.contains("outside the single skill directory")
        ));
        fs::remove_file(outside).ok();
    }

    #[test]
    fn confirmed_archive_import_creates_app_owned_local_skill() {
        let archive = write_archive(
            "import",
            &[(
                "import-helper/SKILL.md",
                "---\nname: import-helper\ndescription: Imported helper\n---\n# Imported",
            )],
        );
        let root = std::env::temp_dir().join(format!(
            "skill-manager-archive-import-{}-{}",
            std::process::id(),
            unix_timestamp_millis()
        ));
        fs::create_dir_all(&root).expect("test root");
        let catalog = Catalog::open(&root.join("catalog.sqlite")).expect("catalog");
        catalog.init().expect("catalog schema");
        let ctx = AdapterContext {
            user_home: root.join("home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        };
        fs::create_dir_all(&ctx.user_home).expect("home");
        let preview = preview_local_archive_import(
            &catalog,
            &root.join("app-data"),
            &ctx,
            &SkillManagerLocalArchiveImportParams {
                archive_path: archive.to_string_lossy().to_string(),
                confirmed: false,
                preview_token: None,
            },
        )
        .expect("import preview");
        let applied = apply_local_archive_import(
            &catalog,
            &root.join("app-data"),
            &ctx,
            &SkillManagerLocalArchiveImportParams {
                archive_path: archive.to_string_lossy().to_string(),
                confirmed: true,
                preview_token: Some(preview.preview_token),
            },
        )
        .expect("confirmed import");

        assert!(applied.applied);
        let imported = applied.imported_skill.expect("imported skill");
        assert_eq!(imported.agent, AgentId::ToolGlobal.as_str());
        assert_eq!(imported.name, "import-helper");
        assert!(Path::new(&imported.path).is_file());

        fs::remove_file(archive).ok();
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn archive_import_rejects_same_name_as_installed_shared_source() {
        let archive = write_archive(
            "duplicate-installed-import",
            &[(
                "shared-helper/SKILL.md",
                "---\nname: shared-helper\ndescription: Replacement\n---\n# Replacement",
            )],
        );
        let root = std::env::temp_dir().join(format!(
            "skill-manager-duplicate-installed-import-{}-{}",
            std::process::id(),
            unix_timestamp_millis()
        ));
        let home = root.join("home");
        let project = root.join("project");
        let skill_dir = project.join(".agents/skills/shared-helper");
        fs::create_dir_all(&skill_dir).expect("installed source directory");
        fs::create_dir_all(&home).expect("home");
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: shared-helper\ndescription: Installed\n---\n# Installed",
        )
        .expect("installed skill");
        let catalog = Catalog::open(&root.join("catalog.sqlite")).expect("catalog");
        catalog.init().expect("catalog schema");
        let ctx = AdapterContext {
            user_home: home,
            project_root: Some(project.clone()),
            project_cwd: Some(project),
            extra_roots: Vec::new(),
        };
        scan_all_catalog_report(&ctx, &catalog).expect("project scan");

        let error = preview_local_archive_import(
            &catalog,
            &root.join("app-data"),
            &ctx,
            &SkillManagerLocalArchiveImportParams {
                archive_path: archive.to_string_lossy().to_string(),
                confirmed: false,
                preview_token: None,
            },
        )
        .expect_err("same-name installed source must block a second import");

        assert!(matches!(
            error,
            CommandError::InvalidSkillManagerRequest(detail)
                if detail.contains("installed skill with the same name")
        ));

        fs::remove_file(archive).ok();
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn confirmed_archive_update_replaces_guarded_project_local_source() {
        let archive = write_archive(
            "project-update",
            &[(
                "project-helper/SKILL.md",
                "---\nname: project-helper\ndescription: Updated locally\n---\n# Updated",
            )],
        );
        let root = std::env::temp_dir().join(format!(
            "skill-manager-project-archive-update-{}-{}",
            std::process::id(),
            unix_timestamp_millis()
        ));
        let home = root.join("home");
        let project = root.join("project");
        let skill_dir = project.join(".agents/skills/project-helper");
        fs::create_dir_all(&skill_dir).expect("project skill directory");
        fs::create_dir_all(&home).expect("home");
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: project-helper\ndescription: Original local\n---\n# Original",
        )
        .expect("original local skill");
        let catalog = Catalog::open(&root.join("catalog.sqlite")).expect("catalog");
        catalog.init().expect("catalog schema");
        let ctx = AdapterContext {
            user_home: home,
            project_root: Some(project.clone()),
            project_cwd: Some(project.clone()),
            extra_roots: Vec::new(),
        };
        scan_all_catalog_report(&ctx, &catalog).expect("initial project scan");
        let canonical_skill_path = skill_dir
            .join("SKILL.md")
            .canonicalize()
            .expect("canonical project skill");
        let instance = catalog
            .list_skill_records()
            .expect("project records")
            .into_iter()
            .find(|record| {
                record.name == "project-helper"
                    && record.scope == Scope::AgentProject.as_str()
                    && record.path == canonical_skill_path
            })
            .expect("guarded project skill record");
        let preview = preview_local_archive_update(
            &catalog,
            &root.join("app-data"),
            &ctx,
            &SkillManagerLocalArchiveUpdateParams {
                instance_id: instance.id.clone(),
                archive_path: archive.to_string_lossy().to_string(),
                confirmed: false,
                preview_token: None,
            },
        )
        .expect("project local update preview");
        let applied = apply_local_archive_update(
            &catalog,
            &root.join("app-data"),
            &ctx,
            &SkillManagerLocalArchiveUpdateParams {
                instance_id: instance.id,
                archive_path: archive.to_string_lossy().to_string(),
                confirmed: true,
                preview_token: Some(preview.preview_token),
            },
        )
        .expect("confirmed project local update");

        assert!(applied.applied);
        let updated = fs::read_to_string(skill_dir.join("SKILL.md")).expect("updated skill");
        assert!(updated.contains("Updated locally"));
        assert!(!updated.contains("Original local"));

        fs::remove_file(archive).ok();
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn archive_update_accepts_nested_guarded_global_source() {
        let archive = write_archive(
            "nested-global-update",
            &[(
                "nested-helper/SKILL.md",
                "---\nname: nested-helper\ndescription: Updated locally\n---\n# Updated",
            )],
        );
        let root = std::env::temp_dir().join(format!(
            "skill-manager-nested-global-update-{}-{}",
            std::process::id(),
            unix_timestamp_millis()
        ));
        let home = root.join("home");
        let skill_dir = home.join(".agents/skills/source-bundle/nested-helper");
        fs::create_dir_all(&skill_dir).expect("nested global skill directory");
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: nested-helper\ndescription: Original local\n---\n# Original",
        )
        .expect("original nested skill");
        let catalog = Catalog::open(&root.join("catalog.sqlite")).expect("catalog");
        catalog.init().expect("catalog schema");
        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        };
        scan_all_catalog_report(&ctx, &catalog).expect("initial global scan");
        let canonical_skill_path = skill_dir
            .join("SKILL.md")
            .canonicalize()
            .expect("canonical nested skill");
        let instance = catalog
            .list_skill_records()
            .expect("global records")
            .into_iter()
            .find(|record| {
                record.name == "nested-helper"
                    && record.scope == Scope::AgentGlobal.as_str()
                    && record.path == canonical_skill_path
            })
            .expect("nested guarded global skill record");

        let preview = preview_local_archive_update(
            &catalog,
            &root.join("app-data"),
            &ctx,
            &SkillManagerLocalArchiveUpdateParams {
                instance_id: instance.id,
                archive_path: archive.to_string_lossy().to_string(),
                confirmed: false,
                preview_token: None,
            },
        )
        .expect("nested global update preview");

        assert_eq!(preview.skill_name, "nested-helper");
        assert!(!preview.applied);

        fs::remove_file(archive).ok();
        fs::remove_dir_all(root).ok();
    }
}
