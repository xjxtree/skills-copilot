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
        "{{\"type\":\"user\",\"role\":\"user\",\"data\":{},\"text\":\"single-line-tail-visible\",\"timestamp\":\"2026-07-10T08:09:10Z\"}}",
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

fn preview_codex_session_fixture(test_name: &str, content: &str) -> Value {
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
        session_root.join(format!("rollout-2026-07-10T08-00-00-{test_name}.jsonl")),
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
    let response = host.handle(ServiceRequest {
        id: Some(format!("session-preview-{test_name}")),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "agent": "codex",
            "limit": 10,
            "max_excerpt_chars": 800
        }),
    });
    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
    assert!(response.ok, "{:?}", response.error);
    response.result.expect("local session preview result")
}
