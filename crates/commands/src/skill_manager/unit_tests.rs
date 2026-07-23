use super::*;

fn semantic_test_preview(operation: &str, skills: Vec<&str>) -> SkillManagerCommandPreview {
    SkillManagerCommandPreview {
        action: None,
        preconditions: Vec::new(),
        tool_id: DEFAULT_MANAGER_TOOL.to_string(),
        operation: operation.to_string(),
        command: vec!["/usr/bin/true".to_string(), "--global".to_string()],
        cwd: "/tmp".to_string(),
        env: Vec::new(),
        requires_confirmation: true,
        confirmed: true,
        network_required: false,
        network_allowed: true,
        will_run: true,
        preview_token: "test".to_string(),
        summary: "test".to_string(),
        risks: Vec::new(),
        source: None,
        skills: skills.into_iter().map(str::to_string).collect(),
    }
}

fn semantic_test_catalog_record(
    id: &str,
    agent: AgentId,
    name: &str,
    path: PathBuf,
) -> SkillRecord {
    SkillRecord {
        id: id.to_string(),
        agent: agent.as_str().to_string(),
        scope: Scope::AgentGlobal.as_str().to_string(),
        path: path.clone(),
        display_path: path,
        definition_id: id.to_string(),
        name: name.to_string(),
        state: "loaded".to_string(),
        enabled: true,
        publisher: None,
        package_name: None,
        package_version: None,
        source_kind: None,
        read_only_reason: None,
    }
}

#[test]
fn local_create_preview_refuses_an_existing_staging_destination_without_modifying_it() {
    let temp_root = std::env::temp_dir().join(format!(
        "skill-manager-local-create-existing-destination-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let app_data = temp_root.join("app-data");
    let destination =
        local_create_staging_destination_path(&app_data, "existing").expect("destination");
    fs::create_dir_all(&destination).expect("create existing destination");
    fs::write(destination.join("sentinel"), b"unchanged").expect("destination sentinel");
    let ctx = AdapterContext {
        user_home: temp_root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };

    let result = build_local_create_preview(
        &app_data,
        &ctx,
        &SkillManagerLocalCreateParams {
            name: "existing".to_string(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    );

    assert!(matches!(result, Err(CommandError::StaleActionReference)));
    assert_eq!(
        fs::read(destination.join("sentinel")).expect("destination remains"),
        b"unchanged"
    );
    fs::remove_dir_all(temp_root).ok();
}

#[test]
#[cfg(unix)]
fn local_create_preview_rejects_an_app_owned_intermediate_symlink_without_reading_its_target() {
    use std::os::unix::fs::symlink;

    crate::initialize_action_preview_secret_for_test([0xA5; 32])
        .expect("initialize action preview test secret");
    let temp_root = std::env::temp_dir().join(format!(
        "skill-manager-local-create-linked-parent-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let app_data = temp_root.join("app-data");
    let victim = temp_root.join("victim");
    fs::create_dir_all(&app_data).expect("app-data");
    fs::create_dir_all(&victim).expect("victim");
    fs::write(victim.join("sentinel"), b"unchanged").expect("victim sentinel");
    symlink(&victim, app_data.join("local-skill-library")).expect("linked parent");
    let ctx = AdapterContext {
        user_home: temp_root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };

    let result = build_local_create_preview(
        &app_data,
        &ctx,
        &SkillManagerLocalCreateParams {
            name: "linked".to_string(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    );

    assert!(matches!(result, Err(CommandError::UnsafeConfigPath(_))));
    assert_eq!(
        fs::read(victim.join("sentinel")).expect("victim remains"),
        b"unchanged"
    );
    assert!(!victim.join("sources/linked").exists());
    fs::remove_dir_all(temp_root).ok();
}

#[test]
fn local_create_preview_keeps_a_missing_app_data_owner_absent() {
    crate::initialize_action_preview_secret_for_test([0xA5; 32])
        .expect("initialize action preview test secret");
    let temp_root = std::env::temp_dir().join(format!(
        "skill-manager-local-create-zero-write-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let app_data = temp_root.join("app-data");
    fs::create_dir_all(&temp_root).expect("existing owner parent");
    let ctx = AdapterContext {
        user_home: temp_root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };

    let preview = build_local_create_preview(
        &app_data,
        &ctx,
        &SkillManagerLocalCreateParams {
            name: "new-skill".to_string(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect("zero-write preview");

    assert_eq!(preview.preconditions.len(), 2);
    assert!(
        !app_data.exists(),
        "a local-create preview must not bootstrap app data"
    );
    fs::remove_dir_all(temp_root).ok();
}

#[test]
fn machine_stdout_capture_is_private_and_removed_on_drop() {
    #[cfg(unix)]
    use std::os::unix::fs::PermissionsExt;

    let path = {
        let capture = MachineStdoutCapture::create().expect("private machine capture");
        let path = capture.path.clone();
        #[cfg(unix)]
        assert_eq!(
            fs::metadata(&path)
                .expect("capture metadata")
                .permissions()
                .mode()
                & 0o777,
            0o600
        );
        assert!(path.is_file());
        path
    };
    assert!(!path.exists(), "capture should be removed by RAII");
}

#[test]
fn resolve_binary_prefers_explicit_override_without_validation() {
    let override_path = PathBuf::from("/custom/node/bin/npx");
    let resolved = resolve_binary_from_sources(
        Some(override_path.as_os_str().to_os_string()),
        NPX_BINARY,
        None,
        &[],
    );

    assert_eq!(resolved, Some(override_path));
}

#[test]
fn resolve_binary_falls_back_to_common_gui_launch_paths() {
    let temp = std::env::temp_dir().join(format!("skill-manager-npx-path-{}", std::process::id()));
    let empty_path_dir = temp.join("empty-path");
    let fallback_dir = temp.join("homebrew-bin");
    fs::create_dir_all(&empty_path_dir).expect("empty path dir");
    fs::create_dir_all(&fallback_dir).expect("fallback dir");
    let npx = fallback_dir.join(NPX_BINARY);
    fs::write(&npx, "#!/bin/sh\n").expect("fake npx");

    let resolved = resolve_binary_from_sources(
        None,
        NPX_BINARY,
        Some(empty_path_dir.as_os_str()),
        &[fallback_dir],
    );

    assert_eq!(resolved, Some(npx));
    fs::remove_dir_all(temp).ok();
}

#[test]
fn manager_command_path_keeps_node_visible_for_env_shebangs() {
    let executable = PathBuf::from("/custom/node/bin/npx");
    let fallback_dir = PathBuf::from("/opt/homebrew/bin");
    let path = manager_command_path_from_sources(
        Some(&executable),
        Some(std::ffi::OsStr::new("/usr/bin:/custom/node/bin")),
        &[fallback_dir.clone(), PathBuf::from("/usr/bin")],
    );
    let dirs = env::split_paths(std::ffi::OsStr::new(&path)).collect::<Vec<_>>();

    assert_eq!(dirs.first(), Some(&PathBuf::from("/custom/node/bin")));
    assert!(dirs.contains(&fallback_dir));
    assert_eq!(
        dirs.iter()
            .filter(|dir| dir.as_path() == Path::new("/custom/node/bin"))
            .count(),
        1
    );
    assert_eq!(
        dirs.iter()
            .filter(|dir| dir.as_path() == Path::new("/usr/bin"))
            .count(),
        1
    );
}

#[test]
fn preview_and_runtime_env_share_the_same_allowlisted_keys() {
    let temp =
        std::env::temp_dir().join(format!("skill-manager-command-env-{}", std::process::id()));
    let ctx = AdapterContext {
        user_home: temp.join("home"),
        project_cwd: Some(temp.join("project")),
        project_root: Some(temp.join("project")),
        extra_roots: Vec::new(),
    };
    let runtime_env = manager_command_env(&ctx, "/custom/node/bin/npx");

    let preview_env = manager_command_env(&ctx, "/custom/node/bin/npx");
    assert!(
        runtime_env
            .iter()
            .any(|env_var| env_var.key == "PATH" && env_var.value.contains("/custom/node/bin")),
        "Runtime command env should make node visible to /usr/bin/env shebangs."
    );
    assert_eq!(
        preview_env
            .iter()
            .map(|env_var| env_var.key.as_str())
            .collect::<BTreeSet<_>>(),
        runtime_env
            .iter()
            .map(|env_var| env_var.key.as_str())
            .collect::<BTreeSet<_>>()
    );
}

#[test]
fn default_agents_cover_supported_app_agents() {
    assert_eq!(
        normalize_manager_agents(&[]).expect("default agents"),
        vec![
            "claude-code",
            "pi",
            "opencode",
            "codex",
            "hermes-agent",
            "openclaw"
        ]
    );
}

#[test]
fn install_preview_uses_symlink_by_default_and_copy_only_when_requested() {
    let temp = std::env::temp_dir().join(format!("skill-manager-preview-{}", std::process::id()));
    let ctx = AdapterContext {
        user_home: temp.join("home"),
        project_cwd: Some(temp.join("project")),
        project_root: Some(temp.join("project")),
        extra_roots: Vec::new(),
    };
    let params = SkillManagerInstallParams {
        source: "vercel-labs/agent-skills".to_string(),
        skills: vec!["frontend-design".to_string()],
        agents: SUPPORTED_MANAGER_AGENTS
            .iter()
            .map(|agent| (*agent).to_string())
            .collect(),
        scope: Some("project".to_string()),
        distribution: None,
        network_allowed: true,
        confirmed: false,
        preview_token: None,
        action_reference: None,
    };
    let preview = build_install_preview(&ctx, &params).expect("preview");
    assert!(preview.command.contains(&"--skill".to_string()));
    assert!(
        preview.command.contains(&"--full-depth".to_string()),
        "installing a named skill from search results should search nested package directories"
    );
    assert!(!preview.command.contains(&"--copy".to_string()));
    assert_eq!(
        preview
            .command
            .iter()
            .filter(|arg| arg.as_str() == "--agent")
            .count(),
        SUPPORTED_MANAGER_AGENTS.len()
    );

    let copy_preview = build_install_preview(
        &ctx,
        &SkillManagerInstallParams {
            distribution: Some("copy".to_string()),
            ..params
        },
    )
    .expect("copy preview");
    assert!(copy_preview.command.contains(&"--copy".to_string()));
}

#[test]
fn install_preview_resolves_relative_local_sources_from_the_manager_cwd() {
    crate::initialize_action_preview_secret_for_test([0xA5; 32])
        .expect("initialize action preview test secret");
    let temp =
        std::env::temp_dir().join(format!("skill-manager-local-source-{}", std::process::id()));
    let project = temp.join("project");
    let local_source = project.join("local-source");
    fs::create_dir_all(&local_source).expect("create local source");
    fs::write(local_source.join("SKILL.md"), "# Local").expect("write local source");
    let ctx = AdapterContext {
        user_home: temp.join("home"),
        project_cwd: Some(project.clone()),
        project_root: Some(project.clone()),
        extra_roots: Vec::new(),
    };
    let params = SkillManagerInstallParams {
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

    let preview = build_install_preview(&ctx, &params).expect("local preview");
    let canonical_source = local_source.canonicalize().expect("canonical local source");

    assert!(!preview.network_required);
    assert!(preview.preconditions.iter().any(|precondition| {
        precondition.kind == ActionPreconditionKind::SourceFile
            && Path::new(&precondition.target_id) == canonical_source
    }));
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn install_preview_rejects_credential_bearing_source_urls_without_echoing_them() {
    crate::initialize_action_preview_secret_for_test([0xA5; 32])
        .expect("initialize action preview test secret");
    let temp = std::env::temp_dir().join(format!(
        "skill-manager-sensitive-source-{}",
        std::process::id()
    ));
    let project = temp.join("project");
    fs::create_dir_all(&project).expect("create project");
    let ctx = AdapterContext {
        user_home: temp.join("home"),
        project_cwd: Some(project.clone()),
        project_root: Some(project),
        extra_roots: Vec::new(),
    };
    let sensitive = "https://user:secret@example.com/repo.git?token=abc#private";
    let params = SkillManagerInstallParams {
        source: sensitive.to_string(),
        skills: Vec::new(),
        agents: vec!["codex".to_string()],
        scope: Some("project".to_string()),
        distribution: None,
        network_allowed: true,
        confirmed: false,
        preview_token: None,
        action_reference: None,
    };

    let error = match build_install_preview(&ctx, &params) {
        Err(error) => error,
        Ok(_) => panic!("sensitive URL must be rejected before preview construction"),
    };
    let message = error.to_string();

    assert!(matches!(error, CommandError::InvalidSkillManagerRequest(_)));
    assert!(!message.contains("user:secret"));
    assert!(!message.contains("secret"));
    assert!(!message.contains("token=abc"));
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn failed_command_error_uses_stdout_when_stderr_is_empty() {
    let stderr = "";
    let stdout = "\u{1b}[31mNo matching skills found for: alibabacloud-find-skills\u{1b}[0m";

    let detail = failed_command_detail(stdout, stderr);

    assert_eq!(
        detail,
        "No matching skills found for: alibabacloud-find-skills"
    );
}

#[test]
fn command_output_redacts_credential_bearing_urls() {
    let ctx = AdapterContext {
        user_home: PathBuf::from("/tmp/example-home"),
        project_cwd: None,
        project_root: None,
        extra_roots: Vec::new(),
    };
    let output =
        "failed to fetch https://user:secret@example.com/repo.git?token=abc#private safely";

    let redacted = redact_command_output(&ctx, output);

    assert_eq!(redacted, "failed to fetch <redacted-source-url> safely");
    assert!(!redacted.contains("user"));
    assert!(!redacted.contains("secret"));
    assert!(!redacted.contains("token=abc"));
    assert!(!redacted.contains("private"));

    let scp_redacted = redact_command_output(&ctx, "failed user:secret@example.com:owner/repo.git");
    assert_eq!(
        scp_redacted, "failed <redacted-source-url>",
        "credential-shaped SCP output must be removed as one token"
    );
    for output in [
        "failed host:owner/repo.git?token=secret",
        "failed host:owner/repo.git#secret",
        "failed git@example.com:owner/repo.git?token=secret",
    ] {
        let redacted = redact_command_output(&ctx, output);
        assert_eq!(redacted, "failed <redacted-source-url>");
        assert!(!redacted.contains("secret"));
    }
}

#[test]
fn codex_manager_global_target_uses_shared_agents_root() {
    let ctx = AdapterContext {
        user_home: PathBuf::from("/tmp/agent-copilot-manager-home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };

    assert_eq!(
        manager_agent_skill_root(&ctx, &ctx.user_home, AgentId::Codex, true,),
        ctx.user_home.join(".agents/skills")
    );
}

#[cfg(unix)]
#[test]
fn manager_target_revision_tracks_symlink_target_content() {
    use std::os::unix::fs::symlink;

    let temp_root = std::env::temp_dir().join(format!(
        "agent-copilot-manager-symlink-revision-{}",
        std::process::id()
    ));
    let source = temp_root.join("source/linked-skill");
    let target_root = temp_root.join("target");
    fs::create_dir_all(&source).expect("create source");
    fs::create_dir_all(&target_root).expect("create target");
    fs::write(
        source.join("SKILL.md"),
        "---\nname: linked-skill\ndescription: before\n---\nbefore\n",
    )
    .expect("write source");
    symlink(&source, target_root.join("linked-skill")).expect("link skill");

    let before = manager_target_revision(&target_root).expect("revision before");
    fs::write(
        source.join("SKILL.md"),
        "---\nname: linked-skill\ndescription: after\n---\nafter\n",
    )
    .expect("change source");
    let after = manager_target_revision(&target_root).expect("revision after");

    assert_ne!(before, after);
    let _ = fs::remove_dir_all(&temp_root);
}

#[cfg(unix)]
#[test]
fn manager_inventory_revision_tracks_symlinked_lock_target_content() {
    use std::os::unix::fs::symlink;

    let temp_root = std::env::temp_dir().join(format!(
        "agent-copilot-manager-lock-symlink-revision-{}",
        std::process::id()
    ));
    fs::create_dir_all(&temp_root).expect("create fixture root");
    let lock_target = temp_root.join("real-skill-lock.json");
    let lock_link = temp_root.join(".skill-lock.json");
    fs::write(&lock_target, "{\"source\":\"before\"}").expect("write lock target");
    symlink(&lock_target, &lock_link).expect("link manager lock");

    let before =
        manager_inventory_revision(std::slice::from_ref(&lock_link)).expect("revision before");
    fs::write(&lock_target, "{\"source\":\"after!\"}").expect("change lock target");
    let after =
        manager_inventory_revision(std::slice::from_ref(&lock_link)).expect("revision after");

    assert_ne!(before, after);
    let _ = fs::remove_dir_all(&temp_root);
}

#[test]
fn manager_source_rejects_credential_shaped_scp_and_encoded_urls() {
    let cwd = Path::new("/tmp");
    for source in [
        "user:secret@example.com:owner/repo.git",
        "host:owner/repo.git?token=secret",
        "host:owner/repo.git#token",
        "git@example.com:owner/repo.git?token=secret",
        "https://example.com/%40token/repo.git",
        "https://example.com/owner/repo.git?token=secret",
        "https://example.com/owner/repo.git#token",
    ] {
        let error = resolve_manager_source(source, cwd)
            .expect_err("credential-shaped source must fail closed");
        assert!(matches!(error, CommandError::InvalidSkillManagerRequest(_)));
        assert!(!error.to_string().contains("secret"));
        assert!(!error.to_string().contains("token"));
    }
    assert_eq!(
        resolve_manager_source("git@example.com:owner/repo.git", cwd)
            .expect("standard scp git source"),
        ManagerSourceResolution::Network
    );
}

#[test]
fn multi_agent_install_allows_an_unchanged_valid_target_when_another_target_is_added() {
    let temp_root = std::env::temp_dir().join(format!(
        "agent-copilot-manager-mixed-install-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let existing_file = temp_root.join("claude/shared/SKILL.md");
    let added_file = temp_root.join("codex/shared/SKILL.md");
    fs::create_dir_all(existing_file.parent().expect("existing parent"))
        .expect("create existing target");
    fs::create_dir_all(added_file.parent().expect("added parent")).expect("create added target");
    fs::write(&existing_file, "existing").expect("write existing target");
    fs::write(&added_file, "added").expect("write added target");
    let existing_file = existing_file.canonicalize().expect("canonical existing");
    let added_file = added_file.canonicalize().expect("canonical added");
    let before = vec![
        ManagerSelectedSkillState {
            agent: AgentId::ClaudeCode,
            root: temp_root.join("claude"),
            skill: "shared".to_string(),
            exists: true,
            canonical_skill_file: Some(existing_file.clone()),
            source_identity: Some("claude-source".to_string()),
            content_fingerprint: Some("same-content".to_string()),
        },
        ManagerSelectedSkillState {
            agent: AgentId::Codex,
            root: temp_root.join("codex"),
            skill: "shared".to_string(),
            exists: false,
            canonical_skill_file: None,
            source_identity: None,
            content_fingerprint: None,
        },
    ];
    let after = vec![
        before[0].clone(),
        ManagerSelectedSkillState {
            agent: AgentId::Codex,
            root: temp_root.join("codex"),
            skill: "shared".to_string(),
            exists: true,
            canonical_skill_file: Some(added_file.clone()),
            source_identity: Some("codex-source".to_string()),
            content_fingerprint: Some("same-content".to_string()),
        },
    ];
    let records = vec![
        semantic_test_catalog_record(
            "claude-shared",
            AgentId::ClaudeCode,
            "shared",
            existing_file,
        ),
        semantic_test_catalog_record("codex-shared", AgentId::Codex, "shared", added_file),
    ];
    fs::create_dir_all(temp_root.join(".agents")).expect("create manager state root");
    fs::write(
        temp_root.join(".agents/.skill-lock.json"),
        r#"{"version":3,"skills":{"shared":{"source":"owner/repository","sourceType":"github"}}}"#,
    )
    .expect("write manager lock");
    let mut preview = semantic_test_preview("install", vec!["shared"]);
    preview.cwd = temp_root.to_string_lossy().to_string();
    preview.source = Some("https://github.com/owner/repository.git".to_string());

    verify_manager_operation(&preview, &records, &before, &after)
        .expect("all selected postconditions are valid and another target changed");
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn install_rejects_postconditions_owned_by_a_different_manager_source() {
    let temp_root = std::env::temp_dir().join(format!(
        "agent-copilot-manager-wrong-install-source-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let skill_file = temp_root.join("codex/shared/SKILL.md");
    fs::create_dir_all(skill_file.parent().expect("skill parent")).expect("create selected target");
    fs::write(&skill_file, "installed").expect("write selected target");
    let skill_file = skill_file.canonicalize().expect("canonical skill");
    fs::create_dir_all(temp_root.join(".agents")).expect("create manager state root");
    fs::write(
        temp_root.join(".agents/.skill-lock.json"),
        r#"{"version":3,"skills":{"shared":{"source":"other/repository","sourceType":"github"}}}"#,
    )
    .expect("write mismatched manager lock");
    let before = vec![ManagerSelectedSkillState {
        agent: AgentId::Codex,
        root: temp_root.join("codex"),
        skill: "shared".to_string(),
        exists: false,
        canonical_skill_file: None,
        source_identity: None,
        content_fingerprint: None,
    }];
    let after = vec![ManagerSelectedSkillState {
        agent: AgentId::Codex,
        root: temp_root.join("codex"),
        skill: "shared".to_string(),
        exists: true,
        canonical_skill_file: Some(skill_file.clone()),
        source_identity: Some("different-source".to_string()),
        content_fingerprint: Some("installed".to_string()),
    }];
    let records = vec![semantic_test_catalog_record(
        "codex-shared",
        AgentId::Codex,
        "shared",
        skill_file,
    )];
    let mut preview = semantic_test_preview("install", vec!["shared"]);
    preview.cwd = temp_root.to_string_lossy().to_string();
    preview.source = Some("owner/repository".to_string());

    assert!(matches!(
        verify_manager_operation(&preview, &records, &before, &after),
        Err(CommandError::VerificationFailed)
    ));
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn update_rejects_a_changed_skill_that_swaps_source_identity() {
    let temp_root = std::env::temp_dir().join(format!(
        "agent-copilot-manager-source-swap-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let skill_file = temp_root.join("codex/selected/SKILL.md");
    fs::create_dir_all(skill_file.parent().expect("skill parent")).expect("create selected target");
    fs::write(&skill_file, "after").expect("write selected target");
    let skill_file = skill_file.canonicalize().expect("canonical skill");
    let before = vec![ManagerSelectedSkillState {
        agent: AgentId::Codex,
        root: temp_root.join("codex"),
        skill: "selected".to_string(),
        exists: true,
        canonical_skill_file: Some(skill_file.clone()),
        source_identity: Some("expected-source".to_string()),
        content_fingerprint: Some("before".to_string()),
    }];
    let after = vec![ManagerSelectedSkillState {
        source_identity: Some("different-source".to_string()),
        content_fingerprint: Some("after".to_string()),
        ..before[0].clone()
    }];
    let records = vec![semantic_test_catalog_record(
        "codex-selected",
        AgentId::Codex,
        "selected",
        skill_file,
    )];

    assert!(matches!(
        verify_manager_operation(
            &semantic_test_preview("update", vec!["selected"]),
            &records,
            &before,
            &after,
        ),
        Err(CommandError::VerificationFailed)
    ));
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn unrelated_target_tree_change_cannot_satisfy_selected_skill_update() {
    let temp_root = std::env::temp_dir().join(format!(
        "agent-copilot-manager-unrelated-update-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let home = temp_root.join("home");
    let selected = home.join(".agents/skills/selected/SKILL.md");
    let unrelated = home.join(".agents/skills/unrelated/SKILL.md");
    fs::create_dir_all(selected.parent().expect("selected parent")).expect("create selected");
    fs::create_dir_all(unrelated.parent().expect("unrelated parent")).expect("create unrelated");
    fs::write(
        &selected,
        "---\nname: selected\ndescription: selected\n---\nbefore\n",
    )
    .expect("write selected");
    fs::write(
        &unrelated,
        "---\nname: unrelated\ndescription: unrelated\n---\nbefore\n",
    )
    .expect("write unrelated");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };
    let preview = SkillManagerCommandPreview {
        action: None,
        preconditions: Vec::new(),
        tool_id: DEFAULT_MANAGER_TOOL.to_string(),
        operation: "update".to_string(),
        command: vec![
            "/usr/bin/true".to_string(),
            "--global".to_string(),
            "--agent".to_string(),
            "codex".to_string(),
        ],
        cwd: home.to_string_lossy().to_string(),
        env: Vec::new(),
        requires_confirmation: true,
        confirmed: true,
        network_required: false,
        network_allowed: true,
        will_run: true,
        preview_token: "test".to_string(),
        summary: "test".to_string(),
        risks: Vec::new(),
        source: None,
        skills: vec!["selected".to_string()],
    };
    let before = manager_selected_skill_snapshot(&ctx, &preview).expect("before snapshot");
    fs::write(
        &unrelated,
        "---\nname: unrelated\ndescription: unrelated\n---\nafter\n",
    )
    .expect("change only unrelated skill");
    let after = manager_selected_skill_snapshot(&ctx, &preview).expect("after snapshot");
    let records = vec![SkillRecord {
        id: "selected".to_string(),
        agent: AgentId::Codex.as_str().to_string(),
        scope: Scope::AgentGlobal.as_str().to_string(),
        path: selected.canonicalize().expect("canonical selected"),
        display_path: selected.clone(),
        definition_id: "selected".to_string(),
        name: "selected".to_string(),
        state: "loaded".to_string(),
        enabled: true,
        publisher: None,
        package_name: None,
        package_version: None,
        source_kind: None,
        read_only_reason: None,
    }];

    assert!(matches!(
        verify_manager_operation(&preview, &records, &before, &after),
        Err(CommandError::VerificationFailed)
    ));
    let _ = fs::remove_dir_all(temp_root);
}
