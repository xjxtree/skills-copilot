use std::io;

use skills_copilot_catalog::CatalogError;
use skills_copilot_core::Scope;
use skills_copilot_scanner::ScannerError;
use thiserror::Error;

use crate::product_projection::ProductProjectionError;

#[derive(Debug, Error)]
pub enum CommandError {
    #[error("scanner error: {0}")]
    Scanner(#[from] ScannerError),
    #[error("catalog error: {0}")]
    Catalog(#[from] CatalogError),
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("adapter error: {0}")]
    Adapter(String),
    #[error("skill instance not found: {0}")]
    InstanceNotFound(String),
    #[error("finding not found for triage key: {0}")]
    FindingNotFound(String),
    #[error("config snapshot not found: {0}")]
    SnapshotNotFound(String),
    #[error("scope not supported for toggle: {0:?}")]
    UnsupportedScope(Scope),
    #[error("config write verification failed; rolled back")]
    VerificationFailed,
    #[error("{operation} reached {state}; cleanup_required={cleanup_required}: {detail}")]
    PartialEffect {
        operation: String,
        state: &'static str,
        cleanup_required: bool,
        detail: String,
    },
    #[error("config changed since it was read (expected {expected}, actual {actual})")]
    ConfigConflict { expected: String, actual: String },
    #[error("invalid json config: {0}")]
    InvalidJson(String),
    #[error("unsafe config path: {0}")]
    UnsafeConfigPath(String),
    #[error("invalid skill bundle: {0}")]
    InvalidSkillBundle(String),
    #[error("invalid skill source: {0}")]
    InvalidSkillSource(String),
    #[error("invalid import source: {0}")]
    InvalidImportSource(String),
    #[error("unsupported import source: {0}")]
    UnsupportedImportSource(String),
    #[error("install is not supported: {0}")]
    InstallUnsupported(String),
    #[error("invalid script execution request: {0}")]
    InvalidScriptExecutionRequest(String),
    #[error("invalid finding triage status: {0}")]
    InvalidFindingTriageStatus(String),
    #[error("invalid rule severity override: {0}")]
    InvalidRuleSeverityOverride(String),
    #[error("invalid rule tuning request: {0}")]
    InvalidRuleTuningRequest(String),
    #[error("invalid batch action: {0}")]
    InvalidBatchAction(String),
    #[error("invalid product projection: {0}")]
    InvalidProductProjection(#[from] ProductProjectionError),
    #[error("unknown action reference: {0}")]
    UnknownActionReference(String),
    #[error("action reference is stale; preview again before confirming")]
    StaleActionReference,
    #[error("action reference does not match the current target: {0}")]
    MismatchedActionReference(String),
    #[error("action confirmation is required: {0}")]
    ActionConfirmationRequired(String),
    #[error("action preview signing is unavailable: {0}")]
    ActionTokenUnavailable(String),
    #[error("action has no applicable mutation: {0}")]
    NoApplicableAction(String),
    #[error("skill manager unavailable: {0}")]
    SkillManagerUnavailable(String),
    #[error("invalid skill manager request: {0}")]
    InvalidSkillManagerRequest(String),
    #[error("skill manager command failed: {0}")]
    SkillManagerCommandFailed(String),
}
