use super::*;
use skills_copilot_core::{ListIncompleteReason, ListSourceCompleteness};

#[cfg(unix)]
use std::os::unix::fs::PermissionsExt;

fn test_preview(operation: &str) -> SkillManagerCommandPreview {
    SkillManagerCommandPreview {
        action: None,
        preconditions: Vec::new(),
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

    let results = parse_search_results(stdout).expect("recognized ANSI search results");

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

    let results = parse_search_results(&stdout).expect("recognized search results");

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
    let search = skill_manager_search_record(
        test_preview("search"),
        None,
        parse_search_results(&stdout).expect("recognized JSON search results"),
        None,
    );

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
fn search_parser_accepts_only_recognized_empty_results() {
    assert!(parse_search_results("No skills found").is_ok());
    for stdout in [
        "",
        "manager changed its output",
        r#"{"unexpected":[]}"#,
        r#"[{"description":"missing identity"}]"#,
        r#"["not a result row"]"#,
    ] {
        assert!(
            matches!(
                parse_search_results(stdout),
                Err(CommandError::SkillManagerCommandFailed(_))
            ),
            "unrecognized output must fail after the external process starts: {stdout:?}"
        );
    }
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
        "sha256:fixture".to_string(),
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
fn search_preview_does_not_create_manager_cwd_or_run_the_manager() {
    crate::initialize_action_preview_secret_for_test([0xA5; 32])
        .expect("initialize action preview test secret");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skill-manager-search-no-network-{}-{unique}",
        std::process::id()
    ));
    let user_home = root.join("home");
    fs::create_dir_all(&user_home).expect("create existing manager cwd");
    let ctx = AdapterContext {
        user_home: user_home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };

    let record = preview_search_skills_with_manager(
        &root.join("app-data"),
        &ctx,
        &SkillManagerSearchParams {
            query: "local-only".to_string(),
            owner: None,
            network_allowed: true,
        },
    )
    .expect("local search preview");

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
    assert!(record.preview.requires_confirmation);
    assert!(record.preview.action.is_some());
    assert!(user_home.is_dir());
    assert!(!root.join("app-data").exists());
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
        action: None,
        preconditions: Vec::new(),
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
fn every_raw_npx_mutation_requires_network_permission_before_app_data_bootstrap() {
    crate::initialize_action_preview_secret_for_test([0xA5; 32])
        .expect("initialize action preview test secret");
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skill-manager-network-gate-{}-{unique}",
        std::process::id()
    ));
    let home = root.join("home");
    let project = root.join("project");
    let local_source = project.join("local-source");
    fs::create_dir_all(&home).expect("home");
    fs::create_dir_all(&local_source).expect("local source");
    fs::write(
        local_source.join("SKILL.md"),
        "---\nname: local-source\ndescription: fixture\n---\n",
    )
    .expect("local source skill");
    let ctx = AdapterContext {
        user_home: home,
        project_root: Some(project.clone()),
        project_cwd: Some(project),
        extra_roots: Vec::new(),
    };

    let install_app_data = root.join("install-app-data");
    let install_params = SkillManagerInstallParams {
        source: "local-source".to_string(),
        skills: vec!["local-source".to_string()],
        agents: vec!["codex".to_string()],
        scope: Some("project".to_string()),
        distribution: None,
        network_allowed: false,
        confirmed: false,
        preview_token: None,
        action_reference: None,
    };
    let install_preview = preview_install_with_manager(&ctx, &install_params)
        .expect("blocked install still has an inspectable preview");
    assert!(install_preview.preview.network_required);
    assert!(!install_preview.preview.network_allowed);
    assert!(!install_preview.preview.will_run);
    assert!(matches!(
        apply_install_with_manager(&install_app_data, &ctx, &install_params),
        Err(CommandError::InvalidSkillManagerRequest(detail))
            if detail.contains("network_allowed=true")
    ));
    assert!(!install_app_data.exists());

    let remove_app_data = root.join("remove-app-data");
    let remove_params = SkillManagerRemoveParams {
        skill: "local-source".to_string(),
        agents: vec!["codex".to_string()],
        scope: Some("project".to_string()),
        cleanup_local_instance_id: None,
        network_allowed: false,
        confirmed: false,
        preview_token: None,
        action_reference: None,
    };
    let remove_preview =
        preview_remove_with_manager(&ctx, &remove_params).expect("blocked remove preview");
    assert!(remove_preview.preview.network_required);
    assert!(!remove_preview.preview.network_allowed);
    assert!(!remove_preview.preview.will_run);
    assert!(matches!(
        apply_remove_with_manager(None, &remove_app_data, &ctx, &remove_params),
        Err(CommandError::InvalidSkillManagerRequest(detail))
            if detail.contains("network_allowed=true")
    ));
    assert!(!remove_app_data.exists());

    let update_app_data = root.join("update-app-data");
    let update_params = SkillManagerUpdateParams {
        skills: vec!["local-source".to_string()],
        agents: Vec::new(),
        scope: Some("project".to_string()),
        network_allowed: false,
        confirmed: false,
        preview_token: None,
        action_reference: None,
    };
    let update_preview =
        preview_update_with_manager(&ctx, &update_params).expect("blocked update preview");
    assert!(update_preview.preview.network_required);
    assert!(!update_preview.preview.network_allowed);
    assert!(!update_preview.preview.will_run);
    assert!(matches!(
        apply_update_with_manager(&update_app_data, &ctx, &update_params),
        Err(CommandError::InvalidSkillManagerRequest(detail))
            if detail.contains("network_allowed=true")
    ));
    assert!(!update_app_data.exists());

    let local_create_app_data = root.join("local-create-app-data");
    let local_create_params = SkillManagerLocalCreateParams {
        name: "new-local".to_string(),
        network_allowed: false,
        confirmed: false,
        preview_token: None,
        action_reference: None,
    };
    let local_create_preview =
        preview_local_create_with_manager(&local_create_app_data, &ctx, &local_create_params)
            .expect("blocked local-create preview");
    assert!(local_create_preview.preview.network_required);
    assert!(!local_create_preview.preview.network_allowed);
    assert!(!local_create_preview.preview.will_run);
    assert!(matches!(
        apply_local_create_with_manager(&local_create_app_data, &ctx, &local_create_params),
        Err(CommandError::InvalidSkillManagerRequest(detail))
            if detail.contains("network_allowed=true")
    ));
    assert!(!local_create_app_data.exists());

    fs::remove_dir_all(root).ok();
}

#[test]
fn manager_apply_rejects_a_missing_working_directory_without_creating_it() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skill-manager-spawn-failure-{}-{unique}",
        std::process::id()
    ));
    fs::create_dir_all(&root).expect("create owner root");
    let cwd = root.join("created/by/manager");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };
    let preview = SkillManagerCommandPreview {
        action: None,
        preconditions: Vec::new(),
        tool_id: DEFAULT_MANAGER_TOOL.to_string(),
        operation: "install".to_string(),
        command: vec![root.join("missing-manager").to_string_lossy().to_string()],
        cwd: cwd.to_string_lossy().to_string(),
        env: Vec::new(),
        requires_confirmation: true,
        confirmed: true,
        network_required: false,
        network_allowed: true,
        will_run: true,
        preview_token: "test-token".to_string(),
        summary: "test".to_string(),
        risks: Vec::new(),
        source: None,
        skills: Vec::new(),
    };

    let error = match run_previewed_command(&ctx, &preview) {
        Ok(_) => panic!("spawn must fail"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        CommandError::InvalidSkillManagerRequest(detail)
            if detail.contains("must already exist")
    ));
    assert!(
        !root.join("created").exists(),
        "failed startup must restore the original zero-tree state"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn manager_rejects_working_directory_replacement_before_spawn_without_touching_victim() {
    use std::os::unix::fs::{symlink, PermissionsExt};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skill-manager-cwd-replacement-{}-{unique}",
        std::process::id()
    ));
    let cwd = root.join("project");
    let moved_cwd = root.join("accepted-project");
    let victim = root.join("victim");
    let process_marker = root.join("manager-process-started");
    let executable = root.join("fake-manager");
    fs::create_dir_all(&cwd).expect("create accepted cwd");
    fs::create_dir_all(&victim).expect("create victim");
    fs::write(cwd.join("accepted"), b"unchanged").expect("accepted cwd sentinel");
    fs::write(victim.join("sentinel"), b"unchanged").expect("victim sentinel");
    fs::write(
        &executable,
        format!("#!/bin/sh\n/usr/bin/touch '{}'\n", process_marker.display()),
    )
    .expect("fake manager");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))
        .expect("make fake manager executable");
    let victim_mode = fs::metadata(&victim)
        .expect("victim metadata")
        .permissions()
        .mode()
        & 0o777;
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: Some(cwd.clone()),
        project_cwd: Some(cwd.clone()),
        extra_roots: Vec::new(),
    };
    let preview = SkillManagerCommandPreview {
        action: None,
        preconditions: Vec::new(),
        tool_id: DEFAULT_MANAGER_TOOL.to_string(),
        operation: "install".to_string(),
        command: vec![executable.to_string_lossy().to_string()],
        cwd: cwd.to_string_lossy().to_string(),
        env: Vec::new(),
        requires_confirmation: true,
        confirmed: true,
        network_required: false,
        network_allowed: true,
        will_run: true,
        preview_token: "test-token".to_string(),
        summary: "test".to_string(),
        risks: Vec::new(),
        source: None,
        skills: Vec::new(),
    };

    let raced_cwd = cwd.clone();
    let raced_moved_cwd = moved_cwd.clone();
    let raced_victim = victim.clone();
    install_manager_cwd_pre_spawn_test_hook("install", move || {
        fs::rename(&raced_cwd, &raced_moved_cwd).expect("move accepted cwd");
        symlink(&raced_victim, &raced_cwd).expect("replace cwd with victim link");
    });
    let result = run_previewed_command(&ctx, &preview);

    assert!(matches!(result, Err(CommandError::UnsafeConfigPath(_))));
    assert!(
        !process_marker.exists(),
        "the manager process must not start"
    );
    assert_eq!(
        fs::read(victim.join("sentinel")).expect("victim sentinel"),
        b"unchanged"
    );
    assert_eq!(
        fs::metadata(&victim)
            .expect("victim metadata")
            .permissions()
            .mode()
            & 0o777,
        victim_mode
    );
    assert_eq!(
        fs::read_dir(&victim).expect("victim entries").count(),
        1,
        "the replacement target receives no manager files"
    );
    assert_eq!(
        fs::read(moved_cwd.join("accepted")).expect("accepted cwd sentinel"),
        b"unchanged"
    );
    fs::remove_file(&cwd).expect("remove replacement link");
    fs::remove_dir_all(root).ok();
}

#[test]
#[cfg(unix)]
fn manager_cwd_replacement_during_process_is_partial_and_rolls_back_catalog_transaction() {
    use std::os::unix::fs::PermissionsExt;

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skill-manager-cwd-during-process-{}-{unique}",
        std::process::id()
    ));
    let cwd = root.join("project");
    let moved_cwd = root.join("accepted-project");
    let app_data = root.join("app-data");
    let executable = root.join("fake-manager");
    fs::create_dir_all(&cwd).expect("create accepted cwd");
    fs::create_dir_all(&app_data).expect("create app data");
    fs::write(cwd.join("accepted"), b"unchanged").expect("accepted cwd sentinel");
    fs::write(
        &executable,
        format!(
            "#!/bin/sh\n/bin/mv '{}' '{}'\n/bin/mkdir '{}'\n/usr/bin/touch child-wrote-here\n",
            cwd.display(),
            moved_cwd.display(),
            cwd.display()
        ),
    )
    .expect("fake manager");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))
        .expect("make fake manager executable");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: Some(cwd.clone()),
        project_cwd: Some(cwd.clone()),
        extra_roots: Vec::new(),
    };
    let preview = SkillManagerCommandPreview {
        action: None,
        preconditions: Vec::new(),
        tool_id: DEFAULT_MANAGER_TOOL.to_string(),
        operation: "install".to_string(),
        command: vec![executable.to_string_lossy().to_string()],
        cwd: cwd.to_string_lossy().to_string(),
        env: Vec::new(),
        requires_confirmation: true,
        confirmed: true,
        network_required: false,
        network_allowed: true,
        will_run: true,
        preview_token: "test-token".to_string(),
        summary: "test".to_string(),
        risks: Vec::new(),
        source: None,
        skills: Vec::new(),
    };
    let catalog = Catalog::open(&app_data.join("catalog.sqlite")).expect("catalog");
    catalog.init().expect("catalog schema");
    let before = catalog
        .catalog_scan_revision()
        .expect("catalog revision before test");
    let transaction = catalog
        .begin_immediate_transaction()
        .expect("catalog transaction");
    catalog
        .advance_catalog_scan_revision("test-cwd-swap", "accepted")
        .expect("stage a transaction-only revision");

    let execution_error = match run_previewed_command(&ctx, &preview) {
        Ok(_) => panic!("cwd replacement during execution must not verify"),
        Err(error) => error,
    };
    let error = rollback_manager_catalog_transaction(&ctx, &preview, transaction, execution_error);

    assert!(matches!(
        error,
        CommandError::PartialEffect {
            state,
            cleanup_required: true,
            ..
        } if state == "applied_unverified"
    ));
    assert!(
        moved_cwd.join("child-wrote-here").is_file(),
        "the child stayed on the retained original cwd inode"
    );
    assert!(
        !cwd.join("child-wrote-here").exists(),
        "the replacement path must not be accepted as readback"
    );
    assert_eq!(
        catalog
            .catalog_scan_revision()
            .expect("catalog revision after rollback"),
        before,
        "post-process cwd drift must not commit catalog state"
    );

    fs::remove_dir_all(root).ok();
}

#[test]
fn confirmed_manager_apply_requires_the_exact_preview_token() {
    let mut preview = test_preview("install");
    preview.requires_confirmation = true;
    preview.confirmed = true;

    let missing = ensure_confirmed(&preview, true, None, None);

    assert!(
        matches!(
            missing,
            Err(CommandError::ActionConfirmationRequired(detail))
                if detail.contains("fresh preview_token")
        ),
        "a confirmed manager apply without its preview token must fail closed"
    );
}

#[test]
#[cfg(unix)]
fn fresh_search_locked_stale_keeps_one_empty_coordination_owner() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skill-manager-fresh-search-locked-stale-{}-{unique}",
        std::process::id()
    ));
    let app_data = root.join("app-data");
    let process_marker = root.join("manager-process-started");
    fs::create_dir_all(&root).expect("create owner parent");

    install_manager_pre_execute_test_hook("search", || {});
    let result = with_search_mutation_lock(&app_data, |_| {
        Err::<(), _>(CommandError::StaleActionReference)
    });

    assert!(
        matches!(result, Err(CommandError::StaleActionReference)),
        "locked revalidation must preserve the stale result"
    );
    assert!(
        app_data.is_dir(),
        "the confirmed bootstrap owner remains so every waiter locks the same inode"
    );
    assert_eq!(
        fs::read_dir(&app_data)
            .expect("read bootstrap owner")
            .count(),
        0,
        "locked stale revalidation must not write replay state or any other app data"
    );
    assert!(
        !app_data.join("skill-manager-discovery-state.json").exists(),
        "locked stale revalidation must not reserve replay state"
    );
    assert!(
        !process_marker.exists(),
        "locked stale revalidation must not reach a manager process"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn fresh_manager_mutation_locked_stale_keeps_only_the_private_empty_owner() {
    for operation in ["install", "remove", "update", "localCreate"] {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "skill-manager-fresh-{operation}-locked-stale-{}-{unique}",
            std::process::id()
        ));
        let app_data = root.join("app-data");
        let process_marker = root.join("manager-process-started");
        let target_marker = root.join("manager-target-written");
        fs::create_dir_all(&root).expect("create owner parent");
        let ctx = AdapterContext {
            user_home: root.join("home"),
            project_root: Some(root.join("project")),
            project_cwd: Some(root.join("project")),
            extra_roots: Vec::new(),
        };
        let mut preview = test_preview(operation);
        preview.requires_confirmation = true;
        preview.confirmed = true;

        install_manager_pre_execute_test_hook(operation, || {});
        let result = prepare_manager_mutation(&app_data, &ctx, &preview, |_| {
            Err(CommandError::StaleActionReference)
        });

        assert!(
            matches!(result, Err(CommandError::StaleActionReference)),
            "{operation} must preserve the locked stale result"
        );
        assert!(app_data.is_dir(), "{operation} retains the owner inode");
        assert_eq!(
            fs::metadata(&app_data)
                .expect("owner metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700,
            "{operation} bootstrap owner must remain private"
        );
        assert_eq!(
            fs::read_dir(&app_data)
                .expect("read bootstrap owner")
                .count(),
            0,
            "{operation} stale revalidation must precede catalog, replay, and audit writes"
        );
        assert!(!app_data.join("catalog.sqlite").exists());
        assert!(!app_data.join("skill-manager-discovery-state.json").exists());
        assert!(!app_data.join("audit").exists());
        assert!(!process_marker.exists());
        assert!(!target_marker.exists());
        fs::remove_dir_all(root).ok();
    }
}

#[test]
#[cfg(unix)]
fn fresh_manager_mutation_initializes_catalog_only_after_locked_validation() {
    for operation in ["install", "remove", "update", "localCreate"] {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "skill-manager-fresh-{operation}-valid-{}-{unique}",
            std::process::id()
        ));
        let app_data = root.join("app-data");
        fs::create_dir_all(&root).expect("create owner parent");
        let ctx = AdapterContext {
            user_home: root.join("home"),
            project_root: Some(root.join("project")),
            project_cwd: Some(root.join("project")),
            extra_roots: Vec::new(),
        };
        let mut preview = test_preview(operation);
        preview.requires_confirmation = true;
        preview.confirmed = true;
        let (_mutation_lock, catalog) =
            prepare_manager_mutation(&app_data, &ctx, &preview, |_| Ok(()))
                .expect("locked validation initializes catalog");
        let _ = catalog
            .list_skill_records()
            .expect("catalog-backed action is available");
        assert_eq!(
            fs::metadata(&app_data)
                .expect("owner metadata")
                .permissions()
                .mode()
                & 0o777,
            0o700
        );
        assert!(app_data.join("catalog.sqlite").is_file());
        fs::remove_dir_all(root).ok();
    }
}

#[test]
#[cfg(unix)]
fn search_rejects_an_owner_path_replaced_after_lock_without_touching_the_victim() {
    use std::os::unix::fs::{symlink, PermissionsExt};

    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skill-manager-search-owner-replacement-{}-{unique}",
        std::process::id()
    ));
    let app_data = root.join("app-data");
    let moved_owner = root.join("locked-owner");
    let victim = root.join("victim");
    fs::create_dir_all(&app_data).expect("create app data");
    fs::create_dir_all(&victim).expect("create victim");
    fs::write(victim.join("sentinel"), b"unchanged").expect("victim sentinel");
    let victim_mode = fs::metadata(&victim)
        .expect("victim metadata")
        .permissions()
        .mode()
        & 0o777;

    let raced_app_data = app_data.clone();
    let raced_moved_owner = moved_owner.clone();
    let raced_victim = victim.clone();
    install_manager_pre_execute_test_hook("search", move || {
        fs::rename(&raced_app_data, &raced_moved_owner).expect("move locked owner");
        symlink(&raced_victim, &raced_app_data).expect("replace owner path");
    });
    let result = with_search_mutation_lock(&app_data, |_| Ok(()));

    assert!(
        matches!(result, Err(CommandError::UnsafeConfigPath(_))),
        "locked search must fail closed when the display path no longer names the owner inode"
    );
    assert_eq!(
        fs::read(victim.join("sentinel")).expect("victim sentinel"),
        b"unchanged"
    );
    assert_eq!(
        fs::metadata(&victim)
            .expect("victim metadata")
            .permissions()
            .mode()
            & 0o777,
        victim_mode
    );
    assert_eq!(
        fs::read_dir(&victim).expect("victim entries").count(),
        1,
        "the victim must receive no replay or temporary files"
    );
    assert_eq!(
        fs::read_dir(&moved_owner)
            .expect("locked owner entries")
            .count(),
        0,
        "fail-closed binding validation runs before replay reservation"
    );

    fs::remove_file(&app_data).expect("remove raced link");
    fs::remove_dir_all(root).ok();
}

#[test]
#[cfg(unix)]
fn app_owned_manager_mutations_reject_owner_replacement_before_any_effect() {
    use std::os::unix::fs::symlink;

    for operation in ["localCreate", "remove"] {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "skill-manager-{operation}-owner-replacement-{}-{unique}",
            std::process::id()
        ));
        let app_data = root.join("app-data");
        let moved_owner = root.join("locked-owner");
        let victim = root.join("victim");
        fs::create_dir_all(&app_data).expect("create app data");
        fs::create_dir_all(&victim).expect("create victim");
        fs::write(victim.join("sentinel"), b"unchanged").expect("victim sentinel");
        let ctx = AdapterContext {
            user_home: root.join("home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        };
        let preview = test_preview(operation);

        let raced_app_data = app_data.clone();
        let raced_moved_owner = moved_owner.clone();
        let raced_victim = victim.clone();
        install_manager_pre_execute_test_hook(operation, move || {
            fs::rename(&raced_app_data, &raced_moved_owner).expect("move locked owner");
            symlink(&raced_victim, &raced_app_data).expect("replace owner path");
        });
        let result = prepare_manager_mutation(&app_data, &ctx, &preview, |_| Ok(()));

        assert!(matches!(result, Err(CommandError::UnsafeConfigPath(_))));
        assert_eq!(
            fs::read(victim.join("sentinel")).expect("victim sentinel"),
            b"unchanged"
        );
        assert_eq!(
            fs::read_dir(&victim).expect("victim entries").count(),
            1,
            "the victim must receive no manager-created files"
        );
        assert_eq!(
            fs::read_dir(&moved_owner)
                .expect("locked owner entries")
                .count(),
            0,
            "binding validation must run before the mutation closure"
        );

        fs::remove_file(&app_data).expect("remove raced link");
        fs::remove_dir_all(root).ok();
    }
}

#[test]
#[cfg(unix)]
fn manager_catalog_initializes_under_the_existing_locked_owner() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skill-manager-existing-owner-catalog-{}-{unique}",
        std::process::id()
    ));
    let app_data = root.join("app-data");
    fs::create_dir_all(&app_data).expect("create existing owner");
    fs::write(app_data.join("sentinel"), b"unchanged").expect("seed existing owner");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };
    let preview = test_preview("install");

    let (mutation_lock, catalog) = prepare_manager_mutation(&app_data, &ctx, &preview, |_| Ok(()))
        .expect("prepare existing owner manager mutation");
    let _ = catalog.list_skill_records().expect("catalog schema");
    mutation_lock
        .owner_fs()
        .atomic_replace_private_file(Path::new("effect"), b"effect", "effect")
        .expect("owner-relative effect");
    assert_eq!(
        fs::read(app_data.join("sentinel")).expect("existing owner sentinel"),
        b"unchanged"
    );
    assert_eq!(
        fs::read(app_data.join("effect")).expect("manager effect"),
        b"effect"
    );
    assert!(
        app_data.join("catalog.sqlite").is_file(),
        "the catalog is initialized only after locked validation succeeds"
    );
    fs::remove_dir_all(root).ok();
}

#[test]
#[cfg(unix)]
fn committed_manager_mutations_report_partial_when_owner_rebinds_before_return() {
    use std::os::unix::fs::symlink;

    for operation in ["install", "update", "localCreate"] {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos();
        let root = std::env::temp_dir().join(format!(
            "skill-manager-{operation}-post-commit-owner-{}-{unique}",
            std::process::id()
        ));
        let app_data = root.join("app-data");
        let moved_owner = root.join("committed-owner");
        let victim = root.join("victim");
        fs::create_dir_all(&app_data).expect("app data");
        fs::create_dir_all(&victim).expect("victim");
        fs::write(victim.join("sentinel"), b"unchanged").expect("victim sentinel");
        let lock = crate::mutation_lock::lock_app_mutations(&app_data).expect("owner lock");
        let owner = lock.owner_fs();
        let mut preview = test_preview(operation);
        preview.requires_confirmation = true;
        preview.confirmed = true;
        let raced_app_data = app_data.clone();
        let raced_moved_owner = moved_owner.clone();
        let raced_victim = victim.clone();
        install_manager_post_commit_test_hook(operation, move || {
            fs::rename(&raced_app_data, &raced_moved_owner).expect("move committed owner");
            symlink(&raced_victim, &raced_app_data).expect("replace owner path");
        });

        let error = validate_manager_owner_after_commit(
            &AdapterContext {
                user_home: root.join("home"),
                project_root: None,
                project_cwd: None,
                extra_roots: Vec::new(),
            },
            &preview,
            &owner,
        )
        .expect_err("post-commit owner rebind");

        assert!(matches!(
            error,
            CommandError::PartialEffect {
                state: "applied_unverified",
                cleanup_required: true,
                ..
            }
        ));
        assert_eq!(
            fs::read(victim.join("sentinel")).expect("victim sentinel"),
            b"unchanged"
        );
        assert_eq!(fs::read_dir(&victim).expect("victim entries").count(), 1);
        fs::remove_file(&app_data).expect("remove raced link");
        fs::remove_dir_all(root).ok();
    }
}

#[test]
#[cfg(unix)]
fn stale_manager_target_tree_is_rejected_before_the_process_starts() {
    let unique = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .expect("system clock")
        .as_nanos();
    let root = std::env::temp_dir().join(format!(
        "skill-manager-stale-target-{}-{unique}",
        std::process::id()
    ));
    let cwd = root.join("project");
    let app_data = root.join("app-data");
    let skill_file = cwd.join(".agents/skills/example/SKILL.md");
    let marker = root.join("process-started");
    let executable = root.join("fake-manager");
    fs::create_dir_all(skill_file.parent().expect("skill parent")).expect("create target tree");
    fs::create_dir_all(&app_data).expect("create app data");
    fs::write(&skill_file, "before").expect("write initial skill");
    fs::write(
        &executable,
        format!("#!/bin/sh\n/usr/bin/touch '{}'\n", marker.display()),
    )
    .expect("write fake manager");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700))
        .expect("make fake manager executable");

    let expected_revision = manager_target_revision(&cwd).expect("preview target revision");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: Some(cwd.clone()),
        project_cwd: Some(cwd.clone()),
        extra_roots: Vec::new(),
    };
    let preview = SkillManagerCommandPreview {
        action: None,
        preconditions: vec![ActionPrecondition {
            kind: ActionPreconditionKind::TargetFile,
            target_id: cwd.to_string_lossy().to_string(),
            expected_revision,
        }],
        tool_id: DEFAULT_MANAGER_TOOL.to_string(),
        operation: "install".to_string(),
        command: vec![executable.to_string_lossy().to_string()],
        cwd: cwd.to_string_lossy().to_string(),
        env: Vec::new(),
        requires_confirmation: true,
        confirmed: true,
        network_required: false,
        network_allowed: true,
        will_run: true,
        preview_token: "test-token".to_string(),
        summary: "test".to_string(),
        risks: Vec::new(),
        source: None,
        skills: Vec::new(),
    };

    let raced_skill_file = skill_file.clone();
    install_manager_pre_execute_test_hook("install", move || {
        fs::write(&raced_skill_file, "after confirmation").expect("drift target tree")
    });
    let result = prepare_manager_mutation(&app_data, &ctx, &preview, |_| {
        validate_manager_preconditions(&ctx, &preview)
    });

    assert!(
        matches!(result, Err(CommandError::StaleActionReference)),
        "target drift after confirmation must be rejected before process execution"
    );
    assert!(
        !marker.exists(),
        "a stale manager action must not start the external process"
    );
    assert_eq!(
        fs::read_dir(&app_data).expect("read app data").count(),
        0,
        "locking a stale apply must not create a persistent lock artifact"
    );
    let _ = fs::remove_dir_all(root);
}
