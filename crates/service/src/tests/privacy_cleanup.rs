use super::*;

fn legacy_prompt_run_fixture(task: &str, draft: &str) -> LlmPromptRunRecord {
    let mut safety_flags =
        crate::service_observability_helpers::llm_prompt_run_safety_flags(true, true);
    safety_flags.raw_prompt_persisted = true;
    safety_flags.raw_response_persisted = true;
    LlmPromptRunRecord {
        id: "legacy-run".to_string(),
        preview_id: "legacy-preview".to_string(),
        confirmation_id: "legacy-confirmation".to_string(),
        action: "recommend".to_string(),
        request_kind: "recommend".to_string(),
        analysis_kind: None,
        scope: Some("selected".to_string()),
        instance_id: Some("fixture-skill".to_string()),
        instance_ids: vec!["fixture-skill".to_string()],
        definition_id: Some("fixture-definition".to_string()),
        agent: Some("codex".to_string()),
        task: Some(task.to_string()),
        profile_id: "fixture-provider".to_string(),
        provider: "openai-compatible".to_string(),
        model: "fixture-model".to_string(),
        destination_host: "example.invalid".to_string(),
        status: "succeeded".to_string(),
        error_code: None,
        error_message: None,
        duration_ms: 42,
        estimated_input_tokens: 12,
        estimated_output_tokens: 8,
        estimated_total_tokens: 20,
        estimated_cost_usd: 0.01,
        draft_output: Some(draft.to_string()),
        draft_requires_user_copy: true,
        provider_request_sent: true,
        credential_accessed: true,
        raw_secret_returned: false,
        raw_prompt_persisted: true,
        raw_response_persisted: true,
        redaction_summary: LlmPromptRunRedactionSummary {
            status: "legacy".to_string(),
            redacted_value_count: 0,
            redacted_fields: Vec::new(),
            placeholders: Vec::new(),
            raw_prompt_persisted: true,
            raw_response_persisted: true,
            raw_trace_persisted: false,
            raw_secret_returned: false,
        },
        created_at: 1,
        completed_at: 2,
        safety_flags,
    }
}

fn inspect(host: &ServiceHost) -> ServiceResponse {
    host.handle(ServiceRequest {
        id: Some("privacy-inspect".to_string()),
        method: "privacy.inspectLegacyContent".to_string(),
        params: Value::Null,
    })
}

fn preview(host: &ServiceHost) -> ServiceResponse {
    host.handle(ServiceRequest {
        id: Some("privacy-preview".to_string()),
        method: "privacy.previewCleanupLegacyContent".to_string(),
        params: Value::Null,
    })
}

#[test]
fn privacy_inspection_of_missing_owner_is_read_only_and_does_not_create_app_data() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-privacy-inspect-missing-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let response = inspect(&host);

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("privacy inspection");
    assert_eq!(
        result.get("cleanup_required").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(result.get("read_only").and_then(Value::as_bool), Some(true));
    assert_eq!(
        result.get("write_performed").and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        !app_data_dir.exists(),
        "read-only inspection must not initialize app data"
    );
}

#[test]
fn privacy_preview_is_zero_write_and_confirmed_cleanup_sanitizes_or_deletes_all_sources() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-privacy-cleanup-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let prompt_path = app_data_dir.join("prompt-runs.json");
    let model_task_path = app_data_dir.join("model-task-matches.json");
    let task_preflight_path = app_data_dir.join("task-preflight-history.json");
    let task_sentinel = "LEGACY_TASK_SENTINEL";
    let draft_sentinel = "LEGACY_DRAFT_SENTINEL";
    write_private_app_data_fixture(
        &prompt_path,
        serde_json::to_vec_pretty(&vec![legacy_prompt_run_fixture(
            task_sentinel,
            draft_sentinel,
        )])
        .expect("serialize legacy prompt run"),
    );
    write_private_app_data_fixture(&model_task_path, br#"[{"task":"MODEL_TASK_SENTINEL"}]"#);
    write_private_app_data_fixture(
        &task_preflight_path,
        br#"[{"task":"TASK_PREFLIGHT_SENTINEL"}]"#,
    );
    let before = [
        fs::read(&prompt_path).expect("prompt bytes"),
        fs::read(&model_task_path).expect("model-task bytes"),
        fs::read(&task_preflight_path).expect("task-preflight bytes"),
    ];

    let preview_response = preview(&host);

    assert!(preview_response.ok, "{:?}", preview_response.error);
    let preview_result = preview_response.result.expect("privacy preview");
    assert_eq!(
        preview_result
            .pointer("/inspection/cleanup_source_count")
            .and_then(Value::as_u64),
        Some(3)
    );
    assert_eq!(
        preview_result
            .get("confirmation_required")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(fs::read(&prompt_path).expect("prompt unchanged"), before[0]);
    assert_eq!(
        fs::read(&model_task_path).expect("model-task unchanged"),
        before[1]
    );
    assert_eq!(
        fs::read(&task_preflight_path).expect("task-preflight unchanged"),
        before[2]
    );
    assert!(
        !app_data_dir.join("llm").exists(),
        "preview must not initialize replay state"
    );

    let confirmation = action_confirmation_from_preview(&preview_result);
    let response = host.handle(ServiceRequest {
        id: Some("privacy-apply".to_string()),
        method: "privacy.cleanupLegacyContent".to_string(),
        params: json!({ "action_confirmation": confirmation.clone() }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("privacy cleanup result");
    assert_eq!(
        result.get("cleaned_source_count").and_then(Value::as_u64),
        Some(3)
    );
    assert_eq!(
        result
            .pointer("/readback/verified")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        result
            .pointer("/inspection/cleanup_required")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(!model_task_path.exists());
    assert!(!task_preflight_path.exists());
    let sanitized = fs::read_to_string(&prompt_path).expect("sanitized prompt runs");
    assert!(!sanitized.contains(task_sentinel));
    assert!(!sanitized.contains(draft_sentinel));
    let runs: Vec<LlmPromptRunRecord> =
        serde_json::from_str(&sanitized).expect("parse sanitized prompt runs");
    assert_eq!(runs.len(), 1);
    assert!(runs[0].task.is_none());
    assert!(runs[0].draft_output.is_none());
    assert!(!runs[0].draft_requires_user_copy);
    assert!(!runs[0].raw_prompt_persisted);
    assert!(!runs[0].raw_response_persisted);
    assert!(!runs[0].redaction_summary.raw_prompt_persisted);
    assert!(!runs[0].redaction_summary.raw_response_persisted);
    assert!(!runs[0].safety_flags.raw_prompt_persisted);
    assert!(!runs[0].safety_flags.raw_response_persisted);

    let replay = host.handle(ServiceRequest {
        id: Some("privacy-replay".to_string()),
        method: "privacy.cleanupLegacyContent".to_string(),
        params: json!({ "action_confirmation": confirmation }),
    });
    assert!(!replay.ok);
    assert_eq!(
        replay.error.expect("replay rejection").code,
        "stale_action_reference"
    );
    let final_inspection = inspect(&host);
    assert!(final_inspection.ok, "{:?}", final_inspection.error);
    assert_eq!(
        final_inspection
            .result
            .expect("final inspection")
            .get("cleanup_required")
            .and_then(Value::as_bool),
        Some(false)
    );
    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn malformed_prompt_history_requires_confirmation_and_is_deleted() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-privacy-malformed-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let prompt_path = app_data_dir.join("prompt-runs.json");
    write_private_app_data_fixture(&prompt_path, b"{not-json");

    let (preview_result, response) = confirmed_action_request(
        &host,
        "privacy.previewCleanupLegacyContent",
        "privacy.cleanupLegacyContent",
        json!({}),
    );

    assert_eq!(
        preview_result
            .pointer("/inspection/sources/0/malformed")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        preview_result
            .pointer("/inspection/sources/0/cleanup_operation")
            .and_then(Value::as_str),
        Some("delete_leaf")
    );
    assert!(response.ok, "{:?}", response.error);
    assert!(!prompt_path.exists());
    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn privacy_cleanup_reservation_durability_failure_is_non_retryable_and_preserves_source() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-privacy-reservation-partial-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let source_path = app_data_dir.join("task-preflight-history.json");
    write_private_app_data_fixture(&source_path, br#"[{"task":"legacy"}]"#);
    let preview = preview(&host);
    assert!(preview.ok, "{:?}", preview.error);
    let confirmation =
        action_confirmation_from_preview(&preview.result.expect("privacy preview result"));
    crate::service_provider_actions::install_test_provider_action_state_fault(
        &app_data_dir,
        crate::service_provider_actions::TestProviderActionStateFault::ReservationDirectorySync,
    );

    let response = host.handle(ServiceRequest {
        id: Some("privacy-reservation-partial".to_string()),
        method: "privacy.cleanupLegacyContent".to_string(),
        params: json!({ "action_confirmation": confirmation }),
    });

    assert!(!response.ok);
    let error = response.error.expect("reservation partial effect");
    assert_eq!(error.code, "partial_effect");
    let details = error.details.expect("typed partial details");
    assert_eq!(details.state, "applied_unverified");
    assert!(details.cleanup_required);
    assert!(!details.retry_allowed);
    assert_eq!(
        fs::read(&source_path).expect("legacy source preserved"),
        br#"[{"task":"legacy"}]"#
    );
    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn privacy_cleanup_outcome_durability_failure_stays_non_retryable_after_effect() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-privacy-outcome-partial-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let source_path = app_data_dir.join("task-preflight-history.json");
    write_private_app_data_fixture(&source_path, br#"[{"task":"legacy"}]"#);
    let preview = preview(&host);
    assert!(preview.ok, "{:?}", preview.error);
    let confirmation =
        action_confirmation_from_preview(&preview.result.expect("privacy preview result"));
    crate::service_provider_actions::install_test_provider_action_state_fault(
        &app_data_dir,
        crate::service_provider_actions::TestProviderActionStateFault::OutcomeDirectorySync,
    );

    let response = host.handle(ServiceRequest {
        id: Some("privacy-outcome-partial".to_string()),
        method: "privacy.cleanupLegacyContent".to_string(),
        params: json!({ "action_confirmation": confirmation }),
    });

    assert!(!response.ok);
    let error = response.error.expect("outcome partial effect");
    assert_eq!(error.code, "partial_effect");
    let details = error.details.expect("typed partial details");
    assert_eq!(details.state, "applied_unverified");
    assert!(details.cleanup_required);
    assert!(!details.retry_allowed);
    assert!(
        !source_path.exists(),
        "the completed deletion must not be reported as unstarted"
    );
    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn stale_privacy_preview_does_not_touch_replacement_content() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-privacy-stale-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let source_path = app_data_dir.join("task-preflight-history.json");
    write_private_app_data_fixture(&source_path, br#"[{"task":"old"}]"#);
    let preview = preview(&host);
    assert!(preview.ok, "{:?}", preview.error);
    let confirmation =
        action_confirmation_from_preview(&preview.result.expect("stale preview result"));
    fs::remove_file(&source_path).expect("remove accepted source");
    write_private_app_data_fixture(&source_path, br#"[{"task":"replacement"}]"#);
    let replacement = fs::read(&source_path).expect("replacement bytes");

    let response = host.handle(ServiceRequest {
        id: Some("privacy-stale-apply".to_string()),
        method: "privacy.cleanupLegacyContent".to_string(),
        params: json!({ "action_confirmation": confirmation }),
    });

    assert!(!response.ok);
    assert_eq!(
        response.error.expect("stale cleanup error").code,
        "stale_action_reference"
    );
    assert_eq!(
        fs::read(&source_path).expect("preserved replacement"),
        replacement
    );
    assert!(
        !app_data_dir.join("llm").exists(),
        "stale apply must reject before replay reservation"
    );
    let _ = fs::remove_dir_all(app_data_dir);
}

#[cfg(unix)]
#[test]
fn privacy_cleanup_deletes_a_bound_symlink_without_touching_its_target() {
    use std::os::unix::fs::symlink;

    let root = env::temp_dir().join(format!(
        "skills-copilot-privacy-symlink-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let app_data_dir = root.join("app-data");
    let host = test_host(app_data_dir.clone());
    fs::create_dir_all(&app_data_dir).expect("create app data");
    let target = root.join("outside-history.json");
    fs::write(&target, b"OUTSIDE_SENTINEL").expect("write outside target");
    let link = app_data_dir.join("task-preflight-history.json");
    symlink(&target, &link).expect("create legacy symlink");

    let (_, response) = confirmed_action_request(
        &host,
        "privacy.previewCleanupLegacyContent",
        "privacy.cleanupLegacyContent",
        json!({}),
    );

    assert!(response.ok, "{:?}", response.error);
    assert!(!link.exists());
    assert_eq!(
        fs::read(&target).expect("outside target preserved"),
        b"OUTSIDE_SENTINEL"
    );
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn privacy_inspection_rejects_directories_and_hardlinks_without_mutation() {
    let root = env::temp_dir().join(format!(
        "skills-copilot-privacy-unsafe-leaves-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let app_data_dir = root.join("app-data");
    let host = test_host(app_data_dir.clone());
    fs::create_dir_all(app_data_dir.join("model-task-matches.json"))
        .expect("create unsafe legacy directory");
    let victim = root.join("victim");
    fs::write(&victim, b"VICTIM_SENTINEL").expect("write victim");
    fs::hard_link(&victim, app_data_dir.join("task-preflight-history.json"))
        .expect("create legacy hardlink");

    let response = inspect(&host);

    assert!(!response.ok);
    assert_eq!(
        response.error.expect("unsafe cleanup source error").code,
        "command_error"
    );
    assert!(app_data_dir.join("model-task-matches.json").is_dir());
    assert_eq!(
        fs::read(&victim).expect("victim preserved"),
        b"VICTIM_SENTINEL"
    );
    assert_eq!(
        fs::read(app_data_dir.join("task-preflight-history.json")).expect("hardlink preserved"),
        b"VICTIM_SENTINEL"
    );
    let _ = fs::remove_dir_all(root);
}
