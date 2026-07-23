use skills_copilot_catalog::{CatalogCommitError, CatalogImmediateTransaction};

use super::*;

pub(super) fn manager_post_process_error(
    ctx: &AdapterContext,
    preview: &SkillManagerCommandPreview,
    error: CommandError,
) -> CommandError {
    if matches!(error, CommandError::PartialEffect { .. }) {
        return error;
    }
    manager_partial_effect(
        ctx,
        preview,
        "applied_unverified",
        true,
        &format!("post-execution verification failed: {error}"),
    )
}

pub(super) fn commit_manager_catalog_transaction(
    ctx: &AdapterContext,
    preview: &SkillManagerCommandPreview,
    transaction: CatalogImmediateTransaction<'_>,
) -> Result<(), CommandError> {
    let (state, detail, error) = match transaction.commit_classified() {
        Ok(()) => return Ok(()),
        Err(CatalogCommitError::NotCommitted(error)) => (
            "applied_unverified",
            "catalog commit was rejected after manager execution; the verified manager target state was preserved and catalog recovery is required",
            error,
        ),
        Err(CatalogCommitError::OutcomeUnknown(error)) => (
            "outcome_unknown",
            "catalog commit outcome is unknown after manager execution; the manager target state was preserved for inspection",
            error,
        ),
    };
    Err(manager_partial_effect(
        ctx,
        preview,
        state,
        true,
        &format!("{detail}: {error}"),
    ))
}

pub(super) fn rollback_manager_catalog_transaction(
    ctx: &AdapterContext,
    preview: &SkillManagerCommandPreview,
    transaction: CatalogImmediateTransaction<'_>,
    original_error: CommandError,
) -> CommandError {
    match transaction.rollback() {
        Ok(()) => original_error,
        Err(rollback_error) => CommandError::PartialEffect {
            operation: preview.operation.clone(),
            state: "outcome_unknown",
            cleanup_required: true,
            detail: redact_command_output(
                ctx,
                &format!(
                    "operation failed ({original_error}); catalog rollback could not be proven ({rollback_error}); the external manager target state was preserved for inspection"
                ),
            ),
        },
    }
}
