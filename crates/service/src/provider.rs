use std::{
    fs, io,
    path::{Path, PathBuf},
    time::{Duration, Instant, SystemTime, UNIX_EPOCH},
};

use keyring::Entry;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use thiserror::Error;
use ureq::Error as UreqError;
use url::Url;

use crate::{append_private_line, write_private_text_file};

const KEYCHAIN_SERVICE: &str = "dev.skills-copilot.native.llm";
const PROFILE_STORE_VERSION: u32 = 1;
const DEFAULT_SINGLE_REQUEST_TOKEN_LIMIT: u32 = 8_000;
const DEFAULT_MONTHLY_BUDGET_USD: f64 = 5.0;
const TEST_INPUT_TOKEN_ESTIMATE: u32 = 12;
const TEST_OUTPUT_TOKEN_ESTIMATE: u32 = 4;

#[derive(Debug, Error)]
pub enum ProviderError {
    #[error("io error: {0}")]
    Io(#[from] io::Error),
    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
    #[error("invalid provider profile: {0}")]
    InvalidProfile(String),
    #[error("provider profile not found: {0}")]
    ProfileNotFound(String),
    #[error("credential storage unavailable: {0}")]
    CredentialStorageUnavailable(String),
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
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteProviderProfileParams {
    pub profile_id: String,
    #[serde(default)]
    pub delete_credential: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TestProviderConnectionParams {
    pub profile_id: String,
    pub confirmation_id: String,
    #[serde(default)]
    pub timeout_ms: Option<u64>,
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
    pub raw_secret_returned: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct DeleteProviderProfileResult {
    pub deleted_profile_id: String,
    pub profile_deleted: bool,
    pub credential_deleted: bool,
    pub raw_secret_returned: bool,
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
    Ok(ListProviderProfilesResult {
        profiles: store.profiles,
        default_profile_id: store.default_profile_id,
        credential_storage: "keychain".to_string(),
        credential_persistence_allowed: true,
        raw_secrets_returned: false,
    })
}

pub fn save_provider_profile(
    app_data_dir: &Path,
    params: SaveProviderProfileParams,
) -> Result<SaveProviderProfileResult, ProviderError> {
    let now = unix_timestamp_millis();
    let mut store = load_store(app_data_dir)?;
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
    let base_url = validate_base_url(&params.base_url)?;
    let model = require_non_empty("model", &params.model)?;
    let display_name = require_non_empty("display_name", &params.display_name)?;
    let token_limit = params
        .single_request_token_limit
        .unwrap_or(DEFAULT_SINGLE_REQUEST_TOKEN_LIMIT)
        .clamp(1, 200_000);
    let monthly_budget = params
        .monthly_budget_usd
        .unwrap_or(DEFAULT_MONTHLY_BUDGET_USD)
        .clamp(0.0, 10_000.0);
    let previous_profile = store
        .profiles
        .iter()
        .find(|profile| profile.id == profile_id)
        .cloned();
    let has_new_secret = params
        .api_key
        .as_deref()
        .map(str::trim)
        .is_some_and(|secret| !secret.is_empty());
    let mut credential_reference = if has_new_secret {
        keychain_reference(&profile_id)
    } else {
        previous_profile
            .as_ref()
            .map(|profile| profile.credential_reference.clone())
            .unwrap_or_else(|| keychain_reference(&profile_id))
    };
    let credential_status = match params.api_key.as_deref().map(str::trim) {
        Some(secret) if !secret.is_empty() => {
            store_and_verify_secret(&credential_reference, secret)?;
            available_credential_status("API key stored in the OS credential store.")
        }
        _ => existing_credential_status(&credential_reference),
    };
    credential_reference.secret_persisted = credential_status.secret_available;
    let previous_created_at = previous_profile
        .as_ref()
        .map(|profile| profile.created_at)
        .unwrap_or(now);
    let profile = ProviderProfileRecord {
        id: profile_id.clone(),
        display_name,
        provider_type: params.provider_type,
        base_url,
        model,
        enabled: params.enabled,
        api_version: params
            .api_version
            .and_then(non_empty_string)
            .or_else(|| default_api_version(params.provider_type)),
        organization: params.organization.and_then(non_empty_string),
        single_request_token_limit: token_limit,
        monthly_budget_usd: monthly_budget,
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
    save_store(app_data_dir, &store)?;

    Ok(SaveProviderProfileResult {
        profile,
        credential_status,
        raw_secret_returned: false,
    })
}

pub fn delete_provider_profile(
    app_data_dir: &Path,
    params: DeleteProviderProfileParams,
) -> Result<DeleteProviderProfileResult, ProviderError> {
    let mut store = load_store(app_data_dir)?;
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
            raw_secret_returned: false,
        });
    };
    store.profiles.retain(|existing| existing.id != profile.id);
    if store.default_profile_id.as_deref() == Some(profile.id.as_str()) {
        store.default_profile_id = store.profiles.first().map(|profile| profile.id.clone());
    }
    save_store(app_data_dir, &store)?;
    let credential_deleted =
        params.delete_credential && delete_secret(&profile.credential_reference).unwrap_or(false);

    Ok(DeleteProviderProfileResult {
        deleted_profile_id: profile.id,
        profile_deleted: true,
        credential_deleted,
        raw_secret_returned: false,
    })
}

pub fn test_provider_connection(
    app_data_dir: &Path,
    params: TestProviderConnectionParams,
) -> Result<TestProviderConnectionResult, ProviderError> {
    let store = load_store(app_data_dir)?;
    let mut profile = store
        .profiles
        .iter()
        .find(|profile| profile.id == params.profile_id)
        .cloned()
        .ok_or_else(|| ProviderError::ProfileNotFound(params.profile_id.clone()))?;
    let destination_host = destination_host(&profile.base_url);
    let budget = budget_status(&profile);
    let started = Instant::now();

    if !profile.enabled {
        return finish_test(
            app_data_dir,
            &profile,
            &destination_host,
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
            &profile,
            &destination_host,
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
            &profile,
            &destination_host,
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
        Ok(secret) => {
            mark_profile_credential_status(
                app_data_dir,
                &mut profile,
                available_credential_status("API key is available from the OS credential store."),
            );
            secret
        }
        Err(error) => {
            mark_profile_credential_status(
                app_data_dir,
                &mut profile,
                missing_credential_status(error.to_string()),
            );
            return finish_test(
                app_data_dir,
                &profile,
                &destination_host,
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
            &profile,
            &destination_host,
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
            &profile,
            &destination_host,
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
            &profile,
            &destination_host,
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
    let store = load_store(app_data_dir)?;
    let mut profile = store
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
        Ok(secret) => {
            mark_profile_credential_status(
                app_data_dir,
                &mut profile,
                available_credential_status("API key is available from the OS credential store."),
            );
            secret
        }
        Err(error) => {
            mark_profile_credential_status(
                app_data_dir,
                &mut profile,
                missing_credential_status(error.to_string()),
            );
            return finish_prompt(
                app_data_dir,
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
    profile: &ProviderProfileRecord,
    destination_host: &str,
    confirmation_id: &str,
    started: Instant,
    budget: ProviderBudgetStatus,
    finish: ProviderTestFinish<'_>,
) -> Result<TestProviderConnectionResult, ProviderError> {
    let audit = ProviderCallMetadata {
        timestamp: unix_timestamp_millis(),
        action_type: "test_connection".to_string(),
        profile_id: profile.id.clone(),
        provider_type: profile.provider_type,
        model: profile.model.clone(),
        destination_host: destination_host.to_string(),
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
    append_call_metadata(app_data_dir, &audit)?;
    Ok(TestProviderConnectionResult {
        profile_id: profile.id.clone(),
        provider_type: profile.provider_type,
        model: profile.model.clone(),
        destination_host: destination_host.to_string(),
        status: finish.status.to_string(),
        provider_request_sent: finish.provider_request_sent,
        credential_accessed: finish.credential_accessed,
        duration_ms: audit.duration_ms,
        error_code: finish.error_code,
        error_message: finish.error_message,
        budget,
        audit,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_secret_returned: false,
    })
}

fn finish_prompt(
    app_data_dir: &Path,
    profile: &ProviderProfileRecord,
    destination_host: &str,
    params: &SendProviderPromptParams,
    started: Instant,
    finish: ProviderPromptFinish,
) -> Result<SendProviderPromptResult, ProviderError> {
    let audit = ProviderCallMetadata {
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
    append_call_metadata(app_data_dir, &audit)?;
    Ok(SendProviderPromptResult {
        profile_id: profile.id.clone(),
        provider_type: profile.provider_type,
        model: profile.model.clone(),
        destination_host: destination_host.to_string(),
        status: finish.status,
        provider_request_sent: finish.provider_request_sent,
        credential_accessed: finish.credential_accessed,
        duration_ms: audit.duration_ms,
        error_code: finish.error_code,
        error_message: finish.error_message,
        output_text: finish.output_text,
        audit,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_secret_returned: false,
    })
}

fn load_store(app_data_dir: &Path) -> Result<ProviderProfileStore, ProviderError> {
    let path = provider_profiles_path(app_data_dir);
    if !path.exists() {
        return Ok(ProviderProfileStore::default());
    }
    let content = fs::read_to_string(path)?;
    let mut store: ProviderProfileStore = serde_json::from_str(&content)?;
    for profile in &mut store.profiles {
        profile.created_at = normalize_epoch_millis(profile.created_at);
        profile.updated_at = normalize_epoch_millis(profile.updated_at);
    }
    Ok(store)
}

fn save_store(app_data_dir: &Path, store: &ProviderProfileStore) -> Result<(), ProviderError> {
    let path = provider_profiles_path(app_data_dir);
    let content = serde_json::to_string_pretty(store)?;
    write_private_text_file(&path, &format!("{content}\n"))?;
    Ok(())
}

fn append_call_metadata(
    app_data_dir: &Path,
    metadata: &ProviderCallMetadata,
) -> Result<(), ProviderError> {
    let path = provider_call_metadata_path(app_data_dir);
    let line = serde_json::to_string(metadata)?;
    append_private_line(&path, &line)?;
    Ok(())
}

fn send_test_request(
    profile: &ProviderProfileRecord,
    secret: &str,
    timeout: Duration,
) -> Result<u16, Box<UreqError>> {
    let url = test_endpoint_url(profile);
    let mut request = ureq::post(&url)
        .timeout(timeout)
        .set("content-type", "application/json");
    if let Some(org) = profile.organization.as_deref() {
        request = request.set("openai-organization", org);
    }
    match profile.provider_type {
        ProviderType::OpenAiCompatible => {
            request = request.set("authorization", &format!("Bearer {secret}"));
            let response = request
                .send_json(json!({
                    "model": profile.model,
                    "messages": [{"role": "user", "content": "connection test"}],
                    "max_tokens": 1,
                    "temperature": 0
                }))
                .map_err(Box::new)?;
            Ok(response.status())
        }
        ProviderType::ClaudeCompatible => {
            request = request.set("x-api-key", secret).set(
                "anthropic-version",
                profile.api_version.as_deref().unwrap_or("2023-06-01"),
            );
            let response = request
                .send_json(json!({
                    "model": profile.model,
                    "messages": [{"role": "user", "content": "connection test"}],
                    "max_tokens": 1
                }))
                .map_err(Box::new)?;
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
) -> Result<ProviderPromptHttpSuccess, Box<UreqError>> {
    let url = test_endpoint_url(profile);
    let max_tokens = params.estimated_output_tokens.clamp(1, 8_000);
    let mut request = ureq::post(&url)
        .timeout(timeout)
        .set("content-type", "application/json");
    if let Some(org) = profile.organization.as_deref() {
        request = request.set("openai-organization", org);
    }
    let response = match profile.provider_type {
        ProviderType::OpenAiCompatible => {
            request = request.set("authorization", &format!("Bearer {secret}"));
            request
                .send_json(json!({
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
                }))
                .map_err(Box::new)?
        }
        ProviderType::ClaudeCompatible => {
            request = request.set("x-api-key", secret).set(
                "anthropic-version",
                profile.api_version.as_deref().unwrap_or("2023-06-01"),
            );
            request
                .send_json(json!({
                    "model": profile.model,
                    "system": "You are reviewing AI agent skills. Return draft-only guidance; do not claim to write files, execute scripts, mutate configuration, or store credentials.",
                    "messages": [{"role": "user", "content": prompt}],
                    "max_tokens": max_tokens
                }))
                .map_err(Box::new)?
        }
    };
    let status = response.status();
    let body = response.into_string().unwrap_or_default();
    Ok(ProviderPromptHttpSuccess { status, body })
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

fn store_secret(
    reference: &ProviderCredentialReference,
    secret: &str,
) -> Result<(), ProviderError> {
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
    #[cfg(test)]
    if let Ok(secret) = std::env::var(test_secret_env_name(&reference.account)) {
        return Ok(secret);
    }
    let entry = Entry::new(&reference.service, &reference.account)
        .map_err(|error| ProviderError::CredentialStorageUnavailable(error.to_string()))?;
    entry
        .get_password()
        .map_err(|error| ProviderError::CredentialStorageUnavailable(error.to_string()))
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

fn delete_secret(reference: &ProviderCredentialReference) -> Result<bool, ProviderError> {
    let entry = Entry::new(&reference.service, &reference.account)
        .map_err(|error| ProviderError::CredentialStorageUnavailable(error.to_string()))?;
    match entry.delete_credential() {
        Ok(()) => Ok(true),
        Err(_) => Ok(false),
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

fn mark_profile_credential_status(
    app_data_dir: &Path,
    profile: &mut ProviderProfileRecord,
    status: ProviderCredentialStatus,
) {
    profile.credential_reference.secret_persisted = status.secret_available;
    profile.credential_status = status;
    let _ = persist_profile_credential_status(app_data_dir, profile);
}

fn persist_profile_credential_status(
    app_data_dir: &Path,
    profile: &ProviderProfileRecord,
) -> Result<(), ProviderError> {
    let mut store = load_store(app_data_dir)?;
    if let Some(stored_profile) = store
        .profiles
        .iter_mut()
        .find(|stored_profile| stored_profile.id == profile.id)
    {
        stored_profile.credential_reference = profile.credential_reference.clone();
        stored_profile.credential_status = profile.credential_status.clone();
        stored_profile.updated_at = unix_timestamp_millis();
        save_store(app_data_dir, &store)?;
    }
    Ok(())
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

fn destination_host(base_url: &str) -> String {
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

fn redact_error(error: &UreqError) -> String {
    match error {
        UreqError::Status(status, _) => format!("Provider returned HTTP status {status}."),
        UreqError::Transport(transport) => transport.to_string(),
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
