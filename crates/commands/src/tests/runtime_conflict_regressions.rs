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
fn codex_runtime_conflicts_ignore_inactive_inventory_and_namespace_active_plugins() {
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
        "[plugins.\"pdf@openai-primary-runtime\"]\nenabled = true\n\n[plugins.\"package-a@active\"]\nenabled = true\n\n[plugins.\"package-b@active\"]\nenabled = true\n",
    )
    .expect("write Codex plugin config");
    let catalog = Catalog::in_memory().expect("catalog opens");
    catalog.init().expect("catalog initializes");
    let ctx = AdapterContext {
        user_home: home,
        project_root: None,
        project_cwd: None,
        extra_roots: vec![],
    };

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

    assert_eq!(conflict_names, std::collections::BTreeSet::from(["pdf"]));
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
        .any(|conflict| conflict.id.contains(":pi:")));

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
