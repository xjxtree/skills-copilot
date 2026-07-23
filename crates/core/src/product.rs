use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{AgentId, ListIncompleteReason, ListSourceCompleteness, Scope};

/// Stable project identity shared by persisted project context and action
/// bindings. Keep this wire format compatible with existing stored contexts.
pub fn canonical_project_id(root_path: &str) -> String {
    let hash = root_path
        .as_bytes()
        .iter()
        .fold(0xcbf29ce484222325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x100000001b3)
        });
    format!("project-{hash:016x}")
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EnvironmentHealthState {
    Healthy,
    Review,
    Blocked,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum SkillEffectivenessState {
    Effective,
    Disabled,
    Shadowed,
    InstalledUnlinked,
    Broken,
    Unavailable,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum EvidenceKind {
    ProjectContext,
    AdapterCapability,
    ScanCoverage,
    SkillDefinition,
    SkillInstance,
    Finding,
    Conflict,
    Session,
    Config,
    Package,
    ActionReadback,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct EvidenceRef {
    pub id: String,
    pub kind: EvidenceKind,
    pub source_revision: String,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub target_id: Option<String>,
}

impl EvidenceRef {
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_nonempty_display_value(&self.id, "evidence id is empty")?;
        validate_nonempty_display_value(
            &self.source_revision,
            "evidence source_revision is empty",
        )?;
        validate_nonempty_display_value(&self.summary, "evidence summary is empty")?;
        if contains_raw_absolute_path(&self.summary) {
            return Err("evidence summary contains a raw absolute path");
        }
        if self
            .target_id
            .as_deref()
            .is_some_and(|value| value.trim().is_empty())
        {
            return Err("evidence target_id is empty");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ActionTargetKind {
    Project,
    Agent,
    Skill,
    Session,
    Config,
    Package,
    ProviderProfile,
    AppData,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionTargetRef {
    pub kind: ActionTargetKind,
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentId>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub scope: Option<Scope>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ActionImpact {
    ReadOnly,
    AppLocalData,
    CredentialStore,
    AgentConfig,
    SkillFiles,
    ExternalManager,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ActionNetworkPosture {
    None,
    Conditional,
    Required,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ActionKind {
    RefreshEvidence,
    ToggleSkill,
    InstallSkill,
    ManagerInstall,
    ManagerRemove,
    ManagerUpdate,
    ManagerLocalCreate,
    ManagerLocalArchiveImport,
    ManagerLocalArchiveUpdate,
    ManagerLocalDelete,
    RollbackConfig,
    SaveConfig,
    ResumeSession,
    TriageFinding,
    TuneRule,
    ProjectContext,
    ProviderProfile,
    ProviderConnectionTest,
    ProviderPrompt,
    PrivacyCleanup,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ActionIntent {
    RefreshEvidence,
    InspectEvidence,
    EnableSkill,
    DisableSkill,
    InstallSkill,
    ManagerInstall,
    ManagerRemove,
    ManagerUpdate,
    ManagerLocalCreate,
    ManagerLocalArchiveImport,
    ManagerLocalArchiveUpdate,
    ManagerLocalDelete,
    RollbackConfig,
    SaveConfig,
    ResumeSession,
    TriageFinding,
    TuneRule,
    SetProjectContext,
    ClearProjectContext,
    RemoveRecentProjectContext,
    ClearRecentProjectContexts,
    SaveProviderProfile,
    DeleteProviderProfile,
    TestProviderConnection,
    SendProviderPrompt,
    CleanLegacyPrivateContent,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ActionReadbackDomain {
    ProjectContext,
    CatalogSkills,
    SkillAggregates,
    SkillFiles,
    AgentConfig,
    ConfigSnapshots,
    ManagerInventory,
    SessionContinuation,
    BlockedAttemptAudit,
    ProviderProfiles,
    ProviderCredentials,
    ProviderActivity,
    PromptRuns,
    PrivateContent,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionDescriptor {
    pub id: String,
    pub kind: ActionKind,
    pub intent: ActionIntent,
    pub target: ActionTargetRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub impacts: Vec<ActionImpact>,
    pub preview_method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apply_method: Option<String>,
    pub source_revision: String,
    pub confirmation_required: bool,
    pub network: ActionNetworkPosture,
    pub readback: Vec<ActionReadbackDomain>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
}

impl ActionDescriptor {
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_nonempty_display_value(&self.id, "action id is empty")?;
        validate_nonempty_display_value(&self.target.id, "action target id is empty")?;
        if let Some(project_id) = self.project_id.as_deref() {
            validate_safe_identity(project_id, "action project_id is empty")?;
        }
        if self.target.scope == Some(Scope::AgentProject) && self.project_id.is_none() {
            return Err("project-scoped action requires project_id");
        }
        if self.target.kind == ActionTargetKind::Project
            && self.project_id.as_deref() != Some(self.target.id.as_str())
        {
            return Err("project action target does not match project_id");
        }
        validate_service_method(&self.preview_method, "invalid action preview method")?;
        validate_nonempty_display_value(&self.source_revision, "action source_revision is empty")?;
        if self.impacts.is_empty() {
            return Err("action impacts are empty");
        }
        let mut impacts = HashSet::new();
        for impact in &self.impacts {
            if !impacts.insert(*impact) {
                return Err("action impacts contain a duplicate");
            }
        }
        if self.readback.is_empty() {
            return Err("action readback domains are empty");
        }
        let mut readback_domains = HashSet::new();
        for domain in &self.readback {
            if !readback_domains.insert(*domain) {
                return Err("action readback domains contain a duplicate");
            }
        }
        if self.evidence_refs.is_empty() {
            return Err("action evidence_refs are empty");
        }
        let mut evidence_ids = HashSet::new();
        for evidence_id in &self.evidence_refs {
            validate_nonempty_display_value(evidence_id, "action evidence ref is empty")?;
            if !evidence_ids.insert(evidence_id.as_str()) {
                return Err("action evidence_refs contain a duplicate");
            }
        }

        let read_only = self
            .impacts
            .iter()
            .all(|impact| *impact == ActionImpact::ReadOnly);
        match self.apply_method.as_deref() {
            Some(method) => {
                validate_service_method(method, "invalid action apply method")?;
                if read_only {
                    return Err("read-only action cannot expose apply_method");
                }
                if !self.confirmation_required {
                    return Err("mutating action requires confirmation");
                }
            }
            None if !read_only => return Err("mutating action requires apply_method"),
            None => {}
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SourceCoverage {
    pub completeness: ListSourceCompleteness,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub incomplete_reason: Option<ListIncompleteReason>,
    pub inspected_sources: usize,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub expected_sources: Option<usize>,
}

impl SourceCoverage {
    pub fn enumerable(inspected_sources: usize, expected_sources: Option<usize>) -> Self {
        Self {
            completeness: ListSourceCompleteness::Enumerable,
            incomplete_reason: None,
            inspected_sources,
            expected_sources,
        }
    }

    pub fn incomplete(
        inspected_sources: usize,
        expected_sources: Option<usize>,
        reason: ListIncompleteReason,
    ) -> Self {
        Self {
            completeness: ListSourceCompleteness::Limited,
            incomplete_reason: Some(reason),
            inspected_sources,
            expected_sources,
        }
    }

    pub fn unknown(reason: ListIncompleteReason) -> Self {
        Self {
            completeness: ListSourceCompleteness::Unknown,
            incomplete_reason: Some(reason),
            inspected_sources: 0,
            expected_sources: None,
        }
    }

    pub fn is_complete(&self) -> bool {
        self.completeness == ListSourceCompleteness::Enumerable
            && self.incomplete_reason.is_none()
            && self
                .expected_sources
                .is_none_or(|expected| expected == self.inspected_sources)
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if self
            .expected_sources
            .is_some_and(|expected| self.inspected_sources > expected)
        {
            return Err("coverage inspected_sources exceeds expected_sources");
        }
        match self.completeness {
            ListSourceCompleteness::Enumerable => {
                if self.incomplete_reason.is_some() {
                    return Err("enumerable coverage cannot have incomplete_reason");
                }
                if self
                    .expected_sources
                    .is_some_and(|expected| expected != self.inspected_sources)
                {
                    return Err("enumerable coverage must inspect every expected source");
                }
            }
            ListSourceCompleteness::Limited | ListSourceCompleteness::Unknown => {
                if self.incomplete_reason.is_none() {
                    return Err("incomplete coverage requires a reason");
                }
            }
        }
        Ok(())
    }

    pub fn merge(coverages: &[Self]) -> Result<Self, &'static str> {
        if coverages.is_empty() {
            return Ok(Self::unknown(ListIncompleteReason::NotInspected));
        }
        for coverage in coverages {
            coverage.validate()?;
        }
        let inspected_sources = coverages.iter().try_fold(0usize, |total, coverage| {
            total
                .checked_add(coverage.inspected_sources)
                .ok_or("coverage inspected_sources overflow")
        })?;
        let expected_sources = coverages
            .iter()
            .map(|coverage| coverage.expected_sources)
            .collect::<Option<Vec<_>>>()
            .map(|values| {
                values.into_iter().try_fold(0usize, |total, value| {
                    total
                        .checked_add(value)
                        .ok_or("coverage expected_sources overflow")
                })
            })
            .transpose()?;
        if coverages.iter().all(Self::is_complete) {
            return Ok(Self::enumerable(inspected_sources, expected_sources));
        }
        let completeness = if coverages
            .iter()
            .any(|coverage| coverage.completeness == ListSourceCompleteness::Unknown)
        {
            ListSourceCompleteness::Unknown
        } else {
            ListSourceCompleteness::Limited
        };
        let incomplete_reason = coverages
            .iter()
            .filter_map(|coverage| coverage.incomplete_reason)
            .max_by_key(|reason| incomplete_reason_rank(*reason))
            .or(Some(ListIncompleteReason::SourceLimited));
        let merged = Self {
            completeness,
            incomplete_reason,
            inspected_sources,
            expected_sources,
        };
        merged.validate()?;
        Ok(merged)
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AttentionKind {
    IncompleteEvidence,
    StaleEvidence,
    SourceUnavailable,
    Finding,
    Conflict,
    BrokenSkill,
    SkillUnavailable,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum AttentionSeverity {
    Critical,
    Error,
    Warning,
    Information,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum NoSafeActionReason {
    Unsupported,
    ReadOnlySource,
    IncompleteEvidence,
    NoGuardedWritePath,
    ManualReviewRequired,
}

impl AttentionSeverity {
    pub fn blocks_health(self) -> bool {
        matches!(self, Self::Critical | Self::Error)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ReadinessBlocker {
    pub id: String,
    pub kind: AttentionKind,
    pub summary: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentId>,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_ids: Vec<String>,
}

impl ReadinessBlocker {
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_nonempty_display_value(&self.id, "readiness blocker id is empty")?;
        validate_safe_summary(&self.summary, "readiness blocker summary is empty")?;
        validate_unique_ids(
            &self.evidence_refs,
            "readiness blocker evidence_refs are empty",
            "readiness blocker evidence_refs contain a duplicate",
        )?;
        validate_optional_unique_ids(
            &self.action_ids,
            "readiness blocker action id is empty",
            "readiness blocker action_ids contain a duplicate",
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AttentionItem {
    pub id: String,
    pub kind: AttentionKind,
    pub severity: AttentionSeverity,
    pub title: String,
    pub summary: String,
    pub target: ActionTargetRef,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentId>,
    pub evidence_refs: Vec<String>,
    #[serde(default)]
    pub action_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub no_safe_action_reason: Option<NoSafeActionReason>,
}

impl AttentionItem {
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_nonempty_display_value(&self.id, "attention item id is empty")?;
        validate_nonempty_display_value(&self.title, "attention item title is empty")?;
        validate_safe_summary(&self.summary, "attention item summary is empty")?;
        validate_nonempty_display_value(&self.target.id, "attention item target id is empty")?;
        validate_unique_ids(
            &self.evidence_refs,
            "attention item evidence_refs are empty",
            "attention item evidence_refs contain a duplicate",
        )?;
        validate_optional_unique_ids(
            &self.action_ids,
            "attention item action id is empty",
            "attention item action_ids contain a duplicate",
        )?;
        match (self.action_ids.is_empty(), self.no_safe_action_reason) {
            (true, None) => Err("attention item without an action requires no_safe_action_reason"),
            (false, Some(_)) => {
                Err("attention item with actions cannot have no_safe_action_reason")
            }
            _ => Ok(()),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AgentReadinessRecord {
    pub agent: AgentId,
    pub health: EnvironmentHealthState,
    pub coverage: SourceCoverage,
    pub effective_skill_count: usize,
    pub issue_count: usize,
    pub conflict_count: usize,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocking_reasons: Vec<ReadinessBlocker>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attention_item_ids: Vec<String>,
}

impl AgentReadinessRecord {
    fn validate(
        &self,
        evidence_ids: &HashSet<&str>,
        action_ids: &HashSet<&str>,
        attention_ids: &HashSet<&str>,
    ) -> Result<(), &'static str> {
        self.coverage.validate()?;
        if self.health != EnvironmentHealthState::Blocked && !self.coverage.is_complete() {
            return Err("incomplete agent readiness coverage requires blocked health");
        }
        if self.health == EnvironmentHealthState::Healthy && !self.blocking_reasons.is_empty() {
            return Err("healthy agent readiness cannot have blocking reasons");
        }
        if self.health == EnvironmentHealthState::Blocked && self.blocking_reasons.is_empty() {
            return Err("blocked agent readiness requires a blocking reason");
        }
        validate_known_ids(
            &self.evidence_refs,
            evidence_ids,
            "agent readiness references unknown evidence",
        )?;
        validate_known_ids(
            &self.action_ids,
            action_ids,
            "agent readiness references unknown action",
        )?;
        validate_known_ids(
            &self.attention_item_ids,
            attention_ids,
            "agent readiness references unknown attention item",
        )?;
        for blocker in &self.blocking_reasons {
            blocker.validate()?;
            if blocker.agent.is_some_and(|agent| agent != self.agent) {
                return Err("agent readiness blocker belongs to another agent");
            }
            validate_known_ids(
                &blocker.evidence_refs,
                evidence_ids,
                "agent readiness blocker references unknown evidence",
            )?;
            validate_known_ids(
                &blocker.action_ids,
                action_ids,
                "agent readiness blocker references unknown action",
            )?;
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ResumeCapabilityState {
    Supported,
    Unsupported,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
#[non_exhaustive]
pub enum ResumeUnsupportedReason {
    AgentUnsupported,
    SessionUnsupported,
    SourceIncomplete,
    SourceChanged,
    MissingNativeId,
    InvalidProjectContext,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ResumeCapability {
    pub state: ResumeCapabilityState,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub argv: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub unsupported_reason: Option<ResumeUnsupportedReason>,
    pub copy_only: bool,
}

impl ResumeCapability {
    pub fn supported(argv: Vec<String>) -> Self {
        Self {
            state: ResumeCapabilityState::Supported,
            argv,
            unsupported_reason: None,
            copy_only: true,
        }
    }

    pub fn unsupported(reason: ResumeUnsupportedReason) -> Self {
        Self {
            state: ResumeCapabilityState::Unsupported,
            argv: Vec::new(),
            unsupported_reason: Some(reason),
            copy_only: true,
        }
    }

    pub fn validate(&self) -> Result<(), &'static str> {
        if !self.copy_only {
            return Err("resume capability must be copy-only");
        }
        match self.state {
            ResumeCapabilityState::Supported => {
                if self.argv.is_empty() || self.argv.iter().any(|arg| arg.is_empty()) {
                    return Err("supported resume capability requires argv");
                }
                if self.unsupported_reason.is_some() {
                    return Err("supported resume capability cannot have unsupported_reason");
                }
            }
            ResumeCapabilityState::Unsupported => {
                if !self.argv.is_empty() {
                    return Err("unsupported resume capability cannot have argv");
                }
                if self.unsupported_reason.is_none() {
                    return Err("unsupported resume capability requires a reason");
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SessionContinuationRecord {
    pub id: String,
    pub agent: AgentId,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub project_id: Option<String>,
    pub title: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub intent: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ended_at: Option<i64>,
    pub modified_at: i64,
    pub source_kind: String,
    pub source_revision: String,
    pub snapshot_revision: String,
    pub coverage: SourceCoverage,
    pub resume: ResumeCapability,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionDescriptor>,
}

impl SessionContinuationRecord {
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_nonempty_display_value(&self.id, "session continuation id is empty")?;
        validate_nonempty_display_value(&self.title, "session continuation title is empty")?;
        validate_nonempty_display_value(
            &self.source_revision,
            "session continuation source_revision is empty",
        )?;
        validate_nonempty_display_value(
            &self.source_kind,
            "session continuation source_kind is empty",
        )?;
        validate_nonempty_display_value(
            &self.snapshot_revision,
            "session continuation snapshot_revision is empty",
        )?;
        self.coverage.validate()?;
        self.resume.validate()?;
        if !self.coverage.is_complete() && self.resume.state == ResumeCapabilityState::Supported {
            return Err("incomplete session coverage cannot expose supported resume");
        }
        validate_session_projection_links(
            &self.source_revision,
            &self.snapshot_revision,
            &self.evidence,
            &self.actions,
        )
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillEffectivenessCount {
    pub state: SkillEffectivenessState,
    pub count: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillInstanceEffectivenessRecord {
    pub instance_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub agent: Option<AgentId>,
    pub scope: Scope,
    pub source_identity: String,
    pub runtime_identity: String,
    pub installed: bool,
    pub linked: bool,
    pub enabled: bool,
    pub precedence_proven: bool,
    pub state: SkillEffectivenessState,
    pub coverage: SourceCoverage,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_ids: Vec<String>,
}

impl SkillInstanceEffectivenessRecord {
    pub fn validate(
        &self,
        evidence_ids: &HashSet<&str>,
        action_ids: &HashSet<&str>,
    ) -> Result<(), &'static str> {
        validate_nonempty_display_value(
            &self.instance_id,
            "skill effectiveness instance_id is empty",
        )?;
        validate_safe_identity(
            &self.source_identity,
            "skill effectiveness source_identity is empty",
        )?;
        validate_safe_identity(
            &self.runtime_identity,
            "skill effectiveness runtime_identity is empty",
        )?;
        self.coverage.validate()?;
        validate_known_ids(
            &self.evidence_refs,
            evidence_ids,
            "skill effectiveness references unknown evidence",
        )?;
        validate_known_ids(
            &self.action_ids,
            action_ids,
            "skill effectiveness references unknown action",
        )?;

        match self.state {
            SkillEffectivenessState::Effective => {
                if !self.installed
                    || !self.linked
                    || !self.enabled
                    || !self.precedence_proven
                    || !self.coverage.is_complete()
                {
                    return Err("effective skill requires complete proven linked enablement");
                }
            }
            SkillEffectivenessState::Disabled => {
                if !self.installed || !self.linked || self.enabled || !self.coverage.is_complete() {
                    return Err(
                        "disabled skill requires a complete installed linked disabled source",
                    );
                }
            }
            SkillEffectivenessState::Shadowed => {
                if !self.installed
                    || !self.linked
                    || !self.enabled
                    || !self.precedence_proven
                    || !self.coverage.is_complete()
                {
                    return Err("shadowed skill requires complete proven precedence evidence");
                }
            }
            SkillEffectivenessState::InstalledUnlinked => {
                if !self.installed || self.linked || !self.coverage.is_complete() {
                    return Err("installed_unlinked skill requires a complete unlinked source");
                }
            }
            SkillEffectivenessState::Broken => {
                if !self.installed || !self.coverage.is_complete() {
                    return Err("broken skill requires a complete installed source");
                }
            }
            SkillEffectivenessState::Unavailable => {
                if self.installed
                    && self.coverage.is_complete()
                    && (!self.linked || !self.enabled || self.precedence_proven)
                {
                    return Err(
                        "unavailable installed skill requires linked enabled unproved precedence",
                    );
                }
            }
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillAggregateRecord {
    pub id: String,
    pub definition_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub definition_fingerprint: Option<String>,
    pub canonical_name: String,
    pub display_name: String,
    pub description: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub publisher: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub package_version: Option<String>,
    pub source_kind: String,
    pub source_identity: String,
    pub runtime_identity: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub read_only_reason: Option<String>,
    pub instance_ids: Vec<String>,
    pub agents: Vec<AgentId>,
    pub scopes: Vec<Scope>,
    pub installed_instance_count: usize,
    pub enabled_instance_count: usize,
    pub effective_instance_count: usize,
    pub primary_effectiveness: SkillEffectivenessState,
    pub effectiveness_counts: Vec<SkillEffectivenessCount>,
    pub instance_effectiveness: Vec<SkillInstanceEffectivenessRecord>,
    pub finding_count: usize,
    pub conflict_count: usize,
    pub source_revision: String,
    pub coverage: SourceCoverage,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionDescriptor>,
}

impl SkillAggregateRecord {
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_nonempty_display_value(&self.id, "skill aggregate id is empty")?;
        validate_nonempty_display_value(
            &self.definition_id,
            "skill aggregate definition_id is empty",
        )?;
        validate_nonempty_display_value(
            &self.canonical_name,
            "skill aggregate canonical_name is empty",
        )?;
        validate_nonempty_display_value(
            &self.source_revision,
            "skill aggregate source_revision is empty",
        )?;
        validate_nonempty_display_value(&self.source_kind, "skill aggregate source_kind is empty")?;
        validate_safe_identity(
            &self.source_identity,
            "skill aggregate source_identity is empty",
        )?;
        validate_safe_identity(
            &self.runtime_identity,
            "skill aggregate runtime_identity is empty",
        )?;
        if self.instance_ids.is_empty() {
            return Err("skill aggregate instance_ids are empty");
        }
        if self.installed_instance_count > self.instance_ids.len() {
            return Err("skill aggregate installed count exceeds instance_ids");
        }
        if self.enabled_instance_count > self.installed_instance_count
            || self.effective_instance_count > self.enabled_instance_count
        {
            return Err("skill aggregate instance counts are inconsistent");
        }
        let effectiveness_total = self
            .effectiveness_counts
            .iter()
            .map(|entry| entry.count)
            .sum::<usize>();
        if effectiveness_total != self.instance_ids.len() {
            return Err("skill aggregate effectiveness counts do not match projected instances");
        }
        let mut counted_states = HashSet::new();
        for entry in &self.effectiveness_counts {
            if !counted_states.insert(entry.state) {
                return Err("skill aggregate effectiveness counts contain a duplicate state");
            }
            if entry.count == 0 {
                return Err("skill aggregate effectiveness count must be positive");
            }
            let actual = self
                .instance_effectiveness
                .iter()
                .filter(|record| record.state == entry.state)
                .count();
            if actual != entry.count {
                return Err("skill aggregate effectiveness count does not match instance rows");
            }
        }
        if self.instance_effectiveness.len() != self.instance_ids.len() {
            return Err("skill aggregate effectiveness rows do not match instance_ids");
        }
        self.coverage.validate()?;
        validate_projection_links(&self.source_revision, &self.evidence, &self.actions)?;
        let evidence_ids = evidence_id_set(&self.evidence);
        let action_ids = action_id_set(&self.actions);
        let expected_ids = self
            .instance_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        let mut actual_ids = HashSet::new();
        for record in &self.instance_effectiveness {
            if !actual_ids.insert(record.instance_id.as_str()) {
                return Err("skill aggregate effectiveness contains a duplicate instance");
            }
            if record.source_identity != self.source_identity
                || record.runtime_identity != self.runtime_identity
            {
                return Err("skill aggregate effectiveness identity does not match aggregate");
            }
            record.validate(&evidence_ids, &action_ids)?;
        }
        if actual_ids != expected_ids {
            return Err("skill aggregate effectiveness instances do not match instance_ids");
        }
        if self
            .instance_effectiveness
            .iter()
            .any(|record| !counted_states.contains(&record.state))
        {
            return Err("skill aggregate effectiveness count is missing an instance state");
        }
        let expected_primary = self
            .instance_effectiveness
            .iter()
            .map(|record| record.state)
            .min_by_key(|state| skill_effectiveness_rank(*state))
            .ok_or("skill aggregate effectiveness rows are empty")?;
        if self.primary_effectiveness != expected_primary {
            return Err("skill aggregate primary effectiveness does not match instance rows");
        }
        let expected_agents = self
            .instance_effectiveness
            .iter()
            .filter_map(|record| record.agent)
            .collect::<HashSet<_>>();
        let actual_agents = self.agents.iter().copied().collect::<HashSet<_>>();
        if actual_agents.len() != self.agents.len() || actual_agents != expected_agents {
            return Err("skill aggregate agents do not match instance rows");
        }
        let expected_scopes = self
            .instance_effectiveness
            .iter()
            .map(|record| record.scope)
            .collect::<HashSet<_>>();
        let actual_scopes = self.scopes.iter().copied().collect::<HashSet<_>>();
        if actual_scopes.len() != self.scopes.len() || actual_scopes != expected_scopes {
            return Err("skill aggregate scopes do not match instance rows");
        }
        let instance_coverages = self
            .instance_effectiveness
            .iter()
            .map(|record| record.coverage.clone())
            .collect::<Vec<_>>();
        let expected_coverage = SourceCoverage::merge(&instance_coverages)?;
        if self.coverage != expected_coverage {
            return Err("skill aggregate coverage does not match instance rows");
        }
        let installed = self
            .instance_effectiveness
            .iter()
            .filter(|record| record.installed)
            .count();
        let enabled = self
            .instance_effectiveness
            .iter()
            .filter(|record| record.installed && record.enabled)
            .count();
        let effective = self
            .instance_effectiveness
            .iter()
            .filter(|record| record.state == SkillEffectivenessState::Effective)
            .count();
        if installed != self.installed_instance_count
            || enabled != self.enabled_instance_count
            || effective != self.effective_instance_count
        {
            return Err("skill aggregate counts do not match effectiveness rows");
        }
        Ok(())
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProjectReadinessRecord {
    pub project_id: String,
    pub project_display_name: String,
    pub source_revision: String,
    pub health: EnvironmentHealthState,
    pub coverage: SourceCoverage,
    pub agents: Vec<AgentReadinessRecord>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub blocking_reasons: Vec<ReadinessBlocker>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub attention: Vec<AttentionItem>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence: Vec<EvidenceRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub recent_sessions: Vec<SessionContinuationRecord>,
}

impl ProjectReadinessRecord {
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_nonempty_display_value(&self.project_id, "project readiness id is empty")?;
        validate_nonempty_display_value(
            &self.project_display_name,
            "project readiness display name is empty",
        )?;
        validate_nonempty_display_value(
            &self.source_revision,
            "project readiness source_revision is empty",
        )?;
        self.coverage.validate()?;
        if self.health != EnvironmentHealthState::Blocked && !self.coverage.is_complete() {
            return Err("incomplete project readiness coverage requires blocked health");
        }
        if self.health == EnvironmentHealthState::Healthy && !self.blocking_reasons.is_empty() {
            return Err("healthy project readiness cannot have blocking reasons");
        }
        if self.health == EnvironmentHealthState::Blocked && self.blocking_reasons.is_empty() {
            return Err("blocked project readiness requires a blocking reason");
        }
        validate_projection_links(&self.source_revision, &self.evidence, &self.actions)?;
        let evidence_ids = evidence_id_set(&self.evidence);
        let action_ids = action_id_set(&self.actions);
        let attention_ids = self
            .attention
            .iter()
            .map(|item| item.id.as_str())
            .collect::<HashSet<_>>();
        if attention_ids.len() != self.attention.len() {
            return Err("project readiness attention contains a duplicate id");
        }
        for item in &self.attention {
            item.validate()?;
            validate_known_ids(
                &item.evidence_refs,
                &evidence_ids,
                "attention item references unknown evidence",
            )?;
            validate_known_ids(
                &item.action_ids,
                &action_ids,
                "attention item references unknown action",
            )?;
        }
        let blocker_ids = self
            .blocking_reasons
            .iter()
            .map(|blocker| blocker.id.as_str())
            .collect::<HashSet<_>>();
        if blocker_ids.len() != self.blocking_reasons.len() {
            return Err("project readiness blocking_reasons contain a duplicate id");
        }
        for blocker in &self.blocking_reasons {
            blocker.validate()?;
            validate_known_ids(
                &blocker.evidence_refs,
                &evidence_ids,
                "project readiness blocker references unknown evidence",
            )?;
            validate_known_ids(
                &blocker.action_ids,
                &action_ids,
                "project readiness blocker references unknown action",
            )?;
        }
        let mut agents = HashSet::new();
        for agent in &self.agents {
            if !agents.insert(agent.agent) {
                return Err("project readiness contains a duplicate agent");
            }
            agent.validate(&evidence_ids, &action_ids, &attention_ids)?;
            if agent
                .blocking_reasons
                .iter()
                .any(|blocker| !blocker_ids.contains(blocker.id.as_str()))
            {
                return Err("agent readiness blocker is missing from project blocking_reasons");
            }
        }
        if !self.agents.is_empty() {
            let agent_coverages = self
                .agents
                .iter()
                .map(|agent| agent.coverage.clone())
                .collect::<Vec<_>>();
            let expected_coverage = SourceCoverage::merge(&agent_coverages)?;
            if self.coverage != expected_coverage {
                return Err("project readiness coverage does not match agent rows");
            }
        }
        for session in &self.recent_sessions {
            session.validate()?;
            if session.snapshot_revision != self.source_revision {
                return Err("session continuation snapshot_revision does not match project");
            }
        }
        Ok(())
    }
}

fn validate_projection_links(
    source_revision: &str,
    evidence: &[EvidenceRef],
    actions: &[ActionDescriptor],
) -> Result<(), &'static str> {
    let mut evidence_ids = HashSet::new();
    for reference in evidence {
        reference.validate()?;
        if reference.source_revision != source_revision {
            return Err("evidence source_revision does not match projection");
        }
        if !evidence_ids.insert(reference.id.as_str()) {
            return Err("projection evidence contains a duplicate id");
        }
    }
    let mut action_ids = HashSet::new();
    for action in actions {
        action.validate()?;
        if !action_ids.insert(action.id.as_str()) {
            return Err("projection actions contain a duplicate id");
        }
        if action.source_revision != source_revision {
            return Err("action source_revision does not match projection");
        }
        if action
            .evidence_refs
            .iter()
            .any(|id| !evidence_ids.contains(id.as_str()))
        {
            return Err("action references unknown projection evidence");
        }
    }
    Ok(())
}

fn validate_session_projection_links(
    source_revision: &str,
    snapshot_revision: &str,
    evidence: &[EvidenceRef],
    actions: &[ActionDescriptor],
) -> Result<(), &'static str> {
    let mut evidence_ids = HashSet::new();
    for reference in evidence {
        reference.validate()?;
        if reference.source_revision != source_revision {
            return Err("evidence source_revision does not match session source");
        }
        if !evidence_ids.insert(reference.id.as_str()) {
            return Err("projection evidence contains a duplicate id");
        }
    }
    let mut action_ids = HashSet::new();
    for action in actions {
        action.validate()?;
        if !action_ids.insert(action.id.as_str()) {
            return Err("projection actions contain a duplicate id");
        }
        if action.source_revision != snapshot_revision {
            return Err("action source_revision does not match accepted session snapshot");
        }
        if action
            .evidence_refs
            .iter()
            .any(|id| !evidence_ids.contains(id.as_str()))
        {
            return Err("action references unknown projection evidence");
        }
    }
    Ok(())
}

fn evidence_id_set(evidence: &[EvidenceRef]) -> HashSet<&str> {
    evidence
        .iter()
        .map(|reference| reference.id.as_str())
        .collect()
}

fn action_id_set(actions: &[ActionDescriptor]) -> HashSet<&str> {
    actions.iter().map(|action| action.id.as_str()).collect()
}

fn validate_known_ids(
    ids: &[String],
    known: &HashSet<&str>,
    error: &'static str,
) -> Result<(), &'static str> {
    let mut seen = HashSet::new();
    for id in ids {
        validate_nonempty_display_value(id, error)?;
        if !seen.insert(id.as_str()) || !known.contains(id.as_str()) {
            return Err(error);
        }
    }
    Ok(())
}

fn validate_unique_ids(
    ids: &[String],
    empty_error: &'static str,
    duplicate_error: &'static str,
) -> Result<(), &'static str> {
    if ids.is_empty() {
        return Err(empty_error);
    }
    validate_optional_unique_ids(ids, empty_error, duplicate_error)
}

fn validate_optional_unique_ids(
    ids: &[String],
    empty_error: &'static str,
    duplicate_error: &'static str,
) -> Result<(), &'static str> {
    let mut seen = HashSet::new();
    for id in ids {
        validate_nonempty_display_value(id, empty_error)?;
        if !seen.insert(id.as_str()) {
            return Err(duplicate_error);
        }
    }
    Ok(())
}

fn validate_safe_summary(value: &str, empty_error: &'static str) -> Result<(), &'static str> {
    validate_nonempty_display_value(value, empty_error)?;
    if contains_raw_absolute_path(value) {
        return Err("product model summary contains a raw absolute path");
    }
    Ok(())
}

fn validate_safe_identity(value: &str, empty_error: &'static str) -> Result<(), &'static str> {
    validate_safe_summary(value, empty_error)?;
    if value.contains(['/', '\\']) {
        return Err("product model identity contains a path separator");
    }
    Ok(())
}

fn skill_effectiveness_rank(state: SkillEffectivenessState) -> u8 {
    match state {
        SkillEffectivenessState::Broken => 0,
        SkillEffectivenessState::Unavailable => 1,
        SkillEffectivenessState::Disabled => 2,
        SkillEffectivenessState::Shadowed => 3,
        SkillEffectivenessState::InstalledUnlinked => 4,
        SkillEffectivenessState::Effective => 5,
    }
}

fn incomplete_reason_rank(reason: ListIncompleteReason) -> u8 {
    match reason {
        ListIncompleteReason::UnsupportedProtocol => 0,
        ListIncompleteReason::PageFailed => 1,
        ListIncompleteReason::SourceChanged => 2,
        ListIncompleteReason::SourceLimited => 3,
        ListIncompleteReason::UnreadableSource => 4,
        ListIncompleteReason::NotInspected => 5,
        ListIncompleteReason::StaleSource => 6,
        ListIncompleteReason::SafetyBudget => 7,
    }
}

fn validate_nonempty_display_value(
    value: &str,
    empty_error: &'static str,
) -> Result<(), &'static str> {
    if value.trim().is_empty() {
        return Err(empty_error);
    }
    if value.chars().any(char::is_control) {
        return Err("product model text contains control characters");
    }
    Ok(())
}

fn validate_service_method(value: &str, invalid_error: &'static str) -> Result<(), &'static str> {
    validate_nonempty_display_value(value, invalid_error)?;
    let mut parts = value.split('.');
    let Some(domain) = parts.next() else {
        return Err(invalid_error);
    };
    let Some(method) = parts.next() else {
        return Err(invalid_error);
    };
    if parts.next().is_some()
        || domain.is_empty()
        || method.is_empty()
        || !value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || character == '.')
    {
        return Err(invalid_error);
    }
    Ok(())
}

fn contains_raw_absolute_path(value: &str) -> bool {
    if value.contains("file://") || value.contains(r"\\") {
        return true;
    }
    value.split_whitespace().any(|word| {
        let token = word.trim_matches(|character: char| {
            matches!(
                character,
                '(' | ')' | '[' | ']' | '{' | '}' | '"' | '\'' | ',' | ';'
            )
        });
        (token.len() > 1 && token.starts_with('/'))
            || (token.len() > 2
                && token.as_bytes()[1] == b':'
                && matches!(token.as_bytes()[2], b'/' | b'\\'))
    })
}
