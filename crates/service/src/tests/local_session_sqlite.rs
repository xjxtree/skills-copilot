use super::*;
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
            "CREATE TABLE session (id TEXT PRIMARY KEY, project_id TEXT, parent_id TEXT, slug TEXT, directory TEXT, title TEXT, version TEXT, time_created INTEGER, time_updated INTEGER);\
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
