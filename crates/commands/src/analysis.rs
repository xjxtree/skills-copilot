use super::*;

pub fn list_conflicts(catalog: &Catalog) -> Result<Vec<ConflictGroupRecord>, CommandError> {
    let groups = catalog.list_conflict_groups()?;
    if groups.is_empty() {
        return Ok(Vec::new());
    }

    let records = catalog.list_skill_records()?;
    let agent_by_instance_id = records
        .iter()
        .map(|record| (record.id.as_str(), record.agent.as_str()))
        .collect::<BTreeMap<_, _>>();
    Ok(runtime_conflict_groups(groups, &agent_by_instance_id))
}

pub fn analyze_catalog(
    catalog: &Catalog,
    ctx: &AdapterContext,
) -> Result<CrossAgentAnalysisRecord, CommandError> {
    let instances = visible_catalog_instances(
        catalog.list_skill_instances_for_project_context(ctx.project_root.as_deref())?,
    );
    Ok(analyze_skill_instances(&instances))
}

pub fn list_cross_agent_comparisons(
    catalog: &Catalog,
    ctx: &AdapterContext,
    selected_instance_id: Option<&str>,
    agent_filter: Option<&str>,
    query: Option<&str>,
    limit: Option<usize>,
) -> Result<CrossAgentComparisonRecord, CommandError> {
    let instances = visible_catalog_instances(
        catalog.list_skill_instances_for_project_context(ctx.project_root.as_deref())?,
    );
    let findings = dedupe_rule_finding_records(&catalog.list_rule_findings()?)
        .into_iter()
        .filter(|finding| !finding.suppressed)
        .collect::<Vec<_>>();
    let analysis = analyze_skill_instances(&instances);
    Ok(build_cross_agent_comparisons(
        CrossAgentComparisonBuildInput {
            instances: &instances,
            findings: &findings,
            analysis: &analysis,
            capabilities: &list_adapter_capabilities(ctx),
            selected_instance_id,
            agent_filter,
            query,
            limit,
        },
    ))
}

pub fn empty_cross_agent_comparison(
    selected_instance_id: Option<&str>,
) -> CrossAgentComparisonRecord {
    CrossAgentComparisonRecord {
        summary: CrossAgentComparisonSummary {
            total_groups: 0,
            returned_groups: 0,
            compared_skill_count: 0,
            agents_covered: Vec::new(),
            missing_agent_cells: 0,
            state_difference_groups: 0,
            source_difference_groups: 0,
            risk_group_count: 0,
            writable_mixed_groups: 0,
            selected_instance_id: selected_instance_id.map(str::to_string),
        },
        groups: Vec::new(),
        suggested_next_steps: vec![
            "Run Scan to build the local catalog before comparing agents.".to_string(),
            "Comparison is read-only and does not create catalog rows, snapshots, or config writes."
                .to_string(),
        ],
        read_only: true,
        writes_allowed: false,
        provider_request_sent: false,
    }
}

pub fn skill_health_summary(
    catalog: &Catalog,
    ctx: &AdapterContext,
) -> Result<SkillHealthSummary, CommandError> {
    let instances = visible_catalog_instances(
        catalog.list_skill_instances_for_project_context(ctx.project_root.as_deref())?,
    );
    let findings = catalog.list_rule_findings()?;
    let conflicts = catalog.list_conflict_groups()?;
    let analysis = analyze_skill_instances(&instances);
    Ok(build_skill_health_summary(
        &instances, &findings, &conflicts, &analysis,
    ))
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossAgentAnalysisRecord {
    pub summary: CrossAgentAnalysisSummary,
    pub groups: Vec<CrossAgentAnalysisGroup>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossAgentAnalysisSummary {
    pub total_groups: usize,
    pub duplicate_name_groups: usize,
    pub canonical_name_groups: usize,
    pub path_overlap_groups: usize,
    pub enabled_mismatch_groups: usize,
    pub malformed_groups: usize,
    pub precedence_groups: usize,
    pub affected_skill_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossAgentAnalysisGroup {
    pub id: String,
    pub kind: String,
    pub severity: String,
    pub title: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub canonical_name: Option<String>,
    pub explanation: String,
    pub instance_ids: Vec<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub winner_id: Option<String>,
    pub agents: Vec<String>,
    pub scopes: Vec<String>,
    pub paths: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossAgentComparisonRecord {
    pub summary: CrossAgentComparisonSummary,
    pub groups: Vec<CrossAgentComparisonGroup>,
    pub suggested_next_steps: Vec<String>,
    pub read_only: bool,
    pub writes_allowed: bool,
    pub provider_request_sent: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossAgentComparisonSummary {
    pub total_groups: usize,
    pub returned_groups: usize,
    pub compared_skill_count: usize,
    pub agents_covered: Vec<String>,
    pub missing_agent_cells: usize,
    pub state_difference_groups: usize,
    pub source_difference_groups: usize,
    pub risk_group_count: usize,
    pub writable_mixed_groups: usize,
    pub selected_instance_id: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossAgentComparisonGroup {
    pub id: String,
    pub canonical_name: String,
    pub display_name: String,
    pub definition_ids: Vec<String>,
    pub agents_present: Vec<String>,
    pub agents_missing: Vec<String>,
    pub member_count: usize,
    pub state_summary: CrossAgentComparisonStateSummary,
    pub source_summary: CrossAgentComparisonSourceSummary,
    pub risk_summary: CrossAgentComparisonRiskSummary,
    pub writable_summary: CrossAgentComparisonWritableSummary,
    pub analysis_group_ids: Vec<String>,
    pub analysis_kinds: Vec<String>,
    pub suggested_next_steps: Vec<String>,
    pub members: Vec<CrossAgentComparisonMember>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossAgentComparisonStateSummary {
    pub has_difference: bool,
    pub enabled_count: usize,
    pub disabled_count: usize,
    pub states: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossAgentComparisonSourceSummary {
    pub has_difference: bool,
    pub scopes: Vec<String>,
    pub root_labels: Vec<String>,
    pub path_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossAgentComparisonRiskSummary {
    pub has_risk: bool,
    pub finding_count: usize,
    pub highest_severity: Option<String>,
    pub broken_or_missing_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossAgentComparisonWritableSummary {
    pub has_mixed_capability: bool,
    pub writable_agent_count: usize,
    pub read_only_agent_count: usize,
    pub blocked_agents: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct CrossAgentComparisonMember {
    pub instance_id: String,
    pub name: String,
    pub agent: String,
    pub scope: String,
    pub state: String,
    pub enabled: bool,
    pub definition_id: String,
    pub path: String,
    pub root_label: String,
    pub finding_count: usize,
    pub highest_finding_severity: Option<String>,
    pub writable_supported: bool,
    pub writable_status: String,
    pub writable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct SkillHealthSummary {
    pub total_count: usize,
    pub enabled_count: usize,
    pub disabled_count: usize,
    pub broken_count: usize,
    pub missing_count: usize,
    pub malformed_count: usize,
    pub finding_count: usize,
    pub conflict_count: usize,
    pub risky_script_count: usize,
    pub risky_permission_count: usize,
    pub findings_by_severity: HealthSeverityCounts,
    pub analysis_groups: HealthAnalysisGroupCounts,
    pub agent_summaries: Vec<AgentSkillHealthSummary>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq, Default)]
pub struct HealthSeverityCounts {
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
}

impl HealthSeverityCounts {
    fn total(&self) -> usize {
        self.error_count + self.warning_count + self.info_count
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct HealthAnalysisGroupCounts {
    pub total_count: usize,
    pub error_count: usize,
    pub warning_count: usize,
    pub info_count: usize,
    pub duplicate_name_count: usize,
    pub canonical_name_count: usize,
    pub path_overlap_count: usize,
    pub enabled_mismatch_count: usize,
    pub malformed_count: usize,
    pub precedence_count: usize,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AgentSkillHealthSummary {
    pub agent: String,
    pub total_count: usize,
    pub enabled_count: usize,
    pub disabled_count: usize,
    pub broken_count: usize,
    pub missing_count: usize,
    pub malformed_count: usize,
    pub finding_count: usize,
    pub conflict_count: usize,
    pub risky_script_count: usize,
    pub risky_permission_count: usize,
    pub analysis_group_count: usize,
}

pub fn build_skill_health_summary(
    instances: &[SkillInstance],
    findings: &[RuleFindingRecord],
    conflicts: &[ConflictGroupRecord],
    analysis: &CrossAgentAnalysisRecord,
) -> SkillHealthSummary {
    let findings = dedupe_rule_finding_records(findings)
        .into_iter()
        .filter(|finding| !finding.suppressed)
        .collect::<Vec<_>>();
    let agent_by_instance_id = instances
        .iter()
        .map(|inst| (inst.id.as_str(), inst.agent.as_str()))
        .collect::<BTreeMap<_, _>>();
    let malformed_instance_ids = malformed_instance_ids(instances, &findings);
    let risky_script_instance_ids = risky_script_instance_ids(instances, &findings);
    let risky_permission_instance_ids = risky_permission_instance_ids(instances, &findings);
    let findings_by_severity = severity_counts(
        findings
            .iter()
            .map(|finding| finding.effective_severity.as_str()),
    );
    let analysis_groups = health_analysis_group_counts(analysis);

    let mut agent_summaries = Vec::new();
    for agent in instances
        .iter()
        .map(|inst| inst.agent.as_str().to_string())
        .collect::<BTreeSet<_>>()
    {
        let members = instances
            .iter()
            .filter(|inst| inst.agent.as_str() == agent)
            .collect::<Vec<_>>();
        let member_ids = members
            .iter()
            .map(|inst| inst.id.as_str())
            .collect::<BTreeSet<_>>();
        let finding_count = findings
            .iter()
            .filter(|finding| finding_applies_to(&member_ids, finding))
            .count();
        let conflict_count = conflicts
            .iter()
            .filter(|conflict| {
                conflict_is_runtime_same_agent(&agent_by_instance_id, conflict)
                    && conflict_applies_to_agent_instances(&member_ids, conflict)
            })
            .count();
        let analysis_group_count = analysis
            .groups
            .iter()
            .filter(|group| group.agents.iter().any(|group_agent| group_agent == &agent))
            .count();

        agent_summaries.push(AgentSkillHealthSummary {
            agent: agent.clone(),
            total_count: members.len(),
            enabled_count: members
                .iter()
                .filter(|inst| is_health_enabled(inst))
                .count(),
            disabled_count: members
                .iter()
                .filter(|inst| is_health_disabled(inst))
                .count(),
            broken_count: members
                .iter()
                .filter(|inst| matches!(inst.state, SkillState::Broken))
                .count(),
            missing_count: members
                .iter()
                .filter(|inst| matches!(inst.state, SkillState::Missing))
                .count(),
            malformed_count: member_ids
                .iter()
                .filter(|id| malformed_instance_ids.contains(**id))
                .count(),
            finding_count,
            conflict_count,
            risky_script_count: member_ids
                .iter()
                .filter(|id| risky_script_instance_ids.contains(**id))
                .count(),
            risky_permission_count: member_ids
                .iter()
                .filter(|id| risky_permission_instance_ids.contains(**id))
                .count(),
            analysis_group_count,
        });
    }

    SkillHealthSummary {
        total_count: instances.len(),
        enabled_count: instances
            .iter()
            .filter(|inst| is_health_enabled(inst))
            .count(),
        disabled_count: instances
            .iter()
            .filter(|inst| is_health_disabled(inst))
            .count(),
        broken_count: instances
            .iter()
            .filter(|inst| matches!(inst.state, SkillState::Broken))
            .count(),
        missing_count: instances
            .iter()
            .filter(|inst| matches!(inst.state, SkillState::Missing))
            .count(),
        malformed_count: malformed_instance_ids.len(),
        finding_count: findings_by_severity.total(),
        conflict_count: conflicts
            .iter()
            .filter(|conflict| conflict_is_runtime_same_agent(&agent_by_instance_id, conflict))
            .count(),
        risky_script_count: risky_script_instance_ids.len(),
        risky_permission_count: risky_permission_instance_ids.len(),
        findings_by_severity,
        analysis_groups,
        agent_summaries,
    }
}

pub fn analyze_skill_instances(instances: &[SkillInstance]) -> CrossAgentAnalysisRecord {
    let mut groups = Vec::new();

    append_duplicate_name_groups(instances, &mut groups);
    append_canonical_name_groups(instances, &mut groups);
    append_path_overlap_groups(instances, &mut groups);
    append_enabled_mismatch_groups(instances, &mut groups);
    append_malformed_groups(instances, &mut groups);
    append_precedence_groups(instances, &mut groups);

    groups.sort_by(|left, right| {
        severity_rank(&left.severity)
            .cmp(&severity_rank(&right.severity))
            .then_with(|| left.kind.cmp(&right.kind))
            .then_with(|| left.title.cmp(&right.title))
    });

    let affected_skill_count = groups
        .iter()
        .flat_map(|group| group.instance_ids.iter().cloned())
        .collect::<BTreeSet<_>>()
        .len();

    CrossAgentAnalysisRecord {
        summary: CrossAgentAnalysisSummary {
            total_groups: groups.len(),
            duplicate_name_groups: count_kind(&groups, "duplicate_name"),
            canonical_name_groups: count_kind(&groups, "canonical_name_overlap"),
            path_overlap_groups: count_kind(&groups, "source_path_overlap"),
            enabled_mismatch_groups: count_kind(&groups, "enabled_state_mismatch"),
            malformed_groups: count_kind(&groups, "malformed_or_broken"),
            precedence_groups: count_kind(&groups, "precedence_shadowing"),
            affected_skill_count,
        },
        groups,
    }
}

struct CrossAgentComparisonBuildInput<'a> {
    instances: &'a [SkillInstance],
    findings: &'a [RuleFindingRecord],
    analysis: &'a CrossAgentAnalysisRecord,
    capabilities: &'a [AdapterCapabilityRecord],
    selected_instance_id: Option<&'a str>,
    agent_filter: Option<&'a str>,
    query: Option<&'a str>,
    limit: Option<usize>,
}

fn build_cross_agent_comparisons(
    input: CrossAgentComparisonBuildInput<'_>,
) -> CrossAgentComparisonRecord {
    let supported_agents = cross_agent_comparison_agents();
    let normalized_agent_filter = input
        .agent_filter
        .map(str::trim)
        .filter(|agent| !agent.is_empty())
        .map(str::to_ascii_lowercase);
    let normalized_query = input
        .query
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(str::to_ascii_lowercase);
    let selected_canonical = input.selected_instance_id.and_then(|selected_id| {
        input
            .instances
            .iter()
            .find(|inst| inst.id == selected_id)
            .map(|inst| canonical_skill_name_suggestion(&inst.name))
    });
    let capability_by_agent = input
        .capabilities
        .iter()
        .map(|capability| (capability.agent.to_string(), capability))
        .collect::<BTreeMap<_, _>>();
    let mut by_canonical = BTreeMap::<String, Vec<&SkillInstance>>::new();
    for inst in input.instances {
        if inst.agent == AgentId::ToolGlobal {
            continue;
        }
        by_canonical
            .entry(canonical_skill_name_suggestion(&inst.name))
            .or_default()
            .push(inst);
    }

    let mut groups = Vec::new();
    for (canonical_name, mut members) in by_canonical {
        members.sort_by(|left, right| {
            left.agent
                .as_str()
                .cmp(right.agent.as_str())
                .then_with(|| left.scope.as_str().cmp(right.scope.as_str()))
                .then_with(|| left.name.cmp(&right.name))
        });
        if members.len() < 2
            && selected_canonical
                .as_deref()
                .is_none_or(|selected| selected != canonical_name)
        {
            continue;
        }
        if let Some(agent) = normalized_agent_filter.as_deref() {
            if !members
                .iter()
                .any(|inst| inst.agent.as_str().eq_ignore_ascii_case(agent))
            {
                continue;
            }
        }
        if let Some(query) = normalized_query.as_deref() {
            if !comparison_group_matches_query(&canonical_name, &members, query) {
                continue;
            }
        }

        groups.push(cross_agent_comparison_group(
            &canonical_name,
            members,
            input.findings,
            &capability_by_agent,
            &supported_agents,
            &input.analysis.groups,
        ));
    }

    groups.sort_by(|left, right| {
        bool_rank(right.risk_summary.has_risk)
            .cmp(&bool_rank(left.risk_summary.has_risk))
            .then_with(|| {
                bool_rank(right.state_summary.has_difference)
                    .cmp(&bool_rank(left.state_summary.has_difference))
            })
            .then_with(|| left.canonical_name.cmp(&right.canonical_name))
    });

    let total_groups = groups.len();
    if let Some(limit) = input.limit.filter(|limit| *limit > 0) {
        groups.truncate(limit);
    }

    let compared_skill_count = groups
        .iter()
        .flat_map(|group| {
            group
                .members
                .iter()
                .map(|member| member.instance_id.clone())
        })
        .collect::<BTreeSet<_>>()
        .len();
    let agents_covered = groups
        .iter()
        .flat_map(|group| group.agents_present.iter().cloned())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();

    CrossAgentComparisonRecord {
        summary: CrossAgentComparisonSummary {
            total_groups,
            returned_groups: groups.len(),
            compared_skill_count,
            agents_covered,
            missing_agent_cells: groups
                .iter()
                .map(|group| group.agents_missing.len())
                .sum(),
            state_difference_groups: groups
                .iter()
                .filter(|group| group.state_summary.has_difference)
                .count(),
            source_difference_groups: groups
                .iter()
                .filter(|group| group.source_summary.has_difference)
                .count(),
            risk_group_count: groups
                .iter()
                .filter(|group| group.risk_summary.has_risk)
                .count(),
            writable_mixed_groups: groups
                .iter()
                .filter(|group| group.writable_summary.has_mixed_capability)
                .count(),
            selected_instance_id: input.selected_instance_id.map(str::to_string),
        },
        groups,
        suggested_next_steps: vec![
            "Open skill details for any member with findings before changing agent config."
                .to_string(),
            "Use Analysis insights to review duplicate names, shared sources, and enabled-state drift."
                .to_string(),
            "Check adapter capability blockers before using existing toggle, install, save, or rollback flows."
                .to_string(),
        ],
        read_only: true,
        writes_allowed: false,
        provider_request_sent: false,
    }
}

fn cross_agent_comparison_group(
    canonical_name: &str,
    members: Vec<&SkillInstance>,
    findings: &[RuleFindingRecord],
    capability_by_agent: &BTreeMap<String, &AdapterCapabilityRecord>,
    supported_agents: &[&str],
    analysis_groups: &[CrossAgentAnalysisGroup],
) -> CrossAgentComparisonGroup {
    let member_ids = members
        .iter()
        .map(|inst| inst.id.as_str())
        .collect::<BTreeSet<_>>();
    let agents_present = members
        .iter()
        .map(|inst| inst.agent.as_str().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let agents_missing = supported_agents
        .iter()
        .filter(|agent| !agents_present.iter().any(|present| present == **agent))
        .map(|agent| (*agent).to_string())
        .collect::<Vec<_>>();
    let states = members
        .iter()
        .map(|inst| inst.state.as_str().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let enabled_values = members
        .iter()
        .map(|inst| inst.enabled)
        .collect::<BTreeSet<_>>();
    let scopes = members
        .iter()
        .map(|inst| inst.scope.as_str().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let root_labels = members
        .iter()
        .map(|inst| comparison_root_label(inst))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let paths = members
        .iter()
        .map(|inst| inst.display_path.to_string_lossy().to_string())
        .collect::<BTreeSet<_>>();
    let group_findings = findings_for_instances(findings, &member_ids);
    let highest_severity = highest_finding_severity(
        group_findings
            .iter()
            .map(|finding| finding.effective_severity.as_str()),
    );
    let broken_or_missing_count = members
        .iter()
        .filter(|inst| matches!(inst.state, SkillState::Broken | SkillState::Missing))
        .count();
    let writable_supported_values = supported_agents
        .iter()
        .map(|agent| {
            capability_by_agent
                .get(*agent)
                .is_some_and(|capability| capability.writable.supported)
        })
        .collect::<BTreeSet<_>>();
    let blocked_agents = supported_agents
        .iter()
        .filter(|agent| {
            !capability_by_agent
                .get(**agent)
                .is_some_and(|capability| capability.writable.supported)
        })
        .map(|agent| (*agent).to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let writable_agent_count = supported_agents
        .iter()
        .filter(|agent| {
            capability_by_agent
                .get(**agent)
                .is_some_and(|capability| capability.writable.supported)
        })
        .collect::<BTreeSet<_>>()
        .len();
    let read_only_agent_count = supported_agents.len().saturating_sub(writable_agent_count);
    let related_analysis_groups = analysis_groups
        .iter()
        .filter(|group| {
            group
                .instance_ids
                .iter()
                .any(|instance_id| member_ids.contains(instance_id.as_str()))
        })
        .collect::<Vec<_>>();
    let has_state_difference = states.len() > 1 || enabled_values.len() > 1;
    let has_source_difference = scopes.len() > 1 || root_labels.len() > 1 || paths.len() > 1;
    let has_risk = !group_findings.is_empty() || broken_or_missing_count > 0;
    let has_mixed_capability = writable_supported_values.len() > 1;
    let analysis_group_ids = related_analysis_groups
        .iter()
        .map(|group| group.id.clone())
        .collect::<Vec<_>>();
    let analysis_kinds = related_analysis_groups
        .iter()
        .map(|group| group.kind.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let comparison_members = members
        .iter()
        .map(|inst| {
            let member_findings =
                findings_for_instances(findings, &BTreeSet::from([inst.id.as_str()]));
            let capability = capability_by_agent.get(inst.agent.as_str());
            CrossAgentComparisonMember {
                instance_id: inst.id.clone(),
                name: inst.name.clone(),
                agent: inst.agent.as_str().to_string(),
                scope: inst.scope.as_str().to_string(),
                state: inst.state.as_str().to_string(),
                enabled: inst.enabled,
                definition_id: inst.definition_id.clone(),
                path: inst.display_path.to_string_lossy().to_string(),
                root_label: comparison_root_label(inst),
                finding_count: member_findings.len(),
                highest_finding_severity: highest_finding_severity(
                    member_findings
                        .iter()
                        .map(|finding| finding.effective_severity.as_str()),
                ),
                writable_supported: capability
                    .is_some_and(|capability| capability.writable.supported),
                writable_status: capability
                    .map(|capability| capability.writable.status.to_string())
                    .unwrap_or_else(|| "unknown".to_string()),
                writable_reason: capability
                    .and_then(|capability| capability.writable.reason.map(str::to_string)),
            }
        })
        .collect::<Vec<_>>();

    CrossAgentComparisonGroup {
        id: format!(
            "comparison:{}:{}",
            canonical_name,
            short_hash(&member_ids.iter().copied().collect::<Vec<_>>().join("|"))
        ),
        canonical_name: canonical_name.to_string(),
        display_name: members
            .first()
            .map(|inst| inst.name.clone())
            .unwrap_or_else(|| canonical_name.to_string()),
        definition_ids: members
            .iter()
            .map(|inst| inst.definition_id.clone())
            .collect::<BTreeSet<_>>()
            .into_iter()
            .collect(),
        agents_present,
        agents_missing,
        member_count: comparison_members.len(),
        state_summary: CrossAgentComparisonStateSummary {
            has_difference: has_state_difference,
            enabled_count: members.iter().filter(|inst| inst.enabled).count(),
            disabled_count: members.iter().filter(|inst| !inst.enabled).count(),
            states,
        },
        source_summary: CrossAgentComparisonSourceSummary {
            has_difference: has_source_difference,
            scopes,
            root_labels,
            path_count: paths.len(),
        },
        risk_summary: CrossAgentComparisonRiskSummary {
            has_risk,
            finding_count: group_findings.len(),
            highest_severity,
            broken_or_missing_count,
        },
        writable_summary: CrossAgentComparisonWritableSummary {
            has_mixed_capability,
            writable_agent_count,
            read_only_agent_count,
            blocked_agents,
        },
        analysis_group_ids,
        analysis_kinds,
        suggested_next_steps: comparison_next_steps(
            has_risk,
            has_state_difference,
            has_source_difference,
            has_mixed_capability,
        ),
        members: comparison_members,
    }
}

fn comparison_group_matches_query(
    canonical_name: &str,
    members: &[&SkillInstance],
    query: &str,
) -> bool {
    canonical_name.contains(query)
        || members.iter().any(|inst| {
            inst.id.to_ascii_lowercase().contains(query)
                || inst.name.to_ascii_lowercase().contains(query)
                || inst.definition_id.to_ascii_lowercase().contains(query)
                || inst
                    .display_path
                    .to_string_lossy()
                    .to_ascii_lowercase()
                    .contains(query)
        })
}

fn findings_for_instances<'a>(
    findings: &'a [RuleFindingRecord],
    member_ids: &BTreeSet<&str>,
) -> Vec<&'a RuleFindingRecord> {
    findings
        .iter()
        .filter(|finding| {
            finding
                .instance_id
                .as_deref()
                .is_some_and(|instance_id| member_ids.contains(instance_id))
        })
        .collect()
}

fn highest_finding_severity<'a>(severities: impl Iterator<Item = &'a str>) -> Option<String> {
    severities
        .min_by_key(|severity| severity_rank(severity))
        .map(|severity| severity.to_string())
}

fn comparison_root_label(inst: &SkillInstance) -> String {
    format!("{}:{}", inst.agent.as_str(), inst.scope.as_str())
}

fn comparison_next_steps(
    has_risk: bool,
    has_state_difference: bool,
    has_source_difference: bool,
    has_mixed_capability: bool,
) -> Vec<String> {
    let mut steps = vec!["Review member rows side-by-side before taking action.".to_string()];
    if has_risk {
        steps.push("Inspect findings and broken/missing rows in skill details.".to_string());
    }
    if has_state_difference {
        steps.push(
            "Compare enabled and load states across agents before using existing toggle flows."
                .to_string(),
        );
    }
    if has_source_difference {
        steps.push("Check whether different roots or shared paths explain the drift.".to_string());
    }
    if has_mixed_capability {
        steps.push("Use adapter capability status to separate writable agents from read-only or blocked agents.".to_string());
    }
    steps
}

fn cross_agent_comparison_agents() -> Vec<&'static str> {
    vec![
        AgentId::ClaudeCode.as_str(),
        AgentId::Codex.as_str(),
        AgentId::Opencode.as_str(),
        AgentId::Pi.as_str(),
        AgentId::Hermes.as_str(),
        AgentId::Openclaw.as_str(),
    ]
}

fn bool_rank(value: bool) -> u8 {
    if value {
        1
    } else {
        0
    }
}

fn is_health_enabled(inst: &SkillInstance) -> bool {
    inst.enabled && matches!(inst.state, SkillState::Loaded)
}

fn is_health_disabled(inst: &SkillInstance) -> bool {
    !inst.enabled || matches!(inst.state, SkillState::Disabled)
}

fn malformed_instance_ids(
    instances: &[SkillInstance],
    findings: &[RuleFindingRecord],
) -> BTreeSet<String> {
    let mut ids = instances
        .iter()
        .filter(|inst| matches!(inst.state, SkillState::Broken | SkillState::Missing))
        .map(|inst| inst.id.clone())
        .collect::<BTreeSet<_>>();
    add_finding_affected_instances(
        findings
            .iter()
            .filter(|finding| finding.rule_id == "frontmatter.required-fields"),
        instances,
        &mut ids,
    );
    ids
}

fn risky_script_instance_ids(
    instances: &[SkillInstance],
    findings: &[RuleFindingRecord],
) -> BTreeSet<String> {
    let mut ids = instances
        .iter()
        .filter(|inst| !inst.scripts.is_empty())
        .map(|inst| inst.id.clone())
        .collect::<BTreeSet<_>>();
    add_finding_affected_instances(
        findings
            .iter()
            .filter(|finding| finding.rule_id.starts_with("script.")),
        instances,
        &mut ids,
    );
    ids
}

fn risky_permission_instance_ids(
    instances: &[SkillInstance],
    findings: &[RuleFindingRecord],
) -> BTreeSet<String> {
    let mut ids = instances
        .iter()
        .filter(|inst| {
            inst.permissions.exec
                || !matches!(inst.permissions.network, NetworkAccess::None)
                || !inst.permissions.tools.is_empty()
        })
        .map(|inst| inst.id.clone())
        .collect::<BTreeSet<_>>();
    add_finding_affected_instances(
        findings.iter().filter(|finding| {
            matches!(
                finding.rule_id.as_str(),
                "frontmatter.tools-not-empty"
                    | "permissions.network-declared"
                    | "permissions.exec-needs-human"
                    | "dependency.unknown"
            )
        }),
        instances,
        &mut ids,
    );
    ids
}

fn add_finding_affected_instances<'a>(
    findings: impl Iterator<Item = &'a RuleFindingRecord>,
    instances: &[SkillInstance],
    ids: &mut BTreeSet<String>,
) {
    for finding in findings {
        if let Some(instance_id) = &finding.instance_id {
            ids.insert(instance_id.clone());
        }
        if let Some(definition_id) = &finding.definition_id {
            ids.extend(
                instances
                    .iter()
                    .filter(|inst| &inst.definition_id == definition_id)
                    .map(|inst| inst.id.clone()),
            );
        }
    }
}

fn severity_counts<'a>(severities: impl Iterator<Item = &'a str>) -> HealthSeverityCounts {
    let mut counts = HealthSeverityCounts::default();
    for severity in severities {
        match severity {
            "error" => counts.error_count += 1,
            "warn" | "warning" => counts.warning_count += 1,
            "info" => counts.info_count += 1,
            _ => counts.info_count += 1,
        }
    }
    counts
}

pub(crate) fn dedupe_rule_finding_records(
    findings: &[RuleFindingRecord],
) -> Vec<RuleFindingRecord> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for finding in findings {
        if seen.insert(rule_finding_record_key(finding)) {
            deduped.push(finding.clone());
        }
    }
    deduped
}

fn rule_finding_record_key(finding: &RuleFindingRecord) -> String {
    stable_finding_key(
        finding.instance_id.as_deref(),
        finding.definition_id.as_deref(),
        &finding.rule_id,
        &finding.message,
        finding.suggestion.as_deref(),
    )
}

pub(crate) fn dedupe_rule_findings(findings: Vec<Finding>) -> Vec<Finding> {
    let mut seen = BTreeSet::new();
    let mut deduped = Vec::new();
    for finding in findings {
        if seen.insert(finding_key(&finding)) {
            deduped.push(finding);
        }
    }
    deduped
}

fn finding_key(finding: &Finding) -> String {
    stable_finding_key(
        finding.instance_id.as_deref(),
        finding.definition_id.as_deref(),
        &finding.rule_id,
        &finding.message,
        finding.suggestion.as_deref(),
    )
}

fn stable_finding_key(
    instance_id: Option<&str>,
    definition_id: Option<&str>,
    rule_id: &str,
    message: &str,
    suggestion: Option<&str>,
) -> String {
    format!(
        "{}\x1f{}\x1f{}\x1f{}\x1f{}",
        instance_id.unwrap_or(""),
        definition_id.unwrap_or(""),
        rule_id,
        message,
        suggestion.unwrap_or("")
    )
}

pub(crate) fn validate_finding_triage_status(status: &str) -> Result<(), CommandError> {
    match status {
        "reviewed" | "ignored" | "needs-follow-up" => Ok(()),
        _ => Err(CommandError::InvalidFindingTriageStatus(status.to_string())),
    }
}

pub(crate) fn validate_rule_tuning_key(rule_id: &str) -> Result<(), CommandError> {
    if rule_id.trim().is_empty() {
        return Err(CommandError::InvalidRuleTuningRequest(
            "rule_id is required".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_rule_scope(
    agent: Option<&str>,
    scope: Option<&str>,
) -> Result<(), CommandError> {
    if agent.is_none() && scope.is_some() {
        return Err(CommandError::InvalidRuleTuningRequest(
            "scope-specific tuning requires agent".to_string(),
        ));
    }
    Ok(())
}

pub(crate) fn validate_rule_severity_override(severity: &str) -> Result<(), CommandError> {
    match severity.trim() {
        "critical" | "error" | "warn" | "warning" | "info" => Ok(()),
        _ => Err(CommandError::InvalidRuleSeverityOverride(
            severity.to_string(),
        )),
    }
}

pub(crate) fn validate_rule_suppression_reason(reason: &str) -> Result<(), CommandError> {
    if reason.trim().is_empty() {
        return Err(CommandError::InvalidRuleTuningRequest(
            "suppression reason is required".to_string(),
        ));
    }
    Ok(())
}

fn health_analysis_group_counts(analysis: &CrossAgentAnalysisRecord) -> HealthAnalysisGroupCounts {
    let severity = severity_counts(analysis.groups.iter().map(|group| group.severity.as_str()));
    HealthAnalysisGroupCounts {
        total_count: analysis.summary.total_groups,
        error_count: severity.error_count,
        warning_count: severity.warning_count,
        info_count: severity.info_count,
        duplicate_name_count: analysis.summary.duplicate_name_groups,
        canonical_name_count: analysis.summary.canonical_name_groups,
        path_overlap_count: analysis.summary.path_overlap_groups,
        enabled_mismatch_count: analysis.summary.enabled_mismatch_groups,
        malformed_count: analysis.summary.malformed_groups,
        precedence_count: analysis.summary.precedence_groups,
    }
}

fn finding_applies_to(instance_ids: &BTreeSet<&str>, finding: &RuleFindingRecord) -> bool {
    finding
        .instance_id
        .as_deref()
        .is_some_and(|instance_id| instance_ids.contains(instance_id))
}

fn conflict_applies_to_agent_instances(
    instance_ids: &BTreeSet<&str>,
    conflict: &ConflictGroupRecord,
) -> bool {
    conflict
        .instance_ids
        .iter()
        .filter(|instance_id| instance_ids.contains(instance_id.as_str()))
        .count()
        > 1
}

fn conflict_has_same_agent_instances(
    agent_by_instance_id: &BTreeMap<&str, &str>,
    conflict: &ConflictGroupRecord,
) -> bool {
    let mut counts_by_agent = BTreeMap::new();
    for instance_id in &conflict.instance_ids {
        if let Some(agent) = agent_by_instance_id.get(instance_id.as_str()) {
            let count = counts_by_agent.entry(*agent).or_insert(0usize);
            *count += 1;
            if *count > 1 {
                return true;
            }
        }
    }
    false
}

fn runtime_conflict_groups(
    conflicts: Vec<ConflictGroupRecord>,
    agent_by_instance_id: &BTreeMap<&str, &str>,
) -> Vec<ConflictGroupRecord> {
    conflicts
        .into_iter()
        .filter(|conflict| conflict_is_runtime_same_agent(agent_by_instance_id, conflict))
        .collect()
}

fn conflict_is_runtime_same_agent(
    agent_by_instance_id: &BTreeMap<&str, &str>,
    conflict: &ConflictGroupRecord,
) -> bool {
    is_runtime_conflict_reason(&conflict.reason)
        && conflict_has_same_agent_instances(agent_by_instance_id, conflict)
}

fn is_runtime_conflict_reason(reason: &str) -> bool {
    matches!(reason, "name-collision" | "content-drift")
}

fn append_duplicate_name_groups(
    instances: &[SkillInstance],
    groups: &mut Vec<CrossAgentAnalysisGroup>,
) {
    let mut by_name: BTreeMap<String, Vec<&SkillInstance>> = BTreeMap::new();
    for inst in instances {
        by_name
            .entry(inst.name.trim().to_ascii_lowercase())
            .or_default()
            .push(inst);
    }
    for (name, members) in by_name {
        if members.len() < 2 {
            continue;
        }
        groups.push(analysis_group(
            "duplicate_name",
            "warning",
            format!("Duplicate skill name '{name}' appears in {} records.", members.len()),
            Some(name.clone()),
            "Multiple visible skills use the same name. Agents load independently, so this is not automatically a runtime conflict across agents, but users may see ambiguous skills in the catalog.".to_string(),
            members,
            None,
        ));
    }
}

fn append_canonical_name_groups(
    instances: &[SkillInstance],
    groups: &mut Vec<CrossAgentAnalysisGroup>,
) {
    let mut by_canonical: BTreeMap<String, Vec<&SkillInstance>> = BTreeMap::new();
    for inst in instances {
        by_canonical
            .entry(canonical_skill_name_suggestion(&inst.name))
            .or_default()
            .push(inst);
    }
    for (canonical_name, members) in by_canonical {
        if members.len() < 2 {
            continue;
        }
        let distinct_names = members
            .iter()
            .map(|inst| inst.name.trim().to_ascii_lowercase())
            .collect::<BTreeSet<_>>();
        if distinct_names.len() < 2 {
            continue;
        }
        groups.push(analysis_group(
            "canonical_name_overlap",
            "info",
            format!(
                "Canonical name '{canonical_name}' maps to {} visible spelling variants.",
                distinct_names.len()
            ),
            Some(canonical_name),
            "These skills are not exact duplicates, but their names normalize to the same canonical slug. Review them together before renaming, exporting, or installing shared copies.".to_string(),
            members,
            None,
        ));
    }
}

fn append_path_overlap_groups(
    instances: &[SkillInstance],
    groups: &mut Vec<CrossAgentAnalysisGroup>,
) {
    let mut by_path: BTreeMap<String, Vec<&SkillInstance>> = BTreeMap::new();
    for inst in instances {
        by_path
            .entry(inst.path.to_string_lossy().to_string())
            .or_default()
            .push(inst);
    }
    for (path, members) in by_path {
        if members.len() < 2 {
            continue;
        }
        groups.push(analysis_group(
            "source_path_overlap",
            "warning",
            format!("Same SKILL.md source is cataloged by {} records.", members.len()),
            None,
            format!(
                "The same physical skill path is visible through multiple catalog rows: {path}. Treat edits to this file as shared-source changes even though this analysis does not write files."
            ),
            members,
            None,
        ));
    }
}

fn append_enabled_mismatch_groups(
    instances: &[SkillInstance],
    groups: &mut Vec<CrossAgentAnalysisGroup>,
) {
    let mut by_canonical: BTreeMap<String, Vec<&SkillInstance>> = BTreeMap::new();
    for inst in instances {
        by_canonical
            .entry(canonical_skill_name_suggestion(&inst.name))
            .or_default()
            .push(inst);
    }
    for (canonical_name, members) in by_canonical {
        if members.len() < 2 {
            continue;
        }
        let enabled_values = members
            .iter()
            .map(|inst| inst.enabled)
            .collect::<BTreeSet<_>>();
        let state_values = members
            .iter()
            .map(|inst| inst.state.as_str())
            .collect::<BTreeSet<_>>();
        if enabled_values.len() < 2 && state_values.len() < 2 {
            continue;
        }
        groups.push(analysis_group(
            "enabled_state_mismatch",
            "warning",
            format!("Canonical name '{canonical_name}' has mixed enabled or load states."),
            Some(canonical_name),
            "Some visible records are enabled/loaded while related records are disabled, shadowed, missing, or broken. This is read-only catalog evidence; use adapter capability blockers before attempting any config action.".to_string(),
            members,
            None,
        ));
    }
}

fn append_malformed_groups(instances: &[SkillInstance], groups: &mut Vec<CrossAgentAnalysisGroup>) {
    let members: Vec<&SkillInstance> = instances
        .iter()
        .filter(|inst| matches!(inst.state, SkillState::Broken | SkillState::Missing))
        .collect();
    if members.is_empty() {
        return;
    }
    groups.push(analysis_group(
        "malformed_or_broken",
        "error",
        format!(
            "{} visible skill record(s) are broken, malformed, or missing.",
            members.len()
        ),
        None,
        "Broken rows usually come from parser/frontmatter failures; missing rows are retained catalog records from previously scanned roots. Rescan or inspect the source before relying on these skills.".to_string(),
        members,
        None,
    ));
}

fn append_precedence_groups(
    instances: &[SkillInstance],
    groups: &mut Vec<CrossAgentAnalysisGroup>,
) {
    let mut by_agent_and_name: BTreeMap<(String, String), Vec<&SkillInstance>> = BTreeMap::new();
    for inst in instances {
        if inst.agent == AgentId::ToolGlobal {
            continue;
        }
        by_agent_and_name
            .entry((
                inst.agent.as_str().to_string(),
                canonical_skill_name_suggestion(&inst.name),
            ))
            .or_default()
            .push(inst);
    }
    for ((agent, canonical_name), members) in by_agent_and_name {
        if members.len() < 2
            && !members
                .iter()
                .any(|inst| matches!(inst.state, SkillState::Shadowed))
        {
            continue;
        }
        let winner_id = precedence_winner_id(&members);
        groups.push(analysis_group(
            "precedence_shadowing",
            "info",
            format!(
                "{} has {} visible records for canonical name '{canonical_name}'.",
                agent,
                members.len()
            ),
            Some(canonical_name),
            "Within a single agent, project-scoped skills are treated as higher precedence than agent-global rows when both are visible. Cross-agent duplicates do not share runtime precedence because each agent loads its own roots independently.".to_string(),
            members,
            winner_id,
        ));
    }
}

fn analysis_group(
    kind: &str,
    severity: &str,
    title: String,
    canonical_name: Option<String>,
    explanation: String,
    members: Vec<&SkillInstance>,
    winner_id: Option<String>,
) -> CrossAgentAnalysisGroup {
    let instance_ids = members
        .iter()
        .map(|inst| inst.id.clone())
        .collect::<Vec<_>>();
    let agents = members
        .iter()
        .map(|inst| inst.agent.as_str().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let scopes = members
        .iter()
        .map(|inst| inst.scope.as_str().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect();
    let paths = members
        .iter()
        .map(|inst| inst.display_path.to_string_lossy().to_string())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    let seed = format!(
        "{kind}|{}|{}",
        canonical_name.as_deref().unwrap_or(""),
        instance_ids.join("|")
    );

    CrossAgentAnalysisGroup {
        id: format!("analysis:{kind}:{}", short_hash(&seed)),
        kind: kind.to_string(),
        severity: severity.to_string(),
        title,
        canonical_name,
        explanation,
        instance_ids,
        winner_id,
        agents,
        scopes,
        paths,
    }
}

fn precedence_winner_id(members: &[&SkillInstance]) -> Option<String> {
    members
        .iter()
        .filter(|inst| inst.enabled && matches!(inst.state, SkillState::Loaded))
        .min_by_key(|inst| (scope_precedence_rank(inst.scope), inst.name.clone()))
        .map(|inst| inst.id.clone())
}

fn scope_precedence_rank(scope: Scope) -> u8 {
    match scope {
        Scope::AgentProject => 0,
        Scope::AgentGlobal => 1,
        Scope::ToolGlobal => 2,
        _ => 3,
    }
}

fn severity_rank(severity: &str) -> u8 {
    match severity {
        "error" => 0,
        "warn" | "warning" => 1,
        "info" => 2,
        _ => 3,
    }
}

fn count_kind(groups: &[CrossAgentAnalysisGroup], kind: &str) -> usize {
    groups.iter().filter(|group| group.kind == kind).count()
}
