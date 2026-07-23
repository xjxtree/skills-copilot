use super::*;
use skills_copilot_core::canonical_project_id;

fn product_test_host(label: &str) -> (PathBuf, ServiceHost, String, String) {
    let root = env::temp_dir().join(format!(
        "agent-copilot-product-read-{label}-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let home = root.join("home");
    let project = root.join("project");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&project).expect("create project");
    let host = ServiceHost {
        app_data_dir: root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: home,
            project_root: Some(project.clone()),
            project_cwd: Some(project.clone()),
            extra_roots: Vec::new(),
        },
    };
    let context_revision =
        effective_project_context_revision(&host.app_data_dir, host.env_project_context().as_ref())
            .expect("context revision");
    let project_id = canonical_project_id(&project.to_string_lossy());
    (root, host, project_id, context_revision)
}

fn write_skill(path: &Path, name: &str) {
    fs::create_dir_all(path.parent().expect("skill parent")).expect("create skill parent");
    fs::write(
        path,
        format!("---\nname: {name}\ndescription: {name} fixture\n---\n"),
    )
    .expect("write skill");
}

fn add_codex_plugin(home: &Path, publisher: &str, package: &str, skill_name: &str) {
    let version = home
        .join(".codex/plugins/cache")
        .join(publisher)
        .join(package)
        .join("1.0.0");
    fs::create_dir_all(version.join(".codex-plugin")).expect("create plugin manifest root");
    fs::write(
        version.join(".codex-plugin/plugin.json"),
        format!(r#"{{"name":"{package}","version":"1.0.0","skills":"./skills/"}}"#),
    )
    .expect("write plugin manifest");
    write_skill(
        &version.join("skills").join(skill_name).join("SKILL.md"),
        skill_name,
    );
}

fn scan_all(host: &ServiceHost, context_revision: &str) {
    let response = host.handle(ServiceRequest {
        id: Some("scan-all".to_string()),
        method: "catalog.scanAll".to_string(),
        params: json!({
            "explicit_refresh": true,
            "expected_context_revision": context_revision
        }),
    });
    assert!(response.ok, "{:?}", response.error);
}

#[test]
fn product_reads_fail_closed_before_scan_and_use_typed_project_errors() {
    let (root, host, project_id, context_revision) = product_test_host("uninspected");
    let response = host.handle(ServiceRequest {
        id: Some("aggregates".to_string()),
        method: "catalog.listSkillAggregates".to_string(),
        params: json!({
            "project_id": project_id,
            "expected_project_context_revision": context_revision,
            "agent": "codex"
        }),
    });
    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("aggregate result");
    assert_eq!(
        result.pointer("/coverage/completeness"),
        Some(&json!("unknown"))
    );
    assert_eq!(
        result.pointer("/coverage/incomplete_reason"),
        Some(&json!("not_inspected"))
    );
    assert_eq!(result.pointer("/page/total_count"), Some(&Value::Null));

    let mismatch = host.handle(ServiceRequest {
        id: Some("wrong-project".to_string()),
        method: "project.getReadiness".to_string(),
        params: json!({
            "project_id": "project-wrong",
            "expected_project_context_revision": context_revision
        }),
    });
    assert_eq!(
        mismatch.error.as_ref().map(|error| error.code.as_str()),
        Some("project_context_mismatch")
    );
    let stale = host.handle(ServiceRequest {
        id: Some("stale-context".to_string()),
        method: "project.getReadiness".to_string(),
        params: json!({
            "project_id": project_id,
            "expected_project_context_revision": "sha256:stale"
        }),
    });
    assert_eq!(
        stale.error.as_ref().map(|error| error.code.as_str()),
        Some("stale_project_context")
    );

    let no_project_host = ServiceHost {
        app_data_dir: root.join("no-project-app-data"),
        adapter_ctx: AdapterContext {
            user_home: root.join("no-project-home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    let no_project_revision = effective_project_context_revision(
        &no_project_host.app_data_dir,
        no_project_host.env_project_context().as_ref(),
    )
    .expect("no-project context revision");
    let required = no_project_host.handle(ServiceRequest {
        id: Some("project-required".to_string()),
        method: "project.getReadiness".to_string(),
        params: json!({
            "project_id": project_id,
            "expected_project_context_revision": no_project_revision
        }),
    });
    assert_eq!(
        required.error.as_ref().map(|error| error.code.as_str()),
        Some("project_context_required")
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn scan_backed_aggregates_keep_plugin_and_compatibility_sources_distinct() {
    let (root, host, project_id, context_revision) = product_test_host("provenance");
    let home = &host.adapter_ctx.user_home;
    write_skill(
        &home.join(".config/opencode/skills/shared/SKILL.md"),
        "shared",
    );
    write_skill(&home.join(".claude/skills/shared/SKILL.md"), "shared");
    add_codex_plugin(home, "publisher-one", "plugin-one", "same-plugin-skill");
    add_codex_plugin(home, "publisher-two", "plugin-two", "same-plugin-skill");
    fs::create_dir_all(home.join(".codex")).expect("create codex config root");
    fs::write(
        home.join(".codex/config.toml"),
        "[plugins.\"plugin-one@publisher-one\"]\nenabled = true\n\
         [plugins.\"plugin-two@publisher-two\"]\nenabled = true\n",
    )
    .expect("write codex config");

    scan_all(&host, &context_revision);

    let codex = host.handle(ServiceRequest {
        id: Some("codex-aggregates".to_string()),
        method: "catalog.listSkillAggregates".to_string(),
        params: json!({
            "project_id": project_id,
            "expected_project_context_revision": context_revision,
            "agent": "codex",
            "limit": 100
        }),
    });
    assert!(codex.ok, "{:?}", codex.error);
    let codex = codex.result.expect("codex aggregate result");
    assert_eq!(
        codex.pointer("/coverage/completeness"),
        Some(&json!("enumerable"))
    );
    let plugin_aggregates = codex["aggregates"]
        .as_array()
        .expect("codex aggregates")
        .iter()
        .filter(|aggregate| {
            aggregate["canonical_name"]
                .as_str()
                .is_some_and(|name| name.ends_with(":same-plugin-skill"))
        })
        .collect::<Vec<_>>();
    assert_eq!(
        plugin_aggregates.len(),
        2,
        "different plugin ids must not aggregate"
    );
    for aggregate in plugin_aggregates {
        assert_eq!(
            aggregate["primary_effectiveness"].as_str(),
            Some("effective")
        );
        let identity = aggregate["source_identity"]
            .as_str()
            .expect("plugin source identity");
        assert!(!identity.contains('/'));
        assert!(!identity.contains("cache"));
        assert!(!identity.contains(&home.to_string_lossy().to_string()));
        for evidence in aggregate["evidence"]
            .as_array()
            .expect("aggregate evidence")
        {
            let summary = evidence["summary"]
                .as_str()
                .expect("aggregate evidence summary");
            assert!(
                !summary.to_ascii_lowercase().contains("cache"),
                "product evidence must describe the accepted snapshot, not cache infrastructure"
            );
        }
    }

    let opencode = host.handle(ServiceRequest {
        id: Some("opencode-aggregates".to_string()),
        method: "catalog.listSkillAggregates".to_string(),
        params: json!({
            "project_id": project_id,
            "expected_project_context_revision": context_revision,
            "agent": "opencode",
            "limit": 100
        }),
    });
    assert!(opencode.ok, "{:?}", opencode.error);
    let shared = opencode.result.expect("opencode aggregate result")["aggregates"]
        .as_array()
        .expect("opencode aggregates")
        .iter()
        .filter(|aggregate| aggregate["canonical_name"].as_str() == Some("shared"))
        .map(|aggregate| {
            aggregate["source_identity"]
                .as_str()
                .expect("source identity")
                .to_string()
        })
        .collect::<BTreeSet<_>>();
    assert_eq!(
        shared.len(),
        2,
        "native and Claude compatibility roots must remain distinct"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn product_source_revision_rejects_stale_config_and_cursor_inputs() {
    let (root, host, project_id, context_revision) = product_test_host("stale");
    let home = &host.adapter_ctx.user_home;
    write_skill(&home.join(".codex/skills/one/SKILL.md"), "one");
    write_skill(&home.join(".codex/skills/two/SKILL.md"), "two");
    scan_all(&host, &context_revision);

    let first = host.handle(ServiceRequest {
        id: Some("first".to_string()),
        method: "catalog.listSkillAggregates".to_string(),
        params: json!({
            "project_id": project_id,
            "expected_project_context_revision": context_revision,
            "agent": "codex",
            "limit": 1
        }),
    });
    assert!(first.ok, "{:?}", first.error);
    let first = first.result.expect("first result");
    let source_revision = first["source_revision"]
        .as_str()
        .expect("source revision")
        .to_string();
    let cursor = first
        .pointer("/page/next_cursor")
        .and_then(Value::as_str)
        .expect("first page cursor")
        .to_string();
    let continuation = host.handle(ServiceRequest {
        id: Some("continuation".to_string()),
        method: "catalog.listSkillAggregates".to_string(),
        params: json!({
            "project_id": project_id,
            "expected_project_context_revision": context_revision,
            "agent": "codex",
            "limit": 1,
            "cursor": cursor,
            "source_revision": source_revision
        }),
    });
    assert!(continuation.ok, "{:?}", continuation.error);

    fs::create_dir_all(home.join(".codex")).expect("create config parent");
    fs::write(
        home.join(".codex/config.toml"),
        "[skills.config]\n\"unused\" = { enabled = false }\n",
    )
    .expect("change config");
    let stale = host.handle(ServiceRequest {
        id: Some("stale".to_string()),
        method: "project.getReadiness".to_string(),
        params: json!({
            "project_id": project_id,
            "expected_project_context_revision": context_revision,
            "source_revision": source_revision
        }),
    });
    assert_eq!(
        stale.error.as_ref().map(|error| error.code.as_str()),
        Some("source_changed")
    );
    let stale_cursor = host.handle(ServiceRequest {
        id: Some("stale-cursor".to_string()),
        method: "catalog.listSkillAggregates".to_string(),
        params: json!({
            "project_id": project_id,
            "expected_project_context_revision": context_revision,
            "agent": "codex",
            "limit": 1,
            "cursor": cursor,
            "source_revision": source_revision
        }),
    });
    assert_eq!(
        stale_cursor.error.as_ref().map(|error| error.code.as_str()),
        Some("source_changed")
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn accepted_product_snapshot_revalidation_rejects_later_config_changes() {
    let (root, host, project_id, context_revision) = product_test_host("revalidate");
    let home = &host.adapter_ctx.user_home;
    write_skill(&home.join(".codex/skills/one/SKILL.md"), "one");
    scan_all(&host, &context_revision);
    let snapshot = host
        .accept_current_product_snapshot(&project_id, &context_revision, None)
        .expect("accept product snapshot");

    fs::create_dir_all(home.join(".codex")).expect("create config parent");
    fs::write(
        home.join(".codex/config.toml"),
        "[skills.config]\n\"one\" = { enabled = false }\n",
    )
    .expect("change config after snapshot acceptance");

    assert!(matches!(
        host.revalidate_current_product_snapshot(&snapshot),
        Err(ServiceError::SourceChanged)
    ));
    let _ = fs::remove_dir_all(root);
}

#[test]
fn session_resume_dispatch_binds_native_inventory_to_current_product_snapshot() {
    let (root, host, project_id, context_revision) = product_test_host("session-resume");
    scan_all(&host, &context_revision);

    let project = host
        .adapter_ctx
        .project_root
        .clone()
        .expect("active project root");
    let sessions = root.join("authorized-sessions");
    fs::create_dir_all(&sessions).expect("create session root");
    fs::write(
        sessions.join("session.jsonl"),
        json!({
            "type": "user",
            "sessionId": "claude-native-fixture",
            "cwd": project,
            "message": {
                "role": "user",
                "content": "Continue the product read protocol"
            }
        })
        .to_string(),
    )
    .expect("write Claude session");

    let list = host.handle(ServiceRequest {
        id: Some("session-list".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "authorized_roots": [sessions],
            "auto_discover": false,
            "agent": "claude-code",
            "scope": "all",
            "include_content_items": false,
            "paging_mode": "keyset",
            "limit": 100,
            "sort": "modified_at",
            "direction": "desc",
            "project_root": project,
            "current_cwd": project
        }),
    });
    assert!(list.ok, "{:?}", list.error);
    let list = list.result.expect("session list result");
    let session_id = list
        .pointer("/session_rows/0/id")
        .and_then(Value::as_str)
        .expect("session row id");
    let session_source_revision = list
        .get("source_revision")
        .and_then(Value::as_str)
        .expect("session source revision");

    let readiness = host.handle(ServiceRequest {
        id: Some("readiness".to_string()),
        method: "project.getReadiness".to_string(),
        params: json!({
            "project_id": project_id,
            "expected_project_context_revision": context_revision
        }),
    });
    assert!(readiness.ok, "{:?}", readiness.error);
    let product_source_revision = readiness
        .result
        .as_ref()
        .and_then(|value| value.get("source_revision"))
        .and_then(Value::as_str)
        .expect("product source revision");

    let resume = host.handle(ServiceRequest {
        id: Some("resume".to_string()),
        method: "session.previewResume".to_string(),
        params: json!({
            "authorized_roots": [sessions],
            "auto_discover": false,
            "agent": "claude-code",
            "project_root": project,
            "current_cwd": project,
            "session_id": session_id,
            "expected_source_revision": session_source_revision,
            "expected_snapshot_revision": product_source_revision
        }),
    });
    assert!(resume.ok, "{:?}", resume.error);
    let record = resume.result.expect("resume result");
    assert_eq!(record["project_id"], json!(project_id));
    assert_eq!(
        record.pointer("/resume/argv"),
        Some(&json!(["claude", "--resume", "claude-native-fixture"]))
    );
    assert_eq!(record["source_revision"], json!(session_source_revision));
    assert_eq!(record["snapshot_revision"], json!(product_source_revision));
    assert_eq!(
        record.pointer("/actions/0/impacts"),
        Some(&json!(["read_only"]))
    );
    let _ = fs::remove_dir_all(root);
}
