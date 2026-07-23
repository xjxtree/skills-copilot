use std::{
    io,
    path::{Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use skills_copilot_commands::{
    action_descriptor, action_preview_binding, action_source_revision, ensure_action_confirmed,
    lock_app_mutations, opaque_sensitive_action_input_binding, ActionConfirmation,
    ActionPrecondition, ActionPreconditionKind, ActionPreviewBinding, ActionReadbackObservation,
    ActionReadbackRecord, AppDataPrivateLeafKind, AppDataPrivateLeafSnapshot, AppMutationLock,
    CommandError,
};
use skills_copilot_core::{
    ActionDescriptor, ActionImpact, ActionIntent, ActionKind, ActionNetworkPosture,
    ActionReadbackDomain, ActionTargetKind, ActionTargetRef,
};

use super::{
    service_host::{
        canonicalize_llm_prompt_runs_for_storage, LLM_PROMPT_RUNS_BOUND_QUARANTINE_PREFIX,
        LLM_PROMPT_RUNS_MAX_BYTES, LLM_PROMPT_RUNS_RELATIVE_PATH, MODEL_TASK_MATCHES_MAX_BYTES,
        MODEL_TASK_MATCHES_RELATIVE_PATH,
    },
    service_provider_actions::{
        finalize_provider_action_while_locked, provider_action_state_revision_while_locked,
        reserve_provider_action_while_locked, ProviderActionState,
    },
    LlmPromptRunRecord, ServiceError, ServiceHost,
};

const TASK_PREFLIGHT_HISTORY_RELATIVE_PATH: &str = "task-preflight-history.json";
const TASK_PREFLIGHT_HISTORY_MAX_BYTES: u64 = 8 * 1_024 * 1_024;
const LEGACY_PRIVATE_CONTENT_TARGET_ID: &str = "legacy-ai-private-content";
const LEGACY_PRIVATE_CONTENT_MAX_QUARANTINES_PER_SOURCE: usize = 32;

const PROMPT_RUNS_QUARANTINE_PREFIX: &str = ".prompt-runs.json.legacy-private-cleanup-";
const MODEL_TASK_MATCHES_QUARANTINE_PREFIX: &str =
    ".model-task-matches.json.legacy-private-cleanup-";
const TASK_PREFLIGHT_HISTORY_QUARANTINE_PREFIX: &str =
    ".task-preflight-history.json.legacy-private-cleanup-";
const LEGACY_PRIVATE_CONTENT_QUARANTINE_SUFFIX: &str = ".quarantine";

#[derive(Debug, Clone, Serialize)]
pub struct LegacyPrivateContentInspection {
    pub generated_by: &'static str,
    pub cleanup_required: bool,
    pub cleanup_source_count: usize,
    pub existing_source_count: usize,
    pub sources: Vec<LegacyPrivateContentSource>,
    pub read_only: bool,
    pub provider_request_sent: bool,
    pub raw_content_returned: bool,
    pub write_performed: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LegacyPrivateContentSource {
    pub id: String,
    pub source_file: &'static str,
    pub item_type: &'static str,
    pub state: &'static str,
    pub cleanup_operation: &'static str,
    pub cleanup_required: bool,
    pub malformed: bool,
    pub generated_residue: bool,
}

#[derive(Debug, Clone, Serialize)]
pub struct LegacyPrivateContentCleanupPreview {
    pub inspection: LegacyPrivateContentInspection,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub action: Option<ActionDescriptor>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub preconditions: Vec<ActionPrecondition>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub preview_token: Option<String>,
    pub confirmation_required: bool,
}

#[derive(Debug, Clone, Deserialize)]
pub struct LegacyPrivateContentCleanupParams {
    pub action_confirmation: ActionConfirmation,
}

#[derive(Debug, Clone, Serialize)]
pub struct LegacyPrivateContentCleanupResult {
    pub inspection: LegacyPrivateContentInspection,
    pub cleaned_source_count: usize,
    pub state: &'static str,
    pub effect: &'static str,
    pub retry_allowed: bool,
    pub readback: ActionReadbackRecord,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum LegacyPrivateContentOperation {
    SanitizePromptRuns,
    Delete,
}

#[derive(Debug, Clone)]
struct LegacyPrivateContentPlan {
    id: String,
    source_file: &'static str,
    relative_path: PathBuf,
    snapshot: AppDataPrivateLeafSnapshot,
    operation: LegacyPrivateContentOperation,
    replacement: Option<Vec<u8>>,
    quarantine_prefix: &'static str,
}

#[derive(Debug)]
struct LegacyPrivateContentProjection {
    inspection: LegacyPrivateContentInspection,
    plans: Vec<LegacyPrivateContentPlan>,
}

impl ServiceHost {
    pub(crate) fn inspect_legacy_private_content(
        &self,
    ) -> Result<LegacyPrivateContentInspection, ServiceError> {
        let Some(owner) = lock_existing_app_data_owner(&self.app_data_dir)? else {
            return Ok(empty_legacy_private_content_inspection());
        };
        Ok(project_legacy_private_content(&owner)?.inspection)
    }

    pub(crate) fn preview_cleanup_legacy_private_content(
        &self,
    ) -> Result<LegacyPrivateContentCleanupPreview, ServiceError> {
        let Some(owner) = lock_existing_app_data_owner(&self.app_data_dir)? else {
            return Ok(clean_preview(empty_legacy_private_content_inspection()));
        };
        self.preview_cleanup_legacy_private_content_with_owner(&owner)
    }

    fn preview_cleanup_legacy_private_content_with_owner(
        &self,
        owner: &AppMutationLock,
    ) -> Result<LegacyPrivateContentCleanupPreview, ServiceError> {
        let projection = project_legacy_private_content(owner)?;
        if projection.plans.is_empty() {
            return Ok(clean_preview(projection.inspection));
        }
        let action_state_revision = provider_action_state_revision_while_locked(owner)?;
        let binding = legacy_private_content_action_binding(&projection, &action_state_revision)?;
        Ok(preview_from_binding(projection.inspection, binding))
    }

    pub(crate) fn cleanup_legacy_private_content(
        &self,
        params: LegacyPrivateContentCleanupParams,
    ) -> Result<LegacyPrivateContentCleanupResult, ServiceError> {
        let preliminary = self.preview_cleanup_legacy_private_content()?;
        let preliminary_binding =
            preview_binding(&preliminary).ok_or(CommandError::StaleActionReference)?;
        ensure_action_confirmed(&preliminary_binding, Some(&params.action_confirmation))?;

        let owner = lock_app_mutations(&self.app_data_dir).map_err(|error| match error {
            CommandError::PartialEffect { .. } => ServiceError::Command(error),
            _ => ServiceError::ActionNotStarted(
                "legacy private-content cleanup owner is no longer available; create a new preview"
                    .to_string(),
            ),
        })?;
        let projection = project_legacy_private_content(&owner)?;
        if projection.plans.is_empty() {
            return Err(CommandError::StaleActionReference.into());
        }
        let action_state_revision = provider_action_state_revision_while_locked(&owner)?;
        let current_binding =
            legacy_private_content_action_binding(&projection, &action_state_revision)?;
        ensure_action_confirmed(&current_binding, Some(&params.action_confirmation))?;
        owner.validate_owner_path_binding()?;
        if let Err(error) = reserve_provider_action_while_locked(
            &self.app_data_dir,
            &owner,
            &current_binding,
            &params.action_confirmation,
        ) {
            if matches!(
                &error,
                ServiceError::Command(CommandError::PartialEffect { .. })
            ) {
                return Err(privacy_cleanup_partial_error(
                    "legacy private-content cleanup reservation could not be durably verified; inspect the persistent cleanup state before creating a new preview",
                ));
            }
            return Err(error);
        }

        let mut cleaned_source_count = 0_usize;
        for plan in &projection.plans {
            let result = apply_legacy_private_content_plan(&owner, plan);
            if let Err(error) = result {
                let partial =
                    matches!(error, CommandError::PartialEffect { .. }) || cleaned_source_count > 0;
                let final_state = if partial {
                    ProviderActionState::Partial
                } else {
                    ProviderActionState::NotStarted
                };
                if finalize_provider_action_while_locked(
                    &self.app_data_dir,
                    &owner,
                    &current_binding,
                    &params.action_confirmation,
                    final_state,
                )
                .is_err()
                {
                    if partial {
                        return Err(privacy_cleanup_partial_error(
                            "legacy private-content cleanup stopped after an effect and its replay finalization could not be verified",
                        ));
                    }
                    return Err(ServiceError::ActionNotStarted(
                        "legacy private-content cleanup did not start and its replay finalization could not be verified; inspect current state before creating a new preview"
                            .to_string(),
                    ));
                }
                if owner.validate_owner_path_binding().is_err() {
                    if partial {
                        return Err(privacy_cleanup_partial_error(
                            "legacy private-content cleanup stopped after an effect on an app-data owner that is no longer bound to the configured path",
                        ));
                    }
                    return Err(ServiceError::ActionNotStarted(
                        "legacy private-content cleanup did not start because the accepted app-data owner is no longer bound to the configured path"
                            .to_string(),
                    ));
                }
                if partial {
                    return Err(privacy_cleanup_partial_error(
                        "legacy private-content cleanup stopped after one or more effects; inspect the persistent cleanup state before creating a new preview",
                    ));
                }
                return Err(ServiceError::ActionNotStarted(format!(
                    "legacy private-content cleanup did not start: {error}; create a new preview"
                )));
            }
            cleaned_source_count = cleaned_source_count.saturating_add(1);
        }

        let readback_projection = project_legacy_private_content(&owner).map_err(|_| {
            privacy_cleanup_partial_error(
                "legacy private-content cleanup completed but semantic read-back failed",
            )
        })?;
        if readback_projection.inspection.cleanup_required {
            let _ = finalize_provider_action_while_locked(
                &self.app_data_dir,
                &owner,
                &current_binding,
                &params.action_confirmation,
                ProviderActionState::Partial,
            );
            return Err(privacy_cleanup_partial_error(
                "legacy private-content cleanup left content requiring another explicit review",
            ));
        }
        let readback_revision =
            legacy_private_content_readback_revision(&readback_projection.inspection)?;
        let readback = ActionReadbackRecord::verified(
            &current_binding.action,
            vec![ActionReadbackObservation {
                domain: ActionReadbackDomain::PrivateContent,
                target_id: LEGACY_PRIVATE_CONTENT_TARGET_ID.to_string(),
                revision: readback_revision,
            }],
        )
        .map_err(|_| {
            privacy_cleanup_partial_error(
                "legacy private-content cleanup completed but its semantic read-back record could not be verified",
            )
        })?;
        finalize_provider_action_while_locked(
            &self.app_data_dir,
            &owner,
            &current_binding,
            &params.action_confirmation,
            ProviderActionState::Verified,
        )
        .map_err(|_| {
            privacy_cleanup_partial_error(
                "legacy private-content cleanup completed but replay finalization could not be verified",
            )
        })?;
        owner.validate_owner_path_binding().map_err(|_| {
            privacy_cleanup_partial_error(
                "legacy private-content cleanup completed on an app-data owner that is no longer bound to the configured path",
            )
        })?;
        Ok(LegacyPrivateContentCleanupResult {
            inspection: readback_projection.inspection,
            cleaned_source_count,
            state: "verified",
            effect: "legacy_private_content_removed_or_sanitized",
            retry_allowed: false,
            readback,
        })
    }
}

fn lock_existing_app_data_owner(
    app_data_dir: &Path,
) -> Result<Option<AppMutationLock>, ServiceError> {
    match lock_app_mutations(app_data_dir) {
        Ok(owner) => Ok(Some(owner)),
        Err(CommandError::Io(error)) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(error.into()),
    }
}

fn empty_legacy_private_content_inspection() -> LegacyPrivateContentInspection {
    LegacyPrivateContentInspection {
        generated_by: "local-v2.64",
        cleanup_required: false,
        cleanup_source_count: 0,
        existing_source_count: 0,
        sources: Vec::new(),
        read_only: true,
        provider_request_sent: false,
        raw_content_returned: false,
        write_performed: false,
    }
}

fn clean_preview(inspection: LegacyPrivateContentInspection) -> LegacyPrivateContentCleanupPreview {
    LegacyPrivateContentCleanupPreview {
        inspection,
        action: None,
        preconditions: Vec::new(),
        preview_token: None,
        confirmation_required: false,
    }
}

fn preview_from_binding(
    inspection: LegacyPrivateContentInspection,
    binding: ActionPreviewBinding,
) -> LegacyPrivateContentCleanupPreview {
    LegacyPrivateContentCleanupPreview {
        inspection,
        action: Some(binding.action),
        preconditions: binding.preconditions,
        preview_token: Some(binding.preview_token),
        confirmation_required: true,
    }
}

fn preview_binding(preview: &LegacyPrivateContentCleanupPreview) -> Option<ActionPreviewBinding> {
    Some(ActionPreviewBinding {
        action: preview.action.clone()?,
        preconditions: preview.preconditions.clone(),
        preview_token: preview.preview_token.clone()?,
    })
}

fn project_legacy_private_content(
    owner: &AppMutationLock,
) -> Result<LegacyPrivateContentProjection, ServiceError> {
    let owner_fs = owner.owner_fs();
    let mut inspection_sources = Vec::new();
    let mut plans = Vec::new();

    if let Some(snapshot) = owner_fs.inspect_private_cleanup_leaf(
        Path::new(LLM_PROMPT_RUNS_RELATIVE_PATH),
        LLM_PROMPT_RUNS_MAX_BYTES,
        "legacy prompt-run history",
    )? {
        let (cleanup_required, malformed, replacement) = prompt_run_cleanup_projection(&snapshot)?;
        let operation = if snapshot.kind() == AppDataPrivateLeafKind::RegularFile && !malformed {
            LegacyPrivateContentOperation::SanitizePromptRuns
        } else {
            LegacyPrivateContentOperation::Delete
        };
        inspection_sources.push(legacy_source_row(
            "prompt-runs",
            "prompt-runs.json",
            &snapshot,
            cleanup_required,
            malformed,
            false,
            operation,
        ));
        if cleanup_required {
            plans.push(LegacyPrivateContentPlan {
                id: "prompt-runs".to_string(),
                source_file: "prompt-runs.json",
                relative_path: PathBuf::from(LLM_PROMPT_RUNS_RELATIVE_PATH),
                snapshot,
                operation,
                replacement,
                quarantine_prefix: PROMPT_RUNS_QUARANTINE_PREFIX,
            });
        }
    }

    project_delete_only_source(
        owner,
        MODEL_TASK_MATCHES_RELATIVE_PATH,
        MODEL_TASK_MATCHES_MAX_BYTES,
        "model-task-matches",
        "model-task-matches.json",
        MODEL_TASK_MATCHES_QUARANTINE_PREFIX,
        &mut inspection_sources,
        &mut plans,
    )?;
    project_delete_only_source(
        owner,
        TASK_PREFLIGHT_HISTORY_RELATIVE_PATH,
        TASK_PREFLIGHT_HISTORY_MAX_BYTES,
        "task-preflight-history",
        "task-preflight-history.json",
        TASK_PREFLIGHT_HISTORY_QUARANTINE_PREFIX,
        &mut inspection_sources,
        &mut plans,
    )?;

    project_quarantine_residues(
        owner,
        "prompt-runs",
        "prompt-runs.json",
        PROMPT_RUNS_QUARANTINE_PREFIX,
        LLM_PROMPT_RUNS_MAX_BYTES,
        &mut inspection_sources,
        &mut plans,
    )?;
    project_quarantine_residues(
        owner,
        "prompt-runs-bound-replace",
        "prompt-runs.json",
        LLM_PROMPT_RUNS_BOUND_QUARANTINE_PREFIX,
        LLM_PROMPT_RUNS_MAX_BYTES,
        &mut inspection_sources,
        &mut plans,
    )?;
    project_quarantine_residues(
        owner,
        "model-task-matches",
        "model-task-matches.json",
        MODEL_TASK_MATCHES_QUARANTINE_PREFIX,
        MODEL_TASK_MATCHES_MAX_BYTES,
        &mut inspection_sources,
        &mut plans,
    )?;
    project_quarantine_residues(
        owner,
        "task-preflight-history",
        "task-preflight-history.json",
        TASK_PREFLIGHT_HISTORY_QUARANTINE_PREFIX,
        TASK_PREFLIGHT_HISTORY_MAX_BYTES,
        &mut inspection_sources,
        &mut plans,
    )?;

    plans.sort_by(|left, right| left.id.cmp(&right.id));
    inspection_sources.sort_by(|left, right| left.id.cmp(&right.id));
    let cleanup_source_count = inspection_sources
        .iter()
        .filter(|source| source.cleanup_required)
        .count();
    Ok(LegacyPrivateContentProjection {
        inspection: LegacyPrivateContentInspection {
            generated_by: "local-v2.64",
            cleanup_required: cleanup_source_count > 0,
            cleanup_source_count,
            existing_source_count: inspection_sources.len(),
            sources: inspection_sources,
            read_only: true,
            provider_request_sent: false,
            raw_content_returned: false,
            write_performed: false,
        },
        plans,
    })
}

#[allow(clippy::too_many_arguments)]
fn project_delete_only_source(
    owner: &AppMutationLock,
    relative_path: &str,
    max_bytes: u64,
    id: &str,
    source_file: &'static str,
    quarantine_prefix: &'static str,
    inspection_sources: &mut Vec<LegacyPrivateContentSource>,
    plans: &mut Vec<LegacyPrivateContentPlan>,
) -> Result<(), ServiceError> {
    let Some(snapshot) = owner.owner_fs().inspect_private_cleanup_leaf(
        Path::new(relative_path),
        max_bytes,
        "legacy private-content source",
    )?
    else {
        return Ok(());
    };
    inspection_sources.push(legacy_source_row(
        id,
        source_file,
        &snapshot,
        true,
        false,
        false,
        LegacyPrivateContentOperation::Delete,
    ));
    plans.push(LegacyPrivateContentPlan {
        id: id.to_string(),
        source_file,
        relative_path: PathBuf::from(relative_path),
        snapshot,
        operation: LegacyPrivateContentOperation::Delete,
        replacement: None,
        quarantine_prefix,
    });
    Ok(())
}

#[allow(clippy::too_many_arguments)]
fn project_quarantine_residues(
    owner: &AppMutationLock,
    id_prefix: &str,
    source_file: &'static str,
    quarantine_prefix: &'static str,
    max_bytes: u64,
    inspection_sources: &mut Vec<LegacyPrivateContentSource>,
    plans: &mut Vec<LegacyPrivateContentPlan>,
) -> Result<(), ServiceError> {
    let matches = owner.owner_fs().list_root_private_cleanup_leaves_matching(
        quarantine_prefix,
        LEGACY_PRIVATE_CONTENT_QUARANTINE_SUFFIX,
        LEGACY_PRIVATE_CONTENT_MAX_QUARANTINES_PER_SOURCE,
        max_bytes,
        "legacy private-content quarantine",
    )?;
    for (index, (name, snapshot)) in matches.into_iter().enumerate() {
        let id = format!("{id_prefix}-cleanup-residue-{index}");
        inspection_sources.push(legacy_source_row(
            &id,
            source_file,
            &snapshot,
            true,
            false,
            true,
            LegacyPrivateContentOperation::Delete,
        ));
        plans.push(LegacyPrivateContentPlan {
            id,
            source_file,
            relative_path: PathBuf::from(name),
            snapshot,
            operation: LegacyPrivateContentOperation::Delete,
            replacement: None,
            quarantine_prefix,
        });
    }
    Ok(())
}

fn prompt_run_cleanup_projection(
    snapshot: &AppDataPrivateLeafSnapshot,
) -> Result<(bool, bool, Option<Vec<u8>>), ServiceError> {
    if snapshot.kind() == AppDataPrivateLeafKind::SymbolicLink {
        return Ok((true, false, None));
    }
    let bytes = snapshot.regular_file_bytes().ok_or_else(|| {
        ServiceError::InvalidRequest(
            "legacy prompt-run cleanup snapshot has no bounded regular-file content".to_string(),
        )
    })?;
    let mut runs = match serde_json::from_slice::<Vec<LlmPromptRunRecord>>(bytes) {
        Ok(runs) => runs,
        Err(_) => return Ok((true, true, None)),
    };
    let cleanup_required = runs.iter().any(prompt_run_contains_legacy_private_content);
    if !cleanup_required {
        return Ok((false, false, None));
    }
    canonicalize_llm_prompt_runs_for_storage(&mut runs);
    let replacement = serde_json::to_vec_pretty(&runs)?;
    if replacement.len() as u64 > LLM_PROMPT_RUNS_MAX_BYTES {
        return Err(ServiceError::InvalidRequest(
            "canonical prompt-run metadata exceeds its private storage safety bound".to_string(),
        ));
    }
    Ok((true, false, Some(replacement)))
}

fn prompt_run_contains_legacy_private_content(run: &LlmPromptRunRecord) -> bool {
    run.task.is_some()
        || run.draft_output.is_some()
        || run.draft_requires_user_copy
        || run.raw_prompt_persisted
        || run.raw_response_persisted
        || run.redaction_summary.raw_prompt_persisted
        || run.redaction_summary.raw_response_persisted
        || run.safety_flags.raw_prompt_persisted
        || run.safety_flags.raw_response_persisted
}

fn legacy_source_row(
    id: &str,
    source_file: &'static str,
    snapshot: &AppDataPrivateLeafSnapshot,
    cleanup_required: bool,
    malformed: bool,
    generated_residue: bool,
    operation: LegacyPrivateContentOperation,
) -> LegacyPrivateContentSource {
    LegacyPrivateContentSource {
        id: id.to_string(),
        source_file,
        item_type: snapshot.kind().as_str(),
        state: if generated_residue {
            "cleanup_residue"
        } else if malformed {
            "malformed_legacy_content"
        } else if cleanup_required {
            "legacy_private_content"
        } else {
            "current_metadata_only"
        },
        cleanup_operation: match operation {
            LegacyPrivateContentOperation::SanitizePromptRuns => "sanitize_metadata",
            LegacyPrivateContentOperation::Delete => "delete_leaf",
        },
        cleanup_required,
        malformed,
        generated_residue,
    }
}

fn legacy_private_content_action_binding(
    projection: &LegacyPrivateContentProjection,
    action_state_revision: &str,
) -> Result<ActionPreviewBinding, ServiceError> {
    let mut preconditions = projection
        .plans
        .iter()
        .map(|plan| {
            let sensitive_revision = format!(
                "{}:{}:{}:{}:{}",
                plan.id,
                plan.source_file,
                plan.snapshot.kind().as_str(),
                plan.snapshot.revision(),
                match plan.operation {
                    LegacyPrivateContentOperation::SanitizePromptRuns => "sanitize",
                    LegacyPrivateContentOperation::Delete => "delete",
                }
            );
            Ok(ActionPrecondition {
                kind: ActionPreconditionKind::LegacyPrivateContent,
                target_id: plan.id.clone(),
                expected_revision: opaque_sensitive_action_input_binding(
                    "legacy-private-content-leaf",
                    &sensitive_revision,
                )?,
            })
        })
        .collect::<Result<Vec<_>, CommandError>>()?;
    preconditions.push(ActionPrecondition {
        kind: ActionPreconditionKind::PromptContext,
        target_id: "provider-action-state".to_string(),
        expected_revision: action_state_revision.to_string(),
    });
    let serialized_preconditions = serde_json::to_string(&preconditions)?;
    let source_revision = action_source_revision(
        "privacy.cleanupLegacyContent",
        &[
            ("leaf_preconditions", &serialized_preconditions),
            ("action_state", action_state_revision),
        ],
    )?;
    let action = action_descriptor(
        ActionKind::PrivacyCleanup,
        ActionIntent::CleanLegacyPrivateContent,
        ActionTargetRef {
            kind: ActionTargetKind::AppData,
            id: LEGACY_PRIVATE_CONTENT_TARGET_ID.to_string(),
            agent: None,
            scope: None,
        },
        None,
        vec![ActionImpact::AppLocalData],
        "privacy.previewCleanupLegacyContent",
        Some("privacy.cleanupLegacyContent"),
        source_revision,
        true,
        ActionNetworkPosture::None,
        vec![ActionReadbackDomain::PrivateContent],
        vec!["app-data:legacy-ai-private-content".to_string()],
    )?;
    Ok(action_preview_binding(action, preconditions)?)
}

fn apply_legacy_private_content_plan(
    owner: &AppMutationLock,
    plan: &LegacyPrivateContentPlan,
) -> Result<(), CommandError> {
    match plan.operation {
        LegacyPrivateContentOperation::SanitizePromptRuns => {
            let replacement = plan
                .replacement
                .as_deref()
                .ok_or(CommandError::StaleActionReference)?;
            owner.owner_fs().replace_private_cleanup_regular_leaf(
                &plan.relative_path,
                &plan.snapshot,
                replacement,
                "prompt-runs-privacy-cleanup",
                plan.quarantine_prefix,
            )
        }
        LegacyPrivateContentOperation::Delete => owner.owner_fs().remove_private_cleanup_leaf(
            &plan.relative_path,
            &plan.snapshot,
            plan.quarantine_prefix,
        ),
    }
}

fn legacy_private_content_readback_revision(
    inspection: &LegacyPrivateContentInspection,
) -> Result<String, ServiceError> {
    let summary = serde_json::to_string(&inspection.sources)?;
    Ok(action_source_revision(
        "privacy.legacyContentReadback",
        &[
            ("cleanup_required", &inspection.cleanup_required.to_string()),
            ("sources", &summary),
        ],
    )?)
}

fn privacy_cleanup_partial_error(detail: &str) -> ServiceError {
    CommandError::PartialEffect {
        operation: "legacy AI/private-content cleanup".to_string(),
        state: "applied_unverified",
        cleanup_required: true,
        detail: detail.to_string(),
    }
    .into()
}
