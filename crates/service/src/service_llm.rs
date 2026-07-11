use super::*;
use crate::service_keyset_cursor::{decode_cursor_for_method, encode_cursor, KeysetCursor};

impl ServiceHost {
    pub fn llm_status(&self) -> LlmStatus {
        let profiles = self.list_llm_provider_profiles().ok();
        let default_profile = profiles.as_ref().and_then(|profiles| {
            profiles
                .default_profile_id
                .as_ref()
                .and_then(|default_id| {
                    profiles
                        .profiles
                        .iter()
                        .find(|profile| profile.id == *default_id)
                })
                .or_else(|| profiles.profiles.iter().find(|profile| profile.enabled))
        });
        let configured = default_profile
            .is_some_and(|profile| profile.enabled && profile.credential_status.secret_available);
        let profile_count = profiles
            .as_ref()
            .map(|profiles| profiles.profiles.len())
            .unwrap_or(0);
        let reason = match default_profile {
            Some(profile) if configured => {
                format!(
                    "Provider profile `{}` is configured; provider calls remain user-triggered and confirmation-gated.",
                    profile.id
                )
            }
            Some(profile) if !profile.enabled => {
                format!("Provider profile `{}` exists but is disabled.", profile.id)
            }
            Some(profile) => format!(
                "Provider profile `{}` exists but its API key is unavailable from the OS credential store.",
                profile.id
            ),
            None if profile_count > 0 => {
                "Provider profiles exist, but none is enabled as the default provider.".to_string()
            }
            None => "LLM actions are disabled by default; no local provider is configured."
                .to_string(),
        };
        LlmStatus {
            enabled: configured,
            configured,
            provider: default_profile.map(|profile| profile.provider_type.as_str().to_string()),
            model: default_profile.map(|profile| profile.model.clone()),
            reason,
            single_request_token_limit: default_profile
                .map(|profile| profile.single_request_token_limit)
                .unwrap_or_else(default_token_limit),
            monthly_budget_usd: default_profile
                .map(|profile| profile.monthly_budget_usd)
                .unwrap_or_else(default_monthly_budget_usd),
            credentials_storage: if profile_count == 0 {
                "none".to_string()
            } else {
                "keychain".to_string()
            },
            credential_persistence_allowed: profile_count > 0,
            provider_profile_count: profile_count,
            default_profile_id: default_profile.map(|profile| profile.id.clone()),
            profiles_path: display_path(&provider_profiles_path(&self.app_data_dir)),
            call_metadata_path: display_path(&provider_call_metadata_path(&self.app_data_dir)),
            raw_prompt_persistence_allowed: false,
            raw_response_persistence_allowed: false,
        }
    }

    pub(crate) fn list_llm_provider_profiles(
        &self,
    ) -> Result<ListProviderProfilesResult, ServiceError> {
        list_provider_profiles(&self.app_data_dir).map_err(Into::into)
    }

    pub fn preview_llm_prompt(
        &self,
        params: LlmPreviewPromptParams,
    ) -> Result<LlmPreviewPromptResult, ServiceError> {
        let profile = self.resolve_llm_prompt_profile(params.profile_id.as_deref())?;
        let built = self.build_llm_prompt(&params)?;
        let provider = profile
            .as_ref()
            .map(|profile| profile.provider_type.as_str().to_string());
        let model = profile.as_ref().map(|profile| profile.model.clone());
        let profile_id = profile.as_ref().map(|profile| profile.id.clone());
        let destination_host = profile
            .as_ref()
            .map(|profile| destination_host_for_url(&profile.base_url));
        let single_request_token_limit = profile
            .as_ref()
            .map(|profile| profile.single_request_token_limit)
            .unwrap_or_else(default_token_limit);
        let monthly_budget_usd = profile
            .as_ref()
            .map(|profile| profile.monthly_budget_usd)
            .unwrap_or_else(default_monthly_budget_usd);
        let estimated_input_tokens = estimate_tokens(&[&built.prompt_preview]);
        let estimated_output_tokens = built.estimated_output_tokens;
        let estimated_total_tokens = estimated_input_tokens.saturating_add(estimated_output_tokens);
        let estimated_cost_usd = profile
            .as_ref()
            .map(|profile| estimate_prompt_cost_usd(profile.provider_type, estimated_total_tokens))
            .unwrap_or(0.0);
        let (allowed, reason) = match profile.as_ref() {
            None => (
                false,
                "No enabled provider profile is configured; no provider request can be sent."
                    .to_string(),
            ),
            Some(profile) if !profile.enabled => (
                false,
                format!("Provider profile `{}` is disabled.", profile.id),
            ),
            Some(profile) if profile.monthly_budget_usd <= 0.0 => (
                false,
                "Monthly provider budget is 0; provider requests are disabled.".to_string(),
            ),
            Some(profile) if profile.single_request_token_limit < estimated_total_tokens => (
                false,
                "Single request token limit is lower than the redacted prompt estimate."
                    .to_string(),
            ),
            Some(_) => (
                true,
                "Redacted prompt preview is ready for explicit confirmation.".to_string(),
            ),
        };
        let preview_id = llm_preview_id(
            &params,
            profile.as_ref(),
            &built.prompt_preview,
            estimated_input_tokens,
            estimated_output_tokens,
        );

        Ok(LlmPreviewPromptResult {
            preview_id,
            status: if allowed { "ready" } else { "blocked" }.to_string(),
            allowed,
            reason,
            action: params.action.as_str(),
            profile_id,
            provider,
            model,
            destination_host,
            prompt_scope: built.prompt_scope,
            included_fields: built.included_fields,
            excluded_fields: built.excluded_fields,
            redaction: built.redaction,
            prompt_preview: built.prompt_preview,
            estimated_input_tokens,
            estimated_output_tokens,
            estimated_total_tokens,
            estimated_cost_usd,
            single_request_token_limit,
            monthly_budget_usd,
            requires_confirmation: true,
            confirmation: LlmConfirmationRequirement {
                required: true,
                message:
                    "Confirm to send only this redacted prompt to the displayed provider endpoint."
                        .to_string(),
                display_fields: vec![
                    "preview_id",
                    "provider",
                    "model",
                    "destination_host",
                    "prompt_scope",
                    "included_fields",
                    "excluded_fields",
                    "redaction",
                    "estimated_total_tokens",
                    "estimated_cost_usd",
                ],
            },
            write_back_allowed: false,
            draft_requires_user_copy: true,
            provider_request_sent: false,
            raw_secret_returned: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
        })
    }

    pub fn confirm_llm_prompt_and_send(
        &self,
        params: LlmConfirmPromptAndSendParams,
    ) -> Result<LlmConfirmPromptAndSendResult, ServiceError> {
        if params.confirmation_id.trim().is_empty() {
            return Err(ServiceError::ConfirmationRequired(
                "llm.confirmPromptAndSend requires an explicit confirmation_id".to_string(),
            ));
        }
        let preview = self.preview_llm_prompt(params.request.clone())?;
        if preview.preview_id != params.preview_id {
            return Err(ServiceError::InvalidRequest(
                "preview_id does not match the current redacted prompt preview".to_string(),
            ));
        }
        let profile_id = preview.profile_id.clone().ok_or_else(|| {
            ServiceError::InvalidRequest(
                "No provider profile is available for the confirmed prompt.".to_string(),
            )
        })?;
        let send = send_provider_prompt(
            &self.app_data_dir,
            SendProviderPromptParams {
                profile_id: profile_id.clone(),
                confirmation_id: params.confirmation_id.clone(),
                action_type: llm_prompt_action_type(&params.request),
                prompt: preview.prompt_preview.clone(),
                estimated_input_tokens: preview.estimated_input_tokens,
                estimated_output_tokens: preview.estimated_output_tokens,
                estimated_cost_usd: preview.estimated_cost_usd,
                redaction_status: preview.redaction.status.clone(),
                timeout_ms: params.timeout_ms,
            },
        )?;
        self.record_llm_prompt_run(&params, &preview, &send)?;

        Ok(LlmConfirmPromptAndSendResult {
            preview_id: params.preview_id,
            confirmation_id: params.confirmation_id,
            status: send.status,
            action: params.request.action.as_str(),
            profile_id,
            provider: send.provider_type.as_str().to_string(),
            model: send.model,
            destination_host: send.destination_host,
            provider_request_sent: send.provider_request_sent,
            credential_accessed: send.credential_accessed,
            draft_output: send.output_text,
            draft_requires_user_copy: true,
            write_back_allowed: false,
            script_execution_allowed: false,
            config_mutation_allowed: false,
            snapshot_created: false,
            triage_mutation_allowed: false,
            audit: send.audit,
            raw_secret_returned: send.raw_secret_returned,
            raw_prompt_persisted: send.raw_prompt_persisted,
            raw_response_persisted: send.raw_response_persisted,
        })
    }

    pub fn list_llm_prompt_runs(
        &self,
        params: LlmPromptRunListParams,
    ) -> Result<LlmPromptRunListResult, ServiceError> {
        let limit = params.limit.map(|limit| limit.clamp(1, 500));
        let action = params
            .action
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());
        let request_kind = params
            .request_kind
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());
        let instance_id = params
            .skill_instance_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let mut runs = self
            .load_llm_prompt_runs()?
            .into_iter()
            .filter(|run| {
                let action_matches = action
                    .as_deref()
                    .is_none_or(|filter| run.action.eq_ignore_ascii_case(filter));
                let request_matches = request_kind
                    .as_deref()
                    .is_none_or(|filter| run.request_kind.eq_ignore_ascii_case(filter));
                let instance_matches = instance_id.as_deref().is_none_or(|filter| {
                    run.instance_id.as_deref() == Some(filter)
                        || run.instance_ids.iter().any(|id| id == filter)
                });
                action_matches && request_matches && instance_matches
            })
            .collect::<Vec<_>>();
        let total_count = runs.len();
        if let Some(limit) = limit {
            runs.truncate(limit);
        }
        let returned_count = runs.len();
        Ok(LlmPromptRunListResult {
            generated_by: "local-v2.61",
            count: returned_count,
            total_count,
            returned_count,
            limit,
            truncated: returned_count < total_count,
            runs,
            app_local_only: true,
            runs_file: "prompt-runs.json",
            provider_request_sent: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_secret_returned: false,
            safety_flags: llm_prompt_run_safety_flags(false, false),
        })
    }

    pub fn llm_provider_observability(
        &self,
        params: LlmProviderObservabilityParams,
    ) -> Result<LlmProviderObservabilityResult, ServiceError> {
        let limit = params.limit.unwrap_or(50).clamp(1, 500);
        let adapter_ctx = self.effective_adapter_ctx()?;
        let redaction_roots = self.trace_redaction_roots(&adapter_ctx);
        let filters = ProviderObservabilityFilters::from_params(&params);

        let (prompt_runs, mut status_rows) =
            self.load_llm_prompt_runs_for_observability(&redaction_roots);
        let (call_metadata, call_status_rows) =
            self.load_provider_call_metadata_for_observability(&redaction_roots);
        status_rows.extend(call_status_rows);
        let (profiles, profile_status_rows) =
            self.load_provider_profiles_for_observability(&redaction_roots);
        status_rows.extend(profile_status_rows);

        let matched_prompt_runs = prompt_runs
            .iter()
            .filter(|run| filters.matches_prompt_run(run))
            .collect::<Vec<_>>();
        let matched_call_metadata = call_metadata
            .iter()
            .filter(|metadata| filters.matches_provider_call(metadata))
            .collect::<Vec<_>>();

        let mut all_history_rows = matched_prompt_runs
            .iter()
            .enumerate()
            .map(|(index, run)| provider_observability_history_row(run, index, &redaction_roots))
            .collect::<Vec<_>>();
        all_history_rows.sort_by(|left, right| {
            right
                .completed_at
                .cmp(&left.completed_at)
                .then_with(|| left.prompt_run_id.cmp(&right.prompt_run_id))
        });

        let mut all_call_rows = matched_call_metadata
            .iter()
            .enumerate()
            .map(|(index, metadata)| {
                provider_observability_call_row(metadata, index, &redaction_roots)
            })
            .collect::<Vec<_>>();
        all_call_rows.sort_by(|left, right| {
            right
                .timestamp
                .cmp(&left.timestamp)
                .then_with(|| left.id.cmp(&right.id))
        });

        let include_history = params.include_history.unwrap_or(true);
        let history_rows = if include_history {
            all_history_rows.iter().take(limit).cloned().collect()
        } else {
            Vec::new()
        };
        let call_rows = if include_history {
            all_call_rows.iter().take(limit).cloned().collect()
        } else {
            Vec::new()
        };

        let grouping_rows = provider_observability_grouping_rows(&all_history_rows, &all_call_rows);
        let budget_usage_hints = provider_observability_budget_usage_hints(
            &profiles,
            &matched_prompt_runs,
            &matched_call_metadata,
            &filters,
            &redaction_roots,
            limit,
        );
        let mut status_rows = provider_observability_status_rows(
            status_rows,
            &matched_prompt_runs,
            &matched_call_metadata,
            limit,
        );
        let blocker_notes = provider_observability_blocker_notes(&status_rows);
        let gap_notes = provider_observability_gap_notes(
            profiles.len(),
            matched_prompt_runs.len(),
            matched_call_metadata.len(),
        );
        let retention_recommendations = provider_observability_retention_recommendations(
            prompt_runs.len(),
            call_metadata.len(),
        );
        let model_task_history_rows = self
            .list_model_task_matches(ModelTaskMatchListParams {
                provider: params.provider.clone(),
                model: params.model.clone(),
                task_kind: params.action.clone(),
                match_status: None,
                agent: None,
                source_kind: None,
                limit: Some(limit),
            })?
            .recent_evidence_rows;
        let evidence_references = provider_observability_evidence_references(
            &all_history_rows,
            &all_call_rows,
            &grouping_rows,
            &budget_usage_hints,
        );
        status_rows.truncate(limit.saturating_mul(2));

        let summary = provider_observability_summary(ProviderObservabilitySummaryInput {
            total_prompt_run_count: prompt_runs.len(),
            total_call_metadata_count: call_metadata.len(),
            history_rows: &all_history_rows,
            call_rows: &all_call_rows,
            returned_prompt_run_count: history_rows.len(),
            returned_call_row_count: call_rows.len(),
            provider_profile_count: profiles.len(),
            enabled_profile_count: profiles.iter().filter(|profile| profile.enabled).count(),
            grouping_count: grouping_rows.len(),
        });
        let status = if blocker_notes.is_empty() {
            "ready".to_string()
        } else {
            "partial".to_string()
        };

        Ok(LlmProviderObservabilityResult {
            generated_by: "local-v2.64",
            status,
            filters: filters.applied_filters(&params, limit),
            summary,
            call_rows,
            history_rows,
            grouping_rows,
            model_task_history_rows,
            status_rows,
            budget_usage_hints,
            retention_recommendations,
            gap_notes,
            blocker_notes,
            evidence_references,
            prompt_metadata: LlmProviderObservabilityPromptMetadata {
                available: false,
                preview_method: "llm.previewPrompt",
                confirm_method: "llm.confirmPromptAndSend",
                provider_request_sent: false,
                copy_only: true,
                note: "Provider observability is a deterministic local read model; this method never sends provider traffic or reads credentials.".to_string(),
            },
            safety_flags: llm_provider_observability_safety_flags(),
        })
    }

    pub fn list_provider_activity(
        &self,
        params: ListProviderActivityParams,
    ) -> Result<ProviderActivityPageResult, ServiceError> {
        self.list_provider_activity_at(params, unix_timestamp_millis())
    }

    pub(crate) fn list_provider_activity_at(
        &self,
        params: ListProviderActivityParams,
        now: i64,
    ) -> Result<ProviderActivityPageResult, ServiceError> {
        const METHOD: &str = "llm.listProviderActivity";
        let limit = params.limit.unwrap_or(50).clamp(1, 100);
        let cursor = params
            .cursor
            .as_deref()
            .map(|text| decode_cursor_for_method(text, METHOD))
            .transpose()?;
        let query = resolve_provider_activity_query(&params, cursor.as_ref(), now)?;
        let query_digest = provider_activity_query_digest(&params, &query)?;
        if cursor
            .as_ref()
            .is_some_and(|cursor| cursor.query_digest != query_digest)
        {
            return Err(ServiceError::InvalidRequest(
                "cursor does not match this list query".to_string(),
            ));
        }
        let adapter_ctx = self.effective_adapter_ctx()?;
        let redaction_roots = self.trace_redaction_roots(&adapter_ctx);
        let filters = ProviderObservabilityFilters::from_activity_bounds(
            &params,
            query.resolved_start_at,
            query.resolved_end_at,
        );
        let raw_snapshot = read_consistent_provider_activity_raw_snapshot(
            &self.llm_prompt_runs_path(),
            &provider_call_metadata_path(&self.app_data_dir),
        )?;
        let source_revision = provider_activity_raw_source_revision(&raw_snapshot);
        let prompt_runs = parse_provider_activity_prompt_runs(&raw_snapshot.prompt_runs)?;
        let call_metadata = parse_provider_activity_provider_calls(&raw_snapshot.provider_calls)?;

        let mut rows = prompt_runs
            .iter()
            .filter(|run| filters.matches_prompt_run(run))
            .enumerate()
            .map(
                |(index, run)| -> Result<ProviderActivityRow, ServiceError> {
                    Ok(provider_activity_history_row(
                        provider_observability_history_row(run, index, &redaction_roots),
                        provider_activity_prompt_run_id(run)?,
                    ))
                },
            )
            .collect::<Result<Vec<_>, _>>()?;
        rows.extend(
            call_metadata
                .iter()
                .filter(|metadata| filters.matches_provider_call(metadata))
                .enumerate()
                .map(
                    |(index, metadata)| -> Result<ProviderActivityRow, ServiceError> {
                        Ok(provider_activity_call_row(
                            provider_observability_call_row(metadata, index, &redaction_roots),
                            provider_activity_provider_call_id(metadata)?,
                        ))
                    },
                )
                .collect::<Result<Vec<_>, _>>()?,
        );
        rows.sort_by(|left, right| {
            right
                .timestamp
                .cmp(&left.timestamp)
                .then_with(|| left.id.cmp(&right.id))
        });

        let cursor_revision = cursor
            .as_ref()
            .map(|cursor| cursor.source_revision.as_str());
        if params
            .source_revision
            .as_deref()
            .is_some_and(|revision| revision != source_revision)
            || cursor_revision.is_some_and(|revision| revision != source_revision)
        {
            return Err(ServiceError::SourceChanged);
        }

        let total_count = rows.len();
        if let Some(cursor) = cursor.as_ref() {
            rows.retain(|row| {
                row.timestamp < cursor.sort_value
                    || (row.timestamp == cursor.sort_value && row.id > cursor.stable_id)
            });
        }
        let has_more = rows.len() > limit;
        if has_more {
            rows.truncate(limit);
        }
        let next_cursor = if has_more {
            rows.last()
                .map(|row| {
                    encode_cursor(&KeysetCursor {
                        version: 1,
                        method: METHOD.to_string(),
                        query_digest: query_digest.clone(),
                        source_revision: source_revision.clone(),
                        sort_value: row.timestamp,
                        stable_id: row.id.clone(),
                        tie_breaker_digest: None,
                        accepted_count: None,
                        resolved_start_at: query.resolved_start_at,
                        resolved_end_at: query.resolved_end_at,
                    })
                })
                .transpose()?
        } else {
            None
        };
        Ok(ProviderActivityPageResult {
            generated_by: "local-v2.64",
            page: ListPageMetadata::enumerable(rows.len(), Some(total_count), next_cursor),
            rows,
            source_revision,
            safety_flags: llm_provider_observability_safety_flags(),
        })
    }

    pub fn list_model_task_matches(
        &self,
        params: ModelTaskMatchListParams,
    ) -> Result<ModelTaskMatchListResult, ServiceError> {
        let limit = params.limit.map(|limit| limit.clamp(1, 500));
        let row_limit = limit.unwrap_or(usize::MAX);
        let adapter_ctx = self.effective_adapter_ctx()?;
        let redaction_roots = self.trace_redaction_roots(&adapter_ctx);
        let filters = ModelTaskMatchFilters::from_params(&params);
        let stored_records = self.load_model_task_matches()?;
        let prompt_runs = self.load_llm_prompt_runs()?;

        let mut records = stored_records
            .iter()
            .filter(|record| filters.matches_record(record))
            .map(|record| redacted_model_task_record(record, &redaction_roots))
            .collect::<Vec<_>>();
        let total_record_count = records.len();
        if let Some(limit) = limit {
            records.truncate(limit);
        }
        let returned_record_count = records.len();

        let matched_prompt_runs = prompt_runs
            .iter()
            .filter(|run| filters.matches_prompt_run(run))
            .collect::<Vec<_>>();
        let total_evidence_count = total_record_count + matched_prompt_runs.len();

        let mut recent_evidence_rows = records
            .iter()
            .map(model_task_record_evidence_row)
            .chain(
                matched_prompt_runs
                    .iter()
                    .map(|run| prompt_run_model_task_evidence_row(run, &redaction_roots)),
            )
            .collect::<Vec<_>>();
        recent_evidence_rows.sort_by(model_task_evidence_row_sort);
        if let Some(limit) = limit {
            recent_evidence_rows.truncate(limit);
        }
        let returned_evidence_count = recent_evidence_rows.len();

        let model_rows = model_task_model_rows(&recent_evidence_rows, row_limit);
        let task_rows = model_task_task_rows(&recent_evidence_rows, row_limit);
        let gap_notes = model_task_match_gap_notes(
            stored_records.len(),
            prompt_runs.len(),
            records.len(),
            matched_prompt_runs.len(),
        );
        let blocker_notes = Vec::new();
        let evidence_references = model_task_match_evidence_references(
            stored_records.len(),
            prompt_runs.len(),
            &recent_evidence_rows,
        );
        let summary = model_task_match_summary(
            stored_records.len(),
            prompt_runs.len(),
            records.len(),
            matched_prompt_runs.len().min(row_limit),
            &model_rows,
            &task_rows,
            &recent_evidence_rows,
        );
        let status = if blocker_notes.is_empty() {
            "ready".to_string()
        } else {
            "partial".to_string()
        };

        Ok(ModelTaskMatchListResult {
            generated_by: "local-v2.91",
            status,
            total_record_count,
            returned_record_count,
            total_evidence_count,
            returned_evidence_count,
            limit,
            truncated: returned_record_count < total_record_count
                || returned_evidence_count < total_evidence_count,
            summary,
            records,
            model_rows,
            task_rows,
            recent_evidence_rows,
            gap_notes,
            blocker_notes,
            evidence_references,
            app_local_only: true,
            history_file: "model-task-matches.json",
            provider_request_sent: false,
            credential_accessed: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
            safety_flags: model_task_match_safety_flags(true),
        })
    }

    pub fn record_model_task_match(
        &self,
        params: ModelTaskMatchRecordParams,
    ) -> Result<ModelTaskMatchRecordResult, ServiceError> {
        let task = params.task.trim();
        if task.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "llm.recordModelTaskMatch requires a non-empty task".to_string(),
            ));
        }
        let model = params.model.trim();
        if model.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "llm.recordModelTaskMatch requires a non-empty model".to_string(),
            ));
        }

        let adapter_ctx = self.effective_adapter_ctx()?;
        let roots = self.trace_redaction_roots(&adapter_ctx);
        let mut redactor = PromptRedactor::new(&roots);
        let now = unix_timestamp_millis();
        let redacted_task = truncate_chars(&redactor.redact(task), 600);
        let redacted_model = truncate_chars(&redactor.redact(model), 160);
        let provider = params
            .provider
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| truncate_chars(&redactor.redact(value), 120))
            .unwrap_or_else(|| "unknown".to_string());
        let destination_host = params
            .destination_host
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| truncate_chars(&redactor.redact(value), 160));
        let profile_id = params
            .profile_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| truncate_chars(&redactor.redact(value), 160));
        let task_kind = params
            .task_kind
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| truncate_chars(&redactor.redact(value), 120))
            .unwrap_or_else(|| "general".to_string());
        let agent = params
            .agent
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| truncate_chars(&redactor.redact(value), 120));
        let title = params
            .title
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| truncate_chars(&redactor.redact(value), 180))
            .unwrap_or_else(|| format!("{task_kind} on {redacted_model}"));
        let source_kind = normalize_model_task_source_kind(params.source_kind.as_deref());
        let match_status = normalize_model_task_match_status(params.match_status.as_deref());
        let confidence_score = params.confidence_score.map(|score| score.min(100));
        let prompt_run_ids =
            redact_model_task_string_list(&params.prompt_run_ids, &mut redactor, 160);
        let benchmark_ids =
            redact_model_task_string_list(&params.benchmark_ids, &mut redactor, 160);
        let mut evidence_refs =
            redact_model_task_string_list(&params.evidence_refs, &mut redactor, 180);
        if evidence_refs.is_empty() {
            evidence_refs.push("app-data:model-task-matches.json".to_string());
        }
        let gap_notes = redact_model_task_string_list(&params.gap_notes, &mut redactor, 300);
        let blocker_notes =
            redact_model_task_string_list(&params.blocker_notes, &mut redactor, 300);
        let outcome_notes =
            redact_model_task_string_list(&params.outcome_notes, &mut redactor, 300);
        let redaction_summary = model_task_match_redaction_summary_from(redactor.summary());
        let id = params
            .id
            .as_deref()
            .map(sanitize_model_task_match_id)
            .filter(|value| !value.is_empty())
            .unwrap_or_else(|| stable_model_task_match_id(&redacted_task, &redacted_model, now));

        let mut records = self.load_model_task_matches()?;
        let created_at = records
            .iter()
            .find(|record| record.id == id)
            .map(|record| record.created_at)
            .unwrap_or(now);
        let record = ModelTaskMatchRecord {
            id: id.clone(),
            title,
            task: redacted_task,
            task_kind,
            agent,
            profile_id,
            provider,
            model: redacted_model,
            destination_host,
            match_status,
            confidence_score,
            latency_ms: params.latency_ms,
            estimated_total_tokens: params.estimated_total_tokens,
            estimated_cost_usd: params.estimated_cost_usd,
            source_kind,
            prompt_run_ids,
            benchmark_ids,
            evidence_refs,
            gap_notes,
            blocker_notes,
            outcome_notes,
            created_at,
            updated_at: now,
            redaction_summary,
            safety_flags: model_task_match_safety_flags(false),
        };
        records.retain(|existing| existing.id != id);
        records.push(record.clone());
        self.save_model_task_matches(&records)?;

        Ok(ModelTaskMatchRecordResult {
            generated_by: "local-v2.91",
            record,
            count: records.len(),
            app_local_only: true,
            history_file: "model-task-matches.json",
            provider_request_sent: false,
            skill_files_mutated: false,
            agent_config_mutated: false,
            snapshot_created: false,
            triage_mutated: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
        })
    }

    pub fn delete_model_task_match(
        &self,
        params: ModelTaskMatchDeleteParams,
    ) -> Result<ModelTaskMatchDeleteResult, ServiceError> {
        let id = sanitize_model_task_match_id(&params.id);
        if id.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "llm.deleteModelTaskMatch requires a non-empty id".to_string(),
            ));
        }
        let mut records = self.load_model_task_matches()?;
        let before = records.len();
        records.retain(|record| record.id != id);
        let deleted = records.len() != before;
        self.save_model_task_matches(&records)?;

        Ok(ModelTaskMatchDeleteResult {
            record_id: id,
            deleted,
            remaining_count: records.len(),
            app_local_only: true,
            provider_request_sent: false,
            skill_files_mutated: false,
            agent_config_mutated: false,
            snapshot_created: false,
            triage_mutated: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
        })
    }

    pub fn script_execution_status(&self) -> ScriptExecutionStatus {
        ScriptExecutionStatus {
            enabled: false,
            default_enabled: false,
            reason: SCRIPT_EXECUTION_DISABLED_REASON.to_string(),
            audit_scope: "app-data/session-local".to_string(),
            audit_path: display_path(&self.script_execution_audit_path()),
            llm_initiation_allowed: false,
        }
    }

    pub(crate) fn resolve_llm_prompt_profile(
        &self,
        requested_profile_id: Option<&str>,
    ) -> Result<Option<ProviderProfileRecord>, ServiceError> {
        let profiles = list_provider_profiles(&self.app_data_dir)?;
        if let Some(profile_id) = requested_profile_id.filter(|id| !id.trim().is_empty()) {
            return profiles
                .profiles
                .into_iter()
                .find(|profile| profile.id == profile_id)
                .map(Some)
                .ok_or_else(|| ProviderError::ProfileNotFound(profile_id.to_string()).into());
        }
        Ok(profiles
            .default_profile_id
            .as_deref()
            .and_then(|default_id| {
                profiles
                    .profiles
                    .iter()
                    .find(|profile| profile.id == default_id)
            })
            .or_else(|| profiles.profiles.iter().find(|profile| profile.enabled))
            .cloned())
    }

    pub(crate) fn build_llm_prompt(
        &self,
        params: &LlmPreviewPromptParams,
    ) -> Result<BuiltLlmPrompt, ServiceError> {
        let adapter_ctx = self.effective_adapter_ctx()?;
        let roots = self.redaction_roots(&adapter_ctx);
        let mut redactor = PromptRedactor::new(&roots);
        let mut prompt_scope = vec![
            "operation metadata".to_string(),
            "app output language preference".to_string(),
            "safety boundaries".to_string(),
        ];
        let mut included_fields = vec![
            "action kind".to_string(),
            "app language code".to_string(),
            "draft-only safety instructions".to_string(),
        ];
        let mut excluded_fields = vec![
            "source paths".to_string(),
            "credential values".to_string(),
            "provider API key".to_string(),
            "agent config mutation instructions".to_string(),
            "script execution instructions".to_string(),
        ];
        let mut sections = vec![
            "You are assisting with AI agent skill governance.".to_string(),
            format!("Action: {}", params.action.as_str()),
            llm_output_language_instruction(params.app_language.as_deref()),
            "Return draft-only analysis. Do not write files, mutate agent config, execute scripts, change triage, create snapshots, call tools, or request secrets.".to_string(),
        ];
        if let Some(intent) = params
            .user_intent
            .as_deref()
            .filter(|intent| !intent.trim().is_empty())
        {
            prompt_scope.push("user intent".to_string());
            included_fields.push("redacted user intent".to_string());
            sections.push(format!("User intent: {}", redactor.redact(intent)));
        }

        match params.action {
            LlmPromptActionKind::Analyze | LlmPromptActionKind::DraftFrontmatter => {
                let instance_id = params.skill_instance_id.as_deref().ok_or_else(|| {
                    ServiceError::InvalidRequest(format!(
                        "llm.previewPrompt {} requires skill_instance_id",
                        params.action.as_str()
                    ))
                })?;
                let skill = self.get_llm_skill_detail(instance_id)?;
                prompt_scope.extend([
                    "selected skill metadata".to_string(),
                    "selected skill redacted frontmatter".to_string(),
                    "selected skill redacted body".to_string(),
                    "related finding summaries".to_string(),
                ]);
                included_fields.extend([
                    "skill id".to_string(),
                    "skill name".to_string(),
                    "agent".to_string(),
                    "scope".to_string(),
                    "enabled state".to_string(),
                    "redacted description".to_string(),
                    "redacted frontmatter".to_string(),
                    "redacted skill body".to_string(),
                    "rule finding ids and messages".to_string(),
                ]);
                sections.push(self.render_skill_prompt_section(&skill, &mut redactor)?);
            }
            LlmPromptActionKind::Recommend => {
                prompt_scope.extend([
                    "user intent".to_string(),
                    "catalog recommendation constraints".to_string(),
                ]);
                included_fields.push("recommendation constraints".to_string());
                excluded_fields.push("raw skill bodies".to_string());
                sections.push(
                    "Recommendation constraints: use current catalog evidence only when available; ask for clarification instead of inventing unavailable skills."
                        .to_string(),
                );
            }
            LlmPromptActionKind::ExplainConflict => {
                prompt_scope.extend([
                    "current conflict summaries".to_string(),
                    "current rule finding summaries".to_string(),
                ]);
                included_fields.extend([
                    "conflict ids".to_string(),
                    "definition ids".to_string(),
                    "rule ids".to_string(),
                    "finding severities".to_string(),
                ]);
                excluded_fields.push("raw skill bodies".to_string());
                let summary = self.llm_conflict_summary()?;
                sections.push(format!(
                    "Conflict and finding summary:\n{}",
                    redactor.redact(&summary)
                ));
            }
            LlmPromptActionKind::TaskCockpit => {
                let task = params.user_intent.as_deref().ok_or_else(|| {
                    ServiceError::InvalidRequest(
                        "llm.previewPrompt task_cockpit requires user_intent/task".to_string(),
                    )
                })?;
                prompt_scope.extend([
                    "redacted task preflight prompt".to_string(),
                    "selected agent catalog summaries".to_string(),
                    "effective enabled skill names and descriptions".to_string(),
                    "adapter capability and diagnostic status summaries".to_string(),
                    "required JSON result schema".to_string(),
                ]);
                included_fields.extend([
                    "redacted task intent".to_string(),
                    "selected agent ids and display names".to_string(),
                    "adapter support statuses without raw config contents".to_string(),
                    "effective skill ids, names, agents, enabled states, and descriptions"
                        .to_string(),
                    "task preflight feature description, evaluation rules, and output schema"
                        .to_string(),
                ]);
                excluded_fields.extend([
                    "raw source paths".to_string(),
                    "raw provider prompt".to_string(),
                    "raw provider response".to_string(),
                    "provider API keys or credentials".to_string(),
                    "raw trace content".to_string(),
                    "agent config contents".to_string(),
                    "raw skill body".to_string(),
                    "skill frontmatter".to_string(),
                    "write/apply instructions".to_string(),
                    "snapshot creation or rollback commands".to_string(),
                ]);
                sections.push(self.render_task_cockpit_provider_prompt_section(
                    task,
                    &params.agents,
                    &params.instance_ids,
                    &mut redactor,
                )?);
            }
        }

        if params.action == LlmPromptActionKind::TaskCockpit {
            sections.push("Required output: return only valid JSON. Do not wrap it in Markdown fences. Use the exact shape requested in the task preflight section. All prose values must use the requested output language. Keep agent ids and skill names unchanged. Treat all recommendations as copy-only and read-only; do not include commands to execute.".to_string());
        } else {
            sections.push("Required output: concise Markdown draft guidance in the requested output language, with evidence notes, uncertainty, and safe next steps. Use narrow, pane-friendly Markdown: prefer bullets and short subsections. Do not use Markdown tables. Do not wrap the answer in fenced code blocks. For score breakdowns, write one bullet per component in the form `component: score - issue - evidence`. Mark all suggestions copy-only.".to_string());
        }
        let estimated_output_tokens = match params.action {
            LlmPromptActionKind::Analyze => 700,
            LlmPromptActionKind::Recommend => 500,
            LlmPromptActionKind::ExplainConflict => 650,
            LlmPromptActionKind::DraftFrontmatter => 450,
            LlmPromptActionKind::TaskCockpit => 1400,
        };
        let prompt_preview = sections.join("\n\n");
        let redaction = redactor.summary();

        Ok(BuiltLlmPrompt {
            prompt_preview,
            prompt_scope,
            included_fields,
            excluded_fields,
            redaction,
            estimated_output_tokens,
        })
    }

    pub(crate) fn render_skill_prompt_section(
        &self,
        skill: &SkillDetailRecord,
        redactor: &mut PromptRedactor<'_>,
    ) -> Result<String, ServiceError> {
        let findings = self.llm_findings_for_skill(skill)?;
        let finding_lines = if findings.is_empty() {
            "none".to_string()
        } else {
            findings
                .iter()
                .take(12)
                .map(|finding| {
                    format!(
                        "- {} severity={} message={} suggestion={}",
                        redactor.redact(&finding.rule_id),
                        redactor.redact(&finding.severity),
                        redactor.redact(&finding.message),
                        finding
                            .suggestion
                            .as_deref()
                            .map(|suggestion| redactor.redact(suggestion))
                            .unwrap_or_else(|| "none".to_string())
                    )
                })
                .collect::<Vec<_>>()
                .join("\n")
        };
        Ok(format!(
            "Selected skill:\n- id: {}\n- name: {}\n- agent: {}\n- scope: {}\n- enabled: {}\n- description: {}\n\nRedacted frontmatter:\n{}\n\nRedacted body:\n{}\n\nRelated findings:\n{}",
            redactor.redact(&skill.id),
            redactor.redact(&skill.name),
            redactor.redact(&skill.agent),
            redactor.redact(&skill.scope),
            skill.enabled,
            redactor.redact(&skill.description),
            redactor.redact(&skill.frontmatter_raw),
            redactor.redact(&skill.body),
            finding_lines
        ))
    }

    pub(crate) fn render_task_cockpit_provider_prompt_section(
        &self,
        task: &str,
        agents: &[String],
        instance_ids: &[String],
        redactor: &mut PromptRedactor<'_>,
    ) -> Result<String, ServiceError> {
        let adapter_ctx = self.effective_adapter_ctx()?;
        let capabilities = list_adapter_capabilities(&adapter_ctx);
        let diagnostics = list_adapter_diagnostics(&adapter_ctx);
        let catalog = self.open_existing_catalog_read_only()?;
        let catalog_available = catalog.is_some();
        let visible_skills = match catalog.as_ref() {
            Some(catalog) => self.list_visible_skill_records(catalog)?,
            None => Vec::new(),
        };
        let selected_agents = selected_task_cockpit_agents(agents, &visible_skills, &capabilities);
        let selected_agent_set = selected_agents
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();
        let candidate_id_set = instance_ids
            .iter()
            .map(String::as_str)
            .collect::<BTreeSet<_>>();

        let mut effective_skills = Vec::new();
        if let Some(catalog) = catalog.as_ref() {
            for skill in visible_skills
                .iter()
                .filter(|skill| selected_agent_set.contains(skill.agent.as_str()))
                .filter(|skill| {
                    candidate_id_set.is_empty() || candidate_id_set.contains(skill.id.as_str())
                })
                .filter(|skill| skill.enabled && skill.state == "loaded")
                .take(320)
            {
                let description = catalog
                    .get_skill_detail(&skill.id)?
                    .map(|detail| detail.description)
                    .unwrap_or_default();
                effective_skills.push(serde_json::json!({
                    "id": redactor.redact(&skill.id),
                    "name": redactor.redact(&skill.name),
                    "agent": redactor.redact(&skill.agent),
                    "definition_id": redactor.redact(&skill.definition_id),
                    "scope": redactor.redact(&skill.scope),
                    "state": redactor.redact(&skill.state),
                    "enabled": skill.enabled,
                    "description": truncate_chars(&redactor.redact(&description), 500),
                }));
            }
        }

        let agent_summaries = selected_agents
            .iter()
            .map(|agent| {
                let capability = capabilities
                    .iter()
                    .find(|capability| capability.agent == agent.as_str());
                let diagnostic = diagnostics
                    .iter()
                    .find(|diagnostic| diagnostic.agent == agent.as_str());
                let active_skill_count = effective_skills
                    .iter()
                    .filter(|skill| skill.get("agent").and_then(Value::as_str) == Some(agent.as_str()))
                    .count();
                serde_json::json!({
                    "agent": redactor.redact(agent),
                    "display_name": capability
                        .map(|capability| redactor.redact(capability.display_name))
                        .unwrap_or_else(|| redactor.redact(agent)),
                    "status": capability
                        .map(|capability| redactor.redact(capability.status))
                        .or_else(|| diagnostic.map(|diagnostic| redactor.redact(diagnostic.status)))
                        .unwrap_or_else(|| "unknown".to_string()),
                    "active_skill_count": active_skill_count,
                    "capabilities": capability.map(|capability| serde_json::json!({
                        "scan": capability.scan.supported,
                        "project_scan": capability.project_scan.supported,
                        "config_toggle": capability.config_toggle.supported,
                        "config_snapshot": capability.config_snapshot.supported,
                        "install": capability.install.supported,
                        "writable": capability.writable.supported,
                    })),
                    "blockers": capability
                        .map(|capability| capability.blockers.iter().map(|value| redactor.redact(value)).collect::<Vec<_>>())
                        .unwrap_or_default(),
                })
            })
            .collect::<Vec<_>>();

        let payload = serde_json::json!({
            "feature": {
                "name": "Task Preflight",
                "description": "Read-only preflight that decides whether a user task is ready for agent handoff, which selected agent and effective skill fit best, what needs human confirmation, and what information is missing.",
                "requirements": [
                    "Compare the task with selected agents and effective skills by product/resource, action intent, required permissions, and likely execution risk.",
                    "Prefer exact product/resource matches over broad or semantically adjacent matches.",
                    "When multiple candidates are close, include the best three candidates with short reasons.",
                    "Do not recommend unavailable agents or skills outside the selected agent scope.",
                    "Do not invent hidden tools, credentials, network access, command execution, or write capability.",
                    "Mark handoff as needs review when the task requires command execution, network/API access, credentials, unclear scope, or ambiguous resource ownership."
                ]
            },
            "task": redactor.redact(task),
            "selected_agents": selected_agents.iter().map(|agent| redactor.redact(agent)).collect::<Vec<_>>(),
            "catalog_available": catalog_available,
            "agent_summaries": agent_summaries,
            "effective_skills": effective_skills,
            "output_schema": {
                "generated_by": "provider-task-cockpit",
                "catalog_available": true,
                "filters": {
                    "task_text": "<same redacted task>",
                    "agents": ["<selected agent id>"]
                },
                "summary": {
                    "task_text": "<same redacted task>",
                    "summary": "<one concise user-facing finding>",
                    "recommended_agent": "<agent id or null>",
                    "recommended_skill_name": "<skill name or null>",
                    "readiness_score": "0-100 integer",
                    "routing_score": "0-100 integer",
                    "agent_candidate_count": "integer",
                    "skill_candidate_count": "integer",
                    "gap_count": "integer",
                    "blocker_count": "integer"
                },
                "agent_candidates": [
                    {
                        "id": "agent:<agent id>",
                        "rank": 1,
                        "title": "<display name>",
                        "agent": "<agent id>",
                        "score": "0-100 integer",
                        "summary": "<why this agent fits or does not fit>",
                        "reasons": ["<short reason>"]
                    }
                ],
                "skill_candidates": [
                    {
                        "id": "skill:<skill id>",
                        "rank": 1,
                        "title": "<skill name>",
                        "agent": "<agent id>",
                        "skill": {
                            "instance_id": "<skill id>",
                            "name": "<skill name>",
                            "agent": "<agent id>",
                            "definition_id": "<definition id>"
                        },
                        "score": "0-100 integer",
                        "routing_score": "0-100 integer",
                        "readiness_score": "0-100 integer",
                        "summary": "<short reason this skill fits>",
                        "reasons": ["<short reason>"]
                    }
                ],
                "readiness_signals": [
                    {
                        "id": "signal:<short id>",
                        "title": "<signal title>",
                        "detail": "<brief user-facing process note>",
                        "status": "ready|review|blocked",
                        "agent": "<agent id or null>"
                    }
                ],
                "gap_rows": [
                    {
                        "id": "gap:<short id>",
                        "title": "<missing information>",
                        "detail": "<what the user should add>",
                        "severity": "info|warning|critical",
                        "agent": "<agent id or null>"
                    }
                ],
                "blocker_rows": [
                    {
                        "id": "blocker:<short id>",
                        "title": "<handoff blocker>",
                        "detail": "<why it blocks or needs confirmation>",
                        "severity": "info|warning|critical",
                        "agent": "<agent id or null>"
                    }
                ],
                "safety_flags": {
                    "provider_request_sent": true,
                    "write_back_allowed": false,
                    "script_execution_allowed": false,
                    "raw_prompt_persisted": false,
                    "raw_response_persisted": false,
                    "notes": ["copy-only recommendation"]
                }
            }
        });

        let payload_text =
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string());
        Ok(format!(
            "Task preflight provider input:\n{}\n\nReturn only JSON matching `output_schema`. The UI will display the top recommended path, the top three skill candidates, key reasons, and concise process notes.",
            payload_text
        ))
    }

    pub(crate) fn llm_findings_for_skill(
        &self,
        skill: &SkillDetailRecord,
    ) -> Result<Vec<RuleFindingRecord>, ServiceError> {
        let Some(catalog) = self.open_existing_catalog_read_only()? else {
            return Ok(Vec::new());
        };
        Ok(catalog
            .list_rule_findings()?
            .into_iter()
            .filter(|finding| {
                finding.instance_id.as_deref() == Some(skill.id.as_str())
                    || finding.definition_id.as_deref() == Some(skill.definition_id.as_str())
            })
            .collect())
    }

    pub fn prepare_llm_action(
        &self,
        params: LlmPrepareActionParams,
    ) -> Result<LlmPrepareActionResult, ServiceError> {
        let status = self.llm_status();
        let action = params.kind;
        let mut prompt_scope = vec!["operation metadata".to_string()];
        let estimated_input_tokens = match action {
            LlmActionKind::Analyze | LlmActionKind::DraftFrontmatter => {
                let instance_id = params.skill_instance_id.as_deref().ok_or_else(|| {
                    ServiceError::InvalidRequest(format!(
                        "llm.prepareAction {} requires skill_instance_id",
                        action.as_str()
                    ))
                })?;
                let skill = self.get_llm_skill_detail(instance_id)?;
                prompt_scope.extend([
                    "selected skill name".to_string(),
                    "selected skill description".to_string(),
                    "selected skill frontmatter".to_string(),
                    "selected skill body".to_string(),
                ]);
                estimate_tokens(&[
                    action.as_str(),
                    &skill.name,
                    &skill.description,
                    &skill.frontmatter_raw,
                    &skill.body,
                    params.user_intent.as_deref().unwrap_or_default(),
                ])
            }
            LlmActionKind::Recommend => {
                prompt_scope.extend([
                    "user intent".to_string(),
                    "catalog recommendation constraints".to_string(),
                ]);
                estimate_tokens(&[
                    action.as_str(),
                    params.user_intent.as_deref().unwrap_or_default(),
                ])
            }
            LlmActionKind::ExplainConflict => {
                prompt_scope.extend([
                    "current conflict summaries".to_string(),
                    "current rule finding summaries".to_string(),
                ]);
                let summary = self.llm_conflict_summary()?;
                estimate_tokens(&[
                    action.as_str(),
                    &summary,
                    params.user_intent.as_deref().unwrap_or_default(),
                ])
            }
        };
        let estimated_output_tokens = match action {
            LlmActionKind::Analyze => 700,
            LlmActionKind::Recommend => 500,
            LlmActionKind::ExplainConflict => 650,
            LlmActionKind::DraftFrontmatter => 450,
        };
        let estimated_total_tokens = estimated_input_tokens
            .saturating_add(estimated_output_tokens)
            .min(status.single_request_token_limit);
        let reason = status.reason.clone();

        Ok(LlmPrepareActionResult {
            action: action.as_str(),
            allowed: status.enabled && status.configured,
            reason: reason.clone(),
            disabled_reason: Some(reason.clone()),
            requires_confirmation: true,
            write_back_allowed: false,
            draft_requires_user_copy: true,
            provider: status.provider.clone(),
            model: status.model.clone(),
            estimated_input_tokens,
            estimated_output_tokens,
            estimated_total_tokens,
            estimated_cost_usd: 0.0,
            single_request_token_limit: status.single_request_token_limit,
            monthly_budget_usd: status.monthly_budget_usd,
            credentials_storage: status.credentials_storage.clone(),
            credential_persistence_allowed: status.credential_persistence_allowed,
            prompt_scope,
            privacy_notes: vec![
                "No credentials are read, logged, stored in SQLite, or written to the project directory.".to_string(),
                "This method does not execute a provider request and performs no network I/O.".to_string(),
                "Any future LLM output must remain a draft; writes require explicit user copy or a separate non-LLM write action.".to_string(),
            ],
            confirmation: LlmConfirmationRequirement {
                required: true,
                message: "User confirmation is required before any future LLM provider request."
                    .to_string(),
                display_fields: vec![
                    "provider",
                    "model",
                    "estimated_total_tokens",
                    "estimated_cost_usd",
                    "prompt_scope",
                ],
            },
        })
    }

    pub(crate) fn get_llm_skill_detail(
        &self,
        instance_id: &str,
    ) -> Result<SkillDetailRecord, ServiceError> {
        let Some(catalog) = self.open_existing_catalog_read_only()? else {
            return Err(ServiceError::SkillNotFound(instance_id.to_string()));
        };
        catalog
            .get_skill_detail(instance_id)?
            .ok_or_else(|| ServiceError::SkillNotFound(instance_id.to_string()))
    }

    pub(crate) fn llm_conflict_summary(&self) -> Result<String, ServiceError> {
        let Some(catalog) = self.open_existing_catalog_read_only()? else {
            return Ok(
                "No catalog is available; no conflicts or findings were loaded.".to_string(),
            );
        };
        let conflicts = catalog.list_conflict_groups()?;
        let findings = catalog.list_rule_findings()?;
        let mut lines = Vec::new();
        for conflict in conflicts.iter().take(20) {
            lines.push(format!(
                "conflict reason={} definition_id={} instances={}",
                conflict.reason,
                conflict.definition_id,
                conflict.instance_ids.len()
            ));
        }
        for finding in findings.iter().take(20) {
            lines.push(format!(
                "finding rule={} severity={} has_instance={} has_suggestion={}",
                finding.rule_id,
                finding.severity,
                finding.instance_id.is_some(),
                finding.suggestion.is_some()
            ));
        }
        if lines.is_empty() {
            Ok("No current conflicts or findings were loaded.".to_string())
        } else {
            Ok(lines.join("\n"))
        }
    }
}

#[derive(Debug, Clone, Copy)]
struct ProviderActivityResolvedQuery {
    requested_window_days: Option<i64>,
    requested_start_at: Option<i64>,
    requested_end_at: Option<i64>,
    resolved_start_at: Option<i64>,
    resolved_end_at: Option<i64>,
}

fn resolve_provider_activity_query(
    params: &ListProviderActivityParams,
    cursor: Option<&KeysetCursor>,
    now: i64,
) -> Result<ProviderActivityResolvedQuery, ServiceError> {
    let requested_window_days = params.window_days.map(|days| days.clamp(1, 3_650));
    let mut start_at = params.start_at.filter(|value| *value >= 0);
    let mut end_at = params.end_at.filter(|value| *value >= 0);
    if let (Some(start), Some(end)) = (start_at, end_at) {
        if start > end {
            start_at = Some(end);
            end_at = Some(start);
        }
    }
    let (resolved_start_at, resolved_end_at) = if start_at.is_some() || end_at.is_some() {
        (start_at, end_at)
    } else if let Some(days) = requested_window_days {
        if let Some(cursor) = cursor {
            let start = cursor.resolved_start_at.ok_or_else(|| {
                ServiceError::InvalidRequest(
                    "rolling activity cursor is missing fixed bounds".to_string(),
                )
            })?;
            let end = cursor.resolved_end_at.ok_or_else(|| {
                ServiceError::InvalidRequest(
                    "rolling activity cursor is missing fixed bounds".to_string(),
                )
            })?;
            (Some(start), Some(end))
        } else {
            let duration = days.saturating_mul(86_400_000);
            (Some(now.saturating_sub(duration)), Some(now))
        }
    } else {
        (None, None)
    };
    Ok(ProviderActivityResolvedQuery {
        requested_window_days,
        requested_start_at: start_at,
        requested_end_at: end_at,
        resolved_start_at,
        resolved_end_at,
    })
}

fn provider_activity_query_digest(
    params: &ListProviderActivityParams,
    query: &ProviderActivityResolvedQuery,
) -> Result<String, ServiceError> {
    provider_activity_digest(
        "llm.listProviderActivity-query",
        &(
            normalized_observability_filter(params.provider.as_deref()),
            normalized_observability_filter(params.model.as_deref()),
            normalized_observability_filter(params.action.as_deref()),
            query.requested_window_days,
            query.requested_start_at,
            query.requested_end_at,
            query.resolved_start_at,
            query.resolved_end_at,
        ),
    )
}

fn provider_activity_digest<T: Serialize>(domain: &str, value: &T) -> Result<String, ServiceError> {
    let mut hasher = Sha256::new();
    hasher.update(domain.as_bytes());
    hasher.update([0]);
    hasher.update(serde_json::to_vec(value)?);
    Ok(format!("sha256:{}", hex_prefix(&hasher.finalize(), 64)))
}

const PROVIDER_ACTIVITY_SNAPSHOT_READ_ATTEMPTS: usize = 3;
const PROVIDER_ACTIVITY_MAX_SOURCE_BYTES: usize = 8 * 1_024 * 1_024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum ProviderActivitySource {
    PromptRuns,
    ProviderCalls,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderActivityRawSource {
    pub(crate) present: bool,
    pub(crate) bytes: Vec<u8>,
}

impl ProviderActivityRawSource {
    pub(crate) fn present(bytes: Vec<u8>) -> Self {
        Self {
            present: true,
            bytes,
        }
    }

    fn missing() -> Self {
        Self {
            present: false,
            bytes: Vec::new(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub(crate) struct ProviderActivityRawSnapshot {
    pub(crate) prompt_runs: ProviderActivityRawSource,
    pub(crate) provider_calls: ProviderActivityRawSource,
}

pub(crate) fn read_consistent_provider_activity_raw_snapshot_with<F>(
    mut read: F,
) -> Result<ProviderActivityRawSnapshot, ServiceError>
where
    F: FnMut(ProviderActivitySource) -> Result<ProviderActivityRawSource, ServiceError>,
{
    for _ in 0..PROVIDER_ACTIVITY_SNAPSHOT_READ_ATTEMPTS {
        let first_prompt_runs = read(ProviderActivitySource::PromptRuns)?;
        let first_provider_calls = read(ProviderActivitySource::ProviderCalls)?;
        let second_prompt_runs = read(ProviderActivitySource::PromptRuns)?;
        let second_provider_calls = read(ProviderActivitySource::ProviderCalls)?;
        if first_prompt_runs == second_prompt_runs && first_provider_calls == second_provider_calls
        {
            return Ok(ProviderActivityRawSnapshot {
                prompt_runs: second_prompt_runs,
                provider_calls: second_provider_calls,
            });
        }
    }
    Err(ServiceError::SourceChanged)
}

fn read_consistent_provider_activity_raw_snapshot(
    prompt_runs_path: &Path,
    provider_calls_path: &Path,
) -> Result<ProviderActivityRawSnapshot, ServiceError> {
    read_consistent_provider_activity_raw_snapshot_with(|source| match source {
        ProviderActivitySource::PromptRuns => {
            read_provider_activity_raw_source(prompt_runs_path, "prompt-runs")
        }
        ProviderActivitySource::ProviderCalls => {
            read_provider_activity_raw_source(provider_calls_path, "provider-call-metadata")
        }
    })
}

fn read_provider_activity_raw_source(
    path: &Path,
    label: &'static str,
) -> Result<ProviderActivityRawSource, ServiceError> {
    let metadata = match fs::symlink_metadata(path) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Ok(ProviderActivityRawSource::missing());
        }
        Err(_) => return Err(ServiceError::ProviderActivitySourceUnreadable(label)),
    };
    if !metadata.is_file()
        || metadata.file_type().is_symlink()
        || metadata.len() > PROVIDER_ACTIVITY_MAX_SOURCE_BYTES as u64
    {
        return Err(ServiceError::ProviderActivitySourceInvalid(label));
    }
    let bytes =
        fs::read(path).map_err(|_| ServiceError::ProviderActivitySourceUnreadable(label))?;
    if bytes.len() > PROVIDER_ACTIVITY_MAX_SOURCE_BYTES {
        return Err(ServiceError::ProviderActivitySourceInvalid(label));
    }
    Ok(ProviderActivityRawSource::present(bytes))
}

fn parse_provider_activity_prompt_runs(
    source: &ProviderActivityRawSource,
) -> Result<Vec<LlmPromptRunRecord>, ServiceError> {
    if !source.present {
        return Ok(Vec::new());
    }
    let mut rows = serde_json::from_slice::<Vec<LlmPromptRunRecord>>(&source.bytes)
        .map_err(|_| ServiceError::ProviderActivitySourceInvalid("prompt-runs"))?;
    rows.sort_by(llm_prompt_run_record_sort);
    Ok(rows)
}

fn parse_provider_activity_provider_calls(
    source: &ProviderActivityRawSource,
) -> Result<Vec<ProviderCallMetadata>, ServiceError> {
    if !source.present {
        return Ok(Vec::new());
    }
    let content = std::str::from_utf8(&source.bytes)
        .map_err(|_| ServiceError::ProviderActivitySourceInvalid("provider-call-metadata"))?;
    let mut rows = Vec::new();
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        rows.push(
            serde_json::from_str::<ProviderCallMetadata>(trimmed).map_err(|_| {
                ServiceError::ProviderActivitySourceInvalid("provider-call-metadata")
            })?,
        );
    }
    rows.sort_by(|left, right| {
        right
            .timestamp
            .cmp(&left.timestamp)
            .then_with(|| left.profile_id.cmp(&right.profile_id))
            .then_with(|| left.action_type.cmp(&right.action_type))
    });
    Ok(rows)
}

fn provider_activity_raw_source_revision(snapshot: &ProviderActivityRawSnapshot) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"provider-activity-raw-source-v1");
    for (label, source) in [
        (b"prompt-runs".as_slice(), &snapshot.prompt_runs),
        (b"provider-calls".as_slice(), &snapshot.provider_calls),
    ] {
        hasher.update([0]);
        hasher.update(label);
        hasher.update([u8::from(source.present)]);
        hasher.update(source.bytes.len().to_le_bytes());
        hasher.update(&source.bytes);
    }
    format!("sha256:{}", hex_prefix(&hasher.finalize(), 64))
}

#[derive(Debug, Default)]
struct ModelTaskMatchFilters {
    provider: Option<String>,
    model: Option<String>,
    task_kind: Option<String>,
    match_status: Option<String>,
    agent: Option<String>,
    source_kind: Option<String>,
}

impl ModelTaskMatchFilters {
    fn from_params(params: &ModelTaskMatchListParams) -> Self {
        Self {
            provider: normalized_model_task_filter(params.provider.as_deref()),
            model: normalized_model_task_filter(params.model.as_deref()),
            task_kind: normalized_model_task_filter(params.task_kind.as_deref()),
            match_status: params
                .match_status
                .as_deref()
                .map(|value| normalize_model_task_match_status(Some(value)))
                .filter(|value| !value.is_empty()),
            agent: normalized_model_task_filter(params.agent.as_deref()),
            source_kind: params
                .source_kind
                .as_deref()
                .map(|value| normalize_model_task_source_kind(Some(value)))
                .filter(|value| !value.is_empty()),
        }
    }

    fn matches_record(&self, record: &ModelTaskMatchRecord) -> bool {
        self.provider
            .as_deref()
            .is_none_or(|filter| record.provider.eq_ignore_ascii_case(filter))
            && self
                .model
                .as_deref()
                .is_none_or(|filter| record.model.eq_ignore_ascii_case(filter))
            && self
                .task_kind
                .as_deref()
                .is_none_or(|filter| record.task_kind.eq_ignore_ascii_case(filter))
            && self
                .match_status
                .as_deref()
                .is_none_or(|filter| record.match_status.eq_ignore_ascii_case(filter))
            && self.agent.as_deref().is_none_or(|filter| {
                record
                    .agent
                    .as_deref()
                    .is_some_and(|agent| agent.eq_ignore_ascii_case(filter))
            })
            && self
                .source_kind
                .as_deref()
                .is_none_or(|filter| record.source_kind.eq_ignore_ascii_case(filter))
    }

    fn matches_prompt_run(&self, run: &LlmPromptRunRecord) -> bool {
        self.provider
            .as_deref()
            .is_none_or(|filter| run.provider.eq_ignore_ascii_case(filter))
            && self
                .model
                .as_deref()
                .is_none_or(|filter| run.model.eq_ignore_ascii_case(filter))
            && self
                .task_kind
                .as_deref()
                .is_none_or(|filter| run.request_kind.eq_ignore_ascii_case(filter))
            && self.match_status.as_deref().is_none_or(|filter| {
                normalize_model_task_match_status(Some(&run.status)).eq_ignore_ascii_case(filter)
            })
            && self.agent.as_deref().is_none_or(|filter| {
                run.agent
                    .as_deref()
                    .is_some_and(|agent| agent.eq_ignore_ascii_case(filter))
            })
            && self
                .source_kind
                .as_deref()
                .is_none_or(|filter| filter == "prompt_run")
    }
}

fn normalized_model_task_filter(value: Option<&str>) -> Option<String> {
    value
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

fn redact_model_task_string_list(
    values: &[String],
    redactor: &mut PromptRedactor<'_>,
    max_chars: usize,
) -> Vec<String> {
    let mut redacted = values
        .iter()
        .filter_map(|value| {
            let trimmed = value.trim();
            if trimmed.is_empty() {
                None
            } else {
                Some(truncate_chars(&redactor.redact(trimmed), max_chars))
            }
        })
        .collect::<Vec<_>>();
    redacted.sort();
    redacted.dedup();
    redacted
}

fn selected_task_cockpit_agents(
    requested_agents: &[String],
    skills: &[SkillRecord],
    capabilities: &[AdapterCapabilityRecord],
) -> Vec<String> {
    let mut requested = requested_agents
        .iter()
        .map(|agent| agent.trim())
        .filter(|agent| !agent.is_empty())
        .collect::<BTreeSet<_>>();
    if requested.is_empty() {
        requested = skills
            .iter()
            .map(|skill| skill.agent.as_str())
            .collect::<BTreeSet<_>>();
    }
    if requested.is_empty() {
        requested = capabilities
            .iter()
            .map(|capability| capability.agent)
            .collect::<BTreeSet<_>>();
    }

    let mut ordered = Vec::new();
    for capability in capabilities {
        if requested.remove(capability.agent) {
            ordered.push(capability.agent.to_string());
        }
    }
    for agent in requested {
        ordered.push(agent.to_string());
    }
    ordered
}

fn redacted_model_task_record(
    record: &ModelTaskMatchRecord,
    redaction_roots: &[(String, &'static str)],
) -> ModelTaskMatchRecord {
    ModelTaskMatchRecord {
        id: observability_redact(&record.id, redaction_roots, 160),
        title: observability_redact(&record.title, redaction_roots, 180),
        task: observability_redact(&record.task, redaction_roots, 600),
        task_kind: observability_redact(&record.task_kind, redaction_roots, 120),
        agent: record
            .agent
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 120)),
        profile_id: record
            .profile_id
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 160)),
        provider: observability_redact(&record.provider, redaction_roots, 120),
        model: observability_redact(&record.model, redaction_roots, 160),
        destination_host: record
            .destination_host
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 160)),
        match_status: normalize_model_task_match_status(Some(&record.match_status)),
        confidence_score: record.confidence_score.map(|score| score.min(100)),
        latency_ms: record.latency_ms,
        estimated_total_tokens: record.estimated_total_tokens,
        estimated_cost_usd: record.estimated_cost_usd,
        source_kind: normalize_model_task_source_kind(Some(&record.source_kind)),
        prompt_run_ids: redact_existing_model_task_list(&record.prompt_run_ids, redaction_roots),
        benchmark_ids: redact_existing_model_task_list(&record.benchmark_ids, redaction_roots),
        evidence_refs: redact_existing_model_task_list(&record.evidence_refs, redaction_roots),
        gap_notes: redact_existing_model_task_list(&record.gap_notes, redaction_roots),
        blocker_notes: redact_existing_model_task_list(&record.blocker_notes, redaction_roots),
        outcome_notes: redact_existing_model_task_list(&record.outcome_notes, redaction_roots),
        created_at: record.created_at,
        updated_at: record.updated_at,
        redaction_summary: record.redaction_summary.clone(),
        safety_flags: model_task_match_safety_flags(record.safety_flags.read_only),
    }
}

fn redact_existing_model_task_list(
    values: &[String],
    redaction_roots: &[(String, &'static str)],
) -> Vec<String> {
    let mut values = values
        .iter()
        .filter_map(|value| {
            let redacted = observability_redact(value, redaction_roots, 300);
            if redacted.trim().is_empty() {
                None
            } else {
                Some(redacted)
            }
        })
        .collect::<Vec<_>>();
    values.sort();
    values.dedup();
    values
}

fn model_task_record_evidence_row(record: &ModelTaskMatchRecord) -> ModelTaskMatchEvidenceRow {
    ModelTaskMatchEvidenceRow {
        id: format!("model-task-match:{}", record.id),
        source: "model-task-matches.json".to_string(),
        source_kind: record.source_kind.clone(),
        title: record.title.clone(),
        task: Some(record.task.clone()),
        task_kind: record.task_kind.clone(),
        agent: record.agent.clone(),
        provider: record.provider.clone(),
        model: record.model.clone(),
        destination_host: record.destination_host.clone(),
        match_status: record.match_status.clone(),
        confidence_score: record.confidence_score,
        status: record.match_status.clone(),
        created_at: record.created_at,
        updated_at: Some(record.updated_at),
        latency_ms: record.latency_ms,
        estimated_total_tokens: record.estimated_total_tokens.unwrap_or(0),
        estimated_cost_usd: record.estimated_cost_usd.unwrap_or(0.0),
        gap_notes: record.gap_notes.clone(),
        blocker_notes: record.blocker_notes.clone(),
        outcome_notes: record.outcome_notes.clone(),
        evidence_refs: record.evidence_refs.clone(),
        redaction_status: record.redaction_summary.status.clone(),
        safety_flags: model_task_match_safety_flags(true),
    }
}

fn prompt_run_model_task_evidence_row(
    run: &LlmPromptRunRecord,
    redaction_roots: &[(String, &'static str)],
) -> ModelTaskMatchEvidenceRow {
    let mut evidence_refs = vec![format!(
        "prompt-run:{}",
        observability_redact(&run.id, redaction_roots, 160)
    )];
    if let Some(instance_id) = run.instance_id.as_deref() {
        evidence_refs.push(format!(
            "skill:{}",
            observability_redact(instance_id, redaction_roots, 160)
        ));
    }
    evidence_refs.sort();
    evidence_refs.dedup();
    let mut outcome_notes = vec![
        "Derived from redacted app-local prompt-run metadata; no raw prompt or response is returned."
            .to_string(),
    ];
    if run.provider_request_sent {
        outcome_notes.push(
            "Historical metadata records that the original confirmed prompt run sent a provider request."
                .to_string(),
        );
    }
    if run.credential_accessed {
        outcome_notes.push(
            "Historical metadata records credential access for the original confirmed prompt run."
                .to_string(),
        );
    }
    ModelTaskMatchEvidenceRow {
        id: provider_observability_row_id("model-task-prompt-run", &[&run.id]),
        source: "prompt-runs.json".to_string(),
        source_kind: "prompt_run".to_string(),
        title: format!(
            "{} on {}",
            observability_redact(&run.request_kind, redaction_roots, 120),
            observability_redact(&run.model, redaction_roots, 160)
        ),
        task: run
            .task
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 600)),
        task_kind: observability_redact(&run.request_kind, redaction_roots, 120),
        agent: run
            .agent
            .as_deref()
            .map(|value| observability_redact(value, redaction_roots, 120)),
        provider: observability_redact(&run.provider, redaction_roots, 120),
        model: observability_redact(&run.model, redaction_roots, 160),
        destination_host: Some(observability_redact(
            &run.destination_host,
            redaction_roots,
            160,
        )),
        match_status: normalize_model_task_match_status(Some(&run.status)),
        confidence_score: None,
        status: observability_redact(&run.status, redaction_roots, 80),
        created_at: run.created_at,
        updated_at: Some(run.completed_at),
        latency_ms: Some(run.duration_ms),
        estimated_total_tokens: run.estimated_total_tokens,
        estimated_cost_usd: run.estimated_cost_usd,
        gap_notes: if run.task.is_none() {
            vec!["Prompt-run metadata has no task text, so only request kind and model fit can be displayed.".to_string()]
        } else {
            Vec::new()
        },
        blocker_notes: Vec::new(),
        outcome_notes,
        evidence_refs,
        redaction_status: observability_redact(&run.redaction_summary.status, redaction_roots, 160),
        safety_flags: model_task_match_safety_flags(true),
    }
}

fn model_task_evidence_row_sort(
    left: &ModelTaskMatchEvidenceRow,
    right: &ModelTaskMatchEvidenceRow,
) -> std::cmp::Ordering {
    right
        .updated_at
        .unwrap_or(right.created_at)
        .cmp(&left.updated_at.unwrap_or(left.created_at))
        .then_with(|| left.provider.cmp(&right.provider))
        .then_with(|| left.model.cmp(&right.model))
        .then_with(|| left.id.cmp(&right.id))
}

#[derive(Debug, Default)]
struct ModelTaskGroupAccumulator {
    stored_record_count: usize,
    prompt_run_count: usize,
    fit_count: usize,
    partial_fit_count: usize,
    mismatch_count: usize,
    unknown_count: usize,
    estimated_total_tokens: u64,
    estimated_cost_usd: f64,
    latest_activity_at: Option<i64>,
    evidence_refs: Vec<String>,
}

impl ModelTaskGroupAccumulator {
    fn add(&mut self, row: &ModelTaskMatchEvidenceRow) {
        if row.source_kind == "prompt_run" {
            self.prompt_run_count += 1;
        } else {
            self.stored_record_count += 1;
        }
        match row.match_status.as_str() {
            "fit" => self.fit_count += 1,
            "partial_fit" => self.partial_fit_count += 1,
            "mismatch" => self.mismatch_count += 1,
            _ => self.unknown_count += 1,
        }
        self.estimated_total_tokens += u64::from(row.estimated_total_tokens);
        self.estimated_cost_usd += row.estimated_cost_usd;
        let activity_at = row.updated_at.unwrap_or(row.created_at);
        self.latest_activity_at = self.latest_activity_at.max(Some(activity_at));
        append_model_task_unique(&mut self.evidence_refs, &row.evidence_refs);
    }
}

fn model_task_model_rows(
    rows: &[ModelTaskMatchEvidenceRow],
    limit: usize,
) -> Vec<ModelTaskMatchModelRow> {
    let mut groups: BTreeMap<(String, String, Option<String>), ModelTaskGroupAccumulator> =
        BTreeMap::new();
    for row in rows {
        groups
            .entry((
                row.provider.clone(),
                row.model.clone(),
                row.destination_host.clone(),
            ))
            .or_default()
            .add(row);
    }
    let mut rows = groups
        .into_iter()
        .map(
            |((provider, model, destination_host), group)| ModelTaskMatchModelRow {
                id: provider_observability_row_id(
                    "model-task-model",
                    &[&provider, &model, destination_host.as_deref().unwrap_or("")],
                ),
                provider,
                model,
                destination_host,
                stored_record_count: group.stored_record_count,
                prompt_run_count: group.prompt_run_count,
                fit_count: group.fit_count,
                partial_fit_count: group.partial_fit_count,
                mismatch_count: group.mismatch_count,
                unknown_count: group.unknown_count,
                estimated_total_tokens: group.estimated_total_tokens,
                estimated_cost_usd: group.estimated_cost_usd,
                latest_activity_at: group.latest_activity_at,
                evidence_refs: group.evidence_refs,
            },
        )
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .latest_activity_at
            .cmp(&left.latest_activity_at)
            .then_with(|| left.provider.cmp(&right.provider))
            .then_with(|| left.model.cmp(&right.model))
    });
    rows.truncate(limit);
    rows
}

fn model_task_task_rows(
    rows: &[ModelTaskMatchEvidenceRow],
    limit: usize,
) -> Vec<ModelTaskMatchTaskRow> {
    let mut groups: BTreeMap<(String, String), ModelTaskGroupAccumulator> = BTreeMap::new();
    for row in rows {
        groups
            .entry((row.task_kind.clone(), row.match_status.clone()))
            .or_default()
            .add(row);
    }
    let mut rows = groups
        .into_iter()
        .map(|((task_kind, status), group)| ModelTaskMatchTaskRow {
            id: provider_observability_row_id("model-task-task", &[&task_kind, &status]),
            task_kind,
            status,
            stored_record_count: group.stored_record_count,
            prompt_run_count: group.prompt_run_count,
            fit_count: group.fit_count,
            partial_fit_count: group.partial_fit_count,
            mismatch_count: group.mismatch_count,
            unknown_count: group.unknown_count,
            estimated_total_tokens: group.estimated_total_tokens,
            estimated_cost_usd: group.estimated_cost_usd,
            latest_activity_at: group.latest_activity_at,
            evidence_refs: group.evidence_refs,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .latest_activity_at
            .cmp(&left.latest_activity_at)
            .then_with(|| left.task_kind.cmp(&right.task_kind))
            .then_with(|| left.status.cmp(&right.status))
    });
    rows.truncate(limit);
    rows
}

fn model_task_match_summary(
    stored_record_count: usize,
    prompt_run_count: usize,
    returned_record_count: usize,
    returned_prompt_run_count: usize,
    model_rows: &[ModelTaskMatchModelRow],
    task_rows: &[ModelTaskMatchTaskRow],
    evidence_rows: &[ModelTaskMatchEvidenceRow],
) -> ModelTaskMatchSummary {
    let mut summary = ModelTaskMatchSummary {
        stored_record_count,
        prompt_run_count,
        returned_record_count,
        returned_prompt_run_count,
        model_count: model_rows.len(),
        task_kind_count: task_rows.len(),
        fit_count: 0,
        partial_fit_count: 0,
        mismatch_count: 0,
        unknown_count: 0,
        estimated_total_tokens: 0,
        estimated_cost_usd: 0.0,
        latest_activity_at: None,
        summary: String::new(),
    };
    for row in evidence_rows {
        match row.match_status.as_str() {
            "fit" => summary.fit_count += 1,
            "partial_fit" => summary.partial_fit_count += 1,
            "mismatch" => summary.mismatch_count += 1,
            _ => summary.unknown_count += 1,
        }
        summary.estimated_total_tokens += u64::from(row.estimated_total_tokens);
        summary.estimated_cost_usd += row.estimated_cost_usd;
        summary.latest_activity_at = summary
            .latest_activity_at
            .max(Some(row.updated_at.unwrap_or(row.created_at)));
    }
    summary.summary = if evidence_rows.is_empty() {
        "No app-local model-task match history or prompt-run model/task metadata matched the selected filters."
            .to_string()
    } else {
        format!(
            "Returned {} model-task evidence row(s) across {} model grouping(s) and {} task grouping(s).",
            evidence_rows.len(),
            summary.model_count,
            summary.task_kind_count
        )
    };
    summary
}

fn model_task_match_gap_notes(
    stored_record_count: usize,
    prompt_run_count: usize,
    returned_record_count: usize,
    returned_prompt_run_count: usize,
) -> Vec<String> {
    let mut notes = Vec::new();
    if stored_record_count == 0 {
        notes.push(
            "No app-local model-task match records exist yet; only prompt-run metadata can be summarized."
                .to_string(),
        );
    }
    if prompt_run_count == 0 {
        notes.push(
            "No app-local prompt-run metadata exists yet; historical provider/model usage is unavailable."
                .to_string(),
        );
    }
    if stored_record_count > 0 && returned_record_count == 0 {
        notes.push("Stored match records exist but none matched the selected filters.".to_string());
    }
    if prompt_run_count > 0 && returned_prompt_run_count == 0 {
        notes.push("Prompt-run metadata exists but none matched the selected filters.".to_string());
    }
    notes
}

fn model_task_match_evidence_references(
    stored_record_count: usize,
    prompt_run_count: usize,
    evidence_rows: &[ModelTaskMatchEvidenceRow],
) -> Vec<LlmProviderObservabilityEvidenceReference> {
    let mut references = Vec::new();
    references.push(LlmProviderObservabilityEvidenceReference {
        id: "app-data:model-task-matches.json".to_string(),
        kind: "app-local-file",
        label: "model-task-matches.json".to_string(),
        source: "model-task-matches.json".to_string(),
    });
    references.push(LlmProviderObservabilityEvidenceReference {
        id: "app-data:prompt-runs.json".to_string(),
        kind: "app-local-file",
        label: "prompt-runs.json".to_string(),
        source: "prompt-runs.json".to_string(),
    });
    if stored_record_count == 0 && prompt_run_count == 0 && evidence_rows.is_empty() {
        return references;
    }
    for row in evidence_rows {
        references.push(LlmProviderObservabilityEvidenceReference {
            id: row.id.clone(),
            kind: "model-task-evidence",
            label: row.title.clone(),
            source: row.source.clone(),
        });
    }
    references.sort_by(|left, right| left.id.cmp(&right.id));
    references.dedup_by(|left, right| left.id == right.id);
    references
}

fn append_model_task_unique(target: &mut Vec<String>, values: &[String]) {
    for value in values {
        if !target.contains(value) {
            target.push(value.clone());
        }
    }
}
