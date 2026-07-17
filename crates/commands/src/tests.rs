use std::{
    path::{Path, PathBuf},
    time::{Instant, SystemTime, UNIX_EPOCH},
};

use skills_copilot_catalog::Catalog;
use skills_copilot_core::{
    AdapterContext, AdapterRoot, AgentId, NetworkAccess, PermissionRequest, RootSource,
    SkillInstance, SkillState,
};

use super::*;

mod runtime_conflict_regressions;
mod scanner_regressions;

#[test]
fn yaml_contract_preserves_permissions_scalars_sequences_bools_and_nested_mapping() {
    let raw = "name: sample-skill\ndescription: Sample\nallowed-tools:\n  - Read\n  - Search\npermissions:\n  network: none\n  exec: false\n  requires_human: true\nmetadata:\n  openclaw:\n    skillKey: routed-key\n";
    let value: serde_norway::Value = serde_norway::from_str(raw).expect("yaml parses");
    let permissions = permissions_from_frontmatter(&value);

    assert_eq!(permissions["tools"], serde_json::json!(["Read", "Search"]));
    assert_eq!(permissions["network"], "none");
    assert_eq!(permissions["exec"], false);
    assert_eq!(permissions["requires_human"], true);
    assert_eq!(
        value
            .get("metadata")
            .and_then(|item| item.get("openclaw"))
            .and_then(|item| item.get("skillKey"))
            .and_then(serde_norway::Value::as_str),
        Some("routed-key")
    );
    assert!(serde_norway::from_str::<serde_norway::Value>("name: [unterminated\n").is_err());
}

#[test]
fn script_execution_preview_is_disabled_and_redacts_env_values() {
    let root = temp_test_dir("script-preview");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: Some(root.clone()),
        project_cwd: Some(root.join("project")),
        extra_roots: Vec::new(),
    };
    let request = ScriptExecutionRequest {
        command: vec!["python3".to_string(), "scripts/task.py".to_string()],
        cwd: Some(PathBuf::from("work")),
        env: std::collections::BTreeMap::from([(
            "API_TOKEN".to_string(),
            "fixture-redacted-value".to_string(),
        )]),
        network: Some("full".to_string()),
        files: vec!["./src/**".to_string()],
        skill_instance_id: Some("skill-fixture".to_string()),
        initiated_by: ScriptExecutionInitiator::User,
        confirmed: false,
    };

    let preview = preview_script_execution(&ctx, &request).expect("preview");

    assert!(!preview.execution_allowed);
    assert!(preview.initiator_allowed);
    assert_eq!(preview.cwd.source, "request-relative");
    assert_eq!(preview.env.provided_keys, vec!["API_TOKEN".to_string()]);
    assert_eq!(preview.env.value_policy, "values-redacted");
    assert!(!preview.network.allowed);
    assert!(!preview.files.read_allowed);
    assert!(!preview.files.write_allowed);
    assert!(preview.confirmation.required);
    let serialized = serde_json::to_string(&preview).expect("serialize preview");
    assert!(!serialized.contains("fixture-redacted-value"));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn blocked_script_execution_writes_app_data_audit_only() {
    let root = temp_test_dir("script-audit");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: Some(root.join("project")),
        project_cwd: Some(root.join("project")),
        extra_roots: Vec::new(),
    };
    let audit_root = root.join("app-data/audit");
    let audit_path = audit_root.join("script-execution.jsonl");
    let skill_dir = root.join("project/skills/demo");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(&skill_path, "name: demo\n").expect("write skill");
    let before = std::fs::read_to_string(&skill_path).expect("read skill");
    let request = ScriptExecutionRequest {
        command: vec![
            "sh".to_string(),
            "-c".to_string(),
            "touch marker".to_string(),
        ],
        cwd: None,
        env: std::collections::BTreeMap::new(),
        network: None,
        files: Vec::new(),
        skill_instance_id: Some("skill-fixture".to_string()),
        initiated_by: ScriptExecutionInitiator::Llm,
        confirmed: true,
    };

    let record = record_blocked_script_execution(&ctx, &audit_root, &audit_path, &request)
        .expect("blocked record");

    assert_eq!(record.status, "blocked");
    assert_eq!(record.outcome, "llm_initiator_not_allowed");
    assert!(!record.spawned_process);
    assert!(!root.join("project/marker").exists());
    assert_eq!(
        std::fs::read_to_string(&skill_path).expect("read skill after"),
        before,
        "audit must not write to skill files"
    );
    let audit_content = std::fs::read_to_string(&audit_path).expect("read audit");
    assert!(audit_content.contains("llm_initiator_not_allowed"));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn blocked_script_execution_rejects_audit_path_outside_root() {
    let root = temp_test_dir("script-audit-outside");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: Some(root.join("project")),
        project_cwd: Some(root.join("project")),
        extra_roots: Vec::new(),
    };
    let audit_root = root.join("app-data/audit");
    let outside_audit_path = root.join("project/script-execution.jsonl");
    let request = ScriptExecutionRequest {
        command: vec![
            "sh".to_string(),
            "-c".to_string(),
            "touch marker".to_string(),
        ],
        cwd: None,
        env: std::collections::BTreeMap::new(),
        network: None,
        files: Vec::new(),
        skill_instance_id: None,
        initiated_by: ScriptExecutionInitiator::User,
        confirmed: true,
    };

    let result = record_blocked_script_execution(&ctx, &audit_root, &outside_audit_path, &request);

    assert!(result.is_err(), "outside audit path should be rejected");
    assert!(
        !outside_audit_path.exists(),
        "rejected audit path must not be created"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn imports_local_skill_to_tool_global_staging_and_refreshes_audit() {
    let root = temp_test_dir("tool-global-import");
    let source = root.join("source/local-skill");
    let staging = root.join("app-data/tool-global-staging");
    let user_home = root.join("home");
    std::fs::create_dir_all(&source).expect("create source");
    std::fs::create_dir_all(user_home.join(".claude")).expect("create claude dir");
    let claude_settings = user_home.join(".claude/settings.json");
    std::fs::write(
        &claude_settings,
        "{\"skillOverrides\":{\"existing\":\"off\"}}\n",
    )
    .expect("write claude settings");
    std::fs::write(
            source.join("SKILL.md"),
            "---\nname: Imported Skill\ndescription: Imported fixture\ntools:\n  - bash\n---\nRun `curl https://example.test/data.json`.\n",
        )
        .expect("write skill");
    std::fs::write(source.join("notes.txt"), "copied supporting file").expect("write support file");
    let catalog = Catalog::in_memory().expect("catalog");
    catalog.init().expect("init catalog");
    let ctx = AdapterContext {
        user_home: user_home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };

    let result =
        import_local_skill_to_tool_global(&catalog, &ctx, &staging, &source).expect("import");

    assert_eq!(result.imported.agent, "tool-global");
    assert_eq!(result.imported.scope, "tool-global");
    assert_eq!(result.imported.name, "Imported Skill");
    assert!(result.audit.read_only_preview);
    assert!(PathBuf::from(&result.staging_path).starts_with(
        staging
            .join("skills")
            .canonicalize()
            .expect("canonical staging skills root")
    ));
    assert!(PathBuf::from(&result.staging_path).exists());
    assert!(PathBuf::from(&result.staging_path)
        .parent()
        .expect("staged parent")
        .join("notes.txt")
        .exists());
    assert_eq!(
        std::fs::read_to_string(&claude_settings).expect("read settings"),
        "{\"skillOverrides\":{\"existing\":\"off\"}}\n"
    );
    assert!(
        result
            .findings
            .iter()
            .any(|finding| finding.rule_id == "name.canonical-case"),
        "import should run local rule audit for staged content"
    );
    let catalog_findings = list_findings(&catalog).expect("list findings");
    assert!(
        catalog_findings
            .iter()
            .any(|finding| finding.instance_id.as_deref() == Some(result.instance_id.as_str())),
        "import should refresh catalog findings"
    );

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn import_local_skill_rejects_missing_skill_md() {
    let root = temp_test_dir("tool-global-import-missing");
    let source = root.join("source/no-skill");
    std::fs::create_dir_all(&source).expect("create source");
    let catalog = Catalog::in_memory().expect("catalog");
    catalog.init().expect("init catalog");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };

    let error = import_local_skill_to_tool_global(&catalog, &ctx, &root.join("staging"), &source)
        .expect_err("missing SKILL.md should fail");

    assert!(matches!(error, CommandError::InvalidImportSource(_)));
    assert!(!root.join("staging/skills").exists());

    let _ = std::fs::remove_dir_all(root);
}

#[cfg(unix)]
#[test]
fn import_local_skill_rejects_source_symlink_escape() {
    let root = temp_test_dir("tool-global-import-symlink");
    let source = root.join("source/symlink-skill");
    let outside = root.join("outside");
    std::fs::create_dir_all(&source).expect("create source");
    std::fs::create_dir_all(&outside).expect("create outside");
    std::fs::write(
        source.join("SKILL.md"),
        "---\nname: symlink-skill\ndescription: symlink fixture\n---\nbody\n",
    )
    .expect("write skill");
    std::os::unix::fs::symlink(&outside, source.join("outside-link")).expect("create symlink");
    let catalog = Catalog::in_memory().expect("catalog");
    catalog.init().expect("init catalog");
    let ctx = AdapterContext {
        user_home: root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    };

    let error = import_local_skill_to_tool_global(&catalog, &ctx, &root.join("staging"), &source)
        .expect_err("symlink should fail");

    assert!(matches!(error, CommandError::InvalidImportSource(_)));

    let _ = std::fs::remove_dir_all(root);
}

#[test]
fn v28_local_rules_flag_permission_script_and_dependency_findings() {
    let network = local_rule_instance(
        "network",
        "name: network\ndescription: network\n",
        "Run `curl https://example.test/report.json` before summarizing.",
    );
    let mut exec = local_rule_instance(
        "exec",
        "name: exec\ndescription: exec\npermissions:\n  exec: true\n",
        "Run the generated command.",
    );
    exec.permissions.exec = true;
    exec.permissions.exec_declared = true;
    let shebang = local_rule_instance(
        "shebang",
        "name: shebang\ndescription: shebang\nscript: |\n  #!/bin/sh\n  echo hi\n",
        "No body script.",
    );
    let dependency = local_rule_instance(
        "dependency",
        "name: dependency\ndescription: dependency\ndependencies:\n  - requests\n",
        "No dependency body.",
    );
    let mut report = RuleReport::default();

    append_v28_local_rule_findings(&[network, exec, shebang, dependency], &mut report);

    assert_rule_present(&report, "permissions.network-declared");
    assert_rule_present(&report, "permissions.exec-needs-human");
    assert_rule_present(&report, "script.no-shebang");
    assert_rule_present(&report, "dependency.unknown");
    for finding in &report.findings {
        assert_eq!(finding.severity, Severity::Warn);
        assert!(finding.suggestion.as_deref().is_some_and(|s| !s.is_empty()));
        assert!(!finding.message.is_empty());
    }
}

#[test]
fn v28_local_rules_do_not_infer_unknown_or_missing_fields_as_safe() {
    let mut unknown_network = local_rule_instance(
        "unknown-network",
        "name: unknown-network\ndescription: unknown\n",
        "Run `curl https://example.test/report.json`.",
    );
    unknown_network.permissions.network = NetworkAccess::Unknown("internet".to_string());
    unknown_network.permissions.network_declared = true;
    let mut explicit_human = local_rule_instance(
            "explicit-human",
            "name: explicit-human\ndescription: exec\nrequires_human: true\npermissions:\n  exec: true\n",
            "Run the command.",
        );
    explicit_human.permissions.exec = true;
    explicit_human.permissions.exec_declared = true;
    let no_dependencies = local_rule_instance(
        "no-dependencies",
        "name: no-dependencies\ndescription: no deps\n",
        "This skill has no dependency declarations.",
    );
    let known_dependencies = local_rule_instance(
            "known-dependencies",
            "name: known-dependencies\ndescription: known deps\ndependencies:\n  - python3\n  - ./tools/local-helper\n",
            "Known local dependencies only.",
        );
    let mut report = RuleReport::default();

    append_v28_local_rule_findings(
        &[
            unknown_network,
            explicit_human,
            no_dependencies,
            known_dependencies,
        ],
        &mut report,
    );

    assert_rule_absent(&report, "permissions.network-declared");
    assert_rule_absent(&report, "permissions.exec-needs-human");
    assert_rule_absent(&report, "dependency.unknown");
}

#[test]
fn scans_claude_fixtures_into_catalog() {
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: fixture_path("fixtures/claude-code/empty-home"),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![AdapterRoot {
            scope: Scope::AgentGlobal,
            path: fixture_path("fixtures/claude-code/personal"),
            source: RootSource::Extra,
        }],
    };

    let count = scan_claude_to_catalog(&ctx, &catalog).expect("scan succeeds");
    let records = catalog.list_skill_records().expect("records list");

    assert_eq!(count, 1);
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].name, "summarize-changes");
}

#[test]
fn scan_all_includes_claude_and_codex_fixtures() {
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: fixture_path("fixtures/codex/user-home"),
        project_root: Some(fixture_path("fixtures/codex/project")),
        project_cwd: Some(fixture_path("fixtures/codex/project/nested")),
        extra_roots: vec![AdapterRoot {
            scope: Scope::AgentGlobal,
            path: fixture_path("fixtures/claude-code/personal"),
            source: RootSource::Extra,
        }],
    };

    let count = scan_all_to_catalog(&ctx, &catalog).expect("scan all succeeds");
    let records = catalog.list_skill_records().expect("records list");

    assert_eq!(count, 11);
    assert!(
        records
            .iter()
            .any(|record| record.agent == "claude-code" && record.name == "summarize-changes"),
        "Claude Code fixture should still be scanned"
    );
    assert!(
        records
            .iter()
            .any(|record| record.agent == "codex" && record.name == "user-alpha"),
        "Codex fixture should be included in scanAll"
    );
    assert!(
        records
            .iter()
            .any(|record| record.agent == "codex" && record.name == "repo-beta"),
        "Codex repo-root fixture should be included in scanAll"
    );
    assert!(
        records
            .iter()
            .any(|record| record.agent == "codex" && record.name == "nested-gamma"),
        "Codex nested cwd fixture should be included in scanAll"
    );
    assert!(
        records
            .iter()
            .any(|record| record.agent == "openclaw" && record.name == "user-alpha"),
        "OpenClaw should include documented shared ~/.agents/skills user roots"
    );
    assert!(
        records
            .iter()
            .any(|record| record.agent == "opencode" && record.name == "user-alpha"),
        "opencode should include documented shared ~/.agents/skills user roots"
    );
    assert!(
        records
            .iter()
            .any(|record| record.agent == "opencode" && record.name == "repo-beta"),
        "opencode should include documented project .agents/skills compatibility roots"
    );
    assert!(
        records
            .iter()
            .any(|record| record.agent == "opencode" && record.name == "nested-gamma"),
        "opencode should include nested project .agents/skills compatibility roots"
    );
    assert!(
        records
            .iter()
            .any(|record| record.agent == "pi" && record.name == "user-alpha"),
        "Pi should include documented shared ~/.agents/skills user roots"
    );
    assert!(
        records
            .iter()
            .any(|record| record.agent == "pi" && record.name == "repo-beta"),
        "Pi should include documented project .agents/skills compatibility roots"
    );
    assert!(
        records
            .iter()
            .any(|record| record.agent == "pi" && record.name == "nested-gamma"),
        "Pi should include nested project .agents/skills compatibility roots"
    );
}

#[test]
fn scan_all_report_splits_agent_counts_and_roots() {
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: fixture_path("fixtures/codex/user-home"),
        project_root: Some(fixture_path("fixtures/codex/project")),
        project_cwd: Some(fixture_path("fixtures/codex/project/nested")),
        extra_roots: vec![AdapterRoot {
            scope: Scope::AgentGlobal,
            path: fixture_path("fixtures/claude-code/personal"),
            source: RootSource::Extra,
        }],
    };

    let report = scan_all_catalog_report(&ctx, &catalog).expect("scan all succeeds");

    assert_eq!(report.scanned_count, 11);
    let claude = report
        .agents
        .iter()
        .find(|agent| agent.agent == AgentId::ClaudeCode)
        .expect("Claude Code report");
    assert_eq!(claude.display_name, "Claude Code");
    assert_eq!(claude.scanned_count, 1);
    assert!(claude
        .roots_considered
        .iter()
        .any(|root| root.ends_with("fixtures/claude-code/personal")));
    let codex = report
        .agents
        .iter()
        .find(|agent| agent.agent == AgentId::Codex)
        .expect("Codex report");
    assert_eq!(codex.display_name, "Codex");
    assert_eq!(codex.scanned_count, 3);
    assert_eq!(
        codex.scanned_roots.len(),
        3,
        "Codex scans user, repo, and nested cwd roots"
    );
    let opencode = report
        .agents
        .iter()
        .find(|agent| agent.agent == AgentId::Opencode)
        .expect("opencode report");
    assert_eq!(opencode.display_name, "opencode");
    assert_eq!(opencode.scanned_count, 3);
    assert_eq!(
        opencode.scanned_roots.len(),
        3,
        "opencode scans user, repo, and nested cwd .agents compatibility roots"
    );
    let openclaw = report
        .agents
        .iter()
        .find(|agent| agent.agent == AgentId::Openclaw)
        .expect("OpenClaw report");
    assert_eq!(openclaw.display_name, "OpenClaw");
    assert_eq!(openclaw.scanned_count, 1);
    let pi = report
        .agents
        .iter()
        .find(|agent| agent.agent == AgentId::Pi)
        .expect("Pi report");
    assert_eq!(pi.display_name, "Pi");
    assert_eq!(pi.scanned_count, 3);
    assert_eq!(
        pi.scanned_roots.len(),
        3,
        "Pi scans user, repo, and nested cwd .agents compatibility roots"
    );
}

#[test]
fn scan_all_includes_opencode_configured_local_paths_and_preserves_config_on_toggle() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-opencode-configured-command-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let configured_root = temp_root.join("configured-skills");
    let skill_dir = configured_root.join("custom-review");
    std::fs::create_dir_all(&skill_dir).expect("create configured skill dir");
    std::fs::create_dir_all(home.join(".config/opencode")).expect("create opencode config dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: custom-review\ndescription: opencode configured command fixture\n---\nbody",
    )
    .expect("write configured opencode skill");
    let config_path = home.join(".config/opencode/opencode.json");
    let config = serde_json::json!({
        "skills": {
            "paths": [path_text(&configured_root)],
            "urls": ["https://example.invalid/skills/"]
        }
    });
    std::fs::write(
        &config_path,
        serde_json::to_string(&config).expect("serialize opencode config"),
    )
    .expect("write opencode config");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let report = scan_all_catalog_report(&ctx, &catalog).expect("scan all succeeds");
    let opencode = report
        .agents
        .iter()
        .find(|agent| agent.agent == AgentId::Opencode)
        .expect("opencode report");
    assert_eq!(opencode.scanned_count, 1);
    assert!(opencode.scanned_roots.iter().any(|root| {
        root == &configured_root
            .canonicalize()
            .expect("canonical configured root")
            .to_string_lossy()
            .to_string()
    }));
    let record = catalog
        .list_skill_records()
        .expect("records")
        .into_iter()
        .find(|record| record.agent == "opencode" && record.name == "custom-review")
        .expect("configured opencode record");

    let disabled =
        toggle_skill(&catalog, &ctx, &record.id, false).expect("configured toggle succeeds");

    assert!(!disabled.enabled);
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path).expect("opencode config"))
            .expect("config json");
    assert_eq!(config["skills"]["paths"][0], path_text(&configured_root));
    assert_eq!(config["permission"]["skill"]["custom-review"], "deny");

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn exports_tool_global_manifest_stably_without_absolute_reproducible_paths() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-export-stable-{}",
        std::process::id()
    ));
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let instance = tool_global_instance(
        "tool-global-export-id",
        &temp_root.join("staging/prompt/SKILL.md"),
    );
    catalog
        .upsert_skill_instance(&instance)
        .expect("upsert tool-global instance");

    let first = export_skill_bundle(&catalog, "tool-global-export-id", &temp_root.join("out-a"))
        .expect("first export");
    let second = export_skill_bundle(&catalog, "tool-global-export-id", &temp_root.join("out-b"))
        .expect("second export");
    let first_manifest =
        std::fs::read_to_string(&first.manifest_path).expect("read first manifest");
    let second_manifest =
        std::fs::read_to_string(&second.manifest_path).expect("read second manifest");

    assert_eq!(
        first_manifest, second_manifest,
        "manifest content must be byte-stable across repeated exports"
    );
    assert!(
        !first_manifest.contains(&temp_root.to_string_lossy().to_string()),
        "reproducible manifest fields must not include absolute local paths"
    );
    assert!(first_manifest.contains("\"skill_path\": \"skill/SKILL.md\""));
    assert_eq!(first.fingerprint, instance.fingerprint);
    assert_eq!(first.metadata.source_scope, "tool-global");
    assert_eq!(first.metadata.version.as_deref(), Some("2.9.0"));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn reimports_export_bundle_with_stable_fingerprint_and_metadata() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-reimport-stable-{}",
        std::process::id()
    ));
    let source_dir = temp_root.join("incoming/review-helper");
    std::fs::create_dir_all(&source_dir).expect("create staging skill");
    std::fs::write(
            source_dir.join("SKILL.md"),
            "---\nname: review-helper\ndescription: Review helper\nversion: 2.9.0\npermissions:\n  network: none\n  requires_human: true\n---\nReview local changes only.\n",
        )
        .expect("write staging skill");

    let exported = export_staging_skill_bundle(&source_dir, &temp_root.join("exports"))
        .expect("export staging skill");
    let reimported =
        reimport_skill_bundle(&exported.bundle_path).expect("reimport exported bundle");

    assert_eq!(reimported.fingerprint, exported.fingerprint);
    assert_eq!(reimported.metadata, exported.metadata);
    assert_eq!(reimported.metadata.source_scope, "tool-global");
    assert_eq!(
        reimported
            .permissions
            .get("network")
            .and_then(serde_json::Value::as_str),
        Some("none")
    );
    assert_eq!(
        reimported
            .permissions
            .get("requires_human")
            .and_then(serde_json::Value::as_bool),
        Some(true)
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn scan_all_includes_openclaw_and_hermes_after_pi() {
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-pi-scan-all-{}", std::process::id()));
    let home = temp_root.join("home");
    let claude_path = write_claude_skill(&home, "claude-alpha");
    let codex_path = write_codex_skill(&home, "codex-alpha");
    let opencode_path = write_opencode_global_skill(&home, "opencode-alpha");
    let pi_path = write_pi_global_skill(&home, "pi-alpha");
    let hermes_path = write_hermes_global_skill(&home, "hermes-alpha");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let report = scan_all_catalog_report(&ctx, &catalog).expect("scan all succeeds");
    let records = catalog.list_skill_records().expect("records list");

    assert_eq!(report.scanned_count, 9);
    assert_eq!(
        report
            .agents
            .iter()
            .map(|agent| agent.agent)
            .collect::<Vec<_>>(),
        vec![
            AgentId::ClaudeCode,
            AgentId::Codex,
            AgentId::Opencode,
            AgentId::Pi,
            AgentId::Openclaw,
            AgentId::Hermes
        ],
        "scanAll reports OpenClaw and Hermes after Pi"
    );
    assert!(records.iter().any(|record| {
        record.agent == "claude-code" && record.name == "claude-alpha" && record.path == claude_path
    }));
    assert!(records.iter().any(|record| {
        record.agent == "codex" && record.name == "codex-alpha" && record.path == codex_path
    }));
    assert!(records.iter().any(|record| {
        record.agent == "opencode"
            && record.name == "opencode-alpha"
            && record.path == opencode_path
    }));
    assert!(records
        .iter()
        .any(|record| record.agent == "opencode" && record.name == "claude-alpha"));
    assert!(records
        .iter()
        .any(|record| record.agent == "opencode" && record.name == "codex-alpha"));
    assert!(records.iter().any(|record| {
        record.agent == "pi" && record.name == "pi-alpha" && record.path == pi_path
    }));
    assert!(records.iter().any(|record| {
        record.agent == "openclaw" && record.name == "codex-alpha" && record.path == codex_path
    }));
    assert!(records.iter().any(|record| {
        record.agent == "hermes" && record.name == "hermes-alpha" && record.path == hermes_path
    }));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn tool_global_staging_root_is_app_data_scoped() {
    let app_data = PathBuf::from("/tmp/skills-copilot-app-data");

    assert_eq!(
        tool_global_staging_skills_root(&app_data),
        app_data.join("tool-global/skills")
    );
}

#[test]
fn upserts_existing_staging_skill_as_tool_global_record() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-tool-global-upsert-{}",
        std::process::id()
    ));
    let app_data = temp_root.join("app-data");
    let home = temp_root.join("home");
    let staging_root =
        ensure_tool_global_staging_skills_root(&app_data).expect("create staging root");
    let skill_path = write_staging_skill(&staging_root, "imported-alpha");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let record = upsert_tool_global_staging_skill(&catalog, &ctx, &app_data, &skill_path)
        .expect("tool-global upsert succeeds");
    let records = catalog.list_skill_records().expect("records list");
    let detail = get_skill(&catalog, &record.id).expect("detail lookup");

    assert_eq!(records.len(), 1);
    assert_eq!(record.agent, "tool-global");
    assert_eq!(record.scope, "tool-global");
    assert_eq!(record.name, "imported-alpha");
    assert_eq!(record.path, skill_path);
    assert_eq!(
        record.display_path,
        PathBuf::from("$APP_DATA").join("tool-global/skills/imported-alpha/SKILL.md")
    );
    assert_eq!(detail.agent, "tool-global");
    assert_eq!(detail.scope, "tool-global");
    assert_eq!(detail.name, "imported-alpha");
    assert!(
        !home.join(".claude/settings.json").exists(),
        "tool-global upsert must not write Claude config"
    );
    assert!(
        !home.join(".codex/config.toml").exists(),
        "tool-global upsert must not write Codex config"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn tool_global_upsert_rejects_paths_outside_staging_root() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-tool-global-outside-{}",
        std::process::id()
    ));
    let app_data = temp_root.join("app-data");
    let outside_root = temp_root.join("outside");
    std::fs::create_dir_all(&outside_root).expect("create outside root");
    ensure_tool_global_staging_skills_root(&app_data).expect("create staging root");
    let outside_path = write_staging_skill(&outside_root, "outside-alpha");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: temp_root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let err = upsert_tool_global_staging_skill(&catalog, &ctx, &app_data, &outside_path)
        .expect_err("outside staging path must be rejected");

    assert!(
        err.to_string().contains("outside staging root"),
        "unexpected error: {err}"
    );
    assert_eq!(catalog.list_skill_records().expect("records").len(), 0);

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn scan_all_preserves_tool_global_record() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-tool-global-scan-{}",
        std::process::id()
    ));
    let app_data = temp_root.join("app-data");
    let home = temp_root.join("home");
    let staging_root =
        ensure_tool_global_staging_skills_root(&app_data).expect("create staging root");
    let tool_global_path = write_staging_skill(&staging_root, "tool-persist");
    let claude_path = write_claude_skill(&home, "claude-visible");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let tool_global =
        upsert_tool_global_staging_skill(&catalog, &ctx, &app_data, &tool_global_path)
            .expect("tool-global upsert succeeds");
    scan_all_to_catalog(&ctx, &catalog).expect("scan all succeeds");
    let records = catalog.list_skill_records().expect("records list");

    assert!(records.iter().any(|record| {
        record.id == tool_global.id && record.agent == "tool-global" && record.state == "loaded"
    }));
    assert!(records.iter().any(|record| {
        record.agent == "claude-code"
            && record.name == "claude-visible"
            && record.path == claude_path
    }));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn tool_global_and_agent_global_same_name_overlap_without_runtime_conflict() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-tool-global-conflict-{}",
        std::process::id()
    ));
    let app_data = temp_root.join("app-data");
    let home = temp_root.join("home");
    let staging_root =
        ensure_tool_global_staging_skills_root(&app_data).expect("create staging root");
    let tool_global_path = write_staging_skill(&staging_root, "shared-alpha");
    let agent_global_path = write_claude_skill(&home, "shared-alpha");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let tool_global =
        upsert_tool_global_staging_skill(&catalog, &ctx, &app_data, &tool_global_path)
            .expect("tool-global upsert succeeds");
    scan_all_to_catalog(&ctx, &catalog).expect("scan all succeeds");
    let records = catalog.list_skill_records().expect("records list");
    let agent_global = records
        .iter()
        .find(|record| record.agent == "claude-code" && record.path == agent_global_path)
        .expect("agent-global record");
    let tool_global_after = records
        .iter()
        .find(|record| record.id == tool_global.id)
        .expect("tool-global record");

    assert_eq!(records.len(), 3);
    assert_eq!(agent_global.scope, "agent-global");
    assert_eq!(tool_global_after.scope, "tool-global");
    assert_eq!(
        agent_global.definition_id, tool_global_after.definition_id,
        "same names share a definition id for conflict display"
    );

    let conflicts = list_conflicts(&catalog).expect("conflicts list");
    assert!(
        conflicts.iter().all(|conflict| {
            !(conflict.instance_ids.contains(&agent_global.id)
                && conflict.instance_ids.contains(&tool_global.id))
        }),
        "tool-global and agent runtime rows overlap in analysis, not conflict tab"
    );
    assert!(records
        .iter()
        .any(|record| record.agent == "opencode" && record.name == "shared-alpha"));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn toggle_opencode_skill_writes_permission_skill_deny_and_rollback_restores_snapshot() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-opencode-toggle-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    write_opencode_global_skill(&home, "writable-skill");
    let config_path = home.join(".config/opencode/opencode.json");
    std::fs::write(&config_path, "{}\n").expect("write original opencode config");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    scan_all_to_catalog(&ctx, &catalog).expect("scan all");
    let opencode_record = catalog
        .list_skill_records()
        .expect("records")
        .into_iter()
        .find(|record| record.agent == "opencode" && record.name == "writable-skill")
        .expect("opencode record");

    let disabled = toggle_skill(&catalog, &ctx, &opencode_record.id, false)
        .expect("opencode disable succeeds");

    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path).expect("opencode config"))
            .expect("config json");
    assert_eq!(config["permission"]["skill"]["writable-skill"], "deny");
    assert!(!disabled.enabled);
    assert_eq!(disabled.state, "disabled");

    let snapshots = catalog
        .list_config_snapshots("opencode", &config_path.to_string_lossy())
        .expect("snapshots");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].scope, "agent-global");

    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, &snapshots[0].id)
        .expect("opencode rollback preview");
    assert_eq!(preview.snapshot.agent, "opencode");
    let preview_current: serde_json::Value =
        serde_json::from_str(&preview.current_content).expect("preview current json");
    assert_eq!(
        preview_current["permission"]["skill"]["writable-skill"],
        "deny"
    );
    assert!(preview.changed);
    assert!(preview.rollback_supported);

    rollback_snapshot(&catalog, &ctx, &snapshots[0].id, &preview.preview_token)
        .expect("opencode rollback succeeds");
    let config_text = std::fs::read_to_string(&config_path).expect("rolled back opencode config");
    assert_eq!(config_text, "{}\n");
    let rolled_back_record = catalog
        .list_skill_records()
        .expect("records after opencode rollback")
        .into_iter()
        .find(|record| record.agent == "opencode" && record.name == "writable-skill")
        .expect("opencode record after rollback");
    assert!(rolled_back_record.enabled);
    assert_eq!(rolled_back_record.state, "loaded");

    let disabled = toggle_skill(&catalog, &ctx, &rolled_back_record.id, false)
        .expect("opencode disable after rollback succeeds");
    assert!(!disabled.enabled);
    let enabled = toggle_skill(&catalog, &ctx, &rolled_back_record.id, true)
        .expect("opencode enable succeeds");
    let config: serde_json::Value =
        serde_json::from_str(&std::fs::read_to_string(&config_path).expect("opencode config"))
            .expect("config json");
    assert!(config["permission"]["skill"]
        .get("writable-skill")
        .is_none());
    assert!(enabled.enabled);
    assert_eq!(enabled.state, "loaded");

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn codex_cwd_walk_records_selected_project_root() {
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let selected_project = fixture_path("fixtures/codex/project");
    let ctx = AdapterContext {
        user_home: fixture_path("fixtures/codex/user-home"),
        project_root: Some(selected_project.clone()),
        project_cwd: Some(selected_project.join("nested")),
        extra_roots: vec![],
    };

    scan_all_to_catalog(&ctx, &catalog).expect("scan all succeeds");
    let nested_record = catalog
        .list_skill_records()
        .expect("records")
        .into_iter()
        .find(|record| record.agent == "codex" && record.name == "nested-gamma")
        .expect("nested cwd Codex record");
    let meta = catalog
        .get_skill_instance_meta(&nested_record.id)
        .expect("meta lookup")
        .expect("meta present");

    assert_eq!(
        meta.project_root,
        Some(selected_project),
        "cwd walk should keep the selected project root as the catalog boundary"
    );
}

#[test]
fn scan_all_project_context_sweeps_only_current_boundary() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-project-context-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let project_a = temp_root.join("project-a");
    let project_b = temp_root.join("project-b");
    let global_path = write_codex_skill(&home, "global-visible");
    let project_a_path = write_codex_skill(&project_a, "project-a-visible");
    let project_b_path = write_codex_skill(&project_b, "project-b-visible");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");

    let ctx_a = AdapterContext {
        user_home: home.clone(),
        project_root: Some(project_a.clone()),
        project_cwd: Some(project_a.clone()),
        extra_roots: vec![],
    };
    scan_all_to_catalog(&ctx_a, &catalog).expect("project A scan");
    let records = catalog.list_skill_records().expect("records after A");
    assert!(
        records.iter().any(|record| record.path == project_a_path),
        "project A scan records project A skill"
    );
    assert!(
        records.iter().any(|record| record.path == global_path),
        "project A scan records user-scope Codex skill"
    );

    let ctx_b = AdapterContext {
        user_home: home.clone(),
        project_root: Some(project_b.clone()),
        project_cwd: Some(project_b.clone()),
        extra_roots: vec![],
    };
    scan_all_to_catalog(&ctx_b, &catalog).expect("project B scan");
    let records = catalog.list_skill_records().expect("records after B");
    assert!(
        records
            .iter()
            .any(|record| record.path == project_a_path && record.state == "loaded"),
        "project B scan does not mark project A record missing"
    );
    assert!(
        records
            .iter()
            .any(|record| record.path == project_b_path && record.state == "loaded"),
        "project B scan records project B skill"
    );

    let foreign_under_b = project_b
        .join(".agents/skills/foreign-project-a-record/SKILL.md")
        .canonicalize()
        .unwrap_or_else(|_| project_b.join(".agents/skills/foreign-project-a-record/SKILL.md"));
    catalog
        .upsert_skill_instance(&synthetic_codex_project_instance(
            "foreign-under-b",
            &project_a,
            foreign_under_b.clone(),
            "foreign-under-b",
        ))
        .expect("upsert foreign project record");
    let foreign_toggle = toggle_skill(&catalog, &ctx_b, "foreign-under-b", false)
        .expect_err("foreign project rows must not be writable in current context");
    assert!(
        foreign_toggle
            .to_string()
            .contains("current project context"),
        "unexpected foreign project toggle error: {foreign_toggle}"
    );
    scan_all_to_catalog(&ctx_b, &catalog).expect("project B rescan");
    let records = catalog
        .list_skill_records()
        .expect("records after B rescan");
    assert!(
        records
            .iter()
            .any(|record| record.id == "foreign-under-b" && record.state == "loaded"),
        "project B scan must not sweep an AgentProject row owned by project A"
    );

    let project_scoped_under_user_root = home.join(".agents/skills/project-scoped-leak/SKILL.md");
    catalog
        .upsert_skill_instance(&synthetic_codex_project_instance(
            "project-scoped-under-user-root",
            &project_a,
            project_scoped_under_user_root,
            "project-scoped-under-user-root",
        ))
        .expect("upsert no-project guard record");
    let clear_ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    scan_all_to_catalog(&clear_ctx, &catalog).expect("clear project scan");
    let records = catalog.list_skill_records().expect("records after clear");
    assert!(
        records.iter().any(|record| {
            record.id == "project-scoped-under-user-root" && record.state == "loaded"
        }),
        "no-project scan must not sweep project-scoped records under scanned user roots"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[cfg(unix)]
#[test]
fn dangling_link_scan_reconciles_stale_rows_without_degrading_root() {
    let temp_root = temp_test_dir("partial-cache-refresh");
    let scan_root = temp_root.join("skills");
    let observed_path = write_command_scan_fixture(&scan_root, "observed");
    let unobserved_path = write_command_scan_fixture(&scan_root, "unobserved");
    let observed = seeded_command_scan_instance("seed-observed", &observed_path, "observed");
    let unobserved =
        seeded_command_scan_instance("seed-unobserved", &unobserved_path, "unobserved");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instances(&[observed, unobserved.clone()])
        .expect("seed catalog rows");
    std::fs::remove_file(&unobserved_path).expect("remove unobserved skill");
    std::os::unix::fs::symlink(
        scan_root.join("missing-target"),
        scan_root.join("dangling-directory-link"),
    )
    .expect("create dangling directory link");
    let ctx = AdapterContext {
        user_home: temp_root.join("empty-home"),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![AdapterRoot {
            scope: Scope::AgentGlobal,
            path: scan_root.clone(),
            source: RootSource::Extra,
        }],
    };

    let report = scan_all_catalog_report(&ctx, &catalog).expect("partial refresh succeeds");
    let claude_report = report
        .agents
        .iter()
        .find(|agent| agent.agent == AgentId::ClaudeCode)
        .expect("Claude report");
    let canonical_root = scan_root.canonicalize().expect("canonical scan root");
    let instances = catalog
        .list_skill_instances_for_project_context(None)
        .expect("instances after partial refresh");
    let observed = instances
        .iter()
        .find(|instance| instance.agent == AgentId::ClaudeCode && instance.name == "observed")
        .expect("observed instance remains");
    let unobserved_after = instances
        .iter()
        .find(|instance| instance.id == unobserved.id)
        .expect("unobserved instance remains");

    assert!(observed.last_seen > 0, "seen row should advance last_seen");
    assert_eq!(observed.state, SkillState::Loaded);
    assert_eq!(unobserved_after.state, SkillState::Missing);
    assert!(claude_report.partial_roots.is_empty());
    assert!(claude_report.scanned_roots.contains(&canonical_root));
    assert!(claude_report
        .issues
        .iter()
        .any(|issue| issue.kind == "dangling_symlink"));
    assert!(
        catalog
            .list_skill_events(&unobserved.id, None)
            .expect("unobserved events")
            .iter()
            .any(|event| event.kind == "missing"),
        "a complete refresh must reconcile the stale row"
    );

    let _ = std::fs::remove_dir_all(temp_root);
}

#[test]
fn complete_scan_marks_removed_rows_missing() {
    let temp_root = temp_test_dir("complete-cache-refresh");
    let scan_root = temp_root.join("skills");
    let observed_path = write_command_scan_fixture(&scan_root, "observed");
    let unobserved_path = write_command_scan_fixture(&scan_root, "unobserved");
    let observed = seeded_command_scan_instance("seed-observed", &observed_path, "observed");
    let unobserved =
        seeded_command_scan_instance("seed-unobserved", &unobserved_path, "unobserved");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instances(&[observed, unobserved.clone()])
        .expect("seed catalog rows");
    std::fs::remove_file(&unobserved_path).expect("remove unobserved skill");
    let ctx = AdapterContext {
        user_home: temp_root.join("empty-home"),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![AdapterRoot {
            scope: Scope::AgentGlobal,
            path: scan_root.clone(),
            source: RootSource::Extra,
        }],
    };

    let report = scan_all_catalog_report(&ctx, &catalog).expect("complete refresh succeeds");
    let claude_report = report
        .agents
        .iter()
        .find(|agent| agent.agent == AgentId::ClaudeCode)
        .expect("Claude report");
    let instances = catalog
        .list_skill_instances_for_project_context(None)
        .expect("instances after complete refresh");
    let observed = instances
        .iter()
        .find(|instance| instance.agent == AgentId::ClaudeCode && instance.name == "observed")
        .expect("observed instance remains");
    let unobserved_after = instances
        .iter()
        .find(|instance| instance.id == unobserved.id)
        .expect("unobserved instance remains");
    let events = catalog
        .list_skill_events(&unobserved.id, None)
        .expect("unobserved events");

    assert!(observed.last_seen > 0, "seen row should advance last_seen");
    assert_eq!(observed.state, SkillState::Loaded);
    assert_eq!(unobserved_after.state, SkillState::Missing);
    assert!(claude_report.partial_roots.is_empty());
    assert_eq!(events.len(), 1);
    assert_eq!(events[0].kind, "missing");

    let _ = std::fs::remove_dir_all(temp_root);
}

#[test]
fn marks_deleted_fixture_as_missing_on_rescan() {
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-sweep-{}", std::process::id()));
    let personal = temp_root.join("personal");
    let skill_dir = personal.join("ephemeral");
    std::fs::create_dir_all(&skill_dir).expect("create temp skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    let skill_body =
        "---\nname: ephemeral\ndescription: temporary sweep test skill\n---\nBody content.\n";
    std::fs::write(&skill_path, skill_body).expect("write temp skill");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: temp_root.join("empty-home"),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![AdapterRoot {
            scope: Scope::AgentGlobal,
            path: personal.clone(),
            source: RootSource::Extra,
        }],
    };

    let first_count = scan_claude_to_catalog(&ctx, &catalog).expect("first scan");
    assert_eq!(first_count, 1);
    let records = catalog
        .list_skill_records()
        .expect("records after first scan");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, "loaded");

    std::fs::remove_file(&skill_path).expect("delete skill file");

    let second_count = scan_claude_to_catalog(&ctx, &catalog).expect("second scan");
    assert_eq!(second_count, 0, "no skills found after deletion");
    let records = catalog
        .list_skill_records()
        .expect("records after second scan");
    assert_eq!(records.len(), 1, "record retained but marked missing");
    assert_eq!(
        records[0].state, "missing",
        "deleted file is marked missing"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn sweep_does_not_touch_records_outside_scanned_roots() {
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-scope-{}", std::process::id()));

    let project_skill_dir = temp_root
        .join("project")
        .join(".claude")
        .join("skills")
        .join("never-scanned");
    std::fs::create_dir_all(&project_skill_dir).expect("create project skill dir");
    let project_path = project_skill_dir.join("SKILL.md");
    std::fs::write(
        &project_path,
        "---\nname: never-scanned\ndescription: synthetic\n---\nbody",
    )
    .expect("write project skill");
    let project_path = project_path
        .canonicalize()
        .expect("canonicalize project path");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let project_inst = SkillInstance {
        id: "synthetic-project-id".to_string(),
        agent: AgentId::ClaudeCode,
        scope: Scope::AgentProject,
        project_root: Some(temp_root.join("project")),
        path: project_path.clone(),
        display_path: project_path.clone(),
        definition_id: "never-scanned".to_string(),
        name: "never-scanned".to_string(),
        display_name: "never-scanned".to_string(),
        description: "synthetic project record".to_string(),
        version: None,
        state: SkillState::Loaded,
        enabled: true,
        frontmatter_raw: String::new(),
        body: String::new(),
        scripts: Vec::new(),
        permissions: PermissionRequest::default(),
        fingerprint: String::new(),
        mtime: 0,
        first_seen: 0,
        last_seen: 0,
    };
    catalog
        .upsert_skill_instance(&project_inst)
        .expect("upsert project record");

    let personal = temp_root.join("personal");
    let ephemeral_dir = personal.join("ephemeral");
    std::fs::create_dir_all(&ephemeral_dir).expect("create personal skill dir");
    std::fs::write(
        ephemeral_dir.join("SKILL.md"),
        "---\nname: ephemeral\ndescription: x\n---\nbody",
    )
    .expect("write personal skill");

    let ctx = AdapterContext {
        user_home: temp_root.join("empty-home"),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![AdapterRoot {
            scope: Scope::AgentGlobal,
            path: personal,
            source: RootSource::Extra,
        }],
    };

    scan_claude_to_catalog(&ctx, &catalog).expect("scan succeeds");

    let records = catalog.list_skill_records().expect("records");
    let project_record = records
        .iter()
        .find(|r| r.path == project_path)
        .expect("project record still present");
    assert_eq!(
        project_record.state, "loaded",
        "project record outside scanned roots is not swept"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn toggle_off_writes_skill_overrides_and_creates_snapshot() {
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-toggle-{}", std::process::id()));
    let home = temp_root.join("home");
    std::fs::create_dir_all(home.join(".claude/skills/foo")).expect("create skill dir");
    std::fs::write(
        home.join(".claude/skills/foo/SKILL.md"),
        "---\nname: foo\n---\nbody",
    )
    .expect("write skill");
    let settings_path = home.join(".claude/settings.json");
    std::fs::write(&settings_path, "{}\n").expect("write initial settings");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let inst = SkillInstance {
        id: "toggle-off-id".to_string(),
        agent: AgentId::ClaudeCode,
        scope: Scope::AgentGlobal,
        project_root: None,
        path: home.join(".claude/skills/foo/SKILL.md"),
        display_path: home.join(".claude/skills/foo/SKILL.md"),
        definition_id: "foo".to_string(),
        name: "foo".to_string(),
        display_name: "foo".to_string(),
        description: "test".to_string(),
        version: None,
        state: SkillState::Loaded,
        enabled: true,
        frontmatter_raw: String::new(),
        body: String::new(),
        scripts: Vec::new(),
        permissions: PermissionRequest::default(),
        fingerprint: String::new(),
        mtime: 0,
        first_seen: 0,
        last_seen: 0,
    };
    catalog.upsert_skill_instance(&inst).expect("upsert");

    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let record = toggle_skill(&catalog, &ctx, "toggle-off-id", false).expect("toggle off");
    assert!(!record.enabled);
    assert_eq!(record.state, "disabled");

    let content = std::fs::read_to_string(&settings_path).expect("read settings");
    assert!(
        content.contains("\"foo\""),
        "skillOverrides has the skill name"
    );
    assert!(content.contains("\"off\""), "skillOverrides set to off");

    let snapshots = catalog
        .list_config_snapshots("claude-code", &settings_path.to_string_lossy())
        .expect("list snapshots");
    assert_eq!(snapshots.len(), 1, "exactly one pre-toggle snapshot");
    assert_eq!(snapshots[0].reason, "pre-toggle");
    assert_eq!(
        snapshots[0].content, "{}\n",
        "snapshot captures pre-toggle state"
    );

    let events = list_skill_events(&catalog, "toggle-off-id", Some(10)).expect("list events");
    assert_eq!(
        events.len(),
        1,
        "toggle writes one current-skill history event"
    );
    assert_eq!(events[0].instance_id, "toggle-off-id");
    assert_eq!(events[0].kind, "toggle");
    assert_eq!(events[0].payload["enabled"], serde_json::json!(false));
    assert_eq!(
        events[0].payload["previous_enabled"],
        serde_json::json!(true)
    );
    assert_eq!(events[0].payload["agent"], serde_json::json!("claude-code"));
    assert_eq!(
        events[0].payload["scope"],
        serde_json::json!("agent-global")
    );
    assert_eq!(events[0].payload["skill_name"], serde_json::json!("foo"));
    assert_eq!(
        events[0].payload["config_scope"],
        serde_json::json!("agent-global")
    );
    assert!(
        events[0].payload.get("target").is_some(),
        "event payload should include the config target for lightweight History"
    );
    assert!(
        events[0].payload.get("body").is_none()
            && events[0].payload.get("frontmatter_raw").is_none()
            && events[0].payload.get("permissions").is_none(),
        "event payload remains a lightweight summary, not a full skill snapshot"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn toggle_on_removes_skill_overrides_entry() {
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-toggle-on-{}", std::process::id()));
    let home = temp_root.join("home");
    std::fs::create_dir_all(home.join(".claude/skills/bar")).expect("create skill dir");
    let settings_path = home.join(".claude/settings.json");
    let initial = "{\n  \"skillOverrides\": {\n    \"bar\": \"off\"\n  }\n}\n";
    std::fs::write(&settings_path, initial).expect("write initial settings");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let inst = SkillInstance {
        id: "toggle-on-id".to_string(),
        agent: AgentId::ClaudeCode,
        scope: Scope::AgentGlobal,
        project_root: None,
        path: home.join(".claude/skills/bar/SKILL.md"),
        display_path: home.join(".claude/skills/bar/SKILL.md"),
        definition_id: "bar".to_string(),
        name: "bar".to_string(),
        display_name: "bar".to_string(),
        description: "test".to_string(),
        version: None,
        state: SkillState::Disabled,
        enabled: false,
        frontmatter_raw: String::new(),
        body: String::new(),
        scripts: Vec::new(),
        permissions: PermissionRequest::default(),
        fingerprint: String::new(),
        mtime: 0,
        first_seen: 0,
        last_seen: 0,
    };
    catalog.upsert_skill_instance(&inst).expect("upsert");

    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let record = toggle_skill(&catalog, &ctx, "toggle-on-id", true).expect("toggle on");
    assert!(record.enabled);
    assert_eq!(record.state, "loaded");

    let content = std::fs::read_to_string(&settings_path).expect("read settings");
    assert!(
        !content.contains("\"bar\""),
        "skillOverrides entry for bar is removed"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn batch_toggle_preview_filters_read_only_and_apply_uses_snapshot_path() {
    let temp_root = temp_test_dir("batch-toggle");
    let home = temp_root.join("home");
    write_claude_skill(&home, "batch-claude");
    write_pi_global_skill(&home, "batch-pi");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    scan_all_to_catalog(&ctx, &catalog).expect("scan all");
    let records = catalog.list_skill_records().expect("records");
    let claude_id = records
        .iter()
        .find(|record| record.agent == "claude-code" && record.name == "batch-claude")
        .expect("claude record")
        .id
        .clone();
    let pi_id = records
        .iter()
        .find(|record| record.agent == "pi" && record.name == "batch-pi")
        .expect("pi record")
        .id
        .clone();

    let selection = vec![claude_id.clone(), pi_id.clone(), claude_id.clone()];
    let preview = preview_skill_toggles(&catalog, &ctx, &selection, false).expect("batch preview");
    assert_eq!(preview.requested_count, 3);
    assert_eq!(preview.writable_count, 2);
    assert_eq!(preview.skipped_count, 1);
    assert!(preview.writes_allowed);
    assert_eq!(preview.affected_items[0].instance_id, claude_id);
    assert!(preview
        .affected_items
        .iter()
        .any(|item| item.instance_id == pi_id && item.capability_label.contains("Pi guarded")));
    assert!(preview
        .skipped_items
        .iter()
        .any(|item| item.instance_id == claude_id && item.reason.contains("Duplicate")));
    assert!(preview
        .snapshot_rollback_notes
        .iter()
        .any(|note| note.contains("pre-batch-toggle")));

    let stale = apply_skill_toggles(&catalog, &ctx, &selection, false, "stale-token")
        .expect_err("stale token must be rejected");
    assert!(matches!(stale, CommandError::InvalidBatchAction(_)));

    let applied = apply_skill_toggles(&catalog, &ctx, &selection, false, &preview.preview_token)
        .expect("batch apply");
    assert_eq!(applied.applied_count, 2);
    assert_eq!(applied.updated_records.len(), 2);
    assert!(applied.updated_records.iter().all(|record| !record.enabled));

    let settings_path = home.join(".claude/settings.json");
    let content = std::fs::read_to_string(&settings_path).expect("read settings");
    assert!(content.contains("\"batch-claude\""));
    assert!(content.contains("\"off\""));
    let pi_settings_path = home.join(".pi/agent/settings.json");
    let pi_content = std::fs::read_to_string(&pi_settings_path).expect("read Pi settings");
    assert!(pi_content.contains("batch-pi/SKILL.md"));

    let snapshots = catalog
        .list_config_snapshots("claude-code", &settings_path.to_string_lossy())
        .expect("list snapshots");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].reason, "pre-batch-toggle");
    let pi_snapshots = catalog
        .list_agent_config_snapshots("pi", Some(Scope::AgentGlobal.as_str()), None)
        .expect("list Pi snapshots");
    assert_eq!(pi_snapshots.len(), 1);
    assert_eq!(pi_snapshots[0].reason, "pre-batch-toggle");
    assert_eq!(
        PathBuf::from(&pi_snapshots[0].target)
            .canonicalize()
            .expect("canonical Pi snapshot target"),
        pi_settings_path
            .canonicalize()
            .expect("canonical Pi settings path")
    );

    for record in &applied.updated_records {
        let events = list_skill_events(&catalog, &record.id, Some(10)).expect("list events");
        assert_eq!(events.len(), 1);
        assert_eq!(events[0].payload["batch"], serde_json::json!(true));
        assert!(events[0].payload.get("snapshot_id").is_some());
    }

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn toggle_pi_global_skill_writes_settings_rescans_and_rolls_back() {
    let temp_root = temp_test_dir("pi-toggle-global");
    let home = temp_root.join("home");
    write_pi_global_skill(&home, "pi-toggle");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    scan_all_to_catalog(&ctx, &catalog).expect("scan all");
    let pi_id = catalog
        .list_skill_records()
        .expect("records")
        .into_iter()
        .find(|record| record.agent == "pi" && record.name == "pi-toggle")
        .expect("pi record")
        .id;

    let record = toggle_skill(&catalog, &ctx, &pi_id, false).expect("toggle Pi off");
    assert!(!record.enabled);
    assert_eq!(record.state, "disabled");

    let settings_path = home.join(".pi/agent/settings.json");
    let content = std::fs::read_to_string(&settings_path).expect("read Pi settings");
    assert!(content.contains("pi-toggle/SKILL.md"));
    assert!(content.contains("-"));

    scan_all_to_catalog(&ctx, &catalog).expect("rescan all");
    let rescanned = catalog
        .get_skill_record(&pi_id)
        .expect("catalog lookup")
        .expect("Pi record remains");
    assert!(!rescanned.enabled);
    assert_eq!(rescanned.state, "disabled");

    let snapshots = catalog
        .list_agent_config_snapshots("pi", Some(Scope::AgentGlobal.as_str()), None)
        .expect("list Pi snapshots");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].reason, "pre-toggle");
    assert_eq!(
        PathBuf::from(&snapshots[0].target)
            .canonicalize()
            .expect("canonical Pi snapshot target"),
        settings_path
            .canonicalize()
            .expect("canonical Pi settings path")
    );
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, &snapshots[0].id)
        .expect("Pi rollback preview");
    rollback_snapshot(&catalog, &ctx, &snapshots[0].id, &preview.preview_token)
        .expect("Pi rollback succeeds");
    let rolled_back = std::fs::read_to_string(&settings_path).unwrap_or_default();
    assert!(
        !rolled_back.contains("pi-toggle"),
        "rollback restores pre-toggle Pi settings"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn toggle_pi_project_skill_allows_default_project_settings_and_blocks_explicit_untrusted() {
    let temp_root = temp_test_dir("pi-toggle-project");
    let home = temp_root.join("home");
    let project = temp_root.join("project");
    let skill_dir = project.join(".pi/skills/pi-project-toggle");
    std::fs::create_dir_all(&skill_dir).expect("create Pi project skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: pi-project-toggle\ndescription: Project Pi toggle fixture\n---\nbody",
    )
    .expect("write Pi project skill");
    let settings_path = project.join(".pi/settings.json");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: Some(project.clone()),
        project_cwd: Some(project.clone()),
        extra_roots: vec![],
    };
    scan_all_to_catalog(&ctx, &catalog).expect("scan all");
    let pi_id = catalog
        .list_skill_records()
        .expect("records")
        .into_iter()
        .find(|record| record.agent == "pi" && record.name == "pi-project-toggle")
        .expect("Pi project record")
        .id;

    let record =
        toggle_skill(&catalog, &ctx, &pi_id, false).expect("default Pi project toggle succeeds");
    assert!(!record.enabled);
    let content = std::fs::read_to_string(&settings_path).expect("read Pi settings");
    assert!(content.contains("pi-project-toggle/SKILL.md"));

    std::fs::write(
        &settings_path,
        "{\n  \"project\": { \"trusted\": false },\n  \"skills\": { \"disabled\": [\"pi-project-toggle\"] }\n}\n",
    )
    .expect("write explicitly untrusted Pi settings");
    let blocked = toggle_skill(&catalog, &ctx, &pi_id, true)
        .expect_err("explicitly untrusted Pi project writes are blocked");
    assert!(matches!(blocked, CommandError::Adapter(_)));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn toggle_pi_project_compatibility_skill_allows_default_project_settings() {
    let temp_root = temp_test_dir("pi-toggle-project-compat");
    let home = temp_root.join("home");
    let project = temp_root.join("project");
    write_pi_project_compatibility_skill(&project, "pi-agent-compat");
    let settings_path = project.join(".pi/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create Pi settings dir");
    std::fs::write(&settings_path, "{\n  \"skills\": { \"disabled\": [] }\n}\n")
        .expect("write default Pi settings");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: Some(project.clone()),
        project_cwd: Some(project.clone()),
        extra_roots: vec![],
    };
    scan_all_to_catalog(&ctx, &catalog).expect("scan all");
    let pi_id = catalog
        .list_skill_records()
        .expect("records")
        .into_iter()
        .find(|record| record.agent == "pi" && record.name == "pi-agent-compat")
        .expect("Pi compatibility project record")
        .id;

    let record = toggle_skill(&catalog, &ctx, &pi_id, false)
        .expect("default Pi compatibility toggle succeeds");
    assert!(!record.enabled);
    let content = std::fs::read_to_string(&settings_path).expect("read Pi settings");
    assert!(content.contains("pi-agent-compat/SKILL.md"));

    scan_all_to_catalog(&ctx, &catalog).expect("rescan all");
    let rescanned = catalog
        .get_skill_record(&pi_id)
        .expect("catalog lookup")
        .expect("Pi compatibility record remains");
    assert!(!rescanned.enabled);
    assert_eq!(rescanned.state, "disabled");

    std::fs::write(
        &settings_path,
        "{\n  \"trust\": { \"projectRootTrusted\": false },\n  \"skills\": { \"disabled\": [\"pi-agent-compat\"] }\n}\n",
    )
    .expect("write explicitly untrusted Pi compatibility settings");
    let blocked = toggle_skill(&catalog, &ctx, &pi_id, true)
        .expect_err("explicitly untrusted Pi compatibility writes are blocked");
    assert!(matches!(blocked, CommandError::Adapter(_)));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn toggle_hermes_global_skill_writes_config_rescans_and_rolls_back() {
    let temp_root = temp_test_dir("hermes-toggle-global");
    let home = temp_root.join("home");
    write_hermes_global_skill(&home, "hermes-toggle");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    scan_all_to_catalog(&ctx, &catalog).expect("scan all");
    let hermes_id = catalog
        .list_skill_records()
        .expect("records")
        .into_iter()
        .find(|record| record.agent == "hermes" && record.name == "hermes-toggle")
        .expect("Hermes record")
        .id;

    let record =
        toggle_skill(&catalog, &ctx, &hermes_id, false).expect("toggle Hermes off succeeds");
    assert!(!record.enabled);
    assert_eq!(record.state, "disabled");
    let config_path = home.join(".hermes/config.yaml");
    let content = std::fs::read_to_string(&config_path).expect("read Hermes config");
    assert!(content.contains("hermes-toggle"));

    scan_all_to_catalog(&ctx, &catalog).expect("rescan all");
    let rescanned = catalog
        .get_skill_record(&hermes_id)
        .expect("catalog lookup")
        .expect("Hermes record remains");
    assert!(!rescanned.enabled);
    assert_eq!(rescanned.state, "disabled");

    let snapshots = catalog
        .list_config_snapshots("hermes", &config_path.to_string_lossy())
        .expect("list Hermes snapshots");
    assert_eq!(snapshots.len(), 1);
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, &snapshots[0].id)
        .expect("Hermes rollback preview");
    rollback_snapshot(&catalog, &ctx, &snapshots[0].id, &preview.preview_token)
        .expect("Hermes rollback succeeds");
    let rolled_back = std::fs::read_to_string(&config_path).unwrap_or_default();
    assert!(!rolled_back.contains("hermes-toggle"));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn toggle_openclaw_skill_writes_json5_entries_rescans_and_rolls_back() {
    let temp_root = temp_test_dir("openclaw-toggle-global");
    let home = temp_root.join("home");
    write_openclaw_global_skill_with_metadata(&home, "visible-openclaw", Some("routed-openclaw"));
    let config_path = home.join(".openclaw/openclaw.json");
    std::fs::create_dir_all(config_path.parent().expect("OpenClaw config parent"))
        .expect("create OpenClaw config dir");
    std::fs::write(
        &config_path,
        "{\n  skills: { entries: { \"other-skill\": { enabled: true } } },\n}\n",
    )
    .expect("write OpenClaw JSON5 config");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    scan_all_to_catalog(&ctx, &catalog).expect("scan all");
    let openclaw_id = catalog
        .list_skill_records()
        .expect("records")
        .into_iter()
        .find(|record| record.agent == "openclaw" && record.name == "visible-openclaw")
        .expect("OpenClaw record")
        .id;

    let record =
        toggle_skill(&catalog, &ctx, &openclaw_id, false).expect("toggle OpenClaw off succeeds");
    assert!(!record.enabled);
    assert_eq!(record.state, "disabled");
    let content = std::fs::read_to_string(&config_path).expect("read OpenClaw config");
    assert!(content.contains("\"routed-openclaw\""));
    assert!(content.contains("\"enabled\": false"));

    scan_all_to_catalog(&ctx, &catalog).expect("rescan all");
    let rescanned = catalog
        .get_skill_record(&openclaw_id)
        .expect("catalog lookup")
        .expect("OpenClaw record remains");
    assert!(!rescanned.enabled);
    assert_eq!(rescanned.state, "disabled");

    let snapshots = catalog
        .list_config_snapshots("openclaw", &config_path.to_string_lossy())
        .expect("list OpenClaw snapshots");
    assert_eq!(snapshots.len(), 1);
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, &snapshots[0].id)
        .expect("OpenClaw rollback preview");
    rollback_snapshot(&catalog, &ctx, &snapshots[0].id, &preview.preview_token)
        .expect("OpenClaw rollback succeeds");
    let rolled_back = std::fs::read_to_string(&config_path).unwrap_or_default();
    assert!(!rolled_back.contains("routed-openclaw"));
    assert!(rolled_back.contains("other-skill"));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn batch_toggle_preview_applies_hermes_config_writes() {
    let temp_root = temp_test_dir("batch-toggle-hermes");
    let home = temp_root.join("home");
    write_hermes_global_skill(&home, "batch-hermes-only");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    scan_all_to_catalog(&ctx, &catalog).expect("scan all");
    let hermes_id = catalog
        .list_skill_records()
        .expect("records")
        .into_iter()
        .find(|record| record.agent == "hermes" && record.name == "batch-hermes-only")
        .expect("hermes record")
        .id;

    let selection = vec![hermes_id];
    let preview = preview_skill_toggles(&catalog, &ctx, &selection, false).expect("batch preview");
    assert_eq!(preview.writable_count, 1);
    assert_eq!(preview.skipped_count, 0);
    assert!(preview.writes_allowed);
    let apply = apply_skill_toggles(&catalog, &ctx, &selection, false, &preview.preview_token)
        .expect("Hermes batch apply succeeds");
    assert_eq!(apply.applied_count, 1);
    let content =
        std::fs::read_to_string(home.join(".hermes/config.yaml")).expect("read Hermes config");
    assert!(content.contains("batch-hermes-only"));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn toggle_codex_project_skill_writes_only_user_config_toml() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-codex-toggle-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let project = temp_root.join("project");
    let skill_dir = project.join(".agents/skills/proj");
    let project_config = project.join(".codex/config.toml");
    std::fs::create_dir_all(&skill_dir).expect("create codex skill dir");
    std::fs::create_dir_all(project_config.parent().expect("project config parent"))
        .expect("create project codex config dir");
    std::fs::create_dir_all(project.join(".git")).expect("create git marker");
    std::fs::write(&project_config, "# project config must remain untouched\n")
        .expect("write existing project config");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(
        &skill_path,
        "---\nname: proj\ndescription: Project Codex skill\n---\nbody",
    )
    .expect("write codex skill");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: Some(project.clone()),
        project_cwd: None,
        extra_roots: vec![],
    };
    scan_all_to_catalog(&ctx, &catalog).expect("scan all");
    let codex_record = catalog
        .list_skill_records()
        .expect("records")
        .into_iter()
        .find(|record| record.agent == "codex" && record.name == "proj")
        .expect("codex project record");

    let disabled = toggle_skill(&catalog, &ctx, &codex_record.id, false).expect("toggle codex off");
    assert!(!disabled.enabled);
    assert_eq!(disabled.state, "disabled");

    let user_config = home.join(".codex/config.toml");
    let content = std::fs::read_to_string(&user_config).expect("read codex config");
    assert!(content.contains("[[skills.config]]"));
    assert!(content.contains("enabled = false"));
    assert!(
        codex_config_contains_path(&content, &skill_path, false),
        "Codex toggle should write the absolute SKILL.md path"
    );
    assert_eq!(
        std::fs::read_to_string(&project_config).expect("read project config"),
        "# project config must remain untouched\n",
        "Codex toggle must not modify project .codex/config.toml"
    );

    let snapshots = catalog
        .list_config_snapshots("codex", &user_config.to_string_lossy())
        .expect("codex snapshots");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].scope, "agent-global");
    assert_eq!(snapshots[0].reason, "pre-toggle");

    let enabled = toggle_skill(&catalog, &ctx, &codex_record.id, true).expect("toggle codex on");
    assert!(enabled.enabled);
    let content = std::fs::read_to_string(&user_config).expect("read codex config");
    assert!(
        !codex_config_contains_path(&content, &skill_path, false),
        "re-enabling removes matching Codex config entries"
    );
    assert_eq!(
        std::fs::read_to_string(&project_config).expect("read project config"),
        "# project config must remain untouched\n",
        "Codex re-enable must not modify project .codex/config.toml"
    );

    let no_project_ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let stale_toggle = toggle_skill(&catalog, &no_project_ctx, &codex_record.id, false)
        .expect_err("stale project records must not be writable without project context");
    assert!(
        stale_toggle.to_string().contains("current project context"),
        "unexpected stale toggle error: {stale_toggle}"
    );

    let other_project = temp_root.join("other-project");
    std::fs::create_dir_all(&other_project).expect("create other project");
    let stale_mismatch_ctx = AdapterContext {
        user_home: home.clone(),
        project_root: Some(other_project),
        project_cwd: None,
        extra_roots: vec![],
    };
    let stale_mismatch = toggle_skill(&catalog, &stale_mismatch_ctx, &codex_record.id, false)
        .expect_err("stale project rows must not be writable from a different project context");
    assert!(
        stale_mismatch
            .to_string()
            .contains("current project context"),
        "unexpected stale mismatch toggle error: {stale_mismatch}"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn codex_config_path_honors_only_safe_codex_home_under_user_home() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-codex-home-boundary-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let safe_codex_home = home.join("custom-codex-home");
    let unsafe_codex_home = temp_root.join("outside-codex-home");
    let escaping_codex_home = home.join("../outside-codex-home");
    std::fs::create_dir_all(&home).expect("create home");

    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    assert_eq!(
        codex_user_config_path_for(&ctx, Some(&safe_codex_home)),
        safe_codex_home.join("config.toml"),
        "safe CODEX_HOME under user_home should be honored"
    );
    assert_eq!(
        codex_user_config_path_for(&ctx, Some(&unsafe_codex_home)),
        home.join(".codex/config.toml"),
        "unsafe CODEX_HOME outside user_home must fall back to user config"
    );
    assert_eq!(
        codex_user_config_path_for(&ctx, Some(&escaping_codex_home)),
        home.join(".codex/config.toml"),
        "CODEX_HOME path traversal must not escape user_home"
    );

    validate_config_write_target(
        &ctx,
        AgentId::Codex,
        Scope::AgentGlobal,
        &home.join(".codex/config.toml"),
    )
    .expect("fallback Codex config target validates");
    let unsafe_result = validate_config_write_target(
        &ctx,
        AgentId::Codex,
        Scope::AgentGlobal,
        &unsafe_codex_home.join("config.toml"),
    );
    assert!(
        matches!(unsafe_result, Err(CommandError::UnsafeConfigPath(_))),
        "unsafe CODEX_HOME target must not validate for writes"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn codex_compatibility_root_is_read_only_and_marketplace_is_not_scanned() {
    let temp_root = temp_test_dir("codex-expanded-roots-read-only");
    let home = temp_root.join("home");
    let native_path = write_codex_skill(&home, "native-toggle");

    let compat_dir = home.join(".codex/skills/compat-readonly");
    std::fs::create_dir_all(&compat_dir).expect("create compat codex skill dir");
    let compat_path = compat_dir.join("SKILL.md");
    std::fs::write(
        &compat_path,
        "---\nname: compat-readonly\ndescription: CODEX_HOME read-only skill\n---\nbody",
    )
    .expect("write compat codex skill");

    let plugin_root = home.join(".codex/plugins/local-review");
    let plugin_skill_dir = plugin_root.join("skills/plugin-readonly");
    std::fs::create_dir_all(&plugin_skill_dir).expect("create plugin codex skill dir");
    std::fs::create_dir_all(plugin_root.join(".codex-plugin"))
        .expect("create codex plugin manifest dir");
    std::fs::create_dir_all(home.join(".agents/plugins")).expect("create plugin marketplace dir");
    let plugin_path = plugin_skill_dir.join("SKILL.md");
    std::fs::write(
        &plugin_path,
        "---\nname: plugin-readonly\ndescription: Plugin read-only skill\n---\nbody",
    )
    .expect("write plugin codex skill");
    std::fs::write(
        plugin_root.join(".codex-plugin/plugin.json"),
        "{\n  \"name\": \"local-review\",\n  \"skills\": \"./skills/\"\n}\n",
    )
    .expect("write codex plugin manifest");
    std::fs::write(
        home.join(".agents/plugins/marketplace.json"),
        "{\n  \"plugins\": [\n    {\"source\": {\"source\": \"local\", \"path\": \"./.codex/plugins/local-review\"}}\n  ]\n}\n",
    )
    .expect("write codex plugin marketplace");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    scan_all_to_catalog(&ctx, &catalog).expect("scan all");
    let records = catalog.list_skill_records().expect("records");
    let native_record = records
        .iter()
        .find(|record| record.agent == "codex" && record.name == "native-toggle")
        .expect("native codex record");
    let compat_record = records
        .iter()
        .find(|record| record.agent == "codex" && record.name == "compat-readonly")
        .expect("compat codex record");
    assert!(records
        .iter()
        .all(|record| record.agent != "codex" || record.name != "plugin-readonly"));

    let selection = vec![native_record.id.clone(), compat_record.id.clone()];
    let preview = preview_skill_toggles(&catalog, &ctx, &selection, false).expect("preview");
    assert_eq!(preview.writable_count, 1);
    assert_eq!(preview.skipped_count, 1);
    assert!(preview
        .affected_items
        .iter()
        .any(|item| item.instance_id == native_record.id));
    assert!(preview
        .skipped_items
        .iter()
        .all(|item| item.reason.contains(".agents/skills")));

    let config_path = home.join(".codex/config.toml");
    let compat_toggle = toggle_skill(&catalog, &ctx, &compat_record.id, false)
        .expect_err("compat root must be read-only");
    assert!(
        compat_toggle.to_string().contains(".agents/skills"),
        "unexpected compat toggle error: {compat_toggle}"
    );
    assert!(
        !config_path.exists(),
        "read-only Codex roots must not create user config"
    );

    let disabled =
        toggle_skill(&catalog, &ctx, &native_record.id, false).expect("native toggle succeeds");
    assert!(!disabled.enabled);
    let content = std::fs::read_to_string(&config_path).expect("read codex config");
    assert!(codex_config_contains_path(&content, &native_path, false));
    assert!(!codex_config_contains_path(&content, &compat_path, false));
    assert!(!codex_config_contains_path(&content, &plugin_path, false));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn codex_diagnostics_include_user_and_project_config_paths() {
    let temp_root = temp_test_dir("codex-diagnostic-config-paths");
    let home = temp_root.join("home");
    let project = temp_root.join("project");
    let project_config = project.join(".codex/config.toml");
    std::fs::create_dir_all(project_config.parent().expect("project config parent"))
        .expect("create project config dir");
    std::fs::write(&project_config, "# project diagnostics only\n")
        .expect("write project codex config");
    std::fs::create_dir_all(&home).expect("create home");

    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: Some(project.clone()),
        project_cwd: Some(project.clone()),
        extra_roots: vec![],
    };
    let diagnostics = list_adapter_diagnostics(&ctx);
    let codex = diagnostics
        .iter()
        .find(|record| record.agent == "codex")
        .expect("codex diagnostics");

    assert!(codex
        .config
        .paths
        .iter()
        .any(|path| path.path == path_text(&home.join(".codex").join("config.toml"))));
    assert!(codex
        .config
        .paths
        .iter()
        .any(|path| path.path == path_text(&project_config) && path.detected));
    assert!(codex.blockers.iter().any(|blocker| {
        blocker.contains(".codex/config.toml") && blocker.contains("unverified")
    }));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn codex_rescan_reads_disabled_state_with_adapter_toml_semantics() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-codex-disabled-toml-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let alpha_path = write_codex_skill(&home, "alpha-disabled");
    let beta_path = write_codex_skill(&home, "beta-disabled");
    let config_path = home.join(".codex/config.toml");
    std::fs::create_dir_all(config_path.parent().expect("codex config parent"))
        .expect("create codex config dir");
    std::fs::write(
            &config_path,
            format!(
                "[[skills.config]]\npath = '{}' # literal string\nenabled = false # disabled\n\n[[skills.config]]\npath = \"{}\" # basic string\nenabled = false # disabled\n",
                path_text(&alpha_path),
                toml_basic_string(&path_text(&beta_path))
            ),
        )
        .expect("write codex config");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    scan_all_to_catalog(&ctx, &catalog).expect("scan all");
    let records = catalog.list_skill_records().expect("records");

    for name in ["alpha-disabled", "beta-disabled"] {
        let record = records
            .iter()
            .find(|record| record.agent == "codex" && record.name == name)
            .expect("codex record");
        assert_eq!(record.state, "disabled");
        assert!(!record.enabled);
    }

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rescan_preserves_disabled_state_from_skill_overrides() {
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-rescan-{}", std::process::id()));
    let home = temp_root.join("home");
    std::fs::create_dir_all(home.join(".claude/skills/foo")).expect("create skill dir");
    std::fs::write(
        home.join(".claude/skills/foo/SKILL.md"),
        "---\nname: foo\ndescription: x\n---\nbody",
    )
    .expect("write skill");
    let settings_path = home.join(".claude/settings.json");
    std::fs::write(&settings_path, "{}\n").expect("write initial settings");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    // First scan: parser default state=loaded.
    scan_claude_to_catalog(&ctx, &catalog).expect("first scan");
    let records = catalog
        .list_skill_records()
        .expect("records after first scan");
    assert_eq!(records.len(), 1);
    assert_eq!(records[0].state, "loaded");

    // Toggle off: settings.json now contains skillOverrides[foo] = "off".
    let inst_id = records[0].id.clone();
    toggle_skill(&catalog, &ctx, &inst_id, false).expect("toggle off");
    let content = std::fs::read_to_string(&settings_path).expect("read settings");
    assert!(content.contains("\"foo\""));
    assert!(content.contains("\"off\""));

    // Re-scan: scanner must read the override and keep the catalog at
    // state=disabled instead of reverting to state=loaded.
    scan_claude_to_catalog(&ctx, &catalog).expect("re-scan");
    let records = catalog.list_skill_records().expect("records after re-scan");
    assert_eq!(records.len(), 1);
    assert_eq!(
        records[0].state, "disabled",
        "re-scan must preserve the disabled state from skillOverrides"
    );
    assert!(!records[0].enabled);

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn scan_records_rule_findings_and_conflicts() {
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: fixture_path("fixtures/claude-code/empty-home"),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![AdapterRoot {
            scope: Scope::AgentGlobal,
            path: fixture_path("fixtures/claude-code/project"),
            source: RootSource::Extra,
        }],
    };

    scan_claude_to_catalog(&ctx, &catalog).expect("scan succeeds");

    let findings = catalog.list_rule_findings().expect("findings list");
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule_id == "frontmatter.required-fields"),
        "broken frontmatter fixtures produce required-field findings"
    );
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule_id == "name.collision"),
        "same-name fixtures produce collision findings"
    );

    let conflicts = catalog.list_conflict_groups().expect("conflicts list");
    assert!(
        conflicts
            .iter()
            .any(|conflict| conflict.reason == "content-drift"),
        "same-name fixtures with different content create a content-drift conflict"
    );
}

#[test]
fn scan_records_v2_8_local_content_rule_findings() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-v2-8-content-rules-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    write_codex_skill_file(
        &home,
        "tools-empty-array",
        "---\nname: tools-empty-array\ndescription: empty tools array\ntools: []\n---\nbody",
    );
    write_codex_skill_file(
        &home,
        "tools-blank-string",
        "---\nname: tools-blank-string\ndescription: blank tools string\ntools: \"   \"\n---\nbody",
    );
    write_codex_skill_file(
        &home,
        "bad-name",
        "---\nname: Bad_Name\ndescription: noncanonical name\n---\nbody",
    );
    write_codex_skill_file(
        &home,
        "long-body",
        &format!(
            "---\nname: long-body\ndescription: long body\n---\n{}",
            "x".repeat(BODY_TOO_LONG_CHAR_THRESHOLD + 1)
        ),
    );
    write_codex_skill_file(
        &home,
        "no-tools",
        "---\nname: no-tools\ndescription: missing tools is valid\n---\nbody",
    );
    write_codex_skill_file(
        &home,
        "has-tools",
        "---\nname: has-tools\ndescription: nonempty tools is valid\ntools:\n  - Read\n---\nbody",
    );

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    scan_all_to_catalog(&ctx, &catalog).expect("scan all succeeds");

    let records = catalog.list_skill_records().expect("records list");
    let findings = catalog.list_rule_findings().expect("findings list");
    assert_eq!(
        records.len(),
        24,
        "Codex, OpenClaw, opencode, and Pi scan the documented shared ~/.agents/skills root"
    );
    assert_eq!(
        findings
            .iter()
            .filter(|finding| finding.rule_id == "frontmatter.tools-not-empty")
            .count(),
        8,
        "empty array and blank string tools fields are reported for all shared-root agents"
    );
    assert!(
        has_rule_for_name(
            &records,
            &findings,
            "tools-empty-array",
            "frontmatter.tools-not-empty"
        ) && has_rule_for_name(
            &records,
            &findings,
            "tools-blank-string",
            "frontmatter.tools-not-empty"
        ),
        "both empty tools forms produce findings"
    );
    assert!(
        has_rule_for_name(&records, &findings, "Bad_Name", "name.canonical-case"),
        "noncanonical case is reported"
    );
    assert!(
        has_rule_for_name(&records, &findings, "long-body", "body.too-long"),
        "body over the local threshold is reported"
    );
    assert!(
        !has_rule_for_name(
            &records,
            &findings,
            "no-tools",
            "frontmatter.tools-not-empty"
        ),
        "missing tools field must not be reported"
    );
    assert!(
        !has_rule_for_name(
            &records,
            &findings,
            "has-tools",
            "frontmatter.tools-not-empty"
        ),
        "nonempty tools field must not be reported"
    );
    assert!(
        !has_rule_for_name(&records, &findings, "has-tools", "name.canonical-case"),
        "canonical lowercase slug must not be reported"
    );
    assert!(
        !has_rule_for_name(&records, &findings, "has-tools", "body.too-long"),
        "short body must not be reported"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn single_agent_scan_preserves_other_agent_findings_without_cross_agent_conflict() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-single-scan-rules-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let project = temp_root.join("project");
    let outside = temp_root.join("outside");
    std::fs::create_dir_all(&project).expect("create project");
    std::fs::create_dir_all(&outside).expect("create outside");
    write_claude_skill(&home, "shared-skill");
    write_codex_skill(&home, "shared-skill");
    write_opencode_global_skill(&home, "shared-skill");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: Some(project.clone()),
        project_cwd: None,
        extra_roots: vec![],
    };

    scan_all_to_catalog(&ctx, &catalog).expect("scan all");
    catalog
        .upsert_skill_instance(&synthetic_opencode_project_instance(
            "opencode:outside-workspace",
            &project,
            outside.join("opencode/SKILL.md"),
            "opencode-outside-workspace",
        ))
        .expect("upsert opencode outside-workspace record");
    let previous_fingerprints = catalog
        .instance_fingerprints()
        .expect("fingerprints before rule refresh");
    refresh_catalog_rule_outputs(&catalog, &ctx, previous_fingerprints)
        .expect("refresh rules after synthetic opencode finding");
    assert!(catalog
        .list_rule_findings()
        .expect("findings after scan all")
        .iter()
        .any(|finding| finding
            .instance_id
            .as_deref()
            .is_some_and(|id| id.starts_with("opencode:"))));

    scan_claude_to_catalog(&ctx, &catalog).expect("scan claude");

    let findings = catalog.list_rule_findings().expect("findings after claude");
    assert!(
        findings.iter().any(|finding| finding
            .instance_id
            .as_deref()
            .is_some_and(|id| id.starts_with("opencode:"))),
        "scanClaude must not drop opencode findings"
    );

    let conflicts = catalog.list_conflict_groups().expect("conflicts");
    let records = catalog.list_skill_records().expect("records");
    let codex_shared_id = records
        .iter()
        .find(|record| record.agent == "codex" && record.name == "shared-skill")
        .expect("codex shared record")
        .id
        .clone();
    let opencode_shared_id = records
        .iter()
        .find(|record| record.agent == "opencode" && record.name == "shared-skill")
        .expect("opencode shared record")
        .id
        .clone();
    assert!(
        conflicts.iter().all(|conflict| {
            !(conflict.instance_ids.contains(&codex_shared_id)
                && conflict.instance_ids.contains(&opencode_shared_id))
        }),
        "cross-agent duplicate names must not be runtime conflict groups"
    );
    let analysis = analyze_catalog(&catalog, &ctx).expect("analysis after scanClaude");
    assert!(
        analysis.groups.iter().any(|group| {
            group.kind == "duplicate_name"
                && group.instance_ids.contains(&codex_shared_id)
                && group.instance_ids.contains(&opencode_shared_id)
        }),
        "cross-agent duplicate names remain visible through analysis"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rescan_records_fingerprint_changed_finding() {
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-fingerprint-{}", std::process::id()));
    let home = temp_root.join("home");
    let skill_dir = home.join(".claude/skills/foo");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(&skill_path, "---\nname: foo\ndescription: x\n---\nbody v1")
        .expect("write initial skill");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    scan_claude_to_catalog(&ctx, &catalog).expect("first scan");
    std::fs::write(&skill_path, "---\nname: foo\ndescription: x\n---\nbody v2")
        .expect("edit skill");
    scan_claude_to_catalog(&ctx, &catalog).expect("second scan");

    let findings = catalog.list_rule_findings().expect("findings list");
    assert!(
        findings
            .iter()
            .any(|finding| finding.rule_id == "fingerprint.changed"),
        "fingerprint changes are reported after re-scan"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rollback_snapshot_restores_settings_and_rescans() {
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-rollback-{}", std::process::id()));
    let home = temp_root.join("home");
    let skill_dir = home.join(".claude/skills/foo");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: foo\ndescription: x\n---\nbody",
    )
    .expect("write skill");
    let settings_path = home.join(".claude/settings.json");
    std::fs::write(&settings_path, "{}\n").expect("write settings");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    scan_claude_to_catalog(&ctx, &catalog).expect("scan");
    let skill_id = catalog.list_skill_records().expect("records")[0].id.clone();
    toggle_skill(&catalog, &ctx, &skill_id, false).expect("toggle off");

    let snapshots = list_snapshots(&catalog, &ctx).expect("snapshots");
    assert_eq!(snapshots.len(), 1);
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, &snapshots[0].id)
        .expect("rollback preview");
    assert_eq!(preview.snapshot.content, "{}\n");
    assert!(
        preview.current_content.contains("skillOverrides"),
        "preview reads the current config before rollback"
    );
    assert!(preview.changed, "preview detects changed content");
    assert!(!preview.redacted);
    assert!(preview.rollback_supported);
    rollback_snapshot(&catalog, &ctx, &snapshots[0].id, &preview.preview_token).expect("rollback");

    let settings = std::fs::read_to_string(&settings_path).expect("settings");
    assert_eq!(settings, "{}\n");
    let records = catalog
        .list_skill_records()
        .expect("records after rollback");
    assert!(records[0].enabled);
    assert_eq!(records[0].state, "loaded");

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn project_snapshots_are_hidden_and_not_previewable_across_projects() {
    let temp_root = temp_test_dir("project-snapshot-isolation");
    let home = temp_root.join("home");
    let project_a = temp_root.join("project-a");
    let project_b = temp_root.join("project-b");
    let target_a = project_a.join(".claude/settings.local.json");
    let target_b = project_b.join(".claude/settings.local.json");
    std::fs::create_dir_all(target_a.parent().expect("project A config parent"))
        .expect("create project A config");
    std::fs::create_dir_all(target_b.parent().expect("project B config parent"))
        .expect("create project B config");
    std::fs::write(&target_a, "{}\n").expect("write project A config");
    std::fs::write(&target_b, "{}\n").expect("write project B config");
    let canonical_a = project_a.canonicalize().expect("canonical project A");
    let canonical_b = project_b.canonicalize().expect("canonical project B");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    for (id, project_root, target) in [
        ("project-a-snapshot", &canonical_a, &target_a),
        ("project-b-snapshot", &canonical_b, &target_b),
    ] {
        let project_root_text = project_root.to_string_lossy();
        catalog
            .create_config_snapshot(ConfigSnapshotDraft {
                id,
                agent: "claude-code",
                scope: "agent-project",
                project_root: Some(&project_root_text),
                target: &target.to_string_lossy(),
                content: "{}\n",
                reason: "pre-config-edit",
                created_at_ms: current_time_ms(),
            })
            .expect("create project snapshot");
    }
    let ctx_a = AdapterContext {
        user_home: home,
        project_root: Some(project_a),
        project_cwd: None,
        extra_roots: vec![],
    };

    let visible = list_snapshots(&catalog, &ctx_a).expect("list project A snapshots");
    assert_eq!(visible.len(), 1);
    assert_eq!(visible[0].id, "project-a-snapshot");
    let cross_project =
        preview_snapshot_rollback_with_context(&catalog, &ctx_a, "project-b-snapshot");
    assert!(matches!(
        cross_project,
        Err(CommandError::UnsafeConfigPath(_))
    ));

    let _ = std::fs::remove_dir_all(temp_root);
}

#[test]
fn stale_rollback_preview_token_rejects_external_change_without_writes() {
    let temp_root = temp_test_dir("stale-rollback-preview");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::write(&settings_path, "{\n  \"current\": true\n}\n").expect("write current settings");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "stale-preview-snapshot",
            agent: ClaudeCodeAdapter.id().as_str(),
            scope: Scope::AgentGlobal.as_str(),
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: current_time_ms(),
        })
        .expect("create snapshot");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, "stale-preview-snapshot")
        .expect("preview rollback");
    assert!(preview.current_revision.starts_with("sha256:"));
    assert!(preview.preview_token.starts_with("sha256:"));
    let external_content = "{\n  \"external\": true\n}\n";
    std::fs::write(&settings_path, external_content).expect("write external change");
    let snapshots_before = catalog
        .list_all_config_snapshots(None)
        .expect("list snapshots before");

    let result = rollback_snapshot(
        &catalog,
        &ctx,
        "stale-preview-snapshot",
        &preview.preview_token,
    );

    assert!(matches!(result, Err(CommandError::StalePreviewToken)));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read preserved external content"),
        external_content
    );
    assert_eq!(
        catalog
            .list_all_config_snapshots(None)
            .expect("list snapshots after"),
        snapshots_before
    );
    assert!(
        std::fs::read_dir(settings_path.parent().expect("settings parent"))
            .expect("list settings directory")
            .all(|entry| !entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")),
        "stale token rejection must not create a temporary config file"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rollback_deleted_before_invocation_returns_stale_without_writes() {
    use rusqlite::Connection;

    let temp_root = temp_test_dir("rollback-deleted-before-call");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    let current_content = "{\n  \"current\": true\n}\n";
    std::fs::write(&settings_path, current_content).expect("write current settings");
    let catalog_path = temp_root.join("catalog.sqlite");
    let catalog = Catalog::open(&catalog_path).expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "deleted-before-call",
            agent: ClaudeCodeAdapter.id().as_str(),
            scope: Scope::AgentGlobal.as_str(),
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: current_time_ms(),
        })
        .expect("create snapshot");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, "deleted-before-call")
        .expect("preview rollback");
    Connection::open(&catalog_path)
        .expect("open raw catalog")
        .execute(
            "DELETE FROM config_snapshot WHERE id = ?1",
            ["deleted-before-call"],
        )
        .expect("delete snapshot after preview");

    let result = rollback_snapshot(
        &catalog,
        &ctx,
        "deleted-before-call",
        &preview.preview_token,
    );

    assert!(matches!(result, Err(CommandError::StalePreviewToken)));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read unchanged target"),
        current_content
    );
    assert!(!settings_path.with_extension("lock").exists());
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rollback_preview_to_call_snapshot_identity_drift_returns_stale_without_writes() {
    use rusqlite::{params, Connection};

    for changed_field in ["target", "scope", "agent"] {
        let temp_root = temp_test_dir(&format!("rollback-before-call-{changed_field}"));
        let home = temp_root.join("home");
        let settings_path = home.join(".claude/settings.json");
        let outside_target = temp_root.join("outside/unsafe-settings.json");
        std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
            .expect("create settings directory");
        std::fs::create_dir_all(outside_target.parent().expect("outside parent"))
            .expect("create outside directory");
        let current_content = "{\n  \"current\": true\n}\n";
        let outside_content = "do not read or write\n";
        std::fs::write(&settings_path, current_content).expect("write current settings");
        std::fs::write(&outside_target, outside_content).expect("write outside sentinel");
        let catalog_path = temp_root.join("catalog.sqlite");
        let catalog = Catalog::open(&catalog_path).expect("catalog opens");
        catalog.init().expect("catalog initializes");
        catalog
            .create_config_snapshot(ConfigSnapshotDraft {
                id: "identity-drift-before-call",
                agent: ClaudeCodeAdapter.id().as_str(),
                scope: Scope::AgentGlobal.as_str(),
                project_root: None,
                target: &settings_path.to_string_lossy(),
                content: "{}\n",
                reason: "pre-config-edit",
                created_at_ms: current_time_ms(),
            })
            .expect("create snapshot");
        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };
        let preview =
            preview_snapshot_rollback_with_context(&catalog, &ctx, "identity-drift-before-call")
                .expect("preview rollback");
        let replacement = match changed_field {
            "target" => outside_target.to_string_lossy().to_string(),
            "scope" => "tool-global".to_string(),
            "agent" => "tool-global".to_string(),
            _ => unreachable!(),
        };
        let connection = Connection::open(&catalog_path).expect("open raw catalog");
        let sql = format!("UPDATE config_snapshot SET {changed_field} = ?1 WHERE id = ?2");
        connection
            .execute(&sql, params![replacement, "identity-drift-before-call"])
            .expect("drift snapshot identity");
        drop(connection);

        let result = rollback_snapshot(
            &catalog,
            &ctx,
            "identity-drift-before-call",
            &preview.preview_token,
        );

        assert!(
            matches!(result, Err(CommandError::StalePreviewToken)),
            "{changed_field} drift returned {result:?}"
        );
        assert_eq!(
            std::fs::read_to_string(&settings_path).expect("read unchanged target"),
            current_content
        );
        assert_eq!(
            std::fs::read_to_string(&outside_target).expect("read outside sentinel"),
            outside_content
        );
        assert!(!settings_path.with_extension("lock").exists());
        assert!(!outside_target.with_extension("lock").exists());

        drop(catalog);
        let _ = std::fs::remove_dir_all(&temp_root);
    }
}

#[test]
fn rollback_rechecks_state_after_lock() {
    let temp_root = temp_test_dir("rollback-lock-recheck");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::write(&settings_path, "{\n  \"current\": true\n}\n").expect("write current settings");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "lock-recheck-snapshot",
            agent: ClaudeCodeAdapter.id().as_str(),
            scope: Scope::AgentGlobal.as_str(),
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: current_time_ms(),
        })
        .expect("create snapshot");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, "lock-recheck-snapshot")
        .expect("preview rollback");
    let external_content = "{\n  \"changedAfterLock\": true\n}\n";
    let snapshots_before = catalog
        .list_all_config_snapshots(None)
        .expect("list snapshots before");

    let result = rollback_snapshot_with_after_lock(
        &catalog,
        &ctx,
        "lock-recheck-snapshot",
        &preview.preview_token,
        || {
            std::fs::write(&settings_path, external_content)
                .expect("write concurrent external change");
        },
    );

    assert!(matches!(result, Err(CommandError::StalePreviewToken)));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read preserved external content"),
        external_content
    );
    assert_eq!(
        catalog
            .list_all_config_snapshots(None)
            .expect("list snapshots after"),
        snapshots_before
    );
    assert!(
        std::fs::read_dir(settings_path.parent().expect("settings parent"))
            .expect("list settings directory")
            .all(|entry| !entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")),
        "lock-time rejection must not create a temporary config file"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rollback_maps_unreadable_target_shape_after_lock_to_stale() {
    let temp_root = temp_test_dir("rollback-directory-after-lock");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::write(&settings_path, "{\n  \"current\": true\n}\n").expect("write current settings");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "directory-after-lock",
            agent: ClaudeCodeAdapter.id().as_str(),
            scope: Scope::AgentGlobal.as_str(),
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: current_time_ms(),
        })
        .expect("create snapshot");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, "directory-after-lock")
        .expect("preview rollback");

    let result = rollback_snapshot_with_after_lock(
        &catalog,
        &ctx,
        "directory-after-lock",
        &preview.preview_token,
        || {
            std::fs::remove_file(&settings_path).expect("remove target after lock");
            std::fs::create_dir(&settings_path).expect("replace target with directory");
        },
    );

    assert!(matches!(result, Err(CommandError::StalePreviewToken)));
    assert!(settings_path.is_dir());
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
#[cfg(unix)]
fn rollback_revalidates_symlinked_target_after_lock_before_reading_it() {
    let temp_root = temp_test_dir("rollback-symlink-after-lock");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    let outside_target = temp_root.join("outside/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::create_dir_all(outside_target.parent().expect("outside parent"))
        .expect("create outside directory");
    let current_content = "{\n  \"current\": true\n}\n";
    std::fs::write(&settings_path, current_content).expect("write current settings");
    std::fs::write(&outside_target, current_content).expect("write identical outside sentinel");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "symlink-after-lock",
            agent: ClaudeCodeAdapter.id().as_str(),
            scope: Scope::AgentGlobal.as_str(),
            project_root: None,
            target: &settings_path.to_string_lossy(),
            content: "{}\n",
            reason: "pre-config-edit",
            created_at_ms: current_time_ms(),
        })
        .expect("create snapshot");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, "symlink-after-lock")
        .expect("preview rollback");

    let result = rollback_snapshot_with_after_lock(
        &catalog,
        &ctx,
        "symlink-after-lock",
        &preview.preview_token,
        || {
            std::fs::remove_file(&settings_path).expect("remove target after lock");
            std::os::unix::fs::symlink(&outside_target, &settings_path)
                .expect("replace target with symlink");
        },
    );

    assert!(matches!(result, Err(CommandError::StalePreviewToken)));
    assert_eq!(
        std::fs::read_to_string(&outside_target).expect("read outside sentinel"),
        current_content
    );
    assert!(std::fs::symlink_metadata(&settings_path)
        .expect("target metadata")
        .file_type()
        .is_symlink());
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rollback_reloaded_snapshot_target_or_content_changes_invalidate_token() {
    use rusqlite::{params, Connection};

    for changed_field in ["content", "target"] {
        let temp_root = temp_test_dir(&format!("rollback-reloaded-{changed_field}"));
        let home = temp_root.join("home");
        let settings_path = home.join(".claude/settings.json");
        std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
            .expect("create settings directory");
        let current_content = "{\n  \"current\": true\n}\n";
        std::fs::write(&settings_path, current_content).expect("write current settings");
        let catalog_path = temp_root.join("catalog.sqlite");
        let catalog = Catalog::open(&catalog_path).expect("catalog opens");
        catalog.init().expect("catalog initializes");
        catalog
            .create_config_snapshot(ConfigSnapshotDraft {
                id: "reloaded-snapshot",
                agent: ClaudeCodeAdapter.id().as_str(),
                scope: Scope::AgentGlobal.as_str(),
                project_root: None,
                target: &settings_path.to_string_lossy(),
                content: "{}\n",
                reason: "pre-config-edit",
                created_at_ms: current_time_ms(),
            })
            .expect("create snapshot");
        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };
        let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, "reloaded-snapshot")
            .expect("preview rollback");
        let replacement = if changed_field == "content" {
            "{\"catalogChanged\":true}\n".to_string()
        } else {
            temp_root
                .join("other-settings.json")
                .to_string_lossy()
                .to_string()
        };

        let result = rollback_snapshot_with_after_lock(
            &catalog,
            &ctx,
            "reloaded-snapshot",
            &preview.preview_token,
            || {
                let connection = Connection::open(&catalog_path).expect("open raw catalog");
                let sql = format!("UPDATE config_snapshot SET {changed_field} = ?1 WHERE id = ?2");
                connection
                    .execute(&sql, params![replacement, "reloaded-snapshot"])
                    .expect("replace snapshot field");
            },
        );

        assert!(matches!(result, Err(CommandError::StalePreviewToken)));
        assert_eq!(
            std::fs::read_to_string(&settings_path).expect("read unchanged target"),
            current_content,
            "changed snapshot {changed_field} must be rejected before a target write"
        );
        if changed_field == "target" {
            assert!(
                !temp_root.join("other-settings.json").exists(),
                "the reloaded replacement target must not be written"
            );
        }
        assert!(
            std::fs::read_dir(settings_path.parent().expect("settings parent"))
                .expect("list settings directory")
                .all(|entry| !entry
                    .expect("directory entry")
                    .file_name()
                    .to_string_lossy()
                    .ends_with(".tmp")),
            "changed snapshot {changed_field} must not create a temporary config file"
        );

        drop(catalog);
        let _ = std::fs::remove_dir_all(&temp_root);
    }
}

#[test]
fn read_claude_settings_returns_default_for_missing_file() {
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-read-config-{}", std::process::id()));
    let ctx = AdapterContext {
        user_home: temp_root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let doc = read_claude_settings(&ctx).expect("read missing settings");

    assert_eq!(doc.agent, "claude-code");
    assert_eq!(doc.scope, "agent-global");
    assert_eq!(doc.content, "{}\n");
    assert!(!doc.exists);

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn read_agent_config_returns_pi_documents_without_creating_missing_project_config_dir() {
    let temp_root = temp_test_dir("read-agent-config-pi");
    let home = temp_root.join("home");
    let project = temp_root.join("project");
    let global_settings = home.join(".pi/agent/settings.json");
    std::fs::create_dir_all(global_settings.parent().expect("Pi settings parent"))
        .expect("create Pi settings parent");
    std::fs::create_dir_all(&project).expect("create project");
    std::fs::write(
        &global_settings,
        "{\"skills\":{\"disabled\":[\"remote-review\"]}}\n",
    )
    .expect("write Pi settings");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: Some(project.clone()),
        project_cwd: Some(project.clone()),
        extra_roots: Vec::new(),
    };

    let documents = read_agent_config(&ctx, "pi", None).expect("read Pi config documents");

    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0].agent, "pi");
    assert_eq!(documents[0].scope, "agent-global");
    assert!(documents[0].exists);
    assert!(documents[0].content.contains("remote-review"));
    assert_eq!(documents[1].scope, "agent-project");
    assert!(!documents[1].exists);
    assert_eq!(documents[1].content, "{\"skills\":[]}\n");
    assert!(
        !project.join(".pi").exists(),
        "read-only config preview must not create missing config directories"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn read_agent_config_returns_codex_user_and_project_documents_without_enabling_project_writes() {
    let temp_root = temp_test_dir("read-agent-config-codex");
    let home = temp_root.join("home");
    let project = temp_root.join("project");
    let user_config = home.join(".codex/config.toml");
    let project_config = project.join(".codex/config.toml");
    std::fs::create_dir_all(user_config.parent().expect("Codex user config parent"))
        .expect("create Codex user config dir");
    std::fs::create_dir_all(
        project_config
            .parent()
            .expect("Codex project config parent"),
    )
    .expect("create Codex project config dir");
    std::fs::write(&user_config, "model = \"gpt-5\"\n").expect("write Codex user config");
    std::fs::write(&project_config, "approval_policy = \"never\"\n")
        .expect("write Codex project config");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: Some(project.clone()),
        project_cwd: Some(project.clone()),
        extra_roots: Vec::new(),
    };

    let documents = read_agent_config(&ctx, "codex", None).expect("read Codex config documents");

    assert_eq!(documents.len(), 2);
    assert_eq!(documents[0].agent, "codex");
    assert_eq!(documents[0].scope, "agent-global");
    assert_eq!(documents[0].format, "toml");
    assert!(documents[0].exists);
    assert!(documents[0].content.contains("gpt-5"));
    assert_eq!(documents[1].scope, "agent-project");
    assert_eq!(
        documents[1].target,
        project_config.to_string_lossy().to_string()
    );
    assert_eq!(documents[1].format, "toml");
    assert!(documents[1].exists);
    assert!(documents[1].content.contains("approval_policy"));

    let project_only = read_agent_config(&ctx, "codex", Some("agent-project"))
        .expect("read Codex project config document");
    assert_eq!(project_only.len(), 1);
    assert_eq!(
        project_only[0].target,
        project_config.to_string_lossy().to_string()
    );
    assert!(
        expected_config_target(&ctx, AgentId::Codex, Scope::AgentProject).is_err(),
        "Codex project config remains read-only for write targets"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn stale_claude_settings_save_is_rejected_without_snapshot_or_write() {
    let temp_root = temp_test_dir("stale-claude-settings-save");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::write(&settings_path, "{}\n").expect("write initial settings");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let read = read_claude_settings(&ctx).expect("read initial settings");
    let external_content = "{\n  \"externallyChanged\": true\n}\n";
    std::fs::write(&settings_path, external_content).expect("write external change");

    let result = save_claude_settings(
        &catalog,
        &ctx,
        "{\n  \"requested\": true\n}\n",
        &read.revision,
    );

    assert!(matches!(result, Err(CommandError::ConfigConflict { .. })));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read preserved external content"),
        external_content
    );
    assert!(
        catalog
            .list_all_config_snapshots(None)
            .expect("list snapshots")
            .is_empty(),
        "stale rejection must happen before snapshot creation"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn save_rechecks_state_after_preflight_under_lock() {
    let temp_root = temp_test_dir("save-preflight-lock-race");
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings directory");
    std::fs::write(&settings_path, "{}\n").expect("write initial settings");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    let revision = read_claude_settings(&ctx)
        .expect("read initial settings")
        .revision;
    let external_content = "{\n  \"changedAfterPreflight\": true\n}\n";

    let result = prepare_claude_settings_save_with_before_lock(
        &ctx,
        "{\n  \"requested\": true\n}\n",
        &revision,
        || {
            std::fs::write(&settings_path, external_content)
                .expect("write concurrent external change");
        },
    );

    assert!(matches!(result, Err(CommandError::ConfigConflict { .. })));
    assert_eq!(
        std::fs::read_to_string(&settings_path).expect("read preserved external content"),
        external_content
    );
    assert!(
        std::fs::read_dir(settings_path.parent().expect("settings parent"))
            .expect("list settings directory")
            .all(|entry| !entry
                .expect("directory entry")
                .file_name()
                .to_string_lossy()
                .ends_with(".tmp")),
        "lock-time conflict must not prepare a temporary config file"
    );
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
#[cfg(unix)]
fn read_claude_settings_rejects_symlinked_config_directory() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-read-symlink-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let outside = temp_root.join("outside");
    std::fs::create_dir_all(&home).expect("create home");
    std::fs::create_dir_all(&outside).expect("create outside dir");
    std::os::unix::fs::symlink(&outside, home.join(".claude")).expect("create config dir symlink");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let result = read_claude_settings(&ctx);

    assert!(
        matches!(result, Err(CommandError::UnsafeConfigPath(_))),
        "read must reject the same symlinked target shape as writes"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn save_claude_settings_snapshots_validates_and_rescans() {
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-save-config-{}", std::process::id()));
    let home = temp_root.join("home");
    let skill_dir = home.join(".claude/skills/config-editor");
    std::fs::create_dir_all(&skill_dir).expect("create skill dir");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        "---\nname: config-editor\ndescription: config editor fixture\n---\nbody",
    )
    .expect("write skill");
    let settings_path = home.join(".claude/settings.json");
    std::fs::write(&settings_path, "{}\n").expect("write initial settings");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };
    scan_claude_to_catalog(&ctx, &catalog).expect("initial scan");

    let initial_revision = read_claude_settings(&ctx)
        .expect("read initial settings")
        .revision;
    let invalid = save_claude_settings(&catalog, &ctx, "{ broken", &initial_revision);
    assert!(matches!(invalid, Err(CommandError::InvalidJson(_))));

    let updated = save_claude_settings(
        &catalog,
        &ctx,
        "{\n  \"skillOverrides\": {\n    \"config-editor\": \"off\"\n  }\n}\n",
        &initial_revision,
    )
    .expect("save config");

    assert!(updated.exists);
    assert!(updated.content.contains("skillOverrides"));
    let snapshots = catalog
        .list_config_snapshots("claude-code", &settings_path.to_string_lossy())
        .expect("snapshots");
    assert_eq!(snapshots.len(), 1);
    assert_eq!(snapshots[0].reason, "pre-config-edit");
    assert_eq!(snapshots[0].content, "{}\n");

    let records = catalog.list_skill_records().expect("records");
    assert_eq!(records.len(), 1);
    assert!(!records[0].enabled);
    assert_eq!(records[0].state, "disabled");

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn install_preview_from_tool_global_does_not_write_disk() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-install-preview-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&temp_root, "portable-alpha");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-alpha",
            source_path.clone(),
            "portable-alpha",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let preview = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-alpha",
        AgentId::Codex,
        Scope::AgentGlobal,
        None,
        false,
    )
    .expect("preview install");

    assert!(!preview.wrote);
    assert_path_text_eq(&preview.source_path, &source_path);
    assert_eq!(
        preview.target_path,
        path_text(
            &home
                .join(".agents")
                .join("skills")
                .join("portable-alpha")
                .join("SKILL.md")
        )
    );
    assert!(
        !home.join(".agents").exists(),
        "preview must not create target dirs"
    );
    assert!(
        catalog
            .list_all_config_snapshots(None)
            .expect("snapshots")
            .is_empty(),
        "preview must not create audit snapshots"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn confirmed_install_writes_target_verified_path_without_config_snapshot() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-install-confirmed-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&temp_root, "portable-beta");
    let source_content = std::fs::read_to_string(&source_path).expect("source content");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-beta",
            source_path,
            "portable-beta",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let result = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-beta",
        AgentId::ClaudeCode,
        Scope::AgentGlobal,
        None,
        true,
    )
    .expect("confirmed install");

    let target = home
        .join(".claude")
        .join("skills")
        .join("portable-beta")
        .join("SKILL.md");
    assert!(result.wrote);
    assert_path_text_eq(&result.target_path, &target);
    assert_eq!(
        std::fs::read_to_string(&target).expect("target content"),
        source_content
    );
    let snapshots = catalog
        .list_config_snapshots("claude-code", &target.to_string_lossy())
        .expect("snapshots");
    assert!(
        snapshots.is_empty(),
        "direct skill-file installs must not create agent config snapshots"
    );
    assert!(catalog
        .list_skill_records()
        .expect("records")
        .iter()
        .any(|record| record.agent == "claude-code" && record.name == "portable-beta"));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn install_to_opencode_writes_native_user_skill_root() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-install-opencode-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&temp_root, "portable-gamma");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-gamma",
            source_path,
            "portable-gamma",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let result = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-gamma",
        AgentId::Opencode,
        Scope::AgentGlobal,
        None,
        true,
    )
    .expect("opencode install succeeds");

    let target = home
        .join(".config")
        .join("opencode")
        .join("skills")
        .join("portable-gamma")
        .join("SKILL.md");
    assert!(result.wrote);
    assert_path_text_eq(&result.target_path, &target);
    assert!(target.exists());
    assert!(catalog
        .list_skill_records()
        .expect("records")
        .iter()
        .any(|record| record.agent == "opencode" && record.name == "portable-gamma"));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn install_to_pi_writes_native_user_skill_root() {
    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-install-pi-{}", std::process::id()));
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&temp_root, "portable-pi");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-pi",
            source_path,
            "portable-pi",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let result = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-pi",
        AgentId::Pi,
        Scope::AgentGlobal,
        None,
        true,
    )
    .expect("Pi install succeeds");

    let target = home
        .join(".pi")
        .join("agent")
        .join("skills")
        .join("portable-pi")
        .join("SKILL.md");
    assert!(result.wrote);
    assert_path_text_eq(&result.target_path, &target);
    assert!(target.exists());
    assert!(catalog
        .list_skill_records()
        .expect("records")
        .iter()
        .any(|record| record.agent == "pi" && record.name == "portable-pi"));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn install_to_hermes_writes_native_user_skill_root() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-install-hermes-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&temp_root, "portable-hermes");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-hermes",
            source_path,
            "portable-hermes",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let result = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-hermes",
        AgentId::Hermes,
        Scope::AgentGlobal,
        None,
        true,
    )
    .expect("Hermes install succeeds");

    let target = home
        .join(".hermes")
        .join("skills")
        .join("portable-hermes")
        .join("SKILL.md");
    assert!(result.wrote);
    assert_path_text_eq(&result.target_path, &target);
    assert!(target.exists());
    assert!(result
        .risks
        .iter()
        .any(|risk| risk.contains("hub, URL, tap, update, uninstall")));
    assert!(catalog
        .list_skill_records()
        .expect("records")
        .iter()
        .any(|record| record.agent == "hermes" && record.name == "portable-hermes"));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn install_to_hermes_project_scope_remains_blocked() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-install-hermes-project-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let project = temp_root.join("project");
    std::fs::create_dir_all(&home).expect("create home");
    std::fs::create_dir_all(&project).expect("create project");
    let source_path = write_tool_global_skill(&temp_root, "project-hermes");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-hermes-project",
            source_path,
            "project-hermes",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home,
        project_root: Some(project.clone()),
        project_cwd: Some(project.clone()),
        extra_roots: vec![],
    };

    let err = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-hermes-project",
        AgentId::Hermes,
        Scope::AgentProject,
        Some(&project),
        false,
    )
    .expect_err("Hermes project install remains blocked");

    assert!(matches!(err, CommandError::InstallUnsupported(_)));
    assert!(!project
        .join(".hermes/skills/project-hermes/SKILL.md")
        .exists());

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn install_to_openclaw_writes_native_user_skill_root() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-install-openclaw-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    std::fs::create_dir_all(&home).expect("create home");
    let source_path = write_tool_global_skill(&temp_root, "portable-openclaw");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-openclaw",
            source_path,
            "portable-openclaw",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let result = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-openclaw",
        AgentId::Openclaw,
        Scope::AgentGlobal,
        None,
        true,
    )
    .expect("OpenClaw install succeeds");

    let target = home
        .join(".openclaw")
        .join("skills")
        .join("portable-openclaw")
        .join("SKILL.md");
    assert!(result.wrote);
    assert_path_text_eq(&result.target_path, &target);
    assert!(target.exists());
    assert!(result
        .risks
        .iter()
        .any(|risk| risk.contains("ClawHub, Git, update, verify, workshop")));
    assert!(catalog
        .list_skill_records()
        .expect("records")
        .iter()
        .any(|record| record.agent == "openclaw" && record.name == "portable-openclaw"));

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn install_to_openclaw_writes_confirmed_workspace_skill_root() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-install-openclaw-workspace-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let workspace = home.join(".openclaw/workspace");
    let repo = workspace.join("repo");
    let nested = repo.join("nested");
    std::fs::create_dir_all(&nested).expect("create workspace repo");
    let source_path = write_tool_global_skill(&temp_root, "workspace-openclaw");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-openclaw-workspace",
            source_path,
            "workspace-openclaw",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: Some(repo.clone()),
        project_cwd: Some(nested),
        extra_roots: vec![],
    };

    let result = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-openclaw-workspace",
        AgentId::Openclaw,
        Scope::AgentProject,
        Some(&repo),
        true,
    )
    .expect("OpenClaw workspace install succeeds");

    let target = workspace.join("skills/workspace-openclaw/SKILL.md");
    assert!(result.wrote);
    assert!(target.exists());
    assert_eq!(
        result.target_path,
        target
            .canonicalize()
            .expect("canonical target")
            .to_string_lossy()
    );
    assert!(catalog
        .list_skill_records()
        .expect("records")
        .iter()
        .any(|record| record.agent == "openclaw" && record.name == "workspace-openclaw"));
    assert!(!workspace
        .join(".agents/skills/workspace-openclaw/SKILL.md")
        .exists());

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn install_to_openclaw_project_scope_outside_workspace_is_rejected() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-install-openclaw-outside-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let project = temp_root.join("project");
    std::fs::create_dir_all(&home).expect("create home");
    std::fs::create_dir_all(&project).expect("create project");
    let source_path = write_tool_global_skill(&temp_root, "outside-openclaw");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-openclaw-outside",
            source_path,
            "outside-openclaw",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home,
        project_root: Some(project.clone()),
        project_cwd: Some(project.clone()),
        extra_roots: vec![],
    };

    let err = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-openclaw-outside",
        AgentId::Openclaw,
        Scope::AgentProject,
        Some(&project),
        false,
    )
    .expect_err("OpenClaw project install outside workspace must be rejected");

    assert!(err.to_string().contains("confirmed OpenClaw workspace"));
    assert!(!project.join("skills/outside-openclaw/SKILL.md").exists());

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn pi_v294_capability_matrix_exposes_native_install_and_compatibility_toggles() {
    let ctx = AdapterContext {
        user_home: PathBuf::from("/tmp/home"),
        project_root: Some(PathBuf::from("/tmp/project")),
        project_cwd: Some(PathBuf::from("/tmp/project")),
        extra_roots: vec![],
    };
    let pi = list_adapter_capabilities(&ctx)
        .into_iter()
        .find(|record| record.agent == AgentId::Pi.as_str())
        .expect("Pi capability record");

    assert_eq!(pi.status, "guarded");
    assert!(pi.project_scan.supported);
    assert_eq!(pi.project_scan.status, "verified-compatibility-roots");
    assert!(pi.config_toggle.supported);
    assert_eq!(pi.config_toggle.status, "guarded-v2.94");
    assert!(pi.install.supported);
    assert_eq!(pi.install.status, "verified-native-roots");
    assert!(pi.writable.supported);
    assert_eq!(pi.writable.status, "guarded-v2.94");
    assert!(pi
        .blockers
        .iter()
        .any(|blocker| blocker.contains("package install/remove")));
    assert!(pi
        .blockers
        .iter()
        .any(|blocker| blocker.contains(".agents compatibility roots")));
}

#[test]
fn hermes_v297_capability_matrix_exposes_guarded_config_toggle() {
    let ctx = AdapterContext {
        user_home: PathBuf::from("/tmp/home"),
        project_root: Some(PathBuf::from("/tmp/project")),
        project_cwd: Some(PathBuf::from("/tmp/project")),
        extra_roots: vec![],
    };
    let hermes = list_adapter_capabilities(&ctx)
        .into_iter()
        .find(|record| record.agent == AgentId::Hermes.as_str())
        .expect("Hermes capability record");

    assert_eq!(hermes.status, "guarded");
    assert!(hermes.scan.supported);
    assert!(!hermes.project_scan.supported);
    assert!(hermes.config_toggle.supported);
    assert_eq!(
        hermes.config_toggle.status,
        "verified-skills-disabled-v2.97"
    );
    assert!(hermes.config_snapshot.supported);
    assert_eq!(hermes.config_snapshot.status, "verified-v2.97");
    assert!(hermes.install.supported);
    assert_eq!(hermes.install.status, "verified-native-root-v2.95");
    assert!(hermes.writable.supported);
    assert_eq!(hermes.writable.status, "guarded-v2.97");
    assert!(hermes
        .blockers
        .iter()
        .any(|blocker| blocker.contains("external_dirs")));
    assert!(hermes
        .blockers
        .iter()
        .any(|blocker| blocker.contains("hub, URL, tap")));
}

#[test]
fn openclaw_v297_capability_matrix_exposes_guarded_config_toggle() {
    let ctx = AdapterContext {
        user_home: PathBuf::from("/tmp/home"),
        project_root: Some(PathBuf::from("/tmp/home/.openclaw/workspace/repo")),
        project_cwd: Some(PathBuf::from("/tmp/home/.openclaw/workspace/repo")),
        extra_roots: vec![],
    };
    let openclaw = list_adapter_capabilities(&ctx)
        .into_iter()
        .find(|record| record.agent == AgentId::Openclaw.as_str())
        .expect("OpenClaw capability record");

    assert_eq!(openclaw.status, "guarded");
    assert!(openclaw.scan.supported);
    assert!(openclaw.project_scan.supported);
    assert!(openclaw.config_toggle.supported);
    assert_eq!(
        openclaw.config_toggle.status,
        "verified-skills-entries-v2.97"
    );
    assert!(openclaw.config_snapshot.supported);
    assert_eq!(openclaw.config_snapshot.status, "verified-v2.97");
    assert!(openclaw.install.supported);
    assert_eq!(openclaw.install.status, "verified-native-workspace-v2.96");
    assert!(openclaw.writable.supported);
    assert_eq!(openclaw.writable.status, "guarded-v2.97");
    assert!(openclaw
        .blockers
        .iter()
        .any(|blocker| blocker.contains(".agents roots")));
    assert!(openclaw
        .blockers
        .iter()
        .any(|blocker| blocker.contains("ClawHub, Git")));
}

#[test]
fn install_project_target_outside_current_root_is_rejected() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-install-project-boundary-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let project_a = temp_root.join("project-a");
    let project_b = temp_root.join("project-b");
    std::fs::create_dir_all(&home).expect("create home");
    std::fs::create_dir_all(&project_a).expect("create project a");
    std::fs::create_dir_all(&project_b).expect("create project b");
    let source_path = write_tool_global_skill(&temp_root, "portable-delta");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instance(&install_tool_global_instance(
            "tool-global-delta",
            source_path,
            "portable-delta",
        ))
        .expect("upsert tool-global");
    let ctx = AdapterContext {
        user_home: home,
        project_root: Some(project_a.clone()),
        project_cwd: Some(project_a),
        extra_roots: vec![],
    };

    let err = install_skill_from_tool_global(
        &catalog,
        &ctx,
        "tool-global-delta",
        AgentId::Codex,
        Scope::AgentProject,
        Some(&project_b),
        false,
    )
    .expect_err("project install outside current context must be rejected");

    assert!(err.to_string().contains("current project context"));
    assert!(
        !project_b.join(".agents").exists(),
        "rejected project install must not create target dirs"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn save_claude_settings_redacts_sensitive_snapshot_content() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-redacted-snapshot-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings dir");
    std::fs::write(
        &settings_path,
        "{\n  \"apiKey\": \"sk-live-secret\",\n  \"OPENAI_API_KEY\": \"sk-openai-secret\",\n  \"nested\": { \"access_token\": \"tok\" }\n}\n",
    )
    .expect("write sensitive settings");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let revision = read_claude_settings(&ctx).expect("read config").revision;
    save_claude_settings(&catalog, &ctx, "{}\n", &revision).expect("save config");

    let snapshots = catalog
        .list_config_snapshots("claude-code", &settings_path.to_string_lossy())
        .expect("snapshots");
    assert_eq!(snapshots.len(), 1);
    assert!(snapshots[0].content.starts_with(REDACTED_SNAPSHOT_PREFIX));
    assert!(!snapshots[0].content.contains("sk-live-secret"));
    assert!(!snapshots[0].content.contains("sk-openai-secret"));
    assert!(!snapshots[0].content.contains("\"tok\""));
    assert!(snapshots[0].content.contains(REDACTED_VALUE));

    let preview = preview_snapshot_rollback_with_context(&catalog, &ctx, &snapshots[0].id)
        .expect("preview redacted snapshot");
    assert!(preview.redacted);
    assert!(!preview.rollback_supported);
    let rollback = rollback_snapshot(&catalog, &ctx, &snapshots[0].id, &preview.preview_token);
    assert!(
        matches!(rollback, Err(CommandError::UnsafeConfigPath(_))),
        "redacted snapshots must not be written back"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
#[cfg(unix)]
fn save_claude_settings_writes_private_config_and_lock_permissions() {
    use std::os::unix::fs::PermissionsExt;

    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-private-config-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let settings_path = home.join(".claude/settings.json");
    std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
        .expect("create settings dir");
    std::fs::write(&settings_path, "{}\n").expect("write settings");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let revision = read_claude_settings(&ctx).expect("read config").revision;
    save_claude_settings(
        &catalog,
        &ctx,
        "{\n  \"skillOverrides\": {}\n}\n",
        &revision,
    )
    .expect("save config");

    let config_mode = std::fs::metadata(&settings_path)
        .expect("config metadata")
        .permissions()
        .mode()
        & 0o777;
    let lock_mode = std::fs::metadata(settings_path.with_extension("lock"))
        .expect("lock metadata")
        .permissions()
        .mode()
        & 0o777;
    assert_eq!(config_mode, 0o600);
    assert_eq!(lock_mode, 0o600);

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
#[cfg(unix)]
fn save_claude_settings_rejects_symlinked_config_directory() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-save-symlink-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    let outside = temp_root.join("outside");
    std::fs::create_dir_all(&home).expect("create home");
    std::fs::create_dir_all(&outside).expect("create outside dir");
    std::os::unix::fs::symlink(&outside, home.join(".claude")).expect("create config dir symlink");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let result = save_claude_settings(&catalog, &ctx, "{}\n", "sha256:missing");

    assert!(
        matches!(result, Err(CommandError::UnsafeConfigPath(_))),
        "symlinked config directory must be rejected"
    );
    assert!(
        !outside.join("settings.json").exists(),
        "write must not follow the symlinked config directory"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn rollback_snapshot_maps_target_outside_expected_config_path_to_stale_preview() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-rollback-path-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    std::fs::create_dir_all(home.join(".claude")).expect("create claude dir");
    let outside_target = temp_root.join("outside-settings.json");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "tampered-snapshot",
            agent: ClaudeCodeAdapter.id().as_str(),
            scope: Scope::AgentGlobal.as_str(),
            project_root: None,
            target: &outside_target.to_string_lossy(),
            content: "{}\n",
            reason: "tampered",
            created_at_ms: current_time_ms(),
        })
        .expect("create tampered snapshot");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let result = rollback_snapshot(&catalog, &ctx, "tampered-snapshot", "sha256:unused");

    assert!(
        matches!(result, Err(CommandError::StalePreviewToken)),
        "rollback must map a drifted snapshot target to a stale preview token"
    );
    assert!(
        !outside_target.exists(),
        "rollback must not write the tampered snapshot target"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn preview_snapshot_rejects_target_outside_expected_config_path() {
    let temp_root = std::env::temp_dir().join(format!(
        "skills-copilot-preview-path-{}",
        std::process::id()
    ));
    let home = temp_root.join("home");
    std::fs::create_dir_all(home.join(".claude")).expect("create claude dir");
    let outside_target = temp_root.join("outside-settings.json");
    std::fs::write(&outside_target, "do not read\n").expect("write outside target");

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .create_config_snapshot(ConfigSnapshotDraft {
            id: "tampered-preview",
            agent: ClaudeCodeAdapter.id().as_str(),
            scope: Scope::AgentGlobal.as_str(),
            project_root: None,
            target: &outside_target.to_string_lossy(),
            content: "{}\n",
            reason: "tampered",
            created_at_ms: current_time_ms(),
        })
        .expect("create tampered snapshot");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let result = preview_snapshot_rollback_with_context(&catalog, &ctx, "tampered-preview");

    assert!(
        matches!(result, Err(CommandError::UnsafeConfigPath(_))),
        "preview must reject snapshot targets outside the expected settings path"
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
#[ignore = "10k benchmark; run with `pnpm benchmark:10k`"]
fn benchmark_10k_scan_to_catalog() {
    const SKILL_COUNT: usize = 10_000;

    let temp_root =
        std::env::temp_dir().join(format!("skills-copilot-bench-{}", std::process::id()));
    let home = temp_root.join("home");
    let skills_root = home.join(".claude/skills");
    std::fs::create_dir_all(&skills_root).expect("create skills root");
    std::fs::write(home.join(".claude/settings.json"), "{}\n").expect("write settings");

    for idx in 0..SKILL_COUNT {
        let skill_dir = skills_root.join(format!("bench-{idx:05}"));
        std::fs::create_dir_all(&skill_dir).expect("create skill dir");
        std::fs::write(
                skill_dir.join("SKILL.md"),
                format!(
                    "---\nname: bench-{idx:05}\ndescription: Synthetic benchmark skill {idx}\n---\n# bench-{idx:05}\n\nBody {idx}.\n"
                ),
            )
            .expect("write skill");
    }

    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let started_at = Instant::now();
    let report = scan_claude_catalog_report(&ctx, &catalog).expect("benchmark scan succeeds");
    let count = report.scanned_count;
    let elapsed = started_at.elapsed();
    let records = catalog.list_skill_records().expect("records list");

    assert_eq!(count, SKILL_COUNT);
    assert_eq!(records.len(), SKILL_COUNT);
    assert!(
        !report.budget_exhausted,
        "production defaults must not exhaust the scanner budget for 10k skills"
    );
    println!(
        "skills-copilot-bench scanned={count} records={} budget_exhausted={} elapsed_ms={} elapsed_s={:.3}",
        records.len(),
        report.budget_exhausted,
        elapsed.as_millis(),
        elapsed.as_secs_f64()
    );

    let _ = std::fs::remove_dir_all(&temp_root);
}

fn fixture_path(relative: &str) -> PathBuf {
    PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("../..")
        .join(relative)
}

fn temp_test_dir(label: &str) -> PathBuf {
    env::temp_dir().join(format!(
        "skills-copilot-{label}-{}-{}",
        std::process::id(),
        SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .expect("system clock")
            .as_nanos()
    ))
}

fn path_text(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

fn assert_path_text_eq(actual: &str, expected: &Path) {
    assert_eq!(PathBuf::from(actual), expected);
}

fn toml_basic_string(value: &str) -> String {
    value
        .chars()
        .flat_map(|ch| match ch {
            '\u{0008}' => "\\b".chars().collect::<Vec<_>>(),
            '\t' => "\\t".chars().collect(),
            '\n' => "\\n".chars().collect(),
            '\u{000c}' => "\\f".chars().collect(),
            '\r' => "\\r".chars().collect(),
            '"' => "\\\"".chars().collect(),
            '\\' => "\\\\".chars().collect(),
            other => vec![other],
        })
        .collect()
}

fn codex_config_contains_path(content: &str, path: &Path, enabled: bool) -> bool {
    let expected_path = path
        .canonicalize()
        .map(|canonical| path_text(&canonical))
        .unwrap_or_else(|_| path_text(path));
    parse_codex_skill_config_entries(content)
        .into_iter()
        .any(|entry| {
            entry.path.as_deref() == Some(expected_path.as_str()) && entry.enabled == Some(enabled)
        })
}

fn write_codex_skill(root: &Path, name: &str) -> PathBuf {
    let skill_dir = root.join(".agents/skills").join(name);
    std::fs::create_dir_all(&skill_dir).expect("create codex skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(
        &skill_path,
        format!("---\nname: {name}\ndescription: {name} fixture\n---\nbody"),
    )
    .expect("write codex skill");
    skill_path.canonicalize().expect("canonicalize skill path")
}

fn write_codex_skill_file(root: &Path, dir_name: &str, content: &str) -> PathBuf {
    let skill_dir = root.join(".agents/skills").join(dir_name);
    std::fs::create_dir_all(&skill_dir).expect("create codex skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(&skill_path, content).expect("write codex skill");
    skill_path.canonicalize().expect("canonicalize skill path")
}

fn tool_global_instance(id: &str, path: &Path) -> SkillInstance {
    let frontmatter =
            "name: exportable\ndescription: Exportable fixture\nversion: 2.9.0\nallowed-tools:\n  - Read";
    let body = "Use local read-only context.\n";
    SkillInstance {
        id: id.to_string(),
        agent: AgentId::Codex,
        scope: Scope::ToolGlobal,
        project_root: None,
        path: path.to_path_buf(),
        display_path: PathBuf::from("tool-global/exportable/SKILL.md"),
        definition_id: "exportable-definition".to_string(),
        name: "exportable".to_string(),
        display_name: "exportable".to_string(),
        description: "Exportable fixture".to_string(),
        version: Some("2.9.0".to_string()),
        state: SkillState::Loaded,
        enabled: true,
        frontmatter_raw: frontmatter.to_string(),
        body: body.to_string(),
        scripts: Vec::new(),
        permissions: PermissionRequest {
            tools: vec!["Read".to_string()],
            ..PermissionRequest::default()
        },
        fingerprint: content_fingerprint(frontmatter, body),
        mtime: 0,
        first_seen: 0,
        last_seen: 0,
    }
}

fn has_rule_for_name(
    records: &[SkillRecord],
    findings: &[RuleFindingRecord],
    name: &str,
    rule_id: &str,
) -> bool {
    let Some(record) = records.iter().find(|record| record.name == name) else {
        return false;
    };
    findings.iter().any(|finding| {
        finding.rule_id == rule_id && finding.instance_id.as_deref() == Some(record.id.as_str())
    })
}

fn write_claude_skill(root: &Path, name: &str) -> PathBuf {
    let skill_dir = root.join(".claude/skills").join(name);
    std::fs::create_dir_all(&skill_dir).expect("create claude skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(
        &skill_path,
        format!("---\nname: {name}\ndescription: {name} fixture\n---\nbody"),
    )
    .expect("write claude skill");
    skill_path.canonicalize().expect("canonicalize skill path")
}

fn write_command_scan_fixture(root: &Path, name: &str) -> PathBuf {
    let skill_dir = root.join(name);
    std::fs::create_dir_all(&skill_dir).expect("create command scan skill directory");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(
        &skill_path,
        format!("---\nname: {name}\ndescription: {name} fixture\n---\nbody\n"),
    )
    .expect("write command scan skill");
    skill_path
        .canonicalize()
        .expect("canonicalize command scan skill")
}

fn seeded_command_scan_instance(id: &str, path: &Path, name: &str) -> SkillInstance {
    let mut instance = ClaudeCodeAdapter
        .parse(path)
        .expect("parse command scan fixture");
    instance.id = id.to_string();
    instance.scope = Scope::AgentGlobal;
    instance.project_root = None;
    instance.path = path.to_path_buf();
    instance.display_path = path.to_path_buf();
    instance.name = name.to_string();
    instance.display_name = name.to_string();
    instance.state = SkillState::Loaded;
    instance.enabled = true;
    instance.mtime = 0;
    instance.first_seen = 0;
    instance.last_seen = 0;
    instance
}

fn write_staging_skill(staging_root: &Path, name: &str) -> PathBuf {
    let skill_dir = staging_root.join(name);
    std::fs::create_dir_all(&skill_dir).expect("create staging skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(
        &skill_path,
        format!("---\nname: {name}\ndescription: {name} staging fixture\n---\nbody"),
    )
    .expect("write staging skill");
    skill_path.canonicalize().expect("canonicalize skill path")
}

fn write_opencode_global_skill(root: &Path, name: &str) -> PathBuf {
    let skill_dir = root.join(".config/opencode/skills").join(name);
    std::fs::create_dir_all(&skill_dir).expect("create opencode skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(
        &skill_path,
        format!("---\nname: {name}\ndescription: {name} fixture\n---\nbody"),
    )
    .expect("write opencode skill");
    skill_path.canonicalize().expect("canonicalize skill path")
}

fn write_pi_global_skill(root: &Path, name: &str) -> PathBuf {
    let skill_dir = root.join(".pi/agent/skills").join(name);
    std::fs::create_dir_all(&skill_dir).expect("create pi skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(
        &skill_path,
        format!("---\nname: {name}\ndescription: {name} fixture\n---\nbody"),
    )
    .expect("write pi skill");
    skill_path.canonicalize().expect("canonicalize skill path")
}

fn write_pi_project_compatibility_skill(root: &Path, name: &str) -> PathBuf {
    let skill_dir = root.join(".agents/skills").join(name);
    std::fs::create_dir_all(&skill_dir).expect("create pi compatibility skill dir");
    let path = skill_dir.join("SKILL.md");
    std::fs::write(
        &path,
        format!("---\nname: {name}\ndescription: Pi compatibility fixture\n---\nbody"),
    )
    .expect("write pi compatibility skill");
    path
}

fn write_hermes_global_skill(root: &Path, name: &str) -> PathBuf {
    let skill_dir = root.join(".hermes/skills").join(name);
    std::fs::create_dir_all(&skill_dir).expect("create hermes skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(
        &skill_path,
        format!("---\nname: {name}\ndescription: {name} fixture\n---\nbody"),
    )
    .expect("write hermes skill");
    skill_path.canonicalize().expect("canonicalize skill path")
}

fn write_openclaw_global_skill_with_metadata(
    root: &Path,
    name: &str,
    skill_key: Option<&str>,
) -> PathBuf {
    let skill_dir = root.join(".openclaw/skills").join(name);
    std::fs::create_dir_all(&skill_dir).expect("create openclaw skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    let metadata = skill_key
        .map(|key| format!("metadata:\n  openclaw:\n    skillKey: {key}\n"))
        .unwrap_or_default();
    std::fs::write(
        &skill_path,
        format!("---\nname: {name}\ndescription: {name} fixture\n{metadata}---\nbody"),
    )
    .expect("write openclaw skill");
    skill_path.canonicalize().expect("canonicalize skill path")
}

fn write_tool_global_skill(root: &Path, name: &str) -> PathBuf {
    let skill_dir = root.join("tool-global").join(name);
    std::fs::create_dir_all(&skill_dir).expect("create tool-global skill dir");
    let skill_path = skill_dir.join("SKILL.md");
    std::fs::write(
        &skill_path,
        format!("---\nname: {name}\ndescription: {name} fixture\n---\nbody"),
    )
    .expect("write tool-global skill");
    skill_path.canonicalize().expect("canonicalize skill path")
}

fn install_tool_global_instance(id: &str, path: PathBuf, name: &str) -> SkillInstance {
    SkillInstance {
        id: id.to_string(),
        agent: AgentId::ClaudeCode,
        scope: Scope::ToolGlobal,
        project_root: None,
        path: path.clone(),
        display_path: path,
        definition_id: name.to_string(),
        name: name.to_string(),
        display_name: name.to_string(),
        description: "synthetic tool-global import".to_string(),
        version: None,
        state: SkillState::Loaded,
        enabled: true,
        frontmatter_raw: String::new(),
        body: String::new(),
        scripts: Vec::new(),
        permissions: PermissionRequest::default(),
        fingerprint: String::new(),
        mtime: 0,
        first_seen: 0,
        last_seen: 0,
    }
}

fn synthetic_codex_project_instance(
    id: &str,
    project_root: &Path,
    path: PathBuf,
    name: &str,
) -> SkillInstance {
    SkillInstance {
        id: id.to_string(),
        agent: AgentId::Codex,
        scope: Scope::AgentProject,
        project_root: Some(project_root.to_path_buf()),
        path: path.clone(),
        display_path: path,
        definition_id: name.to_string(),
        name: name.to_string(),
        display_name: name.to_string(),
        description: "synthetic project context guard".to_string(),
        version: None,
        state: SkillState::Loaded,
        enabled: true,
        frontmatter_raw: String::new(),
        body: String::new(),
        scripts: Vec::new(),
        permissions: PermissionRequest::default(),
        fingerprint: String::new(),
        mtime: 0,
        first_seen: 0,
        last_seen: 0,
    }
}

fn synthetic_opencode_project_instance(
    id: &str,
    project_root: &Path,
    path: PathBuf,
    name: &str,
) -> SkillInstance {
    SkillInstance {
        id: id.to_string(),
        agent: AgentId::Opencode,
        scope: Scope::AgentProject,
        project_root: Some(project_root.to_path_buf()),
        path: path.clone(),
        display_path: path,
        definition_id: name.to_string(),
        name: name.to_string(),
        display_name: name.to_string(),
        description: "synthetic outside-workspace guard".to_string(),
        version: None,
        state: SkillState::Loaded,
        enabled: true,
        frontmatter_raw: format!("name: {name}\ndescription: synthetic\n"),
        body: String::new(),
        scripts: Vec::new(),
        permissions: PermissionRequest::default(),
        fingerprint: String::new(),
        mtime: 0,
        first_seen: 0,
        last_seen: 0,
    }
}

fn local_rule_instance(id: &str, frontmatter_raw: &str, body: &str) -> SkillInstance {
    SkillInstance {
        id: id.to_string(),
        agent: AgentId::ClaudeCode,
        scope: Scope::AgentGlobal,
        project_root: None,
        path: PathBuf::from(format!("/tmp/{id}/SKILL.md")),
        display_path: PathBuf::from(format!("/tmp/{id}/SKILL.md")),
        definition_id: id.to_string(),
        name: id.to_string(),
        display_name: id.to_string(),
        description: "synthetic local rule skill".to_string(),
        version: None,
        state: SkillState::Loaded,
        enabled: true,
        frontmatter_raw: frontmatter_raw.to_string(),
        body: body.to_string(),
        scripts: Vec::new(),
        permissions: PermissionRequest::default(),
        fingerprint: String::new(),
        mtime: 0,
        first_seen: 0,
        last_seen: 0,
    }
}

fn assert_rule_present(report: &RuleReport, rule_id: &str) {
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule_id == rule_id),
        "expected {rule_id} finding"
    );
}

fn assert_rule_absent(report: &RuleReport, rule_id: &str) {
    assert!(
        report
            .findings
            .iter()
            .all(|finding| finding.rule_id != rule_id),
        "did not expect {rule_id} finding"
    );
}
#[cfg(test)]
#[path = "tests/v219_skill_health_tests.rs"]
mod v219_skill_health_tests;

#[cfg(test)]
#[path = "tests/v218_cross_agent_analysis_tests.rs"]
mod v218_cross_agent_analysis_tests;
