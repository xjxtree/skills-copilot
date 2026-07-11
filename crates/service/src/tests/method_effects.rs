use super::*;
use std::collections::BTreeMap;

#[derive(Debug, Clone, PartialEq, Eq)]
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
            .unwrap_or_else(|error| panic!("read snapshot directory {}: {error}", dir.display()))
            .map(|entry| {
                entry.unwrap_or_else(|error| {
                    panic!("read snapshot entry under {}: {error}", dir.display())
                })
            })
            .collect::<Vec<_>>();
        children.sort_by_key(|entry| entry.path());
        for child in children {
            let path = child.path();
            let relative = path.strip_prefix(root).unwrap().to_path_buf();
            let metadata = fs::symlink_metadata(&path).unwrap();
            if metadata.file_type().is_symlink() {
                entries.insert(relative, TreeEntry::Symlink(fs::read_link(&path).unwrap()));
            } else if metadata.is_dir() {
                entries.insert(relative, TreeEntry::Directory);
                visit(root, &path, entries);
            } else if metadata.is_file() {
                entries.insert(relative, TreeEntry::File(fs::read(&path).unwrap()));
            }
        }
    }

    let mut entries = BTreeMap::new();
    visit(root, root, &mut entries);
    entries
}

fn temp_test_dir(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "skills-copilot-service-{label}-{}-{}",
        std::process::id(),
        unique_suffix()
    ))
}

fn assert_tree_unchanged(
    method: &str,
    before: &BTreeMap<PathBuf, TreeEntry>,
    after: &BTreeMap<PathBuf, TreeEntry>,
) {
    if before == after {
        return;
    }
    let changed = before
        .keys()
        .chain(after.keys())
        .filter(|path| before.get(*path) != after.get(*path))
        .collect::<std::collections::BTreeSet<_>>();
    panic!("read-only method {method} changed filesystem entries: {changed:?}");
}

fn read_only_methods_from_manifest() -> Vec<String> {
    let manifest_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/service-protocol/method-effects.json");
    let manifest: Value =
        serde_json::from_slice(&fs::read(manifest_path).expect("read method effects manifest"))
            .expect("decode method effects manifest");
    let methods = manifest["methods"].as_object().expect("method effects map");
    methods
        .iter()
        .filter(|(_, effect)| {
            effect["writes"].as_array().is_some_and(Vec::is_empty)
                && effect["process"] == "never"
                && effect["network"] == "never"
        })
        .map(|(method, _)| method.clone())
        .collect()
}

fn replace_fixture_paths(value: &mut Value, home: &Path, project: &Path) {
    match value {
        Value::Array(values) => {
            for value in values {
                replace_fixture_paths(value, home, project);
            }
        }
        Value::Object(values) => {
            for value in values.values_mut() {
                replace_fixture_paths(value, home, project);
            }
        }
        Value::String(text) => {
            *text = text
                .replace("$HOME", &home.to_string_lossy())
                .replace("/tmp/skills-copilot-project", &project.to_string_lossy());
        }
        _ => {}
    }
}

fn request_fixture(method: &str, home: &Path, project: &Path) -> ServiceRequest {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join("fixtures/service-protocol")
        .join(format!("{method}.request.json"));
    let mut value: Value = serde_json::from_slice(
        &fs::read(&path)
            .unwrap_or_else(|error| panic!("read request fixture {}: {error}", path.display())),
    )
    .unwrap_or_else(|error| panic!("decode request fixture {}: {error}", path.display()));
    replace_fixture_paths(&mut value, home, project);
    serde_json::from_value(value)
        .unwrap_or_else(|error| panic!("deserialize request fixture {}: {error}", path.display()))
}

fn catalog_file_snapshot(app_data_dir: &Path) -> BTreeMap<PathBuf, TreeEntry> {
    tree_snapshot(app_data_dir)
        .into_iter()
        .filter(|(path, _)| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.starts_with("catalog.sqlite"))
        })
        .collect()
}

fn seed_catalog_with_snapshot(host: &ServiceHost, skill_path: &Path, target: &Path) {
    seed_catalog_with_llm_skill(host, skill_path);
    let catalog = Catalog::open(&host.catalog_path()).expect("open seeded catalog");
    catalog.init().expect("initialize seeded catalog");
    catalog
        .create_config_snapshot(skills_copilot_catalog::ConfigSnapshotDraft {
            id: "snapshot-id",
            agent: "claude-code",
            scope: "agent-global",
            target: &target.to_string_lossy(),
            content: "{}\n",
            reason: "pre-toggle",
            created_at_ms: 1,
        })
        .expect("seed config snapshot");
    drop(catalog);
}

#[test]
fn catalog_list_skills_does_not_create_app_data_or_catalog() {
    let root = temp_test_dir("effects-list-skills");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    let host = ServiceHost {
        app_data_dir: root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        },
    };
    let before = tree_snapshot(&root);
    let response = host.handle(ServiceRequest {
        id: Some("effects-list".to_string()),
        method: "catalog.listSkills".to_string(),
        params: json!({}),
    });
    assert!(response.ok, "{response:?}");
    assert_eq!(tree_snapshot(&root), before);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn read_claude_settings_does_not_create_claude_directory() {
    let root = temp_test_dir("effects-read-settings");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    let host = ServiceHost {
        app_data_dir: root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        },
    };
    let before = tree_snapshot(&root);
    let response = host.handle(ServiceRequest {
        id: Some("effects-read-settings".to_string()),
        method: "config.readClaudeSettings".to_string(),
        params: json!({}),
    });
    assert!(response.ok, "{response:?}");
    assert_eq!(tree_snapshot(&root), before);
    assert!(!home.join(".claude").exists());
    assert!(!host.app_data_dir.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn manifest_declared_read_only_methods_leave_fresh_filesystem_unchanged() {
    for method in read_only_methods_from_manifest() {
        let root = temp_test_dir(&format!("effects-fresh-{}", method.replace('.', "-")));
        let home = root.join("home");
        let project = root.join("project");
        fs::create_dir_all(&home).unwrap();
        let host = ServiceHost {
            app_data_dir: root.join("app-data"),
            adapter_ctx: AdapterContext {
                user_home: home.clone(),
                project_root: Some(project.clone()),
                project_cwd: Some(project.clone()),
                extra_roots: vec![],
            },
        };
        let request = request_fixture(&method, &home, &project);
        let before = tree_snapshot(&root);
        let response = host.handle(request);
        if let Some(error) = &response.error {
            assert_ne!(error.code, "unknown_method", "{method} was not dispatched");
        }
        let after = tree_snapshot(&root);
        assert_tree_unchanged(&method, &before, &after);
        let _ = fs::remove_dir_all(root);
    }
}

#[test]
fn session_keyset_continuation_is_stateless_and_does_not_persist_paths() {
    let root = temp_test_dir("effects-session-cursor");
    let home = root.join("home");
    let sessions = home.join(".codex/sessions");
    fs::create_dir_all(&sessions).unwrap();
    fs::write(
        sessions.join("one.jsonl"),
        r#"{"type":"session","title":"One"}"#,
    )
    .unwrap();
    fs::write(
        sessions.join("two.jsonl"),
        r#"{"type":"session","title":"Two"}"#,
    )
    .unwrap();
    let host = ServiceHost {
        app_data_dir: root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        },
    };
    let before = tree_snapshot(&root);
    let first = host.handle(ServiceRequest {
        id: Some("effects-session-first".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "codex",
            "authorized_roots": [sessions.to_string_lossy()],
            "auto_discover": false,
            "scope": "all",
            "include_content_items": false,
            "limit": 1
        }),
    });
    assert!(first.ok, "{first:?}");
    let page = first.result.expect("first session page");
    let cursor = page["next_cursor"].as_str().expect("session cursor");
    let revision = page["source_revision"].as_str().expect("session revision");
    assert!(!cursor.contains(&sessions.to_string_lossy().to_string()));
    let second = host.handle(ServiceRequest {
        id: Some("effects-session-second".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "codex",
            "authorized_roots": [sessions.to_string_lossy()],
            "auto_discover": false,
            "scope": "all",
            "include_content_items": false,
            "limit": 1,
            "cursor": cursor,
            "source_revision": revision
        }),
    });
    assert!(second.ok, "{second:?}");
    assert_tree_unchanged(
        "session.previewLocalSessions cursor",
        &before,
        &tree_snapshot(&root),
    );
    assert!(!host.app_data_dir.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn valid_snapshot_preview_does_not_create_target_parent_or_change_catalog_bytes() {
    let root = temp_test_dir("effects-preview-rollback");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    let host = ServiceHost {
        app_data_dir: root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        },
    };
    let target = home.join(".claude/settings.json");
    seed_catalog_with_snapshot(&host, &root.join("seeded-skill/SKILL.md"), &target);
    assert!(!target.parent().unwrap().exists());

    let before_tree = tree_snapshot(&root);
    let before_catalog = catalog_file_snapshot(&host.app_data_dir);
    assert!(
        before_catalog.contains_key(Path::new("catalog.sqlite")),
        "seeded catalog snapshot must include the closed database"
    );
    let response = host.handle(ServiceRequest {
        id: Some("effects-preview-rollback".to_string()),
        method: "snapshot.previewRollback".to_string(),
        params: json!({ "snapshot_id": "snapshot-id" }),
    });
    assert!(response.ok, "{response:?}");
    let after_catalog = catalog_file_snapshot(&host.app_data_dir);
    assert_tree_unchanged(
        "snapshot.previewRollback catalog bytes or sidecars",
        &before_catalog,
        &after_catalog,
    );
    assert_tree_unchanged(
        "snapshot.previewRollback",
        &before_tree,
        &tree_snapshot(&root),
    );
    assert!(!target.parent().unwrap().exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn catalog_backed_read_handlers_leave_seeded_database_and_sidecars_unchanged() {
    let root = temp_test_dir("effects-seeded-catalog");
    let home = root.join("home");
    fs::create_dir_all(&home).unwrap();
    let host = ServiceHost {
        app_data_dir: root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        },
    };
    let target = home.join(".claude/settings.json");
    seed_catalog_with_snapshot(&host, &root.join("seeded-skill/SKILL.md"), &target);

    let requests = [
        ("app.stateSnapshot", json!({})),
        ("rules.listTuning", json!({})),
        (
            "batch.previewSkillToggles",
            json!({ "instance_ids": ["llm-skill-id"], "target_enabled": false }),
        ),
        ("catalog.listSkills", json!({})),
        ("catalog.getSkill", json!({ "instance_id": "llm-skill-id" })),
        ("catalog.analysis", json!({})),
        ("catalog.listFindings", json!({})),
        ("catalog.listFindingTriage", json!({})),
        ("catalog.listConflicts", json!({})),
        (
            "skill.listEvents",
            json!({ "instance_id": "llm-skill-id", "limit": 12 }),
        ),
        (
            "skill.listEventsPage",
            json!({ "instance_id": "llm-skill-id", "limit": 12 }),
        ),
        ("snapshot.list", json!({})),
        (
            "snapshot.listAgentConfig",
            json!({ "agent": "claude-code" }),
        ),
        (
            "snapshot.listAgentConfigPage",
            json!({ "agent": "claude-code", "limit": 12 }),
        ),
        (
            "snapshot.previewRollback",
            json!({ "snapshot_id": "snapshot-id" }),
        ),
    ];
    let declared_read_only = read_only_methods_from_manifest();

    for (method, params) in requests {
        assert!(
            declared_read_only.iter().any(|declared| declared == method),
            "{method} must stay classified as read-only in the method-effects manifest"
        );
        let before_tree = tree_snapshot(&root);
        let before_catalog = catalog_file_snapshot(&host.app_data_dir);
        assert!(
            before_catalog.contains_key(Path::new("catalog.sqlite")),
            "seeded catalog snapshot must include the closed database"
        );
        let response = host.handle(ServiceRequest {
            id: Some(format!("effects-seeded-{method}")),
            method: method.to_string(),
            params,
        });
        assert!(response.ok, "{method} failed: {response:?}");
        let after_catalog = catalog_file_snapshot(&host.app_data_dir);
        assert_tree_unchanged(
            &format!("{method} catalog bytes or SQLite sidecars"),
            &before_catalog,
            &after_catalog,
        );
        assert_tree_unchanged(method, &before_tree, &tree_snapshot(&root));
    }

    assert!(!target.parent().unwrap().exists());
    let _ = fs::remove_dir_all(root);
}
