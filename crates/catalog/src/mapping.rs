use super::*;

pub(crate) fn record_matches_project_context(
    scope: &str,
    record_project_root: Option<&str>,
    project_context: Option<Option<&Path>>,
) -> bool {
    let Some(current_project_root) = project_context else {
        return true;
    };
    if scope != Scope::AgentProject.as_str() {
        return true;
    }
    let (Some(record_project_root), Some(current_project_root)) =
        (record_project_root, current_project_root)
    else {
        return false;
    };
    same_project_root(Path::new(record_project_root), current_project_root)
}

pub(crate) fn same_project_root(record_project_root: &Path, current_project_root: &Path) -> bool {
    if record_project_root == current_project_root {
        return true;
    }
    match (
        record_project_root.canonicalize(),
        current_project_root.canonicalize(),
    ) {
        (Ok(record), Ok(current)) => record == current,
        _ => false,
    }
}

#[derive(Debug, Clone, Eq, PartialEq, Serialize)]
pub struct ConfigSnapshotRecord {
    pub id: String,
    pub agent: String,
    pub scope: String,
    pub project_root: Option<String>,
    pub target: String,
    pub content: String,
    pub reason: String,
    pub created_at: i64,
}

/// Bundled parameters for [`Catalog::create_config_snapshot`]. Avoids a
/// long parameter list at the call site and keeps the snapshot payload
/// self-describing.
#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConfigSnapshotDraft<'a> {
    pub id: &'a str,
    pub agent: &'a str,
    pub scope: &'a str,
    pub project_root: Option<&'a str>,
    pub target: &'a str,
    pub content: &'a str,
    pub reason: &'a str,
    pub created_at_ms: i64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SkillEventDraft<'a> {
    pub instance_id: &'a str,
    pub kind: &'a str,
    pub payload: &'a str,
    pub occurred_at_ms: i64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct RuleFindingDraft {
    pub id: String,
    pub instance_id: Option<String>,
    pub definition_id: Option<String>,
    pub rule_id: String,
    pub severity: String,
    pub message: String,
    pub suggestion: Option<String>,
    pub created_at: i64,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct SkillDefinitionDraft {
    pub id: String,
    pub canonical_name: String,
    pub description: String,
    pub active_instance: Option<String>,
    pub has_multiple_instances: bool,
    pub has_conflict: bool,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ConflictGroupDraft {
    pub id: String,
    pub definition_id: String,
    pub reason: String,
    pub winner_id: Option<String>,
    pub instance_ids: Vec<String>,
}

pub(crate) fn parse_agent_id(s: &str) -> Option<AgentId> {
    match s {
        "tool-global" => Some(AgentId::ToolGlobal),
        "claude-code" => Some(AgentId::ClaudeCode),
        "codex" => Some(AgentId::Codex),
        "pi" => Some(AgentId::Pi),
        "hermes" => Some(AgentId::Hermes),
        "openclaw" => Some(AgentId::Openclaw),
        "opencode" => Some(AgentId::Opencode),
        _ => None,
    }
}

pub(crate) fn parse_scope(s: &str) -> Option<Scope> {
    match s {
        "tool-global" => Some(Scope::ToolGlobal),
        "agent-global" => Some(Scope::AgentGlobal),
        "agent-project" => Some(Scope::AgentProject),
        _ => None,
    }
}

pub(crate) fn parse_skill_state(s: &str) -> SkillState {
    match s {
        "loaded" => SkillState::Loaded,
        "disabled" => SkillState::Disabled,
        "shadowed" => SkillState::Shadowed,
        "missing" => SkillState::Missing,
        _ => SkillState::Broken,
    }
}

pub(crate) fn skill_instance_from_row(row: &Row<'_>) -> rusqlite::Result<SkillInstance> {
    let agent_str: String = row.get(1)?;
    let scope_str: String = row.get(2)?;
    let state_str: String = row.get(10)?;
    let permissions_raw: String = row.get(14)?;
    let agent = parse_agent_id(&agent_str).ok_or_else(|| {
        rusqlite::Error::InvalidParameterName(format!("unknown agent: {agent_str}"))
    })?;
    let scope = parse_scope(&scope_str).ok_or_else(|| {
        rusqlite::Error::InvalidParameterName(format!("unknown scope: {scope_str}"))
    })?;
    let name: String = row.get(7)?;

    Ok(SkillInstance {
        id: row.get(0)?,
        agent,
        scope,
        project_root: row.get::<_, Option<String>>(3)?.map(PathBuf::from),
        path: PathBuf::from(row.get::<_, String>(4)?),
        display_path: PathBuf::from(row.get::<_, String>(5)?),
        definition_id: row.get(6)?,
        name: name.clone(),
        display_name: name,
        description: row.get(8)?,
        version: row.get(9)?,
        state: parse_skill_state(&state_str),
        enabled: row.get::<_, i64>(11)? != 0,
        frontmatter_raw: row.get(12)?,
        body: row.get(13)?,
        scripts: Vec::new(),
        permissions: parse_permissions_json(&permissions_raw),
        fingerprint: row.get(15)?,
        mtime: row.get(16)?,
        first_seen: row.get(17)?,
        last_seen: row.get(18)?,
    })
}

pub(crate) fn catalog_path_has_skill_shape(agent: &str, path: &Path) -> bool {
    match agent {
        "pi" => {
            if path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md") {
                return false;
            }
            if path
                .parent()
                .and_then(Path::file_name)
                .and_then(|name| name.to_str())
                == Some("skills")
            {
                return false;
            }
            !path
                .components()
                .any(|component| component.as_os_str().to_str() == Some("references"))
        }
        "hermes" | "openclaw" | "opencode" => {
            path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md")
        }
        _ => true,
    }
}

pub(crate) fn dedup_skill_records(records: Vec<SkillRecord>) -> Vec<SkillRecord> {
    let mut by_identity: HashMap<(String, String, String), usize> = HashMap::new();
    let mut deduped: Vec<SkillRecord> = Vec::new();

    for record in records {
        let key = (
            record.agent.clone(),
            record.scope.clone(),
            record.path.to_string_lossy().into_owned(),
        );
        if let Some(&index) = by_identity.get(&key) {
            if catalog_state_rank(&record.state) < catalog_state_rank(&deduped[index].state) {
                deduped[index] = record;
            }
            continue;
        }

        by_identity.insert(key, deduped.len());
        deduped.push(record);
    }

    deduped
}

pub(crate) fn dedup_skill_instances(instances: Vec<SkillInstance>) -> Vec<SkillInstance> {
    let mut by_identity: HashMap<(String, String, String), usize> = HashMap::new();
    let mut deduped: Vec<SkillInstance> = Vec::new();

    for instance in instances {
        let key = (
            instance.agent.as_str().to_string(),
            instance.scope.as_str().to_string(),
            instance.path.to_string_lossy().into_owned(),
        );
        if let Some(&index) = by_identity.get(&key) {
            if skill_state_rank(&instance.state) < skill_state_rank(&deduped[index].state) {
                deduped[index] = instance;
            }
            continue;
        }

        by_identity.insert(key, deduped.len());
        deduped.push(instance);
    }

    deduped
}

pub(crate) fn catalog_state_rank(state: &str) -> usize {
    match state {
        "loaded" | "disabled" => 0,
        "broken" => 1,
        "shadowed" => 2,
        "missing" => 3,
        _ => 4,
    }
}

pub(crate) fn skill_state_rank(state: &SkillState) -> usize {
    match state {
        SkillState::Loaded | SkillState::Disabled => 0,
        SkillState::Broken => 1,
        SkillState::Shadowed => 2,
        SkillState::Missing => 3,
    }
}

pub(crate) fn skill_event_from_row(row: &Row<'_>) -> rusqlite::Result<SkillEventRecord> {
    let payload_raw: String = row.get(3)?;
    Ok(SkillEventRecord {
        id: row.get(0)?,
        instance_id: row.get(1)?,
        kind: row.get(2)?,
        payload: serde_json::from_str(&payload_raw).unwrap_or_else(|_| {
            serde_json::json!({
                "raw": payload_raw,
                "parse_error": true
            })
        }),
        occurred_at: row.get(4)?,
    })
}

pub(crate) fn finding_triage_from_row(row: &Row<'_>) -> rusqlite::Result<FindingTriageRecord> {
    Ok(FindingTriageRecord {
        triage_key: row.get(0)?,
        triage_context: row.get(1)?,
        status: row.get(2)?,
        note: row.get(3)?,
        updated_at: row.get(4)?,
    })
}

pub(crate) fn rule_tuning_from_row(row: &Row<'_>) -> rusqlite::Result<RuleTuningRecord> {
    let agent: String = row.get(1)?;
    let scope: String = row.get(2)?;
    Ok(RuleTuningRecord {
        rule_id: row.get(0)?,
        agent: empty_string_as_none(agent),
        scope: empty_string_as_none(scope),
        severity_override: row.get(3)?,
        suppression_reason: row.get(4)?,
        suppression_note: row.get(5)?,
        updated_at: row.get(6)?,
    })
}

pub(crate) fn rule_tuning_key(agent: Option<&str>, scope: Option<&str>) -> (String, String) {
    (
        agent.unwrap_or_default().trim().to_string(),
        scope.unwrap_or_default().trim().to_string(),
    )
}

pub(crate) fn empty_string_as_none(value: String) -> Option<String> {
    if value.is_empty() {
        None
    } else {
        Some(value)
    }
}

pub fn migration_count() -> usize {
    9
}

pub(crate) fn finding_triage_key(finding: &RuleFindingDraft, triage_context: &str) -> String {
    stable_hash(&format!(
        "{}\x1f{}\x1f{}\x1f{}\x1f{}\x1f{}",
        finding.instance_id.as_deref().unwrap_or(""),
        finding.definition_id.as_deref().unwrap_or(""),
        finding.rule_id,
        finding.message,
        finding.suggestion.as_deref().unwrap_or(""),
        triage_context
    ))
}

pub(crate) fn stable_hash(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    format!("{:x}", hasher.finalize())
}

pub(crate) fn permissions_json(inst: &SkillInstance) -> Result<String, serde_json::Error> {
    let mut value = serde_json::Map::new();
    if !inst.permissions.tools.is_empty() {
        value.insert(
            "tools".to_string(),
            serde_json::Value::Array(
                inst.permissions
                    .tools
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
    }
    if !inst.permissions.files.is_empty() {
        value.insert(
            "files".to_string(),
            serde_json::Value::Array(
                inst.permissions
                    .files
                    .iter()
                    .cloned()
                    .map(serde_json::Value::String)
                    .collect(),
            ),
        );
    }
    if inst.permissions.network_declared {
        value.insert(
            "network".to_string(),
            serde_json::Value::String(network_access_key(&inst.permissions.network).to_string()),
        );
    }
    if inst.permissions.exec_declared || inst.permissions.exec {
        value.insert("exec".to_string(), inst.permissions.exec.into());
    }
    if inst.permissions.requires_human_declared {
        value.insert(
            "requires_human".to_string(),
            inst.permissions.requires_human.into(),
        );
    }
    serde_json::to_string(&serde_json::Value::Object(value))
}

pub(crate) fn parse_permissions_json(raw: &str) -> PermissionRequest {
    let value = match serde_json::from_str::<serde_json::Value>(raw) {
        Ok(serde_json::Value::Object(value)) => value,
        _ => return PermissionRequest::default(),
    };

    let mut permissions = PermissionRequest::default();
    if let Some(tools) = parse_string_array(value.get("tools")) {
        permissions.tools = tools;
    }
    if let Some(files) = parse_string_array(value.get("files")) {
        permissions.files = files;
    }
    if let Some(network_value) = value.get("network") {
        permissions.network_declared = true;
        permissions.network = match network_value.as_str() {
            Some(raw) => {
                parse_network_access(raw).unwrap_or_else(|| NetworkAccess::Unknown(raw.to_string()))
            }
            None => NetworkAccess::Unknown(network_value.to_string()),
        };
    }
    if let Some(exec) = value.get("exec").and_then(serde_json::Value::as_bool) {
        permissions.exec = exec;
        permissions.exec_declared = true;
    }
    if let Some(requires_human) = value
        .get("requires_human")
        .and_then(serde_json::Value::as_bool)
    {
        permissions.requires_human = requires_human;
        permissions.requires_human_declared = true;
    }
    permissions
}

pub(crate) fn parse_string_array(value: Option<&serde_json::Value>) -> Option<Vec<String>> {
    value?
        .as_array()?
        .iter()
        .map(|item| item.as_str().map(ToString::to_string))
        .collect()
}

pub(crate) fn parse_network_access(value: &str) -> Option<NetworkAccess> {
    match value {
        "none" => Some(NetworkAccess::None),
        "read-only" => Some(NetworkAccess::ReadOnly),
        "full" => Some(NetworkAccess::Full),
        _ => None,
    }
}

pub(crate) fn network_access_key(access: &NetworkAccess) -> &str {
    match access {
        NetworkAccess::None => "none",
        NetworkAccess::ReadOnly => "read-only",
        NetworkAccess::Full => "full",
        NetworkAccess::Unknown(raw) => raw.as_str(),
    }
}
