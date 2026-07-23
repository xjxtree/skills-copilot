use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    time::{SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use skills_copilot_catalog::{
    Catalog, CatalogCommitError, CatalogError, ConfigSnapshotRecord, ConflictGroupRecord,
    FindingTriageRecord, RuleFindingRecord, RuleTuningRecord, SkillDetailRecord, SkillEventRecord,
    SkillRecord,
};
use skills_copilot_commands::{
    analyze_catalog, apply_current_config_overrides_to_skill_detail,
    apply_current_config_overrides_to_skill_records, apply_install_with_manager,
    apply_local_archive_import, apply_local_archive_update, apply_local_create_with_manager,
    apply_remove_with_manager, apply_skill_toggles_guarded, apply_update_with_manager,
    commit_prepared_claude_settings_save, delete_local_skill_with_manager, get_skill,
    install_skill_from_tool_global_guarded, list_adapter_capabilities, list_adapter_diagnostics,
    list_agent_config_snapshot_page_snapshot, list_agent_config_snapshots,
    list_conflicts_for_context, list_finding_triage, list_findings,
    list_installed_skills_from_projection, list_rule_tuning, list_skill_event_page_snapshot,
    list_skill_events, list_skill_management_tools, list_snapshots,
    lock_or_create_app_mutations_with_parents, prepare_claude_settings_save,
    preview_claude_settings_save, preview_install_with_manager, preview_local_archive_import,
    preview_local_archive_update, preview_local_create_with_manager, preview_remove_with_manager,
    preview_remove_with_manager_guarded, preview_script_execution, preview_skill_toggles,
    preview_snapshot_rollback_with_context, preview_update_with_manager, read_agent_config,
    read_claude_settings, reject_non_applicable_confirmation, rollback_snapshot,
    scan_all_catalog_report, scan_claude_catalog_report, skill_health_summary,
    user_visible_rule_findings, validate_local_archive_import_confirmation,
    validate_local_archive_update_confirmation, validate_local_delete_confirmation,
    validate_skill_install_confirmation, validate_skill_manager_confirmation,
    validate_skill_toggle_confirmation, validate_snapshot_rollback_confirmation,
    ActionConfirmation, ActionPreviewBinding, ActionReadbackRecord, AdapterCapabilityRecord,
    AdapterDiagnosticsRecord, AgentCatalogScanPathAlias, AgentCatalogScanReport,
    BatchToggleApplyRecord, BatchTogglePreviewRecord, CommandError, ConfigDocumentRecord,
    ConfigSaveApplyRecord, ConfigSavePreviewRecord, CrossAgentAnalysisRecord,
    ScriptExecutionPreviewRecord, ScriptExecutionRequest, SkillHealthSummary,
    SkillInstallPreviewRecord, SkillManagerDeleteLocalParams, SkillManagerInstallParams,
    SkillManagerListInstalledParams, SkillManagerLocalArchiveImportParams,
    SkillManagerLocalArchiveUpdateParams, SkillManagerLocalCreateParams, SkillManagerRemoveParams,
    SkillManagerSearchApplyParams, SkillManagerSearchParams, SkillManagerUpdateParams,
    SnapshotRollbackApplyRecord, SnapshotRollbackPreviewRecord, SCRIPT_EXECUTION_DISABLED_REASON,
};
use skills_copilot_commands::{
    apply_search_skills_with_manager, preview_search_skills_with_manager,
};
use skills_copilot_core::{
    AdapterContext, AgentId, ListIncompleteReason, ListPageMetadata, ListSourceCompleteness, Scope,
};
#[cfg(test)]
use skills_copilot_core::{AdapterRoot, RootSource};
use thiserror::Error;

mod project_context;
mod protocol;
mod provider;
mod service_app_search;
mod service_host;
mod service_keyset_cursor;
mod service_llm;
mod service_llm_prompt_helpers;
mod service_local_session_io;
mod service_local_sessions;
mod service_observability_helpers;
mod service_provider_actions;
mod service_support_helpers;

use project_context::{
    clear_project_context, clear_recent_project_contexts, context_from_paths,
    effective_project_context_revision, load_project_context_state, preview_clear_project_context,
    preview_clear_recent_project_contexts, preview_remove_recent_project_context,
    preview_set_project_context, project_context_summary, remove_recent_project_context,
    set_project_context, stored_active_adapter_paths, validate_project_context_for_response,
    ProjectContext, ProjectContextActionPreview, ProjectContextApplyResult,
    ProjectContextConfirmationParams, ProjectContextIDApplyParams, ProjectContextIDPreviewParams,
    ProjectContextParams, ProjectContextRevisionParams, ProjectContextSetApplyParams,
    ProjectContextSetPreviewParams, ProjectContextState, ProjectContextSummary,
};
pub use protocol::{
    ServiceErrorDetails, ServiceErrorRecord, ServiceRequest, ServiceResponse, DEFAULT_BUNDLE_ID,
    LEGACY_BUNDLE_ID, SERVICE_PROTOCOL_VERSION, SUPPORTED_METHODS,
};
use provider::{
    default_monthly_budget_usd, default_token_limit, estimate_prompt_cost_usd,
    list_provider_profiles, provider_call_metadata_path, provider_profiles_path,
    send_provider_prompt, DeleteProviderProfileParams, ListProviderProfilesResult,
    ProviderCallMetadata, ProviderError, ProviderProfileRecord, SaveProviderProfileParams,
    SendProviderPromptParams, TestProviderConnectionParams,
};
pub(crate) use service_llm_prompt_helpers::*;
pub(crate) use service_observability_helpers::*;
pub use service_support_helpers::handle_request_json;
pub(crate) use service_support_helpers::*;

#[derive(Debug, Clone, Serialize)]
pub struct ServiceStatus {
    pub protocol_version: u32,
    pub version: &'static str,
    pub app_data_dir: String,
    pub catalog_path: String,
    pub user_home: String,
    pub supported_methods: Vec<&'static str>,
    pub refresh: RefreshStatus,
    pub project_context: ProjectContextSummary,
    pub adapter_capabilities: Vec<AdapterCapabilityRecord>,
    pub adapter_diagnostics: Vec<AdapterDiagnosticsRecord>,
    pub llm: LlmStatus,
    pub script_execution: ScriptExecutionStatus,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppVersion {
    pub protocol_version: u32,
    pub version: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppStateSnapshot {
    pub status: ServiceStatus,
    pub skills: Vec<SkillRecord>,
    pub findings: Vec<RuleFindingRecord>,
    pub conflicts: Vec<ConflictGroupRecord>,
    pub analysis: CrossAgentAnalysisRecord,
    pub health: SkillHealthSummary,
    pub snapshots: Vec<ConfigSnapshotRecord>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AppSearchParams {
    pub query: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub limit_per_kind: Option<usize>,
    #[serde(default, alias = "authorized_dirs", alias = "authorized_paths")]
    pub authorized_roots: Vec<String>,
    #[serde(default)]
    pub auto_discover: Option<bool>,
    #[serde(default)]
    pub project_root: Option<String>,
    #[serde(default)]
    pub current_cwd: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppSearchResult {
    pub generated_by: &'static str,
    pub query: String,
    pub count: usize,
    pub total_matched_count: usize,
    pub limit_per_kind: usize,
    pub items: Vec<AppSearchItem>,
    pub read_only: bool,
    pub provider_request_sent: bool,
    pub skill_files_mutated: bool,
    pub agent_config_mutated: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AppSearchItem {
    pub id: String,
    pub kind: String,
    pub target_id: String,
    pub title: String,
    pub subtitle: String,
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub skill: Option<SkillRecord>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub session: Option<LocalSessionPreviewRow>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub config_snapshot: Option<ConfigSnapshotRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub scanned_count: usize,
    pub skills: Vec<SkillRecord>,
    pub activity: RefreshActivity,
    pub accepted_context_revision: String,
    pub catalog_scan_revision: String,
    pub readback: CatalogScanReadback,
}

#[derive(Debug, Clone, Serialize)]
pub struct CatalogScanReadback {
    pub accepted_context_revision: String,
    pub catalog_scan_revision: String,
    pub verified: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshStatus {
    pub scan_progress: &'static str,
    pub watcher_state: &'static str,
    pub watcher_detail: &'static str,
    pub recovery_actions: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshActivity {
    pub operation: &'static str,
    pub status: &'static str,
    pub started_at: i64,
    pub finished_at: i64,
    pub scanned_count: usize,
    pub skill_count: usize,
    pub finding_count: usize,
    pub conflict_count: usize,
    pub snapshot_count: usize,
    pub roots: Vec<String>,
    pub log_entries: Vec<RefreshLogEntry>,
    pub recovery_actions: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub agent_summaries: Option<Vec<AgentRefreshSummary>>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RefreshLogEntry {
    pub level: &'static str,
    pub message: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentRefreshSummary {
    pub agent: String,
    pub display_label: String,
    pub status: &'static str,
    pub scanned_count: usize,
    pub catalog_count: usize,
    pub broken_count: usize,
    pub roots_considered: Vec<String>,
    pub roots_scanned: Vec<String>,
    pub roots_partial: Vec<String>,
    pub roots_skipped: Vec<String>,
    pub scan_issues: Vec<AgentRefreshScanIssue>,
    pub config_detected: bool,
    pub config_paths: Vec<String>,
    pub writable_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writable_reason: Option<String>,
    pub read_only_reason: String,
    pub blockers: Vec<String>,
    pub recovery_actions: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct AgentRefreshScanIssue {
    pub kind: &'static str,
    pub path: String,
    pub detail: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmStatus {
    pub enabled: bool,
    pub configured: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub reason: String,
    pub single_request_token_limit: u32,
    pub monthly_budget_usd: f64,
    pub credentials_storage: String,
    pub credential_persistence_allowed: bool,
    pub provider_profile_count: usize,
    pub default_profile_id: Option<String>,
    pub profiles_path: String,
    pub call_metadata_path: String,
    pub raw_prompt_persistence_allowed: bool,
    pub raw_response_persistence_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ScriptExecutionStatus {
    pub enabled: bool,
    pub default_enabled: bool,
    pub reason: String,
    pub audit_scope: String,
    pub audit_path: String,
    pub llm_initiation_allowed: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LocalPreviewRedactionSummary {
    pub status: String,
    pub redacted_value_count: usize,
    pub redacted_fields: Vec<String>,
    pub placeholders: Vec<String>,
    pub raw_trace_persisted: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_secret_returned: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LocalPreviewSafetyFlags {
    pub read_only: bool,
    pub app_local_only: bool,
    pub provider_request_sent: bool,
    pub write_back_allowed: bool,
    pub write_actions_available: bool,
    pub skill_files_mutated: bool,
    pub agent_config_mutated: bool,
    pub script_execution_allowed: bool,
    pub execution_actions_available: bool,
    pub config_mutation_allowed: bool,
    pub snapshot_created: bool,
    pub triage_mutation_allowed: bool,
    pub credential_accessed: bool,
    pub raw_secret_returned: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
    pub cloud_sync_performed: bool,
    pub telemetry_emitted: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LocalSessionPreviewParams {
    #[serde(default, alias = "authorized_dirs", alias = "authorized_paths")]
    pub authorized_roots: Vec<String>,
    #[serde(default)]
    pub auto_discover: Option<bool>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
    #[serde(default)]
    pub project_root: Option<String>,
    #[serde(default)]
    pub current_cwd: Option<String>,
    #[serde(default)]
    pub include_content_items: Option<bool>,
    #[serde(default)]
    pub session_id: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub offset: Option<usize>,
    #[serde(default)]
    pub paging_mode: Option<String>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub source_revision: Option<String>,
    #[serde(default)]
    pub sort: Option<String>,
    #[serde(default)]
    pub direction: Option<String>,
    #[serde(default)]
    pub max_files: Option<usize>,
    #[serde(default)]
    pub max_excerpt_chars: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSessionPreviewRoot {
    pub root: String,
    pub status: String,
    pub candidate_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocker: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSessionPreviewRow {
    pub id: String,
    pub title: String,
    pub source_kind: String,
    pub scope: String,
    pub agent: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub project_root: Option<String>,
    pub redacted_path: String,
    pub modified_at: Option<i64>,
    pub started_at: Option<i64>,
    pub ended_at: Option<i64>,
    pub excerpt: String,
    pub excerpt_char_count: usize,
    pub user_message_count: usize,
    pub total_message_count: usize,
    pub tool_call_count: usize,
    pub skill_call_count: usize,
    pub content_hash: String,
    pub evidence_refs: Vec<String>,
    pub content_included: bool,
    pub content_items: Vec<LocalSessionContentItem>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSessionContentItem {
    pub id: String,
    pub kind: String,
    pub title: String,
    pub text: String,
    pub char_count: usize,
    pub timestamp: Option<i64>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LocalSessionMessagePageParams {
    #[serde(default, alias = "authorized_dirs", alias = "authorized_paths")]
    pub authorized_roots: Vec<String>,
    #[serde(default)]
    pub auto_discover: Option<bool>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub project_root: Option<String>,
    #[serde(default)]
    pub current_cwd: Option<String>,
    pub session_id: String,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub source_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSessionMessagePageResult {
    pub generated_by: &'static str,
    pub session_id: String,
    pub content_items: Vec<LocalSessionContentItem>,
    pub returned_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub total_count: Option<usize>,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    pub source_revision: String,
    pub source_completeness: ListSourceCompleteness,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incomplete_reason: Option<ListIncompleteReason>,
    pub scanned_bytes: u64,
    pub scanned_through_bytes: u64,
    pub snapshot_bytes: u64,
    pub redaction_summary: LocalPreviewRedactionSummary,
    pub safety_flags: LocalPreviewSafetyFlags,
    pub read_only: bool,
    pub provider_request_sent: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSessionSkillUsageRow {
    pub skill_id: String,
    pub skill_name: String,
    pub agent: String,
    pub call_count: usize,
    pub session_count: usize,
    pub latest_modified_at: Option<i64>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSessionPreviewResult {
    pub generated_by: &'static str,
    pub authorized: bool,
    pub authorization_required: bool,
    pub roots: Vec<LocalSessionPreviewRoot>,
    pub count: usize,
    pub total_candidate_count: usize,
    pub total_matched_count: usize,
    pub offset: usize,
    pub limit: usize,
    pub has_more: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_offset: Option<usize>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub source_revision: Option<String>,
    pub source_completeness: ListSourceCompleteness,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub incomplete_reason: Option<ListIncompleteReason>,
    pub candidate_set_truncated: bool,
    pub user_message_count: usize,
    pub total_message_count: usize,
    pub tool_call_count: usize,
    pub skill_call_count: usize,
    pub skill_usage_rows: Vec<LocalSessionSkillUsageRow>,
    pub session_rows: Vec<LocalSessionPreviewRow>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub redaction_summary: LocalPreviewRedactionSummary,
    pub safety_flags: LocalPreviewSafetyFlags,
    pub read_only: bool,
    pub provider_request_sent: bool,
    pub skill_files_mutated: bool,
    pub agent_config_mutated: bool,
    pub snapshot_created: bool,
    pub triage_mutated: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmPrepareActionParams {
    pub kind: LlmActionKind,
    #[serde(default, alias = "instance_id")]
    pub skill_instance_id: Option<String>,
    #[serde(default)]
    pub user_intent: Option<String>,
}

#[derive(Debug, Clone, Deserialize, Serialize)]
pub struct LlmPreviewPromptParams {
    #[serde(alias = "kind")]
    pub action: LlmPromptActionKind,
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub app_language: Option<String>,
    #[serde(default, alias = "instance_id")]
    pub skill_instance_id: Option<String>,
    #[serde(default)]
    pub instance_ids: Vec<String>,
    #[serde(default)]
    pub agents: Vec<String>,
    #[serde(default)]
    pub user_intent: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfirmPromptAndSendParams {
    pub action_confirmation: ActionConfirmation,
    pub request: LlmPreviewPromptParams,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LlmPromptRunListParams {
    #[serde(default, alias = "instance_id")]
    pub skill_instance_id: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub request_kind: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmPromptActionKind {
    Analyze,
    Recommend,
    ExplainConflict,
    DraftFrontmatter,
    TaskCockpit,
}

impl LlmPromptActionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Analyze => "analyze",
            Self::Recommend => "recommend",
            Self::ExplainConflict => "explain_conflict",
            Self::DraftFrontmatter => "draft_frontmatter",
            Self::TaskCockpit => "task_cockpit",
        }
    }
}

fn llm_output_language_instruction(app_language: Option<&str>) -> String {
    let raw_language = app_language
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("en");
    let normalized = raw_language.to_ascii_lowercase();
    let (language_name, language_code) = match normalized.as_str() {
        "zh" | "zh-hans" | "zh-cn" | "cn" => ("Simplified Chinese", "zh-Hans"),
        "en" | "en-us" | "en-gb" => ("English", "en"),
        _ => ("English", "en"),
    };
    format!(
        "Output language: {language_name} ({language_code}). Write all prose, Markdown headings, evidence notes, uncertainty, and safe next steps in {language_name}. Use narrow Markdown that reads well in a macOS detail pane: prefer short sections and bullets. Do not use Markdown tables. Do not wrap the answer in fenced code blocks. Keep skill names, agent names, rule IDs, paths, code, commands, quoted evidence, and placeholders unchanged."
    )
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmActionKind {
    Analyze,
    Recommend,
    ExplainConflict,
    DraftFrontmatter,
}

impl LlmActionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Analyze => "analyze",
            Self::Recommend => "recommend",
            Self::ExplainConflict => "explain_conflict",
            Self::DraftFrontmatter => "draft_frontmatter",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmPrepareActionResult {
    pub action: &'static str,
    pub allowed: bool,
    pub reason: String,
    pub disabled_reason: Option<String>,
    pub requires_confirmation: bool,
    pub write_back_allowed: bool,
    pub draft_requires_user_copy: bool,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub estimated_input_tokens: u32,
    pub estimated_output_tokens: u32,
    pub estimated_total_tokens: u32,
    pub estimated_cost_usd: f64,
    pub single_request_token_limit: u32,
    pub monthly_budget_usd: f64,
    pub credentials_storage: String,
    pub credential_persistence_allowed: bool,
    pub prompt_scope: Vec<String>,
    pub privacy_notes: Vec<String>,
    pub confirmation: LlmConfirmationRequirement,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmPreviewPromptResult {
    pub preview_id: String,
    pub status: String,
    pub allowed: bool,
    pub reason: String,
    pub request_kind: &'static str,
    #[serde(flatten)]
    pub binding: ActionPreviewBinding,
    pub profile_id: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub endpoint: Option<String>,
    pub destination_host: Option<String>,
    pub prompt_scope: Vec<String>,
    pub included_fields: Vec<String>,
    pub excluded_fields: Vec<String>,
    pub redaction: LlmPromptRedactionSummary,
    pub prompt_preview: String,
    pub estimated_input_tokens: u32,
    pub estimated_output_tokens: u32,
    pub estimated_total_tokens: u32,
    pub estimated_cost_usd: f64,
    pub single_request_token_limit: u32,
    pub monthly_budget_usd: f64,
    pub requires_confirmation: bool,
    pub confirmation: LlmConfirmationRequirement,
    pub write_back_allowed: bool,
    pub draft_requires_user_copy: bool,
    pub provider_request_sent: bool,
    pub raw_secret_returned: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmPromptRedactionSummary {
    pub status: String,
    pub redacted_value_count: usize,
    pub redacted_fields: Vec<String>,
    pub placeholders: Vec<&'static str>,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_secret_returned: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmConfirmPromptAndSendResult {
    pub preview_id: String,
    pub confirmation_id: String,
    pub status: String,
    pub request_kind: &'static str,
    pub profile_id: String,
    pub provider: String,
    pub model: String,
    pub destination_host: String,
    pub provider_request_sent: bool,
    pub credential_accessed: bool,
    pub draft_output: Option<String>,
    pub draft_requires_user_copy: bool,
    pub write_back_allowed: bool,
    pub script_execution_allowed: bool,
    pub config_mutation_allowed: bool,
    pub snapshot_created: bool,
    pub triage_mutation_allowed: bool,
    pub audit: ProviderCallMetadata,
    pub readback: Option<ActionReadbackRecord>,
    pub partial_outcome: Option<LlmPromptPartialOutcome>,
    pub raw_secret_returned: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmPromptPartialOutcome {
    pub remote_effect: String,
    pub local_record: String,
    pub recovery: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPromptRunRedactionSummary {
    pub status: String,
    pub redacted_value_count: usize,
    pub redacted_fields: Vec<String>,
    pub placeholders: Vec<String>,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
    pub raw_secret_returned: bool,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct LlmPromptRunSafetyFlags {
    pub app_local_only: bool,
    pub provider_request_sent: bool,
    pub credential_accessed: bool,
    pub draft_copy_only: bool,
    pub write_back_allowed: bool,
    pub write_actions_available: bool,
    pub skill_files_mutated: bool,
    pub agent_config_mutated: bool,
    pub script_execution_allowed: bool,
    pub execution_actions_available: bool,
    pub config_mutation_allowed: bool,
    pub snapshot_created: bool,
    pub triage_mutation_allowed: bool,
    pub raw_secret_returned: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
    pub cloud_sync_performed: bool,
    pub telemetry_emitted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmPromptRunRecord {
    pub id: String,
    pub preview_id: String,
    pub confirmation_id: String,
    pub action: String,
    pub request_kind: String,
    pub analysis_kind: Option<String>,
    pub scope: Option<String>,
    pub instance_id: Option<String>,
    pub instance_ids: Vec<String>,
    pub definition_id: Option<String>,
    pub agent: Option<String>,
    pub task: Option<String>,
    pub profile_id: String,
    pub provider: String,
    pub model: String,
    pub destination_host: String,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: u64,
    pub estimated_input_tokens: u32,
    pub estimated_output_tokens: u32,
    pub estimated_total_tokens: u32,
    pub estimated_cost_usd: f64,
    pub draft_output: Option<String>,
    pub draft_requires_user_copy: bool,
    pub provider_request_sent: bool,
    pub credential_accessed: bool,
    pub raw_secret_returned: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub redaction_summary: LlmPromptRunRedactionSummary,
    pub created_at: i64,
    pub completed_at: i64,
    pub safety_flags: LlmPromptRunSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmPromptRunListResult {
    pub generated_by: &'static str,
    pub count: usize,
    pub total_count: usize,
    pub returned_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    pub truncated: bool,
    pub runs: Vec<LlmPromptRunRecord>,
    pub app_local_only: bool,
    pub runs_file: &'static str,
    pub provider_request_sent: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_secret_returned: bool,
    pub safety_flags: LlmPromptRunSafetyFlags,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LlmProviderObservabilityParams {
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub window_days: Option<i64>,
    #[serde(default)]
    pub start_at: Option<i64>,
    #[serde(default)]
    pub end_at: Option<i64>,
    #[serde(default)]
    pub include_history: Option<bool>,
    #[serde(default)]
    pub include_budget_hints: Option<bool>,
    #[serde(default)]
    pub include_retention_recommendations: Option<bool>,
    #[serde(default)]
    pub include_evidence: Option<bool>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmProviderObservabilityResult {
    pub generated_by: &'static str,
    pub status: String,
    pub filters: LlmProviderObservabilityAppliedFilters,
    pub summary: LlmProviderObservabilitySummary,
    pub call_rows: Vec<LlmProviderObservabilityCallRow>,
    pub history_rows: Vec<LlmProviderObservabilityHistoryRow>,
    pub grouping_rows: Vec<LlmProviderObservabilityGroupingRow>,
    pub model_task_history_rows: Vec<ModelTaskMatchEvidenceRow>,
    pub status_rows: Vec<LlmProviderObservabilityStatusRow>,
    pub budget_usage_hints: Vec<LlmProviderObservabilityBudgetUsageHint>,
    pub retention_recommendations: Vec<LlmProviderObservabilityRetentionRecommendationRow>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<LlmProviderObservabilityEvidenceReference>,
    pub prompt_metadata: LlmProviderObservabilityPromptMetadata,
    pub safety_flags: LlmProviderObservabilitySafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmProviderObservabilityAppliedFilters {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub profile_id: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub provider: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub model: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub window_days: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub start_at: Option<i64>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub end_at: Option<i64>,
    pub limit: usize,
    pub include_history: bool,
    pub include_budget_hints: bool,
    pub include_retention_recommendations: bool,
    pub include_evidence: bool,
    pub aggregation_uses_full_range: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmProviderObservabilitySummary {
    pub total_prompt_run_count: usize,
    pub total_call_metadata_count: usize,
    pub returned_prompt_run_count: usize,
    pub returned_call_row_count: usize,
    pub provider_profile_count: usize,
    pub enabled_profile_count: usize,
    pub grouping_count: usize,
    pub observed_provider_request_row_count: usize,
    pub observed_credential_access_row_count: usize,
    pub succeeded_count: usize,
    pub failed_count: usize,
    pub estimated_input_tokens: u64,
    pub estimated_output_tokens: u64,
    pub estimated_total_tokens: u64,
    pub estimated_cost_usd: f64,
    pub latest_activity_at: Option<i64>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmProviderObservabilityCallRow {
    pub id: String,
    pub source: &'static str,
    pub timestamp: i64,
    pub action_type: String,
    pub profile_id: String,
    pub provider: String,
    pub model: String,
    pub destination_host: String,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: u128,
    pub estimated_input_tokens: u32,
    pub estimated_output_tokens: u32,
    pub estimated_total_tokens: u32,
    pub estimated_cost_usd: f64,
    pub recorded_provider_request_sent: bool,
    pub recorded_credential_accessed: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub redaction_status: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmProviderObservabilityHistoryRow {
    pub id: String,
    pub source: &'static str,
    pub prompt_run_id: String,
    pub created_at: i64,
    pub completed_at: i64,
    pub action: String,
    pub request_kind: String,
    pub analysis_kind: Option<String>,
    pub scope: Option<String>,
    pub instance_id: Option<String>,
    pub instance_ids: Vec<String>,
    pub definition_id: Option<String>,
    pub agent: Option<String>,
    pub task: Option<String>,
    pub profile_id: String,
    pub provider: String,
    pub model: String,
    pub destination_host: String,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: u64,
    pub estimated_input_tokens: u32,
    pub estimated_output_tokens: u32,
    pub estimated_total_tokens: u32,
    pub estimated_cost_usd: f64,
    pub draft_output_available: bool,
    pub draft_requires_user_copy: bool,
    pub recorded_provider_request_sent: bool,
    pub recorded_credential_accessed: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub redaction_status: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmProviderObservabilityGroupingRow {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub destination_host: String,
    pub profile_ids: Vec<String>,
    pub prompt_run_count: usize,
    pub call_metadata_count: usize,
    pub recorded_provider_request_count: usize,
    pub recorded_credential_access_count: usize,
    pub succeeded_count: usize,
    pub failed_count: usize,
    pub estimated_total_tokens: u64,
    pub estimated_cost_usd: f64,
    pub latest_activity_at: Option<i64>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmProviderObservabilityStatusRow {
    pub id: String,
    pub source: String,
    pub status: String,
    pub severity: &'static str,
    pub message: String,
    pub count: usize,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmProviderObservabilityBudgetUsageHint {
    pub id: String,
    pub profile_id: String,
    pub provider: String,
    pub model: String,
    pub destination_host: String,
    pub enabled: bool,
    pub single_request_token_limit: u32,
    pub monthly_budget_usd: f64,
    pub observed_prompt_run_count: usize,
    pub observed_call_metadata_count: usize,
    pub observed_estimated_total_tokens: u64,
    pub observed_estimated_cost_usd: f64,
    pub budget_state: String,
    pub reason: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmProviderObservabilityRetentionRecommendationRow {
    pub id: String,
    pub source_file: &'static str,
    pub current_record_count: usize,
    pub recommendation: String,
    pub cleanup_action_available: bool,
    pub write_action_available: bool,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmProviderObservabilityEvidenceReference {
    pub id: String,
    pub kind: &'static str,
    pub label: String,
    pub source: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmProviderObservabilityPromptMetadata {
    pub available: bool,
    pub preview_method: &'static str,
    pub confirm_method: &'static str,
    pub provider_request_sent: bool,
    pub copy_only: bool,
    pub note: String,
}

#[derive(Debug, Clone, Copy, Serialize)]
pub struct LlmProviderObservabilitySafetyFlags {
    pub read_only: bool,
    pub app_local_only: bool,
    pub provider_request_sent: bool,
    pub credential_accessed: bool,
    pub draft_copy_only: bool,
    pub write_back_allowed: bool,
    pub write_actions_available: bool,
    pub skill_files_mutated: bool,
    pub agent_config_mutated: bool,
    pub script_execution_allowed: bool,
    pub execution_actions_available: bool,
    pub config_mutation_allowed: bool,
    pub snapshot_created: bool,
    pub triage_mutation_allowed: bool,
    pub raw_secret_returned: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
    pub unredacted_paths_returned: bool,
    pub cloud_sync_performed: bool,
    pub telemetry_emitted: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListProviderActivityParams {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default)]
    pub window_days: Option<i64>,
    #[serde(default)]
    pub start_at: Option<i64>,
    #[serde(default)]
    pub end_at: Option<i64>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub source_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderActivityRow {
    pub id: String,
    pub kind: String,
    pub timestamp: i64,
    pub title: String,
    pub subtitle: String,
    pub status: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderActivityPageResult {
    pub generated_by: &'static str,
    pub rows: Vec<ProviderActivityRow>,
    pub source_revision: String,
    #[serde(flatten)]
    pub page: ListPageMetadata,
    pub safety_flags: LlmProviderObservabilitySafetyFlags,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ModelTaskMatchListParams {
    #[serde(default)]
    pub provider: Option<String>,
    #[serde(default)]
    pub model: Option<String>,
    #[serde(default)]
    pub task_kind: Option<String>,
    #[serde(default)]
    pub match_status: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelTaskMatchRecordParams {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    pub task: String,
    #[serde(default)]
    pub task_kind: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub profile_id: Option<String>,
    #[serde(default)]
    pub provider: Option<String>,
    pub model: String,
    #[serde(default)]
    pub destination_host: Option<String>,
    #[serde(default)]
    pub match_status: Option<String>,
    #[serde(default)]
    pub confidence_score: Option<u8>,
    #[serde(default)]
    pub latency_ms: Option<u64>,
    #[serde(default)]
    pub estimated_total_tokens: Option<u32>,
    #[serde(default)]
    pub estimated_cost_usd: Option<f64>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub prompt_run_ids: Vec<String>,
    #[serde(default)]
    pub benchmark_ids: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub gap_notes: Vec<String>,
    #[serde(default)]
    pub blocker_notes: Vec<String>,
    #[serde(default)]
    pub outcome_notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ModelTaskMatchDeleteParams {
    pub id: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ModelTaskMatchRecord {
    pub id: String,
    pub title: String,
    pub task: String,
    pub task_kind: String,
    pub agent: Option<String>,
    pub profile_id: Option<String>,
    pub provider: String,
    pub model: String,
    pub destination_host: Option<String>,
    pub match_status: String,
    pub confidence_score: Option<u8>,
    pub latency_ms: Option<u64>,
    pub estimated_total_tokens: Option<u32>,
    pub estimated_cost_usd: Option<f64>,
    pub source_kind: String,
    pub prompt_run_ids: Vec<String>,
    pub benchmark_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub outcome_notes: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
    pub redaction_summary: LlmPromptRunRedactionSummary,
    pub safety_flags: ModelTaskMatchSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelTaskMatchListResult {
    pub generated_by: &'static str,
    pub status: String,
    pub total_record_count: usize,
    pub returned_record_count: usize,
    pub total_evidence_count: usize,
    pub returned_evidence_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub limit: Option<usize>,
    pub truncated: bool,
    pub summary: ModelTaskMatchSummary,
    pub records: Vec<ModelTaskMatchRecord>,
    pub model_rows: Vec<ModelTaskMatchModelRow>,
    pub task_rows: Vec<ModelTaskMatchTaskRow>,
    pub recent_evidence_rows: Vec<ModelTaskMatchEvidenceRow>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<LlmProviderObservabilityEvidenceReference>,
    pub app_local_only: bool,
    pub history_file: &'static str,
    pub provider_request_sent: bool,
    pub credential_accessed: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
    pub safety_flags: ModelTaskMatchSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelTaskMatchRecordResult {
    pub generated_by: &'static str,
    pub record: ModelTaskMatchRecord,
    pub count: usize,
    pub app_local_only: bool,
    pub history_file: &'static str,
    pub provider_request_sent: bool,
    pub skill_files_mutated: bool,
    pub agent_config_mutated: bool,
    pub snapshot_created: bool,
    pub triage_mutated: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelTaskMatchDeleteResult {
    pub record_id: String,
    pub deleted: bool,
    pub remaining_count: usize,
    pub app_local_only: bool,
    pub provider_request_sent: bool,
    pub skill_files_mutated: bool,
    pub agent_config_mutated: bool,
    pub snapshot_created: bool,
    pub triage_mutated: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelTaskMatchSummary {
    pub stored_record_count: usize,
    pub prompt_run_count: usize,
    pub returned_record_count: usize,
    pub returned_prompt_run_count: usize,
    pub model_count: usize,
    pub task_kind_count: usize,
    pub fit_count: usize,
    pub partial_fit_count: usize,
    pub mismatch_count: usize,
    pub unknown_count: usize,
    pub estimated_total_tokens: u64,
    pub estimated_cost_usd: f64,
    pub latest_activity_at: Option<i64>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelTaskMatchModelRow {
    pub id: String,
    pub provider: String,
    pub model: String,
    pub destination_host: Option<String>,
    pub stored_record_count: usize,
    pub prompt_run_count: usize,
    pub fit_count: usize,
    pub partial_fit_count: usize,
    pub mismatch_count: usize,
    pub unknown_count: usize,
    pub estimated_total_tokens: u64,
    pub estimated_cost_usd: f64,
    pub latest_activity_at: Option<i64>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelTaskMatchTaskRow {
    pub id: String,
    pub task_kind: String,
    pub status: String,
    pub stored_record_count: usize,
    pub prompt_run_count: usize,
    pub fit_count: usize,
    pub partial_fit_count: usize,
    pub mismatch_count: usize,
    pub unknown_count: usize,
    pub estimated_total_tokens: u64,
    pub estimated_cost_usd: f64,
    pub latest_activity_at: Option<i64>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ModelTaskMatchEvidenceRow {
    pub id: String,
    pub source: String,
    pub source_kind: String,
    pub title: String,
    pub task: Option<String>,
    pub task_kind: String,
    pub agent: Option<String>,
    pub provider: String,
    pub model: String,
    pub destination_host: Option<String>,
    pub match_status: String,
    pub confidence_score: Option<u8>,
    pub status: String,
    pub created_at: i64,
    pub updated_at: Option<i64>,
    pub latency_ms: Option<u64>,
    pub estimated_total_tokens: u32,
    pub estimated_cost_usd: f64,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub outcome_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub redaction_status: String,
    pub safety_flags: ModelTaskMatchSafetyFlags,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct ModelTaskMatchSafetyFlags {
    pub read_only: bool,
    pub app_local_only: bool,
    pub provider_request_sent: bool,
    pub credential_accessed: bool,
    pub draft_copy_only: bool,
    pub write_back_allowed: bool,
    pub write_actions_available: bool,
    pub skill_files_mutated: bool,
    pub agent_config_mutated: bool,
    pub script_execution_allowed: bool,
    pub execution_actions_available: bool,
    pub config_mutation_allowed: bool,
    pub snapshot_created: bool,
    pub triage_mutation_allowed: bool,
    pub raw_secret_returned: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
    pub unredacted_paths_returned: bool,
    pub cloud_sync_performed: bool,
    pub telemetry_emitted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmConfirmationRequirement {
    pub required: bool,
    pub message: String,
    pub display_fields: Vec<&'static str>,
}

#[derive(Debug, Clone, Copy)]
struct ScanActivityCounts {
    scanned_count: usize,
    skill_count: usize,
    finding_count: usize,
    conflict_count: usize,
    snapshot_count: usize,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CatalogScanParams {
    #[serde(default)]
    pub explicit_refresh: bool,
    #[serde(default)]
    pub expected_context_revision: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct GetSkillParams {
    pub instance_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListSkillEventsParams {
    pub instance_id: String,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListSkillEventsPageParams {
    pub instance_id: String,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub source_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillEventPageResult {
    pub records: Vec<SkillEventRecord>,
    pub source_revision: String,
    #[serde(flatten)]
    pub page: ListPageMetadata,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetFindingTriageParams {
    pub triage_key: String,
    pub status: String,
    #[serde(default)]
    pub note: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ClearFindingTriageParams {
    pub triage_key: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RuleTuningScopeParams {
    pub rule_id: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetSeverityOverrideParams {
    pub rule_id: String,
    pub severity: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SetSuppressionParams {
    pub rule_id: String,
    pub reason: String,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ToggleSkillParams {
    pub instance_id: String,
    pub on: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchPreviewSkillTogglesParams {
    pub instance_ids: Vec<String>,
    #[serde(alias = "on", alias = "enabled")]
    pub target_enabled: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct BatchApplySkillTogglesParams {
    pub instance_ids: Vec<String>,
    #[serde(alias = "on", alias = "enabled")]
    pub target_enabled: bool,
    pub confirmation: ActionConfirmation,
}

#[derive(Debug, Clone, Deserialize)]
pub struct InstallSkillParams {
    pub instance_id: String,
    pub target_agent: String,
    pub target_scope: String,
    #[serde(default)]
    pub project_path: Option<PathBuf>,
    #[serde(default)]
    pub confirmed: bool,
    #[serde(default)]
    pub action_confirmation: Option<ActionConfirmation>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SnapshotParams {
    pub snapshot_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RollbackSnapshotParams {
    pub snapshot_id: String,
    pub confirmation: ActionConfirmation,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListAgentConfigSnapshotsParams {
    pub agent: String,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListAgentConfigPageParams {
    pub agent: String,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub cursor: Option<String>,
    #[serde(default)]
    pub source_revision: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ConfigSnapshotPageResult {
    pub records: Vec<ConfigSnapshotRecord>,
    pub source_revision: String,
    #[serde(flatten)]
    pub page: ListPageMetadata,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadAgentConfigParams {
    pub agent: String,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct PreviewSaveClaudeSettingsParams {
    pub content: String,
    pub expected_revision: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveClaudeSettingsParams {
    pub content: String,
    pub confirmation: ActionConfirmation,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ExportSkillBundleParams {
    #[serde(default)]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub source_path: Option<PathBuf>,
    #[serde(default)]
    pub output_dir: Option<PathBuf>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ImportSkillParams {
    #[serde(default)]
    pub source_path: Option<String>,
    #[serde(default)]
    pub github_url: Option<String>,
}

#[derive(Debug, Error)]
pub enum ServiceError {
    #[error("invalid request: {0}")]
    InvalidRequest(String),
    #[error("unknown method: {0}")]
    UnknownMethod(String),
    #[error("io error: {0}")]
    Io(#[from] std::io::Error),
    #[error("catalog error: {0}")]
    Catalog(#[from] skills_copilot_catalog::CatalogError),
    #[error("command error: {0}")]
    Command(#[from] skills_copilot_commands::CommandError),
    #[error("provider error: {0}")]
    Provider(#[from] ProviderError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("skill instance not found: {0}")]
    SkillNotFound(String),
    #[error("confirmation required: {0}")]
    ConfirmationRequired(String),
    #[error("mutation disabled: {0}")]
    MutationDisabled(&'static str),
    #[error("list source changed during pagination")]
    SourceChanged,
    #[error("provider activity source is unreadable: {0}")]
    ProviderActivitySourceUnreadable(&'static str),
    #[error("provider activity source is invalid: {0}")]
    ProviderActivitySourceInvalid(&'static str),
    #[error("action did not start: {0}")]
    ActionNotStarted(String),
    #[error("action applied but could not be verified: {0}")]
    AppliedUnverified(String),
}

impl ServiceError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest(_) => "invalid_request",
            Self::UnknownMethod(_) => "unknown_method",
            Self::Io(_) => "io_error",
            Self::Catalog(_) => "catalog_error",
            Self::Command(skills_copilot_commands::CommandError::ConfigConflict { .. }) => {
                "config_conflict"
            }
            Self::Command(skills_copilot_commands::CommandError::UnknownActionReference(_)) => {
                "unknown_action_reference"
            }
            Self::Command(skills_copilot_commands::CommandError::StaleActionReference) => {
                "stale_action_reference"
            }
            Self::Command(skills_copilot_commands::CommandError::MismatchedActionReference(_)) => {
                "action_target_mismatch"
            }
            Self::Command(skills_copilot_commands::CommandError::ActionConfirmationRequired(_)) => {
                "confirmation_required"
            }
            Self::Command(skills_copilot_commands::CommandError::ActionTokenUnavailable(_)) => {
                "action_token_unavailable"
            }
            Self::Command(skills_copilot_commands::CommandError::NoApplicableAction(_)) => {
                "no_applicable_action"
            }
            Self::Command(skills_copilot_commands::CommandError::VerificationFailed) => {
                "verification_failed"
            }
            Self::Command(skills_copilot_commands::CommandError::PartialEffect { .. }) => {
                "partial_effect"
            }
            Self::Command(_) => "command_error",
            Self::Provider(_) => "provider_error",
            Self::Json(_) => "json_error",
            Self::SkillNotFound(_) => "skill_not_found",
            Self::ConfirmationRequired(_) => "confirmation_required",
            Self::MutationDisabled(_) => "mutation_disabled",
            Self::SourceChanged => "source_changed",
            Self::ProviderActivitySourceUnreadable(_) => "provider_activity_source_unreadable",
            Self::ProviderActivitySourceInvalid(_) => "provider_activity_source_invalid",
            Self::ActionNotStarted(_) => "action_not_started",
            Self::AppliedUnverified(_) => "applied_unverified",
        }
    }

    fn details(&self) -> Option<ServiceErrorDetails> {
        match self {
            Self::Command(skills_copilot_commands::CommandError::PartialEffect {
                operation,
                state,
                cleanup_required,
                ..
            }) => Some(ServiceErrorDetails {
                operation: operation.clone(),
                state: (*state).to_string(),
                cleanup_required: *cleanup_required,
                retry_allowed: false,
            }),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct ServiceHost {
    pub app_data_dir: PathBuf,
    pub adapter_ctx: AdapterContext,
}

#[cfg(test)]
mod tests;
