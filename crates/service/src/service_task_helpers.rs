use super::*;

pub(crate) fn task_readiness_safety_flags() -> TaskReadinessSafetyFlags {
    TaskReadinessSafetyFlags {
        read_only: true,
        provider_request_sent: false,
        write_back_allowed: false,
        script_execution_allowed: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        triage_mutation_allowed: false,
        credential_accessed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
    }
}

#[derive(Debug, Clone)]
pub(crate) struct SkillLifecycleSkillMeta {
    pub(crate) instance_id: String,
    pub(crate) definition_id: String,
    pub(crate) skill_name: String,
    pub(crate) agent: String,
    pub(crate) scope: String,
    pub(crate) enabled: bool,
    pub(crate) state: String,
    pub(crate) first_seen: i64,
    pub(crate) last_seen: i64,
    pub(crate) mtime: i64,
}

#[derive(Debug, Clone, Default)]
pub(crate) struct SkillLifecycleAggregateCounts {
    event_count: usize,
    finding_event_count: usize,
    drift_event_count: usize,
    remediation_event_count: usize,
    prompt_event_count: usize,
    session_review_event_count: usize,
    first_event_at: Option<i64>,
    latest_event_at: Option<i64>,
}

pub(crate) fn default_true() -> bool {
    true
}

pub(crate) fn skill_lifecycle_timeline_safety_flags() -> SkillLifecycleTimelineSafetyFlags {
    agent_readiness_safety_flags()
}

pub(crate) fn skill_lifecycle_filters(
    params: &SkillLifecycleTimelineParams,
    adapter_ctx: &AdapterContext,
    roots: &[(String, &'static str)],
) -> SkillLifecycleTimelineFilters {
    SkillLifecycleTimelineFilters {
        task: skill_lifecycle_filter_text(params.task.as_deref(), roots, 320),
        agent: skill_lifecycle_filter_token(params.agent.as_deref(), roots, 80),
        selected_skill_id: skill_lifecycle_filter_token(
            params.selected_skill_id.as_deref(),
            roots,
            120,
        ),
        selected_skill_name: skill_lifecycle_filter_text(
            params.selected_skill_name.as_deref(),
            roots,
            180,
        ),
        selected_skill_agent: skill_lifecycle_filter_token(
            params.selected_skill_agent.as_deref(),
            roots,
            80,
        ),
        definition_id: skill_lifecycle_filter_token(params.definition_id.as_deref(), roots, 160),
        project_root: params
            .project_root
            .as_deref()
            .map(|value| skill_lifecycle_filter_path_text(value, roots, 240))
            .or_else(|| {
                adapter_ctx.project_root.as_ref().map(|path| {
                    skill_lifecycle_filter_path_text(&path.to_string_lossy(), roots, 240)
                })
            }),
        current_cwd: params
            .current_cwd
            .as_deref()
            .map(|value| skill_lifecycle_filter_path_text(value, roots, 240))
            .or_else(|| {
                adapter_ctx.project_cwd.as_ref().map(|path| {
                    skill_lifecycle_filter_path_text(&path.to_string_lossy(), roots, 240)
                })
            }),
        workspace: skill_lifecycle_filter_text(params.workspace.as_deref(), roots, 240),
        limit: params.limit.unwrap_or(50).clamp(1, 500),
        include_prompt_runs: params.include_prompt_runs,
        include_session_reviews: params.include_session_reviews,
        include_remediation_history: params.include_remediation_history,
        include_stale_drift: params.include_stale_drift,
    }
}

pub(crate) fn skill_lifecycle_filter_text(
    value: Option<&str>,
    roots: &[(String, &'static str)],
    max_chars: usize,
) -> Option<String> {
    let value = value.map(str::trim).filter(|value| !value.is_empty())?;
    let mut redactor = PromptRedactor::new(roots);
    Some(truncate_chars(&redactor.redact(value), max_chars))
}

pub(crate) fn skill_lifecycle_filter_token(
    value: Option<&str>,
    roots: &[(String, &'static str)],
    max_chars: usize,
) -> Option<String> {
    skill_lifecycle_filter_text(value, roots, max_chars)
}

pub(crate) fn skill_lifecycle_filter_path_text(
    value: &str,
    roots: &[(String, &'static str)],
    max_chars: usize,
) -> String {
    let redacted = truncate_chars(&redact_string(value.trim(), roots), max_chars);
    if redacted.starts_with('/') || redacted.contains(":\\") {
        "<local-path>".to_string()
    } else {
        redacted
    }
}

pub(crate) fn skill_lifecycle_meta_from_instance(skill: SkillInstance) -> SkillLifecycleSkillMeta {
    SkillLifecycleSkillMeta {
        instance_id: skill.id,
        definition_id: skill.definition_id,
        skill_name: skill.name,
        agent: skill.agent.as_str().to_string(),
        scope: skill.scope.as_str().to_string(),
        enabled: skill.enabled,
        state: skill.state.as_str().to_string(),
        first_seen: skill.first_seen,
        last_seen: skill.last_seen,
        mtime: skill.mtime,
    }
}

pub(crate) fn empty_skill_lifecycle_timeline_result(
    filters: SkillLifecycleTimelineFilters,
    catalog_available: bool,
) -> SkillLifecycleTimelineResult {
    SkillLifecycleTimelineResult {
        generated_by: "local-v2.66",
        catalog_available,
        filters: filters.clone(),
        summary: SkillLifecycleTimelineSummary {
            summary:
                "No local catalog is available, so lifecycle timeline has no skill evidence."
                    .to_string(),
            ..SkillLifecycleTimelineSummary::default()
        },
        timeline_rows: Vec::new(),
        skill_rows: Vec::new(),
        agent_rows: Vec::new(),
        gap_notes: vec![
            "Run a local scan before relying on skill lifecycle timeline evidence.".to_string(),
        ],
        blocker_notes: vec![
            "No provider request was sent and no fallback network lookup was attempted.".to_string(),
        ],
        evidence_references: Vec::new(),
        prompt_request: AgentReadinessPromptRequest {
            available: false,
            preview_method: "llm.previewPrompt",
            confirm_method: "llm.confirmPromptAndSend",
            action: "skill_lifecycle_timeline",
            request: LlmPreviewPromptParams {
                action: LlmPromptActionKind::SkillLifecycleTimeline,
                profile_id: None,
                app_language: None,
                skill_instance_id: None,
                instance_ids: Vec::new(),
                agents: Vec::new(),
                analysis_kind: None,
                user_intent: Some(
                    "Explain deterministic skill lifecycle timeline rows using only local redacted evidence."
                        .to_string(),
                ),
            },
            note: "Prompt preview is unavailable until local catalog evidence produces lifecycle timeline rows."
                .to_string(),
        },
        safety_flags: skill_lifecycle_timeline_safety_flags(),
    }
}

pub(crate) fn skill_lifecycle_has_skill_filter(filters: &SkillLifecycleTimelineFilters) -> bool {
    filters.agent.is_some()
        || filters.selected_skill_id.is_some()
        || filters.selected_skill_name.is_some()
        || filters.selected_skill_agent.is_some()
        || filters.definition_id.is_some()
}

pub(crate) fn skill_lifecycle_visible_ids(
    filters: &SkillLifecycleTimelineFilters,
    skills: &[SkillLifecycleSkillMeta],
) -> BTreeSet<String> {
    skills
        .iter()
        .filter(|skill| skill_lifecycle_skill_matches(filters, skill))
        .map(|skill| skill.instance_id.clone())
        .collect()
}

pub(crate) fn skill_lifecycle_skill_matches(
    filters: &SkillLifecycleTimelineFilters,
    skill: &SkillLifecycleSkillMeta,
) -> bool {
    if !skill_lifecycle_optional_eq(filters.agent.as_deref(), &skill.agent) {
        return false;
    }
    if !skill_lifecycle_optional_eq(filters.selected_skill_agent.as_deref(), &skill.agent) {
        return false;
    }
    if !skill_lifecycle_optional_eq(filters.selected_skill_id.as_deref(), &skill.instance_id) {
        return false;
    }
    if !skill_lifecycle_optional_eq(filters.definition_id.as_deref(), &skill.definition_id) {
        return false;
    }
    if let Some(name) = filters.selected_skill_name.as_deref() {
        if !skill.skill_name.eq_ignore_ascii_case(name) {
            return false;
        }
    }
    true
}

pub(crate) fn skill_lifecycle_optional_eq(filter: Option<&str>, value: &str) -> bool {
    filter.is_none_or(|filter| value.eq_ignore_ascii_case(filter))
}

pub(crate) fn skill_lifecycle_relation_matches(
    filters: &SkillLifecycleTimelineFilters,
    instance_id: Option<&str>,
    definition_id: Option<&str>,
    agent: Option<&str>,
    skill_name: Option<&str>,
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
    visible_ids: &BTreeSet<String>,
) -> bool {
    let related = skill_lifecycle_related_ids(instance_id, definition_id, skill_by_id);
    if let Some(filter) = filters.selected_skill_id.as_deref() {
        if let Some(instance_id) = instance_id {
            if !instance_id.eq_ignore_ascii_case(filter) {
                return false;
            }
        } else if !related.iter().any(|id| id.eq_ignore_ascii_case(filter)) {
            return false;
        }
    }
    if let Some(filter) = filters.definition_id.as_deref() {
        let direct = definition_id.is_some_and(|id| id.eq_ignore_ascii_case(filter));
        let related_match = related
            .iter()
            .filter_map(|id| skill_by_id.get(id))
            .any(|skill| skill.definition_id.eq_ignore_ascii_case(filter));
        if !direct && !related_match {
            return false;
        }
    }
    if let Some(filter) = filters.selected_skill_name.as_deref() {
        let direct = skill_name.is_some_and(|name| name.eq_ignore_ascii_case(filter));
        let related_match = related
            .iter()
            .filter_map(|id| skill_by_id.get(id))
            .any(|skill| skill.skill_name.eq_ignore_ascii_case(filter));
        if !direct && !related_match {
            return false;
        }
    }
    for filter in [
        filters.agent.as_deref(),
        filters.selected_skill_agent.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        let direct = agent.is_some_and(|agent| agent.eq_ignore_ascii_case(filter));
        let related_match = related
            .iter()
            .filter_map(|id| skill_by_id.get(id))
            .any(|skill| skill.agent.eq_ignore_ascii_case(filter));
        if !direct && !related_match {
            return false;
        }
    }
    if skill_lifecycle_has_skill_filter(filters) {
        related.is_empty() || related.iter().any(|id| visible_ids.contains(id))
    } else {
        true
    }
}

pub(crate) fn skill_lifecycle_related_ids(
    instance_id: Option<&str>,
    definition_id: Option<&str>,
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
) -> BTreeSet<String> {
    let mut related = BTreeSet::new();
    if let Some(instance_id) = instance_id {
        if skill_by_id.contains_key(instance_id) {
            related.insert(instance_id.to_string());
        }
    }
    if let Some(definition_id) = definition_id {
        related.extend(
            skill_by_id
                .values()
                .filter(|skill| skill.definition_id == definition_id)
                .map(|skill| skill.instance_id.clone()),
        );
    }
    related
}

pub(crate) fn skill_lifecycle_text_contains_filter(
    value: Option<&str>,
    filter: Option<&str>,
) -> bool {
    match filter {
        Some(filter) => value
            .map(|value| {
                value
                    .to_ascii_lowercase()
                    .contains(&filter.to_ascii_lowercase())
            })
            .unwrap_or(false),
        None => true,
    }
}

pub(crate) fn skill_lifecycle_finding_matches(
    filters: &SkillLifecycleTimelineFilters,
    finding: &RuleFindingRecord,
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
) -> bool {
    let skills = skill_by_id.values().cloned().collect::<Vec<_>>();
    let visible_ids = skill_lifecycle_visible_ids(filters, &skills);
    skill_lifecycle_relation_matches(
        filters,
        finding.instance_id.as_deref(),
        finding.definition_id.as_deref(),
        finding
            .instance_id
            .as_deref()
            .and_then(|id| skill_by_id.get(id))
            .map(|skill| skill.agent.as_str()),
        finding
            .instance_id
            .as_deref()
            .and_then(|id| skill_by_id.get(id))
            .map(|skill| skill.skill_name.as_str()),
        skill_by_id,
        &visible_ids,
    )
}

pub(crate) fn skill_lifecycle_finding_skill<'a>(
    finding: &RuleFindingRecord,
    skill_by_id: &'a BTreeMap<String, SkillLifecycleSkillMeta>,
) -> Option<&'a SkillLifecycleSkillMeta> {
    finding
        .instance_id
        .as_deref()
        .and_then(|id| skill_by_id.get(id))
        .or_else(|| {
            finding.definition_id.as_deref().and_then(|definition_id| {
                skill_by_id
                    .values()
                    .find(|skill| skill.definition_id == definition_id)
            })
        })
}

pub(crate) fn skill_lifecycle_conflict_matches(
    filters: &SkillLifecycleTimelineFilters,
    conflict: &ConflictGroupRecord,
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
) -> bool {
    let skills = skill_by_id.values().cloned().collect::<Vec<_>>();
    let visible_ids = skill_lifecycle_visible_ids(filters, &skills);
    conflict
        .instance_ids
        .iter()
        .any(|id| visible_ids.contains(id))
        || skill_lifecycle_relation_matches(
            filters,
            conflict.winner_id.as_deref(),
            Some(conflict.definition_id.as_str()),
            None,
            None,
            skill_by_id,
            &visible_ids,
        )
}

pub(crate) fn skill_lifecycle_conflict_skill<'a>(
    conflict: &ConflictGroupRecord,
    skill_by_id: &'a BTreeMap<String, SkillLifecycleSkillMeta>,
    visible_ids: &BTreeSet<String>,
) -> Option<&'a SkillLifecycleSkillMeta> {
    conflict
        .winner_id
        .as_deref()
        .and_then(|id| skill_by_id.get(id))
        .or_else(|| {
            conflict
                .instance_ids
                .iter()
                .find(|id| visible_ids.contains(*id))
                .and_then(|id| skill_by_id.get(id))
        })
        .or_else(|| {
            conflict
                .instance_ids
                .iter()
                .find_map(|id| skill_by_id.get(id))
        })
}

pub(crate) fn skill_lifecycle_analysis_matches(
    filters: &SkillLifecycleTimelineFilters,
    group: &CrossAgentAnalysisGroup,
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
    visible_ids: &BTreeSet<String>,
) -> bool {
    let agent_match = filters.agent.as_deref().is_none_or(|agent| {
        group
            .agents
            .iter()
            .any(|group_agent| group_agent.eq_ignore_ascii_case(agent))
    });
    let selected_agent_match = filters.selected_skill_agent.as_deref().is_none_or(|agent| {
        group
            .agents
            .iter()
            .any(|group_agent| group_agent.eq_ignore_ascii_case(agent))
    });
    let name_match = filters.selected_skill_name.as_deref().is_none_or(|name| {
        group
            .canonical_name
            .as_deref()
            .is_some_and(|canonical| canonical.eq_ignore_ascii_case(name))
            || group.instance_ids.iter().any(|id| {
                skill_by_id
                    .get(id)
                    .is_some_and(|skill| skill.skill_name.eq_ignore_ascii_case(name))
            })
    });
    let definition_match = filters
        .definition_id
        .as_deref()
        .is_none_or(|definition_id| {
            group.instance_ids.iter().any(|id| {
                skill_by_id
                    .get(id)
                    .is_some_and(|skill| skill.definition_id.eq_ignore_ascii_case(definition_id))
            })
        });
    let selected_id_match = filters
        .selected_skill_id
        .as_deref()
        .is_none_or(|selected| group.instance_ids.iter().any(|id| id == selected));
    let visible_match = !skill_lifecycle_has_skill_filter(filters)
        || group.instance_ids.iter().any(|id| visible_ids.contains(id));
    agent_match
        && selected_agent_match
        && name_match
        && definition_match
        && selected_id_match
        && visible_match
}

pub(crate) fn skill_lifecycle_analysis_skill<'a>(
    group: &CrossAgentAnalysisGroup,
    skill_by_id: &'a BTreeMap<String, SkillLifecycleSkillMeta>,
    visible_ids: &BTreeSet<String>,
) -> Option<&'a SkillLifecycleSkillMeta> {
    group
        .instance_ids
        .iter()
        .find(|id| visible_ids.contains(*id))
        .and_then(|id| skill_by_id.get(id))
        .or_else(|| group.instance_ids.iter().find_map(|id| skill_by_id.get(id)))
}

pub(crate) fn skill_lifecycle_stale_row_matches(
    filters: &SkillLifecycleTimelineFilters,
    row: &StaleDriftRow,
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
    visible_ids: &BTreeSet<String>,
) -> bool {
    skill_lifecycle_relation_matches(
        filters,
        Some(row.instance_id.as_str()),
        Some(row.definition_id.as_str()),
        Some(row.agent.as_str()),
        Some(row.skill_name.as_str()),
        skill_by_id,
        visible_ids,
    )
}

pub(crate) fn skill_lifecycle_remediation_history_matches(
    filters: &SkillLifecycleTimelineFilters,
    record: &RemediationHistoryRecord,
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
    visible_ids: &BTreeSet<String>,
) -> bool {
    if !skill_lifecycle_text_contains_filter(record.task.as_deref(), filters.task.as_deref()) {
        return false;
    }
    if let Some(workspace) = filters.workspace.as_deref() {
        if !record
            .workspace
            .as_deref()
            .is_some_and(|value| value.contains(workspace))
        {
            return false;
        }
    }
    if let Some(agent) = filters
        .selected_skill_agent
        .as_deref()
        .or(filters.agent.as_deref())
    {
        if !record
            .agent
            .as_deref()
            .is_some_and(|value| value.eq_ignore_ascii_case(agent))
        {
            return false;
        }
    }
    if !skill_lifecycle_has_skill_filter(filters) {
        return true;
    }
    skill_lifecycle_strings_match_filters(
        filters,
        record
            .source_item_refs
            .iter()
            .chain(record.batch_review_item_ids.iter())
            .chain(record.evidence_refs.iter()),
        skill_by_id,
        visible_ids,
    )
}

pub(crate) fn skill_lifecycle_prompt_run_matches(
    filters: &SkillLifecycleTimelineFilters,
    run: &LlmPromptRunRecord,
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
    visible_ids: &BTreeSet<String>,
) -> bool {
    if !skill_lifecycle_text_contains_filter(run.task.as_deref(), filters.task.as_deref()) {
        return false;
    }
    let instance_id = run.instance_id.as_deref().or_else(|| {
        run.instance_ids
            .iter()
            .find(|id| skill_by_id.contains_key(*id))
            .map(String::as_str)
    });
    skill_lifecycle_relation_matches(
        filters,
        instance_id,
        run.definition_id.as_deref(),
        run.agent.as_deref(),
        None,
        skill_by_id,
        visible_ids,
    )
}

pub(crate) fn skill_lifecycle_session_review_matches(
    filters: &SkillLifecycleTimelineFilters,
    review: &AgentSessionSkillReviewRecord,
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
    visible_ids: &BTreeSet<String>,
) -> bool {
    if !skill_lifecycle_text_contains_filter(review.task.as_deref(), filters.task.as_deref()) {
        return false;
    }
    for filter in [
        filters.agent.as_deref(),
        filters.selected_skill_agent.as_deref(),
    ]
    .into_iter()
    .flatten()
    {
        let review_agent = review
            .agent
            .as_deref()
            .is_some_and(|agent| agent.eq_ignore_ascii_case(filter));
        let detected_agent = review
            .analysis
            .detected_skills
            .iter()
            .any(|skill| skill.agent.eq_ignore_ascii_case(filter));
        if !review_agent && !detected_agent {
            return false;
        }
    }
    if !skill_lifecycle_has_skill_filter(filters) {
        return true;
    }
    let detected_match = review.analysis.detected_skills.iter().any(|skill| {
        skill_lifecycle_relation_matches(
            filters,
            Some(skill.instance_id.as_str()),
            Some(skill.definition_id.as_str()),
            Some(skill.agent.as_str()),
            Some(skill.skill_name.as_str()),
            skill_by_id,
            visible_ids,
        )
    });
    let expected_match = skill_lifecycle_strings_match_filters(
        filters,
        review
            .expected_skill_refs
            .iter()
            .chain(review.expected_skill_names.iter())
            .chain(review.analysis.evidence_refs.iter()),
        skill_by_id,
        visible_ids,
    );
    detected_match || expected_match
}

pub(crate) fn skill_lifecycle_strings_match_filters<'a>(
    filters: &SkillLifecycleTimelineFilters,
    values: impl Iterator<Item = &'a String>,
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
    visible_ids: &BTreeSet<String>,
) -> bool {
    let values = values.collect::<Vec<_>>();
    if let Some(filter) = filters.selected_skill_id.as_deref() {
        if !values.iter().any(|value| value.contains(filter)) {
            return false;
        }
    }
    if let Some(filter) = filters.definition_id.as_deref() {
        let value_match = values.iter().any(|value| value.contains(filter));
        let skill_match = visible_ids.iter().any(|id| {
            skill_by_id
                .get(id)
                .is_some_and(|skill| skill.definition_id.eq_ignore_ascii_case(filter))
        });
        if !value_match && !skill_match {
            return false;
        }
    }
    if let Some(filter) = filters.selected_skill_name.as_deref() {
        let value_match = values
            .iter()
            .any(|value| value.eq_ignore_ascii_case(filter) || value.contains(filter));
        let skill_match = visible_ids.iter().any(|id| {
            skill_by_id
                .get(id)
                .is_some_and(|skill| skill.skill_name.eq_ignore_ascii_case(filter))
        });
        if !value_match && !skill_match {
            return false;
        }
    }
    true
}

pub(crate) fn skill_lifecycle_related_instance_for_strings<'a>(
    values: impl Iterator<Item = &'a String>,
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
) -> Option<String> {
    let values = values.collect::<Vec<_>>();
    skill_by_id.values().find_map(|skill| {
        if values.iter().any(|value| {
            value.contains(&skill.instance_id)
                || value.contains(&skill.definition_id)
                || value.eq_ignore_ascii_case(&skill.skill_name)
        }) {
            Some(skill.instance_id.clone())
        } else {
            None
        }
    })
}

pub(crate) fn skill_lifecycle_skill_seen_row(
    skill: &SkillLifecycleSkillMeta,
    evidence_id: &str,
) -> SkillLifecycleTimelineRow {
    SkillLifecycleTimelineRow {
        id: format!("lifecycle:skill-seen:{}", skill.instance_id),
        occurred_at: Some(skill.first_seen),
        event_type: "skill_seen",
        lifecycle_stage: "discovery",
        title: format!(
            "Skill discovered: {}",
            redact_for_llm_preview(&skill.skill_name)
        ),
        summary: format!(
            "`{}` is a {} {} skill for {} and is currently {}.",
            redact_for_llm_preview(&skill.skill_name),
            redact_for_llm_preview(&skill.scope),
            redact_for_llm_preview(&skill.state),
            redact_for_llm_preview(&skill.agent),
            if skill.enabled { "enabled" } else { "disabled" }
        ),
        agent: Some(skill.agent.clone()),
        skill_name: Some(redact_for_llm_preview(&skill.skill_name)),
        instance_id: Some(skill.instance_id.clone()),
        definition_id: Some(skill.definition_id.clone()),
        source: "catalog",
        severity: None,
        status: Some(skill.state.clone()),
        evidence_refs: vec![evidence_id.to_string()],
        safety_flags: skill_lifecycle_timeline_safety_flags(),
    }
}

pub(crate) fn skill_lifecycle_skill_observed_row(
    skill: &SkillLifecycleSkillMeta,
    evidence_id: &str,
) -> SkillLifecycleTimelineRow {
    SkillLifecycleTimelineRow {
        id: format!("lifecycle:skill-observed:{}", skill.instance_id),
        occurred_at: Some(skill.last_seen),
        event_type: "skill_observed",
        lifecycle_stage: "scan",
        title: format!(
            "Skill observed: {}",
            redact_for_llm_preview(&skill.skill_name)
        ),
        summary: format!(
            "Latest local catalog observation recorded state `{}` with source mtime {}.",
            redact_for_llm_preview(&skill.state),
            skill.mtime
        ),
        agent: Some(skill.agent.clone()),
        skill_name: Some(redact_for_llm_preview(&skill.skill_name)),
        instance_id: Some(skill.instance_id.clone()),
        definition_id: Some(skill.definition_id.clone()),
        source: "catalog",
        severity: None,
        status: Some(skill.state.clone()),
        evidence_refs: vec![evidence_id.to_string()],
        safety_flags: skill_lifecycle_timeline_safety_flags(),
    }
}

pub(crate) fn skill_lifecycle_finding_row(
    finding: &RuleFindingRecord,
    skill: Option<&SkillLifecycleSkillMeta>,
    evidence_id: &str,
) -> SkillLifecycleTimelineRow {
    SkillLifecycleTimelineRow {
        id: format!("lifecycle:finding:{}", finding.id),
        occurred_at: Some(finding.created_at),
        event_type: "finding",
        lifecycle_stage: "risk",
        title: format!(
            "{} finding: {}",
            redact_for_llm_preview(&finding.rule_id),
            skill
                .map(|skill| redact_for_llm_preview(&skill.skill_name))
                .unwrap_or_else(|| "catalog definition".to_string())
        ),
        summary: redact_for_llm_preview(&finding.message),
        agent: skill.map(|skill| skill.agent.clone()),
        skill_name: skill.map(|skill| redact_for_llm_preview(&skill.skill_name)),
        instance_id: finding
            .instance_id
            .clone()
            .or_else(|| skill.map(|skill| skill.instance_id.clone())),
        definition_id: finding
            .definition_id
            .clone()
            .or_else(|| skill.map(|skill| skill.definition_id.clone())),
        source: "catalog:finding",
        severity: Some(finding.effective_severity.clone()),
        status: Some(finding.triage_status.clone()),
        evidence_refs: vec![evidence_id.to_string()],
        safety_flags: skill_lifecycle_timeline_safety_flags(),
    }
}

pub(crate) fn skill_lifecycle_finding_triage_row(
    finding: &RuleFindingRecord,
    skill: Option<&SkillLifecycleSkillMeta>,
    updated_at: i64,
    evidence_id: &str,
) -> SkillLifecycleTimelineRow {
    SkillLifecycleTimelineRow {
        id: format!("lifecycle:finding-triage:{}", finding.id),
        occurred_at: Some(updated_at),
        event_type: "finding_triage",
        lifecycle_stage: "triage",
        title: format!(
            "Finding triage: {}",
            redact_for_llm_preview(&finding.rule_id)
        ),
        summary: format!(
            "Finding triage status is `{}`{}.",
            redact_for_llm_preview(&finding.triage_status),
            finding
                .triage_note
                .as_deref()
                .map(|note| format!(" with note `{}`", redact_for_llm_preview(note)))
                .unwrap_or_default()
        ),
        agent: skill.map(|skill| skill.agent.clone()),
        skill_name: skill.map(|skill| redact_for_llm_preview(&skill.skill_name)),
        instance_id: finding
            .instance_id
            .clone()
            .or_else(|| skill.map(|skill| skill.instance_id.clone())),
        definition_id: finding
            .definition_id
            .clone()
            .or_else(|| skill.map(|skill| skill.definition_id.clone())),
        source: "catalog:finding-triage",
        severity: Some(finding.effective_severity.clone()),
        status: Some(finding.triage_status.clone()),
        evidence_refs: vec![evidence_id.to_string()],
        safety_flags: skill_lifecycle_timeline_safety_flags(),
    }
}

pub(crate) fn skill_lifecycle_conflict_row(
    conflict: &ConflictGroupRecord,
    skill: Option<&SkillLifecycleSkillMeta>,
    evidence_id: &str,
) -> SkillLifecycleTimelineRow {
    SkillLifecycleTimelineRow {
        id: format!("lifecycle:conflict:{}", conflict.id),
        occurred_at: None,
        event_type: "conflict",
        lifecycle_stage: "conflict",
        title: format!("Conflict: {}", redact_for_llm_preview(&conflict.reason)),
        summary: format!(
            "{} instance(s) share definition `{}`; winner is {}.",
            conflict.instance_ids.len(),
            redact_for_llm_preview(&conflict.definition_id),
            conflict
                .winner_id
                .as_deref()
                .map(redact_for_llm_preview)
                .unwrap_or_else(|| "not selected".to_string())
        ),
        agent: skill.map(|skill| skill.agent.clone()),
        skill_name: skill.map(|skill| redact_for_llm_preview(&skill.skill_name)),
        instance_id: skill
            .map(|skill| skill.instance_id.clone())
            .or_else(|| conflict.winner_id.clone()),
        definition_id: Some(conflict.definition_id.clone()),
        source: "catalog:conflict",
        severity: Some("warning".to_string()),
        status: Some(conflict.reason.clone()),
        evidence_refs: vec![evidence_id.to_string()],
        safety_flags: skill_lifecycle_timeline_safety_flags(),
    }
}

pub(crate) fn skill_lifecycle_analysis_row(
    group: &CrossAgentAnalysisGroup,
    skill: Option<&SkillLifecycleSkillMeta>,
    evidence_id: &str,
) -> SkillLifecycleTimelineRow {
    SkillLifecycleTimelineRow {
        id: format!("lifecycle:analysis:{}", group.id),
        occurred_at: None,
        event_type: "analysis",
        lifecycle_stage: "analysis",
        title: redact_for_llm_preview(&group.title),
        summary: redact_for_llm_preview(&group.explanation),
        agent: skill
            .map(|skill| skill.agent.clone())
            .or_else(|| group.agents.first().cloned()),
        skill_name: skill
            .map(|skill| redact_for_llm_preview(&skill.skill_name))
            .or_else(|| group.canonical_name.as_deref().map(redact_for_llm_preview)),
        instance_id: skill
            .map(|skill| skill.instance_id.clone())
            .or_else(|| group.instance_ids.first().cloned()),
        definition_id: skill.map(|skill| skill.definition_id.clone()),
        source: "catalog:analysis",
        severity: Some(group.severity.clone()),
        status: Some(group.kind.clone()),
        evidence_refs: vec![evidence_id.to_string()],
        safety_flags: skill_lifecycle_timeline_safety_flags(),
    }
}

pub(crate) fn skill_lifecycle_stale_drift_row(row: &StaleDriftRow) -> SkillLifecycleTimelineRow {
    SkillLifecycleTimelineRow {
        id: format!("lifecycle:stale-drift:{}", row.instance_id),
        occurred_at: None,
        event_type: "stale_drift",
        lifecycle_stage: "drift",
        title: format!(
            "Stale/drift signal: {}",
            redact_for_llm_preview(&row.skill_name)
        ),
        summary: format!(
            "Stale/drift score {} ({}) with {} local reason(s).",
            row.stale_drift_score,
            row.stale_drift_band,
            row.reasons.len()
        ),
        agent: Some(row.agent.clone()),
        skill_name: Some(redact_for_llm_preview(&row.skill_name)),
        instance_id: Some(row.instance_id.clone()),
        definition_id: Some(row.definition_id.clone()),
        source: "analysis.detectStaleDrift",
        severity: Some(row.stale_drift_band.to_string()),
        status: row
            .readiness_impact
            .as_ref()
            .map(|impact| impact.impact_level.to_string()),
        evidence_refs: row.evidence_refs.clone(),
        safety_flags: skill_lifecycle_timeline_safety_flags(),
    }
}

pub(crate) fn skill_lifecycle_remediation_history_row(
    record: &RemediationHistoryRecord,
    skill: Option<&SkillLifecycleSkillMeta>,
    evidence_id: &str,
) -> SkillLifecycleTimelineRow {
    SkillLifecycleTimelineRow {
        id: format!("lifecycle:remediation-history:{}", record.id),
        occurred_at: Some(record.updated_at),
        event_type: "remediation_history",
        lifecycle_stage: "remediation",
        title: redact_for_llm_preview(&record.title),
        summary: format!(
            "Decision `{}` has status `{}`; {} evidence ref(s).",
            redact_for_llm_preview(&record.decision),
            redact_for_llm_preview(&record.status),
            record.evidence_refs.len()
        ),
        agent: record
            .agent
            .clone()
            .or_else(|| skill.map(|skill| skill.agent.clone())),
        skill_name: skill.map(|skill| redact_for_llm_preview(&skill.skill_name)),
        instance_id: skill.map(|skill| skill.instance_id.clone()),
        definition_id: skill.map(|skill| skill.definition_id.clone()),
        source: "app-local:remediation-history.json",
        severity: record.risk_levels.first().cloned(),
        status: Some(record.status.clone()),
        evidence_refs: vec![evidence_id.to_string()],
        safety_flags: skill_lifecycle_timeline_safety_flags(),
    }
}

pub(crate) fn skill_lifecycle_prompt_run_row(
    run: &LlmPromptRunRecord,
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
    evidence_id: &str,
) -> SkillLifecycleTimelineRow {
    let skill = run
        .instance_id
        .as_deref()
        .and_then(|id| skill_by_id.get(id))
        .or_else(|| run.instance_ids.iter().find_map(|id| skill_by_id.get(id)));
    SkillLifecycleTimelineRow {
        id: format!("lifecycle:prompt-run:{}", run.id),
        occurred_at: Some(run.completed_at),
        event_type: "prompt_run",
        lifecycle_stage: "provider_preview",
        title: format!("Prompt run: {}", redact_for_llm_preview(&run.action)),
        summary: format!(
            "Prompt run metadata status `{}` for request `{}`; draft copy-only={}.",
            redact_for_llm_preview(&run.status),
            redact_for_llm_preview(&run.request_kind),
            run.draft_requires_user_copy
        ),
        agent: run
            .agent
            .clone()
            .or_else(|| skill.map(|skill| skill.agent.clone())),
        skill_name: skill.map(|skill| redact_for_llm_preview(&skill.skill_name)),
        instance_id: run
            .instance_id
            .clone()
            .or_else(|| skill.map(|skill| skill.instance_id.clone())),
        definition_id: run
            .definition_id
            .clone()
            .or_else(|| skill.map(|skill| skill.definition_id.clone())),
        source: "app-local:prompt-runs.json",
        severity: run.error_code.clone(),
        status: Some(run.status.clone()),
        evidence_refs: vec![evidence_id.to_string()],
        safety_flags: skill_lifecycle_timeline_safety_flags(),
    }
}

pub(crate) fn skill_lifecycle_session_review_row(
    review: &AgentSessionSkillReviewRecord,
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
    evidence_id: &str,
) -> SkillLifecycleTimelineRow {
    let detected = review
        .analysis
        .detected_skills
        .iter()
        .find(|skill| skill_by_id.contains_key(&skill.instance_id));
    SkillLifecycleTimelineRow {
        id: format!("lifecycle:session-review:{}", review.id),
        occurred_at: Some(review.reviewed_at),
        event_type: "session_review",
        lifecycle_stage: "session_review",
        title: redact_for_llm_preview(&review.title),
        summary: redact_for_llm_preview(&review.analysis.summary),
        agent: review
            .agent
            .clone()
            .or_else(|| detected.map(|skill| skill.agent.clone())),
        skill_name: detected.map(|skill| redact_for_llm_preview(&skill.skill_name)),
        instance_id: detected.map(|skill| skill.instance_id.clone()),
        definition_id: detected.map(|skill| skill.definition_id.clone()),
        source: "app-local:agent-session-reviews.json",
        severity: Some(review.analysis.outcome.clone()),
        status: Some(review.analysis.outcome.clone()),
        evidence_refs: vec![evidence_id.to_string()],
        safety_flags: skill_lifecycle_timeline_safety_flags(),
    }
}

pub(crate) fn skill_lifecycle_row_sort(
    left: &SkillLifecycleTimelineRow,
    right: &SkillLifecycleTimelineRow,
) -> std::cmp::Ordering {
    right
        .occurred_at
        .unwrap_or(i64::MIN)
        .cmp(&left.occurred_at.unwrap_or(i64::MIN))
        .then_with(|| left.event_type.cmp(right.event_type))
        .then_with(|| left.title.cmp(&right.title))
        .then_with(|| left.id.cmp(&right.id))
}

pub(crate) fn skill_lifecycle_count_row(
    counts: &mut SkillLifecycleAggregateCounts,
    row: &SkillLifecycleTimelineRow,
) {
    counts.event_count += 1;
    match row.event_type {
        "finding" | "finding_triage" => counts.finding_event_count += 1,
        "stale_drift" => counts.drift_event_count += 1,
        "remediation_history" => counts.remediation_event_count += 1,
        "prompt_run" => counts.prompt_event_count += 1,
        "session_review" => counts.session_review_event_count += 1,
        _ => {}
    }
    if let Some(occurred_at) = row.occurred_at {
        counts.first_event_at = Some(
            counts
                .first_event_at
                .map(|current| current.min(occurred_at))
                .unwrap_or(occurred_at),
        );
        counts.latest_event_at = Some(
            counts
                .latest_event_at
                .map(|current| current.max(occurred_at))
                .unwrap_or(occurred_at),
        );
    }
}

pub(crate) fn skill_lifecycle_skill_rows(
    rows: &[SkillLifecycleTimelineRow],
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
) -> Vec<SkillLifecycleSkillRow> {
    let mut counts_by_skill =
        BTreeMap::<String, (SkillLifecycleAggregateCounts, BTreeSet<String>)>::new();
    for row in rows {
        let Some(instance_id) = row.instance_id.as_deref() else {
            continue;
        };
        let Some(skill) = skill_by_id.get(instance_id) else {
            continue;
        };
        let entry = counts_by_skill
            .entry(skill.instance_id.clone())
            .or_insert_with(|| (SkillLifecycleAggregateCounts::default(), BTreeSet::new()));
        skill_lifecycle_count_row(&mut entry.0, row);
        entry.1.extend(row.evidence_refs.iter().cloned());
    }
    let mut skill_rows = counts_by_skill
        .into_iter()
        .filter_map(|(instance_id, (counts, evidence_refs))| {
            let skill = skill_by_id.get(&instance_id)?;
            Some(SkillLifecycleSkillRow {
                instance_id: skill.instance_id.clone(),
                definition_id: skill.definition_id.clone(),
                skill_name: redact_for_llm_preview(&skill.skill_name),
                agent: skill.agent.clone(),
                scope: skill.scope.clone(),
                enabled: skill.enabled,
                state: skill.state.clone(),
                event_count: counts.event_count,
                finding_event_count: counts.finding_event_count,
                drift_event_count: counts.drift_event_count,
                remediation_event_count: counts.remediation_event_count,
                prompt_event_count: counts.prompt_event_count,
                session_review_event_count: counts.session_review_event_count,
                first_event_at: counts.first_event_at,
                latest_event_at: counts.latest_event_at,
                evidence_refs: evidence_refs.into_iter().collect(),
                safety_flags: skill_lifecycle_timeline_safety_flags(),
            })
        })
        .collect::<Vec<_>>();
    skill_rows.sort_by(|left, right| {
        right
            .event_count
            .cmp(&left.event_count)
            .then_with(|| left.agent.cmp(&right.agent))
            .then_with(|| left.skill_name.cmp(&right.skill_name))
            .then_with(|| left.instance_id.cmp(&right.instance_id))
    });
    skill_rows
}

pub(crate) fn skill_lifecycle_agent_rows(
    rows: &[SkillLifecycleTimelineRow],
    skill_by_id: &BTreeMap<String, SkillLifecycleSkillMeta>,
) -> Vec<SkillLifecycleAgentRow> {
    let mut counts_by_agent = BTreeMap::<
        String,
        (
            SkillLifecycleAggregateCounts,
            BTreeSet<String>,
            BTreeSet<String>,
        ),
    >::new();
    for row in rows {
        let agent = row.agent.clone().or_else(|| {
            row.instance_id
                .as_deref()
                .and_then(|id| skill_by_id.get(id))
                .map(|skill| skill.agent.clone())
        });
        let Some(agent) = agent else {
            continue;
        };
        let entry = counts_by_agent.entry(agent).or_insert_with(|| {
            (
                SkillLifecycleAggregateCounts::default(),
                BTreeSet::new(),
                BTreeSet::new(),
            )
        });
        skill_lifecycle_count_row(&mut entry.0, row);
        if let Some(instance_id) = row.instance_id.as_deref() {
            entry.1.insert(instance_id.to_string());
        }
        entry.2.extend(row.evidence_refs.iter().cloned());
    }
    counts_by_agent
        .into_iter()
        .map(
            |(agent, (counts, skill_ids, evidence_refs))| SkillLifecycleAgentRow {
                agent,
                skill_count: skill_ids.len(),
                event_count: counts.event_count,
                finding_event_count: counts.finding_event_count,
                drift_event_count: counts.drift_event_count,
                remediation_event_count: counts.remediation_event_count,
                prompt_event_count: counts.prompt_event_count,
                session_review_event_count: counts.session_review_event_count,
                first_event_at: counts.first_event_at,
                latest_event_at: counts.latest_event_at,
                evidence_refs: evidence_refs.into_iter().collect(),
                safety_flags: skill_lifecycle_timeline_safety_flags(),
            },
        )
        .collect()
}

pub(crate) fn skill_lifecycle_summary(
    rows: &[SkillLifecycleTimelineRow],
    skill_rows: &[SkillLifecycleSkillRow],
    agent_rows: &[SkillLifecycleAgentRow],
    filters: &SkillLifecycleTimelineFilters,
) -> SkillLifecycleTimelineSummary {
    let mut counts = SkillLifecycleAggregateCounts::default();
    for row in rows {
        skill_lifecycle_count_row(&mut counts, row);
    }
    let selected_skill_name = filters
        .selected_skill_name
        .clone()
        .or_else(|| skill_rows.first().map(|row| row.skill_name.clone()));
    let selected_agent = filters
        .selected_skill_agent
        .clone()
        .or_else(|| filters.agent.clone())
        .or_else(|| agent_rows.first().map(|row| row.agent.clone()));
    let summary = if rows.is_empty() {
        "No lifecycle events matched the current local filters.".to_string()
    } else {
        format!(
            "Lifecycle timeline returned {} event(s) across {} skill(s) and {} agent(s) using deterministic local evidence only.",
            rows.len(),
            skill_rows.len(),
            agent_rows.len()
        )
    };
    SkillLifecycleTimelineSummary {
        total_event_count: counts.event_count,
        skill_count: skill_rows.len(),
        agent_count: agent_rows.len(),
        finding_event_count: counts.finding_event_count,
        drift_event_count: counts.drift_event_count,
        remediation_event_count: counts.remediation_event_count,
        prompt_event_count: counts.prompt_event_count,
        session_review_event_count: counts.session_review_event_count,
        first_event_at: counts.first_event_at,
        latest_event_at: counts.latest_event_at,
        selected_skill_name,
        selected_agent,
        summary,
    }
}

pub(crate) fn task_cockpit_safety_flags() -> TaskCockpitSafetyFlags {
    AgentReadinessSafetyFlags {
        read_only: true,
        app_local_only: true,
        provider_request_sent: false,
        write_back_allowed: false,
        write_actions_available: false,
        skill_files_mutated: false,
        agent_config_mutated: false,
        script_execution_allowed: false,
        execution_actions_available: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        triage_mutation_allowed: false,
        credential_accessed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        cloud_sync_performed: false,
        telemetry_emitted: false,
    }
}

pub(crate) fn task_cockpit_budget_reached(started_at: Instant, budget: Duration) -> bool {
    started_at.elapsed() >= budget
}

pub(crate) fn task_cockpit_elapsed_ms(started_at: Instant) -> u64 {
    started_at.elapsed().as_millis().min(u128::from(u64::MAX)) as u64
}

pub(crate) fn push_unique_stage(stages: &mut Vec<&'static str>, stage: &'static str) {
    if !stages.contains(&stage) {
        stages.push(stage);
    }
}

pub(crate) fn normalize_aggregation_stages(stages: &mut Vec<&'static str>) {
    let mut seen = BTreeSet::new();
    stages.retain(|stage| seen.insert(*stage));
}
pub(crate) struct AggregationRuntimeInput {
    pub(crate) started_at: Instant,
    pub(crate) timeout_ms: u64,
    pub(crate) limit: usize,
    pub(crate) scanned_count: usize,
    pub(crate) total_count: usize,
    pub(crate) completed_stages: Vec<&'static str>,
    pub(crate) skipped_stages: Vec<&'static str>,
    pub(crate) blocker_codes: Vec<&'static str>,
    pub(crate) fallback_used: bool,
    pub(crate) notes: Vec<String>,
}

pub(crate) fn aggregation_runtime_metadata(
    input: AggregationRuntimeInput,
) -> AggregationRuntimeMetadata {
    let AggregationRuntimeInput {
        started_at,
        timeout_ms,
        limit,
        scanned_count,
        total_count,
        mut completed_stages,
        mut skipped_stages,
        mut blocker_codes,
        fallback_used,
        notes,
    } = input;
    normalize_aggregation_stages(&mut completed_stages);
    normalize_aggregation_stages(&mut skipped_stages);
    normalize_aggregation_stages(&mut blocker_codes);
    let elapsed_ms = task_cockpit_elapsed_ms(started_at);
    let timed_out = elapsed_ms >= timeout_ms;
    if timed_out {
        push_unique_stage(&mut blocker_codes, "aggregation-timeout");
    }
    let partial = timed_out || fallback_used || !skipped_stages.is_empty();
    AggregationRuntimeMetadata {
        status: if partial { "partial" } else { "complete" },
        elapsed_ms,
        timeout_ms,
        timed_out,
        partial,
        fallback_used,
        limit,
        scanned_count,
        total_count,
        completed_stages,
        skipped_stages,
        blocker_codes,
        notes,
    }
}

pub(crate) fn empty_aggregation_runtime(
    timeout_ms: u64,
    limit: usize,
    completed_stage: &'static str,
    note: impl Into<String>,
) -> AggregationRuntimeMetadata {
    AggregationRuntimeMetadata {
        status: "complete",
        elapsed_ms: 0,
        timeout_ms,
        timed_out: false,
        partial: false,
        fallback_used: false,
        limit,
        scanned_count: 0,
        total_count: 0,
        completed_stages: vec![completed_stage],
        skipped_stages: Vec::new(),
        blocker_codes: Vec::new(),
        notes: vec![note.into()],
    }
}

pub(crate) fn aggregation_with_completed_stage(
    mut aggregation: AggregationRuntimeMetadata,
    stage: &'static str,
) -> AggregationRuntimeMetadata {
    push_unique_stage(&mut aggregation.completed_stages, stage);
    aggregation
}
pub(crate) struct TaskCockpitSummaryCounts {
    pub(crate) session_review_count: usize,
    pub(crate) provider_observability_row_count: usize,
    pub(crate) remediation_next_step_count: usize,
    pub(crate) gap_count: usize,
    pub(crate) blocker_count: usize,
}

pub(crate) fn task_cockpit_summary(
    readiness: &TaskReadinessResult,
    ranking: &SkillRouteRankingResult,
    comparison: &AgentReadinessComparisonResult,
    counts: TaskCockpitSummaryCounts,
) -> TaskCockpitSummary {
    let recommended_agent = comparison
        .recommended_agent
        .as_ref()
        .map(|recommendation| recommendation.agent.clone());
    let top_skill_name = ranking
        .route_candidates
        .first()
        .map(|candidate| candidate.skill_name.clone());
    let summary = if !readiness.catalog_available {
        "Task cockpit has no local catalog evidence yet; scan before using it for routing decisions."
            .to_string()
    } else {
        format!(
            "Task cockpit combined readiness {}, routing {}, {} candidate(s), {} agent row(s), {} session review row(s), {} provider observability row(s), and {} remediation next step(s).",
            readiness.band,
            ranking.overall_confidence_band,
            readiness.candidate_skills.len(),
            comparison.agent_rows.len(),
            counts.session_review_count,
            counts.provider_observability_row_count,
            counts.remediation_next_step_count
        )
    };
    TaskCockpitSummary {
        readiness_score: readiness.score,
        readiness_band: readiness.band,
        routing_confidence_score: ranking.overall_confidence_score,
        routing_confidence_band: ranking.overall_confidence_band,
        candidate_count: readiness.candidate_skills.len(),
        agent_count: comparison.agent_rows.len(),
        session_review_count: counts.session_review_count,
        provider_observability_row_count: counts.provider_observability_row_count,
        remediation_next_step_count: counts.remediation_next_step_count,
        gap_count: counts.gap_count,
        blocker_count: counts.blocker_count,
        recommended_agent,
        top_skill_name,
        summary,
    }
}

pub(crate) fn task_cockpit_sections(
    readiness: &TaskReadinessResult,
    ranking: &SkillRouteRankingResult,
    comparison: &AgentReadinessComparisonResult,
    session_review_rows: &[TaskCockpitSessionReviewRow],
    provider_rows: &[TaskCockpitProviderObservabilityRow],
    remediation_steps: &[TaskCockpitRemediationNextStep],
    safety_flags: TaskCockpitSafetyFlags,
) -> Vec<TaskCockpitSection> {
    vec![
        TaskCockpitSection {
            id: "task",
            title: "Task",
            status: readiness.band,
            score: Some(readiness.score),
            row_count: 1,
            summary: readiness.summary.clone(),
            evidence_refs: readiness
                .evidence_references
                .iter()
                .take(6)
                .map(|evidence| evidence.id.clone())
                .collect(),
            safety_flags,
        },
        TaskCockpitSection {
            id: "routing",
            title: "Routing",
            status: ranking.overall_confidence_band,
            score: Some(ranking.overall_confidence_score),
            row_count: ranking.route_candidates.len(),
            summary: ranking.summary.clone(),
            evidence_refs: ranking
                .evidence_references
                .iter()
                .take(6)
                .map(|evidence| evidence.id.clone())
                .collect(),
            safety_flags,
        },
        TaskCockpitSection {
            id: "agents",
            title: "Agents",
            status: if comparison.summary.blocked_agent_count > 0 {
                "partial"
            } else {
                "ready"
            },
            score: comparison
                .agent_rows
                .first()
                .map(|row| row.comparison_score),
            row_count: comparison.agent_rows.len(),
            summary: comparison.summary.summary.clone(),
            evidence_refs: comparison
                .evidence_references
                .iter()
                .take(6)
                .map(|evidence| evidence.id.clone())
                .collect(),
            safety_flags,
        },
        TaskCockpitSection {
            id: "sessions",
            title: "Sessions",
            status: if session_review_rows.is_empty() {
                "empty"
            } else {
                "ready"
            },
            score: None,
            row_count: session_review_rows.len(),
            summary: if session_review_rows.is_empty() {
                "No app-local session skill review rows matched the current filters.".to_string()
            } else {
                format!(
                    "Showing {} app-local session skill review row(s).",
                    session_review_rows.len()
                )
            },
            evidence_refs: session_review_rows
                .iter()
                .flat_map(|row| row.evidence_refs.iter().cloned())
                .take(6)
                .collect(),
            safety_flags,
        },
        TaskCockpitSection {
            id: "provider_observability",
            title: "Provider Observability",
            status: if provider_rows.is_empty() {
                "empty"
            } else {
                "ready"
            },
            score: None,
            row_count: provider_rows.len(),
            summary: if provider_rows.is_empty() {
                "No app-local provider observability rows matched the current filters.".to_string()
            } else {
                format!(
                    "Showing {} provider observability summary row(s) without reading credentials.",
                    provider_rows.len()
                )
            },
            evidence_refs: provider_rows
                .iter()
                .flat_map(|row| row.evidence_refs.iter().cloned())
                .take(6)
                .collect(),
            safety_flags,
        },
        TaskCockpitSection {
            id: "remediation",
            title: "Remediation",
            status: if remediation_steps.is_empty() {
                "empty"
            } else {
                "ready"
            },
            score: None,
            row_count: remediation_steps.len(),
            summary: if remediation_steps.is_empty() {
                "No read-only remediation next steps matched the current filters.".to_string()
            } else {
                format!(
                    "Showing {} read-only remediation next step(s).",
                    remediation_steps.len()
                )
            },
            evidence_refs: remediation_steps
                .iter()
                .flat_map(|row| row.evidence_refs.iter().cloned())
                .take(6)
                .collect(),
            safety_flags,
        },
    ]
}

pub(crate) fn task_cockpit_agent_route_row(
    row: &AgentReadinessComparisonRow,
) -> TaskCockpitAgentRouteRow {
    TaskCockpitAgentRouteRow {
        rank: row.rank,
        agent: row.agent.clone(),
        display_name: row.display_name.clone(),
        comparison_score: row.comparison_score,
        readiness_score: row.readiness_score,
        readiness_band: row.readiness_band,
        routing_confidence_score: row.routing_confidence_score,
        routing_confidence_band: row.routing_confidence_band,
        best_skill_name: row
            .best_candidate
            .as_ref()
            .map(|candidate| candidate.skill_name.clone()),
        blocker_count: row.blocker_count,
        gap_count: row.gap_count,
        reasons: row.reasons.clone(),
        evidence_refs: row.evidence_refs.clone(),
    }
}

pub(crate) fn task_cockpit_skill_candidate_row(
    candidate: &SkillRouteCandidate,
    readiness: Option<&TaskReadinessCandidate>,
) -> TaskCockpitSkillCandidateRow {
    TaskCockpitSkillCandidateRow {
        rank: candidate.rank,
        instance_id: candidate.instance_id.clone(),
        definition_id: candidate.definition_id.clone(),
        skill_name: candidate.skill_name.clone(),
        agent: candidate.agent.clone(),
        scope: candidate.scope.clone(),
        enabled: candidate.enabled,
        state: candidate.state.clone(),
        readiness_score: candidate.readiness_score,
        readiness_band: candidate.readiness_band,
        routing_confidence_score: candidate.confidence_score,
        routing_confidence_band: candidate.confidence_band,
        quality_score: candidate.quality_score,
        match_reasons: candidate.match_reasons.clone(),
        blocker_notes: readiness
            .map(|row| row.blocker_risk_notes.clone())
            .unwrap_or_default(),
        gap_notes: readiness
            .map(|row| row.missing_gap_notes.clone())
            .unwrap_or_default(),
        evidence_refs: candidate.evidence_refs.clone(),
    }
}

pub(crate) fn task_cockpit_readiness_rows(
    readiness: &TaskReadinessResult,
    ranking: &SkillRouteRankingResult,
    comparison: &AgentReadinessComparisonResult,
    limit: usize,
) -> Vec<TaskCockpitReadinessRow> {
    let mut rows = vec![
        TaskCockpitReadinessRow {
            id: "task-readiness".to_string(),
            row_type: "task",
            label: "Task readiness".to_string(),
            status: readiness.band,
            score: Some(readiness.score),
            summary: readiness.summary.clone(),
            evidence_refs: readiness
                .evidence_references
                .iter()
                .take(6)
                .map(|evidence| evidence.id.clone())
                .collect(),
        },
        TaskCockpitReadinessRow {
            id: "routing-confidence".to_string(),
            row_type: "routing",
            label: "Routing confidence".to_string(),
            status: ranking.overall_confidence_band,
            score: Some(ranking.overall_confidence_score),
            summary: ranking.summary.clone(),
            evidence_refs: ranking
                .evidence_references
                .iter()
                .take(6)
                .map(|evidence| evidence.id.clone())
                .collect(),
        },
    ];
    rows.extend(comparison.gap_issue_rows.iter().take(limit).map(|issue| {
        TaskCockpitReadinessRow {
            id: format!("agent-issue:{}:{}", issue.agent, stable_slug(&issue.title)),
            row_type: issue.source,
            label: issue.title.clone(),
            status: issue.severity,
            score: None,
            summary: issue.detail.clone(),
            evidence_refs: issue.evidence_refs.clone(),
        }
    }));
    rows
}

pub(crate) fn task_cockpit_session_review_row(
    review: &AgentSessionSkillReviewRecord,
) -> TaskCockpitSessionReviewRow {
    TaskCockpitSessionReviewRow {
        id: review.id.clone(),
        title: review.title.clone(),
        agent: review.agent.clone(),
        task: review.task.clone(),
        outcome: review.analysis.outcome.clone(),
        summary: review.analysis.summary.clone(),
        detected_skill_count: review.analysis.detected_skills.len(),
        expected_skill_signal_count: review.analysis.expected_skill_signals.len(),
        reviewed_at: review.reviewed_at,
        evidence_refs: review.analysis.evidence_refs.clone(),
    }
}

pub(crate) fn task_cockpit_provider_observability_rows(
    observability: &LlmProviderObservabilityResult,
    limit: usize,
) -> Vec<TaskCockpitProviderObservabilityRow> {
    let mut rows = observability
        .grouping_rows
        .iter()
        .map(|row| TaskCockpitProviderObservabilityRow {
            id: row.id.clone(),
            source: "provider_group",
            status: if row.failed_count > 0 {
                "partial".to_string()
            } else {
                "ready".to_string()
            },
            provider: Some(row.provider.clone()),
            model: Some(row.model.clone()),
            action: None,
            count: row.prompt_run_count + row.call_metadata_count,
            message: format!(
                "{} prompt run(s), {} call metadata row(s), {} failed row(s).",
                row.prompt_run_count, row.call_metadata_count, row.failed_count
            ),
            evidence_refs: row.evidence_refs.clone(),
        })
        .collect::<Vec<_>>();
    rows.extend(
        observability
            .status_rows
            .iter()
            .map(|row| TaskCockpitProviderObservabilityRow {
                id: row.id.clone(),
                source: "provider_status",
                status: row.status.clone(),
                provider: None,
                model: None,
                action: None,
                count: row.count,
                message: row.message.clone(),
                evidence_refs: row.evidence_refs.clone(),
            }),
    );
    rows.extend(
        observability
            .history_rows
            .iter()
            .map(|row| TaskCockpitProviderObservabilityRow {
                id: row.id.clone(),
                source: "prompt_run",
                status: row.status.clone(),
                provider: Some(row.provider.clone()),
                model: Some(row.model.clone()),
                action: Some(row.action.clone()),
                count: 1,
                message: format!(
                    "{} / {} completed with status {}.",
                    row.action, row.request_kind, row.status
                ),
                evidence_refs: row.evidence_refs.clone(),
            }),
    );
    rows.truncate(limit);
    rows
}

pub(crate) fn task_cockpit_remediation_next_steps(
    plan: Option<&RemediationPlanResult>,
    batch: Option<&RemediationBatchReviewResult>,
    limit: usize,
) -> Vec<TaskCockpitRemediationNextStep> {
    let mut steps = Vec::new();
    if let Some(plan) = plan {
        steps.extend(
            plan.plan_items
                .iter()
                .map(|item| TaskCockpitRemediationNextStep {
                    id: item.id.clone(),
                    source: "remediation_plan",
                    priority: item.priority,
                    title: item.title.clone(),
                    suggested_safe_next_action: item.suggested_safe_next_action.clone(),
                    blocker_notes: item.blockers.clone(),
                    gap_notes: Vec::new(),
                    evidence_refs: item.evidence_refs.clone(),
                }),
        );
    }
    if let Some(batch) = batch {
        steps.extend(
            batch
                .review_items
                .iter()
                .map(|item| TaskCockpitRemediationNextStep {
                    id: item.id.clone(),
                    source: "batch_review",
                    priority: match item.risk {
                        "high" => "high",
                        "medium" => "medium",
                        _ => "low",
                    },
                    title: item.title.clone(),
                    suggested_safe_next_action: item.recommended_next_step_label.clone(),
                    blocker_notes: item.blocker_notes.clone(),
                    gap_notes: item.gap_notes.clone(),
                    evidence_refs: item.evidence_refs.clone(),
                }),
        );
    }
    steps.truncate(limit);
    steps
}

pub(crate) fn provider_observability_evidence_as_task_refs(
    evidence: &[LlmProviderObservabilityEvidenceReference],
) -> Vec<TaskReadinessEvidenceReference> {
    evidence
        .iter()
        .map(|item| TaskReadinessEvidenceReference {
            id: format!("provider-observability:{}", item.id),
            source_type: "provider_observability",
            source_id: item.id.clone(),
            label: item.label.clone(),
            severity: None,
            related_instance_id: None,
        })
        .collect()
}

pub(crate) fn empty_task_readiness_result(
    task: String,
    filters: TaskReadinessFilters,
    catalog_available: bool,
) -> TaskReadinessResult {
    TaskReadinessResult {
        task: task.clone(),
        score: 0,
        band: "blocked",
        summary:
            "No local catalog is available, so task readiness cannot identify candidate skills."
                .to_string(),
        generated_by: "deterministic-service",
        catalog_available,
        filters: filters.clone(),
        candidate_skills: Vec::new(),
        missing_gap_notes: vec![
            "Run a local scan before relying on task readiness for routing decisions.".to_string(),
        ],
        blocker_risk_notes: vec![
            "No provider request was sent and no fallback network lookup was attempted."
                .to_string(),
        ],
        evidence_references: Vec::new(),
        prompt_request: TaskReadinessPromptRequest {
            available: false,
            preview_method: "llm.previewPrompt",
            confirm_method: "llm.confirmPromptAndSend",
            action: "task_readiness",
            request: LlmPreviewPromptParams {
                action: LlmPromptActionKind::TaskReadiness,
                profile_id: None,
                app_language: None,
                skill_instance_id: None,
                instance_ids: Vec::new(),
                agents: Vec::new(),
                analysis_kind: None,
                user_intent: Some(task),
            },
            note: "Prompt preview is unavailable until local catalog evidence exists.".to_string(),
        },
        aggregation: empty_aggregation_runtime(
            TASK_AGGREGATION_TIMEOUT_MS,
            filters.limit,
            "task.checkReadiness",
            "No local catalog was available; task readiness returned an empty read-only result.",
        ),
        safety_flags: task_readiness_safety_flags(),
    }
}

pub(crate) fn task_readiness_terms(task: &str) -> Vec<String> {
    let mut seen = BTreeMap::new();
    let mut terms = Vec::new();
    let mut ascii_run = String::new();
    let mut cjk_run = String::new();

    for ch in task.chars() {
        if ch.is_ascii_alphanumeric() {
            task_readiness_flush_cjk_terms(&mut cjk_run, &mut seen, &mut terms);
            ascii_run.push(ch);
        } else if task_readiness_is_cjk_char(ch) {
            task_readiness_flush_ascii_term(&mut ascii_run, &mut seen, &mut terms);
            cjk_run.push(ch);
        } else {
            task_readiness_flush_ascii_term(&mut ascii_run, &mut seen, &mut terms);
            task_readiness_flush_cjk_terms(&mut cjk_run, &mut seen, &mut terms);
        }
    }
    task_readiness_flush_ascii_term(&mut ascii_run, &mut seen, &mut terms);
    task_readiness_flush_cjk_terms(&mut cjk_run, &mut seen, &mut terms);

    let base_terms = terms.clone();
    for term in base_terms {
        task_readiness_push_expanded_terms(&term, &mut seen, &mut terms);
    }
    terms
}

fn task_readiness_flush_ascii_term(
    run: &mut String,
    seen: &mut BTreeMap<String, ()>,
    terms: &mut Vec<String>,
) {
    if run.is_empty() {
        return;
    }
    let normalized = run.to_ascii_lowercase();
    run.clear();
    if normalized.len() < 3 || task_readiness_is_stopword(&normalized) {
        return;
    }
    task_readiness_push_term(normalized, seen, terms);
}

fn task_readiness_flush_cjk_terms(
    run: &mut String,
    seen: &mut BTreeMap<String, ()>,
    terms: &mut Vec<String>,
) {
    if run.is_empty() {
        return;
    }
    let chars = run.chars().collect::<Vec<_>>();
    run.clear();
    if chars.len() < 2 {
        return;
    }
    if chars.len() <= 12 {
        task_readiness_push_term(chars.iter().collect::<String>(), seen, terms);
    }
    for width in 2..=usize::min(4, chars.len()) {
        for window in chars.windows(width) {
            task_readiness_push_term(window.iter().collect::<String>(), seen, terms);
        }
    }
}

fn task_readiness_push_expanded_terms(
    term: &str,
    seen: &mut BTreeMap<String, ()>,
    terms: &mut Vec<String>,
) {
    if matches!(term, "aliyun" | "alibaba" | "alibabacloud")
        || term.contains("阿里云")
        || term == "阿里"
        || term == "里云"
    {
        for expanded in ["alibabacloud", "aliyun", "alibaba"] {
            task_readiness_push_term(expanded.to_string(), seen, terms);
        }
    }

    if matches!(term, "alert" | "alerts" | "alarm" | "alarms" | "cms")
        || term.contains("报警")
        || term.contains("告警")
    {
        for expanded in ["alert", "alerts", "alarm", "cms", "报警", "告警"] {
            task_readiness_push_term(expanded.to_string(), seen, terms);
        }
    }

    if matches!(term, "metric" | "metrics" | "monitor" | "monitoring")
        || term.contains("指标")
        || term.contains("监控")
    {
        for expanded in ["metric", "metrics", "monitor", "monitoring", "指标", "监控"] {
            task_readiness_push_term(expanded.to_string(), seen, terms);
        }
    }

    if matches!(term, "error" | "errors" | "exception" | "exceptions")
        || term.contains("错误")
        || term.contains("异常")
    {
        for expanded in ["error", "errors", "exception", "异常", "错误"] {
            task_readiness_push_term(expanded.to_string(), seen, terms);
        }
    }

    if matches!(term, "alb" | "slb") || term.contains("负载均衡") {
        for expanded in ["alb", "slb", "load", "balancer", "负载", "均衡", "负载均衡"] {
            task_readiness_push_term(expanded.to_string(), seen, terms);
        }
    }

    if matches!(term, "ecs") || term.contains("云服务器") || term.contains("服务器") {
        for expanded in [
            "ecs",
            "elastic",
            "compute",
            "server",
            "instance",
            "云服务器",
            "服务器",
        ] {
            task_readiness_push_term(expanded.to_string(), seen, terms);
        }
    }

    if matches!(term, "disk" | "disks" | "ebs" | "volume" | "storage")
        || term.contains("磁盘")
        || term.contains("硬盘")
        || term.contains("存储")
    {
        for expanded in [
            "disk", "disks", "ebs", "volume", "storage", "磁盘", "硬盘", "存储",
        ] {
            task_readiness_push_term(expanded.to_string(), seen, terms);
        }
    }

    if matches!(
        term,
        "load" | "usage" | "utilization" | "metric" | "metrics"
    ) || term.contains("负载")
        || term.contains("使用率")
        || term.contains("利用率")
    {
        for expanded in [
            "load",
            "usage",
            "utilization",
            "metric",
            "metrics",
            "负载",
            "使用率",
            "利用率",
        ] {
            task_readiness_push_term(expanded.to_string(), seen, terms);
        }
    }

    if matches!(
        term,
        "analysis" | "analyze" | "diagnose" | "diagnosis" | "status" | "overview"
    ) || term.contains("分析")
        || term.contains("诊断")
        || term.contains("情况")
        || term.contains("状态")
    {
        for expanded in [
            "analysis",
            "analyze",
            "diagnose",
            "diagnosis",
            "monitor",
            "metric",
            "metrics",
            "status",
            "overview",
            "分析",
            "诊断",
            "情况",
            "状态",
        ] {
            task_readiness_push_term(expanded.to_string(), seen, terms);
        }
    }

    if matches!(term, "query" | "list" | "view" | "describe" | "history")
        || term.contains("查看")
        || term.contains("查询")
        || term.contains("列表")
        || term.contains("历史")
    {
        for expanded in ["query", "list", "view", "describe"] {
            task_readiness_push_term(expanded.to_string(), seen, terms);
        }
    }
}

fn task_readiness_push_term(
    term: String,
    seen: &mut BTreeMap<String, ()>,
    terms: &mut Vec<String>,
) {
    if let std::collections::btree_map::Entry::Vacant(entry) = seen.entry(term.clone()) {
        entry.insert(());
        terms.push(term);
    }
}

fn task_readiness_is_stopword(term: &str) -> bool {
    matches!(
        term,
        "the"
            | "and"
            | "for"
            | "with"
            | "from"
            | "that"
            | "this"
            | "into"
            | "using"
            | "need"
            | "task"
    )
}

fn task_readiness_is_cjk_char(ch: char) -> bool {
    let code = ch as u32;
    (0x3400..=0x4DBF).contains(&code)
        || (0x4E00..=0x9FFF).contains(&code)
        || (0xF900..=0xFAFF).contains(&code)
        || (0x20000..=0x2A6DF).contains(&code)
        || (0x2A700..=0x2B73F).contains(&code)
        || (0x2B740..=0x2B81F).contains(&code)
        || (0x2B820..=0x2CEAF).contains(&code)
}

pub(crate) fn task_readiness_candidate_scan_limit(limit: usize, requested_count: usize) -> usize {
    if requested_count > 0 {
        return requested_count
            .max(limit)
            .clamp(1, REMEDIATION_MAX_DETAIL_SCAN);
    }
    limit
        .saturating_mul(TASK_READINESS_CANDIDATE_SCAN_MULTIPLIER)
        .clamp(
            TASK_READINESS_MIN_CANDIDATE_SCAN,
            TASK_READINESS_MAX_CANDIDATE_SCAN,
        )
}

pub(crate) fn task_readiness_detail_scan_limit(limit: usize, prefiltered_count: usize) -> usize {
    limit.saturating_mul(4).clamp(limit, prefiltered_count)
}

pub(crate) fn task_readiness_record_affinity(skill: &SkillRecord, task_terms: &[String]) -> u16 {
    let searchable = format!(
        "{} {} {} {} {}",
        skill.id, skill.definition_id, skill.name, skill.agent, skill.scope
    )
    .to_ascii_lowercase();
    let matched_terms = task_terms
        .iter()
        .filter(|term| searchable.contains(term.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let domain_signal = task_readiness_domain_signal(task_terms, &searchable);
    let mut score = task_readiness_weighted_match_score(&matched_terms).clamp(0, 240) as u16;
    if domain_signal.has_product_match() {
        score = score.saturating_add(28);
    } else if !domain_signal.incompatible_candidate_domains.is_empty() {
        score = score.saturating_sub(30);
    }
    if skill.enabled {
        score = score.saturating_add(4);
    }
    if skill.state == "loaded" {
        score = score.saturating_add(3);
    }
    score
}

#[derive(Debug, Clone)]
pub(crate) struct TaskReadinessDomainSignal {
    task_domains: BTreeSet<&'static str>,
    candidate_domains: BTreeSet<&'static str>,
    matched_domains: BTreeSet<&'static str>,
    compatible_domains: BTreeSet<&'static str>,
    incompatible_candidate_domains: BTreeSet<&'static str>,
}

impl TaskReadinessDomainSignal {
    fn has_product_match(&self) -> bool {
        !self.matched_domains.is_empty() || !self.compatible_domains.is_empty()
    }
}

fn task_readiness_weighted_match_score(matched_terms: &[String]) -> i16 {
    matched_terms
        .iter()
        .map(|term| task_readiness_term_weight(term))
        .sum::<i16>()
        .min(45)
}

fn task_readiness_term_weight(term: &str) -> i16 {
    let normalized = term.to_ascii_lowercase();
    if task_readiness_is_generic_route_term(&normalized) {
        return 3;
    }
    if task_readiness_is_product_or_resource_term(&normalized) {
        return 14;
    }
    if matches!(
        normalized.as_str(),
        "metric"
            | "metrics"
            | "monitor"
            | "monitoring"
            | "alert"
            | "alerts"
            | "alarm"
            | "alarms"
            | "error"
            | "errors"
            | "exception"
            | "exceptions"
            | "usage"
            | "utilization"
    ) || normalized.contains("指标")
        || normalized.contains("监控")
        || normalized.contains("报警")
        || normalized.contains("告警")
        || normalized.contains("错误")
        || normalized.contains("异常")
        || normalized.contains("使用率")
        || normalized.contains("利用率")
    {
        return 8;
    }
    6
}

fn task_readiness_is_generic_route_term(term: &str) -> bool {
    matches!(
        term,
        "aliyun"
            | "alibaba"
            | "alibabacloud"
            | "analysis"
            | "analyze"
            | "diagnose"
            | "diagnosis"
            | "query"
            | "list"
            | "view"
            | "describe"
            | "history"
            | "status"
            | "overview"
            | "情况"
            | "状态"
            | "查看"
            | "查询"
            | "列表"
            | "历史"
    ) || term.contains("阿里")
        || term.contains("里云")
        || term.contains("分析")
        || term.contains("诊断")
}

fn task_readiness_is_product_or_resource_term(term: &str) -> bool {
    matches!(
        term,
        "ecs"
            | "alb"
            | "slb"
            | "rds"
            | "oss"
            | "waf"
            | "sls"
            | "ebs"
            | "disk"
            | "disks"
            | "volume"
            | "storage"
            | "analyticdb"
            | "spark"
            | "flink"
            | "emr"
            | "maxcompute"
            | "odps"
            | "polardb"
            | "mongodb"
            | "elasticsearch"
            | "tair"
            | "lindorm"
            | "dataworks"
            | "dts"
            | "ddos"
            | "ram"
            | "vpc"
            | "cdn"
            | "cms"
    ) || term.contains("云服务器")
        || term.contains("服务器")
        || term.contains("负载均衡")
        || term.contains("磁盘")
        || term.contains("硬盘")
        || term.contains("存储")
        || term.contains("分析型数据库")
        || term.contains("云监控")
}

fn task_readiness_domain_signal(
    task_terms: &[String],
    searchable: &str,
) -> TaskReadinessDomainSignal {
    let task_domains = task_readiness_product_domains_from_terms(task_terms);
    let candidate_domains = task_readiness_product_domains_from_text(searchable);
    let task_capabilities = task_readiness_capabilities_from_terms(task_terms);
    let mut matched_domains = task_domains
        .intersection(&candidate_domains)
        .copied()
        .collect::<BTreeSet<_>>();
    let mut compatible_domains = BTreeSet::new();

    if task_readiness_load_balancer_domains_compatible(&task_domains, &candidate_domains) {
        compatible_domains.insert("load-balancer");
    }
    if task_capabilities.contains("monitoring")
        && candidate_domains.contains("cms")
        && task_readiness_candidate_has_no_exclusive_product_mismatch(
            &task_domains,
            &candidate_domains,
        )
    {
        compatible_domains.insert("cms-monitoring");
    }
    if task_domains.contains("disk") && candidate_domains.contains("ebs") {
        matched_domains.insert("disk");
    }

    let incompatible_candidate_domains = if task_domains.is_empty()
        || candidate_domains.is_empty()
        || !matched_domains.is_empty()
        || !compatible_domains.is_empty()
    {
        BTreeSet::new()
    } else {
        candidate_domains.clone()
    };

    TaskReadinessDomainSignal {
        task_domains,
        candidate_domains,
        matched_domains,
        compatible_domains,
        incompatible_candidate_domains,
    }
}

fn task_readiness_product_domains_from_terms(terms: &[String]) -> BTreeSet<&'static str> {
    let searchable = terms.join(" ").to_ascii_lowercase();
    task_readiness_product_domains_from_text(&searchable)
}

fn task_readiness_product_domains_from_text(text: &str) -> BTreeSet<&'static str> {
    let normalized = text.to_ascii_lowercase();
    let mut domains = BTreeSet::new();
    task_readiness_insert_domain_if(
        &mut domains,
        "ecs",
        &normalized,
        &["ecs", "elastic compute", "云服务器", "服务器"],
    );
    task_readiness_insert_domain_if(
        &mut domains,
        "alb",
        &normalized,
        &["alb", "application load balancer", "应用型负载均衡"],
    );
    task_readiness_insert_domain_if(
        &mut domains,
        "slb",
        &normalized,
        &["slb", "server load balancer", "负载均衡"],
    );
    task_readiness_insert_domain_if(
        &mut domains,
        "disk",
        &normalized,
        &["disk", "disks", "volume", "storage", "磁盘", "硬盘", "存储"],
    );
    task_readiness_insert_domain_if(
        &mut domains,
        "ebs",
        &normalized,
        &["ebs", "elastic block storage", "块存储"],
    );
    task_readiness_insert_domain_if(
        &mut domains,
        "cms",
        &normalized,
        &["cms", "cloudmonitor", "cloud monitor", "云监控"],
    );
    task_readiness_insert_domain_if(
        &mut domains,
        "analyticdb",
        &normalized,
        &["analyticdb", "分析型数据库"],
    );
    task_readiness_insert_domain_if(&mut domains, "spark", &normalized, &["spark"]);
    task_readiness_insert_domain_if(&mut domains, "rds", &normalized, &["rds"]);
    task_readiness_insert_domain_if(&mut domains, "oss", &normalized, &["oss"]);
    task_readiness_insert_domain_if(&mut domains, "sls", &normalized, &["sls", "log service"]);
    task_readiness_insert_domain_if(&mut domains, "waf", &normalized, &["waf"]);
    task_readiness_insert_domain_if(&mut domains, "ddos", &normalized, &["ddos"]);
    task_readiness_insert_domain_if(&mut domains, "ram", &normalized, &["ram"]);
    task_readiness_insert_domain_if(&mut domains, "vpc", &normalized, &["vpc"]);
    task_readiness_insert_domain_if(&mut domains, "cdn", &normalized, &["cdn"]);
    task_readiness_insert_domain_if(&mut domains, "flink", &normalized, &["flink"]);
    task_readiness_insert_domain_if(&mut domains, "emr", &normalized, &["emr"]);
    task_readiness_insert_domain_if(&mut domains, "maxcompute", &normalized, &["maxcompute"]);
    task_readiness_insert_domain_if(&mut domains, "odps", &normalized, &["odps"]);
    task_readiness_insert_domain_if(&mut domains, "polardb", &normalized, &["polardb"]);
    task_readiness_insert_domain_if(&mut domains, "mongodb", &normalized, &["mongodb"]);
    task_readiness_insert_domain_if(
        &mut domains,
        "elasticsearch",
        &normalized,
        &["elasticsearch"],
    );
    task_readiness_insert_domain_if(&mut domains, "tair", &normalized, &["tair"]);
    task_readiness_insert_domain_if(&mut domains, "lindorm", &normalized, &["lindorm"]);
    task_readiness_insert_domain_if(&mut domains, "dataworks", &normalized, &["dataworks"]);
    task_readiness_insert_domain_if(&mut domains, "dts", &normalized, &["dts"]);
    domains
}

fn task_readiness_insert_domain_if(
    domains: &mut BTreeSet<&'static str>,
    domain: &'static str,
    text: &str,
    patterns: &[&str],
) {
    if patterns.iter().any(|pattern| text.contains(pattern)) {
        domains.insert(domain);
    }
}

fn task_readiness_capabilities_from_terms(terms: &[String]) -> BTreeSet<&'static str> {
    let mut capabilities = BTreeSet::new();
    for term in terms {
        let normalized = term.to_ascii_lowercase();
        if matches!(
            normalized.as_str(),
            "metric" | "metrics" | "monitor" | "monitoring" | "status" | "overview"
        ) || normalized.contains("指标")
            || normalized.contains("监控")
        {
            capabilities.insert("monitoring");
        }
        if matches!(
            normalized.as_str(),
            "alert" | "alerts" | "alarm" | "alarms" | "cms"
        ) || normalized.contains("报警")
            || normalized.contains("告警")
        {
            capabilities.insert("monitoring");
            capabilities.insert("alert");
        }
        if matches!(
            normalized.as_str(),
            "error" | "errors" | "exception" | "exceptions"
        ) || normalized.contains("错误")
            || normalized.contains("异常")
        {
            capabilities.insert("monitoring");
            capabilities.insert("error");
        }
    }
    capabilities
}

fn task_readiness_load_balancer_domains_compatible(
    task_domains: &BTreeSet<&'static str>,
    candidate_domains: &BTreeSet<&'static str>,
) -> bool {
    (task_domains.contains("alb") || task_domains.contains("slb"))
        && (candidate_domains.contains("alb") || candidate_domains.contains("slb"))
}

fn task_readiness_candidate_has_no_exclusive_product_mismatch(
    task_domains: &BTreeSet<&'static str>,
    candidate_domains: &BTreeSet<&'static str>,
) -> bool {
    candidate_domains
        .iter()
        .all(|domain| *domain == "cms" || task_domains.contains(domain))
        || task_readiness_load_balancer_domains_compatible(task_domains, candidate_domains)
}

fn task_readiness_domain_list(domains: &BTreeSet<&'static str>) -> String {
    domains.iter().copied().collect::<Vec<_>>().join(", ")
}

pub(crate) fn task_readiness_findings_by_instance(
    findings: &[RuleFindingRecord],
) -> BTreeMap<String, Vec<RuleFindingRecord>> {
    let mut by_instance = BTreeMap::<String, Vec<RuleFindingRecord>>::new();
    for finding in findings {
        if let Some(instance_id) = finding.instance_id.as_ref() {
            by_instance
                .entry(instance_id.clone())
                .or_default()
                .push(finding.clone());
        }
    }
    by_instance
}

pub(crate) fn task_readiness_findings_by_definition(
    findings: &[RuleFindingRecord],
) -> BTreeMap<String, Vec<RuleFindingRecord>> {
    let mut by_definition = BTreeMap::<String, Vec<RuleFindingRecord>>::new();
    for finding in findings {
        if let Some(definition_id) = finding.definition_id.as_ref() {
            by_definition
                .entry(definition_id.clone())
                .or_default()
                .push(finding.clone());
        }
    }
    by_definition
}

pub(crate) fn task_readiness_conflicts_by_instance(
    conflicts: &[ConflictGroupRecord],
) -> BTreeMap<String, Vec<ConflictGroupRecord>> {
    let mut by_instance = BTreeMap::<String, Vec<ConflictGroupRecord>>::new();
    for conflict in conflicts {
        for instance_id in &conflict.instance_ids {
            by_instance
                .entry(instance_id.clone())
                .or_default()
                .push(conflict.clone());
        }
    }
    by_instance
}

pub(crate) fn task_readiness_conflicts_by_definition(
    conflicts: &[ConflictGroupRecord],
) -> BTreeMap<String, Vec<ConflictGroupRecord>> {
    let mut by_definition = BTreeMap::<String, Vec<ConflictGroupRecord>>::new();
    for conflict in conflicts {
        by_definition
            .entry(conflict.definition_id.clone())
            .or_default()
            .push(conflict.clone());
    }
    by_definition
}

pub(crate) fn task_readiness_analysis_by_instance(
    groups: &[CrossAgentAnalysisGroup],
) -> BTreeMap<String, Vec<CrossAgentAnalysisGroup>> {
    let mut by_instance = BTreeMap::<String, Vec<CrossAgentAnalysisGroup>>::new();
    for group in groups {
        for instance_id in &group.instance_ids {
            by_instance
                .entry(instance_id.clone())
                .or_default()
                .push(group.clone());
        }
    }
    by_instance
}

pub(crate) fn task_readiness_related_findings(
    detail: &SkillDetailRecord,
    by_instance: &BTreeMap<String, Vec<RuleFindingRecord>>,
    by_definition: &BTreeMap<String, Vec<RuleFindingRecord>>,
) -> Vec<RuleFindingRecord> {
    let mut seen = BTreeSet::new();
    by_instance
        .get(&detail.id)
        .into_iter()
        .flatten()
        .chain(
            by_definition
                .get(&detail.definition_id)
                .into_iter()
                .flatten(),
        )
        .filter(|finding| seen.insert(finding.id.clone()))
        .cloned()
        .collect()
}

pub(crate) fn task_readiness_related_conflicts(
    detail: &SkillDetailRecord,
    by_instance: &BTreeMap<String, Vec<ConflictGroupRecord>>,
    by_definition: &BTreeMap<String, Vec<ConflictGroupRecord>>,
) -> Vec<ConflictGroupRecord> {
    let mut seen = BTreeSet::new();
    by_instance
        .get(&detail.id)
        .into_iter()
        .flatten()
        .chain(
            by_definition
                .get(&detail.definition_id)
                .into_iter()
                .flatten(),
        )
        .filter(|conflict| seen.insert(conflict.id.clone()))
        .cloned()
        .collect()
}

pub(crate) fn task_readiness_related_analysis(
    detail: &SkillDetailRecord,
    by_instance: &BTreeMap<String, Vec<CrossAgentAnalysisGroup>>,
) -> Vec<CrossAgentAnalysisGroup> {
    by_instance.get(&detail.id).cloned().unwrap_or_default()
}

pub(crate) fn task_readiness_quality_signal(
    skill: &SkillDetailRecord,
    findings: &[RuleFindingRecord],
    conflicts: &[ConflictGroupRecord],
    analysis_groups: &[CrossAgentAnalysisGroup],
    diagnostic: Option<&AdapterDiagnosticsRecord>,
) -> TaskReadinessQualitySignal {
    let (metadata_score, _, _) = quality_metadata_component(skill);
    let (permission_score, _, _, _) = quality_permission_component(skill);
    let (risk_score, _, _, _) = quality_risk_component(skill, findings);
    let (conflict_score, _, _) = quality_conflict_component(conflicts, analysis_groups);
    let (adapter_score, _, _) = quality_adapter_component(skill, diagnostic);
    let score = [
        metadata_score,
        permission_score,
        risk_score,
        conflict_score,
        adapter_score,
    ]
    .into_iter()
    .map(u16::from)
    .sum::<u16>()
    .min(100) as u8;
    let (_, band) = quality_grade_and_band(score);
    TaskReadinessQualitySignal { score, band }
}
pub(crate) struct TaskReadinessCandidateSignals<'a> {
    pub(crate) findings: &'a [RuleFindingRecord],
    pub(crate) conflicts: &'a [ConflictGroupRecord],
    pub(crate) analysis_groups: &'a [CrossAgentAnalysisGroup],
    pub(crate) diagnostic: Option<&'a AdapterDiagnosticsRecord>,
    pub(crate) quality: Option<&'a TaskReadinessQualitySignal>,
}

#[derive(Debug, Clone)]
pub(crate) struct TaskReadinessQualitySignal {
    score: u8,
    band: &'static str,
}

pub(crate) fn task_readiness_candidate(
    task_terms: &[String],
    skill: &SkillDetailRecord,
    signals: TaskReadinessCandidateSignals<'_>,
    evidence: &mut Vec<TaskReadinessEvidenceReference>,
) -> TaskReadinessCandidate {
    let skill_ref = push_task_readiness_evidence(
        evidence,
        "skill",
        &skill.id,
        format!(
            "Catalog metadata for `{}` ({}, {}, enabled={}, state={})",
            redact_for_llm_preview(&skill.name),
            redact_for_llm_preview(&skill.agent),
            redact_for_llm_preview(&skill.scope),
            skill.enabled,
            redact_for_llm_preview(&skill.state)
        ),
        None,
        Some(skill.id.clone()),
    );
    let quality_ref = signals.quality.map(|score| {
        push_task_readiness_evidence(
            evidence,
            "quality_score",
            &skill.id,
            format!("V2.43 quality score {} / 100 ({})", score.score, score.band),
            None,
            Some(skill.id.clone()),
        )
    });

    let searchable = format!(
        "{} {} {} {}",
        skill.name, skill.description, skill.frontmatter_raw, skill.body
    )
    .to_ascii_lowercase();
    let matched_terms = task_terms
        .iter()
        .filter(|term| searchable.contains(term.as_str()))
        .cloned()
        .collect::<Vec<_>>();
    let domain_signal = task_readiness_domain_signal(task_terms, &searchable);
    let mut match_reasons = Vec::new();
    if matched_terms.is_empty() {
        match_reasons.push(
            "No direct lexical overlap with the task was found in local metadata/body evidence."
                .to_string(),
        );
    } else {
        match_reasons.push(format!(
            "Matched task term(s): {}.",
            matched_terms
                .iter()
                .take(8)
                .map(|term| redact_for_llm_preview(term))
                .collect::<Vec<_>>()
                .join(", ")
        ));
    }
    if domain_signal.has_product_match() {
        let mut matched_domains = domain_signal.matched_domains.clone();
        matched_domains.extend(domain_signal.compatible_domains.iter().copied());
        match_reasons.push(format!(
            "Matched product/resource scope: {}.",
            task_readiness_domain_list(&matched_domains)
        ));
    } else if !domain_signal.incompatible_candidate_domains.is_empty() {
        match_reasons.push(format!(
            "Product/resource mismatch: task mentions {}; candidate scope is {}.",
            task_readiness_domain_list(&domain_signal.task_domains),
            task_readiness_domain_list(&domain_signal.incompatible_candidate_domains)
        ));
    } else if !domain_signal.task_domains.is_empty() && domain_signal.candidate_domains.is_empty() {
        match_reasons.push(format!(
            "Task product/resource terms detected ({}), but this candidate has no explicit product scope.",
            task_readiness_domain_list(&domain_signal.task_domains)
        ));
    }
    if skill.description.trim().is_empty() {
        match_reasons
            .push("Description is empty, limiting deterministic task-fit evidence.".to_string());
    } else {
        match_reasons.push(format!(
            "Description evidence: {}",
            redact_for_llm_preview(&skill.description)
        ));
    }

    let mut missing_gap_notes = Vec::new();
    let mut blocker_risk_notes = Vec::new();
    if !skill.enabled {
        blocker_risk_notes.push("Skill is disabled and will not be a ready routing target until reviewed through the existing toggle flow.".to_string());
    }
    if skill.state != "loaded" {
        blocker_risk_notes.push(format!(
            "Skill state is `{}` instead of loaded.",
            redact_for_llm_preview(&skill.state)
        ));
    }
    if skill.scope == Scope::AgentProject.as_str() {
        match_reasons
            .push("Project-scoped skill is visible in the current project context.".to_string());
    }
    if matched_terms.is_empty() {
        missing_gap_notes.push(
            "Task wording did not clearly map to this skill; consider improving description keywords if it should route here."
                .to_string(),
        );
    }

    let mut risk_refs = Vec::new();
    for finding in signals.findings {
        let evidence_id = push_task_readiness_evidence(
            evidence,
            "finding",
            &finding.id,
            format!(
                "{} finding `{}`: {}",
                redact_for_llm_preview(&finding.effective_severity),
                redact_for_llm_preview(&finding.rule_id),
                redact_for_llm_preview(&finding.message)
            ),
            Some(finding.effective_severity.clone()),
            finding.instance_id.clone(),
        );
        risk_refs.push(evidence_id);
        if matches!(
            finding.effective_severity.as_str(),
            "critical" | "error" | "warning" | "warn"
        ) {
            blocker_risk_notes.push(format!(
                "{} finding `{}` affects readiness.",
                redact_for_llm_preview(&finding.effective_severity),
                redact_for_llm_preview(&finding.rule_id)
            ));
        }
    }
    for conflict in signals.conflicts {
        let evidence_id = push_task_readiness_evidence(
            evidence,
            "conflict",
            &conflict.id,
            format!(
                "Same-agent conflict `{}` covers {} instance(s)",
                redact_for_llm_preview(&conflict.reason),
                conflict.instance_ids.len()
            ),
            Some("warning".to_string()),
            Some(skill.id.clone()),
        );
        risk_refs.push(evidence_id);
        blocker_risk_notes
            .push("Same-agent conflict may make runtime selection ambiguous.".to_string());
    }
    for group in signals.analysis_groups {
        let evidence_id = push_task_readiness_evidence(
            evidence,
            "analysis",
            &group.id,
            format!(
                "{} analysis `{}`: {}",
                redact_for_llm_preview(&group.severity),
                redact_for_llm_preview(&group.kind),
                redact_for_llm_preview(&group.title)
            ),
            Some(group.severity.clone()),
            Some(skill.id.clone()),
        );
        risk_refs.push(evidence_id);
        if group.kind == "enabled_mismatch" || group.kind == "duplicate_name" {
            blocker_risk_notes.push(format!(
                "Cross-agent analysis `{}` may affect routing clarity.",
                redact_for_llm_preview(&group.kind)
            ));
        }
    }

    let diagnostic_ref = signals.diagnostic.map(|diagnostic| {
        push_task_readiness_evidence(
            evidence,
            "adapter_diagnostics",
            diagnostic.agent,
            format!(
                "{} adapter diagnostics: status={}, writable_status={}, install_status={}",
                diagnostic.display_name,
                diagnostic.status,
                diagnostic.access.writable_status,
                diagnostic.access.install_status
            ),
            None,
            Some(skill.id.clone()),
        )
    });

    let risk_level = task_readiness_risk_level(
        signals.findings,
        signals.conflicts,
        signals.analysis_groups,
        skill,
    );
    let risk_summary = task_readiness_risk_summary(
        risk_level,
        signals.findings,
        signals.conflicts,
        signals.analysis_groups,
    );
    let mut score = task_readiness_weighted_match_score(&matched_terms);
    if domain_signal.has_product_match() {
        score += 22;
    } else if !domain_signal.incompatible_candidate_domains.is_empty() {
        score -= 45;
    } else if !domain_signal.task_domains.is_empty() {
        score -= 12;
    }
    score += signals
        .quality
        .map(|quality| i16::from(quality.score) / 4)
        .unwrap_or(0);
    if skill.enabled {
        score += 15;
    }
    if skill.state == "loaded" {
        score += 10;
    }
    if !skill.description.trim().is_empty() {
        score += 5;
    }
    score -= task_readiness_risk_deduction(
        signals.findings,
        signals.conflicts,
        signals.analysis_groups,
        skill,
    );
    let score = score.clamp(0, 100) as u8;
    let mut evidence_refs = vec![skill_ref];
    if let Some(quality_ref) = quality_ref {
        evidence_refs.push(quality_ref);
    }
    evidence_refs.extend(risk_refs);
    if let Some(diagnostic_ref) = diagnostic_ref {
        evidence_refs.push(diagnostic_ref);
    }

    TaskReadinessCandidate {
        instance_id: skill.id.clone(),
        definition_id: skill.definition_id.clone(),
        skill_name: redact_for_llm_preview(&skill.name),
        agent: skill.agent.clone(),
        scope: skill.scope.clone(),
        enabled: skill.enabled,
        state: skill.state.clone(),
        score,
        band: task_readiness_band(score),
        quality_score: signals.quality.map(|quality| quality.score),
        match_reasons,
        enabled_scope_risk_state: TaskReadinessState {
            enabled: skill.enabled,
            scope: skill.scope.clone(),
            state: skill.state.clone(),
            risk_level,
            risk_summary,
            writable_status: signals
                .diagnostic
                .map(|diagnostic| diagnostic.access.writable_status.to_string()),
            adapter_status: signals
                .diagnostic
                .map(|diagnostic| diagnostic.status.to_string()),
        },
        missing_gap_notes,
        blocker_risk_notes,
        evidence_refs,
    }
}

pub(crate) fn task_readiness_risk_level(
    findings: &[RuleFindingRecord],
    conflicts: &[ConflictGroupRecord],
    analysis_groups: &[CrossAgentAnalysisGroup],
    skill: &SkillDetailRecord,
) -> &'static str {
    if !skill.enabled || skill.state != "loaded" {
        return "blocked";
    }
    if findings
        .iter()
        .any(|finding| matches!(finding.effective_severity.as_str(), "critical" | "error"))
        || !conflicts.is_empty()
    {
        return "high";
    }
    if findings
        .iter()
        .any(|finding| matches!(finding.effective_severity.as_str(), "warning" | "warn"))
        || !analysis_groups.is_empty()
    {
        return "medium";
    }
    "low"
}

pub(crate) fn task_readiness_risk_summary(
    risk_level: &'static str,
    findings: &[RuleFindingRecord],
    conflicts: &[ConflictGroupRecord],
    analysis_groups: &[CrossAgentAnalysisGroup],
) -> String {
    if risk_level == "low" {
        return "No high-risk local findings, same-agent conflicts, or cross-agent ambiguity were associated with this candidate.".to_string();
    }
    format!(
        "Risk level {risk_level}: {} finding(s), {} same-agent conflict(s), and {} cross-agent analysis group(s) are associated with this candidate.",
        findings.len(),
        conflicts.len(),
        analysis_groups.len()
    )
}

pub(crate) fn task_readiness_risk_deduction(
    findings: &[RuleFindingRecord],
    conflicts: &[ConflictGroupRecord],
    analysis_groups: &[CrossAgentAnalysisGroup],
    skill: &SkillDetailRecord,
) -> i16 {
    let mut deduction = 0i16;
    if !skill.enabled {
        deduction += 25;
    }
    if skill.state != "loaded" {
        deduction += 30;
    }
    for finding in findings {
        deduction += match finding.effective_severity.as_str() {
            "critical" => 25,
            "error" => 18,
            "warning" | "warn" => 10,
            "info" => 3,
            _ => 1,
        };
    }
    deduction += (conflicts.len() as i16 * 18).min(30);
    deduction += (analysis_groups.len() as i16 * 6).min(18);
    deduction
}

pub(crate) fn task_readiness_overall_score(candidates: &[TaskReadinessCandidate]) -> u8 {
    let Some(best) = candidates.first() else {
        return 0;
    };
    let secondary = candidates
        .get(1)
        .map(|candidate| candidate.score)
        .unwrap_or(0);
    ((u16::from(best.score) * 3 + u16::from(secondary)) / 4).min(100) as u8
}

pub(crate) fn task_readiness_band(score: u8) -> &'static str {
    match score {
        80..=100 => "ready",
        60..=79 => "mostly_ready",
        35..=59 => "partial",
        1..=34 => "weak",
        _ => "blocked",
    }
}

pub(crate) fn task_readiness_summary(
    score: u8,
    band: &'static str,
    candidates: &[TaskReadinessCandidate],
    missing_gap_notes: &[String],
) -> String {
    match candidates.first() {
        Some(best) => format!(
            "Task readiness is {band} ({score}/100). Top local candidate is `{}` for {} with score {} and risk {}.",
            best.skill_name,
            best.agent,
            best.score,
            best.enabled_scope_risk_state.risk_level
        ),
        None if missing_gap_notes.is_empty() => {
            "Task readiness is blocked because no local candidate evidence was available."
                .to_string()
        }
        None => format!(
            "Task readiness is blocked because no local candidate evidence was available. {}",
            missing_gap_notes.join(" ")
        ),
    }
}

pub(crate) fn task_readiness_blocker_notes(candidates: &[TaskReadinessCandidate]) -> Vec<String> {
    let mut notes = candidates
        .iter()
        .flat_map(|candidate| candidate.blocker_risk_notes.iter().cloned())
        .collect::<Vec<_>>();
    if notes.is_empty() {
        notes.push(
            "No candidate-level blockers were found in local catalog/rule/conflict/analysis evidence."
                .to_string(),
        );
    }
    notes.sort();
    notes.dedup();
    notes.truncate(10);
    notes
}

pub(crate) fn routing_confidence_safety_flags() -> RoutingConfidenceSafetyFlags {
    RoutingConfidenceSafetyFlags {
        read_only: true,
        provider_request_sent: false,
        write_back_allowed: false,
        script_execution_allowed: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        triage_mutation_allowed: false,
        credential_accessed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
    }
}

pub(crate) fn agent_readiness_safety_flags() -> AgentReadinessSafetyFlags {
    AgentReadinessSafetyFlags {
        read_only: true,
        app_local_only: true,
        provider_request_sent: false,
        write_back_allowed: false,
        write_actions_available: false,
        skill_files_mutated: false,
        agent_config_mutated: false,
        script_execution_allowed: false,
        execution_actions_available: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        triage_mutation_allowed: false,
        credential_accessed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        cloud_sync_performed: false,
        telemetry_emitted: false,
    }
}

pub(crate) fn empty_agent_readiness_comparison(
    task: String,
    filters: AgentReadinessComparisonFilters,
    catalog_available: bool,
    note: &str,
) -> AgentReadinessComparisonResult {
    AgentReadinessComparisonResult {
        generated_by: "deterministic-service",
        catalog_available,
        filters: filters.clone(),
        summary: AgentReadinessComparisonSummary {
            agent_count: 0,
            candidate_count: 0,
            ready_agent_count: 0,
            partial_agent_count: 0,
            blocked_agent_count: 0,
            gap_issue_count: 1,
            recommended_agent: None,
            summary: note.to_string(),
        },
        agent_rows: Vec::new(),
        recommended_agent: None,
        gap_issue_rows: vec![AgentReadinessGapIssueRow {
            source: "task.compareAgentReadiness",
            severity: "high",
            agent: "all".to_string(),
            title: "No cross-agent readiness candidates".to_string(),
            detail: note.to_string(),
            evidence_refs: Vec::new(),
        }],
        evidence_references: Vec::new(),
        prompt_request: AgentReadinessPromptRequest {
            available: false,
            preview_method: "llm.previewPrompt",
            confirm_method: "llm.confirmPromptAndSend",
            action: "task_readiness",
            request: LlmPreviewPromptParams {
                action: LlmPromptActionKind::TaskReadiness,
                profile_id: None,
                app_language: None,
                skill_instance_id: None,
                instance_ids: Vec::new(),
                agents: Vec::new(),
                analysis_kind: None,
                user_intent: Some(task),
            },
            note: "Prompt preview is unavailable until local catalog evidence produces cross-agent candidates."
                .to_string(),
        },
        aggregation: empty_aggregation_runtime(
            TASK_AGGREGATION_TIMEOUT_MS,
            filters.limit_per_agent,
            "task.compareAgentReadiness",
            note.to_string(),
        ),
        safety_flags: agent_readiness_safety_flags(),
    }
}

pub(crate) fn normalize_agent_filter_list(agents: Vec<String>) -> Vec<String> {
    let mut normalized = agents
        .into_iter()
        .filter_map(|agent| normalize_agent_label(&agent))
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect::<Vec<_>>();
    normalized.sort_by_key(|agent| agent_readiness_agent_order(agent));
    normalized
}

pub(crate) fn normalize_agent_label(agent: &str) -> Option<String> {
    let normalized = agent.trim().to_ascii_lowercase().replace(['_', ' '], "-");
    let canonical = match normalized.as_str() {
        "" => return None,
        "claude" | "claude-code" | "claudecode" => "claude-code",
        "codex" => "codex",
        "opencode" | "open-code" => "opencode",
        "pi" => "pi",
        "hermes" => "hermes",
        "openclaw" | "open-claw" => "openclaw",
        other => other,
    };
    Some(canonical.to_string())
}

pub(crate) fn agent_readiness_agents_for_comparison(
    skills: &[SkillRecord],
    adapter_ctx: &AdapterContext,
    requested_agents: &[String],
) -> Vec<String> {
    if !requested_agents.is_empty() {
        return requested_agents.to_vec();
    }
    let mut agents = skills
        .iter()
        .filter_map(|skill| normalize_agent_label(&skill.agent))
        .filter(|agent| agent != "tool-global")
        .collect::<BTreeSet<_>>();
    for diagnostic in list_adapter_diagnostics(adapter_ctx) {
        let present = diagnostic.config.detected_count > 0
            || diagnostic.roots.iter().any(|root| root.exists)
            || skills.iter().any(|skill| skill.agent == diagnostic.agent);
        if present {
            agents.insert(diagnostic.agent.to_string());
        }
    }
    let mut agents = agents.into_iter().collect::<Vec<_>>();
    agents.sort_by_key(|agent| agent_readiness_agent_order(agent));
    agents
}

pub(crate) fn agent_readiness_agent_order(agent: &str) -> usize {
    match agent {
        "claude-code" => 0,
        "codex" => 1,
        "opencode" => 2,
        "pi" => 3,
        "hermes" => 4,
        "openclaw" => 5,
        _ => 99,
    }
}

pub(crate) fn agent_readiness_display_name(agent: &str) -> String {
    match agent {
        "claude-code" => "Claude Code",
        "codex" => "Codex",
        "opencode" => "opencode",
        "pi" => "Pi",
        "hermes" => "Hermes",
        "openclaw" => "OpenClaw",
        other => other,
    }
    .to_string()
}

pub(crate) fn agent_readiness_row_from_results(
    agent: &str,
    readiness: &TaskReadinessResult,
    ranking: &SkillRouteRankingResult,
    accuracy_context: Option<AgentReadinessAccuracyContext>,
    benchmark_context: Option<AgentReadinessBenchmarkContext>,
) -> AgentReadinessComparisonRow {
    let best_route = ranking.route_candidates.first();
    let best_candidate = best_route.map(|route| AgentReadinessBestCandidate {
        instance_id: route.instance_id.clone(),
        definition_id: route.definition_id.clone(),
        skill_name: route.skill_name.clone(),
        scope: route.scope.clone(),
        enabled: route.enabled,
        state: route.state.clone(),
        readiness_score: route.readiness_score,
        readiness_band: route.readiness_band,
        routing_confidence_score: route.confidence_score,
        routing_confidence_band: route.confidence_band,
        quality_score: route.quality_score,
    });
    let blocker_count = readiness
        .candidate_skills
        .iter()
        .map(|candidate| candidate.blocker_risk_notes.len())
        .sum::<usize>();
    let gap_count = readiness.missing_gap_notes.len()
        + readiness
            .candidate_skills
            .iter()
            .map(|candidate| candidate.missing_gap_notes.len())
            .sum::<usize>();
    let mut reasons = Vec::new();
    if let Some(route) = best_route {
        reasons.extend(route.match_reasons.iter().take(3).cloned());
        reasons.extend(route.confidence_rationale.iter().take(2).cloned());
    } else {
        reasons.push("No visible route candidate for this agent matched the task.".to_string());
    }
    let mut blocker_notes = readiness
        .candidate_skills
        .iter()
        .flat_map(|candidate| candidate.blocker_risk_notes.iter().cloned())
        .collect::<Vec<_>>();
    if blocker_notes.is_empty() && readiness.candidate_skills.is_empty() {
        blocker_notes.push("No candidate evidence was available for this agent.".to_string());
    }
    blocker_notes.sort();
    blocker_notes.dedup();
    blocker_notes.truncate(6);
    let mut gap_notes = readiness.missing_gap_notes.clone();
    gap_notes.extend(
        readiness
            .candidate_skills
            .iter()
            .flat_map(|candidate| candidate.missing_gap_notes.iter().cloned()),
    );
    gap_notes.sort();
    gap_notes.dedup();
    gap_notes.truncate(6);
    let routing_confidence_score = ranking.overall_confidence_score;
    let comparison_score = agent_readiness_comparison_score(
        readiness.score,
        routing_confidence_score,
        accuracy_context.as_ref(),
        benchmark_context.as_ref(),
    );
    AgentReadinessComparisonRow {
        rank: 0,
        agent: agent.to_string(),
        display_name: agent_readiness_display_name(agent),
        comparison_score,
        readiness_score: readiness.score,
        readiness_band: readiness.band,
        routing_confidence_score,
        routing_confidence_band: ranking.overall_confidence_band,
        candidate_count: readiness.candidate_skills.len(),
        best_candidate,
        enabled_scope_risk_state: best_route.map(|route| route.enabled_scope_risk_state.clone()),
        blocker_count,
        gap_count,
        reasons,
        blocker_notes,
        gap_notes,
        routing_accuracy_context: accuracy_context,
        benchmark_context,
        evidence_refs: best_route
            .map(|route| route.evidence_refs.clone())
            .unwrap_or_default(),
    }
}

pub(crate) fn agent_readiness_comparison_score(
    readiness_score: u8,
    routing_confidence_score: u8,
    accuracy_context: Option<&AgentReadinessAccuracyContext>,
    benchmark_context: Option<&AgentReadinessBenchmarkContext>,
) -> u8 {
    let mut score =
        ((u16::from(readiness_score) * 3 + u16::from(routing_confidence_score) * 2) / 5) as i16;
    if let Some(context) = accuracy_context {
        score -= (context.regression_count as i16 * 6).min(18);
        score -= (context.benchmark_gap_count as i16 * 4).min(12);
        if context.trace_count > 0 && context.accuracy_rate >= 0.8 {
            score += 3;
        }
    }
    if let Some(context) = benchmark_context {
        score -= (context.gap_count as i16 * 4).min(12);
        score -= (context.regression_count as i16 * 6).min(18);
        if context.evaluated_count > 0 && context.gap_count == 0 {
            score += 2;
        }
    }
    score.clamp(0, 100) as u8
}

pub(crate) fn agent_readiness_gap_issue_rows(
    row: &AgentReadinessComparisonRow,
) -> Vec<AgentReadinessGapIssueRow> {
    let mut issues = Vec::new();
    if row.candidate_count == 0 {
        issues.push(AgentReadinessGapIssueRow {
            source: "task.checkReadiness",
            severity: "high",
            agent: row.agent.clone(),
            title: "No candidate skill for agent".to_string(),
            detail: "No visible skill candidate matched the task for this agent.".to_string(),
            evidence_refs: Vec::new(),
        });
    }
    for note in &row.gap_notes {
        issues.push(AgentReadinessGapIssueRow {
            source: "task.checkReadiness",
            severity: "medium",
            agent: row.agent.clone(),
            title: "Readiness gap".to_string(),
            detail: note.clone(),
            evidence_refs: row.evidence_refs.clone(),
        });
    }
    for note in &row.blocker_notes {
        issues.push(AgentReadinessGapIssueRow {
            source: "task.checkReadiness",
            severity: "high",
            agent: row.agent.clone(),
            title: "Readiness blocker or risk".to_string(),
            detail: note.clone(),
            evidence_refs: row.evidence_refs.clone(),
        });
    }
    if let Some(context) = &row.routing_accuracy_context {
        if context.benchmark_gap_count > 0 || context.regression_count > 0 {
            issues.push(AgentReadinessGapIssueRow {
                source: "routing.accuracyDashboard",
                severity: if context.regression_count > 0 {
                    "critical"
                } else {
                    "medium"
                },
                agent: row.agent.clone(),
                title: "Routing accuracy context requires review".to_string(),
                detail: format!(
                    "{} benchmark gap(s) and {} regression(s) are associated with this agent.",
                    context.benchmark_gap_count, context.regression_count
                ),
                evidence_refs: row.evidence_refs.clone(),
            });
        }
    }
    if let Some(context) = &row.benchmark_context {
        if context.gap_count > 0 || context.regression_count > 0 {
            issues.push(AgentReadinessGapIssueRow {
                source: "task.evaluateBenchmarks",
                severity: if context.regression_count > 0 {
                    "critical"
                } else {
                    "medium"
                },
                agent: row.agent.clone(),
                title: "Benchmark context requires review".to_string(),
                detail: format!(
                    "{} benchmark gap(s) and {} regression(s) are associated with this agent.",
                    context.gap_count, context.regression_count
                ),
                evidence_refs: row.evidence_refs.clone(),
            });
        }
    }
    issues
}

pub(crate) fn agent_readiness_recommendation(
    row: &AgentReadinessComparisonRow,
) -> AgentReadinessRecommendation {
    AgentReadinessRecommendation {
        agent: row.agent.clone(),
        display_name: row.display_name.clone(),
        comparison_score: row.comparison_score,
        readiness_score: row.readiness_score,
        routing_confidence_score: row.routing_confidence_score,
        skill_name: row
            .best_candidate
            .as_ref()
            .map(|candidate| candidate.skill_name.clone()),
        reason: match &row.best_candidate {
            Some(candidate) => format!(
                "{} has the strongest local readiness/routing score for `{}` with risk {}.",
                row.display_name,
                candidate.skill_name,
                row.enabled_scope_risk_state
                    .as_ref()
                    .map(|state| state.risk_level)
                    .unwrap_or("unknown")
            ),
            None => format!(
                "{} is ranked highest, but no concrete candidate was available.",
                row.display_name
            ),
        },
    }
}

pub(crate) fn agent_readiness_summary(
    rows: &[AgentReadinessComparisonRow],
    gap_issue_rows: &[AgentReadinessGapIssueRow],
    recommended_agent: &Option<AgentReadinessRecommendation>,
) -> AgentReadinessComparisonSummary {
    let candidate_count = rows.iter().map(|row| row.candidate_count).sum();
    let ready_agent_count = rows
        .iter()
        .filter(|row| matches!(row.readiness_band, "ready" | "mostly_ready"))
        .count();
    let blocked_agent_count = rows
        .iter()
        .filter(|row| row.candidate_count == 0 || row.readiness_band == "blocked")
        .count();
    let partial_agent_count = rows
        .len()
        .saturating_sub(ready_agent_count + blocked_agent_count);
    let summary = if let Some(recommended) = recommended_agent {
        format!(
            "Compared {} agent(s) and {} candidate skill(s); recommended {} with comparison score {}/100.",
            rows.len(),
            candidate_count,
            recommended.display_name,
            recommended.comparison_score
        )
    } else if rows.is_empty() {
        "No agent readiness rows were available for the selected filters.".to_string()
    } else {
        format!(
            "Compared {} agent(s), but no agent produced a usable candidate for recommendation.",
            rows.len()
        )
    };
    AgentReadinessComparisonSummary {
        agent_count: rows.len(),
        candidate_count,
        ready_agent_count,
        partial_agent_count,
        blocked_agent_count,
        gap_issue_count: gap_issue_rows.len(),
        recommended_agent: recommended_agent
            .as_ref()
            .map(|recommendation| recommendation.agent.clone()),
        summary,
    }
}

pub(crate) fn agent_readiness_accuracy_context(
    dashboard: RoutingAccuracyDashboardResult,
) -> BTreeMap<String, AgentReadinessAccuracyContext> {
    dashboard
        .agent_rows
        .into_iter()
        .map(|row| {
            (
                row.agent,
                AgentReadinessAccuracyContext {
                    trace_count: row.trace_count,
                    accuracy_rate: row.accuracy_rate,
                    benchmark_count: row.benchmark_count,
                    benchmark_gap_count: row.benchmark_gap_count,
                    regression_count: row.regression_count,
                    recent_evidence_count: row.recent_evidence_count,
                    notes: row.notes,
                },
            )
        })
        .collect()
}

pub(crate) fn agent_readiness_benchmark_context(
    evaluation: TaskBenchmarkEvaluationResult,
) -> BTreeMap<String, AgentReadinessBenchmarkContext> {
    let mut by_agent: BTreeMap<String, AgentReadinessBenchmarkContext> = BTreeMap::new();
    for item in evaluation.benchmark_results {
        let Some(route) = item.top_route else {
            continue;
        };
        let context = by_agent.entry(route.agent).or_default();
        context.evaluated_count += 1;
        if matches!(
            item.expected_match_status,
            "expected_match" | "acceptable_match"
        ) {
            context.matched_count += 1;
        } else {
            context.gap_count += 1;
        }
        context.notes.extend(item.gap_notes);
        context.notes.extend(item.blocker_notes);
    }
    for context in by_agent.values_mut() {
        context.notes.sort();
        context.notes.dedup();
        context.notes.truncate(6);
    }
    by_agent
}

pub(crate) fn task_benchmark_safety_flags() -> TaskBenchmarkSafetyFlags {
    TaskBenchmarkSafetyFlags {
        read_only: true,
        provider_request_sent: false,
        write_back_allowed: false,
        script_execution_allowed: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        triage_mutation_allowed: false,
        credential_accessed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
    }
}

pub(crate) fn trace_import_safety_flags() -> TraceImportSafetyFlags {
    TraceImportSafetyFlags {
        read_only: true,
        app_local_only: true,
        provider_request_sent: false,
        write_back_allowed: false,
        skill_files_mutated: false,
        agent_config_mutated: false,
        script_execution_allowed: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        triage_mutation_allowed: false,
        credential_accessed: false,
        raw_secret_returned: false,
        raw_trace_persisted: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        cloud_sync_performed: false,
        telemetry_emitted: false,
    }
}

pub(crate) fn agent_session_review_safety_flags() -> AgentSessionSkillReviewSafetyFlags {
    AgentSessionSkillReviewSafetyFlags {
        read_only: true,
        app_local_only: true,
        provider_request_sent: false,
        write_back_allowed: false,
        write_actions_available: false,
        skill_files_mutated: false,
        agent_config_mutated: false,
        script_execution_allowed: false,
        execution_actions_available: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        triage_mutation_allowed: false,
        credential_accessed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        cloud_sync_performed: false,
        telemetry_emitted: false,
    }
}

pub(crate) fn agent_session_review_redaction_summary_default(
) -> AgentSessionSkillReviewRedactionSummary {
    AgentSessionSkillReviewRedactionSummary {
        status: "redacted-local-only".to_string(),
        redacted_value_count: 0,
        redacted_fields: Vec::new(),
        placeholders: vec![
            "$HOME".to_string(),
            "<project-root>".to_string(),
            "<project-cwd>".to_string(),
            "<app-data-dir>".to_string(),
            "<redacted>".to_string(),
            "<redacted-url>".to_string(),
        ],
        raw_trace_persisted: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_secret_returned: false,
    }
}

pub(crate) fn agent_session_review_redaction_summary_from(
    summary: LlmPromptRedactionSummary,
) -> AgentSessionSkillReviewRedactionSummary {
    AgentSessionSkillReviewRedactionSummary {
        status: "redacted-local-only".to_string(),
        redacted_value_count: summary.redacted_value_count,
        redacted_fields: summary.redacted_fields,
        placeholders: summary
            .placeholders
            .into_iter()
            .map(str::to_string)
            .collect(),
        raw_trace_persisted: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_secret_returned: summary.raw_secret_returned,
    }
}

pub(crate) fn remediation_history_safety_flags() -> RemediationHistorySafetyFlags {
    RemediationHistorySafetyFlags {
        read_only: true,
        app_local_only: true,
        provider_request_sent: false,
        write_back_allowed: false,
        write_actions_available: false,
        skill_files_mutated: false,
        agent_config_mutated: false,
        script_execution_allowed: false,
        execution_actions_available: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        rollback_performed: false,
        triage_mutation_allowed: false,
        credential_accessed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        cloud_sync_performed: false,
        telemetry_emitted: false,
    }
}

pub(crate) fn remediation_history_redaction_summary_default() -> RemediationHistoryRedactionSummary
{
    RemediationHistoryRedactionSummary {
        status: "redacted-local-only".to_string(),
        redacted_value_count: 0,
        redacted_fields: Vec::new(),
        placeholders: vec![
            "$HOME".to_string(),
            "<project-root>".to_string(),
            "<project-cwd>".to_string(),
            "<app-data-dir>".to_string(),
            "<redacted>".to_string(),
            "<redacted-url>".to_string(),
        ],
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        raw_secret_returned: false,
    }
}

pub(crate) fn remediation_history_redaction_summary_from(
    summary: LlmPromptRedactionSummary,
) -> RemediationHistoryRedactionSummary {
    RemediationHistoryRedactionSummary {
        status: "redacted-local-only".to_string(),
        redacted_value_count: summary.redacted_value_count,
        redacted_fields: summary.redacted_fields,
        placeholders: summary
            .placeholders
            .into_iter()
            .map(str::to_string)
            .collect(),
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        raw_secret_returned: summary.raw_secret_returned,
    }
}

pub(crate) fn remediation_history_record_sort(
    left: &RemediationHistoryRecord,
    right: &RemediationHistoryRecord,
) -> std::cmp::Ordering {
    right
        .updated_at
        .cmp(&left.updated_at)
        .then_with(|| right.created_at.cmp(&left.created_at))
        .then_with(|| left.title.cmp(&right.title))
        .then_with(|| left.id.cmp(&right.id))
}

pub(crate) fn remediation_history_matches(
    filters: &RemediationHistoryFilters,
    record: &RemediationHistoryRecord,
) -> bool {
    if let Some(agent) = filters.agent.as_deref() {
        if record.agent.as_deref() != Some(agent) {
            return false;
        }
    }
    if let Some(status) = filters.status.as_deref() {
        if record.status != status {
            return false;
        }
    }
    if let Some(decision) = filters.decision.as_deref() {
        if record.decision != decision {
            return false;
        }
    }
    if let Some(source_item_ref) = filters.source_item_ref.as_deref() {
        if !record
            .source_item_refs
            .iter()
            .chain(record.batch_review_item_ids.iter())
            .any(|item| item == source_item_ref)
        {
            return false;
        }
    }
    if let Some(recurrence_key) = filters.recurrence_key.as_deref() {
        if record.recurrence_key.as_deref() != Some(recurrence_key) {
            return false;
        }
    }
    true
}

pub(crate) fn remediation_history_summary(
    total_count: usize,
    records: &[RemediationHistoryRecord],
) -> RemediationHistorySummary {
    let mut summary = RemediationHistorySummary {
        total_count,
        returned_count: records.len(),
        latest_recorded_at: records.iter().map(|record| record.updated_at).max(),
        ..Default::default()
    };
    let mut recurrence_keys = BTreeSet::new();
    for record in records {
        *summary
            .decision_counts
            .entry(record.decision.clone())
            .or_default() += 1;
        *summary
            .status_counts
            .entry(record.status.clone())
            .or_default() += 1;
        if record.reopened {
            summary.reopened_count += 1;
        }
        if let Some(key) = record.recurrence_key.as_deref() {
            recurrence_keys.insert(key.to_string());
        }
        if !record.blocker_notes.is_empty() {
            summary.blocker_count += 1;
        }
        if !record.readiness_improvement_notes.is_empty() {
            summary.readiness_improvement_count += 1;
        }
        if !record.routing_improvement_notes.is_empty() {
            summary.routing_improvement_count += 1;
        }
    }
    summary.recurrence_group_count = recurrence_keys.len();
    summary.summary = format!(
        "Returned {} of {} app-local remediation history record(s), including {} reopened record(s), {} recurrence group(s), {} blocker-bearing record(s), {} readiness improvement record(s), and {} routing improvement record(s).",
        summary.returned_count,
        summary.total_count,
        summary.reopened_count,
        summary.recurrence_group_count,
        summary.blocker_count,
        summary.readiness_improvement_count,
        summary.routing_improvement_count
    );
    summary
}

pub(crate) fn remediation_history_recurrence_rows(
    records: &[RemediationHistoryRecord],
) -> Vec<RemediationHistoryRecurrenceRow> {
    let mut grouped: BTreeMap<String, Vec<&RemediationHistoryRecord>> = BTreeMap::new();
    for record in records {
        if let Some(key) = record.recurrence_key.as_deref() {
            grouped.entry(key.to_string()).or_default().push(record);
        }
    }
    let mut rows = grouped
        .into_iter()
        .map(|(recurrence_key, mut members)| {
            members.sort_by_key(|record| std::cmp::Reverse(record.updated_at));
            let latest = members[0];
            let mut source_item_refs = members
                .iter()
                .flat_map(|record| {
                    record
                        .source_item_refs
                        .iter()
                        .chain(record.batch_review_item_ids.iter())
                        .cloned()
                })
                .collect::<Vec<_>>();
            source_item_refs.sort();
            source_item_refs.dedup();
            source_item_refs.truncate(12);
            let mut evidence_refs = members
                .iter()
                .flat_map(|record| record.evidence_refs.iter().cloned())
                .collect::<Vec<_>>();
            evidence_refs.sort();
            evidence_refs.dedup();
            evidence_refs.truncate(12);
            RemediationHistoryRecurrenceRow {
                recurrence_key,
                record_count: members.len(),
                reopened_count: members.iter().filter(|record| record.reopened).count(),
                latest_status: latest.status.clone(),
                latest_decision: latest.decision.clone(),
                latest_recorded_at: latest.updated_at,
                source_item_refs,
                evidence_refs,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .record_count
            .cmp(&left.record_count)
            .then_with(|| right.latest_recorded_at.cmp(&left.latest_recorded_at))
            .then_with(|| left.recurrence_key.cmp(&right.recurrence_key))
    });
    rows
}

#[derive(Debug, Clone, Default)]
pub(crate) struct RoutingAccuracyAgentAggregate {
    pub(crate) outcomes: RoutingAccuracyOutcomeCounts,
    pub(crate) benchmark_count: usize,
    pub(crate) benchmark_matched_count: usize,
    pub(crate) benchmark_gap_count: usize,
    pub(crate) regression_count: usize,
    pub(crate) recent_evidence_count: usize,
    pub(crate) notes: Vec<String>,
}

impl RoutingAccuracyAgentAggregate {
    pub(crate) fn record_trace(&mut self, outcome: &'static str) {
        routing_accuracy_increment_counts(&mut self.outcomes, outcome);
        self.recent_evidence_count += 1;
    }

    pub(crate) fn into_row(mut self, agent: String) -> RoutingAccuracyAgentRow {
        let known = self.outcomes.hit
            + self.outcomes.miss
            + self.outcomes.wrong_pick
            + self.outcomes.ambiguous;
        let trace_count = known + self.outcomes.unknown;
        let accuracy_rate = routing_accuracy_rate(self.outcomes.hit, known);
        if self.benchmark_gap_count > 0 {
            self.notes.push(format!(
                "{} benchmark gap(s) require review.",
                self.benchmark_gap_count
            ));
        }
        if self.regression_count > 0 {
            self.notes.push(format!(
                "{} routing regression(s) detected.",
                self.regression_count
            ));
        }
        self.notes.sort();
        self.notes.dedup();
        RoutingAccuracyAgentRow {
            agent,
            trace_count,
            outcomes: self.outcomes,
            accuracy_rate,
            benchmark_count: self.benchmark_count,
            benchmark_matched_count: self.benchmark_matched_count,
            benchmark_gap_count: self.benchmark_gap_count,
            regression_count: self.regression_count,
            recent_evidence_count: self.recent_evidence_count,
            notes: self.notes,
        }
    }
}
