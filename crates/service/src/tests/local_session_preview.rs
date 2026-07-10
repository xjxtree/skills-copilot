use super::*;

#[test]
fn local_session_preview_handles_non_ascii_skill_invocation_text() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-local-session-preview-non-ascii-skill-test-{}-{unique}",
        std::process::id(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-local-session-preview-non-ascii-skill-home-{}-{unique}",
        std::process::id(),
    ));
    let session_root = user_home.join(".codex/sessions/2026/06/22");
    fs::create_dir_all(&session_root).expect("create codex session root");
    fs::write(
        session_root.join("rollout-2026-06-22T08-00-00-non-ascii-skill-fixture.jsonl"),
        "{\"role\":\"user\",\"content\":\"Use skill:配置检查 then skill:fixture-session-skill for diagnostics\"}\n",
    )
    .expect("write codex session");
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
        id: Some("session-preview-non-ascii-skill".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "codex",
            "limit": 10,
            "max_excerpt_chars": 800
        }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("local session preview result");
    assert_eq!(result.get("count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        result.get("skill_call_count").and_then(Value::as_u64),
        Some(1)
    );
    assert!(result
        .pointer("/session_rows/0/content_items")
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| {
            item.get("kind").and_then(Value::as_str) == Some("skill_call")
                && item
                    .get("text")
                    .and_then(Value::as_str)
                    .is_some_and(|text| text.contains("fixture-session-skill"))
        })));
    assert!(!provider_call_metadata_path(&app_data_dir).exists());

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn local_session_preview_keeps_codex_resumed_user_messages_with_timestamps() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-local-session-preview-resume-test-{}-{unique}",
        std::process::id(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-local-session-preview-resume-home-{}-{unique}",
        std::process::id(),
    ));
    let session_root = user_home.join(".codex/sessions/2026/06/16");
    fs::create_dir_all(&session_root).expect("create codex session root");
    let first_user = json!({
        "timestamp": "2026-06-16T10:05:39.910Z",
        "type": "response_item",
        "payload": {
            "type": "message",
            "role": "user",
            "content": [{ "type": "input_text", "text": "调研并设计灰度发布方案" }]
        }
    });
    let resumed_user = json!({
        "timestamp": "2026-06-25T10:21:57.197Z",
        "type": "response_item",
        "payload": {
            "type": "message",
            "role": "user",
            "content": [{
                "type": "input_text",
                "text": "针对灰度方案与 runbook文档，现在确定以下信息：\\n1. apps/sdk-aliyun-route.yaml、apps/sdk-aliyun-route-gf.yaml 没有 gitops同步，目前是kubectl apply。"
            }]
        }
    });
    fs::write(
        session_root.join("rollout-2026-06-16T18-05-31-fixture.jsonl"),
        format!("{first_user}\n{resumed_user}\n"),
    )
    .expect("write codex resumed session");
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
        id: Some("session-preview-resume".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "codex",
            "limit": 10,
            "max_excerpt_chars": 800
        }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("local session preview result");
    assert_eq!(
        result.get("user_message_count").and_then(Value::as_u64),
        Some(2)
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/started_at")
            .and_then(Value::as_i64),
        Some(1_781_604_339_910)
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/ended_at")
            .and_then(Value::as_i64),
        Some(1_782_382_917_197)
    );
    let content_items = result
        .pointer("/session_rows/0/content_items")
        .and_then(Value::as_array)
        .expect("content items");
    let resumed_item = content_items
        .iter()
        .find(|item| {
            item.get("text")
                .and_then(Value::as_str)
                .is_some_and(|text| text.contains("针对灰度方案与 runbook文档"))
        })
        .expect("resumed user message");
    assert_eq!(
        resumed_item.get("kind").and_then(Value::as_str),
        Some("user_message")
    );
    assert_eq!(
        resumed_item.get("timestamp").and_then(Value::as_i64),
        Some(1_782_382_917_197)
    );

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn local_session_preview_redacts_unix_listing_owners() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-local-session-preview-owner-redaction-test-{}-{unique}",
        std::process::id(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-local-session-preview-owner-redaction-home-{}-{unique}",
        std::process::id(),
    ));
    let session_root = user_home.join(".codex/sessions/2026/06/21");
    fs::create_dir_all(&session_root).expect("create codex session root");
    let user_line = json!({
        "role": "user",
        "content": "show repository files"
    });
    let assistant_line = json!({
        "role": "assistant",
        "content": "total 8\n-rw-r--r--@ 1 localuser staff 234 Jun 21 16:34 README.md\ndrwxr-xr-x 12 localuser staff 384 Jun 21 16:35 docs",
        "tool_calls": [{ "name": "shell" }]
    });
    fs::write(
        session_root.join("rollout-2026-06-21T08-00-00-owner-fixture.jsonl"),
        format!("{user_line}\n{assistant_line}\n"),
    )
    .expect("write codex session");
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
        id: Some("session-preview-owner-redaction".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "codex",
            "limit": 10,
            "max_excerpt_chars": 1200
        }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("local session preview result");
    let serialized = serde_json::to_string(&result).expect("serialize local session result");
    assert!(!serialized.contains("localuser staff"), "{serialized}");
    assert!(serialized.contains("<user> <group>"));

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn local_session_preview_ignores_claude_tool_result_sidecars() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-local-session-claude-sidecar-test-{}-{unique}",
        std::process::id(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-local-session-claude-sidecar-home-{}-{unique}",
        std::process::id(),
    ));
    let project_root = app_data_dir.join("project-root");
    let project_session_root = user_home
        .join(".claude/projects")
        .join(encoded_project_session_dir(&project_root));
    fs::create_dir_all(&project_root).expect("create project root");
    fs::create_dir_all(project_session_root.join("session-claude/tool-results"))
        .expect("create claude tool result directory");
    fs::write(
        project_session_root.join("session-claude.jsonl"),
        format!(
            "{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":\"打开最新版 app\"}},\"cwd\":\"{}\",\"sessionId\":\"session-claude\"}}\n{{\"type\":\"ai-title\",\"aiTitle\":\"打开最新版 app\",\"sessionId\":\"session-claude\"}}\n",
            json_path_text(&project_root)
        ),
    )
    .expect("write claude session");
    fs::write(
        project_session_root.join("session-claude/tool-results/b1.txt"),
        "$ cargo fmt --all -- --check\n",
    )
    .expect("write claude tool result sidecar");
    let host = ServiceHost {
        app_data_dir: app_data_dir.clone(),
        adapter_ctx: AdapterContext {
            user_home: user_home.clone(),
            project_root: Some(project_root.clone()),
            project_cwd: Some(project_root.clone()),
            extra_roots: Vec::new(),
        },
    };

    let response = host.handle(ServiceRequest {
        id: Some("session-preview-claude-sidecar".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "claude-code",
            "scope": "project",
            "limit": 10,
            "max_excerpt_chars": 800
        }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("local session preview result");
    assert_eq!(
        result.get("count").and_then(Value::as_u64),
        Some(1),
        "Claude tool result sidecars should not appear as independent sessions"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/title")
            .and_then(Value::as_str),
        Some("打开最新版 app")
    );
    let serialized = serde_json::to_string(&result).expect("serialize result");
    assert!(!serialized.contains("cargo fmt"));

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn local_session_preview_reads_past_large_claude_file_history_snapshots() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-local-session-large-snapshot-test-{}-{unique}",
        std::process::id(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-local-session-large-snapshot-home-{}-{unique}",
        std::process::id(),
    ));
    let project_root = app_data_dir.join("project-root");
    let project_session_root = user_home
        .join(".claude/projects")
        .join(encoded_project_session_dir(&project_root));
    fs::create_dir_all(&project_root).expect("create project root");
    fs::create_dir_all(&project_session_root).expect("create claude project session root");
    let large_snapshot = "x".repeat(600_000);
    fs::write(
        project_session_root.join("session-large-snapshot.jsonl"),
        format!(
            "{{\"type\":\"mode\",\"sessionId\":\"session-large-snapshot\"}}\n{{\"type\":\"file-history-snapshot\",\"content\":\"{}\"}}\n{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":\"继续验证会话识别\"}},\"cwd\":\"{}\",\"sessionId\":\"session-large-snapshot\"}}\n",
            large_snapshot,
            json_path_text(&project_root)
        ),
    )
    .expect("write claude session with large snapshot");
    let host = ServiceHost {
        app_data_dir: app_data_dir.clone(),
        adapter_ctx: AdapterContext {
            user_home: user_home.clone(),
            project_root: Some(project_root.clone()),
            project_cwd: Some(project_root.clone()),
            extra_roots: Vec::new(),
        },
    };

    let response = host.handle(ServiceRequest {
        id: Some("session-preview-large-snapshot".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "claude-code",
            "scope": "project",
            "limit": 10,
            "max_excerpt_chars": 800
        }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("local session preview result");
    assert_eq!(result.get("count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        result
            .pointer("/session_rows/0/title")
            .and_then(Value::as_str),
        Some("继续验证会话识别")
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/user_message_count")
            .and_then(Value::as_u64),
        Some(1)
    );

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn local_session_preview_compacts_large_claude_image_messages_for_titles() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-local-session-large-image-test-{}-{unique}",
        std::process::id(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-local-session-large-image-home-{}-{unique}",
        std::process::id(),
    ));
    let project_root = app_data_dir.join("project-root");
    let project_session_root = user_home
        .join(".claude/projects")
        .join(encoded_project_session_dir(&project_root));
    fs::create_dir_all(&project_root).expect("create project root");
    fs::create_dir_all(&project_session_root).expect("create claude project session root");
    let image_data = "a".repeat(120_000);
    let lines = [
        json!({
            "type": "user",
            "message": {
                "role": "user",
                "content": [
                    {"type": "text", "text": "[Image #1]"},
                    {"type": "image", "source": {"type": "base64", "media_type": "image/png", "data": image_data}},
                    {"type": "text", "text": "截图里会话识别是不是有问题"}
                ]
            },
            "cwd": project_root.to_string_lossy(),
            "sessionId": "session-large-image"
        })
        .to_string(),
    ]
    .join("\n");
    fs::write(
        project_session_root.join("session-large-image.jsonl"),
        lines,
    )
    .expect("write large image claude session");
    let host = ServiceHost {
        app_data_dir: app_data_dir.clone(),
        adapter_ctx: AdapterContext {
            user_home: user_home.clone(),
            project_root: Some(project_root.clone()),
            project_cwd: Some(project_root.clone()),
            extra_roots: Vec::new(),
        },
    };

    let response = host.handle(ServiceRequest {
        id: Some("session-preview-large-image".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "claude-code",
            "scope": "project",
            "limit": 10,
            "max_excerpt_chars": 800
        }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("local session preview result");
    assert_eq!(result.get("count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        result
            .pointer("/session_rows/0/title")
            .and_then(Value::as_str),
        Some("截图里会话识别是不是有问题")
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/user_message_count")
            .and_then(Value::as_u64),
        Some(1)
    );

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn local_session_preview_skips_claude_local_command_caveat_titles() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-local-session-caveat-title-test-{}-{unique}",
        std::process::id(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-local-session-caveat-title-home-{}-{unique}",
        std::process::id(),
    ));
    let project_root = app_data_dir.join("project-root");
    let project_session_root = user_home
        .join(".claude/projects")
        .join(encoded_project_session_dir(&project_root));
    fs::create_dir_all(&project_root).expect("create project root");
    fs::create_dir_all(&project_session_root).expect("create claude project session root");
    fs::write(
        project_session_root.join("session-caveat-title.jsonl"),
        format!(
            "{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":\"<local-command-caveat>Caveat: generated by local command runner</local-command-caveat>\"}},\"cwd\":\"{}\",\"sessionId\":\"session-caveat-title\"}}\n{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":\"clear\"}},\"cwd\":\"{}\",\"sessionId\":\"session-caveat-title\"}}\n{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":\"<command-args></command-args>\"}},\"cwd\":\"{}\",\"sessionId\":\"session-caveat-title\"}}\n{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":\"全量检查其他 agent 会话\"}},\"cwd\":\"{}\",\"sessionId\":\"session-caveat-title\"}}\n",
            json_path_text(&project_root),
            json_path_text(&project_root),
            json_path_text(&project_root),
            json_path_text(&project_root)
        ),
    )
    .expect("write claude session with caveat");
    let host = ServiceHost {
        app_data_dir: app_data_dir.clone(),
        adapter_ctx: AdapterContext {
            user_home: user_home.clone(),
            project_root: Some(project_root.clone()),
            project_cwd: Some(project_root.clone()),
            extra_roots: Vec::new(),
        },
    };

    let response = host.handle(ServiceRequest {
        id: Some("session-preview-caveat-title".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "claude-code",
            "scope": "project",
            "limit": 10,
            "max_excerpt_chars": 800
        }),
    });

    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("local session preview result");
    assert_eq!(result.get("count").and_then(Value::as_u64), Some(1));
    assert_eq!(
        result
            .pointer("/session_rows/0/title")
            .and_then(Value::as_str),
        Some("全量检查其他 agent 会话")
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/user_message_count")
            .and_then(Value::as_u64),
        Some(1),
        "Internal caveat messages should not count as user prompts"
    );

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn local_session_preview_keeps_tail_message_after_large_middle_record() {
    let initial = json!({
        "type": "user",
        "role": "user",
        "text": "clear",
        "timestamp": "2026-07-10T08:00:00Z"
    });
    let middle = json!({
        "type": "file-history-snapshot",
        "data": "x".repeat(600 * 1024)
    });
    let final_event = json!({
        "type": "user",
        "role": "user",
        "text": "tail-event-visible",
        "timestamp": "2026-07-10T08:09:10Z"
    });
    let result = preview_codex_session_fixture(
        "large-middle-tail-message",
        &format!("{initial}\n{middle}\n{final_event}\n"),
    );

    assert_eq!(
        result
            .pointer("/session_rows/0/title")
            .and_then(Value::as_str),
        Some("tail-event-visible")
    );
    assert!(result
        .pointer("/session_rows/0/content_items")
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| {
            item.get("text").and_then(Value::as_str) == Some("tail-event-visible")
        })));
}

#[test]
fn local_session_preview_keeps_tail_timestamp_after_read_cap() {
    let initial = json!({
        "type": "user",
        "role": "user",
        "text": "clear",
        "timestamp": "2026-07-10T08:00:00Z"
    });
    let middle = json!({
        "type": "file-history-snapshot",
        "data": "y".repeat(600 * 1024)
    });
    let final_event = json!({
        "type": "user",
        "role": "user",
        "text": "tail-timestamp-visible",
        "timestamp": "2026-07-10T08:09:10Z"
    });
    let result = preview_codex_session_fixture(
        "large-middle-tail-timestamp",
        &format!("{initial}\n{middle}\n{final_event}\n"),
    );

    assert_eq!(
        result
            .pointer("/session_rows/0/ended_at")
            .and_then(Value::as_i64),
        Some(1_783_670_950_000)
    );
}

#[test]
fn local_session_preview_bounds_single_line_json() {
    let filler = "forbidden-session-blob-".repeat(30_000);
    let session = format!(
        concat!(
            "{{\"type\":\"mode\"}}\n",
            "{{\"type\":\"user\",\"role\":\"user\",",
            "\"text\":\"single-line-tail-visible\",",
            "\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
            "{{\"type\":\"file-history-snapshot\",\"data\":{}}}\n"
        ),
        serde_json::to_string(&filler).expect("serialize filler")
    );
    let result = preview_codex_session_fixture("single-line-bounded-json", &session);
    let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

    assert!(
        serialized.contains("single-line-tail-visible"),
        "{serialized}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/title")
            .and_then(Value::as_str),
        Some("single-line-tail-visible")
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/content_items/0/kind")
            .and_then(Value::as_str),
        Some("user_message")
    );
    assert!(!serialized.contains("forbidden-session-blob-"));
    assert!(!serialized.contains("\"data\""));
}

#[test]
fn local_session_preview_drops_complete_json_prefix_without_a_physical_record_boundary() {
    const HEAD_BYTES: usize = 384 * 1024;
    let marker = "UNPROVEN_HEAD_JSON_MUST_NOT_SURFACE";
    let prefix = format!(
        "{{\"type\":\"user\",\"role\":\"user\",\"text\":{},\"padding\":\"",
        serde_json::to_string(marker).expect("serialize marker")
    );
    let suffix = "\"}";
    let padding = "x".repeat(HEAD_BYTES - prefix.len() - suffix.len());
    let complete_prefix = format!("{prefix}{padding}{suffix}");
    assert_eq!(complete_prefix.len(), HEAD_BYTES);
    let session = format!("{complete_prefix}{}\n", "z".repeat(300 * 1024));

    let result = preview_codex_session_fixture("unproven-complete-head-json", &session);
    let output = serde_json::to_string(&result).expect("serialize preview");

    assert!(!output.contains(marker), "{output}");
    assert_eq!(result.get("count").and_then(Value::as_u64), Some(0));
}

#[test]
fn local_session_preview_drops_plaintext_head_fragment_without_a_physical_record_boundary() {
    const HEAD_BYTES: usize = 384 * 1024;
    let marker = "UNPROVEN_HEAD_PLAINTEXT_MUST_NOT_SURFACE";
    let prefix = format!("user: {marker}");
    let head = format!("{prefix}{}", "x".repeat(HEAD_BYTES - prefix.len()));
    assert_eq!(head.len(), HEAD_BYTES);
    let session = format!("{head}{}\n", "z".repeat(300 * 1024));

    let result =
        preview_codex_session_fixture_with_extension("unproven-plaintext-head", "log", &session);
    let output = serde_json::to_string(&result).expect("serialize preview");

    assert!(!output.contains(marker), "{output}");
    assert_eq!(result.get("count").and_then(Value::as_u64), Some(0));
}

#[test]
fn local_session_preview_keeps_head_records_proven_at_newline_or_eof() {
    let newline_marker = "PROVEN_HEAD_NEWLINE_REMAINS_VISIBLE";
    let newline_record = json!({
        "type": "user",
        "role": "user",
        "text": newline_marker
    });
    let newline_session = format!("{newline_record}\n{}\n", "x".repeat(700 * 1024));
    let newline_result = preview_codex_session_fixture("proven-head-newline", &newline_session);
    assert!(
        serde_json::to_string(&newline_result)
            .expect("serialize newline preview")
            .contains(newline_marker),
        "{newline_result}"
    );

    for (case, extension, content, marker) in [
        (
            "proven-json-eof",
            "jsonl",
            json!({"type":"user","role":"user","text":"PROVEN_JSON_EOF_VISIBLE"}).to_string(),
            "PROVEN_JSON_EOF_VISIBLE",
        ),
        (
            "proven-plaintext-eof",
            "log",
            "user: PROVEN_PLAINTEXT_EOF_VISIBLE".to_string(),
            "PROVEN_PLAINTEXT_EOF_VISIBLE",
        ),
    ] {
        let result = preview_codex_session_fixture_with_extension(case, extension, &content);
        assert!(
            serde_json::to_string(&result)
                .expect("serialize EOF preview")
                .contains(marker),
            "{case}: {result}"
        );
    }
}

#[test]
fn local_session_preview_keeps_head_json_with_bounded_continuity_proof() {
    const HEAD_BYTES: usize = 384 * 1024;
    let marker = "BOUNDED_CONTINUITY_PROOF_REMAINS_VISIBLE";
    let prefix = format!(
        "{{\"type\":\"user\",\"role\":\"user\",\"text\":{},\"padding\":\"",
        serde_json::to_string(marker).expect("serialize marker")
    );
    let suffix = "\"}";
    let padding = "x".repeat(HEAD_BYTES - prefix.len() - suffix.len());
    let complete_prefix = format!("{prefix}{padding}{suffix}");
    assert_eq!(complete_prefix.len(), HEAD_BYTES);

    for (case, terminator) in [("eof", ""), ("newline", "\n")] {
        let session = format!("{complete_prefix}{}{terminator}", " ".repeat(200 * 1024));
        let result = preview_codex_session_fixture(&format!("bounded-continuity-{case}"), &session);
        let output = serde_json::to_string(&result).expect("serialize preview");

        assert!(output.contains(marker), "{case}: {output}");
        assert_eq!(
            result
                .pointer("/session_rows/0/user_message_count")
                .and_then(Value::as_u64),
            Some(1),
            "{case}: {result}"
        );
    }
}

#[test]
fn one_oversized_json_record_recovers_supported_tail_scalars() {
    let value = format!(
        "{{\"type\":\"user\",\"role\":\"user\",\"data\":\"{}\",\"text\":\"FINAL_SCALAR_MUST_SURVIVE\",\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
        "x".repeat(600 * 1024)
    );
    let result = preview_codex_session_fixture("single-record-tail-scalar", &value);
    let output = serde_json::to_string(&result).expect("serialize single-record preview");

    assert!(output.contains("FINAL_SCALAR_MUST_SURVIVE"), "{output}");
    assert_eq!(
        result
            .pointer("/session_rows/0/ended_at")
            .and_then(Value::as_i64),
        Some(1_783_670_950_000),
        "{result}"
    );
    assert!(!output.contains("\"data\""), "{output}");
}

#[test]
fn oversized_hidden_record_with_tail_timestamp_stays_hidden() {
    for record_type in [
        "attachment",
        "file-history-snapshot",
        "last-prompt",
        "mode",
        "permission-mode",
        "queue-operation",
    ] {
        let marker = format!(
            "HIDDEN_{}_TAIL_MUST_NOT_SURFACE",
            record_type.replace('-', "_").to_ascii_uppercase()
        );
        let session = format!(
            "{{\"type\":{record_type},\"data\":\"{}\",\"text\":{marker},\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
            "x".repeat(600 * 1024),
            record_type = serde_json::to_string(record_type).expect("serialize record type"),
            marker = serde_json::to_string(&marker).expect("serialize marker"),
        );
        let result = preview_codex_session_fixture(
            &format!("hidden-tail-timestamp-{record_type}"),
            &session,
        );
        let output = serde_json::to_string(&result).expect("serialize hidden preview");

        assert!(!output.contains(&marker), "{record_type}: {output}");
        assert_eq!(
            result.get("count").and_then(Value::as_u64),
            Some(0),
            "{record_type}: {result}"
        );
    }
}

#[test]
fn oversized_hidden_type_in_unread_gap_stays_hidden() {
    let session = format!(
        "{{\"text\":\"HIDDEN_GAP_TYPE_MUST_NOT_SURFACE\",\"data\":\"{}\",\"type\":\"file-history-snapshot\",\"blob\":\"{}\",\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
        "a".repeat(400 * 1024),
        "b".repeat(200 * 1024)
    );
    let result = preview_codex_session_fixture("hidden-gap-type", &session);
    let output = serde_json::to_string(&result).expect("serialize hidden preview");

    assert!(
        !output.contains("HIDDEN_GAP_TYPE_MUST_NOT_SURFACE"),
        "{output}"
    );
    assert_eq!(
        result.get("count").and_then(Value::as_u64),
        Some(0),
        "{result}"
    );
}

#[test]
fn oversized_hidden_type_in_discarded_tail_alignment_prefix_stays_hidden() {
    let session = format!(
        "{{\"text\":\"HIDDEN_TAIL_PREFIX_TYPE_MUST_NOT_SURFACE\",\"data\":\"{}\",\"type\":\"file-history-snapshot\",\"blob\":\"{}\",\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
        "a".repeat(440 * 1024),
        "b".repeat(160 * 1024)
    );
    let result = preview_codex_session_fixture("hidden-tail-prefix-type", &session);
    let output = serde_json::to_string(&result).expect("serialize hidden preview");

    assert!(
        !output.contains("HIDDEN_TAIL_PREFIX_TYPE_MUST_NOT_SURFACE"),
        "{output}"
    );
    assert_eq!(
        result.get("count").and_then(Value::as_u64),
        Some(0),
        "{result}"
    );
}

#[test]
fn later_hidden_duplicate_type_in_unread_gap_overrides_visible_head_type() {
    let session = format!(
        "{{\"type\":\"user\",\"role\":\"user\",\"text\":\"HIDDEN_DUPLICATE_GAP_TYPE_MUST_NOT_SURFACE\",\"data\":\"{}\",\"type\":\"file-history-snapshot\",\"blob\":\"{}\",\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
        "a".repeat(400 * 1024),
        "b".repeat(200 * 1024)
    );
    let result = preview_codex_session_fixture("hidden-duplicate-gap-type", &session);
    let output = serde_json::to_string(&result).expect("serialize hidden preview");

    assert!(
        !output.contains("HIDDEN_DUPLICATE_GAP_TYPE_MUST_NOT_SURFACE"),
        "{output}"
    );
    assert_eq!(
        result.get("count").and_then(Value::as_u64),
        Some(0),
        "{result}"
    );
}

#[test]
fn oversized_non_message_classifications_never_become_plain_text_replies() {
    for (region, first_blob_bytes) in [("gap", 400 * 1024), ("tail-prefix", 440 * 1024)] {
        for record_type in ["developer", "summary", "tool", "tool_result"] {
            let marker = format!(
                "NON_MESSAGE_{}_{}_MUST_NOT_BECOME_REPLY",
                region.replace('-', "_").to_ascii_uppercase(),
                record_type.to_ascii_uppercase()
            );
            let session = format!(
                "{{\"type\":\"user\",\"role\":\"user\",\"text\":{marker},\"data\":\"{}\",\"type\":{record_type},\"blob\":\"{}\",\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
                "a".repeat(first_blob_bytes),
                "b".repeat((600_usize * 1024).saturating_sub(first_blob_bytes)),
                marker = serde_json::to_string(&marker).expect("serialize marker"),
                record_type = serde_json::to_string(record_type).expect("serialize type"),
            );
            let result = preview_codex_session_fixture(
                &format!("non-message-{region}-{record_type}"),
                &session,
            );
            let output = serde_json::to_string(&result).expect("serialize preview");

            assert!(
                !output.contains(&marker),
                "{region}/{record_type}: {output}"
            );
        }
    }
}

#[test]
fn later_non_message_type_overrides_old_user_classification() {
    for record_type in ["developer", "summary"] {
        let marker = format!(
            "LATER_{}_MUST_OVERRIDE_OLD_USER",
            record_type.to_ascii_uppercase()
        );
        let session = format!(
            "{{\"type\":\"user\",\"role\":\"user\",\"text\":{marker},\"data\":\"{}\",\"type\":{record_type},\"blob\":\"{}\",\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
            "a".repeat(400 * 1024),
            "b".repeat(200 * 1024),
            marker = serde_json::to_string(&marker).expect("serialize marker"),
            record_type = serde_json::to_string(record_type).expect("serialize type"),
        );
        let result =
            preview_codex_session_fixture(&format!("later-non-message-{record_type}"), &session);
        let output = serde_json::to_string(&result).expect("serialize preview");

        assert!(!output.contains(&marker), "{record_type}: {output}");
    }
}

#[test]
fn unusable_later_classification_clears_old_visible_classification() {
    for (field, prefix) in [
        ("type", "\"type\":\"user\",\"role\":\"user\","),
        ("role", "\"role\":\"user\","),
    ] {
        for (case, later_value) in [
            ("null", "null".to_string()),
            ("object", "{}".to_string()),
            (
                "oversized",
                serde_json::to_string(&"x".repeat(5 * 1024))
                    .expect("serialize oversized classification"),
            ),
        ] {
            let marker = format!(
                "UNUSABLE_LATER_{}_{}_MUST_NOT_SURFACE",
                field.to_ascii_uppercase(),
                case.to_ascii_uppercase()
            );
            let session = format!(
                "{{{prefix}\"text\":{marker},\"data\":\"{}\",\"{field}\":{later_value},\"blob\":\"{}\"}}\n",
                "a".repeat(400 * 1024),
                "b".repeat(200 * 1024),
                marker = serde_json::to_string(&marker).expect("serialize marker"),
            );
            let result =
                preview_codex_session_fixture(&format!("unusable-later-{field}-{case}"), &session);
            let output = serde_json::to_string(&result).expect("serialize preview");

            assert!(!output.contains(&marker), "{field}/{case}: {output}");
        }
    }
}

#[test]
fn final_deny_or_unusable_role_overrides_visible_type_during_recovery() {
    for (case, later_role) in [
        (
            "developer",
            serde_json::to_string("developer").expect("serialize role"),
        ),
        (
            "summary",
            serde_json::to_string("summary").expect("serialize role"),
        ),
        (
            "tool",
            serde_json::to_string("tool").expect("serialize role"),
        ),
        ("null", "null".to_string()),
        ("object", "{}".to_string()),
        (
            "oversized",
            serde_json::to_string(&"r".repeat(5 * 1024)).expect("serialize oversized role"),
        ),
    ] {
        let marker = format!("FINAL_ROLE_{}_MUST_NOT_SURFACE", case.to_ascii_uppercase());
        let session = format!(
            "{{\"type\":\"user\",\"role\":\"user\",\"text\":{marker},\"data\":\"{}\",\"role\":{later_role},\"blob\":\"{}\"}}\n",
            "a".repeat(400 * 1024),
            "b".repeat(200 * 1024),
            marker = serde_json::to_string(&marker).expect("serialize marker"),
        );
        let result = preview_codex_session_fixture(&format!("final-role-{case}"), &session);
        let output = serde_json::to_string(&result).expect("serialize preview");

        assert!(!output.contains(&marker), "{case}: {output}");
        assert_eq!(
            result.get("count").and_then(Value::as_u64),
            Some(0),
            "{case}: {result}"
        );
    }
}

#[test]
fn complete_conflicting_classifications_use_fail_closed_precedence() {
    for (role, expected_kind) in [
        ("developer", None),
        ("summary", None),
        ("tool", Some("tool_call")),
    ] {
        let marker = format!("COMPLETE_CONFLICTING_{}_CONTENT", role.to_ascii_uppercase());
        let session = json!({
            "type": "user",
            "role": role,
            "text": marker
        })
        .to_string();
        let result = preview_codex_session_fixture(&format!("complete-conflict-{role}"), &session);

        assert_eq!(
            result
                .pointer("/session_rows/0/user_message_count")
                .and_then(Value::as_u64),
            Some(0),
            "{role}: {result}"
        );
        let items = result
            .pointer("/session_rows/0/content_items")
            .and_then(Value::as_array)
            .expect("content items");
        match expected_kind {
            Some(kind) => assert_eq!(
                items
                    .first()
                    .and_then(|item| item.get("kind"))
                    .and_then(Value::as_str),
                Some(kind),
                "{role}: {result}"
            ),
            None => assert!(items.is_empty(), "{role}: {result}"),
        }
    }
}

#[test]
fn complete_deny_record_blocks_all_nested_content_extraction() {
    let tool_marker = "DENY_TOOL_MUST_NOT_SURFACE";
    let user_marker = "DENY_NESTED_USER_MUST_NOT_SURFACE";
    let session = json!({
        "type": "developer",
        "role": "tool",
        "tool_calls": [{
            "name": tool_marker,
            "arguments": { "secret": "nested" }
        }],
        "data": {
            "type": "user",
            "role": "user",
            "text": user_marker
        }
    })
    .to_string();

    let result = preview_codex_session_fixture("complete-deny-nested-content", &session);

    assert!(
        result
            .pointer("/session_rows/0/content_items")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty),
        "{result}"
    );
    for metric in [
        "tool_call_count",
        "user_message_count",
        "total_message_count",
    ] {
        assert_eq!(
            result
                .pointer(&format!("/session_rows/0/{metric}"))
                .and_then(Value::as_u64),
            Some(0),
            "{metric}: {result}"
        );
    }
}

#[test]
fn denied_descendants_block_every_specialized_tool_path() {
    let session = json!([
        {
            "content": [{
                "type": "developer",
                "role": "developer",
                "content": [{
                    "type": "tool_call",
                    "name": "blocked_content_tool",
                    "arguments": { "marker": "denied content tool payload" }
                }]
            }]
        },
        {
            "parts": [{
                "type": "system",
                "role": "system",
                "parts": [{
                    "type": "tool_call",
                    "name": "blocked_parts_tool",
                    "arguments": { "marker": "denied parts tool payload" }
                }]
            }]
        },
        {
            "message": {
                "type": "summary",
                "role": "summary",
                "content": [{
                    "type": "tool_call",
                    "name": "blocked_message_tool",
                    "arguments": { "marker": "denied message tool payload" }
                }]
            }
        },
        {
            "tool_calls": [{
                "type": "tool_call",
                "role": null,
                "name": "blocked_invalid_tool",
                "arguments": { "marker": "denied invalid tool payload" }
            }]
        },
        {
            "type": "user",
            "role": "user",
            "text": "accepted sibling remains visible"
        }
    ])
    .to_string();

    let result = preview_codex_session_fixture("deny-specialized-tool-paths", &session);
    let items = result
        .pointer("/session_rows/0/content_items")
        .and_then(Value::as_array)
        .expect("content items");

    assert_eq!(items.len(), 1, "{result}");
    assert_eq!(
        items[0].get("text").and_then(Value::as_str),
        Some("accepted sibling remains visible"),
        "{result}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/tool_call_count")
            .and_then(Value::as_u64),
        Some(0),
        "{result}"
    );
    let serialized = serde_json::to_string(&result).expect("serialize denied tool result");
    for hidden in [
        "denied content tool payload",
        "denied parts tool payload",
        "denied message tool payload",
        "denied invalid tool payload",
    ] {
        assert!(
            !serialized.contains(hidden),
            "surfaced {hidden}: {serialized}"
        );
    }
}

#[test]
fn complete_classification_uses_raw_token_spans_and_last_wins() {
    let over_limit_token = format!("\"{}user\"", "\\u005f".repeat(700));
    let exact_limit_token = format!("\"{}____user\"", "\\u005f".repeat(681));
    assert!(over_limit_token.len() > 4 * 1024);
    assert_eq!(exact_limit_token.len(), 4 * 1024);

    let session = format!(
        concat!(
            "[",
            "{{\"type\":\"user\",\"role\":{over},\"text\":\"raw over-limit role hidden\"}},",
            "{{\"type\":\"user\",\"r\\u006fle\":{over},\"text\":\"escaped role key hidden\"}},",
            "{{\"data\":{{\"type\":{over},\"role\":\"user\",\"text\":\"nested raw over-limit type hidden\"}}}},",
            "{{\"type\":\"user\",\"role\":{exact},\"text\":\"exact raw boundary visible\"}},",
            "{{\"type\":\"user\",\"role\":{over},\"role\":\"user\",\"text\":\"final supported duplicate visible\"}},",
            "{{\"type\":\"user\",\"role\":\"user\",\"role\":{over},\"text\":\"final over-limit duplicate hidden\"}},",
            "{{\"type\":\"assistant\",\"role\":\"assistant\",\"text\":\"positive array sibling visible\"}}",
            "]"
        ),
        over = over_limit_token,
        exact = exact_limit_token,
    );

    let result = preview_codex_session_fixture("raw-classification-spans", &session);
    let items = result
        .pointer("/session_rows/0/content_items")
        .and_then(Value::as_array)
        .expect("content items");
    let texts = items
        .iter()
        .filter_map(|item| item.get("text").and_then(Value::as_str))
        .collect::<Vec<_>>();

    assert_eq!(
        result
            .pointer("/session_rows/0/user_message_count")
            .and_then(Value::as_u64),
        Some(2),
        "{result}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/total_message_count")
            .and_then(Value::as_u64),
        Some(3),
        "{result}"
    );
    for visible in [
        "exact raw boundary visible",
        "final supported duplicate visible",
        "positive array sibling visible",
    ] {
        assert!(texts.contains(&visible), "missing {visible}: {result}");
    }
    for hidden in [
        "raw over-limit role hidden",
        "escaped role key hidden",
        "nested raw over-limit type hidden",
        "final over-limit duplicate hidden",
    ] {
        assert!(!texts.contains(&hidden), "surfaced {hidden}: {result}");
    }
    let serialized = serde_json::to_string(&result).expect("serialize raw-bound result");
    for hidden in [
        "raw over-limit role hidden",
        "escaped role key hidden",
        "nested raw over-limit type hidden",
        "final over-limit duplicate hidden",
    ] {
        assert!(
            !serialized.contains(hidden),
            "surfaced {hidden}: {serialized}"
        );
    }
}

#[test]
fn skill_invocations_only_come_from_accepted_records() {
    let skill_name = "accepted-session-skill";
    let denied = preview_codex_session_fixture_with_catalog_skill(
        "deny-skill-invocation",
        &json!([
            {
                "type": "developer",
                "role": "developer",
                "text": format!("skill:{skill_name}")
            },
            {
                "type": "user",
                "role": "user",
                "text": "accepted non-skill sibling"
            }
        ])
        .to_string(),
        skill_name,
    );

    assert_eq!(
        denied
            .pointer("/session_rows/0/skill_call_count")
            .and_then(Value::as_u64),
        Some(0),
        "{denied}"
    );
    assert_eq!(
        denied.get("skill_call_count").and_then(Value::as_u64),
        Some(0),
        "{denied}"
    );
    assert!(
        denied
            .pointer("/session_rows/0/content_items")
            .and_then(Value::as_array)
            .is_some_and(|items| items
                .iter()
                .all(|item| item.get("kind").and_then(Value::as_str) != Some("skill_call"))),
        "{denied}"
    );
    assert!(
        !serde_json::to_string(&denied)
            .expect("serialize denied skill result")
            .contains(&format!("skill:{skill_name}")),
        "{denied}"
    );
    assert!(
        denied
            .get("skill_usage_rows")
            .and_then(Value::as_array)
            .is_some_and(Vec::is_empty),
        "{denied}"
    );

    let accepted = preview_codex_session_fixture_with_catalog_skill(
        "accepted-skill-invocation",
        &json!([
            {
                "type": "user",
                "role": "user",
                "text": format!("skill:{skill_name}")
            },
            {
                "type": "assistant",
                "role": "assistant",
                "text": format!("skill:{skill_name}")
            },
            {
                "type": "tool_call",
                "role": "assistant",
                "name": "accepted_tool",
                "arguments": { "invocation": format!("skill:{skill_name}") }
            }
        ])
        .to_string(),
        skill_name,
    );
    assert_eq!(
        accepted
            .pointer("/session_rows/0/skill_call_count")
            .and_then(Value::as_u64),
        Some(3),
        "{accepted}"
    );
    assert_eq!(
        accepted
            .pointer("/skill_usage_rows/0/call_count")
            .and_then(Value::as_u64),
        Some(3),
        "{accepted}"
    );
    assert!(
        accepted
            .pointer("/session_rows/0/content_items")
            .and_then(Value::as_array)
            .is_some_and(|items| items
                .iter()
                .any(|item| item.get("kind").and_then(Value::as_str) == Some("skill_call"))),
        "{accepted}"
    );
}

#[test]
fn structural_skill_markers_do_not_count_as_invocations() {
    let result = preview_codex_session_fixture(
        "structural-skill-markers",
        &json!({
            "type": "user",
            "role": "user",
            "id": "skill:structural-id",
            "title": "skill:structural-title",
            "cwd": "skill:structural-cwd",
            "path": "skill:structural-path",
            "metadata": {
                "note": "skill:structural-metadata",
                "event": {
                    "type": "user",
                    "role": "user",
                    "text": "skill:structural-metadata-event"
                }
            },
            "text": "accepted message without a skill invocation"
        })
        .to_string(),
    );
    let serialized = serde_json::to_string(&result).expect("serialize structural skill result");

    assert_eq!(
        result
            .pointer("/session_rows/0/skill_call_count")
            .and_then(Value::as_u64),
        Some(0),
        "{result}"
    );
    assert!(
        result
            .pointer("/session_rows/0/content_items")
            .and_then(Value::as_array)
            .is_some_and(|items| items
                .iter()
                .all(|item| item.get("kind").and_then(Value::as_str) != Some("skill_call"))),
        "{result}"
    );
    assert!(
        result
            .pointer("/session_rows/0/content_items")
            .and_then(Value::as_array)
            .is_some_and(|items| items.iter().all(|item| {
                item.get("kind").and_then(Value::as_str) != Some("skill_call")
                    && !item
                        .get("text")
                        .and_then(Value::as_str)
                        .is_some_and(|text| text.contains("structural-"))
            })),
        "{serialized}"
    );
}

#[test]
fn plain_and_root_string_deny_records_do_not_surface() {
    let result = preview_codex_session_fixture_with_extension(
        "plain-deny-records",
        "log",
        concat!(
            "developer: PLAIN_DEVELOPER_MUST_NOT_SURFACE skill:plain-denied\n",
            "summary : SPACED_SUMMARY_MUST_NOT_SURFACE skill:spaced-summary-denied\n",
            "\"system: ROOT_STRING_SYSTEM_MUST_NOT_SURFACE skill:string-denied\"\n",
            "user: accepted plain sibling\n",
        ),
    );
    let serialized = serde_json::to_string(&result).expect("serialize plain deny result");

    for hidden in [
        "PLAIN_DEVELOPER_MUST_NOT_SURFACE",
        "SPACED_SUMMARY_MUST_NOT_SURFACE",
        "ROOT_STRING_SYSTEM_MUST_NOT_SURFACE",
        "plain-denied",
        "spaced-summary-denied",
        "string-denied",
    ] {
        assert!(
            !serialized.contains(hidden),
            "surfaced {hidden}: {serialized}"
        );
    }
    assert!(
        serialized.contains("accepted plain sibling"),
        "{serialized}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/skill_call_count")
            .and_then(Value::as_u64),
        Some(0),
        "{result}"
    );
}

#[test]
fn malformed_json_shaped_deny_records_fail_closed_without_dropping_plaintext() {
    let result = preview_codex_session_fixture_with_extension(
        "malformed-json-shaped-deny",
        "log",
        concat!(
            "{system: MALFORMED_SYSTEM_MUST_NOT_SURFACE skill:malformed-system}\n",
            "{developer: MALFORMED_DEVELOPER_MUST_NOT_SURFACE skill:malformed-developer}\n",
            "[summary: MALFORMED_SUMMARY_MUST_NOT_SURFACE skill:malformed-summary]\n",
            "{plain CURLY_PLAINTEXT_MUST_REMAIN\n",
            "[INFO] BRACKET_PLAINTEXT_MUST_REMAIN\n",
            "user: accepted plain sibling\n",
        ),
    );
    let serialized = serde_json::to_string(&result).expect("serialize malformed deny result");

    for hidden in [
        "MALFORMED_SYSTEM_MUST_NOT_SURFACE",
        "MALFORMED_DEVELOPER_MUST_NOT_SURFACE",
        "MALFORMED_SUMMARY_MUST_NOT_SURFACE",
        "malformed-system",
        "malformed-developer",
        "malformed-summary",
    ] {
        assert!(
            !serialized.contains(hidden),
            "surfaced {hidden}: {serialized}"
        );
    }
    for visible in [
        "CURLY_PLAINTEXT_MUST_REMAIN",
        "BRACKET_PLAINTEXT_MUST_REMAIN",
        "accepted plain sibling",
    ] {
        assert!(
            serialized.contains(visible),
            "missing {visible}: {serialized}"
        );
    }
    assert_eq!(
        result
            .pointer("/session_rows/0/skill_call_count")
            .and_then(Value::as_u64),
        Some(0),
        "{result}"
    );
}

#[test]
fn pretty_printed_primary_records_preserve_only_accepted_descendants() {
    let session = serde_json::to_string_pretty(&json!([
        {
            "type": "developer",
            "role": "developer",
            "text": "PRETTY_PRIMARY_DENY_MUST_NOT_SURFACE skill:pretty-primary-denied"
        },
        {
            "type": "user",
            "role": "user",
            "text": "accepted pretty primary sibling"
        }
    ]))
    .expect("serialize pretty primary fixture");
    let result =
        preview_codex_session_fixture_with_extension("pretty-primary-deny", "json", &session);
    let serialized = serde_json::to_string(&result).expect("serialize pretty primary result");

    assert!(
        serialized.contains("accepted pretty primary sibling"),
        "{serialized}"
    );
    assert!(
        !serialized.contains("PRETTY_PRIMARY_DENY_MUST_NOT_SURFACE"),
        "{serialized}"
    );
    assert!(
        !serialized.contains("pretty-primary-denied"),
        "{serialized}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/user_message_count")
            .and_then(Value::as_u64),
        Some(1),
        "{result}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/skill_call_count")
            .and_then(Value::as_u64),
        Some(0),
        "{result}"
    );
}

#[test]
fn accepted_pretty_primary_is_one_semantic_record() {
    let session = serde_json::to_string_pretty(&json!({
        "type": "user",
        "role": "user",
        "id": "skill:pretty-structural-id",
        "title": "skill:pretty-structural-title",
        "cwd": "skill:pretty-structural-cwd",
        "path": "skill:pretty-structural-path",
        "metadata": {
            "note": "skill:pretty-structural-metadata"
        },
        "text": "accepted pretty user skill:pretty-real-skill then skill:pretty-real-skill"
    }))
    .expect("serialize accepted pretty primary fixture");
    let result =
        preview_codex_session_fixture_with_extension("pretty-primary-accepted", "json", &session);
    let row = result
        .pointer("/session_rows/0")
        .expect("accepted pretty primary row");
    let items = row
        .get("content_items")
        .and_then(Value::as_array)
        .expect("accepted pretty content items");
    let skill_items = items
        .iter()
        .filter(|item| item.get("kind").and_then(Value::as_str) == Some("skill_call"))
        .collect::<Vec<_>>();

    assert_eq!(
        row.get("user_message_count").and_then(Value::as_u64),
        Some(1),
        "{result}"
    );
    assert_eq!(
        row.get("total_message_count").and_then(Value::as_u64),
        Some(1),
        "{result}"
    );
    assert_eq!(
        row.get("skill_call_count").and_then(Value::as_u64),
        Some(2),
        "{result}"
    );
    assert_eq!(skill_items.len(), 1, "{result}");
    assert_eq!(
        skill_items[0].get("title").and_then(Value::as_str),
        Some("Skill: pretty-real-skill"),
        "{result}"
    );
    assert!(
        skill_items[0]
            .get("text")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("2 calls")),
        "{result}"
    );
}

#[test]
fn truncated_pretty_array_strings_do_not_bypass_record_classification() {
    let session = format!(
        concat!(
            "{{\n",
            "  \"role\": \"developer\",\n",
            "  \"content\": [\n",
            "    {},\n",
            "    \"TRUNCATED_PRETTY_ARRAY_DENY_MUST_NOT_SURFACE skill:truncated-pretty-denied\"\n",
            "  ]\n",
            "}}\n",
            "{{\"role\":\"user\",\"content\":\"accepted record after truncated pretty document\"}}\n"
        ),
        serde_json::to_string(&"x".repeat(600 * 1024)).expect("serialize pretty filler"),
    );
    let result = preview_codex_session_fixture_with_extension(
        "truncated-pretty-array-deny",
        "json",
        &session,
    );
    let serialized = serde_json::to_string(&result).expect("serialize truncated pretty result");

    assert!(
        serialized.contains("accepted record after truncated pretty document"),
        "{serialized}"
    );
    assert!(
        !serialized.contains("TRUNCATED_PRETTY_ARRAY_DENY_MUST_NOT_SURFACE"),
        "{serialized}"
    );
    assert!(
        !serialized.contains("truncated-pretty-denied"),
        "{serialized}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/skill_call_count")
            .and_then(Value::as_u64),
        Some(0),
        "{result}"
    );
}

#[test]
fn malformed_or_deep_opencode_sidecars_fail_closed() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-local-session-opencode-sidecar-test-{}-{unique}",
        std::process::id(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-local-session-opencode-sidecar-home-{}-{unique}",
        std::process::id(),
    ));
    let storage_root = user_home.join(".local/share/opencode/storage");
    let session_root = storage_root.join("session");
    let message_root = storage_root.join("message/ses_sidecar_deny");
    let part_root = storage_root.join("part/msg_visible");
    fs::create_dir_all(&session_root).expect("create opencode session root");
    fs::create_dir_all(&message_root).expect("create opencode message root");
    fs::create_dir_all(&part_root).expect("create opencode part root");
    fs::write(
        session_root.join("ses_sidecar_deny.json"),
        json!({ "id": "ses_sidecar_deny", "title": "safe sidecar session" }).to_string(),
    )
    .expect("write opencode session");
    fs::write(
        message_root.join("01-pretty-denied.json"),
        serde_json::to_string_pretty(&json!({
            "id": "msg_pretty_denied",
            "role": "developer",
            "content": "PRETTY_SIDECAR_DENY_MUST_NOT_SURFACE skill:pretty-sidecar-denied"
        }))
        .expect("serialize pretty denied sidecar"),
    )
    .expect("write pretty denied sidecar");
    let deep_value = format!(
        "{}{}{}",
        "[".repeat(140),
        serde_json::to_string("DEEP_SIDECAR_DENY_MUST_NOT_SURFACE skill:deep-sidecar-denied")
            .expect("serialize deep marker"),
        "]".repeat(140)
    );
    fs::write(
        message_root.join("02-deep-denied.json"),
        format!("{{\"id\":\"msg_deep_denied\",\"role\":\"developer\",\"content\":{deep_value}}}"),
    )
    .expect("write deep denied sidecar");
    fs::write(
        message_root.join("03-visible.json"),
        json!({
            "id": "msg_visible",
            "role": "assistant",
            "content": "accepted opencode sidecar sibling"
        })
        .to_string(),
    )
    .expect("write visible sidecar");
    fs::write(
        message_root.join("04-malformed-denied.json"),
        "{developer: MALFORMED_SIDECAR_DENY_MUST_NOT_SURFACE skill:malformed-sidecar-denied}\n",
    )
    .expect("write malformed denied sidecar");
    fs::write(
        part_root.join("01-pretty-denied-part.json"),
        serde_json::to_string_pretty(&json!({
            "id": "part_pretty_denied",
            "role": "summary",
            "content": "PRETTY_PART_DENY_MUST_NOT_SURFACE skill:pretty-part-denied"
        }))
        .expect("serialize pretty denied part"),
    )
    .expect("write pretty denied part");
    fs::write(
        part_root.join("02-visible-part.json"),
        json!({
            "id": "part_visible",
            "role": "assistant",
            "content": "accepted opencode part sibling"
        })
        .to_string(),
    )
    .expect("write visible part");
    fs::write(
        part_root.join("03-malformed-denied-part.json"),
        "{summary: MALFORMED_PART_DENY_MUST_NOT_SURFACE skill:malformed-part-denied}\n",
    )
    .expect("write malformed denied part");

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
        id: Some("session-preview-opencode-sidecar-deny".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "opencode",
            "limit": 10,
            "max_excerpt_chars": 2_000
        }),
    });
    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("opencode sidecar preview result");
    let serialized = serde_json::to_string(&result).expect("serialize sidecar result");

    assert!(
        serialized.contains("accepted opencode sidecar sibling"),
        "{serialized}"
    );
    assert!(
        serialized.contains("accepted opencode part sibling"),
        "{serialized}"
    );
    for hidden in [
        "PRETTY_SIDECAR_DENY_MUST_NOT_SURFACE",
        "DEEP_SIDECAR_DENY_MUST_NOT_SURFACE",
        "PRETTY_PART_DENY_MUST_NOT_SURFACE",
        "MALFORMED_SIDECAR_DENY_MUST_NOT_SURFACE",
        "MALFORMED_PART_DENY_MUST_NOT_SURFACE",
        "pretty-sidecar-denied",
        "deep-sidecar-denied",
        "pretty-part-denied",
        "malformed-sidecar-denied",
        "malformed-part-denied",
    ] {
        assert!(
            !serialized.contains(hidden),
            "surfaced {hidden}: {serialized}"
        );
    }
    assert_eq!(
        result
            .pointer("/session_rows/0/skill_call_count")
            .and_then(Value::as_u64),
        Some(0),
        "{result}"
    );
    assert!(
        !app_data_dir.exists(),
        "sidecar preview must remain read-only"
    );

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn accepted_pretty_opencode_message_and_part_survive_after_enrichment() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-local-session-opencode-pretty-test-{}-{unique}",
        std::process::id(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-local-session-opencode-pretty-home-{}-{unique}",
        std::process::id(),
    ));
    let storage_root = user_home.join(".local/share/opencode/storage");
    let session_root = storage_root.join("session");
    let message_root = storage_root.join("message/ses_pretty_accepted");
    let visible_part_root = storage_root.join("part/msg_pretty_visible");
    let denied_part_root = storage_root.join("part/msg_pretty_denied");
    fs::create_dir_all(&session_root).expect("create pretty opencode session root");
    fs::create_dir_all(&message_root).expect("create pretty opencode message root");
    fs::create_dir_all(&visible_part_root).expect("create pretty opencode part root");
    fs::create_dir_all(&denied_part_root).expect("create denied opencode part root");
    fs::write(
        session_root.join("ses_pretty_accepted.json"),
        json!({ "id": "ses_pretty_accepted", "title": "pretty accepted sidecars" }).to_string(),
    )
    .expect("write pretty opencode session");
    fs::write(
        message_root.join("01-pretty-visible.json"),
        serde_json::to_string_pretty(&json!({
            "id": "msg_pretty_visible",
            "role": "assistant",
            "content": "PRETTY_ACCEPTED_MESSAGE_MUST_SURVIVE skill:pretty-sidecar-skill"
        }))
        .expect("serialize pretty accepted message"),
    )
    .expect("write pretty accepted message");
    fs::write(
        visible_part_root.join("01-pretty-visible-part.json"),
        serde_json::to_string_pretty(&json!({
            "id": "part_pretty_visible",
            "role": "tool",
            "content": "PRETTY_ACCEPTED_PART_MUST_SURVIVE skill:pretty-sidecar-skill"
        }))
        .expect("serialize pretty accepted part"),
    )
    .expect("write pretty accepted part");
    fs::write(
        message_root.join("02-denied-parent.json"),
        json!({
            "id": "msg_pretty_denied",
            "role": "developer",
            "content": "DENIED_PARENT_MUST_NOT_SURFACE skill:denied-parent"
        })
        .to_string(),
    )
    .expect("write denied parent message");
    fs::write(
        denied_part_root.join("01-child.json"),
        json!({
            "id": "part_denied_parent_child",
            "role": "assistant",
            "content": "DENIED_PARENT_CHILD_MUST_NOT_SURFACE skill:denied-parent-child"
        })
        .to_string(),
    )
    .expect("write denied parent child part");

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
        id: Some("session-preview-opencode-pretty-accepted".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "opencode",
            "limit": 10,
            "max_excerpt_chars": 4_000
        }),
    });
    assert!(response.ok, "{:?}", response.error);
    let result = response.result.expect("pretty opencode preview result");
    let serialized = serde_json::to_string(&result).expect("serialize pretty sidecar result");
    let row = result
        .pointer("/session_rows/0")
        .expect("pretty opencode session row");

    for visible in [
        "PRETTY_ACCEPTED_MESSAGE_MUST_SURVIVE",
        "PRETTY_ACCEPTED_PART_MUST_SURVIVE",
    ] {
        assert!(
            serialized.contains(visible),
            "missing {visible}: {serialized}"
        );
    }
    for hidden in [
        "DENIED_PARENT_MUST_NOT_SURFACE",
        "DENIED_PARENT_CHILD_MUST_NOT_SURFACE",
        "denied-parent-child",
    ] {
        assert!(
            !serialized.contains(hidden),
            "surfaced {hidden}: {serialized}"
        );
    }
    assert_eq!(
        row.get("total_message_count").and_then(Value::as_u64),
        Some(1),
        "{result}"
    );
    assert_eq!(
        row.get("tool_call_count").and_then(Value::as_u64),
        Some(1),
        "{result}"
    );
    assert_eq!(
        row.get("skill_call_count").and_then(Value::as_u64),
        Some(2),
        "{result}"
    );
    assert!(
        !app_data_dir.exists(),
        "pretty sidecar preview must remain read-only"
    );

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn invalid_or_denying_classification_blocks_entire_nested_record() {
    for (case, record_type, role) in [
        ("null-role", json!("user"), Value::Null),
        ("object-role", json!("user"), json!({})),
        ("overlong-role", json!("user"), json!("r".repeat(5 * 1024))),
        ("deny-type", json!("developer"), json!("user")),
    ] {
        let tool_marker = format!("{case}_TOOL_MUST_NOT_SURFACE");
        let user_marker = format!("{case}_USER_MUST_NOT_SURFACE");
        let session = json!({
            "type": record_type,
            "role": role,
            "tool_calls": [{
                "name": tool_marker,
                "arguments": { "secret": "nested" }
            }],
            "data": {
                "type": "user",
                "role": "user",
                "text": user_marker
            }
        })
        .to_string();

        let result = preview_codex_session_fixture(&format!("complete-{case}"), &session);

        assert!(
            result
                .pointer("/session_rows/0/content_items")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty),
            "{case}: {result}"
        );
        assert_eq!(
            result
                .pointer("/session_rows/0/tool_call_count")
                .and_then(Value::as_u64),
            Some(0),
            "{case}: {result}"
        );
        assert_eq!(
            result
                .pointer("/session_rows/0/user_message_count")
                .and_then(Value::as_u64),
            Some(0),
            "{case}: {result}"
        );
    }
}

#[test]
fn encoded_classification_tokens_over_limit_deny_complete_records() {
    let escaped_value = "\\".repeat(3_000);
    let encoded_value = serde_json::to_string(&escaped_value).expect("serialize classification");
    assert!(escaped_value.len().saturating_add(2) <= 4 * 1024);
    assert!(encoded_value.len() > 4 * 1024);

    for (case, record_type, role) in [
        ("role", json!("user"), json!(&escaped_value)),
        ("type", json!(&escaped_value), json!("user")),
    ] {
        let marker = format!("ENCODED_{case}_OVER_LIMIT_MUST_NOT_SURFACE");
        let session = json!({
            "type": record_type,
            "role": role,
            "text": marker
        })
        .to_string();

        let result = preview_codex_session_fixture(&format!("encoded-{case}-over-limit"), &session);

        assert!(
            result
                .pointer("/session_rows/0/content_items")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty),
            "{case}: {result}"
        );
        assert_eq!(
            result
                .pointer("/session_rows/0/user_message_count")
                .and_then(Value::as_u64),
            Some(0),
            "{case}: {result}"
        );
    }
}

#[test]
fn classification_tokens_at_encoded_limit_remain_usable() {
    let boundary_value = "\\".repeat(2_047);
    let encoded_value = serde_json::to_string(&boundary_value).expect("serialize classification");
    assert_eq!(encoded_value.len(), 4 * 1024);

    for (case, record_type, role) in [
        ("role", json!("user"), json!(&boundary_value)),
        ("type", json!(&boundary_value), json!("user")),
    ] {
        let marker = format!("ENCODED_{case}_AT_LIMIT_REMAINS_VISIBLE");
        let session = json!({
            "type": record_type,
            "role": role,
            "text": marker
        })
        .to_string();

        let result = preview_codex_session_fixture(&format!("encoded-{case}-at-limit"), &session);

        assert_eq!(
            result
                .pointer("/session_rows/0/content_items/0/kind")
                .and_then(Value::as_str),
            Some("user_message"),
            "{case}: {result}"
        );
        assert_eq!(
            result
                .pointer("/session_rows/0/user_message_count")
                .and_then(Value::as_u64),
            Some(1),
            "{case}: {result}"
        );
    }
}

#[test]
fn non_deny_tool_and_wrapper_records_keep_nested_content() {
    let tool_result = preview_codex_session_fixture(
        "non-deny-real-tool",
        &json!({
            "type": "tool_call",
            "role": "assistant",
            "name": "current_time",
            "arguments": { "marker": "REAL_TOOL_REMAINS_VISIBLE" }
        })
        .to_string(),
    );
    assert_eq!(
        tool_result
            .pointer("/session_rows/0/tool_call_count")
            .and_then(Value::as_u64),
        Some(1),
        "{tool_result}"
    );
    assert!(
        serde_json::to_string(&tool_result)
            .expect("serialize tool preview")
            .contains("REAL_TOOL_REMAINS_VISIBLE"),
        "{tool_result}"
    );

    let wrapper_result = preview_codex_session_fixture(
        "non-deny-visible-wrapper",
        &json!({
            "data": {
                "type": "user",
                "role": "user",
                "text": "VISIBLE_NESTED_USER_REMAINS_VISIBLE"
            }
        })
        .to_string(),
    );
    assert_eq!(
        wrapper_result
            .pointer("/session_rows/0/user_message_count")
            .and_then(Value::as_u64),
        Some(1),
        "{wrapper_result}"
    );
    assert!(
        serde_json::to_string(&wrapper_result)
            .expect("serialize wrapper preview")
            .contains("VISIBLE_NESTED_USER_REMAINS_VISIBLE"),
        "{wrapper_result}"
    );
}

#[test]
fn oversized_visible_user_and_assistant_classifications_remain_visible() {
    for (record_type, role, expected_kind) in [
        ("user", "user", "user_message"),
        ("assistant", "assistant", "agent_reply"),
    ] {
        let marker = format!(
            "VISIBLE_{}_RECOVERED_MESSAGE",
            record_type.to_ascii_uppercase()
        );
        let session = format!(
            "{{\"text\":{marker},\"data\":\"{}\",\"type\":{record_type},\"role\":{role},\"blob\":\"{}\",\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
            "a".repeat(400 * 1024),
            "b".repeat(200 * 1024),
            marker = serde_json::to_string(&marker).expect("serialize marker"),
            record_type = serde_json::to_string(record_type).expect("serialize type"),
            role = serde_json::to_string(role).expect("serialize role"),
        );
        let result =
            preview_codex_session_fixture(&format!("visible-recovered-{record_type}"), &session);

        assert_eq!(
            result
                .pointer("/session_rows/0/content_items/0/kind")
                .and_then(Value::as_str),
            Some(expected_kind),
            "{record_type}: {result}"
        );
        assert!(
            result
                .pointer("/session_rows/0/content_items/0/text")
                .and_then(Value::as_str)
                .is_some_and(|text| text.contains(&marker)),
            "{record_type}: {result}"
        );
    }
}

#[test]
fn invalid_oversized_json_never_produces_recovered_content() {
    let cases = [
        (
            "invalid-nested-token",
            format!(
                "{{\"type\":\"user\",\"role\":\"user\",\"text\":\"INVALID_NESTED_TOKEN_MUST_NOT_SURFACE\",\"data\":{{{}}},\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
                "x".repeat(600 * 1024)
            ),
            "INVALID_NESTED_TOKEN_MUST_NOT_SURFACE",
        ),
        (
            "root-trailing-comma",
            format!(
                "{{\"type\":\"user\",\"role\":\"user\",\"text\":\"ROOT_TRAILING_COMMA_MUST_NOT_SURFACE\",\"data\":\"{}\",\"timestamp\":\"2026-07-10T08:09:10Z\",}}\n",
                "x".repeat(600 * 1024)
            ),
            "ROOT_TRAILING_COMMA_MUST_NOT_SURFACE",
        ),
        (
            "nested-trailing-comma",
            format!(
                "{{\"type\":\"user\",\"role\":\"user\",\"text\":\"NESTED_TRAILING_COMMA_MUST_NOT_SURFACE\",\"data\":{{\"blob\":\"{}\",}},\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
                "x".repeat(600 * 1024)
            ),
            "NESTED_TRAILING_COMMA_MUST_NOT_SURFACE",
        ),
        (
            "invalid-array-token",
            format!(
                "{{\"type\":\"user\",\"role\":\"user\",\"text\":\"INVALID_ARRAY_TOKEN_MUST_NOT_SURFACE\",\"data\":[{}],\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
                "x".repeat(600 * 1024)
            ),
            "INVALID_ARRAY_TOKEN_MUST_NOT_SURFACE",
        ),
    ];

    for (case, session, marker) in cases {
        let result = preview_codex_session_fixture(case, &session);
        let output = serde_json::to_string(&result).expect("serialize invalid preview");

        assert!(!output.contains(marker), "{case}: {output}");
    }
}

#[test]
fn later_supported_scalar_override_clears_stale_visible_text() {
    for (region, first_blob_bytes) in [("gap", 400 * 1024), ("tail-prefix", 440 * 1024)] {
        for (case, later_text) in [
            ("null", "null".to_string()),
            ("object", "{}".to_string()),
            (
                "oversized",
                serde_json::to_string(&"z".repeat(5 * 1024)).expect("serialize oversized text"),
            ),
        ] {
            let marker = format!(
                "STALE_{}_{}_TEXT_MUST_BE_CLEARED",
                region.replace('-', "_").to_ascii_uppercase(),
                case.to_ascii_uppercase()
            );
            let session = format!(
                "{{\"type\":\"user\",\"role\":\"user\",\"text\":{marker},\"data\":\"{}\",\"text\":{later_text},\"blob\":\"{}\",\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
                "a".repeat(first_blob_bytes),
                "b".repeat((600_usize * 1024).saturating_sub(first_blob_bytes)),
                marker = serde_json::to_string(&marker).expect("serialize marker"),
            );
            let result = preview_codex_session_fixture(
                &format!("supported-text-override-{case}-{region}"),
                &session,
            );
            let output =
                serde_json::to_string(&result).expect("serialize supported override preview");

            assert!(!output.contains(&marker), "{case}/{region}: {output}");
        }
    }
}

#[test]
fn bom_prefixed_oversized_record_after_first_line_is_rejected() {
    let first = json!({
        "type": "user",
        "role": "user",
        "text": "FIRST_RECORD_REMAINS_VISIBLE"
    });
    let session = format!(
        "{first}\n\u{feff}{{\"type\":\"user\",\"role\":\"user\",\"text\":\"MID_RECORD_BOM_MUST_NOT_SURFACE\",\"data\":\"{}\",\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
        "x".repeat(600 * 1024)
    );
    let result = preview_codex_session_fixture("mid-record-bom-oversized", &session);
    let output = serde_json::to_string(&result).expect("serialize BOM preview");

    assert!(output.contains("FIRST_RECORD_REMAINS_VISIBLE"), "{output}");
    assert!(
        !output.contains("MID_RECORD_BOM_MUST_NOT_SURFACE"),
        "{output}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/content_items")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1),
        "{result}"
    );
}

#[test]
fn oversized_user_tail_text_with_head_timestamp_preserves_supported_scalars() {
    let session = format!(
        "{{\"type\":\"user\",\"role\":\"user\",\"timestamp\":\"2026-07-10T08:09:10Z\",\"data\":\"{}\",\"text\":\"USER_TAIL_WITH_HEAD_TIMESTAMP\"}}\n",
        "x".repeat(600 * 1024)
    );
    let result = preview_codex_session_fixture("user-head-timestamp", &session);
    let output = serde_json::to_string(&result).expect("serialize user preview");

    assert!(output.contains("USER_TAIL_WITH_HEAD_TIMESTAMP"), "{output}");
    assert_eq!(
        result
            .pointer("/session_rows/0/title")
            .and_then(Value::as_str),
        Some("USER_TAIL_WITH_HEAD_TIMESTAMP"),
        "{result}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/user_message_count")
            .and_then(Value::as_u64),
        Some(1),
        "{result}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/content_items/0/kind")
            .and_then(Value::as_str),
        Some("user_message"),
        "{result}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/content_items/0/timestamp")
            .and_then(Value::as_i64),
        Some(1_783_670_950_000),
        "{result}"
    );
}

#[test]
fn oversized_user_recovery_preserves_prefix_semantics_with_tail_timestamp() {
    let session = format!(
        "{{\"type\":\"user\",\"role\":\"user\",\"data\":\"{}\",\"text\":\"USER_SEMANTICS_MUST_SURVIVE\",\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n",
        "x".repeat(600 * 1024)
    );
    let result = preview_codex_session_fixture("user-semantics", &session);

    assert_eq!(
        result
            .pointer("/session_rows/0/title")
            .and_then(Value::as_str),
        Some("USER_SEMANTICS_MUST_SURVIVE"),
        "{result}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/user_message_count")
            .and_then(Value::as_u64),
        Some(1),
        "{result}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/content_items/0/kind")
            .and_then(Value::as_str),
        Some("user_message"),
        "{result}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/content_items/0/timestamp")
            .and_then(Value::as_i64),
        Some(1_783_670_950_000),
        "{result}"
    );
}

#[test]
fn small_incomplete_json_record_is_not_reinterpreted_as_plaintext() {
    let session =
        "{\"type\":\"user\",\"role\":\"user\",\"text\":\"INCOMPLETE_JSON_MUST_NOT_SURFACE\"";
    let result = preview_codex_session_fixture("small-incomplete-json", session);
    let output = serde_json::to_string(&result).expect("serialize incomplete preview");

    assert!(
        !output.contains("INCOMPLETE_JSON_MUST_NOT_SURFACE"),
        "{output}"
    );
    assert_eq!(
        result.get("count").and_then(Value::as_u64),
        Some(0),
        "{result}"
    );
}

#[test]
fn interior_invalid_utf8_does_not_discard_later_complete_plaintext() {
    let mut bytes = b"user: BEFORE_INVALID_UTF8\n".to_vec();
    bytes.push(0xff);
    bytes.extend_from_slice(b"\nuser: AFTER_INVALID_UTF8\n");
    let result = preview_codex_session_fixture_bytes("interior-invalid-utf8", &bytes);
    let output = serde_json::to_string(&result).expect("serialize invalid-utf8 preview");

    assert!(output.contains("BEFORE_INVALID_UTF8"), "{output}");
    assert!(output.contains("AFTER_INVALID_UTF8"), "{output}");
}

#[test]
fn interior_invalid_utf8_in_tail_does_not_discard_complete_plaintext() {
    let mut bytes = b"user: HEAD_BEFORE_LARGE_GAP\n".to_vec();
    bytes.extend(std::iter::repeat_n(b'x', 600 * 1024));
    bytes.extend_from_slice(b"\nuser: BEFORE_TAIL_INVALID_UTF8\n");
    bytes.push(0xff);
    bytes.extend_from_slice(b"\nuser: AFTER_TAIL_INVALID_UTF8\n");
    let result = preview_codex_session_fixture_bytes("tail-interior-invalid-utf8", &bytes);
    let output = serde_json::to_string(&result).expect("serialize tail invalid-utf8 preview");

    assert!(output.contains("BEFORE_TAIL_INVALID_UTF8"), "{output}");
    assert!(output.contains("AFTER_TAIL_INVALID_UTF8"), "{output}");
}

#[test]
fn local_session_preview_does_not_merge_distinct_oversized_records() {
    let tail_filler = "tail-snapshot-data-".repeat(48_000);
    let session = format!(
        concat!(
            "{{\"type\":\"user\",\"role\":\"user\",\"id\":\"head-user\",",
            "\"text\":\"head-user-visible\"}}\n",
            "{{\"type\":\"file-history-snapshot\",\"id\":\"tail-snapshot\",",
            "\"data\":{},\"role\":\"assistant\",",
            "\"text\":\"skipped-tail-must-not-surface\"}}\n"
        ),
        serde_json::to_string(&tail_filler).expect("serialize tail filler"),
    );

    let result = preview_codex_session_fixture("distinct-oversized-records", &session);
    let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

    assert!(serialized.contains("head-user-visible"), "{serialized}");
    assert!(!serialized.contains("skipped-tail-must-not-surface"));
    assert!(result
        .pointer("/session_rows/0/content_items")
        .and_then(Value::as_array)
        .is_some_and(|items| items.iter().any(|item| {
            item.get("kind").and_then(Value::as_str) == Some("user_message")
                && item.get("text").and_then(Value::as_str) == Some("head-user-visible")
        })));
}

#[test]
fn local_session_preview_does_not_let_oversized_skipped_head_suppress_tail_user() {
    let skipped_filler = "skipped-head-data-".repeat(48_000);
    let session = format!(
        concat!(
            "{{\"type\":\"file-history-snapshot\",\"id\":\"head-snapshot\",",
            "\"text\":\"skipped-head-must-not-surface\",\"data\":{}}}\n",
            "{{\"type\":\"user\",\"role\":\"user\",\"id\":\"tail-user\",",
            "\"text\":\"tail-user-visible\",\"timestamp\":\"2026-07-10T08:09:10Z\"}}\n"
        ),
        serde_json::to_string(&skipped_filler).expect("serialize skipped filler"),
    );

    let result = preview_codex_session_fixture("skipped-oversized-then-tail-user", &session);
    let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

    assert!(serialized.contains("tail-user-visible"), "{serialized}");
    assert!(!serialized.contains("skipped-head-must-not-surface"));
    assert_eq!(
        result
            .pointer("/session_rows/0/title")
            .and_then(Value::as_str),
        Some("tail-user-visible")
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/content_items/0/kind")
            .and_then(Value::as_str),
        Some("user_message")
    );
}

#[test]
fn local_session_preview_preserves_complete_small_data_wrapped_user_event() {
    let session = json!({
        "data": {
            "type": "user",
            "text": "hello"
        }
    })
    .to_string();

    let result = preview_codex_session_fixture("small-data-wrapped-user", &session);
    let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

    assert!(serialized.contains("hello"), "{serialized}");
    assert_eq!(
        result
            .pointer("/session_rows/0/content_items/0/kind")
            .and_then(Value::as_str),
        Some("user_message")
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/content_items/0/text")
            .and_then(Value::as_str),
        Some("hello")
    );
}

#[test]
fn local_session_preview_skips_complete_record_with_late_top_level_type() {
    let session = format!(
        "{{\"role\":\"user\",\"text\":\"COMPLETE_LATE_TYPE_LEAK\",\"data\":\"{}\",\"type\":\"file-history-snapshot\"}}\n",
        "x".repeat(6 * 1024)
    );

    let result = preview_codex_session_fixture("complete-late-root-type", &session);
    let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

    assert!(
        !serialized.contains("COMPLETE_LATE_TYPE_LEAK"),
        "{serialized}"
    );
}

#[test]
fn local_session_preview_decodes_escaped_top_level_type_before_skip() {
    let session = concat!(
        "{\"role\":\"user\",\"text\":\"ESCAPED_TYPE_LEAK\",",
        "\"\\u0074ype\":\"file-history-snapshot\"}\n"
    );

    let result = preview_codex_session_fixture("escaped-root-type", session);
    let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

    assert!(!serialized.contains("ESCAPED_TYPE_LEAK"), "{serialized}");
}

#[test]
fn local_session_preview_ignores_nested_metadata_type_for_skip() {
    let session = json!({
        "role": "user",
        "text": "NESTED_TYPE_VALID_USER",
        "metadata": {
            "type": "file-history-snapshot"
        }
    })
    .to_string();

    let result = preview_codex_session_fixture("nested-metadata-type", &session);
    let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

    assert!(
        serialized.contains("NESTED_TYPE_VALID_USER"),
        "{serialized}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/content_items/0/kind")
            .and_then(Value::as_str),
        Some("user_message")
    );
}

#[test]
fn local_session_preview_uses_final_duplicate_top_level_type() {
    let session = format!(
        concat!(
            "{{\"type\":\"user\",\"role\":\"user\",",
            "\"text\":\"DUPLICATE_COMPLETE_TYPE_LEAK\",\"data\":\"{}\",",
            "\"type\":\"file-history-snapshot\"}}\n"
        ),
        "x".repeat(6 * 1024)
    );

    let result = preview_codex_session_fixture("duplicate-complete-root-type", &session);
    let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

    assert!(
        !serialized.contains("DUPLICATE_COMPLETE_TYPE_LEAK"),
        "{serialized}"
    );
}

#[test]
fn local_session_preview_drops_incomplete_head_with_possible_late_type_override() {
    let filler = serde_json::to_string(&"private".repeat(100 * 1024))
        .expect("serialize duplicate-type filler");
    let session = format!(
        concat!(
            "{{\"type\":\"user\",\"role\":\"user\",",
            "\"text\":\"DUPLICATE_LATE_TYPE_LEAK\",\"data\":{filler},",
            "\"type\":\"file-history-snapshot\"}}\n"
        ),
        filler = filler,
    );

    let result = preview_codex_session_fixture("duplicate-late-root-type", &session);
    let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

    assert!(
        !serialized.contains("DUPLICATE_LATE_TYPE_LEAK"),
        "{serialized}"
    );
}

#[test]
fn local_session_preview_keeps_exact_tail_cap_plain_text_line() {
    const TAIL_BYTES: usize = 128 * 1024;
    let tail_prefix = "user: EXACT_TAIL_CAP_LINE";
    let tail = format!(
        "{tail_prefix}{}\n",
        "y".repeat(TAIL_BYTES - tail_prefix.len() - 1)
    );
    assert_eq!(tail.len(), TAIL_BYTES);

    for (case, boundary) in [("lf", "\n"), ("crlf", "\r\n")] {
        let session = format!(
            "user: early exact-cap line\n{}{boundary}{tail}",
            "m".repeat(700 * 1024)
        );
        let result = preview_codex_session_fixture_with_extension(
            &format!("exact-tail-cap-{case}"),
            "log",
            &session,
        );
        let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

        assert!(
            serialized.contains("EXACT_TAIL_CAP_LINE"),
            "{case}: {serialized}"
        );
    }
}

#[test]
fn local_session_preview_hides_marker_and_does_not_count_structural_record() {
    let marker = "{\"type\":\"skills-copilot-truncation-marker\"}\n";
    let marker_result = preview_codex_session_fixture("marker-only", marker);
    let marker_serialized =
        serde_json::to_string(&marker_result).expect("serialize marker-only preview");

    assert!(!marker_serialized.contains("skills-copilot-truncation-marker"));

    let structural_session = format!(
        "{{\"type\":\"user\",\"role\":\"user\"}}\n{{\"role\":\"user\",\"data\":\"{}\"}}\n",
        "x".repeat(700 * 1024)
    );
    let structural_result =
        preview_codex_session_fixture("structural-user-only", &structural_session);
    let structural_serialized =
        serde_json::to_string(&structural_result).expect("serialize structural preview");

    assert_eq!(
        structural_result
            .pointer("/session_rows/0/user_message_count")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert_eq!(
        structural_result
            .pointer("/session_rows/0/total_message_count")
            .and_then(Value::as_u64),
        Some(0)
    );
    assert!(structural_result
        .pointer("/session_rows/0/content_items")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty));
    assert!(!structural_serialized.contains("skills-copilot-truncation-marker"));
    assert!(!structural_serialized.contains("Session excerpt"));
}

#[test]
fn local_session_preview_uses_retained_ranges_for_continuity() {
    const PREFIX_SEGMENT_BYTES: usize = 553_111;
    let user_record =
        "{\"type\":\"user\",\"role\":\"user\",\"text\":\"HEAD_USER_SHOULD_SURVIVE\"}\n";
    let carrier_prefix = "{\"type\":\"file-history-snapshot\",\"data\":\"";
    let carrier_suffix = "\"}";
    let filler = "u".repeat(
        PREFIX_SEGMENT_BYTES - user_record.len() - carrier_prefix.len() - carrier_suffix.len(),
    );
    let prefix_segment = format!("{user_record}{carrier_prefix}{filler}{carrier_suffix}");
    assert_eq!(prefix_segment.len(), PREFIX_SEGMENT_BYTES);
    let skipped = json!({
        "type": "file-history-snapshot",
        "text": "retained-tail-skip"
    });
    let session = format!("{prefix_segment}\n{skipped}\n");

    let result = preview_codex_session_fixture("retained-range-continuity", &session);
    let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

    assert!(
        serialized.contains("HEAD_USER_SHOULD_SURVIVE"),
        "{serialized}"
    );
    assert!(!serialized.contains("retained-tail-skip"));
}

#[test]
fn local_session_preview_never_correlates_cross_record_scalar_ids() {
    for (case, id, marker) in [
        ("empty-id", "\"\"", "EMPTY_ID_CROSS_RECORD_LEAK"),
        ("numeric-id", "7", "NUMERIC_ID_CROSS_RECORD_LEAK"),
        ("bool-id", "true", "BOOL_ID_CROSS_RECORD_LEAK"),
    ] {
        let head_filler =
            serde_json::to_string(&"h".repeat(700 * 1024)).expect("serialize head filler");
        let tail_filler =
            serde_json::to_string(&"t".repeat(700 * 1024)).expect("serialize tail filler");
        let session = format!(
            concat!(
                "{{\"type\":\"user\",\"role\":\"user\",\"id\":{id},",
                "\"title\":\"head-{case}\",\"data\":{head_filler}}}\n",
                "{{\"type\":\"file-history-snapshot\",\"id\":\"tail-original\",",
                "\"data\":{tail_filler},\"id\":{id},\"text\":\"{marker}\"}}\n"
            ),
            id = id,
            case = case,
            head_filler = head_filler,
            tail_filler = tail_filler,
            marker = marker,
        );

        let result = preview_codex_session_fixture(case, &session);
        let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

        assert!(!serialized.contains(marker), "{serialized}");
    }
}

#[test]
fn local_session_preview_rejects_reused_identity_across_unread_gap() {
    let head_filler =
        serde_json::to_string(&"h".repeat(700 * 1024)).expect("serialize head filler");
    let tail_filler =
        serde_json::to_string(&"t".repeat(700 * 1024)).expect("serialize tail filler");
    let session = format!(
        concat!(
            "{{\"type\":\"user\",\"role\":\"user\",\"id\":\"reused\",",
            "\"sessionId\":\"session-1\",\"cwd\":\"/same\",",
            "\"title\":\"reused-head\",\"data\":{head_filler}}}\n",
            "{{\"type\":\"file-history-snapshot\",\"data\":{tail_filler},",
            "\"type\":\"user\",\"role\":\"user\",\"id\":\"reused\",",
            "\"sessionId\":\"session-1\",\"cwd\":\"/same\",",
            "\"text\":\"REUSED_ID_CROSS_RECORD_LEAK\"}}\n"
        ),
        head_filler = head_filler,
        tail_filler = tail_filler,
    );

    let result = preview_codex_session_fixture("reused-cross-record-identity", &session);
    let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

    assert!(
        !serialized.contains("REUSED_ID_CROSS_RECORD_LEAK"),
        "{serialized}"
    );
}

#[test]
fn local_session_preview_rejects_id_overridden_inside_unread_gap() {
    let head_filler =
        serde_json::to_string(&"h".repeat(700 * 1024)).expect("serialize head filler");
    let tail_filler =
        serde_json::to_string(&"t".repeat(700 * 1024)).expect("serialize tail filler");
    let session = format!(
        concat!(
            "{{\"type\":\"user\",\"role\":\"user\",\"id\":\"shared\",",
            "\"title\":\"duplicate-id-head\",\"data\":{head_filler},",
            "\"id\":\"hidden-final-id\"}}\n",
            "{{\"type\":\"file-history-snapshot\",\"data\":{tail_filler},",
            "\"id\":\"shared\",\"text\":\"DUPLICATE_ID_CROSS_RECORD_LEAK\"}}\n"
        ),
        head_filler = head_filler,
        tail_filler = tail_filler,
    );

    let result = preview_codex_session_fixture("duplicate-id-inside-gap", &session);
    let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

    assert!(
        !serialized.contains("DUPLICATE_ID_CROSS_RECORD_LEAK"),
        "{serialized}"
    );
}

#[test]
fn local_session_preview_drops_incomplete_prefix_with_unknown_root_type() {
    let filler =
        serde_json::to_string(&"private".repeat(100 * 1024)).expect("serialize late-type filler");
    let session = format!(
        concat!(
            "{{\"role\":\"user\",\"text\":\"SKIPPED_LATE_TYPE_LEAK\",",
            "\"data\":{filler},\"type\":\"file-history-snapshot\"}}\n"
        ),
        filler = filler,
    );

    let result = preview_codex_session_fixture("late-skipped-root-type", &session);
    let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

    assert!(
        !serialized.contains("SKIPPED_LATE_TYPE_LEAK"),
        "{serialized}"
    );
}

#[test]
fn local_session_preview_keeps_newline_aligned_plain_text_tail() {
    let session = format!(
        "user: early plain text\n{}\nuser: LATE_PLAIN_TEXT_MISSING\n",
        "x".repeat(700 * 1024)
    );

    for extension in ["log", "txt"] {
        let result = preview_codex_session_fixture_with_extension(
            &format!("newline-aligned-plain-tail-{extension}"),
            extension,
            &session,
        );
        let serialized = serde_json::to_string(&result).expect("serialize bounded preview");

        assert!(
            serialized.contains("LATE_PLAIN_TEXT_MISSING"),
            "{extension}: {serialized}"
        );
    }
}

#[test]
fn local_session_preview_preserves_complete_plaintext_with_json_punctuation() {
    let cases = [
        ("bracket-info", "[INFO] BRACKET_INFO_VISIBLE"),
        ("leading-warn", "   [WARN] LEADING_WARN_VISIBLE"),
        ("curly-prefix", "{plain CURLY_PREFIX_VISIBLE"),
        ("square-suffix", "SQUARE_SUFFIX_VISIBLE ]"),
        ("bom-bracket", "\u{feff}[INFO] BOM_BRACKET_VISIBLE"),
        ("crlf-bracket", "[INFO] CRLF_BRACKET_VISIBLE\r\n"),
    ];

    for extension in ["jsonl", "json", "txt", "log"] {
        for (case, line) in cases {
            let marker = case.to_ascii_uppercase().replace('-', "_");
            let result = preview_codex_session_fixture_with_extension(
                &format!("plain-punctuation-{extension}-{case}"),
                extension,
                line,
            );
            let serialized = serde_json::to_string(&result).expect("serialize plain preview");

            assert!(
                serialized.contains(&marker),
                "{extension}/{case}: {serialized}"
            );
        }
    }
}

#[test]
fn local_session_preview_drops_bom_prefixed_incomplete_json_head() {
    let oversized_data = "ignored-incomplete-data-".repeat(30_000);

    for (case, bom) in [("control", ""), ("bom", "\u{feff}")] {
        let session = format!(
            "{bom}{{\"type\":\"user\",\"role\":\"user\",\"text\":\"BOM_INCOMPLETE_HEAD_LEAK\",\"data\":\"{oversized_data}\""
        );
        let result = preview_codex_session_fixture(case, &session);
        let serialized = serde_json::to_string(&result).expect("serialize incomplete preview");

        assert_eq!(
            result.get("count").and_then(Value::as_u64),
            Some(0),
            "{case}: {result}"
        );
        assert!(
            result
                .get("session_rows")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty),
            "{case}: {result}"
        );
        assert!(
            !serialized.contains("BOM_INCOMPLETE_HEAD_LEAK"),
            "{case}: {serialized}"
        );
    }
}

#[test]
fn local_session_preview_parses_complete_json_after_initial_bom() {
    let session = format!(
        "\u{feff}{}\n",
        json!({
            "role": "user",
            "content": "BOM_COMPLETE_JSON_VISIBLE"
        })
    );
    let result = preview_codex_session_fixture("bom-complete-json", &session);

    assert_eq!(
        result
            .pointer("/session_rows/0/user_message_count")
            .and_then(Value::as_u64),
        Some(1),
        "{result}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/content_items/0/kind")
            .and_then(Value::as_str),
        Some("user_message"),
        "{result}"
    );
    assert!(
        result
            .pointer("/session_rows/0/content_items/0/text")
            .and_then(Value::as_str)
            .is_some_and(|text| text.contains("BOM_COMPLETE_JSON_VISIBLE")),
        "{result}"
    );
}

#[test]
fn local_session_preview_only_treats_file_prefix_as_bom() {
    let session = format!(
        "{}\n\u{feff}{}\n",
        json!({
            "role": "user",
            "content": "FIRST_RECORD_USER_MESSAGE"
        }),
        json!({
            "role": "user",
            "content": "LATER_RECORD_FEFF_IS_CONTENT"
        })
    );
    let result = preview_codex_session_fixture("later-record-feff", &session);

    assert_eq!(
        result
            .pointer("/session_rows/0/user_message_count")
            .and_then(Value::as_u64),
        Some(1),
        "{result}"
    );
    assert_eq!(
        result
            .pointer("/session_rows/0/content_items")
            .and_then(Value::as_array)
            .map(Vec::len),
        Some(1),
        "{result}"
    );
}

#[test]
fn local_session_preview_does_not_count_structure_only_tools() {
    let cases = [
        (
            "tool-call",
            json!({
                "type": "tool_call",
                "role": "assistant",
                "id": "tool_call_structure_only",
                "name": "tool_call_structure_only"
            }),
        ),
        (
            "tool-result",
            json!({ "type": "tool_result", "role": "assistant" }),
        ),
        ("tool-role", json!({ "role": "tool" })),
        (
            "nested-tool",
            json!({ "metadata": { "type": "tool_call" } }),
        ),
        (
            "tool-call-content-structure",
            json!({
                "type": "tool_call",
                "role": "assistant",
                "content": {
                    "type": "tool_call",
                    "id": "nested_content_structure",
                    "name": "NESTED_CONTENT_STRUCTURE_ONLY"
                }
            }),
        ),
        (
            "tool-result-content-structure",
            json!({
                "type": "tool_result",
                "role": "assistant",
                "content": {
                    "type": "tool_result",
                    "tool_call_id": "nested_result_structure",
                    "name": "NESTED_RESULT_STRUCTURE_ONLY"
                }
            }),
        ),
        (
            "tool-role-content-structure",
            json!({
                "role": "tool",
                "content": {
                    "type": "tool_result",
                    "id": "nested_role_structure",
                    "name": "NESTED_ROLE_STRUCTURE_ONLY"
                }
            }),
        ),
        (
            "nested-wrapped-tool-structure",
            json!({
                "metadata": {
                    "event": {
                        "type": "tool_call",
                        "content": {
                            "type": "tool_call",
                            "id": "nested_wrapped_structure",
                            "name": "NESTED_WRAPPED_STRUCTURE_ONLY"
                        }
                    }
                }
            }),
        ),
        (
            "tool-error-structure",
            json!({
                "type": "tool_result",
                "error": {
                    "type": "tool_result",
                    "id": "nested_error_structure",
                    "name": "NESTED_ERROR_STRUCTURE_ONLY"
                }
            }),
        ),
    ];

    for (case, value) in cases {
        let result = preview_codex_session_fixture(case, &value.to_string());

        assert_eq!(
            result
                .pointer("/session_rows/0/tool_call_count")
                .and_then(Value::as_u64),
            Some(0),
            "{case}: {result}"
        );
        assert_eq!(
            result
                .pointer("/session_rows/0/total_message_count")
                .and_then(Value::as_u64),
            Some(0),
            "{case}: {result}"
        );
        assert!(
            result
                .pointer("/session_rows/0/content_items")
                .and_then(Value::as_array)
                .is_some_and(Vec::is_empty),
            "{case}: {result}"
        );
    }
}

#[test]
fn local_session_preview_counts_real_tool_payload_once() {
    for (case, value, marker) in [
        (
            "real-tool-call",
            json!({
                "type": "tool_call",
                "role": "assistant",
                "id": "tool_call_real_payload",
                "name": "tool_call_real_payload",
                "arguments": { "query": "REAL_TOOL_ARGUMENT" }
            }),
            "REAL_TOOL_ARGUMENT",
        ),
        (
            "real-tool-result",
            json!({
                "type": "tool_result",
                "role": "assistant",
                "tool_call_id": "tool_call_real_result",
                "result": { "status": "REAL_TOOL_RESULT" }
            }),
            "REAL_TOOL_RESULT",
        ),
        (
            "real-tool-call-content",
            json!({
                "type": "tool_call",
                "role": "assistant",
                "content": { "status": "REAL_TOOL_CALL_CONTENT" }
            }),
            "REAL_TOOL_CALL_CONTENT",
        ),
        (
            "real-tool-result-content",
            json!({
                "type": "tool_result",
                "role": "assistant",
                "tool_call_id": "real_content_object",
                "content": { "status": "REAL_CONTENT_OBJECT_PAYLOAD" }
            }),
            "REAL_CONTENT_OBJECT_PAYLOAD",
        ),
        (
            "real-tool-error-object",
            json!({
                "type": "tool_result",
                "role": "assistant",
                "tool_call_id": "real_error_object",
                "error": { "code": "REAL_ERROR_OBJECT_PAYLOAD" }
            }),
            "REAL_ERROR_OBJECT_PAYLOAD",
        ),
        (
            "real-tool-role-content",
            json!({
                "role": "tool",
                "content": { "status": "REAL_TOOL_ROLE_CONTENT" }
            }),
            "REAL_TOOL_ROLE_CONTENT",
        ),
        (
            "real-tool-path-argument",
            json!({
                "type": "tool_call",
                "role": "assistant",
                "name": "read_file",
                "arguments": { "path": "REAL_TOOL_PATH_ARGUMENT" }
            }),
            "REAL_TOOL_PATH_ARGUMENT",
        ),
        (
            "real-nested-tool-error",
            json!({
                "metadata": {
                    "event": {
                        "type": "tool_result",
                        "error": { "code": "REAL_NESTED_TOOL_ERROR" }
                    }
                }
            }),
            "REAL_NESTED_TOOL_ERROR",
        ),
    ] {
        let result = preview_codex_session_fixture(case, &value.to_string());
        let items = result
            .pointer("/session_rows/0/content_items")
            .and_then(Value::as_array)
            .expect("tool content items");

        assert_eq!(
            result
                .pointer("/session_rows/0/tool_call_count")
                .and_then(Value::as_u64),
            Some(1),
            "{case}: {result}"
        );
        assert_eq!(items.len(), 1, "{case}: {result}");
        assert_eq!(
            items[0].get("kind").and_then(Value::as_str),
            Some("tool_call")
        );
        assert!(
            items[0]
                .get("text")
                .and_then(Value::as_str)
                .is_some_and(|text| text.contains(marker)),
            "{case}: {result}"
        );
    }
}

#[test]
fn explicit_empty_tool_arguments_still_identify_a_real_call() {
    let value = json!({
        "type": "tool_call",
        "role": "assistant",
        "id": "real-no-argument-call",
        "name": "current_time",
        "arguments": {}
    });
    let result = preview_codex_session_fixture("empty-tool-arguments", &value.to_string());

    assert_eq!(
        result
            .pointer("/session_rows/0/tool_call_count")
            .and_then(Value::as_u64),
        Some(1),
        "{result}"
    );
}

#[test]
fn explicit_empty_tool_payload_containers_count_once() {
    for (case, value) in [
        (
            "empty-tool-input",
            json!({
                "type": "tool_call",
                "name": "current_time",
                "input": {}
            }),
        ),
        (
            "empty-tool-result",
            json!({
                "type": "tool_result",
                "tool_call_id": "empty-result-call",
                "result": {}
            }),
        ),
        (
            "nested-empty-tool-arguments",
            json!({
                "role": "assistant",
                "content": [{
                    "type": "tool_call",
                    "id": "nested-empty-call",
                    "name": "current_time",
                    "arguments": {}
                }]
            }),
        ),
    ] {
        let result = preview_codex_session_fixture(case, &value.to_string());
        let items = result
            .pointer("/session_rows/0/content_items")
            .and_then(Value::as_array)
            .expect("tool content items");

        assert_eq!(
            result
                .pointer("/session_rows/0/tool_call_count")
                .and_then(Value::as_u64),
            Some(1),
            "{case}: {result}"
        );
        assert_eq!(items.len(), 1, "{case}: {result}");
        assert_eq!(
            items[0].get("kind").and_then(Value::as_str),
            Some("tool_call"),
            "{case}: {result}"
        );
    }
}

#[test]
fn local_session_preview_skips_truncated_single_sidecar_record() {
    let filler = "ignored-file-history-data-".repeat(30_000);
    let session = format!(
        "{{\"type\":\"file-history-snapshot\",\"data\":{},\"text\":\"skipped-tail-must-not-surface\"}}\n",
        serde_json::to_string(&filler).expect("serialize filler")
    );
    let result = preview_codex_session_fixture("single-truncated-sidecar", &session);
    let serialized = serde_json::to_string(&result).expect("serialize skipped preview");

    assert_eq!(result.get("count").and_then(Value::as_u64), Some(0));
    assert!(result
        .get("session_rows")
        .and_then(Value::as_array)
        .is_some_and(Vec::is_empty));
    assert!(!serialized.contains("skipped-tail-must-not-surface"));
}

#[test]
fn local_session_preview_does_not_recover_scalars_nested_in_omitted_data() {
    let filler = "nested-private-blob-".repeat(36_000);
    let session = format!(
        "{{\"type\":\"user\",\"role\":\"user\",\"data\":{{\"blob\":{},\"text\":\"inside-data-must-not-surface\",\"type\":\"assistant\"}}}}\n",
        serde_json::to_string(&filler).expect("serialize nested filler")
    );
    let result = preview_codex_session_fixture("nested-omitted-data", &session);
    let serialized = serde_json::to_string(&result).expect("serialize nested-data preview");

    assert!(!serialized.contains("inside-data-must-not-surface"));
    assert!(!serialized.contains("nested-private-blob-"));
    assert_eq!(
        result.get("user_message_count").and_then(Value::as_u64),
        Some(0)
    );
}

fn preview_codex_session_fixture(test_name: &str, content: &str) -> Value {
    preview_codex_session_fixture_with_extension(test_name, "jsonl", content)
}

fn preview_codex_session_fixture_bytes(test_name: &str, content: &[u8]) -> Value {
    preview_codex_session_fixture_with_extension_bytes(test_name, "jsonl", content)
}

fn preview_codex_session_fixture_with_extension(
    test_name: &str,
    extension: &str,
    content: &str,
) -> Value {
    preview_codex_session_fixture_with_extension_bytes(test_name, extension, content.as_bytes())
}

fn preview_codex_session_fixture_with_extension_bytes(
    test_name: &str,
    extension: &str,
    content: &[u8],
) -> Value {
    preview_codex_session_fixture_with_extension_bytes_and_catalog_skill(
        test_name, extension, content, None,
    )
}

fn preview_codex_session_fixture_with_catalog_skill(
    test_name: &str,
    content: &str,
    skill_name: &str,
) -> Value {
    preview_codex_session_fixture_with_extension_bytes_and_catalog_skill(
        test_name,
        "jsonl",
        content.as_bytes(),
        Some(skill_name),
    )
}

fn preview_codex_session_fixture_with_extension_bytes_and_catalog_skill(
    test_name: &str,
    extension: &str,
    content: &[u8],
    catalog_skill_name: Option<&str>,
) -> Value {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-local-session-{test_name}-test-{}-{unique}",
        std::process::id(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-local-session-{test_name}-home-{}-{unique}",
        std::process::id(),
    ));
    let session_root = user_home.join(".codex/sessions/2026/07/10");
    fs::create_dir_all(&session_root).expect("create codex session root");
    fs::write(
        session_root.join(format!(
            "rollout-2026-07-10T08-00-00-{test_name}.{extension}"
        )),
        content,
    )
    .expect("write codex session fixture");
    let host = ServiceHost {
        app_data_dir: app_data_dir.clone(),
        adapter_ctx: AdapterContext {
            user_home: user_home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
    };
    if let Some(skill_name) = catalog_skill_name {
        fs::create_dir_all(&host.app_data_dir).expect("create catalog directory");
        let catalog = Catalog::open(&host.catalog_path()).expect("open local-session catalog");
        catalog.init().expect("initialize local-session catalog");
        let skill_path = user_home
            .join(".codex/skills")
            .join(skill_name)
            .join("SKILL.md");
        let instance = SkillInstance {
            id: format!("{skill_name}-id"),
            agent: AgentId::Codex,
            scope: Scope::AgentGlobal,
            project_root: None,
            path: skill_path.clone(),
            display_path: skill_path,
            definition_id: format!("{skill_name}-definition"),
            name: skill_name.to_string(),
            display_name: skill_name.to_string(),
            description: "Local-session skill matching fixture.".to_string(),
            version: None,
            state: SkillState::Loaded,
            enabled: true,
            frontmatter_raw: format!("name: {skill_name}\ndescription: fixture\n"),
            body: "Fixture body.".to_string(),
            scripts: Vec::new(),
            permissions: PermissionRequest::default(),
            fingerprint: format!("{skill_name}-fingerprint"),
            mtime: 1,
            first_seen: 1,
            last_seen: 1,
        };
        catalog
            .upsert_skill_instance(&instance)
            .expect("seed local-session skill");
    }
    let response = host.handle(ServiceRequest {
        id: Some(format!("session-preview-{test_name}")),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "codex",
            "limit": 10,
            "max_excerpt_chars": 800
        }),
    });
    if catalog_skill_name.is_none() {
        assert!(
            !app_data_dir.exists(),
            "session preview must not create app-local persistence"
        );
    }
    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
    assert!(response.ok, "{:?}", response.error);
    response.result.expect("local session preview result")
}
