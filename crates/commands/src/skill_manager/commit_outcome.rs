use skills_copilot_catalog::{CatalogCommitError, CatalogImmediateTransaction};

use super::*;

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
