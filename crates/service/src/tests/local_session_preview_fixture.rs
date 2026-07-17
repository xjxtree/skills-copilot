pub(super) fn preview_codex_session_fixture(test_name: &str, content: &str) -> Value {
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
