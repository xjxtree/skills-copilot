use super::*;
use crate::service_keyset_cursor::{decode_cursor_for_method, encode_cursor, KeysetCursor};
use skills_copilot_ai_core::{
    AiResponseContract, AiResultSchema, AI_RESPONSE_ENVELOPE_SCHEMA_VERSION,
};
use skills_copilot_commands::{
    action_descriptor, action_preview_binding, action_source_revision, ensure_action_confirmed,
    ActionPrecondition, ActionPreconditionKind, ActionReadbackObservation,
};
use skills_copilot_core::{
    ActionDescriptor, ActionImpact, ActionIntent, ActionKind, ActionNetworkPosture,
    ActionReadbackDomain, ActionTargetKind, ActionTargetRef, EvidenceKind, EvidenceRef,
    SessionContinuationRecord, SkillAggregateRecord,
};

const TASK_COCKPIT_MAX_EFFECTIVE_SKILLS: usize = 24;
const TASK_COCKPIT_MAX_PROMPT_TOKENS: u32 = 12_000;
const LLM_MAX_CONTEXT_ACTIONS: usize = 64;
const LLM_MAX_CONTEXT_EVIDENCE: usize = 128;
const LLM_MAX_SEARCH_CANDIDATES: usize = 18;

struct LlmResponseEvidenceContext {
    contract: AiResponseContract,
    session: Option<SessionContinuationRecord>,
}

fn llm_prompt_runs_revision_while_locked(owner: &AppMutationLock) -> Result<String, ServiceError> {
    let bytes = owner
        .owner_fs()
        .read_bounded_regular_file(
            Path::new(crate::service_host::LLM_PROMPT_RUNS_RELATIVE_PATH),
            crate::service_host::LLM_PROMPT_RUNS_MAX_BYTES,
            "LLM prompt run history",
        )?
        .unwrap_or_default();
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot/prompt-runs-revision/v1");
    hasher.update((bytes.len() as u64).to_be_bytes());
    hasher.update(bytes);
    Ok(format!("sha256:{:x}", hasher.finalize()))
}

fn llm_remote_unknown(detail: impl Into<String>, cleanup_required: bool) -> ServiceError {
    CommandError::PartialEffect {
        operation: "LLM provider prompt".to_string(),
        state: "remote_unknown",
        cleanup_required,
        detail: detail.into(),
    }
    .into()
}

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
        self.preview_llm_prompt_with_owner(params, None)
    }

    fn preview_llm_prompt_with_owner(
        &self,
        params: LlmPreviewPromptParams,
        owner: Option<&AppMutationLock>,
    ) -> Result<LlmPreviewPromptResult, ServiceError> {
        let profile = self.resolve_llm_prompt_profile_for(params.profile_id.as_deref(), owner)?;
        let built = self.build_llm_prompt_for(&params, owner)?;
        let provider = profile
            .as_ref()
            .map(|profile| profile.provider_type.as_str().to_string());
        let model = profile.as_ref().map(|profile| profile.model.clone());
        let endpoint = profile.as_ref().map(|profile| profile.base_url.clone());
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
        let task_cockpit_budget_exceeded = params.action == LlmPromptActionKind::TaskCockpit
            && estimated_total_tokens > TASK_COCKPIT_MAX_PROMPT_TOKENS;
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
            Some(_) if task_cockpit_budget_exceeded => (
                false,
                format!(
                    "Task readiness prompt estimate exceeds the {} token safety budget; narrow the selected agents or skills.",
                    TASK_COCKPIT_MAX_PROMPT_TOKENS
                ),
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
        let profile_store_revision = match owner {
            Some(owner) => crate::provider::provider_profiles_revision_while_locked(owner)?,
            None => crate::provider::provider_profiles_revision(&self.app_data_dir)?,
        };
        let profile_revision = profile
            .as_ref()
            .map(crate::provider::provider_profile_nonsecret_revision)
            .transpose()?
            .unwrap_or_else(|| "missing".to_string());
        let action_state_revision = match owner {
            Some(owner) => {
                crate::service_provider_actions::provider_action_state_revision_while_locked(owner)?
            }
            None => {
                crate::service_provider_actions::provider_action_state_revision(&self.app_data_dir)?
            }
        };
        let prompt_revision =
            action_source_revision("llm.redactedPrompt", &[("prompt", &built.prompt_preview)])?;
        let adapter_ctx = match owner {
            Some(owner) => self.effective_adapter_ctx_while_mutation_owner_held(owner)?,
            None => self.effective_adapter_ctx()?,
        };
        let project_id =
            skills_copilot_commands::canonical_project_id(adapter_ctx.project_root.as_deref());
        if project_id.as_deref() != Some(built.response_contract.project_id.as_str()) {
            return Err(ServiceError::SourceChanged);
        }
        let target_id = profile_id
            .clone()
            .unwrap_or_else(|| "unconfigured-provider".to_string());
        let endpoint_binding = endpoint
            .clone()
            .unwrap_or_else(|| "unconfigured".to_string());
        let model_binding = model.clone().unwrap_or_else(|| "unconfigured".to_string());
        let cost_binding = format!("{estimated_cost_usd:.12}");
        let input_tokens_binding = estimated_input_tokens.to_string();
        let output_tokens_binding = estimated_output_tokens.to_string();
        let source_revision = action_source_revision(
            "llm.confirmPromptAndSend",
            &[
                ("request_kind", params.action.as_str()),
                ("profile_id", &target_id),
                ("profile_store", &profile_store_revision),
                ("profile", &profile_revision),
                ("endpoint", &endpoint_binding),
                ("model", &model_binding),
                ("prompt", &prompt_revision),
                ("product_source", &built.response_contract.source_revision),
                ("estimated_input_tokens", &input_tokens_binding),
                ("estimated_output_tokens", &output_tokens_binding),
                ("estimated_cost_usd", &cost_binding),
                ("action_state", &action_state_revision),
            ],
        )?;
        let mut evidence_refs = vec![
            format!("provider-profile:{target_id}"),
            format!("redacted-prompt:{prompt_revision}"),
            format!("prompt-preview:{preview_id}"),
        ];
        evidence_refs.extend(
            params
                .instance_ids
                .iter()
                .chain(params.skill_instance_id.iter())
                .map(|instance_id| format!("skill:{instance_id}")),
        );
        evidence_refs.extend(
            params
                .agents
                .iter()
                .map(|agent| format!("agent:{}", agent.trim().to_ascii_lowercase())),
        );
        if let Some(project_id) = project_id.as_ref() {
            evidence_refs.push(format!("project:{project_id}"));
        }
        evidence_refs.extend(
            built
                .response_contract
                .evidence
                .iter()
                .map(|reference| reference.id.clone()),
        );
        evidence_refs.sort();
        evidence_refs.dedup();
        let action = action_descriptor(
            ActionKind::ProviderPrompt,
            ActionIntent::SendProviderPrompt,
            ActionTargetRef {
                kind: ActionTargetKind::ProviderProfile,
                id: target_id,
                agent: None,
                scope: None,
            },
            project_id,
            vec![ActionImpact::AppLocalData],
            "llm.previewPrompt",
            Some("llm.confirmPromptAndSend"),
            source_revision,
            true,
            ActionNetworkPosture::Required,
            vec![
                ActionReadbackDomain::ProviderActivity,
                ActionReadbackDomain::PromptRuns,
            ],
            evidence_refs,
        )?;
        let binding = action_preview_binding(
            action,
            vec![
                ActionPrecondition {
                    kind: ActionPreconditionKind::ProviderProfile,
                    target_id: "provider-profiles".to_string(),
                    expected_revision: profile_store_revision,
                },
                ActionPrecondition {
                    kind: ActionPreconditionKind::PromptContext,
                    target_id: "redacted-prompt".to_string(),
                    expected_revision: prompt_revision,
                },
                ActionPrecondition {
                    kind: ActionPreconditionKind::PromptContext,
                    target_id: "provider-action-state".to_string(),
                    expected_revision: action_state_revision,
                },
                ActionPrecondition {
                    kind: ActionPreconditionKind::PromptContext,
                    target_id: "product-evidence".to_string(),
                    expected_revision: built.response_contract.source_revision.clone(),
                },
            ],
        )?;

        Ok(LlmPreviewPromptResult {
            preview_id,
            status: if allowed { "ready" } else { "blocked" }.to_string(),
            allowed,
            reason,
            request_kind: params.action.as_str(),
            binding,
            profile_id,
            provider,
            model,
            endpoint,
            destination_host,
            prompt_scope: built.prompt_scope,
            included_fields: built.included_fields,
            excluded_fields: built.excluded_fields,
            redaction: built.redaction,
            prompt_preview: built.prompt_preview,
            response_contract: built.response_contract,
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
                    "response_contract",
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
        let preliminary = self
            .preview_llm_prompt(params.request.clone())
            .map_err(crate::service_provider_actions::classify_provider_action_preflight_error)?;
        if !preliminary.allowed {
            return Err(ServiceError::InvalidRequest(preliminary.reason));
        }
        ensure_action_confirmed(&preliminary.binding, Some(&params.action_confirmation))?;

        let owner = crate::service_provider_actions::lock_provider_action_owner_for_apply(
            &self.app_data_dir,
        )?;
        let preview = self.preview_llm_prompt_with_owner(params.request.clone(), Some(&owner))?;
        if !preview.allowed {
            return Err(ServiceError::InvalidRequest(preview.reason));
        }
        ensure_action_confirmed(&preview.binding, Some(&params.action_confirmation))?;
        owner.validate_owner_path_binding()?;
        crate::service_provider_actions::reserve_provider_action_while_locked(
            &self.app_data_dir,
            &owner,
            &preview.binding,
            &params.action_confirmation,
        )?;
        let profile_id = preview.profile_id.clone().ok_or_else(|| {
            ServiceError::InvalidRequest(
                "No provider profile is available for the confirmed prompt.".to_string(),
            )
        })?;
        let confirmation_id = preview.binding.action.id.clone();
        let send = match crate::provider::send_provider_prompt_while_locked(
            &self.app_data_dir,
            &owner,
            SendProviderPromptParams {
                profile_id: profile_id.clone(),
                confirmation_id: confirmation_id.clone(),
                action_type: llm_prompt_action_type(&params.request),
                prompt: preview.prompt_preview.clone(),
                estimated_input_tokens: preview.estimated_input_tokens,
                estimated_output_tokens: preview.estimated_output_tokens,
                estimated_cost_usd: preview.estimated_cost_usd,
                redaction_status: preview.redaction.status.clone(),
                timeout_ms: params.timeout_ms,
                response_contract: preview.response_contract.clone(),
            },
        ) {
            Ok(send) => send,
            Err(error) => {
                crate::service_provider_actions::finalize_provider_action_while_locked(
                    &self.app_data_dir,
                    &owner,
                    &preview.binding,
                    &params.action_confirmation,
                    crate::service_provider_actions::ProviderActionState::NotStarted,
                )?;
                return Err(ServiceError::ActionNotStarted(format!(
                    "provider prompt did not start: {error}; create a new preview before retrying"
                )));
            }
        };
        crate::service_provider_actions::run_provider_action_post_effect_hook();
        let target_matches = send.profile_id == profile_id
            && preview.model.as_deref() == Some(send.model.as_str())
            && preview.destination_host.as_deref() == Some(send.destination_host.as_str());
        let response_envelope_verified = send.response_envelope.as_ref().is_some_and(|envelope| {
            preview
                .response_contract
                .validate_envelope(envelope)
                .is_ok()
        });
        let evidence_snapshot_current = self
            .accept_active_product_snapshot_while_locked(
                Some(&preview.response_contract.source_revision),
                &owner,
            )
            .is_ok_and(|snapshot| snapshot.project_id == preview.response_contract.project_id);
        let remote_result_verified = !send.provider_request_sent
            || (send.status.eq_ignore_ascii_case("succeeded")
                && response_envelope_verified
                && evidence_snapshot_current)
            || send
                .audit
                .error_code
                .as_deref()
                .is_some_and(|code| code.starts_with("http_"));
        let prompt_record_persisted = self
            .record_llm_prompt_run_while_locked(&owner, &params, &preview, &send)
            .is_ok();
        let mut readback = if target_matches
            && remote_result_verified
            && send.local_metadata_persisted
            && prompt_record_persisted
        {
            let activity_revision =
                crate::provider::provider_call_metadata_revision_while_locked(&owner);
            let prompt_runs_revision = llm_prompt_runs_revision_while_locked(&owner);
            match (activity_revision, prompt_runs_revision) {
                (Ok(activity_revision), Ok(prompt_runs_revision)) => {
                    ActionReadbackRecord::verified(
                        &preview.binding.action,
                        vec![
                            ActionReadbackObservation {
                                domain: ActionReadbackDomain::ProviderActivity,
                                target_id: profile_id.clone(),
                                revision: activity_revision,
                            },
                            ActionReadbackObservation {
                                domain: ActionReadbackDomain::PromptRuns,
                                target_id: preview.preview_id.clone(),
                                revision: prompt_runs_revision,
                            },
                        ],
                    )
                    .ok()
                }
                _ => None,
            }
        } else {
            None
        };
        if readback.is_some() && owner.validate_owner_path_binding().is_err() {
            readback = None;
        }
        if readback.is_some() {
            if crate::service_provider_actions::finalize_provider_action_while_locked(
                &self.app_data_dir,
                &owner,
                &preview.binding,
                &params.action_confirmation,
                crate::service_provider_actions::ProviderActionState::Verified,
            )
            .is_err()
            {
                readback = None;
            } else if owner.validate_owner_path_binding().is_err() {
                if send.provider_request_sent {
                    return Err(llm_remote_unknown(
                        "the provider request may have left the process, and its finalized metadata owner is no longer bound to the configured path",
                        false,
                    ));
                }
                return Err(CommandError::PartialEffect {
                    operation: "LLM provider prompt".to_string(),
                    state: "applied_unverified",
                    cleanup_required: false,
                    detail: "LLM prompt metadata was finalized on an app-data owner that is no longer bound to the configured path".to_string(),
                }
                .into());
            }
        }
        let partial_outcome = if readback.is_none() {
            let finalization =
                crate::service_provider_actions::finalize_provider_action_while_locked(
                    &self.app_data_dir,
                    &owner,
                    &preview.binding,
                    &params.action_confirmation,
                    crate::service_provider_actions::ProviderActionState::Partial,
                );
            if send.provider_request_sent {
                let local_cleanup_required =
                    !send.local_metadata_persisted || !prompt_record_persisted;
                return Err(llm_remote_unknown(
                    if !target_matches {
                        "the provider response target did not match the confirmed prompt target"
                    } else if !remote_result_verified {
                        "the provider request may have left the process, but its evidence-bound response or current source revision could not be verified"
                    } else if !send.local_metadata_persisted {
                        "the provider request completed, but app-local provider audit persistence failed"
                    } else if !prompt_record_persisted {
                        "the provider request completed, but app-local prompt-run persistence failed"
                    } else if finalization.is_err() {
                        "the provider request completed, but replay finalization could not be verified"
                    } else {
                        "the provider request completed, but semantic read-back or app-data owner binding could not be verified"
                    },
                    local_cleanup_required,
                ));
            }
            finalization?;
            owner.validate_owner_path_binding().map_err(|_| {
                CommandError::PartialEffect {
                    operation: "LLM provider prompt".to_string(),
                    state: "applied_unverified",
                    cleanup_required: false,
                    detail: "LLM prompt metadata was updated on an app-data owner that is no longer bound to the configured path".to_string(),
                }
            })?;
            Some(LlmPromptPartialOutcome {
                remote_effect: "not_sent".to_string(),
                local_record: if send.local_metadata_persisted && prompt_record_persisted {
                    "applied_unverified".to_string()
                } else {
                    "incomplete".to_string()
                },
                recovery:
                    "Review provider activity before creating a new preview; this action reference cannot be replayed."
                        .to_string(),
            })
        } else {
            None
        };
        let status = if partial_outcome.is_some() {
            "partial".to_string()
        } else {
            send.status.clone()
        };

        Ok(LlmConfirmPromptAndSendResult {
            preview_id: preview.preview_id,
            confirmation_id,
            status,
            request_kind: params.request.action.as_str(),
            profile_id,
            provider: send.provider_type.as_str().to_string(),
            model: send.model,
            destination_host: send.destination_host,
            provider_request_sent: send.provider_request_sent,
            credential_accessed: send.credential_accessed,
            draft_output: send.output_text,
            response_envelope: send.response_envelope,
            draft_requires_user_copy: true,
            write_back_allowed: false,
            script_execution_allowed: false,
            config_mutation_allowed: false,
            snapshot_created: false,
            triage_mutation_allowed: false,
            audit: send.audit,
            readback,
            partial_outcome,
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
        let owner = match lock_app_mutations(&self.app_data_dir) {
            Ok(owner) => Some(owner),
            Err(CommandError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let adapter_ctx = match owner.as_ref() {
            Some(owner) => self.effective_adapter_ctx_while_mutation_owner_held(owner)?,
            None => self.adapter_ctx.clone(),
        };
        let redaction_roots = self.trace_redaction_roots(&adapter_ctx);
        let filters = ProviderObservabilityFilters::from_params(&params);

        let (prompt_runs, mut status_rows) =
            self.load_llm_prompt_runs_for_observability(owner.as_ref(), &redaction_roots);
        let (call_metadata, call_status_rows) =
            self.load_provider_call_metadata_for_observability(owner.as_ref(), &redaction_roots);
        status_rows.extend(call_status_rows);
        let (profiles, profile_status_rows) =
            self.load_provider_profiles_for_observability(owner.as_ref(), &redaction_roots);
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
            .list_model_task_matches_for_owner(
                ModelTaskMatchListParams {
                    provider: params.provider.clone(),
                    model: params.model.clone(),
                    task_kind: params.action.clone(),
                    match_status: None,
                    agent: None,
                    source_kind: None,
                    limit: Some(limit),
                },
                owner.as_ref(),
            )?
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
        let owner = match lock_app_mutations(&self.app_data_dir) {
            Ok(owner) => Some(owner),
            Err(CommandError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        let adapter_ctx = match owner.as_ref() {
            Some(owner) => self.effective_adapter_ctx_while_mutation_owner_held(owner)?,
            None => self.adapter_ctx.clone(),
        };
        let redaction_roots = self.trace_redaction_roots(&adapter_ctx);
        let filters = ProviderObservabilityFilters::from_activity_bounds(
            &params,
            query.resolved_start_at,
            query.resolved_end_at,
        );
        let raw_snapshot = match owner.as_ref() {
            Some(owner) => read_consistent_provider_activity_raw_snapshot_while_locked(owner)?,
            None => ProviderActivityRawSnapshot {
                prompt_runs: ProviderActivityRawSource::missing(),
                provider_calls: ProviderActivityRawSource::missing(),
            },
        };
        let source_revision = provider_activity_raw_source_revision(&raw_snapshot);
        let prompt_runs = parse_provider_activity_prompt_runs(&raw_snapshot.prompt_runs)?;
        let call_metadata = parse_provider_activity_provider_calls(&raw_snapshot.provider_calls)?;

        let prompt_ids = prompt_runs
            .iter()
            .map(provider_activity_prompt_run_id)
            .collect::<Result<Vec<_>, _>>()?;
        let provider_call_ids = call_metadata
            .iter()
            .map(provider_activity_provider_call_id)
            .collect::<Result<Vec<_>, _>>()?;
        let mut unique_ids = BTreeSet::new();
        if prompt_ids
            .iter()
            .chain(provider_call_ids.iter())
            .any(|id| !unique_ids.insert(id.as_str()))
        {
            return Err(ServiceError::ProviderActivitySourceInvalid(
                "activity-identifiers",
            ));
        }

        let mut rows = prompt_runs
            .iter()
            .zip(prompt_ids)
            .filter(|(run, _)| filters.matches_prompt_run(run))
            .enumerate()
            .map(
                |(index, (run, stable_id))| -> Result<ProviderActivityRow, ServiceError> {
                    Ok(provider_activity_history_row(
                        provider_observability_history_row(run, index, &redaction_roots),
                        stable_id,
                    ))
                },
            )
            .collect::<Result<Vec<_>, _>>()?;
        rows.extend(
            call_metadata
                .iter()
                .zip(provider_call_ids)
                .filter(|(metadata, _)| filters.matches_provider_call(metadata))
                .enumerate()
                .map(
                    |(index, (metadata, stable_id))| -> Result<ProviderActivityRow, ServiceError> {
                        Ok(provider_activity_call_row(
                            provider_observability_call_row(metadata, index, &redaction_roots),
                            stable_id,
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
                        processed_prefix_digest: None,
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
        let owner = match lock_app_mutations(&self.app_data_dir) {
            Ok(owner) => Some(owner),
            Err(CommandError::Io(error)) if error.kind() == std::io::ErrorKind::NotFound => None,
            Err(error) => return Err(error.into()),
        };
        self.list_model_task_matches_for_owner(params, owner.as_ref())
    }

    fn list_model_task_matches_for_owner(
        &self,
        params: ModelTaskMatchListParams,
        owner: Option<&AppMutationLock>,
    ) -> Result<ModelTaskMatchListResult, ServiceError> {
        let limit = params.limit.map(|limit| limit.clamp(1, 500));
        let row_limit = limit.unwrap_or(usize::MAX);
        let adapter_ctx = match owner {
            Some(owner) => self.effective_adapter_ctx_while_mutation_owner_held(owner)?,
            None => self.adapter_ctx.clone(),
        };
        let redaction_roots = self.trace_redaction_roots(&adapter_ctx);
        let filters = ModelTaskMatchFilters::from_params(&params);
        let (stored_records, prompt_runs) = match owner {
            Some(owner) => (
                self.load_model_task_matches_while_locked(owner)?,
                self.load_llm_prompt_runs_while_locked(owner)?,
            ),
            None => (Vec::new(), Vec::new()),
        };

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

    fn resolve_llm_prompt_profile_for(
        &self,
        requested_profile_id: Option<&str>,
        owner: Option<&AppMutationLock>,
    ) -> Result<Option<ProviderProfileRecord>, ServiceError> {
        let profiles = match owner {
            Some(owner) => crate::provider::list_provider_profiles_while_locked(owner)?,
            None => list_provider_profiles(&self.app_data_dir)?,
        };
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

    fn build_llm_prompt_for(
        &self,
        params: &LlmPreviewPromptParams,
        owner: Option<&AppMutationLock>,
    ) -> Result<BuiltLlmPrompt, ServiceError> {
        let adapter_ctx = match owner {
            Some(owner) => self.effective_adapter_ctx_while_mutation_owner_held(owner)?,
            None => self.effective_adapter_ctx()?,
        };
        let roots = self.redaction_roots(&adapter_ctx);
        let mut redactor = PromptRedactor::new(&roots);
        let mut response_context = self.llm_response_evidence_context_for(params, owner)?;
        let accepted_response_contract = response_context.contract.clone();
        for reference in &mut response_context.contract.evidence {
            reference.summary = truncate_chars(&redactor.redact(&reference.summary), 320);
        }
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
            LlmPromptActionKind::Analyze
            | LlmPromptActionKind::DraftFrontmatter
            | LlmPromptActionKind::SkillChangeReview => {
                let instance_id = params.skill_instance_id.as_deref().ok_or_else(|| {
                    ServiceError::InvalidRequest(format!(
                        "llm.previewPrompt {} requires skill_instance_id",
                        params.action.as_str()
                    ))
                })?;
                let skill = self.get_llm_skill_detail_for(instance_id, owner)?;
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
                sections.push(self.render_skill_prompt_section_for(
                    &skill,
                    &mut redactor,
                    owner,
                )?);
                if params.action == LlmPromptActionKind::SkillChangeReview {
                    sections.push(
                        "Review only the bounded current skill evidence and reported findings. Describe observed changes or uncertainty; do not invent a prior version when no comparison evidence exists."
                            .to_string(),
                    );
                }
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
                let summary = self.llm_conflict_summary_for(owner)?;
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
                    "redacted task readiness prompt".to_string(),
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
                    "task readiness feature description, evaluation rules, and output schema"
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
                sections.push(self.render_task_cockpit_provider_prompt_section_for(
                    task,
                    &params.agents,
                    &params.instance_ids,
                    &mut redactor,
                    owner,
                )?);
            }
            LlmPromptActionKind::SessionDigest => {
                let session = response_context.session.as_ref().ok_or_else(|| {
                    ServiceError::InvalidRequest(
                        "llm.previewPrompt session_digest requires accepted session evidence"
                            .to_string(),
                    )
                })?;
                prompt_scope.extend([
                    "selected session continuation evidence".to_string(),
                    "bounded session intent metadata".to_string(),
                    "required evidence-bound response schema".to_string(),
                ]);
                included_fields.extend([
                    "session id, agent, title, and timing".to_string(),
                    "accepted native and project revisions".to_string(),
                    "redacted intent summary when available".to_string(),
                    "typed session evidence references".to_string(),
                ]);
                excluded_fields.extend([
                    "raw transcript files".to_string(),
                    "raw source paths".to_string(),
                    "resume argv or commands".to_string(),
                    "write/apply instructions".to_string(),
                ]);
                sections.push(render_session_digest_prompt_section(session, &mut redactor));
            }
            LlmPromptActionKind::ProjectHealth => {
                prompt_scope.extend([
                    "accepted deterministic project readiness".to_string(),
                    "per-agent health and coverage".to_string(),
                    "bounded blockers and attention evidence".to_string(),
                    "required evidence-bound response schema".to_string(),
                ]);
                included_fields.extend([
                    "project health and source coverage".to_string(),
                    "per-agent effective skill, issue, and conflict counts".to_string(),
                    "bounded blocker and attention summaries".to_string(),
                    "typed evidence references".to_string(),
                ]);
                excluded_fields.extend([
                    "raw source paths".to_string(),
                    "raw configuration contents".to_string(),
                    "write/apply instructions".to_string(),
                ]);
                sections.push(render_project_health_prompt_section(
                    &response_context.contract,
                    &mut redactor,
                ));
            }
            LlmPromptActionKind::SemanticSearch => {
                prompt_scope.extend([
                    "current lexical search query".to_string(),
                    "already-returned bounded local candidates".to_string(),
                    "required evidence-bound rerank schema".to_string(),
                ]);
                included_fields.extend([
                    "redacted search query".to_string(),
                    "candidate evidence ids, kinds, titles, and subtitles".to_string(),
                ]);
                excluded_fields.extend([
                    "new filesystem scans".to_string(),
                    "additional catalog or session reads".to_string(),
                    "raw source paths".to_string(),
                    "write/apply instructions".to_string(),
                ]);
                sections.push(render_semantic_search_prompt_section(
                    params,
                    &response_context.contract,
                    &mut redactor,
                )?);
            }
        }

        sections.push(render_ai_response_contract(&response_context.contract)?);
        sections.push("Required output: return only the exact JSON response envelope described above, without Markdown fences or extra text. Cite only allowed evidence and action ids. The result is copy-only interpretation; it must not contain commands, argv, scripts, apply methods, preview tokens, action confirmations, mutation instructions, hidden task state, or persistence claims.".to_string());
        let estimated_output_tokens = match params.action {
            LlmPromptActionKind::Analyze => 700,
            LlmPromptActionKind::Recommend => 500,
            LlmPromptActionKind::ExplainConflict => 650,
            LlmPromptActionKind::DraftFrontmatter => 450,
            LlmPromptActionKind::TaskCockpit => 1400,
            LlmPromptActionKind::SessionDigest => 800,
            LlmPromptActionKind::SkillChangeReview => 900,
            LlmPromptActionKind::ProjectHealth => 700,
            LlmPromptActionKind::SemanticSearch => 650,
        };
        let prompt_preview = sections.join("\n\n");
        let current_response_context = self.llm_response_evidence_context_for(params, owner)?;
        if current_response_context.contract != accepted_response_contract {
            return Err(ServiceError::SourceChanged);
        }
        let redaction = redactor.summary();

        Ok(BuiltLlmPrompt {
            prompt_preview,
            prompt_scope,
            included_fields,
            excluded_fields,
            redaction,
            estimated_output_tokens,
            response_contract: response_context.contract,
        })
    }

    fn llm_response_evidence_context_for(
        &self,
        params: &LlmPreviewPromptParams,
        owner: Option<&AppMutationLock>,
    ) -> Result<LlmResponseEvidenceContext, ServiceError> {
        let snapshot = match owner {
            Some(owner) => self.accept_active_product_snapshot_while_locked(
                params.source_revision.as_deref(),
                owner,
            )?,
            None => self.accept_active_product_snapshot(params.source_revision.as_deref())?,
        };
        let readiness = &snapshot.projection.readiness;
        let mut evidence = Vec::new();
        let mut actions = Vec::new();
        let mut session = None;
        let result_schema = match params.action {
            LlmPromptActionKind::TaskCockpit => {
                let requested_agents = params
                    .agents
                    .iter()
                    .map(|agent| agent.trim())
                    .filter(|agent| !agent.is_empty())
                    .collect::<BTreeSet<_>>();
                let requested_instances = params
                    .instance_ids
                    .iter()
                    .map(|instance| instance.trim())
                    .filter(|instance| !instance.is_empty())
                    .collect::<BTreeSet<_>>();
                let task = params.user_intent.as_deref().unwrap_or_default();
                let mut aggregates = snapshot
                    .projection
                    .skill_aggregates
                    .iter()
                    .filter(|aggregate| {
                        requested_agents.is_empty()
                            || aggregate
                                .agents
                                .iter()
                                .any(|agent| requested_agents.contains(agent.as_str()))
                    })
                    .filter(|aggregate| {
                        requested_instances.is_empty()
                            || aggregate
                                .instance_ids
                                .iter()
                                .any(|id| requested_instances.contains(id.as_str()))
                    })
                    .collect::<Vec<_>>();
                aggregates.sort_by(|left, right| {
                    task_cockpit_aggregate_relevance(task, right)
                        .cmp(&task_cockpit_aggregate_relevance(task, left))
                        .then_with(|| left.id.cmp(&right.id))
                });
                aggregates.truncate(TASK_COCKPIT_MAX_EFFECTIVE_SKILLS);
                for aggregate in aggregates {
                    evidence.extend(aggregate.evidence.clone());
                    actions.extend(aggregate.actions.clone());
                }
                evidence.extend(
                    readiness
                        .evidence
                        .iter()
                        .filter(|reference| {
                            reference.kind == EvidenceKind::ScanCoverage
                                && (requested_agents.is_empty()
                                    || reference.agent.is_some_and(|agent| {
                                        requested_agents.contains(agent.as_str())
                                    }))
                        })
                        .cloned(),
                );
                for attention in readiness.attention.iter().take(24).filter(|item| {
                    requested_agents.is_empty()
                        || item
                            .agent
                            .is_some_and(|agent| requested_agents.contains(agent.as_str()))
                }) {
                    append_known_evidence(
                        &mut evidence,
                        &readiness.evidence,
                        &attention.evidence_refs,
                    )?;
                    append_known_actions(&mut actions, &readiness.actions, &attention.action_ids)?;
                }
                AiResultSchema::TaskReadiness
            }
            LlmPromptActionKind::SessionDigest => {
                let input = params.session.as_ref().ok_or_else(|| {
                    ServiceError::InvalidRequest(
                        "llm.previewPrompt session_digest requires session".to_string(),
                    )
                })?;
                let resume_params = crate::service_local_sessions::SessionResumePreviewParams {
                    authorized_roots: input.authorized_roots.clone(),
                    auto_discover: input.auto_discover,
                    agent: input.agent.clone(),
                    project_root: input.project_root.clone(),
                    current_cwd: input.current_cwd.clone(),
                    session_id: input.session_id.clone(),
                    expected_source_revision: input.source_revision.clone(),
                    expected_snapshot_revision: input.snapshot_revision.clone(),
                };
                let accepted = match owner {
                    Some(owner) => self.preview_session_resume_while_locked(
                        resume_params,
                        &snapshot.source_revision,
                        Some(snapshot.project_id.clone()),
                        owner,
                    )?,
                    None => self.preview_session_resume(
                        resume_params,
                        &snapshot.source_revision,
                        Some(snapshot.project_id.clone()),
                    )?,
                };
                evidence.extend(accepted.evidence.clone());
                actions.extend(accepted.actions.clone());
                session = Some(accepted);
                AiResultSchema::SessionDigest
            }
            LlmPromptActionKind::Analyze
            | LlmPromptActionKind::DraftFrontmatter
            | LlmPromptActionKind::SkillChangeReview => {
                let instance_id = params.skill_instance_id.as_deref().ok_or_else(|| {
                    ServiceError::InvalidRequest(format!(
                        "llm.previewPrompt {} requires skill_instance_id",
                        params.action.as_str()
                    ))
                })?;
                let aggregate = snapshot
                    .projection
                    .skill_aggregates
                    .iter()
                    .find(|aggregate| {
                        aggregate.id == instance_id
                            || aggregate.instance_ids.iter().any(|id| id == instance_id)
                    })
                    .ok_or_else(|| {
                        ServiceError::InvalidRequest(
                            "selected skill is not present in the accepted product snapshot"
                                .to_string(),
                        )
                    })?;
                evidence.extend(aggregate.evidence.clone());
                actions.extend(aggregate.actions.clone());
                if params.action == LlmPromptActionKind::SkillChangeReview {
                    AiResultSchema::SkillChangeReview
                } else {
                    AiResultSchema::CopyOnlyMarkdown
                }
            }
            LlmPromptActionKind::Recommend | LlmPromptActionKind::ExplainConflict => {
                evidence.extend(
                    readiness
                        .evidence
                        .iter()
                        .filter(|reference| reference.kind == EvidenceKind::ScanCoverage)
                        .cloned(),
                );
                for attention in readiness.attention.iter().take(24) {
                    append_known_evidence(
                        &mut evidence,
                        &readiness.evidence,
                        &attention.evidence_refs,
                    )?;
                    append_known_actions(&mut actions, &readiness.actions, &attention.action_ids)?;
                }
                AiResultSchema::CopyOnlyMarkdown
            }
            LlmPromptActionKind::ProjectHealth => {
                evidence.extend(
                    readiness
                        .evidence
                        .iter()
                        .filter(|reference| reference.kind == EvidenceKind::ScanCoverage)
                        .cloned(),
                );
                for agent in &readiness.agents {
                    append_known_evidence(
                        &mut evidence,
                        &readiness.evidence,
                        &agent.evidence_refs,
                    )?;
                }
                for attention in readiness.attention.iter().take(24) {
                    append_known_evidence(
                        &mut evidence,
                        &readiness.evidence,
                        &attention.evidence_refs,
                    )?;
                }
                AiResultSchema::CopyOnlyMarkdown
            }
            LlmPromptActionKind::SemanticSearch => {
                let query = params
                    .user_intent
                    .as_deref()
                    .map(str::trim)
                    .filter(|query| !query.is_empty())
                    .ok_or_else(|| {
                        ServiceError::InvalidRequest(
                            "llm.previewPrompt semantic_search requires user_intent/query"
                                .to_string(),
                        )
                    })?;
                validate_semantic_search_candidates(query, &params.search_candidates)?;
                for (index, candidate) in params.search_candidates.iter().enumerate() {
                    evidence.push(semantic_search_candidate_evidence(
                        index,
                        candidate,
                        &snapshot.source_revision,
                    ));
                }
                AiResultSchema::SemanticRerank
            }
        };

        if evidence.is_empty() {
            evidence.extend(readiness.evidence.iter().take(1).cloned());
        }
        normalize_response_contract_members(
            &snapshot.project_id,
            &readiness.evidence,
            &mut evidence,
            &mut actions,
        )?;
        if evidence.len() > LLM_MAX_CONTEXT_EVIDENCE {
            return Err(ServiceError::InvalidRequest(
                "AI evidence context exceeds the local safety bound".to_string(),
            ));
        }
        if actions.len() > LLM_MAX_CONTEXT_ACTIONS {
            return Err(ServiceError::InvalidRequest(
                "AI action reference context exceeds the local safety bound".to_string(),
            ));
        }
        let contract = AiResponseContract::new(
            params.action.as_str(),
            snapshot.project_id,
            snapshot.source_revision,
            result_schema,
            evidence,
            actions,
        )
        .map_err(|error| ServiceError::InvalidRequest(error.to_string()))?;
        Ok(LlmResponseEvidenceContext { contract, session })
    }

    fn render_skill_prompt_section_for(
        &self,
        skill: &SkillDetailRecord,
        redactor: &mut PromptRedactor<'_>,
        owner: Option<&AppMutationLock>,
    ) -> Result<String, ServiceError> {
        let findings = self.llm_findings_for_skill_for(skill, owner)?;
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

    fn render_task_cockpit_provider_prompt_section_for(
        &self,
        task: &str,
        agents: &[String],
        instance_ids: &[String],
        redactor: &mut PromptRedactor<'_>,
        owner: Option<&AppMutationLock>,
    ) -> Result<String, ServiceError> {
        let adapter_ctx = match owner {
            Some(owner) => self.effective_adapter_ctx_while_mutation_owner_held(owner)?,
            None => self.effective_adapter_ctx()?,
        };
        let capabilities = list_adapter_capabilities(&adapter_ctx);
        let diagnostics = list_adapter_diagnostics(&adapter_ctx);
        let catalog = match owner {
            Some(owner) => self.open_existing_catalog_read_only_while_locked(owner)?,
            None => self.open_existing_catalog_read_only()?,
        };
        let catalog_available = catalog.is_some();
        let visible_skills = match catalog.as_ref() {
            Some(catalog) => match owner {
                Some(owner) => self.list_visible_skill_records_while_locked(catalog, owner)?,
                None => self.list_visible_skill_records(catalog)?,
            },
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
        let mut eligible_skills = visible_skills
            .iter()
            .filter(|skill| selected_agent_set.contains(skill.agent.as_str()))
            .filter(|skill| {
                candidate_id_set.is_empty() || candidate_id_set.contains(skill.id.as_str())
            })
            .filter(|skill| skill.enabled && skill.state == "loaded")
            .collect::<Vec<_>>();
        eligible_skills.sort_by(|left, right| {
            task_cockpit_skill_relevance(task, right)
                .cmp(&task_cockpit_skill_relevance(task, left))
                .then_with(|| left.name.cmp(&right.name))
                .then_with(|| left.id.cmp(&right.id))
        });
        let eligible_skill_count = eligible_skills.len();
        if let Some(catalog) = catalog.as_ref() {
            for skill in eligible_skills
                .into_iter()
                .take(TASK_COCKPIT_MAX_EFFECTIVE_SKILLS)
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
                    "description": truncate_chars(&redactor.redact(&description), 320),
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
                "name": "Task Readiness",
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
            "candidate_selection": {
                "eligible_skill_count": eligible_skill_count,
                "included_skill_count": effective_skills.len(),
                "limit": TASK_COCKPIT_MAX_EFFECTIVE_SKILLS,
                "strategy": if candidate_id_set.is_empty() { "task-relevance" } else { "explicit-instance-selection" }
            },
            "agent_summaries": agent_summaries,
            "effective_skills": effective_skills,
            "result_schema": {
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
                ]
            }
        });

        let payload_text =
            serde_json::to_string_pretty(&payload).unwrap_or_else(|_| "{}".to_string());
        Ok(format!(
            "Task preflight provider input:\n{}\n\nPlace a value matching `result_schema` inside the evidence-bound response envelope. The UI will display the top recommended path, the top three skill candidates, key reasons, and concise process notes.",
            payload_text
        ))
    }

    fn llm_findings_for_skill_for(
        &self,
        skill: &SkillDetailRecord,
        owner: Option<&AppMutationLock>,
    ) -> Result<Vec<RuleFindingRecord>, ServiceError> {
        let catalog = match owner {
            Some(owner) => self.open_existing_catalog_read_only_while_locked(owner)?,
            None => self.open_existing_catalog_read_only()?,
        };
        let Some(catalog) = catalog else {
            return Ok(Vec::new());
        };
        let findings = catalog.list_rule_findings()?;
        Ok(user_visible_rule_findings(&findings)
            .into_iter()
            .filter(|finding| finding.instance_id.as_deref() == Some(skill.id.as_str()))
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
            LlmActionKind::Analyze
            | LlmActionKind::DraftFrontmatter
            | LlmActionKind::SkillChangeReview => {
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
            LlmActionKind::SkillChangeReview => 900,
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
        self.get_llm_skill_detail_for(instance_id, None)
    }

    fn get_llm_skill_detail_for(
        &self,
        instance_id: &str,
        owner: Option<&AppMutationLock>,
    ) -> Result<SkillDetailRecord, ServiceError> {
        let catalog = match owner {
            Some(owner) => self.open_existing_catalog_read_only_while_locked(owner)?,
            None => self.open_existing_catalog_read_only()?,
        };
        let Some(catalog) = catalog else {
            return Err(ServiceError::SkillNotFound(instance_id.to_string()));
        };
        let adapter_ctx = match owner {
            Some(owner) => self.effective_adapter_ctx_while_mutation_owner_held(owner)?,
            None => self.effective_adapter_ctx()?,
        };
        let mut detail = catalog
            .get_skill_detail(instance_id)?
            .ok_or_else(|| ServiceError::SkillNotFound(instance_id.to_string()))?;
        apply_current_config_overrides_to_skill_detail(&adapter_ctx, &mut detail)?;
        Ok(detail)
    }

    pub(crate) fn llm_conflict_summary(&self) -> Result<String, ServiceError> {
        self.llm_conflict_summary_for(None)
    }

    fn llm_conflict_summary_for(
        &self,
        owner: Option<&AppMutationLock>,
    ) -> Result<String, ServiceError> {
        let catalog = match owner {
            Some(owner) => self.open_existing_catalog_read_only_while_locked(owner)?,
            None => self.open_existing_catalog_read_only()?,
        };
        let Some(catalog) = catalog else {
            return Ok(
                "No catalog is available; no conflicts or findings were loaded.".to_string(),
            );
        };
        let adapter_ctx = match owner {
            Some(owner) => self.effective_adapter_ctx_while_mutation_owner_held(owner)?,
            None => self.effective_adapter_ctx()?,
        };
        let conflicts = list_conflicts_for_context(&catalog, &adapter_ctx)?;
        let findings = user_visible_rule_findings(&catalog.list_rule_findings()?);
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
pub(crate) const PROVIDER_ACTIVITY_MAX_SOURCE_BYTES: usize = 8 * 1_024 * 1_024;

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

fn read_consistent_provider_activity_raw_snapshot_while_locked(
    owner: &AppMutationLock,
) -> Result<ProviderActivityRawSnapshot, ServiceError> {
    read_consistent_provider_activity_raw_snapshot_with(|source| match source {
        ProviderActivitySource::PromptRuns => read_provider_activity_raw_source_while_locked(
            owner,
            Path::new(crate::service_host::LLM_PROMPT_RUNS_RELATIVE_PATH),
            "prompt-runs",
        ),
        ProviderActivitySource::ProviderCalls => read_provider_activity_raw_source_while_locked(
            owner,
            Path::new(crate::provider::PROVIDER_CALL_METADATA_RELATIVE_PATH),
            "provider-call-metadata",
        ),
    })
}

fn read_provider_activity_raw_source_while_locked(
    owner: &AppMutationLock,
    relative_path: &Path,
    label: &'static str,
) -> Result<ProviderActivityRawSource, ServiceError> {
    match owner.owner_fs().read_bounded_regular_file(
        relative_path,
        PROVIDER_ACTIVITY_MAX_SOURCE_BYTES as u64,
        label,
    ) {
        Ok(Some(bytes)) => Ok(ProviderActivityRawSource::present(bytes)),
        Ok(None) => Ok(ProviderActivityRawSource::missing()),
        Err(CommandError::UnsafeConfigPath(_)) => {
            Err(ServiceError::ProviderActivitySourceInvalid(label))
        }
        Err(_) => Err(ServiceError::ProviderActivitySourceUnreadable(label)),
    }
}

#[cfg(test)]
pub(crate) fn read_provider_activity_raw_source(
    path: &Path,
    label: &'static str,
) -> Result<ProviderActivityRawSource, ServiceError> {
    let mut file = match open_provider_activity_source(path) {
        Ok(file) => file,
        Err(_) => return classify_provider_activity_open_failure(path, label),
    };
    let metadata = file
        .metadata()
        .map_err(|_| ServiceError::ProviderActivitySourceUnreadable(label))?;
    if !metadata.is_file() || metadata.len() > PROVIDER_ACTIVITY_MAX_SOURCE_BYTES as u64 {
        return Err(ServiceError::ProviderActivitySourceInvalid(label));
    }
    let bytes = read_provider_activity_bounded(&mut file, label)?;
    Ok(ProviderActivityRawSource::present(bytes))
}

#[cfg(unix)]
#[cfg(test)]
fn open_provider_activity_source(path: &Path) -> std::io::Result<fs::File> {
    use rustix::fs::{open, Mode, OFlags};

    let flags =
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC;
    open(path, flags, Mode::empty())
        .map(fs::File::from)
        .map_err(std::io::Error::from)
}

#[cfg(not(unix))]
#[cfg(test)]
fn open_provider_activity_source(path: &Path) -> std::io::Result<fs::File> {
    fs::File::open(path)
}

#[cfg(test)]
fn classify_provider_activity_open_failure(
    path: &Path,
    label: &'static str,
) -> Result<ProviderActivityRawSource, ServiceError> {
    match fs::symlink_metadata(path) {
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            Ok(ProviderActivityRawSource::missing())
        }
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(ServiceError::ProviderActivitySourceInvalid(label))
        }
        _ => Err(ServiceError::ProviderActivitySourceUnreadable(label)),
    }
}

#[cfg(test)]
pub(crate) fn read_provider_activity_bounded<R: std::io::Read>(
    reader: &mut R,
    label: &'static str,
) -> Result<Vec<u8>, ServiceError> {
    let mut bytes = vec![0_u8; PROVIDER_ACTIVITY_MAX_SOURCE_BYTES + 1];
    let mut length = 0;
    while length < bytes.len() {
        let count = reader
            .read(&mut bytes[length..])
            .map_err(|_| ServiceError::ProviderActivitySourceUnreadable(label))?;
        if count == 0 {
            break;
        }
        length += count;
    }
    bytes.truncate(length);
    if bytes.len() > PROVIDER_ACTIVITY_MAX_SOURCE_BYTES {
        return Err(ServiceError::ProviderActivitySourceInvalid(label));
    }
    Ok(bytes)
}

fn parse_provider_activity_prompt_runs(
    source: &ProviderActivityRawSource,
) -> Result<Vec<LlmPromptRunRecord>, ServiceError> {
    if !source.present {
        return Ok(Vec::new());
    }
    let mut rows = serde_json::from_slice::<Vec<LlmPromptRunRecord>>(&source.bytes)
        .map_err(|_| ServiceError::ProviderActivitySourceInvalid("prompt-runs"))?;
    crate::service_host::strip_llm_prompt_run_bodies(&mut rows);
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
        let mut metadata = serde_json::from_str::<ProviderCallMetadata>(trimmed)
            .map_err(|_| ServiceError::ProviderActivitySourceInvalid("provider-call-metadata"))?;
        metadata.timestamp = crate::provider::normalize_epoch_millis(metadata.timestamp);
        rows.push(metadata);
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

fn task_cockpit_skill_relevance(task: &str, skill: &SkillRecord) -> u32 {
    let task = task.to_lowercase();
    let name = skill.name.to_lowercase();
    let definition = skill.definition_id.to_lowercase();
    let mut score = 0u32;
    if !name.trim().is_empty() && task.contains(name.trim()) {
        score = score.saturating_add(100);
    }
    let task_terms = task
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| term.chars().count() >= 3)
        .collect::<BTreeSet<_>>();
    for term in task_terms {
        if name.contains(term) {
            score = score.saturating_add(12);
        }
        if definition.contains(term) {
            score = score.saturating_add(8);
        }
        if skill.agent.to_lowercase().contains(term) {
            score = score.saturating_add(3);
        }
    }
    score
}

fn task_cockpit_aggregate_relevance(task: &str, aggregate: &SkillAggregateRecord) -> u32 {
    let task = task.to_lowercase();
    let name = aggregate.canonical_name.to_lowercase();
    let display_name = aggregate.display_name.to_lowercase();
    let description = aggregate.description.to_lowercase();
    let mut score = 0u32;
    if !name.trim().is_empty() && task.contains(name.trim()) {
        score = score.saturating_add(100);
    }
    for term in task
        .split(|character: char| !character.is_alphanumeric())
        .filter(|term| term.chars().count() >= 3)
        .collect::<BTreeSet<_>>()
    {
        if name.contains(term) || display_name.contains(term) {
            score = score.saturating_add(12);
        }
        if description.contains(term) {
            score = score.saturating_add(6);
        }
    }
    score
}

fn validate_semantic_search_candidates(
    query: &str,
    candidates: &[LlmSearchCandidateParams],
) -> Result<(), ServiceError> {
    if query.chars().count() > 500 {
        return Err(ServiceError::InvalidRequest(
            "semantic search query exceeds 500 characters".to_string(),
        ));
    }
    if candidates.is_empty() || candidates.len() > LLM_MAX_SEARCH_CANDIDATES {
        return Err(ServiceError::InvalidRequest(format!(
            "semantic search requires 1 to {LLM_MAX_SEARCH_CANDIDATES} bounded candidates"
        )));
    }
    let mut ids = BTreeSet::new();
    for candidate in candidates {
        let id = candidate.id.trim();
        let title = candidate.title.trim();
        if id.is_empty()
            || id.chars().count() > 320
            || title.is_empty()
            || title.chars().count() > 320
            || candidate.subtitle.chars().count() > 640
            || !matches!(
                candidate.kind.as_str(),
                "skill" | "session" | "config_history"
            )
            || !ids.insert(id)
        {
            return Err(ServiceError::InvalidRequest(
                "semantic search candidates are invalid, duplicated, or outside the local search contract"
                    .to_string(),
            ));
        }
    }
    Ok(())
}

fn semantic_search_candidate_evidence(
    index: usize,
    candidate: &LlmSearchCandidateParams,
    source_revision: &str,
) -> EvidenceRef {
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot/semantic-search-candidate/v1");
    hasher.update(candidate.id.as_bytes());
    hasher.update(candidate.kind.as_bytes());
    hasher.update(candidate.title.as_bytes());
    hasher.update(candidate.subtitle.as_bytes());
    let kind = match candidate.kind.as_str() {
        "skill" => EvidenceKind::SkillDefinition,
        "session" => EvidenceKind::Session,
        "config_history" => EvidenceKind::Config,
        _ => EvidenceKind::ProjectContext,
    };
    EvidenceRef {
        id: format!("search-candidate:{}", hex_prefix(&hasher.finalize(), 32)),
        kind,
        source_revision: source_revision.to_string(),
        summary: format!(
            "Lexical search candidate {} ({})",
            index + 1,
            candidate.kind
        ),
        agent: None,
        target_id: Some(candidate.id.clone()),
    }
}

fn render_project_health_prompt_section(
    contract: &AiResponseContract,
    redactor: &mut PromptRedactor<'_>,
) -> String {
    let evidence = contract
        .evidence
        .iter()
        .map(|reference| {
            format!(
                "- evidence_id={} kind={:?} agent={} summary={}",
                reference.id,
                reference.kind,
                reference
                    .agent
                    .map(|agent| agent.as_str())
                    .unwrap_or("project"),
                redactor.redact(&reference.summary)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    format!(
        "Explain the current project health and prioritize attention using only these accepted deterministic evidence rows. Distinguish facts from interpretation and explicitly list uncertainty when evidence is incomplete.\n{evidence}"
    )
}

fn render_semantic_search_prompt_section(
    params: &LlmPreviewPromptParams,
    contract: &AiResponseContract,
    redactor: &mut PromptRedactor<'_>,
) -> Result<String, ServiceError> {
    validate_semantic_search_candidates(
        params.user_intent.as_deref().unwrap_or_default(),
        &params.search_candidates,
    )?;
    let rows = params
        .search_candidates
        .iter()
        .enumerate()
        .map(|(index, candidate)| {
            let evidence =
                semantic_search_candidate_evidence(index, candidate, &contract.source_revision);
            format!(
                "- evidence_id={} kind={} title={} subtitle={}",
                evidence.id,
                candidate.kind,
                redactor.redact(&candidate.title),
                redactor.redact(&candidate.subtitle)
            )
        })
        .collect::<Vec<_>>()
        .join("\n");
    Ok(format!(
        "Rerank only the candidates below for query `{}`. Return every useful candidate at most once by evidence id. Do not add, discover, or describe any candidate outside this list.\n{rows}",
        redactor.redact(params.user_intent.as_deref().unwrap_or_default())
    ))
}

fn append_known_evidence(
    target: &mut Vec<EvidenceRef>,
    available: &[EvidenceRef],
    ids: &[String],
) -> Result<(), ServiceError> {
    for id in ids {
        let reference = available
            .iter()
            .find(|reference| reference.id == *id)
            .ok_or_else(|| {
                ServiceError::InvalidRequest(format!(
                    "AI context references unknown deterministic evidence `{id}`"
                ))
            })?;
        target.push(reference.clone());
    }
    Ok(())
}

fn append_known_actions(
    target: &mut Vec<ActionDescriptor>,
    available: &[ActionDescriptor],
    ids: &[String],
) -> Result<(), ServiceError> {
    for id in ids {
        let action = available
            .iter()
            .find(|action| action.id == *id)
            .ok_or_else(|| {
                ServiceError::InvalidRequest(format!(
                    "AI context references unknown deterministic action `{id}`"
                ))
            })?;
        target.push(action.clone());
    }
    Ok(())
}

fn normalize_response_contract_members(
    project_id: &str,
    fallback_evidence: &[EvidenceRef],
    evidence: &mut Vec<EvidenceRef>,
    actions: &mut Vec<ActionDescriptor>,
) -> Result<(), ServiceError> {
    evidence.sort_by(|left, right| left.id.cmp(&right.id));
    evidence.dedup_by(|left, right| left.id == right.id);
    actions.sort_by(|left, right| left.id.cmp(&right.id));
    actions.dedup_by(|left, right| left.id == right.id);

    for action in actions.iter() {
        if action.project_id.as_deref() != Some(project_id) {
            return Err(ServiceError::InvalidRequest(format!(
                "AI action `{}` is not bound to the accepted project",
                action.id
            )));
        }
        for evidence_id in &action.evidence_refs {
            if evidence
                .iter()
                .any(|reference| reference.id == *evidence_id)
            {
                continue;
            }
            let reference = fallback_evidence
                .iter()
                .find(|reference| reference.id == *evidence_id)
                .ok_or_else(|| {
                    ServiceError::InvalidRequest(format!(
                        "AI action `{}` references unavailable evidence `{evidence_id}`",
                        action.id
                    ))
                })?;
            evidence.push(reference.clone());
        }
    }
    evidence.sort_by(|left, right| left.id.cmp(&right.id));
    evidence.dedup_by(|left, right| left.id == right.id);
    Ok(())
}

fn render_ai_response_contract(contract: &AiResponseContract) -> Result<String, ServiceError> {
    let result = match contract.result_schema {
        AiResultSchema::CopyOnlyMarkdown => serde_json::json!({
            "markdown": "<concise evidence-cited copy-only explanation>"
        }),
        AiResultSchema::TaskReadiness => serde_json::json!({
            "summary": {
                "summary": "<one concise user-facing finding>",
                "recommended_agent": null,
                "recommended_skill_name": null,
                "readiness_score": 0,
                "routing_score": 0,
                "gap_count": 0,
                "blocker_count": 0
            },
            "agent_candidates": [],
            "skill_candidates": [],
            "readiness_signals": [],
            "gap_rows": [],
            "blocker_rows": []
        }),
        AiResultSchema::SessionDigest => serde_json::json!({
            "summary": "<concise digest of the accepted session evidence>",
            "intent": "<inferred intent or null>",
            "suggested_next_prompt": "<copy-only prompt suggestion, never a command>",
            "evidence_notes": [],
            "uncertainties": []
        }),
        AiResultSchema::SkillChangeReview => serde_json::json!({
            "summary": "<concise review of observed skill evidence>",
            "changes": [],
            "risks": [],
            "recommendations": []
        }),
        AiResultSchema::SemanticRerank => serde_json::json!({
            "summary": "<concise explanation of the rerank>",
            "ranked_evidence_ids": ["<candidate id from allowed_evidence>"],
            "rationales": [{
                "evidence_id": "<ranked candidate id>",
                "rationale": "<query-specific rationale grounded only in that candidate>"
            }],
            "unsupported_claims": []
        }),
    };
    let allowed_evidence = contract
        .evidence
        .iter()
        .map(|reference| {
            serde_json::json!({
                "id": reference.id,
                "kind": reference.kind,
                "summary": reference.summary,
                "agent": reference.agent,
            })
        })
        .collect::<Vec<_>>();
    let allowed_actions = contract
        .actions
        .iter()
        .map(|action| {
            serde_json::json!({
                "id": action.id,
                "target_kind": action.target.kind,
                "agent": action.target.agent,
                "scope": action.target.scope,
                "confirmation_required": action.confirmation_required,
                "network": action.network,
                "evidence_refs": action.evidence_refs,
            })
        })
        .collect::<Vec<_>>();
    let envelope = serde_json::json!({
        "schema_version": AI_RESPONSE_ENVELOPE_SCHEMA_VERSION,
        "request_kind": contract.request_kind,
        "project_id": contract.project_id,
        "source_revision": contract.source_revision,
        "result_schema": contract.result_schema,
        "evidence_refs": ["<one or more ids from allowed_evidence>"],
        "action_refs": ["<zero or more ids from allowed_actions>"],
        "result": result,
        "safety_flags": contract.required_safety_flags,
    });
    let specification = serde_json::json!({
        "response_envelope": envelope,
        "allowed_evidence": allowed_evidence,
        "allowed_actions": allowed_actions,
        "reference_rules": [
            "Every evidence_refs entry must resolve to allowed_evidence.",
            "Every action_refs entry must resolve to allowed_actions.",
            "Use action_refs only to recommend an existing deterministic action; never create or authorize an action.",
            "Keep project_id, source_revision, request_kind, result_schema, schema_version, and safety_flags exactly unchanged."
        ]
    });
    serde_json::to_string_pretty(&specification)
        .map(|json| format!("Evidence-bound response contract:\n{json}"))
        .map_err(ServiceError::from)
}

fn render_session_digest_prompt_section(
    session: &SessionContinuationRecord,
    redactor: &mut PromptRedactor<'_>,
) -> String {
    let payload = serde_json::json!({
        "id": redactor.redact(&session.id),
        "agent": session.agent,
        "title": redactor.redact(&session.title),
        "intent": session.intent.as_deref().map(|intent| redactor.redact(intent)),
        "started_at": session.started_at,
        "ended_at": session.ended_at,
        "modified_at": session.modified_at,
        "source_kind": redactor.redact(&session.source_kind),
        "source_revision": session.source_revision,
        "snapshot_revision": session.snapshot_revision,
        "coverage": session.coverage,
        "resume_state": session.resume.state,
        "evidence_refs": session.evidence.iter().map(|reference| reference.id.as_str()).collect::<Vec<_>>(),
    });
    serde_json::to_string_pretty(&payload)
        .map(|json| format!("Selected session continuation evidence:\n{json}"))
        .unwrap_or_else(|_| "Selected session continuation evidence is unavailable.".to_string())
}

fn redacted_model_task_record(
    record: &ModelTaskMatchRecord,
    redaction_roots: &[(String, &'static str)],
) -> ModelTaskMatchRecord {
    ModelTaskMatchRecord {
        id: observability_redact(&record.id, redaction_roots, 160),
        title: observability_redact(&record.title, redaction_roots, 180),
        task: String::new(),
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
        task: None,
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
        task: None,
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
        gap_notes: vec![
            "Prompt-run metadata does not retain task text, so only request kind and model fit can be displayed."
                .to_string(),
        ],
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
