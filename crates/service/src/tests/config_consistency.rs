use super::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, Eq, PartialEq)]
enum TreeEntry {
    Directory,
    File(Vec<u8>),
    Symlink(PathBuf),
}

fn tree_snapshot(root: &Path) -> BTreeMap<PathBuf, TreeEntry> {
    fn visit(root: &Path, dir: &Path, entries: &mut BTreeMap<PathBuf, TreeEntry>) {
        if !dir.exists() {
            return;
        }
        let mut children = fs::read_dir(dir)
            .unwrap_or_else(|error| panic!("read {}: {error}", dir.display()))
            .map(|entry| entry.expect("tree entry"))
            .collect::<Vec<_>>();
        children.sort_by_key(|entry| entry.path());
        for child in children {
            let path = child.path();
            let relative = path
                .strip_prefix(root)
                .expect("relative path")
                .to_path_buf();
            let metadata = fs::symlink_metadata(&path).expect("tree metadata");
            if metadata.file_type().is_symlink() {
                entries.insert(
                    relative,
                    TreeEntry::Symlink(fs::read_link(&path).expect("read symlink")),
                );
            } else if metadata.is_dir() {
                entries.insert(relative, TreeEntry::Directory);
                visit(root, &path, entries);
            } else if metadata.is_file() {
                entries.insert(
                    relative,
                    TreeEntry::File(fs::read(&path).expect("read file")),
                );
            }
        }
    }

    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

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

fn confirmation_from_preview(response: &ServiceResponse) -> Value {
    let result = response.result.as_ref().expect("preview result");
    let action = result.get("action").expect("preview action");
    json!({
        "reference": {
            "action_id": action["id"].clone(),
            "source_revision": action["source_revision"].clone(),
            "project_id": action.get("project_id").cloned().unwrap_or(Value::Null),
            "target": action["target"].clone()
        },
        "preview_token": result["preview_token"].clone(),
        "confirmed": true
    })
}

#[test]
fn config_save_preview_returns_confirmation_bound_action_without_writes() {
    let root = std::env::temp_dir().join(format!(
        "skills-copilot-service-config-preview-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let host = config_test_host(&root);
    let settings_path = host.adapter_ctx.user_home.join(".claude/settings.json");
    fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    fs::write(&settings_path, "{}\n").expect("write initial settings");
    let read = host.handle(ServiceRequest {
        id: Some("read-before-preview".to_string()),
        method: "config.readClaudeSettings".to_string(),
        params: json!({}),
    });
    let revision = read
        .result
        .as_ref()
        .and_then(|result| result.get("revision"))
        .and_then(Value::as_str)
        .expect("read revision");
    let before = tree_snapshot(&root);

    let response = host.handle(ServiceRequest {
        id: Some("preview-config-save".to_string()),
        method: "config.previewSaveClaudeSettings".to_string(),
        params: json!({
            "content": "{\n  \"requested\": true\n}\n",
            "expected_revision": revision
        }),
    });

    assert!(response.ok, "{response:?}");
    let result = response.result.expect("preview result");
    assert_eq!(result["action"]["kind"], "save_config");
    assert_eq!(result["action"]["target"]["kind"], "config");
    assert_eq!(
        result["action"]["target"]["id"],
        settings_path.to_string_lossy().as_ref()
    );
    assert_eq!(result["action"]["network"], "none");
    assert_eq!(
        result["action"]["readback"],
        json!([
            "catalog_skills",
            "skill_aggregates",
            "agent_config",
            "config_snapshots"
        ])
    );
    assert_eq!(result["preconditions"][0]["expected_revision"], revision);
    assert!(result["preview_token"]
        .as_str()
        .is_some_and(|token| token.starts_with("action-preview:v1:hmac-sha256:")));
    assert_eq!(tree_snapshot(&root), before);
    assert!(!host.app_data_dir.exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn no_op_save_preview_is_read_only_and_replayed_confirmation_has_zero_io() {
    let root = std::env::temp_dir().join(format!(
        "skills-copilot-service-config-no-op-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let host = config_test_host(&root);
    let settings_path = host.adapter_ctx.user_home.join(".claude/settings.json");
    fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    fs::write(&settings_path, "{}\n").expect("write initial settings");
    let read = host.handle(ServiceRequest {
        id: Some("read-before-no-op-preview".to_string()),
        method: "config.readClaudeSettings".to_string(),
        params: json!({}),
    });
    let revision = read
        .result
        .as_ref()
        .and_then(|result| result.get("revision"))
        .and_then(Value::as_str)
        .expect("read revision");
    let preview = host.handle(ServiceRequest {
        id: Some("preview-no-op-save".to_string()),
        method: "config.previewSaveClaudeSettings".to_string(),
        params: json!({"content": "{}\n", "expected_revision": revision}),
    });
    assert!(preview.ok, "{preview:?}");
    let result = preview.result.as_ref().expect("preview result");
    assert_eq!(result["changed"], false);
    assert_eq!(result["action"]["apply_method"], Value::Null);
    assert_eq!(result["action"]["confirmation_required"], false);
    assert_eq!(result["action"]["impacts"], json!(["read_only"]));
    assert!(result["action"]["source_revision"]
        .as_str()
        .is_some_and(|revision| revision.starts_with("no-op:sha256:")));
    let confirmation = confirmation_from_preview(&preview);
    let before = tree_snapshot(&root);

    for attempt in 0..2 {
        let response = host.handle(ServiceRequest {
            id: Some(format!("apply-no-op-save-{attempt}")),
            method: "config.saveClaudeSettings".to_string(),
            params: json!({"content": "{}\n", "confirmation": confirmation.clone()}),
        });
        assert!(!response.ok, "{response:?}");
        assert_eq!(
            response.error.expect("no applicable action error").code,
            "no_applicable_action"
        );
        assert_eq!(tree_snapshot(&root), before);
        assert!(!host.app_data_dir.exists());
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn no_op_rollback_preview_is_read_only_and_replayed_confirmation_has_zero_io() {
    let root = std::env::temp_dir().join(format!(
        "skills-copilot-service-rollback-no-op-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let host = config_test_host(&root);
    let settings_path = host.adapter_ctx.user_home.join(".claude/settings.json");
    fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    fs::write(&settings_path, "{}\n").expect("write initial settings");
    fs::create_dir_all(&host.app_data_dir).expect("create app data");
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    catalog.init().expect("initialize catalog");
    catalog
        .create_config_snapshot(skills_copilot_catalog::ConfigSnapshotDraft {
            id: "service-no-op-rollback",
            agent: "claude-code",
            scope: "agent-global",
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: 1,
        })
        .expect("create snapshot");
    drop(catalog);
    let preview = host.handle(ServiceRequest {
        id: Some("preview-no-op-rollback".to_string()),
        method: "snapshot.previewRollback".to_string(),
        params: json!({"snapshot_id": "service-no-op-rollback"}),
    });
    assert!(preview.ok, "{preview:?}");
    let result = preview.result.as_ref().expect("preview result");
    assert_eq!(result["changed"], false);
    assert_eq!(result["action"]["apply_method"], Value::Null);
    assert_eq!(result["action"]["confirmation_required"], false);
    assert_eq!(result["action"]["impacts"], json!(["read_only"]));
    let confirmation = confirmation_from_preview(&preview);
    let before = tree_snapshot(&root);

    for attempt in 0..2 {
        let response = host.handle(ServiceRequest {
            id: Some(format!("apply-no-op-rollback-{attempt}")),
            method: "snapshot.rollback".to_string(),
            params: json!({
                "snapshot_id": "service-no-op-rollback",
                "confirmation": confirmation.clone()
            }),
        });
        assert!(!response.ok, "{response:?}");
        assert_eq!(
            response.error.expect("no applicable action error").code,
            "no_applicable_action"
        );
        assert_eq!(tree_snapshot(&root), before);
    }

    let _ = fs::remove_dir_all(root);
}

#[test]
fn rollback_preview_rejects_outdated_catalog_without_migrating_it() {
    use rusqlite::Connection;

    let root = std::env::temp_dir().join(format!(
        "skills-copilot-service-rollback-strict-read-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let host = config_test_host(&root);
    let settings_path = host.adapter_ctx.user_home.join(".claude/settings.json");
    fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    fs::write(&settings_path, "{\n  \"current\": true\n}\n").expect("write settings");
    fs::create_dir_all(&host.app_data_dir).expect("create app data");
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    catalog.init().expect("initialize catalog");
    catalog
        .create_config_snapshot(skills_copilot_catalog::ConfigSnapshotDraft {
            id: "strict-read-snapshot",
            agent: "claude-code",
            scope: "agent-global",
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: 1,
        })
        .expect("create snapshot");
    drop(catalog);
    Connection::open(host.catalog_path())
        .expect("open raw catalog")
        .pragma_update(None, "user_version", 1_i64)
        .expect("mark catalog outdated");
    let before = tree_snapshot(&root);

    let response = host.handle(ServiceRequest {
        id: Some("preview-outdated-catalog".to_string()),
        method: "snapshot.previewRollback".to_string(),
        params: json!({"snapshot_id": "strict-read-snapshot"}),
    });

    assert!(!response.ok, "{response:?}");
    assert_eq!(response.error.expect("catalog error").code, "catalog_error");
    assert_eq!(tree_snapshot(&root), before);

    let _ = fs::remove_dir_all(root);
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
    let candidate = "{\n  \"requested\": true\n}\n";
    let preview = host.handle(ServiceRequest {
        id: Some("preview-before-stale-save".to_string()),
        method: "config.previewSaveClaudeSettings".to_string(),
        params: json!({
            "content": candidate,
            "expected_revision": revision
        }),
    });
    assert!(preview.ok, "{preview:?}");
    let confirmation = confirmation_from_preview(&preview);
    let external_content = "{\n  \"external\": true\n}\n";
    fs::write(&settings_path, external_content).expect("write external change");

    let response = host.handle(ServiceRequest {
        id: Some("stale-save".to_string()),
        method: "config.saveClaudeSettings".to_string(),
        params: json!({
            "content": candidate,
            "confirmation": confirmation
        }),
    });

    assert!(
        !response.ok,
        "stale save unexpectedly succeeded: {response:?}"
    );
    assert_eq!(
        response.error.expect("stale action error").code,
        "stale_action_reference"
    );
    assert_eq!(
        fs::read_to_string(&settings_path).expect("read preserved settings"),
        external_content
    );
    assert!(
        !host.app_data_dir.exists(),
        "stale service save must not initialize app data or a catalog"
    );
    assert!(!settings_path.with_extension("lock").exists());

    let _ = fs::remove_dir_all(root);
}

#[test]
fn config_conflict_on_fresh_filesystem_creates_no_catalog_lock_parent_or_target() {
    let root = std::env::temp_dir().join(format!(
        "skills-copilot-service-config-conflict-fresh-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let host = config_test_host(&root);
    fs::create_dir_all(&host.adapter_ctx.user_home).expect("create empty home");
    let before = tree_snapshot(&root);

    let response = host.handle(ServiceRequest {
        id: Some("stale-save-fresh".to_string()),
        method: "config.previewSaveClaudeSettings".to_string(),
        params: json!({
            "content": "{}\n",
            "expected_revision": "sha256:0000000000000000000000000000000000000000000000000000000000000000"
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
    assert_eq!(tree_snapshot(&root), before);
    assert!(!host.app_data_dir.exists());
    assert!(!host.adapter_ctx.user_home.join(".claude").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn fresh_successful_save_initializes_snapshot_and_exact_catalog_projection_readback() {
    let root = std::env::temp_dir().join(format!(
        "skills-copilot-service-config-save-fresh-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let host = config_test_host(&root);
    let settings_path = host.adapter_ctx.user_home.join(".claude/settings.json");
    let skill_path = host
        .adapter_ctx
        .user_home
        .join(".claude/skills/save-fixture/SKILL.md");
    fs::create_dir_all(skill_path.parent().expect("skill parent")).expect("create skill directory");
    fs::write(&settings_path, "{}\n").expect("write initial settings");
    fs::write(
        &skill_path,
        "---\nname: save-fixture\ndescription: save fixture\n---\nbody\n",
    )
    .expect("write skill fixture");
    let read = host.handle(ServiceRequest {
        id: Some("read-before-successful-save".to_string()),
        method: "config.readClaudeSettings".to_string(),
        params: json!({}),
    });
    let revision = read
        .result
        .as_ref()
        .and_then(|result| result.get("revision"))
        .and_then(Value::as_str)
        .expect("read revision");
    let content = "{\n  \"skillOverrides\": {\n    \"save-fixture\": \"off\"\n  }\n}\n";
    let preview = host.handle(ServiceRequest {
        id: Some("preview-successful-save".to_string()),
        method: "config.previewSaveClaudeSettings".to_string(),
        params: json!({
            "content": content,
            "expected_revision": revision
        }),
    });
    assert!(preview.ok, "{preview:?}");
    let confirmation = confirmation_from_preview(&preview);

    let response = host.handle(ServiceRequest {
        id: Some("successful-save".to_string()),
        method: "config.saveClaudeSettings".to_string(),
        params: json!({
            "content": content,
            "confirmation": confirmation
        }),
    });

    assert!(response.ok, "{response:?}");
    assert!(host.catalog_path().exists());
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    assert_eq!(
        catalog
            .list_all_config_snapshots(None)
            .expect("list snapshots")
            .len(),
        1
    );
    let skills = catalog.list_skill_records().expect("list cached skills");
    assert_eq!(skills.len(), 1);
    assert_eq!(skills[0].name, "save-fixture");
    assert!(!skills[0].enabled);
    assert_eq!(
        response.result.as_ref().expect("apply result")["readback"]["verified"],
        true
    );
    assert_eq!(
        response.result.as_ref().expect("apply result")["readback"]["domains"],
        json!([
            "catalog_skills",
            "skill_aggregates",
            "agent_config",
            "config_snapshots"
        ])
    );
    assert!(fs::read_to_string(&settings_path)
        .expect("read saved settings")
        .contains("skillOverrides"));
    drop(catalog);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stale_action_reference_rejects_json_rpc_rollback_without_target_or_catalog_mutation() {
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
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: 1,
        })
        .expect("create snapshot");
    let snapshots_before = catalog
        .list_all_config_snapshots(None)
        .expect("list snapshots before");
    let skills_before = catalog.list_skill_records().expect("list skills before");
    drop(catalog);
    let preview = host.handle(ServiceRequest {
        id: Some("preview-rollback".to_string()),
        method: "snapshot.previewRollback".to_string(),
        params: json!({"snapshot_id": "service-rollback-snapshot"}),
    });
    assert!(preview.ok, "{preview:?}");
    let confirmation = confirmation_from_preview(&preview);
    let external_content = "{\n  \"externalAfterPreview\": true\n}\n";
    fs::write(&settings_path, external_content).expect("write external change");
    let catalog_bytes_before = catalog_bytes(&host.app_data_dir);

    let response = host.handle(ServiceRequest {
        id: Some("stale-rollback".to_string()),
        method: "snapshot.rollback".to_string(),
        params: json!({
            "snapshot_id": "service-rollback-snapshot",
            "confirmation": confirmation
        }),
    });

    assert!(
        !response.ok,
        "stale rollback unexpectedly succeeded: {response:?}"
    );
    assert_eq!(
        response.error.expect("stale action error").code,
        "stale_action_reference"
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
            .list_all_config_snapshots(None)
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

#[test]
fn stale_action_reference_maps_deleted_snapshot_to_json_rpc_without_writes() {
    use rusqlite::Connection;

    let root = std::env::temp_dir().join(format!(
        "skills-copilot-service-deleted-preview-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let host = config_test_host(&root);
    let settings_path = host.adapter_ctx.user_home.join(".claude/settings.json");
    fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    let current_content = "{\n  \"current\": true\n}\n";
    fs::write(&settings_path, current_content).expect("write current settings");
    fs::create_dir_all(&host.app_data_dir).expect("create app data");
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    catalog.init().expect("initialize catalog");
    catalog
        .create_config_snapshot(skills_copilot_catalog::ConfigSnapshotDraft {
            id: "deleted-service-snapshot",
            agent: "claude-code",
            scope: "agent-global",
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: 1,
        })
        .expect("create snapshot");
    drop(catalog);
    let preview = host.handle(ServiceRequest {
        id: Some("preview-before-delete".to_string()),
        method: "snapshot.previewRollback".to_string(),
        params: json!({"snapshot_id": "deleted-service-snapshot"}),
    });
    let confirmation = confirmation_from_preview(&preview);
    Connection::open(host.catalog_path())
        .expect("open raw catalog")
        .execute(
            "DELETE FROM config_snapshot WHERE id = ?1",
            ["deleted-service-snapshot"],
        )
        .expect("delete snapshot after preview");
    let catalog_before = catalog_bytes(&host.app_data_dir);

    let response = host.handle(ServiceRequest {
        id: Some("rollback-after-delete".to_string()),
        method: "snapshot.rollback".to_string(),
        params: json!({
            "snapshot_id": "deleted-service-snapshot",
            "confirmation": confirmation
        }),
    });

    assert!(
        !response.ok,
        "deleted snapshot rollback succeeded: {response:?}"
    );
    assert_eq!(
        response.error.expect("stale action error").code,
        "stale_action_reference"
    );
    assert_eq!(
        fs::read_to_string(&settings_path).expect("read unchanged target"),
        current_content
    );
    assert_eq!(catalog_bytes(&host.app_data_dir), catalog_before);
    assert!(!settings_path.with_extension("lock").exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn stale_action_reference_maps_unsafe_target_drift_without_accessing_drifted_target() {
    use rusqlite::{params, Connection};

    let root = std::env::temp_dir().join(format!(
        "skills-copilot-service-unsafe-preview-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let host = config_test_host(&root);
    let settings_path = host.adapter_ctx.user_home.join(".claude/settings.json");
    let outside_target = root.join("outside/unsafe-settings.json");
    fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    fs::create_dir_all(outside_target.parent().expect("outside parent"))
        .expect("create outside directory");
    let current_content = "{\n  \"current\": true\n}\n";
    let outside_content = "do not read or write\n";
    fs::write(&settings_path, current_content).expect("write current settings");
    fs::write(&outside_target, outside_content).expect("write outside sentinel");
    fs::create_dir_all(&host.app_data_dir).expect("create app data");
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    catalog.init().expect("initialize catalog");
    catalog
        .create_config_snapshot(skills_copilot_catalog::ConfigSnapshotDraft {
            id: "unsafe-target-service-snapshot",
            agent: "claude-code",
            scope: "agent-global",
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: 1,
        })
        .expect("create snapshot");
    drop(catalog);
    let preview = host.handle(ServiceRequest {
        id: Some("preview-before-unsafe-drift".to_string()),
        method: "snapshot.previewRollback".to_string(),
        params: json!({"snapshot_id": "unsafe-target-service-snapshot"}),
    });
    let confirmation = confirmation_from_preview(&preview);
    Connection::open(host.catalog_path())
        .expect("open raw catalog")
        .execute(
            "UPDATE config_snapshot SET target = ?1 WHERE id = ?2",
            params![
                outside_target.to_string_lossy(),
                "unsafe-target-service-snapshot"
            ],
        )
        .expect("drift snapshot target after preview");
    let catalog_before = catalog_bytes(&host.app_data_dir);

    let response = host.handle(ServiceRequest {
        id: Some("rollback-after-unsafe-drift".to_string()),
        method: "snapshot.rollback".to_string(),
        params: json!({
            "snapshot_id": "unsafe-target-service-snapshot",
            "confirmation": confirmation
        }),
    });

    assert!(
        !response.ok,
        "unsafe drift rollback succeeded: {response:?}"
    );
    assert_eq!(
        response.error.expect("target mismatch error").code,
        "action_target_mismatch"
    );
    assert_eq!(
        fs::read_to_string(&settings_path).expect("read unchanged target"),
        current_content
    );
    assert_eq!(
        fs::read_to_string(&outside_target).expect("read outside sentinel"),
        outside_content
    );
    assert_eq!(catalog_bytes(&host.app_data_dir), catalog_before);
    assert!(!settings_path.with_extension("lock").exists());
    assert!(!outside_target.with_extension("lock").exists());
    let _ = fs::remove_dir_all(root);
}
