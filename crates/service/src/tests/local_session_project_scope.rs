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

    let _ = fs::remove_dir_all(app_data_dir);
    let _ = fs::remove_dir_all(user_home);
}
