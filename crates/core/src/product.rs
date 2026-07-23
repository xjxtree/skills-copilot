use std::collections::HashSet;

use serde::{Deserialize, Serialize};

use crate::{AgentId, ListIncompleteReason, ListSourceCompleteness, Scope};

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

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ActionDescriptor {
    pub id: String,
    pub target: ActionTargetRef,
    pub impacts: Vec<ActionImpact>,
    pub preview_method: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub apply_method: Option<String>,
    pub source_revision: String,
    pub confirmation_required: bool,
    pub network: ActionNetworkPosture,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub evidence_refs: Vec<String>,
}

impl ActionDescriptor {
    pub fn validate(&self) -> Result<(), &'static str> {
        validate_nonempty_display_value(&self.id, "action id is empty")?;
        validate_nonempty_display_value(&self.target.id, "action target id is empty")?;
        validate_service_method(&self.preview_method, "invalid action preview method")?;
        validate_nonempty_display_value(&self.source_revision, "action source_revision is empty")?;
        if self.impacts.is_empty() {
            return Err("action impacts are empty");
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

    pub fn is_complete(&self) -> bool {
        self.completeness == ListSourceCompleteness::Enumerable
            && self.incomplete_reason.is_none()
            && self
                .expected_sources
                .is_none_or(|expected| expected == self.inspected_sources)
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
        self.resume.validate()?;
        validate_projection_links(&self.source_revision, &self.evidence, &self.actions)
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillEffectivenessCount {
    pub state: SkillEffectivenessState,
    pub count: usize,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct SkillAggregateRecord {
    pub id: String,
    pub definition_id: String,
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
        if self.instance_ids.is_empty() {
            return Err("skill aggregate instance_ids are empty");
        }
        if self.installed_instance_count != self.instance_ids.len() {
            return Err("skill aggregate installed count does not match instance_ids");
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
        if effectiveness_total != self.installed_instance_count {
            return Err("skill aggregate effectiveness counts do not match installed count");
        }
        validate_projection_links(&self.source_revision, &self.evidence, &self.actions)
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
        if self.health == EnvironmentHealthState::Healthy && !self.coverage.is_complete() {
            return Err("healthy project readiness requires complete coverage");
        }
        for agent in &self.agents {
            if agent.health == EnvironmentHealthState::Healthy && !agent.coverage.is_complete() {
                return Err("healthy agent readiness requires complete coverage");
            }
        }
        validate_projection_links(&self.source_revision, &self.evidence, &self.actions)?;
        for session in &self.recent_sessions {
            session.validate()?;
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
    for action in actions {
        action.validate()?;
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
