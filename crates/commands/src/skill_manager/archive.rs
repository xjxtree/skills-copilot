use std::{
    collections::BTreeSet,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skills_copilot_catalog::Catalog;
use skills_copilot_core::{
    ActionDescriptor, ActionImpact, ActionIntent, ActionKind, ActionNetworkPosture,
    ActionReadbackDomain, ActionTargetKind, ActionTargetRef, AdapterContext, AgentId, Scope,
};
use zip::ZipArchive;

use crate::{
    action_descriptor, action_preview_binding, action_source_revision, canonical_project_id,
    canonical_readback_domains, ensure_action_confirmed, register_tool_global_staged_skill,
    scan_all_catalog_report, tool_global_skill_name_from_content, tool_global_staging_skills_root,
    ActionConfirmation, ActionPrecondition, ActionPreconditionKind, ActionPreviewBinding,
    ActionReadbackObservation, ActionReadbackRecord, ActionReference, CommandError, SkillRecord,
};

use super::{
    local_delete_catalog_revision, local_delete_references, redact_command_output,
    unix_timestamp_millis,
};

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
    #[serde(default)]
    pub action_reference: Option<ActionReference>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SkillManagerLocalArchiveUpdateRecord {
    pub action: ActionDescriptor,
    pub preconditions: Vec<ActionPrecondition>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readback: Option<ActionReadbackRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_up: Option<super::SkillManagerCleanupFollowUp>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerLocalArchiveImportParams {
    pub archive_path: String,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub preview_token: Option<String>,
    #[serde(default)]
    pub action_reference: Option<ActionReference>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct SkillManagerLocalArchiveImportRecord {
    pub action: ActionDescriptor,
    pub preconditions: Vec<ActionPrecondition>,
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
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readback: Option<ActionReadbackRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_up: Option<super::SkillManagerCleanupFollowUp>,
}

struct ArchiveInspection {
    archive_sha256: String,
    skill_name: String,
    skill_root: PathBuf,
    file_count: usize,
    uncompressed_bytes: u64,
    tree_revision: String,
}

enum LocalArchiveUpdateTargetKind {
    AppOwned,
    CatalogLocal { instance_id: String },
}

struct LocalArchiveUpdateTarget {
    instance_id: String,
    skill_name: String,
    skill_path: PathBuf,
    canonical_root: PathBuf,
    agent: AgentId,
    scope: Scope,
    catalog_revision: String,
    kind: LocalArchiveUpdateTargetKind,
}

pub fn preview_local_archive_import(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveImportParams,
) -> Result<SkillManagerLocalArchiveImportRecord, CommandError> {
    let plan = prepare_local_archive_import(catalog, app_data_dir, ctx, params)?;
    Ok(local_archive_import_record(plan, false, None, None))
}

pub fn apply_local_archive_import(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveImportParams,
) -> Result<SkillManagerLocalArchiveImportRecord, CommandError> {
    let preview = prepare_local_archive_import(catalog, app_data_dir, ctx, params)?;
    ensure_archive_confirmation(
        &preview.action_binding,
        params.confirmed,
        params.preview_token.as_deref(),
        params.action_reference.as_ref(),
    )?;
    let _owner = crate::mutation_lock::lock_app_mutations(app_data_dir)?;
    let locked = prepare_local_archive_import(catalog, app_data_dir, ctx, params)?;
    ensure_archive_confirmation(
        &locked.action_binding,
        params.confirmed,
        params.preview_token.as_deref(),
        params.action_reference.as_ref(),
    )?;
    let transaction = catalog.begin_immediate_transaction()?;
    let mut applied = apply_archive_import_guarded(catalog, app_data_dir, ctx, &locked)?;
    let record = local_archive_import_record(
        locked,
        true,
        Some(applied.imported.clone()),
        Some(applied.readback.clone()),
    );
    if let Err(error) = transaction.commit() {
        applied.restore().map_err(|cleanup_error| {
            archive_partial(
                "skillManager.applyLocalArchiveImport",
                "outcome_unknown",
                true,
                format!(
                    "catalog commit failed ({error}); imported tree restoration failed ({cleanup_error})"
                ),
            )
        })?;
        return Err(archive_partial(
            "skillManager.applyLocalArchiveImport",
            "applied_unverified",
            false,
            format!("catalog commit failed after local archive import: {error}"),
        ));
    }
    applied.commit();
    Ok(record)
}

pub fn validate_local_archive_import_confirmation(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveImportParams,
) -> Result<(), CommandError> {
    let preview = prepare_local_archive_import(catalog, app_data_dir, ctx, params)?;
    ensure_archive_confirmation(
        &preview.action_binding,
        params.confirmed,
        params.preview_token.as_deref(),
        params.action_reference.as_ref(),
    )
}

struct LocalArchiveImportPlan {
    archive_path: PathBuf,
    archive_display_path: String,
    inspection: ArchiveInspection,
    destination_directory: PathBuf,
    action_binding: ActionPreviewBinding,
}

fn prepare_local_archive_import(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveImportParams,
) -> Result<LocalArchiveImportPlan, CommandError> {
    let archive_path = validate_archive_path(Path::new(params.archive_path.trim()))?;
    let inspection = inspect_archive(&archive_path)?;
    ensure_local_import_name_available(catalog, &inspection.skill_name)?;
    let destination_directory = tool_global_staging_skills_root(app_data_dir)
        .join(super::safe_skill_name(&inspection.skill_name)?);
    let destination_skill_path = destination_directory.join("SKILL.md");
    let target_revision = skill_tree_content_revision(&destination_skill_path)?;
    if !target_revision.ends_with(":missing") {
        return Err(CommandError::StaleActionReference);
    }
    let catalog_revision = local_import_catalog_revision(catalog, &inspection.skill_name)?;
    let action_binding = local_archive_action_binding(
        ctx,
        ActionKind::ManagerLocalArchiveImport,
        ActionIntent::ManagerLocalArchiveImport,
        ActionTargetRef {
            kind: ActionTargetKind::Skill,
            id: format!(
                "local-archive-import:{}",
                super::safe_skill_name(&inspection.skill_name)?
            ),
            agent: Some(AgentId::ToolGlobal),
            scope: Some(Scope::ToolGlobal),
        },
        None,
        "skillManager.previewLocalArchiveImport",
        "skillManager.applyLocalArchiveImport",
        &archive_path,
        &inspection.archive_sha256,
        &destination_skill_path,
        &target_revision,
        &catalog_revision,
    )?;
    Ok(LocalArchiveImportPlan {
        archive_display_path: redact_command_output(ctx, &archive_path.to_string_lossy()),
        archive_path,
        inspection,
        destination_directory,
        action_binding,
    })
}

fn local_archive_import_record(
    plan: LocalArchiveImportPlan,
    applied: bool,
    imported: Option<SkillRecord>,
    readback: Option<ActionReadbackRecord>,
) -> SkillManagerLocalArchiveImportRecord {
    SkillManagerLocalArchiveImportRecord {
        action: plan.action_binding.action,
        preconditions: plan.action_binding.preconditions,
        skill_name: plan.inspection.skill_name,
        archive_path: plan.archive_display_path,
        archive_sha256: plan.inspection.archive_sha256,
        file_count: plan.inspection.file_count,
        uncompressed_bytes: plan.inspection.uncompressed_bytes,
        preview_token: plan.action_binding.preview_token,
        confirmed: applied,
        applied,
        summary: if applied {
            "Imported the validated ZIP into the app-owned local skill library; skill scripts were not executed."
                .to_string()
        } else {
            "The ZIP contains one local skill and can be imported into the app-owned library after confirmation."
                .to_string()
        },
        instance_id: imported.as_ref().map(|record| record.id.clone()),
        imported_skill: imported,
        readback,
        follow_up: None,
    }
}

fn ensure_local_import_name_available(
    catalog: &Catalog,
    skill_name: &str,
) -> Result<(), CommandError> {
    let duplicate = catalog.list_skill_records()?.into_iter().any(|record| {
        record.name.eq_ignore_ascii_case(skill_name)
            && record.state != "missing"
            && (record.agent == AgentId::ToolGlobal.as_str()
                || is_shared_agents_skill_path(&record.path)
                || is_shared_agents_skill_path(&record.display_path))
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

struct AppliedArchiveImport {
    imported: SkillRecord,
    destination_directory: PathBuf,
    candidate_revision: String,
    readback: ActionReadbackRecord,
    active: bool,
}

impl AppliedArchiveImport {
    fn restore(&mut self) -> Result<(), CommandError> {
        if !self.active {
            return Ok(());
        }
        let skill_path = self.destination_directory.join("SKILL.md");
        if skill_tree_content_revision(&skill_path)? != self.candidate_revision {
            return Err(CommandError::StaleActionReference);
        }
        fs::remove_dir_all(&self.destination_directory)?;
        self.active = false;
        Ok(())
    }

    fn commit(&mut self) {
        self.active = false;
    }
}

fn apply_archive_import_guarded(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    plan: &LocalArchiveImportPlan,
) -> Result<AppliedArchiveImport, CommandError> {
    let skills_root = tool_global_staging_skills_root(app_data_dir);
    create_guarded_directory_chain(&skills_root)?;
    let canonical_root = skills_root.canonicalize()?;
    let destination_directory = canonical_root.join(
        plan.destination_directory
            .file_name()
            .ok_or(CommandError::StaleActionReference)?,
    );
    if destination_directory.exists() {
        return Err(CommandError::StaleActionReference);
    }
    let temp_dir = canonical_root.join(format!(
        ".archive-import-{}-{}",
        unix_timestamp_millis(),
        std::process::id()
    ));
    ensure_child_path(&canonical_root, &temp_dir)?;
    fs::create_dir(&temp_dir)?;
    if let Err(error) = extract_skill_root(&plan.archive_path, &plan.inspection, &temp_dir) {
        return Err(cleanup_staged_archive_after_error(
            "skillManager.applyLocalArchiveImport",
            &temp_dir,
            error,
        ));
    }
    let candidate_skill_path = temp_dir.join("SKILL.md");
    let candidate_revision = match skill_tree_content_revision(&candidate_skill_path) {
        Ok(revision) => revision,
        Err(error) => {
            return Err(cleanup_staged_archive_after_error(
                "skillManager.applyLocalArchiveImport",
                &temp_dir,
                error,
            ));
        }
    };
    if candidate_revision != plan.inspection.tree_revision {
        return Err(cleanup_staged_archive_after_error(
            "skillManager.applyLocalArchiveImport",
            &temp_dir,
            CommandError::VerificationFailed,
        ));
    }
    if let Err(error) = fs::rename(&temp_dir, &destination_directory) {
        return Err(cleanup_staged_archive_after_error(
            "skillManager.applyLocalArchiveImport",
            &temp_dir,
            error.into(),
        ));
    }
    let applied = (|| -> Result<(SkillRecord, ActionReadbackRecord), CommandError> {
        let destination_skill_path = destination_directory.join("SKILL.md").canonicalize()?;
        let imported = register_tool_global_staged_skill(
            catalog,
            ctx,
            &plan.archive_path,
            &destination_skill_path,
        )?
        .imported;
        if imported.path != destination_skill_path
            || !imported
                .name
                .eq_ignore_ascii_case(&plan.inspection.skill_name)
            || skill_tree_content_revision(&destination_skill_path)? != candidate_revision
        {
            return Err(CommandError::VerificationFailed);
        }
        let readback = ActionReadbackRecord::verified(
            &plan.action_binding.action,
            vec![
                ActionReadbackObservation {
                    domain: ActionReadbackDomain::SkillFiles,
                    target_id: destination_skill_path.to_string_lossy().to_string(),
                    revision: candidate_revision.clone(),
                },
                ActionReadbackObservation {
                    domain: ActionReadbackDomain::CatalogSkills,
                    target_id: imported.id.clone(),
                    revision: archive_catalog_record_revision(catalog, &imported.id)?,
                },
            ],
        )?;
        Ok((imported, readback))
    })();
    match applied {
        Ok((imported, readback)) => Ok(AppliedArchiveImport {
            imported,
            destination_directory,
            candidate_revision,
            readback,
            active: true,
        }),
        Err(error) => {
            let current_revision =
                skill_tree_content_revision(&destination_directory.join("SKILL.md")).map_err(
                    |revision_error| {
                        archive_partial(
                            "skillManager.applyLocalArchiveImport",
                            "outcome_unknown",
                            true,
                            format!(
                                "catalog registration failed ({error}); imported target could not be inspected for rollback ({revision_error})"
                            ),
                        )
                    },
                )?;
            if current_revision != candidate_revision {
                return Err(archive_partial(
                    "skillManager.applyLocalArchiveImport",
                    "outcome_unknown",
                    true,
                    "the imported target changed before rollback".to_string(),
                ));
            }
            fs::remove_dir_all(&destination_directory).map_err(|cleanup_error| {
                archive_partial(
                    "skillManager.applyLocalArchiveImport",
                    "outcome_unknown",
                    true,
                    format!(
                        "catalog registration failed ({error}); imported target rollback failed ({cleanup_error})"
                    ),
                )
            })?;
            Err(error)
        }
    }
}

pub fn preview_local_archive_update(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveUpdateParams,
) -> Result<SkillManagerLocalArchiveUpdateRecord, CommandError> {
    let plan = prepare_local_archive_update(catalog, app_data_dir, ctx, params)?;
    Ok(local_archive_update_record(plan, false, None, None, None))
}

pub fn apply_local_archive_update(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveUpdateParams,
) -> Result<SkillManagerLocalArchiveUpdateRecord, CommandError> {
    let preview = prepare_local_archive_update(catalog, app_data_dir, ctx, params)?;
    ensure_archive_confirmation(
        &preview.action_binding,
        params.confirmed,
        params.preview_token.as_deref(),
        params.action_reference.as_ref(),
    )?;
    let _owner = crate::mutation_lock::lock_app_mutations(app_data_dir)?;
    let locked = prepare_local_archive_update(catalog, app_data_dir, ctx, params)?;
    ensure_archive_confirmation(
        &locked.action_binding,
        params.confirmed,
        params.preview_token.as_deref(),
        params.action_reference.as_ref(),
    )?;
    let transaction = catalog.begin_immediate_transaction()?;
    let mut applied = apply_archive_replacement_guarded(catalog, ctx, &locked)?;
    let mut follow_up = None;
    let record = local_archive_update_record(
        locked,
        true,
        Some(applied.updated_skill.clone()),
        Some(applied.readback.clone()),
        None,
    );
    if let Err(error) = transaction.commit() {
        applied.restore().map_err(|cleanup_error| {
            archive_partial(
                "skillManager.applyLocalArchiveUpdate",
                "outcome_unknown",
                true,
                format!(
                    "catalog commit failed ({error}); source restoration failed ({cleanup_error})"
                ),
            )
        })?;
        return Err(archive_partial(
            "skillManager.applyLocalArchiveUpdate",
            "applied_unverified",
            false,
            format!("catalog commit failed after local archive update: {error}"),
        ));
    }
    if applied.finish().is_err() {
        follow_up = Some(super::SkillManagerCleanupFollowUp {
            kind: "quarantine_cleanup".to_string(),
            state: "update_applied_cleanup_pending".to_string(),
            cleanup_required: true,
            message:
                "The local skill update was applied and verified, but private backup cleanup remains pending."
                    .to_string(),
        });
    }
    Ok(SkillManagerLocalArchiveUpdateRecord {
        follow_up,
        ..record
    })
}

pub fn validate_local_archive_update_confirmation(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveUpdateParams,
) -> Result<(), CommandError> {
    let preview = prepare_local_archive_update(catalog, app_data_dir, ctx, params)?;
    ensure_archive_confirmation(
        &preview.action_binding,
        params.confirmed,
        params.preview_token.as_deref(),
        params.action_reference.as_ref(),
    )
}

struct LocalArchiveUpdatePlan {
    target: LocalArchiveUpdateTarget,
    archive_path: PathBuf,
    archive_display_path: String,
    inspection: ArchiveInspection,
    target_tree_revision: String,
    action_binding: ActionPreviewBinding,
}

fn prepare_local_archive_update(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalArchiveUpdateParams,
) -> Result<LocalArchiveUpdatePlan, CommandError> {
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
    let target_tree_revision = skill_tree_content_revision(&target.skill_path)?;
    if target_tree_revision == inspection.tree_revision {
        return Err(CommandError::NoApplicableAction(
            "the selected ZIP is identical to the complete current skill tree".to_string(),
        ));
    }
    let project_id = if target.scope == Scope::AgentProject {
        canonical_project_id(ctx.project_root.as_deref())
    } else {
        None
    };
    let action_binding = local_archive_action_binding(
        ctx,
        ActionKind::ManagerLocalArchiveUpdate,
        ActionIntent::ManagerLocalArchiveUpdate,
        ActionTargetRef {
            kind: ActionTargetKind::Skill,
            id: target.instance_id.clone(),
            agent: Some(target.agent),
            scope: Some(target.scope),
        },
        project_id,
        "skillManager.previewLocalArchiveUpdate",
        "skillManager.applyLocalArchiveUpdate",
        &archive_path,
        &inspection.archive_sha256,
        &target.skill_path,
        &target_tree_revision,
        &target.catalog_revision,
    )?;
    Ok(LocalArchiveUpdatePlan {
        target,
        archive_display_path: redact_command_output(ctx, &archive_path.to_string_lossy()),
        archive_path,
        inspection,
        target_tree_revision,
        action_binding,
    })
}

fn local_archive_update_record(
    plan: LocalArchiveUpdatePlan,
    applied: bool,
    updated_skill: Option<SkillRecord>,
    readback: Option<ActionReadbackRecord>,
    follow_up: Option<super::SkillManagerCleanupFollowUp>,
) -> SkillManagerLocalArchiveUpdateRecord {
    SkillManagerLocalArchiveUpdateRecord {
        action: plan.action_binding.action,
        preconditions: plan.action_binding.preconditions,
        instance_id: plan.target.instance_id,
        skill_name: plan.target.skill_name,
        archive_path: plan.archive_display_path,
        archive_sha256: plan.inspection.archive_sha256,
        file_count: plan.inspection.file_count,
        uncompressed_bytes: plan.inspection.uncompressed_bytes,
        preview_token: plan.action_binding.preview_token,
        confirmed: applied,
        applied,
        summary: if applied {
            "Replaced the selected local skill source from the confirmed ZIP; skill scripts were not executed."
                .to_string()
        } else {
            "The ZIP contains one matching local skill and can replace the selected local source after confirmation."
                .to_string()
        },
        updated_skill,
        readback,
        follow_up,
    }
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
    let records = catalog.list_skill_records()?;
    let references = local_delete_references(&meta, &records);
    let catalog_revision = local_delete_catalog_revision(&meta, &canonical_path, &references)?;
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
            instance_id: meta.id,
            skill_name: meta.name,
            skill_path: canonical_path,
            canonical_root,
            agent: meta.agent,
            scope: meta.scope,
            catalog_revision,
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
        instance_id: meta.id.clone(),
        skill_name: meta.name,
        skill_path: canonical_path,
        canonical_root,
        agent: meta.agent,
        scope: meta.scope,
        catalog_revision,
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
    let mut file_digests = Vec::new();
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
        let mut content = Vec::new();
        entry
            .by_ref()
            .take(MAX_ARCHIVE_FILE_BYTES + 1)
            .read_to_end(&mut content)?;
        if content.len() as u64 != entry.size() || content.len() as u64 > MAX_ARCHIVE_FILE_BYTES {
            return Err(invalid_archive(
                "ZIP entry size changed or exceeded its safe bound while inspecting",
            ));
        }
        let digest = format!("{:x}", Sha256::digest(&content));
        file_digests.push((path.clone(), content.len() as u64, digest));
        if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md") {
            if entry.size() > MAX_SKILL_MD_BYTES {
                return Err(invalid_archive("SKILL.md exceeds 2 MiB"));
            }
            let content = String::from_utf8(content)
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
    let tree_revision = archive_tree_revision(&file_digests, &skill_root)?;
    Ok(ArchiveInspection {
        archive_sha256,
        skill_name,
        skill_root,
        file_count,
        uncompressed_bytes,
        tree_revision,
    })
}

struct AppliedArchiveUpdate {
    updated_skill: SkillRecord,
    target_directory: PathBuf,
    backup_directory: PathBuf,
    target_skill_path: PathBuf,
    original_tree_revision: String,
    candidate_tree_revision: String,
    readback: ActionReadbackRecord,
    active: bool,
}

impl AppliedArchiveUpdate {
    fn restore(&mut self) -> Result<(), CommandError> {
        if !self.active {
            return Ok(());
        }
        if skill_tree_content_revision(&self.target_skill_path)? != self.candidate_tree_revision {
            return Err(CommandError::StaleActionReference);
        }
        fs::remove_dir_all(&self.target_directory)?;
        fs::rename(&self.backup_directory, &self.target_directory)?;
        if skill_tree_content_revision(&self.target_skill_path)? != self.original_tree_revision {
            return Err(CommandError::VerificationFailed);
        }
        self.active = false;
        Ok(())
    }

    fn finish(&mut self) -> Result<(), CommandError> {
        if !self.active {
            return Ok(());
        }
        fs::remove_dir_all(&self.backup_directory)?;
        self.active = false;
        Ok(())
    }
}

fn apply_archive_replacement_guarded(
    catalog: &Catalog,
    ctx: &AdapterContext,
    plan: &LocalArchiveUpdatePlan,
) -> Result<AppliedArchiveUpdate, CommandError> {
    let existing_skill_path = &plan.target.skill_path;
    let canonical_root = &plan.target.canonical_root;
    let nonce = format!("{}-{}", unix_timestamp_millis(), std::process::id());
    let temp_dir = canonical_root.join(format!(".archive-update-{nonce}"));
    let backup_dir = canonical_root.join(format!(".archive-backup-{nonce}"));
    ensure_child_path(canonical_root, &temp_dir)?;
    ensure_child_path(canonical_root, &backup_dir)?;
    fs::create_dir(&temp_dir)?;
    let extraction = extract_skill_root(&plan.archive_path, &plan.inspection, &temp_dir);
    if let Err(error) = extraction {
        return Err(cleanup_staged_archive_after_error(
            "skillManager.applyLocalArchiveUpdate",
            &temp_dir,
            error,
        ));
    }

    let replacement_skill_path = temp_dir.join("SKILL.md");
    let replacement_content = match fs::read_to_string(&replacement_skill_path) {
        Ok(content) => content,
        Err(error) => {
            return Err(cleanup_staged_archive_after_error(
                "skillManager.applyLocalArchiveUpdate",
                &temp_dir,
                error.into(),
            ));
        }
    };
    let replacement_name = tool_global_skill_name_from_content(
        &replacement_content,
        existing_skill_path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("local-skill"),
    );
    if !replacement_name.eq_ignore_ascii_case(&plan.inspection.skill_name) {
        return Err(cleanup_staged_archive_after_error(
            "skillManager.applyLocalArchiveUpdate",
            &temp_dir,
            invalid_archive("ZIP skill identity changed during extraction"),
        ));
    }

    let target_dir = match existing_skill_path.parent() {
        Some(target_dir) => target_dir,
        None => {
            return Err(cleanup_staged_archive_after_error(
                "skillManager.applyLocalArchiveUpdate",
                &temp_dir,
                CommandError::InvalidSkillManagerRequest(
                    "local skill source has no parent directory".to_string(),
                ),
            ));
        }
    };
    if let Err(error) = ensure_child_path(canonical_root, target_dir) {
        return Err(cleanup_staged_archive_after_error(
            "skillManager.applyLocalArchiveUpdate",
            &temp_dir,
            error,
        ));
    }
    let candidate_tree_revision = match skill_tree_content_revision(&replacement_skill_path) {
        Ok(revision) => revision,
        Err(error) => {
            return Err(cleanup_staged_archive_after_error(
                "skillManager.applyLocalArchiveUpdate",
                &temp_dir,
                error,
            ));
        }
    };
    let current_tree_revision = match skill_tree_content_revision(existing_skill_path) {
        Ok(revision) => revision,
        Err(error) => {
            return Err(cleanup_staged_archive_after_error(
                "skillManager.applyLocalArchiveUpdate",
                &temp_dir,
                error,
            ));
        }
    };
    if candidate_tree_revision != plan.inspection.tree_revision
        || current_tree_revision != plan.target_tree_revision
    {
        return Err(cleanup_staged_archive_after_error(
            "skillManager.applyLocalArchiveUpdate",
            &temp_dir,
            CommandError::StaleActionReference,
        ));
    }
    if let Err(error) = fs::rename(target_dir, &backup_dir) {
        return Err(cleanup_staged_archive_after_error(
            "skillManager.applyLocalArchiveUpdate",
            &temp_dir,
            error.into(),
        ));
    }
    if let Err(error) = fs::rename(&temp_dir, target_dir) {
        let restore_result = fs::rename(&backup_dir, target_dir);
        let staged_cleanup = fs::remove_dir_all(&temp_dir);
        let restored_revision = restore_result
            .as_ref()
            .ok()
            .and_then(|_| skill_tree_content_revision(existing_skill_path).ok());
        if restore_result.is_err()
            || staged_cleanup
                .as_ref()
                .is_err_and(|cleanup| cleanup.kind() != std::io::ErrorKind::NotFound)
            || restored_revision.as_deref() != Some(plan.target_tree_revision.as_str())
        {
            return Err(archive_partial(
                "skillManager.applyLocalArchiveUpdate",
                "outcome_unknown",
                true,
                format!(
                    "replacement activation failed ({error}); exact original-tree restoration or private staging cleanup could not be proved"
                ),
            ));
        }
        return Err(error.into());
    }

    let registered = (|| {
        let staged_skill_path = target_dir.join("SKILL.md").canonicalize()?;
        let updated = match &plan.target.kind {
            LocalArchiveUpdateTargetKind::AppOwned => Ok(register_tool_global_staged_skill(
                catalog,
                ctx,
                &plan.archive_path,
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
                                record.name.eq_ignore_ascii_case(&plan.target.skill_name)
                                    && record.path == staged_skill_path
                            })
                    })
                    .ok_or_else(|| CommandError::InstanceNotFound(instance_id.clone()))
            }
        }?;
        if updated.id != plan.target.instance_id
            || updated.path != staged_skill_path
            || !updated.name.eq_ignore_ascii_case(&plan.target.skill_name)
            || skill_tree_content_revision(&staged_skill_path)? != candidate_tree_revision
        {
            return Err(CommandError::VerificationFailed);
        }
        let readback = ActionReadbackRecord::verified(
            &plan.action_binding.action,
            vec![
                ActionReadbackObservation {
                    domain: ActionReadbackDomain::SkillFiles,
                    target_id: staged_skill_path.to_string_lossy().to_string(),
                    revision: candidate_tree_revision.clone(),
                },
                ActionReadbackObservation {
                    domain: ActionReadbackDomain::CatalogSkills,
                    target_id: updated.id.clone(),
                    revision: archive_catalog_record_revision(catalog, &updated.id)?,
                },
            ],
        )?;
        Ok((updated, readback))
    })();
    match registered {
        Ok((updated_skill, readback)) => Ok(AppliedArchiveUpdate {
            updated_skill,
            target_directory: target_dir.to_path_buf(),
            backup_directory: backup_dir,
            target_skill_path: target_dir.join("SKILL.md"),
            original_tree_revision: plan.target_tree_revision.clone(),
            candidate_tree_revision,
            readback,
            active: true,
        }),
        Err(error) => {
            let current_revision =
                skill_tree_content_revision(&target_dir.join("SKILL.md")).map_err(
                    |revision_error| {
                        archive_partial(
                            "skillManager.applyLocalArchiveUpdate",
                            "outcome_unknown",
                            true,
                            format!(
                                "catalog projection failed ({error}); replacement tree could not be inspected for rollback ({revision_error})"
                            ),
                        )
                    },
                )?;
            if current_revision != candidate_tree_revision {
                return Err(archive_partial(
                    "skillManager.applyLocalArchiveUpdate",
                    "outcome_unknown",
                    true,
                    format!(
                        "catalog projection failed ({error}) and the replacement tree changed before rollback"
                    ),
                ));
            }
            fs::remove_dir_all(target_dir).map_err(|cleanup_error| {
                archive_partial(
                    "skillManager.applyLocalArchiveUpdate",
                    "outcome_unknown",
                    true,
                    format!(
                        "catalog projection failed ({error}); replacement cleanup failed ({cleanup_error})"
                    ),
                )
            })?;
            fs::rename(&backup_dir, target_dir).map_err(|restore_error| {
                archive_partial(
                    "skillManager.applyLocalArchiveUpdate",
                    "outcome_unknown",
                    true,
                    format!(
                        "catalog projection failed ({error}); original tree restoration failed ({restore_error})"
                    ),
                )
            })?;
            let restored_revision = skill_tree_content_revision(existing_skill_path).map_err(
                |revision_error| {
                    archive_partial(
                        "skillManager.applyLocalArchiveUpdate",
                        "outcome_unknown",
                        true,
                        format!(
                            "catalog projection failed ({error}); restored tree could not be verified ({revision_error})"
                        ),
                    )
                },
            )?;
            if restored_revision != plan.target_tree_revision {
                return Err(archive_partial(
                    "skillManager.applyLocalArchiveUpdate",
                    "outcome_unknown",
                    true,
                    format!(
                        "catalog projection failed ({error}) and the original tree could not be verified after restoration"
                    ),
                ));
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

#[allow(clippy::too_many_arguments)]
fn local_archive_action_binding(
    _ctx: &AdapterContext,
    kind: ActionKind,
    intent: ActionIntent,
    target: ActionTargetRef,
    project_id: Option<String>,
    preview_method: &str,
    apply_method: &str,
    archive_path: &Path,
    archive_revision: &str,
    target_skill_path: &Path,
    target_tree_revision: &str,
    catalog_revision: &str,
) -> Result<ActionPreviewBinding, CommandError> {
    let evidence_refs = target
        .agent
        .map(crate::coverage_projection_evidence_id)
        .into_iter()
        .collect();
    let source_revision = action_source_revision(
        "manager.local-archive.accepted-snapshot",
        &[
            ("archive_revision", archive_revision),
            ("target_tree_revision", target_tree_revision),
            ("catalog_revision", catalog_revision),
        ],
    )?;
    let descriptor = action_descriptor(
        kind,
        intent,
        target,
        project_id,
        vec![ActionImpact::SkillFiles, ActionImpact::AppLocalData],
        preview_method,
        Some(apply_method),
        source_revision,
        true,
        ActionNetworkPosture::None,
        canonical_readback_domains([
            ActionReadbackDomain::SkillFiles,
            ActionReadbackDomain::CatalogSkills,
        ]),
        evidence_refs,
    )?;
    action_preview_binding(
        descriptor.clone(),
        vec![
            ActionPrecondition {
                kind: ActionPreconditionKind::Archive,
                target_id: archive_path.to_string_lossy().to_string(),
                expected_revision: archive_revision.to_string(),
            },
            ActionPrecondition {
                kind: ActionPreconditionKind::TargetFile,
                target_id: target_skill_path.to_string_lossy().to_string(),
                expected_revision: target_tree_revision.to_string(),
            },
            ActionPrecondition {
                kind: ActionPreconditionKind::CatalogRecord,
                target_id: descriptor.target.id,
                expected_revision: catalog_revision.to_string(),
            },
        ],
    )
}

fn ensure_archive_confirmation(
    binding: &ActionPreviewBinding,
    confirmed: bool,
    preview_token: Option<&str>,
    action_reference: Option<&ActionReference>,
) -> Result<(), CommandError> {
    let token = preview_token.ok_or_else(|| {
        CommandError::ActionConfirmationRequired(
            "local archive apply requires the exact preview_token".to_string(),
        )
    })?;
    let reference = action_reference.ok_or_else(|| {
        CommandError::ActionConfirmationRequired(
            "local archive apply requires the preview action_reference".to_string(),
        )
    })?;
    ensure_action_confirmed(
        binding,
        Some(&ActionConfirmation {
            reference: reference.clone(),
            preview_token: token.to_string(),
            confirmed,
        }),
    )
}

fn local_import_catalog_revision(
    catalog: &Catalog,
    skill_name: &str,
) -> Result<String, CommandError> {
    let mut records = catalog
        .list_skill_records()?
        .into_iter()
        .filter(|record| {
            record.name.eq_ignore_ascii_case(skill_name)
                && record.state != "missing"
                && (record.agent == AgentId::ToolGlobal.as_str()
                    || is_shared_agents_skill_path(&record.path))
        })
        .map(|record| {
            (
                record.id,
                record.agent,
                record.scope,
                record.path.to_string_lossy().to_string(),
                record.definition_id,
                record.state,
                record.enabled,
            )
        })
        .collect::<Vec<_>>();
    records.sort();
    action_source_revision(
        "manager.local-archive.import-catalog",
        &[
            ("skill_name", skill_name),
            ("records", &serde_json::to_string(&records)?),
        ],
    )
}

fn archive_catalog_record_revision(
    catalog: &Catalog,
    instance_id: &str,
) -> Result<String, CommandError> {
    let record = catalog
        .get_skill_record(instance_id)?
        .ok_or(CommandError::VerificationFailed)?;
    let detail = catalog
        .get_skill_detail(instance_id)?
        .ok_or(CommandError::VerificationFailed)?;
    action_source_revision(
        "manager.local-archive.catalog-readback",
        &[
            ("instance_id", &record.id),
            ("agent", &record.agent),
            ("scope", &record.scope),
            ("name", &record.name),
            ("definition_id", &record.definition_id),
            ("fingerprint", &detail.fingerprint),
            ("path", &record.path.to_string_lossy()),
        ],
    )
}

fn archive_tree_revision(
    files: &[(PathBuf, u64, String)],
    skill_root: &Path,
) -> Result<String, CommandError> {
    let mut rows = BTreeSet::new();
    for (path, size, digest) in files {
        let relative = if skill_root.as_os_str().is_empty() {
            path.as_path()
        } else {
            path.strip_prefix(skill_root)
                .map_err(|_| invalid_archive("ZIP file escaped its skill root"))?
        };
        for ancestor in relative.ancestors().skip(1) {
            if !ancestor.as_os_str().is_empty() {
                rows.insert(format!("dir:{}", ancestor.to_string_lossy()));
            }
        }
        rows.insert(format!(
            "file:{}:{size}:{digest}",
            relative.to_string_lossy()
        ));
    }
    present_tree_revision(rows.into_iter().collect())
}

fn skill_tree_content_revision(skill_path: &Path) -> Result<String, CommandError> {
    let root = skill_path.parent().ok_or_else(|| {
        CommandError::UnsafeConfigPath("local archive target has no parent".to_string())
    })?;
    let metadata = match fs::symlink_metadata(root) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(missing_tree_revision())
        }
        Err(error) => return Err(error.into()),
    };
    if metadata.file_type().is_symlink() || !metadata.is_dir() {
        return Err(CommandError::UnsafeConfigPath(
            "local archive target directory must be a regular directory".to_string(),
        ));
    }
    let mut pending = vec![root.to_path_buf()];
    let mut rows = BTreeSet::new();
    let mut entry_count = 0usize;
    let mut total_bytes = 0u64;
    while let Some(directory) = pending.pop() {
        let mut children = fs::read_dir(&directory)?.collect::<Result<Vec<_>, _>>()?;
        children.sort_by_key(|entry| entry.file_name());
        for child in children {
            entry_count = entry_count.saturating_add(1);
            if entry_count > MAX_ARCHIVE_ENTRIES {
                return Err(CommandError::InvalidSkillManagerRequest(
                    "local skill tree exceeds the 2000-entry update safety budget".to_string(),
                ));
            }
            let path = child.path();
            let metadata = fs::symlink_metadata(&path)?;
            let relative = path
                .strip_prefix(root)
                .map_err(|_| {
                    CommandError::UnsafeConfigPath(
                        "local archive target escaped its skill root".to_string(),
                    )
                })?
                .to_string_lossy()
                .to_string();
            if metadata.file_type().is_symlink() {
                return Err(CommandError::UnsafeConfigPath(
                    "local archive replacement refuses symlinked target content".to_string(),
                ));
            }
            if metadata.is_dir() {
                rows.insert(format!("dir:{relative}"));
                pending.push(path);
            } else if metadata.is_file() {
                if metadata.len() > MAX_ARCHIVE_FILE_BYTES {
                    return Err(CommandError::InvalidSkillManagerRequest(
                        "local skill tree contains a file larger than 8 MiB".to_string(),
                    ));
                }
                total_bytes = total_bytes
                    .checked_add(metadata.len())
                    .filter(|total| *total <= MAX_ARCHIVE_UNCOMPRESSED_BYTES)
                    .ok_or_else(|| {
                        CommandError::InvalidSkillManagerRequest(
                            "local skill tree exceeds the 64 MiB update safety budget".to_string(),
                        )
                    })?;
                rows.insert(format!(
                    "file:{relative}:{}:{:x}",
                    metadata.len(),
                    Sha256::digest(fs::read(&path)?)
                ));
            } else {
                return Err(CommandError::UnsafeConfigPath(
                    "local archive replacement refuses special target files".to_string(),
                ));
            }
        }
    }
    present_tree_revision(rows.into_iter().collect())
}

fn present_tree_revision(rows: Vec<String>) -> Result<String, CommandError> {
    Ok(format!(
        "tree:present:{}",
        action_source_revision(
            "manager.local-archive.tree",
            &[("entries", &serde_json::to_string(&rows)?)],
        )?
        .trim_start_matches("sha256:")
    ))
}

fn missing_tree_revision() -> String {
    "tree:missing".to_string()
}

fn create_guarded_directory_chain(path: &Path) -> Result<(), CommandError> {
    if path.exists() {
        let metadata = fs::symlink_metadata(path)?;
        if metadata.file_type().is_symlink() || !metadata.is_dir() {
            return Err(CommandError::UnsafeConfigPath(
                "local archive destination root must be a regular directory".to_string(),
            ));
        }
        return Ok(());
    }
    let parent = path.parent().ok_or_else(|| {
        CommandError::UnsafeConfigPath("local archive destination has no parent".to_string())
    })?;
    create_guarded_directory_chain(parent)?;
    fs::create_dir(path)?;
    Ok(())
}

fn cleanup_staged_archive_after_error(
    operation: &str,
    staged_directory: &Path,
    error: CommandError,
) -> CommandError {
    match fs::remove_dir_all(staged_directory) {
        Ok(()) => error,
        Err(cleanup_error) if cleanup_error.kind() == std::io::ErrorKind::NotFound => error,
        Err(cleanup_error) => archive_partial(
            operation,
            "not_started",
            true,
            format!(
                "archive staging failed ({error}); private staging cleanup failed ({cleanup_error})"
            ),
        ),
    }
}

fn archive_partial(
    operation: &str,
    state: &'static str,
    cleanup_required: bool,
    detail: String,
) -> CommandError {
    CommandError::PartialEffect {
        operation: operation.to_string(),
        state,
        cleanup_required,
        detail,
    }
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
        crate::initialize_action_preview_secret_for_test([0xA5; 32])
            .expect("shared action preview secret");
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
        fs::create_dir_all(root.join("app-data")).expect("app data");
        let preview = preview_local_archive_import(
            &catalog,
            &root.join("app-data"),
            &ctx,
            &SkillManagerLocalArchiveImportParams {
                archive_path: archive.to_string_lossy().to_string(),
                confirmed: false,
                preview_token: None,
                action_reference: None,
            },
        )
        .expect("import preview");
        let confirmation = SkillManagerLocalArchiveImportParams {
            archive_path: archive.to_string_lossy().to_string(),
            confirmed: true,
            preview_token: Some(preview.preview_token.clone()),
            action_reference: Some(ActionReference::from(&preview.action)),
        };
        let applied =
            apply_local_archive_import(&catalog, &root.join("app-data"), &ctx, &confirmation)
                .expect("confirmed import");

        assert!(applied.applied);
        assert!(applied
            .readback
            .as_ref()
            .is_some_and(|readback| readback.verified));
        let imported = applied.imported_skill.expect("imported skill");
        assert_eq!(imported.agent, AgentId::ToolGlobal.as_str());
        assert_eq!(imported.name, "import-helper");
        assert!(Path::new(&imported.path).is_file());
        let replay =
            apply_local_archive_import(&catalog, &root.join("app-data"), &ctx, &confirmation)
                .expect_err("one confirmation must not import a second package");
        assert!(matches!(
            replay,
            CommandError::InvalidSkillManagerRequest(_) | CommandError::StaleActionReference
        ));
        assert_eq!(
            catalog
                .list_skill_records()
                .expect("catalog records")
                .into_iter()
                .filter(|record| {
                    record.agent == AgentId::ToolGlobal.as_str()
                        && record.name == "import-helper"
                        && record.state != "missing"
                })
                .count(),
            1
        );

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
                action_reference: None,
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
        fs::create_dir_all(root.join("app-data")).expect("app data");
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
                action_reference: None,
            },
        )
        .expect("project local update preview");
        let confirmation = SkillManagerLocalArchiveUpdateParams {
            instance_id: instance.id,
            archive_path: archive.to_string_lossy().to_string(),
            confirmed: true,
            preview_token: Some(preview.preview_token.clone()),
            action_reference: Some(ActionReference::from(&preview.action)),
        };
        let applied =
            apply_local_archive_update(&catalog, &root.join("app-data"), &ctx, &confirmation)
                .expect("confirmed project local update");

        assert!(applied.applied);
        assert!(applied
            .readback
            .as_ref()
            .is_some_and(|readback| readback.verified));
        let updated = fs::read_to_string(skill_dir.join("SKILL.md")).expect("updated skill");
        assert!(updated.contains("Updated locally"));
        assert!(!updated.contains("Original local"));
        let replay =
            apply_local_archive_update(&catalog, &root.join("app-data"), &ctx, &confirmation)
                .expect_err("one confirmation must not replace the same tree twice");
        assert!(matches!(replay, CommandError::NoApplicableAction(_)));

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
                action_reference: None,
            },
        )
        .expect("nested global update preview");

        assert_eq!(preview.skill_name, "nested-helper");
        assert!(!preview.applied);

        fs::remove_file(archive).ok();
        fs::remove_dir_all(root).ok();
    }

    #[test]
    fn archive_update_rejects_attachment_drift_without_replacing_any_current_file() {
        let archive = write_archive(
            "attachment-drift",
            &[
                (
                    "drift-helper/SKILL.md",
                    "---\nname: drift-helper\ndescription: Candidate\n---\n# Candidate",
                ),
                ("drift-helper/references/guide.md", "candidate guide"),
            ],
        );
        let root = std::env::temp_dir().join(format!(
            "skill-manager-archive-drift-{}-{}",
            std::process::id(),
            unix_timestamp_millis()
        ));
        let home = root.join("home");
        let project = root.join("project");
        let skill_dir = project.join(".agents/skills/drift-helper");
        fs::create_dir_all(skill_dir.join("references")).expect("skill tree");
        fs::create_dir_all(&home).expect("home");
        fs::create_dir_all(root.join("app-data")).expect("app data");
        fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: drift-helper\ndescription: Original\n---\n# Original",
        )
        .expect("original skill");
        fs::write(skill_dir.join("references/guide.md"), "reviewed guide")
            .expect("original attachment");
        let catalog = Catalog::open(&root.join("catalog.sqlite")).expect("catalog");
        catalog.init().expect("catalog schema");
        let ctx = AdapterContext {
            user_home: home,
            project_root: Some(project.clone()),
            project_cwd: Some(project),
            extra_roots: Vec::new(),
        };
        scan_all_catalog_report(&ctx, &catalog).expect("initial scan");
        let skill_path = skill_dir
            .join("SKILL.md")
            .canonicalize()
            .expect("canonical skill");
        let instance = catalog
            .list_skill_records()
            .expect("records")
            .into_iter()
            .find(|record| record.name == "drift-helper" && record.path == skill_path)
            .expect("skill record");
        let preview = preview_local_archive_update(
            &catalog,
            &root.join("app-data"),
            &ctx,
            &SkillManagerLocalArchiveUpdateParams {
                instance_id: instance.id.clone(),
                archive_path: archive.to_string_lossy().to_string(),
                confirmed: false,
                preview_token: None,
                action_reference: None,
            },
        )
        .expect("preview");

        fs::write(skill_dir.join("references/guide.md"), "third state")
            .expect("external attachment drift");
        let error = apply_local_archive_update(
            &catalog,
            &root.join("app-data"),
            &ctx,
            &SkillManagerLocalArchiveUpdateParams {
                instance_id: instance.id,
                archive_path: archive.to_string_lossy().to_string(),
                confirmed: true,
                preview_token: Some(preview.preview_token.clone()),
                action_reference: Some(ActionReference::from(&preview.action)),
            },
        )
        .expect_err("attachment drift must stale the complete-tree preview");

        assert!(matches!(error, CommandError::StaleActionReference));
        assert!(fs::read_to_string(skill_dir.join("SKILL.md"))
            .expect("preserved skill")
            .contains("# Original"));
        assert_eq!(
            fs::read_to_string(skill_dir.join("references/guide.md"))
                .expect("preserved third state"),
            "third state"
        );

        fs::remove_file(archive).ok();
        fs::remove_dir_all(root).ok();
    }
}
