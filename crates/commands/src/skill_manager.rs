use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    fs::{File, OpenOptions},
    io::{Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use skills_copilot_adapters::{
    claude_config_dir, hermes_home_dir, openclaw_state_dir, opencode_user_skills_dir, pi_agent_dir,
};
use skills_copilot_catalog::{Catalog, CatalogCommitError, SkillEventDraft, SkillRecord};
use skills_copilot_core::{
    ActionDescriptor, ActionImpact, ActionIntent, ActionKind, ActionNetworkPosture,
    ActionReadbackDomain, ActionTargetKind, ActionTargetRef, AdapterContext, AgentId,
    ListIncompleteReason, ListPageMetadata, ListSourceCompleteness, Scope,
};

use crate::{
    action_descriptor, action_preview_binding, action_source_revision, canonical_project_id,
    canonical_readback_domains, coverage_projection_evidence_id, ensure_action_confirmed,
    import_local_skill_to_tool_global, scan_all_catalog_report, tool_global_staging_skills_root,
    transaction_lifecycle::rollback_catalog_before_compensation, ActionConfirmation,
    ActionPrecondition, ActionPreconditionKind, ActionPreviewBinding, ActionReadbackObservation,
    ActionReadbackRecord, ActionReference, CommandError,
};

mod archive;
mod commit_outcome;
mod composite_remove;
mod discovery;
pub use archive::{
    apply_local_archive_import, apply_local_archive_update, preview_local_archive_import,
    preview_local_archive_update, validate_local_archive_import_confirmation,
    validate_local_archive_update_confirmation, SkillManagerLocalArchiveImportParams,
    SkillManagerLocalArchiveImportRecord, SkillManagerLocalArchiveUpdateParams,
    SkillManagerLocalArchiveUpdateRecord,
};
use commit_outcome::{
    commit_manager_catalog_transaction, manager_post_process_error,
    rollback_manager_catalog_transaction,
};
use composite_remove::{
    apply_composite_local_delete, bind_composite_local_delete, commit_composite_local_delete,
    composite_local_delete_plan, rollback_composite_local_delete, CompositeLocalDeleteCommit,
};
pub use discovery::{
    apply_search_skills_with_manager, list_installed_skills_from_projection,
    preview_search_skills_with_manager,
};

const DEFAULT_MANAGER_TOOL: &str = "npx-skills";
const SKILLS_NPM_TOOL: &str = "skills-npm";
const SKILLS_CLI_BINARY: &str = "skills";
const NPX_BINARY: &str = "npx";
const MAX_CAPTURE_BYTES: usize = 32_000;
const MAX_MACHINE_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_MANAGER_LOCK_BYTES: u64 = 2 * 1024 * 1024;
const MAX_MANAGER_TARGET_ENTRIES: usize = 16_384;
const MAX_MANAGER_TARGET_BYTES: u64 = 128 * 1024 * 1024;
const MAX_MANAGER_SEARCH_QUERY_BYTES: usize = 512;
const MAX_MANAGER_SEARCH_OWNER_BYTES: usize = 256;

pub const SUPPORTED_MANAGER_AGENTS: [&str; 6] = [
    "claude-code",
    "pi",
    "opencode",
    "codex",
    "hermes-agent",
    "openclaw",
];

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerToolRecord {
    pub id: String,
    pub display_name: String,
    pub status: String,
    pub executable: Option<String>,
    pub operations: Vec<String>,
    pub default_agents: Vec<String>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerCommandPreview {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preconditions: Vec<ActionPrecondition>,
    pub tool_id: String,
    pub operation: String,
    pub command: Vec<String>,
    pub cwd: String,
    pub env: Vec<SkillManagerEnvPreview>,
    pub requires_confirmation: bool,
    pub confirmed: bool,
    pub network_required: bool,
    pub network_allowed: bool,
    pub will_run: bool,
    pub preview_token: String,
    pub summary: String,
    pub risks: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source: Option<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub skills: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerEnvPreview {
    pub key: String,
    pub value: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerCommandOutput {
    pub status: String,
    pub exit_code: Option<i32>,
    pub stdout: String,
    pub stderr: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerSearchParams {
    pub query: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub network_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerSearchApplyParams {
    pub query: String,
    #[serde(default)]
    pub owner: Option<String>,
    #[serde(default)]
    pub network_allowed: bool,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub preview_token: Option<String>,
    #[serde(default)]
    pub action_reference: Option<ActionReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerSearchResult {
    pub name: String,
    pub source: Option<String>,
    pub description: Option<String>,
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerSearchRecord {
    pub preview: SkillManagerCommandPreview,
    pub output: Option<SkillManagerCommandOutput>,
    pub results: Vec<SkillManagerSearchResult>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub readback: Option<ActionReadbackRecord>,
    #[serde(flatten)]
    pub page: ListPageMetadata,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerListInstalledParams {
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerInstalledRecord {
    pub name: String,
    pub source: Option<String>,
    pub source_kind: String,
    pub agents: Vec<String>,
    pub scope: Option<String>,
    pub path: Option<String>,
    #[serde(default, skip_serializing)]
    pub raw: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerInstalledListRecord {
    pub preview: SkillManagerCommandPreview,
    pub output: SkillManagerCommandOutput,
    pub installed: Vec<SkillManagerInstalledRecord>,
    pub source_revision: String,
    #[serde(flatten)]
    pub page: ListPageMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerInstallParams {
    pub source: String,
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub distribution: Option<String>,
    #[serde(default)]
    pub network_allowed: bool,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub preview_token: Option<String>,
    #[serde(default)]
    pub action_reference: Option<ActionReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerRemoveParams {
    pub skill: String,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub cleanup_local_instance_id: Option<String>,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub preview_token: Option<String>,
    #[serde(default)]
    pub action_reference: Option<ActionReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerUpdateParams {
    #[serde(default)]
    pub skills: Vec<String>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub network_allowed: bool,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub preview_token: Option<String>,
    #[serde(default)]
    pub action_reference: Option<ActionReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerLocalCreateParams {
    pub name: String,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub preview_token: Option<String>,
    #[serde(default)]
    pub action_reference: Option<ActionReference>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerDeleteLocalParams {
    pub instance_id: String,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub preview_token: Option<String>,
    #[serde(default)]
    pub action_reference: Option<ActionReference>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillManagerMutationRecord {
    pub preview: SkillManagerCommandPreview,
    pub output: Option<SkillManagerCommandOutput>,
    pub applied: bool,
    pub scanned_count: usize,
    pub updated_skills: Vec<SkillRecord>,
    pub readback: Option<ActionReadbackRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_up: Option<SkillManagerCleanupFollowUp>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillManagerLocalCreateRecord {
    pub preview: SkillManagerCommandPreview,
    pub output: Option<SkillManagerCommandOutput>,
    pub imported: Option<SkillRecord>,
    pub instance_id: Option<String>,
    pub source_path: String,
    pub applied: bool,
    pub readback: Option<ActionReadbackRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerLocalDeleteRecord {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preconditions: Vec<ActionPrecondition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_token: Option<String>,
    pub instance_id: String,
    pub skill_name: String,
    pub path: String,
    pub app_owned: bool,
    pub physical_delete_allowed: bool,
    pub blocked_by_references: Vec<SkillManagerReferenceRecord>,
    pub confirmed: bool,
    pub deleted: bool,
    pub summary: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub readback: Option<ActionReadbackRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub follow_up: Option<SkillManagerCleanupFollowUp>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerCleanupFollowUp {
    pub kind: String,
    pub state: String,
    pub cleanup_required: bool,
    pub message: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerReferenceRecord {
    pub instance_id: String,
    pub name: String,
    pub agent: String,
    pub scope: String,
    pub path: String,
}

pub fn list_skill_management_tools() -> Vec<SkillManagerToolRecord> {
    let npx = resolve_binary(env::var_os("SKILLS_COPILOT_NPX_PATH"), NPX_BINARY);
    let skills_npm = resolve_binary(
        env::var_os("SKILLS_COPILOT_SKILLS_NPM_PATH"),
        SKILLS_NPM_TOOL,
    );
    vec![
        SkillManagerToolRecord {
            id: DEFAULT_MANAGER_TOOL.to_string(),
            display_name: "npx skills".to_string(),
            status: if npx.is_some() { "available" } else { "missing" }.to_string(),
            executable: npx.map(|path| path.to_string_lossy().to_string()),
            operations: [
                "search",
                "applySearch",
                "listInstalled",
                "previewInstall",
                "applyInstall",
                "previewRemove",
                "applyRemove",
                "previewUpdate",
                "applyUpdate",
                "previewLocalCreate",
                "applyLocalCreate",
                "deleteLocal",
                "previewLocalArchiveImport",
                "applyLocalArchiveImport",
                "previewLocalArchiveUpdate",
                "applyLocalArchiveUpdate",
            ]
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
            default_agents: default_agent_targets(),
            notes: vec![
                "Network-backed search/install/update run only after explicit app confirmation."
                    .to_string(),
                "Symlink distribution is the default; copy is opt-in.".to_string(),
            ],
        },
        SkillManagerToolRecord {
            id: SKILLS_NPM_TOOL.to_string(),
            display_name: "skills-npm".to_string(),
            status: if skills_npm.is_some() {
                "detected-read-only"
            } else {
                "planned"
            }
            .to_string(),
            executable: skills_npm.map(|path| path.to_string_lossy().to_string()),
            operations: vec!["listTools".to_string()],
            default_agents: default_agent_targets(),
            notes: vec![
                "Registry entry only in this slice; write execution is deferred to a later scoped adapter."
                    .to_string(),
            ],
        },
    ]
}

fn skill_manager_search_record(
    preview: SkillManagerCommandPreview,
    output: Option<SkillManagerCommandOutput>,
    results: Vec<SkillManagerSearchResult>,
    readback: Option<ActionReadbackRecord>,
) -> SkillManagerSearchRecord {
    let page = ListPageMetadata {
        returned_count: results.len(),
        total_count: None,
        has_more: false,
        next_cursor: None,
        source_completeness: ListSourceCompleteness::Unknown,
        incomplete_reason: Some(ListIncompleteReason::SourceLimited),
    };
    SkillManagerSearchRecord {
        preview,
        output,
        results,
        readback,
        page,
    }
}

fn skill_manager_installed_record(
    preview: SkillManagerCommandPreview,
    output: SkillManagerCommandOutput,
    installed: Vec<SkillManagerInstalledRecord>,
    source_revision: String,
) -> SkillManagerInstalledListRecord {
    let page = ListPageMetadata::enumerable(installed.len(), Some(installed.len()), None);
    SkillManagerInstalledListRecord {
        preview,
        output,
        installed,
        source_revision,
        page,
    }
}

pub fn preview_install_with_manager(
    ctx: &AdapterContext,
    params: &SkillManagerInstallParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let preview = build_install_preview(ctx, params)?;
    Ok(SkillManagerMutationRecord {
        preview,
        output: None,
        applied: false,
        scanned_count: 0,
        updated_skills: Vec::new(),
        readback: None,
        follow_up: None,
    })
}

pub fn apply_install_with_manager(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerInstallParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let preview = build_install_preview(ctx, params)?;
    ensure_confirmed(
        &preview,
        params.confirmed,
        params.preview_token.as_deref(),
        params.action_reference.as_ref(),
    )?;
    let operation = preview.operation.clone();
    with_manager_mutation_lock(app_data_dir, &operation, || {
        validate_manager_preconditions(ctx, &preview)?;
        let before = manager_selected_skill_snapshot(ctx, &preview)?;
        let transaction = catalog.begin_immediate_transaction()?;
        let output = match run_previewed_command(ctx, &preview) {
            Ok(command) => command.output,
            Err(error) => {
                return Err(rollback_manager_catalog_transaction(
                    ctx,
                    &preview,
                    transaction,
                    error,
                ));
            }
        };
        let mutation = (|| {
            let scan = scan_all_catalog_report(ctx, catalog)?;
            let updated_skills = catalog.list_skill_records()?;
            let after = manager_selected_skill_snapshot(ctx, &preview)?;
            verify_manager_operation(&preview, &updated_skills, &before, &after)?;
            let readback = Some(manager_mutation_readback(
                ctx,
                &preview,
                &updated_skills,
                &[],
                &[],
            )?);
            Ok(SkillManagerMutationRecord {
                preview: preview.clone(),
                output: Some(output),
                applied: true,
                scanned_count: scan.scanned_count,
                updated_skills,
                readback,
                follow_up: None,
            })
        })();
        let record = match mutation {
            Ok(record) => record,
            Err(error) => {
                let error = manager_post_process_error(ctx, &preview, error);
                return Err(rollback_manager_catalog_transaction(
                    ctx,
                    &preview,
                    transaction,
                    error,
                ));
            }
        };
        commit_manager_catalog_transaction(ctx, &preview, transaction)?;
        Ok(record)
    })
}

pub fn preview_remove_with_manager(
    ctx: &AdapterContext,
    params: &SkillManagerRemoveParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let preview = build_remove_preview(ctx, params)?;
    if params.cleanup_local_instance_id.is_some() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "a full uninstall preview requires the current catalog and app-owned source binding"
                .to_string(),
        ));
    }
    Ok(skill_manager_mutation_preview_record(preview))
}

pub fn preview_remove_with_manager_guarded(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerRemoveParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let mut preview = build_remove_preview(ctx, params)?;
    if let Some(instance_id) = params.cleanup_local_instance_id.as_deref() {
        bind_composite_local_delete(
            catalog,
            app_data_dir,
            ctx,
            params,
            instance_id,
            &mut preview,
        )?;
    }
    Ok(skill_manager_mutation_preview_record(preview))
}

fn skill_manager_mutation_preview_record(
    preview: SkillManagerCommandPreview,
) -> SkillManagerMutationRecord {
    SkillManagerMutationRecord {
        preview,
        output: None,
        applied: false,
        scanned_count: 0,
        updated_skills: Vec::new(),
        readback: None,
        follow_up: None,
    }
}

pub fn apply_remove_with_manager(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerRemoveParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let mut preview = build_remove_preview(ctx, params)?;
    let cleanup_plan = if let Some(instance_id) = params.cleanup_local_instance_id.as_deref() {
        bind_composite_local_delete(
            catalog,
            app_data_dir,
            ctx,
            params,
            instance_id,
            &mut preview,
        )?;
        Some(composite_local_delete_plan(
            catalog,
            app_data_dir,
            ctx,
            params,
            instance_id,
        )?)
    } else {
        None
    };
    ensure_confirmed(
        &preview,
        params.confirmed,
        params.preview_token.as_deref(),
        params.action_reference.as_ref(),
    )?;
    let operation = preview.operation.clone();
    with_manager_mutation_lock(app_data_dir, &operation, || {
        let locked_cleanup_plan =
            if let Some(instance_id) = params.cleanup_local_instance_id.as_deref() {
                let plan =
                    composite_local_delete_plan(catalog, app_data_dir, ctx, params, instance_id)?;
                if cleanup_plan.as_ref() != Some(&plan) {
                    return Err(CommandError::StaleActionReference);
                }
                Some(plan)
            } else {
                None
            };
        validate_manager_preconditions_except(
            ctx,
            &preview,
            locked_cleanup_plan
                .as_ref()
                .map(|plan| plan.skill_path.as_path()),
        )?;
        let before = manager_selected_skill_snapshot(ctx, &preview)?;
        let transaction = catalog.begin_immediate_transaction()?;
        let output = match run_previewed_command(ctx, &preview) {
            Ok(command) => command.output,
            Err(error) => {
                return Err(rollback_manager_catalog_transaction(
                    ctx,
                    &preview,
                    transaction,
                    error,
                ));
            }
        };
        let mut cleanup = None;
        let mutation = (|| {
            let scan = scan_all_catalog_report(ctx, catalog)?;
            let mut updated_skills = catalog.list_skill_records()?;
            let after = manager_selected_skill_snapshot(ctx, &preview)?;
            verify_manager_operation(&preview, &updated_skills, &before, &after)?;
            let mut extra_observations = Vec::new();
            if let Some(plan) = locked_cleanup_plan.as_ref() {
                let applied =
                    apply_composite_local_delete(catalog, app_data_dir, plan, &mut cleanup)?;
                extra_observations.extend(applied.observations.clone());
                cleanup = Some(applied);
                updated_skills = catalog.list_skill_records()?;
            }
            let readback = Some(manager_mutation_readback(
                ctx,
                &preview,
                &updated_skills,
                &[],
                &extra_observations,
            )?);
            Ok(SkillManagerMutationRecord {
                preview: preview.clone(),
                output: Some(output),
                applied: true,
                scanned_count: scan.scanned_count,
                updated_skills,
                readback,
                follow_up: None,
            })
        })();
        let mut record = match mutation {
            Ok(record) => record,
            Err(error) => {
                let error = manager_post_process_error(ctx, &preview, error);
                rollback_composite_local_delete(transaction, cleanup.as_mut(), &error)?;
                return Err(error);
            }
        };
        match commit_composite_local_delete(transaction, cleanup.as_mut()) {
            CompositeLocalDeleteCommit::Committed => {}
            CompositeLocalDeleteCommit::NotCommitted(error) => {
                return Err(manager_partial_effect(
                    ctx,
                    &preview,
                    "applied_unverified",
                    cleanup.is_some(),
                    &format!(
                        "catalog commit was rejected after manager execution and the local source was restored: {error}"
                    ),
                ));
            }
            CompositeLocalDeleteCommit::OutcomeUnknown(error) => {
                return Err(manager_partial_effect(
                    ctx,
                    &preview,
                    "outcome_unknown",
                    true,
                    &format!(
                        "catalog commit outcome is unknown after manager execution; private local restoration material was retained: {error}"
                    ),
                ));
            }
            CompositeLocalDeleteCommit::RestorationFailed {
                commit_error,
                cleanup_error,
            } => {
                return Err(manager_partial_effect(
                    ctx,
                    &preview,
                    "outcome_unknown",
                    true,
                    &format!(
                        "catalog commit was rejected ({commit_error}); local source restoration failed ({cleanup_error})"
                    ),
                ));
            }
        }
        if let Some(cleanup) = cleanup.as_mut() {
            if cleanup.finish().is_err() {
                record.follow_up = Some(SkillManagerCleanupFollowUp {
                    kind: "quarantine_cleanup".to_string(),
                    state: "delete_applied_cleanup_pending".to_string(),
                    cleanup_required: true,
                    message:
                        "The agent links and local source were removed and verified, but private cleanup remains pending."
                            .to_string(),
                });
            }
        }
        Ok(record)
    })
}

pub fn preview_update_with_manager(
    ctx: &AdapterContext,
    params: &SkillManagerUpdateParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let preview = build_update_preview(ctx, params)?;
    Ok(SkillManagerMutationRecord {
        preview,
        output: None,
        applied: false,
        scanned_count: 0,
        updated_skills: Vec::new(),
        readback: None,
        follow_up: None,
    })
}

pub fn apply_update_with_manager(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerUpdateParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let preview = build_update_preview(ctx, params)?;
    ensure_confirmed(
        &preview,
        params.confirmed,
        params.preview_token.as_deref(),
        params.action_reference.as_ref(),
    )?;
    let operation = preview.operation.clone();
    with_manager_mutation_lock(app_data_dir, &operation, || {
        validate_manager_preconditions(ctx, &preview)?;
        let before = manager_selected_skill_snapshot(ctx, &preview)?;
        let transaction = catalog.begin_immediate_transaction()?;
        let output = match run_previewed_command(ctx, &preview) {
            Ok(command) => command.output,
            Err(error) => {
                return Err(rollback_manager_catalog_transaction(
                    ctx,
                    &preview,
                    transaction,
                    error,
                ));
            }
        };
        let mutation = (|| {
            let scan = scan_all_catalog_report(ctx, catalog)?;
            let updated_skills = catalog.list_skill_records()?;
            let after = manager_selected_skill_snapshot(ctx, &preview)?;
            verify_manager_operation(&preview, &updated_skills, &before, &after)?;
            let readback = Some(manager_mutation_readback(
                ctx,
                &preview,
                &updated_skills,
                &[],
                &[],
            )?);
            Ok(SkillManagerMutationRecord {
                preview: preview.clone(),
                output: Some(output),
                applied: true,
                scanned_count: scan.scanned_count,
                updated_skills,
                readback,
                follow_up: None,
            })
        })();
        let record = match mutation {
            Ok(record) => record,
            Err(error) => {
                let error = manager_post_process_error(ctx, &preview, error);
                return Err(rollback_manager_catalog_transaction(
                    ctx,
                    &preview,
                    transaction,
                    error,
                ));
            }
        };
        commit_manager_catalog_transaction(ctx, &preview, transaction)?;
        Ok(record)
    })
}

pub fn preview_local_create_with_manager(
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalCreateParams,
) -> Result<SkillManagerLocalCreateRecord, CommandError> {
    let preview = build_local_create_preview(app_data_dir, ctx, params)?;
    let source_path = local_create_source_path(app_data_dir, &params.name)?;
    Ok(SkillManagerLocalCreateRecord {
        preview,
        output: None,
        imported: None,
        instance_id: None,
        source_path: source_path.to_string_lossy().to_string(),
        applied: false,
        readback: None,
    })
}

pub fn apply_local_create_with_manager(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalCreateParams,
) -> Result<SkillManagerLocalCreateRecord, CommandError> {
    let preview = build_local_create_preview(app_data_dir, ctx, params)?;
    ensure_confirmed(
        &preview,
        params.confirmed,
        params.preview_token.as_deref(),
        params.action_reference.as_ref(),
    )?;
    let operation = preview.operation.clone();
    with_manager_mutation_lock(app_data_dir, &operation, || {
        validate_manager_preconditions(ctx, &preview)?;
        let transaction = catalog.begin_immediate_transaction()?;
        let output = match run_previewed_command(ctx, &preview) {
            Ok(command) => command.output,
            Err(error) => {
                return Err(rollback_manager_catalog_transaction(
                    ctx,
                    &preview,
                    transaction,
                    error,
                ));
            }
        };
        let mutation = (|| {
            verify_manager_target_transition(&preview)?;
            let source_path = local_create_source_path(app_data_dir, &params.name)?;
            let imported = import_local_skill_to_tool_global(
                catalog,
                ctx,
                &app_data_dir.join("tool-global"),
                &source_path,
            )?;
            let records = catalog.list_skill_records()?;
            verify_local_create_operation(
                catalog,
                &params.name,
                &source_path,
                &imported,
                &records,
            )?;
            let readback = Some(manager_mutation_readback(
                ctx,
                &preview,
                &records,
                &[source_path.clone(), imported.imported.path.clone()],
                &[],
            )?);
            Ok(SkillManagerLocalCreateRecord {
                preview: preview.clone(),
                output: Some(output),
                imported: Some(imported.imported),
                instance_id: Some(imported.instance_id),
                source_path: source_path.to_string_lossy().to_string(),
                applied: true,
                readback,
            })
        })();
        let record = match mutation {
            Ok(record) => record,
            Err(error) => {
                let error = manager_post_process_error(ctx, &preview, error);
                return Err(rollback_manager_catalog_transaction(
                    ctx,
                    &preview,
                    transaction,
                    error,
                ));
            }
        };
        commit_manager_catalog_transaction(ctx, &preview, transaction)?;
        Ok(record)
    })
}

pub fn delete_local_skill_with_manager(
    catalog: &Catalog,
    app_data_dir: &Path,
    params: &SkillManagerDeleteLocalParams,
) -> Result<SkillManagerLocalDeleteRecord, CommandError> {
    let meta = catalog
        .get_skill_instance_meta(&params.instance_id)?
        .ok_or_else(|| CommandError::InstanceNotFound(params.instance_id.clone()))?;
    let records = catalog.list_skill_records()?;
    let root = tool_global_staging_skills_root(app_data_dir);
    let canonical_root = root.canonicalize().unwrap_or(root.clone());
    let canonical_path = meta
        .path
        .canonicalize()
        .unwrap_or_else(|_| meta.path.clone());
    let app_owned =
        meta.agent == AgentId::ToolGlobal && canonical_path.starts_with(&canonical_root);
    let blocked_by_references = local_delete_references(&meta, &records);
    let physical_delete_allowed = app_owned && blocked_by_references.is_empty();
    let action_binding = if physical_delete_allowed {
        Some(local_delete_action_binding(
            &meta,
            &canonical_path,
            &blocked_by_references,
        )?)
    } else {
        None
    };
    let mut deleted = false;
    let mut readback = None;
    let mut follow_up = None;
    if params.confirmed {
        if !physical_delete_allowed {
            return Err(CommandError::InvalidSkillManagerRequest(
                "local skill physical delete is allowed only for app-owned records with no supported-agent references".to_string(),
            ));
        }
        let binding = action_binding.as_ref().ok_or_else(|| {
            CommandError::MismatchedActionReference(
                "local delete preview has no safe typed action".to_string(),
            )
        })?;
        let preview_token = params.preview_token.as_deref().ok_or_else(|| {
            CommandError::ActionConfirmationRequired(
                "local delete requires the exact preview_token".to_string(),
            )
        })?;
        let action_reference = params.action_reference.as_ref().ok_or_else(|| {
            CommandError::ActionConfirmationRequired(
                "local delete requires the preview action_reference".to_string(),
            )
        })?;
        ensure_action_confirmed(
            binding,
            Some(&ActionConfirmation {
                reference: action_reference.clone(),
                preview_token: preview_token.to_string(),
                confirmed: true,
            }),
        )?;
        let _mutation_lock = crate::mutation_lock::lock_app_mutations(app_data_dir)?;
        run_local_delete_pre_rename_test_hook(&canonical_path);
        let transaction = catalog.begin_immediate_transaction()?;
        let current_meta = catalog
            .get_skill_instance_meta(&params.instance_id)?
            .ok_or(CommandError::StaleActionReference)?;
        let current_records = catalog.list_skill_records()?;
        let current_canonical_path = current_meta
            .path
            .canonicalize()
            .unwrap_or_else(|_| current_meta.path.clone());
        let current_references = local_delete_references(&current_meta, &current_records);
        let current_app_owned = current_meta.agent == AgentId::ToolGlobal
            && current_canonical_path.starts_with(&canonical_root);
        if !current_app_owned || !current_references.is_empty() {
            return Err(CommandError::StaleActionReference);
        }
        let expected_catalog_revision = binding
            .preconditions
            .iter()
            .find(|precondition| {
                precondition.kind == ActionPreconditionKind::CatalogRecord
                    && precondition.target_id == params.instance_id
            })
            .map(|precondition| precondition.expected_revision.as_str())
            .ok_or_else(|| {
                CommandError::MismatchedActionReference(
                    "local delete action has no catalog precondition".to_string(),
                )
            })?;
        if local_delete_catalog_revision(
            &current_meta,
            &current_canonical_path,
            &current_references,
        )? != expected_catalog_revision
        {
            return Err(CommandError::StaleActionReference);
        }
        let expected_tree_revision = binding
            .preconditions
            .iter()
            .find(|precondition| {
                precondition.kind == ActionPreconditionKind::SourceFile
                    && precondition.target_id == canonical_path.to_string_lossy()
            })
            .map(|precondition| precondition.expected_revision.as_str())
            .ok_or_else(|| {
                CommandError::MismatchedActionReference(
                    "local delete action has no source-tree precondition".to_string(),
                )
            })?;
        if current_canonical_path != canonical_path
            || local_delete_tree_revision(&current_canonical_path)? != expected_tree_revision
        {
            return Err(CommandError::StaleActionReference);
        }
        let skill_dir = current_canonical_path.parent().ok_or_else(|| {
            CommandError::UnsafeConfigPath("local skill path has no parent".to_string())
        })?;
        let quarantine = skill_dir.with_file_name(format!(
            ".agent-copilot-delete-{}-{}",
            safe_skill_name(&current_meta.name)?,
            unix_timestamp_millis()
        ));
        if !skill_dir.starts_with(&canonical_root) || !skill_dir.exists() || quarantine.exists() {
            return Err(CommandError::UnsafeConfigPath(
                "local skill delete target changed after preview".to_string(),
            ));
        }
        fs::rename(skill_dir, &quarantine)?;
        run_local_delete_post_rename_test_hook(&current_canonical_path);
        let (missing_tree_revision, observed_tree_revision) =
            match (|| -> Result<_, CommandError> {
                Ok((
                    local_delete_missing_tree_revision(&current_canonical_path)?,
                    local_delete_tree_revision(&current_canonical_path)?,
                ))
            })() {
                Ok(revisions) => revisions,
                Err(error) => {
                    rollback_catalog_before_compensation(
                        transaction,
                        "skillManager.deleteLocal",
                        &error,
                        "the quarantined original was preserved for inspection",
                    )?;
                    restore_local_delete_quarantine(
                        &quarantine,
                        skill_dir,
                        &current_canonical_path,
                        expected_tree_revision,
                    )
                    .map_err(|cleanup_error| CommandError::PartialEffect {
                        operation: "skillManager.deleteLocal".to_string(),
                        state: "outcome_unknown",
                        cleanup_required: true,
                        detail: format!(
                            "post-quarantine verification failed ({error}); source restoration failed ({cleanup_error})"
                        ),
                    })?;
                    return Err(error);
                }
            };
        if observed_tree_revision != missing_tree_revision {
            let error = CommandError::PartialEffect {
                operation: "skillManager.deleteLocal".to_string(),
                state: "outcome_unknown",
                cleanup_required: true,
                detail: "the original local skill path was recreated after quarantine; the unowned path and quarantined original were preserved for review".to_string(),
            };
            rollback_catalog_before_compensation(
                transaction,
                "skillManager.deleteLocal",
                &error,
                "the unowned path and quarantined original were preserved for inspection",
            )?;
            return Err(error);
        }
        let payload = serde_json::json!({
            "deleted": true,
            "path": current_canonical_path.to_string_lossy(),
            "app_owned": current_app_owned,
        });
        let catalog_delete = (|| -> Result<(), CommandError> {
            catalog.create_skill_event(SkillEventDraft {
                instance_id: &current_meta.id,
                kind: "local-delete",
                payload: &serde_json::to_string(&payload)?,
                occurred_at_ms: unix_timestamp_millis(),
            })?;
            catalog.delete_skill_instance(&current_meta.id)?;
            if catalog.get_skill_record(&current_meta.id)?.is_some() {
                return Err(CommandError::VerificationFailed);
            }
            Ok(())
        })();
        if let Err(error) = catalog_delete {
            rollback_catalog_before_compensation(
                transaction,
                "skillManager.deleteLocal",
                &error,
                "the quarantined original was preserved for inspection",
            )?;
            restore_local_delete_quarantine(
                &quarantine,
                skill_dir,
                &current_canonical_path,
                expected_tree_revision,
            )
            .map_err(|cleanup_error| CommandError::PartialEffect {
                operation: "skillManager.deleteLocal".to_string(),
                state: "outcome_unknown",
                cleanup_required: true,
                detail: format!(
                    "catalog delete failed ({error}); source restoration failed ({cleanup_error})"
                ),
            })?;
            return Err(error);
        }
        let verified_readback = match (|| {
            ActionReadbackRecord::verified(
                &binding.action,
                vec![
                    ActionReadbackObservation {
                        domain: ActionReadbackDomain::SkillFiles,
                        target_id: current_canonical_path.to_string_lossy().to_string(),
                        revision: missing_tree_revision.clone(),
                    },
                    ActionReadbackObservation {
                        domain: ActionReadbackDomain::CatalogSkills,
                        target_id: current_meta.id.clone(),
                        revision: action_source_revision(
                            "catalog.skill.missing",
                            &[("instance_id", &current_meta.id)],
                        )?,
                    },
                ],
            )
        })() {
            Ok(readback) => readback,
            Err(error) => {
                rollback_catalog_before_compensation(
                    transaction,
                    "skillManager.deleteLocal",
                    &error,
                    "the quarantined original was preserved for inspection",
                )?;
                restore_local_delete_quarantine(
                    &quarantine,
                    skill_dir,
                    &current_canonical_path,
                    expected_tree_revision,
                )
                .map_err(|cleanup_error| CommandError::PartialEffect {
                    operation: "skillManager.deleteLocal".to_string(),
                    state: "outcome_unknown",
                    cleanup_required: true,
                    detail: format!(
                        "read-back failed ({error}); source restoration failed ({cleanup_error})"
                    ),
                })?;
                return Err(error);
            }
        };
        match transaction.commit_classified() {
            Ok(()) => {}
            Err(CatalogCommitError::NotCommitted(error)) => {
                restore_local_delete_quarantine(
                    &quarantine,
                    skill_dir,
                    &current_canonical_path,
                    expected_tree_revision,
                )
                .map_err(|cleanup_error| CommandError::PartialEffect {
                    operation: "skillManager.deleteLocal".to_string(),
                    state: "outcome_unknown",
                    cleanup_required: true,
                    detail: format!(
                        "catalog commit was rejected ({error}); source restoration failed ({cleanup_error})"
                    ),
                })?;
                return Err(error.into());
            }
            Err(CatalogCommitError::OutcomeUnknown(error)) => {
                return Err(CommandError::PartialEffect {
                    operation: "skillManager.deleteLocal".to_string(),
                    state: "outcome_unknown",
                    cleanup_required: true,
                    detail: format!(
                        "catalog commit outcome is unknown after the local source was quarantined and verified missing ({error}); private restoration material was retained for inspection"
                    ),
                });
            }
        }
        let cleanup_result = if inject_local_delete_cleanup_failure(&current_canonical_path) {
            Err(std::io::Error::other(
                "injected local delete quarantine cleanup failure",
            ))
        } else {
            fs::remove_dir_all(&quarantine)
        };
        if cleanup_result.is_err() {
            follow_up = Some(SkillManagerCleanupFollowUp {
                kind: "quarantine_cleanup".to_string(),
                state: "delete_applied_cleanup_pending".to_string(),
                cleanup_required: true,
                message: "The skill was removed and verified, but private cleanup remains pending."
                    .to_string(),
            });
        }
        readback = Some(verified_readback);
        deleted = true;
    }
    Ok(SkillManagerLocalDeleteRecord {
        action: action_binding
            .as_ref()
            .map(|binding| binding.action.clone()),
        preconditions: action_binding
            .as_ref()
            .map(|binding| binding.preconditions.clone())
            .unwrap_or_default(),
        preview_token: action_binding
            .as_ref()
            .map(|binding| binding.preview_token.clone()),
        instance_id: meta.id,
        skill_name: meta.name,
        path: canonical_path.to_string_lossy().to_string(),
        app_owned,
        physical_delete_allowed,
        blocked_by_references,
        confirmed: params.confirmed,
        deleted,
        summary: if deleted {
            "Deleted the app-owned local skill directory and catalog row.".to_string()
        } else if physical_delete_allowed {
            "Local skill has no supported-agent references and can be physically deleted after confirmation.".to_string()
        } else {
            "Local skill cannot be physically deleted until supported-agent references are removed, or because the source is not app-owned.".to_string()
        },
        readback,
        follow_up,
    })
}

fn restore_local_delete_quarantine(
    quarantine: &Path,
    original_directory: &Path,
    original_skill_file: &Path,
    expected_tree_revision: &str,
) -> Result<(), CommandError> {
    if original_directory.exists() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local delete target changed to an unowned third state during compensation".to_string(),
        ));
    }
    let quarantined_skill = quarantine.join(
        original_skill_file
            .file_name()
            .ok_or_else(|| CommandError::VerificationFailed)?,
    );
    if local_delete_tree_revision(&quarantined_skill)? != expected_tree_revision {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local delete quarantine changed to an unowned third state during compensation"
                .to_string(),
        ));
    }
    fs::rename(quarantine, original_directory)?;
    if !original_skill_file.is_file() {
        return Err(CommandError::VerificationFailed);
    }
    Ok(())
}

pub fn validate_local_delete_confirmation(
    catalog: &Catalog,
    app_data_dir: &Path,
    params: &SkillManagerDeleteLocalParams,
) -> Result<(), CommandError> {
    let preview = delete_local_skill_with_manager(
        catalog,
        app_data_dir,
        &SkillManagerDeleteLocalParams {
            instance_id: params.instance_id.clone(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )?;
    let binding = ActionPreviewBinding {
        action: preview.action.ok_or_else(|| {
            CommandError::InvalidSkillManagerRequest(
                "local skill is not eligible for physical deletion".to_string(),
            )
        })?,
        preconditions: preview.preconditions,
        preview_token: preview.preview_token.ok_or_else(|| {
            CommandError::ActionConfirmationRequired(
                "local delete preview token is unavailable".to_string(),
            )
        })?,
    };
    let token = params.preview_token.as_deref().ok_or_else(|| {
        CommandError::ActionConfirmationRequired(
            "local delete requires the exact preview_token".to_string(),
        )
    })?;
    let reference = params.action_reference.as_ref().ok_or_else(|| {
        CommandError::ActionConfirmationRequired(
            "local delete requires the preview action_reference".to_string(),
        )
    })?;
    ensure_action_confirmed(
        &binding,
        Some(&ActionConfirmation {
            reference: reference.clone(),
            preview_token: token.to_string(),
            confirmed: params.confirmed,
        }),
    )
}

fn with_manager_mutation_lock<T>(
    app_data_dir: &Path,
    operation: &str,
    action: impl FnOnce() -> Result<T, CommandError>,
) -> Result<T, CommandError> {
    let _mutation_lock = crate::mutation_lock::lock_app_mutations(app_data_dir)?;
    run_manager_pre_execute_test_hook(operation);
    action()
}

fn with_search_mutation_lock<T>(
    app_data_dir: &Path,
    action: impl FnOnce() -> Result<T, CommandError>,
) -> Result<T, CommandError> {
    let owner_was_missing_at_preflight =
        crate::mutation_lock::app_mutation_owner_is_missing(app_data_dir)?;
    let _mutation_lock = if owner_was_missing_at_preflight {
        crate::mutation_lock::lock_or_create_app_mutations(app_data_dir)?
    } else {
        crate::mutation_lock::lock_app_mutations(app_data_dir)?
    };
    run_manager_pre_execute_test_hook("search");
    action()
}

#[cfg(test)]
struct ManagerPreExecuteTestHook {
    operation: String,
    action: Box<dyn FnOnce() + Send>,
}

#[cfg(test)]
static MANAGER_PRE_EXECUTE_TEST_HOOK: std::sync::Mutex<Option<ManagerPreExecuteTestHook>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
pub(crate) fn install_manager_pre_execute_test_hook(
    operation: impl Into<String>,
    action: impl FnOnce() + Send + 'static,
) {
    let mut hook = MANAGER_PRE_EXECUTE_TEST_HOOK
        .lock()
        .expect("lock manager pre-execute test hook");
    assert!(hook.is_none(), "manager pre-execute test hook already set");
    *hook = Some(ManagerPreExecuteTestHook {
        operation: operation.into(),
        action: Box::new(action),
    });
}

#[cfg(test)]
fn run_manager_pre_execute_test_hook(operation: &str) {
    let action = {
        let mut hook = MANAGER_PRE_EXECUTE_TEST_HOOK
            .lock()
            .expect("lock manager pre-execute test hook");
        if hook
            .as_ref()
            .is_some_and(|scheduled| scheduled.operation == operation)
        {
            hook.take().map(|scheduled| scheduled.action)
        } else {
            None
        }
    };
    if let Some(action) = action {
        action();
    }
}

#[cfg(not(test))]
fn run_manager_pre_execute_test_hook(_operation: &str) {}

#[cfg(test)]
struct LocalDeletePreRenameTestHook {
    canonical_path: PathBuf,
    action: Box<dyn FnOnce() + Send>,
}

#[cfg(test)]
static LOCAL_DELETE_PRE_RENAME_TEST_HOOK: std::sync::Mutex<Option<LocalDeletePreRenameTestHook>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
pub(crate) fn install_local_delete_pre_rename_test_hook(
    canonical_path: PathBuf,
    action: impl FnOnce() + Send + 'static,
) {
    let mut hook = LOCAL_DELETE_PRE_RENAME_TEST_HOOK
        .lock()
        .expect("lock local-delete pre-rename test hook");
    assert!(
        hook.is_none(),
        "local-delete pre-rename test hook already set"
    );
    *hook = Some(LocalDeletePreRenameTestHook {
        canonical_path,
        action: Box::new(action),
    });
}

#[cfg(test)]
fn run_local_delete_pre_rename_test_hook(canonical_path: &Path) {
    let action = {
        let mut hook = LOCAL_DELETE_PRE_RENAME_TEST_HOOK
            .lock()
            .expect("lock local-delete pre-rename test hook");
        if hook
            .as_ref()
            .is_some_and(|scheduled| scheduled.canonical_path == canonical_path)
        {
            hook.take().map(|scheduled| scheduled.action)
        } else {
            None
        }
    };
    if let Some(action) = action {
        action();
    }
}

#[cfg(not(test))]
fn run_local_delete_pre_rename_test_hook(_canonical_path: &Path) {}

#[cfg(test)]
struct LocalDeletePostRenameTestHook {
    canonical_path: PathBuf,
    action: Box<dyn FnOnce() + Send>,
}

#[cfg(test)]
static LOCAL_DELETE_POST_RENAME_TEST_HOOK: std::sync::Mutex<Option<LocalDeletePostRenameTestHook>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
pub(crate) fn install_local_delete_post_rename_test_hook(
    canonical_path: PathBuf,
    action: impl FnOnce() + Send + 'static,
) {
    let mut hook = LOCAL_DELETE_POST_RENAME_TEST_HOOK
        .lock()
        .expect("lock local-delete post-rename test hook");
    assert!(
        hook.is_none(),
        "local-delete post-rename test hook already set"
    );
    *hook = Some(LocalDeletePostRenameTestHook {
        canonical_path,
        action: Box::new(action),
    });
}

#[cfg(test)]
fn run_local_delete_post_rename_test_hook(canonical_path: &Path) {
    let action = {
        let mut hook = LOCAL_DELETE_POST_RENAME_TEST_HOOK
            .lock()
            .expect("lock local-delete post-rename test hook");
        if hook
            .as_ref()
            .is_some_and(|scheduled| scheduled.canonical_path == canonical_path)
        {
            hook.take().map(|scheduled| scheduled.action)
        } else {
            None
        }
    };
    if let Some(action) = action {
        action();
    }
}

#[cfg(not(test))]
fn run_local_delete_post_rename_test_hook(_canonical_path: &Path) {}

#[cfg(test)]
static LOCAL_DELETE_CLEANUP_FAILURE_TEST_HOOK: std::sync::Mutex<Option<PathBuf>> =
    std::sync::Mutex::new(None);

#[cfg(test)]
pub(crate) fn install_local_delete_cleanup_failure_test_hook(canonical_path: PathBuf) {
    let mut hook = LOCAL_DELETE_CLEANUP_FAILURE_TEST_HOOK
        .lock()
        .expect("lock local-delete cleanup failure test hook");
    assert!(hook.is_none(), "local-delete cleanup test hook already set");
    *hook = Some(canonical_path);
}

#[cfg(test)]
fn inject_local_delete_cleanup_failure(canonical_path: &Path) -> bool {
    let mut hook = LOCAL_DELETE_CLEANUP_FAILURE_TEST_HOOK
        .lock()
        .expect("lock local-delete cleanup failure test hook");
    if hook
        .as_ref()
        .is_some_and(|scheduled| scheduled == canonical_path)
    {
        hook.take();
        true
    } else {
        false
    }
}

#[cfg(not(test))]
fn inject_local_delete_cleanup_failure(_canonical_path: &Path) -> bool {
    false
}

fn local_delete_references(
    meta: &skills_copilot_catalog::SkillInstanceMeta,
    records: &[SkillRecord],
) -> Vec<SkillManagerReferenceRecord> {
    let mut references = records
        .iter()
        .filter(|record| record.id != meta.id)
        .filter(|record| record.name == meta.name)
        .filter(|record| record.agent != AgentId::ToolGlobal.as_str())
        .filter(|record| record.state != "missing")
        .map(|record| SkillManagerReferenceRecord {
            instance_id: record.id.clone(),
            name: record.name.clone(),
            agent: record.agent.clone(),
            scope: record.scope.clone(),
            path: record.display_path.to_string_lossy().to_string(),
        })
        .collect::<Vec<_>>();
    references.sort_by(|left, right| {
        left.instance_id
            .cmp(&right.instance_id)
            .then_with(|| left.agent.cmp(&right.agent))
            .then_with(|| left.scope.cmp(&right.scope))
            .then_with(|| left.path.cmp(&right.path))
    });
    references
}

fn local_delete_catalog_revision(
    meta: &skills_copilot_catalog::SkillInstanceMeta,
    canonical_path: &Path,
    references: &[SkillManagerReferenceRecord],
) -> Result<String, CommandError> {
    let references_json = serde_json::to_string(references)?;
    let project_root = meta
        .project_root
        .as_deref()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_default();
    action_source_revision(
        "catalog.skill.precondition",
        &[
            ("instance_id", &meta.id),
            ("agent", meta.agent.as_str()),
            ("scope", meta.scope.as_str()),
            ("project_root", &project_root),
            ("name", &meta.name),
            ("path", &canonical_path.to_string_lossy()),
            ("enabled", if meta.enabled { "true" } else { "false" }),
            ("references", &references_json),
        ],
    )
}

fn local_delete_action_binding(
    meta: &skills_copilot_catalog::SkillInstanceMeta,
    canonical_path: &Path,
    references: &[SkillManagerReferenceRecord],
) -> Result<ActionPreviewBinding, CommandError> {
    let catalog_revision = local_delete_catalog_revision(meta, canonical_path, references)?;
    let source_revision = action_source_revision(
        "manager.local-delete.accepted-snapshot",
        &[
            ("catalog_revision", &catalog_revision),
            (
                "tree_revision",
                &local_delete_tree_revision(canonical_path)?,
            ),
        ],
    )?;
    let descriptor = action_descriptor(
        ActionKind::ManagerLocalDelete,
        ActionIntent::ManagerLocalDelete,
        ActionTargetRef {
            kind: ActionTargetKind::Skill,
            id: meta.id.clone(),
            agent: Some(AgentId::ToolGlobal),
            scope: Some(Scope::ToolGlobal),
        },
        None,
        vec![ActionImpact::SkillFiles, ActionImpact::AppLocalData],
        "skillManager.deleteLocal",
        Some("skillManager.deleteLocal"),
        source_revision,
        true,
        ActionNetworkPosture::None,
        canonical_readback_domains([
            ActionReadbackDomain::SkillFiles,
            ActionReadbackDomain::CatalogSkills,
        ]),
        vec![crate::skill_projection_evidence_id(&meta.id)],
    )?;
    action_preview_binding(
        descriptor,
        vec![
            ActionPrecondition {
                kind: ActionPreconditionKind::CatalogRecord,
                target_id: meta.id.clone(),
                expected_revision: catalog_revision,
            },
            ActionPrecondition {
                kind: ActionPreconditionKind::SourceFile,
                target_id: canonical_path.to_string_lossy().to_string(),
                expected_revision: local_delete_tree_revision(canonical_path)?,
            },
        ],
    )
}

fn local_delete_tree_revision(skill_file: &Path) -> Result<String, CommandError> {
    const MAX_ENTRIES: usize = 1_024;
    const MAX_BYTES: u64 = 16 * 1024 * 1024;

    let root = skill_file.parent().ok_or_else(|| {
        CommandError::UnsafeConfigPath("local skill path has no parent".to_string())
    })?;
    if !root.exists() {
        return local_delete_missing_tree_revision(skill_file);
    }
    let mut pending = vec![root.to_path_buf()];
    let mut entries = Vec::new();
    let mut total_bytes = 0_u64;
    while let Some(directory) = pending.pop() {
        let mut children = fs::read_dir(&directory)?.collect::<Result<Vec<_>, _>>()?;
        children.sort_by_key(|entry| entry.file_name());
        for entry in children {
            if entries.len() >= MAX_ENTRIES {
                return Err(CommandError::InvalidSkillManagerRequest(
                    "local skill delete preview exceeds the 1024-entry safety budget".to_string(),
                ));
            }
            let path = entry.path();
            let metadata = fs::symlink_metadata(&path)?;
            if metadata.file_type().is_symlink() {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "local skill delete refuses symlinked content: {}",
                    path.display()
                )));
            }
            let relative = path
                .strip_prefix(root)
                .map_err(|_| {
                    CommandError::UnsafeConfigPath(
                        "local skill tree escaped its app-owned root".to_string(),
                    )
                })?
                .to_string_lossy()
                .to_string();
            if metadata.is_dir() {
                entries.push(format!("dir:{relative}"));
                pending.push(path);
            } else if metadata.is_file() {
                total_bytes = total_bytes.saturating_add(metadata.len());
                if total_bytes > MAX_BYTES {
                    return Err(CommandError::InvalidSkillManagerRequest(
                        "local skill delete preview exceeds the 16 MiB safety budget".to_string(),
                    ));
                }
                let digest = format!("{:x}", Sha256::digest(fs::read(&path)?));
                entries.push(format!("file:{relative}:{}:{digest}", metadata.len()));
            } else {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "local skill delete refuses special file: {}",
                    path.display()
                )));
            }
        }
    }
    entries.sort();
    let entries_json = serde_json::to_string(&entries)?;
    action_source_revision(
        "manager.local-delete.tree",
        &[
            ("root", &root.to_string_lossy()),
            ("exists", "true"),
            ("entries", &entries_json),
        ],
    )
}

fn local_delete_missing_tree_revision(skill_file: &Path) -> Result<String, CommandError> {
    let root = skill_file.parent().ok_or_else(|| {
        CommandError::UnsafeConfigPath("local skill path has no parent".to_string())
    })?;
    action_source_revision(
        "manager.local-delete.tree",
        &[("root", &root.to_string_lossy()), ("exists", "false")],
    )
}

fn build_install_preview(
    ctx: &AdapterContext,
    params: &SkillManagerInstallParams,
) -> Result<SkillManagerCommandPreview, CommandError> {
    let source = params.source.trim();
    if source.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skillManager install requires source".to_string(),
        ));
    }
    let cwd = manager_cwd(ctx, params.scope.as_deref())?;
    let source_resolution = resolve_manager_source(source, &cwd)?;
    let mut args = vec![
        SKILLS_CLI_BINARY.to_string(),
        "add".to_string(),
        source.to_string(),
    ];
    let skill_names = normalized_skill_names(&params.skills)?;
    if skill_names.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skillManager install requires at least one explicit skill".to_string(),
        ));
    }
    for skill in &skill_names {
        args.push("--skill".to_string());
        args.push(skill.clone());
    }
    if !skill_names.is_empty() {
        args.push("--full-depth".to_string());
    }
    let agents = required_manager_agents(&params.agents)?;
    append_agent_args(&mut args, &agents);
    append_scope_args(&mut args, params.scope.as_deref())?;
    if params
        .distribution
        .as_deref()
        .is_some_and(|distribution| distribution.eq_ignore_ascii_case("copy"))
    {
        args.push("--copy".to_string());
    }
    args.push("-y".to_string());
    let network_required = matches!(source_resolution, ManagerSourceResolution::Network);
    command_preview(
        ctx,
        CommandPreviewDraft {
            operation: "install",
            args,
            cwd,
            network_required,
            network_allowed: params.network_allowed || !network_required,
            confirmed: params.confirmed,
            summary: format!(
                "Install {source} for {} supported agent target(s).",
                agents.len()
            ),
            risks: install_risks(source, network_required),
            source: Some(source.to_string()),
            skills: skill_names,
            accepted_revision: None,
        },
    )
}

fn build_remove_preview(
    ctx: &AdapterContext,
    params: &SkillManagerRemoveParams,
) -> Result<SkillManagerCommandPreview, CommandError> {
    let skill = params.skill.trim();
    if skill.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skillManager remove requires skill".to_string(),
        ));
    }
    let mut args = vec![
        SKILLS_CLI_BINARY.to_string(),
        "remove".to_string(),
        skill.to_string(),
    ];
    let agents = required_manager_agents(&params.agents)?;
    append_agent_args(&mut args, &agents);
    append_scope_args(&mut args, params.scope.as_deref())?;
    args.push("-y".to_string());
    command_preview(
        ctx,
        CommandPreviewDraft {
            operation: "remove",
            args,
            cwd: manager_cwd(ctx, params.scope.as_deref())?,
            network_required: false,
            network_allowed: true,
            confirmed: params.confirmed,
            summary: format!(
                "Remove {skill} from {} supported agent target(s).",
                agents.len()
            ),
            risks: vec![
                "The manager may delete its canonical copy when no selected or managed agent still references it."
                    .to_string(),
            ],
            source: None,
            skills: vec![skill.to_string()],
            accepted_revision: None,
        },
    )
}

fn build_update_preview(
    ctx: &AdapterContext,
    params: &SkillManagerUpdateParams,
) -> Result<SkillManagerCommandPreview, CommandError> {
    let skill_names = normalized_skill_names(&params.skills)?;
    if skill_names.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skillManager update requires at least one explicit skill".to_string(),
        ));
    }
    let mut args = vec![SKILLS_CLI_BINARY.to_string(), "update".to_string()];
    for skill in &skill_names {
        args.push(skill.clone());
    }
    append_scope_args(&mut args, params.scope.as_deref())?;
    args.push("-y".to_string());
    command_preview(
        ctx,
        CommandPreviewDraft {
            operation: "update",
            args,
            cwd: manager_cwd(ctx, params.scope.as_deref())?,
            network_required: true,
            network_allowed: params.network_allowed,
            confirmed: params.confirmed,
            summary: "Update the selected managed skill source for every linked agent.".to_string(),
            risks: vec![
                "Update may contact remote source repositories or indexes through the external CLI."
                    .to_string(),
            ],
            source: None,
            skills: skill_names,
            accepted_revision: None,
        },
    )
}

fn build_local_create_preview(
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalCreateParams,
) -> Result<SkillManagerCommandPreview, CommandError> {
    let name = safe_skill_name(&params.name)?;
    let cwd = local_create_root(app_data_dir);
    let args = vec![
        SKILLS_CLI_BINARY.to_string(),
        "init".to_string(),
        name.clone(),
    ];
    command_preview(
        ctx,
        CommandPreviewDraft {
            operation: "localCreate",
            args,
            cwd,
            network_required: false,
            network_allowed: true,
            confirmed: params.confirmed,
            summary: format!("Create a local skill template named {name}."),
            risks: vec![
                "After creation, the app imports the local source through the existing Local Skill Library parser and rule checks."
                    .to_string(),
            ],
            source: None,
            skills: vec![name.clone()],
            accepted_revision: None,
        },
    )
}

pub(super) struct CommandPreviewDraft {
    operation: &'static str,
    args: Vec<String>,
    cwd: PathBuf,
    network_required: bool,
    network_allowed: bool,
    confirmed: bool,
    summary: String,
    risks: Vec<String>,
    source: Option<String>,
    skills: Vec<String>,
    accepted_revision: Option<String>,
}

struct SkillManagerCommandExecution {
    output: SkillManagerCommandOutput,
    machine_stdout: String,
}

struct MachineStdoutCapture {
    path: PathBuf,
    file: File,
}

impl MachineStdoutCapture {
    fn create() -> Result<Self, CommandError> {
        let temp_dir = env::temp_dir();
        for attempt in 0..32 {
            let path = temp_dir.join(format!(
                "agent-copilot-skill-manager-{}-{}-{attempt}.json",
                std::process::id(),
                unix_timestamp_millis()
            ));
            match OpenOptions::new()
                .read(true)
                .write(true)
                .create_new(true)
                .open(&path)
            {
                Ok(file) => {
                    #[cfg(unix)]
                    {
                        use std::os::unix::fs::PermissionsExt;
                        fs::set_permissions(&path, fs::Permissions::from_mode(0o600))?;
                    }
                    return Ok(Self { path, file });
                }
                Err(error) if error.kind() == std::io::ErrorKind::AlreadyExists => continue,
                Err(error) => return Err(error.into()),
            }
        }
        Err(CommandError::SkillManagerCommandFailed(
            "could not allocate a private installed-inventory capture".to_string(),
        ))
    }

    fn child_stdout(&self) -> Result<Stdio, CommandError> {
        Ok(Stdio::from(self.file.try_clone()?))
    }

    fn read(&mut self) -> Result<Vec<u8>, CommandError> {
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
        Ok(output)
    }
}

impl Drop for MachineStdoutCapture {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

impl SkillManagerCommandOutput {
    fn without_machine_stdout(mut self) -> Self {
        self.stdout.clear();
        self
    }
}

pub(super) fn command_preview(
    ctx: &AdapterContext,
    mut draft: CommandPreviewDraft,
) -> Result<SkillManagerCommandPreview, CommandError> {
    let executable = npx_executable()?;
    let command = {
        let mut command = vec![executable.to_string_lossy().to_string()];
        command.append(&mut draft.args);
        command
    };
    let will_run = draft.confirmed && (!draft.network_required || draft.network_allowed);
    let action_binding = manager_action_binding(
        ctx,
        &command,
        &draft.cwd,
        draft.operation,
        draft.network_required,
        draft.network_allowed,
        draft.accepted_revision.as_deref(),
    )?;
    let preview_token = action_binding
        .as_ref()
        .map(|binding| binding.preview_token.clone())
        .unwrap_or_else(|| {
            preview_token(
                &command,
                &draft.cwd,
                draft.operation,
                draft.network_required,
                draft.network_allowed,
            )
        });
    let preview_env = manager_command_env(ctx, &command[0])
        .into_iter()
        .map(|env_var| SkillManagerEnvPreview {
            key: env_var.key,
            value: redact_command_output(ctx, &env_var.value),
        })
        .collect();
    Ok(SkillManagerCommandPreview {
        action: action_binding
            .as_ref()
            .map(|binding| binding.action.clone()),
        preconditions: action_binding
            .as_ref()
            .map(|binding| binding.preconditions.clone())
            .unwrap_or_default(),
        tool_id: DEFAULT_MANAGER_TOOL.to_string(),
        operation: draft.operation.to_string(),
        command,
        cwd: draft.cwd.to_string_lossy().to_string(),
        env: preview_env,
        requires_confirmation: action_binding
            .as_ref()
            .is_some_and(|binding| binding.action.confirmation_required),
        confirmed: draft.confirmed,
        network_required: draft.network_required,
        network_allowed: draft.network_allowed,
        will_run,
        preview_token,
        summary: draft.summary,
        risks: draft.risks,
        source: draft.source,
        skills: draft.skills,
    })
}

fn run_previewed_command(
    ctx: &AdapterContext,
    preview: &SkillManagerCommandPreview,
) -> Result<SkillManagerCommandExecution, CommandError> {
    if preview.requires_confirmation && !preview.confirmed {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "{} requires confirmed=true",
            preview.operation
        )));
    }
    if preview.network_required && !preview.network_allowed {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "{} requires network_allowed=true",
            preview.operation
        )));
    }
    validate_manager_preconditions(ctx, preview)?;
    let Some((executable, args)) = preview.command.split_first() else {
        return Err(CommandError::InvalidSkillManagerRequest(
            "empty skill manager command".to_string(),
        ));
    };
    let cwd = PathBuf::from(&preview.cwd);
    let created_cwd_candidates = missing_manager_directories(&cwd)?;
    if let Err(error) = fs::create_dir_all(&cwd) {
        return match remove_created_manager_directories(&created_cwd_candidates) {
            Ok(()) => Err(CommandError::SkillManagerCommandFailed(format!(
                "{} did not start: {}",
                preview.operation,
                redact_command_output(ctx, &error.to_string())
            ))),
            Err(cleanup_error) => Err(manager_partial_effect(
                ctx,
                preview,
                "not_started",
                true,
                &format!(
                    "working directory creation failed ({error}); cleanup failed ({cleanup_error})"
                ),
            )),
        };
    }
    let mut command = Command::new(executable);
    command.env_clear();
    command
        .args(args)
        .current_dir(&cwd)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped());
    for env_var in manager_command_env(ctx, executable) {
        command.env(env_var.key, env_var.value);
    }
    // Node's console writes asynchronously when stdout is a pipe. The external
    // manager exits immediately after printing large JSON, which can drop
    // everything beyond the 64 KiB pipe buffer. A private regular file makes
    // that write synchronous; it is bounded, read only after exit, and removed
    // by RAII on every return path.
    let mut machine_capture = if matches!(preview.operation.as_str(), "search" | "listInstalled") {
        Some(MachineStdoutCapture::create()?)
    } else {
        None
    };
    if let Some(capture) = &machine_capture {
        command.stdout(capture.child_stdout()?);
    }
    let child = match command.spawn() {
        Ok(child) => child,
        Err(error) => {
            return match remove_created_manager_directories(&created_cwd_candidates) {
                Ok(()) => Err(CommandError::SkillManagerCommandFailed(format!(
                    "{} did not start: {}",
                    preview.operation,
                    redact_command_output(ctx, &error.to_string())
                ))),
                Err(cleanup_error) => Err(manager_partial_effect(
                    ctx,
                    preview,
                    "not_started",
                    true,
                    &format!(
                        "manager process did not start ({error}); working directory cleanup failed ({cleanup_error})"
                    ),
                )),
            };
        }
    };
    let output = child.wait_with_output().map_err(|error| {
        manager_partial_effect(
            ctx,
            preview,
            "applied_unverified",
            true,
            &format!("manager process completion could not be observed: {error}"),
        )
    })?;
    let machine_stdout = match &mut machine_capture {
        Some(capture) => capture.read().map_err(|error| {
            manager_partial_effect(
                ctx,
                preview,
                "applied_unverified",
                true,
                &format!("manager output could not be verified: {error}"),
            )
        })?,
        None => output.stdout,
    };
    if machine_stdout.len() > MAX_MACHINE_OUTPUT_BYTES
        || output.stderr.len() > MAX_MACHINE_OUTPUT_BYTES
    {
        return Err(manager_partial_effect(
            ctx,
            preview,
            "applied_unverified",
            true,
            "manager output exceeded the safe capture limit",
        ));
    }
    let status = if output.status.success() {
        "completed"
    } else {
        "failed"
    };
    let stdout = redact_command_output(ctx, &String::from_utf8_lossy(&machine_stdout));
    let stderr = redact_command_output(ctx, &String::from_utf8_lossy(&output.stderr));
    let record = SkillManagerCommandOutput {
        status: status.to_string(),
        exit_code: output.status.code(),
        stdout: truncate_capture(&stdout),
        stderr: truncate_capture(&stderr),
    };
    if !output.status.success() {
        let detail = failed_command_detail(&record.stdout, &record.stderr);
        return Err(manager_partial_effect(
            ctx,
            preview,
            "applied_unverified",
            true,
            &format!(
                "manager exited with status {:?}: {}",
                record.exit_code, detail
            ),
        ));
    }
    Ok(SkillManagerCommandExecution {
        output: record,
        machine_stdout: stdout,
    })
}

fn missing_manager_directories(path: &Path) -> Result<Vec<PathBuf>, CommandError> {
    let mut missing = Vec::new();
    let mut current = path;
    while !current.exists() {
        missing.push(current.to_path_buf());
        current = current.parent().ok_or_else(|| {
            CommandError::UnsafeConfigPath(
                "manager working directory has no existing owner ancestor".to_string(),
            )
        })?;
    }
    if fs::symlink_metadata(current)?.file_type().is_symlink() {
        return Err(CommandError::UnsafeConfigPath(
            "manager working directory owner cannot be a symlink".to_string(),
        ));
    }
    Ok(missing)
}

fn remove_created_manager_directories(paths: &[PathBuf]) -> Result<(), CommandError> {
    for path in paths {
        match fs::remove_dir(path) {
            Ok(()) => {}
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
    }
    Ok(())
}

fn manager_partial_effect(
    ctx: &AdapterContext,
    preview: &SkillManagerCommandPreview,
    state: &'static str,
    cleanup_required: bool,
    detail: &str,
) -> CommandError {
    let detail = redact_command_output(ctx, detail);
    if !preview.requires_confirmation {
        return CommandError::SkillManagerCommandFailed(format!(
            "{} failed: {detail}",
            preview.operation
        ));
    }
    CommandError::PartialEffect {
        operation: preview.operation.clone(),
        state,
        cleanup_required,
        detail,
    }
}

fn ensure_confirmed(
    preview: &SkillManagerCommandPreview,
    confirmed: bool,
    preview_token: Option<&str>,
    action_reference: Option<&ActionReference>,
) -> Result<(), CommandError> {
    if !confirmed {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "{} requires confirmed=true",
            preview.operation
        )));
    }
    let token = preview_token.ok_or_else(|| {
        CommandError::ActionConfirmationRequired(
            "skill manager apply requires a fresh preview_token".to_string(),
        )
    })?;
    let reference = action_reference.ok_or_else(|| {
        CommandError::ActionConfirmationRequired(
            "skill manager apply requires the preview action_reference".to_string(),
        )
    })?;
    let action = preview.action.clone().ok_or_else(|| {
        CommandError::MismatchedActionReference(
            "skill manager apply preview has no typed action".to_string(),
        )
    })?;
    let binding = ActionPreviewBinding {
        action,
        preconditions: preview.preconditions.clone(),
        preview_token: preview.preview_token.clone(),
    };
    let confirmation = ActionConfirmation {
        reference: reference.clone(),
        preview_token: token.to_string(),
        confirmed,
    };
    ensure_action_confirmed(&binding, Some(&confirmation))
}

pub fn validate_skill_manager_confirmation(
    preview: &SkillManagerCommandPreview,
    confirmed: bool,
    preview_token: Option<&str>,
    action_reference: Option<&ActionReference>,
) -> Result<(), CommandError> {
    ensure_confirmed(preview, confirmed, preview_token, action_reference)
}

fn manager_mutation_readback(
    ctx: &AdapterContext,
    preview: &SkillManagerCommandPreview,
    records: &[SkillRecord],
    extra_skill_paths: &[PathBuf],
    extra_observations: &[ActionReadbackObservation],
) -> Result<ActionReadbackRecord, CommandError> {
    let action = preview.action.as_ref().ok_or_else(|| {
        CommandError::MismatchedActionReference(
            "skill manager apply preview has no typed action".to_string(),
        )
    })?;
    let snapshot = manager_target_snapshot(
        ctx,
        &preview.command,
        Path::new(&preview.cwd),
        &preview.operation,
    )?;
    let mut observations = Vec::new();
    if preview.operation == "localCreate" {
        let inventory = serde_json::to_string(records)?;
        observations.push(ActionReadbackObservation {
            domain: ActionReadbackDomain::CatalogSkills,
            target_id: action.target.id.clone(),
            revision: action_source_revision(
                "manager.catalog.readback",
                &[("records", &inventory)],
            )?,
        });
    } else {
        let global = preview
            .command
            .iter()
            .any(|argument| argument == "--global");
        let scope = if global {
            Scope::AgentGlobal
        } else {
            Scope::AgentProject
        };
        for (agent, root) in
            manager_action_targets(ctx, &preview.command, Path::new(&preview.cwd), global)
        {
            let canonical_root = root.canonicalize().unwrap_or_else(|_| root.clone());
            let target_id = format!("{}:{}", action.target.id, agent.as_str());
            let target_records = records
                .iter()
                .filter(|record| {
                    record.agent == agent.as_str()
                        && record.scope == scope.as_str()
                        && (record.path.starts_with(&canonical_root)
                            || record.display_path.starts_with(&root))
                })
                .collect::<Vec<_>>();
            let target_inventory = serde_json::to_string(&target_records)?;
            observations.push(ActionReadbackObservation {
                domain: ActionReadbackDomain::CatalogSkills,
                target_id: target_id.clone(),
                revision: action_source_revision(
                    "manager.catalog.target.readback",
                    &[("records", &target_inventory)],
                )?,
            });
            observations.push(ActionReadbackObservation {
                domain: ActionReadbackDomain::SkillFiles,
                target_id,
                revision: manager_target_revision(&root)?,
            });
        }
    }
    if action
        .readback
        .contains(&ActionReadbackDomain::ManagerInventory)
    {
        observations.push(ActionReadbackObservation {
            domain: ActionReadbackDomain::ManagerInventory,
            target_id: action.target.id.clone(),
            revision: manager_inventory_revision(&snapshot.inventory_paths)?,
        });
    }
    if action.readback.contains(&ActionReadbackDomain::SkillFiles) {
        let mut skill_paths = snapshot.skill_paths;
        skill_paths.extend_from_slice(extra_skill_paths);
        sort_dedup_paths(&mut skill_paths);
        if preview.operation == "localCreate" {
            for (index, skill_path) in skill_paths.iter().enumerate() {
                observations.push(ActionReadbackObservation {
                    domain: ActionReadbackDomain::SkillFiles,
                    target_id: format!("{}:path:{index}", action.target.id),
                    revision: manager_target_revision(skill_path)?,
                });
            }
        }
    }
    observations.extend_from_slice(extra_observations);
    ActionReadbackRecord::verified(action, observations)
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ManagerSelectedSkillState {
    agent: AgentId,
    root: PathBuf,
    skill: String,
    exists: bool,
    canonical_skill_file: Option<PathBuf>,
    source_identity: Option<String>,
    content_fingerprint: Option<String>,
}

fn manager_selected_skill_snapshot(
    ctx: &AdapterContext,
    preview: &SkillManagerCommandPreview,
) -> Result<Vec<ManagerSelectedSkillState>, CommandError> {
    let global = preview
        .command
        .iter()
        .any(|argument| argument == "--global");
    let cwd = Path::new(&preview.cwd);
    let targets = manager_action_targets(ctx, &preview.command, cwd, global)
        .into_iter()
        .collect::<Vec<_>>();
    let mut states = Vec::new();
    for skill in &preview.skills {
        for (agent, root) in &targets {
            states.push(manager_selected_skill_state(*agent, root, skill)?);
        }
    }
    states.sort_by(|left, right| {
        left.agent
            .as_str()
            .cmp(right.agent.as_str())
            .then_with(|| left.root.cmp(&right.root))
            .then_with(|| left.skill.cmp(&right.skill))
    });
    Ok(states)
}

fn manager_selected_skill_state(
    agent: AgentId,
    root: &Path,
    requested_skill: &str,
) -> Result<ManagerSelectedSkillState, CommandError> {
    let mut pending = vec![root.to_path_buf()];
    let mut visited = BTreeSet::new();
    let mut matching = Vec::new();
    let mut entries = 0_usize;
    let mut bytes = 0_u64;
    while let Some(path) = pending.pop() {
        let metadata = match fs::symlink_metadata(&path) {
            Ok(metadata) => metadata,
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => continue,
            Err(error) => return Err(error.into()),
        };
        entries = entries.saturating_add(1);
        if entries > MAX_MANAGER_TARGET_ENTRIES {
            return Err(CommandError::InvalidSkillManagerRequest(
                "selected manager skill verification exceeded the entry safety budget".to_string(),
            ));
        }
        if metadata.file_type().is_symlink() {
            let canonical = path.canonicalize().map_err(|_| {
                CommandError::UnsafeConfigPath(
                    "selected manager skill contains a dangling symlink".to_string(),
                )
            })?;
            pending.push(canonical);
            continue;
        }
        if metadata.is_dir() {
            let canonical = path.canonicalize()?;
            if !visited.insert(canonical) {
                continue;
            }
            let mut children = fs::read_dir(&path)?.collect::<Result<Vec<_>, _>>()?;
            children.sort_by_key(|entry| entry.file_name());
            for child in children.into_iter().rev() {
                pending.push(child.path());
            }
            continue;
        }
        if !metadata.is_file()
            || path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md")
        {
            continue;
        }
        bytes = bytes.saturating_add(metadata.len());
        if bytes > MAX_MANAGER_TARGET_BYTES {
            return Err(CommandError::InvalidSkillManagerRequest(
                "selected manager skill verification exceeded the byte safety budget".to_string(),
            ));
        }
        let content = fs::read_to_string(&path)?;
        let parsed = super::parse_tool_global_skill(&content, requested_skill);
        if !parsed.name.eq_ignore_ascii_case(requested_skill) {
            continue;
        }
        let canonical_skill_file = path.canonicalize()?;
        let canonical_source = canonical_skill_file
            .parent()
            .ok_or_else(|| {
                CommandError::UnsafeConfigPath(
                    "selected manager skill has no source directory".to_string(),
                )
            })?
            .to_string_lossy()
            .to_string();
        matching.push((
            canonical_skill_file,
            action_source_revision(
                "manager.skill.source-identity",
                &[("canonical_source", &canonical_source)],
            )?,
            format!("sha256:{:x}", Sha256::digest(content.as_bytes())),
        ));
    }
    if matching.len() > 1 {
        return Err(CommandError::VerificationFailed);
    }
    let Some((canonical_skill_file, source_identity, content_fingerprint)) =
        matching.into_iter().next()
    else {
        return Ok(ManagerSelectedSkillState {
            agent,
            root: root.to_path_buf(),
            skill: requested_skill.to_string(),
            exists: false,
            canonical_skill_file: None,
            source_identity: None,
            content_fingerprint: None,
        });
    };
    Ok(ManagerSelectedSkillState {
        agent,
        root: root.to_path_buf(),
        skill: requested_skill.to_string(),
        exists: true,
        canonical_skill_file: Some(canonical_skill_file),
        source_identity: Some(source_identity),
        content_fingerprint: Some(content_fingerprint),
    })
}

fn verify_manager_operation(
    preview: &SkillManagerCommandPreview,
    records: &[SkillRecord],
    before: &[ManagerSelectedSkillState],
    after: &[ManagerSelectedSkillState],
) -> Result<(), CommandError> {
    let global = preview
        .command
        .iter()
        .any(|argument| argument == "--global");
    let scope = if global {
        Scope::AgentGlobal
    } else {
        Scope::AgentProject
    };
    let after_for = |state: &ManagerSelectedSkillState| {
        after.iter().find(|candidate| {
            candidate.agent == state.agent
                && candidate.root == state.root
                && candidate.skill == state.skill
        })
    };
    let catalog_proves = |state: &ManagerSelectedSkillState, should_exist: bool| {
        let matching = records.iter().any(|record| {
            record.agent == state.agent.as_str()
                && record.scope == scope.as_str()
                && record.name.eq_ignore_ascii_case(&state.skill)
                && record.state != "missing"
                && record.state != "broken"
                && state.canonical_skill_file.as_ref().is_some_and(|path| {
                    record
                        .path
                        .canonicalize()
                        .is_ok_and(|record_path| record_path == *path)
                })
        });
        matching == should_exist
    };

    match preview.operation.as_str() {
        "install" => {
            if preview.skills.is_empty() || before.is_empty() {
                return Err(CommandError::VerificationFailed);
            }
            verify_manager_install_lock_source(preview)?;
            for prior in before {
                let current = after_for(prior).ok_or(CommandError::VerificationFailed)?;
                if !current.exists
                    || current.source_identity.is_none()
                    || current.content_fingerprint.is_none()
                    || !catalog_proves(current, true)
                {
                    return Err(CommandError::VerificationFailed);
                }
            }
        }
        "remove" => {
            if preview.skills.len() != 1 || before.is_empty() {
                return Err(CommandError::VerificationFailed);
            }
            for prior in before {
                let current = after_for(prior).ok_or(CommandError::VerificationFailed)?;
                if !prior.exists || current.exists || !catalog_proves(prior, false) {
                    return Err(CommandError::VerificationFailed);
                }
            }
        }
        "update" => {
            let linked_before = before
                .iter()
                .filter(|state| state.exists)
                .collect::<Vec<_>>();
            if preview.skills.is_empty() || linked_before.is_empty() {
                return Err(CommandError::VerificationFailed);
            }
            for prior in linked_before {
                let current = after_for(prior).ok_or(CommandError::VerificationFailed)?;
                if !current.exists
                    || current.source_identity.is_none()
                    || current.content_fingerprint.is_none()
                    || prior.source_identity != current.source_identity
                    || prior.content_fingerprint == current.content_fingerprint
                    || !catalog_proves(current, true)
                {
                    return Err(CommandError::VerificationFailed);
                }
            }
        }
        _ => return Err(CommandError::VerificationFailed),
    }
    Ok(())
}

fn verify_manager_install_lock_source(
    preview: &SkillManagerCommandPreview,
) -> Result<(), CommandError> {
    let requested_source = preview
        .source
        .as_deref()
        .ok_or(CommandError::VerificationFailed)?;
    let cwd = Path::new(&preview.cwd);
    let expected_identity = normalized_manager_source_identity(requested_source, cwd)?;
    let global = preview
        .command
        .iter()
        .any(|argument| argument == "--global");
    let lock_path = if global {
        cwd.join(".agents/.skill-lock.json")
    } else {
        cwd.join("skills-lock.json")
    };
    let metadata =
        fs::symlink_metadata(&lock_path).map_err(|_| CommandError::VerificationFailed)?;
    if metadata.file_type().is_symlink()
        || !metadata.is_file()
        || metadata.len() > MAX_MANAGER_LOCK_BYTES
    {
        return Err(CommandError::VerificationFailed);
    }
    let lock: ManagerLockFile = serde_json::from_slice(&fs::read(&lock_path)?)
        .map_err(|_| CommandError::VerificationFailed)?;
    for skill in &preview.skills {
        let entry = lock
            .skills
            .get(skill)
            .or_else(|| {
                lock.skills
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(skill))
                    .map(|(_, entry)| entry)
            })
            .ok_or(CommandError::VerificationFailed)?;
        let installed_source = entry
            .source
            .as_deref()
            .ok_or(CommandError::VerificationFailed)?;
        if normalized_manager_source_identity(installed_source, cwd)? != expected_identity {
            return Err(CommandError::VerificationFailed);
        }
    }
    Ok(())
}

fn normalized_manager_source_identity(source: &str, cwd: &Path) -> Result<String, CommandError> {
    match resolve_manager_source(source, cwd)? {
        ManagerSourceResolution::Local(path) => Ok(format!("local:{}", path.to_string_lossy())),
        ManagerSourceResolution::Network => {
            let source = source.trim().trim_end_matches('/');
            if let Ok(url) = url::Url::parse(source) {
                let host = url
                    .host_str()
                    .ok_or(CommandError::VerificationFailed)?
                    .to_ascii_lowercase();
                let path = url.path().trim_matches('/').trim_end_matches(".git");
                return Ok(format!("network:{host}/{path}"));
            }
            if looks_like_scp_git_source(source) {
                let colon = source.find(':').ok_or(CommandError::VerificationFailed)?;
                let authority = &source[..colon];
                let host = authority
                    .rsplit_once('@')
                    .map_or(authority, |(_, host)| host)
                    .to_ascii_lowercase();
                let path = source[colon + 1..]
                    .trim_matches('/')
                    .trim_end_matches(".git");
                return Ok(format!("network:{host}/{path}"));
            }
            Ok(format!(
                "network:github.com/{}",
                source.trim_matches('/').trim_end_matches(".git")
            ))
        }
    }
}

fn verify_manager_target_transition(
    preview: &SkillManagerCommandPreview,
) -> Result<Vec<PathBuf>, CommandError> {
    let target_preconditions = preview
        .preconditions
        .iter()
        .filter(|precondition| precondition.kind == ActionPreconditionKind::TargetFile)
        .collect::<Vec<_>>();
    if target_preconditions.is_empty() {
        return Err(CommandError::VerificationFailed);
    }
    let changed = target_preconditions
        .into_iter()
        .map(|precondition| {
            let path = PathBuf::from(&precondition.target_id);
            Ok((manager_target_revision(&path)? != precondition.expected_revision).then_some(path))
        })
        .collect::<Result<Vec<_>, CommandError>>()?
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    if changed.is_empty() {
        return Err(CommandError::VerificationFailed);
    }
    Ok(changed)
}

fn verify_local_create_operation(
    catalog: &Catalog,
    requested_name: &str,
    source_path: &Path,
    imported: &crate::ToolGlobalImportResult,
    records: &[SkillRecord],
) -> Result<(), CommandError> {
    let source_skill = source_path.join("SKILL.md");
    if !source_path.is_dir() || !source_skill.is_file() || !imported.imported.path.is_file() {
        return Err(CommandError::VerificationFailed);
    }
    let canonical_source = source_path
        .canonicalize()
        .map_err(|_| CommandError::VerificationFailed)?;
    if Path::new(&imported.source_path) != canonical_source
        || imported.instance_id != imported.imported.id
    {
        return Err(CommandError::VerificationFailed);
    }
    let source_content =
        fs::read_to_string(&source_skill).map_err(|_| CommandError::VerificationFailed)?;
    let imported_content = fs::read_to_string(&imported.imported.path)
        .map_err(|_| CommandError::VerificationFailed)?;
    if source_content != imported_content {
        return Err(CommandError::VerificationFailed);
    }
    let parsed = super::parse_tool_global_skill(&source_content, requested_name);
    if super::canonical_skill_name_suggestion(&parsed.name)
        != super::canonical_skill_name_suggestion(requested_name)
    {
        return Err(CommandError::VerificationFailed);
    }
    let expected_fingerprint =
        super::hash_string(&format!("{}\n---\n{}", parsed.frontmatter_raw, parsed.body));
    let catalog_record = records.iter().find(|record| {
        record.id == imported.imported.id
            && record.agent == AgentId::ToolGlobal.as_str()
            && record.scope == Scope::ToolGlobal.as_str()
            && record.state != "missing"
            && record.path == imported.imported.path
            && record.definition_id == imported.imported.definition_id
            && record.name == imported.imported.name
    });
    if catalog_record.is_none() {
        return Err(CommandError::VerificationFailed);
    }
    let detail = catalog
        .get_skill_detail(&imported.instance_id)?
        .ok_or(CommandError::VerificationFailed)?;
    if detail.id != imported.imported.id
        || detail.agent != AgentId::ToolGlobal.as_str()
        || detail.scope != Scope::ToolGlobal.as_str()
        || detail.path != imported.imported.path
        || detail.definition_id != imported.imported.definition_id
        || detail.name != parsed.name
        || detail.frontmatter_raw != parsed.frontmatter_raw
        || detail.body != parsed.body
        || detail.fingerprint != expected_fingerprint
    {
        return Err(CommandError::VerificationFailed);
    }
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
struct ManagerTargetSnapshot {
    inventory_paths: Vec<PathBuf>,
    skill_paths: Vec<PathBuf>,
}

fn manager_target_snapshot(
    ctx: &AdapterContext,
    command: &[String],
    cwd: &Path,
    operation: &str,
) -> Result<ManagerTargetSnapshot, CommandError> {
    if operation == "search" {
        return Ok(ManagerTargetSnapshot {
            inventory_paths: Vec::new(),
            skill_paths: Vec::new(),
        });
    }
    if operation == "localCreate" {
        let name = command
            .windows(2)
            .find(|window| window[0] == "init")
            .map(|window| safe_skill_name(&window[1]))
            .transpose()?
            .ok_or_else(|| {
                CommandError::InvalidSkillManagerRequest(
                    "localCreate command has no target skill name".to_string(),
                )
            })?;
        return Ok(ManagerTargetSnapshot {
            inventory_paths: Vec::new(),
            skill_paths: vec![cwd.join(name)],
        });
    }

    let global = command.iter().any(|argument| argument == "--global");
    let mut agents = manager_action_agents(command);
    if agents.is_empty() {
        agents = vec![
            AgentId::ClaudeCode,
            AgentId::Codex,
            AgentId::Opencode,
            AgentId::Pi,
            AgentId::Hermes,
            AgentId::Openclaw,
        ];
    }
    let mut skill_paths = agents
        .into_iter()
        .map(|agent| manager_agent_skill_root(ctx, cwd, agent, global))
        .collect::<Vec<_>>();
    sort_dedup_paths(&mut skill_paths);
    let manager_lock = if global {
        ctx.user_home.join(".agents/.skill-lock.json")
    } else {
        cwd.join("skills-lock.json")
    };
    let mut inventory_paths = skill_paths.clone();
    inventory_paths.push(manager_lock);
    sort_dedup_paths(&mut inventory_paths);
    Ok(ManagerTargetSnapshot {
        inventory_paths,
        skill_paths,
    })
}

fn manager_action_targets(
    ctx: &AdapterContext,
    command: &[String],
    cwd: &Path,
    global: bool,
) -> Vec<(AgentId, PathBuf)> {
    let mut agents = manager_action_agents(command);
    if agents.is_empty() {
        agents = vec![
            AgentId::ClaudeCode,
            AgentId::Codex,
            AgentId::Opencode,
            AgentId::Pi,
            AgentId::Hermes,
            AgentId::Openclaw,
        ];
    }
    agents
        .into_iter()
        .map(|agent| (agent, manager_agent_skill_root(ctx, cwd, agent, global)))
        .collect()
}

fn manager_agent_skill_root(
    ctx: &AdapterContext,
    cwd: &Path,
    agent: AgentId,
    global: bool,
) -> PathBuf {
    if global {
        return match agent {
            AgentId::ClaudeCode => claude_config_dir(ctx).join("skills"),
            AgentId::Codex => ctx.user_home.join(".agents/skills"),
            AgentId::Opencode => opencode_user_skills_dir(ctx),
            AgentId::Pi => pi_agent_dir(ctx).join("skills"),
            AgentId::Hermes => hermes_home_dir(ctx).join("skills"),
            AgentId::Openclaw => openclaw_state_dir(ctx).join("skills"),
            _ => ctx.user_home.join(".agents/skills"),
        };
    }
    match agent {
        AgentId::ClaudeCode => cwd.join(".claude/skills"),
        AgentId::Codex | AgentId::Opencode => cwd.join(".agents/skills"),
        AgentId::Pi => cwd.join(".pi/skills"),
        AgentId::Hermes => cwd.join(".hermes/skills"),
        AgentId::Openclaw => cwd.join("skills"),
        _ => cwd.join(".agents/skills"),
    }
}

fn sort_dedup_paths(paths: &mut Vec<PathBuf>) {
    paths.sort();
    paths.dedup();
}

fn manager_inventory_revision(paths: &[PathBuf]) -> Result<String, CommandError> {
    let mut rows = Vec::new();
    for path in paths {
        let metadata = match fs::symlink_metadata(path) {
            Ok(metadata) => Some(metadata),
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let path_text = path.to_string_lossy().to_string();
        let Some(metadata) = metadata else {
            rows.push(format!("missing:{path_text}"));
            continue;
        };
        if metadata.file_type().is_symlink() {
            let target_revision = manager_target_revision(path)?;
            rows.push(format!(
                "symlink:{path_text}:{}:{target_revision}",
                fs::read_link(path)?.to_string_lossy(),
            ));
        } else if metadata.is_file() {
            if metadata.len() > MAX_MANAGER_LOCK_BYTES {
                return Err(CommandError::InvalidSkillManagerRequest(format!(
                    "manager inventory file exceeds the {} byte safety budget: {}",
                    MAX_MANAGER_LOCK_BYTES,
                    path.display()
                )));
            }
            rows.push(format!(
                "file:{path_text}:{}:{:x}",
                metadata.len(),
                Sha256::digest(fs::read(path)?)
            ));
        } else if metadata.is_dir() {
            let mut children = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
            children.sort_by_key(|entry| entry.file_name());
            for child in children {
                let child_path = child.path();
                let child_metadata = fs::symlink_metadata(&child_path)?;
                let name = child.file_name().to_string_lossy().to_string();
                if child_metadata.file_type().is_symlink() {
                    let target_revision = manager_target_revision(&child_path)?;
                    rows.push(format!(
                        "entry:{path_text}:{name}:symlink:{}:{target_revision}",
                        fs::read_link(&child_path)?.to_string_lossy(),
                    ));
                } else if child_metadata.is_dir() {
                    rows.push(format!("entry:{path_text}:{name}:directory"));
                } else if child_metadata.is_file() {
                    rows.push(format!(
                        "entry:{path_text}:{name}:file:{}",
                        child_metadata.len()
                    ));
                } else {
                    return Err(CommandError::UnsafeConfigPath(format!(
                        "manager inventory contains a special file: {}",
                        child_path.display()
                    )));
                }
            }
        } else {
            return Err(CommandError::UnsafeConfigPath(format!(
                "manager inventory target is a special file: {}",
                path.display()
            )));
        }
    }
    rows.sort();
    let rows_json = serde_json::to_string(&rows)?;
    action_source_revision("manager.inventory", &[("entries", &rows_json)])
}

fn manager_source_precondition(
    command: &[String],
    cwd: &Path,
) -> Result<Option<ActionPrecondition>, CommandError> {
    let Some(source) = command
        .windows(2)
        .find(|window| window[0] == "add")
        .map(|window| window[1].as_str())
    else {
        return Ok(None);
    };
    let ManagerSourceResolution::Local(source_path) = resolve_manager_source(source, cwd)? else {
        return Ok(None);
    };
    Ok(Some(ActionPrecondition {
        kind: ActionPreconditionKind::SourceFile,
        target_id: source_path.to_string_lossy().to_string(),
        expected_revision: manager_target_revision(&source_path)?,
    }))
}

fn validate_manager_preconditions(
    ctx: &AdapterContext,
    preview: &SkillManagerCommandPreview,
) -> Result<(), CommandError> {
    validate_manager_preconditions_except(ctx, preview, None)
}

fn validate_manager_preconditions_except(
    ctx: &AdapterContext,
    preview: &SkillManagerCommandPreview,
    excluded_local_skill_path: Option<&Path>,
) -> Result<(), CommandError> {
    if preview.preconditions.is_empty() {
        return Ok(());
    }
    let snapshot = manager_target_snapshot(
        ctx,
        &preview.command,
        Path::new(&preview.cwd),
        &preview.operation,
    )?;
    for precondition in &preview.preconditions {
        if precondition.kind == ActionPreconditionKind::CatalogRecord {
            if excluded_local_skill_path.is_some() {
                continue;
            }
            return Err(CommandError::MismatchedActionReference(
                "manager action contains an unexpected catalog precondition".to_string(),
            ));
        }
        if excluded_local_skill_path.is_some_and(|path| {
            precondition.kind == ActionPreconditionKind::SourceFile
                && Path::new(&precondition.target_id) == path
        }) {
            continue;
        }
        let actual_revision = match precondition.kind {
            ActionPreconditionKind::ManagerInventory => {
                if preview
                    .action
                    .as_ref()
                    .is_some_and(|action| action.target.id != precondition.target_id)
                {
                    return Err(CommandError::MismatchedActionReference(
                        "manager inventory precondition targets another action".to_string(),
                    ));
                }
                manager_inventory_revision(&snapshot.inventory_paths)?
            }
            ActionPreconditionKind::TargetFile | ActionPreconditionKind::SourceFile => {
                manager_target_revision(Path::new(&precondition.target_id))?
            }
            _ => {
                return Err(CommandError::MismatchedActionReference(
                    "manager action contains an unsupported precondition".to_string(),
                ))
            }
        };
        if actual_revision != precondition.expected_revision {
            return Err(CommandError::StaleActionReference);
        }
    }
    Ok(())
}

fn manager_action_binding(
    ctx: &AdapterContext,
    command: &[String],
    cwd: &Path,
    operation: &str,
    network_required: bool,
    network_allowed: bool,
    accepted_revision: Option<&str>,
) -> Result<Option<ActionPreviewBinding>, CommandError> {
    let (kind, intent, preview_method, apply_method) = match operation {
        "search" => (
            ActionKind::RefreshEvidence,
            ActionIntent::InspectEvidence,
            "skillManager.search",
            "skillManager.applySearch",
        ),
        "install" => (
            ActionKind::ManagerInstall,
            ActionIntent::ManagerInstall,
            "skillManager.previewInstall",
            "skillManager.applyInstall",
        ),
        "remove" => (
            ActionKind::ManagerRemove,
            ActionIntent::ManagerRemove,
            "skillManager.previewRemove",
            "skillManager.applyRemove",
        ),
        "update" => (
            ActionKind::ManagerUpdate,
            ActionIntent::ManagerUpdate,
            "skillManager.previewUpdate",
            "skillManager.applyUpdate",
        ),
        "localCreate" => (
            ActionKind::ManagerLocalCreate,
            ActionIntent::ManagerLocalCreate,
            "skillManager.previewLocalCreate",
            "skillManager.applyLocalCreate",
        ),
        _ => return Ok(None),
    };
    let command_json = serde_json::to_string(command)?;
    let cwd_text = cwd.to_string_lossy().to_string();
    let network_required_value = network_required.to_string();
    let network_allowed_value = network_allowed.to_string();
    let env_json = serde_json::to_string(&manager_command_env(ctx, &command[0]))?;
    let source_revision = action_source_revision(
        "manager.accepted-preview",
        &[
            ("operation", operation),
            ("command", &command_json),
            ("cwd", &cwd_text),
            ("network_required", &network_required_value),
            ("network_allowed", &network_allowed_value),
            ("environment", &env_json),
            ("accepted_revision", accepted_revision.unwrap_or("none")),
        ],
    )?;
    let project_id = canonical_project_id(ctx.project_root.as_deref());
    let scope = if command.iter().any(|argument| argument == "--global") {
        Some(Scope::AgentGlobal)
    } else if project_id.is_some() && operation != "localCreate" {
        Some(Scope::AgentProject)
    } else {
        None
    };
    let agents = manager_action_agents(command);
    let target_agent = (agents.len() == 1).then(|| agents[0]);
    let target_identity = action_source_revision(
        "manager.target",
        &[
            ("operation", operation),
            ("command", &command_json),
            ("cwd", &cwd_text),
        ],
    )?;
    let evidence_refs = if agents.is_empty() {
        [
            AgentId::ClaudeCode,
            AgentId::Codex,
            AgentId::Opencode,
            AgentId::Pi,
            AgentId::Hermes,
            AgentId::Openclaw,
        ]
        .into_iter()
        .map(coverage_projection_evidence_id)
        .collect()
    } else {
        agents
            .iter()
            .copied()
            .map(coverage_projection_evidence_id)
            .collect()
    };
    let local_create = operation == "localCreate";
    let discovery = operation == "search";
    let readback = if local_create {
        canonical_readback_domains([
            ActionReadbackDomain::SkillFiles,
            ActionReadbackDomain::CatalogSkills,
        ])
    } else if discovery {
        canonical_readback_domains([ActionReadbackDomain::ManagerInventory])
    } else {
        canonical_readback_domains([
            ActionReadbackDomain::ManagerInventory,
            ActionReadbackDomain::SkillFiles,
            ActionReadbackDomain::CatalogSkills,
        ])
    };
    let impacts = if local_create {
        vec![ActionImpact::SkillFiles, ActionImpact::AppLocalData]
    } else if discovery {
        vec![
            ActionImpact::ReadOnly,
            ActionImpact::ExternalManager,
            ActionImpact::AppLocalData,
        ]
    } else {
        vec![
            ActionImpact::ExternalManager,
            ActionImpact::SkillFiles,
            ActionImpact::AppLocalData,
        ]
    };
    let descriptor = action_descriptor(
        kind,
        intent,
        ActionTargetRef {
            kind: ActionTargetKind::Skill,
            id: format!("manager:{}", target_identity.trim_start_matches("sha256:")),
            agent: target_agent,
            scope,
        },
        project_id,
        impacts,
        preview_method,
        Some(apply_method),
        source_revision,
        true,
        if network_required {
            ActionNetworkPosture::Required
        } else {
            ActionNetworkPosture::None
        },
        readback,
        evidence_refs,
    )?;
    let snapshot = manager_target_snapshot(ctx, command, cwd, operation)?;
    let mut preconditions = Vec::new();
    if !snapshot.inventory_paths.is_empty() {
        preconditions.push(ActionPrecondition {
            kind: ActionPreconditionKind::ManagerInventory,
            target_id: descriptor.target.id.clone(),
            expected_revision: manager_inventory_revision(&snapshot.inventory_paths)?,
        });
    }
    for target in snapshot.skill_paths {
        preconditions.push(ActionPrecondition {
            kind: ActionPreconditionKind::TargetFile,
            target_id: target.to_string_lossy().to_string(),
            expected_revision: manager_target_revision(&target)?,
        });
    }
    if discovery {
        let executable = PathBuf::from(&command[0]);
        preconditions.push(ActionPrecondition {
            kind: ActionPreconditionKind::SourceFile,
            target_id: executable.to_string_lossy().to_string(),
            expected_revision: manager_target_revision(&executable)?,
        });
    }
    if let Some(source) = manager_source_precondition(command, cwd)? {
        preconditions.push(source);
    }
    action_preview_binding(descriptor, preconditions).map(Some)
}

fn manager_target_revision(path: &Path) -> Result<String, CommandError> {
    manager_target_revision_with_depth(path, 0)
}

fn manager_target_revision_with_depth(
    path: &Path,
    symlink_depth: usize,
) -> Result<String, CommandError> {
    if symlink_depth > 16 {
        return Err(CommandError::UnsafeConfigPath(
            "manager target contains an excessive symlink chain".to_string(),
        ));
    }
    let mut entries = Vec::new();
    let mut entry_count = 0_usize;
    let mut total_bytes = 0_u64;
    manager_tree_entries(
        path,
        path,
        &mut entries,
        &mut entry_count,
        &mut total_bytes,
        symlink_depth,
    )?;
    entries.sort();
    let entries_json = serde_json::to_string(&entries)?;
    action_source_revision(
        "manager.target-tree",
        &[
            ("path", &path.to_string_lossy()),
            ("entries", &entries_json),
        ],
    )
}

fn manager_tree_entries(
    root: &Path,
    path: &Path,
    entries: &mut Vec<String>,
    entry_count: &mut usize,
    total_bytes: &mut u64,
    symlink_depth: usize,
) -> Result<(), CommandError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            entries.push("missing:<root>".to_string());
            return Ok(());
        }
        Err(error) => return Err(error.into()),
    };
    *entry_count = entry_count.saturating_add(1);
    if *entry_count > MAX_MANAGER_TARGET_ENTRIES {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "manager target tree exceeds the {}-entry safety budget: {}",
            MAX_MANAGER_TARGET_ENTRIES,
            root.display()
        )));
    }
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .to_string();
    let relative = if relative.is_empty() {
        "<root>".to_string()
    } else {
        relative
    };
    if metadata.file_type().is_symlink() {
        let canonical_target = path.canonicalize().map_err(|_| {
            CommandError::UnsafeConfigPath(
                "manager target contains a dangling or unsafe symlink".to_string(),
            )
        })?;
        let target_revision =
            manager_target_revision_with_depth(&canonical_target, symlink_depth + 1)?;
        entries.push(format!(
            "symlink:{relative}:{}:{target_revision}",
            fs::read_link(path)?.to_string_lossy(),
        ));
        return Ok(());
    }
    if metadata.is_file() {
        *total_bytes = total_bytes.saturating_add(metadata.len());
        if *total_bytes > MAX_MANAGER_TARGET_BYTES {
            return Err(CommandError::InvalidSkillManagerRequest(format!(
                "manager target tree exceeds the {} byte safety budget: {}",
                MAX_MANAGER_TARGET_BYTES,
                root.display()
            )));
        }
        entries.push(format!(
            "file:{relative}:{}:{:x}",
            metadata.len(),
            Sha256::digest(fs::read(path)?)
        ));
        return Ok(());
    }
    if !metadata.is_dir() {
        return Err(CommandError::UnsafeConfigPath(format!(
            "manager target tree contains a special file: {}",
            path.display()
        )));
    }
    entries.push(format!("directory:{relative}"));
    let mut children = fs::read_dir(path)?.collect::<Result<Vec<_>, _>>()?;
    children.sort_by_key(|entry| entry.file_name());
    for child in children {
        manager_tree_entries(
            root,
            &child.path(),
            entries,
            entry_count,
            total_bytes,
            symlink_depth,
        )?;
    }
    Ok(())
}

fn manager_action_agents(command: &[String]) -> Vec<AgentId> {
    let mut agents = command
        .windows(2)
        .filter(|window| window[0] == "--agent")
        .filter_map(|window| match window[1].as_str() {
            "claude-code" => Some(AgentId::ClaudeCode),
            "codex" => Some(AgentId::Codex),
            "opencode" => Some(AgentId::Opencode),
            "pi" => Some(AgentId::Pi),
            "hermes-agent" => Some(AgentId::Hermes),
            "openclaw" => Some(AgentId::Openclaw),
            _ => None,
        })
        .collect::<Vec<_>>();
    agents.sort_by_key(|agent| agent.as_str());
    agents.dedup();
    agents
}

fn manager_env(ctx: &AdapterContext) -> Vec<SkillManagerEnvPreview> {
    vec![
        env_preview("HOME", &ctx.user_home.to_string_lossy()),
        env_preview("LANG", "en_US.UTF-8"),
        env_preview("LC_ALL", "en_US.UTF-8"),
        env_preview("DISABLE_TELEMETRY", "1"),
        env_preview("DO_NOT_TRACK", "1"),
        env_preview("CI", "1"),
        env_preview("npm_config_audit", "false"),
        env_preview("npm_config_fund", "false"),
        env_preview("npm_config_update_notifier", "false"),
    ]
}

fn manager_command_env(ctx: &AdapterContext, executable: &str) -> Vec<SkillManagerEnvPreview> {
    let mut env_vars = manager_env(ctx);
    env_vars.push(env_preview(
        "PATH",
        &manager_command_path(ctx, Path::new(executable)),
    ));
    env_vars
}

fn manager_command_path(ctx: &AdapterContext, executable: &Path) -> String {
    let path_var = env::var_os("PATH");
    let fallback_dirs = fallback_binary_search_dirs_for_home(Some(&ctx.user_home));
    manager_command_path_from_sources(Some(executable), path_var.as_deref(), &fallback_dirs)
}

fn manager_command_path_from_sources(
    executable: Option<&Path>,
    path_var: Option<&std::ffi::OsStr>,
    fallback_dirs: &[PathBuf],
) -> String {
    let mut dirs = Vec::new();
    let mut seen = BTreeSet::new();

    if let Some(parent) = executable
        .and_then(Path::parent)
        .filter(|path| !path.as_os_str().is_empty())
    {
        push_path_dir(&mut dirs, &mut seen, parent.to_path_buf());
    }
    for dir in path_var.into_iter().flat_map(env::split_paths) {
        push_path_dir(&mut dirs, &mut seen, dir);
    }
    for dir in fallback_dirs {
        push_path_dir(&mut dirs, &mut seen, dir.clone());
    }

    env::join_paths(&dirs)
        .ok()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| std::ffi::OsString::from("/usr/bin:/bin"))
        .to_string_lossy()
        .to_string()
}

fn push_path_dir(dirs: &mut Vec<PathBuf>, seen: &mut BTreeSet<PathBuf>, dir: PathBuf) {
    if dir.as_os_str().is_empty() {
        return;
    }
    if seen.insert(dir.clone()) {
        dirs.push(dir);
    }
}

fn env_preview(key: &str, value: &str) -> SkillManagerEnvPreview {
    SkillManagerEnvPreview {
        key: key.to_string(),
        value: value.to_string(),
    }
}

fn npx_executable() -> Result<PathBuf, CommandError> {
    resolve_binary(env::var_os("SKILLS_COPILOT_NPX_PATH"), NPX_BINARY).ok_or_else(|| {
        CommandError::SkillManagerUnavailable(
            "npx executable was not found; install Node/npm or set SKILLS_COPILOT_NPX_PATH"
                .to_string(),
        )
    })
}

fn resolve_binary(override_path: Option<std::ffi::OsString>, binary_name: &str) -> Option<PathBuf> {
    let path_var = env::var_os("PATH");
    let fallback_dirs = fallback_binary_search_dirs();
    resolve_binary_from_sources(
        override_path,
        binary_name,
        path_var.as_deref(),
        &fallback_dirs,
    )
}

fn resolve_binary_from_sources(
    override_path: Option<std::ffi::OsString>,
    binary_name: &str,
    path_var: Option<&std::ffi::OsStr>,
    fallback_dirs: &[PathBuf],
) -> Option<PathBuf> {
    if let Some(path) = override_path
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
    {
        return Some(path);
    }

    path_var
        .into_iter()
        .flat_map(env::split_paths)
        .chain(fallback_dirs.iter().cloned())
        .map(|dir| dir.join(binary_name))
        .find(|candidate| candidate.is_file())
}

fn fallback_binary_search_dirs() -> Vec<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from);
    fallback_binary_search_dirs_for_home(home.as_deref())
}

fn fallback_binary_search_dirs_for_home(home: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/opt/homebrew/sbin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/local/sbin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
        PathBuf::from("/usr/sbin"),
        PathBuf::from("/sbin"),
    ];

    if let Some(home) = home.filter(|path| !path.as_os_str().is_empty()) {
        dirs.extend([
            home.join(".volta/bin"),
            home.join(".asdf/shims"),
            home.join(".local/bin"),
            home.join(".npm-global/bin"),
            home.join(".bun/bin"),
        ]);
        dirs.extend(nvm_node_bin_dirs(home));
    }

    dirs
}

fn nvm_node_bin_dirs(home: &Path) -> Vec<PathBuf> {
    let versions_dir = home.join(".nvm/versions/node");
    let mut dirs = fs::read_dir(versions_dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path().join("bin"))
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    dirs.sort();
    dirs
}

fn default_agent_targets() -> Vec<String> {
    SUPPORTED_MANAGER_AGENTS
        .into_iter()
        .map(ToOwned::to_owned)
        .collect()
}

fn normalize_manager_agents(agents: &[String]) -> Result<Vec<String>, CommandError> {
    let source = if agents.is_empty() {
        default_agent_targets()
    } else {
        agents
            .iter()
            .map(|agent| manager_agent_alias(agent))
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut seen = BTreeSet::new();
    Ok(source
        .into_iter()
        .filter(|agent| seen.insert(agent.clone()))
        .collect())
}

fn required_manager_agents(agents: &[String]) -> Result<Vec<String>, CommandError> {
    if agents.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skill manager mutation requires at least one explicit agent target".to_string(),
        ));
    }
    normalize_manager_agents(agents)
}

fn manager_agent_alias(agent: &str) -> Result<String, CommandError> {
    let normalized = agent.trim().to_ascii_lowercase().replace([' ', '_'], "-");
    let mapped = match normalized.as_str() {
        "claude" | "claude-code" => "claude-code",
        "pi" => "pi",
        "opencode" | "open-code" => "opencode",
        "codex" => "codex",
        "hermes" | "hermes-agent" => "hermes-agent",
        "openclaw" | "open-claw" => "openclaw",
        _ => {
            return Err(CommandError::InvalidSkillManagerRequest(format!(
                "unsupported skill manager agent target: {agent}"
            )))
        }
    };
    Ok(mapped.to_string())
}

fn normalized_skill_names(skills: &[String]) -> Result<Vec<String>, CommandError> {
    let mut names = Vec::new();
    for skill in skills {
        let trimmed = skill.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.contains('\0') {
            return Err(CommandError::InvalidSkillManagerRequest(
                "skill name contains NUL".to_string(),
            ));
        }
        names.push(trimmed.to_string());
    }
    Ok(names)
}

fn append_agent_args(args: &mut Vec<String>, agents: &[String]) {
    for agent in agents {
        args.push("--agent".to_string());
        args.push(agent.clone());
    }
}

fn append_scope_args(args: &mut Vec<String>, scope: Option<&str>) -> Result<(), CommandError> {
    match normalize_manager_scope(scope)?.as_deref() {
        Some("global") => args.push("--global".to_string()),
        Some("project") | None => {}
        Some(_) => unreachable!(),
    }
    Ok(())
}

fn normalize_manager_scope(scope: Option<&str>) -> Result<Option<String>, CommandError> {
    match scope.map(str::trim).filter(|scope| !scope.is_empty()) {
        None => Ok(None),
        Some(scope)
            if scope.eq_ignore_ascii_case("project") || scope == Scope::AgentProject.as_str() =>
        {
            Ok(Some("project".to_string()))
        }
        Some(scope)
            if scope.eq_ignore_ascii_case("global") || scope == Scope::AgentGlobal.as_str() =>
        {
            Ok(Some("global".to_string()))
        }
        Some(other) => Err(CommandError::InvalidSkillManagerRequest(format!(
            "unsupported skill manager scope: {other}"
        ))),
    }
}

fn manager_cwd(ctx: &AdapterContext, scope: Option<&str>) -> Result<PathBuf, CommandError> {
    if normalize_manager_scope(scope)?.as_deref() == Some("global") {
        return Ok(ctx.user_home.clone());
    }
    Ok(ctx
        .project_cwd
        .clone()
        .or_else(|| ctx.project_root.clone())
        .unwrap_or_else(|| ctx.user_home.clone()))
}

#[derive(Debug, Clone, Eq, PartialEq)]
enum ManagerSourceResolution {
    Local(PathBuf),
    Network,
}

fn resolve_manager_source(
    source: &str,
    manager_cwd: &Path,
) -> Result<ManagerSourceResolution, CommandError> {
    let source = source.trim();
    if source.contains('\0') {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skill manager source contains an invalid character".to_string(),
        ));
    }
    if source.contains("://") {
        if source.contains('%') {
            return Err(CommandError::InvalidSkillManagerRequest(
                "skill manager source URLs cannot contain percent-encoded authority or credential data"
                    .to_string(),
            ));
        }
        let url = url::Url::parse(source).map_err(|_| {
            CommandError::InvalidSkillManagerRequest(
                "skill manager source URL is invalid".to_string(),
            )
        })?;
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            return Err(CommandError::InvalidSkillManagerRequest(
                "skill manager source URLs cannot contain userinfo, query, or fragment data"
                    .to_string(),
            ));
        }
        if url.scheme() != "file" {
            return Ok(ManagerSourceResolution::Network);
        }
        let path = url.to_file_path().map_err(|_| {
            CommandError::InvalidSkillManagerRequest(
                "skill manager file URL is not a valid local path".to_string(),
            )
        })?;
        return resolve_local_manager_source(path);
    }

    if looks_like_scp_git_source(source) {
        if source.contains('?')
            || source.contains('#')
            || source.contains('%')
            || !source.starts_with("git@")
        {
            return Err(CommandError::InvalidSkillManagerRequest(
                "skill manager scp sources must use credential-free git@host:path syntax"
                    .to_string(),
            ));
        }
        return Ok(ManagerSourceResolution::Network);
    }

    let path = PathBuf::from(source);
    let candidate = if path.is_absolute() {
        path
    } else {
        manager_cwd.join(path)
    };
    if candidate.exists() {
        return resolve_local_manager_source(candidate);
    }
    if source.starts_with('.') || source.starts_with('/') {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local skill manager source does not exist at the selected manager scope".to_string(),
        ));
    }
    Ok(ManagerSourceResolution::Network)
}

fn looks_like_scp_git_source(source: &str) -> bool {
    if source.contains("://") {
        return false;
    }
    let Some(colon) = source.find(':') else {
        return false;
    };
    if colon == 0 || colon + 1 >= source.len() {
        return false;
    }
    let authority = &source[..colon];
    let path = &source[colon + 1..];
    let valid_authority = authority
        .split_once('@')
        .map_or(!authority.is_empty(), |(user, host)| {
            !user.is_empty() && !host.is_empty() && !host.contains('@')
        });
    valid_authority
        && !authority.contains('/')
        && !authority.contains('\\')
        && !authority.chars().any(char::is_whitespace)
        && !path.chars().any(char::is_whitespace)
        && colon + 1 < source.len()
}

fn resolve_local_manager_source(path: PathBuf) -> Result<ManagerSourceResolution, CommandError> {
    let canonical = path.canonicalize().map_err(|_| {
        CommandError::InvalidSkillManagerRequest(
            "local skill manager source cannot be resolved".to_string(),
        )
    })?;
    let metadata = fs::metadata(&canonical)?;
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local skill manager source is not a regular file or directory".to_string(),
        ));
    }
    Ok(ManagerSourceResolution::Local(canonical))
}

fn install_risks(source: &str, network_required: bool) -> Vec<String> {
    let mut risks = vec![
        "The external manager writes canonical skill files and agent symlinks/copies for selected agents."
            .to_string(),
        "Agent enablement state is not changed; use Agent Enablement toggles separately."
            .to_string(),
    ];
    if network_required {
        risks.push(format!(
            "Source {source} may require network access through npx skills."
        ));
    }
    risks
}

fn parse_search_results(stdout: &str) -> Result<Vec<SkillManagerSearchResult>, CommandError> {
    if let Ok(value) = serde_json::from_str::<Value>(stdout) {
        let items = value
            .as_array()
            .or_else(|| value.get("skills").and_then(Value::as_array))
            .or_else(|| value.get("installed").and_then(Value::as_array))
            .or_else(|| value.get("results").and_then(Value::as_array))
            .ok_or_else(|| {
                CommandError::SkillManagerCommandFailed(
                    "search returned an unrecognized JSON result".to_string(),
                )
            })?;
        if items.iter().any(|item| {
            !item.is_object()
                || string_field(item, &["name", "skill", "id"])
                    .is_none_or(|name| name.trim().is_empty())
        }) {
            return Err(CommandError::SkillManagerCommandFailed(
                "search returned an unrecognized JSON result row".to_string(),
            ));
        }
        return Ok(records_from_json_value(&value)
            .into_iter()
            .map(|record| SkillManagerSearchResult {
                name: record.name,
                source: record.source,
                description: record
                    .raw
                    .get("description")
                    .and_then(Value::as_str)
                    .map(str::to_string),
                raw: record.raw,
            })
            .collect());
    }
    let mut results = Vec::new();
    let mut explicit_empty = false;
    for line in stdout.lines().map(strip_ansi_codes) {
        let line = line.trim();
        if line.eq_ignore_ascii_case("No skills found")
            || line.starts_with("No skills found for ")
            || line.starts_with("No matching skills found")
        {
            explicit_empty = true;
            continue;
        }
        if line.is_empty()
            || line.starts_with("Install with")
            || line.starts_with("npx ")
            || line.starts_with('└')
            || line.starts_with("http://")
            || line.starts_with("https://")
        {
            continue;
        }
        let Some((package, rest)) = line.split_once('@') else {
            continue;
        };
        let package = package.trim();
        let mut skill_and_description = rest.splitn(2, char::is_whitespace);
        let skill = skill_and_description.next().unwrap_or_default().trim();
        if package.is_empty() || skill.is_empty() || !package.contains('/') {
            continue;
        }
        let description = skill_and_description
            .next()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(str::to_string);
        results.push(SkillManagerSearchResult {
            name: skill.to_string(),
            source: Some(package.to_string()),
            description,
            raw: serde_json::json!({
                "name": skill,
                "source": package,
                "raw": line
            }),
        });
    }
    if results.is_empty() && !explicit_empty {
        return Err(CommandError::SkillManagerCommandFailed(
            "search returned no recognizable result collection".to_string(),
        ));
    }
    Ok(results)
}

fn strip_ansi_codes(value: &str) -> String {
    let mut stripped = String::with_capacity(value.len());
    let mut chars = value.chars().peekable();
    while let Some(char) = chars.next() {
        if char == '\u{1b}' {
            if chars.peek() == Some(&'[') {
                chars.next();
                for next in chars.by_ref() {
                    if ('@'..='~').contains(&next) {
                        break;
                    }
                }
            }
            continue;
        }
        stripped.push(char);
    }
    stripped
}

#[cfg(test)]
fn parse_installed_records(stdout: &str) -> Result<Vec<SkillManagerInstalledRecord>, CommandError> {
    let value = serde_json::from_str::<Value>(stdout).map_err(|_| {
        CommandError::SkillManagerCommandFailed(
            "listInstalled returned invalid or truncated JSON".to_string(),
        )
    })?;
    let recognized = value.is_array()
        || ["skills", "installed", "results"]
            .iter()
            .any(|key| value.get(*key).is_some_and(Value::is_array));
    if !recognized {
        return Err(CommandError::SkillManagerCommandFailed(
            "listInstalled returned invalid or truncated JSON".to_string(),
        ));
    }
    let mut records = records_from_json_value(&value);
    for record in &mut records {
        record.agents = record
            .agents
            .iter()
            .filter_map(|agent| manager_agent_alias(agent).ok())
            .collect();
        record.agents.sort();
        record.agents.dedup();
    }
    Ok(records)
}

fn records_from_json_value(value: &Value) -> Vec<SkillManagerInstalledRecord> {
    let items = if let Some(array) = value.as_array() {
        array.clone()
    } else if let Some(array) = value.get("skills").and_then(Value::as_array) {
        array.clone()
    } else if let Some(array) = value.get("installed").and_then(Value::as_array) {
        array.clone()
    } else if let Some(array) = value.get("results").and_then(Value::as_array) {
        array.clone()
    } else {
        Vec::new()
    };
    items
        .into_iter()
        .map(|item| {
            let name = string_field(&item, &["name", "skill", "id"])
                .unwrap_or_else(|| "unknown".to_string());
            let path = string_field(&item, &["path"]);
            SkillManagerInstalledRecord {
                name,
                source: string_field(&item, &["source", "package", "repository", "repo", "url"])
                    .or_else(|| path.clone()),
                // `skills list` inventories local directories too. A row is
                // manager-backed only after the matching scope lock proves it.
                source_kind: "local".to_string(),
                agents: string_array_field(&item, &["agents", "agent_targets", "agentTargets"]),
                scope: string_field(&item, &["scope"]),
                path,
                raw: item,
            }
        })
        .collect()
}

#[derive(Debug, Default, Deserialize)]
struct ManagerLockFile {
    #[serde(default)]
    skills: BTreeMap<String, ManagerLockEntry>,
}

#[derive(Debug, Default, Deserialize)]
struct ManagerLockEntry {
    #[serde(default)]
    source: Option<String>,
    #[serde(default, rename = "sourceType")]
    source_type: Option<String>,
    #[serde(default, rename = "skillPath")]
    skill_path: Option<String>,
}

#[cfg(test)]
fn enrich_installed_records(
    ctx: &AdapterContext,
    scope: Option<&str>,
    records: &mut [SkillManagerInstalledRecord],
) {
    let lock = read_manager_lock(ctx, scope);
    for record in records {
        record.path = record
            .path
            .as_deref()
            .map(|path| redact_command_output(ctx, path));
        record.source = record
            .source
            .as_deref()
            .map(|source| redact_command_output(ctx, source));
        record.source_kind = "local".to_string();
        let entry = lock.as_ref().and_then(|lock| {
            lock.skills.get(&record.name).or_else(|| {
                lock.skills
                    .iter()
                    .find(|(name, _)| name.eq_ignore_ascii_case(&record.name))
                    .map(|(_, entry)| entry)
            })
        });
        let Some(entry) = entry else { continue };
        record.source = entry
            .source
            .as_deref()
            .map(|source| redact_command_output(ctx, source));
        record.source_kind = if entry
            .source_type
            .as_deref()
            .is_some_and(|kind| kind.eq_ignore_ascii_case("local"))
            || entry.source.as_deref().is_some_and(manager_source_is_local)
        {
            "local"
        } else {
            "manager"
        }
        .to_string();
    }
}

#[cfg(test)]
fn read_manager_lock(ctx: &AdapterContext, scope: Option<&str>) -> Option<ManagerLockFile> {
    let normalized_scope = normalize_manager_scope(scope).ok()?;
    let path = if normalized_scope.as_deref() == Some("global") {
        ctx.user_home.join(".agents/.skill-lock.json")
    } else {
        manager_cwd(ctx, scope).ok()?.join("skills-lock.json")
    };
    let metadata = fs::metadata(&path).ok()?;
    if !metadata.is_file() || metadata.len() > MAX_MANAGER_LOCK_BYTES {
        return None;
    }
    serde_json::from_slice(&fs::read(path).ok()?).ok()
}

#[cfg(test)]
fn manager_source_is_local(source: &str) -> bool {
    let source = source.trim();
    source.starts_with('.') || source.starts_with('/') || source.starts_with("file://")
}

fn string_field(value: &Value, keys: &[&str]) -> Option<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_str))
        .map(str::to_string)
}

fn string_array_field(value: &Value, keys: &[&str]) -> Vec<String> {
    keys.iter()
        .find_map(|key| value.get(*key).and_then(Value::as_array))
        .map(|items| {
            items
                .iter()
                .filter_map(Value::as_str)
                .map(str::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn safe_skill_name(name: &str) -> Result<String, CommandError> {
    let trimmed = name.trim();
    let invalid = trimmed.is_empty()
        || trimmed == "."
        || trimmed == ".."
        || trimmed.contains('/')
        || trimmed.contains('\\')
        || trimmed.bytes().any(|byte| byte == 0);
    if invalid {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "invalid local skill name: {name}"
        )));
    }
    Ok(trimmed.to_string())
}

fn local_create_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("local-skill-library").join("sources")
}

fn local_create_source_path(app_data_dir: &Path, name: &str) -> Result<PathBuf, CommandError> {
    Ok(local_create_root(app_data_dir).join(safe_skill_name(name)?))
}

fn preview_token(
    command: &[String],
    cwd: &Path,
    operation: &str,
    network_required: bool,
    network_allowed: bool,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(operation.as_bytes());
    hasher.update(b"\n");
    hasher.update(cwd.to_string_lossy().as_bytes());
    hasher.update(b"\n");
    hasher.update(network_required.to_string().as_bytes());
    hasher.update(b"\n");
    hasher.update(network_allowed.to_string().as_bytes());
    for arg in command {
        hasher.update(b"\n");
        hasher.update(arg.as_bytes());
    }
    format!("skill-manager:{:x}", hasher.finalize())
}

fn redact_command_output(ctx: &AdapterContext, output: &str) -> String {
    let mut redacted = output.replace(&ctx.user_home.to_string_lossy().to_string(), "$HOME");
    if let Some(project_root) = &ctx.project_root {
        redacted = redacted.replace(
            &project_root.to_string_lossy().to_string(),
            "<project-root>",
        );
    }
    if let Some(project_cwd) = &ctx.project_cwd {
        redacted = redacted.replace(&project_cwd.to_string_lossy().to_string(), "<project-cwd>");
    }
    redact_sensitive_url_tokens(&redacted)
}

fn redact_sensitive_url_tokens(output: &str) -> String {
    fn is_boundary(byte: u8) -> bool {
        byte.is_ascii_whitespace()
            || byte.is_ascii_control()
            || matches!(
                byte,
                b'"' | b'\'' | b'<' | b'>' | b'(' | b')' | b'[' | b']' | b'{' | b'}' | b'|' | b'\\'
            )
    }

    let bytes = output.as_bytes();
    let mut ranges = Vec::new();
    for (separator, _) in output.match_indices("://") {
        let mut start = separator;
        while start > 0 && !is_boundary(bytes[start - 1]) && bytes[start - 1] != b'=' {
            start -= 1;
        }
        let mut end = separator + 3;
        while end < bytes.len() && !is_boundary(bytes[end]) {
            end += 1;
        }
        let candidate = &output[start..end];
        let Ok(url) = url::Url::parse(candidate) else {
            continue;
        };
        if !url.username().is_empty()
            || url.password().is_some()
            || url.query().is_some()
            || url.fragment().is_some()
        {
            ranges.push((start, end));
        }
    }
    let mut token_start = 0;
    while token_start < bytes.len() {
        while token_start < bytes.len() && is_boundary(bytes[token_start]) {
            token_start += 1;
        }
        let mut token_end = token_start;
        while token_end < bytes.len() && !is_boundary(bytes[token_end]) {
            token_end += 1;
        }
        if token_start < token_end {
            let candidate = &output[token_start..token_end];
            if looks_like_scp_git_source(candidate)
                && (!candidate.starts_with("git@")
                    || candidate.contains('?')
                    || candidate.contains('#')
                    || candidate.contains('%'))
            {
                ranges.push((token_start, token_end));
            }
        }
        token_start = token_end.saturating_add(1);
    }
    if ranges.is_empty() {
        return output.to_string();
    }

    ranges.sort_unstable();
    ranges.dedup();
    let mut redacted = String::with_capacity(output.len());
    let mut cursor = 0;
    for (start, end) in ranges {
        if start < cursor {
            continue;
        }
        redacted.push_str(&output[cursor..start]);
        redacted.push_str("<redacted-source-url>");
        cursor = end;
    }
    redacted.push_str(&output[cursor..]);
    redacted
}

fn truncate_capture(value: &str) -> String {
    if value.len() <= MAX_CAPTURE_BYTES {
        return value.to_string();
    }
    let mut boundary = MAX_CAPTURE_BYTES;
    while !value.is_char_boundary(boundary) {
        boundary -= 1;
    }
    let mut truncated = value[..boundary].to_string();
    truncated.push_str("\n<truncated>");
    truncated
}

fn failed_command_detail(stdout: &str, stderr: &str) -> String {
    let stderr = strip_ansi_codes(stderr).trim().to_string();
    if !stderr.is_empty() {
        return stderr;
    }
    let stdout = strip_ansi_codes(stdout).trim().to_string();
    if !stdout.is_empty() {
        return stdout;
    }
    "no output captured from external skills manager".to_string()
}

fn unix_timestamp_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as i64
}

#[cfg(test)]
mod commit_fault_tests;

#[cfg(test)]
mod effects_tests;

#[cfg(test)]
mod tests {
    use super::*;

    fn semantic_test_preview(operation: &str, skills: Vec<&str>) -> SkillManagerCommandPreview {
        SkillManagerCommandPreview {
            action: None,
            preconditions: Vec::new(),
            tool_id: DEFAULT_MANAGER_TOOL.to_string(),
            operation: operation.to_string(),
            command: vec!["/usr/bin/true".to_string(), "--global".to_string()],
            cwd: "/tmp".to_string(),
            env: Vec::new(),
            requires_confirmation: true,
            confirmed: true,
            network_required: false,
            network_allowed: true,
            will_run: true,
            preview_token: "test".to_string(),
            summary: "test".to_string(),
            risks: Vec::new(),
            source: None,
            skills: skills.into_iter().map(str::to_string).collect(),
        }
    }

    fn semantic_test_catalog_record(
        id: &str,
        agent: AgentId,
        name: &str,
        path: PathBuf,
    ) -> SkillRecord {
        SkillRecord {
            id: id.to_string(),
            agent: agent.as_str().to_string(),
            scope: Scope::AgentGlobal.as_str().to_string(),
            path: path.clone(),
            display_path: path,
            definition_id: id.to_string(),
            name: name.to_string(),
            state: "loaded".to_string(),
            enabled: true,
            publisher: None,
            package_name: None,
            package_version: None,
            source_kind: None,
            read_only_reason: None,
        }
    }

    #[test]
    fn machine_stdout_capture_is_private_and_removed_on_drop() {
        #[cfg(unix)]
        use std::os::unix::fs::PermissionsExt;

        let path = {
            let capture = MachineStdoutCapture::create().expect("private machine capture");
            let path = capture.path.clone();
            #[cfg(unix)]
            assert_eq!(
                fs::metadata(&path)
                    .expect("capture metadata")
                    .permissions()
                    .mode()
                    & 0o777,
                0o600
            );
            assert!(path.is_file());
            path
        };
        assert!(!path.exists(), "capture should be removed by RAII");
    }

    #[test]
    fn resolve_binary_prefers_explicit_override_without_validation() {
        let override_path = PathBuf::from("/custom/node/bin/npx");
        let resolved = resolve_binary_from_sources(
            Some(override_path.as_os_str().to_os_string()),
            NPX_BINARY,
            None,
            &[],
        );

        assert_eq!(resolved, Some(override_path));
    }

    #[test]
    fn resolve_binary_falls_back_to_common_gui_launch_paths() {
        let temp =
            std::env::temp_dir().join(format!("skill-manager-npx-path-{}", std::process::id()));
        let empty_path_dir = temp.join("empty-path");
        let fallback_dir = temp.join("homebrew-bin");
        fs::create_dir_all(&empty_path_dir).expect("empty path dir");
        fs::create_dir_all(&fallback_dir).expect("fallback dir");
        let npx = fallback_dir.join(NPX_BINARY);
        fs::write(&npx, "#!/bin/sh\n").expect("fake npx");

        let resolved = resolve_binary_from_sources(
            None,
            NPX_BINARY,
            Some(empty_path_dir.as_os_str()),
            &[fallback_dir],
        );

        assert_eq!(resolved, Some(npx));
        fs::remove_dir_all(temp).ok();
    }

    #[test]
    fn manager_command_path_keeps_node_visible_for_env_shebangs() {
        let executable = PathBuf::from("/custom/node/bin/npx");
        let fallback_dir = PathBuf::from("/opt/homebrew/bin");
        let path = manager_command_path_from_sources(
            Some(&executable),
            Some(std::ffi::OsStr::new("/usr/bin:/custom/node/bin")),
            &[fallback_dir.clone(), PathBuf::from("/usr/bin")],
        );
        let dirs = env::split_paths(std::ffi::OsStr::new(&path)).collect::<Vec<_>>();

        assert_eq!(dirs.first(), Some(&PathBuf::from("/custom/node/bin")));
        assert!(dirs.contains(&fallback_dir));
        assert_eq!(
            dirs.iter()
                .filter(|dir| dir.as_path() == Path::new("/custom/node/bin"))
                .count(),
            1
        );
        assert_eq!(
            dirs.iter()
                .filter(|dir| dir.as_path() == Path::new("/usr/bin"))
                .count(),
            1
        );
    }

    #[test]
    fn preview_and_runtime_env_share_the_same_allowlisted_keys() {
        let temp =
            std::env::temp_dir().join(format!("skill-manager-command-env-{}", std::process::id()));
        let ctx = AdapterContext {
            user_home: temp.join("home"),
            project_cwd: Some(temp.join("project")),
            project_root: Some(temp.join("project")),
            extra_roots: Vec::new(),
        };
        let runtime_env = manager_command_env(&ctx, "/custom/node/bin/npx");

        let preview_env = manager_command_env(&ctx, "/custom/node/bin/npx");
        assert!(
            runtime_env
                .iter()
                .any(|env_var| env_var.key == "PATH" && env_var.value.contains("/custom/node/bin")),
            "Runtime command env should make node visible to /usr/bin/env shebangs."
        );
        assert_eq!(
            preview_env
                .iter()
                .map(|env_var| env_var.key.as_str())
                .collect::<BTreeSet<_>>(),
            runtime_env
                .iter()
                .map(|env_var| env_var.key.as_str())
                .collect::<BTreeSet<_>>()
        );
    }

    #[test]
    fn default_agents_cover_supported_app_agents() {
        assert_eq!(
            normalize_manager_agents(&[]).expect("default agents"),
            vec![
                "claude-code",
                "pi",
                "opencode",
                "codex",
                "hermes-agent",
                "openclaw"
            ]
        );
    }

    #[test]
    fn install_preview_uses_symlink_by_default_and_copy_only_when_requested() {
        let temp =
            std::env::temp_dir().join(format!("skill-manager-preview-{}", std::process::id()));
        let ctx = AdapterContext {
            user_home: temp.join("home"),
            project_cwd: Some(temp.join("project")),
            project_root: Some(temp.join("project")),
            extra_roots: Vec::new(),
        };
        let params = SkillManagerInstallParams {
            source: "vercel-labs/agent-skills".to_string(),
            skills: vec!["frontend-design".to_string()],
            agents: SUPPORTED_MANAGER_AGENTS
                .iter()
                .map(|agent| (*agent).to_string())
                .collect(),
            scope: Some("project".to_string()),
            distribution: None,
            network_allowed: true,
            confirmed: false,
            preview_token: None,
            action_reference: None,
        };
        let preview = build_install_preview(&ctx, &params).expect("preview");
        assert!(preview.command.contains(&"--skill".to_string()));
        assert!(
            preview.command.contains(&"--full-depth".to_string()),
            "installing a named skill from search results should search nested package directories"
        );
        assert!(!preview.command.contains(&"--copy".to_string()));
        assert_eq!(
            preview
                .command
                .iter()
                .filter(|arg| arg.as_str() == "--agent")
                .count(),
            SUPPORTED_MANAGER_AGENTS.len()
        );

        let copy_preview = build_install_preview(
            &ctx,
            &SkillManagerInstallParams {
                distribution: Some("copy".to_string()),
                ..params
            },
        )
        .expect("copy preview");
        assert!(copy_preview.command.contains(&"--copy".to_string()));
    }

    #[test]
    fn install_preview_resolves_relative_local_sources_from_the_manager_cwd() {
        crate::initialize_action_preview_secret_for_test([0xA5; 32])
            .expect("initialize action preview test secret");
        let temp =
            std::env::temp_dir().join(format!("skill-manager-local-source-{}", std::process::id()));
        let project = temp.join("project");
        let local_source = project.join("local-source");
        fs::create_dir_all(&local_source).expect("create local source");
        fs::write(local_source.join("SKILL.md"), "# Local").expect("write local source");
        let ctx = AdapterContext {
            user_home: temp.join("home"),
            project_cwd: Some(project.clone()),
            project_root: Some(project.clone()),
            extra_roots: Vec::new(),
        };
        let params = SkillManagerInstallParams {
            source: "local-source".to_string(),
            skills: vec!["local-source".to_string()],
            agents: vec!["codex".to_string()],
            scope: Some("project".to_string()),
            distribution: None,
            network_allowed: false,
            confirmed: false,
            preview_token: None,
            action_reference: None,
        };

        let preview = build_install_preview(&ctx, &params).expect("local preview");
        let canonical_source = local_source.canonicalize().expect("canonical local source");

        assert!(!preview.network_required);
        assert!(preview.preconditions.iter().any(|precondition| {
            precondition.kind == ActionPreconditionKind::SourceFile
                && Path::new(&precondition.target_id) == canonical_source
        }));
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn install_preview_rejects_credential_bearing_source_urls_without_echoing_them() {
        crate::initialize_action_preview_secret_for_test([0xA5; 32])
            .expect("initialize action preview test secret");
        let temp = std::env::temp_dir().join(format!(
            "skill-manager-sensitive-source-{}",
            std::process::id()
        ));
        let project = temp.join("project");
        fs::create_dir_all(&project).expect("create project");
        let ctx = AdapterContext {
            user_home: temp.join("home"),
            project_cwd: Some(project.clone()),
            project_root: Some(project),
            extra_roots: Vec::new(),
        };
        let sensitive = "https://user:secret@example.com/repo.git?token=abc#private";
        let params = SkillManagerInstallParams {
            source: sensitive.to_string(),
            skills: Vec::new(),
            agents: vec!["codex".to_string()],
            scope: Some("project".to_string()),
            distribution: None,
            network_allowed: true,
            confirmed: false,
            preview_token: None,
            action_reference: None,
        };

        let error = match build_install_preview(&ctx, &params) {
            Err(error) => error,
            Ok(_) => panic!("sensitive URL must be rejected before preview construction"),
        };
        let message = error.to_string();

        assert!(matches!(error, CommandError::InvalidSkillManagerRequest(_)));
        assert!(!message.contains("user:secret"));
        assert!(!message.contains("secret"));
        assert!(!message.contains("token=abc"));
        let _ = fs::remove_dir_all(temp);
    }

    #[test]
    fn failed_command_error_uses_stdout_when_stderr_is_empty() {
        let stderr = "";
        let stdout = "\u{1b}[31mNo matching skills found for: alibabacloud-find-skills\u{1b}[0m";

        let detail = failed_command_detail(stdout, stderr);

        assert_eq!(
            detail,
            "No matching skills found for: alibabacloud-find-skills"
        );
    }

    #[test]
    fn command_output_redacts_credential_bearing_urls() {
        let ctx = AdapterContext {
            user_home: PathBuf::from("/tmp/example-home"),
            project_cwd: None,
            project_root: None,
            extra_roots: Vec::new(),
        };
        let output =
            "failed to fetch https://user:secret@example.com/repo.git?token=abc#private safely";

        let redacted = redact_command_output(&ctx, output);

        assert_eq!(redacted, "failed to fetch <redacted-source-url> safely");
        assert!(!redacted.contains("user"));
        assert!(!redacted.contains("secret"));
        assert!(!redacted.contains("token=abc"));
        assert!(!redacted.contains("private"));

        let scp_redacted =
            redact_command_output(&ctx, "failed user:secret@example.com:owner/repo.git");
        assert_eq!(
            scp_redacted, "failed <redacted-source-url>",
            "credential-shaped SCP output must be removed as one token"
        );
        for output in [
            "failed host:owner/repo.git?token=secret",
            "failed host:owner/repo.git#secret",
            "failed git@example.com:owner/repo.git?token=secret",
        ] {
            let redacted = redact_command_output(&ctx, output);
            assert_eq!(redacted, "failed <redacted-source-url>");
            assert!(!redacted.contains("secret"));
        }
    }

    #[test]
    fn codex_manager_global_target_uses_shared_agents_root() {
        let ctx = AdapterContext {
            user_home: PathBuf::from("/tmp/agent-copilot-manager-home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        };

        assert_eq!(
            manager_agent_skill_root(&ctx, &ctx.user_home, AgentId::Codex, true,),
            ctx.user_home.join(".agents/skills")
        );
    }

    #[cfg(unix)]
    #[test]
    fn manager_target_revision_tracks_symlink_target_content() {
        use std::os::unix::fs::symlink;

        let temp_root = std::env::temp_dir().join(format!(
            "agent-copilot-manager-symlink-revision-{}",
            std::process::id()
        ));
        let source = temp_root.join("source/linked-skill");
        let target_root = temp_root.join("target");
        fs::create_dir_all(&source).expect("create source");
        fs::create_dir_all(&target_root).expect("create target");
        fs::write(
            source.join("SKILL.md"),
            "---\nname: linked-skill\ndescription: before\n---\nbefore\n",
        )
        .expect("write source");
        symlink(&source, target_root.join("linked-skill")).expect("link skill");

        let before = manager_target_revision(&target_root).expect("revision before");
        fs::write(
            source.join("SKILL.md"),
            "---\nname: linked-skill\ndescription: after\n---\nafter\n",
        )
        .expect("change source");
        let after = manager_target_revision(&target_root).expect("revision after");

        assert_ne!(before, after);
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[cfg(unix)]
    #[test]
    fn manager_inventory_revision_tracks_symlinked_lock_target_content() {
        use std::os::unix::fs::symlink;

        let temp_root = std::env::temp_dir().join(format!(
            "agent-copilot-manager-lock-symlink-revision-{}",
            std::process::id()
        ));
        fs::create_dir_all(&temp_root).expect("create fixture root");
        let lock_target = temp_root.join("real-skill-lock.json");
        let lock_link = temp_root.join(".skill-lock.json");
        fs::write(&lock_target, "{\"source\":\"before\"}").expect("write lock target");
        symlink(&lock_target, &lock_link).expect("link manager lock");

        let before =
            manager_inventory_revision(std::slice::from_ref(&lock_link)).expect("revision before");
        fs::write(&lock_target, "{\"source\":\"after!\"}").expect("change lock target");
        let after =
            manager_inventory_revision(std::slice::from_ref(&lock_link)).expect("revision after");

        assert_ne!(before, after);
        let _ = fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn manager_source_rejects_credential_shaped_scp_and_encoded_urls() {
        let cwd = Path::new("/tmp");
        for source in [
            "user:secret@example.com:owner/repo.git",
            "host:owner/repo.git?token=secret",
            "host:owner/repo.git#token",
            "git@example.com:owner/repo.git?token=secret",
            "https://example.com/%40token/repo.git",
            "https://example.com/owner/repo.git?token=secret",
            "https://example.com/owner/repo.git#token",
        ] {
            let error = resolve_manager_source(source, cwd)
                .expect_err("credential-shaped source must fail closed");
            assert!(matches!(error, CommandError::InvalidSkillManagerRequest(_)));
            assert!(!error.to_string().contains("secret"));
            assert!(!error.to_string().contains("token"));
        }
        assert_eq!(
            resolve_manager_source("git@example.com:owner/repo.git", cwd)
                .expect("standard scp git source"),
            ManagerSourceResolution::Network
        );
    }

    #[test]
    fn multi_agent_install_allows_an_unchanged_valid_target_when_another_target_is_added() {
        let temp_root = std::env::temp_dir().join(format!(
            "agent-copilot-manager-mixed-install-{}-{}",
            std::process::id(),
            unix_timestamp_millis()
        ));
        let existing_file = temp_root.join("claude/shared/SKILL.md");
        let added_file = temp_root.join("codex/shared/SKILL.md");
        fs::create_dir_all(existing_file.parent().expect("existing parent"))
            .expect("create existing target");
        fs::create_dir_all(added_file.parent().expect("added parent"))
            .expect("create added target");
        fs::write(&existing_file, "existing").expect("write existing target");
        fs::write(&added_file, "added").expect("write added target");
        let existing_file = existing_file.canonicalize().expect("canonical existing");
        let added_file = added_file.canonicalize().expect("canonical added");
        let before = vec![
            ManagerSelectedSkillState {
                agent: AgentId::ClaudeCode,
                root: temp_root.join("claude"),
                skill: "shared".to_string(),
                exists: true,
                canonical_skill_file: Some(existing_file.clone()),
                source_identity: Some("claude-source".to_string()),
                content_fingerprint: Some("same-content".to_string()),
            },
            ManagerSelectedSkillState {
                agent: AgentId::Codex,
                root: temp_root.join("codex"),
                skill: "shared".to_string(),
                exists: false,
                canonical_skill_file: None,
                source_identity: None,
                content_fingerprint: None,
            },
        ];
        let after = vec![
            before[0].clone(),
            ManagerSelectedSkillState {
                agent: AgentId::Codex,
                root: temp_root.join("codex"),
                skill: "shared".to_string(),
                exists: true,
                canonical_skill_file: Some(added_file.clone()),
                source_identity: Some("codex-source".to_string()),
                content_fingerprint: Some("same-content".to_string()),
            },
        ];
        let records = vec![
            semantic_test_catalog_record(
                "claude-shared",
                AgentId::ClaudeCode,
                "shared",
                existing_file,
            ),
            semantic_test_catalog_record("codex-shared", AgentId::Codex, "shared", added_file),
        ];
        fs::create_dir_all(temp_root.join(".agents")).expect("create manager state root");
        fs::write(
            temp_root.join(".agents/.skill-lock.json"),
            r#"{"version":3,"skills":{"shared":{"source":"owner/repository","sourceType":"github"}}}"#,
        )
        .expect("write manager lock");
        let mut preview = semantic_test_preview("install", vec!["shared"]);
        preview.cwd = temp_root.to_string_lossy().to_string();
        preview.source = Some("https://github.com/owner/repository.git".to_string());

        verify_manager_operation(&preview, &records, &before, &after)
            .expect("all selected postconditions are valid and another target changed");
        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn install_rejects_postconditions_owned_by_a_different_manager_source() {
        let temp_root = std::env::temp_dir().join(format!(
            "agent-copilot-manager-wrong-install-source-{}-{}",
            std::process::id(),
            unix_timestamp_millis()
        ));
        let skill_file = temp_root.join("codex/shared/SKILL.md");
        fs::create_dir_all(skill_file.parent().expect("skill parent"))
            .expect("create selected target");
        fs::write(&skill_file, "installed").expect("write selected target");
        let skill_file = skill_file.canonicalize().expect("canonical skill");
        fs::create_dir_all(temp_root.join(".agents")).expect("create manager state root");
        fs::write(
            temp_root.join(".agents/.skill-lock.json"),
            r#"{"version":3,"skills":{"shared":{"source":"other/repository","sourceType":"github"}}}"#,
        )
        .expect("write mismatched manager lock");
        let before = vec![ManagerSelectedSkillState {
            agent: AgentId::Codex,
            root: temp_root.join("codex"),
            skill: "shared".to_string(),
            exists: false,
            canonical_skill_file: None,
            source_identity: None,
            content_fingerprint: None,
        }];
        let after = vec![ManagerSelectedSkillState {
            agent: AgentId::Codex,
            root: temp_root.join("codex"),
            skill: "shared".to_string(),
            exists: true,
            canonical_skill_file: Some(skill_file.clone()),
            source_identity: Some("different-source".to_string()),
            content_fingerprint: Some("installed".to_string()),
        }];
        let records = vec![semantic_test_catalog_record(
            "codex-shared",
            AgentId::Codex,
            "shared",
            skill_file,
        )];
        let mut preview = semantic_test_preview("install", vec!["shared"]);
        preview.cwd = temp_root.to_string_lossy().to_string();
        preview.source = Some("owner/repository".to_string());

        assert!(matches!(
            verify_manager_operation(&preview, &records, &before, &after),
            Err(CommandError::VerificationFailed)
        ));
        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn update_rejects_a_changed_skill_that_swaps_source_identity() {
        let temp_root = std::env::temp_dir().join(format!(
            "agent-copilot-manager-source-swap-{}-{}",
            std::process::id(),
            unix_timestamp_millis()
        ));
        let skill_file = temp_root.join("codex/selected/SKILL.md");
        fs::create_dir_all(skill_file.parent().expect("skill parent"))
            .expect("create selected target");
        fs::write(&skill_file, "after").expect("write selected target");
        let skill_file = skill_file.canonicalize().expect("canonical skill");
        let before = vec![ManagerSelectedSkillState {
            agent: AgentId::Codex,
            root: temp_root.join("codex"),
            skill: "selected".to_string(),
            exists: true,
            canonical_skill_file: Some(skill_file.clone()),
            source_identity: Some("expected-source".to_string()),
            content_fingerprint: Some("before".to_string()),
        }];
        let after = vec![ManagerSelectedSkillState {
            source_identity: Some("different-source".to_string()),
            content_fingerprint: Some("after".to_string()),
            ..before[0].clone()
        }];
        let records = vec![semantic_test_catalog_record(
            "codex-selected",
            AgentId::Codex,
            "selected",
            skill_file,
        )];

        assert!(matches!(
            verify_manager_operation(
                &semantic_test_preview("update", vec!["selected"]),
                &records,
                &before,
                &after,
            ),
            Err(CommandError::VerificationFailed)
        ));
        let _ = fs::remove_dir_all(temp_root);
    }

    #[test]
    fn unrelated_target_tree_change_cannot_satisfy_selected_skill_update() {
        let temp_root = std::env::temp_dir().join(format!(
            "agent-copilot-manager-unrelated-update-{}-{}",
            std::process::id(),
            unix_timestamp_millis()
        ));
        let home = temp_root.join("home");
        let selected = home.join(".agents/skills/selected/SKILL.md");
        let unrelated = home.join(".agents/skills/unrelated/SKILL.md");
        fs::create_dir_all(selected.parent().expect("selected parent")).expect("create selected");
        fs::create_dir_all(unrelated.parent().expect("unrelated parent"))
            .expect("create unrelated");
        fs::write(
            &selected,
            "---\nname: selected\ndescription: selected\n---\nbefore\n",
        )
        .expect("write selected");
        fs::write(
            &unrelated,
            "---\nname: unrelated\ndescription: unrelated\n---\nbefore\n",
        )
        .expect("write unrelated");
        let ctx = AdapterContext {
            user_home: home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        };
        let preview = SkillManagerCommandPreview {
            action: None,
            preconditions: Vec::new(),
            tool_id: DEFAULT_MANAGER_TOOL.to_string(),
            operation: "update".to_string(),
            command: vec![
                "/usr/bin/true".to_string(),
                "--global".to_string(),
                "--agent".to_string(),
                "codex".to_string(),
            ],
            cwd: home.to_string_lossy().to_string(),
            env: Vec::new(),
            requires_confirmation: true,
            confirmed: true,
            network_required: false,
            network_allowed: true,
            will_run: true,
            preview_token: "test".to_string(),
            summary: "test".to_string(),
            risks: Vec::new(),
            source: None,
            skills: vec!["selected".to_string()],
        };
        let before = manager_selected_skill_snapshot(&ctx, &preview).expect("before snapshot");
        fs::write(
            &unrelated,
            "---\nname: unrelated\ndescription: unrelated\n---\nafter\n",
        )
        .expect("change only unrelated skill");
        let after = manager_selected_skill_snapshot(&ctx, &preview).expect("after snapshot");
        let records = vec![SkillRecord {
            id: "selected".to_string(),
            agent: AgentId::Codex.as_str().to_string(),
            scope: Scope::AgentGlobal.as_str().to_string(),
            path: selected.canonicalize().expect("canonical selected"),
            display_path: selected.clone(),
            definition_id: "selected".to_string(),
            name: "selected".to_string(),
            state: "loaded".to_string(),
            enabled: true,
            publisher: None,
            package_name: None,
            package_version: None,
            source_kind: None,
            read_only_reason: None,
        }];

        assert!(matches!(
            verify_manager_operation(&preview, &records, &before, &after),
            Err(CommandError::VerificationFailed)
        ));
        let _ = fs::remove_dir_all(temp_root);
    }
}
