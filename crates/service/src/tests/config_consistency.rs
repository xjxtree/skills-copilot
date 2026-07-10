use super::*;
use std::collections::BTreeMap;

fn config_test_host(root: &Path) -> ServiceHost {
    ServiceHost {
        app_data_dir: root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: root.join("home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    }
}

fn catalog_bytes(app_data_dir: &Path) -> BTreeMap<String, Vec<u8>> {
    if !app_data_dir.exists() {
        return BTreeMap::new();
    }
    fs::read_dir(app_data_dir)
        .expect("list app data")
        .map(|entry| entry.expect("app data entry"))
        .filter_map(|entry| {
            let name = entry.file_name().to_string_lossy().to_string();
            name.starts_with("catalog.sqlite")
                .then(|| (name, fs::read(entry.path()).expect("read catalog file")))
        })
        .collect()
}

#[test]
fn config_conflict_rejects_stale_json_rpc_save_without_snapshot_or_write() {
    let root = std::env::temp_dir().join(format!(
        "skills-copilot-service-config-conflict-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let host = config_test_host(&root);
    let settings_path = host.adapter_ctx.user_home.join(".claude/settings.json");
    fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    fs::write(&settings_path, "{}\n").expect("write initial settings");
    let read = host.handle(ServiceRequest {
        id: Some("read-before-save".to_string()),
        method: "config.readClaudeSettings".to_string(),
        params: json!({}),
    });
    assert!(read.ok, "{read:?}");
    let revision = read
        .result
        .as_ref()
        .and_then(|result| result.get("revision"))
        .and_then(Value::as_str)
        .expect("read revision")
        .to_string();
    let external_content = "{\n  \"external\": true\n}\n";
    fs::write(&settings_path, external_content).expect("write external change");

    let response = host.handle(ServiceRequest {
        id: Some("stale-save".to_string()),
        method: "config.saveClaudeSettings".to_string(),
        params: json!({
            "content": "{\n  \"requested\": true\n}\n",
            "expected_revision": revision
        }),
    });

    assert!(
        !response.ok,
        "stale save unexpectedly succeeded: {response:?}"
    );
    assert_eq!(
        response.error.expect("config conflict error").code,
        "config_conflict"
    );
    assert_eq!(
        fs::read_to_string(&settings_path).expect("read preserved settings"),
        external_content
    );
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    assert!(
        catalog
            .list_all_config_snapshots()
            .expect("list snapshots")
            .is_empty(),
        "stale service save must not create a snapshot"
    );

    let _ = fs::remove_dir_all(root);
}

#[test]
fn stale_preview_token_rejects_json_rpc_rollback_without_target_or_catalog_mutation() {
    let root = std::env::temp_dir().join(format!(
        "skills-copilot-service-stale-preview-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let host = config_test_host(&root);
    let settings_path = host.adapter_ctx.user_home.join(".claude/settings.json");
    let skill_path = host
        .adapter_ctx
        .user_home
        .join(".claude/skills/fixture/SKILL.md");
    fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    fs::write(&settings_path, "{\n  \"current\": true\n}\n").expect("write current settings");
    seed_catalog_with_llm_skill(&host, &skill_path);
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    catalog
        .create_config_snapshot(skills_copilot_catalog::ConfigSnapshotDraft {
            id: "service-rollback-snapshot",
            agent: "claude-code",
            scope: "agent-global",
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: 1,
        })
        .expect("create snapshot");
    let snapshots_before = catalog
        .list_all_config_snapshots()
        .expect("list snapshots before");
    let skills_before = catalog.list_skill_records().expect("list skills before");
    drop(catalog);
    let preview = host.handle(ServiceRequest {
        id: Some("preview-rollback".to_string()),
        method: "snapshot.previewRollback".to_string(),
        params: json!({"snapshot_id": "service-rollback-snapshot"}),
    });
    assert!(preview.ok, "{preview:?}");
    let preview_token = preview
        .result
        .as_ref()
        .and_then(|result| result.get("preview_token"))
        .and_then(Value::as_str)
        .expect("preview token")
        .to_string();
    let external_content = "{\n  \"externalAfterPreview\": true\n}\n";
    fs::write(&settings_path, external_content).expect("write external change");
    let catalog_bytes_before = catalog_bytes(&host.app_data_dir);

    let response = host.handle(ServiceRequest {
        id: Some("stale-rollback".to_string()),
        method: "snapshot.rollback".to_string(),
        params: json!({
            "snapshot_id": "service-rollback-snapshot",
            "preview_token": preview_token
        }),
    });

    assert!(
        !response.ok,
        "stale rollback unexpectedly succeeded: {response:?}"
    );
    assert_eq!(
        response.error.expect("stale preview error").code,
        "stale_preview_token"
    );
    assert_eq!(
        fs::read_to_string(&settings_path).expect("read preserved settings"),
        external_content
    );
    assert!(
        !settings_path.with_extension("lock").exists(),
        "ordinary stale-token rejection must happen before lock-file preparation"
    );
    assert_eq!(catalog_bytes(&host.app_data_dir), catalog_bytes_before);
    let catalog = Catalog::open(&host.catalog_path()).expect("reopen catalog");
    assert_eq!(
        catalog
            .list_all_config_snapshots()
            .expect("list snapshots after"),
        snapshots_before
    );
    assert_eq!(
        catalog.list_skill_records().expect("list skills after"),
        skills_before,
        "stale rollback must not rescan or mutate the catalog"
    );

    drop(catalog);
    let _ = fs::remove_dir_all(root);
}
