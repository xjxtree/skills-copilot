use skills_copilot_catalog::{Catalog, ConfigSnapshotRecord, SkillEventRecord};

use crate::CommandError;

pub fn list_agent_config_snapshot_page(
    catalog: &Catalog,
    agent: &str,
    scope: Option<&str>,
    before: Option<(i64, &str)>,
    limit: usize,
) -> Result<Vec<ConfigSnapshotRecord>, CommandError> {
    Ok(catalog.list_agent_config_snapshot_page(agent, scope, before, limit)?)
}

pub fn list_agent_config_snapshot_revision_metadata(
    catalog: &Catalog,
    agent: &str,
    scope: Option<&str>,
) -> Result<Vec<(String, i64)>, CommandError> {
    Ok(catalog.list_agent_config_snapshot_revision_metadata(agent, scope)?)
}

pub fn list_skill_event_page(
    catalog: &Catalog,
    instance_id: &str,
    before: Option<(i64, i64)>,
    limit: usize,
) -> Result<Vec<SkillEventRecord>, CommandError> {
    Ok(catalog.list_skill_event_page(instance_id, before, limit)?)
}

pub fn list_skill_event_revision_metadata(
    catalog: &Catalog,
    instance_id: &str,
) -> Result<Vec<(i64, i64)>, CommandError> {
    Ok(catalog.list_skill_event_revision_metadata(instance_id)?)
}
