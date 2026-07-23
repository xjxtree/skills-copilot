use super::*;

#[cfg(not(unix))]
pub(super) fn register_tool_global_staged_skill(
    catalog: &Catalog,
    ctx: &AdapterContext,
    source_path: &Path,
    staged_skill_path: &Path,
) -> Result<ToolGlobalImportResult, CommandError> {
    let staged_content = fs::read_to_string(staged_skill_path)?;
    let metadata = fs::metadata(staged_skill_path)?;
    let mtime = metadata
        .modified()
        .ok()
        .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
        .map(|duration| duration.as_millis() as i64)
        .unwrap_or_default();
    register_tool_global_staged_skill_content(
        catalog,
        ctx,
        source_path,
        staged_skill_path,
        &staged_content,
        mtime,
    )
}

pub(super) fn register_tool_global_staged_skill_content(
    catalog: &Catalog,
    ctx: &AdapterContext,
    source_path: &Path,
    staged_skill_path: &Path,
    staged_content: &str,
    mtime: i64,
) -> Result<ToolGlobalImportResult, CommandError> {
    let staged = parse_tool_global_skill(
        staged_content,
        staged_skill_path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("imported-skill"),
    );
    let instance = SkillInstance {
        id: stable_tool_global_instance_id(staged_skill_path),
        agent: AgentId::ToolGlobal,
        scope: Scope::ToolGlobal,
        project_root: None,
        path: staged_skill_path.to_path_buf(),
        display_path: staged_skill_path.to_path_buf(),
        definition_id: hash_string(&staged.name.to_ascii_lowercase()),
        name: staged.name.clone(),
        display_name: staged.name.clone(),
        description: staged.description.clone(),
        version: staged.version.clone(),
        state: staged.state,
        enabled: true,
        frontmatter_raw: staged.frontmatter_raw.clone(),
        body: staged.body.clone(),
        scripts: Vec::new(),
        permissions: staged.permissions.clone(),
        fingerprint: hash_string(&format!("{}\n---\n{}", staged.frontmatter_raw, staged.body)),
        mtime,
        first_seen: mtime,
        last_seen: mtime,
    };
    let previous_fingerprints = catalog.instance_fingerprints()?;
    catalog.upsert_skill_instance(&instance)?;
    refresh_catalog_rule_outputs(catalog, ctx, previous_fingerprints)?;

    let imported = catalog
        .get_skill_record(&instance.id)?
        .ok_or_else(|| CommandError::InstanceNotFound(instance.id.clone()))?;
    let findings: Vec<RuleFindingRecord> = list_findings(catalog)?
        .into_iter()
        .filter(|finding| {
            finding.instance_id.as_deref() == Some(instance.id.as_str())
                || finding.definition_id.as_deref() == Some(instance.definition_id.as_str())
        })
        .collect();
    let audit = import_audit_summary(&findings, list_conflicts(catalog)?.len());
    Ok(ToolGlobalImportResult {
        imported,
        instance_id: instance.id,
        source_path: source_path.to_string_lossy().to_string(),
        staging_path: staged_skill_path.to_string_lossy().to_string(),
        findings,
        audit,
    })
}

pub(super) fn tool_global_skill_name_from_content(content: &str, fallback: &str) -> String {
    parse_tool_global_skill(content, fallback).name
}
