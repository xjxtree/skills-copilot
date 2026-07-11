use std::{
    collections::BTreeSet,
    env, fs,
    path::{Path, PathBuf},
    process::Command,
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::Value;
use sha2::{Digest, Sha256};
use skills_copilot_catalog::{Catalog, SkillEventDraft, SkillRecord};
use skills_copilot_core::{
    AdapterContext, AgentId, ListIncompleteReason, ListPageMetadata, ListSourceCompleteness, Scope,
};

use crate::{
    import_local_skill_to_tool_global, scan_all_catalog_report, tool_global_staging_skills_root,
    CommandError,
};

const DEFAULT_MANAGER_TOOL: &str = "npx-skills";
const SKILLS_NPM_TOOL: &str = "skills-npm";
const SKILLS_CLI_BINARY: &str = "skills";
const NPX_BINARY: &str = "npx";
const MAX_CAPTURE_BYTES: usize = 32_000;

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
    pub agents: Vec<String>,
    pub scope: Option<String>,
    pub path: Option<String>,
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
    pub scope: Option<String>,
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
                "previewInstall",
                "applyInstall",
                "previewRemove",
                "applyRemove",
                "previewUpdate",
                "applyUpdate",
                "previewLocalCreate",
                "applyLocalCreate",
                "deleteLocal",
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
    let output = run_previewed_command(ctx, &preview)?;
    let results = parse_search_results(&output.stdout);
    Ok(skill_manager_search_record(preview, Some(output), results))
}

pub fn list_installed_skills_with_manager(
    ctx: &AdapterContext,
    params: &SkillManagerListInstalledParams,
) -> Result<SkillManagerInstalledListRecord, CommandError> {
    let mut args = vec![
        SKILLS_CLI_BINARY.to_string(),
        "list".to_string(),
        "--json".to_string(),
    ];
    append_scope_args(&mut args, params.scope.as_deref())?;
    append_agent_args(&mut args, &normalize_manager_agents(&params.agents)?);
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
    let output = run_previewed_command(ctx, &preview)?;
    let installed = parse_installed_records(&output.stdout)?;
    Ok(skill_manager_installed_record(preview, output, installed))
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
    })
}

pub fn apply_install_with_manager(
    catalog: &Catalog,
    ctx: &AdapterContext,
    params: &SkillManagerInstallParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let preview = build_install_preview(ctx, params)?;
    ensure_confirmed(&preview, params.confirmed, params.preview_token.as_deref())?;
    let output = run_previewed_command(ctx, &preview)?;
    let scan = scan_all_catalog_report(ctx, catalog)?;
    let updated_skills = catalog.list_skill_records()?;
    Ok(SkillManagerMutationRecord {
        preview,
        output: Some(output),
        applied: true,
        scanned_count: scan.scanned_count,
        updated_skills,
    })
}

pub fn preview_remove_with_manager(
    ctx: &AdapterContext,
    params: &SkillManagerRemoveParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let preview = build_remove_preview(ctx, params)?;
    Ok(SkillManagerMutationRecord {
        preview,
        output: None,
        applied: false,
        scanned_count: 0,
        updated_skills: Vec::new(),
    })
}

pub fn apply_remove_with_manager(
    catalog: &Catalog,
    ctx: &AdapterContext,
    params: &SkillManagerRemoveParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let preview = build_remove_preview(ctx, params)?;
    ensure_confirmed(&preview, params.confirmed, params.preview_token.as_deref())?;
    let output = run_previewed_command(ctx, &preview)?;
    let scan = scan_all_catalog_report(ctx, catalog)?;
    let updated_skills = catalog.list_skill_records()?;
    Ok(SkillManagerMutationRecord {
        preview,
        output: Some(output),
        applied: true,
        scanned_count: scan.scanned_count,
        updated_skills,
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
    })
}

pub fn apply_update_with_manager(
    catalog: &Catalog,
    ctx: &AdapterContext,
    params: &SkillManagerUpdateParams,
) -> Result<SkillManagerMutationRecord, CommandError> {
    let preview = build_update_preview(ctx, params)?;
    ensure_confirmed(&preview, params.confirmed, params.preview_token.as_deref())?;
    let output = run_previewed_command(ctx, &preview)?;
    let scan = scan_all_catalog_report(ctx, catalog)?;
    let updated_skills = catalog.list_skill_records()?;
    Ok(SkillManagerMutationRecord {
        preview,
        output: Some(output),
        applied: true,
        scanned_count: scan.scanned_count,
        updated_skills,
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
    let output = run_previewed_command(ctx, &preview)?;
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
    let agents = normalize_manager_agents(&params.agents)?;
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
    let network_required = source_requires_network(source);
    command_preview(
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
    let agents = normalize_manager_agents(&params.agents)?;
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
        },
    )
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
    let agents = normalize_manager_agents(&params.agents)?;
    append_agent_args(&mut args, &agents);
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
            summary: format!(
                "Update managed skills for {} supported agent target(s).",
                agents.len()
            ),
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

fn command_preview(
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
    let preview_token = preview_token(
        &command,
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
        requires_confirmation: draft.operation != "search" && draft.operation != "listInstalled",
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
) -> Result<SkillManagerCommandOutput, CommandError> {
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
    let output = command.output().map_err(|error| {
        CommandError::SkillManagerCommandFailed(format!(
            "failed to run {}: {error}",
            preview.command.join(" ")
        ))
    })?;
    let status = if output.status.success() {
        "completed"
    } else {
        "failed"
    };
    let stdout = redact_command_output(ctx, &String::from_utf8_lossy(&output.stdout));
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
    Ok(record)
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

fn manager_agent_alias(agent: &str) -> Result<String, CommandError> {
    let normalized = agent.trim().to_ascii_lowercase();
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

fn source_requires_network(source: &str) -> bool {
    let source = source.trim();
    !(source.starts_with('.')
        || source.starts_with('/')
        || source.starts_with("file://")
        || Path::new(source).exists())
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
    Ok(records_from_json_value(&value))
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
            SkillManagerInstalledRecord {
                name,
                source: string_field(&item, &["source", "package", "repository", "repo", "url"]),
                agents: string_array_field(&item, &["agents", "agent_targets", "agentTargets"]),
                scope: string_field(&item, &["scope"]),
                path: string_field(&item, &["path", "target", "target_path", "targetPath"]),
                raw: item,
            }
        })
        .collect()
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
    let mut truncated = value.chars().take(MAX_CAPTURE_BYTES).collect::<String>();
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
            agents: Vec::new(),
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
