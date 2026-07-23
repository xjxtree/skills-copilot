use super::dispatch_fixtures::*;
use super::*;
use std::{
    ffi::{OsStr, OsString},
    sync::{Mutex, MutexGuard},
};

static ENV_MUTATION_LOCK: Mutex<()> = Mutex::new(());

pub(super) struct EnvVarGuard {
    key: OsString,
    previous: Option<OsString>,
    _lock: MutexGuard<'static, ()>,
}

impl EnvVarGuard {
    pub(super) fn set(key: impl AsRef<OsStr>, value: impl AsRef<OsStr>) -> Self {
        let key = key.as_ref().to_os_string();
        let lock = ENV_MUTATION_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous = env::var_os(&key);
        std::env::set_var(&key, value.as_ref());
        Self {
            key,
            previous,
            _lock: lock,
        }
    }

    fn remove_current(&self) {
        std::env::remove_var(&self.key);
    }
}

impl Drop for EnvVarGuard {
    fn drop(&mut self) {
        if let Some(previous) = self.previous.as_ref() {
            std::env::set_var(&self.key, previous);
        } else {
            std::env::remove_var(&self.key);
        }
    }
}

#[test]
fn status_request_returns_supported_methods() {
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
        id: Some("1".to_string()),
        method: "service.status".to_string(),
        params: Value::Null,
    });

    assert!(response.ok);
    let result = response.result.expect("status result");
    assert_eq!(
        result.get("protocol_version").and_then(Value::as_u64),
        Some(u64::from(SERVICE_PROTOCOL_VERSION))
    );
    let methods = result
        .get("supported_methods")
        .and_then(Value::as_array)
        .expect("methods");
    assert!(methods.contains(&Value::String("app.version".to_string())));
    assert!(methods.contains(&Value::String("app.stateSnapshot".to_string())));
    assert!(methods.contains(&Value::String("adapter.listDiagnostics".to_string())));
    assert!(methods.contains(&Value::String("app.search".to_string())));
    assert!(methods.contains(&Value::String("session.previewLocalSessions".to_string())));
    assert!(methods.contains(&Value::String("llm.status".to_string())));
    assert!(methods.contains(&Value::String("llm.listProviderProfiles".to_string())));
    assert!(methods.contains(&Value::String("llm.saveProviderProfile".to_string())));
    assert!(methods.contains(&Value::String("llm.deleteProviderProfile".to_string())));
    assert!(methods.contains(&Value::String("llm.testProviderConnection".to_string())));
    assert!(methods.contains(&Value::String("llm.previewPrompt".to_string())));
    assert!(methods.contains(&Value::String("llm.confirmPromptAndSend".to_string())));
    assert!(methods.contains(&Value::String("llm.listPromptRuns".to_string())));
    assert!(methods.contains(&Value::String("llm.providerObservability".to_string())));
    assert!(methods.contains(&Value::String("llm.prepareAction".to_string())));
    assert!(methods.contains(&Value::String("rules.listTuning".to_string())));
    assert!(methods.contains(&Value::String("rules.setSeverityOverride".to_string())));
    assert!(methods.contains(&Value::String("rules.clearSeverityOverride".to_string())));
    assert!(methods.contains(&Value::String("rules.setSuppression".to_string())));
    assert!(methods.contains(&Value::String("rules.clearSuppression".to_string())));
    assert!(methods.contains(&Value::String("script.previewExecution".to_string())));
    assert!(methods.contains(&Value::String("script.execute".to_string())));
    assert!(methods.contains(&Value::String("project.getContext".to_string())));
    assert!(methods.contains(&Value::String("project.setContext".to_string())));
    assert!(methods.contains(&Value::String("project.clearContext".to_string())));
    assert!(methods.contains(&Value::String("project.removeRecentContext".to_string())));
    assert!(methods.contains(&Value::String("project.clearRecentContexts".to_string())));
    assert!(methods.contains(&Value::String("project.validateContext".to_string())));
    assert!(methods.contains(&Value::String("catalog.listSkills".to_string())));
    assert!(methods.contains(&Value::String("catalog.getSkill".to_string())));
    assert!(methods.contains(&Value::String("catalog.analysis".to_string())));
    assert!(methods.contains(&Value::String("catalog.scanAll".to_string())));
    assert!(methods.contains(&Value::String("skill.exportBundle".to_string())));
    assert!(methods.contains(&Value::String("skill.install".to_string())));
    assert!(methods.contains(&Value::String("config.toggleSkill".to_string())));
    assert!(methods.contains(&Value::String("config.readAgentConfig".to_string())));
    assert!(methods.contains(&Value::String("config.readClaudeSettings".to_string())));
    assert!(methods.contains(&Value::String("config.saveClaudeSettings".to_string())));
    assert!(methods.contains(&Value::String("snapshot.list".to_string())));
    assert!(methods.contains(&Value::String("snapshot.rollback".to_string())));
    let diagnostics = result
        .get("adapter_diagnostics")
        .and_then(Value::as_array)
        .expect("adapter diagnostics");
    assert!(diagnostics.iter().any(|diagnostic| {
        diagnostic.get("agent").and_then(Value::as_str) == Some("hermes")
            && diagnostic.get("status").and_then(Value::as_str) == Some("guarded")
            && diagnostic
                .pointer("/access/writable_status")
                .and_then(Value::as_str)
                == Some("guarded-v2.97")
            && diagnostic.pointer("/config/status").and_then(Value::as_str) == Some("not-detected")
    }));
    let project_context = result
        .get("project_context")
        .and_then(Value::as_object)
        .expect("project context summary");
    assert_eq!(
        project_context.get("source").and_then(Value::as_str),
        Some("none")
    );
    let llm = result.get("llm").and_then(Value::as_object).expect("llm");
    assert_eq!(llm.get("enabled").and_then(Value::as_bool), Some(false));
    assert_eq!(llm.get("configured").and_then(Value::as_bool), Some(false));
    assert_eq!(
        llm.get("credential_persistence_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    let script_execution = result
        .get("script_execution")
        .and_then(Value::as_object)
        .expect("script execution status");
    assert_eq!(
        script_execution.get("enabled").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        script_execution
            .get("llm_initiation_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
}

#[test]
fn app_state_snapshot_migrates_legacy_catalog_before_reading() {
    let root = env::temp_dir().join(format!(
        "skills-copilot-state-snapshot-migration-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let app_data_dir = root.join("app-data");
    fs::create_dir_all(&app_data_dir).expect("create legacy app data");
    let host = test_host(app_data_dir);
    {
        let conn = rusqlite::Connection::open(host.catalog_path()).expect("open legacy catalog");
        conn.execute_batch(
            "CREATE TABLE config_snapshot (
                id TEXT PRIMARY KEY,
                agent TEXT NOT NULL,
                scope TEXT NOT NULL,
                target TEXT NOT NULL,
                content TEXT NOT NULL,
                reason TEXT NOT NULL,
                created_at INTEGER NOT NULL
             );
             INSERT INTO config_snapshot (
                id, agent, scope, target, content, reason, created_at
             ) VALUES (
                'legacy-snapshot', 'claude-code', 'agent-global',
                '/tmp/settings.json', '{}', 'pre-toggle', 1
             );",
        )
        .expect("seed legacy snapshot schema");
    }

    let response = host.handle(ServiceRequest {
        id: Some("legacy-state-snapshot".to_string()),
        method: "app.stateSnapshot".to_string(),
        params: json!({}),
    });
    assert!(response.ok, "{response:?}");
    let result = response.result.expect("state snapshot result");
    assert_eq!(result["snapshots"][0]["id"], "legacy-snapshot");
    assert_eq!(result["snapshots"][0]["project_root"], Value::Null);

    let conn = rusqlite::Connection::open(host.catalog_path()).expect("reopen migrated catalog");
    let columns = conn
        .prepare("PRAGMA table_info(config_snapshot)")
        .expect("prepare schema query")
        .query_map([], |row| row.get::<_, String>(1))
        .expect("query schema")
        .collect::<Result<Vec<_>, _>>()
        .expect("collect schema");
    assert!(columns.iter().any(|column| column == "project_root"));
    drop(conn);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn default_app_data_dir_uses_agent_copilot_bundle_id() {
    let home = env::temp_dir().join(format!(
        "skills-copilot-app-data-default-test-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));

    let preferred = default_app_data_dir(&home);
    let legacy = legacy_app_data_dir(&home);

    assert!(preferred.ends_with(DEFAULT_BUNDLE_ID));
    assert!(legacy.ends_with(LEGACY_BUNDLE_ID));
    assert_ne!(preferred, legacy);
}

#[test]
fn resolve_default_app_data_dir_copies_legacy_data_once() {
    let home = env::temp_dir().join(format!(
        "skills-copilot-app-data-migration-test-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let preferred = default_app_data_dir(&home);
    let legacy = legacy_app_data_dir(&home);
    fs::create_dir_all(legacy.join("llm")).expect("create legacy llm data");
    fs::write(legacy.join("project-context.json"), "{\"active\":null}\n")
        .expect("seed legacy project context");
    fs::write(
        legacy.join("llm").join("provider-profiles.json"),
        "{\"version\":1,\"profiles\":[]}\n",
    )
    .expect("seed legacy provider profiles");

    let resolved = resolve_default_app_data_dir(&home).expect("resolve migrated app data dir");

    assert_eq!(resolved, preferred);
    assert!(legacy.exists(), "legacy app data must not be deleted");
    assert_eq!(
        fs::read_to_string(preferred.join("project-context.json"))
            .expect("migrated project context"),
        "{\"active\":null}\n"
    );
    assert_eq!(
        fs::read_to_string(preferred.join("llm").join("provider-profiles.json"))
            .expect("migrated provider metadata"),
        "{\"version\":1,\"profiles\":[]}\n"
    );
    let marker: Value = serde_json::from_str(
        &fs::read_to_string(preferred.join("agent-copilot-app-data-migration.json"))
            .expect("migration marker"),
    )
    .expect("parse migration marker");
    assert_eq!(
        marker.get("source_bundle_id").and_then(Value::as_str),
        Some(LEGACY_BUNDLE_ID)
    );
    assert_eq!(
        marker.get("target_bundle_id").and_then(Value::as_str),
        Some(DEFAULT_BUNDLE_ID)
    );

    let _ = fs::remove_dir_all(home);
}

#[test]
fn resolve_default_app_data_dir_does_not_overwrite_existing_preferred_data() {
    let home = env::temp_dir().join(format!(
        "skills-copilot-app-data-existing-test-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let preferred = default_app_data_dir(&home);
    let legacy = legacy_app_data_dir(&home);
    fs::create_dir_all(&preferred).expect("create preferred app data");
    fs::create_dir_all(&legacy).expect("create legacy app data");
    fs::write(
        preferred.join("project-context.json"),
        "{\"preferred\":true}\n",
    )
    .expect("seed preferred data");
    fs::write(legacy.join("project-context.json"), "{\"legacy\":true}\n")
        .expect("seed legacy data");

    let resolved = resolve_default_app_data_dir(&home).expect("resolve preferred app data dir");

    assert_eq!(resolved, preferred);
    assert_eq!(
        fs::read_to_string(preferred.join("project-context.json")).expect("preferred data"),
        "{\"preferred\":true}\n"
    );
    assert!(
        !preferred
            .join("agent-copilot-app-data-migration.json")
            .exists(),
        "existing preferred data should not receive a migration marker"
    );

    let _ = fs::remove_dir_all(home);
}

#[test]
fn explicit_app_data_env_override_bypasses_default_migration() {
    let override_dir = env::temp_dir().join(format!(
        "skills-copilot-app-data-override-test-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let _guard = EnvVarGuard::set("SKILLS_COPILOT_APP_DATA_DIR", &override_dir);

    let host = ServiceHost::from_env().expect("host from env");

    assert_eq!(host.app_data_dir, override_dir);
}

#[test]
fn list_agent_config_snapshots_returns_selected_agent_timeline_only() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-service-timeline-{}",
        std::process::id()
    ));
    let app_data_dir = temp_root.join("app-data");
    fs::create_dir_all(&app_data_dir).expect("create app data");
    let host = test_host(app_data_dir);
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    catalog.init().expect("init catalog");

    for (id, agent, scope, target, content, created_at_ms) in [
        (
            "snap-claude",
            "claude-code",
            "agent-global",
            "/tmp/home/.claude/settings.json",
            "{}\n",
            10,
        ),
        (
            "snap-codex-new",
            "codex",
            "agent-global",
            "/tmp/home/.codex/config.toml",
            "disable_response_storage = true\n",
            30,
        ),
        (
            "snap-codex-old",
            "codex",
            "agent-project",
            "/tmp/project/.codex/config.toml",
            "approval_policy = \"never\"\n",
            20,
        ),
        (
            "snap-opencode",
            "opencode",
            "agent-global",
            "/tmp/home/.config/opencode/opencode.json",
            "{}\n",
            40,
        ),
    ] {
        catalog
            .create_config_snapshot(skills_copilot_catalog::ConfigSnapshotDraft {
                id,
                agent,
                scope,
                project_root: None,
                target,
                content,
                reason: "pre-toggle",
                created_at_ms,
            })
            .expect("create snapshot");
    }

    let response = host.handle(ServiceRequest {
        id: Some("timeline".to_string()),
        method: "snapshot.listAgentConfig".to_string(),
        params: json!({ "agent": "codex" }),
    });

    assert!(response.ok);
    let result = response.result.expect("timeline result");
    let snapshots: Vec<WireConfigSnapshotRecord> =
        serde_json::from_value(result).expect("decode snapshots");
    assert_eq!(
        snapshots
            .iter()
            .map(|snapshot| snapshot.id.as_str())
            .collect::<Vec<_>>(),
        vec!["snap-codex-new"]
    );
    assert!(snapshots.iter().all(|snapshot| snapshot.agent == "codex"));

    let scoped_response = host.handle(ServiceRequest {
        id: Some("timeline-scope".to_string()),
        method: "snapshot.listAgentConfig".to_string(),
        params: json!({ "agent": "codex", "scope": "agent-project" }),
    });
    assert!(scoped_response.ok);
    let scoped_result = scoped_response.result.expect("scoped timeline result");
    let scoped_snapshots: Vec<WireConfigSnapshotRecord> =
        serde_json::from_value(scoped_result).expect("decode scoped snapshots");
    assert!(
        scoped_snapshots.is_empty(),
        "legacy project snapshots without a bound project root must fail closed"
    );

    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn catalog_analysis_returns_empty_read_only_summary() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-analysis-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let response = host.handle(ServiceRequest {
        id: Some("analysis".to_string()),
        method: "catalog.analysis".to_string(),
        params: Value::Null,
    });

    assert!(response.ok);
    let result = response.result.expect("analysis result");
    assert_eq!(
        result
            .pointer("/summary/total_groups")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        result
            .pointer("/summary/affected_skill_count")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        result.get("groups").and_then(Value::as_array).map(Vec::len),
        Some(0)
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_status_defaults_disabled_without_creating_files() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-llm-status-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-llm-home-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
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
        id: Some("llm-status".to_string()),
        method: "llm.status".to_string(),
        params: Value::Null,
    });

    assert!(response.ok);
    let result = response.result.expect("llm status");
    assert_eq!(result.get("enabled").and_then(Value::as_bool), Some(false));
    assert_eq!(
        result.get("configured").and_then(Value::as_bool),
        Some(false)
    );
    assert!(result.get("provider").is_some_and(Value::is_null));
    assert!(result.get("model").is_some_and(Value::is_null));
    assert_eq!(
        result.get("credentials_storage").and_then(Value::as_str),
        Some("none")
    );
    assert_eq!(
        result
            .get("credential_persistence_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result.get("provider_profile_count").and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        result
            .get("raw_prompt_persistence_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .get("raw_response_persistence_allowed")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        !app_data_dir.exists(),
        "llm.status must not initialize app data"
    );
    assert!(
        !user_home.exists(),
        "llm.status must not create credential or config roots"
    );
}

#[test]
fn llm_provider_profile_save_persists_metadata_without_secret_file() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-profile-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let (_, response) = confirmed_action_request(
        &host,
        "llm.previewSaveProviderProfile",
        "llm.saveProviderProfile",
        json!({
            "id": "fixture-openai",
            "display_name": "Fixture OpenAI",
            "provider_type": "openai-compatible",
            "base_url": "https://example.invalid/v1",
            "model": "fixture-model",
            "enabled": true,
            "single_request_token_limit": 4096,
            "monthly_budget_usd": 3.5
        }),
    );

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("provider save result");
    assert_eq!(
        result.pointer("/profile/id").and_then(Value::as_str),
        Some("fixture-openai")
    );
    assert_eq!(
        result
            .pointer("/profile/provider_type")
            .and_then(Value::as_str),
        Some("openai-compatible")
    );
    assert_eq!(
        result
            .pointer("/credential_status/secret_available")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result.get("raw_secret_returned").and_then(Value::as_bool),
        Some(false)
    );

    let profiles_path = provider_profiles_path(&app_data_dir);
    let profile_content = fs::read_to_string(&profiles_path).expect("profile metadata");
    assert!(profile_content.contains("fixture-openai"));
    assert!(!profile_content.contains("api_key"));
    assert_private_path_mode(&profiles_path, 0o600);
    assert_private_path_mode(profiles_path.parent().expect("profile parent"), 0o700);
    assert!(!app_data_dir.join("llm-credentials.json").exists());
    assert!(!app_data_dir.join("llm.yaml").exists());

    let list = host.handle(ServiceRequest {
        id: Some("provider-list".to_string()),
        method: "llm.listProviderProfiles".to_string(),
        params: Value::Null,
    });
    assert!(list.ok, "{:?}", list.error);
    let list_result = list.result.expect("provider list");
    assert_eq!(
        list_result
            .pointer("/profiles/0/id")
            .and_then(Value::as_str),
        Some("fixture-openai")
    );
    assert_eq!(
        list_result
            .get("raw_secrets_returned")
            .and_then(Value::as_bool),
        Some(false)
    );

    let status = host.handle(ServiceRequest {
        id: Some("provider-status".to_string()),
        method: "llm.status".to_string(),
        params: Value::Null,
    });
    assert!(status.ok);
    let status_result = status.result.expect("status");
    assert_eq!(
        status_result.get("configured").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        status_result
            .get("provider_profile_count")
            .and_then(Value::as_u64),
        Some(1)
    );
    assert_eq!(
        status_result
            .get("credentials_storage")
            .and_then(Value::as_str),
        Some("keychain")
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_provider_profile_rejects_unsafe_base_urls() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-url-reject-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let unsafe_base_urls = [
        "http://localhost.evil.invalid/v1".to_string(),
        "http://localhost@evil.invalid/v1".to_string(),
        "http://localhostevil.invalid/v1".to_string(),
        "http://127.0.0.2/v1".to_string(),
        "https://user:pass@example.invalid/v1".to_string(),
        format!("https://api.example.invalid/v1?{}=value", "api_key"),
        format!("https://api.example.invalid/v1#{}=value", "token"),
    ];

    for (index, base_url) in unsafe_base_urls.iter().enumerate() {
        let response = host.handle(ServiceRequest {
            id: Some(format!("provider-save-{index}")),
            method: "llm.previewSaveProviderProfile".to_string(),
            params: json!({
                "id": format!("unsafe-{index}"),
                "display_name": format!("Unsafe {index}"),
                "provider_type": "openai-compatible",
                "base_url": base_url,
                "model": "fixture-model",
                "enabled": true
            }),
        });

        assert!(!response.ok, "{base_url} should be rejected");
        let error = response.error.expect("provider error");
        assert_eq!(error.code, "provider_error");
        assert!(
            error.message.contains("base_url"),
            "{base_url} should fail with a base_url validation error, got {}",
            error.message
        );
    }

    assert!(
        !app_data_dir.exists(),
        "rejected provider URLs must not initialize app data"
    );
}

#[test]
fn llm_provider_profile_accepts_https_and_exact_loopback_http_urls() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-url-accept-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let safe_base_urls = [
        "https://api.example.invalid/v1",
        "http://localhost/v1",
        "http://127.0.0.1/v1",
        "http://[::1]/v1",
    ];

    for (index, base_url) in safe_base_urls.iter().enumerate() {
        let (_, response) = confirmed_action_request(
            &host,
            "llm.previewSaveProviderProfile",
            "llm.saveProviderProfile",
            json!({
                "id": format!("safe-{index}"),
                "display_name": format!("Safe {index}"),
                "provider_type": "openai-compatible",
                "base_url": base_url,
                "model": "fixture-model",
                "enabled": true
            }),
        );

        assert!(
            response.ok,
            "{base_url} should be accepted: {:?}",
            response.error
        );
        assert_eq!(
            response
                .result
                .as_ref()
                .and_then(|result| result.pointer("/profile/base_url"))
                .and_then(Value::as_str),
            Some(*base_url)
        );
    }

    let profiles_path = provider_profiles_path(&app_data_dir);
    assert_private_path_mode(&profiles_path, 0o600);
    assert_private_path_mode(profiles_path.parent().expect("profile parent"), 0o700);

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_test_provider_connection_blocks_without_key_and_writes_metadata_only() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-test-call-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let (_, save) = confirmed_action_request(
        &host,
        "llm.previewSaveProviderProfile",
        "llm.saveProviderProfile",
        json!({
            "id": "fixture-claude",
            "display_name": "Fixture Claude",
            "provider_type": "claude-compatible",
            "base_url": "https://example.invalid",
            "model": "fixture-claude-model",
            "enabled": true,
            "api_version": "2023-06-01",
            "single_request_token_limit": 4096,
            "monthly_budget_usd": 2.0
        }),
    );
    assert!(save.ok, "{:?}", save.error);

    let (_, test) = confirmed_action_request(
        &host,
        "llm.previewProviderConnectionTest",
        "llm.testProviderConnection",
        json!({
            "profile_id": "fixture-claude",
            "timeout_ms": 250
        }),
    );

    assert!(test.ok, "{:?}", test.error);
    let result = test.result.expect("test connection");
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
        result.get("raw_secret_returned").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result.pointer("/audit/action_type").and_then(Value::as_str),
        Some("test_connection")
    );
    assert_eq!(
        result
            .pointer("/audit/destination_host")
            .and_then(Value::as_str),
        Some("example.invalid")
    );
    assert_eq!(
        result
            .pointer("/audit/raw_prompt_persisted")
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result
            .pointer("/audit/raw_response_persisted")
            .and_then(Value::as_bool),
        Some(false)
    );

    let audit_path = provider_call_metadata_path(&app_data_dir);
    let audit_content = fs::read_to_string(&audit_path).expect("provider metadata");
    assert!(audit_content.contains("\"action_type\":\"test_connection\""));
    assert!(audit_content.contains("\"destination_host\":\"example.invalid\""));
    assert!(!audit_content.contains("connection test"));
    assert!(!audit_content.contains("api_key"));
    assert!(!host.script_execution_audit_path().exists());

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_test_provider_connection_uses_preserved_key_after_blank_save() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-preserve-key-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let (base_url, server) = spawn_mock_openai_server();
    let host = test_host(app_data_dir.clone());
    let profile_id = format!("mock-openai-preserve-{}", unique_suffix());
    let secret_env = provider_test_secret_env_name(&profile_id);
    let _secret_env_guard = EnvVarGuard::set(&secret_env, "test-secret-key");

    let (_, save) = confirmed_action_request(
        &host,
        "llm.previewSaveProviderProfile",
        "llm.saveProviderProfile",
        json!({
            "id": profile_id,
            "display_name": "Mock OpenAI Preserve",
            "provider_type": "openai-compatible",
            "base_url": base_url,
            "model": "mock-model",
            "enabled": true,
            "single_request_token_limit": 4096,
            "monthly_budget_usd": 10.0
        }),
    );
    assert!(save.ok, "{:?}", save.error);
    assert_eq!(
        save.result
            .as_ref()
            .and_then(|result| result.pointer("/profile/credential_status/secret_available"))
            .and_then(Value::as_bool),
        Some(true)
    );

    let (_, blank_resave) = confirmed_action_request(
        &host,
        "llm.previewSaveProviderProfile",
        "llm.saveProviderProfile",
        json!({
            "id": profile_id,
            "display_name": "Mock OpenAI Preserve",
            "provider_type": "openai-compatible",
            "base_url": base_url,
            "model": "mock-model-updated",
            "enabled": true,
            "single_request_token_limit": 4096,
            "monthly_budget_usd": 10.0
        }),
    );
    assert!(blank_resave.ok, "{:?}", blank_resave.error);
    assert_eq!(
        blank_resave
            .result
            .as_ref()
            .and_then(|result| result.pointer("/profile/credential_status/secret_available"))
            .and_then(Value::as_bool),
        Some(true)
    );

    let (_, test) = confirmed_action_request(
        &host,
        "llm.previewProviderConnectionTest",
        "llm.testProviderConnection",
        json!({
            "profile_id": profile_id,
            "timeout_ms": 2_000
        }),
    );

    assert!(test.ok, "{:?}", test.error);
    let result = test.result.expect("test connection");
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

    let request_text = server.join().expect("mock server thread");
    assert!(request_text
        .to_lowercase()
        .contains("authorization: bearer test-secret-key"));
    let audit_content =
        fs::read_to_string(provider_call_metadata_path(&app_data_dir)).expect("audit content");
    assert!(audit_content.contains("\"status\":\"succeeded\""));
    assert!(!audit_content.contains("test-secret-key"));

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_test_provider_connection_downgrades_stale_credential_metadata() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-provider-stale-key-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let profile_id = format!("mock-openai-stale-{}", unique_suffix());
    let secret_env = provider_test_secret_env_name(&profile_id);
    let secret_env_guard = EnvVarGuard::set(&secret_env, "test-secret-key");

    let (_, save) = confirmed_action_request(
        &host,
        "llm.previewSaveProviderProfile",
        "llm.saveProviderProfile",
        json!({
            "id": profile_id,
            "display_name": "Mock OpenAI Stale",
            "provider_type": "openai-compatible",
            "base_url": "https://api.fixture.invalid/v1",
            "model": "mock-model",
            "enabled": true,
            "single_request_token_limit": 4096,
            "monthly_budget_usd": 10.0
        }),
    );
    assert!(save.ok, "{:?}", save.error);
    secret_env_guard.remove_current();

    let (_, test) = confirmed_action_request(
        &host,
        "llm.previewProviderConnectionTest",
        "llm.testProviderConnection",
        json!({
            "profile_id": profile_id,
            "timeout_ms": 250
        }),
    );
    assert!(test.ok, "{:?}", test.error);
    let result = test.result.expect("test connection");
    assert_eq!(
        result.get("status").and_then(Value::as_str),
        Some("blocked")
    );
    assert_eq!(
        result.pointer("/audit/error_code").and_then(Value::as_str),
        Some("credential_unavailable")
    );

    let list = host.handle(ServiceRequest {
        id: Some("provider-list".to_string()),
        method: "llm.listProviderProfiles".to_string(),
        params: Value::Null,
    });
    assert!(list.ok, "{:?}", list.error);
    assert_eq!(
        list.result
            .as_ref()
            .and_then(|result| result.pointer("/profiles/0/credential_status/secret_available"))
            .and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        list.result
            .as_ref()
            .and_then(|result| result.pointer("/profiles/0/credential_reference/secret_persisted"))
            .and_then(Value::as_bool),
        Some(false)
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn script_preview_returns_disabled_scope_without_writing_audit() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-script-preview-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let response = host.handle(ServiceRequest {
        id: Some("script-preview".to_string()),
        method: "script.previewExecution".to_string(),
        params: json!({
            "command": ["python3", "scripts/build.py"],
            "cwd": "fixture-project",
            "env": {
                "API_TOKEN": "fixture-redacted-value"
            },
            "network": "full",
            "files": ["./src/**"],
            "skill_instance_id": "skill-fixture",
            "initiated_by": "user"
        }),
    });

    assert!(response.ok);
    let result = response.result.expect("preview result");
    assert_eq!(
        result.get("execution_allowed").and_then(Value::as_bool),
        Some(false)
    );
    assert_eq!(
        result.get("initiator_allowed").and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        result
            .pointer("/confirmation/required")
            .and_then(Value::as_bool),
        Some(true)
    );
    assert_eq!(
        result.pointer("/env/value_policy").and_then(Value::as_str),
        Some("values-redacted")
    );
    let serialized = serde_json::to_string(&result).expect("serialize result");
    assert!(!serialized.contains("fixture-redacted-value"));
    assert!(
        !host.script_execution_audit_path().exists(),
        "preview must not write audit records"
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn script_preview_accepts_native_skill_identity_without_guessing_a_command() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-script-native-preview-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let response = host.handle(ServiceRequest {
        id: Some("script-native-preview".to_string()),
        method: "script.previewExecution".to_string(),
        params: json!({
            "instance_id": "skill-fixture",
            "definition_id": "definition-fixture",
            "agent": "codex"
        }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("preview result");
    assert_eq!(
        result.get("skill_instance_id").and_then(Value::as_str),
        Some("skill-fixture")
    );
    assert_eq!(
        result
            .pointer("/command_preview/argv")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(0),
        "identity-only preview must never invent a command"
    );
    assert_eq!(
        result.get("execution_allowed").and_then(Value::as_bool),
        Some(false)
    );
    assert!(
        result
            .get("disabled_reason")
            .and_then(Value::as_str)
            .is_some_and(|reason| reason.contains("No verified script command")),
        "identity-only preview must explain that no verified command is available"
    );
    assert!(
        !host.script_execution_audit_path().exists(),
        "preview must not write audit records"
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn script_execute_is_blocked_before_any_audit_io() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-script-confirm-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let response = host.handle(ServiceRequest {
        id: Some("script-execute-unconfirmed".to_string()),
        method: "script.execute".to_string(),
        params: json!({
            "command": ["sh", "-c", "touch should-not-run"],
            "confirmed": false
        }),
    });

    assert!(!response.ok);
    let error = response.error.expect("blocked mutation error");
    assert_eq!(error.code, "mutation_disabled");
    assert!(
        !host.script_execution_audit_path().exists(),
        "unconfirmed execute must not write an audit record"
    );

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn script_execute_confirmed_still_rejects_without_audit_or_process() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-script-audit-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let response = host.handle(ServiceRequest {
        id: Some("script-execute-confirmed".to_string()),
        method: "script.execute".to_string(),
        params: json!({
            "command": ["sh", "-c", "touch spawned-marker"],
            "confirmed": true
        }),
    });

    assert!(!response.ok);
    assert_eq!(
        response.error.expect("blocked mutation error").code,
        "mutation_disabled"
    );
    assert!(!host.script_execution_audit_path().exists());
    assert!(!app_data_dir.join("spawned-marker").exists());

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn script_execute_llm_initiator_is_rejected_without_audit_io() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-script-llm-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let response = host.handle(ServiceRequest {
        id: Some("script-execute-llm".to_string()),
        method: "script.execute".to_string(),
        params: json!({
            "command": ["python3", "-c", "print('blocked')"],
            "confirmed": true,
            "initiated_by": "llm"
        }),
    });

    assert!(!response.ok);
    assert_eq!(
        response.error.expect("blocked mutation error").code,
        "mutation_disabled"
    );
    assert!(!host.script_execution_audit_path().exists());

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_prepare_action_never_allows_direct_write_or_leaks_skill_content() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-llm-prepare-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());
    let skill_path = app_data_dir.join("secret-project-path").join("SKILL.md");
    seed_catalog_with_llm_skill(&host, &skill_path);

    let response = host.handle(ServiceRequest {
        id: Some("llm-prepare".to_string()),
        method: "llm.prepareAction".to_string(),
        params: json!({
            "kind": "analyze",
            "skill_instance_id": "llm-skill-id",
            "user_intent": "summarize local risk"
        }),
    });

    assert!(response.ok);
    let result = response.result.expect("prepare action");
    assert_eq!(
        result.get("action").and_then(Value::as_str),
        Some("analyze")
    );
    assert_eq!(result.get("allowed").and_then(Value::as_bool), Some(false));
    assert_eq!(
        result.get("requires_confirmation").and_then(Value::as_bool),
        Some(true)
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
        .get("estimated_total_tokens")
        .and_then(Value::as_u64)
        .is_some_and(|tokens| tokens > 0));
    assert!(result
        .get("prompt_scope")
        .and_then(Value::as_array)
        .expect("prompt scope")
        .contains(&Value::String("selected skill body".to_string())));
    let serialized = serde_json::to_string(&result).expect("serialize result");
    assert!(!serialized.contains("OPENAI_API_KEY=<redacted>"));
    assert!(!serialized.contains("Analyze local skill posture"));
    assert!(!serialized.contains(&skill_path.to_string_lossy().to_string()));

    let _ = fs::remove_dir_all(app_data_dir);
}

#[test]
fn llm_prepare_missing_skill_returns_stable_error_without_creating_catalog() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-llm-missing-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = test_host(app_data_dir.clone());

    let response = host.handle(ServiceRequest {
        id: Some("llm-missing".to_string()),
        method: "llm.prepareAction".to_string(),
        params: json!({
            "kind": "draft_frontmatter",
            "skill_instance_id": "missing-skill"
        }),
    });

    assert!(!response.ok);
    let error = response.error.expect("missing skill error");
    assert_eq!(error.code, "skill_not_found");
    assert!(error.message.contains("missing-skill"));
    assert!(
        !app_data_dir.exists(),
        "missing LLM skill lookup must not create catalog or app data"
    );
}

#[test]
fn llm_prepare_action_does_not_create_credentials_config_or_catalog_writes() {
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-llm-no-write-test-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-llm-no-write-home-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    let host = ServiceHost {
        app_data_dir: app_data_dir.clone(),
        adapter_ctx: AdapterContext {
            user_home: user_home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    seed_catalog_with_llm_skill(&host, &app_data_dir.join("fixture-skill").join("SKILL.md"));
    let before_catalog = Catalog::open(&host.catalog_path()).expect("open catalog before");
    let before_records = before_catalog.list_skill_records().expect("records before");
    let before_snapshots = before_catalog
        .list_all_config_snapshots(None)
        .expect("snapshots before");

    let response = host.handle(ServiceRequest {
        id: Some("llm-no-write".to_string()),
        method: "llm.prepareAction".to_string(),
        params: json!({
            "kind": "draft_frontmatter",
            "skill_instance_id": "llm-skill-id",
            "user_intent": "draft safer metadata"
        }),
    });

    assert!(response.ok);
    assert_eq!(
        response
            .result
            .as_ref()
            .and_then(|result| result.get("write_back_allowed"))
            .and_then(Value::as_bool),
        Some(false)
    );
    let after_catalog = Catalog::open(&host.catalog_path()).expect("open catalog after");
    let after_records = after_catalog.list_skill_records().expect("records after");
    let after_snapshots = after_catalog
        .list_all_config_snapshots(None)
        .expect("snapshots after");
    assert_eq!(after_records, before_records);
    assert_eq!(after_snapshots, before_snapshots);
    assert!(!user_home.join(".claude/settings.json").exists());
    assert!(!user_home.join(".codex/config.toml").exists());
    assert!(!app_data_dir.join("llm-credentials.json").exists());
    assert!(!app_data_dir.join("llm-config.json").exists());
    let serialized = serde_json::to_string(&response.result).expect("serialize response");
    assert!(!serialized.contains("OPENAI_API_KEY=<redacted>"));
    assert!(!serialized.contains("Analyze local skill posture"));

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn app_snapshot_projects_external_codex_skill_config_without_rescan() {
    let suffix = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-codex-live-config-app-{}-{suffix}",
        std::process::id(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-codex-live-config-home-{}-{suffix}",
        std::process::id(),
    ));
    let skill_path = user_home
        .join(".agents/skills/using-superpowers")
        .join("SKILL.md");
    fs::create_dir_all(skill_path.parent().expect("skill parent")).expect("create skill dir");
    fs::write(
        &skill_path,
        "---\nname: using-superpowers\ndescription: fixture\n---\nbody\n",
    )
    .expect("write skill");
    fs::create_dir_all(&app_data_dir).expect("create app data");

    let host = ServiceHost {
        app_data_dir: app_data_dir.clone(),
        adapter_ctx: AdapterContext {
            user_home: user_home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    let instance = SkillInstance {
        id: "codex-using-superpowers".to_string(),
        agent: AgentId::Codex,
        scope: Scope::AgentGlobal,
        project_root: None,
        path: skill_path.clone(),
        display_path: skill_path.clone(),
        definition_id: "using-superpowers-definition".to_string(),
        name: "using-superpowers".to_string(),
        display_name: "using-superpowers".to_string(),
        description: "fixture".to_string(),
        version: None,
        state: SkillState::Loaded,
        enabled: true,
        frontmatter_raw: "name: using-superpowers\ndescription: fixture\n".to_string(),
        body: "body".to_string(),
        scripts: Vec::new(),
        permissions: PermissionRequest::default(),
        fingerprint: "using-superpowers-fingerprint".to_string(),
        mtime: 1,
        first_seen: 1,
        last_seen: 1,
    };
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    catalog.init().expect("init catalog");
    catalog
        .upsert_skill_instance(&instance)
        .expect("seed loaded skill");

    let config_path = user_home.join(".codex/config.toml");
    fs::create_dir_all(config_path.parent().expect("config parent")).expect("create config dir");
    fs::write(
        &config_path,
        format!(
            "[[skills.config]]\npath = '{}'\nenabled = false\n",
            skill_path.to_string_lossy()
        ),
    )
    .expect("write disabled config");

    let snapshot = host.app_state_snapshot().expect("load projected snapshot");
    let projected = snapshot
        .skills
        .iter()
        .find(|skill| skill.id == instance.id)
        .expect("projected skill");
    assert_eq!(projected.state, "disabled");
    assert!(!projected.enabled);
    assert_eq!(snapshot.health.disabled_count, 1);
    assert_eq!(snapshot.health.enabled_count, 0);

    let persisted = catalog
        .list_skill_records()
        .expect("list persisted records")
        .into_iter()
        .find(|skill| skill.id == instance.id)
        .expect("persisted skill");
    assert_eq!(persisted.state, "loaded");
    assert!(
        persisted.enabled,
        "read projection must not mutate the catalog"
    );

    let detail_response = host.handle(ServiceRequest {
        id: Some("codex-live-detail".to_string()),
        method: "catalog.getSkill".to_string(),
        params: json!({ "instance_id": instance.id }),
    });
    assert!(detail_response.ok);
    assert_eq!(
        detail_response
            .result
            .as_ref()
            .and_then(|result| result.get("state"))
            .and_then(Value::as_str),
        Some("disabled")
    );
    assert_eq!(
        detail_response
            .result
            .as_ref()
            .and_then(|result| result.get("enabled"))
            .and_then(Value::as_bool),
        Some(false)
    );

    let mut stale_disabled = instance.clone();
    stale_disabled.state = SkillState::Disabled;
    stale_disabled.enabled = false;
    catalog
        .upsert_skill_instance(&stale_disabled)
        .expect("seed stale disabled state");
    fs::write(&config_path, "# external enable removed the override\n")
        .expect("clear disabled config");

    let restored = host.app_state_snapshot().expect("load restored snapshot");
    let restored_skill = restored
        .skills
        .iter()
        .find(|skill| skill.id == instance.id)
        .expect("restored skill");
    assert_eq!(restored_skill.state, "loaded");
    assert!(restored_skill.enabled);
    assert_eq!(restored.health.disabled_count, 0);
    assert_eq!(restored.health.enabled_count, 1);

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}
