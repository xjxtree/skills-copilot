use skills_copilot_catalog::CatalogImmediateTransaction;

use crate::CommandError;

pub(crate) fn rollback_catalog_before_compensation(
    transaction: CatalogImmediateTransaction<'_>,
    operation: &'static str,
    original_error: &CommandError,
    preserved_state: &str,
) -> Result<(), CommandError> {
    transaction
        .rollback()
        .map_err(|rollback_error| CommandError::PartialEffect {
            operation: operation.to_string(),
            state: "outcome_unknown",
            cleanup_required: true,
            detail: format!(
                "operation failed ({original_error}); catalog rollback could not be proven ({rollback_error}); {preserved_state}"
            ),
        })
}
