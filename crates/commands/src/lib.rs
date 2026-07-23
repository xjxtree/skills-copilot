use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs, io,
    io::Write,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use fs4::FileExt;
use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skills_copilot_adapters::{
    codex_home_dir, codex_plugin_cache_id, codex_plugin_is_effectively_enabled,
    parse_codex_plugin_states, parse_codex_skill_config_entries, pi_agent_dir,
    pi_skill_enabled_by_settings, ClaudeCodeAdapter, CodexAdapter, HermesAdapter, OpenclawAdapter,
    OpencodeAdapter, PiAdapter,
};
use skills_copilot_ai_core::{evaluate_mvp_rules, Finding, RuleContext, RuleReport, Severity};
use skills_copilot_catalog::{
    Catalog, CatalogCommitError, CatalogImmediateTransaction, ConfigSnapshotDraft,
    ConfigSnapshotRecord, ConflictGroupDraft, ConflictGroupRecord, FindingTriageRecord,
    RuleFindingDraft, RuleFindingRecord, RuleTuningRecord, SkillDefinitionDraft, SkillDetailRecord,
    SkillEventDraft, SkillEventRecord, SkillInstanceMeta, SkillRecord,
};
use skills_copilot_core::{
    ActionDescriptor, ActionImpact, ActionIntent, ActionKind, ActionNetworkPosture,
    ActionReadbackDomain, ActionTargetKind, ActionTargetRef, AdapterContext, AgentAdapter,
    AgentConfigDocument, AgentId, ConfigFormat, NetworkAccess, PermissionRequest, RootSource,
    Scope, SkillInstance, SkillState,
};
use skills_copilot_scanner::{scan_agent, ScanIssueKind, ScanReport};

#[cfg(test)]
use skills_copilot_core::SkillScript;

mod action_lifecycle;
mod analysis;
mod app_data_owner_fs;
mod config_consistency;
mod config_lifecycle;
mod config_support;
mod error;
mod external_target;
mod history;
mod local_skill_import;
mod mutation_lock;
mod product_projection;
mod script_execution;
mod skill_bundle;
mod skill_install_guard;
mod skill_manager;
mod transaction_lifecycle;

use analysis::{
    dedupe_rule_finding_records, dedupe_rule_findings, validate_finding_triage_status,
    validate_rule_scope, validate_rule_severity_override, validate_rule_suppression_reason,
    validate_rule_tuning_key,
};
pub use app_data_owner_fs::AppDataOwnerFs;
use config_consistency::{
    canonical_snapshot_project_root, config_content_digest, config_revision,
    ensure_expected_revision, read_config_state, snapshot_binding_revision,
    snapshot_project_root_for_scope, validate_snapshot_project_binding, ConfigState,
};
use config_support::{
    agent_from_snapshot, batch_capability_label, batch_capability_labels, batch_skip_reason,
    batch_snapshot_rollback_notes, minimal_skill_instance, normalize_initial_config_text,
    patch_enabled_for_agent, scope_from_snapshot, validate_config_read_target,
};
use external_target::{ExternalFileState, ExternalTargetCapability};
pub use mutation_lock::{
    lock_app_mutations, lock_or_create_app_mutations, lock_or_create_app_mutations_with_parents,
    AppMutationLock,
};
use skill_install_guard::*;
use transaction_lifecycle::rollback_catalog_before_compensation;

const MAX_EXTERNAL_CONFIG_BYTES: u64 = 8 * 1024 * 1024;
const MAX_DIRECT_SKILL_BYTES: u64 = 2 * 1024 * 1024;

pub use action_lifecycle::*;
pub use analysis::*;
pub use config_lifecycle::*;
pub use config_support::read_agent_config;
pub use error::*;
pub use history::*;
pub(crate) use local_skill_import::{
    register_tool_global_staged_skill, register_tool_global_staged_skill_content,
    tool_global_skill_name_from_content,
};
pub use product_projection::*;
pub use script_execution::*;
pub use skill_bundle::*;
pub(crate) use skill_bundle::{
    import_audit_summary, parse_tool_global_skill, short_hash, stable_tool_global_instance_id,
};
pub use skill_manager::*;

fn verify_app_data_owner_binding_after_effect(
    owner_lock: &AppMutationLock,
    operation: &str,
) -> Result<(), CommandError> {
    owner_lock
        .validate_owner_path_binding()
        .map_err(|error| CommandError::PartialEffect {
            operation: operation.to_string(),
            state: "outcome_unknown",
            cleanup_required: false,
            detail: format!(
                "the confirmed mutation was committed and verified on the accepted app-data owner, but that owner is no longer bound to the configured app-data path: {error}"
            ),
        })
}

#[cfg(test)]
pub(crate) use config_lifecycle::{
    commit_prepared_claude_settings_save_with_after_lock,
    commit_prepared_claude_settings_save_with_hooks, rollback_snapshot_with_after_lock,
    rollback_snapshot_with_hooks,
};
#[cfg(test)]
pub(crate) use skill_bundle::permissions_from_frontmatter;

pub fn app_version() -> &'static str {
    env!("CARGO_PKG_VERSION")
}

pub fn scan_claude_to_catalog(
    ctx: &AdapterContext,
    catalog: &Catalog,
) -> Result<usize, CommandError> {
    Ok(scan_claude_catalog_report(ctx, catalog)?.scanned_count)
}

pub fn scan_claude_catalog_report(
    ctx: &AdapterContext,
    catalog: &Catalog,
) -> Result<AgentCatalogScanReport, CommandError> {
    scan_single_agent_catalog_report(&ClaudeCodeAdapter, ctx, catalog)
}

#[derive(Debug, Clone)]
pub struct CatalogScanReport {
    pub scanned_count: usize,
    pub agents: Vec<AgentCatalogScanReport>,
}

#[derive(Debug, Clone)]
pub struct AgentCatalogScanReport {
    pub agent: AgentId,
    pub display_name: &'static str,
    pub scanned_count: usize,
    pub roots_considered: Vec<PathBuf>,
    pub scanned_roots: Vec<PathBuf>,
    pub partial_roots: Vec<PathBuf>,
    pub skipped_roots: Vec<PathBuf>,
    pub issues: Vec<AgentCatalogScanIssue>,
    pub root_aliases: Vec<AgentCatalogScanPathAlias>,
    pub budget_exhausted: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AgentCatalogScanIssue {
    pub path: PathBuf,
    pub kind: &'static str,
    pub detail: String,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AgentCatalogScanPathAlias {
    pub declared: PathBuf,
    pub canonical: PathBuf,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterCapabilityRecord {
    pub agent: &'static str,
    pub display_name: &'static str,
    pub status: &'static str,
    pub scan: AdapterFeatureCapability,
    pub project_scan: AdapterFeatureCapability,
    pub config_toggle: AdapterFeatureCapability,
    pub config_snapshot: AdapterFeatureCapability,
    pub install: AdapterFeatureCapability,
    pub writable: AdapterFeatureCapability,
    pub blockers: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterDiagnosticsRecord {
    pub agent: &'static str,
    pub display_name: &'static str,
    pub status: &'static str,
    pub roots: Vec<AdapterDiagnosticRootRecord>,
    pub config: AdapterDiagnosticConfigSummary,
    pub access: AdapterDiagnosticAccessSummary,
    pub last_scan: AdapterDiagnosticLastScan,
    pub blockers: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterDiagnosticRootRecord {
    pub path: String,
    pub scope: &'static str,
    pub source: &'static str,
    pub exists: bool,
    pub status: &'static str,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterDiagnosticConfigSummary {
    pub status: &'static str,
    pub detected_count: usize,
    pub paths: Vec<AdapterDiagnosticConfigPath>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterDiagnosticConfigPath {
    pub path: String,
    pub detected: bool,
    pub status: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterDiagnosticAccessSummary {
    pub read_only: bool,
    pub writable_supported: bool,
    pub writable_status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writable_reason: Option<&'static str>,
    pub install_supported: bool,
    pub install_status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub install_reason: Option<&'static str>,
    pub read_only_reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterDiagnosticLastScan {
    pub status: &'static str,
    pub reason: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct AdapterFeatureCapability {
    pub supported: bool,
    pub status: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub reason: Option<&'static str>,
}

impl AdapterFeatureCapability {
    fn supported(status: &'static str) -> Self {
        Self {
            supported: true,
            status,
            reason: None,
        }
    }

    fn supported_with_reason(status: &'static str, reason: &'static str) -> Self {
        Self {
            supported: true,
            status,
            reason: Some(reason),
        }
    }

    fn blocked(status: &'static str, reason: &'static str) -> Self {
        Self {
            supported: false,
            status,
            reason: Some(reason),
        }
    }
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BatchTogglePreviewRecord {
    pub action: ActionDescriptor,
    pub preconditions: Vec<ActionPrecondition>,
    pub preview_token: String,
    pub target_enabled: bool,
    pub requested_count: usize,
    pub writable_count: usize,
    pub skipped_count: usize,
    pub writes_allowed: bool,
    pub affected_items: Vec<BatchToggleAffectedItem>,
    pub skipped_items: Vec<BatchToggleSkippedItem>,
    pub capability_labels: Vec<String>,
    pub snapshot_rollback_notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BatchToggleApplyRecord {
    pub action: ActionDescriptor,
    pub preview_token: String,
    pub target_enabled: bool,
    pub requested_count: usize,
    pub writable_count: usize,
    pub skipped_count: usize,
    pub applied_count: usize,
    pub writes_allowed: bool,
    pub affected_items: Vec<BatchToggleAffectedItem>,
    pub skipped_items: Vec<BatchToggleSkippedItem>,
    pub capability_labels: Vec<String>,
    pub snapshot_rollback_notes: Vec<String>,
    pub updated_records: Vec<SkillRecord>,
    pub readback: ActionReadbackRecord,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BatchToggleAffectedItem {
    pub instance_id: String,
    pub name: String,
    pub agent: String,
    pub scope: String,
    pub current_enabled: bool,
    pub target_enabled: bool,
    pub config_scope: String,
    pub config_target: String,
    pub config_revision: String,
    pub catalog_revision: String,
    pub capability_label: String,
    pub snapshot_plan: String,
    pub rollback_plan: String,
}

#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
pub struct BatchToggleSkippedItem {
    pub instance_id: String,
    pub name: Option<String>,
    pub agent: Option<String>,
    pub scope: Option<String>,
    pub reason: String,
    pub capability_label: Option<String>,
}

pub fn scan_all_to_catalog(ctx: &AdapterContext, catalog: &Catalog) -> Result<usize, CommandError> {
    Ok(scan_all_catalog_report(ctx, catalog)?.scanned_count)
}

pub fn tool_global_staging_skills_root(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("tool-global").join("skills")
}

pub fn ensure_tool_global_staging_skills_root(
    app_data_dir: &Path,
) -> Result<PathBuf, CommandError> {
    let root = tool_global_staging_skills_root(app_data_dir);
    fs::create_dir_all(&root)?;
    Ok(root)
}

pub fn upsert_tool_global_staging_skill(
    catalog: &Catalog,
    ctx: &AdapterContext,
    app_data_dir: &Path,
    skill_path: &Path,
) -> Result<SkillRecord, CommandError> {
    let previous_fingerprints = catalog.instance_fingerprints()?;
    let root = tool_global_staging_skills_root(app_data_dir);
    let canonical_path = validate_tool_global_staging_skill_path(&root, skill_path)?;
    let mut instance = ClaudeCodeAdapter
        .parse(&canonical_path)
        .map_err(|err| CommandError::Adapter(err.message))?;
    instance.agent = AgentId::ToolGlobal;
    instance.scope = Scope::ToolGlobal;
    instance.project_root = None;
    instance.path = canonical_path.clone();
    instance.display_path = display_path_under_app_data(app_data_dir, &canonical_path);
    instance.id = stable_catalog_id(
        AgentId::ToolGlobal.as_str(),
        Scope::ToolGlobal.as_str(),
        &canonical_path,
    );
    instance.definition_id = canonical_definition_id(&instance.name);
    instance.fingerprint = content_fingerprint(&instance.frontmatter_raw, &instance.body);
    if let Ok(metadata) = fs::metadata(&canonical_path) {
        instance.mtime = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or_default();
    }
    instance.first_seen = instance.mtime;
    instance.last_seen = instance.mtime;

    let id = instance.id.clone();
    catalog.upsert_skill_instance(&instance)?;
    refresh_catalog_rule_outputs(catalog, ctx, previous_fingerprints)?;
    catalog
        .get_skill_record(&id)?
        .ok_or(CommandError::InstanceNotFound(id))
}

fn validate_tool_global_staging_skill_path(
    staging_root: &Path,
    skill_path: &Path,
) -> Result<PathBuf, CommandError> {
    if skill_path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md") {
        return Err(CommandError::UnsafeConfigPath(format!(
            "tool-global staging path must point to SKILL.md: {}",
            skill_path.display()
        )));
    }
    reject_symlink(skill_path, "tool-global staging skill")?;
    let canonical_root = staging_root.canonicalize()?;
    let canonical_path = skill_path.canonicalize()?;
    if !canonical_path.starts_with(&canonical_root) {
        return Err(CommandError::UnsafeConfigPath(format!(
            "tool-global staging skill {} resolves outside staging root {}",
            canonical_path.display(),
            canonical_root.display()
        )));
    }
    Ok(canonical_path)
}

fn display_path_under_app_data(app_data_dir: &Path, canonical_path: &Path) -> PathBuf {
    match app_data_dir.canonicalize() {
        Ok(canonical_app_data_dir) => canonical_path
            .strip_prefix(&canonical_app_data_dir)
            .map(|relative| PathBuf::from("$APP_DATA").join(relative))
            .unwrap_or_else(|_| canonical_path.to_path_buf()),
        Err(_) => canonical_path.to_path_buf(),
    }
}

fn stable_catalog_id(agent: &str, scope: &str, path: &Path) -> String {
    hash_string(&format!("{}|{}|{}", agent, scope, path.to_string_lossy()))
}

fn canonical_definition_id(name: &str) -> String {
    hash_string(&name.to_ascii_lowercase())
}

fn content_fingerprint(frontmatter: &str, body: &str) -> String {
    hash_string(&format!("{frontmatter}\n---\n{body}"))
}

fn hash_string(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("{digest:x}")
}

pub fn scan_all_catalog_report(
    ctx: &AdapterContext,
    catalog: &Catalog,
) -> Result<CatalogScanReport, CommandError> {
    let previous_fingerprints = catalog.instance_fingerprints()?;
    let mut scanned_count = 0;
    let mut agents = Vec::new();
    for adapter in supported_scan_adapters() {
        let roots_considered: Vec<PathBuf> = adapter
            .roots(ctx)
            .into_iter()
            .map(|root| root.path)
            .collect();
        let report = scan_agent_for_catalog(adapter.as_ref(), ctx)?;
        scanned_count += report.instances.len();
        update_catalog_from_scan_report(adapter.as_ref(), ctx, catalog, &report)?;
        agents.push(agent_catalog_scan_report(
            adapter.as_ref(),
            roots_considered,
            &report,
        ));
    }
    refresh_catalog_rule_outputs(catalog, ctx, previous_fingerprints)?;
    Ok(CatalogScanReport {
        scanned_count,
        agents,
    })
}

fn scan_issue_kind_key(kind: ScanIssueKind) -> &'static str {
    match kind {
        ScanIssueKind::RootUnavailable => "root_unavailable",
        ScanIssueKind::RootOutsideAllowlist => "root_outside_allowlist",
        ScanIssueKind::DanglingSymlink => "dangling_symlink",
        ScanIssueKind::DirectoryUnreadable => "directory_unreadable",
        ScanIssueKind::EntryUnreadable => "entry_unreadable",
        ScanIssueKind::FileUnreadable => "file_unreadable",
        ScanIssueKind::FileTooLarge => "file_too_large",
        ScanIssueKind::BudgetExceeded => "budget_exceeded",
    }
}

fn scan_single_agent_to_catalog(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
    catalog: &Catalog,
) -> Result<usize, CommandError> {
    Ok(scan_single_agent_catalog_report(adapter, ctx, catalog)?.scanned_count)
}

fn scan_single_agent_catalog_report(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
    catalog: &Catalog,
) -> Result<AgentCatalogScanReport, CommandError> {
    let previous_fingerprints = catalog.instance_fingerprints()?;
    let roots_considered = adapter
        .roots(ctx)
        .into_iter()
        .map(|root| root.path)
        .collect();
    let report = scan_agent_for_catalog(adapter, ctx)?;
    update_catalog_from_scan_report(adapter, ctx, catalog, &report)?;
    refresh_catalog_rule_outputs(catalog, ctx, previous_fingerprints)?;
    Ok(agent_catalog_scan_report(
        adapter,
        roots_considered,
        &report,
    ))
}

fn agent_catalog_scan_report(
    adapter: &dyn AgentAdapter,
    roots_considered: Vec<PathBuf>,
    report: &ScanReport,
) -> AgentCatalogScanReport {
    AgentCatalogScanReport {
        agent: adapter.id(),
        display_name: adapter.display_name(),
        scanned_count: report.instances.len(),
        roots_considered,
        scanned_roots: report.scanned_roots.clone(),
        partial_roots: report.partial_roots.clone(),
        skipped_roots: report.skipped_roots.clone(),
        issues: report
            .issues
            .iter()
            .map(|issue| AgentCatalogScanIssue {
                path: issue.path.clone(),
                kind: scan_issue_kind_key(issue.kind),
                detail: issue.detail.clone(),
            })
            .collect(),
        root_aliases: report
            .root_aliases
            .iter()
            .map(|alias| AgentCatalogScanPathAlias {
                declared: alias.declared.clone(),
                canonical: alias.canonical.clone(),
            })
            .collect(),
        budget_exhausted: report.stats.budget_exhausted,
    }
}

fn update_catalog_from_scan_report(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
    catalog: &Catalog,
    report: &ScanReport,
) -> Result<(), CommandError> {
    catalog.upsert_skill_instances(&report.instances)?;
    let seen: Vec<(String, std::path::PathBuf)> = report
        .instances
        .iter()
        .map(|inst| (inst.scope.as_str().to_string(), inst.path.clone()))
        .collect();
    let exact_agent_scan = report.partial_roots.is_empty()
        && report.scoped_partial_roots.is_empty()
        && report.skipped_roots.is_empty()
        && !report.stats.budget_exhausted;
    if exact_agent_scan {
        catalog.mark_missing_except_agent_for_project_context(
            adapter.id().as_str(),
            ctx.project_root.as_deref(),
            &seen,
        )?;
    } else if !report.scoped_scanned_roots.is_empty() {
        let scoped_roots = report
            .scoped_scanned_roots
            .iter()
            .map(|root| (root.scope, root.path.clone()))
            .collect::<Vec<_>>();
        catalog.mark_missing_except_scoped_for_project_context(
            adapter.id().as_str(),
            ctx.project_root.as_deref(),
            &scoped_roots,
            &seen,
        )?;
    }
    Ok(())
}

fn scan_agent_for_catalog(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
) -> Result<skills_copilot_scanner::ScanReport, CommandError> {
    let mut report = scan_agent(adapter, ctx)?;
    if adapter.id() == AgentId::Codex {
        apply_codex_config_overrides(ctx, &mut report.instances)?;
    } else if adapter.id() == AgentId::Pi {
        apply_pi_config_overrides(ctx, &mut report.instances)?;
    }
    Ok(report)
}

fn supported_scan_adapters() -> Vec<Box<dyn AgentAdapter>> {
    vec![
        Box::new(ClaudeCodeAdapter),
        Box::new(CodexAdapter),
        Box::new(OpencodeAdapter),
        Box::new(PiAdapter),
        Box::new(OpenclawAdapter),
        Box::new(HermesAdapter),
    ]
}

pub fn list_adapter_capabilities(_ctx: &AdapterContext) -> Vec<AdapterCapabilityRecord> {
    vec![
        AdapterCapabilityRecord {
            agent: AgentId::ClaudeCode.as_str(),
            display_name: "Claude Code",
            status: "verified",
            scan: AdapterFeatureCapability::supported("verified"),
            project_scan: AdapterFeatureCapability::supported("verified"),
            config_toggle: AdapterFeatureCapability::supported("verified"),
            config_snapshot: AdapterFeatureCapability::supported("verified"),
            install: AdapterFeatureCapability::supported("verified"),
            writable: AdapterFeatureCapability::supported("verified"),
            blockers: Vec::new(),
        },
        AdapterCapabilityRecord {
            agent: AgentId::Codex.as_str(),
            display_name: "Codex",
            status: "verified",
            scan: AdapterFeatureCapability::supported_with_reason(
                "verified-filesystem",
                "Scans verified user/project .agents/skills, read-only CODEX_HOME skills, installed plugin manifest roots, and admin roots from persistent local files.",
            ),
            project_scan: AdapterFeatureCapability::supported_with_reason(
                "verified-bounded",
                "Project discovery is bounded to .agents/skills from the selected working directory through the selected project root.",
            ),
            config_toggle: AdapterFeatureCapability::supported_with_reason(
                "verified-native-roots-only",
                "Project skill toggles write only the user config.toml override for verified user/project .agents/skills instances; project-local .codex/config.toml remains diagnostic-only.",
            ),
            config_snapshot: AdapterFeatureCapability::supported("verified"),
            install: AdapterFeatureCapability::supported("verified"),
            writable: AdapterFeatureCapability::supported_with_reason(
                "verified-native-roots-only",
                "Codex writes are limited to verified user/project .agents/skills instances and the user config.toml override; CODEX_HOME, installed plugin, admin, and system skills are read-only.",
            ),
            blockers: vec![
                "Project-local .codex/config.toml toggle semantics remain unverified.",
                "CODEX_HOME, installed plugin, admin, and system skills are read-only.",
                "Enabled installed-plugin manifests are resolved to bounded versioned copies; the plugin store/cache is never walked broadly, and staging, marketplace source, and unrelated cache content are excluded.",
            ],
        },
        AdapterCapabilityRecord {
            agent: AgentId::Opencode.as_str(),
            display_name: "opencode",
            status: "verified",
            scan: AdapterFeatureCapability::supported_with_reason(
                "verified-configured-local-paths",
                "Scans native roots, official .claude/.agents compatibility roots, exact direct skill-link targets, and local skills.paths directories declared in opencode JSON/JSONC config; skills.urls are never fetched.",
            ),
            project_scan: AdapterFeatureCapability::supported_with_reason(
                "verified-configured-local-paths",
                "Project scan keeps cwd-to-project native/compat roots and accepts project-local skills.paths after canonicalization and project-boundary checks.",
            ),
            config_toggle: AdapterFeatureCapability::supported_with_reason(
                "verified-exact-skill-deny",
                "V2.12 writes exact permission.skill.<name> = deny and re-enables by removing that exact deny without changing wildcard rules.",
            ),
            config_snapshot: AdapterFeatureCapability::supported_with_reason(
                "verified",
                "opencode global/project opencode.json writes use the same snapshot, atomic write, verify, and rollback path as other writable adapters.",
            ),
            install: AdapterFeatureCapability::supported_with_reason(
                "verified",
                "Tool-global skills can be installed to native opencode user/project skill roots after confirmation; compatibility and configured roots are scanned but not install targets.",
            ),
            writable: AdapterFeatureCapability::supported_with_reason(
                "verified",
                "Writable support uses managed exact skill permission overrides; skill-file writes and installs stay limited to native opencode roots.",
            ),
            blockers: vec![
                "skills.urls are recognized as config scope but remain metadata-only; scans and diagnostics do not fetch network skill indexes.",
                "Absolute OPENCODE_CONFIG/OPENCODE_CONFIG_DIR and inline OPENCODE_CONFIG_CONTENT are read for local discovery; remote organization config is never fetched, and managed settings remain read-only.",
            ],
        },
        AdapterCapabilityRecord {
            agent: AgentId::Pi.as_str(),
            display_name: "Pi",
            status: "guarded",
            scan: AdapterFeatureCapability::supported_with_reason(
                "verified-current-sources",
                "Scans installed package/configured roots first, cwd .pi/skills, ancestor .agents/skills, and native user roots in Pi's first-name-wins order without fetching package indexes.",
            ),
            project_scan: AdapterFeatureCapability::supported_with_reason(
                "verified-compatibility-roots",
                "Project scan reads cwd .pi/skills and walks only .agents/skills compatibility roots from cwd through the selected git/project root.",
            ),
            config_toggle: AdapterFeatureCapability::supported_with_reason(
                "guarded-v2.94",
                "Writes Pi's documented skills array exact +path/-path overrides with pre-toggle snapshots, atomic verification, and rollback; explicit untrusted project markers block project writes.",
            ),
            config_snapshot: AdapterFeatureCapability::supported_with_reason(
                "guarded-v2.94",
                "Pi toggle snapshots use the existing config snapshot and rollback path; redacted snapshots are not directly rollbackable.",
            ),
            install: AdapterFeatureCapability::supported_with_reason(
                "verified-native-roots",
                "Tool-global skills can be installed to Pi native ~/.pi/agent/skills and project .pi/skills roots after confirmation; .agents compatibility roots are scanned/toggleable but not install targets.",
            ),
            writable: AdapterFeatureCapability::supported_with_reason(
                "guarded-v2.94",
                "Writable support covers guarded enable/disable settings updates for Pi-native and .agents compatibility roots plus native-root installs; script execution, AI writes, credentials, package install/remove, and compatibility-root skill-file installs remain blocked.",
            ),
            blockers: vec![
                "Pi package install/remove remains blocked.",
                "Explicit untrusted Pi project settings block project/package toggles.",
                ".agents compatibility roots are not direct install targets.",
            ],
        },
        AdapterCapabilityRecord {
            agent: AgentId::Hermes.as_str(),
            display_name: "Hermes",
            status: "guarded",
            scan: AdapterFeatureCapability::supported_with_reason(
                "verified-read-only",
                "V2.38 scans active Hermes home ~/.hermes/skills/**/SKILL.md plus explicit skills.external_dirs as read-only external roots without reading Hermes secrets, cron content, or logs.",
            ),
            project_scan: AdapterFeatureCapability::blocked(
                "blocked",
                "Hermes has no generic project-local skill discovery; explicit skills.external_dirs are read-only external roots, not project roots.",
            ),
            config_toggle: AdapterFeatureCapability::supported_with_reason(
                "verified-skills-disabled-v2.97",
                "V2.97 writes the documented global skills.disabled list in ~/.hermes/config.yaml with pre-toggle snapshots, atomic write verification, and rollback.",
            ),
            config_snapshot: AdapterFeatureCapability::supported_with_reason(
                "verified-v2.97",
                "Hermes config toggle snapshots use the existing redacted config snapshot and rollback path.",
            ),
            install: AdapterFeatureCapability::supported_with_reason(
                "verified-native-root-v2.95",
                "V2.95 supports confirmed local tool-global SKILL.md copy into the Hermes native ~/.hermes/skills root only; Hermes hub, URL, tap, update, uninstall, and external_dirs writes remain out of scope.",
            ),
            writable: AdapterFeatureCapability::supported_with_reason(
                "guarded-v2.97",
                "Writable support covers global skills.disabled toggles and confirmed local skill-file installs into ~/.hermes/skills; per-platform enablement, external_dirs writes, hub/network installs, scripts, credentials, cloud sync, and telemetry remain blocked.",
            ),
            blockers: vec![
                "Generic Hermes project-local skill discovery is not confirmed and remains disabled.",
                "Hermes external_dirs are scan-only external roots, not writable or install targets.",
                "V2.97 only writes global skills.disabled; platform_disabled and other Hermes config keys remain read-only.",
                "Hermes hub, URL, tap, update, uninstall, and reset operations remain blocked.",
                "Do not map Hermes cron jobs to SkillInstance.",
            ],
        },
        AdapterCapabilityRecord {
            agent: AgentId::Openclaw.as_str(),
            display_name: "OpenClaw",
            status: "guarded",
            scan: AdapterFeatureCapability::supported_with_reason(
                "verified-read-only",
                "Scans documented OpenClaw roots in runtime precedence, including effectively enabled plugin manifests and configured local roots, then projects allowlists and eligibility without calling the OpenClaw CLI.",
            ),
            project_scan: AdapterFeatureCapability::supported_with_reason(
                "verified-read-only",
                "Project scan is limited to confirmed OpenClaw home workspace roots, including selected paths inside those workspaces, and only reads <workspace>/skills plus <workspace>/.agents/skills; arbitrary repo roots are skipped.",
            ),
            config_toggle: AdapterFeatureCapability::supported_with_reason(
                "verified-skills-entries-v2.97",
                "V2.97 writes the documented skills.entries.<key>.enabled boolean in ~/.openclaw/openclaw.json with pre-toggle snapshots, atomic write verification, and rollback.",
            ),
            config_snapshot: AdapterFeatureCapability::supported_with_reason(
                "verified-v2.97",
                "OpenClaw config toggle snapshots use the existing redacted config snapshot and rollback path; JSON5 input is parsed and rewritten as strict JSON.",
            ),
            install: AdapterFeatureCapability::supported_with_reason(
                "verified-native-workspace-v2.96",
                "V2.96 supports confirmed local tool-global SKILL.md copy into OpenClaw native ~/.openclaw/skills and confirmed OpenClaw workspace <workspace>/skills roots only; .agents direct installs and ClawHub/Git/network-backed operations remain out of scope.",
            ),
            writable: AdapterFeatureCapability::supported_with_reason(
                "guarded-v2.97",
                "Writable support covers skills.entries enabled toggles plus confirmed local skill-file installs into native OpenClaw skill roots; .agents direct installs, ClawHub, Git, update, verify, workshop, scripts, credentials, cloud sync, and telemetry remain blocked.",
            ),
            blockers: vec![
                "Arbitrary repository roots are not OpenClaw projects and are not scanned as project roots.",
                "OpenClaw .agents roots are scan-only and are not direct install targets.",
                "V2.97 only writes skills.entries.<key>.enabled; agent allowlists, env/apiKey, install policy, workshop, and load roots remain read-only.",
                "OpenClaw ClawHub, Git, update, verify, workshop, and network-backed operations remain blocked.",
            ],
        },
    ]
}

pub fn list_adapter_diagnostics(ctx: &AdapterContext) -> Vec<AdapterDiagnosticsRecord> {
    list_adapter_diagnostics_for_agents(ctx, &[])
}

pub fn list_adapter_diagnostics_for_agents(
    ctx: &AdapterContext,
    agents: &[&str],
) -> Vec<AdapterDiagnosticsRecord> {
    let agent_filter = if agents.is_empty() {
        None
    } else {
        Some(
            agents
                .iter()
                .map(|agent| agent.to_ascii_lowercase())
                .collect::<BTreeSet<_>>(),
        )
    };
    let capabilities = list_adapter_capabilities(ctx);
    supported_scan_adapters()
        .into_iter()
        .filter(|adapter| {
            agent_filter
                .as_ref()
                .is_none_or(|agents| agents.contains(adapter.id().as_str()))
        })
        .filter_map(|adapter| {
            let capability = capabilities
                .iter()
                .find(|capability| capability.agent == adapter.id().as_str())?;
            let roots = adapter
                .roots(ctx)
                .into_iter()
                .map(|root| adapter_diagnostic_root(adapter.id(), root))
                .collect();
            let config_paths: Vec<AdapterDiagnosticConfigPath> = adapter
                .config_paths(ctx)
                .into_iter()
                .map(|path| {
                    let detected = path.exists();
                    AdapterDiagnosticConfigPath {
                        path: path.to_string_lossy().to_string(),
                        detected,
                        status: if detected { "detected" } else { "missing" },
                    }
                })
                .collect();
            let detected_count = config_paths.iter().filter(|path| path.detected).count();
            let config = adapter_diagnostic_config(adapter.id(), config_paths, detected_count);
            let read_only = !capability.writable.supported;
            let access = AdapterDiagnosticAccessSummary {
                read_only,
                writable_supported: capability.writable.supported,
                writable_status: capability.writable.status,
                writable_reason: capability.writable.reason,
                install_supported: capability.install.supported,
                install_status: capability.install.status,
                install_reason: capability.install.reason,
                read_only_reason: adapter_read_only_reason(capability),
            };
            Some(AdapterDiagnosticsRecord {
                agent: capability.agent,
                display_name: capability.display_name,
                status: capability.status,
                roots,
                config,
                access,
                last_scan: AdapterDiagnosticLastScan {
                    status: "not-run",
                    reason: "No scan activity is persisted in adapter diagnostics; catalog.scanAll returns per-agent activity for the current refresh.",
                },
                blockers: capability.blockers.clone(),
            })
        })
        .collect()
}

fn adapter_diagnostic_root(
    agent: AgentId,
    root: skills_copilot_core::AdapterRoot,
) -> AdapterDiagnosticRootRecord {
    let exists = root.path.exists();
    let status = if exists {
        "discovered"
    } else {
        "skipped-missing"
    };
    AdapterDiagnosticRootRecord {
        path: root.path.to_string_lossy().to_string(),
        scope: root.scope.as_str(),
        source: root_source_label(&root.source),
        exists,
        status,
        reason: adapter_root_reason(agent, &root, exists),
    }
}

fn adapter_diagnostic_config(
    agent: AgentId,
    paths: Vec<AdapterDiagnosticConfigPath>,
    detected_count: usize,
) -> AdapterDiagnosticConfigSummary {
    if paths.is_empty() {
        return AdapterDiagnosticConfigSummary {
            status: "blocked",
            detected_count: 0,
            paths,
            reason: match agent {
                AgentId::Hermes => "Hermes config mutation and rollback-safe skill toggle targets are unverified; diagnostics do not read Hermes secrets or cron content.".to_string(),
                AgentId::Openclaw => "OpenClaw plugin config is not a verified skill toggle contract; writable/install remains blocked.".to_string(),
                _ => "No verified adapter config path is declared for this agent.".to_string(),
            },
        };
    }

    AdapterDiagnosticConfigSummary {
        status: if detected_count > 0 {
            "detected"
        } else {
            "not-detected"
        },
        detected_count,
        paths,
        reason: if detected_count > 0 {
            "One or more declared adapter config paths exist; contents were not returned."
                .to_string()
        } else {
            "Declared adapter config paths were not present; no config contents were read."
                .to_string()
        },
    }
}

fn adapter_read_only_reason(capability: &AdapterCapabilityRecord) -> String {
    if capability.writable.supported {
        capability
            .writable
            .reason
            .unwrap_or("Writable support is verified for the bounded adapter operations listed in capabilities.")
            .to_string()
    } else {
        capability
            .writable
            .reason
            .unwrap_or("Writable support is blocked for this adapter.")
            .to_string()
    }
}

fn adapter_root_reason(
    agent: AgentId,
    root: &skills_copilot_core::AdapterRoot,
    exists: bool,
) -> String {
    if !exists {
        return "Root is declared by the adapter but does not exist, so scans skip it.".to_string();
    }
    match (agent, &root.source) {
        (AgentId::Hermes, RootSource::Extra) => {
            "Explicit Hermes skills.external_dirs root; read-only scan source, not a project root, writable target, or install target.".to_string()
        }
        (AgentId::Openclaw, RootSource::Project) => {
            "Confirmed OpenClaw workspace root; arbitrary repository roots are not inferred or scanned.".to_string()
        }
        (AgentId::Pi, RootSource::Project) => {
            "Pi native project/package root is scan-capable; writable toggles use project-bound settings snapshots and are blocked by explicit untrusted markers; tool-global direct install may target native Pi roots after confirmation.".to_string()
        }
        (AgentId::Pi, RootSource::Compatibility) => {
            "Pi .agents compatibility root is scan-capable and toggleable through guarded Pi settings, but is never a direct skill-file install target.".to_string()
        }
        (AgentId::Codex, RootSource::Compatibility) => {
            "Codex compatibility/system-observed skills root; scanned read-only and never used as a toggle, install, snapshot, rollback, or config-write target.".to_string()
        }
        (AgentId::Opencode, RootSource::Configured) => {
            "OpenCode skills.paths local directory from opencode config; scanned read-only as a filesystem source and never used as an install target or skill-file write target. skills.urls are not fetched by diagnostics or scans.".to_string()
        }
        (AgentId::Opencode, RootSource::Compatibility) => {
            if root.path.to_string_lossy().contains(".claude/skills") {
                "Official opencode .claude/skills compatibility source; this is effective opencode runtime input, not a Claude row leaking across the selected-agent filter.".to_string()
            } else {
                "Official opencode .agents/skills compatibility source; scanned with opencode permission rules and exact symlink-target authorization.".to_string()
            }
        }
        (AgentId::Codex, RootSource::Admin) => {
            "Codex admin skills root; scanned read-only when present, skipped when missing or unreadable, and never elevated or written.".to_string()
        }
        (AgentId::Codex, RootSource::Plugin) => {
            "Manifest-declared skill root from an effectively enabled installed Codex plugin; scanned read-only without walking the plugin store/cache broadly or running hooks, MCP servers, installers, or network fetches.".to_string()
        }
        (AgentId::Openclaw, RootSource::Plugin) => {
            "Safe manifest-declared root from an effectively enabled OpenClaw plugin; the extension store is not walked as a generic skill source.".to_string()
        }
        (AgentId::Codex, RootSource::System) => {
            "Codex system skills are bundled by Codex; no stable local filesystem root is scanned or written by this product.".to_string()
        }
        _ => "Root is declared by the adapter and exists for read scanning.".to_string(),
    }
}

fn root_source_label(source: &RootSource) -> &'static str {
    match source {
        RootSource::UserHome => "user-home",
        RootSource::Project => "project",
        RootSource::Extra => "extra",
        RootSource::Compatibility => "compatibility",
        RootSource::Configured => "configured",
        RootSource::Admin => "admin",
        RootSource::Plugin => "plugin",
        RootSource::System => "system",
    }
}

pub fn get_skill(catalog: &Catalog, instance_id: &str) -> Result<SkillDetailRecord, CommandError> {
    catalog
        .get_skill_detail(instance_id)?
        .ok_or_else(|| CommandError::InstanceNotFound(instance_id.to_string()))
}

pub fn list_findings(catalog: &Catalog) -> Result<Vec<RuleFindingRecord>, CommandError> {
    Ok(dedupe_rule_finding_records(&catalog.list_rule_findings()?))
}

pub fn list_finding_triage(catalog: &Catalog) -> Result<Vec<FindingTriageRecord>, CommandError> {
    Ok(catalog.list_finding_triage()?)
}

pub fn set_finding_triage(
    catalog: &Catalog,
    triage_key: &str,
    status: &str,
    note: Option<&str>,
) -> Result<FindingTriageRecord, CommandError> {
    validate_finding_triage_status(status)?;
    catalog
        .set_finding_triage(triage_key, status, note, current_time_ms())?
        .ok_or_else(|| CommandError::FindingNotFound(triage_key.to_string()))
}

pub fn clear_finding_triage(catalog: &Catalog, triage_key: &str) -> Result<bool, CommandError> {
    Ok(catalog.clear_finding_triage(triage_key)?)
}

pub fn list_rule_tuning(catalog: &Catalog) -> Result<Vec<RuleTuningRecord>, CommandError> {
    Ok(catalog.list_rule_tuning()?)
}

pub fn set_rule_severity_override(
    catalog: &Catalog,
    rule_id: &str,
    agent: Option<&str>,
    scope: Option<&str>,
    severity: &str,
) -> Result<RuleTuningRecord, CommandError> {
    validate_rule_tuning_key(rule_id)?;
    validate_rule_scope(agent, scope)?;
    validate_rule_severity_override(severity)?;
    Ok(catalog.set_rule_severity_override(rule_id, agent, scope, severity, current_time_ms())?)
}

pub fn clear_rule_severity_override(
    catalog: &Catalog,
    rule_id: &str,
    agent: Option<&str>,
    scope: Option<&str>,
) -> Result<bool, CommandError> {
    validate_rule_tuning_key(rule_id)?;
    validate_rule_scope(agent, scope)?;
    Ok(catalog.clear_rule_severity_override(rule_id, agent, scope, current_time_ms())?)
}

pub fn set_rule_suppression(
    catalog: &Catalog,
    rule_id: &str,
    agent: Option<&str>,
    scope: Option<&str>,
    reason: &str,
    note: Option<&str>,
) -> Result<RuleTuningRecord, CommandError> {
    validate_rule_tuning_key(rule_id)?;
    validate_rule_scope(agent, scope)?;
    validate_rule_suppression_reason(reason)?;
    Ok(catalog.set_rule_suppression(
        rule_id,
        agent,
        scope,
        reason.trim(),
        note.map(str::trim).filter(|value| !value.is_empty()),
        current_time_ms(),
    )?)
}

pub fn clear_rule_suppression(
    catalog: &Catalog,
    rule_id: &str,
    agent: Option<&str>,
    scope: Option<&str>,
) -> Result<bool, CommandError> {
    validate_rule_tuning_key(rule_id)?;
    validate_rule_scope(agent, scope)?;
    Ok(catalog.clear_rule_suppression(rule_id, agent, scope, current_time_ms())?)
}

pub fn list_snapshots(
    catalog: &Catalog,
    ctx: &AdapterContext,
) -> Result<Vec<ConfigSnapshotRecord>, CommandError> {
    let project_root = canonical_snapshot_project_root(ctx)?;
    Ok(catalog.list_all_config_snapshots(project_root.as_deref())?)
}

pub fn list_agent_config_snapshots(
    catalog: &Catalog,
    ctx: &AdapterContext,
    agent: &str,
    scope: Option<&str>,
) -> Result<Vec<ConfigSnapshotRecord>, CommandError> {
    let project_root = canonical_snapshot_project_root(ctx)?;
    Ok(catalog.list_agent_config_snapshots(agent, scope, project_root.as_deref())?)
}

pub fn list_skill_events(
    catalog: &Catalog,
    instance_id: &str,
    limit: Option<usize>,
) -> Result<Vec<SkillEventRecord>, CommandError> {
    Ok(catalog.list_skill_events(instance_id, limit)?)
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillInstallFilePreview {
    pub source: String,
    pub target: String,
    pub kind: String,
    pub will_write: bool,
    pub target_exists: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillInstallConfirmation {
    pub required: bool,
    pub confirmed: bool,
    pub fields: Vec<&'static str>,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillInstallPreviewRecord {
    pub action: ActionDescriptor,
    pub preconditions: Vec<ActionPrecondition>,
    pub preview_token: String,
    pub source_instance_id: String,
    pub source_path: String,
    pub target_agent: String,
    pub target_scope: String,
    pub target_path: String,
    pub files: Vec<SkillInstallFilePreview>,
    pub risks: Vec<String>,
    pub confirmation: SkillInstallConfirmation,
    pub wrote: bool,
    pub snapshot_id: Option<String>,
    pub readback: Option<ActionReadbackRecord>,
}

pub fn install_skill_from_tool_global(
    catalog: &Catalog,
    ctx: &AdapterContext,
    instance_id: &str,
    target_agent: AgentId,
    target_scope: Scope,
    project_path: Option<&Path>,
    action_confirmation: Option<&ActionConfirmation>,
) -> Result<SkillInstallPreviewRecord, CommandError> {
    install_skill_from_tool_global_guarded(
        catalog,
        &ctx.user_home,
        ctx,
        instance_id,
        target_agent,
        target_scope,
        project_path,
        action_confirmation,
    )
}

#[allow(clippy::too_many_arguments)]
pub fn install_skill_from_tool_global_guarded(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    instance_id: &str,
    target_agent: AgentId,
    target_scope: Scope,
    project_path: Option<&Path>,
    action_confirmation: Option<&ActionConfirmation>,
) -> Result<SkillInstallPreviewRecord, CommandError> {
    install_skill_from_tool_global_guarded_with_hooks(
        catalog,
        app_data_dir,
        ctx,
        instance_id,
        target_agent,
        target_scope,
        project_path,
        action_confirmation,
        || {},
        || {},
    )
}

#[allow(clippy::too_many_arguments)] // The final closures are test-only race injection seams.
fn install_skill_from_tool_global_guarded_with_hooks<F, G>(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    instance_id: &str,
    target_agent: AgentId,
    target_scope: Scope,
    project_path: Option<&Path>,
    action_confirmation: Option<&ActionConfirmation>,
    after_confirmation: F,
    after_commit: G,
) -> Result<SkillInstallPreviewRecord, CommandError>
where
    F: FnOnce(),
    G: FnOnce(),
{
    let Some(action_confirmation) = action_confirmation else {
        return preview_skill_install_from_tool_global(
            catalog,
            ctx,
            instance_id,
            target_agent,
            target_scope,
            project_path,
        );
    };
    let preview = validate_skill_install_confirmation(
        catalog,
        ctx,
        instance_id,
        target_agent,
        target_scope,
        project_path,
        action_confirmation,
    )?;
    if !action_confirmation.confirmed {
        return Ok(preview);
    }
    after_confirmation();

    let target = PathBuf::from(&preview.target_path);
    let source = PathBuf::from(&preview.source_path);
    let mutation_lock = mutation_lock::lock_app_mutations(app_data_dir)?;
    mutation_lock.validate_owner_path_binding()?;
    catalog.ensure_mutation_owner(mutation_lock.owner_directory())?;
    let mut target_capability = prepare_skill_target_capability(
        &mutation_lock,
        ctx,
        target_agent,
        target_scope,
        &target,
        project_path,
    )?;
    let expected_catalog = preview
        .preconditions
        .iter()
        .find(|item| item.kind == ActionPreconditionKind::CatalogRecord)
        .ok_or_else(|| {
            CommandError::MismatchedActionReference(
                "install preview is missing its catalog precondition".to_string(),
            )
        })?;
    let current_meta = catalog
        .get_skill_instance_meta(instance_id)?
        .ok_or(CommandError::StaleActionReference)?;
    let current_source = normalize_path_lexically(&current_meta.path);
    let source_relative = tool_global_source_relative(app_data_dir, &current_source)?;
    if current_meta.scope != Scope::ToolGlobal
        || current_source != normalize_path_lexically(&source)
        || batch_catalog_record_revision(&current_meta)? != expected_catalog.expected_revision
    {
        return Err(CommandError::StaleActionReference);
    }
    let owner = mutation_lock.owner_fs();
    let (source_bytes, source_stamp) = owner
        .read_bounded_regular_file_with_stamp(
            &source_relative,
            MAX_DIRECT_SKILL_BYTES,
            "tool-global install source",
        )?
        .ok_or(CommandError::StaleActionReference)?;
    let source_content = String::from_utf8(source_bytes).map_err(|_| {
        CommandError::UnsafeConfigPath("tool-global install source is not valid UTF-8".to_string())
    })?;
    let locked_source = owner_locked_skill_file_state(&source, source_content, &source_stamp)?;
    let locked_target = read_external_skill_state(&target_capability, &target)?;
    let expected_source = preview
        .preconditions
        .iter()
        .find(|item| item.kind == ActionPreconditionKind::SourceFile)
        .ok_or_else(|| {
            CommandError::MismatchedActionReference(
                "install preview is missing its source precondition".to_string(),
            )
        })?;
    let expected_target = preview
        .preconditions
        .iter()
        .find(|item| item.kind == ActionPreconditionKind::TargetFile)
        .ok_or_else(|| {
            CommandError::MismatchedActionReference(
                "install preview is missing its target precondition".to_string(),
            )
        })?;
    if locked_source.binding_revision != expected_source.expected_revision
        || locked_target.revision != expected_target.expected_revision
    {
        return Err(CommandError::StaleActionReference);
    }

    let new_text = locked_source.content.clone();
    let candidate = parse_tool_global_skill(&new_text, &current_meta.name);
    let candidate_definition_id = canonical_definition_id(&candidate.name);
    let candidate_fingerprint = content_fingerprint(&candidate.frontmatter_raw, &candidate.body);
    let applied_target_revision = skill_file_revision(&target, true, &new_text)?;
    let transaction = catalog.begin_immediate_transaction()?;
    let apply_result = (|| {
        target_capability.atomic_replace(new_text.as_bytes())?;
        let written = read_external_skill_state(&target_capability, &target)?;
        if !written.exists || written.content != new_text {
            return Err(CommandError::VerificationFailed);
        }
        let scan_ctx = install_scan_context(ctx, target_agent, target_scope, project_path)?;
        scan_agent_id_to_catalog(target_agent, &scan_ctx, catalog)?;
        run_install_catalog_scan_test_hook(&target);
        let anchored_target = target_capability.anchored_path().to_path_buf();
        let expected_project_root = if target_scope == Scope::AgentProject {
            Some(
                scan_ctx
                    .project_root
                    .as_deref()
                    .ok_or(CommandError::VerificationFailed)?
                    .canonicalize()?,
            )
        } else {
            None
        };
        let target_record = catalog
            .list_skill_records()?
            .into_iter()
            .find(|record| {
                record.agent == target_agent.as_str()
                    && record.scope == target_scope.as_str()
                    && normalize_path_lexically(&record.path)
                        == normalize_path_lexically(&anchored_target)
            })
            .ok_or(CommandError::VerificationFailed)?;
        let target_meta = catalog
            .get_skill_instance_meta(&target_record.id)?
            .ok_or(CommandError::VerificationFailed)?;
        let project_matches = match (
            expected_project_root.as_deref(),
            target_meta.project_root.as_deref(),
        ) {
            (None, None) => true,
            (Some(expected), Some(actual)) => {
                actual.canonicalize().is_ok_and(|actual| actual == expected)
            }
            _ => false,
        };
        if !project_matches {
            return Err(CommandError::VerificationFailed);
        }
        let target_detail = catalog
            .get_skill_detail(&target_record.id)?
            .ok_or(CommandError::VerificationFailed)?;
        if target_record.state == "missing"
            || target_record.definition_id != candidate_definition_id
            || target_record.name != candidate.name
            || target_detail.id != target_record.id
            || target_detail.agent != target_agent.as_str()
            || target_detail.scope != target_scope.as_str()
            || normalize_path_lexically(&target_detail.path)
                != normalize_path_lexically(&anchored_target)
            || target_detail.definition_id != candidate_definition_id
            || target_detail.name != candidate.name
            || target_detail.description != candidate.description
            || target_detail.frontmatter_raw != candidate.frontmatter_raw
            || target_detail.body != candidate.body
            || target_detail.fingerprint != candidate_fingerprint
            || target_detail.state == "missing"
        {
            return Err(CommandError::VerificationFailed);
        }
        let target_state = read_external_skill_state(&target_capability, &target)?;
        if !target_state.exists
            || target_state.content != new_text
            || target_state.revision != applied_target_revision
        {
            return Err(CommandError::VerificationFailed);
        }
        let target_record_json = serde_json::to_string(&target_record)?;
        ActionReadbackRecord::verified(
            &preview.action,
            vec![
                ActionReadbackObservation {
                    domain: ActionReadbackDomain::SkillFiles,
                    target_id: preview.target_path.clone(),
                    revision: target_state.revision,
                },
                ActionReadbackObservation {
                    domain: ActionReadbackDomain::CatalogSkills,
                    target_id: target_record.id,
                    revision: action_source_revision(
                        "catalog.skill.readback",
                        &[("record", &target_record_json)],
                    )?,
                },
            ],
        )
    })();

    let readback = match apply_result {
        Ok(readback) => readback,
        Err(error) => {
            rollback_catalog_before_compensation(
                transaction,
                "skill.install",
                &error,
                "the written skill candidate was preserved for inspection",
            )?;
            restore_skill_install_target(
                &mut target_capability,
                &target,
                &locked_target,
                &applied_target_revision,
            )
            .map_err(|cleanup_error| CommandError::PartialEffect {
                operation: "skill.install".to_string(),
                state: "outcome_unknown",
                cleanup_required: true,
                detail: format!(
                    "apply failed ({error}); target restoration failed ({cleanup_error})"
                ),
            })?;
            return Err(error);
        }
    };
    match transaction.commit_classified() {
        Ok(()) => {}
        Err(CatalogCommitError::NotCommitted(commit_error)) => {
            restore_skill_install_target(
                &mut target_capability,
                &target,
                &locked_target,
                &applied_target_revision,
            )
            .map_err(|cleanup_error| CommandError::PartialEffect {
                operation: "skill.install".to_string(),
                state: "outcome_unknown",
                cleanup_required: true,
                detail: format!(
                    "catalog commit was rejected ({commit_error}); target restoration failed ({cleanup_error})"
                ),
            })?;
            return Err(commit_error.into());
        }
        Err(CatalogCommitError::OutcomeUnknown(commit_error)) => {
            return Err(CommandError::PartialEffect {
                operation: "skill.install".to_string(),
                state: "outcome_unknown",
                cleanup_required: true,
                detail: format!(
                    "catalog commit outcome is unknown after the skill target was written and verified ({commit_error}); the candidate target was preserved for inspection"
                ),
            });
        }
    }
    after_commit();
    verify_app_data_owner_binding_after_effect(&mutation_lock, "skill.install")?;
    target_capability.finish()?;

    Ok(SkillInstallPreviewRecord {
        wrote: true,
        snapshot_id: None,
        readback: Some(readback),
        confirmation: SkillInstallConfirmation {
            confirmed: true,
            ..preview.confirmation
        },
        ..preview
    })
}

#[cfg(test)]
#[allow(clippy::too_many_arguments)]
fn install_skill_from_tool_global_with_after_confirmation<F>(
    catalog: &Catalog,
    ctx: &AdapterContext,
    instance_id: &str,
    target_agent: AgentId,
    target_scope: Scope,
    project_path: Option<&Path>,
    action_confirmation: Option<&ActionConfirmation>,
    after_confirmation: F,
) -> Result<SkillInstallPreviewRecord, CommandError>
where
    F: FnOnce(),
{
    install_skill_from_tool_global_guarded_with_hooks(
        catalog,
        &ctx.user_home,
        ctx,
        instance_id,
        target_agent,
        target_scope,
        project_path,
        action_confirmation,
        after_confirmation,
        || {},
    )
}

pub fn validate_skill_install_confirmation(
    catalog: &Catalog,
    ctx: &AdapterContext,
    instance_id: &str,
    target_agent: AgentId,
    target_scope: Scope,
    project_path: Option<&Path>,
    action_confirmation: &ActionConfirmation,
) -> Result<SkillInstallPreviewRecord, CommandError> {
    let preview = preview_skill_install_from_tool_global(
        catalog,
        ctx,
        instance_id,
        target_agent,
        target_scope,
        project_path,
    )?;
    let binding = ActionPreviewBinding {
        action: preview.action.clone(),
        preconditions: preview.preconditions.clone(),
        preview_token: preview.preview_token.clone(),
    };
    ensure_action_confirmed(&binding, Some(action_confirmation))?;
    Ok(preview)
}

fn preview_skill_install_from_tool_global(
    catalog: &Catalog,
    ctx: &AdapterContext,
    instance_id: &str,
    target_agent: AgentId,
    target_scope: Scope,
    project_path: Option<&Path>,
) -> Result<SkillInstallPreviewRecord, CommandError> {
    if !matches!(
        target_agent,
        AgentId::ClaudeCode
            | AgentId::Codex
            | AgentId::Opencode
            | AgentId::Pi
            | AgentId::Hermes
            | AgentId::Openclaw
    ) {
        return Err(CommandError::InstallUnsupported(format!(
            "{} skills are not writable by install commands",
            target_agent.as_str()
        )));
    }
    if !matches!(target_scope, Scope::AgentGlobal | Scope::AgentProject) {
        return Err(CommandError::UnsupportedScope(target_scope));
    }

    let meta = catalog
        .get_skill_instance_meta(instance_id)?
        .ok_or_else(|| CommandError::InstanceNotFound(instance_id.to_string()))?;
    if meta.scope != Scope::ToolGlobal {
        return Err(CommandError::UnsupportedScope(meta.scope));
    }
    reject_symlink(&meta.path, "tool-global source file")?;
    let source = meta.path.canonicalize().map_err(|err| {
        CommandError::UnsafeConfigPath(format!(
            "tool-global source {} cannot be canonicalized: {err}",
            meta.path.display()
        ))
    })?;
    validate_tool_global_source(&source)?;

    let target =
        skill_install_target_path(ctx, target_agent, target_scope, &meta.name, project_path)?;
    validate_skill_install_target(
        ctx,
        target_agent,
        target_scope,
        &target,
        project_path,
        false,
    )?;
    let target_exists = target.exists();
    let risks = install_preview_risks(target_agent, target_scope, target_exists);
    let source_state = read_skill_file_state(&source, true)?;
    let target_state = read_skill_file_state(&target, false)?;
    let catalog_revision = batch_catalog_record_revision(&meta)?;
    let project_root = if target_scope == Scope::AgentProject {
        Some(target_project_root(ctx, project_path)?)
    } else {
        ctx.project_root.clone()
    };
    let project_id = canonical_project_id(project_root.as_deref());
    let source_revision = action_source_revision(
        "skill.install.accepted-snapshot",
        &[
            ("catalog_record", &catalog_revision),
            ("source_instance_id", instance_id),
        ],
    )?;
    let descriptor = action_descriptor(
        ActionKind::InstallSkill,
        ActionIntent::InstallSkill,
        ActionTargetRef {
            kind: ActionTargetKind::Skill,
            id: format!("install-target:{}", hash_string(&target.to_string_lossy())),
            agent: Some(target_agent),
            scope: Some(target_scope),
        },
        project_id,
        vec![ActionImpact::SkillFiles, ActionImpact::AppLocalData],
        "skill.install",
        Some("skill.install"),
        source_revision,
        true,
        ActionNetworkPosture::None,
        canonical_readback_domains([
            ActionReadbackDomain::SkillFiles,
            ActionReadbackDomain::CatalogSkills,
        ]),
        vec![skill_projection_evidence_id(instance_id)],
    )?;
    let action_binding = action_preview_binding(
        descriptor,
        vec![
            ActionPrecondition {
                kind: ActionPreconditionKind::CatalogRecord,
                target_id: instance_id.to_string(),
                expected_revision: catalog_revision,
            },
            ActionPrecondition {
                kind: ActionPreconditionKind::SourceFile,
                target_id: source.to_string_lossy().to_string(),
                expected_revision: source_state.binding_revision,
            },
            ActionPrecondition {
                kind: ActionPreconditionKind::TargetFile,
                target_id: target.to_string_lossy().to_string(),
                expected_revision: target_state.revision,
            },
        ],
    )?;
    Ok(SkillInstallPreviewRecord {
        action: action_binding.action,
        preconditions: action_binding.preconditions,
        preview_token: action_binding.preview_token,
        source_instance_id: instance_id.to_string(),
        source_path: source.to_string_lossy().to_string(),
        target_agent: target_agent.as_str().to_string(),
        target_scope: target_scope.as_str().to_string(),
        target_path: target.to_string_lossy().to_string(),
        files: vec![SkillInstallFilePreview {
            source: source.to_string_lossy().to_string(),
            target: target.to_string_lossy().to_string(),
            kind: "skill".to_string(),
            will_write: true,
            target_exists,
        }],
        risks,
        confirmation: SkillInstallConfirmation {
            required: true,
            confirmed: false,
            fields: vec![
                "source_instance_id",
                "source_path",
                "target_agent",
                "target_scope",
                "target_path",
                "files",
                "risks",
            ],
            message: "Confirm install to copy this tool-global skill into the selected agent root."
                .to_string(),
        },
        wrote: false,
        snapshot_id: None,
        readback: None,
    })
}

fn refresh_catalog_rule_outputs(
    catalog: &Catalog,
    ctx: &AdapterContext,
    previous_fingerprints: std::collections::HashMap<String, String>,
) -> Result<(), CommandError> {
    let instances = visible_catalog_instances(
        catalog.list_skill_instances_for_project_context(ctx.project_root.as_deref())?,
    );
    let mut rule_report = evaluate_mvp_rules(
        &instances,
        &RuleContext {
            previous_fingerprints,
            runtime_conflict_namespaces: Some(runtime_conflict_namespaces(&instances, ctx)),
        },
    );
    append_v28_local_rule_findings(&instances, &mut rule_report);
    refresh_rule_outputs(catalog, rule_report)
}

const BODY_TOO_LONG_CHAR_THRESHOLD: usize = 32_000;

fn visible_catalog_instances(instances: Vec<SkillInstance>) -> Vec<SkillInstance> {
    instances
        .into_iter()
        .filter(|instance| !is_pi_plain_markdown_catalog_noise(instance.agent, &instance.path))
        .collect()
}

fn is_pi_plain_markdown_catalog_noise(agent: AgentId, path: &Path) -> bool {
    agent == AgentId::Pi
        && path.extension().and_then(|extension| extension.to_str()) == Some("md")
        && path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md")
}

fn append_v28_local_rule_findings(instances: &[SkillInstance], report: &mut RuleReport) {
    for inst in instances {
        if frontmatter_tools_present_but_empty(&inst.frontmatter_raw) {
            push_instance_finding(
                report,
                inst,
                "frontmatter.tools-not-empty",
                Severity::Warn,
                "Frontmatter tools field is present but empty.",
                "Remove the tools field or list at least one required tool.",
            );
        }
        if !inst.name.contains(':') && !is_canonical_skill_name(&inst.name) {
            let suggestion = canonical_skill_name_suggestion(&inst.name);
            report.findings.push(Finding {
                instance_id: Some(inst.id.clone()),
                definition_id: Some(inst.definition_id.clone()),
                rule_id: "name.canonical-case".to_string(),
                severity: Severity::Warn,
                message: format!(
                    "Skill name '{}' is not a canonical lowercase slug.",
                    inst.name
                ),
                suggestion: Some(format!(
                    "Rename the skill to a lowercase slug such as '{}'.",
                    suggestion
                )),
            });
        }
        let body_len = inst.body.chars().count();
        if body_len > BODY_TOO_LONG_CHAR_THRESHOLD {
            report.findings.push(Finding {
                instance_id: Some(inst.id.clone()),
                definition_id: Some(inst.definition_id.clone()),
                rule_id: "body.too-long".to_string(),
                severity: Severity::Warn,
                message: format!(
                    "Skill body is {body_len} characters, exceeding the local limit of {BODY_TOO_LONG_CHAR_THRESHOLD}."
                ),
                suggestion: Some(
                    "Shorten the body or move detailed reference material into separate files."
                        .to_string(),
                ),
            });
        }
        if skill_uses_network_capability(inst) && !permissions_network_declared(inst) {
            push_instance_finding(
                report,
                inst,
                "permissions.network-declared",
                Severity::Warn,
                "Skill uses network capability without declaring permissions.network.",
                "Declare permissions.network as none, read-only, or full to match the skill's network use.",
            );
        }
        if skill_needs_exec_capability(inst) && !requires_human_explicit_true(inst) {
            push_instance_finding(
                report,
                inst,
                "permissions.exec-needs-human",
                Severity::Warn,
                "Skill requires command or script execution without explicitly requiring human approval.",
                "Set requires_human: true for skills that use exec, scripts, or shell commands.",
            );
        }
        if skill_embeds_shebang_script(inst) {
            push_instance_finding(
                report,
                inst,
                "script.no-shebang",
                Severity::Warn,
                "Skill embeds a script that starts with a shebang.",
                "Move executable script bodies into reviewed files or remove the shebang from inline script fields.",
            );
        }
        if skill_declares_unknown_dependency(inst) {
            push_instance_finding(
                report,
                inst,
                "dependency.unknown",
                Severity::Warn,
                "Skill declares dependencies that are not known safe local dependencies.",
                "Replace unknown packages with reviewed local dependencies or document them in an approved dependency allowlist.",
            );
        }
    }
}

fn push_instance_finding(
    report: &mut RuleReport,
    inst: &SkillInstance,
    rule_id: &str,
    severity: Severity,
    message: &str,
    suggestion: &str,
) {
    report.findings.push(Finding {
        instance_id: Some(inst.id.clone()),
        definition_id: Some(inst.definition_id.clone()),
        rule_id: rule_id.to_string(),
        severity,
        message: message.to_string(),
        suggestion: Some(suggestion.to_string()),
    });
}

fn permissions_network_declared(inst: &SkillInstance) -> bool {
    inst.permissions.network_declared
        || frontmatter_has_any_path(inst, &[&["permissions", "network"], &["network"]])
}

fn requires_human_explicit_true(inst: &SkillInstance) -> bool {
    if inst.permissions.requires_human && inst.permissions.requires_human_declared {
        return true;
    }
    frontmatter_bool(
        inst,
        &[&["permissions", "requires_human"], &["requires_human"]],
    )
    .unwrap_or(false)
}

fn skill_uses_network_capability(inst: &SkillInstance) -> bool {
    if inst.permissions.network_declared
        && !matches!(
            inst.permissions.network,
            NetworkAccess::None | NetworkAccess::Unknown(_)
        )
    {
        return true;
    }
    if inst
        .permissions
        .tools
        .iter()
        .any(|tool| contains_any_ci(tool, &["webfetch", "websearch", "http", "curl", "wget"]))
    {
        return true;
    }
    contains_network_command(&inst.frontmatter_raw) || contains_network_command(&inst.body)
}

fn skill_needs_exec_capability(inst: &SkillInstance) -> bool {
    inst.permissions.exec
        || !inst.scripts.is_empty()
        || inst.permissions.tools.iter().any(|tool| {
            contains_any_ci(
                tool,
                &["bash", "shell", "exec", "python", "node", "npm", "cargo"],
            )
        })
        || frontmatter_has_any_path(
            inst,
            &[
                &["permissions", "exec"],
                &["exec"],
                &["scripts"],
                &["script"],
                &["commands"],
                &["command"],
            ],
        )
        || contains_command_marker(&inst.frontmatter_raw)
        || contains_command_marker(&inst.body)
}

fn skill_embeds_shebang_script(inst: &SkillInstance) -> bool {
    text_has_shebang_line(&inst.body) || frontmatter_script_value_has_shebang(inst)
}

fn skill_declares_unknown_dependency(inst: &SkillInstance) -> bool {
    let Some(frontmatter) = frontmatter_value(inst) else {
        return false;
    };
    dependency_declarations(&frontmatter)
        .into_iter()
        .any(|dependency| !is_known_safe_local_dependency(&dependency))
}

fn contains_network_command(text: &str) -> bool {
    contains_any_ci(
        text,
        &[
            "curl ",
            "wget ",
            "fetch(",
            "requests.get",
            "urllib.request",
            "reqwest::",
            "net/http",
            "http.get",
            "invoke-webrequest",
            "pip install ",
            "npm install ",
            "cargo install ",
            "go get ",
            "docker pull ",
        ],
    )
}

fn contains_command_marker(text: &str) -> bool {
    contains_any_ci(
        text,
        &[
            "```bash", "```sh", "```shell", "bash ", "sh ", "python ", "python3 ", "node ", "npm ",
            "cargo ",
        ],
    )
}

fn contains_any_ci(text: &str, needles: &[&str]) -> bool {
    let lower = text.to_ascii_lowercase();
    needles.iter().any(|needle| lower.contains(needle))
}

fn text_has_shebang_line(text: &str) -> bool {
    text.lines().any(|line| line.trim_start().starts_with("#!"))
}

fn frontmatter_value(inst: &SkillInstance) -> Option<serde_norway::Value> {
    if inst.frontmatter_raw.trim().is_empty() {
        return None;
    }
    serde_norway::from_str(&inst.frontmatter_raw).ok()
}

fn frontmatter_has_any_path(inst: &SkillInstance, paths: &[&[&str]]) -> bool {
    let Some(value) = frontmatter_value(inst) else {
        return false;
    };
    paths.iter().any(|path| yaml_path(&value, path).is_some())
}

fn frontmatter_bool(inst: &SkillInstance, paths: &[&[&str]]) -> Option<bool> {
    let value = frontmatter_value(inst)?;
    paths
        .iter()
        .find_map(|path| yaml_path(&value, path).and_then(serde_norway::Value::as_bool))
}

fn yaml_path<'a>(value: &'a serde_norway::Value, path: &[&str]) -> Option<&'a serde_norway::Value> {
    let mut current = value;
    for part in path {
        let mapping = current.as_mapping()?;
        current = mapping.get(serde_norway::Value::String((*part).to_string()))?;
    }
    Some(current)
}

fn frontmatter_script_value_has_shebang(inst: &SkillInstance) -> bool {
    let Some(value) = frontmatter_value(inst) else {
        return false;
    };
    yaml_script_value_has_shebang(&value, false)
}

fn yaml_script_value_has_shebang(value: &serde_norway::Value, in_script_field: bool) -> bool {
    match value {
        serde_norway::Value::String(raw) => in_script_field && raw.trim_start().starts_with("#!"),
        serde_norway::Value::Sequence(items) => items
            .iter()
            .any(|item| yaml_script_value_has_shebang(item, in_script_field)),
        serde_norway::Value::Mapping(mapping) => mapping.iter().any(|(key, value)| {
            let script_field =
                in_script_field || key.as_str().is_some_and(matches_script_field_name);
            yaml_script_value_has_shebang(value, script_field)
        }),
        _ => false,
    }
}

fn frontmatter_tools_present_but_empty(frontmatter_raw: &str) -> bool {
    let Ok(value) = serde_norway::from_str::<serde_norway::Value>(frontmatter_raw) else {
        return false;
    };
    let Some(tools) = value.get(serde_norway::Value::String("tools".to_string())) else {
        return false;
    };
    match tools {
        serde_norway::Value::Sequence(items) => items.iter().all(yaml_value_is_blank),
        serde_norway::Value::String(value) => value.trim().is_empty(),
        serde_norway::Value::Null => true,
        _ => false,
    }
}

fn matches_script_field_name(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['-', '_'], "");
    matches!(
        normalized.as_str(),
        "script" | "scripts" | "command" | "commands" | "inline" | "inlinescript"
    )
}

fn dependency_declarations(value: &serde_norway::Value) -> Vec<String> {
    let mut declarations = Vec::new();
    collect_dependency_declarations(value, false, &mut declarations);
    declarations
}

fn collect_dependency_declarations(
    value: &serde_norway::Value,
    in_dependency_field: bool,
    declarations: &mut Vec<String>,
) {
    match value {
        serde_norway::Value::String(raw) if in_dependency_field => {
            declarations.extend(split_dependency_string(raw));
        }
        serde_norway::Value::Number(number) if in_dependency_field => {
            declarations.push(number.to_string());
        }
        serde_norway::Value::Sequence(items) => {
            for item in items {
                collect_dependency_declarations(item, in_dependency_field, declarations);
            }
        }
        serde_norway::Value::Mapping(mapping) => {
            for (key, value) in mapping {
                let dependency_field = key.as_str().is_some_and(matches_dependency_field_name);
                if in_dependency_field {
                    if let Some(key) = key.as_str() {
                        declarations.extend(split_dependency_string(key));
                    }
                }
                collect_dependency_declarations(
                    value,
                    in_dependency_field || dependency_field,
                    declarations,
                );
            }
        }
        _ => {}
    }
}

fn split_dependency_string(raw: &str) -> Vec<String> {
    raw.split([',', '\n', '\r'])
        .map(str::trim)
        .filter(|part| !part.is_empty())
        .map(ToString::to_string)
        .collect()
}

fn matches_dependency_field_name(key: &str) -> bool {
    let normalized = key.to_ascii_lowercase().replace(['-', '_'], "");
    matches!(
        normalized.as_str(),
        "dependency" | "dependencies" | "package" | "packages" | "requirements"
    )
}

fn is_known_safe_local_dependency(raw: &str) -> bool {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return true;
    }
    let lower = trimmed.to_ascii_lowercase();
    if lower.starts_with("./")
        || lower.starts_with("../")
        || lower.starts_with('/')
        || lower.starts_with("file:")
    {
        return true;
    }
    let name = lower
        .split(|ch: char| {
            ch.is_whitespace()
                || matches!(
                    ch,
                    '=' | '<' | '>' | '~' | '^' | ':' | '@' | '[' | '(' | ')' | ','
                )
        })
        .next()
        .unwrap_or("");
    matches!(
        name,
        "bash"
            | "sh"
            | "zsh"
            | "python"
            | "python3"
            | "node"
            | "npm"
            | "npx"
            | "bun"
            | "deno"
            | "git"
            | "cargo"
            | "rustc"
            | "go"
            | "make"
            | "jq"
            | "sqlite3"
            | "rg"
            | "ripgrep"
    )
}

fn yaml_value_is_blank(value: &serde_norway::Value) -> bool {
    match value {
        serde_norway::Value::String(value) => value.trim().is_empty(),
        serde_norway::Value::Null => true,
        _ => false,
    }
}

fn is_canonical_skill_name(name: &str) -> bool {
    let mut previous_was_separator = false;
    let mut saw_char = false;
    for (idx, ch) in name.chars().enumerate() {
        let is_separator = matches!(ch, '-' | '_');
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            previous_was_separator = false;
            saw_char = true;
            continue;
        }
        if is_separator && idx > 0 && !previous_was_separator {
            previous_was_separator = true;
            saw_char = true;
            continue;
        }
        return false;
    }
    saw_char && !previous_was_separator
}

fn canonical_skill_name_suggestion(name: &str) -> String {
    let mut output = String::new();
    let mut previous_was_separator = false;
    for ch in name.chars().flat_map(char::to_lowercase) {
        if ch.is_ascii_lowercase() || ch.is_ascii_digit() {
            output.push(ch);
            previous_was_separator = false;
        } else if matches!(ch, '-' | '_') && !output.is_empty() && !previous_was_separator {
            output.push(ch);
            previous_was_separator = true;
        } else if !output.is_empty() && !previous_was_separator {
            output.push('-');
            previous_was_separator = true;
        }
    }
    while output.ends_with(['-', '_']) {
        output.pop();
    }
    if output.is_empty() {
        "skill-name".to_string()
    } else {
        output
    }
}

fn refresh_rule_outputs(catalog: &Catalog, report: RuleReport) -> Result<(), CommandError> {
    let now_ms = current_time_ms();
    let report_findings = dedupe_rule_findings(report.findings);
    let findings: Vec<RuleFindingDraft> = report_findings
        .into_iter()
        .enumerate()
        .map(|(idx, finding)| RuleFindingDraft {
            id: format!(
                "{}:{}",
                finding.rule_id,
                finding
                    .instance_id
                    .as_deref()
                    .or(finding.definition_id.as_deref())
                    .unwrap_or("global")
            )
            .replace('/', "_")
                + &format!(":{idx}"),
            instance_id: finding.instance_id,
            definition_id: finding.definition_id,
            rule_id: finding.rule_id,
            severity: finding.severity.as_str().to_string(),
            message: finding.message,
            suggestion: finding.suggestion,
            created_at: now_ms,
        })
        .collect();
    let mut seen_definition_names = std::collections::HashSet::new();
    let definitions: Vec<SkillDefinitionDraft> = report
        .definitions
        .into_iter()
        .filter(|definition| seen_definition_names.insert(definition.canonical_name.clone()))
        .map(|definition| SkillDefinitionDraft {
            id: definition.id,
            canonical_name: definition.canonical_name,
            description: definition.description,
            active_instance: definition.active_instance,
            has_multiple_instances: definition.has_multiple_instances,
            has_conflict: definition.has_conflict,
        })
        .collect();
    let conflicts: Vec<ConflictGroupDraft> = report
        .conflicts
        .into_iter()
        .map(|conflict| ConflictGroupDraft {
            id: conflict.id,
            definition_id: conflict.definition_id,
            reason: conflict.reason,
            winner_id: conflict.winner_id,
            instance_ids: conflict.instances,
        })
        .collect();

    catalog.refresh_definitions_and_conflicts(&definitions, &conflicts)?;
    catalog.refresh_rule_findings(&findings)?;
    Ok(())
}

/// Toggle a skill's `skillOverrides` entry in the agent's settings file, with
/// snapshot + atomic write + verification + rollback semantics. Returns the
/// updated `SkillRecord` so the UI can refresh the row in place.
pub fn toggle_skill(
    catalog: &Catalog,
    ctx: &AdapterContext,
    instance_id: &str,
    on: bool,
) -> Result<SkillRecord, CommandError> {
    let meta = catalog
        .get_skill_instance_meta(instance_id)?
        .ok_or_else(|| CommandError::InstanceNotFound(instance_id.to_string()))?;

    let config_target = config_target_for_instance(ctx, &meta)?;

    validate_config_write_target(
        ctx,
        config_target.agent,
        config_target.scope,
        &config_target.path,
    )?;
    let lock_file = lock_config(
        ctx,
        config_target.agent,
        config_target.scope,
        &config_target.path,
    )?;

    // 1. Read current settings (empty string if file does not exist yet).
    let original_text = if config_target.path.exists() {
        fs::read_to_string(&config_target.path)?
    } else {
        String::new()
    };
    let original_text = normalize_initial_config_text(&config_target, original_text);

    // 2. Take a pre-toggle snapshot.
    let snapshot_id = generate_snapshot_id();
    let now_ms = current_time_ms();
    let snapshot_content = redact_snapshot_content(&original_text);
    let snapshot_project_root = snapshot_project_root_for_scope(ctx, config_target.scope)?;
    catalog.create_config_snapshot(ConfigSnapshotDraft {
        id: &snapshot_id,
        agent: meta.agent.as_str(),
        scope: config_target.scope.as_str(),
        project_root: snapshot_project_root.as_deref(),
        target: &config_target.path.to_string_lossy(),
        content: &snapshot_content,
        reason: "pre-toggle",
        created_at_ms: now_ms,
    })?;

    // 3. Apply the patch in memory.
    let mut doc = AgentConfigDocument {
        path: config_target.path.clone(),
        format: config_target.format,
        text: original_text.clone(),
    };
    let instance_for_patch = minimal_skill_instance(&meta);
    patch_enabled_for_agent(meta.agent, &mut doc, &instance_for_patch, on)?;

    // 4. Atomic write.
    write_config_atomic(
        ctx,
        config_target.agent,
        config_target.scope,
        &config_target.path,
        &doc.text,
    )?;

    // 5. Verify by reading back.
    let written = fs::read_to_string(&config_target.path)?;
    if written != doc.text {
        // Roll back to the snapshot content (best-effort; surface the failure).
        let _ = write_config_atomic(
            ctx,
            config_target.agent,
            config_target.scope,
            &config_target.path,
            &original_text,
        );
        lock_file.unlock()?;
        return Err(CommandError::VerificationFailed);
    }

    // Release the config lock before mutating the per-process catalog cache.
    lock_file.unlock()?;

    // 6. Update the catalog to reflect the new effective state.
    let new_state = if on { "loaded" } else { "disabled" };
    catalog.set_skill_toggle(instance_id, on, new_state)?;

    let target = config_target.path.to_string_lossy().to_string();
    let event_payload = serde_json::json!({
        "enabled": on,
        "agent": meta.agent.as_str(),
        "scope": meta.scope.as_str(),
        "target": target,
        "skill_name": meta.name.clone(),
        "config_scope": config_target.scope.as_str(),
        "previous_enabled": meta.enabled,
    });
    let event_payload = serde_json::to_string(&event_payload)?;
    catalog.create_skill_event(SkillEventDraft {
        instance_id,
        kind: "toggle",
        payload: &event_payload,
        occurred_at_ms: now_ms,
    })?;

    // 7. Return the updated record for the UI.
    catalog
        .get_skill_record(instance_id)?
        .ok_or_else(|| CommandError::InstanceNotFound(instance_id.to_string()))
}

pub fn preview_skill_toggles(
    catalog: &Catalog,
    ctx: &AdapterContext,
    instance_ids: &[String],
    target_enabled: bool,
) -> Result<BatchTogglePreviewRecord, CommandError> {
    let requested_count = instance_ids.len();
    let mut seen = BTreeSet::new();
    let mut affected_items = Vec::new();
    let mut skipped_items = Vec::new();

    for instance_id in instance_ids {
        if !seen.insert(instance_id.clone()) {
            skipped_items.push(BatchToggleSkippedItem {
                instance_id: instance_id.clone(),
                name: None,
                agent: None,
                scope: None,
                reason: "Duplicate selection entry; each skill is toggled at most once per batch."
                    .to_string(),
                capability_label: None,
            });
            continue;
        }

        let Some(meta) = catalog.get_skill_instance_meta(instance_id)? else {
            skipped_items.push(BatchToggleSkippedItem {
                instance_id: instance_id.clone(),
                name: None,
                agent: None,
                scope: None,
                reason: "Skill instance was not found in the current catalog.".to_string(),
                capability_label: None,
            });
            continue;
        };

        let capability_label = batch_capability_label(meta.agent).to_string();
        if meta.enabled == target_enabled {
            skipped_items.push(BatchToggleSkippedItem {
                instance_id: instance_id.clone(),
                name: Some(meta.name.clone()),
                agent: Some(meta.agent.as_str().to_string()),
                scope: Some(meta.scope.as_str().to_string()),
                reason: "Skill already matches the requested enabled state.".to_string(),
                capability_label: Some(capability_label),
            });
            continue;
        }

        let config_target = match config_target_for_instance(ctx, &meta) {
            Ok(config_target) => config_target,
            Err(error) => {
                skipped_items.push(BatchToggleSkippedItem {
                    instance_id: instance_id.clone(),
                    name: Some(meta.name.clone()),
                    agent: Some(meta.agent.as_str().to_string()),
                    scope: Some(meta.scope.as_str().to_string()),
                    reason: batch_skip_reason(meta.agent, &error),
                    capability_label: Some(capability_label),
                });
                continue;
            }
        };

        affected_items.push(BatchToggleAffectedItem {
            instance_id: instance_id.clone(),
            name: meta.name.clone(),
            agent: meta.agent.as_str().to_string(),
            scope: meta.scope.as_str().to_string(),
            current_enabled: meta.enabled,
            target_enabled,
            config_scope: config_target.scope.as_str().to_string(),
            config_target: config_target.path.to_string_lossy().to_string(),
            config_revision: read_config_state(&config_target.path)?.revision,
            catalog_revision: batch_catalog_record_revision(&meta)?,
            capability_label,
            snapshot_plan: "Create a pre-batch-toggle agent config snapshot before writing."
                .to_string(),
            rollback_plan:
                "Rollback remains available through snapshot.previewRollback and snapshot.rollback."
                    .to_string(),
        });
    }

    let writable_count = affected_items.len();
    let skipped_count = skipped_items.len();
    let writes_allowed = writable_count > 0;
    let capability_labels = batch_capability_labels(&affected_items, &skipped_items);
    let snapshot_rollback_notes = batch_snapshot_rollback_notes(&affected_items);
    let action_binding = batch_toggle_action_binding(
        ctx,
        target_enabled,
        requested_count,
        &affected_items,
        &skipped_items,
    )?;

    Ok(BatchTogglePreviewRecord {
        action: action_binding.action,
        preconditions: action_binding.preconditions,
        preview_token: action_binding.preview_token,
        target_enabled,
        requested_count,
        writable_count,
        skipped_count,
        writes_allowed,
        affected_items,
        skipped_items,
        capability_labels,
        snapshot_rollback_notes,
    })
}

struct BatchToggleConfigPlan<'lock> {
    target: ConfigTarget,
    capability: ExternalTargetCapability<'lock>,
    metas: Vec<SkillInstanceMeta>,
    original: ConfigState,
    new_text: String,
}

pub fn apply_skill_toggles(
    catalog: &Catalog,
    ctx: &AdapterContext,
    instance_ids: &[String],
    target_enabled: bool,
    confirmation: &ActionConfirmation,
) -> Result<BatchToggleApplyRecord, CommandError> {
    apply_skill_toggles_guarded(
        catalog,
        &ctx.user_home,
        ctx,
        instance_ids,
        target_enabled,
        confirmation,
    )
}

pub fn apply_skill_toggles_guarded(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    instance_ids: &[String],
    target_enabled: bool,
    confirmation: &ActionConfirmation,
) -> Result<BatchToggleApplyRecord, CommandError> {
    apply_skill_toggles_guarded_with_hooks(
        catalog,
        app_data_dir,
        ctx,
        instance_ids,
        target_enabled,
        confirmation,
        || {},
        || {},
    )
}

#[cfg(test)]
fn apply_skill_toggles_with_after_confirmation<F>(
    catalog: &Catalog,
    ctx: &AdapterContext,
    instance_ids: &[String],
    target_enabled: bool,
    confirmation: &ActionConfirmation,
    after_confirmation: F,
) -> Result<BatchToggleApplyRecord, CommandError>
where
    F: FnOnce(),
{
    apply_skill_toggles_guarded_with_hooks(
        catalog,
        &ctx.user_home,
        ctx,
        instance_ids,
        target_enabled,
        confirmation,
        after_confirmation,
        || {},
    )
}

#[allow(clippy::too_many_arguments)]
fn apply_skill_toggles_guarded_with_hooks<F, G>(
    catalog: &Catalog,
    app_data_dir: &Path,
    ctx: &AdapterContext,
    instance_ids: &[String],
    target_enabled: bool,
    confirmation: &ActionConfirmation,
    after_confirmation: F,
    after_commit: G,
) -> Result<BatchToggleApplyRecord, CommandError>
where
    F: FnOnce(),
    G: FnOnce(),
{
    let preview = validate_skill_toggle_confirmation(
        catalog,
        ctx,
        instance_ids,
        target_enabled,
        confirmation,
    )?;
    after_confirmation();
    if !preview.writes_allowed {
        return Err(CommandError::InvalidBatchAction(
            "batch apply has no writable items after read-only and no-op filtering".to_string(),
        ));
    }

    let mutation_lock = mutation_lock::lock_app_mutations(app_data_dir)?;
    mutation_lock.validate_owner_path_binding()?;
    catalog.ensure_mutation_owner(mutation_lock.owner_directory())?;
    let mut groups: BTreeMap<(String, String, String, String), Vec<SkillInstanceMeta>> =
        BTreeMap::new();
    for item in &preview.affected_items {
        let meta = catalog
            .get_skill_instance_meta(&item.instance_id)?
            .ok_or_else(|| CommandError::InstanceNotFound(item.instance_id.clone()))?;
        let config_target = config_target_for_instance(ctx, &meta)?;
        groups
            .entry((
                config_target.agent.as_str().to_string(),
                config_target.scope.as_str().to_string(),
                config_target.path.to_string_lossy().to_string(),
                item.config_revision.clone(),
            ))
            .or_default()
            .push(meta);
    }

    let mut plans = Vec::new();
    for ((_, _, _, expected_revision), metas) in &groups {
        let Some(first_meta) = metas.first() else {
            continue;
        };
        let config_target = config_target_for_instance(ctx, first_meta)?;
        validate_config_write_target_noncreating(
            ctx,
            config_target.agent,
            config_target.scope,
            &config_target.path,
        )?;
        let capability = prepare_config_target_capability(
            &mutation_lock,
            ctx,
            config_target.agent,
            config_target.scope,
            &config_target.path,
        )?;
        let locked_state = read_external_config_state(&capability)?;
        if locked_state.revision != *expected_revision {
            return Err(CommandError::StaleActionReference);
        }
        let normalized_original =
            normalize_initial_config_text(&config_target, locked_state.content.clone());
        let mut doc = AgentConfigDocument {
            path: config_target.path.clone(),
            format: config_target.format,
            text: normalized_original,
        };
        for meta in metas {
            let instance_for_patch = minimal_skill_instance(meta);
            patch_enabled_for_agent(meta.agent, &mut doc, &instance_for_patch, target_enabled)?;
        }
        plans.push(BatchToggleConfigPlan {
            target: config_target,
            capability,
            metas: metas.clone(),
            original: locked_state,
            new_text: doc.text,
        });
    }
    let transaction = catalog.begin_immediate_transaction()?;
    for item in &preview.affected_items {
        let current_meta = catalog
            .get_skill_instance_meta(&item.instance_id)?
            .ok_or(CommandError::StaleActionReference)?;
        if batch_catalog_record_revision(&current_meta)? != item.catalog_revision {
            return Err(CommandError::StaleActionReference);
        }
        let current_target = config_target_for_instance(ctx, &current_meta)?;
        if current_target.agent.as_str() != item.agent
            || current_meta.scope.as_str() != item.scope
            || current_target.scope.as_str() != item.config_scope
            || current_target.path.to_string_lossy() != item.config_target
        {
            return Err(CommandError::StaleActionReference);
        }
    }
    for plan in &plans {
        if read_external_config_state(&plan.capability)?.revision != plan.original.revision {
            return Err(CommandError::StaleActionReference);
        }
    }
    let apply_result = (|| {
        for plan in &mut plans {
            apply_skill_toggle_plan(catalog, ctx, plan, target_enabled)?;
        }
        let mut updated_records = Vec::new();
        for plan in &plans {
            for meta in &plan.metas {
                let record = catalog
                    .get_skill_record(&meta.id)?
                    .ok_or_else(|| CommandError::InstanceNotFound(meta.id.clone()))?;
                updated_records.push(record);
            }
        }
        let mut observations = Vec::new();
        for plan in &plans {
            observations.push(ActionReadbackObservation {
                domain: ActionReadbackDomain::AgentConfig,
                target_id: plan.target.path.to_string_lossy().to_string(),
                revision: read_external_config_state(&plan.capability)?.revision,
            });
        }
        for record in &updated_records {
            let serialized = serde_json::to_string(record)?;
            observations.push(ActionReadbackObservation {
                domain: ActionReadbackDomain::CatalogSkills,
                target_id: record.id.clone(),
                revision: action_source_revision(
                    "catalog.skill.readback",
                    &[("record", &serialized)],
                )?,
            });
        }
        let readback = ActionReadbackRecord::verified(&preview.action, observations)?;
        Ok((updated_records, readback))
    })();
    let (updated_records, readback) = match apply_result {
        Ok(result) => result,
        Err(error) => {
            rollback_catalog_before_compensation(
                transaction,
                "batch.applySkillToggles",
                &error,
                "the written agent config candidates were preserved for inspection",
            )?;
            restore_batch_toggle_plans(&mut plans).map_err(|cleanup_error| {
                CommandError::PartialEffect {
                    operation: "batch.applySkillToggles".to_string(),
                    state: "outcome_unknown",
                    cleanup_required: true,
                    detail: format!(
                        "batch apply failed ({error}); reverse restoration failed ({cleanup_error})"
                    ),
                }
            })?;
            if matches!(error, CommandError::PartialEffect { .. }) {
                return Err(CommandError::PartialEffect {
                    operation: "batch.applySkillToggles".to_string(),
                    state: "outcome_unknown",
                    cleanup_required: false,
                    detail: format!(
                        "batch apply failed after a config namespace effect ({error}); every accepted config target was restored and verified"
                    ),
                });
            }
            return Err(error);
        }
    };
    match transaction.commit_classified() {
        Ok(()) => {}
        Err(CatalogCommitError::NotCommitted(commit_error)) => {
            restore_batch_toggle_plans(&mut plans).map_err(|cleanup_error| {
                CommandError::PartialEffect {
                    operation: "batch.applySkillToggles".to_string(),
                    state: "outcome_unknown",
                    cleanup_required: true,
                    detail: format!(
                        "catalog commit was rejected ({commit_error}); reverse restoration failed ({cleanup_error})"
                    ),
                }
            })?;
            return Err(commit_error.into());
        }
        Err(CatalogCommitError::OutcomeUnknown(commit_error)) => {
            return Err(CommandError::PartialEffect {
                operation: "batch.applySkillToggles".to_string(),
                state: "outcome_unknown",
                cleanup_required: true,
                detail: format!(
                    "catalog commit outcome is unknown after the agent configs were written and verified ({commit_error}); the candidate configs were preserved for inspection"
                ),
            });
        }
    }
    after_commit();
    verify_app_data_owner_binding_after_effect(&mutation_lock, "batch.applySkillToggles")?;
    for plan in &mut plans {
        plan.capability.finish()?;
    }
    Ok(BatchToggleApplyRecord {
        action: preview.action,
        preview_token: preview.preview_token,
        target_enabled: preview.target_enabled,
        requested_count: preview.requested_count,
        writable_count: preview.writable_count,
        skipped_count: preview.skipped_count,
        applied_count: updated_records.len(),
        writes_allowed: preview.writes_allowed,
        affected_items: preview.affected_items,
        skipped_items: preview.skipped_items,
        capability_labels: preview.capability_labels,
        snapshot_rollback_notes: preview.snapshot_rollback_notes,
        updated_records,
        readback,
    })
}

pub fn validate_skill_toggle_confirmation(
    catalog: &Catalog,
    ctx: &AdapterContext,
    instance_ids: &[String],
    target_enabled: bool,
    confirmation: &ActionConfirmation,
) -> Result<BatchTogglePreviewRecord, CommandError> {
    let preview = preview_skill_toggles(catalog, ctx, instance_ids, target_enabled)?;
    let binding = ActionPreviewBinding {
        action: preview.action.clone(),
        preconditions: preview.preconditions.clone(),
        preview_token: preview.preview_token.clone(),
    };
    ensure_action_confirmed(&binding, Some(confirmation))?;
    Ok(preview)
}

fn apply_skill_toggle_plan(
    catalog: &Catalog,
    ctx: &AdapterContext,
    plan: &mut BatchToggleConfigPlan<'_>,
    target_enabled: bool,
) -> Result<(), CommandError> {
    let snapshot_original =
        normalize_initial_config_text(&plan.target, plan.original.content.clone());
    let snapshot_id = generate_snapshot_id();
    let now_ms = current_time_ms();
    let snapshot_content = redact_snapshot_content(&snapshot_original);
    let snapshot_project_root = snapshot_project_root_for_scope(ctx, plan.target.scope)?;
    catalog.create_config_snapshot(ConfigSnapshotDraft {
        id: &snapshot_id,
        agent: plan.target.agent.as_str(),
        scope: plan.target.scope.as_str(),
        project_root: snapshot_project_root.as_deref(),
        target: &plan.target.path.to_string_lossy(),
        content: &snapshot_content,
        reason: "pre-batch-toggle",
        created_at_ms: now_ms,
    })?;
    plan.capability.atomic_replace(plan.new_text.as_bytes())?;
    let written = read_external_config_state(&plan.capability)?;
    if !written.exists || written.content != plan.new_text {
        return Err(CommandError::VerificationFailed);
    }

    let new_state = if target_enabled { "loaded" } else { "disabled" };
    let target = plan.target.path.to_string_lossy().to_string();
    for meta in &plan.metas {
        catalog.set_skill_toggle(&meta.id, target_enabled, new_state)?;
        let event_payload = serde_json::json!({
            "enabled": target_enabled,
            "agent": meta.agent.as_str(),
            "scope": meta.scope.as_str(),
            "target": target,
            "skill_name": meta.name.clone(),
            "config_scope": plan.target.scope.as_str(),
            "previous_enabled": meta.enabled,
            "batch": true,
            "snapshot_id": snapshot_id,
        });
        let event_payload = serde_json::to_string(&event_payload)?;
        catalog.create_skill_event(SkillEventDraft {
            instance_id: &meta.id,
            kind: "toggle",
            payload: &event_payload,
            occurred_at_ms: now_ms,
        })?;
    }

    Ok(())
}

fn restore_batch_toggle_plans(plans: &mut [BatchToggleConfigPlan<'_>]) -> Result<(), CommandError> {
    let mut first_error = None;
    for plan in plans.iter_mut().rev() {
        if let Err(error) = restore_batch_config_state(
            &mut plan.capability,
            &plan.original,
            &config_revision(true, &plan.new_text),
        ) {
            first_error.get_or_insert(error);
        }
    }
    if let Some(error) = first_error {
        return Err(error);
    }
    Ok(())
}

fn restore_batch_config_state(
    capability: &mut ExternalTargetCapability<'_>,
    original: &ConfigState,
    expected_applied_revision: &str,
) -> Result<(), CommandError> {
    capability.run_before_compensation_hook();
    let current = read_external_config_state_anchored(capability)?;
    if current.revision == original.revision {
        if !original.exists {
            capability.restore_missing_anchored()?;
        }
        return Ok(());
    }
    if current.revision != expected_applied_revision {
        return Err(CommandError::InvalidBatchAction(
            "config target changed to an unowned third state during compensation".to_string(),
        ));
    }
    if original.exists {
        capability.restore_present_anchored(original.content.as_bytes())?;
    } else {
        capability.restore_missing_anchored()?;
    }
    if read_external_config_state_anchored(capability)?.revision != original.revision {
        return Err(CommandError::VerificationFailed);
    }
    Ok(())
}

fn batch_toggle_action_binding(
    ctx: &AdapterContext,
    target_enabled: bool,
    requested_count: usize,
    affected_items: &[BatchToggleAffectedItem],
    skipped_items: &[BatchToggleSkippedItem],
) -> Result<ActionPreviewBinding, CommandError> {
    let mut affected = affected_items.to_vec();
    affected.sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
    let mut skipped = skipped_items.to_vec();
    skipped.sort_by(|left, right| {
        left.instance_id
            .cmp(&right.instance_id)
            .then_with(|| left.reason.cmp(&right.reason))
    });
    let accepted_catalog_state = affected
        .iter()
        .map(|item| {
            format!(
                "{}:{}:{}:{}:{}",
                item.instance_id,
                item.agent,
                item.scope,
                item.current_enabled,
                item.catalog_revision
            )
        })
        .collect::<Vec<_>>()
        .join("|");
    let skipped_json = serde_json::to_string(&skipped)?;
    let target_enabled_value = target_enabled.to_string();
    let requested_count_value = requested_count.to_string();
    let project_id = canonical_project_id(ctx.project_root.as_deref());
    let project_id_value = project_id.as_deref().unwrap_or("<none>");
    let source_revision = action_source_revision(
        "batch.toggle",
        &[
            ("target_enabled", &target_enabled_value),
            ("requested_count", &requested_count_value),
            ("project_id", project_id_value),
            ("accepted_catalog_state", &accepted_catalog_state),
            ("skipped", &skipped_json),
        ],
    )?;
    let mut selected_ids = affected
        .iter()
        .map(|item| item.instance_id.clone())
        .chain(skipped.iter().map(|item| item.instance_id.clone()))
        .collect::<Vec<_>>();
    selected_ids.sort();
    let target_id = format!("batch:{}", hash_string(&selected_ids.join("|")));
    let agent = common_batch_agent(&affected);
    let scope = common_batch_scope(&affected);
    let evidence_refs = if selected_ids.is_empty() {
        vec!["selection:empty".to_string()]
    } else {
        selected_ids
            .iter()
            .map(|id| skill_projection_evidence_id(id))
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect()
    };
    let descriptor = action_descriptor(
        ActionKind::ToggleSkill,
        if target_enabled {
            ActionIntent::EnableSkill
        } else {
            ActionIntent::DisableSkill
        },
        ActionTargetRef {
            kind: ActionTargetKind::Skill,
            id: target_id,
            agent,
            scope,
        },
        project_id,
        vec![ActionImpact::AppLocalData, ActionImpact::AgentConfig],
        "batch.previewSkillToggles",
        Some("batch.applySkillToggles"),
        source_revision,
        true,
        ActionNetworkPosture::None,
        canonical_readback_domains([
            ActionReadbackDomain::CatalogSkills,
            ActionReadbackDomain::AgentConfig,
        ]),
        evidence_refs,
    )?;
    let mut preconditions = Vec::new();
    let mut config_revisions = BTreeMap::new();
    for item in &affected {
        config_revisions
            .entry(item.config_target.clone())
            .or_insert_with(|| item.config_revision.clone());
        preconditions.push(ActionPrecondition {
            kind: ActionPreconditionKind::CatalogRecord,
            target_id: item.instance_id.clone(),
            expected_revision: item.catalog_revision.clone(),
        });
    }
    preconditions.extend(
        config_revisions
            .into_iter()
            .map(|(target_id, expected_revision)| ActionPrecondition {
                kind: ActionPreconditionKind::AgentConfig,
                target_id,
                expected_revision,
            }),
    );
    action_preview_binding(descriptor, preconditions)
}

fn batch_catalog_record_revision(meta: &SkillInstanceMeta) -> Result<String, CommandError> {
    let project_root = meta
        .project_root
        .as_deref()
        .map(|path| path.to_string_lossy().to_string())
        .unwrap_or_else(|| "<none>".to_string());
    action_source_revision(
        "catalog.skill.precondition",
        &[
            ("id", &meta.id),
            ("name", &meta.name),
            ("agent", meta.agent.as_str()),
            ("scope", meta.scope.as_str()),
            ("enabled", if meta.enabled { "true" } else { "false" }),
            ("path", &meta.path.to_string_lossy()),
            ("project_root", &project_root),
        ],
    )
}

fn common_batch_agent(affected: &[BatchToggleAffectedItem]) -> Option<AgentId> {
    let first = affected.first()?.agent.as_str();
    if !affected.iter().all(|item| item.agent == first) {
        return None;
    }
    match first {
        "claude-code" => Some(AgentId::ClaudeCode),
        "codex" => Some(AgentId::Codex),
        "opencode" => Some(AgentId::Opencode),
        "pi" => Some(AgentId::Pi),
        "hermes" => Some(AgentId::Hermes),
        "openclaw" => Some(AgentId::Openclaw),
        "tool-global" => Some(AgentId::ToolGlobal),
        _ => None,
    }
}

fn common_batch_scope(affected: &[BatchToggleAffectedItem]) -> Option<Scope> {
    let first = affected.first()?.scope.as_str();
    if !affected.iter().all(|item| item.scope == first) {
        return None;
    }
    match first {
        "agent-global" => Some(Scope::AgentGlobal),
        "agent-project" => Some(Scope::AgentProject),
        "tool-global" => Some(Scope::ToolGlobal),
        _ => None,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct ConfigTarget {
    pub(crate) agent: AgentId,
    pub(crate) scope: Scope,
    pub(crate) path: PathBuf,
    pub(crate) format: ConfigFormat,
}

fn config_target_for_instance(
    ctx: &AdapterContext,
    meta: &SkillInstanceMeta,
) -> Result<ConfigTarget, CommandError> {
    if meta.scope == Scope::AgentProject
        && !project_record_matches_context_for_agent(
            meta.agent,
            meta.project_root.as_deref(),
            ctx.project_root.as_deref(),
        )
    {
        return Err(CommandError::UnsafeConfigPath(
            "project skill does not belong to the current project context".to_string(),
        ));
    }

    match meta.agent {
        AgentId::ClaudeCode => config_target_for_claude(ctx, meta.scope),
        AgentId::Codex => match meta.scope {
            Scope::AgentGlobal | Scope::AgentProject => {
                validate_codex_toggle_instance(ctx, meta)?;
                Ok(ConfigTarget {
                    agent: AgentId::Codex,
                    scope: Scope::AgentGlobal,
                    path: codex_user_config_path(ctx),
                    format: ConfigFormat::Toml,
                })
            }
            Scope::ToolGlobal => Err(CommandError::UnsupportedScope(meta.scope)),
            _ => Err(CommandError::UnsupportedScope(meta.scope)),
        },
        AgentId::Opencode => match meta.scope {
            Scope::AgentGlobal | Scope::AgentProject => Ok(ConfigTarget {
                agent: AgentId::Opencode,
                scope: meta.scope,
                path: opencode_config_path(ctx, meta.scope)?,
                format: ConfigFormat::Json,
            }),
            Scope::ToolGlobal => Err(CommandError::UnsupportedScope(meta.scope)),
            _ => Err(CommandError::UnsupportedScope(meta.scope)),
        },
        AgentId::Pi => match meta.scope {
            Scope::AgentGlobal | Scope::AgentProject => Ok(ConfigTarget {
                agent: AgentId::Pi,
                scope: meta.scope,
                path: pi_config_path_for_instance(ctx, meta)?,
                format: ConfigFormat::Json,
            }),
            Scope::ToolGlobal => Err(CommandError::UnsupportedScope(meta.scope)),
            _ => Err(CommandError::UnsupportedScope(meta.scope)),
        },
        AgentId::Hermes => match meta.scope {
            Scope::AgentGlobal => Ok(ConfigTarget {
                agent: AgentId::Hermes,
                scope: Scope::AgentGlobal,
                path: ctx.user_home.join(".hermes/config.yaml"),
                format: ConfigFormat::Yaml,
            }),
            Scope::ToolGlobal | Scope::AgentProject => {
                Err(CommandError::UnsupportedScope(meta.scope))
            }
            _ => Err(CommandError::UnsupportedScope(meta.scope)),
        },
        AgentId::Openclaw => match meta.scope {
            Scope::AgentGlobal | Scope::AgentProject => Ok(ConfigTarget {
                agent: AgentId::Openclaw,
                scope: Scope::AgentGlobal,
                path: ctx.user_home.join(".openclaw/openclaw.json"),
                format: ConfigFormat::Json,
            }),
            Scope::ToolGlobal => Err(CommandError::UnsupportedScope(meta.scope)),
            _ => Err(CommandError::UnsupportedScope(meta.scope)),
        },
        agent => Err(CommandError::UnsafeConfigPath(format!(
            "{} skills are not writable by config.toggleSkill",
            agent.as_str()
        ))),
    }
}

fn validate_codex_toggle_instance(
    ctx: &AdapterContext,
    meta: &SkillInstanceMeta,
) -> Result<(), CommandError> {
    let canonical_skill = meta.path.canonicalize().map_err(|err| {
        CommandError::UnsafeConfigPath(format!(
            "Codex skill {} cannot be canonicalized for the toggle allowlist: {err}",
            meta.path.display()
        ))
    })?;
    if codex_path_has_plugin_marker(&canonical_skill) {
        return Err(CommandError::UnsafeConfigPath(
            "Codex plugin marketplace skills are scan-only and cannot be used as toggle targets"
                .to_string(),
        ));
    }

    match meta.scope {
        Scope::AgentGlobal => {
            let native_root = ctx.user_home.join(".agents/skills");
            if canonical_path_starts_with_existing_root(&canonical_skill, &native_root) {
                Ok(())
            } else {
                Err(CommandError::UnsafeConfigPath(format!(
                    "Codex global toggles are limited to verified user .agents/skills roots; {} is read-only",
                    meta.path.display()
                )))
            }
        }
        Scope::AgentProject => {
            let project_root = ctx
                .project_root
                .as_ref()
                .ok_or(CommandError::UnsupportedScope(meta.scope))?;
            let canonical_project = project_root.canonicalize().map_err(|err| {
                CommandError::UnsafeConfigPath(format!(
                    "Codex project root {} cannot be canonicalized for the toggle allowlist: {err}",
                    project_root.display()
                ))
            })?;
            if !canonical_skill.starts_with(&canonical_project) {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "Codex project skill {} resolves outside the selected project root {}",
                    meta.path.display(),
                    project_root.display()
                )));
            }
            if codex_project_native_root_allows(ctx, &canonical_skill) {
                Ok(())
            } else {
                Err(CommandError::UnsafeConfigPath(format!(
                    "Codex project toggles are limited to verified project .agents/skills roots; {} is read-only",
                    meta.path.display()
                )))
            }
        }
        Scope::ToolGlobal => Err(CommandError::UnsupportedScope(meta.scope)),
        _ => Err(CommandError::UnsupportedScope(meta.scope)),
    }
}

fn codex_project_native_root_allows(ctx: &AdapterContext, canonical_skill: &Path) -> bool {
    let Some(project_root) = &ctx.project_root else {
        return false;
    };
    let start = ctx
        .project_cwd
        .as_deref()
        .filter(|cwd| cwd.starts_with(project_root))
        .unwrap_or(project_root);
    let mut current = Some(start);

    while let Some(dir) = current {
        if canonical_path_starts_with_existing_root(canonical_skill, &dir.join(".agents/skills")) {
            return true;
        }
        if dir == project_root {
            break;
        }
        current = dir
            .parent()
            .filter(|parent| parent.starts_with(project_root));
    }

    has_agents_skills_ancestor_inside_project(canonical_skill, project_root)
}

fn has_agents_skills_ancestor_inside_project(canonical_skill: &Path, project_root: &Path) -> bool {
    let Ok(canonical_project) = project_root.canonicalize() else {
        return false;
    };
    if !canonical_skill.starts_with(&canonical_project) {
        return false;
    }
    canonical_skill.ancestors().any(|ancestor| {
        ancestor.file_name().and_then(|name| name.to_str()) == Some("skills")
            && ancestor
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                == Some(".agents")
            && ancestor.starts_with(&canonical_project)
    })
}

fn canonical_path_starts_with_existing_root(path: &Path, root: &Path) -> bool {
    root.canonicalize()
        .map(|canonical_root| path.starts_with(canonical_root))
        .unwrap_or(false)
}

fn codex_path_has_plugin_marker(path: &Path) -> bool {
    let components = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();
    components.windows(2).any(|window| {
        matches!(
            window,
            [".agents", "plugins"] | [".codex", "plugins"] | [".claude-plugin", _]
        )
    })
}

fn project_record_matches_context(
    record_project_root: Option<&Path>,
    current_project_root: Option<&Path>,
) -> bool {
    let (Some(record_project_root), Some(current_project_root)) =
        (record_project_root, current_project_root)
    else {
        return false;
    };
    if record_project_root == current_project_root {
        return true;
    }
    match (
        record_project_root.canonicalize(),
        current_project_root.canonicalize(),
    ) {
        (Ok(record), Ok(current)) => record == current,
        _ => false,
    }
}

fn project_record_matches_context_for_agent(
    agent: AgentId,
    record_project_root: Option<&Path>,
    current_project_root: Option<&Path>,
) -> bool {
    if project_record_matches_context(record_project_root, current_project_root) {
        return true;
    }
    if agent != AgentId::Openclaw {
        return false;
    }
    let (Some(record_project_root), Some(current_project_root)) =
        (record_project_root, current_project_root)
    else {
        return false;
    };
    match (
        record_project_root.canonicalize(),
        current_project_root.canonicalize(),
    ) {
        (Ok(record), Ok(current)) => current.starts_with(record),
        _ => {
            let record = normalize_path_lexically(record_project_root);
            let current = normalize_path_lexically(current_project_root);
            current.starts_with(record)
        }
    }
}

fn config_target_for_claude(
    ctx: &AdapterContext,
    scope: Scope,
) -> Result<ConfigTarget, CommandError> {
    let path = match scope {
        Scope::AgentGlobal => ctx.user_home.join(".claude/settings.json"),
        Scope::AgentProject => ctx
            .project_root
            .as_ref()
            .map(|root| root.join(".claude/settings.local.json"))
            .ok_or(CommandError::UnsupportedScope(scope))?,
        Scope::ToolGlobal => return Err(CommandError::UnsupportedScope(scope)),
        // Scope is #[non_exhaustive]; unknown variants cannot be toggled.
        _ => return Err(CommandError::UnsupportedScope(scope)),
    };
    Ok(ConfigTarget {
        agent: AgentId::ClaudeCode,
        scope,
        path,
        format: ConfigFormat::Json,
    })
}

fn skill_install_target_path(
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
    skill_name: &str,
    project_path: Option<&Path>,
) -> Result<PathBuf, CommandError> {
    let root = skill_install_root(ctx, agent, scope, project_path)?;
    Ok(root
        .join(safe_install_dir_name(skill_name)?)
        .join("SKILL.md"))
}

fn skill_install_root(
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
    project_path: Option<&Path>,
) -> Result<PathBuf, CommandError> {
    match (agent, scope) {
        (AgentId::ClaudeCode, Scope::AgentGlobal) => {
            Ok(ctx.user_home.join(".claude").join("skills"))
        }
        (AgentId::ClaudeCode, Scope::AgentProject) => Ok(target_project_root(ctx, project_path)?
            .join(".claude")
            .join("skills")),
        (AgentId::Codex, Scope::AgentGlobal) => Ok(ctx.user_home.join(".agents").join("skills")),
        (AgentId::Codex, Scope::AgentProject) => Ok(target_project_root(ctx, project_path)?
            .join(".agents")
            .join("skills")),
        (AgentId::Opencode, Scope::AgentGlobal) => Ok(ctx
            .user_home
            .join(".config")
            .join("opencode")
            .join("skills")),
        (AgentId::Opencode, Scope::AgentProject) => Ok(target_project_root(ctx, project_path)?
            .join(".opencode")
            .join("skills")),
        (AgentId::Pi, Scope::AgentGlobal) => {
            Ok(ctx.user_home.join(".pi").join("agent").join("skills"))
        }
        (AgentId::Pi, Scope::AgentProject) => Ok(target_project_root(ctx, project_path)?
            .join(".pi")
            .join("skills")),
        (AgentId::Hermes, Scope::AgentGlobal) => Ok(ctx.user_home.join(".hermes").join("skills")),
        (AgentId::Openclaw, Scope::AgentGlobal) => {
            Ok(ctx.user_home.join(".openclaw").join("skills"))
        }
        (AgentId::Openclaw, Scope::AgentProject) => {
            Ok(openclaw_install_workspace_root(ctx, project_path)?.join("skills"))
        }
        (_, Scope::ToolGlobal) => Err(CommandError::UnsupportedScope(scope)),
        (agent, _) => Err(CommandError::InstallUnsupported(format!(
            "{} skills are not writable by install commands",
            agent.as_str()
        ))),
    }
}

fn target_project_root(
    ctx: &AdapterContext,
    project_path: Option<&Path>,
) -> Result<PathBuf, CommandError> {
    let project_root = project_path
        .map(Path::to_path_buf)
        .or_else(|| ctx.project_root.clone())
        .ok_or(CommandError::UnsupportedScope(Scope::AgentProject))?;
    let canonical_project = project_root.canonicalize().map_err(|err| {
        CommandError::UnsafeConfigPath(format!(
            "project path {} cannot be canonicalized: {err}",
            project_root.display()
        ))
    })?;
    if !canonical_project.is_dir() {
        return Err(CommandError::UnsafeConfigPath(format!(
            "project path {} is not a directory",
            canonical_project.display()
        )));
    }
    if let Some(current_root) = &ctx.project_root {
        let canonical_current = current_root.canonicalize()?;
        if canonical_project != canonical_current {
            return Err(CommandError::UnsafeConfigPath(format!(
                "target project {} does not match current project context {}",
                canonical_project.display(),
                canonical_current.display()
            )));
        }
    }
    Ok(canonical_project)
}

fn openclaw_install_workspace_root(
    ctx: &AdapterContext,
    project_path: Option<&Path>,
) -> Result<PathBuf, CommandError> {
    let selected_path = project_path
        .map(Path::to_path_buf)
        .or_else(|| ctx.project_root.clone())
        .or_else(|| ctx.project_cwd.clone())
        .ok_or(CommandError::UnsupportedScope(Scope::AgentProject))?;
    let canonical_selected = selected_path.canonicalize().map_err(|err| {
        CommandError::UnsafeConfigPath(format!(
            "OpenClaw project path {} cannot be canonicalized: {err}",
            selected_path.display()
        ))
    })?;
    if !canonical_selected.is_dir() {
        return Err(CommandError::UnsafeConfigPath(format!(
            "OpenClaw project path {} is not a directory",
            canonical_selected.display()
        )));
    }

    let workspace = openclaw_home_workspace_candidates(ctx)
        .into_iter()
        .filter(|candidate| candidate.exists())
        .find_map(|candidate| {
            let canonical_candidate = candidate.canonicalize().ok()?;
            if canonical_selected == canonical_candidate
                || canonical_selected.starts_with(&canonical_candidate)
            {
                Some(canonical_candidate)
            } else {
                None
            }
        })
        .ok_or_else(|| {
            CommandError::UnsafeConfigPath(format!(
                "OpenClaw project installs must target a confirmed OpenClaw workspace under {}",
                ctx.user_home.display()
            ))
        })?;

    for selected_context in [ctx.project_root.as_ref(), ctx.project_cwd.as_ref()]
        .into_iter()
        .flatten()
    {
        let canonical_context = selected_context.canonicalize().map_err(|err| {
            CommandError::UnsafeConfigPath(format!(
                "OpenClaw context path {} cannot be canonicalized: {err}",
                selected_context.display()
            ))
        })?;
        if canonical_context != workspace && !canonical_context.starts_with(&workspace) {
            return Err(CommandError::UnsafeConfigPath(format!(
                "OpenClaw context {} is outside confirmed workspace {}",
                canonical_context.display(),
                workspace.display()
            )));
        }
    }

    Ok(workspace)
}

fn openclaw_home_workspace_candidates(ctx: &AdapterContext) -> [PathBuf; 2] {
    [
        ctx.user_home.join(".openclaw/workspace"),
        ctx.user_home.join("openclaw/workspace"),
    ]
}

fn install_scan_context(
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
    project_path: Option<&Path>,
) -> Result<AdapterContext, CommandError> {
    if scope != Scope::AgentProject {
        return Ok(ctx.clone());
    }
    let project_root = if agent == AgentId::Openclaw {
        openclaw_install_workspace_root(ctx, project_path)?
    } else {
        target_project_root(ctx, project_path)?
    };
    Ok(AdapterContext {
        user_home: ctx.user_home.clone(),
        project_cwd: Some(project_root.clone()),
        project_root: Some(project_root),
        extra_roots: ctx.extra_roots.clone(),
    })
}

fn safe_install_dir_name(name: &str) -> Result<String, CommandError> {
    let name = name.trim();
    let invalid = name.is_empty()
        || name == "."
        || name == ".."
        || name.contains('/')
        || name.contains('\\')
        || name.bytes().any(|byte| byte == 0);
    if invalid {
        return Err(CommandError::UnsafeConfigPath(format!(
            "skill name `{name}` cannot be used as an install directory"
        )));
    }
    Ok(name.to_string())
}

fn install_preview_risks(agent: AgentId, scope: Scope, target_exists: bool) -> Vec<String> {
    let mut risks = vec![
        format!(
            "Will write into the {} {} skill root through the verified install path.",
            agent.as_str(),
            scope.as_str()
        ),
        "Only the tool-global SKILL.md source will be copied.".to_string(),
    ];
    if target_exists {
        risks.push(
            "Target SKILL.md already exists and will be replaced after snapshot.".to_string(),
        );
    }
    if agent == AgentId::Codex {
        risks.push(
            "Codex may need to be restarted before it reads newly installed user/project skills."
                .to_string(),
        );
    }
    if agent == AgentId::Hermes {
        risks.push(
            "Hermes may need a reload or restart before it reads newly installed local skills."
                .to_string(),
        );
        risks.push(
            "Hermes hub, URL, tap, update, uninstall, and external_dirs writes are not part of this install path."
                .to_string(),
        );
    }
    if agent == AgentId::Openclaw {
        risks.push(
            "OpenClaw may need a new session or watcher reload before it reads newly installed local skills."
                .to_string(),
        );
        risks.push(
            "OpenClaw .agents direct installs, skills.entries writes, ClawHub, Git, update, verify, workshop, and network-backed operations are not part of this install path."
                .to_string(),
        );
    }
    risks
}

fn claude_global_settings_path(ctx: &AdapterContext) -> PathBuf {
    ctx.user_home.join(".claude/settings.json")
}

fn codex_user_config_path(ctx: &AdapterContext) -> PathBuf {
    let codex_home = env::var_os("CODEX_HOME").map(PathBuf::from);
    codex_user_config_path_for(ctx, codex_home.as_deref())
}

fn codex_user_config_path_for(ctx: &AdapterContext, codex_home: Option<&Path>) -> PathBuf {
    if let Some(codex_home) = codex_home {
        if should_honor_codex_home(ctx, codex_home) {
            return codex_home.join("config.toml");
        }
    }
    ctx.user_home.join(".codex/config.toml")
}

fn opencode_config_path(ctx: &AdapterContext, scope: Scope) -> Result<PathBuf, CommandError> {
    match scope {
        Scope::AgentGlobal => Ok(ctx.user_home.join(".config/opencode/opencode.json")),
        Scope::AgentProject => ctx
            .project_root
            .as_ref()
            .map(|root| root.join("opencode.json"))
            .ok_or(CommandError::UnsupportedScope(scope)),
        Scope::ToolGlobal => Err(CommandError::UnsupportedScope(scope)),
        _ => Err(CommandError::UnsupportedScope(scope)),
    }
}

fn pi_expected_config_path(ctx: &AdapterContext, scope: Scope) -> Result<PathBuf, CommandError> {
    match scope {
        Scope::AgentGlobal => Ok(pi_agent_dir(ctx).join("settings.json")),
        Scope::AgentProject => ctx
            .project_root
            .as_ref()
            .map(|root| root.join(".pi/settings.json"))
            .ok_or(CommandError::UnsupportedScope(scope)),
        Scope::ToolGlobal => Err(CommandError::UnsupportedScope(scope)),
        _ => Err(CommandError::UnsupportedScope(scope)),
    }
}

fn pi_config_path_for_instance(
    ctx: &AdapterContext,
    meta: &SkillInstanceMeta,
) -> Result<PathBuf, CommandError> {
    pi_config_path_for_skill_path(ctx, meta.scope, &meta.path)
}

fn pi_config_path_for_skill_path(
    ctx: &AdapterContext,
    scope: Scope,
    skill_path: &Path,
) -> Result<PathBuf, CommandError> {
    match scope {
        Scope::AgentGlobal => Ok(pi_agent_dir(ctx).join("settings.json")),
        Scope::AgentProject => {
            if let Some(pi_skill_root) = pi_project_native_skill_root(skill_path) {
                let pi_dir = pi_skill_root.parent().ok_or_else(|| {
                    CommandError::UnsafeConfigPath("Pi skill root has no .pi parent".to_string())
                })?;
                return Ok(pi_dir.join("settings.json"));
            }
            if pi_project_compatibility_skill_root(skill_path).is_some() {
                return ctx
                    .project_root
                    .as_ref()
                    .map(|root| root.join(".pi/settings.json"))
                    .ok_or(CommandError::UnsupportedScope(scope));
            }
            Err(CommandError::UnsafeConfigPath(format!(
                "Pi project skill {} is not under a .pi/skills or .agents/skills root",
                skill_path.display()
            )))
        }
        Scope::ToolGlobal => Err(CommandError::UnsupportedScope(scope)),
        _ => Err(CommandError::UnsupportedScope(scope)),
    }
}

fn pi_project_native_skill_root(skill_path: &Path) -> Option<&Path> {
    skill_path.ancestors().find(|ancestor| {
        ancestor.file_name().and_then(|name| name.to_str()) == Some("skills")
            && ancestor
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                == Some(".pi")
    })
}

fn pi_project_compatibility_skill_root(skill_path: &Path) -> Option<&Path> {
    skill_path.ancestors().find(|ancestor| {
        ancestor.file_name().and_then(|name| name.to_str()) == Some("skills")
            && ancestor
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                == Some(".agents")
    })
}

fn validate_pi_config_write_target(
    ctx: &AdapterContext,
    scope: Scope,
    path: &Path,
) -> Result<(), CommandError> {
    match scope {
        Scope::AgentGlobal => {
            let expected = ctx.user_home.join(".pi/agent/settings.json");
            if path != expected.as_path() {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "{} does not match expected Pi global config path {}",
                    path.display(),
                    expected.display()
                )));
            }
        }
        Scope::AgentProject => {
            if path.file_name().and_then(|name| name.to_str()) != Some("settings.json")
                || path
                    .parent()
                    .and_then(Path::file_name)
                    .and_then(|name| name.to_str())
                    != Some(".pi")
            {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "{} is not a Pi project/package settings path",
                    path.display()
                )));
            }
        }
        Scope::ToolGlobal => return Err(CommandError::UnsupportedScope(scope)),
        _ => return Err(CommandError::UnsupportedScope(scope)),
    }

    let allowed_root = match scope {
        Scope::AgentGlobal => &ctx.user_home,
        Scope::AgentProject => ctx
            .project_root
            .as_ref()
            .ok_or(CommandError::UnsupportedScope(scope))?,
        Scope::ToolGlobal => return Err(CommandError::UnsupportedScope(scope)),
        _ => return Err(CommandError::UnsupportedScope(scope)),
    };
    let parent = path.parent().ok_or_else(|| {
        CommandError::UnsafeConfigPath("Pi config path has no parent".to_string())
    })?;
    fs::create_dir_all(parent)?;

    reject_symlink(parent, "Pi config directory")?;
    reject_symlink(path, "Pi config file")?;

    let canonical_root = allowed_root.canonicalize()?;
    let canonical_parent = parent.canonicalize()?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(CommandError::UnsafeConfigPath(format!(
            "Pi config directory {} resolves outside allowed root {}",
            canonical_parent.display(),
            canonical_root.display()
        )));
    }
    Ok(())
}

fn should_honor_codex_home(ctx: &AdapterContext, codex_home: &Path) -> bool {
    codex_home.is_absolute()
        && normalize_path_lexically(codex_home)
            .starts_with(normalize_path_lexically(&ctx.user_home))
}

pub(crate) fn normalize_path_lexically(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
}

fn scan_agent_id_to_catalog(
    agent: AgentId,
    ctx: &AdapterContext,
    catalog: &Catalog,
) -> Result<usize, CommandError> {
    match agent {
        AgentId::ClaudeCode => scan_single_agent_to_catalog(&ClaudeCodeAdapter, ctx, catalog),
        AgentId::Codex => scan_single_agent_to_catalog(&CodexAdapter, ctx, catalog),
        AgentId::Opencode => scan_single_agent_to_catalog(&OpencodeAdapter, ctx, catalog),
        AgentId::Pi => scan_single_agent_to_catalog(&PiAdapter, ctx, catalog),
        AgentId::Openclaw => scan_single_agent_to_catalog(&OpenclawAdapter, ctx, catalog),
        AgentId::Hermes => scan_single_agent_to_catalog(&HermesAdapter, ctx, catalog),
        agent => Err(CommandError::UnsafeConfigPath(format!(
            "{} skills are not supported by scan commands",
            agent.as_str()
        ))),
    }
}

fn preview_context_from_snapshot(
    snapshot: &ConfigSnapshotRecord,
) -> Result<AdapterContext, CommandError> {
    let target = PathBuf::from(&snapshot.target);
    let scope = scope_from_snapshot(&snapshot.scope)?;
    let agent = agent_from_snapshot(&snapshot.agent)?;
    let parent = target.parent().ok_or_else(|| {
        CommandError::UnsafeConfigPath("snapshot target has no parent".to_string())
    })?;

    match (agent, scope) {
        (AgentId::ClaudeCode, Scope::AgentGlobal) => {
            if target.file_name().and_then(|name| name.to_str()) != Some("settings.json")
                || parent.file_name().and_then(|name| name.to_str()) != Some(".claude")
            {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "snapshot target {} is not a Claude global settings path",
                    target.display()
                )));
            }
            let user_home = parent
                .parent()
                .ok_or_else(|| {
                    CommandError::UnsafeConfigPath(
                        "Claude global settings target has no user home".to_string(),
                    )
                })?
                .to_path_buf();
            Ok(AdapterContext {
                user_home,
                project_root: None,
                project_cwd: None,
                extra_roots: vec![],
            })
        }
        (AgentId::ClaudeCode, Scope::AgentProject) => {
            if target.file_name().and_then(|name| name.to_str()) != Some("settings.local.json")
                || parent.file_name().and_then(|name| name.to_str()) != Some(".claude")
            {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "snapshot target {} is not a Claude project settings path",
                    target.display()
                )));
            }
            let project_root = parent
                .parent()
                .ok_or_else(|| {
                    CommandError::UnsafeConfigPath(
                        "Claude project settings target has no project root".to_string(),
                    )
                })?
                .to_path_buf();
            Ok(AdapterContext {
                user_home: project_root.clone(),
                project_root: Some(project_root),
                project_cwd: None,
                extra_roots: vec![],
            })
        }
        (AgentId::Codex, Scope::AgentGlobal) => {
            if target.file_name().and_then(|name| name.to_str()) != Some("config.toml") {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "snapshot target {} is not a Codex user config path",
                    target.display()
                )));
            }
            let parent_is_codex_home = parent.file_name().and_then(|name| name.to_str())
                == Some(".codex")
                || env::var_os("CODEX_HOME")
                    .map(PathBuf::from)
                    .is_some_and(|codex_home| normalize_path_lexically(&codex_home) == parent);
            if !parent_is_codex_home {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "snapshot target {} is not a Codex user config path",
                    target.display()
                )));
            }
            let user_home = parent
                .parent()
                .ok_or_else(|| {
                    CommandError::UnsafeConfigPath(
                        "Codex config target has no user home".to_string(),
                    )
                })?
                .to_path_buf();
            Ok(AdapterContext {
                user_home,
                project_root: None,
                project_cwd: None,
                extra_roots: vec![],
            })
        }
        (AgentId::Opencode, Scope::AgentGlobal) => {
            if target.file_name().and_then(|name| name.to_str()) != Some("opencode.json")
                || parent.file_name().and_then(|name| name.to_str()) != Some("opencode")
            {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "snapshot target {} is not an opencode global config path",
                    target.display()
                )));
            }
            let user_home = parent
                .parent()
                .and_then(Path::parent)
                .ok_or_else(|| {
                    CommandError::UnsafeConfigPath(
                        "opencode global config target has no user home".to_string(),
                    )
                })?
                .to_path_buf();
            Ok(AdapterContext {
                user_home,
                project_root: None,
                project_cwd: None,
                extra_roots: vec![],
            })
        }
        (AgentId::Opencode, Scope::AgentProject) => {
            if target.file_name().and_then(|name| name.to_str()) != Some("opencode.json") {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "snapshot target {} is not an opencode project config path",
                    target.display()
                )));
            }
            let project_root = parent.to_path_buf();
            Ok(AdapterContext {
                user_home: project_root.clone(),
                project_root: Some(project_root),
                project_cwd: None,
                extra_roots: vec![],
            })
        }
        (AgentId::Pi, Scope::AgentGlobal) => {
            if target.file_name().and_then(|name| name.to_str()) != Some("settings.json")
                || parent.file_name().and_then(|name| name.to_str()) != Some(".pi")
            {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "snapshot target {} is not a Pi global settings path",
                    target.display()
                )));
            }
            let user_home = parent
                .parent()
                .ok_or_else(|| {
                    CommandError::UnsafeConfigPath(
                        "Pi global settings target has no user home".to_string(),
                    )
                })?
                .to_path_buf();
            Ok(AdapterContext {
                user_home,
                project_root: None,
                project_cwd: None,
                extra_roots: vec![],
            })
        }
        (AgentId::Pi, Scope::AgentProject) => {
            if target.file_name().and_then(|name| name.to_str()) != Some("settings.json")
                || parent.file_name().and_then(|name| name.to_str()) != Some(".pi")
            {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "snapshot target {} is not a Pi project/package settings path",
                    target.display()
                )));
            }
            let project_root = parent
                .parent()
                .ok_or_else(|| {
                    CommandError::UnsafeConfigPath(
                        "Pi project settings target has no project root".to_string(),
                    )
                })?
                .to_path_buf();
            Ok(AdapterContext {
                user_home: project_root.clone(),
                project_root: Some(project_root.clone()),
                project_cwd: Some(project_root),
                extra_roots: vec![],
            })
        }
        (AgentId::Hermes, Scope::AgentGlobal) => {
            if target.file_name().and_then(|name| name.to_str()) != Some("config.yaml")
                || parent.file_name().and_then(|name| name.to_str()) != Some(".hermes")
            {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "snapshot target {} is not a Hermes global config path",
                    target.display()
                )));
            }
            let user_home = parent
                .parent()
                .ok_or_else(|| {
                    CommandError::UnsafeConfigPath(
                        "Hermes global config target has no user home".to_string(),
                    )
                })?
                .to_path_buf();
            Ok(AdapterContext {
                user_home,
                project_root: None,
                project_cwd: None,
                extra_roots: vec![],
            })
        }
        (AgentId::Openclaw, Scope::AgentGlobal) => {
            if target.file_name().and_then(|name| name.to_str()) != Some("openclaw.json")
                || parent.file_name().and_then(|name| name.to_str()) != Some(".openclaw")
            {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "snapshot target {} is not an OpenClaw global config path",
                    target.display()
                )));
            }
            let user_home = parent
                .parent()
                .ok_or_else(|| {
                    CommandError::UnsafeConfigPath(
                        "OpenClaw global config target has no user home".to_string(),
                    )
                })?
                .to_path_buf();
            Ok(AdapterContext {
                user_home,
                project_root: None,
                project_cwd: None,
                extra_roots: vec![],
            })
        }
        _ => Err(CommandError::UnsafeConfigPath(format!(
            "snapshot agent {} scope {} is not previewable",
            snapshot.agent, snapshot.scope
        ))),
    }
}

pub(crate) fn expected_config_target(
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
) -> Result<ConfigTarget, CommandError> {
    match agent {
        AgentId::ClaudeCode => config_target_for_claude(ctx, scope),
        AgentId::Codex => {
            if scope != Scope::AgentGlobal {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "codex config writes use user config scope only; snapshot scope {} is not writable",
                    scope.as_str()
                )));
            }
            Ok(ConfigTarget {
                agent,
                scope,
                path: codex_user_config_path(ctx),
                format: ConfigFormat::Toml,
            })
        }
        AgentId::Opencode => Ok(ConfigTarget {
            agent,
            scope,
            path: opencode_config_path(ctx, scope)?,
            format: ConfigFormat::Json,
        }),
        AgentId::Pi => Ok(ConfigTarget {
            agent,
            scope,
            path: pi_expected_config_path(ctx, scope)?,
            format: ConfigFormat::Json,
        }),
        AgentId::Hermes => {
            if scope != Scope::AgentGlobal {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "Hermes config writes use global config scope only; snapshot scope {} is not writable",
                    scope.as_str()
                )));
            }
            Ok(ConfigTarget {
                agent,
                scope,
                path: ctx.user_home.join(".hermes/config.yaml"),
                format: ConfigFormat::Yaml,
            })
        }
        AgentId::Openclaw => {
            if scope != Scope::AgentGlobal {
                return Err(CommandError::UnsafeConfigPath(format!(
                    "OpenClaw config writes use global openclaw.json only; snapshot scope {} is not writable",
                    scope.as_str()
                )));
            }
            Ok(ConfigTarget {
                agent,
                scope,
                path: ctx.user_home.join(".openclaw/openclaw.json"),
                format: ConfigFormat::Json,
            })
        }
        agent => Err(CommandError::UnsafeConfigPath(format!(
            "{} config writes are not supported",
            agent.as_str()
        ))),
    }
}

fn validate_config_write_target(
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
    path: &Path,
) -> Result<(), CommandError> {
    if agent == AgentId::Pi {
        return validate_pi_config_write_target(ctx, scope, path);
    }
    let expected = expected_config_target(ctx, agent, scope)?;
    if path != expected.path.as_path() {
        return Err(CommandError::UnsafeConfigPath(format!(
            "{} does not match expected {} config path {}",
            path.display(),
            agent.as_str(),
            expected.path.display()
        )));
    }

    let allowed_root = match scope {
        Scope::AgentGlobal if agent == AgentId::Codex => &ctx.user_home,
        Scope::AgentGlobal => &ctx.user_home,
        Scope::AgentProject if matches!(agent, AgentId::ClaudeCode | AgentId::Opencode) => ctx
            .project_root
            .as_ref()
            .ok_or(CommandError::UnsupportedScope(scope))?,
        Scope::ToolGlobal => return Err(CommandError::UnsupportedScope(scope)),
        _ => return Err(CommandError::UnsupportedScope(scope)),
    };
    let parent = path
        .parent()
        .ok_or_else(|| CommandError::UnsafeConfigPath("config path has no parent".to_string()))?;
    fs::create_dir_all(parent)?;

    reject_symlink(parent, "config directory")?;
    reject_symlink(path, "config file")?;

    let canonical_root = allowed_root.canonicalize()?;
    let canonical_parent = parent.canonicalize()?;
    if !canonical_parent.starts_with(&canonical_root) {
        return Err(CommandError::UnsafeConfigPath(format!(
            "config directory {} resolves outside allowed root {}",
            canonical_parent.display(),
            canonical_root.display()
        )));
    }

    Ok(())
}

fn validate_config_write_target_noncreating(
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
    path: &Path,
) -> Result<(), CommandError> {
    if agent == AgentId::Pi {
        match scope {
            Scope::AgentGlobal
                if path != ctx.user_home.join(".pi/agent/settings.json").as_path() =>
            {
                return Err(CommandError::UnsafeConfigPath(
                    "Pi global config target does not match the guarded path".to_string(),
                ));
            }
            Scope::AgentProject
                if path.file_name().and_then(|name| name.to_str()) != Some("settings.json")
                    || path
                        .parent()
                        .and_then(Path::file_name)
                        .and_then(|name| name.to_str())
                        != Some(".pi") =>
            {
                return Err(CommandError::UnsafeConfigPath(
                    "Pi project config target does not match the guarded path".to_string(),
                ));
            }
            Scope::AgentGlobal | Scope::AgentProject => {}
            _ => return Err(CommandError::UnsupportedScope(scope)),
        }
    } else {
        let expected = expected_config_target(ctx, agent, scope)?;
        if path != expected.path {
            return Err(CommandError::UnsafeConfigPath(
                "config target does not match the guarded adapter path".to_string(),
            ));
        }
    }

    let allowed_root = match scope {
        Scope::AgentGlobal => &ctx.user_home,
        Scope::AgentProject => ctx
            .project_root
            .as_ref()
            .ok_or(CommandError::UnsupportedScope(scope))?,
        _ => return Err(CommandError::UnsupportedScope(scope)),
    };
    let parent = path
        .parent()
        .ok_or_else(|| CommandError::UnsafeConfigPath("config path has no parent".to_string()))?;
    let normalized_parent = normalize_path_lexically(parent);
    let normalized_root = normalize_path_lexically(allowed_root);
    if !normalized_parent.starts_with(&normalized_root) {
        return Err(CommandError::UnsafeConfigPath(
            "config parent is outside the guarded owner root".to_string(),
        ));
    }
    reject_symlink(path, "config file")?;
    let mut existing = parent;
    while !existing.exists() {
        reject_symlink(existing, "config directory")?;
        existing = existing.parent().ok_or_else(|| {
            CommandError::UnsafeConfigPath(
                "config parent has no existing guarded ancestor".to_string(),
            )
        })?;
    }
    reject_symlink(existing, "config directory")?;
    let canonical_root = allowed_root.canonicalize()?;
    let canonical_existing = existing.canonicalize()?;
    if !canonical_existing.starts_with(&canonical_root) {
        return Err(CommandError::UnsafeConfigPath(
            "config parent resolves outside the guarded owner root".to_string(),
        ));
    }
    Ok(())
}

fn config_target_allowed_root(
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
) -> Result<PathBuf, CommandError> {
    match scope {
        Scope::AgentGlobal => Ok(ctx.user_home.clone()),
        Scope::AgentProject
            if matches!(agent, AgentId::ClaudeCode | AgentId::Opencode | AgentId::Pi) =>
        {
            ctx.project_root
                .clone()
                .ok_or(CommandError::UnsupportedScope(scope))
        }
        _ => Err(CommandError::UnsupportedScope(scope)),
    }
}

fn prepare_config_target_capability<'lock>(
    lock: &'lock AppMutationLock,
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
    path: &Path,
) -> Result<ExternalTargetCapability<'lock>, CommandError> {
    validate_config_write_target_noncreating(ctx, agent, scope, path)?;
    let root = config_target_allowed_root(ctx, agent, scope)?;
    ExternalTargetCapability::prepare(lock, &root, path)
}

fn validate_tool_global_source(path: &Path) -> Result<(), CommandError> {
    if path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md") {
        return Err(CommandError::UnsafeConfigPath(format!(
            "tool-global source {} is not a SKILL.md file",
            path.display()
        )));
    }
    reject_symlink(path, "tool-global source file")?;
    let metadata = fs::metadata(path)?;
    if !metadata.is_file() {
        return Err(CommandError::UnsafeConfigPath(format!(
            "tool-global source {} is not a regular file",
            path.display()
        )));
    }
    Ok(())
}

fn validate_skill_install_target(
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
    path: &Path,
    project_path: Option<&Path>,
    create_dirs: bool,
) -> Result<(), CommandError> {
    let expected_root = skill_install_root(ctx, agent, scope, project_path)?;
    let expected_target = expected_root
        .join(path.parent().and_then(Path::file_name).ok_or_else(|| {
            CommandError::UnsafeConfigPath("install target has no skill directory".to_string())
        })?)
        .join("SKILL.md");
    if path != expected_target.as_path() {
        return Err(CommandError::UnsafeConfigPath(format!(
            "{} does not match expected {} install target {}",
            path.display(),
            agent.as_str(),
            expected_target.display()
        )));
    }
    if path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md") {
        return Err(CommandError::UnsafeConfigPath(format!(
            "install target {} is not a SKILL.md path",
            path.display()
        )));
    }

    let allowed_root = match (agent, scope) {
        (AgentId::Openclaw, Scope::AgentProject) => {
            openclaw_install_workspace_root(ctx, project_path)?
        }
        (_, Scope::AgentGlobal) => ctx.user_home.clone(),
        (_, Scope::AgentProject) => target_project_root(ctx, project_path)?,
        (_, Scope::ToolGlobal) => return Err(CommandError::UnsupportedScope(scope)),
        _ => return Err(CommandError::UnsupportedScope(scope)),
    };
    if create_dirs {
        fs::create_dir_all(&expected_root)?;
    }
    let parent = path.parent().ok_or_else(|| {
        CommandError::UnsafeConfigPath("install target has no parent".to_string())
    })?;
    if create_dirs {
        fs::create_dir_all(parent)?;
    }

    reject_symlink(&expected_root, "install root")?;
    reject_symlink(parent, "install skill directory")?;
    reject_symlink(path, "install target file")?;

    let canonical_allowed_root = allowed_root.canonicalize()?;
    let normalized_install_root = normalize_path_lexically(&expected_root);
    let normalized_parent = normalize_path_lexically(parent);
    let normalized_allowed_root = normalize_path_lexically(&allowed_root);
    if !normalized_install_root.starts_with(&normalized_allowed_root) {
        return Err(CommandError::UnsafeConfigPath(format!(
            "install root {} is outside allowed root {}",
            expected_root.display(),
            allowed_root.display()
        )));
    }
    if !normalized_parent.starts_with(&normalized_install_root) {
        return Err(CommandError::UnsafeConfigPath(format!(
            "install target directory {} is outside install root {}",
            parent.display(),
            expected_root.display()
        )));
    }
    if expected_root.exists() {
        let canonical_install_root = expected_root.canonicalize()?;
        if !canonical_install_root.starts_with(&canonical_allowed_root) {
            return Err(CommandError::UnsafeConfigPath(format!(
                "install root {} resolves outside allowed root {}",
                canonical_install_root.display(),
                canonical_allowed_root.display()
            )));
        }
    }
    if parent.exists() {
        let canonical_install_root = expected_root.canonicalize()?;
        let canonical_parent = parent.canonicalize()?;
        if !canonical_parent.starts_with(&canonical_install_root) {
            return Err(CommandError::UnsafeConfigPath(format!(
                "install target directory {} resolves outside install root {}",
                canonical_parent.display(),
                canonical_install_root.display()
            )));
        }
    }
    Ok(())
}

fn skill_target_allowed_root(
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
    project_path: Option<&Path>,
) -> Result<PathBuf, CommandError> {
    match (agent, scope) {
        (AgentId::Openclaw, Scope::AgentProject) => {
            openclaw_install_workspace_root(ctx, project_path)
        }
        (_, Scope::AgentGlobal) => Ok(ctx.user_home.clone()),
        (_, Scope::AgentProject) => target_project_root(ctx, project_path),
        (_, Scope::ToolGlobal) => Err(CommandError::UnsupportedScope(scope)),
        _ => Err(CommandError::UnsupportedScope(scope)),
    }
}

fn prepare_skill_target_capability<'lock>(
    lock: &'lock AppMutationLock,
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
    path: &Path,
    project_path: Option<&Path>,
) -> Result<ExternalTargetCapability<'lock>, CommandError> {
    validate_skill_install_target(ctx, agent, scope, path, project_path, false)?;
    let root = skill_target_allowed_root(ctx, agent, scope, project_path)?;
    ExternalTargetCapability::prepare(lock, &root, path)
}

fn config_state_from_external(state: ExternalFileState) -> config_consistency::ConfigState {
    config_consistency::ConfigState {
        exists: state.exists,
        revision: config_revision(state.exists, &state.content),
        content: state.content,
    }
}

fn read_external_config_state(
    target: &ExternalTargetCapability<'_>,
) -> Result<config_consistency::ConfigState, CommandError> {
    Ok(config_state_from_external(
        target.read_text_state(MAX_EXTERNAL_CONFIG_BYTES)?,
    ))
}

fn read_external_config_state_anchored(
    target: &ExternalTargetCapability<'_>,
) -> Result<config_consistency::ConfigState, CommandError> {
    Ok(config_state_from_external(
        target.read_text_state_anchored(MAX_EXTERNAL_CONFIG_BYTES)?,
    ))
}

pub(crate) fn reject_symlink(path: &Path, label: &str) -> Result<(), CommandError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(CommandError::UnsafeConfigPath(
            format!("{label} is a symlink: {}", path.display()),
        )),
        Ok(_) => Ok(()),
        Err(err) if err.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(err) => Err(err.into()),
    }
}

fn write_config_atomic(
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
    path: &Path,
    content: &str,
) -> Result<(), CommandError> {
    validate_config_write_target(ctx, agent, scope, path)?;
    let parent = path
        .parent()
        .ok_or_else(|| CommandError::UnsafeConfigPath("config path has no parent".to_string()))?;
    let file_name = path
        .file_name()
        .and_then(|name| name.to_str())
        .ok_or_else(|| {
            CommandError::UnsafeConfigPath("config path has no file name".to_string())
        })?;
    let tmp = parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        current_time_ms()
    ));

    let write_result = (|| {
        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut tmp_file = options.open(&tmp)?;
        set_private_file_permissions(&tmp_file)?;
        tmp_file.write_all(content.as_bytes())?;
        tmp_file.sync_all()?;
        drop(tmp_file);

        run_atomic_pre_rename_failure_test_hook(path)?;
        validate_config_write_target(ctx, agent, scope, path)?;
        fs::rename(&tmp, path)?;
        run_atomic_post_rename_test_hook(path)?;
        set_private_path_permissions(path)?;
        validate_config_write_target(ctx, agent, scope, path)?;
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    finish_atomic_write_with_temp_cleanup("config.write", write_result, &tmp)
}

fn finish_atomic_write_with_temp_cleanup(
    operation: &str,
    result: Result<(), CommandError>,
    temporary_path: &Path,
) -> Result<(), CommandError> {
    let Err(error) = result else {
        return Ok(());
    };
    match fs::remove_file(temporary_path) {
        Ok(()) => Err(error),
        Err(cleanup_error) if cleanup_error.kind() == io::ErrorKind::NotFound => Err(error),
        Err(cleanup_error) => Err(CommandError::PartialEffect {
            operation: operation.to_string(),
            state: "outcome_unknown",
            cleanup_required: true,
            detail: format!(
                "atomic write failed ({error}); private temporary-file cleanup failed ({cleanup_error})"
            ),
        }),
    }
}

#[cfg(test)]
thread_local! {
    static ATOMIC_PRE_RENAME_FAILURE_TEST_HOOKS: std::cell::RefCell<Vec<PathBuf>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
fn install_atomic_pre_rename_failure_test_hook(path: PathBuf) {
    ATOMIC_PRE_RENAME_FAILURE_TEST_HOOKS.with(|hooks| hooks.borrow_mut().push(path));
}

#[cfg(test)]
fn run_atomic_pre_rename_failure_test_hook(path: &Path) -> Result<(), CommandError> {
    ATOMIC_PRE_RENAME_FAILURE_TEST_HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        let Some(index) = hooks.iter().position(|candidate| candidate == path) else {
            return Ok(());
        };
        hooks.remove(index);
        Err(CommandError::VerificationFailed)
    })
}

#[cfg(not(test))]
fn run_atomic_pre_rename_failure_test_hook(_path: &Path) -> Result<(), CommandError> {
    Ok(())
}

#[cfg(test)]
struct AtomicPostRenameTestHook {
    path: PathBuf,
    action: Box<dyn FnOnce(&Path) + Send>,
}

#[cfg(test)]
thread_local! {
    static ATOMIC_POST_RENAME_TEST_HOOKS: std::cell::RefCell<Vec<AtomicPostRenameTestHook>> =
        const { std::cell::RefCell::new(Vec::new()) };
}

#[cfg(test)]
fn install_atomic_post_rename_test_hook(
    path: PathBuf,
    action: impl FnOnce(&Path) + Send + 'static,
) {
    ATOMIC_POST_RENAME_TEST_HOOKS.with(|hooks| {
        hooks.borrow_mut().push(AtomicPostRenameTestHook {
            path,
            action: Box::new(action),
        })
    });
}

#[cfg(test)]
fn run_atomic_post_rename_test_hook(path: &Path) -> Result<(), CommandError> {
    let action = ATOMIC_POST_RENAME_TEST_HOOKS.with(|hooks| {
        let mut hooks = hooks.borrow_mut();
        hooks
            .iter()
            .position(|hook| hook.path == path)
            .map(|position| hooks.remove(position).action)
    });
    if let Some(action) = action {
        action(path);
        return Err(CommandError::VerificationFailed);
    }
    Ok(())
}

#[cfg(not(test))]
fn run_atomic_post_rename_test_hook(_path: &Path) -> Result<(), CommandError> {
    Ok(())
}

fn restore_skill_install_target(
    capability: &mut ExternalTargetCapability<'_>,
    target: &Path,
    original: &SkillFileState,
    expected_applied_revision: &str,
) -> Result<(), CommandError> {
    capability.run_before_compensation_hook();
    let current = read_external_skill_state_anchored(capability, target)?;
    if current.revision == original.revision {
        if !original.exists {
            capability.restore_missing_anchored()?;
        }
        return Ok(());
    }
    if current.revision != expected_applied_revision {
        return Err(CommandError::InvalidBatchAction(
            "install target changed to an unowned third state during compensation".to_string(),
        ));
    }
    if original.exists {
        capability.restore_present_anchored(original.content.as_bytes())?;
    } else {
        capability.restore_missing_anchored()?;
    }
    let restored = read_external_skill_state_anchored(capability, target)?;
    if restored.revision != original.revision {
        return Err(CommandError::VerificationFailed);
    }
    Ok(())
}

#[cfg(unix)]
fn set_private_file_permissions(file: &fs::File) -> Result<(), CommandError> {
    use std::os::unix::fs::PermissionsExt;

    file.set_permissions(fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_file_permissions(_file: &fs::File) -> Result<(), CommandError> {
    Ok(())
}

#[cfg(unix)]
fn set_private_path_permissions(path: &Path) -> Result<(), CommandError> {
    use std::os::unix::fs::PermissionsExt;

    fs::set_permissions(path, fs::Permissions::from_mode(0o600))?;
    Ok(())
}

#[cfg(not(unix))]
fn set_private_path_permissions(_path: &Path) -> Result<(), CommandError> {
    Ok(())
}

fn lock_config(
    ctx: &AdapterContext,
    agent: AgentId,
    scope: Scope,
    path: &Path,
) -> Result<fs::File, CommandError> {
    validate_config_write_target(ctx, agent, scope, path)?;
    let lock_path = path.with_extension("lock");
    reject_symlink(&lock_path, "config lock file")?;
    let mut options = fs::OpenOptions::new();
    options.create(true).read(true).write(true).truncate(false);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
    }
    let lock_file = options.open(&lock_path)?;
    set_private_file_permissions(&lock_file)?;
    reject_symlink(&lock_path, "config lock file")?;
    lock_file.lock_exclusive()?;
    Ok(lock_file)
}

fn apply_codex_config_overrides(
    ctx: &AdapterContext,
    instances: &mut [SkillInstance],
) -> Result<(), CommandError> {
    let projection = codex_config_projection(ctx)?;
    for instance in instances.iter_mut() {
        if instance.agent != AgentId::Codex {
            continue;
        }
        if instance.state == SkillState::Disabled {
            instance.enabled = true;
            instance.state = SkillState::Loaded;
        }
        if instance.state == SkillState::Loaded && projection.is_disabled(&instance.path) {
            instance.enabled = false;
            instance.state = SkillState::Disabled;
        }
    }
    Ok(())
}

fn apply_pi_config_overrides(
    ctx: &AdapterContext,
    instances: &mut [SkillInstance],
) -> Result<(), CommandError> {
    let mut settings_by_path = BTreeMap::<PathBuf, String>::new();
    for instance in instances.iter() {
        let config_path = match pi_config_path_for_skill_instance(ctx, instance) {
            Ok(path) => path,
            Err(_) => continue,
        };
        if settings_by_path.contains_key(&config_path) {
            continue;
        }
        let content = match fs::read_to_string(&config_path) {
            Ok(content) => content,
            Err(err) if err.kind() == io::ErrorKind::NotFound => {
                settings_by_path.insert(config_path, "{}".to_string());
                continue;
            }
            Err(err) => return Err(err.into()),
        };
        settings_by_path.insert(config_path, content);
    }

    for instance in instances.iter_mut() {
        let config_path = match pi_config_path_for_skill_instance(ctx, instance) {
            Ok(path) => path,
            Err(_) => continue,
        };
        let enabled = settings_by_path
            .get(&config_path)
            .map_or(Ok(true), |content| {
                pi_skill_enabled_by_settings(content, &config_path, &instance.path, &ctx.user_home)
                    .map_err(|err| CommandError::Adapter(err.message))
            })?;
        if instance.state == SkillState::Disabled && enabled {
            instance.enabled = true;
            instance.state = SkillState::Loaded;
        } else if instance.state == SkillState::Loaded && !enabled {
            instance.enabled = false;
            instance.state = SkillState::Disabled;
        }
    }
    Ok(())
}

fn pi_config_path_for_skill_instance(
    ctx: &AdapterContext,
    instance: &SkillInstance,
) -> Result<PathBuf, CommandError> {
    pi_config_path_for_skill_path(ctx, instance.scope, &instance.path)
}

struct CodexConfigProjection {
    codex_home: PathBuf,
    disabled_paths: BTreeSet<PathBuf>,
    plugin_states: BTreeMap<String, bool>,
}
impl CodexConfigProjection {
    fn is_disabled(&self, skill_path: &Path) -> bool {
        self.disabled_paths.contains(skill_path)
            || codex_plugin_cache_id(&self.codex_home, skill_path).is_some_and(|plugin_id| {
                !codex_plugin_is_effectively_enabled(&plugin_id, &self.plugin_states)
            })
    }
}

fn codex_config_projection(ctx: &AdapterContext) -> Result<CodexConfigProjection, CommandError> {
    let content = match fs::read_to_string(codex_user_config_path(ctx)) {
        Ok(content) => content,
        Err(error) if error.kind() == io::ErrorKind::NotFound => String::new(),
        Err(error) => return Err(error.into()),
    };
    Ok(CodexConfigProjection {
        codex_home: codex_home_dir(ctx),
        disabled_paths: parse_codex_skill_config_entries(&content)
            .into_iter()
            .filter(|block| block.enabled == Some(false))
            .filter_map(|block| block.path.map(PathBuf::from))
            .collect(),
        plugin_states: parse_codex_plugin_states(&content),
    })
}

const REDACTED_SNAPSHOT_PREFIX: &str = "# skills-copilot: snapshot content redacted\n";
const REDACTED_VALUE: &str = "[REDACTED]";

fn redact_snapshot_content(content: &str) -> String {
    if content.is_empty() || is_redacted_snapshot_content(content) {
        return content.to_string();
    }

    if let Ok(mut value) = serde_json::from_str::<serde_json::Value>(content) {
        if redact_json_value(&mut value) {
            let rendered =
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| content.to_string());
            return format!("{REDACTED_SNAPSHOT_PREFIX}{rendered}\n");
        }
        return content.to_string();
    }

    if let Ok(mut value) = json5::from_str::<serde_json::Value>(content) {
        if redact_json_value(&mut value) {
            let rendered =
                serde_json::to_string_pretty(&value).unwrap_or_else(|_| content.to_string());
            return format!("{REDACTED_SNAPSHOT_PREFIX}{rendered}\n");
        }
        return content.to_string();
    }

    let redacted = content
        .lines()
        .map(redact_simple_secret_line)
        .collect::<Vec<_>>()
        .join("\n");
    let redacted = if content.ends_with('\n') {
        format!("{redacted}\n")
    } else {
        redacted
    };
    if redacted == content {
        content.to_string()
    } else {
        format!("{REDACTED_SNAPSHOT_PREFIX}{redacted}")
    }
}

fn is_redacted_snapshot_content(content: &str) -> bool {
    content.starts_with(REDACTED_SNAPSHOT_PREFIX)
}

fn redact_json_value(value: &mut serde_json::Value) -> bool {
    match value {
        serde_json::Value::Object(map) => {
            let mut changed = false;
            for (key, value) in map {
                if is_sensitive_key(key) {
                    if !matches!(
                        value,
                        serde_json::Value::String(redacted) if redacted == REDACTED_VALUE
                    ) {
                        *value = serde_json::Value::String(REDACTED_VALUE.to_string());
                        changed = true;
                    }
                } else {
                    changed |= redact_json_value(value);
                }
            }
            changed
        }
        serde_json::Value::Array(values) => {
            let mut changed = false;
            for value in values {
                changed |= redact_json_value(value);
            }
            changed
        }
        _ => false,
    }
}

fn redact_simple_secret_line(line: &str) -> String {
    let Some((key_part, value_part, separator)) = split_assignment_line(line) else {
        return line.to_string();
    };
    let key = key_part.trim().trim_matches('"').trim_matches('\'');
    if !is_sensitive_key(key) {
        return line.to_string();
    }
    let trailing_comma = value_part.trim_end().ends_with(',');
    let comment = value_part.find('#').map(|idx| &value_part[idx..]);
    let suffix = match (trailing_comma, comment) {
        (true, Some(comment)) => format!(", {comment}"),
        (true, None) => ",".to_string(),
        (false, Some(comment)) => format!(" {comment}"),
        (false, None) => String::new(),
    };
    format!("{key_part}{separator} \"{REDACTED_VALUE}\"{suffix}")
}

fn split_assignment_line(line: &str) -> Option<(&str, &str, &'static str)> {
    if let Some((key, value)) = line.split_once('=') {
        return Some((key, value, "="));
    }
    line.split_once(':').map(|(key, value)| (key, value, ":"))
}

fn is_sensitive_key(key: &str) -> bool {
    let normalized: String = key
        .chars()
        .filter(|ch| ch.is_ascii_alphanumeric())
        .flat_map(char::to_lowercase)
        .collect();
    matches!(
        normalized.as_str(),
        "apikey"
            | "token"
            | "accesstoken"
            | "refreshtoken"
            | "secret"
            | "clientsecret"
            | "password"
            | "passwd"
    ) || normalized.ends_with("token")
        || normalized.ends_with("apikey")
        || normalized.ends_with("secret")
        || normalized.ends_with("password")
}

fn generate_snapshot_id() -> String {
    let nanos = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_nanos())
        .unwrap_or(0);
    format!("snap-{nanos:x}")
}

fn current_time_ms() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|d| d.as_millis() as i64)
        .unwrap_or(0)
}

#[cfg(test)]
mod tests;
