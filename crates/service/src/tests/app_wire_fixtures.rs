use super::dispatch_fixtures::*;
use super::*;

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAppStateSnapshot {
    pub(super) status: WireServiceStatus,
    pub(super) watch_plan: WireAuthorizedFileWatchPlan,
    pub(super) skills: Vec<WireSkillRecord>,
    pub(super) findings: Vec<WireRuleFindingRecord>,
    pub(super) conflicts: Vec<WireConflictGroupRecord>,
    pub(super) analysis: WireCrossAgentAnalysisRecord,
    pub(super) health: SkillHealthSummary,
    pub(super) snapshots: Vec<WireConfigSnapshotRecord>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAuthorizedFileWatchPlan {
    pub(super) roots: Vec<String>,
    pub(super) total_count: usize,
    pub(super) truncated: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAppSearchResult {
    pub(super) generated_by: String,
    pub(super) query: String,
    pub(super) count: usize,
    pub(super) total_matched_count: usize,
    pub(super) limit_per_kind: usize,
    pub(super) items: Vec<WireAppSearchItem>,
    pub(super) read_only: bool,
    pub(super) provider_request_sent: bool,
    pub(super) skill_files_mutated: bool,
    pub(super) agent_config_mutated: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAppSearchItem {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) target_id: String,
    pub(super) title: String,
    pub(super) subtitle: String,
    pub(super) agent: Option<String>,
    pub(super) skill: Option<WireSkillRecord>,
    pub(super) session: Option<WireLocalSessionPreviewRow>,
    pub(super) config_snapshot: Option<WireConfigSnapshotRecord>,
}
