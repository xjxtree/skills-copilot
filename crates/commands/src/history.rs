use skills_copilot_catalog::{
    Catalog, CatalogError, CatalogPageSnapshot, ConfigSnapshotRecord, SkillEventRecord,
};
use std::path::Path;

use crate::CommandError;

pub fn list_agent_config_snapshot_page_snapshot(
    catalog: &Catalog,
    agent: &str,
    scope: Option<&str>,
    current_project_root: Option<&Path>,
    before: Option<(i64, &str)>,
    limit: usize,
    validate_revision: impl FnOnce(&str) -> Result<(), CatalogError>,
) -> Result<CatalogPageSnapshot<ConfigSnapshotRecord>, CommandError> {
    Ok(catalog.list_agent_config_snapshot_page_snapshot(
        agent,
        scope,
        current_project_root,
        before,
        limit,
        validate_revision,
    )?)
}

pub fn list_skill_event_page_snapshot(
    catalog: &Catalog,
    instance_id: &str,
    before: Option<(i64, i64)>,
    limit: usize,
    validate_revision: impl FnOnce(&str) -> Result<(), CatalogError>,
) -> Result<CatalogPageSnapshot<SkillEventRecord>, CommandError> {
    Ok(catalog.list_skill_event_page_snapshot(instance_id, before, limit, validate_revision)?)
}
