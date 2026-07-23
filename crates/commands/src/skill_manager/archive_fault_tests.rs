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
    let catalog = Catalog::in_memory().expect("catalog");
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
        assert!(
            matches!(
                error,
                CommandError::Catalog(CatalogError::InjectedCommitFailure)
            ),
            "unexpected rejected-import error: {error:?}"
        );
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

#[test]
fn import_rollback_failure_preserves_the_activated_candidate() {
    crate::initialize_action_preview_secret_for_test([0xA5; 32]).expect("action preview secret");
    let root = unique_root("import-rollback-unknown");
    let app_data = root.join("app-data");
    let archive = root.join("candidate.zip");
    let skill_name = "import-rollback-unknown-skill";
    fs::create_dir_all(&app_data).expect("app data");
    fs::create_dir_all(root.join("home")).expect("home");
    write_test_archive(&archive, skill_name, "Candidate");
    let catalog = Catalog::in_memory().expect("catalog");
    catalog.init().expect("catalog schema");
    let ctx = test_context(&root);
    let preview = preview_local_archive_import(
        &catalog,
        &app_data,
        &ctx,
        &SkillManagerLocalArchiveImportParams {
            archive_path: archive.to_string_lossy().to_string(),
            confirmed: false,
            preview_token: None,
            action_reference: None,
        },
    )
    .expect("import preview");
    let destination = tool_global_staging_skills_root(&app_data).join(skill_name);
    install_archive_post_activation_failure(
        app_data
            .canonicalize()
            .expect("canonical app data")
            .join("tool-global/skills")
            .join(skill_name)
            .join("SKILL.md"),
    );
    catalog.inject_next_rollback_failure_for_test();

    let error = apply_local_archive_import(
        &catalog,
        &app_data,
        &ctx,
        &SkillManagerLocalArchiveImportParams {
            archive_path: archive.to_string_lossy().to_string(),
            confirmed: true,
            preview_token: Some(preview.preview_token),
            action_reference: Some(ActionReference::from(&preview.action)),
        },
    )
    .expect_err("rollback fault");

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
        "unknown rollback must preserve the activated import"
    );
    fs::remove_dir_all(root).ok();
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
    let catalog = Catalog::in_memory().expect("catalog");
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
                .starts_with(".archive-update-backup.")
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
fn update_rollback_failure_preserves_replacement_and_backup() {
    crate::initialize_action_preview_secret_for_test([0xA5; 32]).expect("action preview secret");
    let root = unique_root("update-rollback-unknown");
    let home = root.join("home");
    let app_data = root.join("app-data");
    let skill_name = "update-rollback-unknown-skill";
    let skill_dir = home.join(".agents/skills").join(skill_name);
    let archive = root.join("candidate.zip");
    fs::create_dir_all(&skill_dir).expect("skill directory");
    fs::create_dir_all(&app_data).expect("app data");
    fs::write(
        skill_dir.join("SKILL.md"),
        format!("---\nname: {skill_name}\ndescription: Original\n---\n# Original\n"),
    )
    .expect("original skill");
    write_test_archive(&archive, skill_name, "Replacement");
    let catalog = Catalog::in_memory().expect("catalog");
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
        &app_data,
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
    install_archive_post_activation_failure(
        skill_dir
            .join("SKILL.md")
            .canonicalize()
            .expect("canonical target"),
    );
    catalog.inject_next_rollback_failure_for_test();

    let error = apply_local_archive_update(
        &catalog,
        &app_data,
        &ctx,
        &SkillManagerLocalArchiveUpdateParams {
            instance_id: instance.id,
            archive_path: archive.to_string_lossy().to_string(),
            confirmed: true,
            preview_token: Some(preview.preview_token),
            action_reference: Some(ActionReference::from(&preview.action)),
        },
    )
    .expect_err("rollback fault");

    assert!(matches!(
        error,
        CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: true,
            ..
        }
    ));
    assert!(
        fs::read_to_string(skill_dir.join("SKILL.md"))
            .expect("active replacement")
            .contains("Replacement"),
        "unknown rollback must preserve the active replacement"
    );
    assert!(
        fs::read_dir(home.join(".agents/skills"))
            .expect("skills root")
            .filter_map(Result::ok)
            .any(|entry| entry
                .file_name()
                .to_string_lossy()
                .starts_with(".archive-update-backup.")),
        "unknown rollback must preserve the private backup"
    );
    fs::remove_dir_all(root).ok();
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

#[test]
#[cfg(unix)]
fn external_tree_activation_faults_restore_the_exact_original() {
    use crate::external_target::{
        install_external_target_fault, ExternalTargetFaultPoint, ExternalTreeCapability,
    };

    for (label, point) in [
        (
            "backup-stat",
            ExternalTargetFaultPoint::TreeActivationBackupStat,
        ),
        (
            "candidate-stat",
            ExternalTargetFaultPoint::TreeActivationCandidateStat,
        ),
    ] {
        let root = unique_root(label);
        let app_data = root.join("app-data");
        let skills_root = root.join("skills");
        let skill_dir = skills_root.join("fault-skill");
        let skill_path = skill_dir.join("SKILL.md");
        fs::create_dir_all(&app_data).expect("app data");
        fs::create_dir_all(&skill_dir).expect("skill directory");
        fs::write(&skill_path, b"original").expect("original skill");
        let lock = crate::mutation_lock::lock_app_mutations(&app_data).expect("mutation lock");
        let mut capability =
            ExternalTreeCapability::prepare(&lock, &skills_root, &skill_path).expect("capability");
        capability
            .snapshot_target(16, 1024, 1024)
            .expect("bind original tree");
        let staging = crate::external_target::random_external_entry_name("test-staging")
            .expect("staging name");
        capability.create_staging(&staging).expect("create staging");
        let mut candidate = capability
            .create_staging_file(Path::new("SKILL.md"))
            .expect("candidate file");
        candidate.write_all(b"candidate").expect("candidate bytes");
        candidate.sync_all().expect("candidate sync");
        drop(candidate);
        capability.sync_staging().expect("staging sync");
        capability
            .snapshot_staging(16, 1024, 1024)
            .expect("bind staging tree");
        let backup =
            crate::external_target::random_external_entry_name("test-backup").expect("backup name");
        install_external_target_fault(point);

        let error = capability
            .activate(&backup)
            .expect_err("post-rename fault must be typed");
        assert!(matches!(
            error,
            CommandError::PartialEffect {
                state: "outcome_unknown",
                cleanup_required: true,
                ..
            }
        ));
        capability.restore().expect("restore exact original");
        assert_eq!(fs::read(&skill_path).expect("restored skill"), b"original");
        assert_eq!(
            fs::read_dir(&skills_root)
                .expect("skills entries")
                .filter_map(Result::ok)
                .count(),
            1,
            "recovery must remove candidate and backup residues"
        );
        drop(capability);
        drop(lock);
        fs::remove_dir_all(root).ok();
    }
}

#[test]
#[cfg(unix)]
fn external_tree_snapshot_rejects_same_length_in_place_mutation() {
    use crate::external_target::{
        install_external_target_test_hook, ExternalTargetHookPoint, ExternalTreeCapability,
    };

    let root = unique_root("same-length-tree-read");
    let app_data = root.join("app-data");
    let skills_root = root.join("skills");
    let skill_dir = skills_root.join("read-race-skill");
    let skill_path = skill_dir.join("SKILL.md");
    fs::create_dir_all(&app_data).expect("app data");
    fs::create_dir_all(&skill_dir).expect("skill directory");
    fs::write(&skill_path, b"original").expect("original skill");
    let lock = crate::mutation_lock::lock_app_mutations(&app_data).expect("mutation lock");
    let mut capability =
        ExternalTreeCapability::prepare(&lock, &skills_root, &skill_path).expect("capability");
    let raced_path = skill_path.clone();
    install_external_target_test_hook(
        skill_path.clone(),
        ExternalTargetHookPoint::DuringTreeRead,
        move || fs::write(&raced_path, b"attacker").expect("same-length mutation"),
    );

    let error = capability
        .snapshot_target(16, 1024, 1024)
        .expect_err("in-place mutation must stale the tree snapshot");

    assert!(matches!(error, CommandError::StaleActionReference));
    assert_eq!(fs::read(&skill_path).expect("current bytes"), b"attacker");
    drop(capability);
    drop(lock);
    fs::remove_dir_all(root).ok();
}

#[test]
#[cfg(unix)]
fn external_tree_cleanup_fault_retains_private_quarantine() {
    use crate::external_target::{
        install_external_target_fault, ExternalTargetFaultPoint, ExternalTreeCapability,
    };

    let root = unique_root("tree-cleanup-fault");
    let app_data = root.join("app-data");
    let skills_root = root.join("skills");
    let skill_dir = skills_root.join("cleanup-fault-skill");
    let skill_path = skill_dir.join("SKILL.md");
    fs::create_dir_all(&app_data).expect("app data");
    fs::create_dir_all(&skill_dir).expect("skill directory");
    fs::write(&skill_path, b"original").expect("original skill");
    let lock = crate::mutation_lock::lock_app_mutations(&app_data).expect("mutation lock");
    let mut capability =
        ExternalTreeCapability::prepare(&lock, &skills_root, &skill_path).expect("capability");
    capability
        .snapshot_target(16, 1024, 1024)
        .expect("bind original tree");
    let staging =
        crate::external_target::random_external_entry_name("test-staging").expect("staging name");
    capability.create_staging(&staging).expect("create staging");
    let mut candidate = capability
        .create_staging_file(Path::new("SKILL.md"))
        .expect("candidate file");
    candidate.write_all(b"candidate").expect("candidate bytes");
    candidate.sync_all().expect("candidate sync");
    drop(candidate);
    capability.sync_staging().expect("staging sync");
    capability
        .snapshot_staging(16, 1024, 1024)
        .expect("bind staging tree");
    let backup =
        crate::external_target::random_external_entry_name("test-backup").expect("backup name");
    capability.activate(&backup).expect("activate candidate");
    install_external_target_fault(ExternalTargetFaultPoint::TreeQuarantineStat);

    let error = capability
        .finish()
        .expect_err("cleanup fault must preserve quarantine");

    assert!(matches!(
        error,
        CommandError::PartialEffect {
            state: "outcome_unknown",
            cleanup_required: true,
            ..
        }
    ));
    assert_eq!(fs::read(&skill_path).expect("active skill"), b"candidate");
    assert!(
        fs::read_dir(&skills_root)
            .expect("skills entries")
            .filter_map(Result::ok)
            .any(|entry| entry.file_name().to_string_lossy().contains("quarantine")),
        "unverified backup cleanup must leave a private quarantine"
    );
    drop(capability);
    drop(lock);
    fs::remove_dir_all(root).ok();
}
