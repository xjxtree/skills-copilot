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

#[cfg(unix)]
#[test]
fn local_source_inspection_discovers_multiple_skills_without_installing() {
    use std::os::unix::fs::PermissionsExt;

    let root = std::env::temp_dir().join(format!(
        "skill-manager-local-source-inspection-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ));
    let source = root.join("source");
    for (directory, name, description) in [
        ("alpha", "alpha-skill", "Alpha local skill."),
        ("nested/beta", "beta-skill", "Beta local skill."),
    ] {
        let skill_dir = source.join(directory);
        fs::create_dir_all(&skill_dir).expect("create local skill directory");
        fs::write(
            skill_dir.join("SKILL.md"),
            format!("---\nname: {name}\ndescription: {description}\n---\nBody.\n"),
        )
        .expect("write local skill");
    }
    let fake_npx = root.join("npx");
    fs::write(
        &fake_npx,
        "#!/bin/sh\ncase \"$*\" in\n  *\"skills add \"*\" --list --full-depth\"*) printf 'Found 2 skills\\nAvailable Skills\\n' ;;\n  *) exit 64 ;;\nesac\n",
    )
    .expect("write fake npx");
    let mut permissions = fs::metadata(&fake_npx)
        .expect("fake npx metadata")
        .permissions();
    permissions.set_mode(0o755);
    fs::set_permissions(&fake_npx, permissions).expect("make fake npx executable");

    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: Some(root.join("project")),
        project_cwd: Some(root.join("project")),
        extra_roots: Vec::new(),
    };
    let inspection = inspect_local_source_with_executable(
        &ctx,
        &SkillManagerInspectLocalSourceParams {
            source_path: source.to_string_lossy().to_string(),
        },
        &fake_npx,
    )
    .expect("inspect local source");

    assert_eq!(
        inspection
            .skills
            .iter()
            .map(|skill| (skill.name.as_str(), skill.description.as_str()))
            .collect::<Vec<_>>(),
        vec![
            ("alpha-skill", "Alpha local skill."),
            ("beta-skill", "Beta local skill."),
        ]
    );
    assert!(inspection.source_revision.starts_with("sha256:"));
    assert_eq!(inspection.preview.operation, "inspectLocalSource");
    assert!(!inspection.preview.requires_confirmation);
    assert!(!inspection.preview.network_required);
    assert!(inspection.preview.command.contains(&"--list".to_string()));
    assert!(inspection
        .preview
        .command
        .contains(&"--full-depth".to_string()));
    assert!(!inspection.preview.command.contains(&"--agent".to_string()));
    assert!(!inspection.preview.command.contains(&"-y".to_string()));

    let _ = fs::remove_dir_all(root);
}

#[test]
fn local_source_install_preview_token_changes_with_directory_contents() {
    let root = std::env::temp_dir().join(format!(
        "skill-manager-local-source-preview-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ));
    let source = root.join("local-skill");
    fs::create_dir_all(&source).expect("create local source");
    let skill_path = source.join("SKILL.md");
    fs::write(
        &skill_path,
        "---\nname: local-skill\ndescription: First revision.\n---\nBody.\n",
    )
    .expect("write first revision");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: Some(root.join("project")),
        project_cwd: Some(root.join("project")),
        extra_roots: Vec::new(),
    };
    let params = SkillManagerInstallParams {
        source: source.to_string_lossy().to_string(),
        skills: vec!["local-skill".to_string()],
        agents: vec!["codex".to_string()],
        scope: Some("project".to_string()),
        distribution: None,
        network_allowed: false,
        confirmed: false,
        preview_token: None,
    };

    let first = preview_install_with_manager(&ctx, &params).expect("first local preview");
    fs::write(
        &skill_path,
        "---\nname: local-skill\ndescription: Second revision.\n---\nBody changed.\n",
    )
    .expect("write second revision");
    let second = preview_install_with_manager(&ctx, &params).expect("second local preview");

    assert_ne!(first.preview.preview_token, second.preview.preview_token);
    assert!(!first.preview.network_required);
    assert_eq!(first.preview.operation, "install");

    let _ = fs::remove_dir_all(root);
}

#[test]
fn local_source_install_rejects_preview_after_directory_contents_change() {
    let root = std::env::temp_dir().join(format!(
        "skill-manager-local-source-stale-preview-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ));
    let source = root.join("local-skill");
    fs::create_dir_all(&source).expect("create local source");
    let skill_path = source.join("SKILL.md");
    fs::write(
        &skill_path,
        "---\nname: local-skill\ndescription: First revision.\n---\nBody.\n",
    )
    .expect("write first revision");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: Some(root.join("project")),
        project_cwd: Some(root.join("project")),
        extra_roots: Vec::new(),
    };
    let mut params = SkillManagerInstallParams {
        source: source.to_string_lossy().to_string(),
        skills: vec!["local-skill".to_string()],
        agents: vec!["codex".to_string()],
        scope: Some("project".to_string()),
        distribution: None,
        network_allowed: false,
        confirmed: false,
        preview_token: None,
    };
    let preview = preview_install_with_manager(&ctx, &params).expect("local preview");
    fs::write(
        &skill_path,
        "---\nname: local-skill\ndescription: Second revision.\n---\nChanged.\n",
    )
    .expect("write changed revision");
    params.confirmed = true;
    params.preview_token = Some(preview.preview.preview_token);
    let catalog = Catalog::open(&root.join("catalog.sqlite")).expect("catalog opens");

    let error = apply_install_with_manager(&catalog, &ctx, &params)
        .expect_err("changed source must invalidate confirmation");

    assert!(matches!(
        error,
        CommandError::InvalidSkillManagerRequest(message)
            if message.contains("fresh preview_token")
    ));
    let _ = fs::remove_dir_all(root);
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
        "padding": "x".repeat(MAX_CAPTURE_BYTES),
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
        "padding": "x".repeat(MAX_CAPTURE_BYTES),
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

#[cfg(unix)]
#[test]
fn installed_inventory_distinguishes_separable_links_from_shared_consumers() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "skill-manager-separable-inventory-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ));
    let home = root.join("home");
    let shared_source = home.join(".agents/skills/shared-skill");
    fs::create_dir_all(&shared_source).expect("create shared source");
    fs::write(
        shared_source.join("SKILL.md"),
        "---\nname: shared-skill\ndescription: Shared test skill.\n---\n",
    )
    .expect("write shared skill");

    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };
    for agent in ["claude-code", "pi", "hermes-agent", "openclaw"] {
        let agent_root = physical_removal_roots(&ctx, agent, "global")
            .expect("supported Agent roots")
            .into_iter()
            .last()
            .expect("Agent removal root");
        fs::create_dir_all(&agent_root).expect("create Agent skills root");
        symlink(&shared_source, agent_root.join("shared-skill"))
            .expect("link separable Agent target");
    }
    let catalog = Catalog::open(&root.join("catalog.sqlite")).expect("catalog opens");
    catalog.init().expect("catalog initializes");
    scan_all_catalog_report(&ctx, &catalog).expect("scan installed targets");
    let mut installed = parse_installed_records(
        &serde_json::json!([{
            "name": "shared-skill",
            "path": shared_source,
            "agents": ["Claude Code", "Pi", "OpenCode", "Codex", "Hermes Agent", "OpenClaw"]
        }])
        .to_string(),
    )
    .expect("installed inventory row");

    enrich_installed_records(&ctx, Some("global"), &mut installed);
    enrich_installed_removal_capabilities(&catalog, &ctx, Some("global"), &mut installed)
        .expect("derive preview-equivalent removal capabilities");

    let wire = serde_json::to_value(&installed[0]).expect("serialized installed record");
    assert_eq!(
        wire.get("separable_agents"),
        Some(&serde_json::json!(["claude-code"])),
        "the inventory must require both a preview-safe link and an exact current catalog identity"
    );

    let _ = fs::remove_dir_all(root);
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

    let (preview, plan, physical_plan) =
        build_remove_preview(&catalog, &ctx, &params).expect("complete uninstall preview");

    assert_eq!(preview.operation, "remove");
    assert_eq!(plan.mode, "complete-uninstall");
    assert!(plan.full_uninstall);
    assert!(!plan.source_preserved);
    assert!(
        physical_plan.is_none(),
        "complete uninstall does not use a native physical-target preview"
    );
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

#[cfg(unix)]
#[test]
fn partial_remove_deletes_only_selected_agent_symlink_and_preserves_shared_source() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "skill-manager-partial-remove-{}",
        std::process::id()
    ));
    let home = root.join("home");
    let skill_dir = home.join(".agents/skills/shared-skill");
    let claude_link = home.join(".claude/skills/shared-skill");
    fs::create_dir_all(&skill_dir).expect("create shared skill");
    fs::create_dir_all(claude_link.parent().expect("Claude skills parent"))
        .expect("create Claude skills root");
    fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: shared-skill\ndescription: Shared test skill.\n---\n",
    )
    .expect("write shared skill");
    symlink("../../.agents/skills/shared-skill", &claude_link)
        .expect("link shared skill into Claude");
    let catalog = Catalog::open(&root.join("catalog.sqlite")).expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };
    scan_all_catalog_report(&ctx, &catalog).expect("initial scan");
    let initial = catalog.list_skill_records().expect("initial records");
    let claude = initial
        .iter()
        .find(|record| record.agent == "claude-code" && record.name == "shared-skill")
        .expect("Claude linked record");
    let codex = initial
        .iter()
        .find(|record| record.agent == "codex" && record.name == "shared-skill")
        .expect("Codex shared-source record");
    let all_instance_ids = initial
        .iter()
        .filter(|record| record.name == "shared-skill" && record.scope == "agent-global")
        .map(|record| record.id.clone())
        .collect::<Vec<_>>();
    let params = SkillManagerRemoveParams {
        skill: "shared-skill".to_string(),
        agents: vec!["claude-code".to_string()],
        instance_ids: all_instance_ids,
        scope: Some("global".to_string()),
        full_uninstall: false,
        confirmed: false,
        preview_token: None,
    };
    let preview =
        preview_remove_with_manager(&catalog, &ctx, &params).expect("partial remove preview");
    let plan = preview.removal_plan.as_ref().expect("removal plan");

    assert_eq!(preview.preview.tool_id, "agent-copilot-native");
    assert_eq!(plan.mode, "selected-agent-uninstall");
    assert!(plan.source_preserved);
    assert_eq!(plan.actions[0].strategy, "remove-symlink");
    assert!(skill_dir.join("SKILL.md").is_file(), "preview is read-only");
    assert!(
        fs::symlink_metadata(&claude_link).is_ok(),
        "preview keeps the selected link"
    );

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
    let updated_claude = updated
        .iter()
        .find(|record| record.id == claude.id)
        .expect("updated Claude record");
    let updated_codex = updated
        .iter()
        .find(|record| record.id == codex.id)
        .expect("updated Codex record");

    assert_eq!(updated_claude.state, "missing");
    assert!(
        fs::symlink_metadata(&claude_link).is_err(),
        "the selected Agent symlink must be removed"
    );
    assert!(
        skill_dir.join("SKILL.md").is_file(),
        "partial uninstall must preserve the source for unselected agents"
    );
    assert!(
        updated_codex.enabled && updated_codex.state != "missing",
        "the unselected Codex instance remains installed"
    );
    assert!(
        !home.join(".claude/settings.json").exists(),
        "Skill Manager removal must not write Agent enable/disable config"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn partial_remove_deletes_a_selected_copy_and_preserves_other_agent_source() {
    let root = std::env::temp_dir().join(format!(
        "skill-manager-partial-copy-remove-{}",
        std::process::id()
    ));
    let home = root.join("home");
    let shared_source = home.join(".agents/skills/copied-skill");
    let claude_copy = home.join(".claude/skills/copied-skill");
    for directory in [&shared_source, &claude_copy] {
        fs::create_dir_all(directory).expect("create skill directory");
        fs::write(
            directory.join("SKILL.md"),
            "---\nname: copied-skill\ndescription: Copied test skill.\n---\n",
        )
        .expect("write copied skill");
        fs::write(directory.join("helper.txt"), "helper").expect("write helper");
    }
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
    let claude = initial
        .iter()
        .find(|record| record.agent == "claude-code" && record.name == "copied-skill")
        .expect("Claude copied record");
    let params = SkillManagerRemoveParams {
        skill: "copied-skill".to_string(),
        agents: vec!["claude-code".to_string()],
        instance_ids: initial
            .iter()
            .filter(|record| record.name == "copied-skill" && record.scope == "agent-global")
            .map(|record| record.id.clone())
            .collect(),
        scope: Some("global".to_string()),
        full_uninstall: false,
        confirmed: false,
        preview_token: None,
    };
    let preview =
        preview_remove_with_manager(&catalog, &ctx, &params).expect("copy removal preview");
    assert_eq!(
        preview.removal_plan.as_ref().expect("plan").actions[0].strategy,
        "remove-copy-directory"
    );

    apply_remove_with_manager(
        &catalog,
        &ctx,
        &SkillManagerRemoveParams {
            confirmed: true,
            preview_token: Some(preview.preview.preview_token),
            ..params
        },
    )
    .expect("copy removal applies");

    assert!(
        !claude_copy.exists(),
        "selected copied directory is removed"
    );
    assert!(
        shared_source.join("SKILL.md").is_file(),
        "unselected shared source remains"
    );
    assert_eq!(
        catalog
            .get_skill_record(&claude.id)
            .expect("read Claude record")
            .expect("Claude record retained for history")
            .state,
        "missing"
    );
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn partial_remove_rejects_a_copy_that_is_an_unselected_agent_link_source() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "skill-manager-partial-dependent-link-{}",
        std::process::id()
    ));
    let home = root.join("home");
    let claude_copy = home.join(".claude/skills/copied-skill");
    let hermes_link = home.join(".hermes/skills/copied-skill");
    fs::create_dir_all(&claude_copy).expect("create Claude copy");
    fs::create_dir_all(hermes_link.parent().expect("Hermes skills parent"))
        .expect("create Hermes skills root");
    fs::write(
        claude_copy.join("SKILL.md"),
        "---\nname: copied-skill\ndescription: Copied test skill.\n---\n",
    )
    .expect("write copied skill");
    symlink(&claude_copy, &hermes_link).expect("link Hermes to Claude copy");
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
    let params = SkillManagerRemoveParams {
        skill: "copied-skill".to_string(),
        agents: vec!["claude-code".to_string()],
        instance_ids: initial
            .iter()
            .filter(|record| record.name == "copied-skill" && record.scope == "agent-global")
            .map(|record| record.id.clone())
            .collect(),
        scope: Some("global".to_string()),
        full_uninstall: false,
        confirmed: false,
        preview_token: None,
    };

    let error = preview_remove_with_manager(&catalog, &ctx, &params)
        .expect_err("a selected copy used by an unselected Agent must be preserved");
    assert!(
        matches!(
            &error,
            CommandError::InvalidSkillManagerRequest(message)
                if message.contains("directly share")
        ),
        "unexpected error: {error:?}"
    );
    assert!(
        claude_copy.join("SKILL.md").is_file(),
        "rejected preview keeps the selected copy"
    );
    assert!(
        hermes_link.join("SKILL.md").is_file(),
        "rejected preview keeps the unselected Agent link"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
fn partial_remove_rejects_agents_that_directly_share_one_source_directory() {
    let root = std::env::temp_dir().join(format!(
        "skill-manager-partial-shared-source-{}",
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
    let params = SkillManagerRemoveParams {
        skill: "shared-skill".to_string(),
        agents: vec!["codex".to_string()],
        instance_ids: initial
            .iter()
            .filter(|record| record.name == "shared-skill" && record.scope == "agent-global")
            .map(|record| record.id.clone())
            .collect(),
        scope: Some("global".to_string()),
        full_uninstall: false,
        confirmed: false,
        preview_token: None,
    };

    let error = preview_remove_with_manager(&catalog, &ctx, &params)
        .expect_err("a shared direct source cannot be partially deleted");
    assert!(
        matches!(
            &error,
            CommandError::InvalidSkillManagerRequest(message)
                if message.contains("directly share") || message.contains("shared canonical source")
        ),
        "unexpected error: {error:?}"
    );
    assert!(
        skill_dir.join("SKILL.md").is_file(),
        "blocked preview is read-only"
    );
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn partial_remove_requires_every_matching_physical_identity() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "skill-manager-partial-complete-identities-{}",
        std::process::id()
    ));
    let home = root.join("home");
    let shared_source = home.join(".agents/skills/shared-skill");
    let claude_link = home.join(".claude/skills/shared-skill");
    fs::create_dir_all(&shared_source).expect("create shared skill");
    fs::create_dir_all(claude_link.parent().expect("Claude skills parent"))
        .expect("create Claude skills root");
    fs::write(
        shared_source.join("SKILL.md"),
        "---\nname: shared-skill\ndescription: Shared test skill.\n---\n",
    )
    .expect("write shared skill");
    symlink(&shared_source, &claude_link).expect("create Claude link");
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
    let params = SkillManagerRemoveParams {
        skill: "shared-skill".to_string(),
        agents: vec!["claude-code".to_string()],
        instance_ids: initial
            .iter()
            .filter(|record| {
                record.name == "shared-skill"
                    && record.scope == "agent-global"
                    && record.agent != "codex"
            })
            .map(|record| record.id.clone())
            .collect(),
        scope: Some("global".to_string()),
        full_uninstall: false,
        confirmed: false,
        preview_token: None,
    };

    let error = preview_remove_with_manager(&catalog, &ctx, &params)
        .expect_err("an omitted matching physical identity must invalidate the preview");
    assert!(
        matches!(
            &error,
            CommandError::InvalidSkillManagerRequest(message)
                if message.contains("missing exact physical identities")
                    && message.contains("codex")
        ),
        "unexpected error: {error:?}"
    );
    assert!(
        fs::symlink_metadata(&claude_link).is_ok(),
        "rejected preview must preserve the selected link"
    );
    assert!(
        shared_source.join("SKILL.md").is_file(),
        "rejected preview must preserve the shared source"
    );
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn partial_remove_rejects_a_link_when_the_selected_agent_also_loads_the_shared_source() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "skill-manager-partial-redundant-link-{}",
        std::process::id()
    ));
    let home = root.join("home");
    let shared_source = home.join(".agents/skills/shared-skill");
    let opencode_link = home.join(".config/opencode/skills/shared-skill");
    fs::create_dir_all(&shared_source).expect("create shared skill");
    fs::create_dir_all(opencode_link.parent().expect("opencode skills parent"))
        .expect("create opencode skills root");
    fs::write(
        shared_source.join("SKILL.md"),
        "---\nname: shared-skill\ndescription: Shared test skill.\n---\n",
    )
    .expect("write shared skill");
    symlink(&shared_source, &opencode_link).expect("create opencode link");
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
    let params = SkillManagerRemoveParams {
        skill: "shared-skill".to_string(),
        agents: vec!["opencode".to_string()],
        instance_ids: initial
            .iter()
            .filter(|record| record.name == "shared-skill" && record.scope == "agent-global")
            .map(|record| record.id.clone())
            .collect(),
        scope: Some("global".to_string()),
        full_uninstall: false,
        confirmed: false,
        preview_token: None,
    };

    let error = preview_remove_with_manager(&catalog, &ctx, &params)
        .expect_err("removing a redundant link would not uninstall opencode");
    assert!(
        matches!(
            &error,
            CommandError::InvalidSkillManagerRequest(message)
                if message.contains("also loads") && message.contains("would not uninstall")
        ),
        "unexpected error: {error:?}"
    );
    assert!(
        fs::symlink_metadata(&opencode_link).is_ok(),
        "blocked preview keeps the link"
    );
    let _ = fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn partial_remove_confirmation_is_bound_to_the_previewed_physical_target() {
    use std::os::unix::fs::symlink;

    let root = std::env::temp_dir().join(format!(
        "skill-manager-partial-remove-binding-{}",
        std::process::id()
    ));
    let home = root.join("home");
    let skill_dir = home.join(".agents/skills/shared-skill");
    let alternate_dir = home.join(".agents/skills/alternate-shared-skill");
    let claude_link = home.join(".claude/skills/shared-skill");
    fs::create_dir_all(&skill_dir).expect("create shared skill");
    fs::create_dir_all(&alternate_dir).expect("create alternate shared skill");
    fs::create_dir_all(claude_link.parent().expect("Claude link parent"))
        .expect("create Claude skills root");
    for directory in [&skill_dir, &alternate_dir] {
        fs::write(
            directory.join("SKILL.md"),
            "---\nname: shared-skill\ndescription: Shared test skill.\n---\n",
        )
        .expect("write shared skill");
    }
    symlink(&skill_dir, &claude_link).expect("create Claude link");
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
    let params = SkillManagerRemoveParams {
        skill: "shared-skill".to_string(),
        agents: vec!["claude-code".to_string()],
        instance_ids: initial
            .iter()
            .filter(|record| record.name == "shared-skill" && record.scope == "agent-global")
            .map(|record| record.id.clone())
            .collect(),
        scope: Some("global".to_string()),
        full_uninstall: false,
        confirmed: false,
        preview_token: None,
    };
    let preview =
        preview_remove_with_manager(&catalog, &ctx, &params).expect("partial remove preview");
    fs::remove_file(&claude_link).expect("replace previewed link");
    symlink(&alternate_dir, &claude_link).expect("retarget Claude link");

    let error = apply_remove_with_manager(
        &catalog,
        &ctx,
        &SkillManagerRemoveParams {
            confirmed: true,
            preview_token: Some(preview.preview.preview_token),
            ..params
        },
    )
    .expect_err("a changed physical target must invalidate the confirmation");

    assert!(
        matches!(
            &error,
            CommandError::InvalidSkillManagerRequest(message)
                if message.contains("fresh preview_token")
        ),
        "unexpected error: {error:?}"
    );
    assert!(
        fs::symlink_metadata(&claude_link).is_ok(),
        "a stale confirmation must not delete the retargeted link"
    );
    let _ = fs::remove_dir_all(root);
}
