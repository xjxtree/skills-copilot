use super::*;
use skills_copilot_core::{ListIncompleteReason, ListSourceCompleteness};

fn test_preview(operation: &str) -> SkillManagerCommandPreview {
    SkillManagerCommandPreview {
        tool_id: DEFAULT_MANAGER_TOOL.to_string(),
        operation: operation.to_string(),
        command: vec!["/usr/bin/false".to_string()],
        cwd: "/tmp".to_string(),
        env: Vec::new(),
        requires_confirmation: false,
        confirmed: false,
        network_required: operation == "search",
        network_allowed: true,
        will_run: false,
        preview_token: "test-token".to_string(),
        summary: "test".to_string(),
        risks: Vec::new(),
        source: None,
        skills: Vec::new(),
    }
}

#[test]
fn search_parser_extracts_ansi_find_results() {
    let stdout = "\n\u{1b}[38;5;102mInstall with\u{1b}[0m npx skills add <owner/repo@skill>\n\n\u{1b}[38;5;145mobra/superpowers@brainstorming\u{1b}[0m \u{1b}[36m245.4K installs\u{1b}[0m\n\u{1b}[38;5;102m└ https://skills.sh/obra/superpowers/brainstorming\u{1b}[0m\n\n\u{1b}[38;5;145mobra/superpowers@systematic-debugging\u{1b}[0m \u{1b}[36m161.5K installs\u{1b}[0m\n\u{1b}[38;5;102m└ https://skills.sh/obra/superpowers/systematic-debugging\u{1b}[0m\n";

    let results = parse_search_results(stdout);

    assert_eq!(results.len(), 2);
    assert_eq!(results[0].name, "brainstorming");
    assert_eq!(results[0].source.as_deref(), Some("obra/superpowers"));
    assert_eq!(results[0].description.as_deref(), Some("245.4K installs"));
    assert_eq!(results[1].name, "systematic-debugging");
    assert_eq!(results[1].source.as_deref(), Some("obra/superpowers"));
}

#[test]
fn search_parser_does_not_cap_returned_manager_rows() {
    let stdout = (0..55)
        .map(|index| format!("owner/repo@skill-{index} Result {index}"))
        .collect::<Vec<_>>()
        .join("\n");

    let results = parse_search_results(&stdout);

    assert_eq!(results.len(), 55);
}

#[test]
fn search_record_preserves_all_returned_rows_without_claiming_source_total() {
    let stdout = serde_json::to_string(
        &(0..35)
            .map(|index| {
                serde_json::json!({
                    "name": format!("skill-{index}"),
                    "source": "owner/repo",
                    "description": format!("Result {index}")
                })
            })
            .collect::<Vec<_>>(),
    )
    .expect("search JSON");
    let search =
        skill_manager_search_record(test_preview("search"), None, parse_search_results(&stdout));

    assert_eq!(search.results.len(), 35);
    assert_eq!(search.page.returned_count, 35);
    assert_eq!(search.page.total_count, None);
    assert_eq!(
        search.page.source_completeness,
        ListSourceCompleteness::Unknown
    );
    assert_eq!(
        search.page.incomplete_reason,
        Some(ListIncompleteReason::SourceLimited)
    );
}

#[test]
fn installed_record_reports_exact_enumerable_total() {
    let stdout = serde_json::to_string(&serde_json::json!({
        "skills": (0..27)
            .map(|index| serde_json::json!({
                "name": format!("installed-{index}"),
                "source": "owner/repo"
            }))
            .collect::<Vec<_>>()
    }))
    .expect("installed JSON");
    let installed = skill_manager_installed_record(
        test_preview("listInstalled"),
        SkillManagerCommandOutput {
            status: "completed".to_string(),
            exit_code: Some(0),
            stdout: stdout.clone(),
            stderr: String::new(),
        },
        parse_installed_records(&stdout).expect("valid installed JSON"),
    );

    assert_eq!(installed.installed.len(), 27);
    assert_eq!(installed.page.returned_count, 27);
    assert_eq!(installed.page.total_count, Some(27));
    assert_eq!(
        installed.page.source_completeness,
        ListSourceCompleteness::Enumerable
    );
    assert_eq!(installed.page.incomplete_reason, None);
}

#[test]
fn installed_parser_rejects_truncated_json_instead_of_claiming_exact_empty() {
    let stdout = serde_json::to_string(&serde_json::json!({
        "skills": (0..400)
            .map(|index| serde_json::json!({
                "name": format!("installed-{index}"),
                "source": format!("owner/repository-with-a-long-name-{index}"),
                "path": format!("/tmp/installed/{index}/SKILL.md")
            }))
            .collect::<Vec<_>>()
    }))
    .expect("large installed JSON");
    assert!(stdout.len() > MAX_CAPTURE_BYTES);
    let truncated = truncate_capture(&stdout);

    let result = parse_installed_records(&truncated);

    assert!(matches!(
        result,
        Err(CommandError::SkillManagerCommandFailed(detail))
            if detail == "listInstalled returned invalid or truncated JSON"
    ));
}

#[test]
fn installed_parser_accepts_complete_json_larger_than_diagnostic_capture_limit() {
    let stdout = serde_json::to_string(&serde_json::json!({
        "skills": (0..400)
            .map(|index| serde_json::json!({
                "name": format!("installed-{index}"),
                "source": format!("owner/repository-with-a-long-name-{index}"),
                "path": format!("/tmp/installed/{index}/SKILL.md"),
                "scope": "global",
                "agents": ["codex", "claude-code", "unsupported-agent"]
            }))
            .collect::<Vec<_>>()
    }))
    .expect("large installed JSON");
    assert!(stdout.len() > MAX_CAPTURE_BYTES);

    let mut installed = parse_installed_records(&stdout).expect("complete machine JSON");
    enrich_installed_records(
        &AdapterContext {
            user_home: PathBuf::from("/tmp"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
        Some("global"),
        &mut installed,
    );

    assert_eq!(installed.len(), 400);
    assert_eq!(installed[0].agents, vec!["claude-code", "codex"]);
    let wire = serde_json::to_value(&installed[0]).expect("serialized installed record");
    assert!(wire.get("raw").is_none(), "raw manager rows stay internal");
    assert_eq!(
        wire.get("path").and_then(Value::as_str),
        Some("$HOME/installed/0/SKILL.md"),
        "the dedicated identity path is retained in redacted form"
    );
}

#[test]
fn installed_parser_normalizes_cli_agent_display_names_to_supported_ids() {
    let installed = parse_installed_records(
        r#"[{"name":"shared-skill","agents":["Claude Code","Codex","OpenCode","Pi","Hermes Agent","OpenClaw","Windsurf"]}]"#,
    )
    .expect("CLI display-name agents");

    assert_eq!(
        installed[0].agents,
        vec![
            "claude-code",
            "codex",
            "hermes-agent",
            "openclaw",
            "opencode",
            "pi",
        ]
    );
}

#[test]
fn unlocked_installed_inventory_rows_remain_local_sources() {
    let ctx = AdapterContext {
        user_home: PathBuf::from("/home/example"),
        project_root: Some(PathBuf::from("/workspace/project")),
        project_cwd: Some(PathBuf::from("/workspace/project")),
        extra_roots: Vec::new(),
    };
    let mut installed = parse_installed_records(
        r#"[{"name":"bug-fix","path":"/workspace/project/.agents/skills/bug-fix","agents":["Codex"]}]"#,
    )
    .expect("local inventory row");

    enrich_installed_records(&ctx, Some("project"), &mut installed);

    assert_eq!(installed[0].source_kind, "local");
    assert_eq!(
        installed[0].path.as_deref(),
        Some("<project-root>/.agents/skills/bug-fix")
    );
    assert_eq!(
        installed[0].source.as_deref(),
        Some("<project-root>/.agents/skills/bug-fix")
    );
}

#[test]
fn installed_parser_rejects_malformed_or_unrecognized_json_instead_of_exact_empty() {
    for stdout in ["{not-json", r#"{"unexpected":[]}"#] {
        let result = parse_installed_records(stdout);

        assert!(matches!(
            result,
            Err(CommandError::SkillManagerCommandFailed(detail))
                if detail == "listInstalled returned invalid or truncated JSON"
        ));
    }
}

#[test]
fn installed_parser_accepts_a_recognized_exact_empty_list() {
    let installed =
        parse_installed_records(r#"{"skills":[]}"#).expect("recognized empty installed list");

    assert!(installed.is_empty());
}

#[test]
fn search_without_network_does_not_create_manager_cwd() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skill-manager-search-no-network-{}-{unique}",
        std::process::id()
    ));
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };

    let record = search_skills_with_manager(
        &ctx,
        &SkillManagerSearchParams {
            query: "local-only".to_string(),
            owner: None,
            network_allowed: false,
        },
    )
    .expect("preview prohibited network search");

    assert!(record.output.is_none());
    assert!(record.results.is_empty());
    assert_eq!(record.page.returned_count, 0);
    assert_eq!(record.page.total_count, None);
    assert_eq!(
        record.page.source_completeness,
        ListSourceCompleteness::Unknown
    );
    assert_eq!(
        record.page.incomplete_reason,
        Some(ListIncompleteReason::SourceLimited)
    );
    assert!(!ctx.user_home.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn prohibited_previewed_command_does_not_create_cwd() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skill-manager-prohibited-command-{}-{unique}",
        std::process::id()
    ));
    let cwd = root.join("missing-cwd");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };
    let preview = SkillManagerCommandPreview {
        tool_id: DEFAULT_MANAGER_TOOL.to_string(),
        operation: "search".to_string(),
        command: vec!["/usr/bin/false".to_string()],
        cwd: cwd.to_string_lossy().to_string(),
        env: Vec::new(),
        requires_confirmation: false,
        confirmed: false,
        network_required: true,
        network_allowed: false,
        will_run: false,
        preview_token: "test-token".to_string(),
        summary: "test".to_string(),
        risks: Vec::new(),
        source: None,
        skills: Vec::new(),
    };

    let result = run_previewed_command(&ctx, &preview);

    assert!(matches!(
        result,
        Err(CommandError::InvalidSkillManagerRequest(_))
    ));
    assert!(!cwd.exists());
    let _ = fs::remove_dir_all(root);
}

#[test]
fn complete_uninstall_targets_every_external_manager_agent() {
    let root = std::env::temp_dir().join(format!(
        "skill-manager-complete-remove-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create complete-remove root");
    let catalog = Catalog::open(&root.join("catalog.sqlite")).expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };
    let params = SkillManagerRemoveParams {
        skill: "shared-skill".to_string(),
        agents: vec!["codex".to_string(), "opencode".to_string()],
        instance_ids: Vec::new(),
        scope: Some("global".to_string()),
        full_uninstall: true,
        confirmed: false,
        preview_token: None,
    };

    let (preview, plan) =
        build_remove_preview(&catalog, &ctx, &params).expect("complete uninstall preview");

    assert_eq!(preview.operation, "remove");
    assert_eq!(plan.mode, "complete-uninstall");
    assert!(plan.full_uninstall);
    assert!(!plan.source_preserved);
    assert_eq!(
        preview
            .command
            .iter()
            .filter(|argument| argument.as_str() == "--agent")
            .count(),
        0,
        "omitting --agent is the external CLI's all-target complete uninstall contract"
    );
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn complete_uninstall_postcheck_treats_a_dangling_link_as_a_remaining_entry() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "skill-manager-dangling-remove-{}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create dangling-remove root");
    let link = root.join("shared-skill");
    symlink(root.join("missing-source"), &link).expect("create dangling skill link");

    assert!(!link.exists(), "standard exists follows the missing target");
    assert!(
        removal_path_entry_exists(&link),
        "complete-uninstall verification must still detect the link entry"
    );
    let _ = fs::remove_file(link);
    let _ = fs::remove_dir_all(root);
}

#[test]
fn partial_remove_detaches_only_selected_agent_and_preserves_shared_source() {
    let root = std::env::temp_dir().join(format!(
        "skill-manager-partial-remove-{}",
        std::process::id()
    ));
    let home = root.join("home");
    let skill_dir = home.join(".agents/skills/shared-skill");
    fs::create_dir_all(&skill_dir).expect("create shared skill");
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: shared-skill\ndescription: Shared test skill.\n---\n",
    )
    .expect("write shared skill");
    let catalog = Catalog::open(&root.join("catalog.sqlite")).expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };
    scan_all_catalog_report(&ctx, &catalog).expect("initial scan");
    let initial = catalog.list_skill_records().expect("initial records");
    let codex = initial
        .iter()
        .find(|record| record.agent == "codex" && record.name == "shared-skill")
        .expect("Codex shared record");
    let opencode = initial
        .iter()
        .find(|record| record.agent == "opencode" && record.name == "shared-skill")
        .expect("opencode shared record");
    let params = SkillManagerRemoveParams {
        skill: "shared-skill".to_string(),
        agents: vec!["codex".to_string()],
        instance_ids: vec![codex.id.clone()],
        scope: Some("global".to_string()),
        full_uninstall: false,
        confirmed: false,
        preview_token: None,
    };
    let preview =
        preview_remove_with_manager(&catalog, &ctx, &params).expect("partial remove preview");
    let plan = preview.removal_plan.as_ref().expect("removal plan");

    assert_eq!(preview.preview.tool_id, "agent-copilot-native");
    assert_eq!(plan.mode, "selected-agent-detach");
    assert!(plan.source_preserved);
    assert!(skill_dir.join("SKILL.md").is_file(), "preview is read-only");

    let applied = apply_remove_with_manager(
        &catalog,
        &ctx,
        &SkillManagerRemoveParams {
            confirmed: true,
            preview_token: Some(preview.preview.preview_token),
            ..params
        },
    )
    .expect("partial remove applies");
    let updated = applied.updated_skills;
    let updated_codex = updated
        .iter()
        .find(|record| record.id == codex.id)
        .expect("updated Codex record");
    let updated_opencode = updated
        .iter()
        .find(|record| record.id == opencode.id)
        .expect("updated opencode record");

    assert!(!updated_codex.enabled);
    assert!(updated_opencode.enabled);
    assert!(
        skill_dir.join("SKILL.md").is_file(),
        "partial detach must preserve the source for unselected agents"
    );
    let _ = fs::remove_dir_all(root);
}
