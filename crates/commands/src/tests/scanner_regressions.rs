use super::*;

#[cfg(unix)]
#[test]
fn scan_claude_report_keeps_dangling_link_diagnostic_without_partial_root() {
    let temp_root = temp_test_dir("scan-claude-partial-report");
    let scan_root = temp_root.join("skills");
    write_command_scan_fixture(&scan_root, "observed");
    std::os::unix::fs::symlink(
        scan_root.join("missing-target"),
        scan_root.join("dangling-directory-link"),
    )
    .expect("create dangling directory link");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: temp_root.join("empty-home"),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![AdapterRoot {
            scope: Scope::AgentGlobal,
            path: scan_root.clone(),
            source: RootSource::Extra,
            logical_source_id: None,
        }],
    };

    let report = scan_claude_catalog_report(&ctx, &catalog).expect("partial scan succeeds");
    let canonical_root = scan_root.canonicalize().expect("canonical root");

    assert_eq!(report.agent, AgentId::ClaudeCode);
    assert!(report.partial_roots.is_empty());
    assert!(report
        .issues
        .iter()
        .any(|issue| issue.kind == "dangling_symlink"));
    assert!(report.scanned_roots.contains(&canonical_root));

    let _ = std::fs::remove_dir_all(temp_root);
}

#[test]
fn catalog_update_uses_complete_root_scope_for_missing_sweep() {
    let temp_root = temp_test_dir("scoped-cache-refresh");
    let scan_root = temp_root.join("skills");
    let shared_path = write_command_scan_fixture(&scan_root, "shared");
    let mut global = seeded_command_scan_instance("same-root-global", &shared_path, "shared");
    global.scope = Scope::AgentGlobal;
    let mut project = global.clone();
    project.id = "same-root-project".to_string();
    project.scope = Scope::AgentProject;
    project.project_root = Some(temp_root.join("project"));
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    catalog
        .upsert_skill_instances(&[global, project])
        .expect("seed same-root rows");
    let canonical_root = scan_root.canonicalize().expect("canonical root");
    let report = skills_copilot_scanner::ScanReport {
        scoped_scanned_roots: vec![skills_copilot_scanner::ScopedScanRoot {
            scope: Scope::AgentGlobal,
            path: canonical_root.clone(),
        }],
        scoped_partial_roots: vec![skills_copilot_scanner::ScopedScanRoot {
            scope: Scope::AgentProject,
            path: canonical_root,
        }],
        ..Default::default()
    };
    let project_root = temp_root.join("project");
    let ctx = AdapterContext {
        user_home: temp_root.join("home"),
        project_root: Some(project_root.clone()),
        project_cwd: Some(project_root),
        extra_roots: Vec::new(),
    };

    update_catalog_from_scan_report(&ClaudeCodeAdapter, &ctx, &catalog, &report)
        .expect("catalog update succeeds");
    let records = catalog.list_skill_records().expect("records after update");

    assert_eq!(
        records
            .iter()
            .find(|record| record.id == "same-root-global")
            .expect("global row")
            .state,
        "missing"
    );
    assert_eq!(
        records
            .iter()
            .find(|record| record.id == "same-root-project")
            .expect("project row")
            .state,
        "loaded"
    );

    let _ = std::fs::remove_dir_all(temp_root);
}
