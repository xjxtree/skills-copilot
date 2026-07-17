#[derive(Debug, Clone, Copy, Default)]
struct LocalSessionMetrics {
    user_message_count: usize,
    total_message_count: usize,
    tool_call_count: usize,
    skill_call_count: usize,
}

fn local_session_metrics(
    content_drafts: &[LocalSessionContentDraft],
    skill_call_count: usize,
) -> LocalSessionMetrics {
    let mut metrics = LocalSessionMetrics::default();
    for draft in content_drafts {
        match draft.kind.as_str() {
            "user_message" => {
                metrics.user_message_count += 1;
                metrics.total_message_count += 1;
            }
            "agent_reply" | "thinking" => {
                metrics.total_message_count += 1;
            }
            "tool_call" => {
                metrics.tool_call_count += 1;
            }
            "skill_call" => {}
            _ => {}
        }
    }

    metrics.skill_call_count = skill_call_count;
    metrics
}

fn json_session_role(map: &serde_json::Map<String, Value>) -> Option<&str> {
    let evidence = local_session_role_evidence(map);
    (!local_session_classification_is_rejected(evidence.classification))
        .then_some(evidence.role)
        .flatten()
}

fn local_session_title(path: &Path, short_hash: &str) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .map(|name| truncate_chars(name, 120))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("Local session {short_hash}"))
}

fn infer_local_session_agent(path: &Path) -> Option<String> {
    let normalized = path.to_string_lossy().to_ascii_lowercase();
    if normalized.contains(".claude") {
        Some("claude-code".to_string())
    } else if normalized.contains(".codex") {
        Some("codex".to_string())
    } else if normalized.contains("opencode") {
        Some("opencode".to_string())
    } else if normalized.contains(".pi/") {
        Some("pi".to_string())
    } else {
        None
    }
}

fn local_preview_redaction_summary_from(
    summary: LlmPromptRedactionSummary,
) -> LocalPreviewRedactionSummary {
    LocalPreviewRedactionSummary {
        status: "redacted-local-only".to_string(),
        redacted_value_count: summary.redacted_value_count,
        redacted_fields: summary.redacted_fields,
        placeholders: summary
            .placeholders
            .into_iter()
            .map(str::to_string)
            .collect(),
        raw_trace_persisted: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_secret_returned: summary.raw_secret_returned,
    }
}

fn local_preview_safety_flags() -> LocalPreviewSafetyFlags {
    LocalPreviewSafetyFlags {
        read_only: true,
        app_local_only: true,
        provider_request_sent: false,
        write_back_allowed: false,
        write_actions_available: false,
        skill_files_mutated: false,
        agent_config_mutated: false,
        script_execution_allowed: false,
        execution_actions_available: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        triage_mutation_allowed: false,
        credential_accessed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        cloud_sync_performed: false,
        telemetry_emitted: false,
    }
}
