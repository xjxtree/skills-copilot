use super::*;
use rusqlite::{params, Connection};

#[test]
fn all_scope_retains_project_roots_for_supported_agents() {
    let unique = unique_suffix();
    let app_data_dir = env::temp_dir().join(format!(
        "skills-copilot-local-session-all-project-roots-test-{}-{unique}",
        std::process::id(),
    ));
    let user_home = env::temp_dir().join(format!(
        "skills-copilot-local-session-all-project-roots-home-{}-{unique}",
        std::process::id(),
    ));
    let project_root = app_data_dir.join("project-root");
    let project_root_text = project_root.to_string_lossy().to_string();
    let encoded_project = encoded_project_session_dir(&project_root);

    fs::create_dir_all(&project_root).expect("create project root");

    let claude_root = user_home.join(".claude/projects").join(&encoded_project);
    fs::create_dir_all(&claude_root).expect("create claude project session root");
    fs::write(
        claude_root.join("claude-session.jsonl"),
        format!(
            "{{\"type\":\"user\",\"message\":{{\"role\":\"user\",\"content\":\"Claude project task\"}},\"cwd\":\"{}\",\"sessionId\":\"claude-project-session\"}}\n",
            json_path_text(&project_root)
        ),
    )
    .expect("write claude session");

    let codex_root = user_home.join(".codex/sessions/2026/06/28");
    fs::create_dir_all(&codex_root).expect("create codex session root");
    fs::write(
        codex_root.join("rollout-2026-06-28T10-00-00-project.jsonl"),
        format!(
            "{{\"type\":\"session_meta\",\"payload\":{{\"id\":\"codex-project-session\",\"cwd\":\"{}\"}}}}\n{{\"type\":\"response_item\",\"payload\":{{\"type\":\"message\",\"role\":\"user\",\"content\":[{{\"type\":\"input_text\",\"text\":\"Codex project task\"}}]}}}}\n",
            json_path_text(&project_root)
        ),
    )
    .expect("write codex session");

    let opencode_db = user_home.join(".local/share/opencode/opencode.db");
    fs::create_dir_all(opencode_db.parent().expect("OpenCode database parent"))
        .expect("create OpenCode database directory");
    let opencode = Connection::open(&opencode_db).expect("create OpenCode database");
    opencode
        .execute_batch(
            "CREATE TABLE session (id TEXT PRIMARY KEY, project_id TEXT, parent_id TEXT, slug TEXT, directory TEXT, title TEXT, version TEXT, time_created INTEGER, time_updated INTEGER);\
             CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT, time_created INTEGER, time_updated INTEGER, data TEXT);\
             CREATE TABLE part (id TEXT PRIMARY KEY, message_id TEXT, session_id TEXT, time_created INTEGER, time_updated INTEGER, data TEXT);",
        )
        .expect("create current OpenCode schema");
    opencode
        .execute(
            "INSERT INTO session (id, directory, title, time_created, time_updated) VALUES (?1, ?2, ?3, ?4, ?5)",
            params![
                "ses_project",
                project_root_text,
                "opencode project task",
                1_000_i64,
                2_000_i64
            ],
        )
        .expect("write current OpenCode session");
    drop(opencode);

    let hermes_db = user_home.join(".hermes/state.db");
    fs::create_dir_all(hermes_db.parent().expect("Hermes database parent"))
        .expect("create Hermes database directory");
    let hermes = Connection::open(&hermes_db).expect("create Hermes database");
    hermes
        .execute_batch(
            "CREATE TABLE sessions (id TEXT PRIMARY KEY, source TEXT, parent_session_id TEXT, started_at REAL, ended_at REAL, message_count INTEGER, tool_call_count INTEGER, title TEXT);\
             CREATE TABLE messages (id INTEGER PRIMARY KEY, session_id TEXT, role TEXT, content TEXT, timestamp REAL, tool_name TEXT, tool_calls TEXT, reasoning TEXT);",
        )
        .expect("create Hermes schema");
    hermes
        .execute(
            "INSERT INTO sessions (id, source, started_at, ended_at, message_count, tool_call_count, title) VALUES (?1, 'cli', 1.0, 2.0, 0, 0, ?2)",
            params!["hermes-project-unknown", "Hermes unassigned session"],
        )
        .expect("write Hermes session");
    drop(hermes);

    let openclaw_state = user_home.join(".openclaw");
    fs::create_dir_all(&openclaw_state).expect("create OpenClaw state");
    fs::write(
        openclaw_state.join("openclaw.json"),
        format!(
            "{{agents: {{defaults: {{workspace: {:?}}}}}}}",
            project_root.to_string_lossy()
        ),
    )
    .expect("write OpenClaw config");
    let openclaw_db = openclaw_state.join("agents/main/agent/openclaw-agent.sqlite");
    fs::create_dir_all(openclaw_db.parent().expect("OpenClaw database parent"))
        .expect("create OpenClaw database directory");
    let openclaw = Connection::open(&openclaw_db).expect("create OpenClaw database");
    openclaw
        .execute_batch(
            "CREATE TABLE sessions (session_id TEXT PRIMARY KEY, session_key TEXT NOT NULL, session_scope TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, started_at INTEGER, ended_at INTEGER, status TEXT, chat_type TEXT, channel TEXT, account_id TEXT, primary_conversation_id TEXT, model_provider TEXT, model TEXT, agent_harness_id TEXT, parent_session_key TEXT, spawned_by TEXT, display_name TEXT);\
             CREATE TABLE session_routes (session_key TEXT NOT NULL, session_id TEXT NOT NULL, updated_at INTEGER NOT NULL);\
             CREATE TABLE session_entries (session_id TEXT NOT NULL, session_key TEXT NOT NULL, entry_json TEXT NOT NULL, updated_at INTEGER NOT NULL);\
             CREATE TABLE transcript_events (session_id TEXT NOT NULL, seq INTEGER NOT NULL, event_json TEXT NOT NULL, created_at INTEGER NOT NULL);",
        )
        .expect("create OpenClaw schema");
    openclaw
        .execute(
            "INSERT INTO sessions (session_id, session_key, created_at, updated_at, started_at, ended_at, status, display_name) VALUES (?1, ?2, 1000, 2000, 1000, 2000, 'active', ?3)",
            params![
                "openclaw-project-session",
                "agent:main:tui:project",
                "OpenClaw project task"
            ],
        )
        .expect("write OpenClaw session");
    openclaw
        .execute(
            "INSERT INTO session_routes (session_key, session_id, updated_at) VALUES (?1, ?2, 2000)",
            params!["agent:main:tui:project", "openclaw-project-session"],
        )
        .expect("write OpenClaw route");
    drop(openclaw);

    let pi_root = user_home.join(".pi/agent/sessions").join(&encoded_project);
    fs::create_dir_all(&pi_root).expect("create pi project session root");
    fs::write(
        pi_root.join("pi-session.jsonl"),
        format!(
            "{{\"type\":\"session\",\"id\":\"pi-project-session\",\"cwd\":\"{}\"}}\n{{\"type\":\"message\",\"message\":{{\"role\":\"user\",\"content\":[{{\"type\":\"text\",\"text\":\"Pi project task\"}}]}}}}\n",
            json_path_text(&project_root)
        ),
    )
    .expect("write pi session");

    let host = ServiceHost {
        app_data_dir: app_data_dir.clone(),
        adapter_ctx: AdapterContext {
            user_home: user_home.clone(),
            project_root: Some(project_root.clone()),
            project_cwd: Some(project_root.clone()),
            extra_roots: Vec::new(),
        },
    };

    for agent in ["claude-code", "codex", "opencode", "pi"] {
        let response = host.handle(ServiceRequest {
            id: Some(format!("session-preview-all-project-root-{agent}")),
            method: "session.previewLocalSessions".to_string(),
            params: json!({
                "agent": agent,
                "scope": "all",
                "project_root": project_root_text.clone(),
                "current_cwd": project_root_text.clone(),
                "limit": 10,
                "max_excerpt_chars": 800
            }),
        });

        assert!(response.ok, "{:?}", response.error);
        let result = response.result.expect("local session preview result");
        assert_eq!(
            result.get("count").and_then(Value::as_u64),
            Some(1),
            "{agent} should expose exactly the seeded project session"
        );
        assert_eq!(
            result
                .pointer("/session_rows/0/scope")
                .and_then(Value::as_str),
            Some("all"),
            "{agent} all-scope cache rows should keep the requested scope"
        );
        assert_eq!(
            result
                .pointer("/session_rows/0/project_root")
                .and_then(Value::as_str),
            Some("<project-root>"),
            "{agent} all-scope cache rows should retain redacted project-root metadata"
        );
    }

    let all_agents = host.handle(ServiceRequest {
        id: Some("session-preview-all-agents".to_string()),
        method: "session.previewLocalSessions".to_string(),
        params: json!({
            "scope": "all",
            "project_root": project_root_text,
            "current_cwd": project_root_text,
            "include_content_items": false,
            "paging_mode": "keyset",
            "limit": 100,
            "sort": "modified_at",
            "direction": "desc",
            "max_excerpt_chars": 800
        }),
    });
    assert!(all_agents.ok, "{:?}", all_agents.error);
    let result = all_agents.result.expect("all-agent local session result");
    let agents = result
        .get("session_rows")
        .and_then(Value::as_array)
        .expect("all-agent rows")
        .iter()
        .filter_map(|row| row.get("agent").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    assert_eq!(
        agents,
        BTreeSet::from([
            "claude-code",
            "codex",
            "opencode",
            "pi",
            "hermes",
            "openclaw"
        ])
    );
    let hermes_row = result
        .get("session_rows")
        .and_then(Value::as_array)
        .and_then(|rows| {
            rows.iter()
                .find(|row| row.get("agent").and_then(Value::as_str) == Some("hermes"))
        })
        .expect("Hermes row");
    assert!(
        hermes_row.get("project_root").is_none(),
        "Hermes must remain project-unassigned"
    );
    assert!(result
        .get("gap_notes")
        .and_then(Value::as_array)
        .is_some_and(|notes| notes.iter().any(|note| {
            note.as_str()
                .is_some_and(|note| note.contains("remain unassigned"))
        })));
    assert_eq!(
        result.get("source_completeness").and_then(Value::as_str),
        Some("enumerable")
    );

    let mut cursor = None;
    let mut source_revision = None;
    let mut paged_ids = BTreeSet::new();
    loop {
        let page = host.handle(ServiceRequest {
            id: Some("session-preview-all-agents-page".to_string()),
            method: "session.previewLocalSessions".to_string(),
            params: json!({
                "scope": "all",
                "project_root": project_root_text,
                "current_cwd": project_root_text,
                "include_content_items": false,
                "paging_mode": "keyset",
                "limit": 2,
                "cursor": cursor,
                "source_revision": source_revision,
                "sort": "modified_at",
                "direction": "desc",
                "max_excerpt_chars": 800
            }),
        });
        assert!(page.ok, "{:?}", page.error);
        let page = page.result.expect("all-agent page");
        for id in page
            .get("session_rows")
            .and_then(Value::as_array)
            .expect("paged rows")
            .iter()
            .filter_map(|row| row.get("id").and_then(Value::as_str))
        {
            assert!(paged_ids.insert(id.to_string()), "duplicate paged row {id}");
        }
        if !page
            .get("has_more")
            .and_then(Value::as_bool)
            .unwrap_or(false)
        {
            break;
        }
        let next_cursor = page
            .get("next_cursor")
            .and_then(Value::as_str)
            .expect("next cursor")
            .to_string();
        assert_ne!(cursor.as_deref(), Some(next_cursor.as_str()));
        cursor = Some(next_cursor);
        source_revision = page
            .get("source_revision")
            .and_then(Value::as_str)
            .map(ToString::to_string);
    }
    assert_eq!(paged_ids.len(), 6);

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}

#[test]
fn project_scope_matches_only_exact_project_root_or_current_cwd() {
    let unique = unique_suffix();
    let root = env::temp_dir().join(format!(
        "skills-copilot-local-session-exact-project-{}-{unique}",
        std::process::id()
    ));
    let home = root.join("home");
    let project = root.join("project");
    let current_cwd = project.join("packages/app");
    let unrelated_descendant = project.join("packages/other");
    fs::create_dir_all(&current_cwd).expect("create current cwd");
    fs::create_dir_all(&unrelated_descendant).expect("create unrelated descendant");
    let db_path = home.join(".local/share/opencode/opencode.db");
    fs::create_dir_all(db_path.parent().expect("OpenCode database parent"))
        .expect("create OpenCode database directory");
    let connection = Connection::open(&db_path).expect("create OpenCode database");
    connection
        .execute_batch(
            "CREATE TABLE session (id TEXT PRIMARY KEY, project_id TEXT, parent_id TEXT, slug TEXT, directory TEXT, title TEXT, version TEXT, time_created INTEGER, time_updated INTEGER);\
             CREATE TABLE message (id TEXT PRIMARY KEY, session_id TEXT, time_created INTEGER, time_updated INTEGER, data TEXT);\
             CREATE TABLE part (id TEXT PRIMARY KEY, message_id TEXT, session_id TEXT, time_created INTEGER, time_updated INTEGER, data TEXT);",
        )
        .expect("create OpenCode schema");
    for (id, directory, title, updated) in [
        ("root", project.as_path(), "Project root", 1_000_i64),
        ("cwd", current_cwd.as_path(), "Current cwd", 2_000_i64),
        (
            "descendant",
            unrelated_descendant.as_path(),
            "Unrelated descendant",
            3_000_i64,
        ),
    ] {
        connection
            .execute(
                "INSERT INTO session (id, directory, title, time_created, time_updated) VALUES (?1, ?2, ?3, ?4, ?4)",
                params![id, directory.to_string_lossy(), title, updated],
            )
            .expect("insert OpenCode session");
    }
    drop(connection);

    let host = ServiceHost {
        app_data_dir: root.join("app-data"),
        adapter_ctx: AdapterContext {
            user_home: home,
            project_root: Some(project.clone()),
            project_cwd: Some(current_cwd.clone()),
            extra_roots: Vec::new(),
        },
    };
    let result = host
        .preview_local_sessions(LocalSessionPreviewParams {
            auto_discover: Some(true),
            agent: Some("opencode".to_string()),
            scope: Some("project".to_string()),
            project_root: Some(project.to_string_lossy().to_string()),
            current_cwd: Some(current_cwd.to_string_lossy().to_string()),
            include_content_items: Some(false),
            limit: Some(20),
            ..LocalSessionPreviewParams::default()
        })
        .expect("project-scoped OpenCode sessions");
    let titles = result
        .session_rows
        .iter()
        .map(|row| row.title.as_str())
        .collect::<BTreeSet<_>>();
    assert_eq!(titles, BTreeSet::from(["Project root", "Current cwd"]));
    assert!(!titles.contains("Unrelated descendant"));

    let _ = fs::remove_dir_all(root);
}
