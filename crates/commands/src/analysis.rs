use super::*;
use skills_copilot_adapters::{
    codex_home_dir, codex_plugin_cache_id, parse_codex_enabled_plugin_ids,
};
use skills_copilot_ai_core::NATIVE_RUNTIME_NAMESPACE;

pub(super) fn runtime_conflict_namespaces(
    instances: &[SkillInstance],
    ctx: &AdapterContext,
) -> std::collections::HashMap<String, String> {
    let codex_home = codex_home_dir(ctx);
    let enabled_codex_plugins = fs::read_to_string(codex_home.join("config.toml"))
        .map(|content| parse_codex_enabled_plugin_ids(&content))
        .unwrap_or_default();
    let codex_plugin_roots = CodexAdapter
        .roots(ctx)
        .into_iter()
        .filter(|root| root.source == RootSource::Plugin)
        .map(|root| root.path)
        .collect::<Vec<_>>();

    instances
        .iter()
        .filter_map(|instance| {
            if instance.state != SkillState::Loaded
                || !instance.enabled
                || instance.scope == Scope::ToolGlobal
            {
                return None;
            }
            if instance.agent != AgentId::Codex {
                return Some((instance.id.clone(), NATIVE_RUNTIME_NAMESPACE.to_string()));
            }

            let is_plugin_inventory = codex_plugin_roots
                .iter()
                .any(|root| instance.path.starts_with(root));
            if !is_plugin_inventory {
                return Some((instance.id.clone(), NATIVE_RUNTIME_NAMESPACE.to_string()));
            }

            let plugin_id = codex_plugin_cache_id(&codex_home, &instance.path)?;
            enabled_codex_plugins
                .contains(&plugin_id)
                .then(|| (instance.id.clone(), format!("plugin:{plugin_id}")))
        })
        .collect()
}

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

pub fn apply_current_config_overrides_to_skill_records(
    ctx: &AdapterContext,
    records: &mut [SkillRecord],
) -> Result<(), CommandError> {
    let disabled_paths = codex_disabled_skill_paths(&codex_user_config_path(ctx))?;
    for record in records.iter_mut() {
        if record.agent != AgentId::Codex.as_str() {
            continue;
        }
        let (state, enabled) = projected_codex_config_state(
            &record.path,
            &record.state,
            record.enabled,
            &disabled_paths,
        );
        record.state = state;
        record.enabled = enabled;
    }
    Ok(())
}

pub fn apply_current_config_overrides_to_skill_detail(
    ctx: &AdapterContext,
    detail: &mut SkillDetailRecord,
) -> Result<(), CommandError> {
    if detail.agent != AgentId::Codex.as_str() {
        return Ok(());
    }
    let disabled_paths = codex_disabled_skill_paths(&codex_user_config_path(ctx))?;
    let (state, enabled) =
        projected_codex_config_state(&detail.path, &detail.state, detail.enabled, &disabled_paths);
    detail.state = state;
    detail.enabled = enabled;
    Ok(())
}

fn projected_codex_config_state(
    path: &Path,
    state: &str,
    enabled: bool,
    disabled_paths: &BTreeSet<PathBuf>,
) -> (String, bool) {
    if !matches!(state, "loaded" | "disabled") {
        return (state.to_string(), enabled);
    }
    if disabled_paths.contains(path) {
        (SkillState::Disabled.as_str().to_string(), false)
    } else {
        (SkillState::Loaded.as_str().to_string(), true)
    }
}

pub fn list_conflicts_for_context(
    catalog: &Catalog,
    ctx: &AdapterContext,
) -> Result<Vec<ConflictGroupRecord>, CommandError> {
    let instances = projected_visible_catalog_instances(catalog, ctx)?;
    Ok(projected_runtime_conflicts(&instances, ctx))
}

pub fn analyze_catalog(
    catalog: &Catalog,
    ctx: &AdapterContext,
) -> Result<CrossAgentAnalysisRecord, CommandError> {
    let instances = projected_visible_catalog_instances(catalog, ctx)?;
    Ok(analyze_skill_instances(&instances))
}

pub fn skill_health_summary(
    catalog: &Catalog,
    ctx: &AdapterContext,
) -> Result<SkillHealthSummary, CommandError> {
    let instances = projected_visible_catalog_instances(catalog, ctx)?;
    let findings = catalog.list_rule_findings()?;
    let conflicts = projected_runtime_conflicts(&instances, ctx);
    let analysis = analyze_skill_instances(&instances);
    Ok(build_skill_health_summary(
        &instances, &findings, &conflicts, &analysis,
    ))
}

fn projected_visible_catalog_instances(
    catalog: &Catalog,
    ctx: &AdapterContext,
) -> Result<Vec<SkillInstance>, CommandError> {
    let mut instances = visible_catalog_instances(
        catalog.list_skill_instances_for_project_context(ctx.project_root.as_deref())?,
    );
    apply_codex_config_overrides(ctx, &mut instances)?;
    Ok(instances)
}

fn projected_runtime_conflicts(
    instances: &[SkillInstance],
    ctx: &AdapterContext,
) -> Vec<ConflictGroupRecord> {
    let agent_by_instance_id = instances
        .iter()
        .map(|instance| (instance.id.as_str(), instance.agent.as_str()))
        .collect::<BTreeMap<_, _>>();
    let report = evaluate_mvp_rules(
        instances,
        &RuleContext {
            previous_fingerprints: std::collections::HashMap::new(),
            runtime_conflict_namespaces: Some(runtime_conflict_namespaces(instances, ctx)),
        },
    );
    let conflicts = report
        .conflicts
        .into_iter()
        .map(|conflict| ConflictGroupRecord {
            id: conflict.id,
            definition_id: conflict.definition_id,
            reason: conflict.reason,
            winner_id: conflict.winner_id,
            instance_ids: conflict.instances,
        })
        .collect();
    runtime_conflict_groups(conflicts, &agent_by_instance_id)
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
    let findings = user_visible_rule_findings(findings);
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

pub fn user_visible_rule_findings(findings: &[RuleFindingRecord]) -> Vec<RuleFindingRecord> {
    dedupe_rule_finding_records(findings)
        .into_iter()
        .filter(is_user_visible_rule_finding)
        .collect()
}

pub fn is_user_visible_rule_finding(finding: &RuleFindingRecord) -> bool {
    if finding.suppressed || finding.instance_id.is_none() {
        return false;
    }
    if !matches!(
        finding.triage_status.trim(),
        "" | "open" | "needs-follow-up"
    ) {
        return false;
    }

    let rule_id = finding.rule_id.trim().to_ascii_lowercase();
    if rule_id == "name.collision" {
        return false;
    }
    if matches!(
        rule_id.as_str(),
        "frontmatter.tools-not-empty"
            | "permissions.network-declared"
            | "permissions.exec-needs-human"
    ) {
        let severity = finding.effective_severity.trim().to_ascii_lowercase();
        return matches!(severity.as_str(), "critical" | "error");
    }
    true
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
