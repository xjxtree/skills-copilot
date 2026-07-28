use super::*;

#[test]
fn app_search_excludes_historical_missing_skills() {
    let root = env::temp_dir().join(format!(
        "agent-copilot-app-search-missing-{}-{}",
        std::process::id(),
        unique_suffix()
    ));
    let app_data_dir = root.join("app-data");
    fs::create_dir_all(&app_data_dir).expect("create app data");
    let host = test_host(app_data_dir);
    let catalog = Catalog::open(&host.catalog_path()).expect("open catalog");
    catalog.init().expect("init catalog");

    for (id, state) in [
        ("current-search-skill", SkillState::Loaded),
        ("deleted-search-skill", SkillState::Missing),
    ] {
        let path = root.join(id).join("SKILL.md");
        catalog
            .upsert_skill_instance(&SkillInstance {
                id: id.to_string(),
                agent: AgentId::Codex,
                scope: Scope::AgentGlobal,
                project_root: None,
                path: path.clone(),
                display_path: path,
                definition_id: format!("definition-{id}"),
                name: "shared-search-skill".to_string(),
                display_name: "Shared Search Skill".to_string(),
                description: String::new(),
                version: None,
                state,
                enabled: true,
                frontmatter_raw: String::new(),
                body: String::new(),
                scripts: Vec::new(),
                permissions: PermissionRequest::default(),
                fingerprint: format!("fingerprint-{id}"),
                mtime: 1,
                first_seen: 1,
                last_seen: 1,
            })
            .expect("upsert search skill");
    }

    let result = host
        .search_app(AppSearchParams {
            query: "shared-search".to_string(),
            auto_discover: Some(false),
            ..AppSearchParams::default()
        })
        .expect("search app");
    let skill_ids = result
        .items
        .iter()
        .filter(|item| item.kind == "skill")
        .map(|item| item.target_id.as_str())
        .collect::<Vec<_>>();

    assert_eq!(skill_ids, vec!["current-search-skill"]);
    fs::remove_dir_all(root).ok();
}
