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

#[test]
fn successful_manager_result_preview_omits_consumed_authorization_material() {
    let preview =
        semantic_test_preview("install", vec!["selected"]).without_authorization_material();
    let serialized = serde_json::to_value(preview).expect("serialize sanitized manager preview");
    assert!(
        serialized.get("preview_token").is_none(),
        "successful manager results may retain display-safe preview details but not the consumed token"
    );
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
        definition_id: name.to_string(),
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

fn semantic_test_catalog_details(records: &[SkillRecord]) -> Vec<SkillDetailRecord> {
    records
        .iter()
        .map(|record| {
            let content = fs::read_to_string(&record.path).expect("read semantic fixture");
            let parsed = crate::parse_tool_global_skill(&content, &record.name);
            SkillDetailRecord {
                id: record.id.clone(),
                agent: record.agent.clone(),
                scope: record.scope.clone(),
                path: record.path.clone(),
                display_path: record.display_path.clone(),
                definition_id: record.definition_id.clone(),
                name: parsed.name,
                description: parsed.description,
                state: record.state.clone(),
                enabled: record.enabled,
                frontmatter_raw: parsed.frontmatter_raw.clone(),
                body: parsed.body.clone(),
                permissions: serde_json::json!({}),
                fingerprint: crate::content_fingerprint(&parsed.frontmatter_raw, &parsed.body),
                publisher: None,
                package_name: None,
                package_version: None,
                source_kind: None,
                read_only_reason: None,
            }
        })
        .collect()
}

fn semantic_test_state(
    agent: AgentId,
    root: PathBuf,
    skill: &str,
    skill_file: Option<PathBuf>,
    content_fingerprint: Option<&str>,
) -> ManagerSelectedSkillState {
    let Some(skill_file) = skill_file else {
        return ManagerSelectedSkillState {
            agent,
            root,
            skill: skill.to_string(),
            exists: false,
            canonical_skill_file: None,
            source_identity: None,
            content_fingerprint: None,
            definition_id: None,
            semantic_name: None,
            frontmatter_raw: None,
            body: None,
            catalog_fingerprint: None,
        };
    };
    let canonical_skill_file = skill_file.canonicalize().expect("canonical test skill");
    let canonical_source = canonical_skill_file
        .parent()
        .expect("test skill source")
        .to_string_lossy()
        .to_string();
    let content = fs::read_to_string(&canonical_skill_file).expect("test skill content");
    let parsed = crate::parse_tool_global_skill(&content, skill);
    ManagerSelectedSkillState {
        agent,
        root,
        skill: skill.to_string(),
        exists: true,
        canonical_skill_file: Some(canonical_skill_file),
        source_identity: Some(
            action_source_revision(
                "manager.skill.source-identity",
                &[("canonical_source", &canonical_source)],
            )
            .expect("test source identity"),
        ),
        content_fingerprint: content_fingerprint.map(str::to_string),
        definition_id: Some(parsed.name.clone()),
        semantic_name: Some(parsed.name),
        frontmatter_raw: Some(parsed.frontmatter_raw.clone()),
        body: Some(parsed.body.clone()),
        catalog_fingerprint: Some(crate::content_fingerprint(
            &parsed.frontmatter_raw,
            &parsed.body,
        )),
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
            network_allowed: true,
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
            network_allowed: true,
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
            network_allowed: true,
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect("zero-write preview");

    assert_eq!(preview.preconditions.len(), 3);
    assert!(preview.preconditions.iter().any(|precondition| {
        precondition.kind == ActionPreconditionKind::SourceFile
            && Path::new(&precondition.target_id) == Path::new(&preview.command[0])
    }));
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
#[cfg(unix)]
fn machine_stdout_capture_drop_preserves_a_replacement_created_after_quarantine_validation() {
    let capture = MachineStdoutCapture::create().expect("private machine capture");
    let path = capture.path.clone();
    install_machine_capture_pre_unlink_test_hook(path.clone(), |path| {
        fs::write(path, b"replacement").expect("create replacement after quarantine validation");
    });

    drop(capture);

    assert_eq!(
        fs::read(&path).expect("replacement entry remains"),
        b"replacement",
        "cleanup must unlink only the quarantined capture inode"
    );
    fs::remove_file(path).expect("remove replacement");
}

#[test]
#[cfg(unix)]
fn machine_stdout_capture_read_preserves_a_post_validation_replacement() {
    let mut capture = MachineStdoutCapture::create().expect("private machine capture");
    let path = capture.path.clone();
    install_machine_capture_pre_unlink_test_hook(path.clone(), |path| {
        fs::write(path, b"replacement").expect("create replacement after quarantine validation");
    });

    assert!(capture
        .read()
        .expect("read and finalize capture")
        .is_empty());
    assert_eq!(
        fs::read(&path).expect("replacement entry remains"),
        b"replacement"
    );
    drop(capture);
    assert_eq!(
        fs::read(&path).expect("drop leaves replacement"),
        b"replacement"
    );
    fs::remove_file(path).expect("remove replacement");
}

#[test]
#[cfg(unix)]
fn machine_stdout_capture_sync_failure_is_partial_and_retains_quarantine() {
    use std::os::unix::fs::MetadataExt;

    let mut capture = MachineStdoutCapture::create().expect("private machine capture");
    let path = capture.path.clone();
    let expected = capture.file.metadata().expect("capture metadata");
    inject_machine_capture_sync_failure_for_test("quarantine");

    let error = capture.read().expect_err("sync failure must be reported");

    assert!(matches!(
        error,
        CommandError::PartialEffect {
            operation,
            state,
            cleanup_required: true,
            ..
        } if operation == "skillManager.machineCaptureCleanup" && state == "outcome_unknown"
    ));
    assert!(
        !path.exists(),
        "the original name was atomically quarantined"
    );
    let quarantine = fs::read_dir(std::env::temp_dir())
        .expect("temporary directory")
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".agent-copilot-skill-manager-quarantine-")
                && entry.metadata().is_ok_and(|metadata| {
                    metadata.dev() == expected.dev() && metadata.ino() == expected.ino()
                })
        })
        .expect("failed sync retains the matching quarantine")
        .path();
    fs::remove_file(quarantine).expect("remove retained test quarantine");
    drop(capture);
}

#[test]
#[cfg(unix)]
fn explicit_early_return_finalize_surfaces_machine_capture_cleanup_failure() {
    use std::os::unix::fs::MetadataExt;

    let capture = MachineStdoutCapture::create().expect("private machine capture");
    let expected = capture.file.metadata().expect("capture metadata");
    let mut capture = Some(capture);
    inject_machine_capture_sync_failure_for_test("quarantine");
    let error = finalize_machine_capture_after_error(
        &AdapterContext {
            user_home: PathBuf::from("/tmp"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        },
        &semantic_test_preview("search", Vec::new()),
        &mut capture,
        "not_started",
        CommandError::SkillManagerCommandFailed("spawn failed".to_string()),
    );
    assert!(matches!(
        error,
        CommandError::PartialEffect { ref detail, cleanup_required: true, .. }
            if detail.contains("private output cleanup also failed")
    ));
    let quarantine = fs::read_dir(std::env::temp_dir())
        .expect("temp dir")
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".agent-copilot-skill-manager-quarantine-")
                && entry.metadata().is_ok_and(|metadata| {
                    metadata.dev() == expected.dev() && metadata.ino() == expected.ino()
                })
        })
        .expect("retained quarantine")
        .path();
    fs::remove_file(quarantine).expect("remove retained quarantine");
}

#[test]
#[cfg(unix)]
fn machine_stdout_capture_hardlink_fails_closed_without_claiming_cleanup() {
    use std::os::unix::fs::MetadataExt;

    let mut capture = MachineStdoutCapture::create().expect("private machine capture");
    let path = capture.path.clone();
    let hardlink = path.with_extension("hardlink");
    fs::hard_link(&path, &hardlink).expect("add unexpected hardlink");

    let error = capture
        .read()
        .expect_err("single-link invariant must be enforced");

    assert!(matches!(
        error,
        CommandError::PartialEffect {
            operation,
            state,
            cleanup_required: true,
            ..
        } if operation == "skillManager.machineCaptureCleanup" && state == "applied_unverified"
    ));
    let original = fs::metadata(&path).expect("original retained");
    let linked = fs::metadata(&hardlink).expect("hardlink retained");
    assert_eq!(original.ino(), linked.ino());
    assert_eq!(original.nlink(), 2);
    fs::remove_file(path).expect("remove original test capture");
    fs::remove_file(hardlink).expect("remove test hardlink");
    drop(capture);
}

#[test]
#[cfg(unix)]
fn machine_stdout_capture_chmods_only_its_descriptor_and_preserves_a_replacement_entry() {
    use std::{
        os::unix::fs::{symlink, PermissionsExt},
        sync::{Arc, Mutex},
    };

    let root = std::env::temp_dir().join(format!(
        "skill-manager-machine-capture-race-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let victim = root.join("victim");
    let moved_capture = root.join("moved-capture");
    fs::create_dir_all(&victim).expect("create victim");
    fs::write(victim.join("sentinel"), b"unchanged").expect("victim sentinel");
    fs::set_permissions(&victim, fs::Permissions::from_mode(0o755)).expect("victim mode");
    let raced_path = Arc::new(Mutex::new(None::<PathBuf>));
    let hook_raced_path = Arc::clone(&raced_path);
    let hook_victim = victim.clone();
    let hook_moved_capture = moved_capture.clone();
    install_machine_capture_post_create_test_hook(move |path| {
        fs::rename(path, &hook_moved_capture).expect("move created capture");
        symlink(&hook_victim, path).expect("replace capture path");
        *hook_raced_path.lock().expect("record raced path") = Some(path.to_path_buf());
    });

    let error = match MachineStdoutCapture::create() {
        Ok(_) => panic!("capture path replacement must fail"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        CommandError::PartialEffect {
            operation,
            state,
            cleanup_required: false,
            ..
        } if operation == "skillManager.machineCaptureCleanup" && state == "applied_unverified"
    ));
    assert_eq!(
        fs::metadata(&victim)
            .expect("victim metadata")
            .permissions()
            .mode()
            & 0o777,
        0o755,
        "descriptor chmod must not follow the replacement symlink"
    );
    assert_eq!(
        fs::read(victim.join("sentinel")).expect("victim sentinel"),
        b"unchanged"
    );
    assert_eq!(
        fs::metadata(&moved_capture)
            .expect("moved capture metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let raced_path = raced_path
        .lock()
        .expect("read raced path")
        .take()
        .expect("raced path");
    assert!(
        fs::symlink_metadata(&raced_path)
            .expect("replacement entry remains")
            .file_type()
            .is_symlink(),
        "inode-bound cleanup must not unlink a replacement entry"
    );
    fs::remove_file(raced_path).expect("remove replacement link");
    fs::remove_dir_all(root).ok();
}

#[test]
#[cfg(unix)]
fn machine_stdout_capture_restore_conflict_preserves_original_replacement_and_quarantine() {
    use std::{
        os::unix::fs::{symlink, MetadataExt},
        sync::{Arc, Mutex},
    };

    let root = std::env::temp_dir().join(format!(
        "skill-manager-machine-capture-restore-conflict-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let victim = root.join("victim");
    let moved_capture = root.join("moved-capture");
    fs::create_dir_all(&victim).expect("create victim");
    fs::write(victim.join("sentinel"), b"unchanged").expect("victim sentinel");
    let raced_path = Arc::new(Mutex::new(None::<PathBuf>));
    let hook_raced_path = Arc::clone(&raced_path);
    let hook_victim = victim.clone();
    let hook_moved_capture = moved_capture.clone();
    install_machine_capture_post_create_test_hook(move |path| {
        fs::rename(path, &hook_moved_capture).expect("move created capture");
        symlink(&hook_victim, path).expect("install mismatched entry");
        *hook_raced_path.lock().expect("record raced path") = Some(path.to_path_buf());
    });
    let restore_raced_path = Arc::clone(&raced_path);
    install_machine_capture_pre_restore_test_hook(move || {
        let path = restore_raced_path
            .lock()
            .expect("read raced path")
            .clone()
            .expect("raced path recorded");
        fs::write(path, b"blocking replacement").expect("create restore conflict");
    });

    let error = match MachineStdoutCapture::create() {
        Ok(_) => panic!("restore conflict must fail closed"),
        Err(error) => error,
    };

    assert!(matches!(
        error,
        CommandError::PartialEffect {
            operation,
            state,
            cleanup_required: true,
            ..
        } if operation == "skillManager.machineCaptureCleanup" && state == "outcome_unknown"
    ));
    let raced_path = raced_path
        .lock()
        .expect("read raced path")
        .clone()
        .expect("raced path");
    assert_eq!(
        fs::read(&raced_path).expect("blocking replacement remains"),
        b"blocking replacement"
    );
    assert_eq!(
        fs::read(victim.join("sentinel")).expect("victim remains"),
        b"unchanged"
    );
    let quarantine = fs::read_dir(std::env::temp_dir())
        .expect("temporary directory")
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".agent-copilot-skill-manager-quarantine-")
                && fs::read_link(entry.path()).is_ok_and(|target| target == victim)
        })
        .expect("restore conflict retains quarantine")
        .path();
    assert_eq!(
        fs::read_link(&quarantine).expect("quarantined link"),
        victim
    );
    let moved = fs::metadata(&moved_capture).expect("moved capture");
    assert!(moved.is_file());
    assert_eq!(moved.nlink(), 1);

    fs::remove_file(quarantine).expect("remove retained quarantine");
    fs::remove_file(raced_path).expect("remove blocker");
    fs::remove_dir_all(root).ok();
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
    for (key, expected) in [
        ("npm_config_userconfig", "/dev/null"),
        ("npm_config_globalconfig", "/.agent-copilot-no-global-npmrc"),
        ("npm_config_ignore_scripts", "true"),
        ("npm_config_registry", "https://registry.npmjs.org/"),
        ("GIT_CONFIG_NOSYSTEM", "1"),
        ("GIT_CONFIG_GLOBAL", "/dev/null"),
        ("GIT_TERMINAL_PROMPT", "0"),
        ("GCM_INTERACTIVE", "never"),
    ] {
        assert!(runtime_env
            .iter()
            .any(|env_var| env_var.key == key && env_var.value == expected));
    }
    assert!(runtime_env.iter().any(|env_var| {
        env_var.key == "npm_config_cache"
            && env_var
                .value
                .contains("dev.agent-copilot.native/external-manager/npm-cache")
    }));
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
    fs::create_dir_all(temp.join("project")).expect("create manager cwd");
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
    assert_eq!(preview.command[1], "--yes");
    assert_eq!(preview.command[2], "skills@1.5.20");
    assert!(preview.risks.iter().any(|risk| {
        risk.contains("unsandboxed with the user's filesystem authority")
            && risk.contains("may read HOME files")
            && risk.contains("explicit external-code trust boundary")
    }));
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
    fs::remove_dir_all(temp).ok();
}

#[test]
#[cfg(unix)]
fn every_manager_spawn_action_binds_and_rejects_a_stale_executable() {
    use std::os::unix::fs::PermissionsExt;

    crate::initialize_action_preview_secret_for_test([0xA5; 32])
        .expect("initialize action preview test secret");
    let root = std::env::temp_dir().join(format!(
        "manager-executable-binding-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let project = root.join("project");
    let app_data = root.join("app-data");
    let executable = root.join("npx");
    fs::create_dir_all(&project).expect("project");
    fs::write(&executable, "#!/bin/sh\nexit 0\n").expect("manager executable");
    fs::set_permissions(&executable, fs::Permissions::from_mode(0o700)).expect("executable mode");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: Some(project.clone()),
        project_cwd: Some(project.clone()),
        extra_roots: Vec::new(),
    };
    let cases = [
        (
            "install",
            project.clone(),
            vec![
                executable.to_string_lossy().to_string(),
                "--yes".to_string(),
                SKILLS_CLI_BINARY.to_string(),
                "add".to_string(),
                "owner/repository".to_string(),
                "--skill".to_string(),
                "selected".to_string(),
                "--agent".to_string(),
                "codex".to_string(),
            ],
            None,
        ),
        (
            "remove",
            project.clone(),
            vec![
                executable.to_string_lossy().to_string(),
                "--yes".to_string(),
                SKILLS_CLI_BINARY.to_string(),
                "remove".to_string(),
                "selected".to_string(),
                "--agent".to_string(),
                "codex".to_string(),
            ],
            None,
        ),
        (
            "update",
            project.clone(),
            vec![
                executable.to_string_lossy().to_string(),
                "--yes".to_string(),
                SKILLS_CLI_BINARY.to_string(),
                "update".to_string(),
                "selected".to_string(),
                "--agent".to_string(),
                "codex".to_string(),
            ],
            None,
        ),
        (
            "localCreate",
            local_create_root(&app_data),
            vec![
                executable.to_string_lossy().to_string(),
                "--yes".to_string(),
                SKILLS_CLI_BINARY.to_string(),
                "init".to_string(),
                "selected".to_string(),
            ],
            Some(app_data.as_path()),
        ),
    ];
    for (index, (operation, cwd, command, owner)) in cases.into_iter().enumerate() {
        fs::write(&executable, format!("#!/bin/sh\n# accepted {index}\n"))
            .expect("accepted executable");
        let binding = manager_action_binding(
            &ctx,
            &command,
            &cwd,
            operation,
            ManagerActionBindingOptions {
                network_required: true,
                network_allowed: true,
                accepted_revision: None,
                app_data_dir: owner,
            },
        )
        .expect("action binding")
        .expect("typed manager action");
        assert!(
            binding
                .action
                .impacts
                .contains(&ActionImpact::ExternalManager),
            "{operation} starts the external manager and must declare that impact"
        );
        assert!(binding.preconditions.iter().any(|precondition| {
            precondition.kind == ActionPreconditionKind::SourceFile
                && Path::new(&precondition.target_id) == executable
        }));
        let preview = SkillManagerCommandPreview {
            action: Some(binding.action),
            preconditions: binding.preconditions,
            tool_id: DEFAULT_MANAGER_TOOL.to_string(),
            operation: operation.to_string(),
            command,
            cwd: cwd.to_string_lossy().to_string(),
            env: manager_command_env(&ctx, &executable.to_string_lossy()),
            requires_confirmation: true,
            confirmed: true,
            network_required: true,
            network_allowed: true,
            will_run: true,
            preview_token: binding.preview_token,
            summary: "test".to_string(),
            risks: Vec::new(),
            source: None,
            skills: vec!["selected".to_string()],
        };
        fs::write(&executable, format!("#!/bin/sh\n# stale {index}\n")).expect("stale executable");
        assert!(matches!(
            validate_manager_preconditions(&ctx, &preview),
            Err(CommandError::StaleActionReference)
        ));
    }
    fs::remove_dir_all(root).ok();
}

#[test]
fn install_preview_rejects_local_sources_for_every_scope_and_supported_agent_without_echo() {
    crate::initialize_action_preview_secret_for_test([0xA5; 32])
        .expect("initialize action preview test secret");
    let temp =
        std::env::temp_dir().join(format!("skill-manager-local-source-{}", std::process::id()));
    let home = temp.join("home");
    let project = temp.join("project");
    let local_source = project.join("local-source");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&local_source).expect("create local source");
    fs::write(local_source.join("SKILL.md"), "# Local").expect("write local source");
    fs::create_dir_all(home.join("local-source")).expect("create global relative local source");
    fs::write(home.join("local-source/SKILL.md"), "# Local")
        .expect("write global relative local source");
    let ctx = AdapterContext {
        user_home: home,
        project_cwd: Some(project.clone()),
        project_root: Some(project.clone()),
        extra_roots: Vec::new(),
    };
    let file_url = url::Url::from_file_path(&local_source)
        .expect("local file URL")
        .to_string();
    let sources = [
        "local-source".to_string(),
        local_source.to_string_lossy().to_string(),
        file_url,
    ];
    for scope in ["project", "global"] {
        for agent in SUPPORTED_MANAGER_AGENTS {
            for source in &sources {
                let error = build_install_preview(
                    &ctx,
                    &SkillManagerInstallParams {
                        source: source.clone(),
                        skills: vec!["local-source".to_string()],
                        agents: vec![agent.to_string()],
                        scope: Some(scope.to_string()),
                        distribution: None,
                        network_allowed: true,
                        confirmed: false,
                        preview_token: None,
                        action_reference: None,
                    },
                )
                .expect_err("raw manager local install must fail before preview");
                assert!(matches!(
                    error,
                    CommandError::LocalSkillManagerSourceUnsupported
                ));
                assert!(!error
                    .to_string()
                    .contains(&local_source.to_string_lossy().to_string()));
            }
        }
    }
    let _ = fs::remove_dir_all(temp);
}

#[test]
#[cfg(unix)]
fn install_preview_rejects_symlinked_local_source_without_echo() {
    use std::os::unix::fs::symlink;

    let temp = std::env::temp_dir().join(format!(
        "skill-manager-local-symlink-{}",
        std::process::id()
    ));
    let project = temp.join("project");
    let private_source = temp.join("private-source-name-must-not-leak");
    let source_link = project.join("source-link");
    fs::create_dir_all(&project).expect("create project");
    fs::create_dir_all(&private_source).expect("create local source");
    fs::write(private_source.join("SKILL.md"), "# Local").expect("write local source");
    symlink(&private_source, &source_link).expect("create source symlink");
    let ctx = AdapterContext {
        user_home: temp.join("home"),
        project_cwd: Some(project.clone()),
        project_root: Some(project),
        extra_roots: Vec::new(),
    };
    let error = build_install_preview(
        &ctx,
        &SkillManagerInstallParams {
            source: "source-link".to_string(),
            skills: vec!["local-source".to_string()],
            agents: vec!["codex".to_string()],
            scope: Some("project".to_string()),
            distribution: None,
            network_allowed: true,
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect_err("symlinked local source must be rejected");
    assert!(matches!(
        error,
        CommandError::LocalSkillManagerSourceUnsupported
    ));
    assert!(!error
        .to_string()
        .contains("private-source-name-must-not-leak"));
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn remote_shorthand_is_normalized_before_signing_and_local_shadow_fails_before_bootstrap() {
    crate::initialize_action_preview_secret_for_test([0xA5; 32])
        .expect("initialize action preview test secret");
    let temp = std::env::temp_dir().join(format!(
        "skill-manager-remote-shadow-{}",
        std::process::id()
    ));
    let home = temp.join("home");
    let project = temp.join("project");
    let app_data = temp.join("app-data");
    fs::create_dir_all(&home).expect("create home");
    fs::create_dir_all(&project).expect("create project");
    let ctx = AdapterContext {
        user_home: home,
        project_cwd: Some(project.clone()),
        project_root: Some(project.clone()),
        extra_roots: Vec::new(),
    };
    let base = SkillManagerInstallParams {
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
    let preview = build_install_preview(&ctx, &base).expect("remote preview");
    assert_eq!(
        preview
            .command
            .windows(2)
            .find(|window| window[0] == "add")
            .map(|window| window[1].as_str()),
        Some("https://github.com/vercel-labs/agent-skills.git")
    );
    assert_eq!(
        preview.source.as_deref(),
        Some("https://github.com/vercel-labs/agent-skills.git")
    );

    let local_shadow = project.join("vercel-labs/agent-skills");
    fs::create_dir_all(&local_shadow).expect("create local shadow");
    fs::write(local_shadow.join("SKILL.md"), "# Shadow").expect("write local shadow");
    let confirmed = SkillManagerInstallParams {
        confirmed: true,
        preview_token: Some(preview.preview_token.clone()),
        action_reference: preview.action.as_ref().map(ActionReference::from),
        ..base
    };
    let error = apply_install_with_manager(&app_data, &ctx, &confirmed)
        .expect_err("local shadow must fail before app-data bootstrap");
    assert!(matches!(
        error,
        CommandError::LocalSkillManagerSourceUnsupported
    ));
    assert!(!app_data.exists());
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
fn install_preview_accepts_only_explicit_vcs_urls_and_rejects_unsafe_schemes_and_paths_without_echo(
) {
    crate::initialize_action_preview_secret_for_test([0xA5; 32])
        .expect("initialize action preview test secret");
    let temp = std::env::temp_dir().join(format!(
        "skill-manager-vcs-source-matrix-{}",
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

    for accepted in [
        "https://example.com/owner/repository.git",
        "ssh://git@example.com/owner/repository.git",
        "git@example.com:owner/repository.git",
    ] {
        let preview = build_install_preview(
            &ctx,
            &SkillManagerInstallParams {
                source: accepted.to_string(),
                skills: vec!["selected".to_string()],
                agents: vec!["codex".to_string()],
                scope: Some("project".to_string()),
                distribution: None,
                network_allowed: true,
                confirmed: false,
                preview_token: None,
                action_reference: None,
            },
        )
        .expect("explicit VCS source should be accepted");
        assert_eq!(preview.source.as_deref(), Some(accepted));
    }

    let rejected = [
        ("http://http-marker.example/owner/repo.git", "http-marker"),
        ("ftp://ftp-marker.example/owner/repo.git", "ftp-marker"),
        ("git://git-marker.example/owner/repo.git", "git-marker"),
        ("data://data-marker.example/owner/repo.git", "data-marker"),
        (
            "javascript://javascript-marker.example/owner/repo.git",
            "javascript-marker",
        ),
        (
            "helper+custom://custom-marker.example/owner/repo.git",
            "custom-marker",
        ),
        ("https:///empty-host-marker/repo.git", "empty-host-marker"),
        ("https://no-path-marker.example", "no-path-marker"),
        (
            "https://example.com/owner/../url-traversal-marker.git",
            "url-traversal-marker",
        ),
        (
            "https://example.com/owner\\url-backslash-marker.git",
            "url-backslash-marker",
        ),
        (
            "ssh://user-marker@example.com/owner/repo.git",
            "user-marker",
        ),
        (
            "ssh://git@example.com/owner/../ssh-traversal-marker.git",
            "ssh-traversal-marker",
        ),
        (
            "ssh:///owner/ssh-empty-host-marker.git",
            "ssh-empty-host-marker",
        ),
        (
            "git@example.com:owner/../scp-traversal-marker.git",
            "scp-traversal-marker",
        ),
        (
            "git@example.com:owner\\scp-backslash-marker.git",
            "scp-backslash-marker",
        ),
        (
            "git@scp-empty-path-marker.example:",
            "scp-empty-path-marker",
        ),
    ];
    for (source, marker) in rejected {
        let error = build_install_preview(
            &ctx,
            &SkillManagerInstallParams {
                source: source.to_string(),
                skills: vec!["selected".to_string()],
                agents: vec!["codex".to_string()],
                scope: Some("project".to_string()),
                distribution: None,
                network_allowed: true,
                confirmed: false,
                preview_token: None,
                action_reference: None,
            },
        )
        .expect_err("unsafe remote source must fail before preview construction");
        assert!(matches!(error, CommandError::InvalidSkillManagerRequest(_)));
        assert!(!error.to_string().contains(source));
        assert!(!error.to_string().contains(marker));
    }
    for encoded_file_source in [
        "file:///tmp/file-url-marker%20secret/SKILL.md",
        "FILE:///tmp/uppercase-file-url-marker%20secret/SKILL.md",
    ] {
        let error = build_install_preview(
            &ctx,
            &SkillManagerInstallParams {
                source: encoded_file_source.to_string(),
                skills: vec!["selected".to_string()],
                agents: vec!["codex".to_string()],
                scope: Some("project".to_string()),
                distribution: None,
                network_allowed: true,
                confirmed: false,
                preview_token: None,
                action_reference: None,
            },
        )
        .expect_err("every file URL must be kept out of the external manager");
        assert!(matches!(
            error,
            CommandError::LocalSkillManagerSourceUnsupported
        ));
        assert!(!error.to_string().contains(encoded_file_source));
        assert!(!error.to_string().contains("file-url-marker"));
    }
    let _ = fs::remove_dir_all(temp);
}

#[test]
fn install_preview_rejects_empty_repository_after_git_suffix_normalization() {
    crate::initialize_action_preview_secret_for_test([0xA5; 32])
        .expect("initialize action preview test secret");
    let temp = std::env::temp_dir().join(format!(
        "skill-manager-empty-repository-{}",
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
    let error = build_install_preview(
        &ctx,
        &SkillManagerInstallParams {
            source: "owner/.git".to_string(),
            skills: vec!["skill".to_string()],
            agents: vec!["codex".to_string()],
            scope: Some("project".to_string()),
            distribution: None,
            network_allowed: true,
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect_err("empty normalized repository must be rejected");
    assert!(matches!(error, CommandError::InvalidSkillManagerRequest(_)));
    assert!(!error.to_string().contains("owner/.git"));
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
    let shared_file = temp_root.join(".agents/skills/shared/SKILL.md");
    fs::create_dir_all(shared_file.parent().expect("shared parent")).expect("create shared target");
    fs::write(
        &shared_file,
        "---\nname: shared\ndescription: shared\n---\ninstalled\n",
    )
    .expect("write shared target");
    let shared_file = shared_file.canonicalize().expect("canonical shared");
    let before = vec![
        semantic_test_state(
            AgentId::ClaudeCode,
            temp_root.join("claude"),
            "shared",
            Some(shared_file.clone()),
            Some("same-content"),
        ),
        semantic_test_state(
            AgentId::Codex,
            temp_root.join("codex"),
            "shared",
            None,
            None,
        ),
    ];
    let after = vec![
        before[0].clone(),
        semantic_test_state(
            AgentId::Codex,
            temp_root.join("codex"),
            "shared",
            Some(shared_file.clone()),
            Some("same-content"),
        ),
    ];
    let records = vec![
        semantic_test_catalog_record(
            "claude-shared",
            AgentId::ClaudeCode,
            "shared",
            shared_file.clone(),
        ),
        semantic_test_catalog_record("codex-shared", AgentId::Codex, "shared", shared_file),
    ];
    let details = semantic_test_catalog_details(&records);
    fs::create_dir_all(temp_root.join(".agents")).expect("create manager state root");
    fs::write(
        temp_root.join(".agents/.skill-lock.json"),
        r#"{"version":3,"skills":{"shared":{"source":"owner/repository","sourceType":"github","skillPath":"skills/shared/SKILL.md"}}}"#,
    )
    .expect("write manager lock");
    let mut preview = semantic_test_preview("install", vec!["shared"]);
    preview.cwd = temp_root.to_string_lossy().to_string();
    preview.source = Some("https://github.com/owner/repository.git".to_string());

    verify_manager_operation(&preview, &records, &details, &before, &after)
        .expect("all selected postconditions are valid and another target changed");
    let _ = fs::remove_dir_all(temp_root);
}

#[test]
fn install_rejects_same_path_catalog_content_from_a_scan_window_third_state() {
    let temp_root = std::env::temp_dir().join(format!(
        "agent-copilot-manager-catalog-third-state-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ));
    let skill_file = temp_root.join(".agents/skills/shared/SKILL.md");
    fs::create_dir_all(skill_file.parent().expect("skill parent")).expect("create skill");
    fs::write(
        &skill_file,
        "---\nname: shared\ndescription: final\n---\nfinal-a\n",
    )
    .expect("final skill");
    fs::write(
        temp_root.join(".agents/.skill-lock.json"),
        r#"{"version":3,"skills":{"shared":{"source":"owner/repository","sourceType":"github","skillPath":"skills/shared/SKILL.md"}}}"#,
    )
    .expect("manager lock");
    let before = vec![semantic_test_state(
        AgentId::Codex,
        temp_root.join(".agents/skills"),
        "shared",
        None,
        None,
    )];
    let after = vec![semantic_test_state(
        AgentId::Codex,
        temp_root.join(".agents/skills"),
        "shared",
        Some(skill_file.clone()),
        Some("final"),
    )];
    let records = vec![semantic_test_catalog_record(
        "codex-shared",
        AgentId::Codex,
        "shared",
        skill_file,
    )];
    let mut details = semantic_test_catalog_details(&records);
    details[0].body = "final-b\n".to_string();
    details[0].fingerprint =
        crate::content_fingerprint(&details[0].frontmatter_raw, &details[0].body);
    let mut preview = semantic_test_preview("install", vec!["shared"]);
    preview.cwd = temp_root.to_string_lossy().to_string();
    preview.source = Some("owner/repository".to_string());

    assert!(matches!(
        verify_manager_operation(&preview, &records, &details, &before, &after),
        Err(CommandError::VerificationFailed)
    ));
    fs::remove_dir_all(temp_root).ok();
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
    let before = vec![semantic_test_state(
        AgentId::Codex,
        temp_root.join("codex"),
        "shared",
        None,
        None,
    )];
    let after = vec![semantic_test_state(
        AgentId::Codex,
        temp_root.join("codex"),
        "shared",
        Some(skill_file.clone()),
        Some("installed"),
    )];
    let records = vec![semantic_test_catalog_record(
        "codex-shared",
        AgentId::Codex,
        "shared",
        skill_file,
    )];
    let details = semantic_test_catalog_details(&records);
    let mut preview = semantic_test_preview("install", vec!["shared"]);
    preview.cwd = temp_root.to_string_lossy().to_string();
    preview.source = Some("owner/repository".to_string());

    assert!(matches!(
        verify_manager_operation(&preview, &records, &details, &before, &after),
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
    let before = vec![semantic_test_state(
        AgentId::Codex,
        temp_root.join("codex"),
        "selected",
        Some(skill_file.clone()),
        Some("before"),
    )];
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
    let details = semantic_test_catalog_details(&records);

    assert!(matches!(
        verify_manager_operation(
            &semantic_test_preview("update", vec!["selected"]),
            &records,
            &details,
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
    let details = semantic_test_catalog_details(&records);

    assert!(matches!(
        verify_manager_operation(&preview, &records, &details, &before, &after),
        Err(CommandError::VerificationFailed)
    ));
    let _ = fs::remove_dir_all(temp_root);
}
