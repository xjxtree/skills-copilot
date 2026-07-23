use super::*;
use fs4::FileExt;
use skills_copilot_commands::{
    action_descriptor, action_preview_binding, action_source_revision, ensure_action_confirmed,
    opaque_sensitive_action_input_binding, ActionPrecondition, ActionPreconditionKind,
    ActionPreviewBinding, ActionReadbackObservation, ActionReadbackRecord,
};
use skills_copilot_core::{
    ActionImpact, ActionIntent, ActionKind, ActionNetworkPosture, ActionReadbackDomain,
    ActionTargetKind, ActionTargetRef,
};
use std::io::{self, Read, Write};

#[cfg(test)]
use std::sync::{LazyLock, Mutex};

const PROVIDER_ACTION_STATE_DOMAIN: &str = "agent-copilot/provider-action-state/v1";
const PROVIDER_ACTION_STATE_VERSION: u32 = 1;
const PROVIDER_ACTION_STATE_MAX_BYTES: usize = 4 * 1024;
const PROVIDER_ACTION_STATE_MAX_DIRECTORY_ENTRIES: usize = 256;
const PROVIDER_ACTION_STATE_LEGACY_REPLACEMENT_PREFIX: &str = ".provider-action-state.json.";
const PROVIDER_ACTION_STATE_LEGACY_REPLACEMENT_SUFFIX: &str = ".tmp";

#[cfg(test)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum TestProviderActionStateFault {
    OutcomeDirectorySync,
}

#[cfg(test)]
static TEST_PROVIDER_ACTION_STATE_FAULTS: LazyLock<
    Mutex<Vec<(PathBuf, TestProviderActionStateFault)>>,
> = LazyLock::new(|| Mutex::new(Vec::new()));

#[cfg(test)]
pub(crate) fn install_test_provider_action_state_fault(
    app_data_dir: &Path,
    fault: TestProviderActionStateFault,
) {
    TEST_PROVIDER_ACTION_STATE_FAULTS
        .lock()
        .expect("lock provider action-state faults")
        .push((app_data_dir.to_path_buf(), fault));
}

#[cfg(test)]
fn take_test_provider_action_state_fault(
    app_data_dir: &Path,
    fault: TestProviderActionStateFault,
) -> bool {
    let mut faults = TEST_PROVIDER_ACTION_STATE_FAULTS
        .lock()
        .expect("lock provider action-state faults");
    let Some(index) = faults
        .iter()
        .position(|(path, candidate)| path == app_data_dir && *candidate == fault)
    else {
        return false;
    };
    faults.swap_remove(index);
    true
}

#[derive(Debug, Serialize)]
pub struct ProviderActionPreviewResult {
    #[serde(flatten)]
    pub binding: ActionPreviewBinding,
    pub operation: &'static str,
    pub profile_id: String,
    pub provider_type: String,
    pub destination_host: String,
    pub model: String,
    pub expected_revision: String,
    pub credential_change: bool,
    pub raw_secret_returned: bool,
}

#[derive(Debug, Serialize)]
pub struct SaveProviderProfileApplyResult {
    #[serde(flatten)]
    pub result: crate::provider::SaveProviderProfileResult,
    pub outcome: ProviderActionExecutionOutcome,
    pub readback: Option<ActionReadbackRecord>,
}

#[derive(Debug, Serialize)]
pub struct DeleteProviderProfileApplyResult {
    #[serde(flatten)]
    pub result: crate::provider::DeleteProviderProfileResult,
    pub outcome: ProviderActionExecutionOutcome,
    pub readback: Option<ActionReadbackRecord>,
}

#[derive(Debug, Serialize)]
pub struct TestProviderConnectionApplyResult {
    #[serde(flatten)]
    pub result: crate::provider::TestProviderConnectionResult,
    pub outcome: ProviderActionExecutionOutcome,
    pub readback: Option<ActionReadbackRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub struct ProviderActionExecutionOutcome {
    pub state: &'static str,
    pub effect: &'static str,
    pub remote_effect: &'static str,
    pub local_effect: String,
    pub credential_effect: String,
    pub recovery: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderActionState {
    NotStarted,
    Verified,
    Partial,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub(crate) enum ProviderActionStatePhase {
    Reservation,
    Outcome,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
struct ProviderActionStateRecord {
    version: u32,
    generation: u64,
    token_digest: String,
    action_id: String,
    source_revision: String,
    phase: ProviderActionStatePhase,
    state: ProviderActionState,
    updated_at: i64,
}

impl ServiceHost {
    pub(crate) fn preview_save_provider_profile(
        &self,
        params: &SaveProviderProfileParams,
    ) -> Result<ProviderActionPreviewResult, ServiceError> {
        let normalized = crate::provider::normalize_save_provider_profile_params(params)?;
        let store_revision = crate::provider::provider_profiles_revision(&self.app_data_dir)?;
        let input_revision = crate::provider::normalized_provider_input_revision(&normalized)?;
        let action_state_revision = provider_action_state_revision(&self.app_data_dir)?;
        let credential_input_binding = match params
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|secret| !secret.is_empty())
        {
            Some(secret) => opaque_sensitive_action_input_binding("provider-api-key", secret)?,
            None => "preserve-existing-credential".to_string(),
        };
        let source_revision = action_source_revision(
            "llm.saveProviderProfile",
            &[
                ("profile_id", &normalized.id),
                ("profile_store", &store_revision),
                ("profile_input", &input_revision),
                ("credential_input", &credential_input_binding),
                ("action_state", &action_state_revision),
            ],
        )?;
        let mut impacts = vec![ActionImpact::AppLocalData];
        let mut readback = vec![ActionReadbackDomain::ProviderProfiles];
        if normalized.replaces_credential {
            impacts.push(ActionImpact::CredentialStore);
            readback.push(ActionReadbackDomain::ProviderCredentials);
        }
        let action = action_descriptor(
            ActionKind::ProviderProfile,
            ActionIntent::SaveProviderProfile,
            provider_target(&normalized.id),
            None,
            impacts,
            "llm.previewSaveProviderProfile",
            Some("llm.saveProviderProfile"),
            source_revision,
            true,
            ActionNetworkPosture::None,
            readback,
            vec![
                format!("provider-profile:{}", normalized.id),
                format!("provider-input:{input_revision}"),
            ],
        )?;
        let binding = action_preview_binding(
            action,
            provider_preconditions(&store_revision, &action_state_revision),
        )?;
        Ok(ProviderActionPreviewResult {
            binding,
            operation: "save",
            profile_id: normalized.id,
            provider_type: normalized.provider_type.as_str().to_string(),
            destination_host: crate::provider::destination_host(&normalized.base_url),
            model: normalized.model,
            expected_revision: store_revision,
            credential_change: normalized.replaces_credential,
            raw_secret_returned: false,
        })
    }

    pub(crate) fn save_provider_profile_with_confirmation(
        &self,
        params: SaveProviderProfileParams,
    ) -> Result<SaveProviderProfileApplyResult, ServiceError> {
        let confirmation = params.action_confirmation.clone();
        let preliminary = self.preview_save_provider_profile(&params)?;
        ensure_action_confirmed(&preliminary.binding, confirmation.as_ref())?;

        let _lock = lock_provider_actions(&self.app_data_dir)?;
        let current = self.preview_save_provider_profile(&params)?;
        ensure_action_confirmed(&current.binding, confirmation.as_ref())?;
        reserve_provider_action(
            &self.app_data_dir,
            &current.binding,
            confirmation.as_ref().expect("confirmed above"),
        )?;

        let normalized = crate::provider::normalize_save_provider_profile_params(&params)?;
        let expected_secret = params
            .api_key
            .as_deref()
            .map(str::trim)
            .filter(|secret| !secret.is_empty())
            .map(ToOwned::to_owned);
        let result = match crate::provider::save_provider_profile(&self.app_data_dir, params) {
            Ok(result) => result,
            Err(ProviderError::CredentialMutationPartial(message)) => {
                let _ = finalize_provider_action(
                    &self.app_data_dir,
                    &current.binding,
                    confirmation.as_ref().expect("confirmed above"),
                    ProviderActionState::Partial,
                );
                return Err(ServiceError::AppliedUnverified(format!(
                    "{message}; inspect Keychain state before creating a new preview"
                )));
            }
            Err(error) => {
                let _ = finalize_provider_action(
                    &self.app_data_dir,
                    &current.binding,
                    confirmation.as_ref().expect("confirmed above"),
                    ProviderActionState::NotStarted,
                );
                return Err(ServiceError::ActionNotStarted(format!(
                    "provider save did not start: {error}; create a new preview before retrying"
                )));
            }
        };

        match result.operation_state {
            crate::provider::ProviderMutationState::NotStarted => {
                let _ = finalize_provider_action(
                    &self.app_data_dir,
                    &current.binding,
                    confirmation.as_ref().expect("confirmed above"),
                    ProviderActionState::NotStarted,
                );
                Ok(SaveProviderProfileApplyResult {
                    outcome: action_outcome(
                        "not_started",
                        "not_applied",
                        "not_applicable",
                        "provider_profile_not_saved",
                        &result.credential_effect,
                        result
                            .error_message
                            .clone()
                            .or_else(|| Some("Create a new preview before retrying.".to_string())),
                    ),
                    result,
                    readback: None,
                })
            }
            crate::provider::ProviderMutationState::Partial => {
                let _ = finalize_provider_action(
                    &self.app_data_dir,
                    &current.binding,
                    confirmation.as_ref().expect("confirmed above"),
                    ProviderActionState::Partial,
                );
                Ok(SaveProviderProfileApplyResult {
                    outcome: action_outcome(
                        "partial",
                        "applied_unverified",
                        "not_applicable",
                        if result.profile_persisted {
                            "provider_profile_saved"
                        } else {
                            "provider_profile_not_saved"
                        },
                        &result.credential_effect,
                        result.error_message.clone().or_else(|| {
                            Some(
                                "Reload provider settings and inspect credential state before creating a new preview."
                                    .to_string(),
                            )
                        }),
                    ),
                    result,
                    readback: None,
                })
            }
            crate::provider::ProviderMutationState::Applied => {
                let readback = (|| -> Result<ActionReadbackRecord, ServiceError> {
                    ensure_saved_profile_matches(&result.profile, &normalized)?;
                    let persisted = crate::provider::list_provider_profiles(&self.app_data_dir)?
                        .profiles
                        .into_iter()
                        .find(|profile| profile.id == normalized.id)
                        .ok_or_else(|| {
                            ServiceError::InvalidRequest(
                                "provider save read-back did not find the confirmed profile"
                                    .to_string(),
                            )
                        })?;
                    ensure_saved_profile_matches(&persisted, &normalized)?;
                    let revision = crate::provider::provider_profiles_revision(&self.app_data_dir)?;
                    let mut observations = vec![ActionReadbackObservation {
                        domain: ActionReadbackDomain::ProviderProfiles,
                        target_id: normalized.id.clone(),
                        revision,
                    }];
                    if let Some(secret) = expected_secret.as_deref() {
                        observations.push(ActionReadbackObservation {
                            domain: ActionReadbackDomain::ProviderCredentials,
                            target_id: normalized.id.clone(),
                            revision: crate::provider::verify_provider_credential_matches(
                                &normalized.id,
                                secret,
                            )?,
                        });
                    }
                    ActionReadbackRecord::verified(&current.binding.action, observations)
                        .map_err(Into::into)
                })();
                finish_save_readback(
                    &self.app_data_dir,
                    &current.binding,
                    confirmation.as_ref().expect("confirmed above"),
                    result,
                    readback,
                )
            }
        }
    }

    pub(crate) fn preview_delete_provider_profile(
        &self,
        params: &DeleteProviderProfileParams,
    ) -> Result<ProviderActionPreviewResult, ServiceError> {
        let profile = required_provider_profile(&self.app_data_dir, &params.profile_id)?;
        let store_revision = crate::provider::provider_profiles_revision(&self.app_data_dir)?;
        let profile_revision = crate::provider::provider_profile_nonsecret_revision(&profile)?;
        let action_state_revision = provider_action_state_revision(&self.app_data_dir)?;
        let delete_credential = params.delete_credential.to_string();
        let source_revision = action_source_revision(
            "llm.deleteProviderProfile",
            &[
                ("profile_id", &profile.id),
                ("profile_store", &store_revision),
                ("profile", &profile_revision),
                ("delete_credential", &delete_credential),
                ("action_state", &action_state_revision),
            ],
        )?;
        let mut impacts = vec![ActionImpact::AppLocalData];
        let mut readback = vec![ActionReadbackDomain::ProviderProfiles];
        if params.delete_credential {
            impacts.push(ActionImpact::CredentialStore);
            readback.push(ActionReadbackDomain::ProviderCredentials);
        }
        let action = action_descriptor(
            ActionKind::ProviderProfile,
            ActionIntent::DeleteProviderProfile,
            provider_target(&profile.id),
            None,
            impacts,
            "llm.previewDeleteProviderProfile",
            Some("llm.deleteProviderProfile"),
            source_revision,
            true,
            ActionNetworkPosture::None,
            readback,
            vec![
                format!("provider-profile:{}", profile.id),
                format!("provider-state:{profile_revision}"),
            ],
        )?;
        let binding = action_preview_binding(
            action,
            provider_preconditions(&store_revision, &action_state_revision),
        )?;
        Ok(ProviderActionPreviewResult {
            binding,
            operation: "delete",
            profile_id: profile.id,
            provider_type: profile.provider_type.as_str().to_string(),
            destination_host: crate::provider::destination_host(&profile.base_url),
            model: profile.model,
            expected_revision: store_revision,
            credential_change: params.delete_credential,
            raw_secret_returned: false,
        })
    }

    pub(crate) fn delete_provider_profile_with_confirmation(
        &self,
        params: DeleteProviderProfileParams,
    ) -> Result<DeleteProviderProfileApplyResult, ServiceError> {
        let confirmation = params.action_confirmation.clone();
        let preliminary = self.preview_delete_provider_profile(&params)?;
        ensure_action_confirmed(&preliminary.binding, confirmation.as_ref())?;

        let _lock = lock_provider_actions(&self.app_data_dir)?;
        let current = self.preview_delete_provider_profile(&params)?;
        ensure_action_confirmed(&current.binding, confirmation.as_ref())?;
        reserve_provider_action(
            &self.app_data_dir,
            &current.binding,
            confirmation.as_ref().expect("confirmed above"),
        )?;
        let profile_id = current.profile_id.clone();
        let delete_credential = params.delete_credential;
        let result = match crate::provider::delete_provider_profile(&self.app_data_dir, params) {
            Ok(result) => result,
            Err(error) => {
                let _ = finalize_provider_action(
                    &self.app_data_dir,
                    &current.binding,
                    confirmation.as_ref().expect("confirmed above"),
                    ProviderActionState::NotStarted,
                );
                return Err(ServiceError::ActionNotStarted(format!(
                    "provider delete did not start: {error}; create a new preview before retrying"
                )));
            }
        };
        match result.operation_state {
            crate::provider::ProviderMutationState::NotStarted => {
                let _ = finalize_provider_action(
                    &self.app_data_dir,
                    &current.binding,
                    confirmation.as_ref().expect("confirmed above"),
                    ProviderActionState::NotStarted,
                );
                Ok(DeleteProviderProfileApplyResult {
                    outcome: action_outcome(
                        "not_started",
                        "not_applied",
                        "not_applicable",
                        "provider_profile_not_deleted",
                        &result.credential_effect,
                        result
                            .error_message
                            .clone()
                            .or_else(|| Some("Create a new preview before retrying.".to_string())),
                    ),
                    result,
                    readback: None,
                })
            }
            crate::provider::ProviderMutationState::Partial => {
                let _ = finalize_provider_action(
                    &self.app_data_dir,
                    &current.binding,
                    confirmation.as_ref().expect("confirmed above"),
                    ProviderActionState::Partial,
                );
                Ok(DeleteProviderProfileApplyResult {
                    outcome: action_outcome(
                        "partial",
                        "applied_unverified",
                        "not_applicable",
                        if result.profile_deleted {
                            "provider_profile_deleted"
                        } else {
                            "provider_profile_delete_unknown"
                        },
                        &result.credential_effect,
                        result.error_message.clone().or_else(|| {
                            Some(
                                "Reload provider settings and inspect Keychain state before creating a new preview."
                                    .to_string(),
                            )
                        }),
                    ),
                    result,
                    readback: None,
                })
            }
            crate::provider::ProviderMutationState::Applied => {
                let readback = (|| -> Result<ActionReadbackRecord, ServiceError> {
                    if crate::provider::list_provider_profiles(&self.app_data_dir)?
                        .profiles
                        .iter()
                        .any(|profile| profile.id == profile_id)
                    {
                        return Err(ServiceError::InvalidRequest(
                            "provider delete read-back still found the confirmed profile"
                                .to_string(),
                        ));
                    }
                    let revision = crate::provider::provider_profiles_revision(&self.app_data_dir)?;
                    let mut observations = vec![ActionReadbackObservation {
                        domain: ActionReadbackDomain::ProviderProfiles,
                        target_id: profile_id.clone(),
                        revision,
                    }];
                    if delete_credential {
                        observations.push(ActionReadbackObservation {
                            domain: ActionReadbackDomain::ProviderCredentials,
                            target_id: profile_id.clone(),
                            revision: crate::provider::verify_provider_credential_absent(
                                &profile_id,
                            )?,
                        });
                    }
                    ActionReadbackRecord::verified(&current.binding.action, observations)
                        .map_err(Into::into)
                })();
                finish_delete_readback(
                    &self.app_data_dir,
                    &current.binding,
                    confirmation.as_ref().expect("confirmed above"),
                    result,
                    readback,
                )
            }
        }
    }

    pub(crate) fn preview_provider_connection_test(
        &self,
        params: &TestProviderConnectionParams,
    ) -> Result<ProviderActionPreviewResult, ServiceError> {
        let profile = required_provider_profile(&self.app_data_dir, &params.profile_id)?;
        let store_revision = crate::provider::provider_profiles_revision(&self.app_data_dir)?;
        let profile_revision = crate::provider::provider_profile_nonsecret_revision(&profile)?;
        let action_state_revision = provider_action_state_revision(&self.app_data_dir)?;
        let timeout_ms = params.timeout_ms.unwrap_or(4_000).clamp(250, 15_000);
        let timeout = timeout_ms.to_string();
        let source_revision = action_source_revision(
            "llm.testProviderConnection",
            &[
                ("profile_id", &profile.id),
                ("profile_store", &store_revision),
                ("profile", &profile_revision),
                ("timeout_ms", &timeout),
                ("action_state", &action_state_revision),
            ],
        )?;
        let action = action_descriptor(
            ActionKind::ProviderConnectionTest,
            ActionIntent::TestProviderConnection,
            provider_target(&profile.id),
            None,
            vec![ActionImpact::AppLocalData],
            "llm.previewProviderConnectionTest",
            Some("llm.testProviderConnection"),
            source_revision,
            true,
            ActionNetworkPosture::Required,
            vec![
                ActionReadbackDomain::ProviderProfiles,
                ActionReadbackDomain::ProviderActivity,
            ],
            vec![
                format!("provider-profile:{}", profile.id),
                format!("provider-state:{profile_revision}"),
            ],
        )?;
        let binding = action_preview_binding(
            action,
            provider_preconditions(&store_revision, &action_state_revision),
        )?;
        Ok(ProviderActionPreviewResult {
            binding,
            operation: "test",
            profile_id: profile.id,
            provider_type: profile.provider_type.as_str().to_string(),
            destination_host: crate::provider::destination_host(&profile.base_url),
            model: profile.model,
            expected_revision: store_revision,
            credential_change: false,
            raw_secret_returned: false,
        })
    }

    pub(crate) fn test_provider_connection_with_confirmation(
        &self,
        mut params: TestProviderConnectionParams,
    ) -> Result<TestProviderConnectionApplyResult, ServiceError> {
        let confirmation = params.action_confirmation.clone();
        let preliminary = self.preview_provider_connection_test(&params)?;
        ensure_action_confirmed(&preliminary.binding, confirmation.as_ref())?;

        let _lock = lock_provider_actions(&self.app_data_dir)?;
        let current = self.preview_provider_connection_test(&params)?;
        ensure_action_confirmed(&current.binding, confirmation.as_ref())?;
        reserve_provider_action(
            &self.app_data_dir,
            &current.binding,
            confirmation.as_ref().expect("confirmed above"),
        )?;
        params.confirmation_id = current.binding.action.id.clone();
        let mut result = match crate::provider::test_provider_connection(&self.app_data_dir, params)
        {
            Ok(result) => result,
            Err(error) => {
                let _ = finalize_provider_action(
                    &self.app_data_dir,
                    &current.binding,
                    confirmation.as_ref().expect("confirmed above"),
                    ProviderActionState::NotStarted,
                );
                return Err(ServiceError::ActionNotStarted(format!(
                    "provider test did not start: {error}; create a new preview before retrying"
                )));
            }
        };
        let target_matches = result.profile_id == current.profile_id
            && result.model == current.model
            && result.destination_host == current.destination_host;
        let remote_result_verified = !result.provider_request_sent
            || result.status.eq_ignore_ascii_case("succeeded")
            || result
                .error_code
                .as_deref()
                .is_some_and(|code| code.starts_with("http_"));
        let readback =
            if target_matches && result.local_metadata_persisted && remote_result_verified {
                (|| -> Result<ActionReadbackRecord, ServiceError> {
                    let profile_revision =
                        crate::provider::provider_profiles_revision(&self.app_data_dir)?;
                    let activity_revision =
                        crate::provider::provider_call_metadata_revision(&self.app_data_dir)?;
                    ActionReadbackRecord::verified(
                        &current.binding.action,
                        vec![
                            ActionReadbackObservation {
                                domain: ActionReadbackDomain::ProviderProfiles,
                                target_id: current.profile_id.clone(),
                                revision: profile_revision,
                            },
                            ActionReadbackObservation {
                                domain: ActionReadbackDomain::ProviderActivity,
                                target_id: current.profile_id.clone(),
                                revision: activity_revision,
                            },
                        ],
                    )
                    .map_err(Into::into)
                })()
                .ok()
            } else {
                None
            };
        if let Some(readback) = readback {
            if finalize_provider_action(
                &self.app_data_dir,
                &current.binding,
                confirmation.as_ref().expect("confirmed above"),
                ProviderActionState::Verified,
            )
            .is_ok()
            {
                return Ok(TestProviderConnectionApplyResult {
                    outcome: action_outcome(
                        "verified",
                        "verified",
                        if result.provider_request_sent {
                            "completed"
                        } else {
                            "not_sent"
                        },
                        "provider_activity_verified",
                        "read_only",
                        None,
                    ),
                    result,
                    readback: Some(readback),
                });
            }
        }

        result.status = "partial".to_string();
        result.error_code = Some(if result.provider_request_sent {
            "remote_outcome_unverified".to_string()
        } else {
            "local_outcome_unverified".to_string()
        });
        result.error_message = Some(if !target_matches {
            "Provider request result did not match the confirmed target; outcome requires review."
                .to_string()
        } else if !remote_result_verified {
            "The provider request may have left the process, but its remote result could not be verified."
                .to_string()
        } else if !result.local_metadata_persisted {
            "Provider request returned, but local metadata persistence failed.".to_string()
        } else {
            "Provider request returned, but semantic read-back could not be verified.".to_string()
        });
        let _ = finalize_provider_action(
            &self.app_data_dir,
            &current.binding,
            confirmation.as_ref().expect("confirmed above"),
            ProviderActionState::Partial,
        );
        Ok(TestProviderConnectionApplyResult {
            outcome: action_outcome(
                "partial",
                if result.provider_request_sent {
                    "remote_unknown"
                } else {
                    "applied_unverified"
                },
                if result.provider_request_sent {
                    "remote_unknown"
                } else {
                    "not_sent"
                },
                if result.local_metadata_persisted {
                    "provider_activity_unverified"
                } else {
                    "provider_activity_not_persisted"
                },
                "read_only",
                Some(
                    "Review provider activity before creating a new preview; this action reference cannot be replayed."
                        .to_string(),
                ),
            ),
            result,
            readback: None,
        })
    }
}

fn action_outcome(
    state: &'static str,
    effect: &'static str,
    remote_effect: &'static str,
    local_effect: impl Into<String>,
    credential_effect: impl Into<String>,
    recovery: Option<String>,
) -> ProviderActionExecutionOutcome {
    ProviderActionExecutionOutcome {
        state,
        effect,
        remote_effect,
        local_effect: local_effect.into(),
        credential_effect: credential_effect.into(),
        recovery,
    }
}

fn finish_save_readback(
    app_data_dir: &Path,
    binding: &ActionPreviewBinding,
    confirmation: &ActionConfirmation,
    mut result: crate::provider::SaveProviderProfileResult,
    readback: Result<ActionReadbackRecord, ServiceError>,
) -> Result<SaveProviderProfileApplyResult, ServiceError> {
    if let Ok(readback) = readback {
        if finalize_provider_action(
            app_data_dir,
            binding,
            confirmation,
            ProviderActionState::Verified,
        )
        .is_ok()
        {
            return Ok(SaveProviderProfileApplyResult {
                outcome: action_outcome(
                    "verified",
                    "verified",
                    "not_applicable",
                    "provider_profile_verified",
                    &result.credential_effect,
                    None,
                ),
                result,
                readback: Some(readback),
            });
        }
    }
    let _ = finalize_provider_action(
        app_data_dir,
        binding,
        confirmation,
        ProviderActionState::Partial,
    );
    result.error_code = Some("provider_save_readback_unverified".to_string());
    result.error_message = Some(
        "Provider save may have completed, but semantic read-back could not be verified."
            .to_string(),
    );
    Ok(SaveProviderProfileApplyResult {
        outcome: action_outcome(
            "partial",
            "applied_unverified",
            "not_applicable",
            if result.profile_persisted {
                "provider_profile_saved_unverified"
            } else {
                "provider_profile_state_unknown"
            },
            &result.credential_effect,
            Some(
                "Reload provider settings and inspect credential state before creating a new preview."
                    .to_string(),
            ),
        ),
        result,
        readback: None,
    })
}

fn finish_delete_readback(
    app_data_dir: &Path,
    binding: &ActionPreviewBinding,
    confirmation: &ActionConfirmation,
    mut result: crate::provider::DeleteProviderProfileResult,
    readback: Result<ActionReadbackRecord, ServiceError>,
) -> Result<DeleteProviderProfileApplyResult, ServiceError> {
    if let Ok(readback) = readback {
        if finalize_provider_action(
            app_data_dir,
            binding,
            confirmation,
            ProviderActionState::Verified,
        )
        .is_ok()
        {
            return Ok(DeleteProviderProfileApplyResult {
                outcome: action_outcome(
                    "verified",
                    "verified",
                    "not_applicable",
                    "provider_profile_absence_verified",
                    &result.credential_effect,
                    None,
                ),
                result,
                readback: Some(readback),
            });
        }
    }
    let _ = finalize_provider_action(
        app_data_dir,
        binding,
        confirmation,
        ProviderActionState::Partial,
    );
    result.error_code = Some("provider_delete_readback_unverified".to_string());
    result.error_message = Some(
        "Provider delete may have completed, but semantic read-back could not be verified."
            .to_string(),
    );
    Ok(DeleteProviderProfileApplyResult {
        outcome: action_outcome(
            "partial",
            "applied_unverified",
            "not_applicable",
            if result.profile_deleted {
                "provider_profile_deleted_unverified"
            } else {
                "provider_profile_state_unknown"
            },
            &result.credential_effect,
            Some(
                "Reload provider settings and inspect Keychain state before creating a new preview."
                    .to_string(),
            ),
        ),
        result,
        readback: None,
    })
}

fn provider_target(profile_id: &str) -> ActionTargetRef {
    ActionTargetRef {
        kind: ActionTargetKind::ProviderProfile,
        id: profile_id.to_string(),
        agent: None,
        scope: None,
    }
}

fn provider_preconditions(
    store_revision: &str,
    action_state_revision: &str,
) -> Vec<ActionPrecondition> {
    vec![
        ActionPrecondition {
            kind: ActionPreconditionKind::ProviderProfile,
            target_id: "provider-profiles".to_string(),
            expected_revision: store_revision.to_string(),
        },
        ActionPrecondition {
            kind: ActionPreconditionKind::PromptContext,
            target_id: "provider-action-state".to_string(),
            expected_revision: action_state_revision.to_string(),
        },
    ]
}

fn required_provider_profile(
    app_data_dir: &Path,
    profile_id: &str,
) -> Result<ProviderProfileRecord, ServiceError> {
    crate::provider::list_provider_profiles(app_data_dir)?
        .profiles
        .into_iter()
        .find(|profile| profile.id == profile_id)
        .ok_or_else(|| ProviderError::ProfileNotFound(profile_id.to_string()).into())
}

fn ensure_saved_profile_matches(
    profile: &ProviderProfileRecord,
    input: &crate::provider::NormalizedProviderProfileInput,
) -> Result<(), ServiceError> {
    let matches = profile.id == input.id
        && profile.display_name == input.display_name
        && profile.provider_type == input.provider_type
        && profile.base_url == input.base_url
        && profile.model == input.model
        && profile.enabled == input.enabled
        && profile.api_version == input.api_version
        && profile.organization == input.organization
        && profile.single_request_token_limit == input.single_request_token_limit
        && (profile.monthly_budget_usd - input.monthly_budget_usd).abs() < f64::EPSILON
        && (!input.replaces_credential || profile.credential_status.secret_available);
    if matches {
        Ok(())
    } else {
        Err(ServiceError::InvalidRequest(
            "provider save semantic read-back did not match the confirmed preview".to_string(),
        ))
    }
}

fn provider_action_state_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("llm").join("provider-action-state.json")
}

fn provider_action_state_replacement_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir
        .join("llm")
        .join(".provider-action-state.replacement")
}

fn provider_action_state_revision_for_bytes(bytes: Option<&[u8]>) -> String {
    let mut hasher = Sha256::new();
    hasher.update(PROVIDER_ACTION_STATE_DOMAIN.as_bytes());
    match bytes {
        Some(bytes) => {
            hasher.update(b"present");
            hasher.update((bytes.len() as u64).to_be_bytes());
            hasher.update(bytes);
        }
        None => hasher.update(b"missing"),
    }
    format!("sha256:{:x}", hasher.finalize())
}

pub(crate) fn provider_action_state_revision(app_data_dir: &Path) -> Result<String, ServiceError> {
    let state = read_provider_action_state(app_data_dir)?;
    Ok(provider_action_state_revision_for_bytes(
        state.as_ref().map(|(bytes, _)| bytes.as_slice()),
    ))
}

pub(crate) fn reserve_provider_action(
    app_data_dir: &Path,
    binding: &ActionPreviewBinding,
    confirmation: &ActionConfirmation,
) -> Result<(), ServiceError> {
    let token_digest = action_token_digest(&confirmation.preview_token);
    let current = read_provider_action_state(app_data_dir)?;
    let current_revision = provider_action_state_revision_for_bytes(
        current.as_ref().map(|(bytes, _)| bytes.as_slice()),
    );
    let expected_revision = binding
        .preconditions
        .iter()
        .find(|precondition| precondition.target_id == "provider-action-state")
        .map(|precondition| precondition.expected_revision.as_str())
        .ok_or_else(|| {
            ServiceError::InvalidRequest(
                "provider action preview is missing replay-state precondition".to_string(),
            )
        })?;
    if current_revision != expected_revision
        || current
            .as_ref()
            .is_some_and(|(_, record)| record.token_digest == token_digest)
    {
        return Err(CommandError::StaleActionReference.into());
    }
    let generation = match current.as_ref() {
        Some((_, record)) => record.generation.checked_add(1).ok_or_else(|| {
            ServiceError::InvalidRequest(
                "provider action replay-state generation is exhausted".to_string(),
            )
        })?,
        None => 1,
    };
    write_provider_action_state(
        app_data_dir,
        &ProviderActionStateRecord {
            version: PROVIDER_ACTION_STATE_VERSION,
            generation,
            token_digest,
            action_id: binding.action.id.clone(),
            source_revision: binding.action.source_revision.clone(),
            phase: ProviderActionStatePhase::Reservation,
            state: ProviderActionState::NotStarted,
            updated_at: unix_timestamp_millis(),
        },
    )
}

pub(crate) fn finalize_provider_action(
    app_data_dir: &Path,
    binding: &ActionPreviewBinding,
    confirmation: &ActionConfirmation,
    state: ProviderActionState,
) -> Result<(), ServiceError> {
    let token_digest = action_token_digest(&confirmation.preview_token);
    let (_, current) = read_provider_action_state(app_data_dir)?.ok_or_else(|| {
        ServiceError::InvalidRequest(
            "provider action outcome has no matching reservation".to_string(),
        )
    })?;
    if current.token_digest != token_digest
        || current.action_id != binding.action.id
        || current.source_revision != binding.action.source_revision
        || current.phase != ProviderActionStatePhase::Reservation
    {
        return Err(CommandError::StaleActionReference.into());
    }
    write_provider_action_state(
        app_data_dir,
        &ProviderActionStateRecord {
            version: PROVIDER_ACTION_STATE_VERSION,
            generation: current.generation,
            token_digest,
            action_id: binding.action.id.clone(),
            source_revision: binding.action.source_revision.clone(),
            phase: ProviderActionStatePhase::Outcome,
            state,
            updated_at: unix_timestamp_millis(),
        },
    )
}

fn write_provider_action_state(
    app_data_dir: &Path,
    record: &ProviderActionStateRecord,
) -> Result<(), ServiceError> {
    validate_provider_action_state(record)?;
    let bytes = serde_json::to_vec(record)?;
    if bytes.len() > PROVIDER_ACTION_STATE_MAX_BYTES {
        return Err(invalid_provider_action_state());
    }
    write_provider_action_state_atomic(app_data_dir, record, &bytes)
}

fn write_provider_action_state_atomic(
    app_data_dir: &Path,
    record: &ProviderActionStateRecord,
    bytes: &[u8],
) -> Result<(), ServiceError> {
    let path = provider_action_state_path(app_data_dir);
    let replacement = provider_action_state_replacement_path(app_data_dir);
    let parent = path.parent().ok_or_else(|| {
        ServiceError::InvalidRequest("provider action state has no parent".to_string())
    })?;
    let mut renamed = false;
    let result = (|| -> io::Result<()> {
        create_private_dir_all(parent)?;
        ensure_provider_action_state_destination_safe(&path)?;
        remove_legacy_provider_action_state_replacements(parent)?;
        remove_provider_action_state_replacement(&replacement)?;

        let mut options = fs::OpenOptions::new();
        options.write(true).create_new(true);
        #[cfg(unix)]
        {
            use std::os::unix::fs::OpenOptionsExt;
            options.mode(0o600);
        }
        let mut file = options.open(&replacement)?;
        set_provider_action_state_file_permissions(&file)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        drop(file);

        ensure_provider_action_state_destination_safe(&path)?;
        fs::rename(&replacement, &path)?;
        renamed = true;
        set_provider_action_state_path_permissions(&path)?;

        if should_fail_provider_action_state_directory_sync(app_data_dir, record.phase) {
            return Err(io::Error::other(
                "injected provider action-state directory sync failure",
            ));
        }
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    })();
    if result.is_err() {
        let _ = remove_provider_action_state_replacement(&replacement);
    }
    match result {
        Ok(()) => Ok(()),
        Err(_) if renamed => Err(ServiceError::AppliedUnverified(
            "provider replay state was replaced, but its durable directory update could not be verified"
                .to_string(),
        )),
        Err(_) => Err(ServiceError::ActionNotStarted(
            "provider replay state could not be replaced before the action started".to_string(),
        )),
    }
}

#[cfg(test)]
fn should_fail_provider_action_state_directory_sync(
    app_data_dir: &Path,
    phase: ProviderActionStatePhase,
) -> bool {
    phase == ProviderActionStatePhase::Outcome
        && take_test_provider_action_state_fault(
            app_data_dir,
            TestProviderActionStateFault::OutcomeDirectorySync,
        )
}

#[cfg(not(test))]
fn should_fail_provider_action_state_directory_sync(
    _app_data_dir: &Path,
    _phase: ProviderActionStatePhase,
) -> bool {
    false
}

fn ensure_provider_action_state_destination_safe(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "provider action-state destination is not a regular file",
            ))
        }
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn remove_provider_action_state_replacement(path: &Path) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_file() => {
            Err(io::Error::new(
                io::ErrorKind::InvalidInput,
                "provider action-state replacement is not a regular file",
            ))
        }
        Ok(_) => fs::remove_file(path),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

fn remove_legacy_provider_action_state_replacements(parent: &Path) -> io::Result<()> {
    for (index, entry) in fs::read_dir(parent)?.enumerate() {
        if index >= PROVIDER_ACTION_STATE_MAX_DIRECTORY_ENTRIES {
            return Err(io::Error::other(
                "provider action-state directory exceeds the bounded cleanup scan",
            ));
        }
        let entry = entry?;
        let name = entry.file_name();
        let Some(name) = name.to_str() else {
            continue;
        };
        if !name.starts_with(PROVIDER_ACTION_STATE_LEGACY_REPLACEMENT_PREFIX)
            || !name.ends_with(PROVIDER_ACTION_STATE_LEGACY_REPLACEMENT_SUFFIX)
        {
            continue;
        }
        remove_provider_action_state_replacement(&entry.path())?;
    }
    Ok(())
}

#[cfg(unix)]
fn set_provider_action_state_file_permissions(file: &fs::File) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    file.set_permissions(fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_provider_action_state_file_permissions(_file: &fs::File) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_provider_action_state_path_permissions(path: &Path) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;
    fs::set_permissions(path, fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_provider_action_state_path_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn read_provider_action_state(
    app_data_dir: &Path,
) -> Result<Option<(Vec<u8>, ProviderActionStateRecord)>, ServiceError> {
    let path = provider_action_state_path(app_data_dir);
    let mut file = match open_provider_action_state(&path) {
        Ok(file) => file,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
        Err(_) => return Err(invalid_provider_action_state()),
    };
    let metadata = file
        .metadata()
        .map_err(|_| invalid_provider_action_state())?;
    if !metadata.is_file() || metadata.len() > PROVIDER_ACTION_STATE_MAX_BYTES as u64 {
        return Err(invalid_provider_action_state());
    }
    let mut bytes = vec![0_u8; PROVIDER_ACTION_STATE_MAX_BYTES + 1];
    let mut length = 0;
    while length < bytes.len() {
        let read = file
            .read(&mut bytes[length..])
            .map_err(|_| invalid_provider_action_state())?;
        if read == 0 {
            break;
        }
        length += read;
    }
    bytes.truncate(length);
    if bytes.len() > PROVIDER_ACTION_STATE_MAX_BYTES {
        return Err(invalid_provider_action_state());
    }
    let record = serde_json::from_slice::<ProviderActionStateRecord>(&bytes)
        .map_err(|_| invalid_provider_action_state())?;
    validate_provider_action_state(&record)?;
    Ok(Some((bytes, record)))
}

fn validate_provider_action_state(record: &ProviderActionStateRecord) -> Result<(), ServiceError> {
    let valid = record.version == PROVIDER_ACTION_STATE_VERSION
        && record.generation > 0
        && is_canonical_sha256(&record.token_digest)
        && !record.action_id.trim().is_empty()
        && record.action_id.len() <= 256
        && is_canonical_sha256(&record.source_revision)
        && record.updated_at >= 0
        && (record.phase != ProviderActionStatePhase::Reservation
            || record.state == ProviderActionState::NotStarted);
    if valid {
        Ok(())
    } else {
        Err(invalid_provider_action_state())
    }
}

fn is_canonical_sha256(value: &str) -> bool {
    value.strip_prefix("sha256:").is_some_and(|digest| {
        digest.len() == 64
            && digest
                .bytes()
                .all(|byte| byte.is_ascii_digit() || matches!(byte, b'a'..=b'f'))
    })
}

fn invalid_provider_action_state() -> ServiceError {
    ServiceError::InvalidRequest("provider action replay state is invalid".to_string())
}

#[cfg(unix)]
fn open_provider_action_state(path: &Path) -> io::Result<fs::File> {
    use rustix::fs::{open, Mode, OFlags};

    let flags =
        OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::NONBLOCK | OFlags::NOCTTY | OFlags::CLOEXEC;
    open(path, flags, Mode::empty())
        .map(fs::File::from)
        .map_err(io::Error::from)
}

#[cfg(not(unix))]
fn open_provider_action_state(path: &Path) -> io::Result<fs::File> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => {
            Err(io::Error::new(io::ErrorKind::InvalidInput, "symlink"))
        }
        _ => fs::File::open(path),
    }
}

#[cfg(test)]
pub(crate) fn provider_action_state_snapshot(
    app_data_dir: &Path,
) -> Result<(u64, ProviderActionStatePhase, ProviderActionState, usize), ServiceError> {
    let (bytes, record) = read_provider_action_state(app_data_dir)?.ok_or_else(|| {
        ServiceError::InvalidRequest("provider action state is missing".to_string())
    })?;
    Ok((record.generation, record.phase, record.state, bytes.len()))
}

#[cfg(test)]
pub(crate) fn provider_action_state_file(app_data_dir: &Path) -> PathBuf {
    provider_action_state_path(app_data_dir)
}

#[cfg(test)]
pub(crate) fn provider_action_state_replacement_file(app_data_dir: &Path) -> PathBuf {
    provider_action_state_replacement_path(app_data_dir)
}

fn action_token_digest(token: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot/action-token-digest/v1");
    hasher.update((token.len() as u64).to_be_bytes());
    hasher.update(token.as_bytes());
    format!("sha256:{:x}", hasher.finalize())
}

pub(crate) fn lock_provider_actions(app_data_dir: &Path) -> Result<fs::File, ServiceError> {
    let parent = app_data_dir.parent().ok_or_else(|| {
        ServiceError::InvalidRequest("provider app-data path has no parent".to_string())
    })?;
    let canonical_parent = fs::canonicalize(parent)?;
    if !fs::metadata(&canonical_parent)?.is_dir() {
        return Err(ServiceError::InvalidRequest(
            "provider app-data parent must be an existing directory".to_string(),
        ));
    }
    // Lock the already-existing canonical parent directory itself. This
    // creates no lock file or app-data directory, so stale or mismatched
    // confirmations remain a zero-artifact rejection.
    let file = fs::File::open(canonical_parent)?;
    file.lock_exclusive()?;
    Ok(file)
}
