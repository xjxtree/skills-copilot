use super::*;
use serde::Deserialize;
use skills_copilot_catalog::{ConfigSnapshotDraft, SkillEventDraft};
use std::collections::HashSet;

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
