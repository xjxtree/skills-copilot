use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    fs::{File, OpenOptions},
    io::{ErrorKind, Read, Seek, SeekFrom},
    path::{Path, PathBuf},
    process::{Command, Stdio},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use skills_copilot_adapters::{
    claude_config_dir, codex_home_dir, hermes_home_dir, openclaw_state_dir,
    opencode_user_skills_dir, pi_agent_dir,
};
use skills_copilot_catalog::{Catalog, SkillEventDraft, SkillRecord};
use skills_copilot_core::{
    AdapterContext, AgentId, ListIncompleteReason, ListPageMetadata, ListSourceCompleteness, Scope,
};

use crate::{
    import_local_skill_to_tool_global, scan_all_catalog_report, tool_global_staging_skills_root,
    CommandError,
};

mod archive;
mod local_source;
pub use archive::{
    apply_local_archive_import, apply_local_archive_update, preview_local_archive_import,
    preview_local_archive_update, SkillManagerLocalArchiveImportParams,
    SkillManagerLocalArchiveImportRecord, SkillManagerLocalArchiveUpdateParams,
    SkillManagerLocalArchiveUpdateRecord,
};
#[cfg(all(test, unix))]
use local_source::inspect_local_source_with_executable;
pub use local_source::{
    inspect_local_source_with_manager, SkillManagerInspectLocalSourceParams,
    SkillManagerLocalSourceInspectionRecord, SkillManagerLocalSourceSkillRecord,
};

const DEFAULT_MANAGER_TOOL: &str = "npx-skills";
const SKILLS_NPM_TOOL: &str = "skills-npm";
const SKILLS_CLI_BINARY: &str = "skills";
const NPX_BINARY: &str = "npx";
const MAX_MACHINE_OUTPUT_BYTES: usize = 4 * 1024 * 1024;
const MAX_CAPTURE_BYTES: usize = MAX_MACHINE_OUTPUT_BYTES;
const MAX_MANAGER_LOCK_BYTES: u64 = 2 * 1024 * 1024;
const MAX_REMOVAL_ENTRY_COUNT: usize = 10_000;

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
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub separable_agents: Option<Vec<String>>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerRemoveParams {
    pub skill: String,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub instance_ids: Vec<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub full_uninstall: bool,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub preview_token: Option<String>,
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
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerLocalCreateParams {
    pub name: String,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub preview_token: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerDeleteLocalParams {
    pub instance_id: String,
    #[serde(default)]
    pub confirmed: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillManagerMutationRecord {
    pub preview: SkillManagerCommandPreview,
    pub output: Option<SkillManagerCommandOutput>,
    pub applied: bool,
    pub scanned_count: usize,
    pub updated_skills: Vec<SkillRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub removal_plan: Option<SkillManagerRemovalPlan>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillManagerRemovalPlan {
    pub mode: String,
    pub full_uninstall: bool,
    pub selected_agents: Vec<String>,
    pub instance_ids: Vec<String>,
    pub source_preserved: bool,
    pub actions: Vec<SkillManagerRemovalAction>,
    pub verification: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillManagerRemovalAction {
    pub instance_id: String,
    pub agent: String,
    pub scope: String,
    pub strategy: String,
    pub target: String,
    pub summary: String,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum PhysicalRemovalEntryKind {
    Symlink,
    Directory,
}

impl PhysicalRemovalEntryKind {
    fn strategy(self) -> &'static str {
        match self {
            Self::Symlink => "remove-symlink",
            Self::Directory => "remove-copy-directory",
        }
    }

    fn binding_name(self) -> &'static str {
        match self {
            Self::Symlink => "symlink",
            Self::Directory => "directory",
        }
    }
}

#[derive(Debug, Clone)]
struct PhysicalRemovalTarget {
    instance_ids: Vec<String>,
    agents: Vec<String>,
    entry_path: PathBuf,
    kind: PhysicalRemovalEntryKind,
    revision: String,
}

#[derive(Debug, Clone)]
struct PhysicalRemovalPlan {
    targets: Vec<PhysicalRemovalTarget>,
    preserved_paths: Vec<PathBuf>,
}

#[derive(Debug)]
struct StagedRemovalTarget {
    original_path: PathBuf,
    backup_path: PathBuf,
    backup_root: PathBuf,
    kind: PhysicalRemovalEntryKind,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct SkillManagerLocalCreateRecord {
    pub preview: SkillManagerCommandPreview,
    pub output: Option<SkillManagerCommandOutput>,
    pub imported: Option<SkillRecord>,
    pub instance_id: Option<String>,
    pub source_path: String,
    pub applied: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillManagerLocalDeleteRecord {
    pub instance_id: String,
    pub skill_name: String,
    pub path: String,
    pub app_owned: bool,
    pub physical_delete_allowed: bool,
    pub blocked_by_references: Vec<SkillManagerReferenceRecord>,
    pub confirmed: bool,
    pub deleted: bool,
    pub summary: String,
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
                "listInstalled",
                "inspectLocalSource",
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
                "Local folders are inspected without installation and remain in place."
                    .to_string(),
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

pub fn search_skills_with_manager(
    ctx: &AdapterContext,
    params: &SkillManagerSearchParams,
) -> Result<SkillManagerSearchRecord, CommandError> {
    let query = params.query.trim();
    if query.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skillManager.search requires a non-empty query".to_string(),
        ));
    }
    let mut args = vec![
        SKILLS_CLI_BINARY.to_string(),
        "find".to_string(),
        query.to_string(),
    ];
    if let Some(owner) = params
        .owner
        .as_deref()
        .map(str::trim)
        .filter(|owner| !owner.is_empty())
    {
        args.push("--owner".to_string());
        args.push(owner.to_string());
    }
    let preview = command_preview(
        ctx,
        CommandPreviewDraft {
            operation: "search",
            args,
            cwd: manager_cwd(ctx, None)?,
            network_required: true,
            network_allowed: params.network_allowed,
            confirmed: false,
            summary: "Search remote skill indexes with npx skills.".to_string(),
            risks: vec![
                "Search may contact skills.sh, npm, or git-host metadata through the external CLI."
                    .to_string(),
            ],
            source: None,
            skills: Vec::new(),
        },
    )?;
    if !params.network_allowed {
        return Ok(skill_manager_search_record(preview, None, Vec::new()));
    }
    let execution = run_previewed_command(ctx, &preview)?;
    let results = parse_search_results(&execution.machine_stdout);
    Ok(skill_manager_search_record(
        preview,
        Some(execution.output.without_machine_stdout()),
        results,
    ))
}

pub fn list_installed_skills_with_manager(
    catalog: &Catalog,
    ctx: &AdapterContext,
    params: &SkillManagerListInstalledParams,
) -> Result<SkillManagerInstalledListRecord, CommandError> {
    let mut result = list_installed_skills_with_manager_targets(ctx, params, true)?;
    enrich_installed_removal_capabilities(
        catalog,
        ctx,
        params.scope.as_deref(),
        &mut result.installed,
    )?;
    Ok(result)
}

fn list_installed_skills_with_manager_targets(
    ctx: &AdapterContext,
    params: &SkillManagerListInstalledParams,
    restrict_to_supported_agents: bool,
) -> Result<SkillManagerInstalledListRecord, CommandError> {
    let mut args = vec![
        SKILLS_CLI_BINARY.to_string(),
        "list".to_string(),
        "--json".to_string(),
    ];
    // The UI inventory stays scoped to the adapters the app supports. Complete
    // uninstall deliberately uses the unrestricted form as a fail-closed
    // postcondition: a residual target known only to the external manager must
    // still keep the operation from being reported as successful. Both forms
    // use the bounded private regular-file capture below.
    if restrict_to_supported_agents {
        append_agent_args(&mut args, &default_agent_targets());
    }
    append_scope_args(&mut args, params.scope.as_deref())?;
    let preview = command_preview(
        ctx,
        CommandPreviewDraft {
            operation: "listInstalled",
            args,
            cwd: manager_cwd(ctx, params.scope.as_deref())?,
            network_required: false,
            network_allowed: true,
            confirmed: false,
            summary: "List skills currently managed by npx skills.".to_string(),
            risks: Vec::new(),
            source: None,
            skills: Vec::new(),
        },
    )?;
    let execution = run_previewed_command(ctx, &preview)?;
    let mut installed = parse_installed_records(&execution.machine_stdout)?;
    enrich_installed_records(ctx, params.scope.as_deref(), &mut installed);
    Ok(skill_manager_installed_record(
        preview,
        execution.output.without_machine_stdout(),
        installed,
    ))
}

fn skill_manager_search_record(
    preview: SkillManagerCommandPreview,
    output: Option<SkillManagerCommandOutput>,
    results: Vec<SkillManagerSearchResult>,
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
        page,
    }
}

fn skill_manager_installed_record(
    preview: SkillManagerCommandPreview,
    output: SkillManagerCommandOutput,
    installed: Vec<SkillManagerInstalledRecord>,
) -> SkillManagerInstalledListRecord {
    let page = ListPageMetadata::enumerable(installed.len(), Some(installed.len()), None);
    SkillManagerInstalledListRecord {
        preview,
        output,
        installed,
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
        removal_plan: None,
    })
}

pub fn apply_install_with_manager(
    catalog: &Catalog,
    ctx: &AdapterContext,
    params: &SkillManagerInstallParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let preview = build_install_preview(ctx, params)?;
    ensure_confirmed(&preview, params.confirmed, params.preview_token.as_deref())?;
    let output = run_previewed_command(ctx, &preview)?.output;
    let scan = scan_all_catalog_report(ctx, catalog)?;
    let updated_skills = catalog.list_skill_records()?;
    Ok(SkillManagerMutationRecord {
        preview,
        output: Some(output),
        applied: true,
        scanned_count: scan.scanned_count,
        updated_skills,
        removal_plan: None,
    })
}

pub fn preview_remove_with_manager(
    catalog: &Catalog,
    ctx: &AdapterContext,
    params: &SkillManagerRemoveParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let (preview, removal_plan, _) = build_remove_preview(catalog, ctx, params)?;
    Ok(SkillManagerMutationRecord {
        preview,
        output: None,
        applied: false,
        scanned_count: 0,
        updated_skills: Vec::new(),
        removal_plan: Some(removal_plan),
    })
}

pub fn apply_remove_with_manager(
    catalog: &Catalog,
    ctx: &AdapterContext,
    params: &SkillManagerRemoveParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let (preview, removal_plan, physical_plan) = build_remove_preview(catalog, ctx, params)?;
    ensure_confirmed(&preview, params.confirmed, params.preview_token.as_deref())?;

    let (output, scanned_count, updated_skills) = if params.full_uninstall {
        let output = run_previewed_command(ctx, &preview)?.output;
        let scan = scan_all_catalog_report(ctx, catalog)?;
        let updated_skills = catalog.list_skill_records()?;
        let installed_after = list_installed_skills_with_manager_targets(
            ctx,
            &SkillManagerListInstalledParams {
                agents: Vec::new(),
                scope: params.scope.clone(),
            },
            false,
        )?;
        verify_complete_remove_postcondition(
            ctx,
            params,
            &removal_plan,
            &updated_skills,
            &installed_after,
        )?;
        (output, scan.scanned_count, updated_skills)
    } else {
        let physical_plan = physical_plan.ok_or_else(|| {
            CommandError::InvalidSkillManagerRequest(
                "partial removal is missing its bound physical-target preview".to_string(),
            )
        })?;
        apply_physical_remove(catalog, ctx, &removal_plan, &physical_plan)?
    };

    Ok(SkillManagerMutationRecord {
        preview,
        output: Some(output),
        applied: true,
        scanned_count,
        updated_skills,
        removal_plan: Some(removal_plan),
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
        removal_plan: None,
    })
}

pub fn apply_update_with_manager(
    catalog: &Catalog,
    ctx: &AdapterContext,
    params: &SkillManagerUpdateParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let preview = build_update_preview(ctx, params)?;
    ensure_confirmed(&preview, params.confirmed, params.preview_token.as_deref())?;
    let output = run_previewed_command(ctx, &preview)?.output;
    let scan = scan_all_catalog_report(ctx, catalog)?;
    let updated_skills = catalog.list_skill_records()?;
    Ok(SkillManagerMutationRecord {
        preview,
        output: Some(output),
        applied: true,
        scanned_count: scan.scanned_count,
        updated_skills,
        removal_plan: None,
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
    })
}

pub fn apply_local_create_with_manager(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    params: &SkillManagerLocalCreateParams,
) -> Result<SkillManagerLocalCreateRecord, CommandError> {
    let preview = build_local_create_preview(app_data_dir, ctx, params)?;
    ensure_confirmed(&preview, params.confirmed, params.preview_token.as_deref())?;
    let output = run_previewed_command(ctx, &preview)?.output;
    let source_path = local_create_source_path(app_data_dir, &params.name)?;
    let imported = import_local_skill_to_tool_global(
        catalog,
        ctx,
        &app_data_dir.join("tool-global"),
        &source_path,
    )?;
    Ok(SkillManagerLocalCreateRecord {
        preview,
        output: Some(output),
        imported: Some(imported.imported),
        instance_id: Some(imported.instance_id),
        source_path: source_path.to_string_lossy().to_string(),
        applied: true,
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
    let blocked_by_references = records
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
    let physical_delete_allowed = app_owned && blocked_by_references.is_empty();
    let mut deleted = false;
    if params.confirmed {
        if !physical_delete_allowed {
            return Err(CommandError::InvalidSkillManagerRequest(
                "local skill physical delete is allowed only for app-owned records with no supported-agent references".to_string(),
            ));
        }
        let skill_dir = canonical_path.parent().ok_or_else(|| {
            CommandError::UnsafeConfigPath("local skill path has no parent".to_string())
        })?;
        if skill_dir.starts_with(&canonical_root) && skill_dir.exists() {
            fs::remove_dir_all(skill_dir)?;
        }
        catalog.delete_skill_instance(&meta.id)?;
        deleted = true;
        let payload = serde_json::json!({
            "deleted": true,
            "path": canonical_path.to_string_lossy(),
            "app_owned": app_owned,
        });
        catalog.create_skill_event(SkillEventDraft {
            instance_id: &meta.id,
            kind: "local-delete",
            payload: &serde_json::to_string(&payload)?,
            occurred_at_ms: unix_timestamp_millis(),
        })?;
    }
    Ok(SkillManagerLocalDeleteRecord {
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
    })
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
    let mut args = vec![
        SKILLS_CLI_BINARY.to_string(),
        "add".to_string(),
        source.to_string(),
    ];
    let skill_names = normalized_skill_names(&params.skills)?;
    for skill in &skill_names {
        args.push("--skill".to_string());
        args.push(skill.clone());
    }
    if !skill_names.is_empty() {
        args.push("--full-depth".to_string());
    }
    let local_source_path = resolve_manager_local_source_path(ctx, params.scope.as_deref(), source);
    let source_binding = local_source_path
        .as_deref()
        .map(|path| local_source::local_source_install_binding(path, &skill_names))
        .transpose()?;
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
    let network_required = local_source_path.is_none();
    command_preview_with_binding(
        ctx,
        CommandPreviewDraft {
            operation: "install",
            args,
            cwd: manager_cwd(ctx, params.scope.as_deref())?,
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
        },
        source_binding.as_deref(),
    )
}

fn build_remove_preview(
    catalog: &Catalog,
    ctx: &AdapterContext,
    params: &SkillManagerRemoveParams,
) -> Result<
    (
        SkillManagerCommandPreview,
        SkillManagerRemovalPlan,
        Option<PhysicalRemovalPlan>,
    ),
    CommandError,
> {
    let skill = params.skill.trim();
    if skill.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skillManager remove requires skill".to_string(),
        ));
    }
    if !params.full_uninstall {
        return build_physical_remove_preview(catalog, ctx, params, skill);
    }

    let mut args = vec![
        SKILLS_CLI_BINARY.to_string(),
        "remove".to_string(),
        skill.to_string(),
    ];
    let agents = normalize_manager_agents(&params.agents)?;
    append_scope_args(&mut args, params.scope.as_deref())?;
    args.push("-y".to_string());
    let preview = command_preview(
        ctx,
        CommandPreviewDraft {
            operation: "remove",
            args,
            cwd: manager_cwd(ctx, params.scope.as_deref())?,
            network_required: false,
            network_allowed: true,
            confirmed: params.confirmed,
            summary: format!(
                "Completely uninstall {skill} from every agent target recognized by the external manager."
            ),
            risks: vec![
                "This complete uninstall intentionally removes the manager's canonical source and every agent target it recognizes, including targets outside the app's six catalog adapters.".to_string(),
            ],
            source: None,
            skills: vec![skill.to_string()],
        },
    )?;
    let mut instance_ids = params
        .instance_ids
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    instance_ids.sort();
    instance_ids.dedup();
    Ok((
        preview,
        SkillManagerRemovalPlan {
            mode: "complete-uninstall".to_string(),
            full_uninstall: true,
            selected_agents: agents,
            instance_ids,
            source_preserved: false,
            actions: vec![SkillManagerRemovalAction {
                instance_id: String::new(),
                agent: "all".to_string(),
                scope: normalize_manager_scope(params.scope.as_deref())?
                    .unwrap_or_else(|| "project".to_string()),
                strategy: "external-manager-complete-uninstall".to_string(),
                target: "all manager-recognized agent targets".to_string(),
                summary: "Remove every manager-recognized target and delete the canonical source."
                    .to_string(),
            }],
            verification: "Refresh the catalog and external-manager inventory; the skill must no longer be installed for any supported agent.".to_string(),
        },
        None,
    ))
}

fn build_physical_remove_preview(
    catalog: &Catalog,
    ctx: &AdapterContext,
    params: &SkillManagerRemoveParams,
    skill: &str,
) -> Result<
    (
        SkillManagerCommandPreview,
        SkillManagerRemovalPlan,
        Option<PhysicalRemovalPlan>,
    ),
    CommandError,
> {
    let agents = required_manager_agents(&params.agents)?;
    let expected_scope =
        normalize_manager_scope(params.scope.as_deref())?.unwrap_or_else(|| "project".to_string());
    let mut instance_ids = params
        .instance_ids
        .iter()
        .map(|value| value.trim())
        .filter(|value| !value.is_empty())
        .map(str::to_string)
        .collect::<Vec<_>>();
    instance_ids.sort();
    instance_ids.dedup();
    if instance_ids.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "partial agent removal requires every exact catalog identity from the selected package row; refresh the inventory and try again".to_string(),
        ));
    }

    let selected = agents.iter().cloned().collect::<BTreeSet<_>>();
    let mut records = Vec::new();
    let mut covered_agents = BTreeSet::new();
    for instance_id in &instance_ids {
        let record = catalog
            .get_skill_record(instance_id)?
            .ok_or_else(|| CommandError::InstanceNotFound(instance_id.clone()))?;
        let manager_agent = manager_agent_alias(&record.agent)?;
        if !record.name.trim().eq_ignore_ascii_case(skill) {
            return Err(CommandError::InvalidSkillManagerRequest(format!(
                "instance {instance_id} does not match selected skill {skill}"
            )));
        }
        if normalize_record_scope(&record.scope) != expected_scope {
            return Err(CommandError::InvalidSkillManagerRequest(format!(
                "instance {instance_id} is outside the selected {expected_scope} scope"
            )));
        }
        if selected.contains(&manager_agent) {
            covered_agents.insert(manager_agent.clone());
        }
        records.push((record, manager_agent));
    }
    require_complete_physical_identity_set(
        catalog,
        ctx,
        skill,
        &expected_scope,
        &records,
        &instance_ids,
    )?;
    let uncovered = selected
        .difference(&covered_agents)
        .cloned()
        .collect::<Vec<_>>();
    if !uncovered.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "partial removal has no exact physical catalog target for: {}; refresh the inventory or use complete uninstall",
            uncovered.join(", ")
        )));
    }

    let (selected_records, preserved_records): (Vec<_>, Vec<_>) = records
        .into_iter()
        .partition(|(_, manager_agent)| selected.contains(manager_agent));
    if preserved_records.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "all known instances of {skill} are selected; use complete uninstall so the canonical source and every manager-recognized target are handled together"
        )));
    }

    let cwd = manager_cwd(ctx, params.scope.as_deref())?;
    let mut command = vec![
        "agent-copilot".to_string(),
        "remove-skill-targets".to_string(),
        skill.to_string(),
    ];
    append_agent_args(&mut command, &agents);
    append_scope_args(&mut command, params.scope.as_deref())?;

    let (physical_targets, preserved_paths) = validated_physical_removal_targets(
        ctx,
        &expected_scope,
        &selected_records,
        &preserved_records,
    )?;
    let selected_instance_ids = physical_targets
        .iter()
        .flat_map(|target| target.instance_ids.iter().cloned())
        .collect::<Vec<_>>();

    let mut confirmation_binding = command.clone();
    for target in &physical_targets {
        confirmation_binding.push("--physical-target".to_string());
        confirmation_binding.push(target.entry_path.to_string_lossy().to_string());
        confirmation_binding.push("--entry-kind".to_string());
        confirmation_binding.push(target.kind.binding_name().to_string());
        confirmation_binding.push("--entry-revision".to_string());
        confirmation_binding.push(target.revision.clone());
    }
    for path in &preserved_paths {
        confirmation_binding.push("--preserve".to_string());
        confirmation_binding.push(path.to_string_lossy().to_string());
    }
    let token = preview_token(&confirmation_binding, &cwd, "remove", false, true);
    let preview = SkillManagerCommandPreview {
        tool_id: "agent-copilot-native".to_string(),
        operation: "remove".to_string(),
        command,
        cwd: cwd.to_string_lossy().to_string(),
        env: Vec::new(),
        requires_confirmation: true,
        confirmed: params.confirmed,
        network_required: false,
        network_allowed: true,
        will_run: params.confirmed,
        preview_token: token,
        summary: format!(
            "Remove {skill} physical install target(s) from {} selected agent(s) while preserving the shared source and every unselected agent target.",
            agents.len()
        ),
        risks: vec![
            "Only the exact selected symlink or copied skill directory is removed; Agent enable/disable configuration is not changed.".to_string(),
            "If selected and unselected agents directly share one physical source directory, partial uninstall is refused because no separable target exists.".to_string(),
        ],
        source: None,
        skills: vec![skill.to_string()],
    };
    let actions = selected_records
        .iter()
        .map(|(record, agent)| {
            let entry_path = removal_entry_path(record)?;
            let target = physical_targets
                .iter()
                .find(|target| target.entry_path == entry_path)
                .ok_or_else(|| {
                    CommandError::InvalidSkillManagerRequest(
                        "physical removal action lost its selected target".to_string(),
                    )
                })?;
            Ok(SkillManagerRemovalAction {
                instance_id: record.id.clone(),
                agent: agent.clone(),
                scope: expected_scope.clone(),
                strategy: target.kind.strategy().to_string(),
                target: redact_command_output(ctx, &target.entry_path.to_string_lossy()),
                summary: format!(
                    "Remove the selected {} target without changing Agent enablement or deleting the shared source.",
                    target.kind.binding_name()
                ),
            })
        })
        .collect::<Result<Vec<_>, CommandError>>()?;
    Ok((
        preview,
        SkillManagerRemovalPlan {
            mode: "selected-agent-uninstall".to_string(),
            full_uninstall: false,
            selected_agents: agents,
            instance_ids: selected_instance_ids,
            source_preserved: true,
            actions,
            verification: "Refresh the catalog after a reversible filesystem removal; every selected link/copy must be absent while the shared source and unselected targets remain present.".to_string(),
        },
        Some(PhysicalRemovalPlan {
            targets: physical_targets,
            preserved_paths,
        }),
    ))
}

fn validated_physical_removal_targets(
    ctx: &AdapterContext,
    scope: &str,
    selected_records: &[(SkillRecord, String)],
    preserved_records: &[(SkillRecord, String)],
) -> Result<(Vec<PhysicalRemovalTarget>, Vec<PathBuf>), CommandError> {
    let mut targets_by_path: BTreeMap<PathBuf, PhysicalRemovalTarget> = BTreeMap::new();
    for (record, manager_agent) in selected_records {
        let entry_path = removal_entry_path(record)?;
        validate_physical_removal_target(ctx, manager_agent, scope, &entry_path)?;
        let (kind, revision) = physical_removal_entry_revision(&entry_path)?;
        let target = targets_by_path
            .entry(entry_path.clone())
            .or_insert_with(|| PhysicalRemovalTarget {
                instance_ids: Vec::new(),
                agents: Vec::new(),
                entry_path,
                kind,
                revision: revision.clone(),
            });
        if target.kind != kind || target.revision != revision {
            return Err(CommandError::InvalidSkillManagerRequest(
                "selected physical target changed while the removal preview was built".to_string(),
            ));
        }
        target.instance_ids.push(record.id.clone());
        target.agents.push(manager_agent.clone());
    }
    let mut physical_targets = targets_by_path.into_values().collect::<Vec<_>>();
    for target in &mut physical_targets {
        target.instance_ids.sort();
        target.instance_ids.dedup();
        target.agents.sort();
        target.agents.dedup();
    }
    validate_selected_targets_are_effective(ctx, scope, selected_records, &physical_targets)?;
    let preserved_paths =
        validate_and_collect_preserved_paths(ctx, scope, preserved_records, &physical_targets)?;
    Ok((physical_targets, preserved_paths))
}

fn require_complete_physical_identity_set(
    catalog: &Catalog,
    ctx: &AdapterContext,
    skill: &str,
    scope: &str,
    supplied_records: &[(SkillRecord, String)],
    supplied_instance_ids: &[String],
) -> Result<(), CommandError> {
    let definition_ids = supplied_records
        .iter()
        .map(|(record, _)| record.definition_id.as_str())
        .collect::<BTreeSet<_>>();
    let supplied = supplied_instance_ids
        .iter()
        .map(String::as_str)
        .collect::<BTreeSet<_>>();
    let mut omitted_agents = BTreeSet::new();

    for record in catalog.list_skill_records()? {
        if record.state.eq_ignore_ascii_case("missing")
            || record.read_only_reason.is_some()
            || !record.name.trim().eq_ignore_ascii_case(skill)
            || normalize_record_scope(&record.scope) != scope
            || !definition_ids.contains(record.definition_id.as_str())
            || supplied.contains(record.id.as_str())
        {
            continue;
        }
        let Ok(agent) = manager_agent_alias(&record.agent) else {
            continue;
        };
        let Ok(entry_path) = removal_entry_path(&record) else {
            continue;
        };
        let roots = physical_removal_roots(ctx, &agent, scope)?;
        if entry_path
            .parent()
            .is_some_and(|parent| roots.iter().any(|root| root == parent))
        {
            omitted_agents.insert(agent);
        }
    }

    if omitted_agents.is_empty() {
        return Ok(());
    }
    Err(CommandError::InvalidSkillManagerRequest(format!(
        "partial removal is missing exact physical identities for: {}; refresh the installed package row and preview again",
        omitted_agents.into_iter().collect::<Vec<_>>().join(", ")
    )))
}

fn validate_selected_targets_are_effective(
    ctx: &AdapterContext,
    scope: &str,
    records: &[(SkillRecord, String)],
    selected_targets: &[PhysicalRemovalTarget],
) -> Result<(), CommandError> {
    for (record, agent) in records {
        if let Some(path) =
            independent_preserved_skill_path(ctx, agent, scope, record, selected_targets)?
        {
            return Err(CommandError::InvalidSkillManagerRequest(format!(
                "{agent} also loads {} outside the selected physical target; removing only the selected link/copy would not uninstall the skill for that Agent",
                redact_command_output(ctx, &path.to_string_lossy())
            )));
        }
    }
    Ok(())
}

fn normalize_record_scope(scope: &str) -> String {
    match scope {
        value if value.eq_ignore_ascii_case(Scope::AgentGlobal.as_str()) => "global".to_string(),
        value if value.eq_ignore_ascii_case(Scope::AgentProject.as_str()) => "project".to_string(),
        other => other.to_ascii_lowercase(),
    }
}

fn removal_entry_path(record: &SkillRecord) -> Result<PathBuf, CommandError> {
    if record
        .display_path
        .file_name()
        .and_then(|name| name.to_str())
        .is_none_or(|name| !name.eq_ignore_ascii_case("SKILL.md"))
    {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "instance {} does not expose a physical SKILL.md target",
            record.id
        )));
    }
    record
        .display_path
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| {
            CommandError::InvalidSkillManagerRequest(format!(
                "instance {} has no physical skill directory",
                record.id
            ))
        })
}

fn validate_and_collect_preserved_paths(
    ctx: &AdapterContext,
    scope: &str,
    records: &[(SkillRecord, String)],
    selected_targets: &[PhysicalRemovalTarget],
) -> Result<Vec<PathBuf>, CommandError> {
    let manager_source_root = if scope == "global" {
        ctx.user_home.join(".agents/skills")
    } else {
        manager_cwd(ctx, Some("project"))?.join(".agents/skills")
    };
    if let Some(target) = selected_targets.iter().find(|target| {
        target.kind == PhysicalRemovalEntryKind::Directory
            && target.entry_path.starts_with(&manager_source_root)
    }) {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "{} is the shared canonical source, not a separable per-agent install target; keep it for remaining agents or use complete uninstall",
            redact_command_output(ctx, &target.entry_path.to_string_lossy())
        )));
    }

    let mut paths = BTreeSet::new();
    for (record, agent) in records {
        let Some(path) =
            independent_preserved_skill_path(ctx, agent, scope, record, selected_targets)?
        else {
            let selected = selected_targets
                .iter()
                .flat_map(|target| target.agents.iter())
                .next()
                .map(String::as_str)
                .unwrap_or("selected agent");
            return Err(shared_physical_target_error(
                ctx,
                selected,
                agent,
                &removal_entry_path(record)?,
            ));
        };
        paths.insert(path);
    }
    Ok(paths.into_iter().collect())
}

fn independent_preserved_skill_path(
    ctx: &AdapterContext,
    agent: &str,
    scope: &str,
    record: &SkillRecord,
    selected_targets: &[PhysicalRemovalTarget],
) -> Result<Option<PathBuf>, CommandError> {
    let roots = physical_removal_roots(ctx, agent, scope)?;
    let is_selected = |path: &Path| {
        selected_targets
            .iter()
            .any(|target| path == target.entry_path || path.starts_with(&target.entry_path))
    };
    let direct_candidate = |path: &Path| {
        path.parent()
            .and_then(Path::parent)
            .is_some_and(|parent| roots.iter().any(|root| root == parent))
            && !is_selected(path)
            && path.is_file()
            && survives_selected_directory_removal(path, selected_targets)
    };
    if direct_candidate(&record.display_path) {
        return Ok(Some(record.display_path.clone()));
    }
    if direct_candidate(&record.path) {
        return Ok(Some(record.path.clone()));
    }

    let mut directory_names = BTreeSet::new();
    for path in [&record.display_path, &record.path] {
        if let Some(name) = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
        {
            directory_names.insert(name.to_string());
        }
    }
    for root in roots {
        for directory_name in &directory_names {
            let candidate = root.join(directory_name).join("SKILL.md");
            if is_selected(&candidate)
                || !candidate.is_file()
                || !survives_selected_directory_removal(&candidate, selected_targets)
            {
                continue;
            }
            let Ok(metadata) = fs::metadata(&candidate) else {
                continue;
            };
            if !metadata.is_file() || metadata.len() > 2 * 1024 * 1024 {
                continue;
            }
            let Ok(content) = fs::read_to_string(&candidate) else {
                continue;
            };
            let parsed_name = crate::tool_global_skill_name_from_content(&content, directory_name);
            if parsed_name.eq_ignore_ascii_case(&record.name) {
                return Ok(Some(candidate));
            }
        }
    }
    Ok(None)
}

fn survives_selected_directory_removal(
    candidate: &Path,
    selected_targets: &[PhysicalRemovalTarget],
) -> bool {
    let canonical_candidate = fs::canonicalize(candidate).ok();
    selected_targets.iter().all(|target| {
        if target.kind != PhysicalRemovalEntryKind::Directory {
            return true;
        }
        if candidate.starts_with(&target.entry_path) {
            return false;
        }
        let Some(canonical_candidate) = canonical_candidate.as_ref() else {
            return false;
        };
        fs::canonicalize(&target.entry_path)
            .is_ok_and(|canonical_target| !canonical_candidate.starts_with(canonical_target))
    })
}

fn validate_physical_removal_target(
    ctx: &AdapterContext,
    agent: &str,
    scope: &str,
    entry_path: &Path,
) -> Result<(), CommandError> {
    let parent = entry_path.parent().ok_or_else(|| {
        CommandError::InvalidSkillManagerRequest(
            "physical removal target has no parent install root".to_string(),
        )
    })?;
    let allowed_roots = physical_removal_roots(ctx, agent, scope)?;
    let Some(root) = allowed_roots.iter().find(|root| root.as_path() == parent) else {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "{} does not identify a separable {} {} install target; shared/configured source roots cannot be deleted by partial uninstall",
            redact_command_output(ctx, &entry_path.to_string_lossy()),
            agent,
            scope
        )));
    };
    let root_metadata = fs::symlink_metadata(root)?;
    if root_metadata.file_type().is_symlink() || !root_metadata.is_dir() {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "physical install root {} must be a real directory",
            redact_command_output(ctx, &root.to_string_lossy())
        )));
    }
    let name = entry_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or_default();
    if name.is_empty() || matches!(name, "." | "..") {
        return Err(CommandError::InvalidSkillManagerRequest(
            "physical removal target has an unsafe directory name".to_string(),
        ));
    }
    Ok(())
}

fn physical_removal_roots(
    ctx: &AdapterContext,
    agent: &str,
    scope: &str,
) -> Result<Vec<PathBuf>, CommandError> {
    let mut roots = Vec::new();
    if scope == "global" {
        match agent {
            "claude-code" => roots.push(claude_config_dir(ctx).join("skills")),
            "codex" => roots.extend([
                ctx.user_home.join(".agents/skills"),
                codex_home_dir(ctx).join("skills"),
            ]),
            "opencode" => roots.extend([
                ctx.user_home.join(".agents/skills"),
                opencode_user_skills_dir(ctx),
            ]),
            "pi" => roots.extend([
                ctx.user_home.join(".agents/skills"),
                pi_agent_dir(ctx).join("skills"),
            ]),
            "hermes-agent" => roots.push(hermes_home_dir(ctx).join("skills")),
            "openclaw" => roots.extend([
                ctx.user_home.join(".agents/skills"),
                openclaw_state_dir(ctx).join("skills"),
            ]),
            other => {
                return Err(CommandError::InvalidSkillManagerRequest(format!(
                    "unsupported physical removal agent: {other}"
                )))
            }
        }
    } else if scope == "project" {
        for directory in active_project_directories(ctx)? {
            match agent {
                "claude-code" => roots.push(directory.join(".claude/skills")),
                "codex" => roots.push(directory.join(".agents/skills")),
                "opencode" => roots.extend([
                    directory.join(".agents/skills"),
                    directory.join(".opencode/skills"),
                ]),
                "pi" => roots.extend([
                    directory.join(".agents/skills"),
                    directory.join(".pi/skills"),
                ]),
                "hermes-agent" => roots.push(directory.join(".hermes/skills")),
                "openclaw" => {
                    roots.extend([directory.join(".agents/skills"), directory.join("skills")])
                }
                other => {
                    return Err(CommandError::InvalidSkillManagerRequest(format!(
                        "unsupported physical removal agent: {other}"
                    )))
                }
            }
        }
    } else {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "unsupported physical removal scope: {scope}"
        )));
    }
    roots.sort();
    roots.dedup();
    Ok(roots)
}

fn active_project_directories(ctx: &AdapterContext) -> Result<Vec<PathBuf>, CommandError> {
    let root = ctx.project_root.as_ref().ok_or_else(|| {
        CommandError::InvalidSkillManagerRequest(
            "project-scoped physical removal requires an active project root".to_string(),
        )
    })?;
    let start = ctx
        .project_cwd
        .as_ref()
        .filter(|cwd| cwd.starts_with(root))
        .unwrap_or(root);
    let mut directories = Vec::new();
    let mut current = start.as_path();
    loop {
        directories.push(current.to_path_buf());
        if current == root {
            break;
        }
        current = current.parent().ok_or_else(|| {
            CommandError::InvalidSkillManagerRequest(
                "project context escaped its selected root".to_string(),
            )
        })?;
        if !current.starts_with(root) {
            return Err(CommandError::InvalidSkillManagerRequest(
                "project context escaped its selected root".to_string(),
            ));
        }
    }
    Ok(directories)
}

fn shared_physical_target_error(
    ctx: &AdapterContext,
    selected_agent: &str,
    preserved_agent: &str,
    entry_path: &Path,
) -> CommandError {
    CommandError::InvalidSkillManagerRequest(format!(
        "{selected_agent} and {preserved_agent} directly share {}; no per-agent symlink or copied directory exists to remove without affecting the other agent",
        redact_command_output(ctx, &entry_path.to_string_lossy())
    ))
}

fn physical_removal_entry_revision(
    entry_path: &Path,
) -> Result<(PhysicalRemovalEntryKind, String), CommandError> {
    let metadata = fs::symlink_metadata(entry_path)?;
    let mut hasher = Sha256::new();
    if metadata.file_type().is_symlink() {
        let target = fs::read_link(entry_path)?;
        if !entry_path.join("SKILL.md").is_file() {
            return Err(CommandError::InvalidSkillManagerRequest(
                "selected symlink target no longer contains SKILL.md".to_string(),
            ));
        }
        hasher.update(b"\nsymlink\n");
        hasher.update(target.to_string_lossy().as_bytes());
        hash_removal_metadata(&mut hasher, &metadata);
        return Ok((
            PhysicalRemovalEntryKind::Symlink,
            format!("{:x}", hasher.finalize()),
        ));
    }
    if !metadata.is_dir() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "selected physical target is neither a symlink nor a copied skill directory"
                .to_string(),
        ));
    }
    if !entry_path.join("SKILL.md").is_file() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "selected copied directory no longer contains SKILL.md".to_string(),
        ));
    }
    hasher.update(b"directory\n");
    hash_removal_directory(entry_path, &mut hasher)?;
    Ok((
        PhysicalRemovalEntryKind::Directory,
        format!("{:x}", hasher.finalize()),
    ))
}

fn hash_removal_directory(root: &Path, hasher: &mut Sha256) -> Result<(), CommandError> {
    let mut stack = vec![root.to_path_buf()];
    let mut entry_count = 0usize;
    while let Some(directory) = stack.pop() {
        let mut entries = fs::read_dir(&directory)?.collect::<Result<Vec<_>, _>>()?;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            entry_count += 1;
            if entry_count > MAX_REMOVAL_ENTRY_COUNT {
                return Err(CommandError::InvalidSkillManagerRequest(format!(
                    "selected copied skill directory exceeds the {}-entry removal preview limit",
                    MAX_REMOVAL_ENTRY_COUNT
                )));
            }
            let path = entry.path();
            let relative = path.strip_prefix(root).map_err(|_| {
                CommandError::InvalidSkillManagerRequest(
                    "selected copied skill entry escaped its target directory".to_string(),
                )
            })?;
            let metadata = fs::symlink_metadata(&path)?;
            hasher.update(b"\n");
            hasher.update(relative.to_string_lossy().as_bytes());
            if metadata.file_type().is_symlink() {
                hasher.update(b"\nsymlink\n");
                hasher.update(fs::read_link(&path)?.to_string_lossy().as_bytes());
            } else if metadata.is_dir() {
                hasher.update(b"\ndirectory");
                stack.push(path);
            } else if metadata.is_file() {
                hasher.update(b"\nfile");
            } else {
                return Err(CommandError::InvalidSkillManagerRequest(
                    "selected copied skill contains an unsupported special entry".to_string(),
                ));
            }
            hash_removal_metadata(hasher, &metadata);
        }
    }
    Ok(())
}

fn hash_removal_metadata(hasher: &mut Sha256, metadata: &fs::Metadata) {
    hasher.update(b"\n");
    hasher.update(metadata.len().to_le_bytes());
    let modified = metadata
        .modified()
        .ok()
        .and_then(|value| value.duration_since(UNIX_EPOCH).ok())
        .map(|value| value.as_nanos())
        .unwrap_or_default();
    hasher.update(modified.to_le_bytes());
}

fn apply_physical_remove(
    catalog: &Catalog,
    ctx: &AdapterContext,
    plan: &SkillManagerRemovalPlan,
    physical_plan: &PhysicalRemovalPlan,
) -> Result<(SkillManagerCommandOutput, usize, Vec<SkillRecord>), CommandError> {
    for target in &physical_plan.targets {
        let (kind, revision) = physical_removal_entry_revision(&target.entry_path)?;
        if kind != target.kind || revision != target.revision {
            return Err(CommandError::InvalidSkillManagerRequest(format!(
                "physical target {} changed after preview; request a fresh preview_token",
                redact_command_output(ctx, &target.entry_path.to_string_lossy())
            )));
        }
    }

    let staged = stage_physical_removals(&physical_plan.targets)?;
    let verified = (|| {
        let scan = scan_all_catalog_report(ctx, catalog)?;
        let updated_skills = catalog.list_skill_records()?;
        verify_physical_remove_postcondition(ctx, plan, physical_plan, &updated_skills)?;
        Ok((scan.scanned_count, updated_skills))
    })();

    let (scanned_count, updated_skills) = match verified {
        Ok(value) => value,
        Err(error) => {
            if let Err(rollback_error) = rollback_staged_removals(&staged) {
                let _ = scan_all_catalog_report(ctx, catalog);
                return Err(rollback_error);
            }
            let _ = scan_all_catalog_report(ctx, catalog);
            return Err(error);
        }
    };
    commit_staged_removals(&staged)?;
    Ok((
        SkillManagerCommandOutput {
            status: "completed".to_string(),
            exit_code: Some(0),
            stdout: String::new(),
            stderr: String::new(),
        },
        scanned_count,
        updated_skills,
    ))
}

fn stage_physical_removals(
    targets: &[PhysicalRemovalTarget],
) -> Result<Vec<StagedRemovalTarget>, CommandError> {
    let nonce = format!(
        "{}-{}",
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap_or_default()
            .as_nanos(),
        std::process::id()
    );
    let mut staged = Vec::new();
    for (index, target) in targets.iter().enumerate() {
        let install_root = target.entry_path.parent().ok_or_else(|| {
            CommandError::InvalidSkillManagerRequest(
                "physical removal target has no parent".to_string(),
            )
        })?;
        let backup_parent = install_root.parent().ok_or_else(|| {
            CommandError::InvalidSkillManagerRequest(
                "physical removal install root has no parent".to_string(),
            )
        })?;
        let name = target
            .entry_path
            .file_name()
            .and_then(|value| value.to_str())
            .unwrap_or("skill");
        let backup_root = backup_parent.join(format!(".agent-copilot-remove-{nonce}-{index}"));
        let backup_path = backup_root.join(name);
        if fs::symlink_metadata(&backup_root).is_ok() {
            rollback_staged_removals(&staged)?;
            return Err(CommandError::InvalidSkillManagerRequest(
                "private temporary removal target already exists".to_string(),
            ));
        }
        if let Err(error) = create_private_removal_backup_root(&backup_root) {
            rollback_staged_removals(&staged)?;
            return Err(error);
        }
        if let Err(error) = fs::rename(&target.entry_path, &backup_path) {
            let _ = fs::remove_dir(&backup_root);
            rollback_staged_removals(&staged)?;
            return Err(error.into());
        }
        staged.push(StagedRemovalTarget {
            original_path: target.entry_path.clone(),
            backup_path,
            backup_root,
            kind: target.kind,
        });
        let staged_target = staged.last().expect("staged target was just appended");
        let staged_revision = physical_removal_entry_revision(&staged_target.backup_path);
        if !matches!(
            staged_revision,
            Ok((kind, ref revision)) if kind == target.kind && revision == &target.revision
        ) {
            rollback_staged_removals(&staged)?;
            return Err(CommandError::InvalidSkillManagerRequest(
                "physical target changed while it was staged; request a fresh preview_token"
                    .to_string(),
            ));
        }
    }
    Ok(staged)
}

fn create_private_removal_backup_root(path: &Path) -> Result<(), CommandError> {
    fs::create_dir(path)?;
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if let Err(error) = fs::set_permissions(path, fs::Permissions::from_mode(0o700)) {
            let _ = fs::remove_dir(path);
            return Err(error.into());
        }
    }
    Ok(())
}

fn rollback_staged_removals(staged: &[StagedRemovalTarget]) -> Result<(), CommandError> {
    let mut failure_count = 0usize;
    for target in staged.iter().rev() {
        let original_exists = path_entry_exists(&target.original_path);
        let backup_exists = path_entry_exists(&target.backup_path);
        match (original_exists, backup_exists) {
            (Ok(false), Ok(true)) => {
                if fs::rename(&target.backup_path, &target.original_path).is_err() {
                    failure_count += 1;
                }
            }
            (Ok(true), Ok(false)) => {}
            (Ok(true), Ok(true)) | (Ok(false), Ok(false)) | (Err(_), _) | (_, Err(_)) => {
                failure_count += 1;
            }
        }
        match path_entry_exists(&target.backup_path) {
            Ok(false) => {
                if fs::remove_dir(&target.backup_root).is_err() {
                    failure_count += 1;
                }
            }
            Ok(true) => {}
            Err(_) => failure_count += 1,
        }
    }
    if failure_count == 0 {
        Ok(())
    } else {
        Err(CommandError::SkillManagerRemovalIncomplete(format!(
            "could not restore {failure_count} staged removal entry state(s); private recovery data was retained where restoration did not complete"
        )))
    }
}

fn commit_staged_removals(staged: &[StagedRemovalTarget]) -> Result<(), CommandError> {
    let mut failure_count = 0usize;
    for target in staged {
        let removed = match target.kind {
            PhysicalRemovalEntryKind::Symlink => {
                if let Err(first_error) = fs::remove_file(&target.backup_path) {
                    fs::remove_dir(&target.backup_path).map_err(|_| first_error)
                } else {
                    Ok(())
                }
            }
            PhysicalRemovalEntryKind::Directory => fs::remove_dir_all(&target.backup_path),
        };
        if removed.is_err() {
            failure_count += 1;
            continue;
        }
        if fs::remove_dir(&target.backup_root).is_err() {
            failure_count += 1;
        }
    }
    if failure_count == 0 {
        Ok(())
    } else {
        Err(CommandError::SkillManagerRemovalIncomplete(format!(
            "physical targets were removed, but {failure_count} private staging cleanup step(s) did not complete"
        )))
    }
}

fn path_entry_exists(path: &Path) -> Result<bool, std::io::Error> {
    match fs::symlink_metadata(path) {
        Ok(_) => Ok(true),
        Err(error) if error.kind() == ErrorKind::NotFound => Ok(false),
        Err(error) => Err(error),
    }
}

fn verify_physical_remove_postcondition(
    ctx: &AdapterContext,
    plan: &SkillManagerRemovalPlan,
    physical_plan: &PhysicalRemovalPlan,
    updated_skills: &[SkillRecord],
) -> Result<(), CommandError> {
    let remaining_targets = physical_plan
        .targets
        .iter()
        .filter(|target| removal_path_entry_exists(&target.entry_path))
        .map(|target| redact_command_output(ctx, &target.entry_path.to_string_lossy()))
        .collect::<Vec<_>>();
    if !remaining_targets.is_empty() {
        return Err(CommandError::SkillManagerRemovalIncomplete(format!(
            "selected physical targets still exist after removal: {}",
            remaining_targets.join(", ")
        )));
    }
    let missing_preserved = physical_plan
        .preserved_paths
        .iter()
        .filter(|path| !removal_path_entry_exists(path))
        .map(|path| redact_command_output(ctx, &path.to_string_lossy()))
        .collect::<Vec<_>>();
    if !missing_preserved.is_empty() {
        return Err(CommandError::SkillManagerRemovalIncomplete(format!(
            "shared source or unselected agent targets disappeared during partial removal: {}",
            missing_preserved.join(", ")
        )));
    }
    let still_installed = plan
        .instance_ids
        .iter()
        .filter_map(|instance_id| {
            updated_skills
                .iter()
                .find(|record| &record.id == instance_id)
        })
        .filter(|record| record.state != "missing")
        .map(|record| format!("{}:{}", record.agent, record.name))
        .collect::<Vec<_>>();
    if !still_installed.is_empty() {
        return Err(CommandError::SkillManagerRemovalIncomplete(format!(
            "selected agent instances are still physically installed after removal: {}",
            still_installed.join(", ")
        )));
    }
    Ok(())
}

fn verify_complete_remove_postcondition(
    ctx: &AdapterContext,
    params: &SkillManagerRemoveParams,
    plan: &SkillManagerRemovalPlan,
    updated_skills: &[SkillRecord],
    installed_after: &SkillManagerInstalledListRecord,
) -> Result<(), CommandError> {
    if installed_after
        .installed
        .iter()
        .any(|record| record.name.trim().eq_ignore_ascii_case(params.skill.trim()))
    {
        return Err(CommandError::SkillManagerRemovalIncomplete(format!(
            "{} is still present in the external-manager inventory after complete uninstall",
            params.skill.trim()
        )));
    }
    let remaining_paths = plan
        .instance_ids
        .iter()
        .filter_map(|instance_id| {
            updated_skills
                .iter()
                .find(|record| &record.id == instance_id)
        })
        .filter(|record| removal_record_entry_exists(record))
        .map(|record| redact_command_output(ctx, &record.display_path.to_string_lossy()))
        .collect::<Vec<_>>();
    if !remaining_paths.is_empty() {
        return Err(CommandError::SkillManagerRemovalIncomplete(format!(
            "verified catalog paths still exist after complete uninstall: {}",
            remaining_paths.join(", ")
        )));
    }
    Ok(())
}

fn removal_record_entry_exists(record: &SkillRecord) -> bool {
    removal_entry_path(record).is_ok_and(|path| removal_path_entry_exists(&path))
        || removal_path_entry_exists(&record.path)
}

fn removal_path_entry_exists(path: &Path) -> bool {
    fs::symlink_metadata(path).is_ok()
}

fn build_update_preview(
    ctx: &AdapterContext,
    params: &SkillManagerUpdateParams,
) -> Result<SkillManagerCommandPreview, CommandError> {
    let skill_names = normalized_skill_names(&params.skills)?;
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
        },
    )
}

struct CommandPreviewDraft {
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
                "listInstalled output exceeded the safe capture limit".to_string(),
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
                "listInstalled output exceeded the safe capture limit".to_string(),
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

fn command_preview(
    ctx: &AdapterContext,
    draft: CommandPreviewDraft,
) -> Result<SkillManagerCommandPreview, CommandError> {
    command_preview_with_binding(ctx, draft, None)
}

fn command_preview_with_binding(
    ctx: &AdapterContext,
    draft: CommandPreviewDraft,
    confirmation_binding: Option<&str>,
) -> Result<SkillManagerCommandPreview, CommandError> {
    let executable = npx_executable()?;
    command_preview_with_executable_and_binding(ctx, draft, &executable, confirmation_binding)
}

fn command_preview_with_executable(
    ctx: &AdapterContext,
    draft: CommandPreviewDraft,
    executable: &Path,
) -> Result<SkillManagerCommandPreview, CommandError> {
    command_preview_with_executable_and_binding(ctx, draft, executable, None)
}

fn command_preview_with_executable_and_binding(
    ctx: &AdapterContext,
    mut draft: CommandPreviewDraft,
    executable: &Path,
    confirmation_binding: Option<&str>,
) -> Result<SkillManagerCommandPreview, CommandError> {
    let command = {
        let mut command = vec![executable.to_string_lossy().to_string()];
        command.append(&mut draft.args);
        command
    };
    let will_run = draft.confirmed && (!draft.network_required || draft.network_allowed);
    let mut token_binding = command.clone();
    if let Some(binding) = confirmation_binding {
        token_binding.push("--agent-copilot-source-revision".to_string());
        token_binding.push(binding.to_string());
    }
    let preview_token = preview_token(
        &token_binding,
        &draft.cwd,
        draft.operation,
        draft.network_required,
        draft.network_allowed,
    );
    Ok(SkillManagerCommandPreview {
        tool_id: DEFAULT_MANAGER_TOOL.to_string(),
        operation: draft.operation.to_string(),
        command,
        cwd: draft.cwd.to_string_lossy().to_string(),
        env: manager_env(ctx),
        requires_confirmation: !matches!(
            draft.operation,
            "search" | "listInstalled" | "inspectLocalSource"
        ),
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
    let Some((executable, args)) = preview.command.split_first() else {
        return Err(CommandError::InvalidSkillManagerRequest(
            "empty skill manager command".to_string(),
        ));
    };
    let cwd = PathBuf::from(&preview.cwd);
    fs::create_dir_all(&cwd)?;
    let mut command = Command::new(executable);
    command.args(args).current_dir(&cwd);
    for env_var in manager_command_env(ctx, executable) {
        command.env(env_var.key, env_var.value);
    }
    // Node's console writes asynchronously when stdout is a pipe. The external
    // manager exits immediately after printing large JSON, which can drop
    // everything beyond the 64 KiB pipe buffer. A private regular file makes
    // that write synchronous; it is bounded, read only after exit, and removed
    // by RAII on every return path.
    let mut machine_capture = if preview.operation == "listInstalled" {
        Some(MachineStdoutCapture::create()?)
    } else {
        None
    };
    if let Some(capture) = &machine_capture {
        command.stdout(capture.child_stdout()?);
    }
    let output = command.output().map_err(|error| {
        CommandError::SkillManagerCommandFailed(format!(
            "failed to run {}: {error}",
            preview.command.join(" ")
        ))
    })?;
    let machine_stdout = match &mut machine_capture {
        Some(capture) => capture.read()?,
        None => output.stdout,
    };
    if machine_stdout.len() > MAX_MACHINE_OUTPUT_BYTES
        || output.stderr.len() > MAX_MACHINE_OUTPUT_BYTES
    {
        return Err(CommandError::SkillManagerCommandFailed(format!(
            "{} output exceeded the safe capture limit",
            preview.operation
        )));
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
        return Err(CommandError::SkillManagerCommandFailed(format!(
            "{} failed with status {:?}: {}",
            preview.operation, record.exit_code, detail
        )));
    }
    Ok(SkillManagerCommandExecution {
        output: record,
        machine_stdout: stdout,
    })
}

fn ensure_confirmed(
    preview: &SkillManagerCommandPreview,
    confirmed: bool,
    preview_token: Option<&str>,
) -> Result<(), CommandError> {
    if !confirmed {
        return Err(CommandError::InvalidSkillManagerRequest(format!(
            "{} requires confirmed=true",
            preview.operation
        )));
    }
    if let Some(token) = preview_token {
        if token != preview.preview_token {
            return Err(CommandError::InvalidSkillManagerRequest(
                "skill manager apply requires a fresh preview_token for the same command"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

fn manager_env(ctx: &AdapterContext) -> Vec<SkillManagerEnvPreview> {
    vec![
        env_preview("HOME", &ctx.user_home.to_string_lossy()),
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

fn resolve_manager_local_source_path(
    ctx: &AdapterContext,
    scope: Option<&str>,
    source: &str,
) -> Option<PathBuf> {
    let source = source.trim();
    let path = if let Some(path) = source.strip_prefix("file://") {
        PathBuf::from(path)
    } else {
        PathBuf::from(source)
    };
    if path.is_absolute() {
        return Some(path);
    }
    let candidate = manager_cwd(ctx, scope).ok()?.join(&path);
    (source.starts_with('.') || candidate.exists()).then_some(candidate)
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

fn parse_search_results(stdout: &str) -> Vec<SkillManagerSearchResult> {
    if let Ok(value) = serde_json::from_str::<Value>(stdout) {
        return records_from_json_value(&value)
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
            .collect();
    }
    let mut results = Vec::new();
    for line in stdout.lines().map(strip_ansi_codes) {
        let line = line.trim();
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
    results
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
                separable_agents: None,
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
}

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

fn enrich_installed_removal_capabilities(
    catalog: &Catalog,
    ctx: &AdapterContext,
    scope: Option<&str>,
    records: &mut [SkillManagerInstalledRecord],
) -> Result<(), CommandError> {
    let normalized_scope = normalize_manager_scope(scope)?.unwrap_or_else(|| "project".to_string());
    let catalog_records = catalog.list_skill_records()?;
    for record in records {
        let physical_records = installed_record_physical_records(
            ctx,
            scope,
            &normalized_scope,
            record,
            &catalog_records,
        );
        let reported_agents = record
            .agents
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        record.separable_agents = Some(
            default_agent_targets()
                .into_iter()
                .filter(|agent| reported_agents.contains(agent.as_str()))
                .filter(|agent| {
                    can_separately_remove_installed_agent(
                        ctx,
                        &normalized_scope,
                        agent,
                        &physical_records,
                    )
                })
                .collect(),
        );
    }
    Ok(())
}

fn installed_record_physical_records(
    ctx: &AdapterContext,
    scope: Option<&str>,
    normalized_scope: &str,
    installed: &SkillManagerInstalledRecord,
    catalog_records: &[SkillRecord],
) -> Vec<(SkillRecord, String)> {
    let mut candidates = catalog_records
        .iter()
        .filter(|record| {
            !record.state.eq_ignore_ascii_case("missing")
                && record.read_only_reason.is_none()
                && record.name.trim().eq_ignore_ascii_case(&installed.name)
                && normalize_record_scope(&record.scope) == normalized_scope
        })
        .filter_map(|record| {
            manager_agent_alias(&record.agent)
                .ok()
                .map(|agent| (record, agent))
        })
        .collect::<Vec<_>>();
    if candidates.is_empty() {
        return Vec::new();
    }

    let canonical_source = installed
        .path
        .as_deref()
        .and_then(|path| installed_record_local_path(ctx, scope, path))
        .map(|path| {
            if path
                .file_name()
                .is_some_and(|name| name.eq_ignore_ascii_case("SKILL.md"))
            {
                path
            } else {
                path.join("SKILL.md")
            }
        })
        .and_then(|path| path.canonicalize().ok());
    let matched_definition = canonical_source.as_ref().and_then(|source| {
        candidates.iter().find_map(|(record, _)| {
            [&record.path, &record.display_path]
                .into_iter()
                .any(|path| path.canonicalize().is_ok_and(|path| path == *source))
                .then_some(record.definition_id.clone())
        })
    });
    let definition_id = matched_definition.or_else(|| {
        let definitions = candidates
            .iter()
            .map(|(record, _)| record.definition_id.as_str())
            .collect::<BTreeSet<_>>();
        if definitions.len() == 1 {
            definitions.into_iter().next().map(str::to_string)
        } else {
            None
        }
    });
    let Some(definition_id) = definition_id else {
        return Vec::new();
    };
    candidates.retain(|(record, _)| record.definition_id == definition_id);
    candidates
        .into_iter()
        .map(|(record, agent)| (record.clone(), agent))
        .collect()
}

fn can_separately_remove_installed_agent(
    ctx: &AdapterContext,
    scope: &str,
    selected_agent: &str,
    records: &[(SkillRecord, String)],
) -> bool {
    let (selected, preserved): (Vec<_>, Vec<_>) = records
        .iter()
        .cloned()
        .partition(|(_, agent)| agent == selected_agent);
    !selected.is_empty()
        && !preserved.is_empty()
        && validated_physical_removal_targets(ctx, scope, &selected, &preserved).is_ok()
}

fn installed_record_local_path(
    ctx: &AdapterContext,
    scope: Option<&str>,
    value: &str,
) -> Option<PathBuf> {
    let value = value.trim();
    let path = if value == "$HOME" || value == "~" {
        ctx.user_home.clone()
    } else if value == "<project-root>" {
        manager_cwd(ctx, scope).ok()?
    } else if let Some(relative) = value
        .strip_prefix("$HOME/")
        .or_else(|| value.strip_prefix("~/"))
    {
        ctx.user_home.join(relative)
    } else if let Some(relative) = value.strip_prefix("<project-root>/") {
        manager_cwd(ctx, scope).ok()?.join(relative)
    } else {
        let path = PathBuf::from(value);
        if path.is_absolute() {
            path
        } else {
            manager_cwd(ctx, scope).ok()?.join(path)
        }
    };
    Some(path)
}

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
mod effects_tests;

#[cfg(test)]
mod tests {
    use super::*;

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
    fn preview_env_stays_minimal_while_runtime_env_adds_path() {
        let temp =
            std::env::temp_dir().join(format!("skill-manager-command-env-{}", std::process::id()));
        let ctx = AdapterContext {
            user_home: temp.join("home"),
            project_cwd: Some(temp.join("project")),
            project_root: Some(temp.join("project")),
            extra_roots: Vec::new(),
        };
        let runtime_env = manager_command_env(&ctx, "/custom/node/bin/npx");

        assert!(
            manager_env(&ctx)
                .iter()
                .all(|env_var| env_var.key != "PATH"),
            "Preview metadata should not display the full local PATH."
        );
        assert!(
            runtime_env
                .iter()
                .any(|env_var| env_var.key == "PATH" && env_var.value.contains("/custom/node/bin")),
            "Runtime command env should make node visible to /usr/bin/env shebangs."
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
    fn failed_command_error_uses_stdout_when_stderr_is_empty() {
        let stderr = "";
        let stdout = "\u{1b}[31mNo matching skills found for: alibabacloud-find-skills\u{1b}[0m";

        let detail = failed_command_detail(stdout, stderr);

        assert_eq!(
            detail,
            "No matching skills found for: alibabacloud-find-skills"
        );
    }
}
