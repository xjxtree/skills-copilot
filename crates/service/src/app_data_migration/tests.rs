use super::*;
use std::{
    fs,
    path::{Path, PathBuf},
    sync::{Arc, Barrier},
    thread,
};

#[cfg(unix)]
use std::os::unix::fs::{symlink, PermissionsExt};

fn fixture(label: &str) -> (PathBuf, PathBuf, PathBuf) {
    let root = std::env::temp_dir().join(format!(
        "agent-copilot-migration-{label}-{}-{}",
        std::process::id(),
        now_unix_millis()
    ));
    let parent = root.join("app-data-parent");
    let source = parent.join("legacy");
    let target = parent.join("current");
    fs::create_dir_all(&source).expect("create legacy source");
    (root, source, target)
}

fn assert_no_owned_staging(target: &Path) {
    let parent = target.parent().expect("target parent");
    let prefix = format!(
        ".{}.migration.",
        target.file_name().expect("target name").to_string_lossy()
    );
    let staging = fs::read_dir(parent)
        .expect("read migration parent")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().starts_with(&prefix))
        .collect::<Vec<_>>();
    assert!(
        staging.is_empty(),
        "migration must not retain private staging: {staging:?}"
    );
}

fn assert_partial(result: Result<(), ServiceError>) {
    assert!(matches!(
        result,
        Err(ServiceError::Command(CommandError::PartialEffect {
            operation,
            state: "outcome_unknown",
            cleanup_required: true,
            ..
        })) if operation == "legacy_app_data_migration"
    ));
}

#[test]
#[cfg(unix)]
fn verified_directory_rejects_a_device_outside_the_accepted_tree() {
    use std::os::unix::fs::MetadataExt;

    let (root, source, _) = fixture("cross-device");
    let child_name = OsStr::new("mounted-child");
    fs::create_dir(source.join(child_name)).expect("create child");
    let source_directory = File::open(&source).expect("open source");
    let stat = rustix::fs::statat(
        &source_directory,
        child_name,
        rustix::fs::AtFlags::SYMLINK_NOFOLLOW,
    )
    .expect("stat child");
    let source_metadata = source_directory.metadata().expect("source metadata");

    let result = open_verified_directory(
        &source_directory,
        child_name,
        &stat,
        source_metadata.uid(),
        source_metadata.dev().wrapping_add(1),
    );

    assert!(matches!(result, Err(ServiceError::InvalidRequest(_))));
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn migration_copies_a_stable_tree_privately_and_writes_a_path_free_marker() {
    let (root, source, target) = fixture("normal");
    let nested = source.join("llm");
    fs::create_dir(&nested).expect("create nested source");
    fs::write(source.join("project-context.json"), b"{\"active\":null}\n")
        .expect("write source file");
    fs::write(
        nested.join("provider-profiles.json"),
        b"{\"profiles\":[]}\n",
    )
    .expect("write nested source file");
    fs::set_permissions(&source, fs::Permissions::from_mode(0o755)).expect("set source mode");
    fs::set_permissions(&nested, fs::Permissions::from_mode(0o755))
        .expect("set nested source mode");
    fs::set_permissions(
        source.join("project-context.json"),
        fs::Permissions::from_mode(0o644),
    )
    .expect("set source file mode");

    migrate_legacy_app_data_dir(&source, &target).expect("migrate legacy app data");

    assert_eq!(
        fs::read(target.join("project-context.json")).expect("read migrated file"),
        b"{\"active\":null}\n"
    );
    assert_eq!(
        fs::read(target.join("llm/provider-profiles.json")).expect("read nested migrated file"),
        b"{\"profiles\":[]}\n"
    );
    assert_eq!(
        fs::metadata(&target)
            .expect("target metadata")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(target.join("llm"))
            .expect("nested metadata")
            .permissions()
            .mode()
            & 0o777,
        0o700
    );
    assert_eq!(
        fs::metadata(target.join("project-context.json"))
            .expect("file metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    assert_eq!(
        fs::metadata(target.join("llm/provider-profiles.json"))
            .expect("nested file metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let marker_path = target.join(MIGRATION_MARKER_NAME);
    assert_eq!(
        fs::metadata(&marker_path)
            .expect("marker metadata")
            .permissions()
            .mode()
            & 0o777,
        0o600
    );
    let marker = fs::read_to_string(marker_path).expect("read marker");
    assert!(marker.contains(LEGACY_BUNDLE_ID));
    assert!(marker.contains(DEFAULT_BUNDLE_ID));
    assert!(
        !marker.contains(&root.to_string_lossy().to_string()),
        "migration marker must not persist private paths"
    );
    assert!(
        source.exists(),
        "migration must preserve the legacy source tree"
    );
    assert_no_owned_staging(&target);
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn concurrent_migrations_serialize_on_the_shared_parent_and_activate_once() {
    let (root, source, target) = fixture("concurrent");
    fs::write(source.join("state.json"), b"stable").expect("seed source");
    let barrier = Arc::new(Barrier::new(3));
    let mut workers = Vec::new();
    for _ in 0..2 {
        let barrier = Arc::clone(&barrier);
        let source = source.clone();
        let target = target.clone();
        workers.push(thread::spawn(move || {
            barrier.wait();
            migrate_legacy_app_data_dir(&source, &target)
        }));
    }
    barrier.wait();
    for worker in workers {
        worker
            .join()
            .expect("migration worker")
            .expect("serialized migration");
    }
    assert_eq!(
        fs::read(target.join("state.json")).expect("migrated state"),
        b"stable"
    );
    assert_no_owned_staging(&target);
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn source_and_target_symlinks_are_rejected_without_touching_victims() {
    for linked_side in ["source", "target"] {
        let (root, source, target) = fixture(linked_side);
        let victim = root.join("victim");
        fs::create_dir(&victim).expect("create victim");
        fs::write(victim.join("sentinel"), b"unchanged").expect("seed victim");
        let victim_mode = fs::metadata(&victim)
            .expect("victim metadata")
            .permissions()
            .mode()
            & 0o777;
        if linked_side == "source" {
            fs::remove_dir(&source).expect("remove source");
            symlink(&victim, &source).expect("link source");
        } else {
            symlink(&victim, &target).expect("link target");
        }

        assert!(migrate_legacy_app_data_dir(&source, &target).is_err());
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
        let _ = if linked_side == "source" {
            fs::remove_file(&source)
        } else {
            fs::remove_file(&target)
        };
        let _ = fs::remove_dir_all(root);
    }
}

#[test]
#[cfg(unix)]
fn an_intermediate_symlink_is_rejected_before_migration_effects() {
    let root = std::env::temp_dir().join(format!(
        "agent-copilot-migration-intermediate-{}-{}",
        std::process::id(),
        now_unix_millis()
    ));
    let real_parent = root.join("real-parent");
    let linked_parent = root.join("linked-parent");
    let source = linked_parent.join("legacy");
    let target = linked_parent.join("current");
    fs::create_dir_all(real_parent.join("legacy")).expect("create real legacy");
    fs::write(real_parent.join("legacy/state"), b"unchanged").expect("seed source");
    symlink(&real_parent, &linked_parent).expect("link intermediate parent");

    assert!(migrate_legacy_app_data_dir(&source, &target).is_err());
    assert!(!real_parent.join("current").exists());
    assert_eq!(
        fs::read(real_parent.join("legacy/state")).expect("source state"),
        b"unchanged"
    );
    let _ = fs::remove_file(linked_parent);
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn hardlinked_source_file_is_rejected_and_the_victim_is_unchanged() {
    let (root, source, target) = fixture("hardlink");
    let victim = root.join("victim");
    fs::write(&victim, b"unchanged").expect("seed victim");
    fs::hard_link(&victim, source.join("linked-state")).expect("hardlink source");

    assert!(migrate_legacy_app_data_dir(&source, &target).is_err());
    assert_eq!(fs::read(&victim).expect("victim bytes"), b"unchanged");
    assert!(!target.exists());
    assert_no_owned_staging(&target);
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn nested_symlink_failure_removes_only_the_owned_staging_tree() {
    let (root, source, target) = fixture("nested-symlink");
    let victim = root.join("victim");
    fs::write(&victim, b"unchanged").expect("seed victim");
    symlink(&victim, source.join("linked-state")).expect("link nested source");

    assert!(migrate_legacy_app_data_dir(&source, &target).is_err());
    assert_eq!(fs::read(&victim).expect("victim bytes"), b"unchanged");
    assert!(!target.exists());
    assert_no_owned_staging(&target);
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn special_source_file_is_rejected_and_owned_staging_is_cleaned() {
    use std::os::unix::net::UnixListener;

    let root =
        PathBuf::from("/tmp").join(format!("acm{}{}", std::process::id(), now_unix_millis()));
    let source = root.join("p/l");
    let target = root.join("p/c");
    fs::create_dir_all(&source).expect("create short source path");
    let _listener = UnixListener::bind(source.join("s")).expect("bind source socket");

    assert!(migrate_legacy_app_data_dir(&source, &target).is_err());
    assert!(!target.exists());
    assert_no_owned_staging(&target);
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn target_race_is_not_overwritten_and_owned_staging_is_cleaned() {
    let (root, source, target) = fixture("target-race");
    fs::write(source.join("state"), b"legacy").expect("seed source");
    let mut injected = false;
    let result = migrate_legacy_app_data_dir_with_hook(&source, &target, &mut |point, context| {
        if point == MigrationPoint::BeforeActivation && !injected {
            injected = true;
            fs::create_dir(context.target_path).expect("create competing target");
            fs::write(context.target_path.join("sentinel"), b"winner")
                .expect("seed competing target");
        }
    });

    assert!(result.is_err());
    assert_eq!(
        fs::read(target.join("sentinel")).expect("competing target sentinel"),
        b"winner"
    );
    assert!(!target.join("state").exists());
    assert_no_owned_staging(&target);
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn parent_rebind_before_the_first_effect_fails_closed_without_touching_replacement() {
    let (root, source, target) = fixture("parent-prebind");
    fs::write(source.join("state"), b"legacy").expect("seed source");
    let parent = source.parent().expect("source parent").to_path_buf();
    let moved_parent = root.join("moved-parent");
    let mut injected = false;
    let result = migrate_legacy_app_data_dir_with_hook(&source, &target, &mut |point, context| {
        if point == MigrationPoint::BeforeFirstEffect && !injected {
            injected = true;
            fs::rename(context.parent_path, &moved_parent).expect("move locked parent");
            fs::create_dir(context.parent_path).expect("create replacement parent");
            fs::write(context.parent_path.join("sentinel"), b"unchanged")
                .expect("seed replacement");
        }
    });

    assert!(result.is_err());
    assert_eq!(
        fs::read(parent.join("sentinel")).expect("replacement sentinel"),
        b"unchanged"
    );
    assert!(!parent.join("current").exists());
    assert!(!moved_parent.join("current").exists());
    assert_no_owned_staging(&moved_parent.join("current"));
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn source_rebind_after_activation_is_a_partial_effect_and_does_not_follow_the_link() {
    let (root, source, target) = fixture("source-postbind");
    fs::write(source.join("state"), b"legacy").expect("seed source");
    let moved_source = source.with_file_name("legacy-moved");
    let victim = root.join("victim");
    fs::create_dir(&victim).expect("create victim");
    fs::write(victim.join("sentinel"), b"unchanged").expect("seed victim");
    let mut injected = false;
    let result = migrate_legacy_app_data_dir_with_hook(&source, &target, &mut |point, context| {
        if point == MigrationPoint::AfterActivation && !injected {
            injected = true;
            fs::rename(context.source_path, &moved_source).expect("move source");
            symlink(&victim, context.source_path).expect("replace source with link");
        }
    });

    assert_partial(result);
    assert_eq!(
        fs::read(victim.join("sentinel")).expect("victim sentinel"),
        b"unchanged"
    );
    assert_eq!(
        fs::read(target.join("state")).expect("target state"),
        b"legacy"
    );
    let _ = fs::remove_file(&source);
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn target_rebind_after_activation_is_a_partial_effect_and_preserves_the_replacement() {
    let (root, source, target) = fixture("target-postbind");
    fs::write(source.join("state"), b"legacy").expect("seed source");
    let moved_target = target.with_file_name("activated-target");
    let mut injected = false;
    let result = migrate_legacy_app_data_dir_with_hook(&source, &target, &mut |point, context| {
        if point == MigrationPoint::AfterActivation && !injected {
            injected = true;
            fs::rename(context.target_path, &moved_target).expect("move activated target");
            fs::create_dir(context.target_path).expect("create replacement target");
            fs::write(context.target_path.join("sentinel"), b"unchanged")
                .expect("seed replacement target");
        }
    });

    assert_partial(result);
    assert_eq!(
        fs::read(target.join("sentinel")).expect("replacement sentinel"),
        b"unchanged"
    );
    assert_eq!(
        fs::read(moved_target.join("state")).expect("activated target state"),
        b"legacy"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn parent_rebind_after_activation_is_a_partial_effect_and_preserves_the_replacement() {
    let (root, source, target) = fixture("parent-postbind");
    fs::write(source.join("state"), b"legacy").expect("seed source");
    let parent = source.parent().expect("source parent").to_path_buf();
    let moved_parent = root.join("activated-parent");
    let mut injected = false;
    let result = migrate_legacy_app_data_dir_with_hook(&source, &target, &mut |point, context| {
        if point == MigrationPoint::AfterActivation && !injected {
            injected = true;
            fs::rename(context.parent_path, &moved_parent).expect("move activated parent");
            fs::create_dir(context.parent_path).expect("create replacement parent");
            fs::write(context.parent_path.join("sentinel"), b"unchanged")
                .expect("seed replacement parent");
        }
    });

    assert_partial(result);
    assert_eq!(
        fs::read(parent.join("sentinel")).expect("replacement sentinel"),
        b"unchanged"
    );
    assert_eq!(
        fs::read(moved_parent.join("current/state")).expect("activated state"),
        b"legacy"
    );
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn a_colliding_staging_name_is_never_opened_changed_or_deleted() {
    let (root, source, target) = fixture("staging-collision");
    fs::write(source.join("state"), b"legacy").expect("seed source");
    let mut collision: Option<PathBuf> = None;
    migrate_legacy_app_data_dir_with_hook(&source, &target, &mut |point, context| {
        if point == MigrationPoint::BeforeStagingCreate && collision.is_none() {
            let path = context
                .parent_path
                .join(context.staging_name.expect("staging candidate"));
            fs::create_dir(&path).expect("create staging collision");
            fs::write(path.join("sentinel"), b"unchanged").expect("seed collision");
            collision = Some(path);
        }
    })
    .expect("migration skips collision");

    let collision = collision.expect("collision path");
    assert_eq!(
        fs::read(collision.join("sentinel")).expect("collision sentinel"),
        b"unchanged"
    );
    assert_eq!(
        fs::read(target.join("state")).expect("target state"),
        b"legacy"
    );
    fs::remove_dir_all(collision).expect("remove test collision");
    assert_no_owned_staging(&target);
    let _ = fs::remove_dir_all(root);
}

#[test]
#[cfg(unix)]
fn cleanup_refuses_a_rebound_staging_root_and_reports_partial_effect() {
    let (root, source, target) = fixture("staging-rebind");
    fs::write(source.join("state"), b"legacy").expect("seed source");
    let moved_staging = Arc::new(std::sync::Mutex::new(None::<PathBuf>));
    let moved_for_hook = Arc::clone(&moved_staging);
    let mut injected = false;
    let result = migrate_legacy_app_data_dir_with_hook(&source, &target, &mut |point, context| {
        if point == MigrationPoint::AfterStagingCreated && !injected {
            injected = true;
            let staging = context
                .parent_path
                .join(context.staging_name.expect("staging name"));
            let moved = context.parent_path.join("moved-private-staging");
            fs::rename(&staging, &moved).expect("move owned staging");
            fs::create_dir(&staging).expect("create staging replacement");
            fs::write(staging.join("sentinel"), b"unchanged").expect("seed replacement");
            *moved_for_hook.lock().expect("lock moved staging") = Some(moved);
        }
    });

    assert_partial(result);
    let rebound = fs::read_dir(target.parent().expect("target parent"))
        .expect("read parent")
        .filter_map(Result::ok)
        .find(|entry| {
            entry
                .file_name()
                .to_string_lossy()
                .starts_with(".current.migration.")
        })
        .expect("rebound staging");
    assert_eq!(
        fs::read(rebound.path().join("sentinel")).expect("replacement sentinel"),
        b"unchanged"
    );
    assert!(
        moved_staging
            .lock()
            .expect("lock moved staging")
            .as_ref()
            .is_some_and(|path| path.exists()),
        "unbound original staging must be retained for explicit cleanup"
    );
    let _ = fs::remove_dir_all(root);
}
