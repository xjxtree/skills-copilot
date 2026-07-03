use std::{
    collections::{BTreeMap, BTreeSet},
    env, fs,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use skills_copilot_catalog::{
    Catalog, ConfigSnapshotRecord, ConflictGroupRecord, FindingTriageRecord, RuleFindingRecord,
    RuleTuningRecord, SkillDetailRecord, SkillEventRecord, SkillRecord,
};
use skills_copilot_commands::{
    analyze_catalog, apply_install_with_manager, apply_local_create_with_manager,
    apply_remove_with_manager, apply_skill_toggles, apply_update_with_manager,
    clear_finding_triage, clear_rule_severity_override, clear_rule_suppression,
    delete_local_skill_with_manager, empty_cross_agent_comparison, export_skill_bundle,
    export_staging_skill_bundle, get_skill, import_github_skill_to_tool_global_deferred,
    import_local_skill_to_tool_global, install_skill_from_tool_global, list_adapter_capabilities,
    list_adapter_diagnostics, list_adapter_diagnostics_for_agents, list_agent_config_snapshots,
    list_conflicts, list_cross_agent_comparisons, list_finding_triage, list_findings,
    list_installed_skills_with_manager, list_rule_tuning, list_skill_events,
    list_skill_management_tools, list_snapshots, preview_install_with_manager,
    preview_local_create_with_manager, preview_remove_with_manager, preview_script_execution,
    preview_skill_toggles, preview_snapshot_rollback, preview_update_with_manager,
    read_agent_config, read_claude_settings, record_blocked_script_execution, rollback_snapshot,
    run_pi_writable_evidence_harness, save_claude_settings, scan_all_catalog_report,
    scan_claude_to_catalog, search_skills_with_manager, set_finding_triage,
    set_rule_severity_override, set_rule_suppression, skill_health_summary, toggle_skill,
    AdapterCapabilityRecord, AdapterDiagnosticsRecord, AgentCatalogScanReport,
    BatchToggleApplyRecord, BatchTogglePreviewRecord, ConfigDocumentRecord,
    CrossAgentAnalysisGroup, CrossAgentAnalysisRecord, CrossAgentComparisonRecord,
    ExportedSkillBundle, PiWritableHarnessReport, ScriptExecutionAttemptRecord,
    ScriptExecutionPreviewRecord, ScriptExecutionRequest, SkillHealthSummary,
    SkillInstallPreviewRecord, SkillManagerDeleteLocalParams, SkillManagerInstallParams,
    SkillManagerListInstalledParams, SkillManagerLocalCreateParams, SkillManagerRemoveParams,
    SkillManagerSearchParams, SkillManagerUpdateParams, SnapshotRollbackPreviewRecord,
    ToolGlobalImportResult, SCRIPT_EXECUTION_DISABLED_REASON,
};
use skills_copilot_core::{AdapterContext, AdapterRoot, AgentId, RootSource, Scope, SkillInstance};
use thiserror::Error;

mod cleanup_queue;
mod project_context;
mod protocol;
mod provider;
mod service_cleanup;
mod service_evidence;
mod service_guided_cleanup_helpers;
mod service_host;
mod service_knowledge;
mod service_knowledge_helpers;
mod service_llm;
mod service_llm_prompt_helpers;
mod service_observability_helpers;
mod service_remediation;
mod service_remediation_helpers;
mod service_support_helpers;
mod service_task;
mod service_task_helpers;

use cleanup_queue::cleanup_queue_response;
pub use cleanup_queue::{
    CleanupListQueueParams, CleanupQueue, CleanupQueueItem, CleanupQueueSummary,
};
use project_context::{
    clear_project_context, context_from_paths, load_project_context_state, project_context_summary,
    set_project_context, stored_active_adapter_paths, validate_project_context_for_response,
    ProjectContext, ProjectContextParams, ProjectContextState, ProjectContextSummary,
};
pub use protocol::{
    ServiceErrorRecord, ServiceRequest, ServiceResponse, DEFAULT_BUNDLE_ID, LEGACY_BUNDLE_ID,
    SERVICE_PROTOCOL_VERSION, SUPPORTED_METHODS,
};
use provider::{
    default_monthly_budget_usd, default_token_limit, delete_provider_profile,
    estimate_prompt_cost_usd, list_provider_profiles, provider_call_metadata_path,
    provider_profiles_path, save_provider_profile, send_provider_prompt, test_provider_connection,
    DeleteProviderProfileParams, ListProviderProfilesResult, ProviderCallMetadata, ProviderError,
    ProviderProfileRecord, SaveProviderProfileParams, SendProviderPromptParams,
    TestProviderConnectionParams,
};
pub(crate) use service_guided_cleanup_helpers::*;
pub(crate) use service_knowledge_helpers::*;
pub(crate) use service_llm_prompt_helpers::*;
pub(crate) use service_observability_helpers::*;
pub(crate) use service_remediation_helpers::*;
pub use service_support_helpers::handle_request_json;
pub(crate) use service_support_helpers::*;
pub(crate) use service_task_helpers::*;

const TASK_READINESS_MAX_CANDIDATE_SCAN: usize = 160;
const TASK_READINESS_MIN_CANDIDATE_SCAN: usize = 48;
const TASK_READINESS_CANDIDATE_SCAN_MULTIPLIER: usize = 12;
const TASK_AGGREGATION_TIMEOUT_MS: u64 = 2_000;
const TASK_COCKPIT_TIMEOUT_MS: u64 = 1_500;
const REMEDIATION_AGGREGATION_TIMEOUT_MS: u64 = 2_500;
const REMEDIATION_MAX_DETAIL_SCAN: usize = 240;

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
    pub trace_imports: TraceImportStatus,
    pub session_reviews: AgentSessionSkillReviewStatus,
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

#[derive(Debug, Clone, Serialize)]
pub struct ScanResult {
    pub scanned_count: usize,
    pub skills: Vec<SkillRecord>,
    pub activity: RefreshActivity,
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
    pub roots_skipped: Vec<String>,
    pub config_detected: bool,
    pub config_paths: Vec<String>,
    pub writable_status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub writable_reason: Option<String>,
    pub read_only_reason: String,
    pub blockers: Vec<String>,
    pub recovery_actions: Vec<String>,
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

#[derive(Debug, Clone, Serialize)]
pub struct TraceImportStatus {
    pub count: usize,
    pub imports_path: String,
    pub app_local_only: bool,
    pub raw_trace_persistence_allowed: bool,
    pub provider_request_allowed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentSessionSkillReviewStatus {
    pub count: usize,
    pub reviews_path: String,
    pub app_local_only: bool,
    pub raw_trace_persistence_allowed: bool,
    pub provider_request_allowed: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GuidedCleanupPlanParams {
    #[serde(default, alias = "task_text", alias = "user_intent")]
    pub task: Option<String>,
    #[serde(default, alias = "target_agent")]
    pub agent: Option<String>,
    #[serde(default, alias = "instance_id", alias = "skill_id")]
    pub selected_skill_id: Option<String>,
    #[serde(default)]
    pub selected_skill_name: Option<String>,
    #[serde(default, alias = "target_skill_agent")]
    pub selected_skill_agent: Option<String>,
    #[serde(default, alias = "workspace_path")]
    pub project_root: Option<String>,
    #[serde(default)]
    pub current_cwd: Option<String>,
    #[serde(default, alias = "workspace_label")]
    pub workspace: Option<String>,
    #[serde(default, alias = "instance_ids")]
    pub candidate_instance_ids: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub include_recorded_steps: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidedCleanupFlowResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: GuidedCleanupFlowFilters,
    pub summary: GuidedCleanupFlowSummary,
    pub flow_steps: Vec<GuidedCleanupFlowStep>,
    pub issue_groups: Vec<GuidedCleanupIssueGroup>,
    pub safe_next_actions: Vec<GuidedCleanupSafeNextAction>,
    pub recorded_steps: Vec<GuidedCleanupStepRecord>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: GuidedCleanupPromptRequest,
    pub safety_flags: GuidedCleanupSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidedCleanupFlowFilters {
    pub task: Option<String>,
    pub agent: Option<String>,
    pub selected_skill_id: Option<String>,
    pub selected_skill_name: Option<String>,
    pub selected_skill_agent: Option<String>,
    pub project_root: Option<String>,
    pub current_cwd: Option<String>,
    pub workspace: Option<String>,
    pub candidate_instance_ids: Vec<String>,
    pub limit: usize,
    pub include_recorded_steps: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct GuidedCleanupFlowSummary {
    pub total_step_count: usize,
    pub returned_step_count: usize,
    pub issue_group_count: usize,
    pub safe_next_action_count: usize,
    pub recorded_step_count: usize,
    pub high_risk_count: usize,
    pub medium_risk_count: usize,
    pub low_risk_count: usize,
    pub blocker_count: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidedCleanupFlowStep {
    pub id: String,
    pub rank: usize,
    pub step_type: &'static str,
    pub phase: &'static str,
    pub title: String,
    pub summary: String,
    pub status: String,
    pub risk: &'static str,
    pub source_method: &'static str,
    pub source_id: String,
    pub agent: Option<String>,
    pub skill_name: Option<String>,
    pub instance_id: Option<String>,
    pub definition_id: Option<String>,
    pub recommended_action_label: String,
    pub safe_entry_method: &'static str,
    pub existing_safe_method: Option<&'static str>,
    pub safe_action_deep_link: GuidedCleanupSafeActionDeepLink,
    pub requires_explicit_confirmation: bool,
    pub evidence_refs: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub gap_notes: Vec<String>,
    pub side_effect_flags: Vec<&'static str>,
    pub safety_flags: GuidedCleanupSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidedCleanupIssueGroup {
    pub id: String,
    pub group_type: &'static str,
    pub label: String,
    pub step_count: usize,
    pub high_risk_count: usize,
    pub medium_risk_count: usize,
    pub low_risk_count: usize,
    pub step_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub safety_flags: GuidedCleanupSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidedCleanupSafeNextAction {
    pub id: String,
    pub label: String,
    pub entry_method: &'static str,
    pub description: String,
    pub requires_preview: bool,
    pub requires_confirmation: bool,
    pub copy_only: bool,
    pub deep_link: GuidedCleanupSafeActionDeepLink,
    pub related_step_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub safety_flags: GuidedCleanupSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidedCleanupSafeActionDeepLink {
    pub label: String,
    pub target: &'static str,
    pub detail_section: &'static str,
    pub method: &'static str,
    pub trigger: &'static str,
    pub preview_only: bool,
    pub requires_confirmation: bool,
    pub copy_only: bool,
    pub can_apply: bool,
    pub instance_ids: Vec<String>,
    pub related_step_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub safety_flags: GuidedCleanupSafetyFlags,
}

pub type GuidedCleanupPromptRequest = AgentReadinessPromptRequest;
pub type GuidedCleanupSafetyFlags = RemediationHistorySafetyFlags;

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GuidedCleanupStepRecord {
    pub id: String,
    pub flow_step_id: String,
    pub title: String,
    pub decision: String,
    pub status: String,
    pub note: Option<String>,
    pub task: Option<String>,
    pub agent: Option<String>,
    pub instance_id: Option<String>,
    pub definition_id: Option<String>,
    pub skill_name: Option<String>,
    pub source_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
    #[serde(default = "remediation_history_redaction_summary_default")]
    pub redaction_summary: RemediationHistoryRedactionSummary,
    pub created_at: i64,
    pub updated_at: i64,
    #[serde(default = "guided_cleanup_safety_flags")]
    pub safety_flags: GuidedCleanupSafetyFlags,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct GuidedCleanupRecordStepParams {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, alias = "step_id")]
    pub flow_step_id: Option<String>,
    #[serde(default, alias = "step_title")]
    pub title: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub note: Option<String>,
    #[serde(default, alias = "task_text", alias = "user_intent")]
    pub task: Option<String>,
    #[serde(default, alias = "target_agent")]
    pub agent: Option<String>,
    #[serde(default, alias = "skill_id", alias = "selected_skill_id")]
    pub instance_id: Option<String>,
    #[serde(default)]
    pub definition_id: Option<String>,
    #[serde(default, alias = "selected_skill_name")]
    pub skill_name: Option<String>,
    #[serde(default, alias = "source_item_refs")]
    pub source_refs: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct GuidedCleanupRecordStepResult {
    pub generated_by: &'static str,
    pub record: GuidedCleanupStepRecord,
    pub created: bool,
    pub count: usize,
    pub app_local_only: bool,
    pub record_file: &'static str,
    pub provider_request_sent: bool,
    pub skill_files_mutated: bool,
    pub agent_config_mutated: bool,
    pub snapshot_created: bool,
    pub rollback_performed: bool,
    pub triage_mutated: bool,
    pub script_executed: bool,
    pub credential_accessed: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
    pub safety_flags: GuidedCleanupSafetyFlags,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListCrossAgentComparisonParams {
    #[serde(default)]
    pub selected_instance_id: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ReportExportLocalParams {
    #[serde(default)]
    pub formats: Vec<ReportExportFormat>,
    #[serde(default, alias = "agentFilter")]
    pub agent: Option<String>,
    #[serde(default, alias = "instanceId", alias = "skill_instance_id")]
    pub instance_id: Option<String>,
    #[serde(default, alias = "stateFilter")]
    pub state_filter: Option<String>,
    #[serde(default)]
    pub search: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ReportExportFormat {
    Json,
    Markdown,
}

impl ReportExportFormat {
    fn extension(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Markdown => "md",
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Json => "json",
            Self::Markdown => "markdown",
        }
    }
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportExportLocalResult {
    pub export_id: String,
    pub generated_at: i64,
    pub output_dir: String,
    pub files: Vec<ReportExportedFile>,
    pub catalog_available: bool,
    pub summary: ReportExportSummary,
    pub sections: Vec<ReportExportSection>,
    pub redaction: ReportExportRedaction,
    pub read_only: bool,
    pub writes_allowed: bool,
    pub provider_request_sent: bool,
    pub script_execution_allowed: bool,
    pub credential_accessed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportExportedFile {
    pub format: &'static str,
    pub path: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportExportSection {
    pub name: &'static str,
    pub count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportExportSummary {
    pub skill_count: usize,
    pub finding_count: usize,
    pub open_finding_count: usize,
    pub triage_count: usize,
    pub cleanup_item_count: usize,
    pub comparison_group_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct ReportExportRedaction {
    pub enabled: bool,
    pub placeholders: Vec<&'static str>,
    pub path_policy: &'static str,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ScoreSkillQualityParams {
    #[serde(alias = "skill_instance_id")]
    pub instance_id: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub definition_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillQualityScoreResult {
    pub instance_id: String,
    pub definition_id: String,
    pub agent: String,
    pub scope: String,
    pub skill_name: String,
    pub score: u8,
    pub grade: &'static str,
    pub band: &'static str,
    pub generated_by: &'static str,
    pub components: Vec<SkillQualityScoreComponent>,
    pub reasons: Vec<String>,
    pub risk_notes: Vec<String>,
    pub evidence_references: Vec<SkillQualityEvidenceReference>,
    pub suggested_improvements: Vec<SkillQualitySuggestion>,
    pub prompt_request: SkillQualityPromptRequest,
    pub safety_flags: SkillQualitySafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillQualityScoreComponent {
    pub id: &'static str,
    pub label: &'static str,
    pub score: u8,
    pub max_score: u8,
    pub summary: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillQualityEvidenceReference {
    pub id: String,
    pub source_type: &'static str,
    pub source_id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_instance_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillQualitySuggestion {
    pub priority: &'static str,
    pub title: String,
    pub detail: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillQualityPromptRequest {
    pub available: bool,
    pub preview_method: &'static str,
    pub confirm_method: &'static str,
    pub action: &'static str,
    pub request: LlmPreviewPromptParams,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillQualitySafetyFlags {
    pub read_only: bool,
    pub provider_request_sent: bool,
    pub write_back_allowed: bool,
    pub script_execution_allowed: bool,
    pub config_mutation_allowed: bool,
    pub snapshot_created: bool,
    pub triage_mutation_allowed: bool,
    pub credential_accessed: bool,
    pub raw_secret_returned: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AggregationRuntimeMetadata {
    pub status: &'static str,
    pub elapsed_ms: u64,
    pub timeout_ms: u64,
    pub timed_out: bool,
    pub partial: bool,
    pub fallback_used: bool,
    pub limit: usize,
    pub scanned_count: usize,
    pub total_count: usize,
    pub completed_stages: Vec<&'static str>,
    pub skipped_stages: Vec<&'static str>,
    pub blocker_codes: Vec<&'static str>,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TaskReadinessParams {
    #[serde(alias = "user_intent", alias = "task_text")]
    pub task: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default, alias = "instance_ids")]
    pub candidate_instance_ids: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskReadinessResult {
    pub task: String,
    pub score: u8,
    pub band: &'static str,
    pub summary: String,
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: TaskReadinessFilters,
    pub candidate_skills: Vec<TaskReadinessCandidate>,
    pub missing_gap_notes: Vec<String>,
    pub blocker_risk_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: TaskReadinessPromptRequest,
    pub aggregation: AggregationRuntimeMetadata,
    pub safety_flags: TaskReadinessSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskReadinessFilters {
    pub agent: Option<String>,
    pub candidate_instance_ids: Vec<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskReadinessCandidate {
    pub instance_id: String,
    pub definition_id: String,
    pub skill_name: String,
    pub agent: String,
    pub scope: String,
    pub enabled: bool,
    pub state: String,
    pub score: u8,
    pub band: &'static str,
    pub quality_score: Option<u8>,
    pub match_reasons: Vec<String>,
    pub enabled_scope_risk_state: TaskReadinessState,
    pub missing_gap_notes: Vec<String>,
    pub blocker_risk_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskReadinessState {
    pub enabled: bool,
    pub scope: String,
    pub state: String,
    pub risk_level: &'static str,
    pub risk_summary: String,
    pub writable_status: Option<String>,
    pub adapter_status: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskReadinessEvidenceReference {
    pub id: String,
    pub source_type: &'static str,
    pub source_id: String,
    pub label: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub severity: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub related_instance_id: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskReadinessPromptRequest {
    pub available: bool,
    pub preview_method: &'static str,
    pub confirm_method: &'static str,
    pub action: &'static str,
    pub request: LlmPreviewPromptParams,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskReadinessSafetyFlags {
    pub read_only: bool,
    pub provider_request_sent: bool,
    pub write_back_allowed: bool,
    pub script_execution_allowed: bool,
    pub config_mutation_allowed: bool,
    pub snapshot_created: bool,
    pub triage_mutation_allowed: bool,
    pub credential_accessed: bool,
    pub raw_secret_returned: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RankSkillRoutesParams {
    #[serde(alias = "user_intent", alias = "task_text")]
    pub task: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default, alias = "instance_ids")]
    pub candidate_instance_ids: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillRouteRankingResult {
    pub task: String,
    pub overall_confidence_score: u8,
    pub overall_confidence_band: &'static str,
    pub summary: String,
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: TaskReadinessFilters,
    pub route_candidates: Vec<SkillRouteCandidate>,
    pub ambiguity_warnings: Vec<String>,
    pub likely_wrong_pick_risks: Vec<String>,
    pub likely_miss_risks: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: RoutingConfidencePromptRequest,
    pub aggregation: AggregationRuntimeMetadata,
    pub safety_flags: RoutingConfidenceSafetyFlags,
}

#[derive(Debug, Clone, Deserialize)]
pub struct CompareAgentReadinessParams {
    #[serde(alias = "user_intent", alias = "task_text")]
    pub task: String,
    #[serde(default, alias = "target_agents")]
    pub agents: Vec<String>,
    #[serde(default)]
    pub limit_per_agent: Option<usize>,
    #[serde(default)]
    pub include_routing_accuracy: bool,
    #[serde(default)]
    pub include_benchmarks: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentReadinessComparisonResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: AgentReadinessComparisonFilters,
    pub summary: AgentReadinessComparisonSummary,
    pub agent_rows: Vec<AgentReadinessComparisonRow>,
    pub recommended_agent: Option<AgentReadinessRecommendation>,
    pub gap_issue_rows: Vec<AgentReadinessGapIssueRow>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: AgentReadinessPromptRequest,
    pub aggregation: AggregationRuntimeMetadata,
    pub safety_flags: AgentReadinessSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentReadinessComparisonFilters {
    pub agents: Vec<String>,
    pub limit_per_agent: usize,
    pub include_routing_accuracy: bool,
    pub include_benchmarks: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentReadinessComparisonSummary {
    pub agent_count: usize,
    pub candidate_count: usize,
    pub ready_agent_count: usize,
    pub partial_agent_count: usize,
    pub blocked_agent_count: usize,
    pub gap_issue_count: usize,
    pub recommended_agent: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentReadinessComparisonRow {
    pub rank: usize,
    pub agent: String,
    pub display_name: String,
    pub comparison_score: u8,
    pub readiness_score: u8,
    pub readiness_band: &'static str,
    pub routing_confidence_score: u8,
    pub routing_confidence_band: &'static str,
    pub candidate_count: usize,
    pub best_candidate: Option<AgentReadinessBestCandidate>,
    pub enabled_scope_risk_state: Option<TaskReadinessState>,
    pub blocker_count: usize,
    pub gap_count: usize,
    pub reasons: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub gap_notes: Vec<String>,
    pub routing_accuracy_context: Option<AgentReadinessAccuracyContext>,
    pub benchmark_context: Option<AgentReadinessBenchmarkContext>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentReadinessBestCandidate {
    pub instance_id: String,
    pub definition_id: String,
    pub skill_name: String,
    pub scope: String,
    pub enabled: bool,
    pub state: String,
    pub readiness_score: u8,
    pub readiness_band: &'static str,
    pub routing_confidence_score: u8,
    pub routing_confidence_band: &'static str,
    pub quality_score: Option<u8>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct AgentReadinessAccuracyContext {
    pub trace_count: usize,
    pub accuracy_rate: f64,
    pub benchmark_count: usize,
    pub benchmark_gap_count: usize,
    pub regression_count: usize,
    pub recent_evidence_count: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct AgentReadinessBenchmarkContext {
    pub evaluated_count: usize,
    pub matched_count: usize,
    pub gap_count: usize,
    pub regression_count: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentReadinessRecommendation {
    pub agent: String,
    pub display_name: String,
    pub comparison_score: u8,
    pub readiness_score: u8,
    pub routing_confidence_score: u8,
    pub skill_name: Option<String>,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentReadinessGapIssueRow {
    pub source: &'static str,
    pub severity: &'static str,
    pub agent: String,
    pub title: String,
    pub detail: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentReadinessPromptRequest {
    pub available: bool,
    pub preview_method: &'static str,
    pub confirm_method: &'static str,
    pub action: &'static str,
    pub request: LlmPreviewPromptParams,
    pub note: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct AgentReadinessSafetyFlags {
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

#[derive(Debug, Clone, Deserialize)]
pub struct TaskCockpitParams {
    #[serde(alias = "user_intent", alias = "task_text")]
    pub task: String,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default, alias = "instance_ids")]
    pub candidate_instance_ids: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub include_session_review: Option<bool>,
    #[serde(default)]
    pub include_provider_observability: Option<bool>,
    #[serde(default)]
    pub include_remediation_context: Option<bool>,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCockpitResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub partial: bool,
    pub elapsed_ms: u64,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub fallback_reason: Option<String>,
    pub filters: TaskCockpitFilters,
    pub summary: TaskCockpitSummary,
    pub cockpit_sections: Vec<TaskCockpitSection>,
    pub task_rows: Vec<TaskCockpitTaskRow>,
    pub agent_route_rows: Vec<TaskCockpitAgentRouteRow>,
    pub skill_candidate_rows: Vec<TaskCockpitSkillCandidateRow>,
    pub readiness_rows: Vec<TaskCockpitReadinessRow>,
    pub session_review_rows: Vec<TaskCockpitSessionReviewRow>,
    pub provider_observability_rows: Vec<TaskCockpitProviderObservabilityRow>,
    pub remediation_next_steps: Vec<TaskCockpitRemediationNextStep>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: AgentReadinessPromptRequest,
    pub aggregation: AggregationRuntimeMetadata,
    pub safety_flags: TaskCockpitSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCockpitFilters {
    pub task: String,
    pub agent: Option<String>,
    pub candidate_instance_ids: Vec<String>,
    pub limit: usize,
    pub include_session_review: bool,
    pub include_provider_observability: bool,
    pub include_remediation_context: bool,
    pub timeout_ms: u64,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCockpitSummary {
    pub readiness_score: u8,
    pub readiness_band: &'static str,
    pub routing_confidence_score: u8,
    pub routing_confidence_band: &'static str,
    pub candidate_count: usize,
    pub agent_count: usize,
    pub session_review_count: usize,
    pub provider_observability_row_count: usize,
    pub remediation_next_step_count: usize,
    pub gap_count: usize,
    pub blocker_count: usize,
    pub recommended_agent: Option<String>,
    pub top_skill_name: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCockpitSection {
    pub id: &'static str,
    pub title: &'static str,
    pub status: &'static str,
    pub score: Option<u8>,
    pub row_count: usize,
    pub summary: String,
    pub evidence_refs: Vec<String>,
    pub safety_flags: TaskCockpitSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCockpitTaskRow {
    pub id: &'static str,
    pub task: String,
    pub readiness_score: u8,
    pub readiness_band: &'static str,
    pub routing_confidence_score: u8,
    pub routing_confidence_band: &'static str,
    pub recommended_agent: Option<String>,
    pub top_skill_name: Option<String>,
    pub candidate_count: usize,
    pub gap_count: usize,
    pub blocker_count: usize,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCockpitAgentRouteRow {
    pub rank: usize,
    pub agent: String,
    pub display_name: String,
    pub comparison_score: u8,
    pub readiness_score: u8,
    pub readiness_band: &'static str,
    pub routing_confidence_score: u8,
    pub routing_confidence_band: &'static str,
    pub best_skill_name: Option<String>,
    pub blocker_count: usize,
    pub gap_count: usize,
    pub reasons: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCockpitSkillCandidateRow {
    pub rank: usize,
    pub instance_id: String,
    pub definition_id: String,
    pub skill_name: String,
    pub agent: String,
    pub scope: String,
    pub enabled: bool,
    pub state: String,
    pub readiness_score: u8,
    pub readiness_band: &'static str,
    pub routing_confidence_score: u8,
    pub routing_confidence_band: &'static str,
    pub quality_score: Option<u8>,
    pub match_reasons: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub gap_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCockpitReadinessRow {
    pub id: String,
    pub row_type: &'static str,
    pub label: String,
    pub status: &'static str,
    pub score: Option<u8>,
    pub summary: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCockpitSessionReviewRow {
    pub id: String,
    pub title: String,
    pub agent: Option<String>,
    pub task: Option<String>,
    pub outcome: String,
    pub summary: String,
    pub detected_skill_count: usize,
    pub expected_skill_signal_count: usize,
    pub reviewed_at: i64,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCockpitProviderObservabilityRow {
    pub id: String,
    pub source: &'static str,
    pub status: String,
    pub provider: Option<String>,
    pub model: Option<String>,
    pub action: Option<String>,
    pub count: usize,
    pub message: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskCockpitRemediationNextStep {
    pub id: String,
    pub source: &'static str,
    pub priority: &'static str,
    pub title: String,
    pub suggested_safe_next_action: String,
    pub blocker_notes: Vec<String>,
    pub gap_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub type TaskCockpitSafetyFlags = AgentReadinessSafetyFlags;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SkillLifecycleTimelineParams {
    #[serde(default, alias = "task_text", alias = "user_intent")]
    pub task: Option<String>,
    #[serde(default, alias = "target_agent")]
    pub agent: Option<String>,
    #[serde(default, alias = "instance_id", alias = "skill_id")]
    pub selected_skill_id: Option<String>,
    #[serde(default)]
    pub selected_skill_name: Option<String>,
    #[serde(default, alias = "target_skill_agent")]
    pub selected_skill_agent: Option<String>,
    #[serde(default)]
    pub definition_id: Option<String>,
    #[serde(default, alias = "workspace_path")]
    pub project_root: Option<String>,
    #[serde(default)]
    pub current_cwd: Option<String>,
    #[serde(default, alias = "workspace_root", alias = "workspace_label")]
    pub workspace: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default = "default_true")]
    pub include_prompt_runs: bool,
    #[serde(default = "default_true")]
    pub include_session_reviews: bool,
    #[serde(default = "default_true")]
    pub include_remediation_history: bool,
    #[serde(default = "default_true")]
    pub include_stale_drift: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillLifecycleTimelineResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: SkillLifecycleTimelineFilters,
    pub summary: SkillLifecycleTimelineSummary,
    pub timeline_rows: Vec<SkillLifecycleTimelineRow>,
    pub skill_rows: Vec<SkillLifecycleSkillRow>,
    pub agent_rows: Vec<SkillLifecycleAgentRow>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: AgentReadinessPromptRequest,
    pub safety_flags: SkillLifecycleTimelineSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillLifecycleTimelineFilters {
    pub task: Option<String>,
    pub agent: Option<String>,
    pub selected_skill_id: Option<String>,
    pub selected_skill_name: Option<String>,
    pub selected_skill_agent: Option<String>,
    pub definition_id: Option<String>,
    pub project_root: Option<String>,
    pub current_cwd: Option<String>,
    pub workspace: Option<String>,
    pub limit: usize,
    pub include_prompt_runs: bool,
    pub include_session_reviews: bool,
    pub include_remediation_history: bool,
    pub include_stale_drift: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct SkillLifecycleTimelineSummary {
    pub total_event_count: usize,
    pub skill_count: usize,
    pub agent_count: usize,
    pub finding_event_count: usize,
    pub drift_event_count: usize,
    pub remediation_event_count: usize,
    pub prompt_event_count: usize,
    pub session_review_event_count: usize,
    pub first_event_at: Option<i64>,
    pub latest_event_at: Option<i64>,
    pub selected_skill_name: Option<String>,
    pub selected_agent: Option<String>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillLifecycleTimelineRow {
    pub id: String,
    pub occurred_at: Option<i64>,
    pub event_type: &'static str,
    pub lifecycle_stage: &'static str,
    pub title: String,
    pub summary: String,
    pub agent: Option<String>,
    pub skill_name: Option<String>,
    pub instance_id: Option<String>,
    pub definition_id: Option<String>,
    pub source: &'static str,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub evidence_refs: Vec<String>,
    pub safety_flags: SkillLifecycleTimelineSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillLifecycleSkillRow {
    pub instance_id: String,
    pub definition_id: String,
    pub skill_name: String,
    pub agent: String,
    pub scope: String,
    pub enabled: bool,
    pub state: String,
    pub event_count: usize,
    pub finding_event_count: usize,
    pub drift_event_count: usize,
    pub remediation_event_count: usize,
    pub prompt_event_count: usize,
    pub session_review_event_count: usize,
    pub first_event_at: Option<i64>,
    pub latest_event_at: Option<i64>,
    pub evidence_refs: Vec<String>,
    pub safety_flags: SkillLifecycleTimelineSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct SkillLifecycleAgentRow {
    pub agent: String,
    pub skill_count: usize,
    pub event_count: usize,
    pub finding_event_count: usize,
    pub drift_event_count: usize,
    pub remediation_event_count: usize,
    pub prompt_event_count: usize,
    pub session_review_event_count: usize,
    pub first_event_at: Option<i64>,
    pub latest_event_at: Option<i64>,
    pub evidence_refs: Vec<String>,
    pub safety_flags: SkillLifecycleTimelineSafetyFlags,
}

pub type SkillLifecycleTimelineSafetyFlags = AgentReadinessSafetyFlags;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct KnowledgeSearchParams {
    #[serde(default)]
    pub query: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub risk: Option<String>,
    #[serde(default)]
    pub scope: Option<String>,
    #[serde(default)]
    pub enabled: Option<bool>,
    #[serde(default)]
    pub tool: Option<String>,
    #[serde(default)]
    pub keyword: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSearchResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub summary: KnowledgeSearchSummary,
    pub filters: KnowledgeSearchFilters,
    pub rows: Vec<KnowledgeSearchRow>,
    pub facets: KnowledgeSearchFacets,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: KnowledgeSearchPromptRequest,
    pub safety_flags: KnowledgeSearchSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSearchSummary {
    pub indexed_skill_count: usize,
    pub matched_row_count: usize,
    pub returned_row_count: usize,
    pub enabled_count: usize,
    pub disabled_count: usize,
    pub high_risk_count: usize,
    pub stale_or_drift_count: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSearchFilters {
    pub query: Option<String>,
    pub normalized_terms: Vec<String>,
    pub agent: Option<String>,
    pub limit: usize,
    pub risk: Option<String>,
    pub scope: Option<String>,
    pub enabled: Option<bool>,
    pub tool: Option<String>,
    pub keyword: Option<String>,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct KnowledgeSearchFacets {
    pub agents: BTreeMap<String, usize>,
    pub scopes: BTreeMap<String, usize>,
    pub states: BTreeMap<String, usize>,
    pub enabled: BTreeMap<String, usize>,
    pub risks: BTreeMap<String, usize>,
    pub tools: BTreeMap<String, usize>,
    pub keywords: BTreeMap<String, usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSearchRow {
    pub rank: usize,
    pub instance_id: String,
    pub definition_id: String,
    pub skill_name: String,
    pub agent: String,
    pub scope: String,
    pub enabled: bool,
    pub state: String,
    pub source: KnowledgeSearchSource,
    pub purpose_snippet: Option<String>,
    pub description_snippet: Option<String>,
    pub matched_fields: Vec<String>,
    pub match_reasons: Vec<String>,
    pub keywords: Vec<String>,
    pub tools: Vec<String>,
    pub rules: Vec<String>,
    pub capability_tags: Vec<String>,
    pub risk_tags: Vec<String>,
    pub quality_context: Option<KnowledgeQualityContext>,
    pub readiness_context: Option<KnowledgeReadinessContext>,
    pub stale_drift_context: Option<KnowledgeStaleDriftContext>,
    pub evidence_refs: Vec<String>,
    pub safety_flags: KnowledgeSearchSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeSearchSource {
    pub source_path: String,
    pub display_path: String,
    pub root_provenance: String,
    pub fingerprint: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeQualityContext {
    pub score: u8,
    pub grade: &'static str,
    pub band: &'static str,
    pub reasons: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeReadinessContext {
    pub score: u8,
    pub band: &'static str,
    pub risk_level: &'static str,
    pub risk_summary: String,
    pub gap_count: usize,
    pub blocker_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct KnowledgeStaleDriftContext {
    pub score: u8,
    pub band: &'static str,
    pub fingerprint_drift: bool,
    pub finding_drift: bool,
    pub source_drift: bool,
    pub stale_by_mtime: bool,
    pub readiness_impact_level: Option<&'static str>,
}

pub type KnowledgeSearchPromptRequest = AgentReadinessPromptRequest;
pub type KnowledgeSearchSafetyFlags = AgentReadinessSafetyFlags;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SimilarSkillGroupingParams {
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub min_score: Option<f64>,
    #[serde(default)]
    pub include_singletons: bool,
    #[serde(default)]
    pub candidate_instance_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimilarSkillGroupingResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: SimilarSkillGroupingFilters,
    pub summary: SimilarSkillGroupingSummary,
    pub groups: Vec<SimilarSkillGroup>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: SimilarSkillGroupingPromptRequest,
    pub safety_flags: SimilarSkillGroupingSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimilarSkillGroupingFilters {
    pub agent: Option<String>,
    pub limit: usize,
    pub min_score: u8,
    pub include_singletons: bool,
    pub candidate_instance_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimilarSkillGroupingSummary {
    pub indexed_skill_count: usize,
    pub candidate_skill_count: usize,
    pub matched_group_count: usize,
    pub returned_group_count: usize,
    pub duplicate_group_count: usize,
    pub confusable_group_count: usize,
    pub coverage_redundancy_group_count: usize,
    pub routing_ambiguity_count: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimilarSkillGroup {
    pub group_id: String,
    pub rank: usize,
    pub group_type: &'static str,
    pub similarity_score: u8,
    pub ambiguity_risk: &'static str,
    pub coverage_redundancy: &'static str,
    pub routing_ambiguity: &'static str,
    pub canonical_name: String,
    pub canonical_key: String,
    pub title: String,
    pub summary: String,
    pub why_grouped: Vec<String>,
    pub shared_terms: Vec<String>,
    pub shared_tools: Vec<String>,
    pub shared_rules: Vec<String>,
    pub shared_capability_tags: Vec<String>,
    pub shared_risk_tags: Vec<String>,
    pub shared_source_signals: Vec<String>,
    pub members: Vec<SimilarSkillMember>,
    pub evidence_refs: Vec<String>,
    pub safety_flags: SimilarSkillGroupingSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct SimilarSkillMember {
    pub instance_id: String,
    pub definition_id: String,
    pub skill_name: String,
    pub agent: String,
    pub scope: String,
    pub enabled: bool,
    pub state: String,
    pub source: KnowledgeSearchSource,
    pub quality_context: Option<KnowledgeQualityContext>,
    pub readiness_context: Option<KnowledgeReadinessContext>,
    pub stale_drift_context: Option<KnowledgeStaleDriftContext>,
    pub match_reasons: Vec<String>,
    pub similarity_reasons: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub type SimilarSkillGroupingPromptRequest = AgentReadinessPromptRequest;
pub type SimilarSkillGroupingSafetyFlags = AgentReadinessSafetyFlags;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct CapabilityTaxonomyParams {
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub include_single_skill_domains: bool,
    #[serde(default, alias = "instance_ids")]
    pub candidate_instance_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityTaxonomyResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: CapabilityTaxonomyFilters,
    pub summary: CapabilityTaxonomySummary,
    pub domains: Vec<CapabilityDomainRow>,
    pub coverage_rows: Vec<CapabilityCoverageRow>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: CapabilityTaxonomyPromptRequest,
    pub safety_flags: CapabilityTaxonomySafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityTaxonomyFilters {
    pub agent: Option<String>,
    pub limit: usize,
    pub include_single_skill_domains: bool,
    pub candidate_instance_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityTaxonomySummary {
    pub indexed_skill_count: usize,
    pub candidate_skill_count: usize,
    pub domain_count: usize,
    pub returned_domain_count: usize,
    pub total_representative_skill_count: usize,
    pub agent_count: usize,
    pub workspace_count: usize,
    pub duplicate_or_redundant_domain_count: usize,
    pub routing_ambiguity_domain_count: usize,
    pub gap_count: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityDomainRow {
    pub domain_id: String,
    pub rank: usize,
    pub domain_key: String,
    pub domain_name: String,
    pub coverage_level: &'static str,
    pub coverage_score: u8,
    pub skill_count: usize,
    pub enabled_skill_count: usize,
    pub disabled_skill_count: usize,
    pub agent_count: usize,
    pub workspace_count: usize,
    pub agents: BTreeMap<String, usize>,
    pub workspaces: BTreeMap<String, usize>,
    pub duplicate_or_redundant_count: usize,
    pub routing_ambiguity_count: usize,
    pub representative_skills: Vec<CapabilityRepresentativeSkill>,
    pub capability_tags: Vec<String>,
    pub risk_tags: Vec<String>,
    pub tools: Vec<String>,
    pub rules: Vec<String>,
    pub keywords: Vec<String>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub safety_flags: CapabilityTaxonomySafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityCoverageRow {
    pub domain_key: String,
    pub domain_name: String,
    pub coverage_level: &'static str,
    pub coverage_score: u8,
    pub skill_count: usize,
    pub enabled_skill_count: usize,
    pub agent_count: usize,
    pub workspace_count: usize,
    pub agents: BTreeMap<String, usize>,
    pub gaps: Vec<String>,
    pub duplicates_redundancy: &'static str,
    pub routing_ambiguity: &'static str,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct CapabilityRepresentativeSkill {
    pub instance_id: String,
    pub definition_id: String,
    pub skill_name: String,
    pub agent: String,
    pub scope: String,
    pub enabled: bool,
    pub state: String,
    pub source: KnowledgeSearchSource,
    pub quality_context: Option<KnowledgeQualityContext>,
    pub stale_drift_context: Option<KnowledgeStaleDriftContext>,
    pub similarity_group_ids: Vec<String>,
    pub match_reasons: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub type CapabilityTaxonomyPromptRequest = AgentReadinessPromptRequest;
pub type CapabilityTaxonomySafetyFlags = AgentReadinessSafetyFlags;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct LocalSkillMapParams {
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default, alias = "task_text", alias = "user_intent")]
    pub task: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub node_limit: Option<usize>,
    #[serde(default)]
    pub edge_limit: Option<usize>,
    #[serde(default)]
    pub cluster_limit: Option<usize>,
    #[serde(default, alias = "instance_ids")]
    pub candidate_instance_ids: Vec<String>,
    #[serde(default)]
    pub include_task_context: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSkillMapResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: LocalSkillMapFilters,
    pub summary: LocalSkillMapSummary,
    pub nodes: Vec<LocalSkillMapNode>,
    pub edges: Vec<LocalSkillMapEdge>,
    pub clusters: Vec<LocalSkillMapCluster>,
    pub domains: Vec<LocalSkillMapDomain>,
    pub risk_notes: Vec<String>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: LocalSkillMapPromptRequest,
    pub safety_flags: LocalSkillMapSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSkillMapFilters {
    pub agent: Option<String>,
    pub task: Option<String>,
    pub limit: usize,
    pub node_limit: usize,
    pub edge_limit: usize,
    pub cluster_limit: usize,
    pub candidate_instance_ids: Vec<String>,
    pub include_task_context: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSkillMapSummary {
    pub indexed_skill_count: usize,
    pub candidate_skill_count: usize,
    pub returned_node_count: usize,
    pub returned_edge_count: usize,
    pub cluster_count: usize,
    pub returned_cluster_count: usize,
    pub domain_count: usize,
    pub skill_node_count: usize,
    pub capability_node_count: usize,
    pub similar_group_node_count: usize,
    pub conflict_node_count: usize,
    pub risk_node_count: usize,
    pub task_coverage_edge_count: usize,
    pub cross_agent_edge_count: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSkillMapNode {
    pub id: String,
    pub node_type: String,
    pub rank: usize,
    pub label: String,
    pub summary: String,
    pub weight: u8,
    pub agent: Option<String>,
    pub scope: Option<String>,
    pub enabled: Option<bool>,
    pub state: Option<String>,
    pub source: Option<KnowledgeSearchSource>,
    pub risk_level: Option<String>,
    pub tags: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub safety_flags: LocalSkillMapSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSkillMapEdge {
    pub id: String,
    pub edge_type: String,
    pub source: String,
    pub target: String,
    pub label: String,
    pub weight: u8,
    pub reasons: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub safety_flags: LocalSkillMapSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSkillMapCluster {
    pub id: String,
    pub cluster_type: String,
    pub label: String,
    pub summary: String,
    pub score: u8,
    pub risk_level: String,
    pub node_ids: Vec<String>,
    pub edge_ids: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub safety_flags: LocalSkillMapSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct LocalSkillMapDomain {
    pub domain_id: String,
    pub domain_key: String,
    pub domain_name: String,
    pub coverage_level: &'static str,
    pub coverage_score: u8,
    pub node_ids: Vec<String>,
    pub skill_count: usize,
    pub enabled_skill_count: usize,
    pub agent_count: usize,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub type LocalSkillMapPromptRequest = AgentReadinessPromptRequest;
pub type LocalSkillMapSafetyFlags = AgentReadinessSafetyFlags;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct WorkspaceReadinessParams {
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default, alias = "task_text", alias = "user_intent")]
    pub task: Option<String>,
    #[serde(default, alias = "workspace_path")]
    pub project_root: Option<String>,
    #[serde(default)]
    pub expected_capabilities: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default, alias = "instance_ids")]
    pub candidate_instance_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceReadinessResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: WorkspaceReadinessFilters,
    pub summary: WorkspaceReadinessSummary,
    pub readiness_rows: Vec<WorkspaceReadinessChecklistRow>,
    pub checklist_rows: Vec<WorkspaceReadinessChecklistRow>,
    pub agent_rows: Vec<WorkspaceReadinessAgentRow>,
    pub capability_rows: Vec<WorkspaceReadinessCapabilityRow>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: WorkspaceReadinessPromptRequest,
    pub safety_flags: WorkspaceReadinessSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceReadinessFilters {
    pub agent: Option<String>,
    pub task: Option<String>,
    pub project_root: Option<String>,
    pub expected_capabilities: Vec<String>,
    pub limit: usize,
    pub candidate_instance_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceReadinessSummary {
    pub workspace_available: bool,
    pub project_available: bool,
    pub visible_skill_count: usize,
    pub enabled_skill_count: usize,
    pub agent_count: usize,
    pub domain_count: usize,
    pub capability_count: usize,
    pub ready_count: usize,
    pub partial_count: usize,
    pub blocked_count: usize,
    pub gap_count: usize,
    pub blocker_count: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceReadinessChecklistRow {
    pub id: String,
    pub category: &'static str,
    pub status: &'static str,
    pub score: u8,
    pub title: String,
    pub detail: String,
    pub agent: Option<String>,
    pub capability: Option<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceReadinessAgentRow {
    pub agent: String,
    pub display_name: String,
    pub status: &'static str,
    pub score: u8,
    pub visible_skill_count: usize,
    pub enabled_skill_count: usize,
    pub project_skill_count: usize,
    pub best_candidate: Option<AgentReadinessBestCandidate>,
    pub adapter_status: Option<String>,
    pub writable_status: Option<String>,
    pub install_status: Option<String>,
    pub gap_count: usize,
    pub blocker_count: usize,
    pub notes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct WorkspaceReadinessCapabilityRow {
    pub capability: String,
    pub domain_key: String,
    pub domain_name: String,
    pub status: &'static str,
    pub coverage_level: &'static str,
    pub coverage_score: u8,
    pub expected: bool,
    pub skill_count: usize,
    pub enabled_skill_count: usize,
    pub agent_count: usize,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub type WorkspaceReadinessPromptRequest = AgentReadinessPromptRequest;
pub type WorkspaceReadinessSafetyFlags = AgentReadinessSafetyFlags;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RemediationPlanParams {
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default, alias = "task_text", alias = "user_intent")]
    pub task: Option<String>,
    #[serde(default, alias = "workspace_path")]
    pub project_root: Option<String>,
    #[serde(default)]
    pub focus: Option<String>,
    #[serde(default)]
    pub focus_areas: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default, alias = "instance_ids")]
    pub candidate_instance_ids: Vec<String>,
    #[serde(default)]
    pub include_deferred: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationPlanResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: RemediationPlanFilters,
    pub summary: RemediationPlanSummary,
    pub plan_items: Vec<RemediationPlanItem>,
    pub priority_rows: Vec<RemediationPriorityRow>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: RemediationPlanPromptRequest,
    pub aggregation: AggregationRuntimeMetadata,
    pub safety_flags: RemediationPlanSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationPlanFilters {
    pub agent: Option<String>,
    pub task: Option<String>,
    pub project_root: Option<String>,
    pub focus_areas: Vec<String>,
    pub limit: usize,
    pub candidate_instance_ids: Vec<String>,
    pub include_deferred: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationPlanSummary {
    pub total_item_count: usize,
    pub returned_item_count: usize,
    pub high_priority_count: usize,
    pub medium_priority_count: usize,
    pub low_priority_count: usize,
    pub deferred_count: usize,
    pub finding_item_count: usize,
    pub gap_item_count: usize,
    pub ambiguity_item_count: usize,
    pub drift_item_count: usize,
    pub readiness_item_count: usize,
    pub policy_item_count: usize,
    pub blocker_count: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationPlanItem {
    pub id: String,
    pub rank: usize,
    pub priority: &'static str,
    pub severity: &'static str,
    pub category: &'static str,
    pub title: String,
    pub summary: String,
    pub detail: String,
    pub affected_agent: Option<String>,
    pub affected_skill: Option<RemediationAffectedSkill>,
    pub affected_capability: Option<String>,
    pub affected_task: Option<String>,
    pub affected_instance_ids: Vec<String>,
    pub suggested_safe_next_action: String,
    pub prerequisites: Vec<String>,
    pub blockers: Vec<String>,
    pub deferred: bool,
    pub evidence_refs: Vec<String>,
    pub side_effect_flags: Vec<&'static str>,
    pub safety_flags: RemediationPlanSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationAffectedSkill {
    pub instance_id: String,
    pub definition_id: String,
    pub skill_name: String,
    pub agent: String,
    pub scope: String,
    pub enabled: bool,
    pub state: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationPriorityRow {
    pub priority: &'static str,
    pub severity: &'static str,
    pub item_count: usize,
    pub category_counts: BTreeMap<String, usize>,
    pub top_item_ids: Vec<String>,
}

pub type RemediationPlanPromptRequest = AgentReadinessPromptRequest;
pub type RemediationPlanSafetyFlags = AgentReadinessSafetyFlags;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RemediationPreviewDraftsParams {
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default, alias = "task_text", alias = "user_intent")]
    pub task: Option<String>,
    #[serde(default, alias = "instance_ids", alias = "candidate_instance_ids")]
    pub skill_ids: Vec<String>,
    #[serde(default)]
    pub finding_ids: Vec<String>,
    #[serde(default)]
    pub draft_types: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub include_policy_drafts: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationPreviewDraftsResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: RemediationPreviewDraftsFilters,
    pub summary: RemediationPreviewDraftsSummary,
    pub draft_items: Vec<RemediationDraftItem>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: RemediationPreviewDraftsPromptRequest,
    pub safety_flags: RemediationPreviewDraftsSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationPreviewDraftsFilters {
    pub agent: Option<String>,
    pub task: Option<String>,
    pub skill_ids: Vec<String>,
    pub finding_ids: Vec<String>,
    pub draft_types: Vec<String>,
    pub limit: usize,
    pub include_policy_drafts: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationPreviewDraftsSummary {
    pub total_draft_count: usize,
    pub returned_draft_count: usize,
    pub frontmatter_count: usize,
    pub description_count: usize,
    pub permissions_count: usize,
    pub dependency_count: usize,
    pub policy_count: usize,
    pub high_confidence_count: usize,
    pub medium_confidence_count: usize,
    pub low_confidence_count: usize,
    pub blocker_count: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationDraftItem {
    pub id: String,
    pub rank: usize,
    pub title: String,
    pub draft_type: &'static str,
    pub agent: Option<String>,
    pub affected_skill: Option<RemediationAffectedSkill>,
    pub finding_id: Option<String>,
    pub rule_id: Option<String>,
    pub current_text: Option<String>,
    pub proposed_text: String,
    pub patch_like_snippet: String,
    pub rationale: String,
    pub confidence: u8,
    pub confidence_band: &'static str,
    pub copy_label: String,
    pub edit_guidance: String,
    pub evidence_refs: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub side_effect_flags: Vec<&'static str>,
    pub safety_flags: RemediationPreviewDraftsSafetyFlags,
}

pub type RemediationPreviewDraftsPromptRequest = AgentReadinessPromptRequest;
pub type RemediationPreviewDraftsSafetyFlags = AgentReadinessSafetyFlags;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RemediationPreviewImpactParams {
    #[serde(default)]
    pub action: Option<String>,
    #[serde(default, alias = "task_text", alias = "user_intent")]
    pub task: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default, alias = "workspace_path")]
    pub project_root: Option<String>,
    #[serde(default, alias = "instance_ids")]
    pub skill_ids: Vec<String>,
    #[serde(default)]
    pub candidate_instance_ids: Vec<String>,
    #[serde(default)]
    pub draft_ids: Vec<String>,
    #[serde(default)]
    pub plan_item_ids: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub include_snapshot_plan: bool,
    #[serde(default)]
    pub include_rollback_plan: bool,
    #[serde(default)]
    pub include_risk_impact: bool,
    #[serde(default)]
    pub include_task_impact: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationPreviewImpactResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: RemediationPreviewImpactFilters,
    pub summary: RemediationPreviewImpactSummary,
    pub impact_rows: Vec<RemediationImpactRow>,
    pub task_impact_rows: Vec<RemediationTaskImpactRow>,
    pub agent_impact_rows: Vec<RemediationAgentImpactRow>,
    pub skill_impact_rows: Vec<RemediationSkillImpactRow>,
    pub risk_delta_rows: Vec<RemediationRiskDeltaRow>,
    pub snapshot_rollback_plan_rows: Vec<RemediationSnapshotRollbackPlanRow>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: RemediationPreviewImpactPromptRequest,
    pub safety_flags: RemediationPreviewImpactSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationPreviewImpactFilters {
    pub action: String,
    pub task: Option<String>,
    pub agent: Option<String>,
    pub project_root: Option<String>,
    pub skill_ids: Vec<String>,
    pub candidate_instance_ids: Vec<String>,
    pub draft_ids: Vec<String>,
    pub plan_item_ids: Vec<String>,
    pub limit: usize,
    pub include_snapshot_plan: bool,
    pub include_rollback_plan: bool,
    pub include_risk_impact: bool,
    pub include_task_impact: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationPreviewImpactSummary {
    pub total_impact_count: usize,
    pub returned_impact_count: usize,
    pub task_impact_count: usize,
    pub agent_impact_count: usize,
    pub skill_impact_count: usize,
    pub risk_delta_count: usize,
    pub snapshot_plan_count: usize,
    pub rollback_plan_count: usize,
    pub blocker_count: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationImpactRow {
    pub id: String,
    pub rank: usize,
    pub area: &'static str,
    pub title: String,
    pub summary: String,
    pub action_intent: String,
    pub expected_direction: &'static str,
    pub confidence: u8,
    pub confidence_band: &'static str,
    pub affected_agent: Option<String>,
    pub affected_skill: Option<RemediationAffectedSkill>,
    pub affected_task: Option<String>,
    pub evidence_refs: Vec<String>,
    pub blockers: Vec<String>,
    pub side_effect_flags: Vec<&'static str>,
    pub safety_flags: RemediationPreviewImpactSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationTaskImpactRow {
    pub task: String,
    pub action_intent: String,
    pub expected_direction: &'static str,
    pub readiness_score_before: Option<u8>,
    pub readiness_score_after_estimate: Option<u8>,
    pub routing_confidence_before: Option<u8>,
    pub routing_confidence_after_estimate: Option<u8>,
    pub notes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationAgentImpactRow {
    pub agent: String,
    pub action_intent: String,
    pub expected_direction: &'static str,
    pub impacted_skill_count: usize,
    pub enabled_before_count: usize,
    pub enabled_after_estimate_count: usize,
    pub writable_status: Option<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationSkillImpactRow {
    pub affected_skill: RemediationAffectedSkill,
    pub action_intent: String,
    pub expected_direction: &'static str,
    pub enabled_before: bool,
    pub enabled_after_estimate: bool,
    pub finding_count: usize,
    pub conflict_count: usize,
    pub analysis_count: usize,
    pub notes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationRiskDeltaRow {
    pub id: String,
    pub source: &'static str,
    pub severity: String,
    pub title: String,
    pub current_risk: &'static str,
    pub expected_risk_after: &'static str,
    pub expected_direction: &'static str,
    pub affected_instance_ids: Vec<String>,
    pub blockers: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationSnapshotRollbackPlanRow {
    pub id: String,
    pub agent: String,
    pub instance_id: String,
    pub skill_name: String,
    pub action_intent: String,
    pub snapshot_required: bool,
    pub rollback_available: bool,
    pub verified_writable: bool,
    pub blocked_reason: Option<String>,
    pub plan_only: bool,
    pub evidence_refs: Vec<String>,
}

pub type RemediationPreviewImpactPromptRequest = AgentReadinessPromptRequest;
pub type RemediationPreviewImpactSafetyFlags = AgentReadinessSafetyFlags;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RemediationBatchReviewParams {
    #[serde(default, alias = "task_text", alias = "user_intent")]
    pub task: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default, alias = "workspace_path")]
    pub project_root: Option<String>,
    #[serde(default, alias = "workspace")]
    pub workspace_label: Option<String>,
    #[serde(default)]
    pub rule_id: Option<String>,
    #[serde(default, alias = "risk")]
    pub severity: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default, alias = "triage")]
    pub triage_status: Option<String>,
    #[serde(default, alias = "instance_ids")]
    pub candidate_instance_ids: Vec<String>,
    #[serde(default)]
    pub group_by: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationBatchReviewResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: RemediationBatchReviewFilters,
    pub summary: RemediationBatchReviewSummary,
    pub review_groups: Vec<RemediationBatchReviewGroup>,
    pub review_items: Vec<RemediationBatchReviewItem>,
    pub recommended_next_step_labels: Vec<String>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: RemediationBatchReviewPromptRequest,
    pub aggregation: AggregationRuntimeMetadata,
    pub safety_flags: RemediationBatchReviewSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationBatchReviewFilters {
    pub task: Option<String>,
    pub agent: Option<String>,
    pub project_root: Option<String>,
    pub workspace_label: Option<String>,
    pub rule_id: Option<String>,
    pub severity: Option<String>,
    pub status: Option<String>,
    pub triage_status: Option<String>,
    pub candidate_instance_ids: Vec<String>,
    pub group_by: Vec<String>,
    pub limit: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationBatchReviewSummary {
    pub total_item_count: usize,
    pub returned_item_count: usize,
    pub group_count: usize,
    pub high_risk_count: usize,
    pub medium_risk_count: usize,
    pub low_risk_count: usize,
    pub task_group_count: usize,
    pub agent_group_count: usize,
    pub workspace_group_count: usize,
    pub rule_group_count: usize,
    pub blocker_count: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationBatchReviewGroup {
    pub id: String,
    pub group_type: &'static str,
    pub label: String,
    pub item_count: usize,
    pub high_risk_count: usize,
    pub medium_risk_count: usize,
    pub low_risk_count: usize,
    pub top_item_ids: Vec<String>,
    pub recommended_next_step_label: String,
    pub blocker_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationBatchReviewItem {
    pub id: String,
    pub rank: usize,
    pub source: &'static str,
    pub source_id: String,
    pub title: String,
    pub summary: String,
    pub risk: &'static str,
    pub severity: String,
    pub status: String,
    pub triage_status: Option<String>,
    pub rule_id: Option<String>,
    pub task: Option<String>,
    pub agent: Option<String>,
    pub workspace: Option<String>,
    pub affected_skill: Option<RemediationAffectedSkill>,
    pub affected_instance_ids: Vec<String>,
    pub recommended_next_step_label: String,
    pub blocker_notes: Vec<String>,
    pub gap_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub side_effect_flags: Vec<&'static str>,
    pub safety_flags: RemediationBatchReviewSafetyFlags,
}

pub type RemediationBatchReviewPromptRequest = AgentReadinessPromptRequest;
pub type RemediationBatchReviewSafetyFlags = AgentReadinessSafetyFlags;

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DetectStaleDriftParams {
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default, alias = "instance_ids")]
    pub candidate_instance_ids: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub stale_days: Option<u32>,
    #[serde(default)]
    pub thresholds: StaleDriftThresholds,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct StaleDriftThresholds {
    #[serde(default)]
    pub stale_days: Option<u32>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StaleDriftDetectionResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: StaleDriftFilters,
    pub summary: StaleDriftSummary,
    pub stale_drift_rows: Vec<StaleDriftRow>,
    pub readiness_impact_rows: Vec<StaleDriftReadinessImpactRow>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_references: Vec<TaskReadinessEvidenceReference>,
    pub prompt_request: StaleDriftPromptRequest,
    pub safety_flags: StaleDriftSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct StaleDriftFilters {
    pub agent: Option<String>,
    pub candidate_instance_ids: Vec<String>,
    pub limit: usize,
    pub stale_days: u32,
}

#[derive(Debug, Clone, Serialize)]
pub struct StaleDriftSummary {
    pub scanned_skill_count: usize,
    pub returned_row_count: usize,
    pub stale_count: usize,
    pub drift_count: usize,
    pub high_risk_count: usize,
    pub medium_risk_count: usize,
    pub low_risk_count: usize,
    pub missing_history_count: usize,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct StaleDriftRow {
    pub rank: usize,
    pub instance_id: String,
    pub definition_id: String,
    pub skill_name: String,
    pub agent: String,
    pub scope: String,
    pub enabled: bool,
    pub state: String,
    pub stale_drift_score: u8,
    pub stale_drift_band: &'static str,
    pub drift_signals: StaleDriftSignals,
    pub readiness_impact: Option<StaleDriftReadinessImpact>,
    pub reasons: Vec<String>,
    pub gap_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub safety_flags: StaleDriftSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct StaleDriftSignals {
    pub fingerprint_drift: bool,
    pub finding_drift: bool,
    pub source_drift: bool,
    pub modified_age_days: Option<i64>,
    pub stale_by_mtime: bool,
    pub missing_mtime: bool,
    pub missing_previous_scan: bool,
    pub related_finding_count: usize,
    pub related_conflict_count: usize,
    pub related_analysis_count: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct StaleDriftReadinessImpact {
    pub impact_level: &'static str,
    pub readiness_risk_score: u8,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct StaleDriftReadinessImpactRow {
    pub instance_id: String,
    pub skill_name: String,
    pub agent: String,
    pub impact_level: &'static str,
    pub stale_drift_score: u8,
    pub notes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

pub type StaleDriftPromptRequest = AgentReadinessPromptRequest;
pub type StaleDriftSafetyFlags = AgentReadinessSafetyFlags;

#[derive(Debug, Clone, Serialize)]
pub struct SkillRouteCandidate {
    pub rank: usize,
    pub instance_id: String,
    pub definition_id: String,
    pub skill_name: String,
    pub agent: String,
    pub scope: String,
    pub enabled: bool,
    pub state: String,
    pub confidence_score: u8,
    pub confidence_band: &'static str,
    pub readiness_score: u8,
    pub readiness_band: &'static str,
    pub quality_score: Option<u8>,
    pub match_reasons: Vec<String>,
    pub confidence_rationale: Vec<String>,
    pub ambiguity_warnings: Vec<String>,
    pub likely_wrong_pick_risks: Vec<String>,
    pub likely_miss_risks: Vec<String>,
    pub enabled_scope_risk_state: TaskReadinessState,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingConfidencePromptRequest {
    pub available: bool,
    pub preview_method: &'static str,
    pub confirm_method: &'static str,
    pub action: &'static str,
    pub request: LlmPreviewPromptParams,
    pub note: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingConfidenceSafetyFlags {
    pub read_only: bool,
    pub provider_request_sent: bool,
    pub write_back_allowed: bool,
    pub script_execution_allowed: bool,
    pub config_mutation_allowed: bool,
    pub snapshot_created: bool,
    pub triage_mutation_allowed: bool,
    pub credential_accessed: bool,
    pub raw_secret_returned: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TaskBenchmarkRecord {
    pub id: String,
    pub title: String,
    #[serde(alias = "task_text", alias = "user_intent")]
    pub task: String,
    #[serde(default)]
    pub expected_skill_refs: Vec<String>,
    #[serde(default)]
    pub expected_skill_names: Vec<String>,
    #[serde(default)]
    pub acceptable_agents: Vec<String>,
    #[serde(default)]
    pub acceptable_scopes: Vec<String>,
    #[serde(default)]
    pub success_criteria: Vec<String>,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct ListTaskBenchmarksParams {
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveTaskBenchmarkParams {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default, alias = "name")]
    pub title: Option<String>,
    #[serde(alias = "task_text", alias = "user_intent")]
    pub task: String,
    #[serde(default)]
    pub expected_skill_refs: Vec<String>,
    #[serde(default)]
    pub expected_skill_names: Vec<String>,
    #[serde(default)]
    pub acceptable_agents: Vec<String>,
    #[serde(default)]
    pub acceptable_scopes: Vec<String>,
    #[serde(default)]
    pub success_criteria: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct DeleteTaskBenchmarkParams {
    #[serde(alias = "benchmark_id")]
    pub id: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct EvaluateTaskBenchmarksParams {
    #[serde(default, alias = "benchmark_ids")]
    pub ids: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskBenchmarkListResult {
    pub benchmarks: Vec<TaskBenchmarkRecord>,
    pub count: usize,
    pub app_local_only: bool,
    pub provider_request_sent: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveTaskBenchmarkResult {
    pub benchmark: TaskBenchmarkRecord,
    pub created: bool,
    pub app_local_only: bool,
    pub provider_request_sent: bool,
    pub agent_config_mutated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct DeleteTaskBenchmarkResult {
    pub benchmark_id: String,
    pub deleted: bool,
    pub remaining_count: usize,
    pub app_local_only: bool,
    pub provider_request_sent: bool,
    pub agent_config_mutated: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskBenchmarkEvaluationResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub evaluated_count: usize,
    pub summary: String,
    pub benchmark_results: Vec<TaskBenchmarkEvaluationItem>,
    pub blocker_notes: Vec<String>,
    pub prompt_request: TaskBenchmarkPromptRequest,
    pub safety_flags: TaskBenchmarkSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskBenchmarkEvaluationItem {
    pub benchmark_id: String,
    pub title: String,
    pub task: String,
    pub score: u8,
    pub band: &'static str,
    pub expected_match_status: &'static str,
    pub expected_match_reasons: Vec<String>,
    pub top_route: Option<TaskBenchmarkRouteSummary>,
    pub route_confidence_score: u8,
    pub route_confidence_band: &'static str,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub safety_flags: TaskBenchmarkSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskBenchmarkRouteSummary {
    pub instance_id: String,
    pub definition_id: String,
    pub skill_name: String,
    pub agent: String,
    pub scope: String,
    pub confidence_score: u8,
    pub confidence_band: &'static str,
    pub readiness_score: u8,
    pub readiness_band: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct TaskBenchmarkPromptRequest {
    pub available: bool,
    pub preview_method: &'static str,
    pub confirm_method: &'static str,
    pub action: &'static str,
    pub request: LlmPreviewPromptParams,
    pub note: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TaskBenchmarkSafetyFlags {
    pub read_only: bool,
    pub provider_request_sent: bool,
    pub write_back_allowed: bool,
    pub script_execution_allowed: bool,
    pub config_mutation_allowed: bool,
    pub snapshot_created: bool,
    pub triage_mutation_allowed: bool,
    pub credential_accessed: bool,
    pub raw_secret_returned: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct SaveRoutingBaselineParams {
    #[serde(default, alias = "benchmark_ids")]
    pub ids: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct DetectRoutingRegressionParams {
    #[serde(default, alias = "benchmark_ids")]
    pub ids: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub score_drop_threshold: Option<u8>,
    #[serde(default)]
    pub confidence_drop_threshold: Option<u8>,
}

#[derive(Debug, Clone, Serialize)]
pub struct SaveRoutingBaselineResult {
    pub generated_by: &'static str,
    pub baseline: RoutingRegressionBaseline,
    pub benchmark_count: usize,
    pub app_local_only: bool,
    pub baseline_file: &'static str,
    pub provider_request_sent: bool,
    pub agent_config_mutated: bool,
    pub skill_files_mutated: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingRegressionDetectionResult {
    pub generated_by: &'static str,
    pub status: &'static str,
    pub baseline_available: bool,
    pub catalog_available: bool,
    pub baseline_evaluated_count: usize,
    pub current_evaluated_count: usize,
    pub regression_count: usize,
    pub missing_benchmark_count: usize,
    pub summary: String,
    pub items: Vec<RoutingRegressionItem>,
    pub blocker_notes: Vec<String>,
    pub baseline: Option<RoutingRegressionBaseline>,
    pub current_evaluation: TaskBenchmarkEvaluationResult,
    pub safety_flags: TaskBenchmarkSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingRegressionItem {
    pub benchmark_id: String,
    pub title: String,
    pub status: &'static str,
    pub regression: bool,
    pub reasons: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub score_delta: Option<i16>,
    pub confidence_delta: Option<i16>,
    pub baseline: Option<RoutingRegressionComparisonFields>,
    pub current: Option<RoutingRegressionComparisonFields>,
    pub safety_flags: TaskBenchmarkSafetyFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRegressionBaseline {
    pub schema_version: u32,
    pub generated_by: String,
    pub generated_at: i64,
    pub catalog_available: bool,
    pub evaluated_count: usize,
    pub benchmark_results: Vec<RoutingRegressionBaselineItem>,
    pub safety_flags: TaskBenchmarkSafetyFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RoutingRegressionBaselineItem {
    pub benchmark_id: String,
    pub title: String,
    pub task: String,
    pub score: u8,
    pub band: String,
    pub expected_match_status: String,
    pub top_route: Option<RoutingRegressionRouteSnapshot>,
    pub route_confidence_score: u8,
    pub route_confidence_band: String,
    pub gap_count: usize,
    pub blocker_count: usize,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct RoutingRegressionRouteSnapshot {
    pub instance_id: String,
    pub definition_id: String,
    pub skill_name: String,
    pub agent: String,
    pub scope: String,
    pub confidence_score: u8,
    pub confidence_band: String,
    pub readiness_score: u8,
    pub readiness_band: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingRegressionComparisonFields {
    pub task: String,
    pub expected_match_status: String,
    pub score: u8,
    pub band: String,
    pub top_route: Option<RoutingRegressionRouteSnapshot>,
    pub route_confidence_score: u8,
    pub route_confidence_band: String,
    pub gap_count: usize,
    pub blocker_count: usize,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct RoutingAccuracyDashboardParams {
    #[serde(default, alias = "target_agent")]
    pub agent: Option<String>,
    #[serde(default, alias = "days")]
    pub window_days: Option<u32>,
    #[serde(default)]
    pub limit: Option<usize>,
    #[serde(default)]
    pub include_history: bool,
    #[serde(default)]
    pub include_recent_evidence: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingAccuracyDashboardResult {
    pub generated_by: &'static str,
    pub catalog_available: bool,
    pub filters: RoutingAccuracyDashboardFilters,
    pub summary: RoutingAccuracyDashboardSummary,
    pub agent_rows: Vec<RoutingAccuracyAgentRow>,
    pub history_rows: Vec<RoutingAccuracyHistoryRow>,
    pub gap_issue_rows: Vec<RoutingAccuracyIssueRow>,
    pub recent_evidence_rows: Vec<RoutingAccuracyEvidenceRow>,
    pub blocker_notes: Vec<String>,
    pub prompt_request: RoutingAccuracyPromptRequest,
    pub safety_flags: RoutingAccuracySafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingAccuracyDashboardFilters {
    pub agent: Option<String>,
    pub window_days: u32,
    pub limit: usize,
    pub include_history: bool,
    pub include_recent_evidence: bool,
    pub window_start_millis: i64,
    pub window_end_millis: i64,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RoutingAccuracyDashboardSummary {
    pub trace_count: usize,
    pub hit_count: usize,
    pub miss_count: usize,
    pub wrong_pick_count: usize,
    pub ambiguous_count: usize,
    pub unknown_count: usize,
    pub benchmark_count: usize,
    pub benchmark_matched_count: usize,
    pub benchmark_gap_count: usize,
    pub regression_count: usize,
    pub missing_benchmark_count: usize,
    pub accuracy_rate: f64,
    pub known_outcome_rate: f64,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RoutingAccuracyOutcomeCounts {
    pub hit: usize,
    pub miss: usize,
    pub wrong_pick: usize,
    pub ambiguous: usize,
    pub unknown: usize,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingAccuracyAgentRow {
    pub agent: String,
    pub trace_count: usize,
    pub outcomes: RoutingAccuracyOutcomeCounts,
    pub accuracy_rate: f64,
    pub benchmark_count: usize,
    pub benchmark_matched_count: usize,
    pub benchmark_gap_count: usize,
    pub regression_count: usize,
    pub recent_evidence_count: usize,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingAccuracyHistoryRow {
    pub unix_day: i64,
    pub trace_count: usize,
    pub outcomes: RoutingAccuracyOutcomeCounts,
    pub accuracy_rate: f64,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingAccuracyIssueRow {
    pub source: &'static str,
    pub severity: &'static str,
    pub agent: Option<String>,
    pub title: String,
    pub detail: String,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingAccuracyEvidenceRow {
    pub source: &'static str,
    pub agent: Option<String>,
    pub title: String,
    pub outcome: Option<String>,
    pub detail: String,
    pub evidence_refs: Vec<String>,
    pub observed_at: Option<i64>,
}

#[derive(Debug, Clone, Serialize)]
pub struct RoutingAccuracyPromptRequest {
    pub available: bool,
    pub preview_method: &'static str,
    pub confirm_method: &'static str,
    pub action: &'static str,
    pub request: LlmPreviewPromptParams,
    pub note: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct RoutingAccuracySafetyFlags {
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionSkillReviewRecord {
    pub id: String,
    pub title: String,
    pub source_kind: String,
    pub agent: Option<String>,
    pub task: Option<String>,
    pub trace_import_ids: Vec<String>,
    pub missing_trace_import_ids: Vec<String>,
    pub expected_skill_refs: Vec<String>,
    pub expected_skill_names: Vec<String>,
    pub excerpt: String,
    pub excerpt_char_count: usize,
    pub content_hash: String,
    #[serde(default = "agent_session_review_redaction_summary_default")]
    pub redaction_summary: AgentSessionSkillReviewRedactionSummary,
    pub reviewed_at: i64,
    pub analysis: AgentSessionSkillReviewAnalysis,
    #[serde(default = "agent_session_review_safety_flags")]
    pub safety_flags: AgentSessionSkillReviewSafetyFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionSkillReviewAnalysis {
    pub generated_by: String,
    pub catalog_available: bool,
    pub outcome: String,
    pub summary: String,
    pub reasons: Vec<String>,
    pub detected_skills: Vec<TraceDetectedSkill>,
    pub expected_skill_signals: Vec<AgentSessionExpectedSkillSignal>,
    pub referenced_traces: Vec<AgentSessionReferencedTrace>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionExpectedSkillSignal {
    pub kind: String,
    pub value: String,
    pub matched: bool,
    pub matched_instance_ids: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionReferencedTrace {
    pub id: String,
    pub title: String,
    pub outcome: String,
    pub imported_at: i64,
    pub detected_skill_count: usize,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct AgentSessionSkillReviewRedactionSummary {
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
pub struct AgentSessionSkillReviewSafetyFlags {
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
pub struct McpServerPreviewParams {
    #[serde(default, alias = "authorized_paths", alias = "config_paths")]
    pub authorized_config_paths: Vec<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpServerPreviewPath {
    pub path: String,
    pub status: String,
    pub server_count: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub blocker: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpServerPreviewRow {
    pub id: String,
    pub name: String,
    pub source_path: String,
    pub transport: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub command: Option<String>,
    pub args_count: usize,
    pub env_key_count: usize,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct McpServerPreviewResult {
    pub generated_by: &'static str,
    pub authorized: bool,
    pub authorization_required: bool,
    pub evidence_available: bool,
    pub evidence_insufficient: bool,
    pub authorized_paths: Vec<McpServerPreviewPath>,
    pub count: usize,
    pub server_rows: Vec<McpServerPreviewRow>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub redaction_summary: AgentSessionSkillReviewRedactionSummary,
    pub safety_flags: AgentSessionSkillReviewSafetyFlags,
    pub read_only: bool,
    pub provider_request_sent: bool,
    pub skill_files_mutated: bool,
    pub agent_config_mutated: bool,
    pub snapshot_created: bool,
    pub triage_mutated: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
    pub credential_accessed: bool,
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
    pub limit: Option<usize>,
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
    pub user_message_count: usize,
    pub total_message_count: usize,
    pub tool_call_count: usize,
    pub skill_call_count: usize,
    pub skill_usage_rows: Vec<LocalSessionSkillUsageRow>,
    pub session_rows: Vec<LocalSessionPreviewRow>,
    pub gap_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub redaction_summary: AgentSessionSkillReviewRedactionSummary,
    pub safety_flags: AgentSessionSkillReviewSafetyFlags,
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

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgentSessionSkillReviewParams {
    #[serde(
        default,
        alias = "trace_text",
        alias = "transcript",
        alias = "transcript_text"
    )]
    pub content: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default, alias = "task_text", alias = "user_intent")]
    pub task: Option<String>,
    #[serde(default, alias = "trace_ids", alias = "import_ids")]
    pub trace_import_ids: Vec<String>,
    #[serde(default)]
    pub expected_skill_refs: Vec<String>,
    #[serde(default)]
    pub expected_skill_names: Vec<String>,
    #[serde(default)]
    pub max_excerpt_chars: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct AgentSessionListSkillReviewsParams {
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub outcome: Option<String>,
    #[serde(default, alias = "trace_id", alias = "import_id")]
    pub trace_import_id: Option<String>,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct AgentSessionDeleteSkillReviewParams {
    #[serde(alias = "review_id")]
    pub id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentSessionSkillReviewResult {
    pub generated_by: &'static str,
    pub review: AgentSessionSkillReviewRecord,
    pub count: usize,
    pub app_local_only: bool,
    pub review_file: &'static str,
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
pub struct AgentSessionSkillReviewListResult {
    pub generated_by: &'static str,
    pub count: usize,
    pub total_count: usize,
    pub reviews: Vec<AgentSessionSkillReviewRecord>,
    pub app_local_only: bool,
    pub review_file: &'static str,
    pub provider_request_sent: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
    pub safety_flags: AgentSessionSkillReviewSafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct AgentSessionSkillReviewDeleteResult {
    pub review_id: String,
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

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceImportRecord {
    pub id: String,
    pub title: String,
    pub source_kind: String,
    pub agent: Option<String>,
    pub task: Option<String>,
    pub expected_skill_refs: Vec<String>,
    pub expected_skill_names: Vec<String>,
    pub excerpt: String,
    pub excerpt_char_count: usize,
    #[serde(default = "trace_import_redaction_summary_default")]
    pub redaction_summary: TraceImportRedactionSummary,
    pub content_hash: String,
    pub imported_at: i64,
    pub analysis: TraceImportAnalysis,
    pub safety_flags: TraceImportSafetyFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceImportRedactionSummary {
    pub status: String,
    pub redacted_value_count: usize,
    pub redacted_fields: Vec<String>,
    pub placeholders: Vec<String>,
    pub raw_trace_persisted: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_secret_returned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceImportAnalysis {
    pub generated_by: String,
    pub catalog_available: bool,
    pub outcome: String,
    pub reasons: Vec<String>,
    pub detected_skills: Vec<TraceDetectedSkill>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TraceDetectedSkill {
    pub instance_id: String,
    pub definition_id: String,
    pub skill_name: String,
    pub agent: String,
    pub scope: String,
    pub evidence_refs: Vec<String>,
    pub match_terms: Vec<String>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
pub struct TraceImportSafetyFlags {
    pub read_only: bool,
    pub app_local_only: bool,
    pub provider_request_sent: bool,
    pub write_back_allowed: bool,
    pub skill_files_mutated: bool,
    pub agent_config_mutated: bool,
    pub script_execution_allowed: bool,
    pub config_mutation_allowed: bool,
    pub snapshot_created: bool,
    pub triage_mutation_allowed: bool,
    pub credential_accessed: bool,
    pub raw_secret_returned: bool,
    pub raw_trace_persisted: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub cloud_sync_performed: bool,
    pub telemetry_emitted: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TraceImportLocalParams {
    #[serde(default, alias = "trace_text", alias = "transcript")]
    pub content: String,
    #[serde(default)]
    pub title: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub agent: Option<String>,
    #[serde(default)]
    pub task: Option<String>,
    #[serde(default)]
    pub expected_skill_refs: Vec<String>,
    #[serde(default)]
    pub expected_skill_names: Vec<String>,
    #[serde(default)]
    pub max_excerpt_chars: Option<usize>,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct TraceListImportsParams {
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct TraceDeleteImportParams {
    #[serde(alias = "import_id")]
    pub id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceImportLocalResult {
    pub generated_by: &'static str,
    pub import: TraceImportRecord,
    pub count: usize,
    pub app_local_only: bool,
    pub import_file: &'static str,
    pub provider_request_sent: bool,
    pub raw_trace_persisted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceImportListResult {
    pub imports: Vec<TraceImportRecord>,
    pub count: usize,
    pub app_local_only: bool,
    pub provider_request_sent: bool,
    pub raw_trace_persisted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct TraceDeleteImportResult {
    pub import_id: String,
    pub deleted: bool,
    pub remaining_count: usize,
    pub app_local_only: bool,
    pub provider_request_sent: bool,
    pub raw_trace_persisted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationHistoryRecord {
    pub id: String,
    pub title: String,
    pub decision: String,
    pub status: String,
    pub source_kind: String,
    pub source_method: Option<String>,
    pub source_item_refs: Vec<String>,
    pub batch_review_item_ids: Vec<String>,
    pub agent: Option<String>,
    pub workspace: Option<String>,
    pub task: Option<String>,
    pub rule_ids: Vec<String>,
    pub risk_levels: Vec<String>,
    pub recurrence_key: Option<String>,
    pub recurrence_count_marker: Option<u32>,
    pub reopened: bool,
    pub reopened_from_ids: Vec<String>,
    pub readiness_improvement_notes: Vec<String>,
    pub routing_improvement_notes: Vec<String>,
    pub blocker_notes: Vec<String>,
    pub gap_notes: Vec<String>,
    pub evidence_refs: Vec<String>,
    pub notes: Option<String>,
    #[serde(default = "remediation_history_redaction_summary_default")]
    pub redaction_summary: RemediationHistoryRedactionSummary,
    pub created_at: i64,
    pub updated_at: i64,
    pub safety_flags: RemediationHistorySafetyFlags,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RemediationHistoryRedactionSummary {
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
pub struct RemediationHistorySafetyFlags {
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
    pub rollback_performed: bool,
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
pub struct RemediationHistoryListParams {
    #[serde(default, alias = "target_agent")]
    pub agent: Option<String>,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub decision: Option<String>,
    #[serde(default, alias = "batch_item_id")]
    pub source_item_ref: Option<String>,
    #[serde(default)]
    pub recurrence_key: Option<String>,
    #[serde(default)]
    pub include_recurrence_rows: bool,
    #[serde(default)]
    pub limit: Option<usize>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemediationHistoryRecordParams {
    #[serde(default)]
    pub id: Option<String>,
    #[serde(default)]
    pub title: Option<String>,
    pub decision: String,
    #[serde(default)]
    pub status: Option<String>,
    #[serde(default)]
    pub source_kind: Option<String>,
    #[serde(default)]
    pub source_method: Option<String>,
    #[serde(default, alias = "source_item_ids")]
    pub source_item_refs: Vec<String>,
    #[serde(default, alias = "batch_review_ids")]
    pub batch_review_item_ids: Vec<String>,
    #[serde(default, alias = "target_agent")]
    pub agent: Option<String>,
    #[serde(default, alias = "project_root", alias = "workspace_root")]
    pub workspace: Option<String>,
    #[serde(default, alias = "task_text", alias = "user_intent")]
    pub task: Option<String>,
    #[serde(default)]
    pub rule_ids: Vec<String>,
    #[serde(default)]
    pub risk_levels: Vec<String>,
    #[serde(default)]
    pub recurrence_key: Option<String>,
    #[serde(default)]
    pub recurrence_count_marker: Option<u32>,
    #[serde(default)]
    pub reopened: Option<bool>,
    #[serde(default, alias = "reopened_from")]
    pub reopened_from_ids: Vec<String>,
    #[serde(default)]
    pub readiness_improvement_notes: Vec<String>,
    #[serde(default)]
    pub routing_improvement_notes: Vec<String>,
    #[serde(default)]
    pub blocker_notes: Vec<String>,
    #[serde(default)]
    pub gap_notes: Vec<String>,
    #[serde(default)]
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub notes: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct RemediationHistoryDeleteParams {
    #[serde(alias = "history_id")]
    pub id: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationHistoryListResult {
    pub generated_by: &'static str,
    pub filters: RemediationHistoryFilters,
    pub summary: RemediationHistorySummary,
    pub records: Vec<RemediationHistoryRecord>,
    pub recurrence_rows: Vec<RemediationHistoryRecurrenceRow>,
    pub blocker_notes: Vec<String>,
    pub app_local_only: bool,
    pub history_file: &'static str,
    pub provider_request_sent: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
    pub safety_flags: RemediationHistorySafetyFlags,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationHistoryRecordResult {
    pub generated_by: &'static str,
    pub record: RemediationHistoryRecord,
    pub created: bool,
    pub count: usize,
    pub app_local_only: bool,
    pub history_file: &'static str,
    pub provider_request_sent: bool,
    pub skill_files_mutated: bool,
    pub agent_config_mutated: bool,
    pub snapshot_created: bool,
    pub rollback_performed: bool,
    pub triage_mutated: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationHistoryDeleteResult {
    pub history_id: String,
    pub deleted: bool,
    pub remaining_count: usize,
    pub app_local_only: bool,
    pub provider_request_sent: bool,
    pub skill_files_mutated: bool,
    pub agent_config_mutated: bool,
    pub snapshot_created: bool,
    pub rollback_performed: bool,
    pub triage_mutated: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationHistoryFilters {
    pub agent: Option<String>,
    pub status: Option<String>,
    pub decision: Option<String>,
    pub source_item_ref: Option<String>,
    pub recurrence_key: Option<String>,
    pub limit: usize,
    pub include_recurrence_rows: bool,
}

#[derive(Debug, Clone, Serialize, Default)]
pub struct RemediationHistorySummary {
    pub total_count: usize,
    pub returned_count: usize,
    pub decision_counts: BTreeMap<String, usize>,
    pub status_counts: BTreeMap<String, usize>,
    pub reopened_count: usize,
    pub recurrence_group_count: usize,
    pub blocker_count: usize,
    pub readiness_improvement_count: usize,
    pub routing_improvement_count: usize,
    pub latest_recorded_at: Option<i64>,
    pub summary: String,
}

#[derive(Debug, Clone, Serialize)]
pub struct RemediationHistoryRecurrenceRow {
    pub recurrence_key: String,
    pub record_count: usize,
    pub reopened_count: usize,
    pub latest_status: String,
    pub latest_decision: String,
    pub latest_recorded_at: i64,
    pub source_item_refs: Vec<String>,
    pub evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmPrepareActionParams {
    pub kind: LlmActionKind,
    #[serde(default, alias = "instance_id")]
    pub skill_instance_id: Option<String>,
    #[serde(default)]
    pub user_intent: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmPrepareSkillAnalysisParams {
    #[serde(default)]
    pub instance_ids: Vec<String>,
    pub analysis_kind: LlmSkillAnalysisKind,
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
    pub analysis_kind: Option<LlmSkillAnalysisKind>,
    #[serde(default)]
    pub user_intent: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LlmConfirmPromptAndSendParams {
    pub preview_id: String,
    pub confirmation_id: String,
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
pub enum LlmSkillAnalysisKind {
    Overview,
    Risk,
    Cleanup,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum LlmPromptActionKind {
    Analyze,
    Recommend,
    ExplainConflict,
    DraftFrontmatter,
    SkillAnalysis,
    QualityScore,
    StaleDriftDetection,
    KnowledgeSearch,
    SimilarSkillGrouping,
    CapabilityTaxonomy,
    LocalSkillMap,
    WorkspaceReadiness,
    RemediationPlan,
    RemediationPreviewDrafts,
    RemediationPreviewImpact,
    RemediationBatchReview,
    GuidedCleanupFlow,
    TaskReadiness,
    RoutingConfidence,
    TaskCockpit,
    SkillLifecycleTimeline,
}

impl LlmPromptActionKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Analyze => "analyze",
            Self::Recommend => "recommend",
            Self::ExplainConflict => "explain_conflict",
            Self::DraftFrontmatter => "draft_frontmatter",
            Self::SkillAnalysis => "skill_analysis",
            Self::QualityScore => "quality_score",
            Self::StaleDriftDetection => "stale_drift_detection",
            Self::KnowledgeSearch => "knowledge_search",
            Self::SimilarSkillGrouping => "similar_skill_grouping",
            Self::CapabilityTaxonomy => "capability_taxonomy",
            Self::LocalSkillMap => "local_skill_map",
            Self::WorkspaceReadiness => "workspace_readiness",
            Self::RemediationPlan => "remediation_plan",
            Self::RemediationPreviewDrafts => "remediation_preview_drafts",
            Self::RemediationPreviewImpact => "remediation_preview_impact",
            Self::RemediationBatchReview => "remediation_batch_review",
            Self::GuidedCleanupFlow => "guided_cleanup_flow",
            Self::TaskReadiness => "task_readiness",
            Self::RoutingConfidence => "routing_confidence",
            Self::TaskCockpit => "task_cockpit",
            Self::SkillLifecycleTimeline => "skill_lifecycle_timeline",
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

impl LlmSkillAnalysisKind {
    fn as_str(self) -> &'static str {
        match self {
            Self::Overview => "overview",
            Self::Risk => "risk",
            Self::Cleanup => "cleanup",
        }
    }

    fn output_token_estimate(self) -> u32 {
        match self {
            Self::Overview => 650,
            Self::Risk => 800,
            Self::Cleanup => 700,
        }
    }
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
    pub review_preview: LlmReviewPreview,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmPrepareSkillAnalysisResult {
    pub enabled: bool,
    pub disabled_reason: String,
    pub analysis_kind: &'static str,
    pub selected_skill_count: usize,
    pub included_skill_count: usize,
    pub excluded_missing_count: usize,
    pub included_skills: Vec<LlmSkillAnalysisIncludedSkill>,
    pub prompt_draft: String,
    pub summary_draft: String,
    pub safety_flags: LlmSkillAnalysisSafetyFlags,
    pub estimated_input_tokens: u32,
    pub estimated_output_tokens: u32,
    pub estimated_total_tokens: u32,
    pub provider_request_sent: bool,
    pub generated_by: &'static str,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmPreviewPromptResult {
    pub preview_id: String,
    pub status: String,
    pub allowed: bool,
    pub reason: String,
    pub action: &'static str,
    pub profile_id: Option<String>,
    pub provider: Option<String>,
    pub model: Option<String>,
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
    pub action: &'static str,
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
    pub raw_secret_returned: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
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
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmProviderObservabilityResult {
    pub generated_by: &'static str,
    pub status: String,
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
    pub session_review_ids: Vec<String>,
    #[serde(default)]
    pub trace_import_ids: Vec<String>,
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
    pub session_review_ids: Vec<String>,
    pub trace_import_ids: Vec<String>,
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
pub struct LlmSkillAnalysisIncludedSkill {
    pub instance_id: String,
    pub name: String,
    pub agent: String,
    pub scope: String,
    pub enabled: bool,
    pub disabled_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmSkillAnalysisSafetyFlags {
    pub write_back_enabled: bool,
    pub script_execution_enabled: bool,
    pub credential_storage_enabled: bool,
    pub confirmation_required: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmConfirmationRequirement {
    pub required: bool,
    pub message: String,
    pub display_fields: Vec<&'static str>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmReviewPreview {
    pub status: &'static str,
    pub generated_by: &'static str,
    pub provider_request_sent: bool,
    pub write_actions_available: bool,
    pub execution_actions_available: bool,
    pub purpose: String,
    pub risk: LlmReviewRisk,
    pub finding_explanations: Vec<LlmReviewFindingExplanation>,
    pub cross_agent_fit: LlmReviewCrossAgentFit,
    pub redaction: LlmReviewRedaction,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmReviewRisk {
    pub level: &'static str,
    pub summary: String,
    pub signals: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmReviewFindingExplanation {
    pub rule_id: String,
    pub severity: String,
    pub explanation: String,
    pub suggested_next_step: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmReviewCrossAgentFit {
    pub agent: String,
    pub scope: String,
    pub comparable_instance_count: usize,
    pub summary: String,
    pub notes: Vec<String>,
}

#[derive(Debug, Clone, Serialize)]
pub struct LlmReviewRedaction {
    pub skill_body_returned: bool,
    pub paths_returned: bool,
    pub credentials_returned: bool,
    pub included_fields: Vec<&'static str>,
    pub excluded_fields: Vec<&'static str>,
}

impl LlmReviewPreview {
    fn unavailable() -> Self {
        Self {
            status: "unavailable",
            generated_by: "deterministic-service",
            provider_request_sent: false,
            write_actions_available: false,
            execution_actions_available: false,
            purpose: "No catalog record was available for offline AI review preparation."
                .to_string(),
            risk: LlmReviewRisk {
                level: "unknown",
                summary:
                    "Risk was not assessed because the selected catalog record was unavailable."
                        .to_string(),
                signals: Vec::new(),
            },
            finding_explanations: Vec::new(),
            cross_agent_fit: LlmReviewCrossAgentFit {
                agent: "unknown".to_string(),
                scope: "unknown".to_string(),
                comparable_instance_count: 0,
                summary: "Cross-agent fit was not assessed.".to_string(),
                notes: Vec::new(),
            },
            redaction: llm_review_redaction(),
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ScanActivityCounts {
    scanned_count: usize,
    skill_count: usize,
    finding_count: usize,
    conflict_count: usize,
    snapshot_count: usize,
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
    pub preview_token: String,
}

#[derive(Debug, Clone, Default, Deserialize)]
pub struct PiWritableHarnessParams {
    #[serde(default)]
    pub run_label: Option<String>,
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
}

#[derive(Debug, Clone, Deserialize)]
pub struct SnapshotParams {
    pub snapshot_id: String,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ListAgentConfigSnapshotsParams {
    pub agent: String,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct ReadAgentConfigParams {
    pub agent: String,
    #[serde(default)]
    pub scope: Option<String>,
}

#[derive(Debug, Clone, Deserialize)]
pub struct SaveClaudeSettingsParams {
    pub content: String,
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
}

impl ServiceError {
    fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest(_) => "invalid_request",
            Self::UnknownMethod(_) => "unknown_method",
            Self::Io(_) => "io_error",
            Self::Catalog(_) => "catalog_error",
            Self::Command(_) => "command_error",
            Self::Provider(_) => "provider_error",
            Self::Json(_) => "json_error",
            Self::SkillNotFound(_) => "skill_not_found",
            Self::ConfirmationRequired(_) => "confirmation_required",
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
