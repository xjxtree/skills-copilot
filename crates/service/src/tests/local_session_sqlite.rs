use super::*;
use crate::service_local_sessions::local_session_row_id;
use rusqlite::{params, Connection};

fn sqlite_host(label: &str) -> (ServiceHost, PathBuf) {
    let root = env::temp_dir().join(format!(
        "skills-copilot-{label}-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let home = root.join("home");
    fs::create_dir_all(&home).expect("create test home");
    (
        ServiceHost {
            app_data_dir: root.join("app-data"),
            adapter_ctx: AdapterContext {
                user_home: home,
                project_root: None,
                project_cwd: None,
                extra_roots: Vec::new(),
            },
        },
        root,
    )
}

fn sqlite_preview_params(agent: &str) -> LocalSessionPreviewParams {
    LocalSessionPreviewParams {
        auto_discover: Some(true),
        agent: Some(agent.to_string()),
        limit: Some(20),
        ..LocalSessionPreviewParams::default()
    }
}

fn sqlite_message_params(
    agent: &str,
    session_id: &str,
    cursor: Option<String>,
    source_revision: Option<String>,
) -> LocalSessionMessagePageParams {
    LocalSessionMessagePageParams {
        authorized_roots: Vec::new(),
        auto_discover: Some(true),
        agent: Some(agent.to_string()),
        project_root: None,
        current_cwd: None,
        session_id: session_id.to_string(),
        limit: Some(40),
        cursor,
        source_revision,
    }
}

#[test]
fn codex_summary_uses_effective_thread_index_and_exact_project_scope() {
    let (host, root) = sqlite_host("codex-state-index");
    let codex_home = host.adapter_ctx.user_home.join(".codex");
    let sessions = codex_home.join("sessions/2026/07/22");
    fs::create_dir_all(&sessions).expect("create Codex sessions directory");
    let project_path = sessions.join("rollout-project.jsonl");
    let nested_path = sessions.join("rollout-nested.jsonl");
    let archived_path = sessions.join("rollout-archived.jsonl");
    let exec_path = sessions.join("rollout-exec.jsonl");
    let subagent_path = sessions.join("rollout-subagent.jsonl");
    for path in [
        &project_path,
        &nested_path,
        &archived_path,
        &exec_path,
        &subagent_path,
    ] {
        fs::write(path, "{\"type\":\"session_meta\",\"payload\":{}}\n")
            .expect("write rollout fixture");
    }
    let connection =
        Connection::open(codex_home.join("state_5.sqlite")).expect("open Codex state database");
    connection
        .execute_batch(
            "CREATE TABLE threads (id TEXT PRIMARY KEY, title TEXT NOT NULL, cwd TEXT NOT NULL, rollout_path TEXT NOT NULL, preview TEXT NOT NULL, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, created_at_ms INTEGER, updated_at_ms INTEGER, archived INTEGER NOT NULL, source TEXT NOT NULL);",
        )
        .expect("create Codex thread index schema");
    for (id, title, cwd, path, archived, source, timestamp) in [
        (
            "project",
            "Project thread",
            "/tmp/project",
            &project_path,
            0_i64,
            "vscode",
            5_000_i64,
        ),
        (
            "nested",
            "Nested cwd thread",
            "/tmp/project/nested",
            &nested_path,
            0_i64,
            "cli",
            4_000_i64,
        ),
        (
            "archived",
            "Archived thread",
            "/tmp/project",
            &archived_path,
            1_i64,
            "vscode",
            3_000_i64,
        ),
        (
            "exec",
            "Exec thread",
            "/tmp/project",
            &exec_path,
            0_i64,
            "exec",
            2_000_i64,
        ),
        (
            "subagent",
            "Subagent thread",
            "/tmp/project",
            &subagent_path,
            0_i64,
            r#"{"subagent":{"other":"guardian"}}"#,
            1_000_i64,
        ),
    ] {
        connection
            .execute(
                "INSERT INTO threads (id, title, cwd, rollout_path, preview, created_at, updated_at, created_at_ms, updated_at_ms, archived, source) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9, ?10, ?11)",
                params![
                    id,
                    title,
                    cwd,
                    path.to_string_lossy(),
                    format!("preview {title}"),
                    timestamp / 1_000,
                    timestamp / 1_000,
                    timestamp,
                    timestamp,
                    archived,
                    source
                ],
            )
            .expect("insert Codex thread");
    }
    drop(connection);

    let all = host
        .preview_local_sessions(LocalSessionPreviewParams {
            include_content_items: Some(false),
            scope: Some("all".to_string()),
            ..sqlite_preview_params("codex")
        })
        .expect("preview all Codex sessions");
    assert_eq!(
        all.session_rows
            .iter()
            .map(|row| row.title.as_str())
            .collect::<Vec<_>>(),
        vec!["Project thread", "Nested cwd thread"]
    );
    assert_eq!(all.source_completeness, ListSourceCompleteness::Enumerable);
    assert!(!all.candidate_set_truncated);
    assert_eq!(all.session_rows[0].id, local_session_row_id(&project_path));

    let project = host
        .preview_local_sessions(LocalSessionPreviewParams {
            include_content_items: Some(false),
            scope: Some("project".to_string()),
            project_root: Some("/tmp/project".to_string()),
            current_cwd: Some("/tmp/project".to_string()),
            ..sqlite_preview_params("codex")
        })
        .expect("preview project Codex sessions");
    assert_eq!(project.session_rows.len(), 1);
    assert_eq!(project.session_rows[0].title, "Project thread");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn opencode_sqlite_summary_is_bounded_and_final_messages_page_past_tool_noise() {
    let (host, root) = sqlite_host("opencode-sqlite");
    let db_path = host
        .adapter_ctx
        .user_home
        .join(".local/share/opencode/opencode.db");
    fs::create_dir_all(db_path.parent().expect("database parent")).expect("create database parent");
    let mut connection = Connection::open(&db_path).expect("open database");
    connection
        .execute_batch(
            "CREATE TABLE session (id TEXT PRIMARY KEY, project_id TEXT, parent_id TEXT, slug TEXT, directory TEXT, title TEXT, version TEXT, time_created INTEGER, time_updated INTEGER, time_archived INTEGER);\
             CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT, time_created INTEGER, time_updated INTEGER, data TEXT);\
             CREATE TABLE part (id TEXT PRIMARY KEY, message_id TEXT, session_id TEXT, time_created INTEGER, time_updated INTEGER, data TEXT);",
        )
        .expect("create OpenCode schema");
    connection
        .execute(
            "INSERT INTO session (id, directory, title, time_created, time_updated) VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["session-1", "/tmp/project", "Large OpenCode Session", 1_000_i64, 2_500_i64],
        )
        .expect("insert session");
    connection
        .execute(
            "INSERT INTO session (id, directory, title, time_created, time_updated, time_archived) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
            params!["session-archived", "/tmp/project", "Archived OpenCode Session", 500_i64, 600_i64, 700_i64],
        )
        .expect("insert archived session");
    connection
        .execute(
            "INSERT INTO message (id, session_id, time_created, time_updated, data) VALUES (?1, ?2, ?3, ?4, ?5)",
            params!["assistant-tools", "session-1", 1_001_i64, 2_200_i64, r#"{"role":"assistant"}"#],
        )
        .expect("insert tool message");
    let transaction = connection.transaction().expect("begin tool transaction");
    for index in 0..1_200 {
        transaction
            .execute(
                "INSERT INTO part (id, message_id, session_id, time_created, time_updated, data) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![
                    format!("tool-{index:04}"),
                    "assistant-tools",
                    "session-1",
                    1_001_i64 + index,
                    1_001_i64 + index,
                    r#"{"type":"tool","callID":"call"}"#
                ],
            )
            .expect("insert tool part");
    }
    transaction.commit().expect("commit tool transaction");
    for (message_id, role, text, timestamp) in [
        ("user-goal", "user", "目标消息必须展示", 2_300_i64),
        (
            "assistant-final",
            "assistant",
            "最终回复必须展示",
            2_400_i64,
        ),
    ] {
        connection
            .execute(
                "INSERT INTO message (id, session_id, time_created, time_updated, data) VALUES (?1, ?2, ?3, ?4, ?5)",
                params![message_id, "session-1", timestamp, timestamp, format!(r#"{{"role":"{role}"}}"#)],
            )
            .expect("insert final message");
        connection
            .execute(
                "INSERT INTO part (id, message_id, session_id, time_created, time_updated, data) VALUES (?1, ?2, ?3, ?4, ?5, ?6)",
                params![format!("part-{message_id}"), message_id, "session-1", timestamp, timestamp, serde_json::json!({"type":"text","text":text}).to_string()],
            )
            .expect("insert final part");
    }
    drop(connection);

    let preview = host
        .preview_local_sessions(sqlite_preview_params("opencode"))
        .expect("preview OpenCode sessions");
    assert_eq!(preview.session_rows.len(), 1);
    assert_eq!(preview.session_rows[0].title, "Large OpenCode Session");
    assert_eq!(preview.session_rows[0].user_message_count, 1);
    assert_eq!(preview.session_rows[0].tool_call_count, 1_200);
    assert_eq!(preview.session_rows[0].excerpt, "目标消息必须展示");
    let session_id = preview.session_rows[0].id.clone();

    let first = host
        .list_local_session_messages(sqlite_message_params("opencode", &session_id, None, None))
        .expect("first OpenCode message page");
    assert!(first.content_items.is_empty());
    assert!(first.has_more);
    let second = host
        .list_local_session_messages(sqlite_message_params(
            "opencode",
            &session_id,
            first.next_cursor,
            Some(first.source_revision),
        ))
        .expect("second OpenCode message page");
    assert_eq!(
        second
            .content_items
            .iter()
            .map(|item| (item.kind.as_str(), item.text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("user_message", "目标消息必须展示"),
            ("agent_reply", "最终回复必须展示")
        ]
    );
    assert!(!second.has_more);
    assert_eq!(second.total_count, Some(2));

    let detail = host
        .preview_local_sessions(LocalSessionPreviewParams {
            session_id: Some(session_id),
            include_content_items: Some(true),
            limit: Some(1),
            ..sqlite_preview_params("opencode")
        })
        .expect("exact OpenCode detail");
    assert_eq!(detail.session_rows.len(), 1);
    assert!(detail.session_rows[0]
        .content_items
        .iter()
        .all(|item| matches!(item.kind.as_str(), "thinking" | "tool_call")));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn hermes_sqlite_uses_current_state_database_and_pages_only_user_and_final_reply() {
    let (host, root) = sqlite_host("hermes-sqlite");
    let db_path = host.adapter_ctx.user_home.join(".hermes/state.db");
    fs::create_dir_all(db_path.parent().expect("database parent")).expect("create database parent");
    let connection = Connection::open(&db_path).expect("open database");
    connection
        .execute_batch(
            "CREATE TABLE sessions (id TEXT PRIMARY KEY, source TEXT, parent_session_id TEXT, started_at REAL, ended_at REAL, message_count INTEGER, tool_call_count INTEGER, title TEXT);\
             CREATE TABLE messages (id INTEGER PRIMARY KEY, session_id TEXT, role TEXT, content TEXT, timestamp REAL, tool_name TEXT, tool_calls TEXT, reasoning TEXT);",
        )
        .expect("create Hermes schema");
    connection
        .execute(
            "INSERT INTO sessions (id, source, started_at, ended_at, message_count, tool_call_count, title) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params!["hermes-1", "cli", 10.0_f64, 13.0_f64, 4_i64, 1_i64, "Hermes Current Session"],
        )
        .expect("insert Hermes session");
    connection
        .execute(
            "INSERT INTO sessions (id, source, started_at, ended_at, message_count, tool_call_count, title) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
            params!["hermes-memory", "memory_consolidation", 9.0_f64, 9.5_f64, 1_i64, 0_i64, "Hidden memory consolidation"],
        )
        .expect("insert Hermes internal session");
    for (role, content, timestamp, tool_name, tool_calls, reasoning) in [
        ("user", "Hermes 用户目标", 10.0_f64, "", "", ""),
        ("assistant", "", 11.0_f64, "", "", "内部思考"),
        ("assistant", "", 12.0_f64, "search", "{}", ""),
        ("assistant", "Hermes 最终回复", 13.0_f64, "", "", ""),
    ] {
        connection
            .execute(
                "INSERT INTO messages (session_id, role, content, timestamp, tool_name, tool_calls, reasoning) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7)",
                params!["hermes-1", role, content, timestamp, tool_name, tool_calls, reasoning],
            )
            .expect("insert Hermes message");
    }
    drop(connection);

    let preview = host
        .preview_local_sessions(sqlite_preview_params("hermes"))
        .expect("preview Hermes sessions");
    assert_eq!(preview.session_rows.len(), 1);
    assert_eq!(preview.session_rows[0].title, "Hermes Current Session");
    assert_eq!(preview.session_rows[0].user_message_count, 1);
    let page = host
        .list_local_session_messages(sqlite_message_params(
            "hermes",
            &preview.session_rows[0].id,
            None,
            None,
        ))
        .expect("Hermes message page");
    assert_eq!(
        page.content_items
            .iter()
            .map(|item| (item.kind.as_str(), item.text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("user_message", "Hermes 用户目标"),
            ("agent_reply", "Hermes 最终回复")
        ]
    );
    assert_eq!(page.total_count, Some(2));
    assert!(!page.has_more);

    let _ = fs::remove_dir_all(root);
}

#[test]
fn openclaw_uses_current_agent_sqlite_and_ignores_legacy_or_internal_sessions() {
    let (mut host, root) = sqlite_host("openclaw-sqlite");
    let workspace = root.join("workspace-main");
    fs::create_dir_all(&workspace).expect("create OpenClaw workspace");
    host.adapter_ctx.project_root = Some(workspace.clone());
    host.adapter_ctx.project_cwd = Some(workspace.clone());
    let state_dir = host.adapter_ctx.user_home.join(".openclaw");
    fs::create_dir_all(&state_dir).expect("create OpenClaw state dir");
    fs::write(
        state_dir.join("openclaw.json"),
        format!(
            "{{agents: {{defaults: {{workspace: {:?}}}}}}}",
            workspace.to_string_lossy()
        ),
    )
    .expect("write OpenClaw config");
    let db_path = state_dir.join("agents/main/agent/openclaw-agent.sqlite");
    fs::create_dir_all(db_path.parent().expect("database parent"))
        .expect("create OpenClaw agent dir");
    let connection = Connection::open(&db_path).expect("open OpenClaw database");
    connection
        .execute_batch(
            "CREATE TABLE sessions (session_id TEXT PRIMARY KEY, session_key TEXT NOT NULL, session_scope TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, started_at INTEGER, ended_at INTEGER, status TEXT, chat_type TEXT, channel TEXT, account_id TEXT, primary_conversation_id TEXT, model_provider TEXT, model TEXT, agent_harness_id TEXT, parent_session_key TEXT, spawned_by TEXT, display_name TEXT);\
             CREATE TABLE session_routes (session_key TEXT NOT NULL, session_id TEXT NOT NULL, updated_at INTEGER NOT NULL);\
             CREATE TABLE session_entries (session_id TEXT NOT NULL, session_key TEXT NOT NULL, entry_json TEXT NOT NULL, updated_at INTEGER NOT NULL);\
             CREATE TABLE transcript_events (session_id TEXT NOT NULL, seq INTEGER NOT NULL, event_json TEXT NOT NULL, created_at INTEGER NOT NULL);",
        )
        .expect("create OpenClaw schema");
    for (id, key, status, title, updated) in [
        (
            "active",
            "agent:main:main",
            "active",
            "Main chat",
            3_000_i64,
        ),
        (
            "archived",
            "agent:main:old",
            "archived",
            "Archived chat",
            2_000_i64,
        ),
        (
            "cron",
            "agent:main:cron:daily",
            "active",
            "Internal cron",
            1_000_i64,
        ),
    ] {
        connection
            .execute(
                "INSERT INTO sessions (session_id, session_key, created_at, updated_at, started_at, ended_at, status, display_name) VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8)",
                params![id, key, updated - 500, updated, updated - 500, updated, status, title],
            )
            .expect("insert OpenClaw session");
        connection
            .execute(
                "INSERT INTO session_routes (session_key, session_id, updated_at) VALUES (?1, ?2, ?3)",
                params![key, id, updated],
            )
            .expect("insert OpenClaw route");
        connection
            .execute(
                "INSERT INTO session_entries (session_id, session_key, entry_json, updated_at) VALUES (?1, ?2, '{}', ?3)",
                params![id, key, updated],
            )
            .expect("insert OpenClaw entry");
    }
    for (seq, event) in [
        (
            1_i64,
            serde_json::json!({"type":"message","message":{"role":"user","content":[{"type":"text","text":"OpenClaw 用户目标"}]}}),
        ),
        (
            2_i64,
            serde_json::json!({"type":"message","message":{"role":"assistant","content":[{"type":"toolCall","name":"read"}]}}),
        ),
        (
            3_i64,
            serde_json::json!({"type":"message","message":{"role":"assistant","content":[{"type":"text","text":"OpenClaw 最终回复"}]}}),
        ),
    ] {
        connection
            .execute(
                "INSERT INTO transcript_events (session_id, seq, event_json, created_at) VALUES ('active', ?1, ?2, ?3)",
                params![seq, event.to_string(), 2_000_i64 + seq],
            )
            .expect("insert OpenClaw transcript event");
    }
    let legacy_dir = state_dir.join("agents/main/sessions");
    fs::create_dir_all(&legacy_dir).expect("create legacy OpenClaw dir");
    fs::write(
        legacy_dir.join("sessions.json"),
        r#"{"legacy":{"sessionId":"legacy","updatedAt":9999}}"#,
    )
    .expect("write legacy store");
    drop(connection);

    let preview = host
        .preview_local_sessions(LocalSessionPreviewParams {
            scope: Some("project".to_string()),
            project_root: Some(workspace.to_string_lossy().to_string()),
            current_cwd: Some(workspace.to_string_lossy().to_string()),
            ..sqlite_preview_params("openclaw")
        })
        .expect("preview OpenClaw sessions");
    assert_eq!(preview.session_rows.len(), 1);
    assert_eq!(preview.session_rows[0].title, "Main chat");
    assert_eq!(preview.session_rows[0].user_message_count, 1);
    assert_eq!(preview.session_rows[0].tool_call_count, 1);
    assert_eq!(
        preview.session_rows[0].project_root.as_deref(),
        Some("<project-root>")
    );
    let page = host
        .list_local_session_messages(sqlite_message_params(
            "openclaw",
            &preview.session_rows[0].id,
            None,
            None,
        ))
        .expect("page OpenClaw messages");
    assert_eq!(
        page.content_items
            .iter()
            .map(|item| (item.kind.as_str(), item.text.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("user_message", "OpenClaw 用户目标"),
            ("agent_reply", "OpenClaw 最终回复")
        ]
    );

    let _ = fs::remove_dir_all(root);
}
