use super::*;

pub(crate) fn llm_prompt_run_redaction_summary_from(
    preview_summary: LlmPromptRedactionSummary,
    request_summary: LlmPromptRedactionSummary,
) -> LlmPromptRunRedactionSummary {
    let mut redacted_fields = preview_summary.redacted_fields;
    redacted_fields.extend(request_summary.redacted_fields);
    redacted_fields.sort();
    redacted_fields.dedup();
    let mut placeholders = preview_summary
        .placeholders
        .into_iter()
        .chain(request_summary.placeholders)
        .map(str::to_string)
        .collect::<Vec<_>>();
    placeholders.sort();
    placeholders.dedup();
    LlmPromptRunRedactionSummary {
        status: "redacted-local-only".to_string(),
        redacted_value_count: preview_summary
            .redacted_value_count
            .saturating_add(request_summary.redacted_value_count),
        redacted_fields,
        placeholders,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        raw_secret_returned: preview_summary.raw_secret_returned
            || request_summary.raw_secret_returned,
    }
}

pub(crate) fn llm_prompt_run_safety_flags(
    provider_request_sent: bool,
    credential_accessed: bool,
) -> LlmPromptRunSafetyFlags {
    LlmPromptRunSafetyFlags {
        app_local_only: true,
        provider_request_sent,
        credential_accessed,
        draft_copy_only: true,
        write_back_allowed: false,
        write_actions_available: false,
        skill_files_mutated: false,
        agent_config_mutated: false,
        script_execution_allowed: false,
        execution_actions_available: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        triage_mutation_allowed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        cloud_sync_performed: false,
        telemetry_emitted: false,
    }
}

pub(crate) fn llm_provider_observability_safety_flags() -> LlmProviderObservabilitySafetyFlags {
    LlmProviderObservabilitySafetyFlags {
        read_only: true,
        app_local_only: true,
        provider_request_sent: false,
        credential_accessed: false,
        draft_copy_only: true,
        write_back_allowed: false,
        write_actions_available: false,
        skill_files_mutated: false,
        agent_config_mutated: false,
        script_execution_allowed: false,
        execution_actions_available: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        triage_mutation_allowed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        unredacted_paths_returned: false,
        cloud_sync_performed: false,
        telemetry_emitted: false,
    }
}

pub(crate) fn model_task_match_safety_flags(read_only: bool) -> ModelTaskMatchSafetyFlags {
    ModelTaskMatchSafetyFlags {
        read_only,
        app_local_only: true,
        provider_request_sent: false,
        credential_accessed: false,
        draft_copy_only: true,
        write_back_allowed: false,
        write_actions_available: false,
        skill_files_mutated: false,
        agent_config_mutated: false,
        script_execution_allowed: false,
        execution_actions_available: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        triage_mutation_allowed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        unredacted_paths_returned: false,
        cloud_sync_performed: false,
        telemetry_emitted: false,
    }
}

pub(crate) fn model_task_match_record_sort(
    left: &ModelTaskMatchRecord,
    right: &ModelTaskMatchRecord,
) -> std::cmp::Ordering {
    right
        .updated_at
        .cmp(&left.updated_at)
        .then_with(|| right.created_at.cmp(&left.created_at))
        .then_with(|| left.provider.cmp(&right.provider))
        .then_with(|| left.model.cmp(&right.model))
        .then_with(|| left.id.cmp(&right.id))
}

pub(crate) fn normalize_model_task_match_status(value: Option<&str>) -> String {
    let normalized = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("unknown")
        .to_ascii_lowercase()
        .replace('-', "_");
    match normalized.as_str() {
        "fit" | "hit" | "matched" | "match" | "succeeded" | "success" => "fit".to_string(),
        "partial" | "partial_fit" | "ambiguous" | "warning" => "partial_fit".to_string(),
        "mismatch" | "miss" | "wrong_pick" | "failed" | "failure" | "error" => {
            "mismatch".to_string()
        }
        _ => "unknown".to_string(),
    }
}

pub(crate) fn normalize_model_task_source_kind(value: Option<&str>) -> String {
    let normalized = value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .unwrap_or("manual")
        .to_ascii_lowercase()
        .replace('-', "_");
    match normalized.as_str() {
        "prompt_run" | "provider_call" | "manual" | "benchmark" => normalized,
        _ => "manual".to_string(),
    }
}

pub(crate) fn sanitize_model_task_match_id(id: &str) -> String {
    let mut value = id
        .trim()
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | ':' | '.') {
                ch
            } else {
                '-'
            }
        })
        .collect::<String>();
    while value.contains("--") {
        value = value.replace("--", "-");
    }
    value.trim_matches('-').chars().take(96).collect::<String>()
}

pub(crate) fn stable_model_task_match_id(task: &str, model: &str, now: i64) -> String {
    let mut hasher = Sha256::new();
    hasher.update(task.as_bytes());
    hasher.update(b"\0");
    hasher.update(model.as_bytes());
    hasher.update(b"\0");
    hasher.update(now.to_string().as_bytes());
    let digest = hasher.finalize();
    format!("model-task-{}", hex_prefix(&digest, 12))
}

pub(crate) fn model_task_match_redaction_summary_from(
    summary: LlmPromptRedactionSummary,
) -> LlmPromptRunRedactionSummary {
    LlmPromptRunRedactionSummary {
        status: "redacted-local-only".to_string(),
        redacted_value_count: summary.redacted_value_count,
        redacted_fields: summary.redacted_fields,
        placeholders: summary
            .placeholders
            .into_iter()
            .map(ToOwned::to_owned)
            .collect(),
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        raw_secret_returned: false,
    }
}

#[derive(Debug, Default)]
pub(crate) struct ProviderObservabilityFilters {
    profile_id: Option<String>,
    provider: Option<String>,
    model: Option<String>,
    status: Option<String>,
    action: Option<String>,
    window_days: Option<i64>,
    start_at: Option<i64>,
    end_at: Option<i64>,
}

impl ProviderObservabilityFilters {
    pub(crate) fn from_params(params: &LlmProviderObservabilityParams) -> Self {
        let window_days = params.window_days.map(|days| days.clamp(1, 3_650));
        let explicit_start_at = params.start_at.filter(|value| *value >= 0);
        let explicit_end_at = params.end_at.filter(|value| *value >= 0);
        let (mut start_at, mut end_at) = match (explicit_start_at, explicit_end_at) {
            (None, None) => {
                if let Some(days) = window_days {
                    let end = unix_timestamp_millis();
                    let duration = days.saturating_mul(86_400_000);
                    (Some(end.saturating_sub(duration)), Some(end))
                } else {
                    (None, None)
                }
            }
            values => values,
        };
        if let (Some(start), Some(end)) = (start_at, end_at) {
            if start > end {
                start_at = Some(end);
                end_at = Some(start);
            }
        }
        Self {
            profile_id: normalized_observability_filter(params.profile_id.as_deref()),
            provider: normalized_observability_filter(params.provider.as_deref()),
            model: normalized_observability_filter(params.model.as_deref()),
            status: normalized_observability_filter(params.status.as_deref()),
            action: normalized_observability_filter(params.action.as_deref()),
            window_days,
            start_at,
            end_at,
        }
    }

    pub(crate) fn matches_prompt_run(&self, run: &LlmPromptRunRecord) -> bool {
        self.profile_id
            .as_deref()
            .is_none_or(|filter| run.profile_id.eq_ignore_ascii_case(filter))
            && self
                .provider
                .as_deref()
                .is_none_or(|filter| run.provider.eq_ignore_ascii_case(filter))
            && self
                .model
                .as_deref()
                .is_none_or(|filter| run.model.eq_ignore_ascii_case(filter))
            && self
                .status
                .as_deref()
                .is_none_or(|filter| run.status.eq_ignore_ascii_case(filter))
            && self.action.as_deref().is_none_or(|filter| {
                run.action.eq_ignore_ascii_case(filter)
                    || run.request_kind.eq_ignore_ascii_case(filter)
                    || run
                        .analysis_kind
                        .as_deref()
                        .is_some_and(|kind| kind.eq_ignore_ascii_case(filter))
            })
            && self.matches_timestamp(run.completed_at)
    }

    pub(crate) fn matches_provider_call(&self, metadata: &ProviderCallMetadata) -> bool {
        self.profile_id
            .as_deref()
            .is_none_or(|filter| metadata.profile_id.eq_ignore_ascii_case(filter))
            && self
                .provider
                .as_deref()
                .is_none_or(|filter| metadata.provider_type.as_str().eq_ignore_ascii_case(filter))
            && self
                .model
                .as_deref()
                .is_none_or(|filter| metadata.model.eq_ignore_ascii_case(filter))
            && self
                .status
                .as_deref()
                .is_none_or(|filter| metadata.status.eq_ignore_ascii_case(filter))
            && self
                .action
                .as_deref()
                .is_none_or(|filter| metadata.action_type.eq_ignore_ascii_case(filter))
            && self.matches_timestamp(metadata.timestamp)
    }

    pub(crate) fn matches_profile(&self, profile: &ProviderProfileRecord) -> bool {
        self.profile_id
            .as_deref()
            .is_none_or(|filter| profile.id.eq_ignore_ascii_case(filter))
            && self
                .provider
                .as_deref()
                .is_none_or(|filter| profile.provider_type.as_str().eq_ignore_ascii_case(filter))
            && self
                .model
                .as_deref()
                .is_none_or(|filter| profile.model.eq_ignore_ascii_case(filter))
    }

    pub(crate) fn applied_filters(
        &self,
        params: &LlmProviderObservabilityParams,
        limit: usize,
    ) -> LlmProviderObservabilityAppliedFilters {
        LlmProviderObservabilityAppliedFilters {
            profile_id: self.profile_id.clone(),
            provider: self.provider.clone(),
            model: self.model.clone(),
            status: self.status.clone(),
            action: self.action.clone(),
            window_days: self.window_days,
            start_at: self.start_at,
            end_at: self.end_at,
            limit,
            include_history: params.include_history.unwrap_or(true),
            include_budget_hints: params.include_budget_hints.unwrap_or(true),
            include_retention_recommendations: params
                .include_retention_recommendations
                .unwrap_or(true),
            include_evidence: params.include_evidence.unwrap_or(true),
            aggregation_uses_full_range: true,
        }
    }

    fn matches_timestamp(&self, timestamp: i64) -> bool {
        self.start_at.is_none_or(|start| timestamp >= start)
            && self.end_at.is_none_or(|end| timestamp <= end)
    }
}

#[derive(Debug, Default)]
pub(crate) struct ProviderObservabilityGroupAccumulator {
    provider: String,
    model: String,
    destination_host: String,
    profile_ids: BTreeSet<String>,
    prompt_run_count: usize,
    call_metadata_count: usize,
    recorded_provider_request_count: usize,
    recorded_credential_access_count: usize,
    succeeded_count: usize,
    failed_count: usize,
    estimated_total_tokens: u64,
    estimated_cost_usd: f64,
    latest_activity_at: Option<i64>,
    evidence_refs: BTreeSet<String>,
}

pub(crate) fn normalized_observability_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(|value| value.to_ascii_lowercase())
}

pub(crate) fn provider_observability_call_row(
    metadata: &ProviderCallMetadata,
    index: usize,
    redaction_roots: &[(String, &'static str)],
) -> LlmProviderObservabilityCallRow {
    let id = provider_observability_row_id(
        "provider-call",
        &[
            &metadata.timestamp.to_string(),
            &metadata.profile_id,
            &metadata.action_type,
            &index.to_string(),
        ],
    );
    let evidence_id = format!("provider-call:{id}");
    LlmProviderObservabilityCallRow {
        id,
        source: "provider-call-metadata",
        timestamp: metadata.timestamp,
        action_type: observability_redact(&metadata.action_type, redaction_roots, 120),
        profile_id: observability_redact(&metadata.profile_id, redaction_roots, 160),
        provider: metadata.provider_type.as_str().to_string(),
        model: observability_redact(&metadata.model, redaction_roots, 160),
        destination_host: observability_redact(&metadata.destination_host, redaction_roots, 160),
        status: observability_redact(&metadata.status, redaction_roots, 80),
        error_code: metadata
            .error_code
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 120)),
        error_message: metadata
            .error_message
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 400)),
        duration_ms: metadata.duration_ms,
        estimated_input_tokens: metadata.estimated_input_tokens,
        estimated_output_tokens: metadata.estimated_output_tokens,
        estimated_total_tokens: metadata
            .estimated_input_tokens
            .saturating_add(metadata.estimated_output_tokens),
        estimated_cost_usd: metadata.estimated_cost_usd,
        recorded_provider_request_sent: metadata.provider_request_sent,
        recorded_credential_accessed: metadata.credential_accessed,
        raw_prompt_persisted: metadata.raw_prompt_persisted,
        raw_response_persisted: metadata.raw_response_persisted,
        redaction_status: observability_redact(&metadata.redaction_status, redaction_roots, 160),
        evidence_refs: vec![evidence_id],
    }
}

pub(crate) fn provider_observability_history_row(
    run: &LlmPromptRunRecord,
    index: usize,
    redaction_roots: &[(String, &'static str)],
) -> LlmProviderObservabilityHistoryRow {
    let id = provider_observability_row_id(
        "prompt-run",
        &[
            &run.completed_at.to_string(),
            &run.id,
            &run.profile_id,
            &index.to_string(),
        ],
    );
    let evidence_id = format!("prompt-run:{id}");
    LlmProviderObservabilityHistoryRow {
        id,
        source: "prompt-runs",
        prompt_run_id: observability_redact(&run.id, redaction_roots, 160),
        created_at: run.created_at,
        completed_at: run.completed_at,
        action: observability_redact(&run.action, redaction_roots, 120),
        request_kind: observability_redact(&run.request_kind, redaction_roots, 120),
        analysis_kind: run
            .analysis_kind
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 120)),
        scope: run
            .scope
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 120)),
        instance_id: run
            .instance_id
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 160)),
        instance_ids: redact_normalized_string_list(run.instance_ids.clone(), redaction_roots),
        definition_id: run
            .definition_id
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 160)),
        agent: run
            .agent
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 120)),
        task: run
            .task
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 400)),
        profile_id: observability_redact(&run.profile_id, redaction_roots, 160),
        provider: observability_redact(&run.provider, redaction_roots, 120),
        model: observability_redact(&run.model, redaction_roots, 160),
        destination_host: observability_redact(&run.destination_host, redaction_roots, 160),
        status: observability_redact(&run.status, redaction_roots, 80),
        error_code: run
            .error_code
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 120)),
        error_message: run
            .error_message
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 400)),
        duration_ms: run.duration_ms,
        estimated_input_tokens: run.estimated_input_tokens,
        estimated_output_tokens: run.estimated_output_tokens,
        estimated_total_tokens: run.estimated_total_tokens,
        estimated_cost_usd: run.estimated_cost_usd,
        draft_output_available: run.draft_output.is_some(),
        draft_requires_user_copy: run.draft_requires_user_copy,
        recorded_provider_request_sent: run.provider_request_sent,
        recorded_credential_accessed: run.credential_accessed,
        raw_prompt_persisted: run.raw_prompt_persisted,
        raw_response_persisted: run.raw_response_persisted,
        redaction_status: observability_redact(&run.redaction_summary.status, redaction_roots, 160),
        evidence_refs: vec![evidence_id],
    }
}

pub(crate) fn provider_observability_grouping_rows(
    history_rows: &[LlmProviderObservabilityHistoryRow],
    call_rows: &[LlmProviderObservabilityCallRow],
) -> Vec<LlmProviderObservabilityGroupingRow> {
    let mut groups: BTreeMap<(String, String, String), ProviderObservabilityGroupAccumulator> =
        BTreeMap::new();
    for row in history_rows {
        let key = (
            row.provider.clone(),
            row.model.clone(),
            row.destination_host.clone(),
        );
        let group = groups.entry(key).or_default();
        group.provider = row.provider.clone();
        group.model = row.model.clone();
        group.destination_host = row.destination_host.clone();
        group.profile_ids.insert(row.profile_id.clone());
        group.prompt_run_count += 1;
        if row.recorded_provider_request_sent {
            group.recorded_provider_request_count += 1;
        }
        if row.recorded_credential_accessed {
            group.recorded_credential_access_count += 1;
        }
        if observability_status_succeeded(&row.status) {
            group.succeeded_count += 1;
        } else if observability_status_failed(&row.status) {
            group.failed_count += 1;
        }
        group.estimated_total_tokens = group
            .estimated_total_tokens
            .saturating_add(u64::from(row.estimated_total_tokens));
        group.estimated_cost_usd += row.estimated_cost_usd;
        group.latest_activity_at = Some(
            group
                .latest_activity_at
                .unwrap_or(row.completed_at)
                .max(row.completed_at),
        );
        group
            .evidence_refs
            .extend(row.evidence_refs.iter().cloned());
    }
    for row in call_rows {
        let key = (
            row.provider.clone(),
            row.model.clone(),
            row.destination_host.clone(),
        );
        let group = groups.entry(key).or_default();
        group.provider = row.provider.clone();
        group.model = row.model.clone();
        group.destination_host = row.destination_host.clone();
        group.profile_ids.insert(row.profile_id.clone());
        group.call_metadata_count += 1;
        if row.recorded_provider_request_sent {
            group.recorded_provider_request_count += 1;
        }
        if row.recorded_credential_accessed {
            group.recorded_credential_access_count += 1;
        }
        if observability_status_succeeded(&row.status) {
            group.succeeded_count += 1;
        } else if observability_status_failed(&row.status) {
            group.failed_count += 1;
        }
        group.estimated_total_tokens = group
            .estimated_total_tokens
            .saturating_add(u64::from(row.estimated_total_tokens));
        group.estimated_cost_usd += row.estimated_cost_usd;
        group.latest_activity_at = Some(
            group
                .latest_activity_at
                .unwrap_or(row.timestamp)
                .max(row.timestamp),
        );
        group
            .evidence_refs
            .extend(row.evidence_refs.iter().cloned());
    }
    let mut rows = groups
        .into_values()
        .map(|group| {
            let id = provider_observability_row_id(
                "group",
                &[&group.provider, &group.model, &group.destination_host],
            );
            LlmProviderObservabilityGroupingRow {
                id,
                provider: group.provider,
                model: group.model,
                destination_host: group.destination_host,
                profile_ids: group.profile_ids.into_iter().collect(),
                prompt_run_count: group.prompt_run_count,
                call_metadata_count: group.call_metadata_count,
                recorded_provider_request_count: group.recorded_provider_request_count,
                recorded_credential_access_count: group.recorded_credential_access_count,
                succeeded_count: group.succeeded_count,
                failed_count: group.failed_count,
                estimated_total_tokens: group.estimated_total_tokens,
                estimated_cost_usd: group.estimated_cost_usd,
                latest_activity_at: group.latest_activity_at,
                evidence_refs: group.evidence_refs.into_iter().take(12).collect(),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .latest_activity_at
            .cmp(&left.latest_activity_at)
            .then_with(|| {
                (right.prompt_run_count + right.call_metadata_count)
                    .cmp(&(left.prompt_run_count + left.call_metadata_count))
            })
            .then_with(|| left.provider.cmp(&right.provider))
            .then_with(|| left.model.cmp(&right.model))
    });
    rows
}

pub(crate) fn provider_observability_budget_usage_hints(
    profiles: &[ProviderProfileRecord],
    prompt_runs: &[&LlmPromptRunRecord],
    call_metadata: &[&ProviderCallMetadata],
    filters: &ProviderObservabilityFilters,
    redaction_roots: &[(String, &'static str)],
    limit: usize,
) -> Vec<LlmProviderObservabilityBudgetUsageHint> {
    let mut rows = profiles
        .iter()
        .filter(|profile| filters.matches_profile(profile))
        .map(|profile| {
            let profile_prompt_runs = prompt_runs
                .iter()
                .filter(|run| run.profile_id == profile.id)
                .collect::<Vec<_>>();
            let profile_call_metadata = call_metadata
                .iter()
                .filter(|metadata| metadata.profile_id == profile.id)
                .collect::<Vec<_>>();
            let observed_estimated_total_tokens = profile_prompt_runs
                .iter()
                .map(|run| u64::from(run.estimated_total_tokens))
                .chain(profile_call_metadata.iter().map(|metadata| {
                    u64::from(
                        metadata
                            .estimated_input_tokens
                            .saturating_add(metadata.estimated_output_tokens),
                    )
                }))
                .sum::<u64>();
            let observed_estimated_cost_usd = profile_prompt_runs
                .iter()
                .map(|run| run.estimated_cost_usd)
                .chain(
                    profile_call_metadata
                        .iter()
                        .map(|metadata| metadata.estimated_cost_usd),
                )
                .sum::<f64>();
            let budget_state = if !profile.enabled {
                "profile_disabled"
            } else if profile.monthly_budget_usd <= 0.0 {
                "budget_zero"
            } else if observed_estimated_cost_usd <= f64::EPSILON {
                "no_usage_observed"
            } else if observed_estimated_cost_usd > profile.monthly_budget_usd {
                "over_configured_budget_hint"
            } else if observed_estimated_cost_usd >= profile.monthly_budget_usd * 0.8 {
                "near_configured_budget_hint"
            } else {
                "within_configured_budget_hint"
            };
            let id = provider_observability_row_id("budget", &[&profile.id, budget_state]);
            let mut evidence_refs = vec!["app-data:llm/provider-profiles.json".to_string()];
            evidence_refs.extend(profile_prompt_runs.iter().map(|run| {
                format!(
                    "prompt-run:{}",
                    provider_observability_row_id(
                        "prompt-run-evidence",
                        &[&run.completed_at.to_string(), &run.id]
                    )
                )
            }));
            evidence_refs.extend(profile_call_metadata.iter().map(|metadata| {
                format!(
                    "provider-call:{}",
                    provider_observability_row_id(
                        "provider-call-evidence",
                        &[
                            &metadata.timestamp.to_string(),
                            &metadata.profile_id,
                            &metadata.action_type
                        ]
                    )
                )
            }));
            evidence_refs.sort();
            evidence_refs.dedup();
            LlmProviderObservabilityBudgetUsageHint {
                id,
                profile_id: observability_redact(&profile.id, redaction_roots, 160),
                provider: profile.provider_type.as_str().to_string(),
                model: observability_redact(&profile.model, redaction_roots, 160),
                destination_host: observability_redact(
                    &destination_host_for_url(&profile.base_url),
                    redaction_roots,
                    160,
                ),
                enabled: profile.enabled,
                single_request_token_limit: profile.single_request_token_limit,
                monthly_budget_usd: profile.monthly_budget_usd,
                observed_prompt_run_count: profile_prompt_runs.len(),
                observed_call_metadata_count: profile_call_metadata.len(),
                observed_estimated_total_tokens,
                observed_estimated_cost_usd,
                budget_state: budget_state.to_string(),
                reason: "Budget hint uses configured profile limits plus stored redacted metadata only; no provider request or credential read was performed.".to_string(),
                evidence_refs: evidence_refs.into_iter().take(12).collect(),
            }
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .observed_estimated_cost_usd
            .partial_cmp(&left.observed_estimated_cost_usd)
            .unwrap_or(std::cmp::Ordering::Equal)
            .then_with(|| left.profile_id.cmp(&right.profile_id))
    });
    rows.truncate(limit);
    rows
}

pub(crate) fn provider_observability_status_rows(
    mut status_rows: Vec<LlmProviderObservabilityStatusRow>,
    prompt_runs: &[&LlmPromptRunRecord],
    call_metadata: &[&ProviderCallMetadata],
    limit: usize,
) -> Vec<LlmProviderObservabilityStatusRow> {
    let mut counts: BTreeMap<(&str, String), usize> = BTreeMap::new();
    for run in prompt_runs {
        *counts
            .entry(("prompt-runs", run.status.clone()))
            .or_default() += 1;
    }
    for metadata in call_metadata {
        *counts
            .entry(("provider-call-metadata", metadata.status.clone()))
            .or_default() += 1;
    }
    for ((source, status), count) in counts {
        let severity = provider_observability_status_severity(&status);
        let id = provider_observability_row_id("status", &[source, &status]);
        status_rows.push(LlmProviderObservabilityStatusRow {
            id,
            source: source.to_string(),
            status: status.clone(),
            severity,
            message: format!("{count} {source} metadata row(s) reported status `{status}`."),
            count,
            evidence_refs: vec![format!("app-data:{source}")],
        });
    }
    status_rows.sort_by(|left, right| {
        provider_observability_severity_rank(left.severity)
            .cmp(&provider_observability_severity_rank(right.severity))
            .then_with(|| left.source.cmp(&right.source))
            .then_with(|| left.status.cmp(&right.status))
    });
    status_rows.truncate(limit.saturating_mul(2));
    status_rows
}

pub(crate) struct ProviderObservabilitySummaryInput<'a> {
    pub(crate) total_prompt_run_count: usize,
    pub(crate) total_call_metadata_count: usize,
    pub(crate) history_rows: &'a [LlmProviderObservabilityHistoryRow],
    pub(crate) call_rows: &'a [LlmProviderObservabilityCallRow],
    pub(crate) returned_prompt_run_count: usize,
    pub(crate) returned_call_row_count: usize,
    pub(crate) provider_profile_count: usize,
    pub(crate) enabled_profile_count: usize,
    pub(crate) grouping_count: usize,
}

pub(crate) fn provider_observability_summary(
    input: ProviderObservabilitySummaryInput<'_>,
) -> LlmProviderObservabilitySummary {
    let observed_provider_request_row_count = input
        .history_rows
        .iter()
        .filter(|row| row.recorded_provider_request_sent)
        .count()
        + input
            .call_rows
            .iter()
            .filter(|row| row.recorded_provider_request_sent)
            .count();
    let observed_credential_access_row_count = input
        .history_rows
        .iter()
        .filter(|row| row.recorded_credential_accessed)
        .count()
        + input
            .call_rows
            .iter()
            .filter(|row| row.recorded_credential_accessed)
            .count();
    let succeeded_count = input
        .history_rows
        .iter()
        .filter(|row| observability_status_succeeded(&row.status))
        .count()
        + input
            .call_rows
            .iter()
            .filter(|row| observability_status_succeeded(&row.status))
            .count();
    let failed_count = input
        .history_rows
        .iter()
        .filter(|row| observability_status_failed(&row.status))
        .count()
        + input
            .call_rows
            .iter()
            .filter(|row| observability_status_failed(&row.status))
            .count();
    let estimated_input_tokens = input
        .history_rows
        .iter()
        .map(|row| u64::from(row.estimated_input_tokens))
        .chain(
            input
                .call_rows
                .iter()
                .map(|row| u64::from(row.estimated_input_tokens)),
        )
        .sum::<u64>();
    let estimated_output_tokens = input
        .history_rows
        .iter()
        .map(|row| u64::from(row.estimated_output_tokens))
        .chain(
            input
                .call_rows
                .iter()
                .map(|row| u64::from(row.estimated_output_tokens)),
        )
        .sum::<u64>();
    let estimated_total_tokens = input
        .history_rows
        .iter()
        .map(|row| u64::from(row.estimated_total_tokens))
        .chain(
            input
                .call_rows
                .iter()
                .map(|row| u64::from(row.estimated_total_tokens)),
        )
        .sum::<u64>();
    let estimated_cost_usd = input
        .history_rows
        .iter()
        .map(|row| row.estimated_cost_usd)
        .chain(input.call_rows.iter().map(|row| row.estimated_cost_usd))
        .sum::<f64>();
    let latest_activity_at = input
        .history_rows
        .iter()
        .map(|row| row.completed_at)
        .chain(input.call_rows.iter().map(|row| row.timestamp))
        .max();
    let matched_prompt_run_count = input.history_rows.len();
    let matched_call_row_count = input.call_rows.len();
    let summary = if matched_prompt_run_count == 0 && matched_call_row_count == 0 {
        "No app-local provider prompt-run or call metadata matched the selected filters."
            .to_string()
    } else {
        let grouping_count = input.grouping_count;
        let returned_prompt_run_count = input.returned_prompt_run_count;
        let returned_call_row_count = input.returned_call_row_count;
        format!(
            "Reviewed {matched_prompt_run_count} prompt run row(s) and {matched_call_row_count} provider call metadata row(s) across {grouping_count} provider/model/destination group(s); returned {returned_prompt_run_count} prompt run row(s) and {returned_call_row_count} provider call row(s) as bounded evidence."
        )
    };
    LlmProviderObservabilitySummary {
        total_prompt_run_count: input.total_prompt_run_count,
        total_call_metadata_count: input.total_call_metadata_count,
        returned_prompt_run_count: input.returned_prompt_run_count,
        returned_call_row_count: input.returned_call_row_count,
        provider_profile_count: input.provider_profile_count,
        enabled_profile_count: input.enabled_profile_count,
        grouping_count: input.grouping_count,
        observed_provider_request_row_count,
        observed_credential_access_row_count,
        succeeded_count,
        failed_count,
        estimated_input_tokens,
        estimated_output_tokens,
        estimated_total_tokens,
        estimated_cost_usd,
        latest_activity_at,
        summary,
    }
}

pub(crate) fn provider_observability_gap_notes(
    provider_profile_count: usize,
    prompt_run_count: usize,
    call_metadata_count: usize,
) -> Vec<String> {
    let mut notes = Vec::new();
    if provider_profile_count == 0 {
        notes.push(
            "No provider profile metadata is configured; observability can still show historical app-local call metadata if present.".to_string(),
        );
    }
    if prompt_run_count == 0 && call_metadata_count == 0 {
        notes.push(
            "No app-local prompt run or provider call metadata has been recorded for the selected filters.".to_string(),
        );
    }
    notes
}

pub(crate) fn provider_observability_blocker_notes(
    status_rows: &[LlmProviderObservabilityStatusRow],
) -> Vec<String> {
    status_rows
        .iter()
        .filter(|row| matches!(row.status.as_str(), "read_error" | "parse_error"))
        .map(|row| row.message.clone())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

pub(crate) fn provider_observability_retention_recommendations(
    prompt_run_count: usize,
    call_metadata_count: usize,
) -> Vec<LlmProviderObservabilityRetentionRecommendationRow> {
    vec![
        LlmProviderObservabilityRetentionRecommendationRow {
            id: "retention:prompt-runs".to_string(),
            source_file: "prompt-runs.json",
            current_record_count: prompt_run_count,
            recommendation: if prompt_run_count == 0 {
                "No prompt run metadata cleanup is needed.".to_string()
            } else {
                "Keep prompt run history as app-local redacted metadata; any cleanup should be a separate explicit user action.".to_string()
            },
            cleanup_action_available: false,
            write_action_available: false,
            evidence_refs: vec!["app-data:prompt-runs.json".to_string()],
        },
        LlmProviderObservabilityRetentionRecommendationRow {
            id: "retention:provider-call-metadata".to_string(),
            source_file: "provider-call-metadata.jsonl",
            current_record_count: call_metadata_count,
            recommendation: if call_metadata_count == 0 {
                "No provider call metadata cleanup is needed.".to_string()
            } else {
                "Keep provider call metadata as app-local redacted audit rows; any cleanup should be a separate explicit user action.".to_string()
            },
            cleanup_action_available: false,
            write_action_available: false,
            evidence_refs: vec!["app-data:llm/provider-call-metadata.jsonl".to_string()],
        },
    ]
}

pub(crate) fn provider_observability_evidence_references(
    history_rows: &[LlmProviderObservabilityHistoryRow],
    call_rows: &[LlmProviderObservabilityCallRow],
    grouping_rows: &[LlmProviderObservabilityGroupingRow],
    budget_usage_hints: &[LlmProviderObservabilityBudgetUsageHint],
) -> Vec<LlmProviderObservabilityEvidenceReference> {
    let mut refs = BTreeMap::<String, LlmProviderObservabilityEvidenceReference>::new();
    for (id, kind, label, source) in [
        (
            "app-data:prompt-runs.json",
            "app-local-file",
            "prompt-runs.json",
            "prompt-runs.json",
        ),
        (
            "app-data:llm/provider-call-metadata.jsonl",
            "app-local-file",
            "provider-call-metadata.jsonl",
            "llm/provider-call-metadata.jsonl",
        ),
        (
            "app-data:llm/provider-profiles.json",
            "app-local-file",
            "provider-profiles.json",
            "llm/provider-profiles.json",
        ),
    ] {
        refs.insert(
            id.to_string(),
            LlmProviderObservabilityEvidenceReference {
                id: id.to_string(),
                kind,
                label: label.to_string(),
                source: source.to_string(),
            },
        );
    }
    for row in history_rows {
        for evidence_ref in &row.evidence_refs {
            refs.insert(
                evidence_ref.clone(),
                LlmProviderObservabilityEvidenceReference {
                    id: evidence_ref.clone(),
                    kind: "prompt-run",
                    label: format!("Prompt run {}", row.prompt_run_id),
                    source: "prompt-runs.json".to_string(),
                },
            );
        }
    }
    for row in call_rows {
        for evidence_ref in &row.evidence_refs {
            refs.insert(
                evidence_ref.clone(),
                LlmProviderObservabilityEvidenceReference {
                    id: evidence_ref.clone(),
                    kind: "provider-call-metadata",
                    label: format!("Provider call {}", row.action_type),
                    source: "llm/provider-call-metadata.jsonl".to_string(),
                },
            );
        }
    }
    for row in grouping_rows {
        for evidence_ref in &row.evidence_refs {
            refs.entry(evidence_ref.clone()).or_insert_with(|| {
                LlmProviderObservabilityEvidenceReference {
                    id: evidence_ref.clone(),
                    kind: "group-source",
                    label: format!("Group source {}", row.id),
                    source: "derived-local-observability".to_string(),
                }
            });
        }
    }
    for row in budget_usage_hints {
        for evidence_ref in &row.evidence_refs {
            refs.entry(evidence_ref.clone()).or_insert_with(|| {
                LlmProviderObservabilityEvidenceReference {
                    id: evidence_ref.clone(),
                    kind: "budget-source",
                    label: format!("Budget source {}", row.profile_id),
                    source: "derived-local-observability".to_string(),
                }
            });
        }
    }
    refs.into_values().take(60).collect()
}

pub(crate) fn provider_observability_status_row(
    id_suffix: &str,
    source: &str,
    status: &str,
    severity: &'static str,
    message: impl Into<String>,
    count: usize,
    evidence_refs: Vec<String>,
) -> LlmProviderObservabilityStatusRow {
    LlmProviderObservabilityStatusRow {
        id: provider_observability_row_id("status-row", &[id_suffix, source, status]),
        source: source.to_string(),
        status: status.to_string(),
        severity,
        message: message.into(),
        count,
        evidence_refs,
    }
}

pub(crate) fn provider_observability_status_severity(status: &str) -> &'static str {
    if observability_status_failed(status) || matches!(status, "read_error" | "parse_error") {
        "warning"
    } else {
        "info"
    }
}

pub(crate) fn provider_observability_severity_rank(severity: &str) -> u8 {
    match severity {
        "critical" => 0,
        "warning" | "warn" => 1,
        "info" => 2,
        _ => 3,
    }
}

pub(crate) fn observability_status_succeeded(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "succeeded" | "success" | "ok" | "loaded"
    )
}

pub(crate) fn observability_status_failed(status: &str) -> bool {
    matches!(
        status.to_ascii_lowercase().as_str(),
        "failed" | "error" | "blocked" | "timeout" | "network_error"
    )
}

pub(crate) fn observability_redact(
    value: &str,
    redaction_roots: &[(String, &'static str)],
    max_chars: usize,
) -> String {
    let mut redactor = PromptRedactor::new(redaction_roots);
    truncate_chars(&redactor.redact(value), max_chars)
}

pub(crate) fn provider_observability_row_id(prefix: &str, parts: &[&str]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(prefix.as_bytes());
    for part in parts {
        hasher.update(b"\0");
        hasher.update(part.as_bytes());
    }
    let digest = hasher.finalize();
    format!("{prefix}-{}", hex_prefix(&digest, 12))
}
