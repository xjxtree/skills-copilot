use std::collections::{BTreeMap, BTreeSet, HashMap, HashSet};
use std::fmt;

use serde::{Deserialize, Serialize};
use serde_json::Value;
use skills_copilot_core::{ActionDescriptor, EvidenceRef, Scope, SkillInstance, SkillState};

pub const NATIVE_RUNTIME_NAMESPACE: &str = "native";
pub const AI_RESPONSE_ENVELOPE_SCHEMA_VERSION: u32 = 1;

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum AiResultSchema {
    CopyOnlyMarkdown,
    TaskReadiness,
    SessionDigest,
    SkillChangeReview,
}

impl AiResultSchema {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CopyOnlyMarkdown => "copy_only_markdown",
            Self::TaskReadiness => "task_readiness",
            Self::SessionDigest => "session_digest",
            Self::SkillChangeReview => "skill_change_review",
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
pub struct AiResponseSafetyFlags {
    pub copy_only: bool,
    pub write_back_allowed: bool,
    pub command_execution_allowed: bool,
    pub script_execution_allowed: bool,
    pub mutation_allowed: bool,
    pub hidden_task_state_created: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_trace_persisted: bool,
}

impl AiResponseSafetyFlags {
    pub const fn required_copy_only() -> Self {
        Self {
            copy_only: true,
            write_back_allowed: false,
            command_execution_allowed: false,
            script_execution_allowed: false,
            mutation_allowed: false,
            hidden_task_state_created: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
        }
    }

    pub fn is_required_copy_only(self) -> bool {
        self == Self::required_copy_only()
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct AiResponseContract {
    pub schema_version: u32,
    pub request_kind: String,
    pub project_id: String,
    pub source_revision: String,
    pub result_schema: AiResultSchema,
    pub evidence: Vec<EvidenceRef>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub actions: Vec<ActionDescriptor>,
    pub required_safety_flags: AiResponseSafetyFlags,
}

impl AiResponseContract {
    pub fn new(
        request_kind: impl Into<String>,
        project_id: impl Into<String>,
        source_revision: impl Into<String>,
        result_schema: AiResultSchema,
        evidence: Vec<EvidenceRef>,
        actions: Vec<ActionDescriptor>,
    ) -> Result<Self, AiResponseValidationError> {
        let contract = Self {
            schema_version: AI_RESPONSE_ENVELOPE_SCHEMA_VERSION,
            request_kind: request_kind.into(),
            project_id: project_id.into(),
            source_revision: source_revision.into(),
            result_schema,
            evidence,
            actions,
            required_safety_flags: AiResponseSafetyFlags::required_copy_only(),
        };
        contract.validate()?;
        Ok(contract)
    }

    pub fn evidence_ids(&self) -> BTreeSet<&str> {
        self.evidence
            .iter()
            .map(|reference| reference.id.as_str())
            .collect()
    }

    pub fn action_ids(&self) -> BTreeSet<&str> {
        self.actions
            .iter()
            .map(|action| action.id.as_str())
            .collect()
    }

    pub fn validate(&self) -> Result<(), AiResponseValidationError> {
        if self.schema_version != AI_RESPONSE_ENVELOPE_SCHEMA_VERSION {
            return Err(AiResponseValidationError::UnsupportedSchemaVersion(
                self.schema_version,
            ));
        }
        require_contract_value(&self.request_kind, "request_kind")?;
        require_contract_value(&self.project_id, "project_id")?;
        require_contract_value(&self.source_revision, "source_revision")?;
        if !self.required_safety_flags.is_required_copy_only() {
            return Err(AiResponseValidationError::UnsafeSafetyFlags);
        }
        if self.evidence.is_empty() {
            return Err(AiResponseValidationError::EvidenceRequired);
        }

        let mut evidence_ids = HashSet::new();
        for reference in &self.evidence {
            reference
                .validate()
                .map_err(AiResponseValidationError::InvalidEvidence)?;
            if !evidence_ids.insert(reference.id.as_str()) {
                return Err(AiResponseValidationError::DuplicateEvidenceReference(
                    reference.id.clone(),
                ));
            }
        }

        let mut action_ids = HashSet::new();
        for action in &self.actions {
            action
                .validate()
                .map_err(AiResponseValidationError::InvalidAction)?;
            if !action_ids.insert(action.id.as_str()) {
                return Err(AiResponseValidationError::DuplicateActionReference(
                    action.id.clone(),
                ));
            }
            if action.project_id.as_deref() != Some(self.project_id.as_str()) {
                return Err(AiResponseValidationError::ActionTargetDrift(
                    action.id.clone(),
                ));
            }
            for evidence_id in &action.evidence_refs {
                if !evidence_ids.contains(evidence_id.as_str()) {
                    return Err(AiResponseValidationError::ActionReferencesUnknownEvidence(
                        evidence_id.clone(),
                    ));
                }
            }
        }
        Ok(())
    }

    pub fn validate_envelope(
        &self,
        envelope: &AiResponseEnvelope,
    ) -> Result<(), AiResponseValidationError> {
        self.validate()?;
        if envelope.schema_version != self.schema_version {
            return Err(AiResponseValidationError::UnsupportedSchemaVersion(
                envelope.schema_version,
            ));
        }
        if envelope.request_kind != self.request_kind {
            return Err(AiResponseValidationError::RequestKindMismatch);
        }
        if envelope.project_id != self.project_id {
            return Err(AiResponseValidationError::ProjectTargetDrift);
        }
        if envelope.source_revision != self.source_revision {
            return Err(AiResponseValidationError::SourceRevisionStale);
        }
        if envelope.result_schema != self.result_schema {
            return Err(AiResponseValidationError::ResultSchemaMismatch);
        }
        if envelope.safety_flags != self.required_safety_flags
            || !envelope.safety_flags.is_required_copy_only()
        {
            return Err(AiResponseValidationError::UnsafeSafetyFlags);
        }
        if envelope.evidence_refs.is_empty() {
            return Err(AiResponseValidationError::EvidenceRequired);
        }

        validate_reference_subset(
            &envelope.evidence_refs,
            &self.evidence_ids(),
            AiResponseValidationError::DuplicateEvidenceReference,
            AiResponseValidationError::UnknownEvidenceReference,
        )?;
        validate_reference_subset(
            &envelope.action_refs,
            &self.action_ids(),
            AiResponseValidationError::DuplicateActionReference,
            AiResponseValidationError::UnknownActionReference,
        )?;
        validate_structured_result(self.result_schema, &envelope.result)
    }

    pub fn parse_and_validate(
        &self,
        output: &str,
    ) -> Result<AiResponseEnvelope, AiResponseValidationError> {
        let output = output.trim();
        if output.is_empty() {
            return Err(AiResponseValidationError::EmptyResponse);
        }
        let envelope = serde_json::from_str::<AiResponseEnvelope>(output)
            .map_err(|_| AiResponseValidationError::MalformedJson)?;
        self.validate_envelope(&envelope)?;
        Ok(envelope)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct AiResponseEnvelope {
    pub schema_version: u32,
    pub request_kind: String,
    pub project_id: String,
    pub source_revision: String,
    pub result_schema: AiResultSchema,
    pub evidence_refs: Vec<String>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub action_refs: Vec<String>,
    pub result: Value,
    pub safety_flags: AiResponseSafetyFlags,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum AiResponseValidationError {
    EmptyResponse,
    MalformedJson,
    UnsupportedSchemaVersion(u32),
    MissingContractValue(&'static str),
    EvidenceRequired,
    InvalidEvidence(&'static str),
    InvalidAction(&'static str),
    DuplicateEvidenceReference(String),
    DuplicateActionReference(String),
    UnknownEvidenceReference(String),
    UnknownActionReference(String),
    ActionReferencesUnknownEvidence(String),
    ActionTargetDrift(String),
    RequestKindMismatch,
    ProjectTargetDrift,
    SourceRevisionStale,
    ResultSchemaMismatch,
    UnsafeSafetyFlags,
    InvalidResult(&'static str),
    ForbiddenResultKey(String),
}

impl fmt::Display for AiResponseValidationError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyResponse => write!(formatter, "provider returned no AI response envelope"),
            Self::MalformedJson => {
                write!(
                    formatter,
                    "provider response is not a valid AI response envelope"
                )
            }
            Self::UnsupportedSchemaVersion(version) => {
                write!(
                    formatter,
                    "unsupported AI response schema version {version}"
                )
            }
            Self::MissingContractValue(field) => {
                write!(formatter, "AI response contract field `{field}` is empty")
            }
            Self::EvidenceRequired => {
                write!(
                    formatter,
                    "AI response must reference at least one evidence item"
                )
            }
            Self::InvalidEvidence(message) => {
                write!(formatter, "invalid AI evidence contract: {message}")
            }
            Self::InvalidAction(message) => {
                write!(formatter, "invalid AI action contract: {message}")
            }
            Self::DuplicateEvidenceReference(reference) => {
                write!(formatter, "duplicate AI evidence reference `{reference}`")
            }
            Self::DuplicateActionReference(reference) => {
                write!(formatter, "duplicate AI action reference `{reference}`")
            }
            Self::UnknownEvidenceReference(reference) => {
                write!(formatter, "unknown AI evidence reference `{reference}`")
            }
            Self::UnknownActionReference(reference) => {
                write!(formatter, "unknown AI action reference `{reference}`")
            }
            Self::ActionReferencesUnknownEvidence(reference) => {
                write!(
                    formatter,
                    "AI action contract references unknown evidence `{reference}`"
                )
            }
            Self::ActionTargetDrift(action) => {
                write!(formatter, "AI action `{action}` targets another project")
            }
            Self::RequestKindMismatch => write!(formatter, "AI response request kind changed"),
            Self::ProjectTargetDrift => write!(formatter, "AI response project target changed"),
            Self::SourceRevisionStale => write!(formatter, "AI response source revision is stale"),
            Self::ResultSchemaMismatch => write!(formatter, "AI response result schema changed"),
            Self::UnsafeSafetyFlags => {
                write!(formatter, "AI response safety flags are not copy-only")
            }
            Self::InvalidResult(message) => {
                write!(formatter, "AI response result is invalid: {message}")
            }
            Self::ForbiddenResultKey(key) => {
                write!(
                    formatter,
                    "AI response result contains forbidden field `{key}`"
                )
            }
        }
    }
}

impl std::error::Error for AiResponseValidationError {}

fn require_contract_value(
    value: &str,
    field: &'static str,
) -> Result<(), AiResponseValidationError> {
    if value.trim().is_empty() {
        Err(AiResponseValidationError::MissingContractValue(field))
    } else {
        Ok(())
    }
}

fn validate_reference_subset<F, G>(
    references: &[String],
    allowed: &BTreeSet<&str>,
    duplicate: F,
    unknown: G,
) -> Result<(), AiResponseValidationError>
where
    F: Fn(String) -> AiResponseValidationError,
    G: Fn(String) -> AiResponseValidationError,
{
    let mut seen = HashSet::new();
    for reference in references {
        if reference.trim().is_empty() || !seen.insert(reference.as_str()) {
            return Err(duplicate(reference.clone()));
        }
        if !allowed.contains(reference.as_str()) {
            return Err(unknown(reference.clone()));
        }
    }
    Ok(())
}

fn validate_structured_result(
    schema: AiResultSchema,
    result: &Value,
) -> Result<(), AiResponseValidationError> {
    reject_forbidden_result_keys(result)?;
    let object = result
        .as_object()
        .ok_or(AiResponseValidationError::InvalidResult(
            "top-level result must be an object",
        ))?;
    match schema {
        AiResultSchema::CopyOnlyMarkdown => {
            require_nonempty_string(object.get("markdown"), "markdown")
        }
        AiResultSchema::TaskReadiness => {
            validate_task_readiness_summary(object.get("summary"))?;
            require_array(object.get("agent_candidates"), "agent_candidates")?;
            require_array(object.get("skill_candidates"), "skill_candidates")?;
            require_array(object.get("readiness_signals"), "readiness_signals")?;
            require_array(object.get("gap_rows"), "gap_rows")?;
            require_array(object.get("blocker_rows"), "blocker_rows")
        }
        AiResultSchema::SessionDigest => {
            require_nonempty_string(object.get("summary"), "summary")?;
            require_nonempty_string(object.get("suggested_next_prompt"), "suggested_next_prompt")?;
            require_array(object.get("evidence_notes"), "evidence_notes")?;
            require_array(object.get("uncertainties"), "uncertainties")
        }
        AiResultSchema::SkillChangeReview => {
            require_nonempty_string(object.get("summary"), "summary")?;
            require_array(object.get("changes"), "changes")?;
            require_array(object.get("risks"), "risks")?;
            require_array(object.get("recommendations"), "recommendations")
        }
    }
}

fn validate_task_readiness_summary(value: Option<&Value>) -> Result<(), AiResponseValidationError> {
    let summary = value
        .and_then(Value::as_object)
        .ok_or(AiResponseValidationError::InvalidResult("summary"))?;
    require_nonempty_string(summary.get("summary"), "summary.summary")?;
    for field in ["recommended_agent", "recommended_skill_name"] {
        if !summary
            .get(field)
            .is_some_and(|value| value.is_null() || value.is_string())
        {
            return Err(AiResponseValidationError::InvalidResult(field));
        }
    }
    for field in ["readiness_score", "routing_score"] {
        if summary
            .get(field)
            .and_then(Value::as_u64)
            .is_none_or(|value| value > 100)
        {
            return Err(AiResponseValidationError::InvalidResult(field));
        }
    }
    for field in ["gap_count", "blocker_count"] {
        if summary.get(field).and_then(Value::as_u64).is_none() {
            return Err(AiResponseValidationError::InvalidResult(field));
        }
    }
    Ok(())
}

fn require_nonempty_string(
    value: Option<&Value>,
    field: &'static str,
) -> Result<(), AiResponseValidationError> {
    if value
        .and_then(Value::as_str)
        .is_some_and(|value| !value.trim().is_empty())
    {
        Ok(())
    } else {
        Err(AiResponseValidationError::InvalidResult(field))
    }
}

fn require_array(
    value: Option<&Value>,
    field: &'static str,
) -> Result<(), AiResponseValidationError> {
    if value.is_some_and(Value::is_array) {
        Ok(())
    } else {
        Err(AiResponseValidationError::InvalidResult(field))
    }
}

fn reject_forbidden_result_keys(value: &Value) -> Result<(), AiResponseValidationError> {
    const FORBIDDEN_KEYS: &[&str] = &[
        "action_confirmation",
        "apply_method",
        "argv",
        "command",
        "commands",
        "execute",
        "execution",
        "mutation",
        "preview_token",
        "script",
        "scripts",
        "tool_call",
        "tool_calls",
        "write_back",
    ];
    match value {
        Value::Object(object) => {
            for (key, nested) in object {
                let normalized = key.trim().to_ascii_lowercase();
                if FORBIDDEN_KEYS.contains(&normalized.as_str()) {
                    return Err(AiResponseValidationError::ForbiddenResultKey(key.clone()));
                }
                reject_forbidden_result_keys(nested)?;
            }
        }
        Value::Array(values) => {
            for nested in values {
                reject_forbidden_result_keys(nested)?;
            }
        }
        _ => {}
    }
    Ok(())
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub enum Severity {
    Info,
    Warn,
    Error,
}

impl Severity {
    pub fn as_str(&self) -> &'static str {
        match self {
            Severity::Info => "info",
            Severity::Warn => "warn",
            Severity::Error => "error",
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct Finding {
    pub instance_id: Option<String>,
    pub definition_id: Option<String>,
    pub rule_id: String,
    pub severity: Severity,
    pub message: String,
    pub suggestion: Option<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct DefinitionSummary {
    pub id: String,
    pub canonical_name: String,
    pub description: String,
    pub instances: Vec<String>,
    pub active_instance: Option<String>,
    pub has_multiple_instances: bool,
    pub has_conflict: bool,
    pub fingerprint_set: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConflictSummary {
    pub id: String,
    pub definition_id: String,
    pub reason: String,
    pub winner_id: Option<String>,
    pub instances: Vec<String>,
}

#[derive(Debug, Default)]
pub struct RuleContext {
    pub previous_fingerprints: HashMap<String, String>,
    pub runtime_conflict_namespaces: Option<HashMap<String, String>>,
}

#[derive(Debug, Default)]
pub struct RuleReport {
    pub findings: Vec<Finding>,
    pub definitions: Vec<DefinitionSummary>,
    pub conflicts: Vec<ConflictSummary>,
}

pub trait Rule: Send + Sync {
    fn id(&self) -> &'static str;
    fn applies_to(&self, inst: &SkillInstance) -> bool;
    fn check(&self, inst: &SkillInstance, ctx: &RuleContext) -> Vec<Finding>;
}

pub fn evaluate_mvp_rules(instances: &[SkillInstance], ctx: &RuleContext) -> RuleReport {
    let rules: [&dyn Rule; 3] = [
        &FrontmatterRequiredFields,
        &PathOutsideWorkspace,
        &FingerprintChanged,
    ];
    let mut report = RuleReport::default();

    for inst in instances {
        for rule in rules {
            if rule.applies_to(inst) {
                report.findings.extend(rule.check(inst, ctx));
            }
        }
    }

    append_name_collision_results(instances, ctx, &mut report);
    report
}

struct FrontmatterRequiredFields;

impl Rule for FrontmatterRequiredFields {
    fn id(&self) -> &'static str {
        "frontmatter.required-fields"
    }

    fn applies_to(&self, _inst: &SkillInstance) -> bool {
        true
    }

    fn check(&self, inst: &SkillInstance, _ctx: &RuleContext) -> Vec<Finding> {
        let missing = missing_frontmatter_fields(inst);
        if missing.is_empty() {
            return Vec::new();
        }
        vec![Finding {
            instance_id: Some(inst.id.clone()),
            definition_id: Some(inst.definition_id.clone()),
            rule_id: self.id().to_string(),
            severity: Severity::Error,
            message: format!(
                "Missing required frontmatter fields: {}",
                missing.join(", ")
            ),
            suggestion: Some(
                "Add name and description to the SKILL.md YAML frontmatter.".to_string(),
            ),
        }]
    }
}

fn missing_frontmatter_fields(inst: &SkillInstance) -> Vec<&'static str> {
    if inst.frontmatter_raw.trim().is_empty() {
        return vec!["name", "description"];
    }
    let Ok(value) = serde_norway::from_str::<serde_norway::Value>(&inst.frontmatter_raw) else {
        return vec!["name", "description"];
    };
    let mut missing = Vec::new();
    if yaml_string_field(&value, "name").is_none() {
        missing.push("name");
    }
    if yaml_string_field(&value, "description").is_none() {
        missing.push("description");
    }
    missing
}

fn yaml_string_field<'a>(value: &'a serde_norway::Value, key: &str) -> Option<&'a str> {
    value
        .get(serde_norway::Value::String(key.to_string()))
        .and_then(serde_norway::Value::as_str)
        .map(str::trim)
        .filter(|s| !s.is_empty())
}

struct PathOutsideWorkspace;

impl Rule for PathOutsideWorkspace {
    fn id(&self) -> &'static str {
        "path.outside-workspace"
    }

    fn applies_to(&self, inst: &SkillInstance) -> bool {
        inst.scope == Scope::AgentProject
    }

    fn check(&self, inst: &SkillInstance, _ctx: &RuleContext) -> Vec<Finding> {
        let Some(project_root) = &inst.project_root else {
            return vec![outside_workspace_finding(
                inst,
                "Project-scoped skill has no project root.",
            )];
        };
        if inst.path.starts_with(project_root) {
            return Vec::new();
        }
        vec![outside_workspace_finding(
            inst,
            "Project-scoped skill path is outside its project root.",
        )]
    }
}

fn outside_workspace_finding(inst: &SkillInstance, message: &str) -> Finding {
    Finding {
        instance_id: Some(inst.id.clone()),
        definition_id: Some(inst.definition_id.clone()),
        rule_id: "path.outside-workspace".to_string(),
        severity: Severity::Error,
        message: message.to_string(),
        suggestion: Some(
            "Move the skill under <project>/.claude/skills or rescan it as a global skill."
                .to_string(),
        ),
    }
}

struct FingerprintChanged;

impl Rule for FingerprintChanged {
    fn id(&self) -> &'static str {
        "fingerprint.changed"
    }

    fn applies_to(&self, inst: &SkillInstance) -> bool {
        !inst.fingerprint.is_empty()
    }

    fn check(&self, inst: &SkillInstance, ctx: &RuleContext) -> Vec<Finding> {
        let Some(previous) = ctx.previous_fingerprints.get(&inst.id) else {
            return Vec::new();
        };
        if previous == &inst.fingerprint {
            return Vec::new();
        }
        vec![Finding {
            instance_id: Some(inst.id.clone()),
            definition_id: Some(inst.definition_id.clone()),
            rule_id: self.id().to_string(),
            severity: Severity::Info,
            message: "Skill content fingerprint changed since the previous scan.".to_string(),
            suggestion: Some(
                "Review the skill details before relying on this version.".to_string(),
            ),
        }]
    }
}

fn append_name_collision_results(
    instances: &[SkillInstance],
    ctx: &RuleContext,
    report: &mut RuleReport,
) {
    let mut groups: BTreeMap<&str, Vec<&SkillInstance>> = BTreeMap::new();
    for inst in instances {
        groups
            .entry(inst.definition_id.as_str())
            .or_default()
            .push(inst);
    }

    for (definition_id, group) in groups {
        let canonical_name = group[0].name.clone();
        let description = group
            .iter()
            .find(|inst| !inst.description.trim().is_empty())
            .map(|inst| inst.description.clone())
            .unwrap_or_default();
        let instances: Vec<String> = group.iter().map(|inst| inst.id.clone()).collect();
        let fingerprint_set: Vec<String> = group
            .iter()
            .map(|inst| inst.fingerprint.clone())
            .filter(|fp| !fp.is_empty())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect();
        let runtime_conflicts = runtime_name_collision_groups(&group, ctx);
        let has_multiple_instances = instances.len() > 1;
        let has_conflict = !runtime_conflicts.is_empty();

        report.definitions.push(DefinitionSummary {
            id: definition_id.to_string(),
            canonical_name: canonical_name.clone(),
            description,
            active_instance: group
                .iter()
                .find(|inst| inst.state == SkillState::Loaded && inst.enabled)
                .map(|inst| inst.id.clone())
                .or_else(|| group.first().map(|inst| inst.id.clone())),
            instances: instances.clone(),
            has_multiple_instances,
            has_conflict,
            fingerprint_set: fingerprint_set.clone(),
        });

        for (agent, runtime_namespace, collision_group) in runtime_conflicts {
            let collision_instances: Vec<String> =
                collision_group.iter().map(|inst| inst.id.clone()).collect();
            let collision_fingerprints: Vec<String> = collision_group
                .iter()
                .map(|inst| inst.fingerprint.clone())
                .filter(|fp| !fp.is_empty())
                .collect::<BTreeSet<_>>()
                .into_iter()
                .collect();
            let has_content_drift = collision_fingerprints.len() > 1;
            let reason = if has_content_drift {
                "content-drift"
            } else {
                "name-collision"
            };
            let id = if runtime_namespace == NATIVE_RUNTIME_NAMESPACE
                || runtime_namespace == "native-plus-plugin"
            {
                format!("{definition_id}:{agent}:{reason}")
            } else {
                format!("{definition_id}:{agent}:{runtime_namespace}:{reason}")
            };
            report.conflicts.push(ConflictSummary {
                id,
                definition_id: definition_id.to_string(),
                reason: reason.to_string(),
                winner_id: None,
                instances: collision_instances.clone(),
            });

            let severity = if has_content_drift {
                Severity::Warn
            } else {
                Severity::Info
            };
            for inst in collision_group {
                report.findings.push(Finding {
                    instance_id: Some(inst.id.clone()),
                    definition_id: Some(definition_id.to_string()),
                    rule_id: "name.collision".to_string(),
                    severity: severity.clone(),
                    message: format!(
                        "{} runtime sees skill name '{}' in {} locations.",
                        inst.agent.as_str(),
                        canonical_name,
                        collision_instances.len()
                    ),
                    suggestion: Some(
                        if has_content_drift {
                            "Compare the conflicting skill bodies and choose the intended version."
                        } else {
                            "Confirm that duplicate skill locations are intentional."
                        }
                        .to_string(),
                    ),
                });
            }
        }
    }
}

fn runtime_name_collision_groups<'a>(
    group: &[&'a SkillInstance],
    ctx: &RuleContext,
) -> Vec<(String, String, Vec<&'a SkillInstance>)> {
    let mut by_agent: BTreeMap<String, BTreeMap<String, Vec<&SkillInstance>>> = BTreeMap::new();
    for inst in group {
        if inst.state != SkillState::Loaded || !inst.enabled {
            continue;
        }
        let runtime_namespace = match &ctx.runtime_conflict_namespaces {
            Some(namespaces) => {
                let Some(namespace) = namespaces.get(&inst.id) else {
                    continue;
                };
                namespace.clone()
            }
            None => NATIVE_RUNTIME_NAMESPACE.to_string(),
        };
        by_agent
            .entry(inst.agent.as_str().to_string())
            .or_default()
            .entry(runtime_namespace)
            .or_default()
            .push(*inst);
    }

    let mut conflicts = Vec::new();
    for (agent, mut by_namespace) in by_agent {
        let native = by_namespace
            .remove(NATIVE_RUNTIME_NAMESPACE)
            .unwrap_or_default();
        if !native.is_empty() && !by_namespace.is_empty() {
            let mut members = native;
            members.extend(by_namespace.into_values().flatten());
            if has_distinct_runtime_paths(&members) {
                conflicts.push((agent, "native-plus-plugin".to_string(), members));
            }
            continue;
        }

        if has_distinct_runtime_paths(&native) {
            conflicts.push((agent.clone(), NATIVE_RUNTIME_NAMESPACE.to_string(), native));
        }
        for (runtime_namespace, members) in by_namespace {
            if has_distinct_runtime_paths(&members) {
                conflicts.push((agent.clone(), runtime_namespace, members));
            }
        }
    }
    conflicts
}

fn has_distinct_runtime_paths(members: &[&SkillInstance]) -> bool {
    members.len() > 1
        && members
            .iter()
            .map(|inst| inst.path.to_string_lossy().to_string())
            .collect::<BTreeSet<_>>()
            .len()
            > 1
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use skills_copilot_core::{
        ActionImpact, ActionIntent, ActionKind, ActionNetworkPosture, ActionReadbackDomain,
        ActionTargetKind, ActionTargetRef, AgentId, EvidenceKind, NetworkAccess, PermissionRequest,
        Scope, SkillInstance, SkillState,
    };

    use super::*;

    #[test]
    fn evidence_bound_response_accepts_known_copy_only_references() {
        let contract = response_contract(AiResultSchema::TaskReadiness, true);
        let envelope = AiResponseEnvelope {
            schema_version: AI_RESPONSE_ENVELOPE_SCHEMA_VERSION,
            request_kind: "task_cockpit".to_string(),
            project_id: "project-test".to_string(),
            source_revision: "source-1".to_string(),
            result_schema: AiResultSchema::TaskReadiness,
            evidence_refs: vec!["evidence:skill".to_string()],
            action_refs: vec!["action:inspect".to_string()],
            result: serde_json::json!({
                "summary": {
                    "summary": "Ready",
                    "recommended_agent": null,
                    "recommended_skill_name": null,
                    "readiness_score": 80,
                    "routing_score": 75,
                    "gap_count": 0,
                    "blocker_count": 0
                },
                "agent_candidates": [],
                "skill_candidates": [],
                "readiness_signals": [],
                "gap_rows": [],
                "blocker_rows": []
            }),
            safety_flags: AiResponseSafetyFlags::required_copy_only(),
        };

        contract
            .validate_envelope(&envelope)
            .expect("known evidence-bound result should validate");
        let encoded = serde_json::to_string(&envelope).expect("encode envelope");
        assert_eq!(
            contract
                .parse_and_validate(&encoded)
                .expect("roundtrip envelope"),
            envelope
        );
    }

    #[test]
    fn evidence_bound_response_rejects_stale_unknown_and_drifted_references() {
        let contract = response_contract(AiResultSchema::TaskReadiness, true);
        let mut envelope = valid_task_readiness_envelope();

        let mut drifted_contract = contract.clone();
        drifted_contract.actions[0].project_id = Some("project-other".to_string());
        assert_eq!(
            drifted_contract.validate(),
            Err(AiResponseValidationError::ActionTargetDrift(
                "action:inspect".to_string()
            ))
        );

        envelope.source_revision = "source-stale".to_string();
        assert_eq!(
            contract.validate_envelope(&envelope),
            Err(AiResponseValidationError::SourceRevisionStale)
        );

        envelope.source_revision = "source-1".to_string();
        envelope.evidence_refs = vec!["evidence:unknown".to_string()];
        assert_eq!(
            contract.validate_envelope(&envelope),
            Err(AiResponseValidationError::UnknownEvidenceReference(
                "evidence:unknown".to_string()
            ))
        );

        envelope.evidence_refs = vec!["evidence:skill".to_string()];
        envelope.action_refs = vec!["action:unknown".to_string()];
        assert_eq!(
            contract.validate_envelope(&envelope),
            Err(AiResponseValidationError::UnknownActionReference(
                "action:unknown".to_string()
            ))
        );

        envelope.action_refs = vec!["action:inspect".to_string()];
        envelope.project_id = "project-other".to_string();
        assert_eq!(
            contract.validate_envelope(&envelope),
            Err(AiResponseValidationError::ProjectTargetDrift)
        );
    }

    #[test]
    fn evidence_bound_response_rejects_unsafe_or_command_shaped_results() {
        let contract = response_contract(AiResultSchema::SessionDigest, false);
        let mut envelope = AiResponseEnvelope {
            schema_version: AI_RESPONSE_ENVELOPE_SCHEMA_VERSION,
            request_kind: "task_cockpit".to_string(),
            project_id: "project-test".to_string(),
            source_revision: "source-1".to_string(),
            result_schema: AiResultSchema::SessionDigest,
            evidence_refs: vec!["evidence:skill".to_string()],
            action_refs: Vec::new(),
            result: serde_json::json!({
                "summary": "Current work summary",
                "suggested_next_prompt": "Continue reviewing the current evidence.",
                "evidence_notes": [],
                "uncertainties": []
            }),
            safety_flags: AiResponseSafetyFlags::required_copy_only(),
        };

        envelope.safety_flags.command_execution_allowed = true;
        assert_eq!(
            contract.validate_envelope(&envelope),
            Err(AiResponseValidationError::UnsafeSafetyFlags)
        );

        envelope.safety_flags = AiResponseSafetyFlags::required_copy_only();
        envelope.result = serde_json::json!({
            "summary": "Current work summary",
            "suggested_next_prompt": "Continue reviewing the current evidence.",
            "evidence_notes": [],
            "uncertainties": [],
            "argv": ["unsafe"]
        });
        assert_eq!(
            contract.validate_envelope(&envelope),
            Err(AiResponseValidationError::ForbiddenResultKey(
                "argv".to_string()
            ))
        );
    }

    #[test]
    fn evidence_bound_response_enforces_each_structured_result_schema() {
        let cases = [
            (
                AiResultSchema::CopyOnlyMarkdown,
                serde_json::json!({"markdown": "Evidence-bound explanation."}),
            ),
            (
                AiResultSchema::TaskReadiness,
                serde_json::json!({
                    "summary": {
                        "summary": "Ready",
                        "recommended_agent": null,
                        "recommended_skill_name": null,
                        "readiness_score": 80,
                        "routing_score": 75,
                        "gap_count": 0,
                        "blocker_count": 0
                    },
                    "agent_candidates": [],
                    "skill_candidates": [],
                    "readiness_signals": [],
                    "gap_rows": [],
                    "blocker_rows": []
                }),
            ),
            (
                AiResultSchema::SessionDigest,
                serde_json::json!({
                    "summary": "Digest",
                    "suggested_next_prompt": "Continue",
                    "evidence_notes": [],
                    "uncertainties": []
                }),
            ),
            (
                AiResultSchema::SkillChangeReview,
                serde_json::json!({
                    "summary": "Review",
                    "changes": [],
                    "risks": [],
                    "recommendations": []
                }),
            ),
        ];

        for (schema, result) in cases {
            let contract = response_contract(schema, false);
            let envelope = AiResponseEnvelope {
                schema_version: AI_RESPONSE_ENVELOPE_SCHEMA_VERSION,
                request_kind: "task_cockpit".to_string(),
                project_id: "project-test".to_string(),
                source_revision: "source-1".to_string(),
                result_schema: schema,
                evidence_refs: vec!["evidence:skill".to_string()],
                action_refs: Vec::new(),
                result,
                safety_flags: AiResponseSafetyFlags::required_copy_only(),
            };
            contract
                .validate_envelope(&envelope)
                .unwrap_or_else(|error| panic!("schema {} failed: {error}", schema.as_str()));
        }
    }

    fn response_contract(schema: AiResultSchema, include_action: bool) -> AiResponseContract {
        let evidence = EvidenceRef {
            id: "evidence:skill".to_string(),
            kind: EvidenceKind::SkillInstance,
            source_revision: "source-1".to_string(),
            summary: "Selected skill evidence".to_string(),
            agent: Some(AgentId::Codex),
            target_id: Some("skill-1".to_string()),
        };
        let actions = include_action
            .then(|| ActionDescriptor {
                id: "action:inspect".to_string(),
                kind: ActionKind::RefreshEvidence,
                intent: ActionIntent::InspectEvidence,
                target: ActionTargetRef {
                    kind: ActionTargetKind::Skill,
                    id: "skill-1".to_string(),
                    agent: Some(AgentId::Codex),
                    scope: Some(Scope::AgentProject),
                },
                project_id: Some("project-test".to_string()),
                impacts: vec![ActionImpact::ReadOnly],
                preview_method: "catalog.getSkill".to_string(),
                apply_method: None,
                source_revision: "source-1".to_string(),
                confirmation_required: false,
                network: ActionNetworkPosture::None,
                readback: vec![ActionReadbackDomain::SkillAggregates],
                evidence_refs: vec!["evidence:skill".to_string()],
            })
            .into_iter()
            .collect();
        AiResponseContract::new(
            "task_cockpit",
            "project-test",
            "source-1",
            schema,
            vec![evidence],
            actions,
        )
        .expect("valid response contract")
    }

    fn valid_task_readiness_envelope() -> AiResponseEnvelope {
        AiResponseEnvelope {
            schema_version: AI_RESPONSE_ENVELOPE_SCHEMA_VERSION,
            request_kind: "task_cockpit".to_string(),
            project_id: "project-test".to_string(),
            source_revision: "source-1".to_string(),
            result_schema: AiResultSchema::TaskReadiness,
            evidence_refs: vec!["evidence:skill".to_string()],
            action_refs: vec!["action:inspect".to_string()],
            result: serde_json::json!({
                "summary": {
                    "summary": "Ready",
                    "recommended_agent": null,
                    "recommended_skill_name": null,
                    "readiness_score": 80,
                    "routing_score": 75,
                    "gap_count": 0,
                    "blocker_count": 0
                },
                "agent_candidates": [],
                "skill_candidates": [],
                "readiness_signals": [],
                "gap_rows": [],
                "blocker_rows": []
            }),
            safety_flags: AiResponseSafetyFlags::required_copy_only(),
        }
    }

    #[test]
    fn yaml_contract_frontmatter_required_fields_handles_nested_values_and_malformed_input() {
        let valid = skill(
            "yaml-valid",
            "yaml-valid",
            "yaml-valid",
            "name: yaml-valid\ndescription: Valid\nenabled: true\nallowed-tools:\n  - Read\nmetadata:\n  openclaw:\n    skillKey: routed-key\n",
            "body",
        );
        let valid_report = evaluate_mvp_rules(&[valid], &RuleContext::default());
        assert!(valid_report
            .findings
            .iter()
            .all(|finding| finding.rule_id != "frontmatter.required-fields"));

        let malformed = skill(
            "yaml-malformed",
            "yaml-malformed",
            "yaml-malformed",
            "name: [unterminated\n",
            "body",
        );
        let malformed_report = evaluate_mvp_rules(&[malformed], &RuleContext::default());
        assert!(malformed_report
            .findings
            .iter()
            .any(|finding| finding.rule_id == "frontmatter.required-fields"));
    }

    #[test]
    fn required_fields_reports_missing_description() {
        let inst = skill("a", "same", "same", "---\nname: same\n---\n", "body");
        let report = evaluate_mvp_rules(&[inst], &RuleContext::default());

        assert!(report.findings.iter().any(|finding| {
            finding.rule_id == "frontmatter.required-fields"
                && finding.message.contains("description")
        }));
    }

    #[test]
    fn same_agent_name_collision_creates_conflict_and_findings() {
        let first = skill(
            "a",
            "same",
            "same",
            "---\nname: same\ndescription: A\n---\n",
            "body a",
        );
        let second = skill(
            "b",
            "same",
            "same",
            "---\nname: same\ndescription: B\n---\n",
            "body b",
        );
        let report = evaluate_mvp_rules(&[first, second], &RuleContext::default());

        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].reason, "content-drift");
        assert_eq!(
            report
                .findings
                .iter()
                .filter(|finding| finding.rule_id == "name.collision")
                .count(),
            2
        );
    }

    #[test]
    fn cross_agent_duplicate_does_not_create_runtime_conflict() {
        let first = skill(
            "claude",
            "same",
            "same",
            "---\nname: same\ndescription: A\n---\n",
            "body a",
        );
        let mut second = skill(
            "codex",
            "same",
            "same",
            "---\nname: same\ndescription: B\n---\n",
            "body b",
        );
        second.agent = AgentId::Codex;

        let report = evaluate_mvp_rules(&[first, second], &RuleContext::default());
        let definition = report
            .definitions
            .iter()
            .find(|definition| definition.id == "same")
            .expect("definition");

        assert!(definition.has_multiple_instances);
        assert!(!definition.has_conflict);
        assert!(report.conflicts.is_empty());
        assert!(report
            .findings
            .iter()
            .all(|finding| finding.rule_id != "name.collision"));
    }

    #[test]
    fn same_agent_same_path_duplicate_does_not_create_runtime_conflict() {
        let first = skill(
            "a",
            "same",
            "same",
            "---\nname: same\ndescription: A\n---\n",
            "body a",
        );
        let mut second = skill(
            "b",
            "same",
            "same",
            "---\nname: same\ndescription: B\n---\n",
            "body b",
        );
        second.path = first.path.clone();
        second.display_path = first.display_path.clone();

        let report = evaluate_mvp_rules(&[first, second], &RuleContext::default());
        assert!(report.conflicts.is_empty());
    }

    #[test]
    fn disabled_and_missing_instances_do_not_create_runtime_conflicts() {
        let first = skill(
            "loaded",
            "same",
            "same",
            "---\nname: same\ndescription: Loaded\n---\n",
            "loaded body",
        );
        let mut disabled = skill(
            "disabled",
            "same",
            "same",
            "---\nname: same\ndescription: Disabled\n---\n",
            "disabled body",
        );
        disabled.state = SkillState::Disabled;
        disabled.enabled = false;
        let mut missing = skill(
            "missing",
            "same",
            "same",
            "---\nname: same\ndescription: Missing\n---\n",
            "missing body",
        );
        missing.state = SkillState::Missing;

        let report = evaluate_mvp_rules(&[first, disabled, missing], &RuleContext::default());

        assert!(report.conflicts.is_empty());
        assert!(report
            .findings
            .iter()
            .all(|finding| finding.rule_id != "name.collision"));
    }

    #[test]
    fn distinct_plugin_namespaces_do_not_collide_on_raw_skill_name() {
        let first = skill(
            "plugin-a",
            "same",
            "same",
            "---\nname: same\ndescription: Plugin A\n---\n",
            "plugin a body",
        );
        let second = skill(
            "plugin-b",
            "same",
            "same",
            "---\nname: same\ndescription: Plugin B\n---\n",
            "plugin b body",
        );
        let ctx = RuleContext {
            runtime_conflict_namespaces: Some(HashMap::from([
                (first.id.clone(), "plugin:package-a@publisher".to_string()),
                (second.id.clone(), "plugin:package-b@publisher".to_string()),
            ])),
            ..Default::default()
        };

        let report = evaluate_mvp_rules(&[first, second], &ctx);

        assert!(report.conflicts.is_empty());
    }

    #[test]
    fn native_and_active_plugin_instances_with_same_name_still_conflict() {
        let native = skill(
            "native",
            "same",
            "same",
            "---\nname: same\ndescription: Native\n---\n",
            "native body",
        );
        let plugin = skill(
            "plugin",
            "same",
            "same",
            "---\nname: same\ndescription: Plugin\n---\n",
            "plugin body",
        );
        let ctx = RuleContext {
            runtime_conflict_namespaces: Some(HashMap::from([
                (native.id.clone(), NATIVE_RUNTIME_NAMESPACE.to_string()),
                (plugin.id.clone(), "plugin:same@publisher".to_string()),
            ])),
            ..Default::default()
        };

        let report = evaluate_mvp_rules(&[native, plugin], &ctx);

        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].reason, "content-drift");
    }

    #[test]
    fn duplicate_paths_inside_one_plugin_namespace_still_conflict() {
        let first = skill(
            "plugin-a-first",
            "same",
            "same",
            "---\nname: same\ndescription: Plugin A\n---\n",
            "same body",
        );
        let second = skill(
            "plugin-a-second",
            "same",
            "same",
            "---\nname: same\ndescription: Plugin A\n---\n",
            "same body",
        );
        let ctx = RuleContext {
            runtime_conflict_namespaces: Some(HashMap::from([
                (first.id.clone(), "plugin:package-a@publisher".to_string()),
                (second.id.clone(), "plugin:package-a@publisher".to_string()),
            ])),
            ..Default::default()
        };

        let report = evaluate_mvp_rules(&[first, second], &ctx);

        assert_eq!(report.conflicts.len(), 1);
        assert_eq!(report.conflicts[0].reason, "name-collision");
    }

    #[test]
    fn path_outside_workspace_flags_project_skill() {
        let mut inst = skill(
            "a",
            "same",
            "same",
            "---\nname: same\ndescription: A\n---\n",
            "body",
        );
        inst.scope = Scope::AgentProject;
        inst.project_root = Some(PathBuf::from("/tmp/project"));
        inst.path = PathBuf::from("/tmp/other/.claude/skills/same/SKILL.md");

        let report = evaluate_mvp_rules(&[inst], &RuleContext::default());

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.rule_id == "path.outside-workspace"));
    }

    #[test]
    fn fingerprint_changed_compares_previous_scan() {
        let inst = skill(
            "a",
            "same",
            "same",
            "---\nname: same\ndescription: A\n---\n",
            "body",
        );
        let mut ctx = RuleContext::default();
        ctx.previous_fingerprints
            .insert(inst.id.clone(), "old-fingerprint".to_string());

        let report = evaluate_mvp_rules(&[inst], &ctx);

        assert!(report
            .findings
            .iter()
            .any(|finding| finding.rule_id == "fingerprint.changed"));
    }

    fn skill(
        id: &str,
        definition_id: &str,
        name: &str,
        frontmatter_raw: &str,
        body: &str,
    ) -> SkillInstance {
        SkillInstance {
            id: id.to_string(),
            agent: AgentId::ClaudeCode,
            scope: Scope::AgentGlobal,
            project_root: None,
            path: PathBuf::from(format!("/tmp/{id}/{name}/SKILL.md")),
            display_path: PathBuf::from(format!("/tmp/{id}/{name}/SKILL.md")),
            definition_id: definition_id.to_string(),
            name: name.to_string(),
            display_name: name.to_string(),
            description: "description".to_string(),
            version: None,
            state: SkillState::Loaded,
            enabled: true,
            frontmatter_raw: frontmatter_raw.to_string(),
            body: body.to_string(),
            scripts: Vec::new(),
            permissions: PermissionRequest {
                network: NetworkAccess::None,
                ..PermissionRequest::default()
            },
            fingerprint: format!("{body}-fingerprint"),
            mtime: 0,
            first_seen: 0,
            last_seen: 0,
        }
    }
}
