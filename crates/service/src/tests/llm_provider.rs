use super::dispatch_fixtures::*;
use super::*;
use crate::service_llm::{
    read_consistent_provider_activity_raw_snapshot_with, read_provider_activity_bounded,
    read_provider_activity_raw_source, ProviderActivityRawSource, ProviderActivitySource,
    PROVIDER_ACTIVITY_MAX_SOURCE_BYTES,
};

#[test]
fn legacy_provider_epoch_seconds_are_normalized_without_changing_fixture_clocks() {
    assert_eq!(
        crate::provider::normalize_epoch_millis(1_789_000_000),
        1_789_000_000_000
    );
    assert_eq!(
        crate::provider::normalize_epoch_millis(1_789_000_000_000),
        1_789_000_000_000
    );
    assert_eq!(crate::provider::normalize_epoch_millis(2_200), 2_200);
}

#[test]
fn task_cockpit_transport_success_requires_business_schema() {
    let valid = json!({
        "summary": {},
        "agent_candidates": [],
        "skill_candidates": [],
        "safety_flags": {}
    })
    .to_string();
    assert!(crate::provider::validate_prompt_business_output("task_cockpit", Some(&valid)).is_ok());
    assert!(
        crate::provider::validate_prompt_business_output("task_cockpit", Some("not-json")).is_err()
    );
    assert!(crate::provider::validate_prompt_business_output(
        "task_cockpit",
        Some(r#"{"summary":{},"agent_candidates":[],"skill_candidates":[]}"#),
    )
    .is_err());
    assert!(
        crate::provider::validate_prompt_business_output("analyze", Some("plain draft")).is_ok()
    );
}

#[test]
fn llm_preview_prompt_returns_redacted_confirmation_payload() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-llm-preview-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let skill_path = app_data_dir.join("secret-project-path").join("SKILL.md");
    seed_catalog_with_llm_skill(&host, &skill_path);
    let save = host.handle(ServiceRequest {
        id: Some("provider-save".to_string()),
        method: "llm.saveProviderProfile".to_string(),
        params: json!({
            "id": "fixture-openai",
            "display_name": "Fixture OpenAI",
            "provider_type": "openai-compatible",
            "base_url": "https://example.invalid/v1",
            "model": "fixture-model",
            "enabled": true,
            "single_request_token_limit": 4096,
            "monthly_budget_usd": 3.5
        }),
    });
    assert!(save.ok, "{:?}", save.error);

    let response = host.handle(ServiceRequest {
            id: Some("preview".to_string()),
            method: "llm.previewPrompt".to_string(),
            params: json!({
                "action": "analyze",
                "skill_instance_id": "llm-skill-id",
                "user_intent": "review credential_marker=fixture-redacted-value without leaking local paths"
            }),
        });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("preview result");
    assert_eq!(result.get("status").and_then(Value::as_str), Some("ready"));
    assert_eq!(result.get("allowed").and_then(Value::as_bool), Some(true));
    assert_eq!(
        result.get("requires_confirmation").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        result.get("provider_request_sent").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result.get("write_back_allowed").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .get("draft_requires_user_copy")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(result
        .get("preview_id")
        .and_then(Value::as_str)
        .is_some_and(|id| id.starts_with("prompt-preview-")));
    assert_eq!(
        result
            .pointer("/redaction/raw_prompt_persisted")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .pointer("/redaction/raw_secret_returned")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(result
        .pointer("/redaction/redacted_value_count")
        .and_then(Value::as_u64)
        .is_some_and(|count| count > 0));

    let serialized = serde_json::to_string(&result).expect("serialize preview");
    assert!(serialized.contains("<redacted>"));
    assert!(!serialized.contains("OPENAI_API_KEY"));
    assert!(!serialized.contains("fixture-redacted-value"));
    assert!(!serialized.contains(&skill_path.to_string_lossy().to_string()));
    assert!(!provider_call_metadata_path(&app_data_dir).exists());

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_skill_prompt_uses_the_same_visible_issue_policy_as_the_app() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-llm-visible-findings-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    seed_catalog_with_llm_skill(&host, &app_data_dir.join("fixture-skill").join("SKILL.md"));
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    let drafts = [
        (
            "baseline-warning",
            "permissions.network-declared",
            "warning",
        ),
        ("baseline-error", "permissions.exec-needs-human", "error"),
        ("collision", "name.collision", "info"),
        ("ignored", "script.no-shebang", "warning"),
        ("reviewed", "name.canonical-case", "warning"),
        ("suppressed", "dependency.unknown", "warning"),
        ("visible", "body.too-long", "warning"),
        ("sibling", "frontmatter.description-missing", "warning"),
    ]
    .into_iter()
    .map(|(id, rule_id, severity)| RuleFindingDraft {
        id: id.to_string(),
        instance_id: Some(if id == "sibling" {
            "llm-sibling-id".to_string()
        } else {
            "llm-skill-id".to_string()
        }),
        definition_id: Some("llm-definition-id".to_string()),
        rule_id: rule_id.to_string(),
        severity: severity.to_string(),
        message: format!("{rule_id} marker"),
        suggestion: Some(format!("fix {rule_id}")),
        created_at: 1,
    })
    .collect::<Vec<_>>();
    catalog
        .refresh_rule_findings(&drafts)
        .expect("refresh prompt findings");
    let seeded = catalog.list_rule_findings().expect("list prompt findings");
    for (rule_id, status) in [
        ("script.no-shebang", "ignored"),
        ("name.canonical-case", "reviewed"),
    ] {
        let triage_key = seeded
            .iter()
            .find(|finding| finding.rule_id == rule_id)
            .map(|finding| finding.triage_key.as_str())
            .expect("triage key");
        catalog
            .set_finding_triage(triage_key, status, None, 2)
            .expect("set prompt finding triage");
    }
    catalog
        .set_rule_suppression(
            "dependency.unknown",
            Some("claude-code"),
            Some("agent-global"),
            "fixture suppression",
            None,
            2,
        )
        .expect("suppress prompt finding");

    let response = host.handle(ServiceRequest {
        id: Some("preview-visible-findings".to_string()),
        method: "llm.previewPrompt".to_string(),
        params: json!({
            "action": "analyze",
            "skill_instance_id": "llm-skill-id"
        }),
    });

    assert!(response.ok, "{:?}", response.error);
    let prompt = response
        .result
        .as_ref()
        .and_then(|result| result.get("prompt_preview"))
        .and_then(Value::as_str)
        .expect("prompt preview");
    assert!(prompt.contains("body.too-long"));
    assert!(prompt.contains("permissions.exec-needs-human"));
    for hidden_rule in [
        "permissions.network-declared",
        "name.collision",
        "script.no-shebang",
        "name.canonical-case",
        "dependency.unknown",
        "frontmatter.description-missing",
    ] {
        assert!(
            !prompt.contains(hidden_rule),
            "hidden rule leaked into prompt: {hidden_rule}"
        );
    }

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_confirm_prompt_rejects_mismatched_preview_without_metadata() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-llm-preview-mismatch-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    seed_catalog_with_llm_skill(&host, &app_data_dir.join("fixture-skill").join("SKILL.md"));
    let save = host.handle(ServiceRequest {
        id: Some("provider-save".to_string()),
        method: "llm.saveProviderProfile".to_string(),
        params: json!({
            "id": "fixture-openai",
            "display_name": "Fixture OpenAI",
            "provider_type": "openai-compatible",
            "base_url": "https://example.invalid/v1",
            "model": "fixture-model",
            "enabled": true
        }),
    });
    assert!(save.ok, "{:?}", save.error);

    let response = host.handle(ServiceRequest {
        id: Some("confirm".to_string()),
        method: "llm.confirmPromptAndSend".to_string(),
        params: json!({
            "preview_id": "prompt-preview-stale",
            "confirmation_id": "confirm-preview",
            "request": {
                "action": "analyze",
                "skill_instance_id": "llm-skill-id"
            }
        }),
    });

    assert!(!response.ok);
    let error = response.error.expect("mismatch error");
    assert_eq!(error.code, "invalid_request");
    assert!(error.message.contains("preview_id"));
    assert!(!provider_call_metadata_path(&app_data_dir).exists());

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_confirm_prompt_blocks_without_credential_and_writes_metadata_only() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-llm-confirm-blocked-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let save = host.handle(ServiceRequest {
        id: Some("provider-save".to_string()),
        method: "llm.saveProviderProfile".to_string(),
        params: json!({
            "id": "fixture-openai",
            "display_name": "Fixture OpenAI",
            "provider_type": "openai-compatible",
            "base_url": "https://example.invalid/v1",
            "model": "fixture-model",
            "enabled": true
        }),
    });
    assert!(save.ok, "{:?}", save.error);
    let request = json!({
        "action": "recommend",
        "user_intent": "review token=fixture-redacted-value"
    });
    let preview = host.handle(ServiceRequest {
        id: Some("preview".to_string()),
        method: "llm.previewPrompt".to_string(),
        params: request.clone(),
    });
    assert!(preview.ok, "{:?}", preview.error);
    let preview_id = preview
        .result
        .as_ref()
        .and_then(|result| result.get("preview_id"))
        .and_then(Value::as_str)
        .expect("preview id")
        .to_string();

    let confirm = host.handle(ServiceRequest {
        id: Some("confirm".to_string()),
        method: "llm.confirmPromptAndSend".to_string(),
        params: json!({
            "preview_id": preview_id,
            "confirmation_id": "confirm-without-credential",
            "request": request,
            "timeout_ms": 250
        }),
    });

    assert!(confirm.ok, "{:?}", confirm.error);
    let result = confirm.result.expect("confirm result");
    assert_eq!(
        result.get("status").and_then(Value::as_str),
        Some("blocked")
    );
    assert_eq!(
        result.get("provider_request_sent").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result.get("credential_accessed").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result.pointer("/audit/error_code").and_then(Value::as_str),
        Some("credential_unavailable")
    );
    assert_eq!(
        result
            .pointer("/audit/provider_request_sent")
            .and_then(Value::as_bool),
        Some(false)
    );

    let audit_path = provider_call_metadata_path(&app_data_dir);
    let audit_content = fs::read_to_string(&audit_path).expect("audit content");
    assert!(audit_content.contains("\"action_type\":\"recommend\""));
    assert!(audit_content.contains("\"status\":\"blocked\""));
    assert!(!audit_content.contains("fixture-redacted-value"));
    assert!(!audit_content.contains("review token"));
    assert!(!audit_content.contains("api_key"));

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_confirm_prompt_sends_redacted_prompt_to_mock_provider_and_audits_metadata_only() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-llm-confirm-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let (base_url, server) = spawn_mock_openai_server();
    let host = test_host(app_data_dir.clone());
    let skill_path = app_data_dir.join("fixture-skill").join("SKILL.md");
    seed_catalog_with_llm_skill(&host, &skill_path);
    let save = host.handle(ServiceRequest {
        id: Some("provider-save".to_string()),
        method: "llm.saveProviderProfile".to_string(),
        params: json!({
            "id": "mock-openai",
            "display_name": "Mock OpenAI",
            "provider_type": "openai-compatible",
            "base_url": base_url,
            "model": "mock-model",
            "enabled": true,
            "single_request_token_limit": 4096,
            "monthly_budget_usd": 10.0
        }),
    });
    assert!(save.ok, "{:?}", save.error);
    let _secret_env_guard = EnvVarGuard::set(
        "SKILLS_COPILOT_TEST_SECRET_PROVIDER_MOCK_OPENAI",
        "test-secret-key",
    );

    let request = json!({
        "action": "analyze",
        "skill_instance_id": "llm-skill-id",
        "user_intent": "summarize risk without exposing token=fixture-redacted-value"
    });
    let preview = host.handle(ServiceRequest {
        id: Some("preview".to_string()),
        method: "llm.previewPrompt".to_string(),
        params: request.clone(),
    });
    assert!(preview.ok, "{:?}", preview.error);
    let preview_result = preview.result.expect("preview result");
    let preview_id = preview_result
        .get("preview_id")
        .and_then(Value::as_str)
        .expect("preview id")
        .to_string();

    let confirm = host.handle(ServiceRequest {
        id: Some("confirm".to_string()),
        method: "llm.confirmPromptAndSend".to_string(),
        params: json!({
            "preview_id": preview_id,
            "confirmation_id": "confirm-mock-provider",
            "request": request,
            "timeout_ms": 2_000
        }),
    });

    assert!(confirm.ok, "{:?}", confirm.error);
    let result = confirm.result.expect("confirm result");
    assert_eq!(
        result.get("status").and_then(Value::as_str),
        Some("succeeded")
    );
    assert_eq!(
        result.get("provider_request_sent").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        result.get("credential_accessed").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        result.get("draft_output").and_then(Value::as_str),
        Some("Draft-only review from mock provider.")
    );
    assert_eq!(
        result.get("write_back_allowed").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .get("script_execution_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result.get("raw_prompt_persisted").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .get("raw_response_persisted")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result.pointer("/audit/action_type").and_then(Value::as_str),
        Some("analyze")
    );
    assert_eq!(
        result
            .pointer("/audit/confirmation_id")
            .and_then(Value::as_str),
        Some("confirm-mock-provider")
    );

    let request_text = server.join().expect("mock server thread");
    assert!(request_text
        .to_lowercase()
        .contains("authorization: bearer test-secret-key"));
    assert!(request_text.contains("<redacted>"));
    assert!(!request_text.contains("OPENAI_API_KEY"));
    assert!(!request_text.contains("fixture-redacted-value"));
    assert!(!request_text.contains(&skill_path.to_string_lossy().to_string()));

    let audit_path = provider_call_metadata_path(&app_data_dir);
    let audit_content = fs::read_to_string(&audit_path).expect("audit content");
    assert!(audit_content.contains("\"action_type\":\"analyze\""));
    assert!(audit_content.contains("\"status\":\"succeeded\""));
    assert!(audit_content.contains("\"provider_request_sent\":true"));
    assert!(!audit_content.contains("Draft-only review from mock provider."));
    assert!(!audit_content.contains("OPENAI_API_KEY"));
    assert!(!audit_content.contains("test-secret-key"));
    assert_private_path_mode(&audit_path, 0o600);
    assert_private_path_mode(audit_path.parent().expect("audit parent"), 0o700);

    let list_runs = host.handle(ServiceRequest {
        id: Some("runs".to_string()),
        method: "llm.listPromptRuns".to_string(),
        params: json!({ "instance_id": "llm-skill-id" }),
    });
    assert!(list_runs.ok, "{:?}", list_runs.error);
    let runs = list_runs.result.expect("prompt runs");
    assert_eq!(runs.get("count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        runs.pointer("/runs/0/status").and_then(Value::as_str),
        Some("succeeded")
    );
    assert_eq!(
        runs.pointer("/runs/0/draft_output").and_then(Value::as_str),
        Some("Draft-only review from mock provider.")
    );
    assert_eq!(
        runs.pointer("/runs/0/raw_prompt_persisted")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        runs.pointer("/runs/0/raw_response_persisted")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        runs.pointer("/runs/0/safety_flags/write_back_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );

    let prompt_runs_path = host.llm_prompt_runs_path();
    let prompt_run_content = fs::read_to_string(&prompt_runs_path).expect("prompt run content");
    assert!(prompt_run_content.contains("Draft-only review from mock provider."));
    assert!(prompt_run_content.contains("\"request_kind\": \"analyze\""));
    assert!(!prompt_run_content.contains("test-secret-key"));
    assert!(!prompt_run_content.contains("fixture-redacted-value"));
    assert!(!prompt_run_content.contains(&skill_path.to_string_lossy().to_string()));
    assert!(!prompt_run_content.contains("\"choices\""));
    assert_private_path_mode(&prompt_runs_path, 0o600);
    assert_private_path_mode(prompt_runs_path.parent().expect("prompt run parent"), 0o700);

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_confirm_prompt_redacts_persisted_draft_output() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-llm-draft-redaction-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let local_path = app_data_dir
        .join("fixture-project")
        .join("private")
        .join("SKILL.md")
        .to_string_lossy()
        .to_string();
    let high_entropy_secret = "AbCDefGhIjKlMnOpQrStUvWxYz1234567890__++";
    let provider_draft =
        format!("Draft cites {local_path} and opaque value {high_entropy_secret}.");
    let (base_url, server) = spawn_mock_openai_server_with_content(provider_draft.clone());
    let host = test_host(app_data_dir.clone());
    let skill_path = app_data_dir.join("fixture-skill").join("SKILL.md");
    seed_catalog_with_llm_skill(&host, &skill_path);

    let save = host.handle(ServiceRequest {
        id: Some("provider-save".to_string()),
        method: "llm.saveProviderProfile".to_string(),
        params: json!({
            "id": "mock-openai-draft-redaction",
            "display_name": "Mock OpenAI Draft Redaction",
            "provider_type": "openai-compatible",
            "base_url": base_url,
            "model": "mock-model",
            "enabled": true,
            "single_request_token_limit": 4096,
            "monthly_budget_usd": 10.0
        }),
    });
    assert!(save.ok, "{:?}", save.error);
    let _secret_env_guard = EnvVarGuard::set(
        "SKILLS_COPILOT_TEST_SECRET_PROVIDER_MOCK_OPENAI_DRAFT_REDACTION",
        "test-secret-key",
    );

    let request = json!({
        "action": "analyze",
        "skill_instance_id": "llm-skill-id",
        "user_intent": "summarize draft redaction posture"
    });
    let preview = host.handle(ServiceRequest {
        id: Some("preview".to_string()),
        method: "llm.previewPrompt".to_string(),
        params: request.clone(),
    });
    assert!(preview.ok, "{:?}", preview.error);
    let preview_id = preview
        .result
        .expect("preview result")
        .get("preview_id")
        .and_then(Value::as_str)
        .expect("preview id")
        .to_string();

    let confirm = host.handle(ServiceRequest {
        id: Some("confirm".to_string()),
        method: "llm.confirmPromptAndSend".to_string(),
        params: json!({
            "preview_id": preview_id,
            "confirmation_id": "confirm-draft-redaction",
            "request": request,
            "timeout_ms": 2_000
        }),
    });
    assert!(confirm.ok, "{:?}", confirm.error);
    let result = confirm.result.expect("confirm result");
    assert_eq!(
        result.get("draft_output").and_then(Value::as_str),
        Some(provider_draft.as_str()),
        "copy-only provider output remains available in the immediate result"
    );
    let _request_text = server.join().expect("mock server thread");

    let list_runs = host.handle(ServiceRequest {
        id: Some("runs".to_string()),
        method: "llm.listPromptRuns".to_string(),
        params: json!({ "instance_id": "llm-skill-id" }),
    });
    assert!(list_runs.ok, "{:?}", list_runs.error);
    let runs = list_runs.result.expect("prompt runs");
    let persisted_draft = runs
        .pointer("/runs/0/draft_output")
        .and_then(Value::as_str)
        .expect("persisted draft");
    assert!(!persisted_draft.contains(&local_path));
    assert!(!persisted_draft.contains(high_entropy_secret));
    assert!(persisted_draft.contains("<app-data-dir>"));
    assert!(persisted_draft.contains("<redacted-secret>"));

    let prompt_run_content =
        fs::read_to_string(host.llm_prompt_runs_path()).expect("prompt run content");
    assert!(!prompt_run_content.contains(&local_path));
    assert!(!prompt_run_content.contains(high_entropy_secret));
    assert!(prompt_run_content.contains("<app-data-dir>"));
    assert!(prompt_run_content.contains("<redacted-secret>"));

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_provider_observability_missing_files_returns_safe_empty_ready() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-observability-empty-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let response = host.handle(ServiceRequest {
        id: Some("provider-observability".to_string()),
        method: "llm.providerObservability".to_string(),
        params: Value::Null,
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("provider observability result");
    assert_eq!(
        result.get("generated_by").and_then(Value::as_str),
        Some("local-v2.64")
    );
    assert_eq!(result.get("status").and_then(Value::as_str), Some("ready"));
    assert_eq!(
        result
            .pointer("/summary/total_prompt_run_count")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        result
            .pointer("/summary/total_call_metadata_count")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        result
            .pointer("/call_rows")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        result
            .pointer("/history_rows")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        result
            .pointer("/safety_flags/provider_request_sent")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .pointer("/safety_flags/credential_accessed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .pointer("/safety_flags/skill_files_mutated")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .pointer("/safety_flags/agent_config_mutated")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .pointer("/safety_flags/snapshot_created")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .pointer("/safety_flags/triage_mutation_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .pointer("/safety_flags/script_execution_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .pointer("/safety_flags/cloud_sync_performed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .pointer("/safety_flags/telemetry_emitted")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        !app_data_dir.exists(),
        "provider observability must not initialize app data for absent files"
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_provider_observability_aggregates_seeded_metadata_and_preserves_privacy_boundary() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-observability-seeded-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    fs::create_dir_all(app_data_dir.join("llm")).expect("create llm app data");
    let raw_secret = "fixture-redacted-value";
    let local_path = app_data_dir
        .join("fixture-project")
        .join("SKILL.md")
        .to_string_lossy()
        .to_string();

    let run = LlmPromptRunRecord {
        id: "prompt-run-fixture".to_string(),
        preview_id: "preview-fixture".to_string(),
        confirmation_id: "confirm-fixture".to_string(),
        action: "analyze".to_string(),
        request_kind: "analyze".to_string(),
        analysis_kind: None,
        scope: Some("selected".to_string()),
        instance_id: Some("fixture-skill".to_string()),
        instance_ids: vec!["fixture-skill".to_string()],
        definition_id: Some("fixture-definition".to_string()),
        agent: Some("codex".to_string()),
        task: Some(format!("Review token={raw_secret} at {local_path}")),
        profile_id: "fixture-openai".to_string(),
        provider: "openai-compatible".to_string(),
        model: "fixture-model".to_string(),
        destination_host: "api.fixture.invalid".to_string(),
        status: "succeeded".to_string(),
        error_code: None,
        error_message: None,
        duration_ms: 42,
        estimated_input_tokens: 120,
        estimated_output_tokens: 40,
        estimated_total_tokens: 160,
        estimated_cost_usd: 0.02,
        draft_output: Some(format!("Draft with {raw_secret} and {local_path}")),
        draft_requires_user_copy: true,
        provider_request_sent: true,
        credential_accessed: true,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        redaction_summary: LlmPromptRunRedactionSummary {
            status: "redacted-local-only".to_string(),
            redacted_value_count: 2,
            redacted_fields: vec!["local paths".to_string()],
            placeholders: vec![
                "$HOME".to_string(),
                "<app-data-dir>".to_string(),
                "<redacted>".to_string(),
            ],
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
            raw_secret_returned: false,
        },
        created_at: 2_000,
        completed_at: 2_100,
        safety_flags: llm_prompt_run_safety_flags(true, true),
    };
    host.save_llm_prompt_runs(&[run])
        .expect("save seeded prompt run");

    let metadata = ProviderCallMetadata {
        timestamp: 2_200,
        action_type: "analyze".to_string(),
        profile_id: "fixture-openai".to_string(),
        provider_type: provider::ProviderType::OpenAiCompatible,
        model: "fixture-model".to_string(),
        destination_host: "api.fixture.invalid".to_string(),
        status: "failed".to_string(),
        error_code: Some("network_error".to_string()),
        error_message: Some(format!(
            "Authorization: Bearer {raw_secret}; path={local_path}"
        )),
        duration_ms: 7,
        estimated_input_tokens: 20,
        estimated_output_tokens: 10,
        estimated_cost_usd: 0.03,
        confirmation_id: "confirm-fixture".to_string(),
        redaction_status: "metadata-only-no-raw-prompt-or-response".to_string(),
        provider_request_sent: true,
        credential_accessed: true,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
    };
    let metadata_line = serde_json::to_string(&metadata).expect("serialize metadata");
    fs::write(
        provider_call_metadata_path(&app_data_dir),
        format!("{metadata_line}\n"),
    )
    .expect("write provider metadata");

    fs::write(
        provider_profiles_path(&app_data_dir),
        serde_json::to_string_pretty(&json!({
            "version": 1,
            "default_profile_id": "fixture-openai",
            "profiles": [
                {
                    "id": "fixture-openai",
                    "display_name": "Fixture OpenAI",
                    "provider_type": "openai-compatible",
                    "base_url": "https://api.fixture.invalid/v1",
                    "model": "fixture-model",
                    "enabled": true,
                    "api_version": null,
                    "organization": null,
                    "single_request_token_limit": 4096,
                    "monthly_budget_usd": 1.0,
                    "credential_reference": {
                        "storage": "keychain",
                        "service": "dev.skills-copilot.native.llm",
                        "account": "provider:fixture-openai",
                        "secret_persisted": false
                    },
                    "credential_status": {
                        "state": "missing",
                        "reason": "seeded fixture metadata only",
                        "secret_available": false,
                        "fallback_available": false
                    },
                    "created_at": 1,
                    "updated_at": 1
                }
            ]
        }))
        .expect("serialize provider profiles"),
    )
    .expect("write provider profiles");

    let response = host.handle(ServiceRequest {
        id: Some("provider-observability".to_string()),
        method: "llm.providerObservability".to_string(),
        params: json!({ "limit": 10 }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("provider observability result");
    assert_eq!(
        result
            .pointer("/summary/total_prompt_run_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        result
            .pointer("/summary/total_call_metadata_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        result
            .pointer("/summary/provider_profile_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        result
            .pointer("/summary/succeeded_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        result
            .pointer("/summary/failed_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        result
            .pointer("/history_rows/0/draft_output_available")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        result.pointer("/history_rows/0/draft_output").is_none(),
        "observability must not return provider draft text"
    );
    assert_eq!(
        result
            .pointer("/history_rows/0/recorded_provider_request_sent")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        result
            .pointer("/call_rows/0/recorded_credential_accessed")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        result
            .pointer("/budget_usage_hints/0/budget_state")
            .and_then(Value::as_str),
        Some("within_configured_budget_hint")
    );
    assert_eq!(
        result
            .pointer("/safety_flags/provider_request_sent")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .pointer("/safety_flags/credential_accessed")
            .and_then(Value::as_bool),
        Some(false)
    );

    let serialized = serde_json::to_string(&result).expect("serialize result");
    assert!(!serialized.contains(raw_secret));
    assert!(!serialized.contains(&local_path));
    assert!(!serialized.contains("Draft with"));
    assert!(!serialized.contains("Bearer"));
    assert!(!serialized.contains("\"api_key\""));
    assert!(!serialized.contains("\"credential_reference\""));
    assert!(serialized.contains("<redacted>"));
    assert!(serialized.contains("<app-data-dir>"));

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_list_prompt_runs_returns_full_history_without_limit_and_reports_limited_page() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-llm-list-runs-full-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    fs::create_dir_all(app_data_dir.join("llm")).expect("create llm app data");

    let base_run = LlmPromptRunRecord {
        id: "prompt-run-base".to_string(),
        preview_id: "preview-base".to_string(),
        confirmation_id: "confirm-base".to_string(),
        action: "analyze".to_string(),
        request_kind: "analyze".to_string(),
        analysis_kind: None,
        scope: Some("selected".to_string()),
        instance_id: Some("fixture-skill".to_string()),
        instance_ids: vec!["fixture-skill".to_string()],
        definition_id: Some("fixture-definition".to_string()),
        agent: Some("codex".to_string()),
        task: Some("Review prompt history pagination.".to_string()),
        profile_id: "fixture-openai".to_string(),
        provider: "openai-compatible".to_string(),
        model: "fixture-model".to_string(),
        destination_host: "api.fixture.invalid".to_string(),
        status: "succeeded".to_string(),
        error_code: None,
        error_message: None,
        duration_ms: 10,
        estimated_input_tokens: 6,
        estimated_output_tokens: 4,
        estimated_total_tokens: 10,
        estimated_cost_usd: 0.01,
        draft_output: None,
        draft_requires_user_copy: true,
        provider_request_sent: true,
        credential_accessed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        redaction_summary: LlmPromptRunRedactionSummary {
            status: "redacted-local-only".to_string(),
            redacted_value_count: 0,
            redacted_fields: Vec::new(),
            placeholders: Vec::new(),
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
            raw_secret_returned: false,
        },
        created_at: 900,
        completed_at: 1_000,
        safety_flags: llm_prompt_run_safety_flags(true, false),
    };
    let runs = (0..60)
        .map(|index| LlmPromptRunRecord {
            id: format!("prompt-run-{index:02}"),
            preview_id: format!("preview-{index:02}"),
            confirmation_id: format!("confirm-{index:02}"),
            task: Some(format!("Review prompt history item {index:02}.")),
            created_at: 1_000 + index,
            completed_at: 1_100 + index,
            ..base_run.clone()
        })
        .collect::<Vec<_>>();
    host.save_llm_prompt_runs(&runs).expect("save prompt runs");

    let full_response = host.handle(ServiceRequest {
        id: Some("prompt-runs-full".to_string()),
        method: "llm.listPromptRuns".to_string(),
        params: json!({}),
    });
    assert!(full_response.ok, "{:?}", full_response.error);
    let full = full_response.result.expect("full prompt runs");
    assert_eq!(full.get("count").and_then(Value::as_u64), Some(60));
    assert_eq!(full.get("total_count").and_then(Value::as_u64), Some(60));
    assert_eq!(full.get("returned_count").and_then(Value::as_u64), Some(60));
    assert_eq!(full.get("limit"), None);
    assert_eq!(full.get("truncated").and_then(Value::as_bool), Some(false));
    assert_eq!(
        full.get("runs").and_then(Value::as_array).map(Vec::len),
        Some(60)
    );

    let limited_response = host.handle(ServiceRequest {
        id: Some("prompt-runs-limited".to_string()),
        method: "llm.listPromptRuns".to_string(),
        params: json!({ "limit": 5 }),
    });
    assert!(limited_response.ok, "{:?}", limited_response.error);
    let limited = limited_response.result.expect("limited prompt runs");
    assert_eq!(limited.get("count").and_then(Value::as_u64), Some(5));
    assert_eq!(limited.get("total_count").and_then(Value::as_u64), Some(60));
    assert_eq!(
        limited.get("returned_count").and_then(Value::as_u64),
        Some(5)
    );
    assert_eq!(limited.get("limit").and_then(Value::as_u64), Some(5));
    assert_eq!(
        limited.get("truncated").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        limited.get("runs").and_then(Value::as_array).map(Vec::len),
        Some(5)
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_provider_observability_aggregates_full_date_range_before_row_limit() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-observability-range-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    fs::create_dir_all(app_data_dir.join("llm")).expect("create llm app data");

    let base_run = LlmPromptRunRecord {
        id: "prompt-run-base".to_string(),
        preview_id: "preview-base".to_string(),
        confirmation_id: "confirm-base".to_string(),
        action: "analyze".to_string(),
        request_kind: "analyze".to_string(),
        analysis_kind: None,
        scope: Some("selected".to_string()),
        instance_id: Some("fixture-skill".to_string()),
        instance_ids: vec!["fixture-skill".to_string()],
        definition_id: Some("fixture-definition".to_string()),
        agent: Some("codex".to_string()),
        task: Some("Review provider observability aggregation.".to_string()),
        profile_id: "fixture-openai".to_string(),
        provider: "openai-compatible".to_string(),
        model: "visible-model".to_string(),
        destination_host: "api.fixture.invalid".to_string(),
        status: "succeeded".to_string(),
        error_code: None,
        error_message: None,
        duration_ms: 10,
        estimated_input_tokens: 6,
        estimated_output_tokens: 4,
        estimated_total_tokens: 10,
        estimated_cost_usd: 0.01,
        draft_output: None,
        draft_requires_user_copy: true,
        provider_request_sent: true,
        credential_accessed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        redaction_summary: LlmPromptRunRedactionSummary {
            status: "redacted-local-only".to_string(),
            redacted_value_count: 0,
            redacted_fields: Vec::new(),
            placeholders: Vec::new(),
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
            raw_secret_returned: false,
        },
        created_at: 900,
        completed_at: 1_000,
        safety_flags: llm_prompt_run_safety_flags(true, false),
    };
    let run = |id: &str, model: &str, completed_at: i64| LlmPromptRunRecord {
        id: id.to_string(),
        preview_id: format!("preview-{id}"),
        confirmation_id: format!("confirm-{id}"),
        model: model.to_string(),
        created_at: completed_at - 10,
        completed_at,
        ..base_run.clone()
    };
    host.save_llm_prompt_runs(&[
        run("newest", "visible-model", 3_000),
        run("middle", "visible-model", 2_000),
        run("range-only", "full-range-only-model", 1_000),
        run("old", "old-model", 100),
    ])
    .expect("save ranged prompt runs");

    let base_metadata = ProviderCallMetadata {
        timestamp: 1_100,
        action_type: "analyze".to_string(),
        profile_id: "fixture-openai".to_string(),
        provider_type: provider::ProviderType::OpenAiCompatible,
        model: "visible-model".to_string(),
        destination_host: "api.fixture.invalid".to_string(),
        status: "succeeded".to_string(),
        error_code: None,
        error_message: None,
        duration_ms: 10,
        estimated_input_tokens: 7,
        estimated_output_tokens: 3,
        estimated_cost_usd: 0.01,
        confirmation_id: "confirm-base".to_string(),
        redaction_status: "metadata-only-no-raw-prompt-or-response".to_string(),
        provider_request_sent: true,
        credential_accessed: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
    };
    let metadata = |id: &str, model: &str, timestamp: i64| ProviderCallMetadata {
        timestamp,
        model: model.to_string(),
        confirmation_id: format!("confirm-{id}"),
        ..base_metadata.clone()
    };
    let metadata_lines = [
        metadata("newest", "visible-model", 3_100),
        metadata("middle", "visible-model", 2_100),
        metadata("range-only", "full-range-only-model", 1_100),
        metadata("old", "old-model", 90),
    ]
    .into_iter()
    .map(|row| serde_json::to_string(&row).expect("serialize metadata"))
    .collect::<Vec<_>>()
    .join("\n");
    fs::write(
        provider_call_metadata_path(&app_data_dir),
        format!("{metadata_lines}\n"),
    )
    .expect("write provider metadata");

    let response = host.handle(ServiceRequest {
        id: Some("provider-observability-range".to_string()),
        method: "llm.providerObservability".to_string(),
        params: json!({
            "start_at": 1_000,
            "end_at": 3_200,
            "limit": 2
        }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("provider observability result");
    assert_eq!(
        result
            .pointer("/filters/aggregation_uses_full_range")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        result
            .pointer("/history_rows")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        result
            .pointer("/call_rows")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        result
            .pointer("/summary/returned_prompt_run_count")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        result
            .pointer("/summary/returned_call_row_count")
            .and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        result
            .pointer("/summary/succeeded_count")
            .and_then(Value::as_u64),
        Some(6)
    );
    assert_eq!(
        result
            .pointer("/summary/estimated_total_tokens")
            .and_then(Value::as_u64),
        Some(60)
    );
    assert!(
        result
            .pointer("/grouping_rows")
            .and_then(Value::as_array)
            .is_some_and(|rows| rows.iter().any(
                |row| row.get("model").and_then(Value::as_str) == Some("full-range-only-model")
            )),
        "grouping rows should include rows beyond the returned evidence limit"
    );
    assert!(
        !serde_json::to_string(&result)
            .expect("serialize result")
            .contains("old-model"),
        "date range should exclude old rows"
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn provider_activity_pages_unified_redacted_metadata_in_stable_order() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-activity-paged-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    fs::create_dir_all(app_data_dir.join("llm")).expect("create llm app data");
    let private_path = host
        .adapter_ctx
        .user_home
        .join("private-project")
        .join("SKILL.md");

    let prompt_runs = (0..55)
        .map(|index| {
            let completed_at = 20_000 - i64::from(index / 2);
            LlmPromptRunRecord {
                id: format!("activity-prompt-run-{index:03}"),
                preview_id: format!("activity-preview-{index:03}"),
                confirmation_id: format!("activity-confirm-{index:03}"),
                action: "analyze".to_string(),
                request_kind: "analyze".to_string(),
                analysis_kind: None,
                scope: Some("selected".to_string()),
                instance_id: Some(format!("activity-skill-{index:03}")),
                instance_ids: vec![format!("activity-skill-{index:03}")],
                definition_id: Some("activity-definition".to_string()),
                agent: Some("codex".to_string()),
                task: Some(format!(
                    "Review activity {index:03} at {}",
                    private_path.display()
                )),
                profile_id: "fixture-openai".to_string(),
                provider: "openai-compatible".to_string(),
                model: "fixture-model".to_string(),
                destination_host: "api.fixture.invalid".to_string(),
                status: "succeeded".to_string(),
                error_code: None,
                error_message: None,
                duration_ms: 10,
                estimated_input_tokens: 6,
                estimated_output_tokens: 4,
                estimated_total_tokens: 10,
                estimated_cost_usd: 0.01,
                draft_output: None,
                draft_requires_user_copy: true,
                provider_request_sent: true,
                credential_accessed: true,
                raw_secret_returned: false,
                raw_prompt_persisted: false,
                raw_response_persisted: false,
                redaction_summary: LlmPromptRunRedactionSummary {
                    status: "redacted-local-only".to_string(),
                    redacted_value_count: 1,
                    redacted_fields: vec!["local paths".to_string()],
                    placeholders: vec!["$HOME".to_string()],
                    raw_prompt_persisted: false,
                    raw_response_persisted: false,
                    raw_trace_persisted: false,
                    raw_secret_returned: false,
                },
                created_at: completed_at - 10,
                completed_at,
                safety_flags: llm_prompt_run_safety_flags(true, true),
            }
        })
        .collect::<Vec<_>>();
    host.save_llm_prompt_runs(&prompt_runs)
        .expect("save activity prompt runs");

    let provider_calls = (0..75)
        .map(|index| ProviderCallMetadata {
            timestamp: 20_000 - i64::from(index / 2),
            action_type: "analyze".to_string(),
            profile_id: "fixture-openai".to_string(),
            provider_type: provider::ProviderType::OpenAiCompatible,
            model: "fixture-model".to_string(),
            destination_host: "api.fixture.invalid".to_string(),
            status: "succeeded".to_string(),
            error_code: None,
            error_message: Some(format!(
                "Metadata {index:03} from {}",
                private_path.display()
            )),
            duration_ms: 8,
            estimated_input_tokens: 5,
            estimated_output_tokens: 3,
            estimated_cost_usd: 0.01,
            confirmation_id: format!("activity-call-confirm-{index:03}"),
            redaction_status: "metadata-only-no-raw-prompt-or-response".to_string(),
            provider_request_sent: true,
            credential_accessed: true,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
        })
        .map(|row| serde_json::to_string(&row).expect("serialize activity metadata"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        provider_call_metadata_path(&app_data_dir),
        format!("{provider_calls}\n"),
    )
    .expect("write activity metadata");

    let mut rows = Vec::new();
    let mut cursor = None;
    let mut source_revision = None;
    let final_page = loop {
        let response = host.handle(ServiceRequest {
            id: Some("provider-activity-page".to_string()),
            method: "llm.listProviderActivity".to_string(),
            params: json!({
                "provider": "openai-compatible",
                "model": "fixture-model",
                "action": "analyze",
                "start_at": 19_000,
                "end_at": 21_000,
                "limit": 50,
                "cursor": cursor,
                "source_revision": source_revision,
            }),
        });
        assert!(response.ok, "{:?}", response.error);
        let page = response.result.expect("provider activity page");
        rows.extend(
            page.get("rows")
                .and_then(Value::as_array)
                .expect("activity rows")
                .iter()
                .cloned(),
        );
        cursor = page
            .get("next_cursor")
            .and_then(Value::as_str)
            .map(str::to_string);
        source_revision = page
            .get("source_revision")
            .and_then(Value::as_str)
            .map(str::to_string);
        if cursor.is_none() {
            break page;
        }
    };

    assert_eq!(rows.len(), 130);
    assert_eq!(
        rows.iter()
            .filter_map(|row| row.get("id").and_then(Value::as_str))
            .collect::<std::collections::HashSet<_>>()
            .len(),
        130
    );
    assert!(rows.windows(2).all(|pair| {
        let left_timestamp = pair[0]["timestamp"].as_i64().expect("left timestamp");
        let right_timestamp = pair[1]["timestamp"].as_i64().expect("right timestamp");
        let left_id = pair[0]["id"].as_str().expect("left id");
        let right_id = pair[1]["id"].as_str().expect("right id");
        left_timestamp > right_timestamp
            || (left_timestamp == right_timestamp && left_id <= right_id)
    }));
    let serialized = serde_json::to_string(&rows).expect("serialize activity rows");
    assert!(!serialized.contains(private_path.to_string_lossy().as_ref()));
    assert!(!serialized.contains("Authorization: Bearer"));
    assert_eq!(
        final_page
            .pointer("/safety_flags/provider_request_sent")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        final_page
            .pointer("/safety_flags/raw_prompt_persisted")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        final_page
            .pointer("/safety_flags/raw_response_persisted")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        final_page
            .pointer("/safety_flags/raw_trace_persisted")
            .and_then(Value::as_bool),
        Some(false)
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn provider_activity_cursor_binds_filters_and_rejects_source_mutation() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-activity-source-change-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    fs::create_dir_all(app_data_dir.join("llm")).expect("create llm app data");
    let metadata = |timestamp: i64, confirmation_id: &str| ProviderCallMetadata {
        timestamp,
        action_type: "analyze".to_string(),
        profile_id: "fixture-openai".to_string(),
        provider_type: provider::ProviderType::OpenAiCompatible,
        model: "fixture-model".to_string(),
        destination_host: "api.fixture.invalid".to_string(),
        status: "succeeded".to_string(),
        error_code: None,
        error_message: None,
        duration_ms: 8,
        estimated_input_tokens: 5,
        estimated_output_tokens: 3,
        estimated_cost_usd: 0.01,
        confirmation_id: confirmation_id.to_string(),
        redaction_status: "metadata-only-no-raw-prompt-or-response".to_string(),
        provider_request_sent: true,
        credential_accessed: true,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
    };
    let initial_lines = (0..3)
        .map(|index| metadata(2_000 - index, &format!("confirm-{index}")))
        .map(|row| serde_json::to_string(&row).expect("serialize metadata"))
        .collect::<Vec<_>>()
        .join("\n");
    let metadata_path = provider_call_metadata_path(&app_data_dir);
    fs::write(&metadata_path, format!("{initial_lines}\n")).expect("write metadata");

    let first = host.handle(ServiceRequest {
        id: Some("provider-activity-first".to_string()),
        method: "llm.listProviderActivity".to_string(),
        params: json!({
            "provider": "openai-compatible",
            "model": "fixture-model",
            "action": "analyze",
            "limit": 1
        }),
    });
    assert!(first.ok, "{:?}", first.error);
    let first = first.result.expect("first activity page");
    let cursor = first["next_cursor"].as_str().expect("next cursor");
    let source_revision = first["source_revision"].as_str().expect("source revision");

    let mismatched_filter = host.handle(ServiceRequest {
        id: Some("provider-activity-filter-mismatch".to_string()),
        method: "llm.listProviderActivity".to_string(),
        params: json!({
            "provider": "openai-compatible",
            "model": "other-model",
            "action": "analyze",
            "limit": 1,
            "cursor": cursor,
            "source_revision": source_revision
        }),
    });
    assert!(!mismatched_filter.ok);
    assert_eq!(
        mismatched_filter.error.expect("filter mismatch error").code,
        "invalid_request"
    );

    let mut content = fs::read_to_string(&metadata_path).expect("read metadata");
    content = content.replacen("confirm-0", "confirm-mutated", 1);
    fs::write(&metadata_path, content).expect("mutate non-visible metadata");

    let changed = host.handle(ServiceRequest {
        id: Some("provider-activity-source-changed".to_string()),
        method: "llm.listProviderActivity".to_string(),
        params: json!({
            "provider": "openai-compatible",
            "model": "fixture-model",
            "action": "analyze",
            "limit": 1000,
            "cursor": cursor,
            "source_revision": source_revision
        }),
    });
    assert!(!changed.ok);
    assert_eq!(
        changed.error.expect("source changed error").code,
        "source_changed"
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn provider_activity_snapshot_retries_mixed_source_window_or_returns_source_changed() {
    use std::collections::VecDeque;

    let mut reads = VecDeque::from([
        (ProviderActivitySource::PromptRuns, b"prompt-old".to_vec()),
        (ProviderActivitySource::ProviderCalls, b"calls-old".to_vec()),
        (ProviderActivitySource::PromptRuns, b"prompt-new".to_vec()),
        (ProviderActivitySource::ProviderCalls, b"calls-old".to_vec()),
        (ProviderActivitySource::PromptRuns, b"prompt-new".to_vec()),
        (ProviderActivitySource::ProviderCalls, b"calls-new".to_vec()),
        (ProviderActivitySource::PromptRuns, b"prompt-new".to_vec()),
        (ProviderActivitySource::ProviderCalls, b"calls-new".to_vec()),
    ]);
    let snapshot = read_consistent_provider_activity_raw_snapshot_with(|source| {
        let (expected, bytes) = reads.pop_front().expect("scheduled activity source read");
        assert_eq!(
            source, expected,
            "activity sources must use a fixed read order"
        );
        Ok(ProviderActivityRawSource::present(bytes))
    })
    .expect("a stable retry should produce one source snapshot");
    assert_eq!(snapshot.prompt_runs.bytes, b"prompt-new");
    assert_eq!(snapshot.provider_calls.bytes, b"calls-new");
    assert!(reads.is_empty());

    let mut read_number = 0usize;
    let error = read_consistent_provider_activity_raw_snapshot_with(|source| {
        read_number += 1;
        Ok(ProviderActivityRawSource::present(
            format!("{source:?}-{read_number}").into_bytes(),
        ))
    })
    .expect_err("a source that never stabilizes must fail closed");
    assert_eq!(error.code(), "source_changed");
}

#[cfg(unix)]
#[test]
fn provider_activity_source_reader_rejects_symlinks_and_nonregular_files() {
    use std::os::unix::{fs::symlink, net::UnixListener};

    let root = env::temp_dir().join(format!(
        "ac-pa-shape-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    fs::create_dir_all(&root).expect("create source-shape fixture root");
    let target = root.join("target.json");
    fs::write(&target, b"[]").expect("write symlink target");
    let source = root.join("activity-source");
    symlink(&target, &source).expect("create activity source symlink");

    let symlink_error = read_provider_activity_raw_source(&source, "prompt-runs")
        .expect_err("activity source symlinks must be rejected by the opened-handle reader");
    assert_eq!(symlink_error.code(), "provider_activity_source_invalid");

    fs::remove_file(&source).expect("remove activity source symlink");
    let listener = UnixListener::bind(&source).expect("create nonregular source socket");
    let nonregular_error = read_provider_activity_raw_source(&source, "prompt-runs")
        .expect_err("nonregular activity sources must be rejected without blocking");
    assert_eq!(nonregular_error.code(), "provider_activity_source_invalid");

    drop(listener);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn provider_activity_source_reader_rejects_files_over_eight_mib() {
    let root = env::temp_dir().join(format!(
        "skills-copilot-provider-activity-source-size-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    fs::create_dir_all(&root).expect("create source-size fixture root");
    let source = root.join("activity-source.json");
    let file = fs::File::create(&source).expect("create oversized activity source");
    file.set_len((PROVIDER_ACTIVITY_MAX_SOURCE_BYTES + 1) as u64)
        .expect("declare oversized activity source length");
    drop(file);

    let error = read_provider_activity_raw_source(&source, "prompt-runs")
        .expect_err("activity sources over eight MiB must fail closed");
    assert_eq!(error.code(), "provider_activity_source_invalid");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn provider_activity_bounded_reader_consumes_at_most_max_plus_one_bytes() {
    use std::io::{self, Read};

    struct CountingReader {
        bytes_read: usize,
        remaining: usize,
    }

    impl Read for CountingReader {
        fn read(&mut self, buffer: &mut [u8]) -> io::Result<usize> {
            let count = buffer.len().min(self.remaining);
            buffer[..count].fill(b'x');
            self.bytes_read += count;
            self.remaining -= count;
            Ok(count)
        }
    }

    let mut reader = CountingReader {
        bytes_read: 0,
        remaining: PROVIDER_ACTIVITY_MAX_SOURCE_BYTES * 4,
    };
    let error = read_provider_activity_bounded(&mut reader, "prompt-runs")
        .expect_err("MAX+1 bytes must reject a growing or oversized activity source");
    assert_eq!(error.code(), "provider_activity_source_invalid");
    assert_eq!(
        reader.bytes_read,
        PROVIDER_ACTIVITY_MAX_SOURCE_BYTES + 1,
        "the reader must never consume an unbounded source after opening it"
    );
}

#[test]
fn provider_activity_corrupt_sources_fail_closed_without_enumerable_page() {
    for (label, seed) in [
        ("prompt-runs", ("prompt-runs.json", "{not-valid-json\n")),
        (
            "provider-calls",
            (
                "llm/provider-call-metadata.jsonl",
                "{\"status\":\"succeeded\"}\nnot-json\n",
            ),
        ),
    ] {
        let app_data_dir = env::temp_dir().join(format!(
            "skills-copilot-provider-activity-corrupt-{label}-{}-{}",
            std::process::id(),
            unique_suffix(),
        ));
        let host = test_host(app_data_dir.clone());
        let path = app_data_dir.join(seed.0);
        fs::create_dir_all(path.parent().expect("corrupt source parent"))
            .expect("create corrupt source parent");
        fs::write(path, seed.1).expect("write corrupt activity source");

        let response = host.handle(ServiceRequest {
            id: Some(format!("provider-activity-corrupt-{label}")),
            method: "llm.listProviderActivity".to_string(),
            params: json!({ "limit": 50 }),
        });
        assert!(
            !response.ok,
            "corrupt {label} must not return an exact page"
        );
        let error = response.error.expect("corrupt activity source error");
        assert_eq!(error.code, "provider_activity_source_invalid");
        assert!(!error.message.contains("not-valid-json"));
        assert!(!error.message.contains("not-json"));

        let _ = fs::remove_dir_all(app_data_dir);
    }
}

#[test]
fn provider_activity_revision_hashes_complete_raw_source_bytes() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-activity-raw-revision-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    fs::create_dir_all(app_data_dir.join("llm")).expect("create llm app data");
    let rows = (0..3)
        .map(|index| ProviderCallMetadata {
            timestamp: 3_000 - index,
            action_type: "analyze".to_string(),
            profile_id: "fixture-openai".to_string(),
            provider_type: provider::ProviderType::OpenAiCompatible,
            model: "fixture-model".to_string(),
            destination_host: "api.fixture.invalid".to_string(),
            status: "succeeded".to_string(),
            error_code: None,
            error_message: None,
            duration_ms: 8,
            estimated_input_tokens: 5,
            estimated_output_tokens: 3,
            estimated_cost_usd: 0.01,
            confirmation_id: format!("raw-revision-{index}"),
            redaction_status: "metadata-only-no-raw-prompt-or-response".to_string(),
            provider_request_sent: true,
            credential_accessed: true,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
        })
        .map(|row| serde_json::to_string(&row).expect("serialize metadata"))
        .collect::<Vec<_>>()
        .join("\n");
    let metadata_path = provider_call_metadata_path(&app_data_dir);
    fs::write(&metadata_path, format!("{rows}\n")).expect("write provider metadata");

    let first = host.handle(ServiceRequest {
        id: Some("provider-activity-raw-first".to_string()),
        method: "llm.listProviderActivity".to_string(),
        params: json!({ "limit": 1 }),
    });
    assert!(first.ok, "{:?}", first.error);
    let first = first.result.expect("first raw revision page");
    let cursor = first["next_cursor"].as_str().expect("activity cursor");
    let revision = first["source_revision"]
        .as_str()
        .expect("activity revision");

    let mut raw = fs::read(&metadata_path).expect("read provider metadata bytes");
    raw.extend_from_slice(b" \n");
    fs::write(&metadata_path, raw).expect("change only raw whitespace bytes");

    let continuation = host.handle(ServiceRequest {
        id: Some("provider-activity-raw-continuation".to_string()),
        method: "llm.listProviderActivity".to_string(),
        params: json!({
            "limit": 1,
            "cursor": cursor,
            "source_revision": revision
        }),
    });
    assert!(!continuation.ok);
    assert_eq!(
        continuation.error.expect("raw mutation error").code,
        "source_changed"
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn provider_activity_ids_are_stable_across_filters_windows_and_front_inserts() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-activity-stable-ids-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    fs::create_dir_all(app_data_dir.join("llm")).expect("create llm app data");
    let prompt_run = |id: &str, action: &str, task: &str, completed_at: i64| LlmPromptRunRecord {
        id: id.to_string(),
        preview_id: format!("preview-{id}"),
        confirmation_id: format!("confirm-{id}"),
        action: action.to_string(),
        request_kind: action.to_string(),
        analysis_kind: None,
        scope: Some("selected".to_string()),
        instance_id: Some("fixture-skill".to_string()),
        instance_ids: vec!["fixture-skill".to_string()],
        definition_id: Some("fixture-definition".to_string()),
        agent: Some("codex".to_string()),
        task: Some(task.to_string()),
        profile_id: "fixture-openai".to_string(),
        provider: "openai-compatible".to_string(),
        model: "fixture-model".to_string(),
        destination_host: "api.fixture.invalid".to_string(),
        status: "succeeded".to_string(),
        error_code: None,
        error_message: None,
        duration_ms: 10,
        estimated_input_tokens: 6,
        estimated_output_tokens: 4,
        estimated_total_tokens: 10,
        estimated_cost_usd: 0.01,
        draft_output: None,
        draft_requires_user_copy: true,
        provider_request_sent: true,
        credential_accessed: true,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        redaction_summary: LlmPromptRunRedactionSummary {
            status: "redacted-local-only".to_string(),
            redacted_value_count: 0,
            redacted_fields: Vec::new(),
            placeholders: Vec::new(),
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
            raw_secret_returned: false,
        },
        created_at: completed_at - 10,
        completed_at,
        safety_flags: llm_prompt_run_safety_flags(true, true),
    };
    let provider_call =
        |confirmation_id: &str, action: &str, timestamp: i64| ProviderCallMetadata {
            timestamp,
            action_type: action.to_string(),
            profile_id: "fixture-openai".to_string(),
            provider_type: provider::ProviderType::OpenAiCompatible,
            model: "fixture-model".to_string(),
            destination_host: "api.fixture.invalid".to_string(),
            status: "succeeded".to_string(),
            error_code: None,
            error_message: None,
            duration_ms: 8,
            estimated_input_tokens: 5,
            estimated_output_tokens: 3,
            estimated_cost_usd: 0.01,
            confirmation_id: confirmation_id.to_string(),
            redaction_status: "metadata-only-no-raw-prompt-or-response".to_string(),
            provider_request_sent: true,
            credential_accessed: true,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
        };
    let target_prompt = prompt_run(
        "shared-stable-id",
        "analyze",
        "Target prompt activity",
        2_000,
    );
    let front_prompt = prompt_run("front-prompt", "recommend", "Front prompt", 3_000);
    host.save_llm_prompt_runs(&[front_prompt.clone(), target_prompt.clone()])
        .expect("save initial prompt rows");
    let target_call = provider_call("shared-stable-id", "analyze", 2_100);
    let front_call = provider_call("front-call", "recommend", 3_100);
    let write_calls = |rows: &[ProviderCallMetadata]| {
        let content = rows
            .iter()
            .map(|row| serde_json::to_string(row).expect("serialize call row"))
            .collect::<Vec<_>>()
            .join("\n");
        fs::write(
            provider_call_metadata_path(&app_data_dir),
            format!("{content}\n"),
        )
        .expect("write call rows");
    };
    write_calls(&[front_call.clone(), target_call.clone()]);

    let request_rows = |params: Value| {
        let response = host.handle(ServiceRequest {
            id: Some("provider-activity-stable-ids".to_string()),
            method: "llm.listProviderActivity".to_string(),
            params,
        });
        assert!(response.ok, "{:?}", response.error);
        response.result.expect("stable id result")["rows"]
            .as_array()
            .expect("stable id rows")
            .clone()
    };
    let target_ids = |rows: &[Value]| {
        let prompt_id = rows
            .iter()
            .find(|row| row["kind"] == "prompt_run" && row["timestamp"].as_i64() == Some(2_000))
            .and_then(|row| row["id"].as_str())
            .expect("target prompt id")
            .to_string();
        let call_id = rows
            .iter()
            .find(|row| row["kind"] == "provider_call" && row["timestamp"].as_i64() == Some(2_100))
            .and_then(|row| row["id"].as_str())
            .expect("target provider call id")
            .to_string();
        (prompt_id, call_id)
    };

    let all_rows = request_rows(json!({ "limit": 100 }));
    let original_ids = target_ids(&all_rows);
    let filtered_rows = request_rows(json!({
        "action": "analyze",
        "start_at": 1_900,
        "end_at": 2_200,
        "limit": 100
    }));
    assert_eq!(target_ids(&filtered_rows), original_ids);
    assert_ne!(
        original_ids.0, original_ids.1,
        "source prefixes prevent collisions"
    );

    let inserted_prompt = prompt_run("inserted-prompt", "analyze", "Inserted prompt", 4_000);
    host.save_llm_prompt_runs(&[inserted_prompt, front_prompt, target_prompt])
        .expect("insert prompt before target");
    let inserted_call = provider_call("inserted-call", "analyze", 4_100);
    write_calls(&[inserted_call, front_call, target_call]);
    let after_insert = request_rows(json!({ "limit": 100 }));
    assert_eq!(target_ids(&after_insert), original_ids);
    assert_eq!(
        after_insert
            .iter()
            .filter_map(|row| row["id"].as_str())
            .collect::<std::collections::HashSet<_>>()
            .len(),
        after_insert.len(),
        "unified activity IDs must be unique across both sources"
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

fn provider_activity_id_fixture_prompt(
    id: &str,
    action: &str,
    task: &str,
    completed_at: i64,
) -> LlmPromptRunRecord {
    LlmPromptRunRecord {
        id: id.to_string(),
        preview_id: format!("preview-{task}"),
        confirmation_id: format!("confirm-{task}"),
        action: action.to_string(),
        request_kind: action.to_string(),
        analysis_kind: None,
        scope: Some("selected".to_string()),
        instance_id: Some("fixture-skill".to_string()),
        instance_ids: vec!["fixture-skill".to_string()],
        definition_id: Some("fixture-definition".to_string()),
        agent: Some("codex".to_string()),
        task: Some(task.to_string()),
        profile_id: "fixture-openai".to_string(),
        provider: "openai-compatible".to_string(),
        model: "fixture-model".to_string(),
        destination_host: "api.fixture.invalid".to_string(),
        status: "succeeded".to_string(),
        error_code: None,
        error_message: None,
        duration_ms: 10,
        estimated_input_tokens: 6,
        estimated_output_tokens: 4,
        estimated_total_tokens: 10,
        estimated_cost_usd: 0.01,
        draft_output: None,
        draft_requires_user_copy: true,
        provider_request_sent: true,
        credential_accessed: true,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        redaction_summary: LlmPromptRunRedactionSummary {
            status: "redacted-local-only".to_string(),
            redacted_value_count: 0,
            redacted_fields: Vec::new(),
            placeholders: Vec::new(),
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
            raw_secret_returned: false,
        },
        created_at: completed_at - 10,
        completed_at,
        safety_flags: llm_prompt_run_safety_flags(true, true),
    }
}

fn provider_activity_id_fixture_call(
    confirmation_id: &str,
    action: &str,
    timestamp: i64,
) -> ProviderCallMetadata {
    ProviderCallMetadata {
        timestamp,
        action_type: action.to_string(),
        profile_id: "fixture-openai".to_string(),
        provider_type: provider::ProviderType::OpenAiCompatible,
        model: "fixture-model".to_string(),
        destination_host: "api.fixture.invalid".to_string(),
        status: "succeeded".to_string(),
        error_code: None,
        error_message: None,
        duration_ms: 8,
        estimated_input_tokens: 5,
        estimated_output_tokens: 3,
        estimated_cost_usd: 0.01,
        confirmation_id: confirmation_id.to_string(),
        redaction_status: "metadata-only-no-raw-prompt-or-response".to_string(),
        provider_request_sent: true,
        credential_accessed: true,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
    }
}

fn write_provider_activity_id_fixture_calls(app_data_dir: &Path, rows: &[ProviderCallMetadata]) {
    fs::create_dir_all(app_data_dir.join("llm")).expect("create provider call fixture parent");
    let content = rows
        .iter()
        .map(|row| serde_json::to_string(row).expect("serialize provider call fixture"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(
        provider_call_metadata_path(app_data_dir),
        format!("{content}\n"),
    )
    .expect("write provider call fixtures");
}

fn assert_provider_activity_duplicate_error(response: ServiceResponse, forbidden: &str) {
    assert!(
        !response.ok,
        "duplicate activity identities must fail closed"
    );
    let error = response.error.expect("duplicate activity identity error");
    assert_eq!(error.code, "provider_activity_source_invalid");
    assert!(!error.message.contains(forbidden));
}

#[test]
fn provider_activity_duplicate_prompt_intrinsic_ids_fail_before_filtering() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-activity-duplicate-prompt-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let duplicate_id = "duplicate-prompt-private-id";
    host.save_llm_prompt_runs(&[
        provider_activity_id_fixture_prompt(duplicate_id, "recommend", "first", 2_000),
        provider_activity_id_fixture_prompt(duplicate_id, "recommend", "second", 1_000),
    ])
    .expect("save duplicate prompt IDs");

    let response = host.handle(ServiceRequest {
        id: Some("provider-activity-duplicate-prompt".to_string()),
        method: "llm.listProviderActivity".to_string(),
        params: json!({ "action": "analyze", "limit": 100 }),
    });
    assert_provider_activity_duplicate_error(response, duplicate_id);

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn provider_activity_duplicate_provider_confirmation_ids_fail_closed() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-activity-duplicate-call-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let duplicate_id = "duplicate-provider-private-id";
    write_provider_activity_id_fixture_calls(
        &app_data_dir,
        &[
            provider_activity_id_fixture_call(duplicate_id, "analyze", 2_000),
            provider_activity_id_fixture_call(duplicate_id, "recommend", 1_000),
        ],
    );

    let response = host.handle(ServiceRequest {
        id: Some("provider-activity-duplicate-call".to_string()),
        method: "llm.listProviderActivity".to_string(),
        params: json!({ "limit": 100 }),
    });
    assert_provider_activity_duplicate_error(response, duplicate_id);

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn provider_activity_duplicate_fallback_identities_fail_closed() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-activity-duplicate-fallback-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let row = provider_activity_id_fixture_prompt("", "analyze", "fallback-identity", 2_000);
    host.save_llm_prompt_runs(&[row.clone(), row])
        .expect("save duplicate fallback identities");

    let response = host.handle(ServiceRequest {
        id: Some("provider-activity-duplicate-fallback".to_string()),
        method: "llm.listProviderActivity".to_string(),
        params: json!({ "limit": 100 }),
    });
    assert_provider_activity_duplicate_error(response, "fallback-identity");

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn provider_activity_versioned_fallback_ids_survive_filters_and_front_inserts() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-activity-fallback-stability-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let target_prompt =
        provider_activity_id_fixture_prompt("", "analyze", "fallback-target", 2_000);
    let target_call = provider_activity_id_fixture_call("", "analyze", 2_100);
    host.save_llm_prompt_runs(std::slice::from_ref(&target_prompt))
        .expect("save fallback prompt");
    write_provider_activity_id_fixture_calls(&app_data_dir, std::slice::from_ref(&target_call));

    let request_rows = |params: Value| {
        let response = host.handle(ServiceRequest {
            id: Some("provider-activity-fallback-stability".to_string()),
            method: "llm.listProviderActivity".to_string(),
            params,
        });
        assert!(response.ok, "{:?}", response.error);
        response.result.expect("fallback activity result")["rows"]
            .as_array()
            .expect("fallback activity rows")
            .clone()
    };
    let target_ids = |rows: &[Value]| {
        let prompt = rows
            .iter()
            .find(|row| row["kind"] == "prompt_run" && row["timestamp"] == 2_000)
            .and_then(|row| row["id"].as_str())
            .expect("fallback prompt ID")
            .to_string();
        let call = rows
            .iter()
            .find(|row| row["kind"] == "provider_call" && row["timestamp"] == 2_100)
            .and_then(|row| row["id"].as_str())
            .expect("fallback provider-call ID")
            .to_string();
        (prompt, call)
    };

    let initial = target_ids(&request_rows(json!({ "limit": 100 })));
    assert!(initial
        .0
        .starts_with("provider-activity-prompt-run-fallback-v1-"));
    assert!(initial
        .1
        .starts_with("provider-activity-provider-call-fallback-v1-"));
    assert_ne!(initial.0, initial.1, "fallback IDs must be source-prefixed");
    assert_eq!(
        target_ids(&request_rows(json!({
            "action": "analyze",
            "start_at": 1_900,
            "end_at": 2_200,
            "limit": 100
        }))),
        initial
    );

    let front_prompt = provider_activity_id_fixture_prompt("front", "analyze", "front", 4_000);
    host.save_llm_prompt_runs(&[front_prompt, target_prompt])
        .expect("insert prompt ahead of fallback row");
    let front_call = provider_activity_id_fixture_call("front", "analyze", 4_100);
    write_provider_activity_id_fixture_calls(&app_data_dir, &[front_call, target_call]);
    assert_eq!(target_ids(&request_rows(json!({ "limit": 100 }))), initial);

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn provider_activity_rolling_window_is_fixed_across_continuation_clock_changes() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-activity-fixed-window-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    fs::create_dir_all(app_data_dir.join("llm")).expect("create llm app data");
    let metadata = |timestamp: i64, confirmation_id: &str| ProviderCallMetadata {
        timestamp,
        action_type: "analyze".to_string(),
        profile_id: "fixture-openai".to_string(),
        provider_type: provider::ProviderType::OpenAiCompatible,
        model: "fixture-model".to_string(),
        destination_host: "api.fixture.invalid".to_string(),
        status: "succeeded".to_string(),
        error_code: None,
        error_message: None,
        duration_ms: 8,
        estimated_input_tokens: 5,
        estimated_output_tokens: 3,
        estimated_cost_usd: 0.01,
        confirmation_id: confirmation_id.to_string(),
        redaction_status: "metadata-only-no-raw-prompt-or-response".to_string(),
        provider_request_sent: true,
        credential_accessed: true,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
    };
    let raw = [
        metadata(190_000_000, "fixed-window-new"),
        metadata(120_000_000, "fixed-window-boundary"),
        metadata(100_000_000, "fixed-window-old"),
    ]
    .into_iter()
    .map(|row| serde_json::to_string(&row).expect("serialize fixed-window metadata"))
    .collect::<Vec<_>>()
    .join("\n");
    fs::write(
        provider_call_metadata_path(&app_data_dir),
        format!("{raw}\n"),
    )
    .expect("write fixed-window metadata");

    let first_params = ListProviderActivityParams {
        window_days: Some(1),
        limit: Some(1),
        ..ListProviderActivityParams::default()
    };
    let first = host
        .list_provider_activity_at(first_params.clone(), 200_000_000)
        .expect("first fixed-window page");
    assert_eq!(first.page.total_count, Some(2));
    assert_eq!(first.rows[0].timestamp, 190_000_000);
    let cursor = first.page.next_cursor.clone().expect("fixed-window cursor");
    let revision = first.source_revision.clone();

    let continuation = host
        .list_provider_activity_at(
            ListProviderActivityParams {
                cursor: Some(cursor.clone()),
                source_revision: Some(revision.clone()),
                ..first_params.clone()
            },
            300_000_000,
        )
        .expect("continuation should reuse first-page bounds");
    assert_eq!(continuation.page.total_count, Some(2));
    assert_eq!(continuation.rows.len(), 1);
    assert_eq!(continuation.rows[0].timestamp, 120_000_000);

    let changed_window = host
        .list_provider_activity_at(
            ListProviderActivityParams {
                window_days: Some(2),
                cursor: Some(cursor),
                source_revision: Some(revision),
                limit: Some(1),
                ..ListProviderActivityParams::default()
            },
            300_000_000,
        )
        .expect_err("a changed rolling filter must reject the cursor");
    assert_eq!(changed_window.code(), "invalid_request");

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn model_task_matches_empty_list_is_safe_and_does_not_initialize_app_data() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-model-task-empty-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let response = host.handle(ServiceRequest {
        id: Some("model-task-list".to_string()),
        method: "llm.listModelTaskMatches".to_string(),
        params: Value::Null,
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("model task match list result");
    assert_eq!(
        result.get("generated_by").and_then(Value::as_str),
        Some("local-v2.91")
    );
    assert_eq!(
        result
            .pointer("/summary/stored_record_count")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        result
            .pointer("/summary/prompt_run_count")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        result
            .pointer("/safety_flags/provider_request_sent")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .pointer("/safety_flags/raw_prompt_persisted")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        !app_data_dir.exists(),
        "empty model-task history list must not initialize app data"
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn model_task_match_record_rejects_empty_task_or_model() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-model-task-invalid-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let missing_task = host.handle(ServiceRequest {
        id: Some("model-task-record-missing-task".to_string()),
        method: "llm.recordModelTaskMatch".to_string(),
        params: json!({ "task": " ", "model": "fixture-model" }),
    });
    assert!(!missing_task.ok);
    assert_eq!(
        missing_task.error.expect("missing task error").code,
        "invalid_request"
    );

    let missing_model = host.handle(ServiceRequest {
        id: Some("model-task-record-missing-model".to_string()),
        method: "llm.recordModelTaskMatch".to_string(),
        params: json!({ "task": "fixture task", "model": " " }),
    });
    assert!(!missing_model.ok);
    assert_eq!(
        missing_model.error.expect("missing model error").code,
        "invalid_request"
    );

    assert!(
        !app_data_dir.exists(),
        "invalid model-task record requests must not initialize app data"
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn model_task_match_record_redacts_and_writes_only_app_local_history() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-model-task-record-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let local_path = app_data_dir
        .join("fixture-project")
        .join("SKILL.md")
        .to_string_lossy()
        .to_string();
    let secret = "ABCDEF1234567890ABCDEF1234567890";

    let response = host.handle(ServiceRequest {
        id: Some("model-task-record".to_string()),
        method: "llm.recordModelTaskMatch".to_string(),
        params: json!({
            "id": "model-task-redaction",
            "title": format!("Review api_key {secret}"),
            "task": format!("Audit task at {local_path} api_key {secret}"),
            "task_kind": "task_cockpit",
            "provider": "openai-compatible",
            "model": "fixture-model",
            "destination_host": "https://api.fixture.invalid/v1",
            "match_status": "fit",
            "source_kind": "manual",
            "evidence_refs": [format!("path:{local_path}")],
            "outcome_notes": [format!("Observed api_key {secret}")]
        }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("model task record result");
    let serialized = serde_json::to_string(&result).expect("serialize result");
    assert!(!serialized.contains(&local_path));
    assert!(!serialized.contains(secret));
    assert!(serialized.contains("<app-data-dir>"));
    assert!(serialized.contains("<redacted"));
    assert_eq!(
        result
            .pointer("/record/safety_flags/provider_request_sent")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(host.model_task_matches_path().exists());
    assert!(
        !host.llm_prompt_runs_path().exists(),
        "recording model-task history must not create prompt-run history"
    );

    let content =
        fs::read_to_string(host.model_task_matches_path()).expect("model task history content");
    assert!(!content.contains(&local_path));
    assert!(!content.contains(secret));
    assert!(content.contains("<app-data-dir>"));

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn model_task_match_list_aggregates_records_and_prompt_runs_with_filters() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-model-task-list-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let prompt_run = LlmPromptRunRecord {
        id: "prompt-run-model-task".to_string(),
        preview_id: "preview-model-task".to_string(),
        confirmation_id: "confirm-model-task".to_string(),
        action: "task_cockpit".to_string(),
        request_kind: "task_cockpit".to_string(),
        analysis_kind: None,
        scope: Some("selected".to_string()),
        instance_id: Some("fixture-skill".to_string()),
        instance_ids: vec!["fixture-skill".to_string()],
        definition_id: Some("fixture-definition".to_string()),
        agent: Some("codex".to_string()),
        task: Some("Review release evidence.".to_string()),
        profile_id: "fixture-openai".to_string(),
        provider: "openai-compatible".to_string(),
        model: "fixture-model".to_string(),
        destination_host: "api.fixture.invalid".to_string(),
        status: "succeeded".to_string(),
        error_code: None,
        error_message: None,
        duration_ms: 50,
        estimated_input_tokens: 100,
        estimated_output_tokens: 25,
        estimated_total_tokens: 125,
        estimated_cost_usd: 0.01,
        draft_output: None,
        draft_requires_user_copy: true,
        provider_request_sent: true,
        credential_accessed: true,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        redaction_summary: LlmPromptRunRedactionSummary {
            status: "redacted-local-only".to_string(),
            redacted_value_count: 0,
            redacted_fields: Vec::new(),
            placeholders: vec!["<redacted>".to_string()],
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
            raw_secret_returned: false,
        },
        created_at: 10,
        completed_at: 20,
        safety_flags: llm_prompt_run_safety_flags(true, true),
    };
    host.save_llm_prompt_runs(&[prompt_run])
        .expect("save prompt run");

    let record = host.handle(ServiceRequest {
        id: Some("model-task-record".to_string()),
        method: "llm.recordModelTaskMatch".to_string(),
        params: json!({
            "id": "model-task-fit",
            "title": "Fixture model fit",
            "task": "Review release evidence.",
            "task_kind": "task_cockpit",
            "agent": "codex",
            "provider": "openai-compatible",
            "model": "fixture-model",
            "destination_host": "api.fixture.invalid",
            "match_status": "fit",
            "confidence_score": 90,
            "estimated_total_tokens": 75,
            "estimated_cost_usd": 0.02,
            "evidence_refs": ["prompt-run:prompt-run-model-task"]
        }),
    });
    assert!(record.ok, "{:?}", record.error);

    let response = host.handle(ServiceRequest {
        id: Some("model-task-list-filtered".to_string()),
        method: "llm.listModelTaskMatches".to_string(),
        params: json!({
            "provider": "openai-compatible",
            "model": "fixture-model",
            "task_kind": "task_cockpit",
            "match_status": "fit",
            "agent": "codex",
            "limit": 10
        }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("model task list result");
    assert_eq!(
        result
            .pointer("/summary/returned_record_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        result
            .pointer("/summary/returned_prompt_run_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        result
            .pointer("/model_rows/0/provider")
            .and_then(Value::as_str),
        Some("openai-compatible")
    );
    assert_eq!(
        result
            .pointer("/recent_evidence_rows")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        result
            .pointer("/safety_flags/provider_request_sent")
            .and_then(Value::as_bool),
        Some(false),
        "listing history must not send fresh provider traffic"
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn model_task_match_list_returns_full_history_without_limit_and_reports_limited_page() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-model-task-list-full-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let safety_flags = ModelTaskMatchSafetyFlags {
        read_only: true,
        app_local_only: true,
        provider_request_sent: false,
        credential_accessed: false,
        draft_copy_only: true,
        write_back_allowed: false,
        write_actions_available: false,
        skill_files_mutated: false,
        agent_config_mutated: false,
        script_execution_allowed: false,
        execution_actions_available: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        triage_mutation_allowed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        unredacted_paths_returned: false,
        cloud_sync_performed: false,
        telemetry_emitted: false,
    };
    let redaction_summary = LlmPromptRunRedactionSummary {
        status: "redacted-local-only".to_string(),
        redacted_value_count: 0,
        redacted_fields: Vec::new(),
        placeholders: Vec::new(),
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        raw_secret_returned: false,
    };
    let records = (0..60)
        .map(|index| ModelTaskMatchRecord {
            id: format!("model-task-{index:02}"),
            title: format!("Fixture model task {index:02}"),
            task: format!("Review model task item {index:02}."),
            task_kind: "task_cockpit".to_string(),
            agent: Some("codex".to_string()),
            profile_id: Some("fixture-openai".to_string()),
            provider: "openai-compatible".to_string(),
            model: "fixture-model".to_string(),
            destination_host: Some("api.fixture.invalid".to_string()),
            match_status: "fit".to_string(),
            confidence_score: Some(90),
            latency_ms: Some(100),
            estimated_total_tokens: Some(120),
            estimated_cost_usd: Some(0.01),
            source_kind: "manual".to_string(),
            prompt_run_ids: Vec::new(),
            benchmark_ids: Vec::new(),
            evidence_refs: vec![format!("model-task:{index:02}")],
            gap_notes: Vec::new(),
            blocker_notes: Vec::new(),
            outcome_notes: Vec::new(),
            created_at: 1_000 + index,
            updated_at: 1_100 + index,
            redaction_summary: redaction_summary.clone(),
            safety_flags,
        })
        .collect::<Vec<_>>();
    host.save_model_task_matches(&records)
        .expect("save model-task history");

    let full_response = host.handle(ServiceRequest {
        id: Some("model-task-list-full".to_string()),
        method: "llm.listModelTaskMatches".to_string(),
        params: json!({}),
    });
    assert!(full_response.ok, "{:?}", full_response.error);
    let full = full_response.result.expect("full model-task list result");
    assert_eq!(
        full.get("total_record_count").and_then(Value::as_u64),
        Some(60)
    );
    assert_eq!(
        full.get("returned_record_count").and_then(Value::as_u64),
        Some(60)
    );
    assert_eq!(
        full.get("total_evidence_count").and_then(Value::as_u64),
        Some(60)
    );
    assert_eq!(
        full.get("returned_evidence_count").and_then(Value::as_u64),
        Some(60)
    );
    assert_eq!(full.get("limit"), None);
    assert_eq!(full.get("truncated").and_then(Value::as_bool), Some(false));
    assert_eq!(
        full.get("records").and_then(Value::as_array).map(Vec::len),
        Some(60)
    );
    assert_eq!(
        full.get("recent_evidence_rows")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(60)
    );

    let limited_response = host.handle(ServiceRequest {
        id: Some("model-task-list-limited".to_string()),
        method: "llm.listModelTaskMatches".to_string(),
        params: json!({ "limit": 5 }),
    });
    assert!(limited_response.ok, "{:?}", limited_response.error);
    let limited = limited_response
        .result
        .expect("limited model-task list result");
    assert_eq!(
        limited.get("total_record_count").and_then(Value::as_u64),
        Some(60)
    );
    assert_eq!(
        limited.get("returned_record_count").and_then(Value::as_u64),
        Some(5)
    );
    assert_eq!(
        limited.get("total_evidence_count").and_then(Value::as_u64),
        Some(60)
    );
    assert_eq!(
        limited
            .get("returned_evidence_count")
            .and_then(Value::as_u64),
        Some(5)
    );
    assert_eq!(limited.get("limit").and_then(Value::as_u64), Some(5));
    assert_eq!(
        limited.get("truncated").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        limited
            .get("records")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(5)
    );
    assert_eq!(
        limited
            .get("recent_evidence_rows")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(5)
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn model_task_match_delete_is_app_local_only() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-model-task-delete-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let record = host.handle(ServiceRequest {
        id: Some("model-task-record".to_string()),
        method: "llm.recordModelTaskMatch".to_string(),
        params: json!({
            "id": "model-task-delete-me",
            "task": "Fixture task",
            "model": "fixture-model"
        }),
    });
    assert!(record.ok, "{:?}", record.error);

    let response = host.handle(ServiceRequest {
        id: Some("model-task-delete".to_string()),
        method: "llm.deleteModelTaskMatch".to_string(),
        params: json!({ "id": "model-task-delete-me" }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("delete result");
    assert_eq!(result.get("deleted").and_then(Value::as_bool), Some(true));
    assert_eq!(
        result
            .pointer("/provider_request_sent")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .pointer("/skill_files_mutated")
            .and_then(Value::as_bool),
        Some(false)
    );
    let records = host
        .load_model_task_matches()
        .expect("load model task matches");
    assert!(records.is_empty());

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn app_version_returns_version_and_protocol() {
    let host = ServiceHost {
        app_data_dir: PathBuf::from("/tmp/skills-copilot-test"),
        adapter_ctx: AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    let response = host.handle(ServiceRequest {
        id: Some("version".to_string()),
        method: "app.version".to_string(),
        params: Value::Null,
    });

    assert!(response.ok);
    let result = response.result.expect("version result");
    assert_eq!(
        result.get("protocol_version").and_then(Value::as_u64),
        Some(u64::from(SERVICE_PROTOCOL_VERSION))
    );
    assert_eq!(
        result.get("version").and_then(Value::as_str),
        Some(skills_copilot_commands::app_version())
    );
}

#[test]
fn rules_tuning_methods_store_app_local_state_and_affect_findings() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-rule-tuning-service-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    fs::create_dir_all(&host.app_data_dir).expect("create app data");
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    catalog.init().expect("init catalog");
    let skill_path = app_data_dir.join("skills/review/SKILL.md");
    let instance = SkillInstance {
        id: "rule-tuning-skill-id".to_string(),
        agent: AgentId::Codex,
        scope: Scope::AgentGlobal,
        project_root: None,
        path: skill_path.clone(),
        display_path: skill_path,
        definition_id: "rule-tuning-definition-id".to_string(),
        name: "rule-tuning-fixture".to_string(),
        display_name: "rule-tuning-fixture".to_string(),
        description: "Rule tuning fixture.".to_string(),
        version: None,
        state: SkillState::Loaded,
        enabled: true,
        frontmatter_raw: "name: rule-tuning-fixture\ndescription: Rule tuning fixture\n"
            .to_string(),
        body: "Fixture body.".to_string(),
        scripts: Vec::new(),
        permissions: PermissionRequest::default(),
        fingerprint: "rule-tuning-fingerprint".to_string(),
        mtime: 1,
        first_seen: 1,
        last_seen: 1,
    };
    catalog
        .upsert_skill_instance(&instance)
        .expect("upsert skill");
    catalog
        .refresh_rule_findings(&[RuleFindingDraft {
            id: "rule-tuning-finding-id".to_string(),
            instance_id: Some(instance.id.clone()),
            definition_id: Some(instance.definition_id.clone()),
            rule_id: "body.too-long".to_string(),
            severity: "warn".to_string(),
            message: "Skill body is longer than the local review threshold.".to_string(),
            suggestion: Some("Move long reference material into references/.".to_string()),
            created_at: 1,
        }])
        .expect("seed finding");
    drop(catalog);

    let override_response = host.handle(ServiceRequest {
        id: Some("set-override".to_string()),
        method: "rules.setSeverityOverride".to_string(),
        params: json!({
            "rule_id": "body.too-long",
            "agent": "codex",
            "severity": "info"
        }),
    });
    assert!(override_response.ok);

    let suppression_response = host.handle(ServiceRequest {
        id: Some("set-suppression".to_string()),
        method: "rules.setSuppression".to_string(),
        params: json!({
            "rule_id": "body.too-long",
            "agent": "codex",
            "reason": "Accepted locally after review.",
            "note": "V2.32 app-local suppression."
        }),
    });
    assert!(suppression_response.ok);

    let findings_response = host.handle(ServiceRequest {
        id: Some("list-findings".to_string()),
        method: "catalog.listFindings".to_string(),
        params: Value::Null,
    });
    assert!(findings_response.ok);
    let findings = findings_response
        .result
        .expect("findings result")
        .as_array()
        .expect("findings array")
        .clone();
    let finding = findings.first().expect("finding exists");
    assert_eq!(
        finding.get("effective_severity").and_then(Value::as_str),
        Some("info")
    );
    assert_eq!(
        finding.get("suppressed").and_then(Value::as_bool),
        Some(true)
    );

    let clear_suppression_response = host.handle(ServiceRequest {
        id: Some("clear-suppression".to_string()),
        method: "rules.clearSuppression".to_string(),
        params: json!({
            "rule_id": "body.too-long",
            "agent": "codex"
        }),
    });
    assert!(clear_suppression_response.ok);
    let clear_override_response = host.handle(ServiceRequest {
        id: Some("clear-override".to_string()),
        method: "rules.clearSeverityOverride".to_string(),
        params: json!({
            "rule_id": "body.too-long",
            "agent": "codex"
        }),
    });
    assert!(clear_override_response.ok);

    let tuning_response = host.handle(ServiceRequest {
        id: Some("list-tuning".to_string()),
        method: "rules.listTuning".to_string(),
        params: Value::Null,
    });
    assert!(tuning_response.ok);
    assert_eq!(
        tuning_response
            .result
            .and_then(|value| value.as_array().cloned())
            .map(|rows| rows.len()),
        Some(0)
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn app_state_snapshot_returns_current_catalog_state() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let host = ServiceHost {
        app_data_dir: env::temp_dir().join(format!(
            "skills-copilot-state-snapshot-test-{}-{unique}",
            std::process::id(),
        )),
        adapter_ctx: AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    let response = host.handle(ServiceRequest {
        id: Some("snapshot".to_string()),
        method: "app.stateSnapshot".to_string(),
        params: Value::Null,
    });

    assert!(response.ok);
    let result = response.result.expect("snapshot result");
    assert!(result.get("status").is_some());
    assert_eq!(
        result.get("skills").and_then(Value::as_array).map(Vec::len),
        Some(0)
    );
    assert_eq!(
        result
            .get("findings")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        result
            .get("conflicts")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );
    assert_eq!(
        result
            .get("snapshots")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );

    let _ = fs::remove_dir_all(&host.app_data_dir);
}

#[test]
fn finding_triage_service_writes_only_app_local_catalog() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-triage-service-test-{}-{unique}",
        std::process::id()
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-triage-home-test-{}-{unique}",
        std::process::id()
    ));
    let settings_path = user_home.join(".claude/settings.json");
    fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings parent");
    fs::write(&settings_path, "{\"skillOverrides\":{\"keep\":\"on\"}}\n").expect("write settings");
    let host = ServiceHost {
        app_data_dir: app_data_dir.clone(),
        adapter_ctx: AdapterContext {
            user_home: user_home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    fs::create_dir_all(&host.app_data_dir).expect("create app data");
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    catalog.init().expect("init catalog");
    catalog
        .refresh_rule_findings(&[RuleFindingDraft {
            id: "triage-finding-id".to_string(),
            instance_id: Some("triage-skill-id".to_string()),
            definition_id: Some("triage-definition-id".to_string()),
            rule_id: "body.too-long".to_string(),
            severity: "warn".to_string(),
            message: "Skill body is longer than the local review threshold.".to_string(),
            suggestion: Some("Split long reference material into references/.".to_string()),
            created_at: 1,
        }])
        .expect("seed finding");
    let triage_key = catalog
        .list_rule_findings()
        .expect("list findings")
        .pop()
        .expect("finding exists")
        .triage_key;

    let response = host.handle(ServiceRequest {
        id: Some("set-triage".to_string()),
        method: "catalog.setFindingTriage".to_string(),
        params: json!({
            "triage_key": triage_key,
            "status": "ignored",
            "note": "not actionable locally"
        }),
    });

    assert!(
        response.ok,
        "triage set should succeed: {:?}",
        response.error
    );
    assert_eq!(
        fs::read_to_string(&settings_path).expect("read settings"),
        "{\"skillOverrides\":{\"keep\":\"on\"}}\n",
        "finding triage must not write agent config"
    );
    let catalog = Catalog::open(&host.catalog_path()).expect("reopen catalog");
    catalog.init().expect("re-init catalog");
    assert_eq!(
        catalog
            .list_all_config_snapshots(None)
            .expect("snapshots")
            .len(),
        0,
        "finding triage must not create agent config snapshots"
    );
    let finding = catalog
        .list_rule_findings()
        .expect("findings")
        .pop()
        .expect("finding exists");
    assert_eq!(finding.triage_status, "ignored");
    assert_eq!(
        finding.triage_note.as_deref(),
        Some("not actionable locally")
    );

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn unknown_method_returns_stable_error_code() {
    let host = ServiceHost {
        app_data_dir: PathBuf::from("/tmp/skills-copilot-test"),
        adapter_ctx: AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    let response = host.handle(ServiceRequest {
        id: Some("2".to_string()),
        method: "missing.method".to_string(),
        params: Value::Null,
    });

    assert!(!response.ok);
    assert_eq!(
        response.error.expect("error").code,
        "unknown_method".to_string()
    );
}

#[test]
fn get_skill_requires_instance_id_param() {
    let host = ServiceHost {
        app_data_dir: PathBuf::from("/tmp/skills-copilot-test"),
        adapter_ctx: AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    let response = host.handle(ServiceRequest {
        id: Some("3".to_string()),
        method: "catalog.getSkill".to_string(),
        params: json!({}),
    });

    assert!(!response.ok);
    assert_eq!(response.error.expect("error").code, "json_error");
}

#[test]
fn toggle_requires_on_param() {
    let host = ServiceHost {
        app_data_dir: PathBuf::from("/tmp/skills-copilot-test"),
        adapter_ctx: AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    let response = host.handle(ServiceRequest {
        id: Some("4".to_string()),
        method: "config.toggleSkill".to_string(),
        params: json!({"instance_id": "x"}),
    });

    assert!(!response.ok);
    assert_eq!(response.error.expect("error").code, "json_error");
}

#[test]
fn save_settings_requires_content_param() {
    let host = ServiceHost {
        app_data_dir: PathBuf::from("/tmp/skills-copilot-test"),
        adapter_ctx: AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    let response = host.handle(ServiceRequest {
        id: Some("5".to_string()),
        method: "config.saveClaudeSettings".to_string(),
        params: json!({}),
    });

    assert!(!response.ok);
    assert_eq!(response.error.expect("error").code, "json_error");
}

#[test]
fn project_context_set_get_and_clear_persist_state() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-project-context-test-{}-{unique}",
        std::process::id(),
    ));
    let root = app_data_dir.join("project");
    let nested = root.join("nested");
    fs::create_dir_all(&nested).expect("create project dirs");
    let host = test_host(app_data_dir.clone());

    let set_response = host.handle(ServiceRequest {
        id: Some("set-context".to_string()),
        method: "project.setContext".to_string(),
        params: json!({
            "root_path": root,
            "current_cwd": nested,
            "name": "Fixture Project"
        }),
    });
    assert!(set_response.ok);
    let set_result = set_response.result.expect("set result");
    assert_eq!(
        set_result.pointer("/active/name").and_then(Value::as_str),
        Some("Fixture Project")
    );
    assert_eq!(
        set_result
            .pointer("/active/is_active")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        set_result
            .get("recent")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert!(app_data_dir.join("project-context.json").exists());

    let get_response = host.handle(ServiceRequest {
        id: Some("get-context".to_string()),
        method: "project.getContext".to_string(),
        params: Value::Null,
    });
    assert!(get_response.ok);
    assert_eq!(
        get_response
            .result
            .as_ref()
            .and_then(|result| result.pointer("/active/name"))
            .and_then(Value::as_str),
        Some("Fixture Project")
    );

    let clear_response = host.handle(ServiceRequest {
        id: Some("clear-context".to_string()),
        method: "project.clearContext".to_string(),
        params: Value::Null,
    });
    assert!(clear_response.ok);
    let clear_result = clear_response.result.expect("clear result");
    assert!(clear_result.get("active").is_some_and(Value::is_null));
    assert_eq!(
        clear_result
            .pointer("/recent/0/is_active")
            .and_then(Value::as_bool),
        Some(false)
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn project_recent_context_entries_can_be_removed_or_cleared_without_changing_active_context() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-project-recents-test-{}-{unique}",
        std::process::id(),
    ));
    let first_root = app_data_dir.join("first-project");
    let active_root = app_data_dir.join("active-project");
    fs::create_dir_all(&first_root).expect("create first project");
    fs::create_dir_all(&active_root).expect("create active project");
    let host = test_host(app_data_dir.clone());

    for (id, root, name) in [
        ("set-first", &first_root, "First Project"),
        ("set-active", &active_root, "Active Project"),
    ] {
        let response = host.handle(ServiceRequest {
            id: Some(id.to_string()),
            method: "project.setContext".to_string(),
            params: json!({ "root_path": root, "name": name }),
        });
        assert!(response.ok, "{id} should succeed: {:?}", response.error);
    }

    let state = host.handle(ServiceRequest {
        id: Some("get-before-remove".to_string()),
        method: "project.getContext".to_string(),
        params: Value::Null,
    });
    let state = state.result.expect("project state before remove");
    let active_id = state
        .pointer("/active/id")
        .and_then(Value::as_str)
        .expect("active project id")
        .to_string();
    assert_eq!(
        state.get("recent").and_then(Value::as_array).map(Vec::len),
        Some(2)
    );

    let remove = host.handle(ServiceRequest {
        id: Some("remove-active-recent".to_string()),
        method: "project.removeRecentContext".to_string(),
        params: json!({ "id": active_id }),
    });
    assert!(
        remove.ok,
        "remove recent should succeed: {:?}",
        remove.error
    );
    let removed_state = remove.result.expect("state after remove");
    assert_eq!(
        removed_state
            .pointer("/active/name")
            .and_then(Value::as_str),
        Some("Active Project"),
        "removing a recent row must not clear the active project"
    );
    assert_eq!(
        removed_state
            .get("recent")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1)
    );
    assert_eq!(
        removed_state
            .pointer("/recent/0/name")
            .and_then(Value::as_str),
        Some("First Project")
    );

    let clear = host.handle(ServiceRequest {
        id: Some("clear-recents".to_string()),
        method: "project.clearRecentContexts".to_string(),
        params: Value::Null,
    });
    assert!(clear.ok, "clear recents should succeed: {:?}", clear.error);
    let cleared_state = clear.result.expect("state after clearing recents");
    assert_eq!(
        cleared_state
            .pointer("/active/name")
            .and_then(Value::as_str),
        Some("Active Project")
    );
    assert_eq!(
        cleared_state
            .get("recent")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0)
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn project_validate_context_reports_validation_error_without_persisting() {
    let host = test_host(env::temp_dir().join(format!(
        "skills-copilot-project-validate-test-{}-{}",
        std::process::id(),
        unique_suffix()
    )));

    let response = host.handle(ServiceRequest {
        id: Some("validate-context".to_string()),
        method: "project.validateContext".to_string(),
        params: json!({
            "root_path": "/tmp/skills-copilot-missing-project-root-for-validation"
        }),
    });

    assert!(response.ok);
    let result = response.result.expect("validate result");
    assert!(result
        .get("validation_error")
        .and_then(Value::as_str)
        .is_some_and(|message| message.contains("root_path")));
    assert!(!host.app_data_dir.join("project-context.json").exists());

    let _ = fs::remove_dir_all(host.app_data_dir);
}

#[test]
fn project_set_context_rejects_cwd_outside_root() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-project-reject-test-{}-{unique}",
        std::process::id(),
    ));
    let root = app_data_dir.join("project");
    let outside = app_data_dir.join("outside");
    fs::create_dir_all(&root).expect("create root");
    fs::create_dir_all(&outside).expect("create outside");
    let host = test_host(app_data_dir.clone());

    let response = host.handle(ServiceRequest {
        id: Some("set-invalid-context".to_string()),
        method: "project.setContext".to_string(),
        params: json!({
            "root_path": root,
            "current_cwd": outside
        }),
    });

    assert!(!response.ok);
    assert_eq!(
        response.error.expect("error").code,
        "invalid_request".to_string()
    );
    assert!(!app_data_dir.join("project-context.json").exists());

    let _ = fs::remove_dir_all(app_data_dir);
}

#[cfg(unix)]
#[test]
fn project_set_context_rejects_symlink_escape_cwd() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-project-symlink-test-{}-{unique}",
        std::process::id(),
    ));
    let root = app_data_dir.join("project");
    let outside = app_data_dir.join("outside");
    let link = root.join("link-outside");
    fs::create_dir_all(&root).expect("create root");
    fs::create_dir_all(&outside).expect("create outside");
    std::os::unix::fs::symlink(&outside, &link).expect("create symlink");
    let host = test_host(app_data_dir.clone());

    let response = host.handle(ServiceRequest {
        id: Some("set-symlink-context".to_string()),
        method: "project.setContext".to_string(),
        params: json!({
            "root_path": root,
            "current_cwd": link
        }),
    });

    assert!(!response.ok);
    assert_eq!(
        response.error.expect("error").code,
        "invalid_request".to_string()
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn scan_claude_returns_refresh_activity() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let fixture_root = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/claude-code/personal");
    let host = ServiceHost {
        app_data_dir: env::temp_dir().join(format!(
            "skills-copilot-scan-activity-test-{}-{unique}",
            std::process::id(),
        )),
        adapter_ctx: AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: fixture_root,
                source: RootSource::Extra,
            }],
        },
    };
    let response = host.handle(ServiceRequest {
        id: Some("scan".to_string()),
        method: "catalog.scanClaude".to_string(),
        params: Value::Null,
    });

    assert!(response.ok);
    let result = response.result.expect("scan result");
    assert_eq!(result.get("scanned_count").and_then(Value::as_u64), Some(1));
    let activity = result
        .get("activity")
        .and_then(Value::as_object)
        .expect("activity");
    assert_eq!(
        activity.get("status").and_then(Value::as_str),
        Some("completed")
    );
    assert_eq!(activity.get("skill_count").and_then(Value::as_u64), Some(1));
    assert!(activity
        .get("log_entries")
        .and_then(Value::as_array)
        .is_some_and(|entries| !entries.is_empty()));

    let _ = fs::remove_dir_all(&host.app_data_dir);
}

#[test]
#[cfg(unix)]
fn scan_claude_surfaces_dangling_link_without_degrading_refresh_activity() {
    let temp_root = env::temp_dir().join(format!(
        "skills-copilot-scan-claude-partial-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let home = temp_root.join("home");
    let skill_root = home.join(".claude/skills");
    fs::create_dir_all(skill_root.join("valid")).expect("create skill root");
    fs::write(
        skill_root.join("valid/SKILL.md"),
        "---\nname: valid\ndescription: valid fixture\n---\nbody\n",
    )
    .expect("write skill");
    std::os::unix::fs::symlink(
        skill_root.join("missing-target"),
        skill_root.join("dangling-link"),
    )
    .expect("create dangling link");
    let host = ServiceHost {
        app_data_dir: temp_root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };

    let response = host.handle(ServiceRequest {
        id: Some("scan-partial".to_string()),
        method: "catalog.scanClaude".to_string(),
        params: Value::Null,
    });
    let result = response.result.expect("scan result");
    let activity = result.get("activity").expect("activity");
    let summary = activity
        .get("agent_summaries")
        .and_then(Value::as_array)
        .and_then(|summaries| summaries.first())
        .expect("Claude summary");

    assert_eq!(
        activity.get("status").and_then(Value::as_str),
        Some("completed")
    );
    assert_eq!(
        summary.get("status").and_then(Value::as_str),
        Some("completed")
    );
    assert!(summary
        .get("roots_partial")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty));
    assert_eq!(
        summary
            .get("scan_issues")
            .and_then(Value::as_array)
            .and_then(|issues| issues.first())
            .and_then(|issue| issue.get("kind"))
            .and_then(Value::as_str),
        Some("dangling_symlink")
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn import_skill_imports_local_directory_to_tool_global_staging_only() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-service-import-test-{}-{unique}",
        std::process::id(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-service-import-home-{}-{unique}",
        std::process::id(),
    ));
    let source = app_data_dir.join("external-source").join("service-import");
    std::fs::create_dir_all(&source).expect("create source");
    std::fs::create_dir_all(user_home.join(".claude")).expect("create claude dir");
    let settings_path = user_home.join(".claude/settings.json");
    std::fs::write(&settings_path, "{\"skillOverrides\":{\"keep\":\"off\"}}\n")
        .expect("write settings");
    std::fs::write(
            source.join("SKILL.md"),
            "---\nname: Service Import\ndescription: Service import fixture\ntools:\n  - bash\n---\nRun `curl https://example.test/input.json`.\n",
        )
        .expect("write skill");
    let host = ServiceHost {
        app_data_dir: app_data_dir.clone(),
        adapter_ctx: AdapterContext {
            user_home: user_home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };

    let response = host.handle(ServiceRequest {
        id: Some("import-local".to_string()),
        method: "catalog.importSkill".to_string(),
        params: json!({ "source_path": source }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("import result");
    assert_eq!(
        result.pointer("/imported/agent").and_then(Value::as_str),
        Some("tool-global")
    );
    assert_eq!(
        result.pointer("/imported/scope").and_then(Value::as_str),
        Some("tool-global")
    );
    let staging_path = result
        .get("staging_path")
        .and_then(Value::as_str)
        .expect("staging path");
    assert!(PathBuf::from(staging_path).starts_with(
        host.tool_global_staging_root()
            .join("skills")
            .canonicalize()
            .expect("canonical staging skills root")
    ));
    assert!(PathBuf::from(staging_path).exists());
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read settings"),
        "{\"skillOverrides\":{\"keep\":\"off\"}}\n"
    );
    assert!(
        !user_home.join(".codex/config.toml").exists(),
        "tool-global import must not create agent config"
    );
    assert_eq!(
        result
            .pointer("/audit/read_only_preview")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert!(
        result
            .get("findings")
            .and_then(Value::as_array)
            .is_some_and(|findings| !findings.is_empty()),
        "import should return audit findings"
    );

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn import_skill_rejects_github_url_without_network_clone() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-service-import-github-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let response = host.handle(ServiceRequest {
        id: Some("import-github".to_string()),
        method: "catalog.importSkill".to_string(),
        params: json!({ "github_url": "https://github.com/example/skill.git" }),
    });

    assert!(!response.ok);
    let error = response.error.expect("github unsupported error");
    assert_eq!(error.code, "command_error");
    assert!(error.message.contains("explicitly deferred"));
    assert!(
        !host.tool_global_staging_root().exists(),
        "unsupported GitHub import must not initialize staging"
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn scan_all_returns_multi_agent_refresh_activity() {
    let unique = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let host = ServiceHost {
        app_data_dir: env::temp_dir().join(format!(
            "skills-copilot-scan-all-activity-test-{}-{unique}",
            std::process::id(),
        )),
        adapter_ctx: AdapterContext {
            user_home: repo_root.join("fixtures/codex/user-home"),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: repo_root.join("fixtures/claude-code/personal"),
                source: RootSource::Extra,
            }],
        },
    };
    let response = host.handle(ServiceRequest {
        id: Some("scan-all".to_string()),
        method: "catalog.scanAll".to_string(),
        params: Value::Null,
    });

    assert!(response.ok);
    let result = response.result.expect("scan all result");
    assert_eq!(result.get("scanned_count").and_then(Value::as_u64), Some(5));
    let activity = result
        .get("activity")
        .and_then(Value::as_object)
        .expect("activity");
    assert_eq!(
        activity.get("operation").and_then(Value::as_str),
        Some("catalog.scanAll")
    );
    let first_message = activity
        .get("log_entries")
        .and_then(Value::as_array)
        .and_then(|entries| entries.first())
        .and_then(|entry| entry.get("message"))
        .and_then(Value::as_str)
        .expect("first log message");
    assert!(
        first_message.contains("Claude Code, Codex, opencode, Pi, OpenClaw, and Hermes"),
        "scanAll activity should name all supported adapters"
    );
    let summaries = activity
        .get("agent_summaries")
        .and_then(Value::as_array)
        .expect("agent summaries");
    assert_eq!(summaries.len(), 6);
    let hermes = summaries
        .iter()
        .find(|summary| summary.get("agent").and_then(Value::as_str) == Some("hermes"))
        .expect("Hermes summary");
    assert_eq!(
        hermes.get("writable_status").and_then(Value::as_str),
        Some("guarded-v2.97")
    );
    assert!(hermes
        .get("read_only_reason")
        .and_then(Value::as_str)
        .is_some_and(|reason| reason.contains("skills.disabled")));
    let log_messages: Vec<&str> = activity
        .get("log_entries")
        .and_then(Value::as_array)
        .expect("log entries")
        .iter()
        .filter_map(|entry| entry.get("message").and_then(Value::as_str))
        .collect();
    assert!(
        log_messages
            .iter()
            .all(|message| !message.contains("root-error skipped-root path(s):")),
        "missing implicit built-in roots must not create skipped-root warnings"
    );
    assert_eq!(
        activity.get("status").and_then(Value::as_str),
        Some("completed")
    );
    let claude = summaries
        .iter()
        .find(|summary| summary.get("agent").and_then(Value::as_str) == Some("claude-code"))
        .expect("Claude Code summary");
    assert_eq!(
        claude.get("display_label").and_then(Value::as_str),
        Some("Claude Code")
    );
    assert_eq!(claude.get("scanned_count").and_then(Value::as_u64), Some(1));
    assert!(claude
        .get("roots_considered")
        .and_then(Value::as_array)
        .is_some_and(|roots| roots.len() >= 2));
    let codex = summaries
        .iter()
        .find(|summary| summary.get("agent").and_then(Value::as_str) == Some("codex"))
        .expect("Codex summary");
    assert_eq!(
        codex.get("display_label").and_then(Value::as_str),
        Some("Codex")
    );
    assert_eq!(codex.get("scanned_count").and_then(Value::as_u64), Some(1));
    assert_eq!(codex.get("catalog_count").and_then(Value::as_u64), Some(1));
    let pi = summaries
        .iter()
        .find(|summary| summary.get("agent").and_then(Value::as_str) == Some("pi"))
        .expect("Pi summary");
    assert_eq!(pi.get("scanned_count").and_then(Value::as_u64), Some(1));

    let _ = fs::remove_dir_all(&host.app_data_dir);
}

#[test]
fn adapter_list_diagnostics_reports_roots_config_and_blockers() {
    let unique = unique_suffix();
    let temp_root = env::temp_dir().join(format!(
        "skills-copilot-adapter-diagnostics-test-{}-{unique}",
        std::process::id(),
    ));
    let home = temp_root.join("home");
    let project = temp_root.join("project");
    let opencode_configured_root = temp_root.join("opencode-configured-skills");
    fs::create_dir_all(home.join(".pi/agent/skills")).expect("create Pi skills root");
    fs::create_dir_all(home.join(".codex")).expect("create Codex config parent");
    fs::write(home.join(".codex/config.toml"), "[skills]\n").expect("write Codex config");
    fs::create_dir_all(home.join(".config/opencode")).expect("create opencode config parent");
    fs::create_dir_all(&opencode_configured_root).expect("create opencode configured root");
    fs::write(
        home.join(".config/opencode/opencode.json"),
        format!(
            "{{\"skills\":{{\"paths\":[\"{}\"],\"urls\":[\"https://example.invalid/skills/\"]}}}}\n",
            json_path_text(&opencode_configured_root)
        ),
    )
    .expect("write opencode config");

    let host = ServiceHost {
        app_data_dir: temp_root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: home,
            project_root: Some(project),
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };

    let response = host.handle(ServiceRequest {
        id: Some("diagnostics".to_string()),
        method: "adapter.listDiagnostics".to_string(),
        params: Value::Null,
    });

    assert!(response.ok);
    let diagnostics = response.result.expect("diagnostics result");
    let records = diagnostics.as_array().expect("diagnostic records");
    let codex = records
        .iter()
        .find(|record| record.get("agent").and_then(Value::as_str) == Some("codex"))
        .expect("Codex diagnostics");
    assert_eq!(
        codex.pointer("/config/status").and_then(Value::as_str),
        Some("detected")
    );
    assert_eq!(
        codex
            .pointer("/access/writable_status")
            .and_then(Value::as_str),
        Some("verified-native-roots-only")
    );
    assert!(codex
        .get("roots")
        .and_then(Value::as_array)
        .is_some_and(|roots| roots
            .iter()
            .any(|root| { root.get("source").and_then(Value::as_str) == Some("compatibility") })
            && roots
                .iter()
                .any(|root| { root.get("source").and_then(Value::as_str) == Some("admin") })));
    let opencode = records
        .iter()
        .find(|record| record.get("agent").and_then(Value::as_str) == Some("opencode"))
        .expect("opencode diagnostics");
    assert_eq!(
        opencode.pointer("/config/status").and_then(Value::as_str),
        Some("detected")
    );
    assert!(opencode
        .get("roots")
        .and_then(Value::as_array)
        .is_some_and(|roots| roots.iter().any(|root| {
            root.get("source").and_then(Value::as_str) == Some("configured")
                && root
                    .get("reason")
                    .and_then(Value::as_str)
                    .is_some_and(|reason| reason.contains("skills.paths"))
        })));
    assert!(opencode
        .get("blockers")
        .and_then(Value::as_array)
        .is_some_and(|blockers| blockers.iter().any(|blocker| {
            blocker
                .as_str()
                .is_some_and(|text| text.contains("skills.urls"))
        })));
    let pi = records
        .iter()
        .find(|record| record.get("agent").and_then(Value::as_str) == Some("pi"))
        .expect("Pi diagnostics");
    assert!(pi
        .get("blockers")
        .and_then(Value::as_array)
        .is_some_and(|blockers| blockers.iter().any(|blocker| {
            blocker.as_str() == Some("Pi package install/remove remains blocked.")
        })));
    let hermes = records
        .iter()
        .find(|record| record.get("agent").and_then(Value::as_str) == Some("hermes"))
        .expect("Hermes diagnostics");
    assert_eq!(
        hermes.pointer("/config/status").and_then(Value::as_str),
        Some("not-detected")
    );
    assert_eq!(
        hermes
            .pointer("/access/writable_status")
            .and_then(Value::as_str),
        Some("guarded-v2.97")
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn scan_all_label_formats_four_agent_reports() {
    let reports = vec![
        AgentCatalogScanReport {
            agent: AgentId::ClaudeCode,
            display_name: "Claude Code",
            scanned_count: 1,
            roots_considered: vec![PathBuf::from("/tmp/home/.claude/skills")],
            scanned_roots: vec![PathBuf::from("/tmp/home/.claude/skills")],
            partial_roots: Vec::new(),
            skipped_roots: Vec::new(),
            issues: Vec::new(),
            root_aliases: Vec::new(),
            budget_exhausted: false,
        },
        AgentCatalogScanReport {
            agent: AgentId::Codex,
            display_name: "Codex",
            scanned_count: 1,
            roots_considered: vec![PathBuf::from("/tmp/home/.agents/skills")],
            scanned_roots: vec![PathBuf::from("/tmp/home/.agents/skills")],
            partial_roots: Vec::new(),
            skipped_roots: Vec::new(),
            issues: Vec::new(),
            root_aliases: Vec::new(),
            budget_exhausted: false,
        },
        AgentCatalogScanReport {
            agent: AgentId::Opencode,
            display_name: "opencode",
            scanned_count: 1,
            roots_considered: vec![PathBuf::from("/tmp/home/.config/opencode/skills")],
            scanned_roots: vec![PathBuf::from("/tmp/home/.config/opencode/skills")],
            partial_roots: Vec::new(),
            skipped_roots: Vec::new(),
            issues: Vec::new(),
            root_aliases: Vec::new(),
            budget_exhausted: false,
        },
        AgentCatalogScanReport {
            agent: AgentId::Pi,
            display_name: "Pi",
            scanned_count: 1,
            roots_considered: vec![PathBuf::from("/tmp/home/.pi/agent/skills")],
            scanned_roots: vec![PathBuf::from("/tmp/home/.pi/agent/skills")],
            partial_roots: Vec::new(),
            skipped_roots: Vec::new(),
            issues: Vec::new(),
            root_aliases: Vec::new(),
            budget_exhausted: false,
        },
    ];

    assert_eq!(
        scan_all_label(&reports),
        "Claude Code, Codex, opencode, and Pi"
    );
}

#[test]
fn partial_agent_refresh_summary_is_warning_and_redacts_issue_details() {
    let temp_root = env::temp_dir().join(format!(
        "skills-copilot-partial-refresh-summary-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let home = temp_root.join("private-home");
    let partial_root = home.join(".claude/skills");
    let issue_path = partial_root.join("dangling-link");
    let host = ServiceHost {
        app_data_dir: temp_root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    let report = AgentCatalogScanReport {
        agent: AgentId::ClaudeCode,
        display_name: "Claude Code",
        scanned_count: 1,
        roots_considered: vec![partial_root.clone()],
        scanned_roots: Vec::new(),
        partial_roots: vec![partial_root.clone()],
        skipped_roots: Vec::new(),
        issues: vec![skills_copilot_commands::AgentCatalogScanIssue {
            path: issue_path,
            kind: "entry_unreadable",
            detail: format!(
                "failed to canonicalize {} because target /Volumes/private-scan-target is unavailable",
                partial_root.join("private-target").display(),
            ),
        }],
        root_aliases: Vec::new(),
        budget_exhausted: false,
    };

    let summaries = host.agent_refresh_summaries(&[report], &[], &[]);
    let summary = summaries.first().expect("partial summary");

    assert_eq!(summary.status, "completed-partial");
    assert_eq!(summary.roots_partial, vec!["$HOME/.claude/skills"]);
    assert!(summary.roots_scanned.is_empty());
    assert_eq!(summary.scan_issues.len(), 1);
    assert_eq!(summary.scan_issues[0].kind, "entry_unreadable");
    assert_eq!(
        summary.scan_issues[0].path,
        "$HOME/.claude/skills/dangling-link"
    );
    assert_eq!(
        summary.scan_issues[0].detail,
        "A directory entry could not be inspected or resolved."
    );
    assert!(!summary.scan_issues[0]
        .detail
        .contains(&home.to_string_lossy().to_string()));
    assert!(!summary.scan_issues[0]
        .detail
        .contains("/Volumes/private-scan-target"));
    assert!(summary
        .recovery_actions
        .iter()
        .any(|action| action.contains("preserved")));

    let activity = host.scan_activity(
        "catalog.scanAll",
        "supported-agent",
        vec![partial_root],
        1,
        ScanActivityCounts {
            scanned_count: 1,
            skill_count: 1,
            finding_count: 0,
            conflict_count: 0,
            snapshot_count: 0,
        },
        Some(summaries),
    );
    assert_eq!(activity.status, "completed-partial");
    let warning = activity
        .log_entries
        .iter()
        .find(|entry| {
            entry.level == "warning"
                && entry.message.contains("partial")
                && entry.message.contains("entry_unreadable")
        })
        .expect("partial warning log");
    assert!(
        !warning.message.ends_with(".."),
        "stable issue detail and enclosing sentence must not duplicate punctuation"
    );

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
#[cfg(unix)]
fn scan_summary_uses_immutable_nested_aliases_after_declared_symlink_changes() {
    let temp_root = env::temp_dir().join(format!(
        "skills-copilot-scan-alias-redaction-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let home = temp_root.join("private-home");
    let declared_root = home.join(".claude/skills");
    let external_root = temp_root.join("private-external-root");
    let nested_root = external_root.join("nested-private-root");
    let replacement_root = temp_root.join("replacement-private-root");
    fs::create_dir_all(declared_root.parent().expect("declared parent"))
        .expect("create declared parent");
    fs::create_dir_all(&nested_root).expect("create nested external root");
    fs::create_dir_all(&replacement_root).expect("create replacement root");
    std::os::unix::fs::symlink(&external_root, &declared_root)
        .expect("create declared root symlink");
    let canonical_external = external_root
        .canonicalize()
        .expect("canonical external root");
    let canonical_nested = nested_root.canonicalize().expect("canonical nested root");
    let issue_path =
        canonical_external.join("nested-private-root/../nested-private-root/private-entry");
    let report = AgentCatalogScanReport {
        agent: AgentId::ClaudeCode,
        display_name: "Claude Code",
        scanned_count: 0,
        roots_considered: vec![home.join(".claude/../.claude/skills")],
        scanned_roots: Vec::new(),
        partial_roots: vec![canonical_external.clone()],
        skipped_roots: Vec::new(),
        issues: vec![skills_copilot_commands::AgentCatalogScanIssue {
            path: issue_path,
            kind: "entry_unreadable",
            detail: "private raw detail".to_string(),
        }],
        root_aliases: vec![
            skills_copilot_commands::AgentCatalogScanPathAlias {
                declared: declared_root.clone(),
                canonical: canonical_external,
            },
            skills_copilot_commands::AgentCatalogScanPathAlias {
                declared: nested_root.clone(),
                canonical: canonical_nested,
            },
        ],
        budget_exhausted: false,
    };
    fs::remove_file(&declared_root).expect("remove original symlink");
    std::os::unix::fs::symlink(&replacement_root, &declared_root)
        .expect("replace declared symlink after scan");
    let host = ServiceHost {
        app_data_dir: temp_root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };

    let summaries = host.agent_refresh_summaries(&[report], &[], &[]);
    let summary = summaries.first().expect("summary");
    let serialized = serde_json::to_string(summary).expect("serialize summary");

    assert_eq!(summary.roots_considered, vec!["$HOME/.claude/skills"]);
    assert_eq!(summary.roots_partial, vec!["<adapter-root>"]);
    assert_eq!(
        summary.scan_issues.first().expect("scan issue").path,
        "<adapter-root>/private-entry"
    );
    let activity = host.scan_activity(
        "catalog.scanAll",
        "Claude Code",
        vec![declared_root],
        1,
        ScanActivityCounts {
            scanned_count: 0,
            skill_count: 0,
            finding_count: 0,
            conflict_count: 0,
            snapshot_count: 0,
        },
        Some(summaries),
    );
    let serialized_activity = serde_json::to_string(&activity).expect("serialize activity");
    assert!(activity.log_entries.iter().any(|entry| {
        entry.level == "warning"
            && entry.message.contains("<adapter-root>/private-entry")
            && entry.message.contains("entry_unreadable")
    }));
    for private_path in [
        &home,
        &external_root,
        &nested_root,
        &replacement_root,
        &host.app_data_dir,
    ] {
        assert!(
            !serialized.contains(&private_path.to_string_lossy().to_string()),
            "serialized scan diagnostics leaked {}",
            private_path.display()
        );
        assert!(
            !serialized_activity.contains(&private_path.to_string_lossy().to_string()),
            "serialized scan activity leaked {}",
            private_path.display()
        );
    }

    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn scan_all_uses_stored_project_context_when_env_context_is_absent() {
    let unique = unique_suffix();
    let repo_root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("../..");
    let host = ServiceHost {
        app_data_dir: env::temp_dir().join(format!(
            "skills-copilot-scan-all-stored-project-test-{}-{unique}",
            std::process::id(),
        )),
        adapter_ctx: AdapterContext {
            user_home: repo_root.join("fixtures/codex/user-home"),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: repo_root.join("fixtures/claude-code/personal"),
                source: RootSource::Extra,
            }],
        },
    };
    let set_response = host.handle(ServiceRequest {
        id: Some("set-context".to_string()),
        method: "project.setContext".to_string(),
        params: json!({
            "root_path": repo_root.join("fixtures/codex/project"),
            "current_cwd": repo_root.join("fixtures/codex/project/nested")
        }),
    });
    assert!(set_response.ok);

    let scan_response = host.handle(ServiceRequest {
        id: Some("scan-all".to_string()),
        method: "catalog.scanAll".to_string(),
        params: Value::Null,
    });

    assert!(scan_response.ok);
    let result = scan_response.result.expect("scan all result");
    assert_eq!(
        result.get("scanned_count").and_then(Value::as_u64),
        Some(11)
    );
    let skills = result
        .get("skills")
        .and_then(Value::as_array)
        .expect("scan skills");
    assert!(
        skills.iter().any(|skill| {
            skill.get("agent").and_then(Value::as_str) == Some("codex")
                && skill.get("name").and_then(Value::as_str) == Some("repo-beta")
        }),
        "project context scan should expose the current project skill"
    );
    let codex = result
        .pointer("/activity/agent_summaries")
        .and_then(Value::as_array)
        .and_then(|summaries| {
            summaries
                .iter()
                .find(|summary| summary.get("agent").and_then(Value::as_str) == Some("codex"))
        })
        .expect("Codex summary");
    assert_eq!(codex.get("scanned_count").and_then(Value::as_u64), Some(3));
    assert!(codex
        .get("roots_considered")
        .and_then(Value::as_array)
        .is_some_and(|roots| roots.len() >= 3));
    let pi = result
        .pointer("/activity/agent_summaries")
        .and_then(Value::as_array)
        .and_then(|summaries| {
            summaries
                .iter()
                .find(|summary| summary.get("agent").and_then(Value::as_str) == Some("pi"))
        })
        .expect("Pi summary");
    assert_eq!(pi.get("scanned_count").and_then(Value::as_u64), Some(3));

    let clear_response = host.handle(ServiceRequest {
        id: Some("clear-context".to_string()),
        method: "project.clearContext".to_string(),
        params: Value::Null,
    });
    assert!(clear_response.ok);

    let cleared_scan_response = host.handle(ServiceRequest {
        id: Some("scan-all-cleared".to_string()),
        method: "catalog.scanAll".to_string(),
        params: Value::Null,
    });
    assert!(cleared_scan_response.ok);
    let cleared = cleared_scan_response.result.expect("cleared scan result");
    let cleared_skills = cleared
        .get("skills")
        .and_then(Value::as_array)
        .expect("cleared scan skills");
    assert!(
        cleared_skills.iter().any(|skill| {
            skill.get("agent").and_then(Value::as_str) == Some("codex")
                && skill.get("name").and_then(Value::as_str) == Some("user-alpha")
        }),
        "no-project scan should keep user-scope Codex skills visible"
    );
    assert!(
        !cleared_skills.iter().any(|skill| {
            skill.get("agent").and_then(Value::as_str) == Some("codex")
                && skill.get("name").and_then(Value::as_str) == Some("repo-beta")
        }),
        "no-project scan should hide previously cataloged project skills"
    );

    let _ = fs::remove_dir_all(&host.app_data_dir);
}

#[test]
fn skill_export_bundle_exports_staging_skill_through_service() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-service-export-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let source_dir = app_data_dir.join("staging/demo");
    fs::create_dir_all(&source_dir).expect("create source skill");
    fs::write(
        source_dir.join("SKILL.md"),
        "---\nname: service-demo\ndescription: Service export demo\nversion: 2.9.0\n---\nBody.\n",
    )
    .expect("write source skill");
    let output_dir = app_data_dir.join("exports");
    let host = test_host(app_data_dir.clone());

    let response = host.handle(ServiceRequest {
        id: Some("export-service".to_string()),
        method: "skill.exportBundle".to_string(),
        params: json!({
            "source_path": source_dir,
            "output_dir": output_dir,
        }),
    });

    assert!(response.ok);
    let result = response.result.expect("export result");
    let export: WireExportedSkillBundle =
        serde_json::from_value(result).expect("decode export result");
    assert!(export.manifest_path.exists());
    assert!(export.bundle_path.join("skill/SKILL.md").exists());
    assert_eq!(export.metadata.name, "service-demo");
    assert_eq!(export.metadata.source_scope, "tool-global");
    let manifest = fs::read_to_string(&export.manifest_path).expect("read manifest");
    assert!(
        !manifest.contains(&app_data_dir.to_string_lossy().to_string()),
        "manifest reproducible fields should not include absolute app-data paths"
    );

    let _ = fs::remove_dir_all(app_data_dir);
}
