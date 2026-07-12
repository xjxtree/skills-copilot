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
        r#"{"type":"session","id":"custom-home-session"}"#,
    )
    .expect("write custom Codex session");
    fs::write(
        codex_home.join("session_index.jsonl"),
        json!({"id": "custom-home-session", "thread_name": "Custom home title"}).to_string(),
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
