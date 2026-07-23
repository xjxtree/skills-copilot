use super::*;

#[test]
fn rollback_snapshot_restores_settings_and_rescans() {
    initialize_action_preview_secret_for_test([0xA5; 32])
        .expect("initialize action preview test secret");
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-rollback-{}", std::process::id()));
    let home = temp_root.join("home");
    let skill_dir = home.join(".claude/skills/foo");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: foo\ndescription: x\n---\nbody",
    )
    .expect("write skill");
    let settings_path = home.join(".claude/settings.json");
    std::fs::write(&settings_path, "{}\n").expect("write settings");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    scan_claude_to_catalog(&ctx, &catalog).expect("scan");
    let skill_id = catalog.list_skill_records().expect("records")[0].id.clone();
    toggle_skill(&catalog, &ctx, &skill_id, false).expect("toggle off");

    let snapshots = list_snapshots(&catalog, &ctx).expect("snapshots");
    assert_eq!(snapshots.len(), 1);
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, &snapshots[0].id)
        .expect("rollback preview");
    assert_eq!(preview.snapshot.content, "{}\n");
    assert!(
        preview.current_content.contains("skillOverrides"),
        "preview reads the current config before rollback"
    );
    assert!(preview.changed, "preview detects changed content");
    assert!(!preview.redacted);
    assert!(preview.rollback_supported);
    let applied = rollback_snapshot(
        &catalog,
        &ctx.user_home,
        &ctx,
        &snapshots[0].id,
        &confirmed_action(&preview.action, &preview.preview_token),
    )
    .expect("rollback");
    assert!(applied.readback.verified);
    assert_eq!(
        applied.readback.domains,
        vec![
            ActionReadbackDomain::CatalogSkills,
            ActionReadbackDomain::SkillAggregates,
            ActionReadbackDomain::AgentConfig,
        ]
    );
    assert_eq!(applied.document.content, "{}\n");

    let settings = std::fs::read_to_string(&settings_path).expect("settings");
    assert_eq!(settings, "{}\n");
    let records = catalog.list_skill_records().expect("records after refresh");
    assert!(records[0].enabled);
    assert_eq!(records[0].state, "loaded");

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn project_snapshots_are_hidden_and_not_previewable_across_projects() {
    let temp_root = temp_test_dir("project-snapshot-isolation");
    let home = temp_root.join("home");
    let project_a = temp_root.join("project-a");
    let project_b = temp_root.join("project-b");
    let target_a = project_a.join(".claude/settings.local.json");
    let target_b = project_b.join(".claude/settings.local.json");
    std::fs::create_dir_all(target_a.parent().expect("project A config parent"))
        .expect("create project A config");
    std::fs::create_dir_all(target_b.parent().expect("project B config parent"))
        .expect("create project B config");
    std::fs::write(&target_a, "{}\n").expect("write project A config");
    std::fs::write(&target_b, "{}\n").expect("write project B config");
    let canonical_a = project_a.canonicalize().expect("canonical project A");
    let canonical_b = project_b.canonicalize().expect("canonical project B");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    for (id, project_root, target) in [
        ("project-a-snapshot", &canonical_a, &target_a),
        ("project-b-snapshot", &canonical_b, &target_b),
    ] {
        let project_root_text = project_root.to_string_lossy();
        catalog
            .create_config_snapshot(ConfigSnapshotDraft {
                id,
                agent: "claude-code",
                scope: "agent-project",
                project_root: Some(&project_root_text),
                target: &target.to_string_lossy(),
                content: "{}\n",
                reason: "pre-config-edit",
                created_at_ms: current_time_ms(),
            })
            .expect("create project snapshot");
    }
    let ctx_a = AdapterContext {
        user_home: home,
        project_root: Some(project_a),
        project_cwd: None,
        extra_roots: vec![],
    };

    let visible = list_snapshots(&catalog, &ctx_a).expect("list project A snapshots");
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id, "project-a-snapshot");
    let cross_project =
        preview_snapshot_rollback_with_context(&catalog, &ctx_a, "project-b-snapshot");
    assert!(matches!(
        cross_project,
        Err(CommandError::UnsafeConfigPath(_))
    ));

    let _ = std::fs::remove_dir_all(temp_root);
}

#[test]
fn stale_rollback_preview_token_rejects_external_change_without_writes() {
    let temp_root = temp_test_dir("stale-rollback-preview");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::write(&settings_path, "{\n  \"current\": true\n}\n").expect("write current settings");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "stale-preview-snapshot",
            agent: ClaudeCodeAdapter.id().as_str(),
            scope: Scope::AgentGlobal.as_str(),
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: current_time_ms(),
        })
        .expect("create snapshot");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, "stale-preview-snapshot")
        .expect("preview rollback");
    assert!(preview.current_revision.starts_with("sha256:"));
    assert!(preview
        .preview_token
        .starts_with("action-preview:v1:hmac-sha256:"));
    let external_content = "{\n  \"external\": true\n}\n";
    std::fs::write(&settings_path, external_content).expect("write external change");
    let snapshots_before = catalog
        .list_all_config_snapshots(None)
        .expect("list snapshots before");

    let result = rollback_snapshot(
        &catalog,
        &ctx.user_home,
        &ctx,
        "stale-preview-snapshot",
        &confirmed_action(&preview.action, &preview.preview_token),
    );

    assert!(matches!(result, Err(CommandError::StaleActionReference)));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read preserved external content"),
        external_content
    );
    assert_eq!(
        catalog
            .list_all_config_snapshots(None)
            .expect("list snapshots after"),
        snapshots_before
    );
    assert!(
        std::fs::read_dir(settings_path.parent().expect("settings parent"))
            .expect("list settings directory")
            .all(|entry| !entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")),
        "stale token rejection must not create a temporary config file"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rollback_deleted_before_invocation_returns_stale_without_writes() {
    use rusqlite::Connection;

    let temp_root = temp_test_dir("rollback-deleted-before-call");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    let current_content = "{\n  \"current\": true\n}\n";
    std::fs::write(&settings_path, current_content).expect("write current settings");
    let catalog_path = temp_root.join("catalog.sqlite");
    let catalog = Catalog::open(&catalog_path).expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "deleted-before-call",
            agent: ClaudeCodeAdapter.id().as_str(),
            scope: Scope::AgentGlobal.as_str(),
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: current_time_ms(),
        })
        .expect("create snapshot");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, "deleted-before-call")
        .expect("preview rollback");
    Connection::open(&catalog_path)
        .expect("open raw catalog")
        .execute(
            "DELETE FROM config_snapshot WHERE id = ?1",
            ["deleted-before-call"],
        )
        .expect("delete snapshot after preview");

    let result = rollback_snapshot(
        &catalog,
        &ctx.user_home,
        &ctx,
        "deleted-before-call",
        &confirmed_action(&preview.action, &preview.preview_token),
    );

    assert!(matches!(result, Err(CommandError::StaleActionReference)));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read unchanged target"),
        current_content
    );
    assert!(!settings_path.with_extension("lock").exists());
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rollback_preview_to_call_snapshot_identity_drift_returns_stale_without_writes() {
    use rusqlite::{params, Connection};

    for changed_field in ["target", "scope", "agent"] {
        let temp_root = temp_test_dir(&format!("rollback-before-call-{changed_field}"));
        let home = temp_root.join("home");
        let settings_path = home.join(".claude/settings.json");
        let outside_target = temp_root.join("outside/unsafe-settings.json");
        std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
            .expect("create settings directory");
        std::fs::create_dir_all(outside_target.parent().expect("outside parent"))
            .expect("create outside directory");
        let current_content = "{\n  \"current\": true\n}\n";
        let outside_content = "do not read or write\n";
        std::fs::write(&settings_path, current_content).expect("write current settings");
        std::fs::write(&outside_target, outside_content).expect("write outside sentinel");
        let catalog_path = temp_root.join("catalog.sqlite");
        let catalog = Catalog::open(&catalog_path).expect("catalog opens");
        catalog.init().expect("catalog initializes");
        catalog
            .create_config_snapshot(ConfigSnapshotDraft {
                id: "identity-drift-before-call",
                agent: ClaudeCodeAdapter.id().as_str(),
                scope: Scope::AgentGlobal.as_str(),
                project_root: None,
                target: &settings_path.to_string_lossy(),
                content: "{}\n",
                reason: "pre-config-edit",
                created_at_ms: current_time_ms(),
            })
            .expect("create snapshot");
        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };
        let preview =
            preview_snapshot_rollback_with_context(&catalog, &ctx, "identity-drift-before-call")
                .expect("preview rollback");
        let replacement = match changed_field {
            "target" => outside_target.to_string_lossy().to_string(),
            "scope" => "tool-global".to_string(),
            "agent" => "tool-global".to_string(),
            _ => unreachable!(),
        };
        let connection = Connection::open(&catalog_path).expect("open raw catalog");
        let sql = format!("UPDATE config_snapshot SET {changed_field} = ?1 WHERE id = ?2");
        connection
            .execute(&sql, params![replacement, "identity-drift-before-call"])
            .expect("drift snapshot identity");
        drop(connection);

        let result = rollback_snapshot(
            &catalog,
            &ctx.user_home,
            &ctx,
            "identity-drift-before-call",
            &confirmed_action(&preview.action, &preview.preview_token),
        );

        assert!(
            matches!(
                result,
                Err(CommandError::MismatchedActionReference(_))
                    | Err(CommandError::StaleActionReference)
            ),
            "{changed_field} drift returned {result:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&settings_path).expect("read unchanged target"),
            current_content
        );
        assert_eq!(
            std::fs::read_to_string(&outside_target).expect("read outside sentinel"),
            outside_content
        );
        assert!(!settings_path.with_extension("lock").exists());
        assert!(!outside_target.with_extension("lock").exists());

        drop(catalog);
        let _ = std::fs::remove_dir_all(&temp_root);
    }
}

#[test]
fn rollback_rechecks_state_after_lock() {
    let temp_root = temp_test_dir("rollback-lock-recheck");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::write(&settings_path, "{\n  \"current\": true\n}\n").expect("write current settings");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "lock-recheck-snapshot",
            agent: ClaudeCodeAdapter.id().as_str(),
            scope: Scope::AgentGlobal.as_str(),
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: current_time_ms(),
        })
        .expect("create snapshot");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, "lock-recheck-snapshot")
        .expect("preview rollback");
    let external_content = "{\n  \"changedAfterLock\": true\n}\n";
    let snapshots_before = catalog
        .list_all_config_snapshots(None)
        .expect("list snapshots before");

    let result = rollback_snapshot_with_after_lock(
        &catalog,
        &ctx.user_home,
        &ctx,
        "lock-recheck-snapshot",
        &confirmed_action(&preview.action, &preview.preview_token),
        || {
            std::fs::write(&settings_path, external_content)
                .expect("write concurrent external change");
        },
    );

    assert!(matches!(result, Err(CommandError::StaleActionReference)));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read preserved external content"),
        external_content
    );
    assert_eq!(
        catalog
            .list_all_config_snapshots(None)
            .expect("list snapshots after"),
        snapshots_before
    );
    assert!(
        std::fs::read_dir(settings_path.parent().expect("settings parent"))
            .expect("list settings directory")
            .all(|entry| !entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")),
        "lock-time rejection must not create a temporary config file"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rollback_readback_failure_restores_multilevel_missing_target_tree() {
    let temp_root = temp_test_dir("rollback-readback-compensation-missing");
    let home = temp_root.join("home");
    let app_data_dir = temp_root.join("app-data");
    std::fs::create_dir_all(&home).expect("create home");
    std::fs::create_dir_all(&app_data_dir).expect("create app data");
    let settings_path = home.join(".config/opencode/opencode.json");
    let catalog = Catalog::open(&app_data_dir.join("catalog.sqlite")).expect("open catalog");
    catalog.init().expect("initialize catalog");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "missing-target-rollback",
            agent: OpencodeAdapter.id().as_str(),
            scope: Scope::AgentGlobal.as_str(),
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{\n  \"restored\": true\n}\n",
            reason: "pre-config-edit",
            created_at_ms: current_time_ms(),
        })
        .expect("create rollback snapshot");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, "missing-target-rollback")
        .expect("preview rollback");

    let result = rollback_snapshot_with_hooks(
        &catalog,
        &app_data_dir,
        &ctx,
        "missing-target-rollback",
        &confirmed_action(&preview.action, &preview.preview_token),
        || {},
        || Err(CommandError::VerificationFailed),
    );

    assert!(
        matches!(
            result,
            Err(CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: false,
                ..
            })
        ),
        "unexpected rollback read-back failure: {result:?}"
    );
    assert!(
        !settings_path.exists(),
        "rollback compensation must restore a missing target to missing"
    );
    assert!(
        !settings_path.parent().expect("settings parent").exists(),
        "rollback compensation must remove the deepest parent created solely by the failed write"
    );
    assert!(
        !home.join(".config").exists(),
        "rollback compensation must remove every newly created empty ancestor"
    );
    assert!(
        std::fs::read_dir(&home)
            .expect("list restored home tree")
            .next()
            .is_none(),
        "the user-home tree must exactly match its pre-write empty state"
    );
    assert!(
        catalog
            .get_config_snapshot("missing-target-rollback")
            .expect("read rollback snapshot")
            .is_some(),
        "the pre-existing rollback snapshot must remain available"
    );

    drop(catalog);
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rollback_maps_unreadable_target_shape_after_lock_to_stale_action() {
    let temp_root = temp_test_dir("rollback-directory-after-lock");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::write(&settings_path, "{\n  \"current\": true\n}\n").expect("write current settings");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "directory-after-lock",
            agent: ClaudeCodeAdapter.id().as_str(),
            scope: Scope::AgentGlobal.as_str(),
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: current_time_ms(),
        })
        .expect("create snapshot");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, "directory-after-lock")
        .expect("preview rollback");

    let result = rollback_snapshot_with_after_lock(
        &catalog,
        &ctx.user_home,
        &ctx,
        "directory-after-lock",
        &confirmed_action(&preview.action, &preview.preview_token),
        || {
            std::fs::remove_file(&settings_path).expect("remove target after lock");
            std::fs::create_dir(&settings_path).expect("replace target with directory");
        },
    );

    assert!(matches!(result, Err(CommandError::StaleActionReference)));
    assert!(settings_path.is_dir());
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
#[cfg(unix)]
fn rollback_revalidates_symlinked_target_after_lock_before_reading_it() {
    let temp_root = temp_test_dir("rollback-symlink-after-lock");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    let outside_target = temp_root.join("outside/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::create_dir_all(outside_target.parent().expect("outside parent"))
        .expect("create outside directory");
    let current_content = "{\n  \"current\": true\n}\n";
    std::fs::write(&settings_path, current_content).expect("write current settings");
    std::fs::write(&outside_target, current_content).expect("write identical outside sentinel");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "symlink-after-lock",
            agent: ClaudeCodeAdapter.id().as_str(),
            scope: Scope::AgentGlobal.as_str(),
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: current_time_ms(),
        })
        .expect("create snapshot");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, "symlink-after-lock")
        .expect("preview rollback");

    let result = rollback_snapshot_with_after_lock(
        &catalog,
        &ctx.user_home,
        &ctx,
        "symlink-after-lock",
        &confirmed_action(&preview.action, &preview.preview_token),
        || {
            std::fs::remove_file(&settings_path).expect("remove target after lock");
            std::os::unix::fs::symlink(&outside_target, &settings_path)
                .expect("replace target with symlink");
        },
    );

    assert!(matches!(
        result,
        Err(CommandError::MismatchedActionReference(_))
    ));
    assert_eq!(
        std::fs::read_to_string(&outside_target).expect("read outside sentinel"),
        current_content
    );
    assert!(std::fs::symlink_metadata(&settings_path)
        .expect("target metadata")
        .file_type()
        .is_symlink());
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rollback_reloaded_snapshot_target_or_content_changes_invalidate_token() {
    use rusqlite::{params, Connection};

    for changed_field in ["content", "target"] {
        let temp_root = temp_test_dir(&format!("rollback-reloaded-{changed_field}"));
        let home = temp_root.join("home");
        let settings_path = home.join(".claude/settings.json");
        std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
            .expect("create settings directory");
        let current_content = "{\n  \"current\": true\n}\n";
        std::fs::write(&settings_path, current_content).expect("write current settings");
        let catalog_path = temp_root.join("catalog.sqlite");
        let catalog = Catalog::open(&catalog_path).expect("catalog opens");
        catalog.init().expect("catalog initializes");
        catalog
            .create_config_snapshot(ConfigSnapshotDraft {
                id: "reloaded-snapshot",
                agent: ClaudeCodeAdapter.id().as_str(),
                scope: Scope::AgentGlobal.as_str(),
                project_root: None,
                target: &settings_path.to_string_lossy(),
                content: "{}\n",
                reason: "pre-config-edit",
                created_at_ms: current_time_ms(),
            })
            .expect("create snapshot");
        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };
        let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, "reloaded-snapshot")
            .expect("preview rollback");
        let replacement = if changed_field == "content" {
            "{\"catalogChanged\":true}\n".to_string()
        } else {
            temp_root
                .join("other-settings.json")
                .to_string_lossy()
                .to_string()
        };

        let result = rollback_snapshot_with_after_lock(
            &catalog,
            &temp_root,
            &ctx,
            "reloaded-snapshot",
            &confirmed_action(&preview.action, &preview.preview_token),
            || {
                let connection = Connection::open(&catalog_path).expect("open raw catalog");
                let sql = format!("UPDATE config_snapshot SET {changed_field} = ?1 WHERE id = ?2");
                connection
                    .execute(&sql, params![replacement, "reloaded-snapshot"])
                    .expect("replace snapshot field");
            },
        );

        assert!(matches!(
            result,
            Err(CommandError::StaleActionReference)
                | Err(CommandError::MismatchedActionReference(_))
        ));
        assert_eq!(
            std::fs::read_to_string(&settings_path).expect("read unchanged target"),
            current_content,
            "changed snapshot {changed_field} must be rejected before a target write"
        );
        if changed_field == "target" {
            assert!(
                !temp_root.join("other-settings.json").exists(),
                "the reloaded replacement target must not be written"
            );
        }
        assert!(
            std::fs::read_dir(settings_path.parent().expect("settings parent"))
                .expect("list settings directory")
                .all(|entry| !entry
                    .expect("directory entry")
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".tmp")),
            "changed snapshot {changed_field} must not create a temporary config file"
        );

        drop(catalog);
        let _ = std::fs::remove_dir_all(&temp_root);
    }
}

#[test]
fn read_claude_settings_returns_default_for_missing_file() {
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-read-config-{}", std::process::id()));
    let ctx = AdapterContext {
        user_home: temp_root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let doc = read_claude_settings(&ctx).expect("read missing settings");

    assert_eq!(doc.agent, "claude-code");
    assert_eq!(doc.scope, "agent-global");
    assert_eq!(doc.content, "{}\n");
    assert!(!doc.exists);

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn read_agent_config_returns_pi_documents_without_creating_missing_project_config_dir() {
    let temp_root = temp_test_dir("read-agent-config-pi");
    let home = temp_root.join("home");
    let project = temp_root.join("project");
    let global_settings = home.join(".pi/agent/settings.json");
    std::fs::create_dir_all(global_settings.parent().expect("Pi settings parent"))
        .expect("create Pi settings parent");
    std::fs::create_dir_all(&project).expect("create project");
    std::fs::write(
        &global_settings,
        "{\"skills\":{\"disabled\":[\"remote-review\"]}}\n",
    )
    .expect("write Pi settings");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: Some(project.clone()),
        project_cwd: Some(project.clone()),
        extra_roots: Vec::new(),
    };

    let documents = read_agent_config(&ctx, "pi", None).expect("read Pi config documents");

    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0].agent, "pi");
    assert_eq!(documents[0].scope, "agent-global");
    assert!(documents[0].exists);
    assert!(documents[0].content.contains("remote-review"));
    assert_eq!(documents[1].scope, "agent-project");
    assert!(!documents[1].exists);
    assert_eq!(documents[1].content, "{\"skills\":[]}\n");
    assert!(
        !project.join(".pi").exists(),
        "read-only config preview must not create missing config directories"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn read_agent_config_returns_codex_user_and_project_documents_without_enabling_project_writes() {
    let temp_root = temp_test_dir("read-agent-config-codex");
    let home = temp_root.join("home");
    let project = temp_root.join("project");
    let user_config = home.join(".codex/config.toml");
    let project_config = project.join(".codex/config.toml");
    std::fs::create_dir_all(user_config.parent().expect("Codex user config parent"))
        .expect("create Codex user config dir");
    std::fs::create_dir_all(
        project_config
            .parent()
            .expect("Codex project config parent"),
    )
    .expect("create Codex project config dir");
    std::fs::write(&user_config, "model = \"gpt-5\"\n").expect("write Codex user config");
    std::fs::write(&project_config, "approval_policy = \"never\"\n")
        .expect("write Codex project config");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: Some(project.clone()),
        project_cwd: Some(project.clone()),
        extra_roots: Vec::new(),
    };

    let documents = read_agent_config(&ctx, "codex", None).expect("read Codex config documents");

    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0].agent, "codex");
    assert_eq!(documents[0].scope, "agent-global");
    assert_eq!(documents[0].format, "toml");
    assert!(documents[0].exists);
    assert!(documents[0].content.contains("gpt-5"));
    assert_eq!(documents[1].scope, "agent-project");
    assert_eq!(
        documents[1].target,
        project_config.to_string_lossy().to_string()
    );
    assert_eq!(documents[1].format, "toml");
    assert!(documents[1].exists);
    assert!(documents[1].content.contains("approval_policy"));

    let project_only = read_agent_config(&ctx, "codex", Some("agent-project"))
        .expect("read Codex project config document");
    assert_eq!(project_only.len(), 1);
    assert_eq!(
        project_only[0].target,
        project_config.to_string_lossy().to_string()
    );
    assert!(
        expected_config_target(&ctx, AgentId::Codex, Scope::AgentProject).is_err(),
        "Codex project config remains read-only for write targets"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn stale_claude_settings_confirmation_is_rejected_without_snapshot_or_write() {
    let temp_root = temp_test_dir("stale-claude-settings-save");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::write(&settings_path, "{}\n").expect("write initial settings");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let read = read_claude_settings(&ctx).expect("read initial settings");
    let candidate = "{\n  \"requested\": true\n}\n";
    let preview =
        preview_claude_settings_save(&ctx, candidate, &read.revision).expect("preview config save");
    let external_content = "{\n  \"externallyChanged\": true\n}\n";
    std::fs::write(&settings_path, external_content).expect("write external change");

    let result = save_claude_settings(
        &catalog,
        &ctx.user_home,
        &ctx,
        candidate,
        &confirmed_action(&preview.action, &preview.preview_token),
    );

    assert!(matches!(result, Err(CommandError::StaleActionReference)));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read preserved external content"),
        external_content
    );
    assert!(
        catalog
            .list_all_config_snapshots(None)
            .expect("list snapshots")
            .is_empty(),
        "stale rejection must happen before snapshot creation"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn save_rechecks_state_after_preflight_under_lock() {
    let temp_root = temp_test_dir("save-preflight-lock-race");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::write(&settings_path, "{}\n").expect("write initial settings");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let revision = read_claude_settings(&ctx)
        .expect("read initial settings")
        .revision;
    let candidate = "{\n  \"requested\": true\n}\n";
    let preview =
        preview_claude_settings_save(&ctx, candidate, &revision).expect("preview config save");
    let confirmation = confirmed_action(&preview.action, &preview.preview_token);
    let external_content = "{\n  \"changedAfterPreflight\": true\n}\n";
    let app_data_dir = temp_root.join("app-data");
    std::fs::create_dir_all(&app_data_dir).expect("create app data");
    let catalog = Catalog::open(&app_data_dir.join("catalog.sqlite")).expect("open catalog");
    catalog.init().expect("initialize catalog");
    let prepared = prepare_claude_settings_save(&ctx, candidate, &confirmation)
        .expect("read-only config save preflight");
    let result = commit_prepared_claude_settings_save_with_after_lock(
        &catalog,
        &app_data_dir,
        prepared,
        || {
            std::fs::write(&settings_path, external_content)
                .expect("write concurrent external change");
        },
    );

    assert!(matches!(result, Err(CommandError::StaleActionReference)));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read preserved external content"),
        external_content
    );
    assert!(
        std::fs::read_dir(settings_path.parent().expect("settings parent"))
            .expect("list settings directory")
            .all(|entry| !entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")),
        "lock-time conflict must not prepare a temporary config file"
    );
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[cfg(unix)]
#[test]
fn save_rejects_catalog_bound_to_a_different_mutation_owner_before_effects() {
    let temp_root = temp_test_dir("save-catalog-owner-mismatch");
    let home = temp_root.join("home");
    let app_data_dir = temp_root.join("app-data");
    let accepted_owner = temp_root.join("accepted-owner");
    let replacement_owner = temp_root.join("replacement-owner");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::create_dir_all(&app_data_dir).expect("create app data");
    std::fs::write(&settings_path, "{}\n").expect("write initial settings");
    let catalog = Catalog::open_anchored(
        std::fs::File::open(&app_data_dir).expect("open accepted catalog owner"),
    )
    .expect("open anchored catalog");
    catalog.init().expect("initialize catalog");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let current = read_claude_settings(&ctx).expect("read initial settings");
    let candidate = "{\n  \"requested\": true\n}\n";
    let preview = preview_claude_settings_save(&ctx, candidate, &current.revision)
        .expect("preview config save");
    let prepared = prepare_claude_settings_save(
        &ctx,
        candidate,
        &confirmed_action(&preview.action, &preview.preview_token),
    )
    .expect("prepare config save");

    std::fs::rename(&app_data_dir, &accepted_owner).expect("move accepted owner");
    std::fs::create_dir(&replacement_owner).expect("create replacement owner");
    std::fs::rename(&replacement_owner, &app_data_dir).expect("bind replacement owner");
    std::fs::write(app_data_dir.join("sentinel"), b"unchanged").expect("seed replacement owner");

    let result = commit_prepared_claude_settings_save(&catalog, &app_data_dir, prepared);

    assert!(
        matches!(
            result,
            Err(CommandError::Catalog(
                skills_copilot_catalog::CatalogError::MutationOwner(_)
            ))
        ),
        "unexpected owner mismatch result: {result:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read unchanged settings"),
        "{}\n"
    );
    assert_eq!(
        std::fs::read(app_data_dir.join("sentinel")).expect("read replacement sentinel"),
        b"unchanged"
    );
    assert!(
        catalog
            .list_all_config_snapshots(None)
            .expect("list snapshots")
            .is_empty(),
        "owner mismatch must be rejected before the catalog transaction"
    );

    drop(catalog);
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[cfg(unix)]
#[test]
fn save_reports_partial_effect_if_owner_path_changes_after_file_write() {
    use std::os::unix::fs::symlink;

    let temp_root = temp_test_dir("save-owner-rebind-after-write");
    let home = temp_root.join("home");
    let app_data_dir = temp_root.join("app-data");
    let accepted_owner = temp_root.join("accepted-owner");
    let victim = temp_root.join("victim");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::create_dir_all(&app_data_dir).expect("create app data");
    std::fs::create_dir_all(&victim).expect("create victim");
    std::fs::write(&settings_path, "{}\n").expect("write initial settings");
    std::fs::write(victim.join("sentinel"), b"unchanged").expect("seed victim");
    let catalog = Catalog::open_anchored(
        std::fs::File::open(&app_data_dir).expect("open accepted catalog owner"),
    )
    .expect("open anchored catalog");
    catalog.init().expect("initialize catalog");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let current = read_claude_settings(&ctx).expect("read initial settings");
    let candidate = "{\n  \"requested\": true\n}\n";
    let preview = preview_claude_settings_save(&ctx, candidate, &current.revision)
        .expect("preview config save");
    let prepared = prepare_claude_settings_save(
        &ctx,
        candidate,
        &confirmed_action(&preview.action, &preview.preview_token),
    )
    .expect("prepare config save");
    let hook_app_data = app_data_dir.clone();
    let hook_accepted = accepted_owner.clone();
    let hook_victim = victim.clone();

    let result = commit_prepared_claude_settings_save_with_hooks(
        &catalog,
        &app_data_dir,
        prepared,
        || {},
        move || {
            std::fs::rename(&hook_app_data, &hook_accepted).expect("move accepted owner");
            symlink(&hook_victim, &hook_app_data).expect("replace owner path");
            Ok(())
        },
    );

    assert!(
        matches!(
            result,
            Err(CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: false,
                ..
            })
        ),
        "unexpected save read-back failure: {result:?}"
    );
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read applied settings"),
        candidate
    );
    assert_eq!(
        std::fs::read(victim.join("sentinel")).expect("read victim sentinel"),
        b"unchanged"
    );
    assert!(!victim.join("catalog.sqlite").exists());

    drop(catalog);
    let _ = std::fs::remove_file(&app_data_dir);
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn save_readback_failure_rolls_back_snapshot_and_restores_missing_target() {
    let temp_root = temp_test_dir("save-readback-compensation-missing");
    let home = temp_root.join("home");
    let app_data_dir = temp_root.join("app-data");
    std::fs::create_dir_all(&home).expect("create home");
    std::fs::create_dir_all(&app_data_dir).expect("create app data");
    let catalog = Catalog::open(&app_data_dir.join("catalog.sqlite")).expect("open catalog");
    catalog.init().expect("initialize catalog");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let settings_path = home.join(".claude/settings.json");
    let current = read_claude_settings(&ctx).expect("read missing config state");
    let candidate = "{\n  \"requested\": true\n}\n";
    let preview = preview_claude_settings_save(&ctx, candidate, &current.revision)
        .expect("preview missing config save");
    let prepared = prepare_claude_settings_save(
        &ctx,
        candidate,
        &confirmed_action(&preview.action, &preview.preview_token),
    )
    .expect("prepare config save");

    let result = commit_prepared_claude_settings_save_with_hooks(
        &catalog,
        &app_data_dir,
        prepared,
        || {},
        || Err(CommandError::VerificationFailed),
    );

    assert!(matches!(
        result,
        Err(CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: false,
            ..
        })
    ));
    assert!(
        !settings_path.exists(),
        "compensation must restore an originally missing config target to missing"
    );
    assert!(
        !settings_path.parent().expect("settings parent").exists(),
        "compensation must remove the config parent created solely by the failed write"
    );
    assert!(
        std::fs::read_dir(&home)
            .expect("list restored home tree")
            .next()
            .is_none(),
        "the user-home tree must exactly match its pre-write empty state"
    );
    assert!(
        catalog
            .list_all_config_snapshots(None)
            .expect("list snapshots after rollback")
            .is_empty(),
        "the immediate catalog transaction must roll back the safety snapshot"
    );
    assert!(
        !settings_path.with_extension("lock").exists(),
        "a failed save must not leave a target lock artifact"
    );

    drop(catalog);
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn save_third_state_after_write_is_not_overwritten_by_compensation() {
    let temp_root = temp_test_dir("save-compensation-partial-effect");
    let home = temp_root.join("home");
    let app_data_dir = temp_root.join("app-data");
    std::fs::create_dir_all(&home).expect("create home");
    std::fs::create_dir_all(&app_data_dir).expect("create app data");
    let catalog = Catalog::open(&app_data_dir.join("catalog.sqlite")).expect("open catalog");
    catalog.init().expect("initialize catalog");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let settings_path = home.join(".claude/settings.json");
    let current = read_claude_settings(&ctx).expect("read missing config state");
    let candidate = "{\n  \"requested\": true\n}\n";
    let preview = preview_claude_settings_save(&ctx, candidate, &current.revision)
        .expect("preview missing config save");
    let prepared = prepare_claude_settings_save(
        &ctx,
        candidate,
        &confirmed_action(&preview.action, &preview.preview_token),
    )
    .expect("prepare config save");

    let result = commit_prepared_claude_settings_save_with_hooks(
        &catalog,
        &app_data_dir,
        prepared,
        || {},
        || {
            std::fs::remove_file(&settings_path).expect("remove written config");
            std::fs::create_dir(&settings_path).expect("inject uncompensatable target shape");
            Ok(())
        },
    );

    assert!(matches!(
        result,
        Err(CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: true,
            ..
        })
    ));
    assert!(
        settings_path.is_dir(),
        "a third state observed after the write must be preserved for user inspection"
    );
    assert!(
        catalog
            .list_all_config_snapshots(None)
            .expect("list snapshots after rollback")
            .is_empty(),
        "the catalog transaction must still roll back when file compensation fails"
    );

    drop(catalog);
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn save_post_rename_error_is_never_reclassified_as_success() {
    let temp_root = temp_test_dir("save-post-rename-error");
    let home = temp_root.join("home");
    let app_data_dir = temp_root.join("app-data");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::create_dir_all(&app_data_dir).expect("create app data");
    let original = "{}\n";
    let candidate = "{\n  \"requested\": true\n}\n";
    std::fs::write(&settings_path, original).expect("write original settings");
    let catalog = Catalog::open(&app_data_dir.join("catalog.sqlite")).expect("open catalog");
    catalog.init().expect("initialize catalog");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let current = read_claude_settings(&ctx).expect("read settings");
    let preview =
        preview_claude_settings_save(&ctx, candidate, &current.revision).expect("preview save");
    let prepared = prepare_claude_settings_save(
        &ctx,
        candidate,
        &confirmed_action(&preview.action, &preview.preview_token),
    )
    .expect("prepare save");
    install_atomic_post_rename_test_hook(settings_path.clone(), |_| {});

    let result = commit_prepared_claude_settings_save(&catalog, &app_data_dir, prepared);

    assert!(matches!(
        result,
        Err(CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: false,
            ..
        })
    ));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read restored settings"),
        original
    );
    assert!(catalog
        .list_all_config_snapshots(None)
        .expect("list snapshots")
        .is_empty());
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rollback_post_rename_error_is_never_reclassified_as_success() {
    let temp_root = temp_test_dir("rollback-post-rename-error");
    let home = temp_root.join("home");
    let app_data_dir = temp_root.join("app-data");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::create_dir_all(&app_data_dir).expect("create app data");
    let original = "{\n  \"current\": true\n}\n";
    std::fs::write(&settings_path, original).expect("write original settings");
    let catalog = Catalog::open(&app_data_dir.join("catalog.sqlite")).expect("open catalog");
    catalog.init().expect("initialize catalog");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "rollback-post-rename-error",
            agent: "claude-code",
            scope: "agent-global",
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: current_time_ms(),
        })
        .expect("create snapshot");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview =
        preview_snapshot_rollback_with_context(&catalog, &ctx, "rollback-post-rename-error")
            .expect("preview rollback");
    install_atomic_post_rename_test_hook(settings_path.clone(), |_| {});

    let result = rollback_snapshot(
        &catalog,
        &app_data_dir,
        &ctx,
        "rollback-post-rename-error",
        &confirmed_action(&preview.action, &preview.preview_token),
    );

    assert!(matches!(
        result,
        Err(CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: false,
            ..
        })
    ));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read restored settings"),
        original
    );
    assert!(catalog
        .get_config_snapshot("rollback-post-rename-error")
        .expect("read snapshot")
        .is_some());
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn config_rollback_failure_is_structured_partial_effect_and_preserves_candidate() {
    let temp_root = temp_test_dir("config-rollback-outcome-unknown");
    let home = temp_root.join("home");
    let app_data_dir = temp_root.join("app-data");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::create_dir_all(&app_data_dir).expect("create app data");
    std::fs::write(&settings_path, "{}\n").expect("write original settings");
    let catalog = Catalog::open(&app_data_dir.join("catalog.sqlite")).expect("open catalog");
    catalog.init().expect("initialize catalog");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let candidate = "{\n  \"requested\": true\n}\n";
    let current = read_claude_settings(&ctx).expect("read settings");
    let preview =
        preview_claude_settings_save(&ctx, candidate, &current.revision).expect("preview save");
    let prepared = prepare_claude_settings_save(
        &ctx,
        candidate,
        &confirmed_action(&preview.action, &preview.preview_token),
    )
    .expect("prepare save");
    catalog.inject_next_rollback_failure_for_test();

    let result = commit_prepared_claude_settings_save_with_hooks(
        &catalog,
        &app_data_dir,
        prepared,
        || {},
        || Err(CommandError::VerificationFailed),
    );

    assert!(matches!(
        result,
        Err(CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: true,
            ..
        })
    ));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read preserved candidate"),
        candidate
    );
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn config_commit_outcome_unknown_is_structured_partial_effect() {
    let temp_root = temp_test_dir("config-commit-outcome-unknown");
    let home = temp_root.join("home");
    let app_data_dir = temp_root.join("app-data");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::create_dir_all(&app_data_dir).expect("create app data");
    std::fs::write(&settings_path, "{}\n").expect("write original settings");
    let catalog = Catalog::open(&app_data_dir.join("catalog.sqlite")).expect("open catalog");
    catalog.init().expect("initialize catalog");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let candidate = "{\n  \"requested\": true\n}\n";
    let current = read_claude_settings(&ctx).expect("read settings");
    let preview =
        preview_claude_settings_save(&ctx, candidate, &current.revision).expect("preview save");
    let prepared = prepare_claude_settings_save(
        &ctx,
        candidate,
        &confirmed_action(&preview.action, &preview.preview_token),
    )
    .expect("prepare save");
    catalog.inject_next_commit_outcome_unknown_for_test();

    let result = commit_prepared_claude_settings_save(&catalog, &app_data_dir, prepared);

    assert!(matches!(
        result,
        Err(CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: true,
            ..
        })
    ));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read preserved candidate"),
        candidate
    );
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn config_precommit_failure_rolls_back_and_restores_original() {
    let temp_root = temp_test_dir("config-precommit-failure");
    let home = temp_root.join("home");
    let app_data_dir = temp_root.join("app-data");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::create_dir_all(&app_data_dir).expect("create app data");
    let original = "{}\n";
    std::fs::write(&settings_path, original).expect("write original settings");
    let catalog = Catalog::open(&app_data_dir.join("catalog.sqlite")).expect("open catalog");
    catalog.init().expect("initialize catalog");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let candidate = "{\n  \"requested\": true\n}\n";
    let current = read_claude_settings(&ctx).expect("read settings");
    let preview =
        preview_claude_settings_save(&ctx, candidate, &current.revision).expect("preview save");
    let prepared = prepare_claude_settings_save(
        &ctx,
        candidate,
        &confirmed_action(&preview.action, &preview.preview_token),
    )
    .expect("prepare save");
    catalog.inject_next_commit_failure_for_test();

    let result = commit_prepared_claude_settings_save(&catalog, &app_data_dir, prepared);

    assert!(matches!(
        result,
        Err(CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: false,
            ..
        })
    ));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read restored original"),
        original
    );
    assert!(catalog
        .list_all_config_snapshots(None)
        .expect("list snapshots")
        .is_empty());
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
#[cfg(unix)]
fn read_claude_settings_rejects_symlinked_config_directory() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-read-symlink-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let outside = temp_root.join("outside");
    std::fs::create_dir_all(&home).expect("create home");
    std::fs::create_dir_all(&outside).expect("create outside dir");
    std::os::unix::fs::symlink(&outside, home.join(".claude")).expect("create config dir symlink");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let result = read_claude_settings(&ctx);

    assert!(
        matches!(result, Err(CommandError::UnsafeConfigPath(_))),
        "read must reject the same symlinked target shape as writes"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn save_claude_settings_snapshots_validates_and_rescans() {
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-save-config-{}", std::process::id()));
    let home = temp_root.join("home");
    let skill_dir = home.join(".claude/skills/config-editor");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: config-editor\ndescription: config editor fixture\n---\nbody",
    )
    .expect("write skill");
    let settings_path = home.join(".claude/settings.json");
    std::fs::write(&settings_path, "{}\n").expect("write initial settings");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    scan_claude_to_catalog(&ctx, &catalog).expect("initial scan");

    let initial_revision = read_claude_settings(&ctx)
        .expect("read initial settings")
        .revision;
    let invalid = preview_claude_settings_save(&ctx, "{ broken", &initial_revision);
    assert!(matches!(invalid, Err(CommandError::InvalidJson(_))));

    let updated = save_claude_settings_after_preview(
        &catalog,
        &ctx,
        "{\n  \"skillOverrides\": {\n    \"config-editor\": \"off\"\n  }\n}\n",
    )
    .expect("save config");

    assert!(updated.document.exists);
    assert!(updated.document.content.contains("skillOverrides"));
    assert!(updated.readback.verified);
    assert_eq!(
        updated.readback.domains,
        vec![
            ActionReadbackDomain::CatalogSkills,
            ActionReadbackDomain::SkillAggregates,
            ActionReadbackDomain::AgentConfig,
            ActionReadbackDomain::ConfigSnapshots,
        ]
    );
    let snapshots = catalog
        .list_config_snapshots("claude-code", &settings_path.to_string_lossy())
        .expect("snapshots");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].reason, "pre-config-edit");
    assert_eq!(snapshots[0].content, "{}\n");

    let records = catalog.list_skill_records().expect("records");
    assert_eq!(records.len(), 1);
    assert!(!records[0].enabled);
    assert_eq!(records[0].state, "disabled");

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn install_preview_from_tool_global_does_not_write_disk() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-install-preview-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&home, "portable-alpha");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-alpha",
            source_path.clone(),
            "portable-alpha",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let preview = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-alpha",
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        None,
    )
    .expect("preview install");

    assert!(!preview.wrote);
    assert_path_text_eq(&preview.source_path, &source_path);
    assert_eq!(
        preview.target_path,
        path_text(
            &home
                .join(".agents")
                .join("skills")
                .join("portable-alpha")
                .join("SKILL.md")
        )
    );
    assert!(
        !home.join(".agents").exists(),
        "preview must not create target dirs"
    );
    assert!(
        catalog
            .list_all_config_snapshots(None)
            .expect("snapshots")
            .is_empty(),
        "preview must not create audit snapshots"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn confirmed_install_rejects_source_drift_after_preview_without_writing_target() {
    let temp_root = temp_test_dir("install-source-drift");
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&home, "portable-drift");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-drift",
            source_path.clone(),
            "portable-drift",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let preview = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-drift",
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        None,
    )
    .expect("preview install");
    std::fs::write(
        &source_path,
        "---\nname: portable-drift\ndescription: externally changed\n---\nchanged",
    )
    .expect("change source after preview");

    let confirmation =
        ActionConfirmation::confirmed(&preview.action, preview.preview_token.clone());
    let result = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-drift",
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        Some(&confirmation),
    );

    assert!(
        matches!(result, Err(CommandError::StaleActionReference)),
        "source drift after preview must invalidate install"
    );
    assert!(
        !Path::new(&preview.target_path).exists(),
        "a stale install must not create its target"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
#[cfg(unix)]
fn confirmed_install_rejects_same_length_source_mutation_during_locked_read() {
    let temp_root = temp_test_dir("install-source-read-race");
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&home, "portable-read-race");
    let original = std::fs::read(&source_path).expect("source bytes");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let instance_id = "tool-global-read-race";
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            instance_id,
            source_path.clone(),
            "portable-read-race",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = install_skill_from_tool_global(
        &catalog,
        &ctx,
        instance_id,
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        None,
    )
    .expect("preview install");
    let source_relative = PathBuf::from("tool-global")
        .join("skills")
        .join("portable-read-race")
        .join("SKILL.md");
    let raced_source = source_path.clone();
    let mut attacker = original.clone();
    *attacker.last_mut().expect("non-empty source") ^= 1;
    crate::app_data_owner_fs::install_owner_read_test_hook(source_relative, move || {
        std::fs::write(&raced_source, &attacker).expect("same-length source mutation");
    });

    let result = install_skill_from_tool_global(
        &catalog,
        &ctx,
        instance_id,
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        Some(&confirmed_action(&preview.action, &preview.preview_token)),
    );

    assert!(matches!(result, Err(CommandError::StaleActionReference)));
    assert!(
        !Path::new(&preview.target_path).exists(),
        "a raced source read must remain zero-write at the target"
    );
    assert_eq!(
        std::fs::metadata(&source_path)
            .expect("source metadata")
            .len(),
        original.len() as u64
    );
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
#[cfg(unix)]
fn confirmed_install_rejects_scan_window_target_replacement() {
    let temp_root = temp_test_dir("install-scan-window-race");
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&home, "portable-scan-race");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let instance_id = "tool-global-scan-race";
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            instance_id,
            source_path,
            "portable-scan-race",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = install_skill_from_tool_global(
        &catalog,
        &ctx,
        instance_id,
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        None,
    )
    .expect("preview install");
    let target = PathBuf::from(&preview.target_path);
    let raced_target = target.clone();
    crate::skill_install_guard::install_catalog_scan_test_hook(target.clone(), move || {
        let replacement = raced_target.with_extension("attacker");
        std::fs::write(
            &replacement,
            "---\nname: portable-scan-race\ndescription: attacker\n---\nattacker",
        )
        .expect("attacker replacement");
        std::fs::rename(&replacement, &raced_target).expect("replace scanned target");
    });

    let error = install_skill_from_tool_global(
        &catalog,
        &ctx,
        instance_id,
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        Some(&confirmed_action(&preview.action, &preview.preview_token)),
    )
    .expect_err("path-only scan match must not prove the install");

    assert!(matches!(
        error,
        CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: true,
            ..
        }
    ));
    assert!(
        std::fs::read_to_string(&target)
            .expect("third-party target remains")
            .contains("attacker"),
        "compensation must not overwrite the unowned third state"
    );
    assert!(
        catalog
            .list_skill_records()
            .expect("catalog rows")
            .iter()
            .all(|record| {
                normalize_path_lexically(&record.path) != normalize_path_lexically(&target)
            }),
        "the scan transaction must not retain a path-only acceptance row"
    );
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn confirmed_install_rechecks_the_source_catalog_record_under_lock() {
    let temp_root = temp_test_dir("install-catalog-drift");
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&home, "portable-catalog-drift");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let instance_id = "tool-global-catalog-drift";
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            instance_id,
            source_path,
            "portable-catalog-drift",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = install_skill_from_tool_global(
        &catalog,
        &ctx,
        instance_id,
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        None,
    )
    .expect("preview install");
    let confirmation =
        ActionConfirmation::confirmed(&preview.action, preview.preview_token.clone());

    let result = install_skill_from_tool_global_with_after_confirmation(
        &catalog,
        &ctx,
        instance_id,
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        Some(&confirmation),
        || {
            catalog
                .set_skill_toggle(instance_id, false, "disabled")
                .expect("inject source catalog drift");
        },
    );

    assert!(matches!(result, Err(CommandError::StaleActionReference)));
    assert!(
        !Path::new(&preview.target_path).exists(),
        "catalog drift after confirmation must be rejected before target write"
    );
    assert!(
        catalog
            .list_all_config_snapshots(None)
            .expect("snapshots")
            .is_empty(),
        "rejected install must not create audit snapshots"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[cfg(unix)]
#[test]
fn confirmed_install_rejects_a_catalog_bound_to_another_app_data_owner() {
    let temp_root = temp_test_dir("install-catalog-owner-mismatch");
    let home = temp_root.join("home");
    let app_data = temp_root.join("app-data");
    let accepted_owner = temp_root.join("accepted-owner");
    let replacement_owner = temp_root.join("replacement-owner");
    std::fs::create_dir_all(&home).expect("create home");
    std::fs::create_dir_all(&app_data).expect("create app data");
    let source_path = write_tool_global_skill(&app_data, "portable-owner-mismatch");
    let catalog = Catalog::open_anchored(
        std::fs::File::open(&app_data).expect("open accepted catalog owner"),
    )
    .expect("open anchored catalog");
    catalog.init().expect("initialize catalog");
    let instance_id = "tool-global-owner-mismatch";
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            instance_id,
            source_path,
            "portable-owner-mismatch",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = install_skill_from_tool_global_guarded(
        &catalog,
        &app_data,
        &ctx,
        instance_id,
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        None,
    )
    .expect("preview install");
    let confirmation = confirmed_action(&preview.action, &preview.preview_token);
    let hook_app_data = app_data.clone();
    let hook_accepted = accepted_owner.clone();
    let hook_replacement = replacement_owner.clone();

    let result = install_skill_from_tool_global_guarded_with_hooks(
        &catalog,
        &app_data,
        &ctx,
        instance_id,
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        Some(&confirmation),
        move || {
            std::fs::rename(&hook_app_data, &hook_accepted).expect("move accepted owner");
            std::fs::create_dir(&hook_replacement).expect("create replacement owner");
            std::fs::rename(&hook_replacement, &hook_app_data).expect("bind replacement owner");
            std::fs::write(hook_app_data.join("sentinel"), b"unchanged")
                .expect("seed replacement owner");
        },
        || {},
    );

    assert!(
        matches!(
            result,
            Err(CommandError::Catalog(
                skills_copilot_catalog::CatalogError::MutationOwner(_)
            ))
        ),
        "unexpected install owner mismatch result: {result:?}"
    );
    assert!(
        !Path::new(&preview.target_path).exists(),
        "owner mismatch must be rejected before writing the agent skill"
    );
    assert_eq!(
        std::fs::read(app_data.join("sentinel")).expect("read replacement sentinel"),
        b"unchanged"
    );

    drop(catalog);
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[cfg(unix)]
#[test]
fn confirmed_install_reports_partial_effect_if_owner_rebinds_after_commit() {
    use std::os::unix::fs::symlink;

    let temp_root = temp_test_dir("install-owner-rebind-after-commit");
    let home = temp_root.join("home");
    let app_data = temp_root.join("app-data");
    let accepted_owner = temp_root.join("accepted-owner");
    let victim = temp_root.join("victim");
    std::fs::create_dir_all(&home).expect("create home");
    std::fs::create_dir_all(&app_data).expect("create app data");
    std::fs::create_dir_all(&victim).expect("create victim");
    std::fs::write(victim.join("sentinel"), b"unchanged").expect("seed victim");
    let source_path = write_tool_global_skill(&app_data, "portable-owner-rebind");
    let catalog = Catalog::open_anchored(
        std::fs::File::open(&app_data).expect("open accepted catalog owner"),
    )
    .expect("open anchored catalog");
    catalog.init().expect("initialize catalog");
    let instance_id = "tool-global-owner-rebind";
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            instance_id,
            source_path,
            "portable-owner-rebind",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = install_skill_from_tool_global_guarded(
        &catalog,
        &app_data,
        &ctx,
        instance_id,
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        None,
    )
    .expect("preview install");
    let confirmation = confirmed_action(&preview.action, &preview.preview_token);
    let hook_app_data = app_data.clone();
    let hook_accepted = accepted_owner.clone();
    let hook_victim = victim.clone();

    let result = install_skill_from_tool_global_guarded_with_hooks(
        &catalog,
        &app_data,
        &ctx,
        instance_id,
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        Some(&confirmation),
        || {},
        move || {
            std::fs::rename(&hook_app_data, &hook_accepted).expect("move accepted owner");
            symlink(&hook_victim, &hook_app_data).expect("replace owner path");
        },
    );

    assert!(matches!(
        result,
        Err(CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: false,
            ..
        })
    ));
    assert!(
        Path::new(&preview.target_path).is_file(),
        "the committed and verified agent skill remains applied"
    );
    assert_eq!(
        std::fs::read(victim.join("sentinel")).expect("read victim sentinel"),
        b"unchanged"
    );
    assert!(!victim.join("catalog.sqlite").exists());

    drop(catalog);
    std::fs::remove_file(&app_data).expect("remove replacement symlink");
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn local_delete_rejects_source_drift_after_preview_without_deleting() {
    let temp_root = temp_test_dir("local-delete-source-drift");
    let app_data = temp_root.join("app-data");
    let source_path = tool_global_staging_skills_root(&app_data)
        .join("delete-drift")
        .join("SKILL.md");
    std::fs::create_dir_all(source_path.parent().expect("source parent")).expect("create source");
    std::fs::write(
        &source_path,
        "---\nname: delete-drift\ndescription: before\n---\nbefore",
    )
    .expect("write source");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let mut local_instance = install_tool_global_instance(
        "tool-global-delete-drift",
        source_path.clone(),
        "delete-drift",
    );
    local_instance.agent = AgentId::ToolGlobal;
    catalog
        .upsert_skill_instance(&local_instance)
        .expect("upsert tool-global");

    let preview = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: "tool-global-delete-drift".to_string(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect("delete preview");
    assert!(preview.physical_delete_allowed);
    std::fs::write(
        &source_path,
        "---\nname: delete-drift\ndescription: after\n---\nafter",
    )
    .expect("change source after preview");

    let result = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: "tool-global-delete-drift".to_string(),
            confirmed: true,
            preview_token: preview.preview_token.clone(),
            action_reference: preview.action.as_ref().map(ActionReference::from),
        },
    );

    assert!(
        matches!(result, Err(CommandError::StaleActionReference)),
        "source drift after preview must invalidate delete"
    );
    assert!(
        source_path.exists(),
        "a stale delete must preserve the source directory"
    );
    assert!(
        catalog
            .get_skill_record("tool-global-delete-drift")
            .expect("catalog lookup")
            .is_some(),
        "a stale delete must preserve the catalog row"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn local_delete_rechecks_the_whole_tree_under_target_lock_before_rename() {
    let temp_root = temp_test_dir("local-delete-pre-rename-drift");
    let app_data = temp_root.join("app-data");
    let source_path = tool_global_staging_skills_root(&app_data)
        .join("delete-race")
        .join("SKILL.md");
    std::fs::create_dir_all(source_path.parent().expect("source parent")).expect("create source");
    std::fs::write(
        &source_path,
        "---\nname: delete-race\ndescription: before\n---\nbefore",
    )
    .expect("write source");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let mut local_instance = install_tool_global_instance(
        "tool-global-delete-race",
        source_path.clone(),
        "delete-race",
    );
    local_instance.agent = AgentId::ToolGlobal;
    catalog
        .upsert_skill_instance(&local_instance)
        .expect("upsert tool-global");

    let preview = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: "tool-global-delete-race".to_string(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect("delete preview");
    let canonical_source = source_path.canonicalize().expect("canonical source");
    let raced_asset = source_path
        .parent()
        .expect("source parent")
        .join("added-after-confirmation.txt");
    let hook_asset = raced_asset.clone();
    skill_manager::install_local_delete_pre_rename_test_hook(canonical_source, move || {
        std::fs::write(&hook_asset, "not previewed").expect("inject raced asset");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(&hook_asset, std::fs::Permissions::from_mode(0o600))
                .expect("private raced asset");
        }
    });

    let result = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: "tool-global-delete-race".to_string(),
            confirmed: true,
            preview_token: preview.preview_token.clone(),
            action_reference: preview.action.as_ref().map(ActionReference::from),
        },
    );

    assert!(
        matches!(result, Err(CommandError::StaleActionReference)),
        "a file added after confirmation but before rename must invalidate delete"
    );
    assert!(source_path.exists(), "the skill file must remain");
    assert!(
        raced_asset.exists(),
        "the unconfirmed raced asset must remain"
    );
    assert!(
        catalog
            .get_skill_record("tool-global-delete-race")
            .expect("catalog lookup")
            .is_some(),
        "the catalog row must remain"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[cfg(unix)]
fn exercise_local_delete_owner_replacement(after_commit: bool) {
    use std::os::unix::fs::{symlink, PermissionsExt};

    let phase = if after_commit {
        "post-commit"
    } else {
        "pre-effect"
    };
    let temp_root = temp_test_dir(&format!("local-delete-owner-replacement-{phase}"));
    let app_data = temp_root.join("app-data");
    let moved_owner = temp_root.join("locked-owner");
    let victim = temp_root.join("victim");
    let source_path = tool_global_staging_skills_root(&app_data)
        .join(format!("delete-owner-{phase}"))
        .join("SKILL.md");
    std::fs::create_dir_all(source_path.parent().expect("source parent")).expect("create source");
    std::fs::write(
        &source_path,
        format!("---\nname: delete-owner-{phase}\ndescription: original\n---\noriginal"),
    )
    .expect("write source");
    std::fs::create_dir(&victim).expect("create victim");
    std::fs::write(victim.join("sentinel"), b"unchanged").expect("victim sentinel");
    std::fs::set_permissions(&victim, std::fs::Permissions::from_mode(0o750)).expect("victim mode");
    let victim_mode = std::fs::metadata(&victim)
        .expect("victim metadata")
        .permissions()
        .mode()
        & 0o777;
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let instance_id = format!("tool-global-delete-owner-{phase}");
    let mut instance = install_tool_global_instance(
        &instance_id,
        source_path.clone(),
        &format!("delete-owner-{phase}"),
    );
    instance.agent = AgentId::ToolGlobal;
    catalog
        .upsert_skill_instance(&instance)
        .expect("upsert tool-global");
    let preview = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: instance_id.clone(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect("delete preview");
    let raced_app_data = app_data.clone();
    let raced_moved_owner = moved_owner.clone();
    let raced_victim = victim.clone();
    let race = move || {
        std::fs::rename(&raced_app_data, &raced_moved_owner).expect("move locked owner");
        symlink(&raced_victim, &raced_app_data).expect("replace owner path");
    };
    if after_commit {
        skill_manager::install_manager_post_commit_test_hook("deleteLocal", race);
    } else {
        skill_manager::install_local_delete_pre_rename_test_hook(
            source_path.canonicalize().expect("canonical source"),
            race,
        );
    }

    let error = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: instance_id.clone(),
            confirmed: true,
            preview_token: preview.preview_token.clone(),
            action_reference: preview.action.as_ref().map(ActionReference::from),
        },
    )
    .expect_err("owner replacement must fail closed");

    if after_commit {
        assert!(matches!(
            error,
            CommandError::PartialEffect {
                state: "applied_unverified",
                cleanup_required: true,
                ..
            }
        ));
    } else {
        assert!(matches!(error, CommandError::UnsafeConfigPath(_)));
    }
    assert_eq!(
        std::fs::read(victim.join("sentinel")).expect("victim sentinel"),
        b"unchanged"
    );
    assert_eq!(
        std::fs::metadata(&victim)
            .expect("victim metadata")
            .permissions()
            .mode()
            & 0o777,
        victim_mode
    );
    assert_eq!(
        std::fs::read_dir(&victim).expect("victim entries").count(),
        1,
        "the victim must receive no delete or cleanup files"
    );
    assert_eq!(
        catalog
            .get_skill_record(&instance_id)
            .expect("catalog lookup")
            .is_some(),
        !after_commit,
        "the catalog may change only before the final post-commit binding check"
    );
    assert_eq!(
        moved_owner
            .join(
                source_path
                    .strip_prefix(&app_data)
                    .expect("source relative to owner")
            )
            .exists(),
        !after_commit,
        "pre-effect rejection preserves the source; committed delete remains applied"
    );

    std::fs::remove_file(&app_data).expect("remove raced owner link");
    std::fs::remove_dir_all(&temp_root).ok();
}

#[test]
#[cfg(unix)]
fn local_delete_rejects_owner_replacement_before_any_effect_without_touching_victim() {
    exercise_local_delete_owner_replacement(false);
}

#[test]
#[cfg(unix)]
fn local_delete_reports_partial_when_owner_rebinds_after_commit_without_touching_victim() {
    exercise_local_delete_owner_replacement(true);
}

static LOCAL_DELETE_RECREATION_TEST_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

fn exercise_local_delete_recreation(rollback_failure: bool) {
    let _serial = LOCAL_DELETE_RECREATION_TEST_LOCK
        .lock()
        .expect("serialize local-delete recreation hooks");
    let label = if rollback_failure {
        "local-delete-rollback-failure"
    } else {
        "local-delete-post-rename-recreation"
    };
    let skill_name = if rollback_failure {
        "delete-rollback-unknown"
    } else {
        "delete-recreated"
    };
    let instance_id = format!("tool-global-{skill_name}");
    let temp_root = temp_test_dir(label);
    let app_data = temp_root.join("app-data");
    let source_path = tool_global_staging_skills_root(&app_data)
        .join(skill_name)
        .join("SKILL.md");
    std::fs::create_dir_all(source_path.parent().expect("source parent")).expect("create source");
    std::fs::write(
        &source_path,
        format!("---\nname: {skill_name}\ndescription: original\n---\noriginal"),
    )
    .expect("write source");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let mut instance = install_tool_global_instance(&instance_id, source_path.clone(), skill_name);
    instance.agent = AgentId::ToolGlobal;
    catalog
        .upsert_skill_instance(&instance)
        .expect("upsert tool-global");
    let preview = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: instance.id.clone(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect("delete preview");
    let canonical_source = source_path.canonicalize().expect("canonical source");
    let recreated_path = source_path.clone();
    skill_manager::install_local_delete_post_rename_test_hook(canonical_source, move || {
        std::fs::create_dir_all(recreated_path.parent().expect("recreated parent"))
            .expect("recreate source directory");
        std::fs::write(
            &recreated_path,
            format!("---\nname: {skill_name}\ndescription: third state\n---\nthird state"),
        )
        .expect("write unowned third state");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            std::fs::set_permissions(
                recreated_path.parent().expect("recreated parent"),
                std::fs::Permissions::from_mode(0o700),
            )
            .expect("private recreated directory");
            std::fs::set_permissions(&recreated_path, std::fs::Permissions::from_mode(0o600))
                .expect("private recreated skill");
        }
    });
    if rollback_failure {
        catalog.inject_next_rollback_failure_for_test();
    }

    let result = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: instance.id.clone(),
            confirmed: true,
            preview_token: preview.preview_token.clone(),
            action_reference: preview.action.as_ref().map(ActionReference::from),
        },
    );

    let error = result.expect_err("recreated source must fail closed");
    assert!(matches!(
        error,
        CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: true,
            ..
        }
    ));
    assert!(
        std::fs::read_to_string(&source_path)
            .expect("recreated source remains")
            .contains("third state"),
        "the unowned third state must never be overwritten"
    );
    assert!(
        catalog
            .get_skill_record(&instance.id)
            .expect("catalog lookup")
            .is_some(),
        "the catalog transaction must roll back"
    );
    assert!(
        std::fs::read_dir(
            source_path
                .parent()
                .expect("source parent")
                .parent()
                .expect("library")
        )
        .expect("read library")
        .filter_map(Result::ok)
        .any(|entry| entry
            .file_name()
            .to_string_lossy()
            .starts_with(&format!(".agent-copilot-delete-{skill_name}-"))),
        "the quarantined original remains available for explicit recovery"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn local_delete_does_not_verify_a_source_recreated_after_quarantine() {
    exercise_local_delete_recreation(false);
}

#[test]
fn local_delete_rollback_failure_preserves_recreated_source_and_quarantine() {
    exercise_local_delete_recreation(true);
}

#[test]
fn local_delete_reports_applied_verified_when_only_quarantine_cleanup_fails() {
    let temp_root = temp_test_dir("local-delete-cleanup-failure");
    let app_data = temp_root.join("app-data");
    let source_path = tool_global_staging_skills_root(&app_data)
        .join("cleanup-failure")
        .join("SKILL.md");
    std::fs::create_dir_all(source_path.parent().expect("source parent")).expect("create source");
    std::fs::write(
        &source_path,
        "---\nname: cleanup-failure\ndescription: cleanup failure\n---\nbody",
    )
    .expect("write source");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let mut instance = install_tool_global_instance(
        "tool-global-cleanup-failure",
        source_path.clone(),
        "cleanup-failure",
    );
    instance.agent = AgentId::ToolGlobal;
    catalog
        .upsert_skill_instance(&instance)
        .expect("upsert tool-global");
    let preview = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: instance.id.clone(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect("delete preview");
    let canonical_source = source_path.canonicalize().expect("canonical source");
    skill_manager::install_local_delete_cleanup_failure_test_hook(canonical_source);

    let record = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: instance.id.clone(),
            confirmed: true,
            preview_token: preview.preview_token.clone(),
            action_reference: preview.action.as_ref().map(ActionReference::from),
        },
    )
    .expect("verified delete remains a successful apply");

    assert!(record.deleted);
    assert!(record.readback.as_ref().is_some_and(|item| item.verified));
    let follow_up = record
        .follow_up
        .expect("cleanup failure must be explicit in the success record");
    assert_eq!(follow_up.kind, "quarantine_cleanup");
    assert_eq!(follow_up.state, "delete_applied_cleanup_pending");
    assert!(follow_up.cleanup_required);
    assert!(!follow_up
        .message
        .contains(&temp_root.to_string_lossy().to_string()));
    assert!(!source_path.exists(), "committed delete remains applied");
    assert!(
        catalog
            .get_skill_record(&instance.id)
            .expect("catalog lookup")
            .is_none(),
        "catalog delete is committed before quarantine cleanup"
    );
    let source_parent = source_path.parent().expect("source parent");
    let library_root = source_parent.parent().expect("library root");
    assert!(
        std::fs::read_dir(library_root)
            .expect("read library")
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with(".agent-copilot-delete-cleanup-failure-")),
        "the partial-effect cleanup target remains available for explicit cleanup"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn local_delete_retains_quarantine_when_commit_outcome_is_unknown() {
    let temp_root = temp_test_dir("local-delete-commit-outcome-unknown");
    let app_data = temp_root.join("app-data");
    let source_path = tool_global_staging_skills_root(&app_data)
        .join("delete-commit-unknown")
        .join("SKILL.md");
    std::fs::create_dir_all(source_path.parent().expect("source parent")).expect("create source");
    std::fs::write(
        &source_path,
        "---\nname: delete-commit-unknown\ndescription: fixture\n---\nbody",
    )
    .expect("write source");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let mut instance = install_tool_global_instance(
        "tool-global-delete-commit-unknown",
        source_path.clone(),
        "delete-commit-unknown",
    );
    instance.agent = AgentId::ToolGlobal;
    catalog
        .upsert_skill_instance(&instance)
        .expect("upsert tool-global");
    let preview = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: instance.id.clone(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect("delete preview");
    catalog.inject_next_commit_outcome_unknown_for_test();

    let error = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: instance.id,
            confirmed: true,
            preview_token: preview.preview_token,
            action_reference: preview.action.as_ref().map(ActionReference::from),
        },
    )
    .expect_err("unknown commit outcome must return a partial effect");

    assert!(matches!(
        error,
        CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: true,
            ..
        }
    ));
    assert!(
        !source_path.exists(),
        "an uncertain catalog commit must not restore the source"
    );
    assert!(
        std::fs::read_dir(
            source_path
                .parent()
                .expect("source parent")
                .parent()
                .expect("library root")
        )
        .expect("read library")
        .filter_map(Result::ok)
        .any(|entry| entry
            .file_name()
            .to_string_lossy()
            .starts_with(".agent-copilot-delete-delete-commit-unknown-")),
        "private restoration material must remain available for inspection"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn local_delete_rechecks_catalog_references_under_the_shared_lock() {
    let temp_root = temp_test_dir("local-delete-reference-race");
    let app_data = temp_root.join("app-data");
    let source_path = tool_global_staging_skills_root(&app_data)
        .join("delete-reference-race")
        .join("SKILL.md");
    std::fs::create_dir_all(source_path.parent().expect("source parent")).expect("create source");
    std::fs::write(
        &source_path,
        "---\nname: delete-reference-race\ndescription: before\n---\nbefore",
    )
    .expect("write source");
    let catalog_path = app_data.join("catalog.sqlite");
    let catalog = Catalog::open(&catalog_path).expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let racing_catalog = Catalog::open(&catalog_path).expect("racing catalog opens");
    let mut local_instance = install_tool_global_instance(
        "tool-global-delete-reference-race",
        source_path.clone(),
        "delete-reference-race",
    );
    local_instance.agent = AgentId::ToolGlobal;
    catalog
        .upsert_skill_instance(&local_instance)
        .expect("upsert tool-global");

    let preview = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: local_instance.id.clone(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect("delete preview");
    assert!(preview.physical_delete_allowed);
    let canonical_source = source_path.canonicalize().expect("canonical source");
    let reference_path = temp_root.join("home/.codex/skills/delete-reference-race/SKILL.md");
    std::fs::create_dir_all(reference_path.parent().expect("reference parent"))
        .expect("create reference root");
    std::fs::write(
        &reference_path,
        "---\nname: delete-reference-race\ndescription: linked\n---\nlinked",
    )
    .expect("write reference");
    let mut reference = install_tool_global_instance(
        "codex-delete-reference-race",
        reference_path,
        "delete-reference-race",
    );
    reference.agent = AgentId::Codex;
    reference.scope = Scope::AgentGlobal;
    skill_manager::install_local_delete_pre_rename_test_hook(canonical_source, move || {
        racing_catalog
            .upsert_skill_instance(&reference)
            .expect("inject supported-agent reference")
    });

    let result = delete_local_skill_with_manager(
        &catalog,
        &app_data,
        &SkillManagerDeleteLocalParams {
            instance_id: local_instance.id.clone(),
            confirmed: true,
            preview_token: preview.preview_token.clone(),
            action_reference: preview.action.as_ref().map(ActionReference::from),
        },
    );

    assert!(
        matches!(result, Err(CommandError::StaleActionReference)),
        "a supported-agent reference added after confirmation must invalidate delete"
    );
    assert!(source_path.exists(), "the source directory must remain");
    assert!(
        catalog
            .get_skill_record(&local_instance.id)
            .expect("catalog lookup")
            .is_some(),
        "the app-owned catalog row must remain"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn direct_install_preview_and_apply_share_source_binding_stamp() {
    initialize_action_preview_secret_for_test([0xA5; 32])
        .expect("initialize action preview test secret");
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-install-confirmed-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&home, "portable-beta");
    let source_content = std::fs::read_to_string(&source_path).expect("source content");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-beta",
            source_path,
            "portable-beta",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let preview = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-beta",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        None,
        None,
    )
    .expect("install preview");
    let confirmation =
        ActionConfirmation::confirmed(&preview.action, preview.preview_token.clone());
    let result = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-beta",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        None,
        Some(&confirmation),
    )
    .expect("the owner-locked apply stamp must match the preview binding");

    let target = home
        .join(".claude")
        .join("skills")
        .join("portable-beta")
        .join("SKILL.md");
    assert!(result.wrote);
    assert_path_text_eq(&result.target_path, &target);
    assert_eq!(
        std::fs::read_to_string(&target).expect("target content"),
        source_content
    );
    let snapshots = catalog
        .list_config_snapshots("claude-code", &target.to_string_lossy())
        .expect("snapshots");
    assert!(
        snapshots.is_empty(),
        "direct skill-file installs must not create agent config snapshots"
    );
    assert!(catalog
        .list_skill_records()
        .expect("records")
        .iter()
        .any(|record| record.agent == "claude-code" && record.name == "portable-beta"));

    let _ = std::fs::remove_dir_all(&temp_root);
}

fn exercise_install_post_rename_failure(rollback_failure: bool) {
    let label = if rollback_failure {
        "install-rollback-failure"
    } else {
        "install-post-rename-failure"
    };
    let skill_name = if rollback_failure {
        "preserve-install"
    } else {
        "restore-install"
    };
    let instance_id = format!("tool-global-{skill_name}");
    let temp_root = temp_test_dir(label);
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&home, skill_name);
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            &instance_id,
            source_path,
            skill_name,
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = install_skill_from_tool_global(
        &catalog,
        &ctx,
        &instance_id,
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        None,
        None,
    )
    .expect("preview");
    let target = PathBuf::from(&preview.target_path);
    install_atomic_post_rename_test_hook(target.clone(), |_| {});
    if rollback_failure {
        catalog.inject_next_rollback_failure_for_test();
    }
    let confirmation =
        ActionConfirmation::confirmed(&preview.action, preview.preview_token.clone());

    let error = install_skill_from_tool_global(
        &catalog,
        &ctx,
        &instance_id,
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        None,
        Some(&confirmation),
    )
    .expect_err("post-rename failure must fail install");

    if rollback_failure {
        assert!(matches!(
            error,
            CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            }
        ));
        assert!(
            target.is_file(),
            "unknown rollback must preserve the written skill candidate"
        );
    } else {
        assert!(matches!(
            error,
            CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            }
        ));
        assert!(!target.exists(), "candidate target must be removed");
        assert!(
            !home.join(".claude/skills").join(skill_name).exists(),
            "directories created only for the failed install must be removed"
        );
        assert!(!catalog
            .list_skill_records()
            .expect("records")
            .iter()
            .any(|record| {
                record.agent == "claude-code"
                    && record.scope == Scope::AgentGlobal.as_str()
                    && record.name == skill_name
            }));
    }

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn confirmed_install_restores_a_missing_target_when_atomic_write_fails_after_rename() {
    exercise_install_post_rename_failure(false);
}

#[test]
fn confirmed_install_rollback_failure_preserves_the_written_candidate() {
    exercise_install_post_rename_failure(true);
}

#[test]
fn confirmed_install_removes_temp_and_owned_parents_when_write_fails_before_rename() {
    let temp_root = temp_test_dir("install-pre-rename-failure");
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&home, "pre-rename-install");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-pre-rename-install",
            source_path,
            "pre-rename-install",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-pre-rename-install",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        None,
        None,
    )
    .expect("preview");
    let target = PathBuf::from(&preview.target_path);
    install_atomic_pre_rename_failure_test_hook(target.clone());
    let confirmation =
        ActionConfirmation::confirmed(&preview.action, preview.preview_token.clone());

    let error = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-pre-rename-install",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        None,
        Some(&confirmation),
    )
    .expect_err("pre-rename failure must fail install");

    assert!(matches!(error, CommandError::VerificationFailed));
    assert!(!target.exists());
    assert!(
        !home.join(".claude").exists(),
        "the complete parent chain created only for the failed install must be removed"
    );
    assert!(!catalog
        .list_skill_records()
        .expect("records")
        .iter()
        .any(|record| {
            record.agent == "claude-code"
                && record.scope == Scope::AgentGlobal.as_str()
                && record.name == "pre-rename-install"
        }));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn confirmed_install_preserves_a_concurrent_third_state_and_reports_partial_effect() {
    let temp_root = temp_test_dir("install-third-state");
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&home, "third-state-install");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-third-state-install",
            source_path,
            "third-state-install",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-third-state-install",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        None,
        None,
    )
    .expect("preview");
    let target = PathBuf::from(&preview.target_path);
    let third_state = "---\nname: third-state-install\n---\nconcurrent third state\n";
    install_atomic_post_rename_test_hook(target.clone(), move |path| {
        std::fs::write(path, third_state).expect("write third state");
    });
    let confirmation =
        ActionConfirmation::confirmed(&preview.action, preview.preview_token.clone());

    let error = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-third-state-install",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        None,
        Some(&confirmation),
    )
    .expect_err("third state must make the outcome partial");

    assert!(matches!(
        error,
        CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: true,
            ..
        }
    ));
    assert_eq!(
        std::fs::read_to_string(&target).expect("third state remains"),
        third_state,
        "compensation must never overwrite an unowned concurrent state"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn confirmed_install_restores_files_and_catalog_when_commit_fails() {
    let temp_root = temp_test_dir("install-commit-failure");
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&home, "commit-fail-install");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-commit-fail-install",
            source_path,
            "commit-fail-install",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-commit-fail-install",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        None,
        None,
    )
    .expect("preview");
    let target = PathBuf::from(&preview.target_path);
    let confirmation =
        ActionConfirmation::confirmed(&preview.action, preview.preview_token.clone());
    catalog.inject_next_commit_failure_for_test();

    let error = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-commit-fail-install",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        None,
        Some(&confirmation),
    )
    .expect_err("injected commit failure must fail install");

    assert!(error
        .to_string()
        .contains("injected catalog commit failure"));
    assert!(
        !target.exists(),
        "installed file must be restored to missing"
    );
    assert!(!catalog
        .list_skill_records()
        .expect("records")
        .iter()
        .any(|record| {
            record.agent == "claude-code"
                && record.scope == Scope::AgentGlobal.as_str()
                && record.name == "commit-fail-install"
        }));
    assert!(
        catalog
            .list_config_snapshots("claude-code", &target.to_string_lossy())
            .expect("snapshots")
            .is_empty(),
        "no transaction-owned catalog state may commit"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn confirmed_install_preserves_candidate_when_commit_outcome_is_unknown() {
    let temp_root = temp_test_dir("install-commit-outcome-unknown");
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&home, "commit-unknown-install");
    let expected_content = std::fs::read_to_string(&source_path).expect("source content");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-commit-unknown-install",
            source_path,
            "commit-unknown-install",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-commit-unknown-install",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        None,
        None,
    )
    .expect("preview");
    let target = PathBuf::from(&preview.target_path);
    let confirmation =
        ActionConfirmation::confirmed(&preview.action, preview.preview_token.clone());
    catalog.inject_next_commit_outcome_unknown_for_test();

    let error = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-commit-unknown-install",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        None,
        Some(&confirmation),
    )
    .expect_err("unknown commit outcome must return a partial effect");

    assert!(matches!(
        error,
        CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: true,
            ..
        }
    ));
    assert_eq!(
        std::fs::read_to_string(&target).expect("preserved candidate target"),
        expected_content,
        "an uncertain catalog commit must never trigger reverse compensation"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[cfg(unix)]
fn exercise_config_parent_swap(point: crate::external_target::ExternalTargetHookPoint) {
    use std::os::unix::fs::symlink;

    let temp_root = temp_test_dir("external-config-parent-swap");
    let home = temp_root.join("home");
    let app_data = temp_root.join("app-data");
    let config_parent = home.join(".claude");
    let accepted_parent = temp_root.join("accepted-config-parent");
    let victim = temp_root.join("victim");
    let settings_path = config_parent.join("settings.json");
    std::fs::create_dir_all(&config_parent).expect("config parent");
    std::fs::create_dir_all(&app_data).expect("app data");
    std::fs::create_dir_all(&victim).expect("victim");
    std::fs::write(&settings_path, "{}\n").expect("original config");
    std::fs::write(victim.join("sentinel"), b"unchanged").expect("victim sentinel");
    let catalog = Catalog::open(&app_data.join("catalog.sqlite")).expect("catalog");
    catalog.init().expect("catalog schema");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let current = read_claude_settings(&ctx).expect("current config");
    let candidate = "{\n  \"requested\": true\n}\n";
    let preview =
        preview_claude_settings_save(&ctx, candidate, &current.revision).expect("save preview");
    let prepared = prepare_claude_settings_save(
        &ctx,
        candidate,
        &confirmed_action(&preview.action, &preview.preview_token),
    )
    .expect("prepared save");
    let raced_parent = config_parent.clone();
    let raced_accepted = accepted_parent.clone();
    let raced_victim = victim.clone();
    crate::external_target::install_external_target_test_hook(
        settings_path.clone(),
        point,
        move || {
            std::fs::rename(&raced_parent, &raced_accepted).expect("move accepted config parent");
            symlink(&raced_victim, &raced_parent).expect("replace config parent with victim link");
        },
    );

    let error = commit_prepared_claude_settings_save(&catalog, &app_data, prepared)
        .expect_err("detached config parent must fail");

    assert!(
        matches!(
            error,
            CommandError::StaleActionReference | CommandError::PartialEffect { .. }
        ),
        "unexpected error: {error:?}"
    );
    assert_eq!(
        std::fs::read_to_string(accepted_parent.join("settings.json"))
            .expect("accepted config restored"),
        "{}\n"
    );
    assert_eq!(
        std::fs::read(victim.join("sentinel")).expect("victim sentinel"),
        b"unchanged"
    );
    assert_eq!(
        std::fs::read_dir(&victim).expect("victim entries").count(),
        1
    );
    assert!(
        std::fs::read_dir(&accepted_parent)
            .expect("accepted entries")
            .all(|entry| {
                !entry
                    .expect("accepted entry")
                    .file_name()
                    .to_string_lossy()
                    .contains("agent-copilot-write")
            }),
        "candidate and retained-original temp entries must be gone"
    );

    std::fs::remove_file(&config_parent).expect("remove raced parent link");
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
#[cfg(unix)]
fn config_save_rejects_parent_swap_before_temp_without_touching_victim() {
    exercise_config_parent_swap(crate::external_target::ExternalTargetHookPoint::BeforeTempCreate);
}

#[test]
#[cfg(unix)]
fn config_save_cleans_candidate_when_parent_swaps_before_rename() {
    exercise_config_parent_swap(crate::external_target::ExternalTargetHookPoint::BeforeRename);
}

#[test]
#[cfg(unix)]
fn config_save_restores_exact_original_when_parent_swaps_after_rename() {
    exercise_config_parent_swap(crate::external_target::ExternalTargetHookPoint::AfterRename);
}

#[test]
#[cfg(unix)]
fn config_save_compensation_stays_anchored_when_parent_swaps_before_restore() {
    use std::os::unix::fs::symlink;

    let temp_root = temp_test_dir("external-config-compensation-swap");
    let home = temp_root.join("home");
    let app_data = temp_root.join("app-data");
    let config_parent = home.join(".claude");
    let accepted_parent = temp_root.join("accepted-config-parent");
    let victim = temp_root.join("victim");
    let settings_path = config_parent.join("settings.json");
    std::fs::create_dir_all(&config_parent).expect("config parent");
    std::fs::create_dir_all(&app_data).expect("app data");
    std::fs::create_dir_all(&victim).expect("victim");
    std::fs::write(&settings_path, "{}\n").expect("original config");
    std::fs::write(victim.join("sentinel"), b"unchanged").expect("victim sentinel");
    let catalog = Catalog::open(&app_data.join("catalog.sqlite")).expect("catalog");
    catalog.init().expect("catalog schema");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let current = read_claude_settings(&ctx).expect("current config");
    let candidate = "{\n  \"requested\": true\n}\n";
    let preview =
        preview_claude_settings_save(&ctx, candidate, &current.revision).expect("save preview");
    let prepared = prepare_claude_settings_save(
        &ctx,
        candidate,
        &confirmed_action(&preview.action, &preview.preview_token),
    )
    .expect("prepared save");
    install_atomic_post_rename_test_hook(settings_path.clone(), |_| {});
    let raced_parent = config_parent.clone();
    let raced_accepted = accepted_parent.clone();
    let raced_victim = victim.clone();
    crate::external_target::install_external_target_test_hook(
        settings_path,
        crate::external_target::ExternalTargetHookPoint::BeforeCompensation,
        move || {
            std::fs::rename(&raced_parent, &raced_accepted).expect("move accepted config parent");
            symlink(&raced_victim, &raced_parent).expect("replace config parent with victim link");
        },
    );

    let error = commit_prepared_claude_settings_save(&catalog, &app_data, prepared)
        .expect_err("compensation binding drift must be partial");

    assert!(matches!(error, CommandError::PartialEffect { .. }));
    assert_eq!(
        std::fs::read_to_string(accepted_parent.join("settings.json"))
            .expect("accepted config restored"),
        "{}\n"
    );
    assert_eq!(
        std::fs::read(victim.join("sentinel")).expect("victim sentinel"),
        b"unchanged"
    );
    assert_eq!(
        std::fs::read_dir(&victim).expect("victim entries").count(),
        1
    );

    std::fs::remove_file(&config_parent).expect("remove raced parent link");
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
#[cfg(unix)]
fn config_read_rejects_same_length_in_place_mutation() {
    let temp_root = temp_test_dir("external-config-same-length-write");
    let home = temp_root.join("home");
    let app_data = temp_root.join("app-data");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("config parent")).expect("config parent");
    std::fs::create_dir_all(&app_data).expect("app data");
    std::fs::write(&settings_path, "{\"a\":1}\n").expect("original config");
    let catalog = Catalog::open(&app_data.join("catalog.sqlite")).expect("catalog");
    catalog.init().expect("catalog schema");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let current = read_claude_settings(&ctx).expect("current config");
    let preview =
        preview_claude_settings_save(&ctx, "{\"c\":1}\n", &current.revision).expect("preview");
    let prepared = prepare_claude_settings_save(
        &ctx,
        "{\"c\":1}\n",
        &confirmed_action(&preview.action, &preview.preview_token),
    )
    .expect("prepared");
    let raced_target = settings_path.clone();
    crate::external_target::install_external_target_test_hook(
        settings_path.clone(),
        crate::external_target::ExternalTargetHookPoint::DuringRead,
        move || std::fs::write(raced_target, "{\"b\":1}\n").expect("same-length mutation"),
    );

    let error = commit_prepared_claude_settings_save(&catalog, &app_data, prepared)
        .expect_err("same-length in-place mutation must stale the read");

    assert!(matches!(error, CommandError::StaleActionReference));
    assert_eq!(
        std::fs::read_to_string(settings_path).expect("third state"),
        "{\"b\":1}\n"
    );
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[cfg(unix)]
fn exercise_config_parent_sync_failure(original: Option<&str>) {
    let temp_root = temp_test_dir("external-config-sync-failure");
    let home = temp_root.join("home");
    let app_data = temp_root.join("app-data");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("config parent")).expect("config parent");
    std::fs::create_dir_all(&app_data).expect("app data");
    if let Some(original) = original {
        std::fs::write(&settings_path, original).expect("original config");
    }
    let catalog = Catalog::open(&app_data.join("catalog.sqlite")).expect("catalog");
    catalog.init().expect("catalog schema");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let current = read_claude_settings(&ctx).expect("current config");
    let candidate = "{\n  \"requested\": true\n}\n";
    let preview =
        preview_claude_settings_save(&ctx, candidate, &current.revision).expect("save preview");
    let prepared = prepare_claude_settings_save(
        &ctx,
        candidate,
        &confirmed_action(&preview.action, &preview.preview_token),
    )
    .expect("prepared");
    crate::external_target::install_external_parent_sync_failure(settings_path.clone());

    let error = commit_prepared_claude_settings_save(&catalog, &app_data, prepared)
        .expect_err("injected parent sync failure");

    assert!(matches!(error, CommandError::PartialEffect { .. }));
    match original {
        Some(original) => assert_eq!(
            std::fs::read_to_string(&settings_path).expect("restored original"),
            original
        ),
        None => assert!(!settings_path.exists(), "missing target must be restored"),
    }
    assert!(
        std::fs::read_dir(settings_path.parent().expect("config parent"))
            .expect("config entries")
            .all(|entry| {
                !entry
                    .expect("config entry")
                    .file_name()
                    .to_string_lossy()
                    .contains("agent-copilot-write")
            }),
        "sync-failure compensation must remove candidate and retained backup"
    );
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
#[cfg(unix)]
fn config_save_present_target_recovers_from_post_rename_parent_sync_failure() {
    exercise_config_parent_sync_failure(Some("{}\n"));
}

#[test]
#[cfg(unix)]
fn config_save_missing_target_recovers_from_post_rename_parent_sync_failure() {
    exercise_config_parent_sync_failure(None);
}
