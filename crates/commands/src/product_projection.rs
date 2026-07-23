use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashMap, HashSet},
};

use serde::{Deserialize, Serialize};
use sha2::{Digest, Sha256};
use skills_copilot_catalog::{ConflictGroupRecord, RuleFindingRecord};
use skills_copilot_core::{
    ActionDescriptor, ActionImpact, ActionTargetKind, ActionTargetRef, AgentId,
    AgentReadinessRecord, AttentionItem, AttentionKind, AttentionSeverity, EnvironmentHealthState,
    EvidenceKind, EvidenceRef, ListIncompleteReason, NoSafeActionReason, ProjectReadinessRecord,
    ReadinessBlocker, ResumeCapability, ResumeUnsupportedReason, Scope, SessionContinuationRecord,
    SkillAggregateRecord, SkillEffectivenessCount, SkillEffectivenessState,
    SkillInstanceEffectivenessRecord, SkillState, SourceCoverage,
};
use thiserror::Error;

use super::AgentCatalogScanReport;
use crate::action_lifecycle::{
    deterministic_action_id, validate_action_intent, validate_action_method_ownership,
};

const PRODUCT_AGENTS: [AgentId; 6] = [
    AgentId::ClaudeCode,
    AgentId::Codex,
    AgentId::Opencode,
    AgentId::Pi,
    AgentId::Hermes,
    AgentId::Openclaw,
];

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct AgentProjectionInput {
    pub agent: AgentId,
    pub source_revision: String,
    pub coverage: SourceCoverage,
    pub evidence_summary: String,
    pub action_ids: Vec<String>,
}

impl AgentProjectionInput {
    pub fn from_scan_report(report: &AgentCatalogScanReport, source_revision: &str) -> Self {
        Self {
            agent: report.agent,
            source_revision: source_revision.to_string(),
            coverage: source_coverage_from_scan_report(report),
            evidence_summary: format!(
                "{} adapter scan evidence was evaluated",
                agent_display_name(report.agent)
            ),
            action_ids: Vec::new(),
        }
    }

    pub fn stale(agent: AgentId, source_revision: &str) -> Self {
        Self {
            agent,
            source_revision: source_revision.to_string(),
            coverage: SourceCoverage::incomplete(0, None, ListIncompleteReason::StaleSource),
            evidence_summary: format!(
                "{} adapter evidence is stale and requires refresh",
                agent_display_name(agent)
            ),
            action_ids: Vec::new(),
        }
    }

    pub fn not_inspected(agent: AgentId, source_revision: &str) -> Self {
        Self {
            agent,
            source_revision: source_revision.to_string(),
            coverage: SourceCoverage::unknown(ListIncompleteReason::NotInspected),
            evidence_summary: format!(
                "{} has no current adapter inspection evidence",
                agent_display_name(agent)
            ),
            action_ids: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SkillProjectionInput {
    pub instance_id: String,
    pub agent: Option<AgentId>,
    pub scope: Scope,
    pub definition_id: String,
    pub definition_fingerprint: Option<String>,
    pub canonical_name: String,
    pub display_name: String,
    pub description: String,
    pub publisher: Option<String>,
    pub package_name: Option<String>,
    pub package_version: Option<String>,
    pub source_kind: String,
    pub source_identity: String,
    pub runtime_identity: String,
    pub source_revision: String,
    pub read_only_reason: Option<String>,
    pub installed: bool,
    pub linked: bool,
    pub enabled: bool,
    pub precedence_proven: bool,
    pub adapter_state: SkillState,
    pub coverage: SourceCoverage,
    pub evidence_summary: String,
    pub action_ids: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SessionProjectionInput {
    pub id: String,
    pub agent: AgentId,
    pub project_id: Option<String>,
    pub title: String,
    pub intent: Option<String>,
    pub started_at: Option<i64>,
    pub ended_at: Option<i64>,
    pub modified_at: i64,
    pub source_kind: String,
    pub source_revision: String,
    /// Revision of the accepted product snapshot containing this source row.
    pub snapshot_revision: String,
    pub coverage: SourceCoverage,
    /// Adapter-verified capability. The projection never constructs argv.
    pub resume: ResumeCapability,
    pub evidence_summary: String,
    pub action_ids: Vec<String>,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct FindingProjectionInput {
    pub source_revision: String,
    pub record: RuleFindingRecord,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConflictProjectionInput {
    pub source_revision: String,
    pub record: ConflictGroupRecord,
}

/// Complete, adapter-normalized facts accepted under one immutable revision.
///
/// Callers must finish adapter-specific plugin, compatibility, precedence,
/// config, manager-link, and project-match normalization before constructing
/// this value. Task 4 owns I/O and accepted-snapshot assembly; this projector
/// never reads mutable sources or infers provenance from paths.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ProductProjectionInput {
    pub project_id: String,
    pub project_display_name: String,
    pub source_revision: String,
    pub agent_sources: Vec<AgentProjectionInput>,
    /// Catalog, adapter, config, and manager facts captured at source_revision.
    pub skills: Vec<SkillProjectionInput>,
    /// Current catalog findings captured in the same logical snapshot.
    pub findings: Vec<FindingProjectionInput>,
    /// Current runtime conflicts captured in the same logical snapshot.
    pub conflicts: Vec<ConflictProjectionInput>,
    pub sessions: Vec<SessionProjectionInput>,
    /// Existing typed capabilities only. Task 2 does not mint authorization.
    pub actions: Vec<ActionDescriptor>,
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize, Deserialize)]
pub struct ProductProjection {
    pub readiness: ProjectReadinessRecord,
    pub skill_aggregates: Vec<SkillAggregateRecord>,
    pub session_continuations: Vec<SessionContinuationRecord>,
}

#[derive(Debug, Error, Clone, Eq, PartialEq)]
pub enum ProductProjectionError {
    #[error("{0}")]
    Invalid(String),
}

#[derive(Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
struct SkillAggregateKey {
    definition_id: String,
    definition_fingerprint: String,
    source_identity: String,
    source_kind: String,
    package_identity: String,
    scope: String,
    runtime_identity: String,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum ConflictDisposition {
    Winner,
    Shadowed,
    Unresolved,
}

pub fn source_coverage_from_scan_report(report: &AgentCatalogScanReport) -> SourceCoverage {
    let scanned = report.scanned_roots.iter().collect::<BTreeSet<_>>();
    let partial = report.partial_roots.iter().collect::<BTreeSet<_>>();
    let skipped = report.skipped_roots.iter().collect::<BTreeSet<_>>();
    let expected = scanned
        .iter()
        .chain(partial.iter())
        .chain(skipped.iter())
        .copied()
        .collect::<BTreeSet<_>>()
        .len();
    let inspected = scanned
        .iter()
        .filter(|path| !partial.contains(**path) && !skipped.contains(**path))
        .count();

    if report.budget_exhausted {
        return SourceCoverage::incomplete(
            inspected,
            Some(expected),
            ListIncompleteReason::SafetyBudget,
        );
    }
    if !partial.is_empty() {
        let reason = if report.issues.iter().any(|issue| {
            matches!(
                issue.kind,
                "directory_unreadable" | "entry_unreadable" | "file_unreadable"
            )
        }) {
            ListIncompleteReason::UnreadableSource
        } else {
            ListIncompleteReason::SourceLimited
        };
        return SourceCoverage::incomplete(inspected, Some(expected), reason);
    }
    if !skipped.is_empty() {
        let reason = if report.issues.iter().any(|issue| {
            matches!(
                issue.kind,
                "root_unavailable"
                    | "directory_unreadable"
                    | "entry_unreadable"
                    | "file_unreadable"
            )
        }) {
            ListIncompleteReason::UnreadableSource
        } else {
            ListIncompleteReason::SourceLimited
        };
        return SourceCoverage::incomplete(inspected, Some(expected), reason);
    }
    SourceCoverage::enumerable(inspected, Some(expected))
}

pub fn derive_product_projection(
    input: ProductProjectionInput,
) -> Result<ProductProjection, ProductProjectionError> {
    validate_identity(&input.project_id, "project id")?;
    validate_display(&input.project_display_name, "project display name")?;
    validate_identity(&input.source_revision, "project source revision")?;

    for source in &input.agent_sources {
        validate_component_revision(
            &source.source_revision,
            &input.source_revision,
            "agent projection",
        )?;
    }
    for skill in &input.skills {
        validate_component_revision(
            &skill.source_revision,
            &input.source_revision,
            "skill projection",
        )?;
    }
    for finding in &input.findings {
        validate_component_revision(
            &finding.source_revision,
            &input.source_revision,
            "finding projection",
        )?;
    }
    for conflict in &input.conflicts {
        validate_component_revision(
            &conflict.source_revision,
            &input.source_revision,
            "conflict projection",
        )?;
    }
    for session in &input.sessions {
        validate_component_revision(
            &session.snapshot_revision,
            &input.source_revision,
            "session projection",
        )?;
    }

    let actions = action_map(&input.actions, &input.source_revision)?;
    let agent_sources =
        normalized_agent_sources(input.agent_sources.clone(), &input.source_revision)?;
    validate_agent_source_actions(&agent_sources, &actions)?;
    let findings = input
        .findings
        .iter()
        .map(|input| input.record.clone())
        .collect::<Vec<_>>();
    let conflicts = input
        .conflicts
        .iter()
        .map(|input| input.record.clone())
        .collect::<Vec<_>>();
    let visible_findings = super::user_visible_rule_findings(&findings);
    let conflict_dispositions = conflict_dispositions(&conflicts, &input.skills)?;
    let skill_aggregates = derive_skill_aggregates(
        &input.source_revision,
        input.skills.clone(),
        &visible_findings,
        &conflicts,
        &conflict_dispositions,
        &actions,
    )?;
    let session_continuations =
        project_session_continuations(&input.project_id, input.sessions.clone(), &actions)?;
    let readiness = derive_project_readiness(
        &input,
        ProjectReadinessContext {
            agent_sources: &agent_sources,
            aggregates: &skill_aggregates,
            findings: &visible_findings,
            conflicts: &conflicts,
            conflict_dispositions: &conflict_dispositions,
            sessions: &session_continuations,
            action_map: &actions,
        },
    )?;

    Ok(ProductProjection {
        readiness,
        skill_aggregates,
        session_continuations,
    })
}

pub fn skill_projection_evidence_id(instance_id: &str) -> String {
    format!("evidence:skill:{}", stable_digest(instance_id))
}

pub fn finding_projection_evidence_id(finding_id: &str) -> String {
    format!("evidence:finding:{}", stable_digest(finding_id))
}

pub fn conflict_projection_evidence_id(conflict_id: &str) -> String {
    format!("evidence:conflict:{}", stable_digest(conflict_id))
}

pub fn coverage_projection_evidence_id(agent: AgentId) -> String {
    format!("evidence:coverage:{}", agent.as_str())
}

pub fn session_projection_evidence_id(session_id: &str, source_revision: &str) -> String {
    format!(
        "evidence:session:{}",
        stable_digest(&format!("{session_id}|{source_revision}"))
    )
}

fn normalized_agent_sources(
    sources: Vec<AgentProjectionInput>,
    source_revision: &str,
) -> Result<BTreeMap<String, AgentProjectionInput>, ProductProjectionError> {
    let mut normalized = BTreeMap::new();
    for mut source in sources {
        if source.agent == AgentId::ToolGlobal {
            return Err(invalid("tool-global is not a product agent"));
        }
        source
            .coverage
            .validate()
            .map_err(|error| invalid(error.to_string()))?;
        validate_display(&source.evidence_summary, "agent evidence summary")?;
        source.action_ids = checked_action_ids(&source.action_ids, "agent projection")?
            .into_iter()
            .collect();
        if normalized
            .insert(source.agent.as_str().to_string(), source)
            .is_some()
        {
            return Err(invalid("duplicate agent projection input"));
        }
    }
    for agent in PRODUCT_AGENTS {
        normalized
            .entry(agent.as_str().to_string())
            .or_insert_with(|| AgentProjectionInput::not_inspected(agent, source_revision));
    }
    Ok(normalized)
}

fn derive_skill_aggregates(
    source_revision: &str,
    inputs: Vec<SkillProjectionInput>,
    findings: &[RuleFindingRecord],
    conflicts: &[ConflictGroupRecord],
    conflict_dispositions: &HashMap<String, ConflictDisposition>,
    actions: &HashMap<String, ActionDescriptor>,
) -> Result<Vec<SkillAggregateRecord>, ProductProjectionError> {
    let mut groups = BTreeMap::<SkillAggregateKey, Vec<SkillProjectionInput>>::new();
    let mut ids = HashSet::new();
    for mut input in inputs {
        if input.adapter_state == SkillState::Missing {
            continue;
        }
        validate_skill_input(&input)?;
        input.action_ids = checked_action_ids(&input.action_ids, "skill projection")?
            .into_iter()
            .collect();
        if !ids.insert(input.instance_id.clone()) {
            return Err(invalid("duplicate skill projection instance id"));
        }
        groups
            .entry(skill_aggregate_key(&input))
            .or_default()
            .push(input);
    }

    let mut aggregates = Vec::new();
    for (key, mut members) in groups {
        members.sort_by(|left, right| left.instance_id.cmp(&right.instance_id));
        let member_ids = members
            .iter()
            .map(|member| member.instance_id.as_str())
            .collect::<HashSet<_>>();
        let member_findings = findings
            .iter()
            .filter(|finding| {
                finding
                    .instance_id
                    .as_deref()
                    .is_some_and(|id| member_ids.contains(id))
            })
            .collect::<Vec<_>>();
        let member_conflicts = conflicts
            .iter()
            .filter(|conflict| {
                conflict
                    .instance_ids
                    .iter()
                    .any(|id| member_ids.contains(id.as_str()))
            })
            .collect::<Vec<_>>();

        let mut evidence = Vec::new();
        for member in &members {
            evidence.push(EvidenceRef {
                id: skill_projection_evidence_id(&member.instance_id),
                kind: EvidenceKind::SkillInstance,
                source_revision: source_revision.to_string(),
                summary: member.evidence_summary.clone(),
                agent: member.agent,
                target_id: Some(member.instance_id.clone()),
            });
        }
        for finding in &member_findings {
            evidence.push(EvidenceRef {
                id: finding_projection_evidence_id(&finding.id),
                kind: EvidenceKind::Finding,
                source_revision: source_revision.to_string(),
                summary: safe_finding_summary(finding),
                agent: finding
                    .instance_id
                    .as_deref()
                    .and_then(|id| members.iter().find(|member| member.instance_id == id))
                    .and_then(|member| member.agent),
                target_id: finding.instance_id.clone(),
            });
        }
        for conflict in &member_conflicts {
            evidence.push(EvidenceRef {
                id: conflict_projection_evidence_id(&conflict.id),
                kind: EvidenceKind::Conflict,
                source_revision: source_revision.to_string(),
                summary: format!(
                    "Runtime conflict {} affects current skill evidence",
                    conflict.reason
                ),
                agent: conflict_agent(conflict, &members),
                target_id: Some(conflict.id.clone()),
            });
        }
        dedupe_evidence(&mut evidence);

        let action_ids = members
            .iter()
            .flat_map(|member| member.action_ids.iter().cloned())
            .collect::<BTreeSet<_>>();
        validate_skill_actions(&members, &member_findings, &member_conflicts, actions)?;
        let aggregate_actions = select_actions(&action_ids, actions, &evidence, "skill aggregate")?;

        let mut instance_effectiveness = Vec::new();
        for member in &members {
            let disposition = conflict_dispositions.get(&member.instance_id).copied();
            let state = derive_effectiveness(member, disposition);
            let linked_actions = member
                .action_ids
                .iter()
                .filter(|id| action_ids.contains(*id))
                .cloned()
                .collect::<Vec<_>>();
            instance_effectiveness.push(SkillInstanceEffectivenessRecord {
                instance_id: member.instance_id.clone(),
                agent: member.agent,
                scope: member.scope,
                source_identity: member.source_identity.clone(),
                runtime_identity: member.runtime_identity.clone(),
                installed: member.installed && member.coverage.is_complete(),
                linked: member.linked,
                enabled: member.enabled,
                precedence_proven: member.precedence_proven
                    && disposition != Some(ConflictDisposition::Unresolved),
                state,
                coverage: member.coverage.clone(),
                evidence_refs: vec![skill_projection_evidence_id(&member.instance_id)],
                action_ids: linked_actions,
            });
        }

        let effectiveness_counts = effectiveness_counts(&instance_effectiveness);
        let primary_effectiveness = primary_effectiveness(&instance_effectiveness);
        let installed_instance_count = instance_effectiveness
            .iter()
            .filter(|record| record.installed)
            .count();
        let enabled_instance_count = instance_effectiveness
            .iter()
            .filter(|record| record.installed && record.enabled)
            .count();
        let effective_instance_count = instance_effectiveness
            .iter()
            .filter(|record| record.state == SkillEffectivenessState::Effective)
            .count();
        let coverage = merge_coverages(
            members
                .iter()
                .map(|member| member.coverage.clone())
                .collect(),
        )?;
        let first = &members[0];
        let aggregate_id = format!(
            "aggregate:{}",
            stable_digest(&format!(
                "{}|{}|{}|{}|{}|{}|{}",
                key.definition_id,
                key.definition_fingerprint,
                key.source_identity,
                key.source_kind,
                key.package_identity,
                key.scope,
                key.runtime_identity
            ))
        );
        let mut agents = members
            .iter()
            .filter_map(|member| member.agent)
            .collect::<Vec<_>>();
        agents.sort_by_key(|agent| product_agent_rank(*agent));
        agents.dedup();
        let mut scopes = members
            .iter()
            .map(|member| member.scope)
            .collect::<Vec<_>>();
        scopes.sort_by_key(|scope| scope_rank(*scope));
        scopes.dedup();
        let mut aggregate = SkillAggregateRecord {
            id: aggregate_id,
            definition_id: first.definition_id.clone(),
            definition_fingerprint: first.definition_fingerprint.clone(),
            canonical_name: first.canonical_name.clone(),
            display_name: first.display_name.clone(),
            description: first.description.clone(),
            publisher: first.publisher.clone(),
            package_name: first.package_name.clone(),
            package_version: first.package_version.clone(),
            source_kind: first.source_kind.clone(),
            source_identity: first.source_identity.clone(),
            runtime_identity: first.runtime_identity.clone(),
            read_only_reason: first.read_only_reason.clone(),
            instance_ids: members
                .iter()
                .map(|member| member.instance_id.clone())
                .collect(),
            agents,
            scopes,
            installed_instance_count,
            enabled_instance_count,
            effective_instance_count,
            primary_effectiveness,
            effectiveness_counts,
            instance_effectiveness,
            finding_count: member_findings.len(),
            conflict_count: member_conflicts.len(),
            source_revision: source_revision.to_string(),
            coverage,
            evidence,
            actions: aggregate_actions,
        };
        normalize_aggregate_counts(&mut aggregate);
        aggregate
            .validate()
            .map_err(|error| invalid(format!("invalid skill aggregate: {error}")))?;
        aggregates.push(aggregate);
    }
    aggregates.sort_by(|left, right| {
        effectiveness_rank(left.primary_effectiveness)
            .cmp(&effectiveness_rank(right.primary_effectiveness))
            .then_with(|| left.canonical_name.cmp(&right.canonical_name))
            .then_with(|| left.source_identity.cmp(&right.source_identity))
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(aggregates)
}

struct ProjectReadinessContext<'a> {
    agent_sources: &'a BTreeMap<String, AgentProjectionInput>,
    aggregates: &'a [SkillAggregateRecord],
    findings: &'a [RuleFindingRecord],
    conflicts: &'a [ConflictGroupRecord],
    conflict_dispositions: &'a HashMap<String, ConflictDisposition>,
    sessions: &'a [SessionContinuationRecord],
    action_map: &'a HashMap<String, ActionDescriptor>,
}

fn derive_project_readiness(
    input: &ProductProjectionInput,
    context: ProjectReadinessContext<'_>,
) -> Result<ProjectReadinessRecord, ProductProjectionError> {
    let ProjectReadinessContext {
        agent_sources,
        aggregates,
        findings,
        conflicts,
        conflict_dispositions,
        sessions,
        action_map,
    } = context;
    let mut evidence = Vec::new();
    for source in agent_sources.values() {
        evidence.push(EvidenceRef {
            id: coverage_projection_evidence_id(source.agent),
            kind: EvidenceKind::ScanCoverage,
            source_revision: input.source_revision.clone(),
            summary: source.evidence_summary.clone(),
            agent: Some(source.agent),
            target_id: Some(source.agent.as_str().to_string()),
        });
    }
    for aggregate in aggregates {
        evidence.extend(aggregate.evidence.clone());
    }
    dedupe_evidence(&mut evidence);

    let mut attention = Vec::new();
    for source in agent_sources.values() {
        if !source.coverage.is_complete() {
            let kind = coverage_attention_kind(&source.coverage);
            attention.push(AttentionItem {
                id: format!("attention:coverage:{}", source.agent.as_str()),
                kind,
                severity: AttentionSeverity::Error,
                title: format!(
                    "{} evidence is incomplete",
                    agent_display_name(source.agent)
                ),
                summary: coverage_attention_summary(source.agent, &source.coverage),
                target: ActionTargetRef {
                    kind: ActionTargetKind::Agent,
                    id: source.agent.as_str().to_string(),
                    agent: Some(source.agent),
                    scope: None,
                },
                agent: Some(source.agent),
                evidence_refs: vec![coverage_projection_evidence_id(source.agent)],
                action_ids: source.action_ids.clone(),
                no_safe_action_reason: source
                    .action_ids
                    .is_empty()
                    .then_some(NoSafeActionReason::IncompleteEvidence),
            });
        }
    }
    for aggregate in aggregates {
        for record in &aggregate.instance_effectiveness {
            if matches!(
                record.state,
                SkillEffectivenessState::Broken | SkillEffectivenessState::Unavailable
            ) {
                let kind = if record.state == SkillEffectivenessState::Broken {
                    AttentionKind::BrokenSkill
                } else {
                    AttentionKind::SkillUnavailable
                };
                attention.push(AttentionItem {
                    id: format!(
                        "attention:{}:{}",
                        if record.state == SkillEffectivenessState::Broken {
                            "broken"
                        } else {
                            "unavailable"
                        },
                        stable_digest(&record.instance_id)
                    ),
                    kind,
                    severity: AttentionSeverity::Error,
                    title: format!(
                        "{} is {}",
                        aggregate.display_name,
                        if record.state == SkillEffectivenessState::Broken {
                            "broken"
                        } else {
                            "unavailable"
                        }
                    ),
                    summary: format!(
                        "{} cannot be asserted effective from current deterministic evidence",
                        aggregate.display_name
                    ),
                    target: ActionTargetRef {
                        kind: ActionTargetKind::Skill,
                        id: record.instance_id.clone(),
                        agent: record.agent,
                        scope: Some(record.scope),
                    },
                    agent: record.agent,
                    evidence_refs: record.evidence_refs.clone(),
                    action_ids: record.action_ids.clone(),
                    no_safe_action_reason: record.action_ids.is_empty().then_some(
                        if record.state == SkillEffectivenessState::Unavailable {
                            NoSafeActionReason::IncompleteEvidence
                        } else {
                            NoSafeActionReason::NoGuardedWritePath
                        },
                    ),
                });
            }
        }
    }
    let instance_agents = aggregate_instance_agents(aggregates);
    let instance_actions = aggregate_instance_actions(aggregates);
    for finding in findings {
        let Some(instance_id) = finding.instance_id.as_deref() else {
            continue;
        };
        let Some(agent) = instance_agents.get(instance_id).copied().flatten() else {
            continue;
        };
        let action_ids = instance_actions
            .get(instance_id)
            .cloned()
            .unwrap_or_default();
        attention.push(AttentionItem {
            id: format!("attention:finding:{}", stable_digest(&finding.id)),
            kind: AttentionKind::Finding,
            severity: finding_attention_severity(&finding.effective_severity),
            title: format!("Skill finding: {}", finding.rule_id),
            summary: safe_finding_summary(finding),
            target: ActionTargetRef {
                kind: ActionTargetKind::Skill,
                id: instance_id.to_string(),
                agent: Some(agent),
                scope: None,
            },
            agent: Some(agent),
            evidence_refs: vec![finding_projection_evidence_id(&finding.id)],
            no_safe_action_reason: action_ids
                .is_empty()
                .then_some(NoSafeActionReason::ManualReviewRequired),
            action_ids,
        });
    }
    for conflict in conflicts {
        let agent = conflict
            .instance_ids
            .iter()
            .filter_map(|id| instance_agents.get(id).copied().flatten())
            .next();
        let mut action_ids = conflict
            .instance_ids
            .iter()
            .flat_map(|id| instance_actions.get(id).cloned().unwrap_or_default())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        action_ids.sort();
        attention.push(AttentionItem {
            id: format!("attention:conflict:{}", stable_digest(&conflict.id)),
            kind: AttentionKind::Conflict,
            severity: if conflict_resolution_is_proven(conflict, conflict_dispositions) {
                AttentionSeverity::Warning
            } else {
                AttentionSeverity::Error
            },
            title: "Runtime skill conflict".to_string(),
            summary: format!(
                "A {} conflict affects {} current skill instances",
                conflict.reason,
                conflict.instance_ids.len()
            ),
            target: ActionTargetRef {
                kind: ActionTargetKind::Skill,
                id: conflict.definition_id.clone(),
                agent,
                scope: None,
            },
            agent,
            evidence_refs: vec![conflict_projection_evidence_id(&conflict.id)],
            no_safe_action_reason: action_ids
                .is_empty()
                .then_some(NoSafeActionReason::ManualReviewRequired),
            action_ids,
        });
    }
    attention.sort_by(compare_attention);
    attention.dedup_by(|left, right| left.id == right.id);

    let blocking_reasons = attention
        .iter()
        .filter(|item| item.severity.blocks_health())
        .map(blocker_from_attention)
        .collect::<Vec<_>>();
    let project_action_ids = agent_sources
        .values()
        .flat_map(|source| source.action_ids.iter().cloned())
        .chain(
            aggregates
                .iter()
                .flat_map(|aggregate| aggregate.actions.iter().map(|action| action.id.clone())),
        )
        .chain(
            attention
                .iter()
                .flat_map(|item| item.action_ids.iter().cloned()),
        )
        .collect::<BTreeSet<_>>();
    let project_actions = select_actions(
        &project_action_ids,
        action_map,
        &evidence,
        "project readiness",
    )?;

    let mut agents = Vec::new();
    for source in agent_sources.values() {
        let agent = source.agent;
        let agent_attention = attention
            .iter()
            .filter(|item| item.agent == Some(agent))
            .collect::<Vec<_>>();
        let blockers = blocking_reasons
            .iter()
            .filter(|blocker| blocker.agent == Some(agent))
            .cloned()
            .collect::<Vec<_>>();
        let health = if !source.coverage.is_complete() || !blockers.is_empty() {
            EnvironmentHealthState::Blocked
        } else if !agent_attention.is_empty() {
            EnvironmentHealthState::Review
        } else {
            EnvironmentHealthState::Healthy
        };
        let effective_skill_count = aggregates
            .iter()
            .flat_map(|aggregate| aggregate.instance_effectiveness.iter())
            .filter(|record| {
                record.agent == Some(agent) && record.state == SkillEffectivenessState::Effective
            })
            .count();
        let issue_count = agent_attention
            .iter()
            .filter(|item| item.kind != AttentionKind::Conflict)
            .count();
        let conflict_count = conflicts
            .iter()
            .filter(|conflict| {
                conflict.instance_ids.iter().any(|id| {
                    instance_agents
                        .get(id)
                        .is_some_and(|record_agent| *record_agent == Some(agent))
                })
            })
            .count();
        let evidence_refs = evidence
            .iter()
            .filter(|reference| reference.agent == Some(agent))
            .map(|reference| reference.id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        let action_ids = agent_attention
            .iter()
            .flat_map(|item| item.action_ids.iter().cloned())
            .chain(source.action_ids.iter().cloned())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect::<Vec<_>>();
        agents.push(AgentReadinessRecord {
            agent,
            health,
            coverage: source.coverage.clone(),
            effective_skill_count,
            issue_count,
            conflict_count,
            evidence_refs,
            action_ids,
            blocking_reasons: blockers,
            attention_item_ids: agent_attention.iter().map(|item| item.id.clone()).collect(),
        });
    }
    agents.sort_by_key(|record| product_agent_rank(record.agent));

    let coverage = merge_coverages(
        agents
            .iter()
            .map(|record| record.coverage.clone())
            .collect(),
    )?;
    let health = if !coverage.is_complete() || !blocking_reasons.is_empty() {
        EnvironmentHealthState::Blocked
    } else if !attention.is_empty() {
        EnvironmentHealthState::Review
    } else {
        EnvironmentHealthState::Healthy
    };
    let mut readiness = ProjectReadinessRecord {
        project_id: input.project_id.clone(),
        project_display_name: input.project_display_name.clone(),
        source_revision: input.source_revision.clone(),
        health,
        coverage,
        agents,
        blocking_reasons,
        attention,
        evidence,
        actions: project_actions,
        recent_sessions: sessions.to_vec(),
    };
    readiness.recent_sessions.sort_by(|left, right| {
        right
            .modified_at
            .cmp(&left.modified_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    readiness
        .validate()
        .map_err(|error| invalid(format!("invalid project readiness: {error}")))?;
    Ok(readiness)
}

fn project_session_continuations(
    project_id: &str,
    inputs: Vec<SessionProjectionInput>,
    actions: &HashMap<String, ActionDescriptor>,
) -> Result<Vec<SessionContinuationRecord>, ProductProjectionError> {
    let mut ids = HashSet::new();
    let mut result = Vec::new();
    for input in inputs {
        if input.project_id.as_deref() != Some(project_id) {
            continue;
        }
        if !ids.insert(input.id.clone()) {
            return Err(invalid("duplicate session projection id"));
        }
        validate_identity(&input.id, "session id")?;
        validate_display(&input.title, "session title")?;
        validate_identity(&input.source_kind, "session source kind")?;
        validate_identity(&input.source_revision, "session source revision")?;
        validate_display(&input.evidence_summary, "session evidence summary")?;
        input
            .coverage
            .validate()
            .map_err(|error| invalid(error.to_string()))?;
        input
            .resume
            .validate()
            .map_err(|error| invalid(error.to_string()))?;

        let resume = if input.coverage.is_complete() {
            input.resume
        } else {
            ResumeCapability::unsupported(ResumeUnsupportedReason::SourceIncomplete)
        };
        let evidence = vec![EvidenceRef {
            id: session_projection_evidence_id(&input.id, &input.source_revision),
            kind: EvidenceKind::Session,
            source_revision: input.source_revision.clone(),
            summary: input.evidence_summary,
            agent: Some(input.agent),
            target_id: Some(input.id.clone()),
        }];
        let action_ids = checked_action_ids(&input.action_ids, "session projection")?;
        validate_session_actions(&input.id, input.agent, &action_ids, actions, &evidence)?;
        let selected_actions =
            select_actions(&action_ids, actions, &evidence, "session continuation")?;
        let record = SessionContinuationRecord {
            id: input.id,
            agent: input.agent,
            project_id: input.project_id,
            title: input.title,
            intent: input.intent,
            started_at: input.started_at,
            ended_at: input.ended_at,
            modified_at: input.modified_at,
            source_kind: input.source_kind,
            source_revision: input.source_revision,
            snapshot_revision: input.snapshot_revision,
            coverage: input.coverage,
            resume,
            evidence,
            actions: selected_actions,
        };
        record
            .validate()
            .map_err(|error| invalid(format!("invalid session continuation: {error}")))?;
        result.push(record);
    }
    result.sort_by(|left, right| {
        right
            .modified_at
            .cmp(&left.modified_at)
            .then_with(|| left.id.cmp(&right.id))
    });
    Ok(result)
}

fn validate_skill_input(input: &SkillProjectionInput) -> Result<(), ProductProjectionError> {
    validate_identity(&input.instance_id, "skill instance id")?;
    validate_identity(&input.definition_id, "skill definition id")?;
    validate_display(&input.canonical_name, "skill canonical name")?;
    validate_display(&input.display_name, "skill display name")?;
    validate_identity(&input.source_kind, "skill source kind")?;
    validate_identity(&input.source_identity, "skill source identity")?;
    validate_identity(&input.runtime_identity, "skill runtime identity")?;
    validate_display(&input.evidence_summary, "skill evidence summary")?;
    input
        .coverage
        .validate()
        .map_err(|error| invalid(error.to_string()))
}

fn skill_aggregate_key(input: &SkillProjectionInput) -> SkillAggregateKey {
    SkillAggregateKey {
        definition_id: input.definition_id.clone(),
        definition_fingerprint: input.definition_fingerprint.clone().unwrap_or_default(),
        source_identity: input.source_identity.clone(),
        source_kind: input.source_kind.clone(),
        package_identity: format!(
            "{}|{}|{}",
            input.publisher.as_deref().unwrap_or_default(),
            input.package_name.as_deref().unwrap_or_default(),
            input.package_version.as_deref().unwrap_or_default()
        ),
        scope: input.scope.as_str().to_string(),
        runtime_identity: input.runtime_identity.clone(),
    }
}

fn derive_effectiveness(
    input: &SkillProjectionInput,
    conflict: Option<ConflictDisposition>,
) -> SkillEffectivenessState {
    if !input.coverage.is_complete() || !input.installed {
        return SkillEffectivenessState::Unavailable;
    }
    if input.adapter_state == SkillState::Broken {
        return SkillEffectivenessState::Broken;
    }
    if !input.linked {
        return SkillEffectivenessState::InstalledUnlinked;
    }
    if !input.enabled || input.adapter_state == SkillState::Disabled {
        return SkillEffectivenessState::Disabled;
    }
    if input.adapter_state == SkillState::Missing {
        return SkillEffectivenessState::Unavailable;
    }
    if !input.precedence_proven || conflict == Some(ConflictDisposition::Unresolved) {
        return SkillEffectivenessState::Unavailable;
    }
    if input.adapter_state == SkillState::Shadowed
        || conflict == Some(ConflictDisposition::Shadowed)
    {
        return SkillEffectivenessState::Shadowed;
    }
    SkillEffectivenessState::Effective
}

fn conflict_dispositions(
    conflicts: &[ConflictGroupRecord],
    skills: &[SkillProjectionInput],
) -> Result<HashMap<String, ConflictDisposition>, ProductProjectionError> {
    let current = skills
        .iter()
        .filter(|skill| skill.adapter_state != SkillState::Missing)
        .map(|skill| (skill.instance_id.as_str(), skill))
        .collect::<HashMap<_, _>>();
    let mut dispositions = HashMap::new();
    let mut conflict_ids = HashSet::new();
    for conflict in conflicts {
        validate_identity(&conflict.id, "conflict id")?;
        validate_identity(&conflict.definition_id, "conflict definition id")?;
        validate_display(&conflict.reason, "conflict reason")?;
        if !conflict_ids.insert(conflict.id.as_str()) {
            return Err(invalid("duplicate conflict projection id"));
        }
        if conflict.instance_ids.len() < 2 {
            return Err(invalid(
                "conflict projection requires at least two current instances",
            ));
        }
        let instance_ids = conflict
            .instance_ids
            .iter()
            .map(String::as_str)
            .collect::<HashSet<_>>();
        if instance_ids.len() != conflict.instance_ids.len() {
            return Err(invalid("conflict projection contains duplicate instances"));
        }
        let members = conflict
            .instance_ids
            .iter()
            .map(|instance_id| {
                current.get(instance_id.as_str()).copied().ok_or_else(|| {
                    invalid(format!(
                        "conflict {} references a non-current instance {instance_id}",
                        conflict.id
                    ))
                })
            })
            .collect::<Result<Vec<_>, _>>()?;
        if members
            .iter()
            .any(|member| member.definition_id != conflict.definition_id)
        {
            return Err(invalid(format!(
                "conflict {} definition does not match its current instances",
                conflict.id
            )));
        }
        let expected_agent = members[0].agent;
        if members.iter().any(|member| member.agent != expected_agent) {
            return Err(invalid(format!(
                "conflict {} crosses agent runtime ownership",
                conflict.id
            )));
        }
        let expected_runtime_identity = &members[0].runtime_identity;
        if members
            .iter()
            .any(|member| member.runtime_identity.as_str() != expected_runtime_identity.as_str())
        {
            return Err(invalid(format!(
                "conflict {} crosses runtime identity",
                conflict.id
            )));
        }
        let winner_is_proven = match conflict.winner_id.as_deref() {
            Some(winner_id) => {
                if !instance_ids.contains(winner_id) {
                    return Err(invalid(format!(
                        "conflict {} winner is not a conflict member",
                        conflict.id
                    )));
                }
                let winner = current.get(winner_id).copied().ok_or_else(|| {
                    invalid(format!(
                        "conflict {} winner is not a current instance",
                        conflict.id
                    ))
                })?;
                is_proven_conflict_winner(winner)
            }
            None => false,
        };
        for instance_id in &conflict.instance_ids {
            let disposition = match conflict.winner_id.as_deref().filter(|_| winner_is_proven) {
                Some(winner) if winner == instance_id => ConflictDisposition::Winner,
                Some(_) => ConflictDisposition::Shadowed,
                None => ConflictDisposition::Unresolved,
            };
            dispositions
                .entry(instance_id.clone())
                .and_modify(|current| {
                    if disposition_rank(disposition) > disposition_rank(*current) {
                        *current = disposition;
                    }
                })
                .or_insert(disposition);
        }
    }
    Ok(dispositions)
}

fn is_proven_conflict_winner(skill: &SkillProjectionInput) -> bool {
    skill.installed
        && skill.linked
        && skill.enabled
        && skill.precedence_proven
        && skill.coverage.is_complete()
        && skill.adapter_state == SkillState::Loaded
}

fn conflict_resolution_is_proven(
    conflict: &ConflictGroupRecord,
    dispositions: &HashMap<String, ConflictDisposition>,
) -> bool {
    let Some(winner_id) = conflict.winner_id.as_deref() else {
        return false;
    };
    dispositions.get(winner_id) == Some(&ConflictDisposition::Winner)
        && conflict.instance_ids.iter().all(|instance_id| {
            matches!(
                dispositions.get(instance_id),
                Some(ConflictDisposition::Winner | ConflictDisposition::Shadowed)
            )
        })
}

fn disposition_rank(disposition: ConflictDisposition) -> u8 {
    match disposition {
        ConflictDisposition::Winner => 0,
        ConflictDisposition::Shadowed => 1,
        ConflictDisposition::Unresolved => 2,
    }
}

fn effectiveness_counts(
    records: &[SkillInstanceEffectivenessRecord],
) -> Vec<SkillEffectivenessCount> {
    [
        SkillEffectivenessState::Effective,
        SkillEffectivenessState::Disabled,
        SkillEffectivenessState::Shadowed,
        SkillEffectivenessState::InstalledUnlinked,
        SkillEffectivenessState::Broken,
        SkillEffectivenessState::Unavailable,
    ]
    .into_iter()
    .filter_map(|state| {
        let count = records
            .iter()
            .filter(|record| record.state == state)
            .count();
        (count > 0).then_some(SkillEffectivenessCount { state, count })
    })
    .collect()
}

fn primary_effectiveness(records: &[SkillInstanceEffectivenessRecord]) -> SkillEffectivenessState {
    records
        .iter()
        .map(|record| record.state)
        .min_by_key(|state| effectiveness_rank(*state))
        .unwrap_or(SkillEffectivenessState::Unavailable)
}

fn effectiveness_rank(state: SkillEffectivenessState) -> u8 {
    match state {
        SkillEffectivenessState::Broken => 0,
        SkillEffectivenessState::Unavailable => 1,
        SkillEffectivenessState::Disabled => 2,
        SkillEffectivenessState::Shadowed => 3,
        SkillEffectivenessState::InstalledUnlinked => 4,
        SkillEffectivenessState::Effective => 5,
        _ => 6,
    }
}

fn normalize_aggregate_counts(aggregate: &mut SkillAggregateRecord) {
    aggregate.effectiveness_counts = effectiveness_counts(&aggregate.instance_effectiveness);
}

fn merge_coverages(
    coverages: Vec<SourceCoverage>,
) -> Result<SourceCoverage, ProductProjectionError> {
    SourceCoverage::merge(&coverages).map_err(|error| invalid(error.to_string()))
}

fn checked_action_ids(
    action_ids: &[String],
    owner: &str,
) -> Result<BTreeSet<String>, ProductProjectionError> {
    let unique = action_ids.iter().cloned().collect::<BTreeSet<_>>();
    if unique.len() != action_ids.len() {
        return Err(invalid(format!("{owner} contains a duplicate action id")));
    }
    Ok(unique)
}

fn validate_agent_source_actions(
    sources: &BTreeMap<String, AgentProjectionInput>,
    actions: &HashMap<String, ActionDescriptor>,
) -> Result<(), ProductProjectionError> {
    for source in sources.values() {
        let action_ids = checked_action_ids(&source.action_ids, "agent projection")?;
        let evidence_ids = HashSet::from([coverage_projection_evidence_id(source.agent)]);
        for action_id in action_ids {
            let action = actions.get(&action_id).ok_or_else(|| {
                invalid(format!(
                    "agent projection references unknown action {action_id}"
                ))
            })?;
            if action.target.kind != ActionTargetKind::Agent
                || action.target.id != source.agent.as_str()
                || action.target.agent != Some(source.agent)
                || action.target.scope.is_some()
            {
                return Err(invalid(format!(
                    "action {action_id} does not target its owning agent"
                )));
            }
            validate_owned_action_evidence(action, &evidence_ids, "owning agent")?;
        }
    }
    Ok(())
}

fn validate_skill_actions(
    members: &[SkillProjectionInput],
    findings: &[&RuleFindingRecord],
    conflicts: &[&ConflictGroupRecord],
    actions: &HashMap<String, ActionDescriptor>,
) -> Result<(), ProductProjectionError> {
    for member in members {
        let action_ids = checked_action_ids(&member.action_ids, "skill projection")?;
        let mut evidence_ids = HashSet::from([skill_projection_evidence_id(&member.instance_id)]);
        evidence_ids.extend(
            findings
                .iter()
                .filter(|finding| finding.instance_id.as_deref() == Some(&member.instance_id))
                .map(|finding| finding_projection_evidence_id(&finding.id)),
        );
        evidence_ids.extend(
            conflicts
                .iter()
                .filter(|conflict| conflict.instance_ids.contains(&member.instance_id))
                .map(|conflict| conflict_projection_evidence_id(&conflict.id)),
        );
        for action_id in action_ids {
            let action = actions.get(&action_id).ok_or_else(|| {
                invalid(format!(
                    "skill projection references unknown action {action_id}"
                ))
            })?;
            if action.target.kind != ActionTargetKind::Skill
                || action.target.id != member.instance_id
                || action.target.agent != member.agent
                || action.target.scope != Some(member.scope)
            {
                return Err(invalid(format!(
                    "action {action_id} does not target its owning skill"
                )));
            }
            validate_owned_action_evidence(action, &evidence_ids, "owning skill")?;
        }
    }
    Ok(())
}

fn validate_session_actions(
    session_id: &str,
    agent: AgentId,
    action_ids: &BTreeSet<String>,
    actions: &HashMap<String, ActionDescriptor>,
    evidence: &[EvidenceRef],
) -> Result<(), ProductProjectionError> {
    let evidence_ids = evidence
        .iter()
        .map(|reference| reference.id.clone())
        .collect::<HashSet<_>>();
    for action_id in action_ids {
        let action = actions.get(action_id).ok_or_else(|| {
            invalid(format!(
                "session projection references unknown action {action_id}"
            ))
        })?;
        if action.target.kind != ActionTargetKind::Session
            || action.target.id != session_id
            || action.target.agent != Some(agent)
            || !matches!(action.target.scope, None | Some(Scope::AgentProject))
        {
            return Err(invalid(format!(
                "action {action_id} does not target its owning session"
            )));
        }
        validate_owned_action_evidence(action, &evidence_ids, "owning session")?;
    }
    Ok(())
}

fn validate_owned_action_evidence(
    action: &ActionDescriptor,
    evidence_ids: &HashSet<String>,
    owner: &str,
) -> Result<(), ProductProjectionError> {
    if action
        .evidence_refs
        .iter()
        .any(|evidence_id| !evidence_ids.contains(evidence_id))
    {
        return Err(invalid(format!(
            "action {} references evidence outside its {owner}",
            action.id
        )));
    }
    Ok(())
}

fn action_map(
    actions: &[ActionDescriptor],
    source_revision: &str,
) -> Result<HashMap<String, ActionDescriptor>, ProductProjectionError> {
    let mut result = HashMap::new();
    for action in actions {
        action
            .validate()
            .map_err(|error| invalid(format!("invalid action capability: {error}")))?;
        validate_action_method_ownership(
            action.kind,
            &action.preview_method,
            action.apply_method.as_deref(),
        )
        .map_err(|error| invalid(format!("invalid action capability: {error}")))?;
        validate_action_intent(action.kind, action.intent)
            .map_err(|error| invalid(format!("invalid action capability: {error}")))?;
        let expected_id = deterministic_action_id(
            action.kind,
            action.intent,
            &action.target,
            action.project_id.as_deref(),
        )
        .map_err(|error| invalid(format!("invalid action capability: {error}")))?;
        if action.id != expected_id {
            return Err(invalid(
                "action capability id was not minted by the registry",
            ));
        }
        if action.source_revision != source_revision {
            return Err(invalid("action capability revision does not match project"));
        }
        let normalized = canonical_action(action.clone());
        if result.insert(action.id.clone(), normalized).is_some() {
            return Err(invalid("duplicate action capability id"));
        }
    }
    Ok(result)
}

fn canonical_action(mut action: ActionDescriptor) -> ActionDescriptor {
    action
        .impacts
        .sort_by_key(|impact| action_impact_rank(*impact));
    action.readback.sort();
    action.evidence_refs.sort();
    action
}

fn action_impact_rank(impact: ActionImpact) -> u8 {
    match impact {
        ActionImpact::ReadOnly => 0,
        ActionImpact::AppLocalData => 1,
        ActionImpact::AgentConfig => 2,
        ActionImpact::SkillFiles => 3,
        ActionImpact::ExternalManager => 4,
        _ => 5,
    }
}

fn select_actions(
    ids: &BTreeSet<String>,
    actions: &HashMap<String, ActionDescriptor>,
    evidence: &[EvidenceRef],
    owner: &str,
) -> Result<Vec<ActionDescriptor>, ProductProjectionError> {
    let evidence_ids = evidence
        .iter()
        .map(|reference| reference.id.as_str())
        .collect::<HashSet<_>>();
    let mut selected = Vec::new();
    for id in ids {
        let Some(action) = actions.get(id) else {
            return Err(invalid(format!("{owner} references unknown action {id}")));
        };
        if action
            .evidence_refs
            .iter()
            .any(|evidence_id| !evidence_ids.contains(evidence_id.as_str()))
        {
            return Err(invalid(format!(
                "{owner} action {id} references evidence outside the projection"
            )));
        }
        selected.push(action.clone());
    }
    Ok(selected)
}

fn conflict_agent(
    conflict: &ConflictGroupRecord,
    members: &[SkillProjectionInput],
) -> Option<AgentId> {
    conflict
        .instance_ids
        .iter()
        .filter_map(|id| {
            members
                .iter()
                .find(|member| member.instance_id == *id)
                .and_then(|member| member.agent)
        })
        .next()
}

fn aggregate_instance_agents(
    aggregates: &[SkillAggregateRecord],
) -> HashMap<String, Option<AgentId>> {
    aggregates
        .iter()
        .flat_map(|aggregate| {
            aggregate
                .instance_effectiveness
                .iter()
                .map(|record| (record.instance_id.clone(), record.agent))
        })
        .collect()
}

fn aggregate_instance_actions(aggregates: &[SkillAggregateRecord]) -> HashMap<String, Vec<String>> {
    aggregates
        .iter()
        .flat_map(|aggregate| {
            aggregate
                .instance_effectiveness
                .iter()
                .map(|record| (record.instance_id.clone(), record.action_ids.clone()))
        })
        .collect()
}

fn coverage_attention_kind(coverage: &SourceCoverage) -> AttentionKind {
    match coverage.incomplete_reason {
        Some(ListIncompleteReason::StaleSource) => AttentionKind::StaleEvidence,
        Some(ListIncompleteReason::UnreadableSource) => AttentionKind::SourceUnavailable,
        _ => AttentionKind::IncompleteEvidence,
    }
}

fn coverage_attention_summary(agent: AgentId, coverage: &SourceCoverage) -> String {
    format!(
        "{} inspection is not complete ({})",
        agent_display_name(agent),
        coverage_reason_label(coverage.incomplete_reason)
    )
}

fn coverage_reason_label(reason: Option<ListIncompleteReason>) -> &'static str {
    match reason {
        Some(ListIncompleteReason::SafetyBudget) => "safety budget",
        Some(ListIncompleteReason::SourceChanged) => "source changed",
        Some(ListIncompleteReason::SourceLimited) => "source limited",
        Some(ListIncompleteReason::UnreadableSource) => "source unavailable",
        Some(ListIncompleteReason::PageFailed) => "page failed",
        Some(ListIncompleteReason::UnsupportedProtocol) => "unsupported protocol",
        Some(ListIncompleteReason::StaleSource) => "stale source",
        Some(ListIncompleteReason::NotInspected) => "not inspected",
        None => "unknown reason",
    }
}

fn finding_attention_severity(severity: &str) -> AttentionSeverity {
    match severity.trim().to_ascii_lowercase().as_str() {
        "critical" => AttentionSeverity::Critical,
        "error" => AttentionSeverity::Error,
        "warning" | "warn" => AttentionSeverity::Warning,
        _ => AttentionSeverity::Information,
    }
}

fn safe_finding_summary(finding: &RuleFindingRecord) -> String {
    let candidate = EvidenceRef {
        id: "validation".to_string(),
        kind: EvidenceKind::Finding,
        source_revision: "validation".to_string(),
        summary: finding.message.clone(),
        agent: None,
        target_id: None,
    };
    if candidate.validate().is_ok() {
        finding.message.clone()
    } else {
        format!(
            "Active {} finding {}",
            finding.effective_severity, finding.rule_id
        )
    }
}

fn blocker_from_attention(item: &AttentionItem) -> ReadinessBlocker {
    ReadinessBlocker {
        id: format!("blocker:{}", stable_digest(&item.id)),
        kind: item.kind,
        summary: item.summary.clone(),
        agent: item.agent,
        evidence_refs: item.evidence_refs.clone(),
        action_ids: item.action_ids.clone(),
    }
}

fn compare_attention(left: &AttentionItem, right: &AttentionItem) -> Ordering {
    attention_rank(left.severity)
        .cmp(&attention_rank(right.severity))
        .then_with(|| attention_kind_rank(left.kind).cmp(&attention_kind_rank(right.kind)))
        .then_with(|| left.title.cmp(&right.title))
        .then_with(|| left.id.cmp(&right.id))
}

fn attention_rank(severity: AttentionSeverity) -> u8 {
    match severity {
        AttentionSeverity::Critical => 0,
        AttentionSeverity::Error => 1,
        AttentionSeverity::Warning => 2,
        AttentionSeverity::Information => 3,
        _ => 4,
    }
}

fn attention_kind_rank(kind: AttentionKind) -> u8 {
    match kind {
        AttentionKind::IncompleteEvidence => 0,
        AttentionKind::StaleEvidence => 1,
        AttentionKind::SourceUnavailable => 2,
        AttentionKind::BrokenSkill => 3,
        AttentionKind::SkillUnavailable => 4,
        AttentionKind::Conflict => 5,
        AttentionKind::Finding => 6,
        _ => 7,
    }
}

fn product_agent_rank(agent: AgentId) -> usize {
    PRODUCT_AGENTS
        .iter()
        .position(|candidate| *candidate == agent)
        .unwrap_or(usize::MAX)
}

fn scope_rank(scope: Scope) -> u8 {
    match scope {
        Scope::AgentProject => 0,
        Scope::AgentGlobal => 1,
        Scope::ToolGlobal => 2,
        _ => 3,
    }
}

fn dedupe_evidence(evidence: &mut Vec<EvidenceRef>) {
    evidence.sort_by(|left, right| left.id.cmp(&right.id));
    evidence.dedup_by(|left, right| left.id == right.id);
}

fn stable_digest(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("{digest:x}")
}

fn validate_identity(value: &str, label: &str) -> Result<(), ProductProjectionError> {
    validate_display(value, label)?;
    let candidate = EvidenceRef {
        id: "validation".to_string(),
        kind: EvidenceKind::ProjectContext,
        source_revision: "validation".to_string(),
        summary: value.to_string(),
        agent: None,
        target_id: None,
    };
    if candidate.validate().is_err() {
        return Err(invalid(format!("{label} contains private path data")));
    }
    Ok(())
}

fn validate_component_revision(
    actual: &str,
    expected: &str,
    label: &str,
) -> Result<(), ProductProjectionError> {
    validate_identity(actual, &format!("{label} source revision"))?;
    if actual != expected {
        return Err(invalid(format!(
            "{label} source revision does not match accepted snapshot"
        )));
    }
    Ok(())
}

fn validate_display(value: &str, label: &str) -> Result<(), ProductProjectionError> {
    if value.trim().is_empty() || value.chars().any(char::is_control) {
        return Err(invalid(format!("{label} is empty or invalid")));
    }
    Ok(())
}

fn invalid(message: impl Into<String>) -> ProductProjectionError {
    ProductProjectionError::Invalid(message.into())
}

fn agent_display_name(agent: AgentId) -> &'static str {
    match agent {
        AgentId::ToolGlobal => "Tool Global",
        AgentId::ClaudeCode => "Claude Code",
        AgentId::Codex => "Codex",
        AgentId::Pi => "Pi",
        AgentId::Hermes => "Hermes",
        AgentId::Openclaw => "OpenClaw",
        AgentId::Opencode => "opencode",
    }
}
