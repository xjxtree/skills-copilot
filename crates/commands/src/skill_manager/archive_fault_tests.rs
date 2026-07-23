use super::*;
use skills_copilot_catalog::CatalogError;
use zip::{write::SimpleFileOptions, ZipWriter};

fn unique_root(label: &str) -> PathBuf {
    std::env::temp_dir().join(format!(
        "skill-manager-archive-fault-{label}-{}-{}",
        std::process::id(),
        unix_timestamp_millis()
    ))
}

fn write_test_archive(path: &Path, skill_name: &str, marker: &str) {
    let file = File::create(path).expect("archive file");
    let mut writer = ZipWriter::new(file);
    writer
        .start_file(
            format!("{skill_name}/SKILL.md"),
            SimpleFileOptions::default(),
        )
        .expect("archive entry");
    writer
        .write_all(
            format!("---\nname: {skill_name}\ndescription: {marker}\n---\n# {marker}\n").as_bytes(),
        )
        .expect("archive content");
    writer.finish().expect("finish archive");
}

fn test_context(root: &Path) -> AdapterContext {
    AdapterContext {
        user_home: root.join("home"),
        project_root: None,
        project_cwd: None,
        extra_roots: Vec::new(),
    }
}

fn exercise_import_commit_failure(outcome_unknown: bool) {
    crate::initialize_action_preview_secret_for_test([0xA5; 32]).expect("action preview secret");
    let label = if outcome_unknown {
        "import-unknown"
    } else {
        "import-rejected"
    };
    let root = unique_root(label);
    let archive = root.join("candidate.zip");
    let skill_name = format!("{label}-skill");
    fs::create_dir_all(root.join("app-data")).expect("app data");
    fs::create_dir_all(root.join("home")).expect("home");
    write_test_archive(&archive, &skill_name, "Candidate");
    let catalog = Catalog::open(&root.join("catalog.sqlite")).expect("catalog");
    catalog.init().expect("catalog schema");
    let ctx = test_context(&root);
    let preview = preview_local_archive_import(
        &catalog,
        &root.join("app-data"),
        &ctx,
        &SkillManagerLocalArchiveImportParams {
            archive_path: archive.to_string_lossy().to_string(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect("import preview");
    if outcome_unknown {
        catalog.inject_next_commit_outcome_unknown_for_test();
    } else {
        catalog.inject_next_commit_failure_for_test();
    }
    let error = apply_local_archive_import(
        &catalog,
        &root.join("app-data"),
        &ctx,
        &SkillManagerLocalArchiveImportParams {
            archive_path: archive.to_string_lossy().to_string(),
            confirmed: true,
            preview_token: Some(preview.preview_token),
            action_reference: Some(ActionReference::from(&preview.action)),
        },
    )
    .expect_err("commit fault");
    let destination = tool_global_staging_skills_root(&root.join("app-data")).join(&skill_name);
    if outcome_unknown {
        assert!(matches!(
            error,
            CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            }
        ));
        assert!(
            destination.join("SKILL.md").is_file(),
            "uncertain commit must retain the imported tree"
        );
    } else {
        assert!(matches!(
            error,
            CommandError::Catalog(CatalogError::InjectedCommitFailure)
        ));
        assert!(
            !destination.exists(),
            "proven-uncommitted import must restore the original missing state"
        );
    }
    fs::remove_dir_all(root).ok();
}

#[test]
fn rejected_import_commit_restores_imported_tree() {
    exercise_import_commit_failure(false);
}

#[test]
fn uncertain_import_commit_retains_recovery_tree() {
    exercise_import_commit_failure(true);
}

fn exercise_update_commit_failure(outcome_unknown: bool) {
    crate::initialize_action_preview_secret_for_test([0xA5; 32]).expect("action preview secret");
    let label = if outcome_unknown {
        "update-unknown"
    } else {
        "update-rejected"
    };
    let root = unique_root(label);
    let home = root.join("home");
    let skill_name = format!("{label}-skill");
    let skill_dir = home.join(".agents/skills").join(&skill_name);
    let archive = root.join("candidate.zip");
    fs::create_dir_all(&skill_dir).expect("skill directory");
    fs::create_dir_all(root.join("app-data")).expect("app data");
    fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: {skill_name}\ndescription: Original\n---\n# Original\n"),
    )
    .expect("original skill");
    write_test_archive(&archive, &skill_name, "Replacement");
    let catalog = Catalog::open(&root.join("catalog.sqlite")).expect("catalog");
    catalog.init().expect("catalog schema");
    let ctx = test_context(&root);
    scan_all_catalog_report(&ctx, &catalog).expect("initial scan");
    let instance = catalog
        .list_skill_records()
        .expect("records")
        .into_iter()
        .find(|record| record.name == skill_name)
        .expect("local skill");
    let preview = preview_local_archive_update(
        &catalog,
        &root.join("app-data"),
        &ctx,
        &SkillManagerLocalArchiveUpdateParams {
            instance_id: instance.id.clone(),
            archive_path: archive.to_string_lossy().to_string(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect("update preview");
    if outcome_unknown {
        catalog.inject_next_commit_outcome_unknown_for_test();
    } else {
        catalog.inject_next_commit_failure_for_test();
    }
    let error = apply_local_archive_update(
        &catalog,
        &root.join("app-data"),
        &ctx,
        &SkillManagerLocalArchiveUpdateParams {
            instance_id: instance.id,
            archive_path: archive.to_string_lossy().to_string(),
            confirmed: true,
            preview_token: Some(preview.preview_token),
            action_reference: Some(ActionReference::from(&preview.action)),
        },
    )
    .expect_err("commit fault");
    let active = fs::read_to_string(skill_dir.join("SKILL.md")).expect("active skill");
    let backup_exists = fs::read_dir(home.join(".agents/skills"))
        .expect("skills root")
        .filter_map(Result::ok)
        .any(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".archive-backup-")
        });
    if outcome_unknown {
        assert!(matches!(
            error,
            CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            }
        ));
        assert!(active.contains("Replacement"));
        assert!(backup_exists, "uncertain commit must retain the backup");
    } else {
        assert!(matches!(
            error,
            CommandError::Catalog(CatalogError::InjectedCommitFailure)
        ));
        assert!(active.contains("Original"));
        assert!(
            !backup_exists,
            "proven-uncommitted update must restore exactly"
        );
    }
    fs::remove_dir_all(root).ok();
}

#[test]
fn rejected_update_commit_restores_original_tree() {
    exercise_update_commit_failure(false);
}

#[test]
fn uncertain_update_commit_retains_recovery_backup() {
    exercise_update_commit_failure(true);
}

#[test]
fn extraction_uses_the_inspected_archive_snapshot_after_path_replacement() {
    let root = unique_root("snapshot");
    fs::create_dir_all(&root).expect("test root");
    let archive = root.join("candidate.zip");
    write_test_archive(&archive, "snapshot-skill", "First");
    let inspection = inspect_archive(&archive).expect("first inspection");
    write_test_archive(&archive, "snapshot-skill", "Second");
    let destination = root.join("extracted");
    fs::create_dir(&destination).expect("destination");

    extract_skill_root(&inspection, &destination).expect("snapshot extraction");

    let content = fs::read_to_string(destination.join("SKILL.md")).expect("extracted skill");
    assert!(content.contains("First"));
    assert!(!content.contains("Second"));
    fs::remove_dir_all(root).ok();
}
