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
        method: "config.saveClaudeSettings".to_string(),
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
fn fresh_successful_save_initializes_catalog_snapshots_writes_and_rescans() {
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

    let response = host.handle(ServiceRequest {
        id: Some("successful-save".to_string()),
        method: "config.saveClaudeSettings".to_string(),
        params: json!({
            "content": "{\n  \"skillOverrides\": {\n    \"save-fixture\": \"off\"\n  }\n}\n",
            "expected_revision": revision
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
    let skills = catalog.list_skill_records().expect("list rescanned skills");
    assert_eq!(skills.len(), 1);
    assert!(!skills[0].enabled);
    assert!(fs::read_to_string(&settings_path)
        .expect("read saved settings")
        .contains("skillOverrides"));
    drop(catalog);
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
fn stale_preview_token_maps_deleted_snapshot_to_json_rpc_without_writes() {
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
    let preview_token = preview
        .result
        .as_ref()
        .and_then(|result| result.get("preview_token"))
        .and_then(Value::as_str)
        .expect("preview token")
        .to_string();
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
            "preview_token": preview_token
        }),
    });

    assert!(
        !response.ok,
        "deleted snapshot rollback succeeded: {response:?}"
    );
    assert_eq!(
        response.error.expect("stale preview error").code,
        "stale_preview_token"
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
fn stale_preview_token_maps_unsafe_target_drift_without_accessing_drifted_target() {
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
    let preview_token = preview
        .result
        .as_ref()
        .and_then(|result| result.get("preview_token"))
        .and_then(Value::as_str)
        .expect("preview token")
        .to_string();
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
            "preview_token": preview_token
        }),
    });

    assert!(
        !response.ok,
        "unsafe drift rollback succeeded: {response:?}"
    );
    assert_eq!(
        response.error.expect("stale preview error").code,
        "stale_preview_token"
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
