#[cfg(test)]
use std::{
    collections::HashMap,
    sync::{LazyLock, Mutex},
};
use std::{
    io::{self, Read},
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use keyring::Entry;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use sha2::{Digest, Sha256};
use skills_copilot_commands::{
    lock_app_mutations, lock_or_create_app_mutations, ActionConfirmation, AppMutationLock,
    CommandError,
};
use thiserror::Error;
use ureq::Error as UreqError;
use url::Url;

const KEYCHAIN_SERVICE: &str = "dev.skills-copilot.native.llm";
const PROFILE_STORE_VERSION: u32 = 1;
const PROVIDER_PROFILE_STORE_RELATIVE_PATH: &str = "llm/provider-profiles.json";
const PROVIDER_CALL_METADATA_RELATIVE_PATH: &str = "llm/provider-call-metadata.jsonl";
const PROVIDER_PROFILE_STORE_MAX_BYTES: u64 = 1024 * 1024;
const PROVIDER_CALL_METADATA_MAX_BYTES: u64 = 8 * 1024 * 1024;
const DEFAULT_SINGLE_REQUEST_TOKEN_LIMIT: u32 = 8_000;
const DEFAULT_MONTHLY_BUDGET_USD: f64 = 5.0;
const TEST_INPUT_TOKEN_ESTIMATE: u32 = 12;
const TEST_OUTPUT_TOKEN_ESTIMATE: u32 = 4;
const MAX_PROVIDER_RESPONSE_BYTES: usize = 4 * 1024 * 1024;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("app-data owner error: {0}")]
    Command(#[from] CommandError),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid provider profile: {0}")]
    InvalidProfile(String),
    #[error("provider profile not found: {0}")]
    ProfileNotFound(String),
    #[error("credential storage unavailable: {0}")]
    CredentialStorageUnavailable(String),
    #[error("credential mutation outcome is unverified: {0}")]
    CredentialMutationPartial(String),
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderProfileRecord {
    pub id: String,
    pub display_name: String,
    pub provider_type: ProviderType,
    pub base_url: String,
    pub model: String,
    pub enabled: bool,
    pub api_version: Option<String>,
    pub organization: Option<String>,
    pub single_request_token_limit: u32,
    pub monthly_budget_usd: f64,
    pub credential_reference: ProviderCredentialReference,
    pub credential_status: ProviderCredentialStatus,
    pub created_at: i64,
    pub updated_at: i64,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
pub enum ProviderType {
    #[serde(rename = "openai-compatible")]
    OpenAiCompatible,
    #[serde(rename = "claude-compatible")]
    ClaudeCompatible,
}

impl ProviderType {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::OpenAiCompatible => "openai-compatible",
            Self::ClaudeCompatible => "claude-compatible",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCredentialReference {
    pub storage: String,
    pub service: String,
    pub account: String,
    pub secret_persisted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCredentialStatus {
    pub state: String,
    pub reason: String,
    pub secret_available: bool,
    pub fallback_available: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderBudgetStatus {
    pub single_request_token_limit: u32,
    pub monthly_budget_usd: f64,
    pub estimated_test_tokens: u32,
    pub estimated_test_cost_usd: f64,
    pub state: String,
    pub reason: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveProviderProfileParams {
    #[serde(default)]
    pub id: Option<String>,
    pub display_name: String,
    pub provider_type: ProviderType,
    pub base_url: String,
    pub model: String,
    #[serde(default = "default_enabled")]
    pub enabled: bool,
    #[serde(default)]
    pub api_version: Option<String>,
    #[serde(default)]
    pub organization: Option<String>,
    #[serde(default)]
    pub api_key: Option<String>,
    #[serde(default)]
    pub single_request_token_limit: Option<u32>,
    #[serde(default)]
    pub monthly_budget_usd: Option<f64>,
    #[serde(default)]
    pub action_confirmation: Option<ActionConfirmation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteProviderProfileParams {
    pub profile_id: String,
    #[serde(default)]
    pub delete_credential: bool,
    #[serde(default)]
    pub action_confirmation: Option<ActionConfirmation>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestProviderConnectionParams {
    pub profile_id: String,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
    #[serde(default)]
    pub action_confirmation: Option<ActionConfirmation>,
    #[serde(skip)]
    pub confirmation_id: String,
}

#[derive(Debug, Clone)]
pub struct SendProviderPromptParams {
    pub profile_id: String,
    pub confirmation_id: String,
    pub action_type: String,
    pub prompt: String,
    pub estimated_input_tokens: u32,
    pub estimated_output_tokens: u32,
    pub estimated_cost_usd: f64,
    pub redaction_status: String,
    pub timeout_ms: Option<u64>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ListProviderProfilesResult {
    pub profiles: Vec<ProviderProfileRecord>,
    pub default_profile_id: Option<String>,
    pub credential_storage: String,
    pub credential_persistence_allowed: bool,
    pub raw_secrets_returned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SaveProviderProfileResult {
    pub profile: ProviderProfileRecord,
    pub credential_status: ProviderCredentialStatus,
    pub profile_persisted: bool,
    pub credential_effect: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub raw_secret_returned: bool,
    #[serde(skip, default)]
    pub(crate) operation_state: ProviderMutationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteProviderProfileResult {
    pub deleted_profile_id: String,
    pub profile_deleted: bool,
    pub credential_deleted: bool,
    pub credential_effect: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub raw_secret_returned: bool,
    #[serde(skip, default)]
    pub(crate) operation_state: ProviderMutationState,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestProviderConnectionResult {
    pub profile_id: String,
    pub provider_type: ProviderType,
    pub model: String,
    pub destination_host: String,
    pub status: String,
    pub provider_request_sent: bool,
    pub credential_accessed: bool,
    pub duration_ms: u128,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub budget: ProviderBudgetStatus,
    pub audit: ProviderCallMetadata,
    pub local_metadata_persisted: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_secret_returned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SendProviderPromptResult {
    pub profile_id: String,
    pub provider_type: ProviderType,
    pub model: String,
    pub destination_host: String,
    pub status: String,
    pub provider_request_sent: bool,
    pub credential_accessed: bool,
    pub duration_ms: u128,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub output_text: Option<String>,
    pub audit: ProviderCallMetadata,
    pub local_metadata_persisted: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
    pub raw_secret_returned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ProviderCallMetadata {
    pub timestamp: i64,
    pub action_type: String,
    pub profile_id: String,
    pub provider_type: ProviderType,
    pub model: String,
    pub destination_host: String,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub duration_ms: u128,
    pub estimated_input_tokens: u32,
    pub estimated_output_tokens: u32,
    pub estimated_cost_usd: f64,
    pub confirmation_id: String,
    pub redaction_status: String,
    pub provider_request_sent: bool,
    pub credential_accessed: bool,
    pub raw_prompt_persisted: bool,
    pub raw_response_persisted: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
struct ProviderProfileStore {
    version: u32,
    default_profile_id: Option<String>,
    profiles: Vec<ProviderProfileRecord>,
}

#[derive(Debug, Clone, Serialize)]
pub(crate) struct NormalizedProviderProfileInput {
    pub id: String,
    pub display_name: String,
    pub provider_type: ProviderType,
    pub base_url: String,
    pub model: String,
    pub enabled: bool,
    pub api_version: Option<String>,
    pub organization: Option<String>,
    pub single_request_token_limit: u32,
    pub monthly_budget_usd: f64,
    pub replaces_credential: bool,
}

struct ProviderTestFinish<'a> {
    status: &'a str,
    provider_request_sent: bool,
    credential_accessed: bool,
    error_code: Option<String>,
    error_message: Option<String>,
}

struct ProviderPromptFinish {
    status: String,
    provider_request_sent: bool,
    credential_accessed: bool,
    error_code: Option<String>,
    error_message: Option<String>,
    output_text: Option<String>,
}

struct ProviderPromptHttpSuccess {
    status: u16,
    body: String,
}

#[derive(Debug, Error)]
enum ProviderRequestError {
    #[error("provider HTTP transport failed")]
    Transport(#[source] Box<UreqError>),
    #[error("provider response body could not be read")]
    BodyUnreadable(#[source] io::Error),
    #[error("provider response body exceeded the local safety limit")]
    BodyTooLarge,
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub(crate) enum ProviderMutationState {
    #[default]
    NotStarted,
    Applied,
    Partial,
}

struct CredentialCommit {
    target: ProviderCredentialReference,
    staging: ProviderCredentialReference,
    previous_secret: Option<String>,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum CredentialDeleteOutcome {
    Deleted,
    AlreadyAbsent,
}

impl CredentialCommit {
    fn compensate(&self) -> Result<(), ProviderError> {
        #[cfg(test)]
        if take_test_credential_fault(
            &self.target.account,
            TestProviderCredentialFault::Compensation,
        ) {
            return Err(ProviderError::CredentialStorageUnavailable(
                "injected credential compensation failure".to_string(),
            ));
        }
        match self.previous_secret.as_deref() {
            Some(secret) => {
                store_and_verify_secret(&self.target, secret)?;
            }
            None => {
                delete_and_verify_secret_absent(&self.target)?;
            }
        }
        delete_and_verify_secret_absent(&self.staging)?;
        Ok(())
    }

    fn finish(self) -> Result<(), ProviderError> {
        delete_and_verify_secret_absent(&self.staging)?;
        Ok(())
    }
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum TestProviderIoFault {
    SaveStore,
    AppendCallMetadata,
}

#[cfg(test)]
#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub(crate) enum TestProviderCredentialFault {
    Compensation,
    Delete,
    StagingDelete,
    StagingReadback,
    StagingReadbackMismatch,
}

#[cfg(test)]
static TEST_PROVIDER_IO_FAULTS: LazyLock<Mutex<Vec<(PathBuf, TestProviderIoFault)>>> =
    LazyLock::new(|| Mutex::new(Vec::new()));

#[cfg(test)]
static TEST_PROVIDER_CREDENTIAL_FAULTS: LazyLock<
    Mutex<Vec<(String, TestProviderCredentialFault)>>,
> = LazyLock::new(|| Mutex::new(Vec::new()));

#[cfg(test)]
static TEST_PROVIDER_KEYCHAIN: LazyLock<Mutex<HashMap<String, Option<String>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

#[cfg(test)]
pub(crate) fn install_test_provider_io_fault(app_data_dir: &Path, fault: TestProviderIoFault) {
    TEST_PROVIDER_IO_FAULTS
        .lock()
        .expect("lock provider IO faults")
        .push((app_data_dir.to_path_buf(), fault));
}

#[cfg(test)]
pub(crate) fn install_test_provider_credential_fault(
    profile_id: &str,
    fault: TestProviderCredentialFault,
) {
    let account = match fault {
        TestProviderCredentialFault::StagingDelete
        | TestProviderCredentialFault::StagingReadback
        | TestProviderCredentialFault::StagingReadbackMismatch => {
            format!("provider:{profile_id}:staging")
        }
        TestProviderCredentialFault::Compensation | TestProviderCredentialFault::Delete => {
            format!("provider:{profile_id}")
        }
    };
    TEST_PROVIDER_CREDENTIAL_FAULTS
        .lock()
        .expect("lock provider credential faults")
        .push((account, fault));
}

#[cfg(test)]
pub(crate) fn manage_test_provider_credential(profile_id: &str, secret: Option<&str>) {
    TEST_PROVIDER_KEYCHAIN
        .lock()
        .expect("lock provider test keychain")
        .insert(
            format!("provider:{profile_id}"),
            secret.map(ToOwned::to_owned),
        );
}

#[cfg(test)]
fn take_test_provider_io_fault(app_data_dir: &Path, fault: TestProviderIoFault) -> bool {
    let mut faults = TEST_PROVIDER_IO_FAULTS
        .lock()
        .expect("lock provider IO faults");
    let Some(index) = faults
        .iter()
        .position(|(path, candidate)| path == app_data_dir && *candidate == fault)
    else {
        return false;
    };
    faults.swap_remove(index);
    true
}

#[cfg(test)]
fn take_test_credential_fault(account: &str, fault: TestProviderCredentialFault) -> bool {
    let mut faults = TEST_PROVIDER_CREDENTIAL_FAULTS
        .lock()
        .expect("lock provider credential faults");
    let Some(index) = faults
        .iter()
        .position(|(candidate, kind)| candidate == account && *kind == fault)
    else {
        return false;
    };
    faults.swap_remove(index);
    true
}

impl Default for ProviderProfileStore {
    fn default() -> Self {
        Self {
            version: PROFILE_STORE_VERSION,
            default_profile_id: None,
            profiles: Vec::new(),
        }
    }
}

pub fn list_provider_profiles(
    app_data_dir: &Path,
) -> Result<ListProviderProfilesResult, ProviderError> {
    let store = load_store(app_data_dir)?;
    Ok(provider_profile_list(store))
}

pub(crate) fn list_provider_profiles_while_locked(
    owner: &AppMutationLock,
) -> Result<ListProviderProfilesResult, ProviderError> {
    let store = load_store_while_locked(owner)?;
    Ok(provider_profile_list(store))
}

fn provider_profile_list(store: ProviderProfileStore) -> ListProviderProfilesResult {
    ListProviderProfilesResult {
        profiles: store.profiles,
        default_profile_id: store.default_profile_id,
        credential_storage: "keychain".to_string(),
        credential_persistence_allowed: true,
        raw_secrets_returned: false,
    }
}

pub(crate) fn save_provider_profile_while_locked(
    app_data_dir: &Path,
    owner: &AppMutationLock,
    params: SaveProviderProfileParams,
) -> Result<SaveProviderProfileResult, ProviderError> {
    let now = unix_timestamp_millis();
    let mut store = load_store_while_locked(owner)?;
    let normalized = normalize_save_provider_profile_params(&params)?;
    let profile_id = normalized.id.clone();
    let replaces_credential = normalized.replaces_credential;
    let previous_profile = store
        .profiles
        .iter()
        .find(|profile| profile.id == profile_id)
        .cloned();
    let mut credential_reference = if replaces_credential {
        keychain_reference(&profile_id)
    } else {
        previous_profile
            .as_ref()
            .map(|profile| profile.credential_reference.clone())
            .unwrap_or_else(|| keychain_reference(&profile_id))
    };
    let credential_commit = match params.api_key.as_deref().map(str::trim) {
        Some(secret) if !secret.is_empty() => {
            Some(stage_and_commit_secret(&credential_reference, secret)?)
        }
        _ => None,
    };
    let credential_status = match credential_commit.as_ref() {
        Some(_) => available_credential_status("API key stored in the OS credential store."),
        None => previous_profile
            .as_ref()
            .map(|profile| profile.credential_status.clone())
            .unwrap_or_else(|| existing_credential_status(&credential_reference)),
    };
    credential_reference.secret_persisted = credential_status.secret_available;
    let previous_created_at = previous_profile
        .as_ref()
        .map(|profile| profile.created_at)
        .unwrap_or(now);
    let profile = ProviderProfileRecord {
        id: profile_id.clone(),
        display_name: normalized.display_name,
        provider_type: normalized.provider_type,
        base_url: normalized.base_url,
        model: normalized.model,
        enabled: normalized.enabled,
        api_version: normalized.api_version,
        organization: normalized.organization,
        single_request_token_limit: normalized.single_request_token_limit,
        monthly_budget_usd: normalized.monthly_budget_usd,
        credential_reference,
        credential_status: credential_status.clone(),
        created_at: previous_created_at,
        updated_at: now,
    };

    store.profiles.retain(|existing| existing.id != profile_id);
    store.profiles.push(profile.clone());
    store.profiles.sort_by(|left, right| left.id.cmp(&right.id));
    if profile.enabled {
        store.default_profile_id = Some(profile.id.clone());
    } else if store.default_profile_id.is_none() {
        store.default_profile_id = store.profiles.first().map(|profile| profile.id.clone());
    }
    if let Err(error) = save_store(app_data_dir, owner, &store) {
        if let Some(commit) = credential_commit.as_ref() {
            return match commit.compensate() {
                Ok(()) => Ok(SaveProviderProfileResult {
                    profile,
                    credential_status,
                    profile_persisted: false,
                    credential_effect: "restored_after_profile_write_failure".to_string(),
                    error_code: Some("provider_profile_write_failed".to_string()),
                    error_message: Some(
                        "Provider profile was not saved; the previous credential state was restored."
                            .to_string(),
                    ),
                    raw_secret_returned: false,
                    operation_state: ProviderMutationState::NotStarted,
                }),
                Err(_) => Ok(SaveProviderProfileResult {
                    profile,
                    credential_status,
                    profile_persisted: false,
                    credential_effect: "unknown_after_compensation_failure".to_string(),
                    error_code: Some("credential_compensation_failed".to_string()),
                    error_message: Some(
                        "Provider profile was not saved and credential restoration could not be verified."
                            .to_string(),
                    ),
                    raw_secret_returned: false,
                    operation_state: ProviderMutationState::Partial,
                }),
            };
        }
        return Err(error);
    }
    if let Some(commit) = credential_commit {
        if commit.finish().is_err() {
            return Ok(SaveProviderProfileResult {
                profile,
                credential_status,
                profile_persisted: true,
                credential_effect: "target_verified_staging_cleanup_unknown".to_string(),
                error_code: Some("credential_staging_cleanup_failed".to_string()),
                error_message: Some(
                    "Provider profile and target credential were saved, but staging credential cleanup could not be verified."
                        .to_string(),
                ),
                raw_secret_returned: false,
                operation_state: ProviderMutationState::Partial,
            });
        }
    }

    Ok(SaveProviderProfileResult {
        profile,
        credential_status,
        profile_persisted: true,
        credential_effect: if replaces_credential {
            "stored_and_verified".to_string()
        } else {
            "preserved".to_string()
        },
        error_code: None,
        error_message: None,
        raw_secret_returned: false,
        operation_state: ProviderMutationState::Applied,
    })
}

pub(crate) fn normalize_save_provider_profile_params(
    params: &SaveProviderProfileParams,
) -> Result<NormalizedProviderProfileInput, ProviderError> {
    let profile_id = params
        .id
        .as_deref()
        .map(sanitize_profile_id)
        .filter(|id| !id.is_empty())
        .unwrap_or_else(|| sanitize_profile_id(&params.display_name));
    if profile_id.is_empty() {
        return Err(ProviderError::InvalidProfile(
            "profile id or display name must contain an ASCII letter or digit".to_string(),
        ));
    }
    let provider_type = params.provider_type;
    Ok(NormalizedProviderProfileInput {
        id: profile_id,
        display_name: require_non_empty("display_name", &params.display_name)?,
        provider_type,
        base_url: validate_base_url(&params.base_url)?,
        model: require_non_empty("model", &params.model)?,
        enabled: params.enabled,
        api_version: params
            .api_version
            .clone()
            .and_then(non_empty_string)
            .or_else(|| default_api_version(provider_type)),
        organization: params.organization.clone().and_then(non_empty_string),
        single_request_token_limit: params
            .single_request_token_limit
            .unwrap_or(DEFAULT_SINGLE_REQUEST_TOKEN_LIMIT)
            .clamp(1, 200_000),
        monthly_budget_usd: params
            .monthly_budget_usd
            .unwrap_or(DEFAULT_MONTHLY_BUDGET_USD)
            .clamp(0.0, 10_000.0),
        replaces_credential: params
            .api_key
            .as_deref()
            .map(str::trim)
            .is_some_and(|secret| !secret.is_empty()),
    })
}

pub(crate) fn delete_provider_profile_while_locked(
    app_data_dir: &Path,
    owner: &AppMutationLock,
    params: DeleteProviderProfileParams,
) -> Result<DeleteProviderProfileResult, ProviderError> {
    let mut store = load_store_while_locked(owner)?;
    let Some(profile) = store
        .profiles
        .iter()
        .find(|profile| profile.id == params.profile_id)
        .cloned()
    else {
        return Ok(DeleteProviderProfileResult {
            deleted_profile_id: params.profile_id,
            profile_deleted: false,
            credential_deleted: false,
            credential_effect: "not_started".to_string(),
            error_code: Some("provider_profile_not_found".to_string()),
            error_message: Some("Provider profile no longer exists.".to_string()),
            raw_secret_returned: false,
            operation_state: ProviderMutationState::NotStarted,
        });
    };
    store.profiles.retain(|existing| existing.id != profile.id);
    if store.default_profile_id.as_deref() == Some(profile.id.as_str()) {
        store.default_profile_id = store.profiles.first().map(|profile| profile.id.clone());
    }
    save_store(app_data_dir, owner, &store)?;
    let (credential_deleted, credential_effect, error_code, error_message, operation_state) =
        if params.delete_credential {
            match delete_secret(&profile.credential_reference) {
                Ok(CredentialDeleteOutcome::Deleted) => (
                    true,
                    "deleted_and_verified".to_string(),
                    None,
                    None,
                    ProviderMutationState::Applied,
                ),
                Ok(CredentialDeleteOutcome::AlreadyAbsent) => (
                    false,
                    "absence_verified".to_string(),
                    None,
                    None,
                    ProviderMutationState::Applied,
                ),
                Err(_) => (
                    false,
                    "unknown_after_delete_failure".to_string(),
                    Some("credential_delete_unverified".to_string()),
                    Some(
                        "Provider profile was deleted, but credential deletion could not be verified."
                            .to_string(),
                    ),
                    ProviderMutationState::Partial,
                ),
            }
        } else {
            (
                false,
                "preserved".to_string(),
                None,
                None,
                ProviderMutationState::Applied,
            )
        };

    Ok(DeleteProviderProfileResult {
        deleted_profile_id: profile.id,
        profile_deleted: true,
        credential_deleted,
        credential_effect,
        error_code,
        error_message,
        raw_secret_returned: false,
        operation_state,
    })
}

pub(crate) fn test_provider_connection_while_locked(
    app_data_dir: &Path,
    owner: &AppMutationLock,
    params: TestProviderConnectionParams,
) -> Result<TestProviderConnectionResult, ProviderError> {
    let store = load_store_while_locked(owner)?;
    let profile = store
        .profiles
        .iter()
        .find(|profile| profile.id == params.profile_id)
        .cloned()
        .ok_or_else(|| ProviderError::ProfileNotFound(params.profile_id.clone()))?;
    let budget = budget_status(&profile);
    let started = Instant::now();

    if !profile.enabled {
        return finish_test(
            app_data_dir,
            owner,
            &profile,
            &params.confirmation_id,
            started,
            budget,
            ProviderTestFinish {
                status: "blocked",
                provider_request_sent: false,
                credential_accessed: false,
                error_code: Some("profile_disabled".to_string()),
                error_message: Some(
                    "Provider profile is disabled; no request was sent.".to_string(),
                ),
            },
        );
    }
    if params.confirmation_id.trim().is_empty() {
        return finish_test(
            app_data_dir,
            owner,
            &profile,
            &params.confirmation_id,
            started,
            budget,
            ProviderTestFinish {
                status: "blocked",
                provider_request_sent: false,
                credential_accessed: false,
                error_code: Some("missing_confirmation".to_string()),
                error_message: Some(
                    "Explicit confirmation id is required before a provider test.".to_string(),
                ),
            },
        );
    }
    if budget.state != "ok" {
        return finish_test(
            app_data_dir,
            owner,
            &profile,
            &params.confirmation_id,
            started,
            budget,
            ProviderTestFinish {
                status: "blocked",
                provider_request_sent: false,
                credential_accessed: false,
                error_code: Some("budget_blocked".to_string()),
                error_message: Some("Provider budget settings block the test request.".to_string()),
            },
        );
    }

    let secret = match load_secret(&profile.credential_reference) {
        Ok(secret) => secret,
        Err(error) => {
            return finish_test(
                app_data_dir,
                owner,
                &profile,
                &params.confirmation_id,
                started,
                budget,
                ProviderTestFinish {
                    status: "blocked",
                    provider_request_sent: false,
                    credential_accessed: false,
                    error_code: Some("credential_unavailable".to_string()),
                    error_message: Some(error.to_string()),
                },
            );
        }
    };
    let timeout = Duration::from_millis(params.timeout_ms.unwrap_or(4_000).clamp(250, 15_000));
    let call_result = send_test_request(&profile, &secret, timeout);
    drop(secret);

    match call_result {
        Ok(status) if (200..300).contains(&status) => finish_test(
            app_data_dir,
            owner,
            &profile,
            &params.confirmation_id,
            started,
            budget,
            ProviderTestFinish {
                status: "succeeded",
                provider_request_sent: true,
                credential_accessed: true,
                error_code: None,
                error_message: None,
            },
        ),
        Ok(status) => finish_test(
            app_data_dir,
            owner,
            &profile,
            &params.confirmation_id,
            started,
            budget,
            ProviderTestFinish {
                status: "failed",
                provider_request_sent: true,
                credential_accessed: true,
                error_code: Some(format!("http_{status}")),
                error_message: Some("Provider returned a non-success HTTP status.".to_string()),
            },
        ),
        Err(error) => finish_test(
            app_data_dir,
            owner,
            &profile,
            &params.confirmation_id,
            started,
            budget,
            ProviderTestFinish {
                status: "failed",
                provider_request_sent: true,
                credential_accessed: true,
                error_code: Some("network_error".to_string()),
                error_message: Some(redact_error(&error)),
            },
        ),
    }
}

pub fn send_provider_prompt(
    app_data_dir: &Path,
    params: SendProviderPromptParams,
) -> Result<SendProviderPromptResult, ProviderError> {
    let owner = lock_or_create_app_mutations(app_data_dir)?;
    send_provider_prompt_while_locked(app_data_dir, &owner, params)
}

pub(crate) fn send_provider_prompt_while_locked(
    app_data_dir: &Path,
    owner: &AppMutationLock,
    params: SendProviderPromptParams,
) -> Result<SendProviderPromptResult, ProviderError> {
    let store = load_store_while_locked(owner)?;
    let profile = store
        .profiles
        .iter()
        .find(|profile| profile.id == params.profile_id)
        .cloned()
        .ok_or_else(|| ProviderError::ProfileNotFound(params.profile_id.clone()))?;
    let destination_host = destination_host(&profile.base_url);
    let started = Instant::now();
    let estimated_total_tokens = params
        .estimated_input_tokens
        .saturating_add(params.estimated_output_tokens);

    if !profile.enabled {
        return finish_prompt(
            app_data_dir,
            owner,
            &profile,
            &destination_host,
            &params,
            started,
            ProviderPromptFinish {
                status: "blocked".to_string(),
                provider_request_sent: false,
                credential_accessed: false,
                error_code: Some("profile_disabled".to_string()),
                error_message: Some(
                    "Provider profile is disabled; no request was sent.".to_string(),
                ),
                output_text: None,
            },
        );
    }
    if params.confirmation_id.trim().is_empty() {
        return finish_prompt(
            app_data_dir,
            owner,
            &profile,
            &destination_host,
            &params,
            started,
            ProviderPromptFinish {
                status: "blocked".to_string(),
                provider_request_sent: false,
                credential_accessed: false,
                error_code: Some("missing_confirmation".to_string()),
                error_message: Some(
                    "Explicit confirmation id is required before a provider prompt request."
                        .to_string(),
                ),
                output_text: None,
            },
        );
    }
    if params.prompt.trim().is_empty() {
        return finish_prompt(
            app_data_dir,
            owner,
            &profile,
            &destination_host,
            &params,
            started,
            ProviderPromptFinish {
                status: "blocked".to_string(),
                provider_request_sent: false,
                credential_accessed: false,
                error_code: Some("empty_prompt".to_string()),
                error_message: Some("Redacted prompt is empty; no request was sent.".to_string()),
                output_text: None,
            },
        );
    }
    if profile.single_request_token_limit < estimated_total_tokens {
        return finish_prompt(
            app_data_dir,
            owner,
            &profile,
            &destination_host,
            &params,
            started,
            ProviderPromptFinish {
                status: "blocked".to_string(),
                provider_request_sent: false,
                credential_accessed: false,
                error_code: Some("budget_blocked".to_string()),
                error_message: Some(
                    "Single request token limit is lower than the prompt estimate.".to_string(),
                ),
                output_text: None,
            },
        );
    }
    if profile.monthly_budget_usd <= 0.0 {
        return finish_prompt(
            app_data_dir,
            owner,
            &profile,
            &destination_host,
            &params,
            started,
            ProviderPromptFinish {
                status: "blocked".to_string(),
                provider_request_sent: false,
                credential_accessed: false,
                error_code: Some("budget_blocked".to_string()),
                error_message: Some(
                    "Monthly provider budget is 0; provider requests are disabled.".to_string(),
                ),
                output_text: None,
            },
        );
    }

    let secret = match load_secret(&profile.credential_reference) {
        Ok(secret) => secret,
        Err(error) => {
            return finish_prompt(
                app_data_dir,
                owner,
                &profile,
                &destination_host,
                &params,
                started,
                ProviderPromptFinish {
                    status: "blocked".to_string(),
                    provider_request_sent: false,
                    credential_accessed: false,
                    error_code: Some("credential_unavailable".to_string()),
                    error_message: Some(error.to_string()),
                    output_text: None,
                },
            );
        }
    };
    let timeout = Duration::from_millis(params.timeout_ms.unwrap_or(600_000).clamp(250, 600_000));
    let call_result = send_prompt_request(&profile, &secret, &params.prompt, &params, timeout);
    drop(secret);

    match call_result {
        Ok(success) if (200..300).contains(&success.status) => {
            let output_text = extract_output_text(profile.provider_type, &success.body);
            let response_validation =
                validate_prompt_business_output(&params.action_type, output_text.as_deref());
            let (status, error_code, error_message) = match response_validation {
                Ok(()) => ("succeeded".to_string(), None, None),
                Err(message) => (
                    "parse_failed".to_string(),
                    Some("response_schema_invalid".to_string()),
                    Some(message),
                ),
            };
            finish_prompt(
                app_data_dir,
                owner,
                &profile,
                &destination_host,
                &params,
                started,
                ProviderPromptFinish {
                    status,
                    provider_request_sent: true,
                    credential_accessed: true,
                    error_code,
                    error_message,
                    output_text,
                },
            )
        }
        Ok(success) => finish_prompt(
            app_data_dir,
            owner,
            &profile,
            &destination_host,
            &params,
            started,
            ProviderPromptFinish {
                status: "failed".to_string(),
                provider_request_sent: true,
                credential_accessed: true,
                error_code: Some(format!("http_{}", success.status)),
                error_message: Some("Provider returned a non-success HTTP status.".to_string()),
                output_text: None,
            },
        ),
        Err(error) => finish_prompt(
            app_data_dir,
            owner,
            &profile,
            &destination_host,
            &params,
            started,
            ProviderPromptFinish {
                status: "failed".to_string(),
                provider_request_sent: true,
                credential_accessed: true,
                error_code: Some("network_error".to_string()),
                error_message: Some(redact_error(&error)),
                output_text: None,
            },
        ),
    }
}

pub fn provider_profiles_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir.join("llm").join("provider-profiles.json")
}

pub fn provider_call_metadata_path(app_data_dir: &Path) -> PathBuf {
    app_data_dir
        .join("llm")
        .join("provider-call-metadata.jsonl")
}

pub(crate) fn provider_profiles_revision(app_data_dir: &Path) -> Result<String, ProviderError> {
    let owner = match lock_app_mutations(app_data_dir) {
        Ok(owner) => owner,
        Err(CommandError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(digest_revision("provider-profiles:missing", &[]))
        }
        Err(error) => return Err(error.into()),
    };
    provider_profiles_revision_while_locked(&owner)
}

pub(crate) fn provider_profiles_revision_while_locked(
    owner: &AppMutationLock,
) -> Result<String, ProviderError> {
    file_revision_while_locked(
        owner,
        Path::new(PROVIDER_PROFILE_STORE_RELATIVE_PATH),
        "provider-profiles",
        PROVIDER_PROFILE_STORE_MAX_BYTES,
    )
}

pub(crate) fn provider_call_metadata_revision(
    app_data_dir: &Path,
) -> Result<String, ProviderError> {
    let owner = match lock_app_mutations(app_data_dir) {
        Ok(owner) => owner,
        Err(CommandError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(digest_revision("provider-call-metadata:missing", &[]))
        }
        Err(error) => return Err(error.into()),
    };
    provider_call_metadata_revision_while_locked(&owner)
}

pub(crate) fn provider_call_metadata_revision_while_locked(
    owner: &AppMutationLock,
) -> Result<String, ProviderError> {
    file_revision_while_locked(
        owner,
        Path::new(PROVIDER_CALL_METADATA_RELATIVE_PATH),
        "provider-call-metadata",
        PROVIDER_CALL_METADATA_MAX_BYTES,
    )
}

pub(crate) fn provider_profile_nonsecret_revision(
    profile: &ProviderProfileRecord,
) -> Result<String, ProviderError> {
    let content = serde_json::to_vec(profile)?;
    Ok(digest_revision("provider-profile", &content))
}

pub(crate) fn normalized_provider_input_revision(
    input: &NormalizedProviderProfileInput,
) -> Result<String, ProviderError> {
    let content = serde_json::to_vec(input)?;
    Ok(digest_revision("provider-profile-input", &content))
}

pub(crate) fn verify_provider_credential_matches(
    profile_id: &str,
    expected_secret: &str,
) -> Result<String, ProviderError> {
    let reference = keychain_reference(profile_id);
    let stored = load_secret(&reference)?;
    if expected_secret.is_empty()
        || !constant_time_secret_eq(stored.as_bytes(), expected_secret.as_bytes())
    {
        return Err(ProviderError::CredentialStorageUnavailable(
            "saved API key read-back did not match the confirmed credential input".to_string(),
        ));
    }
    Ok(provider_credential_state_revision(
        profile_id,
        "present-and-matched",
    ))
}

pub(crate) fn verify_provider_credential_absent(profile_id: &str) -> Result<String, ProviderError> {
    let reference = keychain_reference(profile_id);
    #[cfg(test)]
    {
        let keychain = TEST_PROVIDER_KEYCHAIN
            .lock()
            .expect("lock provider test keychain");
        if let Some(secret) = keychain.get(&reference.account) {
            return if secret.is_none() {
                Ok(provider_credential_state_revision(
                    profile_id,
                    "absence-verified",
                ))
            } else {
                Err(ProviderError::CredentialStorageUnavailable(
                    "credential still exists after confirmed deletion".to_string(),
                ))
            };
        }
    }
    let entry = Entry::new(&reference.service, &reference.account)
        .map_err(|error| ProviderError::CredentialStorageUnavailable(error.to_string()))?;
    match entry.get_password() {
        Err(keyring::Error::NoEntry) => Ok(provider_credential_state_revision(
            profile_id,
            "absence-verified",
        )),
        Ok(_) => Err(ProviderError::CredentialStorageUnavailable(
            "credential still exists after confirmed deletion".to_string(),
        )),
        Err(error) => Err(ProviderError::CredentialStorageUnavailable(
            error.to_string(),
        )),
    }
}

#[cfg(test)]
pub(crate) fn verify_provider_staging_credential_absent(
    profile_id: &str,
) -> Result<(), ProviderError> {
    let reference = staging_keychain_reference(&keychain_reference(profile_id));
    match load_optional_secret(&reference)? {
        None => Ok(()),
        Some(_) => Err(ProviderError::CredentialStorageUnavailable(
            "staging credential still exists".to_string(),
        )),
    }
}

fn provider_credential_state_revision(profile_id: &str, state: &str) -> String {
    digest_revision(
        "provider-credential-state",
        format!("{profile_id}\0{state}").as_bytes(),
    )
}

pub fn default_token_limit() -> u32 {
    DEFAULT_SINGLE_REQUEST_TOKEN_LIMIT
}

pub fn default_monthly_budget_usd() -> f64 {
    DEFAULT_MONTHLY_BUDGET_USD
}

pub fn estimate_prompt_cost_usd(provider_type: ProviderType, tokens: u32) -> f64 {
    estimated_provider_cost(provider_type, tokens)
}

fn finish_test(
    app_data_dir: &Path,
    owner: &AppMutationLock,
    profile: &ProviderProfileRecord,
    confirmation_id: &str,
    started: Instant,
    budget: ProviderBudgetStatus,
    finish: ProviderTestFinish<'_>,
) -> Result<TestProviderConnectionResult, ProviderError> {
    let destination_host = destination_host(&profile.base_url);
    let mut audit = ProviderCallMetadata {
        timestamp: unix_timestamp_millis(),
        action_type: "test_connection".to_string(),
        profile_id: profile.id.clone(),
        provider_type: profile.provider_type,
        model: profile.model.clone(),
        destination_host: destination_host.clone(),
        status: finish.status.to_string(),
        error_code: finish.error_code.clone(),
        error_message: finish.error_message.clone(),
        duration_ms: started.elapsed().as_millis(),
        estimated_input_tokens: TEST_INPUT_TOKEN_ESTIMATE,
        estimated_output_tokens: TEST_OUTPUT_TOKEN_ESTIMATE,
        estimated_cost_usd: budget.estimated_test_cost_usd,
        confirmation_id: confirmation_id.to_string(),
        redaction_status: "metadata-only-no-raw-prompt-or-response".to_string(),
        provider_request_sent: finish.provider_request_sent,
        credential_accessed: finish.credential_accessed,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
    };
    let local_metadata_persisted = append_call_metadata(app_data_dir, owner, &audit).is_ok();
    if !local_metadata_persisted {
        audit.status = if finish.provider_request_sent {
            "partial".to_string()
        } else {
            finish.status.to_string()
        };
        audit.error_code = Some("local_metadata_write_failed".to_string());
        audit.error_message =
            Some("Provider result returned, but local metadata could not be recorded.".to_string());
    }
    Ok(TestProviderConnectionResult {
        profile_id: profile.id.clone(),
        provider_type: profile.provider_type,
        model: profile.model.clone(),
        destination_host,
        status: audit.status.clone(),
        provider_request_sent: finish.provider_request_sent,
        credential_accessed: finish.credential_accessed,
        duration_ms: audit.duration_ms,
        error_code: audit.error_code.clone(),
        error_message: audit.error_message.clone(),
        budget,
        audit,
        local_metadata_persisted,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_secret_returned: false,
    })
}

fn finish_prompt(
    app_data_dir: &Path,
    owner: &AppMutationLock,
    profile: &ProviderProfileRecord,
    destination_host: &str,
    params: &SendProviderPromptParams,
    started: Instant,
    finish: ProviderPromptFinish,
) -> Result<SendProviderPromptResult, ProviderError> {
    let mut audit = ProviderCallMetadata {
        timestamp: unix_timestamp_millis(),
        action_type: params.action_type.clone(),
        profile_id: profile.id.clone(),
        provider_type: profile.provider_type,
        model: profile.model.clone(),
        destination_host: destination_host.to_string(),
        status: finish.status.clone(),
        error_code: finish.error_code.clone(),
        error_message: finish.error_message.clone(),
        duration_ms: started.elapsed().as_millis(),
        estimated_input_tokens: params.estimated_input_tokens,
        estimated_output_tokens: params.estimated_output_tokens,
        estimated_cost_usd: params.estimated_cost_usd,
        confirmation_id: params.confirmation_id.clone(),
        redaction_status: params.redaction_status.clone(),
        provider_request_sent: finish.provider_request_sent,
        credential_accessed: finish.credential_accessed,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
    };
    let local_metadata_persisted = append_call_metadata(app_data_dir, owner, &audit).is_ok();
    if !local_metadata_persisted {
        audit.status = if finish.provider_request_sent {
            "partial".to_string()
        } else {
            finish.status.clone()
        };
        audit.error_code = Some("local_metadata_write_failed".to_string());
        audit.error_message =
            Some("Provider result returned, but local metadata could not be recorded.".to_string());
    }
    Ok(SendProviderPromptResult {
        profile_id: profile.id.clone(),
        provider_type: profile.provider_type,
        model: profile.model.clone(),
        destination_host: destination_host.to_string(),
        status: audit.status.clone(),
        provider_request_sent: finish.provider_request_sent,
        credential_accessed: finish.credential_accessed,
        duration_ms: audit.duration_ms,
        error_code: audit.error_code.clone(),
        error_message: audit.error_message.clone(),
        output_text: finish.output_text,
        audit,
        local_metadata_persisted,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_secret_returned: false,
    })
}

fn load_store(app_data_dir: &Path) -> Result<ProviderProfileStore, ProviderError> {
    let owner = match lock_app_mutations(app_data_dir) {
        Ok(owner) => owner,
        Err(CommandError::Io(error)) if error.kind() == io::ErrorKind::NotFound => {
            return Ok(ProviderProfileStore::default())
        }
        Err(error) => return Err(error.into()),
    };
    load_store_while_locked(&owner)
}

fn load_store_while_locked(owner: &AppMutationLock) -> Result<ProviderProfileStore, ProviderError> {
    let Some(content) = owner.owner_fs().read_bounded_regular_file(
        Path::new(PROVIDER_PROFILE_STORE_RELATIVE_PATH),
        PROVIDER_PROFILE_STORE_MAX_BYTES,
        "provider profile store",
    )?
    else {
        return Ok(ProviderProfileStore::default());
    };
    let mut store: ProviderProfileStore = serde_json::from_slice(&content)?;
    for profile in &mut store.profiles {
        profile.created_at = normalize_epoch_millis(profile.created_at);
        profile.updated_at = normalize_epoch_millis(profile.updated_at);
    }
    Ok(store)
}

fn save_store(
    _app_data_dir: &Path,
    owner: &AppMutationLock,
    store: &ProviderProfileStore,
) -> Result<(), ProviderError> {
    #[cfg(test)]
    if take_test_provider_io_fault(_app_data_dir, TestProviderIoFault::SaveStore) {
        return Err(io::Error::other("injected provider profile write failure").into());
    }
    let mut content = serde_json::to_vec_pretty(store)?;
    content.push(b'\n');
    owner.owner_fs().ensure_directory_all(Path::new("llm"))?;
    owner.owner_fs().atomic_replace_private_file(
        Path::new(PROVIDER_PROFILE_STORE_RELATIVE_PATH),
        &content,
        "provider-profiles",
    )?;
    Ok(())
}

fn append_call_metadata(
    _app_data_dir: &Path,
    owner: &AppMutationLock,
    metadata: &ProviderCallMetadata,
) -> Result<(), ProviderError> {
    #[cfg(test)]
    if take_test_provider_io_fault(_app_data_dir, TestProviderIoFault::AppendCallMetadata) {
        return Err(io::Error::other("injected provider metadata write failure").into());
    }
    let mut line = serde_json::to_vec(metadata)?;
    line.push(b'\n');
    owner.owner_fs().ensure_directory_all(Path::new("llm"))?;
    owner.owner_fs().append_private_file(
        Path::new(PROVIDER_CALL_METADATA_RELATIVE_PATH),
        &line,
        PROVIDER_CALL_METADATA_MAX_BYTES,
        "provider call metadata",
    )?;
    Ok(())
}

fn send_test_request(
    profile: &ProviderProfileRecord,
    secret: &str,
    timeout: Duration,
) -> Result<u16, ProviderRequestError> {
    let url = test_endpoint_url(profile);
    let agent = provider_http_agent();
    let mut request = agent
        .post(&url)
        .timeout(timeout)
        .set("content-type", "application/json");
    if let Some(org) = profile.organization.as_deref() {
        request = request.set("openai-organization", org);
    }
    match profile.provider_type {
        ProviderType::OpenAiCompatible => {
            request = request.set("authorization", &format!("Bearer {secret}"));
            let response = send_json_without_redirect(
                request,
                json!({
                    "model": profile.model,
                    "messages": [{"role": "user", "content": "connection test"}],
                    "max_tokens": 1,
                    "temperature": 0
                }),
            )?;
            Ok(response.status())
        }
        ProviderType::ClaudeCompatible => {
            request = request.set("x-api-key", secret).set(
                "anthropic-version",
                profile.api_version.as_deref().unwrap_or("2023-06-01"),
            );
            let response = send_json_without_redirect(
                request,
                json!({
                    "model": profile.model,
                    "messages": [{"role": "user", "content": "connection test"}],
                    "max_tokens": 1
                }),
            )?;
            Ok(response.status())
        }
    }
}

fn send_prompt_request(
    profile: &ProviderProfileRecord,
    secret: &str,
    prompt: &str,
    params: &SendProviderPromptParams,
    timeout: Duration,
) -> Result<ProviderPromptHttpSuccess, ProviderRequestError> {
    let url = test_endpoint_url(profile);
    let max_tokens = params.estimated_output_tokens.clamp(1, 8_000);
    let agent = provider_http_agent();
    let mut request = agent
        .post(&url)
        .timeout(timeout)
        .set("content-type", "application/json");
    if let Some(org) = profile.organization.as_deref() {
        request = request.set("openai-organization", org);
    }
    let response = match profile.provider_type {
        ProviderType::OpenAiCompatible => {
            request = request.set("authorization", &format!("Bearer {secret}"));
            send_json_without_redirect(
                request,
                json!({
                    "model": profile.model,
                    "messages": [
                        {
                            "role": "system",
                            "content": "You are reviewing AI agent skills. Return draft-only guidance; do not claim to write files, execute scripts, mutate configuration, or store credentials."
                        },
                        {"role": "user", "content": prompt}
                    ],
                    "max_tokens": max_tokens,
                    "temperature": 0.2
                }),
            )?
        }
        ProviderType::ClaudeCompatible => {
            request = request.set("x-api-key", secret).set(
                "anthropic-version",
                profile.api_version.as_deref().unwrap_or("2023-06-01"),
            );
            send_json_without_redirect(
                request,
                json!({
                    "model": profile.model,
                    "system": "You are reviewing AI agent skills. Return draft-only guidance; do not claim to write files, execute scripts, mutate configuration, or store credentials.",
                    "messages": [{"role": "user", "content": prompt}],
                    "max_tokens": max_tokens
                }),
            )?
        }
    };
    let status = response.status();
    let mut body = Vec::new();
    let mut reader = response
        .into_reader()
        .take((MAX_PROVIDER_RESPONSE_BYTES + 1) as u64);
    reader
        .read_to_end(&mut body)
        .map_err(ProviderRequestError::BodyUnreadable)?;
    if body.len() > MAX_PROVIDER_RESPONSE_BYTES {
        return Err(ProviderRequestError::BodyTooLarge);
    }
    let body = String::from_utf8(body).map_err(|error| {
        ProviderRequestError::BodyUnreadable(io::Error::new(io::ErrorKind::InvalidData, error))
    })?;
    Ok(ProviderPromptHttpSuccess { status, body })
}

fn provider_http_agent() -> ureq::Agent {
    // Provider destinations are explicitly previewed. Following a redirect
    // would make the real destination differ from that preview and, for
    // non-standard auth headers such as x-api-key, could disclose a credential
    // to another host.
    ureq::builder().redirects(0).build()
}

fn send_json_without_redirect(
    request: ureq::Request,
    body: Value,
) -> Result<ureq::Response, ProviderRequestError> {
    match request.send_json(body) {
        Ok(response) => Ok(response),
        Err(UreqError::Status(_, response)) => Ok(response),
        Err(error) => Err(ProviderRequestError::Transport(Box::new(error))),
    }
}

fn extract_output_text(provider_type: ProviderType, body: &str) -> Option<String> {
    let value: Value = serde_json::from_str(body).ok()?;
    match provider_type {
        ProviderType::OpenAiCompatible => value
            .pointer("/choices/0/message/content")
            .and_then(Value::as_str)
            .map(str::trim)
            .filter(|text| !text.is_empty())
            .map(ToOwned::to_owned),
        ProviderType::ClaudeCompatible => {
            value
                .get("content")
                .and_then(Value::as_array)
                .and_then(|items| {
                    let text = items
                        .iter()
                        .filter_map(|item| item.get("text").and_then(Value::as_str))
                        .collect::<Vec<_>>()
                        .join("\n");
                    if text.trim().is_empty() {
                        None
                    } else {
                        Some(text.trim().to_string())
                    }
                })
        }
    }
}

fn test_endpoint_url(profile: &ProviderProfileRecord) -> String {
    let trimmed = profile.base_url.trim_end_matches('/');
    let path = match profile.provider_type {
        ProviderType::OpenAiCompatible => {
            if trimmed.ends_with("/chat/completions") {
                ""
            } else {
                "/chat/completions"
            }
        }
        ProviderType::ClaudeCompatible => {
            if trimmed.ends_with("/v1/messages") {
                ""
            } else {
                "/v1/messages"
            }
        }
    };
    format!("{trimmed}{path}")
}

fn keychain_reference(profile_id: &str) -> ProviderCredentialReference {
    ProviderCredentialReference {
        storage: "keychain".to_string(),
        service: KEYCHAIN_SERVICE.to_string(),
        account: format!("provider:{profile_id}"),
        secret_persisted: false,
    }
}

fn staging_keychain_reference(target: &ProviderCredentialReference) -> ProviderCredentialReference {
    ProviderCredentialReference {
        storage: target.storage.clone(),
        service: target.service.clone(),
        account: format!("{}:staging", target.account),
        secret_persisted: false,
    }
}

fn stage_and_commit_secret(
    target: &ProviderCredentialReference,
    secret: &str,
) -> Result<CredentialCommit, ProviderError> {
    let previous_secret = load_optional_secret(target)?.filter(|value| !value.is_empty());
    let staging = staging_keychain_reference(target);
    stage_and_verify_secret(&staging, secret)?;
    if let Err(error) = store_and_verify_secret(target, secret) {
        let target_restored = match previous_secret.as_deref() {
            Some(previous) => store_and_verify_secret(target, previous).is_ok(),
            None => delete_and_verify_secret_absent(target).is_ok(),
        };
        let staging_removed = delete_and_verify_secret_absent(&staging).is_ok();
        if !target_restored || !staging_removed {
            return Err(ProviderError::CredentialMutationPartial(
                "target restoration or staging cleanup failed after the credential update could not be verified"
                    .to_string(),
            ));
        }
        return Err(error);
    }
    Ok(CredentialCommit {
        target: target.clone(),
        staging,
        previous_secret,
    })
}

fn stage_and_verify_secret(
    staging: &ProviderCredentialReference,
    secret: &str,
) -> Result<(), ProviderError> {
    // The staging account is fixed per provider and is protected by the
    // provider-action lock. Remove any crash residue before placing the new
    // candidate secret there.
    delete_and_verify_secret_absent(staging)?;
    let verification = store_secret(staging, secret).and_then(|()| {
        let stored = load_secret(staging).map_err(|error| {
            ProviderError::CredentialStorageUnavailable(format!(
                "staged API key could not be read back from the OS credential store: {error}"
            ))
        })?;
        if constant_time_secret_eq(stored.as_bytes(), secret.as_bytes()) && !stored.is_empty() {
            Ok(())
        } else {
            Err(ProviderError::CredentialStorageUnavailable(
                "staged API key did not match after OS credential-store read-back".to_string(),
            ))
        }
    });
    if let Err(error) = verification {
        return match delete_and_verify_secret_absent(staging) {
            Ok(_) => Err(error),
            Err(_) => Err(ProviderError::CredentialMutationPartial(
                "staged API key could not be verified and its cleanup could not be semantically confirmed"
                    .to_string(),
            )),
        };
    }
    Ok(())
}

fn store_secret(
    reference: &ProviderCredentialReference,
    secret: &str,
) -> Result<(), ProviderError> {
    #[cfg(test)]
    {
        let mut keychain = TEST_PROVIDER_KEYCHAIN
            .lock()
            .expect("lock provider test keychain");
        let target_account = reference
            .account
            .strip_suffix(":staging")
            .unwrap_or(&reference.account);
        if keychain.contains_key(&reference.account) || keychain.contains_key(target_account) {
            keychain.insert(reference.account.clone(), Some(secret.to_string()));
            return Ok(());
        }
    }
    let entry = Entry::new(&reference.service, &reference.account)
        .map_err(|error| ProviderError::CredentialStorageUnavailable(error.to_string()))?;
    entry
        .set_password(secret)
        .map_err(|error| ProviderError::CredentialStorageUnavailable(error.to_string()))
}

fn store_and_verify_secret(
    reference: &ProviderCredentialReference,
    secret: &str,
) -> Result<(), ProviderError> {
    store_secret(reference, secret)?;
    let stored = load_secret(reference).map_err(|error| {
        ProviderError::CredentialStorageUnavailable(format!(
            "saved API key could not be read back from the OS credential store: {error}"
        ))
    })?;
    if stored == secret && !stored.is_empty() {
        Ok(())
    } else {
        Err(ProviderError::CredentialStorageUnavailable(
            "saved API key could not be verified in the OS credential store".to_string(),
        ))
    }
}

fn load_secret(reference: &ProviderCredentialReference) -> Result<String, ProviderError> {
    load_optional_secret(reference)?.ok_or_else(|| {
        ProviderError::CredentialStorageUnavailable(
            "No matching entry found in secure storage".to_string(),
        )
    })
}

fn load_optional_secret(
    reference: &ProviderCredentialReference,
) -> Result<Option<String>, ProviderError> {
    #[cfg(test)]
    {
        let credential_is_present = TEST_PROVIDER_KEYCHAIN
            .lock()
            .expect("lock provider test keychain")
            .get(&reference.account)
            .is_some_and(Option::is_some);
        if credential_is_present
            && take_test_credential_fault(
                &reference.account,
                TestProviderCredentialFault::StagingReadback,
            )
        {
            return Err(ProviderError::CredentialStorageUnavailable(
                "injected staging credential read-back failure".to_string(),
            ));
        }
        if credential_is_present
            && take_test_credential_fault(
                &reference.account,
                TestProviderCredentialFault::StagingReadbackMismatch,
            )
        {
            return Ok(Some("injected-staging-mismatch".to_string()));
        }
    }
    #[cfg(test)]
    if let Ok(secret) = std::env::var(test_secret_env_name(&reference.account)) {
        return Ok(Some(secret));
    }
    #[cfg(test)]
    {
        let keychain = TEST_PROVIDER_KEYCHAIN
            .lock()
            .expect("lock provider test keychain");
        let target_account = reference
            .account
            .strip_suffix(":staging")
            .unwrap_or(&reference.account);
        if keychain.contains_key(&reference.account) || keychain.contains_key(target_account) {
            return Ok(keychain.get(&reference.account).and_then(Clone::clone));
        }
    }
    let entry = Entry::new(&reference.service, &reference.account)
        .map_err(|error| ProviderError::CredentialStorageUnavailable(error.to_string()))?;
    match entry.get_password() {
        Ok(secret) => Ok(Some(secret)),
        Err(keyring::Error::NoEntry) => Ok(None),
        Err(error) => Err(ProviderError::CredentialStorageUnavailable(
            error.to_string(),
        )),
    }
}

#[cfg(test)]
fn test_secret_env_name(account: &str) -> String {
    let suffix = account
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() {
                ch.to_ascii_uppercase()
            } else {
                '_'
            }
        })
        .collect::<String>();
    format!("SKILLS_COPILOT_TEST_SECRET_{suffix}")
}

fn delete_secret(
    reference: &ProviderCredentialReference,
) -> Result<CredentialDeleteOutcome, ProviderError> {
    #[cfg(test)]
    if take_test_credential_fault(&reference.account, TestProviderCredentialFault::Delete) {
        return Err(ProviderError::CredentialStorageUnavailable(
            "injected credential delete failure".to_string(),
        ));
    }
    #[cfg(test)]
    {
        let credential_is_present = TEST_PROVIDER_KEYCHAIN
            .lock()
            .expect("lock provider test keychain")
            .get(&reference.account)
            .is_some_and(Option::is_some);
        if credential_is_present
            && take_test_credential_fault(
                &reference.account,
                TestProviderCredentialFault::StagingDelete,
            )
        {
            return Err(ProviderError::CredentialStorageUnavailable(
                "injected staging credential delete failure".to_string(),
            ));
        }
    }
    #[cfg(test)]
    {
        let mut keychain = TEST_PROVIDER_KEYCHAIN
            .lock()
            .expect("lock provider test keychain");
        let target_account = reference
            .account
            .strip_suffix(":staging")
            .unwrap_or(&reference.account);
        if keychain.contains_key(&reference.account) || keychain.contains_key(target_account) {
            let deleted = keychain
                .insert(reference.account.clone(), None)
                .flatten()
                .is_some();
            return Ok(if deleted {
                CredentialDeleteOutcome::Deleted
            } else {
                CredentialDeleteOutcome::AlreadyAbsent
            });
        }
    }
    let entry = Entry::new(&reference.service, &reference.account)
        .map_err(|error| ProviderError::CredentialStorageUnavailable(error.to_string()))?;
    match entry.delete_credential() {
        Ok(()) => Ok(CredentialDeleteOutcome::Deleted),
        Err(keyring::Error::NoEntry) => Ok(CredentialDeleteOutcome::AlreadyAbsent),
        Err(error) => Err(ProviderError::CredentialStorageUnavailable(
            error.to_string(),
        )),
    }
}

fn delete_and_verify_secret_absent(
    reference: &ProviderCredentialReference,
) -> Result<CredentialDeleteOutcome, ProviderError> {
    let outcome = delete_secret(reference)?;
    match load_optional_secret(reference)? {
        None => Ok(outcome),
        Some(_) => Err(ProviderError::CredentialStorageUnavailable(
            "credential still exists after deletion".to_string(),
        )),
    }
}

fn available_credential_status(reason: &str) -> ProviderCredentialStatus {
    ProviderCredentialStatus {
        state: "available".to_string(),
        reason: reason.to_string(),
        secret_available: true,
        fallback_available: false,
    }
}

fn missing_credential_status(reason: String) -> ProviderCredentialStatus {
    ProviderCredentialStatus {
        state: "missing".to_string(),
        reason,
        secret_available: false,
        fallback_available: false,
    }
}

fn existing_credential_status(reference: &ProviderCredentialReference) -> ProviderCredentialStatus {
    match load_secret(reference) {
        Ok(secret) if !secret.is_empty() => {
            available_credential_status("API key is available from the OS credential store.")
        }
        Ok(_) => missing_credential_status("No API key is stored for this profile.".to_string()),
        Err(error) => missing_credential_status(error.to_string()),
    }
}

fn budget_status(profile: &ProviderProfileRecord) -> ProviderBudgetStatus {
    let estimated_test_tokens = TEST_INPUT_TOKEN_ESTIMATE + TEST_OUTPUT_TOKEN_ESTIMATE;
    let estimated_test_cost_usd =
        estimated_provider_cost(profile.provider_type, estimated_test_tokens);
    if profile.single_request_token_limit < estimated_test_tokens {
        ProviderBudgetStatus {
            single_request_token_limit: profile.single_request_token_limit,
            monthly_budget_usd: profile.monthly_budget_usd,
            estimated_test_tokens,
            estimated_test_cost_usd,
            state: "blocked".to_string(),
            reason: "Single request token limit is lower than the connection test estimate."
                .to_string(),
        }
    } else if profile.monthly_budget_usd <= 0.0 {
        ProviderBudgetStatus {
            single_request_token_limit: profile.single_request_token_limit,
            monthly_budget_usd: profile.monthly_budget_usd,
            estimated_test_tokens,
            estimated_test_cost_usd,
            state: "blocked".to_string(),
            reason: "Monthly provider budget is 0; provider requests are disabled.".to_string(),
        }
    } else {
        ProviderBudgetStatus {
            single_request_token_limit: profile.single_request_token_limit,
            monthly_budget_usd: profile.monthly_budget_usd,
            estimated_test_tokens,
            estimated_test_cost_usd,
            state: "ok".to_string(),
            reason: "Connection test is within configured local budget limits.".to_string(),
        }
    }
}

fn estimated_provider_cost(provider_type: ProviderType, tokens: u32) -> f64 {
    let per_million = match provider_type {
        ProviderType::OpenAiCompatible => 2.50,
        ProviderType::ClaudeCompatible => 3.00,
    };
    f64::from(tokens) * per_million / 1_000_000.0
}

fn file_revision_while_locked(
    owner: &AppMutationLock,
    relative_path: &Path,
    label: &str,
    max_bytes: u64,
) -> Result<String, ProviderError> {
    match owner
        .owner_fs()
        .read_bounded_regular_file(relative_path, max_bytes, label)?
    {
        Some(bytes) => Ok(digest_revision(&format!("{label}:present"), &bytes)),
        None => Ok(digest_revision(&format!("{label}:missing"), &[])),
    }
}

fn digest_revision(label: &str, content: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot/provider-revision/v1");
    hasher.update((label.len() as u64).to_be_bytes());
    hasher.update(label.as_bytes());
    hasher.update((content.len() as u64).to_be_bytes());
    hasher.update(content);
    format!("sha256:{:x}", hasher.finalize())
}

fn constant_time_secret_eq(left: &[u8], right: &[u8]) -> bool {
    let mut difference = left.len() ^ right.len();
    for index in 0..left.len().max(right.len()) {
        let left_byte = left.get(index).copied().unwrap_or_default();
        let right_byte = right.get(index).copied().unwrap_or_default();
        difference |= usize::from(left_byte ^ right_byte);
    }
    difference == 0
}

fn validate_base_url(value: &str) -> Result<String, ProviderError> {
    let value = require_non_empty("base_url", value)?;
    let url = Url::parse(&value).map_err(|_| {
        ProviderError::InvalidProfile("base_url must be an absolute http(s) URL".to_string())
    })?;
    let scheme = url.scheme();
    if !matches!(scheme, "https" | "http") {
        return Err(ProviderError::InvalidProfile(
            "base_url must use https://, or http:// for exact local loopback hosts".to_string(),
        ));
    }
    let Some(host) = url.host_str() else {
        return Err(ProviderError::InvalidProfile(
            "base_url must include a host".to_string(),
        ));
    };
    if !url.username().is_empty() || url.password().is_some() {
        return Err(ProviderError::InvalidProfile(
            "base_url must not include username or password credentials".to_string(),
        ));
    }
    if url.query().is_some() || url.fragment().is_some() {
        return Err(ProviderError::InvalidProfile(
            "base_url must not include query strings or fragments".to_string(),
        ));
    }
    if scheme == "http" && !is_loopback_provider_host(host) {
        return Err(ProviderError::InvalidProfile(
            "base_url may use http only for exact localhost, 127.0.0.1, or ::1 hosts".to_string(),
        ));
    }
    Ok(url.as_str().trim_end_matches('/').to_string())
}

fn is_loopback_provider_host(host: &str) -> bool {
    let normalized = host
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
        .unwrap_or(host)
        .to_ascii_lowercase();
    matches!(normalized.as_str(), "localhost" | "127.0.0.1" | "::1")
}

fn require_non_empty(field: &str, value: &str) -> Result<String, ProviderError> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return Err(ProviderError::InvalidProfile(format!(
            "{field} must not be empty"
        )));
    }
    Ok(trimmed.to_string())
}

fn non_empty_string(value: String) -> Option<String> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        None
    } else {
        Some(trimmed.to_string())
    }
}

fn default_api_version(provider_type: ProviderType) -> Option<String> {
    match provider_type {
        ProviderType::OpenAiCompatible => None,
        ProviderType::ClaudeCompatible => Some("2023-06-01".to_string()),
    }
}

fn sanitize_profile_id(value: &str) -> String {
    value
        .chars()
        .filter_map(|ch| {
            let lower = ch.to_ascii_lowercase();
            if lower.is_ascii_alphanumeric() || matches!(lower, '-' | '_') {
                Some(lower)
            } else if ch.is_whitespace() {
                Some('-')
            } else {
                None
            }
        })
        .take(80)
        .collect::<String>()
        .trim_matches('-')
        .to_string()
}

pub(crate) fn destination_host(base_url: &str) -> String {
    let Ok(url) = Url::parse(base_url) else {
        return "<unknown>".to_string();
    };
    let Some(host) = url.host_str() else {
        return "<unknown>".to_string();
    };
    let host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host,
    }
}

fn redact_error(error: &ProviderRequestError) -> String {
    match error {
        ProviderRequestError::Transport(_) => {
            "Provider network transport failed after the request was attempted.".to_string()
        }
        ProviderRequestError::BodyUnreadable(_) => {
            "Provider response body could not be read completely.".to_string()
        }
        ProviderRequestError::BodyTooLarge => {
            "Provider response body exceeded the local safety limit.".to_string()
        }
    }
}

fn unix_timestamp_millis() -> i64 {
    let millis = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis();
    i64::try_from(millis).unwrap_or(i64::MAX)
}

pub(crate) fn normalize_epoch_millis(value: i64) -> i64 {
    let magnitude = value.checked_abs().unwrap_or(i64::MAX);
    // Persisted production values in epoch seconds are currently ten digits.
    // Keep tiny synthetic/relative fixture clocks unchanged and only migrate a
    // plausible wall-clock seconds value; millisecond epochs are already 13 digits.
    if (1_000_000_000..10_000_000_000).contains(&magnitude) {
        value.saturating_mul(1_000)
    } else {
        value
    }
}

pub(crate) fn validate_prompt_business_output(
    action_type: &str,
    output_text: Option<&str>,
) -> Result<(), String> {
    if action_type != "task_cockpit" {
        return Ok(());
    }
    let output = output_text
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .ok_or_else(|| "Provider returned no Task Preflight output.".to_string())?;
    let value: Value = serde_json::from_str(output).map_err(|_| {
        "Provider returned Task Preflight output that is not valid JSON.".to_string()
    })?;
    let object = value.as_object().ok_or_else(|| {
        "Provider returned Task Preflight JSON with an invalid top-level shape.".to_string()
    })?;
    for key in [
        "summary",
        "agent_candidates",
        "skill_candidates",
        "safety_flags",
    ] {
        if !object.contains_key(key) {
            return Err(format!(
                "Provider Task Preflight JSON is missing required field `{key}`."
            ));
        }
    }
    if !object.get("summary").is_some_and(Value::is_object)
        || !object.get("agent_candidates").is_some_and(Value::is_array)
        || !object.get("skill_candidates").is_some_and(Value::is_array)
        || !object.get("safety_flags").is_some_and(Value::is_object)
    {
        return Err(
            "Provider Task Preflight JSON contains fields with incompatible schema types."
                .to_string(),
        );
    }
    Ok(())
}

fn default_enabled() -> bool {
    false
}

#[allow(dead_code)]
fn _assert_no_raw_secret_in_value(value: &Value) -> bool {
    !value.to_string().contains("api_key")
}
