use super::{skill_manager_fixtures::skill_manager_dispatch_params, *};

#[test]
fn supported_methods_have_dispatch_coverage() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-dispatch-coverage-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    for method in supported_methods() {
        let response = host.handle(ServiceRequest {
            id: Some(format!("dispatch-{method}")),
            method: method.to_string(),
            params: dispatch_coverage_params(method),
        });
        if let Some(error) = response.error {
            assert_ne!(
                error.code, "unknown_method",
                "supported method {method} was not covered by dispatch"
            );
        }
    }

    let unknown = host.handle(ServiceRequest {
        id: Some("dispatch-unknown".to_string()),
        method: "service.notReal".to_string(),
        params: Value::Null,
    });
    assert!(!unknown.ok);
    let error = unknown.error.expect("unknown method error");
    assert_eq!(error.code, "unknown_method");
    assert!(error.message.contains("service.notReal"));

    let _ = fs::remove_dir_all(app_data_dir);
}

fn dispatch_coverage_params(method: &str) -> Value {
    match method {
        "catalog.getSkill" | "config.toggleSkill" => {
            json!({ "instance_id": "missing-skill", "on": false })
        }
        "skill.exportBundle" => {
            json!({ "source_path": "/tmp/skills-copilot-missing-skill/SKILL.md" })
        }
        "skill.install" => json!({
            "instance_id": "missing-skill",
            "target_agent": "codex",
            "target_scope": "agent-global",
            "confirmed": false
        }),
        "llm.prepareAction" => json!({ "kind": "recommend", "user_intent": "fixture" }),
        "llm.previewSaveProviderProfile" => json!({
            "id": "dispatch-provider",
            "display_name": "Dispatch Provider",
            "provider_type": "openai-compatible",
            "base_url": "https://example.invalid/v1",
            "model": "dispatch-model",
            "enabled": false
        }),
        "llm.saveProviderProfile" => json!({
            "id": "dispatch-provider",
            "display_name": "Dispatch Provider",
            "provider_type": "openai-compatible",
            "base_url": "https://example.invalid/v1",
            "model": "dispatch-model",
            "enabled": false,
            "action_confirmation": {
                "reference": {
                    "action_id": "dispatch-provider-save",
                    "source_revision": "sha256:dispatch",
                    "target": {
                        "kind": "provider_profile",
                        "id": "dispatch-provider"
                    }
                },
                "preview_token": "action-preview:v1:hmac-sha256:dispatch",
                "confirmed": true
            }
        }),
        "llm.previewDeleteProviderProfile" => json!({
            "profile_id": "dispatch-provider",
            "delete_credential": false
        }),
        "llm.deleteProviderProfile" => json!({
            "profile_id": "dispatch-provider",
            "delete_credential": false,
            "action_confirmation": {
                "reference": {
                    "action_id": "dispatch-provider-delete",
                    "source_revision": "sha256:dispatch",
                    "target": {
                        "kind": "provider_profile",
                        "id": "dispatch-provider"
                    }
                },
                "preview_token": "action-preview:v1:hmac-sha256:dispatch",
                "confirmed": true
            }
        }),
        "llm.previewProviderConnectionTest" => json!({
            "profile_id": "dispatch-provider"
        }),
        "llm.testProviderConnection" => json!({
            "profile_id": "dispatch-provider",
            "action_confirmation": {
                "reference": {
                    "action_id": "dispatch-provider-test",
                    "source_revision": "sha256:dispatch",
                    "target": {
                        "kind": "provider_profile",
                        "id": "dispatch-provider"
                    }
                },
                "preview_token": "action-preview:v1:hmac-sha256:dispatch",
                "confirmed": true
            }
        }),
        "llm.previewPrompt" => json!({
            "action": "recommend",
            "user_intent": "fixture"
        }),
        "llm.confirmPromptAndSend" => json!({
            "action_confirmation": {
                "reference": {
                    "action_id": "dispatch-provider-prompt",
                    "source_revision": "sha256:dispatch",
                    "target": {
                        "kind": "provider_profile",
                        "id": "dispatch-provider"
                    }
                },
                "preview_token": "action-preview:v1:hmac-sha256:dispatch",
                "confirmed": true
            },
            "request": {
                "action": "recommend",
                "user_intent": "fixture"
            }
        }),
        "llm.listPromptRuns" => json!({
            "limit": 4
        }),
        "llm.providerObservability" => json!({
            "limit": 4
        }),
        "llm.listModelTaskMatches" => json!({
            "limit": 4
        }),
        "llm.recordModelTaskMatch" => json!({
            "id": "dispatch-model-task-match",
            "title": "Dispatch model-task match",
            "task": "Fixture local release audit task",
            "task_kind": "task_cockpit",
            "provider": "openai-compatible",
            "model": "dispatch-model",
            "match_status": "fit",
            "source_kind": "manual",
            "evidence_refs": ["dispatch:model-task"]
        }),
        "llm.deleteModelTaskMatch" => json!({
            "id": "dispatch-model-task-match"
        }),
        "app.search" => json!({
            "query": "fixture",
            "agent": "codex",
            "limit_per_kind": 3,
            "authorized_roots": ["/tmp/skills-copilot-fixture-sessions"],
            "auto_discover": false
        }),
        "session.previewLocalSessions" => json!({
            "authorized_roots": ["/tmp/skills-copilot-fixture-sessions"],
            "include_content_items": false,
            "paging_mode": "keyset",
            "limit": 4,
            "sort": "modified_at",
            "direction": "desc",
            "max_excerpt_chars": 800
        }),
        "session.listLocalSessionMessages" => json!({
            "authorized_roots": ["/tmp/skills-copilot-fixture-sessions"],
            "auto_discover": false,
            "agent": "codex",
            "session_id": "missing-session",
            "limit": 4
        }),
        "script.previewExecution" => json!({
            "instance_id": "missing-skill",
            "definition_id": "missing-definition",
            "agent": "codex"
        }),
        "script.execute" => json!({
            "command": ["echo", "blocked"],
            "confirmed": true
        }),
        method if method.starts_with("skillManager.") => skill_manager_dispatch_params(method),
        "config.readAgentConfig" => json!({ "agent": "codex" }),
        "snapshot.listAgentConfigPage" => json!({ "agent": "codex", "limit": 2 }),
        "skill.listEventsPage" => json!({ "instance_id": "missing-skill", "limit": 2 }),
        "config.previewSaveClaudeSettings" => json!({
            "content": "{}\n",
            "expected_revision": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
        }),
        "config.saveClaudeSettings" => json!({
            "content": "{}\n",
            "confirmation": {
                "reference": {
                    "action_id": "action:save_config:missing",
                    "source_revision": "sha256:missing",
                    "target": {
                        "kind": "config",
                        "id": "/tmp/missing/.claude/settings.json",
                        "agent": "claude-code",
                        "scope": "agent-global"
                    }
                },
                "preview_token": "action-preview:v1:hmac-sha256:missing",
                "confirmed": true
            }
        }),
        "project.setContext" | "project.validateContext" => {
            json!({ "root_path": "/tmp/skills-copilot-missing-project" })
        }
        "project.removeRecentContext" => json!({ "id": "missing-project" }),
        "snapshot.previewRollback" => {
            json!({ "snapshot_id": "missing-snapshot" })
        }
        "snapshot.rollback" => json!({
            "snapshot_id": "missing-snapshot",
            "confirmation": {
                "reference": {
                    "action_id": "action:rollback_config:missing",
                    "source_revision": "sha256:missing",
                    "target": {
                        "kind": "config",
                        "id": "/tmp/missing/.claude/settings.json",
                        "agent": "claude-code",
                        "scope": "agent-global"
                    }
                },
                "preview_token": "action-preview:v1:hmac-sha256:missing",
                "confirmed": true
            }
        }),
        "catalog.setFindingTriage" => json!({
            "triage_key": "missing-finding-key",
            "status": "reviewed"
        }),
        "catalog.clearFindingTriage" => json!({ "triage_key": "missing-finding-key" }),
        "rules.setSeverityOverride" => json!({
            "rule_id": "body.too-long",
            "severity": "info"
        }),
        "rules.clearSeverityOverride" => json!({ "rule_id": "body.too-long" }),
        "rules.setSuppression" => json!({
            "rule_id": "body.too-long",
            "reason": "local false positive"
        }),
        "rules.clearSuppression" => json!({ "rule_id": "body.too-long" }),
        _ => Value::Null,
    }
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAppVersion {
    pub(super) protocol_version: u32,
    pub(super) version: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireServiceStatus {
    pub(super) protocol_version: u32,
    pub(super) version: String,
    pub(super) app_data_dir: String,
    pub(super) catalog_path: String,
    pub(super) user_home: String,
    pub(super) supported_methods: Vec<String>,
    pub(super) refresh: WireRefreshStatus,
    pub(super) project_context: WireProjectContextSummary,
    pub(super) llm: WireLlmStatus,
    pub(super) script_execution: WireScriptExecutionStatus,
    pub(super) adapter_capabilities: Vec<WireAdapterCapabilityRecord>,
    #[serde(default)]
    pub(super) adapter_diagnostics: Option<Vec<WireAdapterDiagnosticsRecord>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireCrossAgentAnalysisRecord {
    pub(super) summary: WireCrossAgentAnalysisSummary,
    pub(super) groups: Vec<WireCrossAgentAnalysisGroup>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireCrossAgentAnalysisSummary {
    pub(super) total_groups: usize,
    pub(super) duplicate_name_groups: usize,
    pub(super) canonical_name_groups: usize,
    pub(super) path_overlap_groups: usize,
    pub(super) enabled_mismatch_groups: usize,
    pub(super) malformed_groups: usize,
    pub(super) precedence_groups: usize,
    pub(super) affected_skill_count: usize,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireCrossAgentAnalysisGroup {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) severity: String,
    pub(super) title: String,
    pub(super) canonical_name: Option<String>,
    pub(super) explanation: String,
    pub(super) instance_ids: Vec<String>,
    pub(super) winner_id: Option<String>,
    pub(super) agents: Vec<String>,
    pub(super) scopes: Vec<String>,
    pub(super) paths: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAdapterCapabilityRecord {
    pub(super) agent: String,
    pub(super) display_name: String,
    pub(super) status: String,
    pub(super) scan: WireAdapterFeatureCapability,
    pub(super) project_scan: WireAdapterFeatureCapability,
    pub(super) config_toggle: WireAdapterFeatureCapability,
    pub(super) config_snapshot: WireAdapterFeatureCapability,
    pub(super) install: WireAdapterFeatureCapability,
    pub(super) writable: WireAdapterFeatureCapability,
    pub(super) blockers: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAdapterFeatureCapability {
    pub(super) supported: bool,
    pub(super) status: String,
    pub(super) reason: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAdapterDiagnosticsRecord {
    pub(super) agent: String,
    pub(super) display_name: String,
    pub(super) status: String,
    pub(super) roots: Vec<WireAdapterDiagnosticRootRecord>,
    pub(super) config: WireAdapterDiagnosticConfigSummary,
    pub(super) access: WireAdapterDiagnosticAccessSummary,
    pub(super) last_scan: WireAdapterDiagnosticLastScan,
    pub(super) blockers: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAdapterDiagnosticRootRecord {
    pub(super) path: String,
    pub(super) scope: String,
    pub(super) source: String,
    pub(super) exists: bool,
    pub(super) status: String,
    pub(super) reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAdapterDiagnosticConfigSummary {
    pub(super) status: String,
    pub(super) detected_count: usize,
    pub(super) paths: Vec<WireAdapterDiagnosticConfigPath>,
    pub(super) reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAdapterDiagnosticConfigPath {
    pub(super) path: String,
    pub(super) detected: bool,
    pub(super) status: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAdapterDiagnosticAccessSummary {
    pub(super) read_only: bool,
    pub(super) writable_supported: bool,
    pub(super) writable_status: String,
    pub(super) writable_reason: Option<String>,
    pub(super) install_supported: bool,
    pub(super) install_status: String,
    pub(super) install_reason: Option<String>,
    pub(super) read_only_reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAdapterDiagnosticLastScan {
    pub(super) status: String,
    pub(super) reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLocalPreviewRedactionSummary {
    pub(super) status: String,
    pub(super) redacted_value_count: usize,
    pub(super) redacted_fields: Vec<String>,
    pub(super) placeholders: Vec<String>,
    pub(super) raw_trace_persisted: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) raw_secret_returned: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLocalPreviewSafetyFlags {
    pub(super) read_only: bool,
    pub(super) app_local_only: bool,
    pub(super) provider_request_sent: bool,
    pub(super) write_back_allowed: bool,
    pub(super) write_actions_available: bool,
    pub(super) skill_files_mutated: bool,
    pub(super) agent_config_mutated: bool,
    pub(super) script_execution_allowed: bool,
    pub(super) execution_actions_available: bool,
    pub(super) config_mutation_allowed: bool,
    pub(super) snapshot_created: bool,
    pub(super) triage_mutation_allowed: bool,
    pub(super) credential_accessed: bool,
    pub(super) raw_secret_returned: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) raw_trace_persisted: bool,
    pub(super) cloud_sync_performed: bool,
    pub(super) telemetry_emitted: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[rustfmt::skip]
pub(super) struct WireLocalSessionPreviewRoot { pub(super) root: String, pub(super) status: String, pub(super) candidate_count: usize, pub(super) blocker: Option<String> }

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[rustfmt::skip]
pub(super) struct WireLocalSessionContentItem { pub(super) id: String, pub(super) kind: String, pub(super) title: String, pub(super) text: String, pub(super) char_count: usize, pub(super) timestamp: Option<i64>, pub(super) evidence_refs: Vec<String> }

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[rustfmt::skip]
pub(super) struct WireLocalSessionPreviewRow {
    pub(super) id: String, pub(super) title: String, pub(super) source_kind: String,
    pub(super) agent: Option<String>, pub(super) redacted_path: String, pub(super) modified_at: Option<i64>,
    pub(super) started_at: Option<i64>, pub(super) ended_at: Option<i64>, pub(super) excerpt: String,
    pub(super) excerpt_char_count: usize, pub(super) user_message_count: usize,
    pub(super) total_message_count: usize, pub(super) tool_call_count: usize,
    pub(super) skill_call_count: usize, pub(super) content_hash: String,
    pub(super) evidence_refs: Vec<String>, pub(super) content_included: bool,
    pub(super) content_items: Vec<WireLocalSessionContentItem>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[rustfmt::skip]
pub(super) struct WireLocalSessionSkillUsageRow {
    pub(super) skill_id: String, pub(super) skill_name: String,
    pub(super) agent: String, pub(super) call_count: usize,
    pub(super) session_count: usize, pub(super) latest_modified_at: Option<i64>,
    pub(super) evidence_refs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[rustfmt::skip]
pub(super) struct WireLocalSessionPreviewResult {
    pub(super) generated_by: String, pub(super) authorized: bool,
    pub(super) authorization_required: bool, pub(super) roots: Vec<WireLocalSessionPreviewRoot>,
    pub(super) count: usize, pub(super) total_candidate_count: usize,
    pub(super) total_matched_count: usize, pub(super) offset: usize,
    pub(super) limit: usize, pub(super) has_more: bool,
    pub(super) next_offset: Option<usize>,
    pub(super) next_cursor: Option<String>,
    pub(super) source_revision: Option<String>,
    pub(super) source_completeness: String,
    pub(super) incomplete_reason: Option<String>,
    pub(super) candidate_set_truncated: bool,
    pub(super) user_message_count: usize, pub(super) total_message_count: usize,
    pub(super) tool_call_count: usize, pub(super) skill_call_count: usize,
    pub(super) skill_usage_rows: Vec<WireLocalSessionSkillUsageRow>,
    pub(super) session_rows: Vec<WireLocalSessionPreviewRow>, pub(super) gap_notes: Vec<String>,
    pub(super) blocker_notes: Vec<String>,
    pub(super) redaction_summary: WireLocalPreviewRedactionSummary,
    pub(super) safety_flags: WireLocalPreviewSafetyFlags, pub(super) read_only: bool,
    pub(super) provider_request_sent: bool, pub(super) skill_files_mutated: bool,
    pub(super) agent_config_mutated: bool, pub(super) snapshot_created: bool,
    pub(super) triage_mutated: bool, pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool, pub(super) raw_trace_persisted: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
#[rustfmt::skip]
pub(super) struct WireLocalSessionMessagePageResult {
    pub(super) generated_by: String, pub(super) session_id: String,
    pub(super) content_items: Vec<WireLocalSessionContentItem>,
    pub(super) returned_count: usize, pub(super) total_count: Option<usize>,
    pub(super) has_more: bool, pub(super) next_cursor: Option<String>,
    pub(super) source_revision: String, pub(super) source_completeness: String,
    pub(super) incomplete_reason: Option<String>, pub(super) scanned_bytes: u64,
    pub(super) scanned_through_bytes: u64, pub(super) snapshot_bytes: u64,
    pub(super) redaction_summary: WireLocalPreviewRedactionSummary,
    pub(super) safety_flags: WireLocalPreviewSafetyFlags, pub(super) read_only: bool,
    pub(super) provider_request_sent: bool, pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool, pub(super) raw_trace_persisted: bool,
}
#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireScanResult {
    pub(super) scanned_count: usize,
    pub(super) skills: Vec<WireSkillRecord>,
    pub(super) activity: WireRefreshActivity,
    pub(super) accepted_context_revision: String,
    pub(super) catalog_scan_revision: String,
    pub(super) readback: WireCatalogScanReadback,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireCatalogScanReadback {
    pub(super) accepted_context_revision: String,
    pub(super) catalog_scan_revision: String,
    pub(super) verified: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireRefreshStatus {
    pub(super) scan_progress: String,
    pub(super) watcher_state: String,
    pub(super) watcher_detail: String,
    pub(super) recovery_actions: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmStatus {
    pub(super) enabled: bool,
    pub(super) configured: bool,
    pub(super) provider: Option<String>,
    pub(super) model: Option<String>,
    pub(super) reason: String,
    pub(super) single_request_token_limit: u32,
    pub(super) monthly_budget_usd: f64,
    pub(super) credentials_storage: String,
    pub(super) credential_persistence_allowed: bool,
    pub(super) provider_profile_count: usize,
    pub(super) default_profile_id: Option<String>,
    pub(super) profiles_path: String,
    pub(super) call_metadata_path: String,
    pub(super) raw_prompt_persistence_allowed: bool,
    pub(super) raw_response_persistence_allowed: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireListProviderProfilesResult {
    pub(super) profiles: Vec<WireProviderProfileRecord>,
    pub(super) default_profile_id: Option<String>,
    pub(super) credential_storage: String,
    pub(super) credential_persistence_allowed: bool,
    pub(super) raw_secrets_returned: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireProviderActionPreviewResult {
    pub(super) action: WireActionDescriptor,
    pub(super) preconditions: Vec<WireActionPrecondition>,
    pub(super) preview_token: String,
    pub(super) operation: String,
    pub(super) profile_id: String,
    pub(super) provider_type: String,
    pub(super) destination_host: String,
    pub(super) model: String,
    pub(super) expected_revision: String,
    pub(super) credential_change: bool,
    pub(super) raw_secret_returned: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireSaveProviderProfileResult {
    pub(super) profile: WireProviderProfileRecord,
    pub(super) credential_status: WireProviderCredentialStatus,
    pub(super) profile_persisted: bool,
    pub(super) credential_effect: String,
    pub(super) error_code: Option<String>,
    pub(super) error_message: Option<String>,
    pub(super) raw_secret_returned: bool,
    pub(super) outcome: WireProviderActionExecutionOutcome,
    pub(super) readback: Option<WireActionReadbackRecord>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireDeleteProviderProfileResult {
    pub(super) deleted_profile_id: String,
    pub(super) profile_deleted: bool,
    pub(super) credential_deleted: bool,
    pub(super) credential_effect: String,
    pub(super) error_code: Option<String>,
    pub(super) error_message: Option<String>,
    pub(super) raw_secret_returned: bool,
    pub(super) outcome: WireProviderActionExecutionOutcome,
    pub(super) readback: Option<WireActionReadbackRecord>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireTestProviderConnectionResult {
    pub(super) profile_id: String,
    pub(super) provider_type: String,
    pub(super) model: String,
    pub(super) destination_host: String,
    pub(super) status: String,
    pub(super) provider_request_sent: bool,
    pub(super) credential_accessed: bool,
    pub(super) duration_ms: u128,
    pub(super) error_code: Option<String>,
    pub(super) error_message: Option<String>,
    pub(super) budget: WireProviderBudgetStatus,
    pub(super) audit: WireProviderCallMetadata,
    pub(super) local_metadata_persisted: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) raw_secret_returned: bool,
    pub(super) outcome: WireProviderActionExecutionOutcome,
    pub(super) readback: Option<WireActionReadbackRecord>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireProviderActionExecutionOutcome {
    pub(super) state: String,
    pub(super) effect: String,
    pub(super) remote_effect: String,
    pub(super) local_effect: String,
    pub(super) credential_effect: String,
    pub(super) recovery: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireProviderProfileRecord {
    pub(super) id: String,
    pub(super) display_name: String,
    pub(super) provider_type: String,
    pub(super) base_url: String,
    pub(super) model: String,
    pub(super) enabled: bool,
    pub(super) api_version: Option<String>,
    pub(super) organization: Option<String>,
    pub(super) single_request_token_limit: u32,
    pub(super) monthly_budget_usd: f64,
    pub(super) credential_reference: WireProviderCredentialReference,
    pub(super) credential_status: WireProviderCredentialStatus,
    pub(super) created_at: i64,
    pub(super) updated_at: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireProviderCredentialReference {
    pub(super) storage: String,
    pub(super) service: String,
    pub(super) account: String,
    pub(super) secret_persisted: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireProviderCredentialStatus {
    pub(super) state: String,
    pub(super) reason: String,
    pub(super) secret_available: bool,
    pub(super) fallback_available: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireProviderBudgetStatus {
    pub(super) single_request_token_limit: u32,
    pub(super) monthly_budget_usd: f64,
    pub(super) estimated_test_tokens: u32,
    pub(super) estimated_test_cost_usd: f64,
    pub(super) state: String,
    pub(super) reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireProviderCallMetadata {
    pub(super) timestamp: i64,
    pub(super) action_type: String,
    pub(super) profile_id: String,
    pub(super) provider_type: String,
    pub(super) model: String,
    pub(super) destination_host: String,
    pub(super) status: String,
    pub(super) error_code: Option<String>,
    pub(super) error_message: Option<String>,
    pub(super) duration_ms: u128,
    pub(super) estimated_input_tokens: u32,
    pub(super) estimated_output_tokens: u32,
    pub(super) estimated_cost_usd: f64,
    pub(super) confirmation_id: String,
    pub(super) redaction_status: String,
    pub(super) provider_request_sent: bool,
    pub(super) credential_accessed: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireScriptExecutionStatus {
    pub(super) enabled: bool,
    pub(super) default_enabled: bool,
    pub(super) reason: String,
    pub(super) audit_scope: String,
    pub(super) audit_path: String,
    pub(super) llm_initiation_allowed: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmPrepareActionResult {
    pub(super) action: String,
    pub(super) allowed: bool,
    pub(super) reason: String,
    pub(super) disabled_reason: Option<String>,
    pub(super) requires_confirmation: bool,
    pub(super) write_back_allowed: bool,
    pub(super) draft_requires_user_copy: bool,
    pub(super) provider: Option<String>,
    pub(super) model: Option<String>,
    pub(super) estimated_input_tokens: u32,
    pub(super) estimated_output_tokens: u32,
    pub(super) estimated_total_tokens: u32,
    pub(super) estimated_cost_usd: f64,
    pub(super) single_request_token_limit: u32,
    pub(super) monthly_budget_usd: f64,
    pub(super) credentials_storage: String,
    pub(super) credential_persistence_allowed: bool,
    pub(super) prompt_scope: Vec<String>,
    pub(super) privacy_notes: Vec<String>,
    pub(super) confirmation: WireLlmConfirmationRequirement,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmPreviewPromptResult {
    pub(super) preview_id: String,
    pub(super) status: String,
    pub(super) allowed: bool,
    pub(super) reason: String,
    pub(super) request_kind: String,
    pub(super) action: WireActionDescriptor,
    pub(super) preconditions: Vec<WireActionPrecondition>,
    pub(super) preview_token: String,
    pub(super) profile_id: Option<String>,
    pub(super) provider: Option<String>,
    pub(super) model: Option<String>,
    pub(super) endpoint: Option<String>,
    pub(super) destination_host: Option<String>,
    pub(super) prompt_scope: Vec<String>,
    pub(super) included_fields: Vec<String>,
    pub(super) excluded_fields: Vec<String>,
    pub(super) redaction: WireLlmPromptRedactionSummary,
    pub(super) prompt_preview: String,
    pub(super) estimated_input_tokens: u32,
    pub(super) estimated_output_tokens: u32,
    pub(super) estimated_total_tokens: u32,
    pub(super) estimated_cost_usd: f64,
    pub(super) single_request_token_limit: u32,
    pub(super) monthly_budget_usd: f64,
    pub(super) requires_confirmation: bool,
    pub(super) confirmation: WireLlmConfirmationRequirement,
    pub(super) write_back_allowed: bool,
    pub(super) draft_requires_user_copy: bool,
    pub(super) provider_request_sent: bool,
    pub(super) raw_secret_returned: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmPromptRedactionSummary {
    pub(super) status: String,
    pub(super) redacted_value_count: usize,
    pub(super) redacted_fields: Vec<String>,
    pub(super) placeholders: Vec<String>,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) raw_secret_returned: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmConfirmPromptAndSendResult {
    pub(super) preview_id: String,
    pub(super) confirmation_id: String,
    pub(super) status: String,
    pub(super) request_kind: String,
    pub(super) profile_id: String,
    pub(super) provider: String,
    pub(super) model: String,
    pub(super) destination_host: String,
    pub(super) provider_request_sent: bool,
    pub(super) credential_accessed: bool,
    pub(super) draft_output: Option<String>,
    pub(super) draft_requires_user_copy: bool,
    pub(super) write_back_allowed: bool,
    pub(super) script_execution_allowed: bool,
    pub(super) config_mutation_allowed: bool,
    pub(super) snapshot_created: bool,
    pub(super) triage_mutation_allowed: bool,
    pub(super) audit: WireProviderCallMetadata,
    pub(super) readback: Option<WireActionReadbackRecord>,
    pub(super) partial_outcome: Option<WireLlmPromptPartialOutcome>,
    pub(super) raw_secret_returned: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmPromptPartialOutcome {
    pub(super) remote_effect: String,
    pub(super) local_record: String,
    pub(super) recovery: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmPromptRunListResult {
    pub(super) generated_by: String,
    pub(super) count: usize,
    pub(super) total_count: usize,
    pub(super) returned_count: usize,
    pub(super) limit: Option<usize>,
    pub(super) truncated: bool,
    pub(super) runs: Vec<WireLlmPromptRunRecord>,
    pub(super) app_local_only: bool,
    pub(super) runs_file: String,
    pub(super) provider_request_sent: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) raw_secret_returned: bool,
    pub(super) safety_flags: WireLlmPromptRunSafetyFlags,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmPromptRunRecord {
    pub(super) id: String,
    pub(super) preview_id: String,
    pub(super) confirmation_id: String,
    pub(super) action: String,
    pub(super) request_kind: String,
    pub(super) analysis_kind: Option<String>,
    pub(super) scope: Option<String>,
    pub(super) instance_id: Option<String>,
    pub(super) instance_ids: Vec<String>,
    pub(super) definition_id: Option<String>,
    pub(super) agent: Option<String>,
    pub(super) task: Option<String>,
    pub(super) profile_id: String,
    pub(super) provider: String,
    pub(super) model: String,
    pub(super) destination_host: String,
    pub(super) status: String,
    pub(super) error_code: Option<String>,
    pub(super) error_message: Option<String>,
    pub(super) duration_ms: u64,
    pub(super) estimated_input_tokens: u32,
    pub(super) estimated_output_tokens: u32,
    pub(super) estimated_total_tokens: u32,
    pub(super) estimated_cost_usd: f64,
    pub(super) draft_output: Option<String>,
    pub(super) draft_requires_user_copy: bool,
    pub(super) provider_request_sent: bool,
    pub(super) credential_accessed: bool,
    pub(super) raw_secret_returned: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) redaction_summary: WireLlmPromptRunRedactionSummary,
    pub(super) created_at: i64,
    pub(super) completed_at: i64,
    pub(super) safety_flags: WireLlmPromptRunSafetyFlags,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmPromptRunRedactionSummary {
    pub(super) status: String,
    pub(super) redacted_value_count: usize,
    pub(super) redacted_fields: Vec<String>,
    pub(super) placeholders: Vec<String>,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) raw_trace_persisted: bool,
    pub(super) raw_secret_returned: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmPromptRunSafetyFlags {
    pub(super) app_local_only: bool,
    pub(super) provider_request_sent: bool,
    pub(super) credential_accessed: bool,
    pub(super) draft_copy_only: bool,
    pub(super) write_back_allowed: bool,
    pub(super) write_actions_available: bool,
    pub(super) skill_files_mutated: bool,
    pub(super) agent_config_mutated: bool,
    pub(super) script_execution_allowed: bool,
    pub(super) execution_actions_available: bool,
    pub(super) config_mutation_allowed: bool,
    pub(super) snapshot_created: bool,
    pub(super) triage_mutation_allowed: bool,
    pub(super) raw_secret_returned: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) raw_trace_persisted: bool,
    pub(super) cloud_sync_performed: bool,
    pub(super) telemetry_emitted: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmProviderObservabilityResult {
    pub(super) generated_by: String,
    pub(super) status: String,
    pub(super) filters: Value,
    pub(super) summary: WireLlmProviderObservabilitySummary,
    pub(super) call_rows: Vec<WireLlmProviderObservabilityCallRow>,
    pub(super) history_rows: Vec<WireLlmProviderObservabilityHistoryRow>,
    pub(super) grouping_rows: Vec<WireLlmProviderObservabilityGroupingRow>,
    #[serde(default)]
    pub(super) model_task_history_rows: Vec<WireModelTaskMatchEvidenceRow>,
    pub(super) status_rows: Vec<WireLlmProviderObservabilityStatusRow>,
    pub(super) budget_usage_hints: Vec<WireLlmProviderObservabilityBudgetUsageHint>,
    pub(super) retention_recommendations:
        Vec<WireLlmProviderObservabilityRetentionRecommendationRow>,
    pub(super) gap_notes: Vec<String>,
    pub(super) blocker_notes: Vec<String>,
    pub(super) evidence_references: Vec<WireLlmProviderObservabilityEvidenceReference>,
    pub(super) prompt_metadata: WireLlmProviderObservabilityPromptMetadata,
    pub(super) safety_flags: WireLlmProviderObservabilitySafetyFlags,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireProviderActivityPageResult {
    pub(super) generated_by: String,
    pub(super) rows: Vec<WireProviderActivityRow>,
    pub(super) source_revision: String,
    pub(super) returned_count: usize,
    pub(super) total_count: Option<usize>,
    pub(super) has_more: bool,
    pub(super) next_cursor: Option<String>,
    pub(super) source_completeness: String,
    pub(super) incomplete_reason: Option<String>,
    pub(super) safety_flags: WireLlmProviderObservabilitySafetyFlags,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireProviderActivityRow {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) timestamp: i64,
    pub(super) title: String,
    pub(super) subtitle: String,
    pub(super) status: String,
    pub(super) evidence_refs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmProviderObservabilitySummary {
    pub(super) total_prompt_run_count: usize,
    pub(super) total_call_metadata_count: usize,
    pub(super) returned_prompt_run_count: usize,
    pub(super) returned_call_row_count: usize,
    pub(super) provider_profile_count: usize,
    pub(super) enabled_profile_count: usize,
    pub(super) grouping_count: usize,
    pub(super) observed_provider_request_row_count: usize,
    pub(super) observed_credential_access_row_count: usize,
    pub(super) succeeded_count: usize,
    pub(super) failed_count: usize,
    pub(super) estimated_input_tokens: u64,
    pub(super) estimated_output_tokens: u64,
    pub(super) estimated_total_tokens: u64,
    pub(super) estimated_cost_usd: f64,
    pub(super) latest_activity_at: Option<i64>,
    pub(super) summary: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmProviderObservabilityCallRow {
    pub(super) id: String,
    pub(super) source: String,
    pub(super) timestamp: i64,
    pub(super) action_type: String,
    pub(super) profile_id: String,
    pub(super) provider: String,
    pub(super) model: String,
    pub(super) destination_host: String,
    pub(super) status: String,
    pub(super) error_code: Option<String>,
    pub(super) error_message: Option<String>,
    pub(super) duration_ms: u128,
    pub(super) estimated_input_tokens: u32,
    pub(super) estimated_output_tokens: u32,
    pub(super) estimated_total_tokens: u32,
    pub(super) estimated_cost_usd: f64,
    pub(super) recorded_provider_request_sent: bool,
    pub(super) recorded_credential_accessed: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) redaction_status: String,
    pub(super) evidence_refs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmProviderObservabilityHistoryRow {
    pub(super) id: String,
    pub(super) source: String,
    pub(super) prompt_run_id: String,
    pub(super) created_at: i64,
    pub(super) completed_at: i64,
    pub(super) action: String,
    pub(super) request_kind: String,
    pub(super) analysis_kind: Option<String>,
    pub(super) scope: Option<String>,
    pub(super) instance_id: Option<String>,
    pub(super) instance_ids: Vec<String>,
    pub(super) definition_id: Option<String>,
    pub(super) agent: Option<String>,
    pub(super) task: Option<String>,
    pub(super) profile_id: String,
    pub(super) provider: String,
    pub(super) model: String,
    pub(super) destination_host: String,
    pub(super) status: String,
    pub(super) error_code: Option<String>,
    pub(super) error_message: Option<String>,
    pub(super) duration_ms: u64,
    pub(super) estimated_input_tokens: u32,
    pub(super) estimated_output_tokens: u32,
    pub(super) estimated_total_tokens: u32,
    pub(super) estimated_cost_usd: f64,
    pub(super) draft_output_available: bool,
    pub(super) draft_requires_user_copy: bool,
    pub(super) recorded_provider_request_sent: bool,
    pub(super) recorded_credential_accessed: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) redaction_status: String,
    pub(super) evidence_refs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmProviderObservabilityGroupingRow {
    pub(super) id: String,
    pub(super) provider: String,
    pub(super) model: String,
    pub(super) destination_host: String,
    pub(super) profile_ids: Vec<String>,
    pub(super) prompt_run_count: usize,
    pub(super) call_metadata_count: usize,
    pub(super) recorded_provider_request_count: usize,
    pub(super) recorded_credential_access_count: usize,
    pub(super) succeeded_count: usize,
    pub(super) failed_count: usize,
    pub(super) estimated_total_tokens: u64,
    pub(super) estimated_cost_usd: f64,
    pub(super) latest_activity_at: Option<i64>,
    pub(super) evidence_refs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmProviderObservabilityStatusRow {
    pub(super) id: String,
    pub(super) source: String,
    pub(super) status: String,
    pub(super) severity: String,
    pub(super) message: String,
    pub(super) count: usize,
    pub(super) evidence_refs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmProviderObservabilityBudgetUsageHint {
    pub(super) id: String,
    pub(super) profile_id: String,
    pub(super) provider: String,
    pub(super) model: String,
    pub(super) destination_host: String,
    pub(super) enabled: bool,
    pub(super) single_request_token_limit: u32,
    pub(super) monthly_budget_usd: f64,
    pub(super) observed_prompt_run_count: usize,
    pub(super) observed_call_metadata_count: usize,
    pub(super) observed_estimated_total_tokens: u64,
    pub(super) observed_estimated_cost_usd: f64,
    pub(super) budget_state: String,
    pub(super) reason: String,
    pub(super) evidence_refs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmProviderObservabilityRetentionRecommendationRow {
    pub(super) id: String,
    pub(super) source_file: String,
    pub(super) current_record_count: usize,
    pub(super) recommendation: String,
    pub(super) cleanup_action_available: bool,
    pub(super) write_action_available: bool,
    pub(super) evidence_refs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmProviderObservabilityEvidenceReference {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) label: String,
    pub(super) source: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmProviderObservabilityPromptMetadata {
    pub(super) available: bool,
    pub(super) preview_method: String,
    pub(super) confirm_method: String,
    pub(super) provider_request_sent: bool,
    pub(super) copy_only: bool,
    pub(super) note: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmProviderObservabilitySafetyFlags {
    pub(super) read_only: bool,
    pub(super) app_local_only: bool,
    pub(super) provider_request_sent: bool,
    pub(super) credential_accessed: bool,
    pub(super) draft_copy_only: bool,
    pub(super) write_back_allowed: bool,
    pub(super) write_actions_available: bool,
    pub(super) skill_files_mutated: bool,
    pub(super) agent_config_mutated: bool,
    pub(super) script_execution_allowed: bool,
    pub(super) execution_actions_available: bool,
    pub(super) config_mutation_allowed: bool,
    pub(super) snapshot_created: bool,
    pub(super) triage_mutation_allowed: bool,
    pub(super) raw_secret_returned: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) raw_trace_persisted: bool,
    pub(super) unredacted_paths_returned: bool,
    pub(super) cloud_sync_performed: bool,
    pub(super) telemetry_emitted: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireModelTaskMatchListResult {
    pub(super) generated_by: String,
    pub(super) status: String,
    pub(super) total_record_count: usize,
    pub(super) returned_record_count: usize,
    pub(super) total_evidence_count: usize,
    pub(super) returned_evidence_count: usize,
    pub(super) limit: Option<usize>,
    pub(super) truncated: bool,
    pub(super) summary: WireModelTaskMatchSummary,
    pub(super) records: Vec<WireModelTaskMatchRecord>,
    pub(super) model_rows: Vec<WireModelTaskMatchModelRow>,
    pub(super) task_rows: Vec<WireModelTaskMatchTaskRow>,
    pub(super) recent_evidence_rows: Vec<WireModelTaskMatchEvidenceRow>,
    pub(super) gap_notes: Vec<String>,
    pub(super) blocker_notes: Vec<String>,
    pub(super) evidence_references: Vec<WireLlmProviderObservabilityEvidenceReference>,
    pub(super) app_local_only: bool,
    pub(super) history_file: String,
    pub(super) provider_request_sent: bool,
    pub(super) credential_accessed: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) raw_trace_persisted: bool,
    pub(super) safety_flags: WireModelTaskMatchSafetyFlags,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireModelTaskMatchRecordResult {
    pub(super) generated_by: String,
    pub(super) record: WireModelTaskMatchRecord,
    pub(super) count: usize,
    pub(super) app_local_only: bool,
    pub(super) history_file: String,
    pub(super) provider_request_sent: bool,
    pub(super) skill_files_mutated: bool,
    pub(super) agent_config_mutated: bool,
    pub(super) snapshot_created: bool,
    pub(super) triage_mutated: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) raw_trace_persisted: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireModelTaskMatchDeleteResult {
    pub(super) record_id: String,
    pub(super) deleted: bool,
    pub(super) remaining_count: usize,
    pub(super) app_local_only: bool,
    pub(super) provider_request_sent: bool,
    pub(super) skill_files_mutated: bool,
    pub(super) agent_config_mutated: bool,
    pub(super) snapshot_created: bool,
    pub(super) triage_mutated: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) raw_trace_persisted: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireModelTaskMatchRecord {
    pub(super) id: String,
    pub(super) title: String,
    pub(super) task: String,
    pub(super) task_kind: String,
    pub(super) agent: Option<String>,
    pub(super) profile_id: Option<String>,
    pub(super) provider: String,
    pub(super) model: String,
    pub(super) destination_host: Option<String>,
    pub(super) match_status: String,
    pub(super) confidence_score: Option<u8>,
    pub(super) latency_ms: Option<u64>,
    pub(super) estimated_total_tokens: Option<u32>,
    pub(super) estimated_cost_usd: Option<f64>,
    pub(super) source_kind: String,
    pub(super) prompt_run_ids: Vec<String>,
    pub(super) benchmark_ids: Vec<String>,
    pub(super) evidence_refs: Vec<String>,
    pub(super) gap_notes: Vec<String>,
    pub(super) blocker_notes: Vec<String>,
    pub(super) outcome_notes: Vec<String>,
    pub(super) created_at: i64,
    pub(super) updated_at: i64,
    pub(super) redaction_summary: WireLlmPromptRunRedactionSummary,
    pub(super) safety_flags: WireModelTaskMatchSafetyFlags,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireModelTaskMatchSummary {
    pub(super) stored_record_count: usize,
    pub(super) prompt_run_count: usize,
    pub(super) returned_record_count: usize,
    pub(super) returned_prompt_run_count: usize,
    pub(super) model_count: usize,
    pub(super) task_kind_count: usize,
    pub(super) fit_count: usize,
    pub(super) partial_fit_count: usize,
    pub(super) mismatch_count: usize,
    pub(super) unknown_count: usize,
    pub(super) estimated_total_tokens: u64,
    pub(super) estimated_cost_usd: f64,
    pub(super) latest_activity_at: Option<i64>,
    pub(super) summary: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireModelTaskMatchModelRow {
    pub(super) id: String,
    pub(super) provider: String,
    pub(super) model: String,
    pub(super) destination_host: Option<String>,
    pub(super) stored_record_count: usize,
    pub(super) prompt_run_count: usize,
    pub(super) fit_count: usize,
    pub(super) partial_fit_count: usize,
    pub(super) mismatch_count: usize,
    pub(super) unknown_count: usize,
    pub(super) estimated_total_tokens: u64,
    pub(super) estimated_cost_usd: f64,
    pub(super) latest_activity_at: Option<i64>,
    pub(super) evidence_refs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireModelTaskMatchTaskRow {
    pub(super) id: String,
    pub(super) task_kind: String,
    pub(super) status: String,
    pub(super) stored_record_count: usize,
    pub(super) prompt_run_count: usize,
    pub(super) fit_count: usize,
    pub(super) partial_fit_count: usize,
    pub(super) mismatch_count: usize,
    pub(super) unknown_count: usize,
    pub(super) estimated_total_tokens: u64,
    pub(super) estimated_cost_usd: f64,
    pub(super) latest_activity_at: Option<i64>,
    pub(super) evidence_refs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireModelTaskMatchEvidenceRow {
    pub(super) id: String,
    pub(super) source: String,
    pub(super) source_kind: String,
    pub(super) title: String,
    pub(super) task: Option<String>,
    pub(super) task_kind: String,
    pub(super) agent: Option<String>,
    pub(super) provider: String,
    pub(super) model: String,
    pub(super) destination_host: Option<String>,
    pub(super) match_status: String,
    pub(super) confidence_score: Option<u8>,
    pub(super) status: String,
    pub(super) created_at: i64,
    pub(super) updated_at: Option<i64>,
    pub(super) latency_ms: Option<u64>,
    pub(super) estimated_total_tokens: u32,
    pub(super) estimated_cost_usd: f64,
    pub(super) gap_notes: Vec<String>,
    pub(super) blocker_notes: Vec<String>,
    pub(super) outcome_notes: Vec<String>,
    pub(super) evidence_refs: Vec<String>,
    pub(super) redaction_status: String,
    pub(super) safety_flags: WireModelTaskMatchSafetyFlags,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireModelTaskMatchSafetyFlags {
    pub(super) read_only: bool,
    pub(super) app_local_only: bool,
    pub(super) provider_request_sent: bool,
    pub(super) credential_accessed: bool,
    pub(super) draft_copy_only: bool,
    pub(super) write_back_allowed: bool,
    pub(super) write_actions_available: bool,
    pub(super) skill_files_mutated: bool,
    pub(super) agent_config_mutated: bool,
    pub(super) script_execution_allowed: bool,
    pub(super) execution_actions_available: bool,
    pub(super) config_mutation_allowed: bool,
    pub(super) snapshot_created: bool,
    pub(super) triage_mutation_allowed: bool,
    pub(super) raw_secret_returned: bool,
    pub(super) raw_prompt_persisted: bool,
    pub(super) raw_response_persisted: bool,
    pub(super) raw_trace_persisted: bool,
    pub(super) unredacted_paths_returned: bool,
    pub(super) cloud_sync_performed: bool,
    pub(super) telemetry_emitted: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireLlmConfirmationRequirement {
    pub(super) required: bool,
    pub(super) message: String,
    pub(super) display_fields: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireScriptExecutionPreviewRecord {
    pub(super) skill_instance_id: Option<String>,
    pub(super) initiated_by: String,
    pub(super) initiator_allowed: bool,
    pub(super) cwd: WireScriptExecutionCwdScope,
    pub(super) env: WireScriptExecutionEnvScope,
    pub(super) network: WireScriptExecutionNetworkScope,
    pub(super) files: WireScriptExecutionFilesScope,
    pub(super) command_preview: WireScriptExecutionCommandPreview,
    pub(super) risks: Vec<String>,
    pub(super) confirmation: WireScriptExecutionConfirmation,
    pub(super) execution_allowed: bool,
    pub(super) disabled_reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireScriptExecutionCwdScope {
    pub(super) requested: Option<String>,
    pub(super) effective: String,
    pub(super) source: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireScriptExecutionEnvScope {
    pub(super) inherit_parent: bool,
    pub(super) provided_keys: Vec<String>,
    pub(super) redacted_keys: Vec<String>,
    pub(super) value_policy: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireScriptExecutionNetworkScope {
    pub(super) requested: String,
    pub(super) allowed: bool,
    pub(super) reason: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireScriptExecutionFilesScope {
    pub(super) requested: Vec<String>,
    pub(super) read_allowed: bool,
    pub(super) write_allowed: bool,
    pub(super) allowed_roots: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireScriptExecutionCommandPreview {
    pub(super) argv: Vec<String>,
    pub(super) display: String,
    pub(super) shell: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireScriptExecutionConfirmation {
    pub(super) required: bool,
    pub(super) confirmed: bool,
    pub(super) fields: Vec<String>,
    pub(super) message: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireScriptExecutionAttemptRecord {
    pub(super) id: String,
    pub(super) created_at: i64,
    pub(super) status: String,
    pub(super) outcome: String,
    pub(super) reason: String,
    pub(super) spawned_process: bool,
    pub(super) audit_path: String,
    pub(super) preview: WireScriptExecutionPreviewRecord,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireProjectContextSummary {
    pub(super) source: String,
    pub(super) revision: String,
    pub(super) active: Option<WireProjectContext>,
    pub(super) recent_count: usize,
    pub(super) validation_error: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireProjectContextState {
    pub(super) revision: String,
    pub(super) active: Option<WireProjectContext>,
    pub(super) recent: Vec<WireProjectContext>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireProjectContext {
    pub(super) id: String,
    pub(super) name: String,
    pub(super) root_path: String,
    pub(super) current_cwd: String,
    pub(super) last_used_at: i64,
    pub(super) is_active: bool,
    pub(super) validation_error: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireRefreshActivity {
    pub(super) operation: String,
    pub(super) status: String,
    pub(super) started_at: i64,
    pub(super) finished_at: i64,
    pub(super) scanned_count: usize,
    pub(super) skill_count: usize,
    pub(super) finding_count: usize,
    pub(super) conflict_count: usize,
    pub(super) snapshot_count: usize,
    pub(super) roots: Vec<String>,
    pub(super) log_entries: Vec<WireRefreshLogEntry>,
    pub(super) recovery_actions: Vec<String>,
    pub(super) agent_summaries: Option<Vec<WireAgentRefreshSummary>>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireRefreshLogEntry {
    pub(super) level: String,
    pub(super) message: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAgentRefreshSummary {
    pub(super) agent: String,
    pub(super) display_label: String,
    pub(super) status: String,
    pub(super) scanned_count: usize,
    pub(super) catalog_count: usize,
    pub(super) broken_count: usize,
    pub(super) roots_considered: Vec<String>,
    pub(super) roots_scanned: Vec<String>,
    pub(super) roots_partial: Vec<String>,
    pub(super) roots_skipped: Vec<String>,
    pub(super) scan_issues: Vec<WireAgentRefreshScanIssue>,
    #[serde(default)]
    pub(super) config_detected: bool,
    #[serde(default)]
    pub(super) config_paths: Vec<String>,
    #[serde(default)]
    pub(super) writable_status: String,
    #[serde(default)]
    pub(super) writable_reason: Option<String>,
    #[serde(default)]
    pub(super) read_only_reason: String,
    #[serde(default)]
    pub(super) blockers: Vec<String>,
    pub(super) recovery_actions: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireAgentRefreshScanIssue {
    pub(super) kind: String,
    pub(super) path: String,
    pub(super) detail: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireSkillRecord {
    pub(super) id: String,
    pub(super) agent: String,
    pub(super) scope: String,
    pub(super) path: PathBuf,
    pub(super) display_path: PathBuf,
    pub(super) definition_id: String,
    pub(super) name: String,
    pub(super) state: String,
    pub(super) enabled: bool,
    pub(super) publisher: Option<String>,
    pub(super) package_name: Option<String>,
    pub(super) package_version: Option<String>,
    pub(super) source_kind: Option<String>,
    pub(super) read_only_reason: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireActionTarget {
    pub(super) kind: String,
    pub(super) id: String,
    pub(super) agent: Option<String>,
    pub(super) scope: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireActionDescriptor {
    pub(super) id: String,
    pub(super) kind: String,
    pub(super) intent: String,
    pub(super) target: WireActionTarget,
    pub(super) project_id: Option<String>,
    pub(super) impacts: Vec<String>,
    pub(super) preview_method: String,
    pub(super) apply_method: Option<String>,
    pub(super) source_revision: String,
    pub(super) confirmation_required: bool,
    pub(super) network: String,
    pub(super) readback: Vec<String>,
    pub(super) evidence_refs: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireActionPrecondition {
    pub(super) kind: String,
    pub(super) target_id: String,
    pub(super) expected_revision: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireActionReadbackObservation {
    pub(super) domain: String,
    pub(super) target_id: String,
    pub(super) revision: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireActionReadbackRecord {
    pub(super) action_id: String,
    pub(super) source_revision: String,
    pub(super) project_id: Option<String>,
    pub(super) domains: Vec<String>,
    pub(super) target_ids: Vec<String>,
    pub(super) observations: Vec<WireActionReadbackObservation>,
    pub(super) verified: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireBatchTogglePreviewRecord {
    pub(super) action: WireActionDescriptor,
    pub(super) preconditions: Vec<WireActionPrecondition>,
    pub(super) preview_token: String,
    pub(super) target_enabled: bool,
    pub(super) requested_count: usize,
    pub(super) writable_count: usize,
    pub(super) skipped_count: usize,
    pub(super) writes_allowed: bool,
    pub(super) affected_items: Vec<WireBatchToggleAffectedItem>,
    pub(super) skipped_items: Vec<WireBatchToggleSkippedItem>,
    pub(super) capability_labels: Vec<String>,
    pub(super) snapshot_rollback_notes: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireBatchToggleApplyRecord {
    pub(super) action: WireActionDescriptor,
    pub(super) preview_token: String,
    pub(super) target_enabled: bool,
    pub(super) requested_count: usize,
    pub(super) writable_count: usize,
    pub(super) skipped_count: usize,
    pub(super) applied_count: usize,
    pub(super) writes_allowed: bool,
    pub(super) affected_items: Vec<WireBatchToggleAffectedItem>,
    pub(super) skipped_items: Vec<WireBatchToggleSkippedItem>,
    pub(super) capability_labels: Vec<String>,
    pub(super) snapshot_rollback_notes: Vec<String>,
    pub(super) updated_records: Vec<WireSkillRecord>,
    pub(super) readback: WireActionReadbackRecord,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireBatchToggleAffectedItem {
    pub(super) instance_id: String,
    pub(super) name: String,
    pub(super) agent: String,
    pub(super) scope: String,
    pub(super) current_enabled: bool,
    pub(super) target_enabled: bool,
    pub(super) config_scope: String,
    pub(super) config_target: String,
    pub(super) config_revision: String,
    pub(super) catalog_revision: String,
    pub(super) capability_label: String,
    pub(super) snapshot_plan: String,
    pub(super) rollback_plan: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireBatchToggleSkippedItem {
    pub(super) instance_id: String,
    pub(super) name: Option<String>,
    pub(super) agent: Option<String>,
    pub(super) scope: Option<String>,
    pub(super) reason: String,
    pub(super) capability_label: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireSkillDetailRecord {
    pub(super) id: String,
    pub(super) agent: String,
    pub(super) scope: String,
    pub(super) path: PathBuf,
    pub(super) display_path: PathBuf,
    pub(super) definition_id: String,
    pub(super) name: String,
    pub(super) description: String,
    pub(super) state: String,
    pub(super) enabled: bool,
    pub(super) frontmatter_raw: String,
    pub(super) body: String,
    pub(super) permissions: Value,
    pub(super) fingerprint: String,
    pub(super) publisher: Option<String>,
    pub(super) package_name: Option<String>,
    pub(super) package_version: Option<String>,
    pub(super) source_kind: Option<String>,
    pub(super) read_only_reason: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireExportedSkillBundle {
    pub(super) manifest_path: PathBuf,
    pub(super) bundle_path: PathBuf,
    pub(super) fingerprint: String,
    pub(super) metadata: WireExportedSkillMetadata,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireExportedSkillMetadata {
    pub(super) name: String,
    pub(super) description: String,
    pub(super) skill_path: String,
    pub(super) source_agent: String,
    pub(super) source_scope: String,
    pub(super) version: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireRuleFindingRecord {
    pub(super) id: String,
    pub(super) triage_key: String,
    pub(super) triage_context: String,
    pub(super) instance_id: Option<String>,
    pub(super) definition_id: Option<String>,
    pub(super) rule_id: String,
    pub(super) severity: String,
    pub(super) effective_severity: String,
    pub(super) severity_override: Option<String>,
    pub(super) message: String,
    pub(super) suggestion: Option<String>,
    pub(super) created_at: i64,
    pub(super) suppressed: bool,
    pub(super) suppression_reason: Option<String>,
    pub(super) suppression_note: Option<String>,
    pub(super) rule_tuning_updated_at: Option<i64>,
    pub(super) triage_status: String,
    pub(super) triage_note: Option<String>,
    pub(super) triage_updated_at: Option<i64>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireFindingTriageRecord {
    pub(super) triage_key: String,
    pub(super) triage_context: String,
    pub(super) status: String,
    pub(super) note: Option<String>,
    pub(super) updated_at: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireRuleTuningRecord {
    pub(super) rule_id: String,
    pub(super) agent: Option<String>,
    pub(super) scope: Option<String>,
    pub(super) severity_override: Option<String>,
    pub(super) suppression_reason: Option<String>,
    pub(super) suppression_note: Option<String>,
    pub(super) updated_at: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireConflictGroupRecord {
    pub(super) id: String,
    pub(super) definition_id: String,
    pub(super) reason: String,
    pub(super) winner_id: Option<String>,
    pub(super) instance_ids: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireToolGlobalImportResult {
    pub(super) imported: WireSkillRecord,
    pub(super) instance_id: String,
    pub(super) source_path: String,
    pub(super) staging_path: String,
    pub(super) findings: Vec<WireRuleFindingRecord>,
    pub(super) audit: WireToolGlobalImportAudit,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireToolGlobalImportAudit {
    pub(super) status: String,
    pub(super) read_only_preview: bool,
    pub(super) finding_count: usize,
    pub(super) error_count: usize,
    pub(super) warn_count: usize,
    pub(super) info_count: usize,
    pub(super) conflict_count: usize,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireConfigDocumentRecord {
    pub(super) agent: String,
    pub(super) scope: String,
    pub(super) target: String,
    pub(super) format: String,
    pub(super) content: String,
    pub(super) exists: bool,
    pub(super) revision: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireConfigSavePreviewRecord {
    pub(super) action: WireActionDescriptor,
    pub(super) preconditions: Vec<WireActionPrecondition>,
    pub(super) preview_token: String,
    pub(super) current: WireConfigDocumentRecord,
    pub(super) candidate_content_digest: String,
    pub(super) current_revision: String,
    pub(super) changed: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireConfigSaveApplyRecord {
    pub(super) action: WireActionDescriptor,
    pub(super) document: WireConfigDocumentRecord,
    pub(super) snapshot_id: String,
    pub(super) readback: WireActionReadbackRecord,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireSkillInstallPreviewRecord {
    pub(super) action: WireActionDescriptor,
    pub(super) preconditions: Vec<WireActionPrecondition>,
    pub(super) preview_token: String,
    pub(super) source_instance_id: String,
    pub(super) source_path: String,
    pub(super) target_agent: String,
    pub(super) target_scope: String,
    pub(super) target_path: String,
    pub(super) files: Vec<WireSkillInstallFilePreview>,
    pub(super) risks: Vec<String>,
    pub(super) confirmation: WireSkillInstallConfirmation,
    pub(super) wrote: bool,
    pub(super) snapshot_id: Option<String>,
    pub(super) readback: Option<WireActionReadbackRecord>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireSkillInstallFilePreview {
    pub(super) source: String,
    pub(super) target: String,
    pub(super) kind: String,
    pub(super) will_write: bool,
    pub(super) target_exists: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireSkillInstallConfirmation {
    pub(super) required: bool,
    pub(super) confirmed: bool,
    pub(super) fields: Vec<String>,
    pub(super) message: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireConfigSnapshotRecord {
    pub(super) id: String,
    pub(super) agent: String,
    pub(super) scope: String,
    pub(super) project_root: Option<String>,
    pub(super) target: String,
    pub(super) content: String,
    pub(super) reason: String,
    pub(super) created_at: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireSkillEventRecord {
    pub(super) id: i64,
    pub(super) instance_id: String,
    pub(super) kind: String,
    pub(super) payload: Value,
    pub(super) occurred_at: i64,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireConfigSnapshotPageResult {
    pub(super) records: Vec<WireConfigSnapshotRecord>,
    pub(super) source_revision: String,
    pub(super) returned_count: usize,
    pub(super) total_count: Option<usize>,
    pub(super) has_more: bool,
    pub(super) next_cursor: Option<String>,
    pub(super) source_completeness: String,
    pub(super) incomplete_reason: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireSkillEventPageResult {
    pub(super) records: Vec<WireSkillEventRecord>,
    pub(super) source_revision: String,
    pub(super) returned_count: usize,
    pub(super) total_count: Option<usize>,
    pub(super) has_more: bool,
    pub(super) next_cursor: Option<String>,
    pub(super) source_completeness: String,
    pub(super) incomplete_reason: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireSnapshotRollbackPreviewRecord {
    pub(super) action: WireActionDescriptor,
    pub(super) preconditions: Vec<WireActionPrecondition>,
    pub(super) preview_token: String,
    pub(super) snapshot: WireConfigSnapshotRecord,
    pub(super) snapshot_content_digest: String,
    pub(super) current_content: String,
    pub(super) current_read_error: Option<String>,
    pub(super) current_revision: String,
    pub(super) changed: bool,
    pub(super) redacted: bool,
    pub(super) rollback_supported: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
pub(super) struct WireSnapshotRollbackApplyRecord {
    pub(super) action: WireActionDescriptor,
    pub(super) snapshot_id: String,
    pub(super) document: WireConfigDocumentRecord,
    pub(super) readback: WireActionReadbackRecord,
}
