use super::app_wire_fixtures::*;
use super::dispatch_fixtures::*;
use super::skill_manager_fixtures::assert_skill_manager_page_metadata;
use super::*;
use crate::privacy_cleanup::LegacyPrivateContentCleanupParams;

#[test]
pub(super) fn service_protocol_fixtures_decode() {
    let fixtures_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/service-protocol");
    let mut request_methods = Vec::new();
    let mut response_methods = Vec::new();
    for entry in fs::read_dir(fixtures_dir).expect("read fixtures") {
        let path = entry.expect("fixture entry").path();
        let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        if name.ends_with(".request.json") {
            let content = fs::read_to_string(&path).expect("read request fixture");
            let request =
                serde_json::from_str::<ServiceRequest>(&content).unwrap_or_else(|error| {
                    panic!("request fixture {} failed: {error}", path.display())
                });
            if request.method == "session.previewLocalSessions" {
                let params =
                    serde_json::from_value::<WireLocalSessionPreviewParams>(request.params.clone())
                        .unwrap_or_else(|error| {
                            panic!("request fixture {} params failed: {error}", path.display())
                        });
                assert_eq!(params.paging_mode.as_deref(), Some("keyset"));
            }
            if request.method == "session.listLocalSessionMessages" {
                let params = serde_json::from_value::<WireLocalSessionMessagePageParams>(
                    request.params.clone(),
                )
                .unwrap_or_else(|error| {
                    panic!("request fixture {} params failed: {error}", path.display())
                });
                assert!(!params.session_id.is_empty());
            }
            match request.method.as_str() {
                "catalog.scanClaude" | "catalog.scanAll" => {
                    let params =
                        serde_json::from_value::<CatalogScanParams>(request.params.clone())
                            .unwrap_or_else(|error| {
                                panic!("request fixture {} params failed: {error}", path.display())
                            });
                    assert!(
                        params.explicit_refresh,
                        "catalog scan fixtures must prove explicit refresh authorization"
                    );
                    assert!(params.expected_context_revision.is_some());
                }
                "batch.applySkillToggles" => {
                    let params = serde_json::from_value::<BatchApplySkillTogglesParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(params.confirmation.confirmed);
                    assert!(!params.confirmation.preview_token.is_empty());
                }
                "skillManager.applyInstall" => {
                    let params =
                        serde_json::from_value::<SkillManagerInstallParams>(request.params.clone())
                            .unwrap_or_else(|error| {
                                panic!("request fixture {} params failed: {error}", path.display())
                            });
                    assert!(params.confirmed);
                    assert!(params.action_reference.is_some());
                }
                "skillManager.applyRemove" => {
                    let params =
                        serde_json::from_value::<SkillManagerRemoveParams>(request.params.clone())
                            .unwrap_or_else(|error| {
                                panic!("request fixture {} params failed: {error}", path.display())
                            });
                    assert!(params.confirmed);
                    assert!(params.action_reference.is_some());
                }
                "skillManager.applyUpdate" => {
                    let params =
                        serde_json::from_value::<SkillManagerUpdateParams>(request.params.clone())
                            .unwrap_or_else(|error| {
                                panic!("request fixture {} params failed: {error}", path.display())
                            });
                    assert!(params.confirmed);
                    assert!(params.action_reference.is_some());
                }
                "skillManager.applyLocalCreate" => {
                    let params = serde_json::from_value::<SkillManagerLocalCreateParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(params.confirmed);
                    assert!(params.action_reference.is_some());
                }
                "skillManager.applyLocalArchiveImport" => {
                    let params = serde_json::from_value::<SkillManagerLocalArchiveImportParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(params.confirmed);
                    assert!(params.action_reference.is_some());
                }
                "skillManager.applyLocalArchiveUpdate" => {
                    let params = serde_json::from_value::<SkillManagerLocalArchiveUpdateParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(params.confirmed);
                    assert!(params.action_reference.is_some());
                }
                "llm.previewSaveProviderProfile" => {
                    let params =
                        serde_json::from_value::<SaveProviderProfileParams>(request.params.clone())
                            .unwrap_or_else(|error| {
                                panic!("request fixture {} params failed: {error}", path.display())
                            });
                    assert!(params.action_confirmation.is_none());
                }
                "llm.saveProviderProfile" => {
                    let params =
                        serde_json::from_value::<SaveProviderProfileParams>(request.params.clone())
                            .unwrap_or_else(|error| {
                                panic!("request fixture {} params failed: {error}", path.display())
                            });
                    assert!(params
                        .action_confirmation
                        .as_ref()
                        .is_some_and(|confirmation| confirmation.confirmed));
                }
                "llm.previewDeleteProviderProfile" => {
                    let params = serde_json::from_value::<DeleteProviderProfileParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(params.action_confirmation.is_none());
                }
                "llm.deleteProviderProfile" => {
                    let params = serde_json::from_value::<DeleteProviderProfileParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(params
                        .action_confirmation
                        .as_ref()
                        .is_some_and(|confirmation| confirmation.confirmed));
                }
                "llm.previewProviderConnectionTest" => {
                    let params = serde_json::from_value::<TestProviderConnectionParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(params.action_confirmation.is_none());
                }
                "llm.testProviderConnection" => {
                    let params = serde_json::from_value::<TestProviderConnectionParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(params
                        .action_confirmation
                        .as_ref()
                        .is_some_and(|confirmation| confirmation.confirmed));
                }
                "llm.confirmPromptAndSend" => {
                    let params = serde_json::from_value::<LlmConfirmPromptAndSendParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(params.action_confirmation.confirmed);
                }
                "privacy.cleanupLegacyContent" => {
                    let params = serde_json::from_value::<LegacyPrivateContentCleanupParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(params.action_confirmation.confirmed);
                }
                "project.previewSetContext" => {
                    let params = serde_json::from_value::<ProjectContextSetPreviewParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(!params.expected_revision.is_empty());
                }
                "project.setContext" => {
                    let params = serde_json::from_value::<ProjectContextSetApplyParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(params.candidate_last_used_at > 0);
                    assert!(params.action_confirmation.confirmed);
                }
                "project.previewClearContext" | "project.previewClearRecentContexts" => {
                    let params = serde_json::from_value::<ProjectContextRevisionParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(!params.expected_revision.is_empty());
                }
                "project.clearContext" | "project.clearRecentContexts" => {
                    let params = serde_json::from_value::<ProjectContextConfirmationParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(params.action_confirmation.confirmed);
                }
                "project.previewRemoveRecentContext" => {
                    let params = serde_json::from_value::<ProjectContextIDPreviewParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(!params.id.is_empty());
                    assert!(!params.expected_revision.is_empty());
                }
                "project.removeRecentContext" => {
                    let params = serde_json::from_value::<ProjectContextIDApplyParams>(
                        request.params.clone(),
                    )
                    .unwrap_or_else(|error| {
                        panic!("request fixture {} params failed: {error}", path.display())
                    });
                    assert!(!params.id.is_empty());
                    assert!(params.action_confirmation.confirmed);
                }
                _ => {}
            }
            request_methods.push(request.method);
        }
        if name.ends_with(".response.json") {
            let content = fs::read_to_string(&path).expect("read response fixture");
            let response =
                serde_json::from_str::<ServiceResponse>(&content).unwrap_or_else(|error| {
                    panic!("response fixture {} failed: {error}", path.display())
                });
            let method = fixture_method_from_name(name, ".response.json");
            if name.contains(".error.response.json") {
                assert!(
                    !response.ok,
                    "error response fixture {} is ok",
                    path.display()
                );
                assert!(
                    response.error.is_some(),
                    "error response fixture {} missing error",
                    path.display()
                );
            } else {
                assert!(response.ok, "response fixture {} is not ok", path.display());
                let result = response.result.unwrap_or_else(|| {
                    panic!("response fixture {} missing result", path.display())
                });
                decode_response_fixture(method, &result, &path);
            }
            response_methods.push(method.to_string());
        }
    }
    for (request_fixture, response_fixture) in inline_service_protocol_fixtures() {
        let request = serde_json::from_value::<ServiceRequest>(request_fixture)
            .expect("inline request fixture should decode");
        let method = request.method.clone();
        request_methods.push(method.clone());
        let response = serde_json::from_value::<ServiceResponse>(response_fixture)
            .expect("inline response fixture should decode");
        assert!(
            response.ok,
            "inline response fixture for {method} is not ok"
        );
        let result = response
            .result
            .unwrap_or_else(|| panic!("inline response fixture for {method} missing result"));
        let path = PathBuf::from(format!("<inline:{method}.response.json>"));
        decode_response_fixture(&method, &result, &path);
        response_methods.push(method);
    }

    let supported = supported_methods();
    for method in &supported {
        assert!(
            request_methods.iter().any(|fixture| fixture == method),
            "missing request fixture for {method}"
        );
        assert!(
            response_methods.iter().any(|fixture| fixture == method),
            "missing response fixture for {method}"
        );
    }
    for method in request_methods.iter().chain(response_methods.iter()) {
        assert!(
            supported.iter().any(|supported| supported == method),
            "fixture covers unsupported method {method}"
        );
    }
}

pub(super) fn inline_service_protocol_fixtures() -> Vec<(Value, Value)> {
    Vec::new()
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLocalSessionPreviewParams {
    authorized_roots: Vec<String>,
    auto_discover: Option<bool>,
    agent: Option<String>,
    scope: Option<String>,
    search: Option<String>,
    project_root: Option<String>,
    current_cwd: Option<String>,
    session_id: Option<String>,
    include_content_items: Option<bool>,
    limit: Option<usize>,
    offset: Option<usize>,
    paging_mode: Option<String>,
    cursor: Option<String>,
    source_revision: Option<String>,
    sort: Option<String>,
    direction: Option<String>,
    max_files: Option<usize>,
    max_excerpt_chars: Option<usize>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLocalSessionMessagePageParams {
    authorized_roots: Vec<String>,
    auto_discover: Option<bool>,
    agent: Option<String>,
    project_root: Option<String>,
    current_cwd: Option<String>,
    session_id: String,
    limit: Option<usize>,
    cursor: Option<String>,
    source_revision: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLegacyPrivateContentSource {
    id: String,
    source_file: String,
    item_type: String,
    state: String,
    cleanup_operation: String,
    cleanup_required: bool,
    malformed: bool,
    generated_residue: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLegacyPrivateContentInspection {
    generated_by: String,
    cleanup_required: bool,
    cleanup_source_count: usize,
    existing_source_count: usize,
    sources: Vec<WireLegacyPrivateContentSource>,
    read_only: bool,
    provider_request_sent: bool,
    raw_content_returned: bool,
    write_performed: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLegacyPrivateContentCleanupPreview {
    inspection: WireLegacyPrivateContentInspection,
    action: Option<WireActionDescriptor>,
    preconditions: Vec<WireActionPrecondition>,
    preview_token: Option<String>,
    confirmation_required: bool,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireLegacyPrivateContentCleanupResult {
    inspection: WireLegacyPrivateContentInspection,
    cleaned_source_count: usize,
    state: String,
    effect: String,
    retry_allowed: bool,
    readback: WireActionReadbackRecord,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerToolRecord {
    id: String,
    display_name: String,
    status: String,
    executable: Option<String>,
    operations: Vec<String>,
    default_agents: Vec<String>,
    notes: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerCommandPreview {
    #[serde(default)]
    action: Option<WireActionDescriptor>,
    #[serde(default)]
    preconditions: Vec<WireActionPrecondition>,
    tool_id: String,
    operation: String,
    command: Vec<String>,
    cwd: String,
    env: Vec<WireSkillManagerEnvPreview>,
    requires_confirmation: bool,
    confirmed: bool,
    network_required: bool,
    network_allowed: bool,
    will_run: bool,
    preview_token: String,
    summary: String,
    risks: Vec<String>,
    #[serde(default)]
    source: Option<String>,
    #[serde(default)]
    skills: Vec<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerEnvPreview {
    key: String,
    value: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerCommandOutput {
    status: String,
    exit_code: Option<i32>,
    stdout: String,
    stderr: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerSearchRecord {
    preview: WireSkillManagerCommandPreview,
    output: Option<WireSkillManagerCommandOutput>,
    results: Vec<WireSkillManagerSearchResult>,
    #[serde(default)]
    readback: Option<WireActionReadbackRecord>,
    returned_count: usize,
    total_count: Option<usize>,
    has_more: bool,
    #[serde(default)]
    next_cursor: Option<String>,
    source_completeness: String,
    #[serde(default)]
    incomplete_reason: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerSearchResult {
    name: String,
    source: Option<String>,
    description: Option<String>,
    raw: Value,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerInstalledListRecord {
    preview: WireSkillManagerCommandPreview,
    output: WireSkillManagerCommandOutput,
    installed: Vec<WireSkillManagerInstalledRecord>,
    source_revision: String,
    returned_count: usize,
    total_count: Option<usize>,
    has_more: bool,
    #[serde(default)]
    next_cursor: Option<String>,
    source_completeness: String,
    #[serde(default)]
    incomplete_reason: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerInstalledRecord {
    name: String,
    source: Option<String>,
    source_kind: String,
    agents: Vec<String>,
    scope: Option<String>,
    path: Option<String>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerMutationRecord {
    preview: WireSkillManagerCommandPreview,
    output: Option<WireSkillManagerCommandOutput>,
    applied: bool,
    scanned_count: usize,
    updated_skills: Vec<WireSkillRecord>,
    readback: Option<WireActionReadbackRecord>,
    #[serde(default)]
    follow_up: Option<WireSkillManagerCleanupFollowUp>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerLocalCreateRecord {
    preview: WireSkillManagerCommandPreview,
    output: Option<WireSkillManagerCommandOutput>,
    imported: Option<WireSkillRecord>,
    instance_id: Option<String>,
    source_path: String,
    applied: bool,
    readback: Option<WireActionReadbackRecord>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerLocalDeleteRecord {
    #[serde(default)]
    action: Option<WireActionDescriptor>,
    #[serde(default)]
    preconditions: Vec<WireActionPrecondition>,
    #[serde(default)]
    preview_token: Option<String>,
    instance_id: String,
    skill_name: String,
    path: String,
    app_owned: bool,
    physical_delete_allowed: bool,
    blocked_by_references: Vec<WireSkillManagerReferenceRecord>,
    confirmed: bool,
    deleted: bool,
    summary: String,
    #[serde(default)]
    readback: Option<WireActionReadbackRecord>,
    #[serde(default)]
    follow_up: Option<WireSkillManagerCleanupFollowUp>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerCleanupFollowUp {
    kind: String,
    state: String,
    cleanup_required: bool,
    message: String,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerLocalArchiveUpdateRecord {
    action: WireActionDescriptor,
    preconditions: Vec<WireActionPrecondition>,
    instance_id: String,
    skill_name: String,
    archive_path: String,
    archive_sha256: String,
    file_count: usize,
    uncompressed_bytes: u64,
    preview_token: String,
    confirmed: bool,
    applied: bool,
    summary: String,
    #[serde(default)]
    updated_skill: Option<WireSkillRecord>,
    #[serde(default)]
    readback: Option<WireActionReadbackRecord>,
    #[serde(default)]
    follow_up: Option<WireSkillManagerCleanupFollowUp>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerLocalArchiveImportRecord {
    action: WireActionDescriptor,
    preconditions: Vec<WireActionPrecondition>,
    skill_name: String,
    archive_path: String,
    archive_sha256: String,
    file_count: usize,
    uncompressed_bytes: u64,
    preview_token: String,
    confirmed: bool,
    applied: bool,
    summary: String,
    #[serde(default)]
    imported_skill: Option<WireSkillRecord>,
    #[serde(default)]
    instance_id: Option<String>,
    #[serde(default)]
    readback: Option<WireActionReadbackRecord>,
    #[serde(default)]
    follow_up: Option<WireSkillManagerCleanupFollowUp>,
}

#[allow(dead_code)]
#[derive(Debug, Deserialize)]
#[serde(deny_unknown_fields)]
struct WireSkillManagerReferenceRecord {
    instance_id: String,
    name: String,
    agent: String,
    scope: String,
    path: String,
}

pub(super) fn decode_response_fixture(method: &str, result: &Value, path: &Path) {
    match method {
        "app.version" => {
            let version: WireAppVersion = decode_fixture_result(method, result, path);
            assert_eq!(version.protocol_version, SERVICE_PROTOCOL_VERSION);
            assert!(!version.version.is_empty());
        }
        "app.stateSnapshot" => {
            let snapshot: WireAppStateSnapshot = decode_fixture_result(method, result, path);
            assert_supported_methods(method, &snapshot.status.supported_methods);
            assert_eq!(
                snapshot.analysis.summary.total_groups,
                snapshot.analysis.groups.len()
            );
            assert_findings_cover_v28_contract(
                &snapshot.findings,
                &["frontmatter.required-fields"],
                method,
            );
        }
        "app.search" => {
            let search: WireAppSearchResult = decode_fixture_result(method, result, path);
            assert!(search.read_only);
            assert!(!search.provider_request_sent);
            assert_eq!(search.count, search.items.len());
        }
        "service.status" => {
            let status: WireServiceStatus = decode_fixture_result(method, result, path);
            assert_eq!(status.protocol_version, SERVICE_PROTOCOL_VERSION);
            assert_supported_methods(method, &status.supported_methods);
            assert!(!status.script_execution.enabled);
            assert!(!status.script_execution.llm_initiation_allowed);
        }
        "adapter.listCapabilities" => {
            let _: Vec<WireAdapterCapabilityRecord> = decode_fixture_result(method, result, path);
        }
        "adapter.listDiagnostics" => {
            let diagnostics: Vec<WireAdapterDiagnosticsRecord> =
                decode_fixture_result(method, result, path);
            assert!(diagnostics.iter().any(|diagnostic| {
                diagnostic.agent == "hermes"
                    && diagnostic.status == "guarded"
                    && diagnostic.config.status == "not-detected"
                    && diagnostic.access.writable_status == "guarded-v2.97"
            }));
        }
        "session.previewLocalSessions" => {
            let preview: WireLocalSessionPreviewResult =
                decode_fixture_result(method, result, path);
            assert_eq!(preview.generated_by, "local-v2.98");
            assert!(preview.read_only);
            assert!(!preview.provider_request_sent);
            assert!(!preview.skill_files_mutated);
            assert!(!preview.agent_config_mutated);
            assert!(!preview.snapshot_created);
            assert!(!preview.triage_mutated);
            assert!(!preview.raw_prompt_persisted);
            assert!(!preview.raw_response_persisted);
            assert!(!preview.raw_trace_persisted);
            assert_eq!(preview.count, preview.session_rows.len());
            assert!(!preview.candidate_set_truncated);
            assert_eq!(preview.source_completeness, "enumerable");
            assert!(preview
                .source_revision
                .as_deref()
                .is_some_and(|value| value.starts_with("sha256:")));
            assert!(preview.next_cursor.is_none());
            assert!(preview.incomplete_reason.is_none());
            assert_eq!(
                preview.user_message_count,
                preview
                    .session_rows
                    .iter()
                    .map(|row| row.user_message_count)
                    .sum::<usize>()
            );
            assert_eq!(
                preview.total_message_count,
                preview
                    .session_rows
                    .iter()
                    .map(|row| row.total_message_count)
                    .sum::<usize>()
            );
            assert_eq!(
                preview.tool_call_count,
                preview
                    .session_rows
                    .iter()
                    .map(|row| row.tool_call_count)
                    .sum::<usize>()
            );
            assert_eq!(
                preview.skill_call_count,
                preview
                    .session_rows
                    .iter()
                    .map(|row| row.skill_call_count)
                    .sum::<usize>()
            );
            assert_local_preview_safety(&preview.safety_flags);
            assert!(!preview.redaction_summary.raw_trace_persisted);
            for row in &preview.skill_usage_rows {
                assert!(row.call_count >= row.session_count);
                assert!(!row.skill_name.is_empty());
                assert!(!row.evidence_refs.is_empty());
            }
            for row in &preview.session_rows {
                assert_eq!(row.source_kind, "authorized-local-session");
                assert!(!row.excerpt.is_empty());
                assert!(!row.evidence_refs.is_empty());
                assert!(!row.content_included);
                assert!(row.content_items.is_empty());
            }
        }
        "session.listLocalSessionMessages" => {
            let page: WireLocalSessionMessagePageResult =
                decode_fixture_result(method, result, path);
            assert_eq!(page.generated_by, "local-v2.99");
            assert_eq!(page.returned_count, page.content_items.len());
            assert_eq!(page.total_count, Some(page.returned_count));
            assert!(!page.has_more);
            assert!(page.next_cursor.is_none());
            assert!(page.source_revision.starts_with("sha256:"));
            assert_eq!(page.source_completeness, "enumerable");
            assert!(page.incomplete_reason.is_none());
            assert_eq!(page.scanned_through_bytes, page.snapshot_bytes);
            assert!(page.scanned_bytes <= page.snapshot_bytes);
            assert!(page.read_only);
            assert!(!page.provider_request_sent);
            assert!(!page.raw_prompt_persisted);
            assert!(!page.raw_response_persisted);
            assert!(!page.raw_trace_persisted);
            assert_local_preview_safety(&page.safety_flags);
            assert!(!page.redaction_summary.raw_trace_persisted);
            assert!(page
                .content_items
                .iter()
                .all(|item| { matches!(item.kind.as_str(), "user_message" | "agent_reply") }));
        }
        "llm.status" => {
            let status: WireLlmStatus = decode_fixture_result(method, result, path);
            assert!(!status.enabled);
            assert!(!status.configured);
            assert!(!status.credential_persistence_allowed);
        }
        "llm.listProviderProfiles" => {
            let profiles: WireListProviderProfilesResult =
                decode_fixture_result(method, result, path);
            assert!(!profiles.raw_secrets_returned);
        }
        "llm.previewSaveProviderProfile"
        | "llm.previewDeleteProviderProfile"
        | "llm.previewProviderConnectionTest" => {
            let preview: WireProviderActionPreviewResult =
                decode_fixture_result(method, result, path);
            assert_eq!(preview.action.preview_method, method);
            assert_eq!(
                preview.action.apply_method.as_deref(),
                Some(match method {
                    "llm.previewSaveProviderProfile" => "llm.saveProviderProfile",
                    "llm.previewDeleteProviderProfile" => "llm.deleteProviderProfile",
                    "llm.previewProviderConnectionTest" => "llm.testProviderConnection",
                    _ => unreachable!(),
                })
            );
            assert!(preview.action.confirmation_required);
            assert_eq!(preview.action.target.kind, "provider_profile");
            assert_eq!(preview.action.target.id, preview.profile_id);
            assert!(preview
                .preconditions
                .iter()
                .any(|precondition| precondition.kind == "provider_profile"));
            assert!(preview
                .preconditions
                .iter()
                .any(|precondition| precondition.kind == "prompt_context"));
            assert!(preview
                .preview_token
                .starts_with("action-preview:v1:hmac-sha256:"));
            assert!(!preview.raw_secret_returned);
        }
        "llm.saveProviderProfile" => {
            let saved: WireSaveProviderProfileResult = decode_fixture_result(method, result, path);
            assert!(!saved.raw_secret_returned);
            assert_eq!(saved.outcome.state, "verified");
            assert!(saved
                .readback
                .as_ref()
                .is_some_and(|readback| readback.verified));
            assert!(saved
                .readback
                .as_ref()
                .expect("verified save readback")
                .domains
                .iter()
                .any(|domain| domain == "provider_profiles"));
        }
        "llm.deleteProviderProfile" => {
            let deleted: WireDeleteProviderProfileResult =
                decode_fixture_result(method, result, path);
            assert!(!deleted.raw_secret_returned);
            assert_eq!(deleted.outcome.state, "verified");
            assert!(deleted
                .readback
                .as_ref()
                .is_some_and(|readback| readback.verified));
            assert!(deleted
                .readback
                .as_ref()
                .expect("verified delete readback")
                .domains
                .iter()
                .any(|domain| domain == "provider_profiles"));
        }
        "llm.testProviderConnection" => {
            let tested: WireTestProviderConnectionResult =
                decode_fixture_result(method, result, path);
            assert!(tested.local_metadata_persisted);
            assert!(!tested.raw_prompt_persisted);
            assert!(!tested.raw_response_persisted);
            assert!(!tested.raw_secret_returned);
            assert_eq!(tested.outcome.state, "verified");
            assert!(tested
                .readback
                .as_ref()
                .is_some_and(|readback| readback.verified));
            assert!(tested
                .readback
                .as_ref()
                .expect("verified test readback")
                .domains
                .iter()
                .any(|domain| domain == "provider_activity"));
        }
        "llm.previewPrompt" => {
            let preview: WireLlmPreviewPromptResult = decode_fixture_result(method, result, path);
            assert_eq!(preview.action.kind, "provider_prompt");
            assert_eq!(preview.action.intent, "send_provider_prompt");
            assert_eq!(preview.action.preview_method, method);
            assert_eq!(
                preview.action.apply_method.as_deref(),
                Some("llm.confirmPromptAndSend")
            );
            assert_eq!(preview.action.network, "required");
            assert!(preview
                .preconditions
                .iter()
                .any(|precondition| precondition.target_id == "redacted-prompt"));
            assert!(preview
                .preview_token
                .starts_with("action-preview:v1:hmac-sha256:"));
            assert!(preview.requires_confirmation);
            assert!(!preview.provider_request_sent);
            assert!(!preview.write_back_allowed);
            assert!(preview.draft_requires_user_copy);
            assert!(!preview.raw_secret_returned);
            assert!(!preview.raw_prompt_persisted);
            assert!(!preview.raw_response_persisted);
            assert!(!preview.redaction.raw_prompt_persisted);
            assert!(!preview.redaction.raw_response_persisted);
            assert!(!preview.redaction.raw_secret_returned);
        }
        "llm.confirmPromptAndSend" => {
            let confirmed: WireLlmConfirmPromptAndSendResult =
                decode_fixture_result(method, result, path);
            assert!(!confirmed.write_back_allowed);
            assert!(!confirmed.script_execution_allowed);
            assert!(!confirmed.config_mutation_allowed);
            assert!(!confirmed.snapshot_created);
            assert!(!confirmed.triage_mutation_allowed);
            assert!(!confirmed.raw_secret_returned);
            assert!(!confirmed.raw_prompt_persisted);
            assert!(!confirmed.raw_response_persisted);
            if confirmed.status == "partial" {
                assert!(confirmed.readback.is_none());
                assert!(confirmed.partial_outcome.is_some());
            } else {
                assert!(confirmed
                    .readback
                    .as_ref()
                    .is_some_and(|readback| readback.verified));
                assert!(confirmed.partial_outcome.is_none());
            }
        }
        "llm.listPromptRuns" => {
            let runs: WireLlmPromptRunListResult = decode_fixture_result(method, result, path);
            assert_eq!(runs.generated_by, "local-v2.61");
            assert_eq!(runs.count, runs.runs.len());
            assert_eq!(runs.returned_count, runs.runs.len());
            assert!(runs.total_count >= runs.returned_count);
            assert_eq!(runs.truncated, runs.returned_count < runs.total_count);
            assert!(runs.app_local_only);
            assert!(!runs.provider_request_sent);
            assert!(!runs.raw_prompt_persisted);
            assert!(!runs.raw_response_persisted);
            assert!(!runs.raw_secret_returned);
            assert!(runs.safety_flags.app_local_only);
            assert!(!runs.safety_flags.raw_prompt_persisted);
            assert!(!runs.safety_flags.raw_response_persisted);
            assert!(!runs.safety_flags.raw_secret_returned);
            for run in &runs.runs {
                assert!(run.safety_flags.app_local_only);
                assert!(!run.safety_flags.write_back_allowed);
                assert!(!run.safety_flags.script_execution_allowed);
                assert!(!run.safety_flags.raw_prompt_persisted);
                assert!(!run.safety_flags.raw_response_persisted);
                assert!(!run.redaction_summary.raw_prompt_persisted);
                assert!(!run.redaction_summary.raw_response_persisted);
            }
        }
        "llm.providerObservability" => {
            let observability: WireLlmProviderObservabilityResult =
                decode_fixture_result(method, result, path);
            assert_eq!(observability.generated_by, "local-v2.64");
            assert!(matches!(observability.status.as_str(), "ready" | "partial"));
            assert!(observability
                .filters
                .get("aggregation_uses_full_range")
                .and_then(Value::as_bool)
                .unwrap_or(false));
            assert_eq!(
                observability.summary.returned_prompt_run_count,
                observability.history_rows.len()
            );
            assert_eq!(
                observability.summary.returned_call_row_count,
                observability.call_rows.len()
            );
            assert!(observability.prompt_metadata.copy_only);
            assert!(!observability.prompt_metadata.provider_request_sent);
            assert_provider_observability_safety(&observability.safety_flags);
            for row in &observability.history_rows {
                assert!(!row.raw_prompt_persisted);
                assert!(!row.raw_response_persisted);
                assert!(!row.evidence_refs.is_empty());
            }
            for row in &observability.call_rows {
                assert!(!row.raw_prompt_persisted);
                assert!(!row.raw_response_persisted);
                assert!(!row.evidence_refs.is_empty());
            }
            for recommendation in &observability.retention_recommendations {
                assert!(!recommendation.cleanup_action_available);
                assert!(!recommendation.write_action_available);
            }
            for row in &observability.model_task_history_rows {
                assert_model_task_match_safety(&row.safety_flags);
            }
        }
        "llm.listProviderActivity" => {
            let activity: WireProviderActivityPageResult =
                decode_fixture_result(method, result, path);
            assert_eq!(activity.generated_by, "local-v2.64");
            assert_eq!(activity.returned_count, activity.rows.len());
            assert!(activity
                .total_count
                .is_some_and(|total| total >= activity.returned_count));
            assert_eq!(activity.has_more, activity.next_cursor.is_some());
            assert_eq!(activity.source_completeness, "enumerable");
            assert!(activity.incomplete_reason.is_none());
            assert!(!activity.source_revision.is_empty());
            assert_provider_observability_safety(&activity.safety_flags);
            for row in &activity.rows {
                assert!(matches!(row.kind.as_str(), "provider_call" | "prompt_run"));
                assert!(!row.id.is_empty());
                assert!(!row.evidence_refs.is_empty());
            }
        }
        "llm.listModelTaskMatches" => {
            let history: WireModelTaskMatchListResult = decode_fixture_result(method, result, path);
            assert_eq!(history.generated_by, "local-v2.91");
            assert_eq!(history.history_file, "model-task-matches.json");
            assert!(history.app_local_only);
            assert!(!history.provider_request_sent);
            assert!(!history.credential_accessed);
            assert!(!history.raw_prompt_persisted);
            assert!(!history.raw_response_persisted);
            assert!(!history.raw_trace_persisted);
            assert_model_task_match_safety(&history.safety_flags);
            assert_eq!(history.returned_record_count, history.records.len());
            assert_eq!(
                history.returned_evidence_count,
                history.recent_evidence_rows.len()
            );
            assert!(history.total_record_count >= history.returned_record_count);
            assert!(history.total_evidence_count >= history.returned_evidence_count);
            assert_eq!(
                history.truncated,
                history.returned_record_count < history.total_record_count
                    || history.returned_evidence_count < history.total_evidence_count
            );
            assert_eq!(history.summary.returned_record_count, history.records.len());
            for record in &history.records {
                assert_model_task_match_safety(&record.safety_flags);
                assert!(!record.redaction_summary.raw_prompt_persisted);
                assert!(!record.redaction_summary.raw_response_persisted);
                assert!(!record.redaction_summary.raw_trace_persisted);
            }
            for row in &history.recent_evidence_rows {
                assert_model_task_match_safety(&row.safety_flags);
            }
        }
        "llm.recordModelTaskMatch" => {
            let recorded: WireModelTaskMatchRecordResult =
                decode_fixture_result(method, result, path);
            assert_eq!(recorded.generated_by, "local-v2.91");
            assert_eq!(recorded.history_file, "model-task-matches.json");
            assert!(recorded.app_local_only);
            assert!(!recorded.provider_request_sent);
            assert!(!recorded.skill_files_mutated);
            assert!(!recorded.agent_config_mutated);
            assert!(!recorded.snapshot_created);
            assert!(!recorded.triage_mutated);
            assert!(!recorded.raw_prompt_persisted);
            assert!(!recorded.raw_response_persisted);
            assert!(!recorded.raw_trace_persisted);
            assert_model_task_match_safety(&recorded.record.safety_flags);
        }
        "llm.deleteModelTaskMatch" => {
            let deleted: WireModelTaskMatchDeleteResult =
                decode_fixture_result(method, result, path);
            assert!(deleted.app_local_only);
            assert!(!deleted.provider_request_sent);
            assert!(!deleted.skill_files_mutated);
            assert!(!deleted.agent_config_mutated);
            assert!(!deleted.snapshot_created);
            assert!(!deleted.triage_mutated);
            assert!(!deleted.raw_prompt_persisted);
            assert!(!deleted.raw_response_persisted);
            assert!(!deleted.raw_trace_persisted);
        }
        "llm.prepareAction" => {
            let prepare: WireLlmPrepareActionResult = decode_fixture_result(method, result, path);
            assert!(!prepare.write_back_allowed);
            assert!(prepare.draft_requires_user_copy);
            assert!(prepare.confirmation.required);
        }
        "rules.listTuning" => {
            let _: Vec<WireRuleTuningRecord> = decode_fixture_result(method, result, path);
        }
        "rules.setSeverityOverride" | "rules.setSuppression" => {
            let tuning: WireRuleTuningRecord = decode_fixture_result(method, result, path);
            assert!(!tuning.rule_id.is_empty());
            assert!(tuning.severity_override.is_some() || tuning.suppression_reason.is_some());
        }
        "rules.clearSeverityOverride" | "rules.clearSuppression" => {
            let _: bool = decode_fixture_result(method, result, path);
        }
        "batch.previewSkillToggles" => {
            let preview: WireBatchTogglePreviewRecord = decode_fixture_result(method, result, path);
            assert_eq!(
                preview.requested_count,
                preview.writable_count + preview.skipped_count
            );
            assert_eq!(preview.writable_count, preview.affected_items.len());
            assert_eq!(preview.skipped_count, preview.skipped_items.len());
            assert_eq!(preview.writes_allowed, preview.writable_count > 0);
            assert_eq!(preview.action.preview_method, method);
            assert_eq!(
                preview.action.apply_method.as_deref(),
                Some("batch.applySkillToggles")
            );
            assert!(!preview.preconditions.is_empty());
            assert!(!preview.preview_token.is_empty());
            assert!(!preview.capability_labels.is_empty());
            assert!(!preview.snapshot_rollback_notes.is_empty());
        }
        "batch.applySkillToggles" => {
            let applied: WireBatchToggleApplyRecord = decode_fixture_result(method, result, path);
            assert_eq!(
                applied.requested_count,
                applied.writable_count + applied.skipped_count
            );
            assert_eq!(applied.applied_count, applied.updated_records.len());
            assert!(applied.writes_allowed);
            assert_eq!(applied.action.apply_method.as_deref(), Some(method));
            assert!(applied.readback.verified);
            assert_eq!(applied.readback.action_id, applied.action.id);
            assert!(!applied.preview_token.is_empty());
            assert!(!applied.snapshot_rollback_notes.is_empty());
        }
        "script.previewExecution" => {
            let preview: WireScriptExecutionPreviewRecord =
                decode_fixture_result(method, result, path);
            assert!(!preview.execution_allowed);
            assert!(preview.confirmation.required);
            assert!(
                preview.command_preview.argv.is_empty(),
                "native identity-only preview must not invent a command"
            );
            assert!(preview
                .disabled_reason
                .contains("No verified script command"));
        }
        "script.execute" => {
            let attempt: WireScriptExecutionAttemptRecord =
                decode_fixture_result(method, result, path);
            assert_eq!(attempt.status, "blocked");
            assert!(!attempt.spawned_process);
            assert!(!attempt.preview.execution_allowed);
        }
        "skillManager.listTools" => {
            let tools: Vec<WireSkillManagerToolRecord> =
                decode_fixture_result(method, result, path);
            let npx = tools
                .iter()
                .find(|tool| tool.id == "npx-skills")
                .expect("npx skills tool fixture");
            assert_skill_manager_agents(&npx.default_agents);
            assert!(npx
                .operations
                .iter()
                .any(|operation| operation == "applyInstall"));
        }
        "skillManager.search" | "skillManager.applySearch" => {
            let search: WireSkillManagerSearchRecord = decode_fixture_result(method, result, path);
            assert_skill_manager_page_metadata(method, result);
            assert_eq!(search.preview.operation, "search");
            assert!(search.preview.requires_confirmation);
            assert!(search.preview.network_required);
            if method == "skillManager.search" {
                assert!(!search.preview.will_run);
                assert!(search.output.is_none());
                assert!(search.readback.is_none());
            } else {
                assert!(search.preview.will_run);
                assert!(search.output.is_some());
                assert!(search
                    .readback
                    .as_ref()
                    .is_some_and(|readback| readback.verified));
            }
        }
        "skillManager.listInstalled" => {
            let installed: WireSkillManagerInstalledListRecord =
                decode_fixture_result(method, result, path);
            assert_skill_manager_page_metadata(method, result);
            assert_eq!(installed.preview.operation, "listInstalled");
            assert_eq!(installed.preview.tool_id, "catalog-projection");
            assert!(installed.preview.command.is_empty());
            assert!(!installed.source_revision.is_empty());
            assert!(!installed.installed.is_empty());
        }
        "skillManager.previewInstall"
        | "skillManager.applyInstall"
        | "skillManager.previewRemove"
        | "skillManager.applyRemove"
        | "skillManager.previewUpdate"
        | "skillManager.applyUpdate" => {
            let mutation: WireSkillManagerMutationRecord =
                decode_fixture_result(method, result, path);
            assert!(mutation
                .preview
                .command
                .iter()
                .any(|arg| arg == "skills@1.5.20"));
            assert!(!mutation.preview.command.iter().any(|arg| arg == "*"));
            let action = mutation
                .preview
                .action
                .as_ref()
                .expect("mutating manager preview must expose a typed action");
            assert!(!mutation.preview.preconditions.is_empty());
            assert!(!mutation.preview.preview_token.is_empty());
            assert_eq!(mutation.applied, method.contains(".apply"));
            if method.contains("preview") {
                assert!(mutation.output.is_none());
                assert!(!mutation.preview.confirmed);
                assert!(mutation.readback.is_none());
            }
            if method.contains("apply") {
                assert!(mutation.output.is_some());
                assert!(mutation.preview.confirmed);
                let readback = mutation
                    .readback
                    .as_ref()
                    .expect("manager apply must include readback");
                assert!(readback.verified);
                assert_eq!(readback.action_id, action.id);
            }
            if method.ends_with("Install") && method.contains("preview") {
                assert_eq!(
                    mutation
                        .preview
                        .command
                        .iter()
                        .filter(|arg| arg.as_str() == "--agent")
                        .count(),
                    6
                );
                assert!(!mutation.preview.command.iter().any(|arg| arg == "--copy"));
            }
        }
        "skillManager.previewLocalCreate" | "skillManager.applyLocalCreate" => {
            let create: WireSkillManagerLocalCreateRecord =
                decode_fixture_result(method, result, path);
            assert_eq!(create.preview.operation, "localCreate");
            let action = create
                .preview
                .action
                .as_ref()
                .expect("local create preview must expose a typed action");
            assert!(!create.preview.preconditions.is_empty());
            assert!(create.source_path.contains("local-skill-library"));
            assert_eq!(create.applied, method.contains(".apply"));
            if create.applied {
                assert_eq!(
                    create.imported.as_ref().expect("imported skill").agent,
                    "tool-global"
                );
                let readback = create
                    .readback
                    .as_ref()
                    .expect("local create apply must include readback");
                assert!(readback.verified);
                assert_eq!(readback.action_id, action.id);
            } else {
                assert!(create.readback.is_none());
            }
        }
        "skillManager.deleteLocal" => {
            let delete: WireSkillManagerLocalDeleteRecord =
                decode_fixture_result(method, result, path);
            assert!(delete.app_owned);
            assert!(!delete.deleted);
            assert!(!delete.blocked_by_references.is_empty());
            assert!(delete.follow_up.is_none());
        }
        "skillManager.previewLocalArchiveImport" | "skillManager.applyLocalArchiveImport" => {
            let import: WireSkillManagerLocalArchiveImportRecord =
                decode_fixture_result(method, result, path);
            assert!(!import.preview_token.is_empty());
            assert_eq!(import.action.kind, "manager_local_archive_import");
            assert!(!import.preconditions.is_empty());
            assert!(import.follow_up.is_none());
            assert_eq!(import.applied, method.contains(".apply"));
            assert_eq!(import.confirmed, import.applied);
            if import.applied {
                assert_eq!(
                    import
                        .imported_skill
                        .as_ref()
                        .expect("imported skill")
                        .agent,
                    "tool-global"
                );
                assert!(import.instance_id.is_some());
                let readback = import
                    .readback
                    .as_ref()
                    .expect("local archive import apply must include readback");
                assert!(readback.verified);
                assert_eq!(readback.action_id, import.action.id);
            } else {
                assert!(import.readback.is_none());
            }
        }
        "skillManager.previewLocalArchiveUpdate" | "skillManager.applyLocalArchiveUpdate" => {
            let update: WireSkillManagerLocalArchiveUpdateRecord =
                decode_fixture_result(method, result, path);
            assert!(!update.preview_token.is_empty());
            assert_eq!(update.action.kind, "manager_local_archive_update");
            assert!(!update.preconditions.is_empty());
            assert!(update.follow_up.is_none());
            assert_eq!(update.applied, method.contains(".apply"));
            assert_eq!(update.confirmed, update.applied);
            if update.applied {
                let readback = update
                    .readback
                    .as_ref()
                    .expect("local archive update apply must include readback");
                assert!(readback.verified);
                assert_eq!(readback.action_id, update.action.id);
            } else {
                assert!(update.readback.is_none());
            }
        }
        "project.getContext" => {
            let _: ProjectContextState = decode_fixture_result(method, result, path);
            let state: WireProjectContextState = decode_fixture_result(method, result, path);
            assert!(
                state.active.is_some() || !state.recent.is_empty(),
                "project.getContext fixture should cover active or recent context state"
            );
        }
        "project.previewSetContext"
        | "project.previewClearContext"
        | "project.previewRemoveRecentContext"
        | "project.previewClearRecentContexts" => {
            let preview: ProjectContextActionPreview = decode_fixture_result(method, result, path);
            assert_ne!(preview.current.revision, preview.candidate.revision);
            assert!(preview.affected_count > 0);
        }
        "project.setContext"
        | "project.clearContext"
        | "project.removeRecentContext"
        | "project.clearRecentContexts" => {
            let apply: ProjectContextApplyResult = decode_fixture_result(method, result, path);
            assert!(apply.readback.verified);
            assert_eq!(
                apply.state.revision,
                apply
                    .readback
                    .observations
                    .first()
                    .expect("project readback observation")
                    .revision
            );
        }
        "project.validateContext" => {
            let _: ProjectContext = decode_fixture_result(method, result, path);
            let context: WireProjectContext = decode_fixture_result(method, result, path);
            assert!(!context.root_path.is_empty());
        }
        "catalog.scanClaude" | "catalog.scanAll" => {
            let scan: WireScanResult = decode_fixture_result(method, result, path);
            assert_eq!(scan.activity.operation, method);
            assert_eq!(scan.scanned_count, scan.activity.scanned_count);
            assert!(scan.readback.verified);
            assert_eq!(
                scan.accepted_context_revision,
                scan.readback.accepted_context_revision
            );
            assert_eq!(
                scan.catalog_scan_revision,
                scan.readback.catalog_scan_revision
            );
            if method == "catalog.scanAll" {
                assert_eq!(
                    scan.activity.status, "completed-partial",
                    "scanAll fixture must exercise degraded completion semantics"
                );
                let agents = scan
                    .activity
                    .agent_summaries
                    .as_ref()
                    .expect("scanAll fixture should include agent summaries");
                for agent in [
                    "claude-code",
                    "codex",
                    "opencode",
                    "pi",
                    "openclaw",
                    "hermes",
                ] {
                    assert!(
                        agents.iter().any(|summary| summary.agent == agent),
                        "scanAll fixture missing {agent} summary"
                    );
                    assert!(
                        scan.skills.iter().any(|skill| skill.agent == agent),
                        "scanAll fixture missing {agent} skill"
                    );
                }
                assert!(
                    agents.iter().any(|summary| {
                        summary.status == "completed"
                            && !summary.roots_scanned.is_empty()
                            && summary.roots_partial.is_empty()
                            && summary.roots_skipped.is_empty()
                    }),
                    "scanAll fixture must include a complete adapter root"
                );
                assert!(
                    agents.iter().any(|summary| {
                        summary.status == "completed-partial"
                            && !summary.roots_partial.is_empty()
                            && !summary.scan_issues.is_empty()
                    }),
                    "scanAll fixture must include a partial root with a typed issue"
                );
                assert!(
                    agents.iter().any(|summary| {
                        summary.status == "completed-with-skipped-roots"
                            && !summary.roots_skipped.is_empty()
                    }),
                    "scanAll fixture must include skipped-root recovery semantics"
                );
                let serialized =
                    serde_json::to_string(result.get("activity").expect("scan fixture activity"))
                        .expect("serialize scan fixture activity");
                assert!(!serialized.contains("/tmp/skills-copilot-home"));
                assert!(serialized.contains("$HOME"));
                assert!(serialized.contains("<adapter-root>"));
                assert!(scan.activity.log_entries.iter().any(|entry| {
                    entry.level == "warning"
                        && entry.message.contains("partial")
                        && entry.message.contains("entry_unreadable")
                }));
            } else {
                assert_eq!(
                    scan.activity.status, "completed-partial",
                    "scanClaude fixture must exercise mixed complete/partial/skipped semantics"
                );
                let agents = scan
                    .activity
                    .agent_summaries
                    .as_ref()
                    .expect("scanClaude fixture should include its Claude summary");
                assert_eq!(agents.len(), 1);
                let summary = &agents[0];
                assert_eq!(summary.agent, "claude-code");
                assert_eq!(summary.status, "completed-partial");
                assert_eq!(summary.roots_scanned, ["$HOME/.claude/skills"]);
                assert_eq!(summary.roots_partial, ["<adapter-root>/configured-claude"]);
                assert_eq!(summary.roots_skipped, ["<adapter-root>/missing-claude"]);
                assert_eq!(summary.scan_issues.len(), 2);
                assert!(summary.scan_issues.iter().any(|issue| {
                    issue.kind == "entry_unreadable"
                        && issue.path == "<adapter-root>/configured-claude/dangling-link"
                        && issue.detail == "A directory entry could not be inspected or resolved."
                }));
                assert!(summary.scan_issues.iter().any(|issue| {
                    issue.kind == "root_unavailable"
                        && issue.path == "<adapter-root>/missing-claude"
                        && issue.detail
                            == "A declared scan root was unavailable or not a directory."
                }));
                assert!(!summary.recovery_actions.is_empty());
                assert!(!scan.activity.recovery_actions.is_empty());
                assert!(scan.activity.log_entries.iter().any(|entry| {
                    entry.level == "warning"
                        && entry.message.contains("partial")
                        && entry.message.contains("entry_unreadable")
                }));
                let serialized =
                    serde_json::to_string(result.get("activity").expect("scan fixture activity"))
                        .expect("serialize scan fixture activity");
                assert!(!serialized.contains("/tmp/skills-copilot-home"));
                assert!(serialized.contains("$HOME"));
                assert!(serialized.contains("<adapter-root>"));
            }
        }
        "catalog.listSkills" => {
            let _: Vec<WireSkillRecord> = decode_fixture_result(method, result, path);
        }
        "catalog.getSkill" => {
            let skill: WireSkillDetailRecord = decode_fixture_result(method, result, path);
            assert_v28_permissions_payload(&skill.permissions, method);
        }
        "catalog.analysis" => {
            let analysis: WireCrossAgentAnalysisRecord =
                decode_fixture_result(method, result, path);
            assert_eq!(analysis.summary.total_groups, analysis.groups.len());
        }
        "catalog.listFindings" => {
            let findings: Vec<WireRuleFindingRecord> = decode_fixture_result(method, result, path);
            assert_findings_cover_v28_contract(
                &findings,
                &[
                    "frontmatter.required-fields",
                    "path.outside-workspace",
                    "fingerprint.changed",
                ],
                method,
            );
        }
        "catalog.listFindingTriage" | "catalog.setFindingTriage" => {
            let _: serde_json::Value = result.clone();
            if method == "catalog.listFindingTriage" {
                let _: Vec<WireFindingTriageRecord> = decode_fixture_result(method, result, path);
            } else {
                let _: WireFindingTriageRecord = decode_fixture_result(method, result, path);
            }
        }
        "catalog.clearFindingTriage" => {
            let _: bool = decode_fixture_result(method, result, path);
        }
        "catalog.listConflicts" => {
            let _: Vec<WireConflictGroupRecord> = decode_fixture_result(method, result, path);
        }
        "catalog.importSkill" => {
            let import: WireToolGlobalImportResult = decode_fixture_result(method, result, path);
            assert_eq!(import.imported.agent, "tool-global");
            assert_eq!(import.imported.scope, "tool-global");
            assert!(import.audit.read_only_preview);
            assert_eq!(import.instance_id, import.imported.id);
        }
        "config.toggleSkill" => {
            let _: WireSkillRecord = decode_fixture_result(method, result, path);
        }
        "skill.exportBundle" => {
            let export: WireExportedSkillBundle = decode_fixture_result(method, result, path);
            assert_eq!(export.metadata.skill_path, "skill/SKILL.md");
            assert!(!export.fingerprint.is_empty());
        }
        "skill.install" => {
            let install: WireSkillInstallPreviewRecord =
                decode_fixture_result(method, result, path);
            assert_eq!(install.action.preview_method, method);
            assert_eq!(install.action.apply_method.as_deref(), Some(method));
            assert!(!install.preconditions.is_empty());
            assert!(!install.preview_token.is_empty());
            assert!(install.confirmation.required);
            assert!(!install.files.is_empty());
        }
        "skill.listEvents" => {
            let _: Vec<WireSkillEventRecord> = decode_fixture_result(method, result, path);
        }
        "skill.listEventsPage" => {
            let page: WireSkillEventPageResult = decode_fixture_result(method, result, path);
            assert_eq!(page.records.len(), 2);
            assert_eq!(page.returned_count, 2);
            assert_eq!(page.total_count, Some(3));
            assert!(page.has_more);
            assert!(page
                .next_cursor
                .as_deref()
                .is_some_and(|value| !value.is_empty()));
            assert!(!page.source_revision.is_empty());
            assert_eq!(page.source_completeness, "enumerable");
            assert_eq!(page.incomplete_reason, None);
        }
        "config.readAgentConfig" => {
            let documents: Vec<WireConfigDocumentRecord> =
                decode_fixture_result(method, result, path);
            assert!(!documents.is_empty());
            assert!(
                documents
                    .iter()
                    .all(|document| document.revision.starts_with("sha256:")),
                "{method} revisions must use the tagged digest contract"
            );
        }
        "config.readClaudeSettings" => {
            let document: WireConfigDocumentRecord = decode_fixture_result(method, result, path);
            assert!(document.revision.starts_with("sha256:"));
        }
        "config.previewSaveClaudeSettings" => {
            let preview: WireConfigSavePreviewRecord = decode_fixture_result(method, result, path);
            assert_eq!(preview.action.preview_method, method);
            assert_eq!(
                preview.action.apply_method.as_deref(),
                Some("config.saveClaudeSettings")
            );
            assert!(preview.current_revision.starts_with("sha256:"));
            assert!(preview.candidate_content_digest.starts_with("sha256:"));
            assert!(preview
                .preview_token
                .starts_with("action-preview:v1:hmac-sha256:"));
            assert!(!preview.preconditions.is_empty());
        }
        "config.saveClaudeSettings" => {
            let applied: WireConfigSaveApplyRecord = decode_fixture_result(method, result, path);
            assert_eq!(applied.action.apply_method.as_deref(), Some(method));
            assert!(applied.document.revision.starts_with("sha256:"));
            assert!(applied.readback.verified);
            assert_eq!(applied.readback.action_id, applied.action.id);
            assert!(!applied.snapshot_id.is_empty());
        }
        "privacy.inspectLegacyContent" => {
            let inspection: WireLegacyPrivateContentInspection =
                decode_fixture_result(method, result, path);
            assert!(inspection.cleanup_required);
            assert!(inspection.read_only);
            assert!(!inspection.write_performed);
            assert!(!inspection.provider_request_sent);
            assert!(!inspection.raw_content_returned);
            assert_eq!(
                inspection.cleanup_source_count,
                inspection
                    .sources
                    .iter()
                    .filter(|source| source.cleanup_required)
                    .count()
            );
        }
        "privacy.previewCleanupLegacyContent" => {
            let preview: WireLegacyPrivateContentCleanupPreview =
                decode_fixture_result(method, result, path);
            assert!(preview.inspection.read_only);
            assert!(!preview.inspection.write_performed);
            assert!(preview.confirmation_required);
            assert_eq!(
                preview
                    .action
                    .as_ref()
                    .map(|action| action.preview_method.as_str()),
                Some(method)
            );
            assert_eq!(
                preview
                    .action
                    .as_ref()
                    .and_then(|action| action.apply_method.as_deref()),
                Some("privacy.cleanupLegacyContent")
            );
            assert!(!preview.preconditions.is_empty());
            assert!(preview
                .preview_token
                .as_deref()
                .is_some_and(|token| token.starts_with("action-preview:v1:hmac-sha256:")));
        }
        "privacy.cleanupLegacyContent" => {
            let applied: WireLegacyPrivateContentCleanupResult =
                decode_fixture_result(method, result, path);
            assert!(!applied.inspection.cleanup_required);
            assert!(applied.inspection.read_only);
            assert!(!applied.inspection.write_performed);
            assert_eq!(applied.state, "verified");
            assert!(!applied.retry_allowed);
            assert!(applied.readback.verified);
            assert_eq!(
                applied.readback.domains,
                vec!["private_content".to_string()]
            );
        }
        "snapshot.list" | "snapshot.listAgentConfig" => {
            let _: Vec<WireConfigSnapshotRecord> = decode_fixture_result(method, result, path);
        }
        "snapshot.listAgentConfigPage" => {
            let page: WireConfigSnapshotPageResult = decode_fixture_result(method, result, path);
            assert_eq!(page.records.len(), 2);
            assert_eq!(page.returned_count, 2);
            assert_eq!(page.total_count, Some(3));
            assert!(page.has_more);
            assert!(page
                .next_cursor
                .as_deref()
                .is_some_and(|value| !value.is_empty()));
            assert!(!page.source_revision.is_empty());
            assert_eq!(page.source_completeness, "enumerable");
            assert_eq!(page.incomplete_reason, None);
        }
        "snapshot.previewRollback" => {
            let preview: WireSnapshotRollbackPreviewRecord =
                decode_fixture_result(method, result, path);
            assert!(preview.current_revision.starts_with("sha256:"));
            assert!(preview.snapshot_content_digest.starts_with("sha256:"));
            assert!(preview
                .preview_token
                .starts_with("action-preview:v1:hmac-sha256:"));
            assert_eq!(preview.action.preview_method, method);
            assert!(!preview.preconditions.is_empty());
        }
        "snapshot.rollback" => {
            let applied: WireSnapshotRollbackApplyRecord =
                decode_fixture_result(method, result, path);
            assert_eq!(applied.action.apply_method.as_deref(), Some(method));
            assert!(applied.readback.verified);
            assert_eq!(applied.readback.action_id, applied.action.id);
            assert_eq!(applied.snapshot_id, "snapshot-id");
        }
        _ => panic!("no typed response decoder for fixture method {method}"),
    }
}

pub(super) fn fixture_method_from_name<'a>(name: &'a str, suffix: &str) -> &'a str {
    let stem = name
        .strip_suffix(suffix)
        .unwrap_or_else(|| panic!("fixture {name} missing suffix {suffix}"));
    supported_methods()
        .into_iter()
        .filter(|method| stem == *method || stem.starts_with(&format!("{method}.")))
        .max_by_key(|method| method.len())
        .unwrap_or(stem)
}

pub(super) fn decode_fixture_result<T: DeserializeOwned>(
    method: &str,
    result: &Value,
    path: &Path,
) -> T {
    serde_json::from_value::<T>(result.clone()).unwrap_or_else(|error| {
        panic!(
            "response fixture {} result for {method} failed typed decode: {error}",
            path.display()
        )
    })
}

pub(super) fn assert_supported_methods(method: &str, actual: &[String]) {
    let expected: Vec<String> = supported_methods()
        .into_iter()
        .map(ToOwned::to_owned)
        .collect();
    assert_eq!(actual, expected, "{method} supported_methods drifted");
}

pub(super) fn assert_skill_manager_agents(actual: &[String]) {
    assert_eq!(
        actual,
        vec![
            "claude-code",
            "pi",
            "opencode",
            "codex",
            "hermes-agent",
            "openclaw"
        ]
        .into_iter()
        .map(ToOwned::to_owned)
        .collect::<Vec<_>>()
    );
}

pub(super) fn assert_local_preview_safety(flags: &WireLocalPreviewSafetyFlags) {
    assert!(flags.read_only);
    assert!(flags.app_local_only);
    assert!(!flags.provider_request_sent);
    assert!(!flags.write_back_allowed);
    assert!(!flags.write_actions_available);
    assert!(!flags.skill_files_mutated);
    assert!(!flags.agent_config_mutated);
    assert!(!flags.script_execution_allowed);
    assert!(!flags.execution_actions_available);
    assert!(!flags.config_mutation_allowed);
    assert!(!flags.snapshot_created);
    assert!(!flags.triage_mutation_allowed);
    assert!(!flags.credential_accessed);
    assert!(!flags.raw_secret_returned);
    assert!(!flags.raw_prompt_persisted);
    assert!(!flags.raw_response_persisted);
    assert!(!flags.raw_trace_persisted);
    assert!(!flags.cloud_sync_performed);
    assert!(!flags.telemetry_emitted);
}

pub(super) fn assert_provider_observability_safety(
    flags: &WireLlmProviderObservabilitySafetyFlags,
) {
    assert!(flags.read_only);
    assert!(flags.app_local_only);
    assert!(!flags.provider_request_sent);
    assert!(!flags.credential_accessed);
    assert!(flags.draft_copy_only);
    assert!(!flags.write_back_allowed);
    assert!(!flags.write_actions_available);
    assert!(!flags.skill_files_mutated);
    assert!(!flags.agent_config_mutated);
    assert!(!flags.script_execution_allowed);
    assert!(!flags.execution_actions_available);
    assert!(!flags.config_mutation_allowed);
    assert!(!flags.snapshot_created);
    assert!(!flags.triage_mutation_allowed);
    assert!(!flags.raw_secret_returned);
    assert!(!flags.raw_prompt_persisted);
    assert!(!flags.raw_response_persisted);
    assert!(!flags.raw_trace_persisted);
    assert!(!flags.unredacted_paths_returned);
    assert!(!flags.cloud_sync_performed);
    assert!(!flags.telemetry_emitted);
}

pub(super) fn assert_model_task_match_safety(flags: &WireModelTaskMatchSafetyFlags) {
    assert!(flags.app_local_only);
    assert!(!flags.provider_request_sent);
    assert!(!flags.credential_accessed);
    assert!(flags.draft_copy_only);
    assert!(!flags.write_back_allowed);
    assert!(!flags.write_actions_available);
    assert!(!flags.skill_files_mutated);
    assert!(!flags.agent_config_mutated);
    assert!(!flags.script_execution_allowed);
    assert!(!flags.execution_actions_available);
    assert!(!flags.config_mutation_allowed);
    assert!(!flags.snapshot_created);
    assert!(!flags.triage_mutation_allowed);
    assert!(!flags.raw_secret_returned);
    assert!(!flags.raw_prompt_persisted);
    assert!(!flags.raw_response_persisted);
    assert!(!flags.raw_trace_persisted);
    assert!(!flags.unredacted_paths_returned);
    assert!(!flags.cloud_sync_performed);
    assert!(!flags.telemetry_emitted);
}

pub(super) fn assert_findings_cover_v28_contract(
    findings: &[WireRuleFindingRecord],
    expected_rule_ids: &[&str],
    method: &str,
) {
    for rule_id in expected_rule_ids {
        let finding = findings
            .iter()
            .find(|finding| finding.rule_id == *rule_id)
            .unwrap_or_else(|| panic!("{method} fixture missing V2.8 rule id {rule_id}"));
        assert!(
            finding
                .suggestion
                .as_deref()
                .is_some_and(|suggestion| !suggestion.is_empty()),
            "{method} fixture rule {rule_id} should include suggestion text"
        );
    }
}

pub(super) fn assert_v28_permissions_payload(permissions: &Value, method: &str) {
    let Some(object) = permissions.as_object() else {
        panic!("{method} fixture permissions should be an object");
    };
    for key in ["raw", "normalized", "unknown_safe"] {
        assert!(
            object.contains_key(key),
            "{method} fixture permissions missing {key} payload"
        );
    }
    assert_eq!(
        permissions
            .get("normalized")
            .and_then(|payload| payload.get("network"))
            .and_then(Value::as_str),
        Some("unknown"),
        "{method} fixture should preserve unknown normalized network state"
    );
    assert_eq!(
        permissions
            .get("unknown_safe")
            .and_then(|payload| payload.get("network"))
            .and_then(Value::as_str),
        Some("none"),
        "{method} fixture should include unknown-safe network fallback"
    );
}

#[test]
pub(super) fn skill_detail_contract_accepts_legacy_and_v28_permission_payloads() {
    let base = serde_json::json!({
        "id": "skill-instance-id",
        "agent": "claude-code",
        "scope": "agent-global",
        "path": "/tmp/skills-copilot-home/.claude/skills/demo/SKILL.md",
        "display_path": "/tmp/skills-copilot-home/.claude/skills/demo/SKILL.md",
        "definition_id": "definition-id",
        "name": "demo",
        "description": "Fixture skill",
        "state": "loaded",
        "enabled": true,
        "frontmatter_raw": "name: demo\ndescription: Fixture skill\n",
        "body": "Fixture body.",
        "fingerprint": "fixture-fingerprint"
    });

    for permissions in [
        serde_json::json!({}),
        serde_json::json!({
            "tools": ["Read"],
            "files": [],
            "network": "none",
            "exec": false,
            "requires_human": false
        }),
        serde_json::json!({
            "raw": {
                "allowed-tools": "Read",
                "network": "unexpected-network-mode"
            },
            "normalized": {
                "tools": ["Read"],
                "files": [],
                "network": "unknown",
                "exec": false,
                "requires_human": true
            },
            "unknown_safe": {
                "tools": [],
                "files": [],
                "network": "none",
                "exec": false,
                "requires_human": true
            }
        }),
    ] {
        let mut payload = base.clone();
        payload["permissions"] = permissions;
        let _: WireSkillDetailRecord = serde_json::from_value(payload)
            .expect("skill detail fixture should decode permissions payload variant");
    }
}
