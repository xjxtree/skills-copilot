use serde::{Deserialize, Serialize};
use serde_json::Value;

pub const DEFAULT_BUNDLE_ID: &str = "dev.agent-copilot.native";
pub const LEGACY_BUNDLE_ID: &str = "dev.skills-copilot.native";
pub const SERVICE_PROTOCOL_VERSION: u32 = 2;
pub const SUPPORTED_METHODS: &[&str] = &[
    "app.version",
    "app.stateSnapshot",
    "app.search",
    "service.status",
    "adapter.listCapabilities",
    "adapter.listDiagnostics",
    "session.previewLocalSessions",
    "session.listLocalSessionMessages",
    "llm.status",
    "llm.listProviderProfiles",
    "llm.previewSaveProviderProfile",
    "llm.saveProviderProfile",
    "llm.previewDeleteProviderProfile",
    "llm.deleteProviderProfile",
    "llm.previewProviderConnectionTest",
    "llm.testProviderConnection",
    "llm.previewPrompt",
    "llm.confirmPromptAndSend",
    "llm.listPromptRuns",
    "llm.providerObservability",
    "llm.listProviderActivity",
    "llm.listModelTaskMatches",
    "llm.recordModelTaskMatch",
    "llm.deleteModelTaskMatch",
    "llm.prepareAction",
    "privacy.inspectLegacyContent",
    "privacy.previewCleanupLegacyContent",
    "privacy.cleanupLegacyContent",
    "rules.listTuning",
    "rules.setSeverityOverride",
    "rules.clearSeverityOverride",
    "rules.setSuppression",
    "rules.clearSuppression",
    "batch.previewSkillToggles",
    "batch.applySkillToggles",
    "script.previewExecution",
    "script.execute",
    "skillManager.listTools",
    "skillManager.search",
    "skillManager.applySearch",
    "skillManager.listInstalled",
    "skillManager.previewInstall",
    "skillManager.applyInstall",
    "skillManager.previewRemove",
    "skillManager.applyRemove",
    "skillManager.previewUpdate",
    "skillManager.applyUpdate",
    "skillManager.previewLocalCreate",
    "skillManager.applyLocalCreate",
    "skillManager.previewLocalArchiveImport",
    "skillManager.applyLocalArchiveImport",
    "skillManager.previewLocalArchiveUpdate",
    "skillManager.applyLocalArchiveUpdate",
    "skillManager.deleteLocal",
    "project.getContext",
    "project.getReadiness",
    "project.previewSetContext",
    "project.setContext",
    "project.previewClearContext",
    "project.clearContext",
    "project.previewRemoveRecentContext",
    "project.removeRecentContext",
    "project.previewClearRecentContexts",
    "project.clearRecentContexts",
    "project.validateContext",
    "catalog.listSkills",
    "catalog.listSkillAggregates",
    "catalog.getSkill",
    "catalog.analysis",
    "catalog.listFindings",
    "catalog.listFindingTriage",
    "catalog.setFindingTriage",
    "catalog.clearFindingTriage",
    "catalog.listConflicts",
    "catalog.importSkill",
    "catalog.scanClaude",
    "catalog.scanAll",
    "skill.exportBundle",
    "skill.install",
    "skill.listEvents",
    "skill.listEventsPage",
    "config.toggleSkill",
    "config.readAgentConfig",
    "config.readClaudeSettings",
    "config.previewSaveClaudeSettings",
    "config.saveClaudeSettings",
    "snapshot.list",
    "snapshot.listAgentConfig",
    "snapshot.listAgentConfigPage",
    "snapshot.previewRollback",
    "snapshot.rollback",
];

#[derive(Debug, Clone, Deserialize)]
pub struct ServiceRequest {
    pub id: Option<String>,
    pub method: String,
    #[serde(default)]
    pub params: Value,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceResponse {
    pub id: Option<String>,
    pub ok: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<ServiceErrorRecord>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ServiceErrorRecord {
    pub code: String,
    pub message: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub details: Option<ServiceErrorDetails>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ServiceErrorDetails {
    pub operation: String,
    pub state: String,
    pub cleanup_required: bool,
    pub retry_allowed: bool,
}
