use super::*;
use serde::Deserialize;
use skills_copilot_catalog::{CatalogError, ConfigSnapshotDraft, SkillEventDraft};
use std::{collections::HashSet, sync::mpsc, thread, time::Duration};

#[derive(Debug, Deserialize)]
struct ConfigPageRow {
    id: String,
}

#[derive(Debug, Deserialize)]
struct EventPageRow {
    id: i64,
}

trait DispatchValue {
    fn dispatch_value(&self, method: &str, params: Value) -> Result<Value, ServiceError>;
}

impl DispatchValue for ServiceHost {
    fn dispatch_value(&self, method: &str, params: Value) -> Result<Value, ServiceError> {
        self.handle_result(ServiceRequest {
            id: None,
            method: method.to_string(),
            params,
        })
    }
}

fn local_catalog_fixture(label: &str) -> Result<(PathBuf, ServiceHost, Catalog), ServiceError> {
    let root = env::temp_dir().join(format!(
        "skills-copilot-{label}-{}-{}",
        std::process::id(),
        unique_suffix(),
    ));
    fs::create_dir_all(&root)?;
    let host = test_host(root.join("app-data"));
    fs::create_dir_all(&host.app_data_dir)?;
    let catalog = Catalog::open(&host.catalog_path())?;
    catalog.init()?;
    Ok((root, host, catalog))
}

fn seed_agent_config_snapshot(
    catalog: &Catalog,
    id: &str,
    created_at_ms: i64,
) -> Result<(), ServiceError> {
    catalog.create_config_snapshot(ConfigSnapshotDraft {
        id,
        agent: "claude-code",
        scope: "agent-global",
        target: "/tmp/home/.claude/settings.json",
        content: "{}\n",
        reason: "pre-toggle",
        created_at_ms,
    })?;
    Ok(())
}

fn seed_skill_event(
    catalog: &Catalog,
    instance_id: &str,
    occurred_at_ms: i64,
) -> Result<(), ServiceError> {
    catalog.create_skill_event(SkillEventDraft {
        instance_id,
        kind: "updated",
        payload: "{}",
        occurred_at_ms,
    })?;
    Ok(())
}

fn collect_config_pages(
    host: &ServiceHost,
    agent: &str,
    limit: usize,
) -> Result<Vec<ConfigPageRow>, ServiceError> {
    let mut rows = Vec::new();
    let mut cursor = None;
    let mut source_revision = None;
    loop {
        let page = host.dispatch_value(
            "snapshot.listAgentConfigPage",
            json!({
                "agent": agent,
                "limit": limit,
                "cursor": cursor,
                "source_revision": source_revision,
            }),
        )?;
        rows.extend(serde_json::from_value::<Vec<ConfigPageRow>>(
            page["records"].clone(),
        )?);
        if !page["has_more"].as_bool().unwrap_or(false) {
            break;
        }
        cursor = page["next_cursor"].as_str().map(str::to_string);
        source_revision = page["source_revision"].as_str().map(str::to_string);
    }
    Ok(rows)
}

fn collect_event_pages(
    host: &ServiceHost,
    instance_id: &str,
    limit: usize,
) -> Result<Vec<EventPageRow>, ServiceError> {
    let mut rows = Vec::new();
    let mut cursor = None;
    let mut source_revision = None;
    loop {
        let page = host.dispatch_value(
            "skill.listEventsPage",
            json!({
                "instance_id": instance_id,
                "limit": limit,
                "cursor": cursor,
                "source_revision": source_revision,
            }),
        )?;
        rows.extend(serde_json::from_value::<Vec<EventPageRow>>(
            page["records"].clone(),
        )?);
        if !page["has_more"].as_bool().unwrap_or(false) {
            break;
        }
        cursor = page["next_cursor"].as_str().map(str::to_string);
        source_revision = page["source_revision"].as_str().map(str::to_string);
    }
    Ok(rows)
}

#[test]
fn config_and_event_pages_match_legacy_order() -> Result<(), ServiceError> {
    let (root, host, catalog) = local_catalog_fixture("list-page-order")?;
    let instance_id = "equal-time-instance".to_string();
    for index in 0..6 {
        seed_agent_config_snapshot(&catalog, &format!("snapshot-{index}"), 1_800_000_000_000)?;
        catalog.create_skill_event(SkillEventDraft {
            instance_id: &instance_id,
            kind: "updated",
            payload: "{}",
            occurred_at_ms: 1_800_000_000_000,
        })?;
    }

    let legacy_snapshots = list_agent_config_snapshots(&catalog, "claude-code", None)?;
    let paged_snapshots = collect_config_pages(&host, "claude-code", 2)?;
    assert_eq!(
        paged_snapshots
            .iter()
            .map(|row| &row.id)
            .collect::<Vec<_>>(),
        legacy_snapshots
            .iter()
            .map(|row| &row.id)
            .collect::<Vec<_>>(),
    );
    let legacy_events = list_skill_events(&catalog, &instance_id, None)?;
    let paged_events = collect_event_pages(&host, &instance_id, 2)?;
    assert_eq!(
        paged_events.iter().map(|row| &row.id).collect::<Vec<_>>(),
        legacy_events.iter().map(|row| &row.id).collect::<Vec<_>>(),
    );
    assert_eq!(
        paged_snapshots
            .iter()
            .map(|row| &row.id)
            .collect::<HashSet<_>>()
            .len(),
        6
    );
    assert_eq!(
        paged_events
            .iter()
            .map(|row| &row.id)
            .collect::<HashSet<_>>()
            .len(),
        6
    );

    drop(catalog);
    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn changed_catalog_revision_rejects_continuation() -> Result<(), ServiceError> {
    let (root, host, catalog) = local_catalog_fixture("list-page-mutation")?;
    for index in 0..3 {
        seed_agent_config_snapshot(&catalog, &format!("snapshot-{index}"), 1_800_000_000_000)?;
    }

    let first = host.dispatch_value(
        "snapshot.listAgentConfigPage",
        json!({
            "agent": "claude-code", "limit": 2
        }),
    )?;
    seed_agent_config_snapshot(&catalog, "inserted-after-first-page", 1_900_000_000_000)?;
    let error = host
        .dispatch_value(
            "snapshot.listAgentConfigPage",
            json!({
                "agent": "claude-code", "limit": 2,
                "cursor": first["next_cursor"], "source_revision": first["source_revision"]
            }),
        )
        .expect_err("changed source must reject continuation");
    assert_eq!(error.code(), "source_changed");

    drop(catalog);
    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn config_page_revision_and_rows_share_one_read_snapshot() -> Result<(), ServiceError> {
    let (root, host, catalog) = local_catalog_fixture("config-page-atomic")?;
    rusqlite::Connection::open(host.catalog_path())
        .map_err(CatalogError::from)?
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(CatalogError::from)?;
    for index in 0..3 {
        seed_agent_config_snapshot(&catalog, &format!("snapshot-{index}"), 1_800_000_000_000)?;
    }
    let first = host.dispatch_value(
        "snapshot.listAgentConfigPage",
        json!({"agent": "claude-code", "limit": 2}),
    )?;
    let expected_revision = first["source_revision"]
        .as_str()
        .expect("first revision")
        .to_string();
    let expected_ids = serde_json::from_value::<Vec<ConfigPageRow>>(first["records"].clone())?
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();

    let reader = Catalog::open_read_only(&host.catalog_path())?;
    let writer_path = host.catalog_path();
    let (start_writer_tx, start_writer_rx) = mpsc::channel();
    let (writer_done_tx, writer_done_rx) = mpsc::channel();
    let writer = thread::spawn(move || {
        start_writer_rx.recv().expect("start config writer");
        let writer_catalog = Catalog::open(&writer_path).expect("open config writer");
        writer_catalog
            .create_config_snapshot(ConfigSnapshotDraft {
                id: "snapshot-concurrent",
                agent: "claude-code",
                scope: "agent-global",
                target: "/tmp/home/.claude/settings.json",
                content: "{}\n",
                reason: "pre-toggle",
                created_at_ms: 1_900_000_000_000,
            })
            .expect("insert concurrent config snapshot");
        writer_done_tx.send(()).expect("finish config writer");
    });

    let atomic = reader.list_agent_config_snapshot_page_snapshot(
        "claude-code",
        None,
        None,
        2,
        |revision| {
            if revision != expected_revision {
                return Err(CatalogError::SourceChanged);
            }
            start_writer_tx.send(()).expect("release config writer");
            writer_done_rx
                .recv_timeout(Duration::from_secs(5))
                .expect("config writer commits during read snapshot");
            Ok(())
        },
    )?;
    writer.join().expect("join config writer");
    let atomic_ids = atomic
        .records
        .iter()
        .map(|row| row.id.clone())
        .collect::<Vec<_>>();
    assert_eq!(
        &atomic_ids[..expected_ids.len()],
        expected_ids.as_slice(),
        "revision and page rows must come from the same pre-write snapshot",
    );
    assert!(!atomic_ids.iter().any(|id| id == "snapshot-concurrent"));
    let error = host
        .dispatch_value(
            "snapshot.listAgentConfigPage",
            json!({
                "agent": "claude-code",
                "limit": 2,
                "cursor": first["next_cursor"],
                "source_revision": first["source_revision"],
            }),
        )
        .expect_err("continuation must reject the committed config revision");
    assert_eq!(error.code(), "source_changed");

    drop(catalog);
    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn event_page_revision_and_rows_share_one_read_snapshot() -> Result<(), ServiceError> {
    let (root, host, catalog) = local_catalog_fixture("event-page-atomic")?;
    rusqlite::Connection::open(host.catalog_path())
        .map_err(CatalogError::from)?
        .pragma_update(None, "journal_mode", "WAL")
        .map_err(CatalogError::from)?;
    let instance_id = "atomic-event-instance";
    for _ in 0..3 {
        seed_skill_event(&catalog, instance_id, 1_800_000_000_000)?;
    }
    let first = host.dispatch_value(
        "skill.listEventsPage",
        json!({"instance_id": instance_id, "limit": 2}),
    )?;
    let expected_revision = first["source_revision"]
        .as_str()
        .expect("first revision")
        .to_string();
    let expected_ids = serde_json::from_value::<Vec<EventPageRow>>(first["records"].clone())?
        .into_iter()
        .map(|row| row.id)
        .collect::<Vec<_>>();

    let reader = Catalog::open_read_only(&host.catalog_path())?;
    let writer_path = host.catalog_path();
    let (start_writer_tx, start_writer_rx) = mpsc::channel();
    let (writer_done_tx, writer_done_rx) = mpsc::channel();
    let writer = thread::spawn(move || {
        start_writer_rx.recv().expect("start event writer");
        let writer_catalog = Catalog::open(&writer_path).expect("open event writer");
        writer_catalog
            .create_skill_event(SkillEventDraft {
                instance_id: "atomic-event-instance",
                kind: "updated",
                payload: "{}",
                occurred_at_ms: 1_900_000_000_000,
            })
            .expect("insert concurrent event");
        writer_done_tx.send(()).expect("finish event writer");
    });

    let atomic = reader.list_skill_event_page_snapshot(instance_id, None, 2, |revision| {
        if revision != expected_revision {
            return Err(CatalogError::SourceChanged);
        }
        start_writer_tx.send(()).expect("release event writer");
        writer_done_rx
            .recv_timeout(Duration::from_secs(5))
            .expect("event writer commits during read snapshot");
        Ok(())
    })?;
    writer.join().expect("join event writer");
    let atomic_ids = atomic.records.iter().map(|row| row.id).collect::<Vec<_>>();
    assert_eq!(
        &atomic_ids[..expected_ids.len()],
        expected_ids.as_slice(),
        "event revision and rows must come from the same pre-write snapshot",
    );
    assert_eq!(
        atomic_ids.len(),
        3,
        "the concurrent fourth event must not enter the read snapshot"
    );
    let error = host
        .dispatch_value(
            "skill.listEventsPage",
            json!({
                "instance_id": instance_id,
                "limit": 2,
                "cursor": first["next_cursor"],
                "source_revision": first["source_revision"],
            }),
        )
        .expect_err("continuation must reject the committed event revision");
    assert_eq!(error.code(), "source_changed");

    drop(catalog);
    let _ = fs::remove_dir_all(root);
    Ok(())
}

#[test]
fn local_history_cursor_binding_and_limit_clamps_are_stable() -> Result<(), ServiceError> {
    let (root, host, catalog) = local_catalog_fixture("list-page-boundaries")?;
    for index in 0..105 {
        seed_agent_config_snapshot(&catalog, &format!("snapshot-{index:03}"), index)?;
    }
    let one = host.dispatch_value(
        "snapshot.listAgentConfigPage",
        json!({"agent": "claude-code", "limit": 0}),
    )?;
    assert_eq!(one["returned_count"].as_u64(), Some(1));
    let hundred = host.dispatch_value(
        "snapshot.listAgentConfigPage",
        json!({"agent": "claude-code", "limit": 101}),
    )?;
    assert_eq!(hundred["returned_count"].as_u64(), Some(100));

    for (method, params) in [
        (
            "skill.listEventsPage",
            json!({"instance_id": "unused", "cursor": hundred["next_cursor"]}),
        ),
        (
            "snapshot.listAgentConfigPage",
            json!({"agent": "codex", "cursor": hundred["next_cursor"]}),
        ),
        (
            "snapshot.listAgentConfigPage",
            json!({"agent": "claude-code", "cursor": "v1:not-hex"}),
        ),
    ] {
        let error = host
            .dispatch_value(method, params)
            .expect_err("foreign, query-mismatched, and malformed cursors must fail closed");
        assert_eq!(error.code(), "invalid_request");
    }

    drop(catalog);
    let _ = fs::remove_dir_all(root);
    Ok(())
}
