use super::*;

#[test]
fn custom_codex_home_discovers_sessions_and_index_titles() {
    let fixture = env::temp_dir().join(format!(
        "skills-copilot-custom-codex-home-test-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let user_home = fixture.join("home");
    let codex_home = user_home.join("profiles/work");
    let session_root = codex_home.join("sessions/2026/07/12");
    fs::create_dir_all(&session_root).expect("create custom Codex session root");
    fs::write(
        session_root.join("rollout-custom-home.jsonl"),
        concat!(
            r#"{"type":"session_meta","payload":{"id":"custom-home-session","source":"vscode"}}"#,
            "\n",
            r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Transcript-derived title"}]}}"#,
        ),
    )
    .expect("write custom Codex session");
    fs::write(
        codex_home.join("session_index.jsonl"),
        format!(
            "{}\n{}\n",
            json!({"id": "custom-home-session", "thread_name": "Stale custom home title"}),
            json!({"id": "custom-home-session", "thread_name": "Custom home title"})
        ),
    )
    .expect("write custom Codex index");
    let host = ServiceHost {
        app_data_dir: fixture.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home,
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };

    let _codex_home = EnvVarGuard::set("CODEX_HOME", &codex_home);
    let response = host.handle(ServiceRequest {
        id: Some("custom-codex-home".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({"agent": "codex", "limit": 10, "max_excerpt_chars": 4_000}),
    });
    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("custom Codex home preview result");

    assert_eq!(result["count"], json!(1), "{result}");
    assert_eq!(result["session_rows"][0]["title"], "Custom home title");
    let _ = fs::remove_dir_all(fixture);
}

#[test]
fn custom_codex_home_ignores_recommended_plugin_injections() {
    let fixture = env::temp_dir().join(format!(
        "skills-copilot-custom-codex-plugin-injection-test-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let user_home = fixture.join("home");
    let codex_home = user_home.join("profiles/work");
    let session_root = codex_home.join("sessions/2026/07/17");
    fs::create_dir_all(&session_root).expect("create custom Codex session root");
    fs::write(
        session_root.join("rollout-plugin-injection.jsonl"),
        [
            json!({"type":"session_meta","payload":{"id":"plugin-injection-session","source":"vscode","thread_source":"user"}}),
            json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"<recommended_plugins>\nInternal plugin catalog\n</recommended_plugins>"}]}}),
            json!({"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"修复会话列表"}]}}),
            json!({"type":"event_msg","payload":{"type":"user_message","message":"修复会话列表"}}),
        ]
        .into_iter()
        .map(|row| row.to_string())
        .collect::<Vec<_>>()
        .join("\n"),
    )
    .expect("write Codex plugin-injection session");
    fs::write(
        codex_home.join("session_index.jsonl"),
        json!({"id":"plugin-injection-session","thread_name":"<recommended_plugins>"}).to_string(),
    )
    .expect("write internal Codex index title");
    let host = ServiceHost {
        app_data_dir: fixture.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home,
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };

    let _codex_home = EnvVarGuard::set("CODEX_HOME", &codex_home);
    let response = host.handle(ServiceRequest {
        id: Some("custom-codex-plugin-injection".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({"agent": "codex", "limit": 10, "max_excerpt_chars": 4_000}),
    });
    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("Codex plugin-injection preview");

    assert_eq!(result["count"], json!(1), "{result}");
    assert_eq!(result["session_rows"][0]["title"], "修复会话列表");
    assert_eq!(result["session_rows"][0]["user_message_count"], json!(1));
    assert!(
        !serde_json::to_string(&result)
            .expect("serialize Codex plugin-injection preview")
            .contains("recommended_plugins"),
        "{result}"
    );
    let session_id = result["session_rows"][0]["id"]
        .as_str()
        .expect("preview session id");
    let messages = host.handle(ServiceRequest {
        id: Some("custom-codex-plugin-injection-messages".to_string()),
        method: "session.listLocalSessionMessages".to_string(),
        params: json!({
            "agent": "codex",
            "authorized_roots": [codex_home.join("sessions").to_string_lossy()],
            "auto_discover": false,
            "session_id": session_id,
            "limit": 40
        }),
    });
    assert!(messages.ok, "{:?}", messages.error);
    let messages = messages.result.expect("Codex plugin-injection messages");
    assert_eq!(messages["content_items"].as_array().map(Vec::len), Some(1));
    assert_eq!(messages["content_items"][0]["text"], "修复会话列表");
    assert!(!messages.to_string().contains("recommended_plugins"));
    let _ = fs::remove_dir_all(fixture);
}

#[test]
fn custom_codex_home_excludes_non_user_and_subagent_rollouts_from_session_inventory() {
    let fixture = env::temp_dir().join(format!(
        "skills-copilot-custom-codex-subagent-test-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let user_home = fixture.join("home");
    let codex_home = user_home.join("profiles/work");
    let session_root = codex_home.join("sessions/2026/07/17");
    fs::create_dir_all(&session_root).expect("create custom Codex session root");
    fs::write(
        session_root.join("rollout-parent.jsonl"),
        concat!(
            r#"{"type":"session_meta","payload":{"id":"parent-session","source":"vscode","thread_source":"user"}}"#,
            "\n",
            r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Parent task"}]}}"#,
        ),
    )
    .expect("write parent Codex session");
    fs::write(
        session_root.join("rollout-child.jsonl"),
        concat!(
            r#"{"type":"session_meta","payload":{"id":"child-session","source":{"subagent":{"thread_spawn":{"parent_thread_id":"parent-session","depth":1}}}}}"#,
            "\n",
            r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Parent task"}]}}"#,
        ),
    )
    .expect("write child Codex session");
    fs::write(
        session_root.join("rollout-host-internal.jsonl"),
        concat!(
            r#"{"type":"session_meta","payload":{"id":"host-session","source":"vscode","thread_source":"agent-pet-companion"}}"#,
            "\n",
            r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Generate an internal pet asset"}]}}"#,
        ),
    )
    .expect("write host-created Codex session");
    fs::write(
        session_root.join("rollout-memory.jsonl"),
        concat!(
            r#"{"type":"session_meta","payload":{"id":"memory-session","source":"vscode","thread_source":"memory_consolidation"}}"#,
            "\n",
            r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Consolidate memory"}]}}"#,
        ),
    )
    .expect("write memory-consolidation Codex session");
    fs::write(
        session_root.join("rollout-structured-child.jsonl"),
        concat!(
            r#"{"type":"session_meta","payload":{"id":"structured-child-session","source":"vscode","thread_source":{"subagent":{"other":"guardian"}}}}"#,
            "\n",
            r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Review parent task"}]}}"#,
        ),
    )
    .expect("write structured-source Codex child session");
    fs::write(
        session_root.join("rollout-legacy.jsonl"),
        concat!(
            r#"{"type":"session_meta","payload":{"id":"legacy-session","source":"vscode"}}"#,
            "\n",
            r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Legacy user task"}]}}"#,
        ),
    )
    .expect("write legacy top-level Codex session");
    fs::write(
        session_root.join("rollout-exec-carrier.jsonl"),
        concat!(
            r#"{"type":"session_meta","payload":{"id":"exec-carrier-session","source":"exec","thread_source":"user"}}"#,
            "\n",
            r#"{"type":"response_item","payload":{"type":"message","role":"user","content":[{"type":"input_text","text":"Carrier thread for internal handoff"}]}}"#,
        ),
    )
    .expect("write non-interactive Codex exec session");
    let host = ServiceHost {
        app_data_dir: fixture.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home,
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };

    let _codex_home = EnvVarGuard::set("CODEX_HOME", &codex_home);
    let response = host.handle(ServiceRequest {
        id: Some("custom-codex-subagent".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({"agent": "codex", "limit": 10, "max_excerpt_chars": 4_000}),
    });
    assert!(response.ok, "{:?}", response.error);
    let result = response
        .result
        .expect("custom Codex subagent preview result");

    assert_eq!(result["total_candidate_count"], json!(7), "{result}");
    assert_eq!(result["total_matched_count"], json!(2), "{result}");
    assert_eq!(result["count"], json!(2), "{result}");
    let serialized =
        serde_json::to_string(&result).expect("serialize custom Codex internal preview");
    for visible in ["Parent task", "Legacy user task"] {
        assert!(serialized.contains(visible), "{serialized}");
    }
    for hidden in [
        "child-session",
        "host-session",
        "memory-session",
        "structured-child-session",
        "exec-carrier-session",
        "Generate an internal pet asset",
        "Consolidate memory",
    ] {
        assert!(!serialized.contains(hidden), "{serialized}");
    }
    let _ = fs::remove_dir_all(fixture);
}
