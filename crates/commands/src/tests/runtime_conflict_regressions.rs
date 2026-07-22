use super::*;

#[test]
fn codex_cached_records_project_external_config_changes_without_rescan() {
    let temp_root = temp_test_dir("codex-live-config-projection");
    let home = temp_root.join("home");
    let skill_path = write_codex_skill(&home, "using-superpowers");
    let config_path = home.join(".codex/config.toml");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    scan_all_to_catalog(&ctx, &catalog).expect("initial scan");
    std::fs::create_dir_all(config_path.parent().expect("config parent"))
        .expect("create config dir");
    std::fs::write(
        &config_path,
        format!(
            "[[skills.config]]\npath = '{}'\nenabled = false\n",
            path_text(&skill_path)
        ),
    )
    .expect("write external disabled config");

    let mut records = catalog.list_skill_records().expect("cached records");
    let record = records
        .iter()
        .find(|record| record.agent == "codex" && record.name == "using-superpowers")
        .expect("cached codex record");
    assert_eq!(record.state, "loaded");
    assert!(record.enabled);

    apply_current_config_overrides_to_skill_records(&ctx, &mut records)
        .expect("project record config");
    let projected = records
        .iter()
        .find(|record| record.agent == "codex" && record.name == "using-superpowers")
        .expect("projected codex record");
    assert_eq!(projected.state, "disabled");
    assert!(!projected.enabled);

    let mut detail = get_skill(&catalog, &projected.id).expect("cached detail");
    apply_current_config_overrides_to_skill_detail(&ctx, &mut detail)
        .expect("project detail config");
    assert_eq!(detail.state, "disabled");
    assert!(!detail.enabled);

    std::fs::write(&config_path, "# external enable removed the override\n")
        .expect("clear disabled config");
    apply_current_config_overrides_to_skill_records(&ctx, &mut records)
        .expect("restore record config");
    apply_current_config_overrides_to_skill_detail(&ctx, &mut detail)
        .expect("restore detail config");
    let restored = records
        .iter()
        .find(|record| record.agent == "codex" && record.name == "using-superpowers")
        .expect("restored codex record");
    assert_eq!(restored.state, "loaded");
    assert!(restored.enabled);
    assert_eq!(detail.state, "loaded");
    assert!(detail.enabled);

    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn codex_plugin_file_state_follows_persisted_plugin_config() {
    let temp_root = temp_test_dir("codex-plugin-config-projection");
    let home = temp_root.join("home");
    write_codex_plugin_skill(
        &home,
        "openai-bundled",
        "visualize",
        "1.0.14",
        "visualize",
        "plugin body",
    );
    let config_path = home.join(".codex/config.toml");
    std::fs::write(
        &config_path,
        "[plugins.\"visualize@openai-bundled\"]\nenabled = true\n",
    )
    .expect("write enabled plugin config");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    scan_all_to_catalog(&ctx, &catalog).expect("plugin scan succeeds");
    let mut records = catalog.list_skill_records().expect("plugin records list");
    let enabled = records
        .iter()
        .find(|record| record.agent == "codex" && record.name == "visualize:visualize")
        .expect("plugin record exists");
    assert_eq!(enabled.state, "loaded");
    assert!(enabled.enabled);
    assert_eq!(enabled.source_kind.as_deref(), Some("chatgpt-plugin-cache"));

    std::fs::write(
        &config_path,
        "[plugins.\"visualize@openai-bundled\"]\nenabled = false\n",
    )
    .expect("write disabled plugin config");
    apply_current_config_overrides_to_skill_records(&ctx, &mut records)
        .expect("project current plugin state");
    let disabled = records
        .iter()
        .find(|record| record.agent == "codex" && record.name == "visualize:visualize")
        .expect("projected plugin record exists");
    assert_eq!(disabled.state, "disabled");
    assert!(!disabled.enabled);

    std::fs::write(&config_path, "# plugin installation entry removed\n")
        .expect("remove plugin installation entry");
    apply_current_config_overrides_to_skill_records(&ctx, &mut records)
        .expect("project removed plugin state");
    let uninstalled = records
        .iter()
        .find(|record| record.agent == "codex" && record.name == "visualize:visualize")
        .expect("cached plugin record remains available for projection");
    assert_eq!(uninstalled.state, "disabled");
    assert!(!uninstalled.enabled);

    std::fs::write(
        &config_path,
        "[plugins.\"visualize@openai-bundled\"]\nenabled = true\n",
    )
    .expect("restore enabled plugin config");
    apply_current_config_overrides_to_skill_records(&ctx, &mut records)
        .expect("restore current plugin state");
    let restored = records
        .iter()
        .find(|record| record.agent == "codex" && record.name == "visualize:visualize")
        .expect("restored plugin record exists");
    assert_eq!(restored.state, "loaded");
    assert!(restored.enabled);

    let _ = std::fs::remove_dir_all(temp_root);
}

#[test]
fn codex_plugin_files_are_cataloged_without_cross_plugin_runtime_conflicts() {
    let temp_root = temp_test_dir("codex-runtime-conflicts");
    let home = temp_root.join("home");
    let local_pdf = home.join(".codex/skills/pdf");
    std::fs::create_dir_all(&local_pdf).expect("create local pdf skill");
    std::fs::write(
        local_pdf.join("SKILL.md"),
        "---\nname: pdf\ndescription: local pdf\n---\nlocal body\n",
    )
    .expect("write local pdf skill");
    for (publisher, package, version, skill, body) in [
        (
            "openai-primary-runtime",
            "pdf",
            "1.0.0",
            "pdf",
            "runtime body",
        ),
        (
            "openai-curated",
            "legacy-mirror",
            "1.0.0",
            "shared-review",
            "legacy body",
        ),
        (
            "openai-curated-remote",
            "remote-mirror",
            "2.0.0",
            "shared-review",
            "remote body",
        ),
        ("active", "package-a", "1.0.0", "index", "package a body"),
        ("active", "package-b", "1.0.0", "index", "package b body"),
    ] {
        write_codex_plugin_skill(&home, publisher, package, version, skill, body);
    }
    std::fs::write(
        home.join(".codex/config.toml"),
        "[plugins.\"pdf@openai-primary-runtime\"]\nenabled = true\n\n[plugins.\"legacy-mirror@openai-curated\"]\nenabled = true\n\n[plugins.\"remote-mirror@openai-curated-remote\"]\nenabled = true\n\n[plugins.\"package-a@active\"]\nenabled = true\n\n[plugins.\"package-b@active\"]\nenabled = true\n",
    )
    .expect("write Codex plugin config");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home.clone(),
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

    let plugin_roots = CodexAdapter
        .roots(&ctx)
        .into_iter()
        .filter(|root| root.source == RootSource::Plugin)
        .collect::<Vec<_>>();
    assert_eq!(
        plugin_roots.len(),
        5,
        "fixture plugin roots must be discovered from manifests"
    );
    let canonical_cache = home
        .join(".codex/plugins/cache")
        .canonicalize()
        .expect("plugin cache canonicalizes");
    let codex_report = scan_agent(&CodexAdapter, &ctx).expect("Codex fixture scans");
    assert_eq!(
        codex_report
            .instances
            .iter()
            .filter(|instance| instance.path.starts_with(&canonical_cache))
            .count(),
        5,
        "every manifest-declared plugin skill should be scanned from its persisted file; skipped={:?}, issues={:?}",
        codex_report.skipped_roots,
        codex_report.issues
    );

    scan_all_to_catalog(&ctx, &catalog).expect("scan all succeeds");
    let records = catalog.list_skill_records().expect("records");
    let names_by_id = records
        .iter()
        .map(|record| (record.id.as_str(), record.name.as_str()))
        .collect::<std::collections::HashMap<_, _>>();
    let conflict_names = catalog
        .list_conflict_groups()
        .expect("conflicts")
        .into_iter()
        .flat_map(|conflict| conflict.instance_ids)
        .filter_map(|instance_id| names_by_id.get(instance_id.as_str()).copied())
        .collect::<std::collections::BTreeSet<_>>();

    assert!(records
        .iter()
        .any(|record| record.path.starts_with(&canonical_cache)));
    assert!(records.iter().all(|record| {
        !record
            .path
            .to_string_lossy()
            .contains(".agent-copilot-runtime")
    }));
    assert!(
        conflict_names.is_empty(),
        "native and plugin skills have distinct runtime identities, and separate plugin namespaces must stay isolated"
    );
    let _ = std::fs::remove_dir_all(&temp_root);
}

#[test]
fn exact_rescan_retires_removed_pi_root_and_clears_its_conflict() {
    let temp_root = temp_test_dir("retired-pi-root");
    let home = temp_root.join("home");
    let project = temp_root.join("project");
    let native = project.join(".pi/skills/shared-review");
    let compatibility = project.join(".agents/skills/shared-review");
    for (directory, body) in [(&native, "native body"), (&compatibility, "compat body")] {
        std::fs::create_dir_all(directory).expect("create Pi skill directory");
        std::fs::write(
            directory.join("SKILL.md"),
            format!("---\nname: shared-review\ndescription: Pi shared review\n---\n{body}\n"),
        )
        .expect("write Pi skill");
    }
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: Some(project.clone()),
        project_cwd: Some(project.clone()),
        extra_roots: vec![],
    };

    scan_all_to_catalog(&ctx, &catalog).expect("initial scan succeeds");
    assert!(catalog
        .list_conflict_groups()
        .expect("initial conflicts")
        .iter()
        .all(|conflict| !conflict.id.contains(":pi:")));
    let initial_pi_records = catalog
        .list_skill_records()
        .expect("initial records")
        .into_iter()
        .filter(|record| record.agent == "pi" && record.name == "shared-review")
        .collect::<Vec<_>>();
    assert_eq!(initial_pi_records.len(), 1);
    assert!(initial_pi_records[0]
        .path
        .starts_with(native.canonicalize().expect("native root canonicalizes")));

    std::fs::remove_dir_all(project.join(".pi/skills")).expect("remove retired Pi root");
    scan_all_to_catalog(&ctx, &catalog).expect("rescan succeeds");

    assert!(catalog
        .list_conflict_groups()
        .expect("conflicts after rescan")
        .iter()
        .all(|conflict| !conflict.id.contains(":pi:")));
    let pi_records = catalog
        .list_skill_records()
        .expect("records after rescan")
        .into_iter()
        .filter(|record| record.agent == "pi" && record.name == "shared-review")
        .collect::<Vec<_>>();
    assert_eq!(pi_records.len(), 2);
    assert_eq!(
        pi_records
            .iter()
            .filter(|record| record.state == "missing")
            .count(),
        1
    );
    let _ = std::fs::remove_dir_all(&temp_root);
}

fn write_codex_plugin_skill(
    home: &Path,
    publisher: &str,
    package: &str,
    version: &str,
    skill_name: &str,
    body: &str,
) {
    let package_root = home
        .join(".codex/plugins/cache")
        .join(publisher)
        .join(package)
        .join(version);
    let skill_dir = package_root.join("skills").join(skill_name);
    std::fs::create_dir_all(package_root.join(".codex-plugin"))
        .expect("create plugin manifest directory");
    std::fs::create_dir_all(&skill_dir).expect("create plugin skill directory");
    std::fs::write(
        package_root.join(".codex-plugin/plugin.json"),
        format!("{{\"name\":\"{package}\",\"version\":\"{version}\",\"skills\":\"./skills/\"}}"),
    )
    .expect("write plugin manifest");
    std::fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: {skill_name}\ndescription: plugin fixture\n---\n{body}\n"),
    )
    .expect("write plugin skill");
}
