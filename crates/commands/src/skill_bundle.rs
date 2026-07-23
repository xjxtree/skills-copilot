use super::*;

#[derive(Debug, Clone, Serialize)]
pub struct ToolGlobalImportResult {
    pub imported: SkillRecord,
    pub instance_id: String,
    pub source_path: String,
    pub staging_path: String,
    pub findings: Vec<RuleFindingRecord>,
    pub audit: ToolGlobalImportAudit,
}

#[derive(Debug, Clone, Serialize)]
pub struct ToolGlobalImportAudit {
    pub status: &'static str,
    pub read_only_preview: bool,
    pub finding_count: usize,
    pub error_count: usize,
    pub warn_count: usize,
    pub info_count: usize,
    pub conflict_count: usize,
}

#[derive(Debug, Clone)]
pub(crate) struct ParsedToolGlobalSkill {
    pub(crate) frontmatter_raw: String,
    pub(crate) body: String,
    pub(crate) name: String,
    pub(crate) description: String,
    pub(crate) version: Option<String>,
    pub(crate) state: SkillState,
    pub(crate) permissions: PermissionRequest,
}

pub(crate) fn parse_tool_global_skill(content: &str, fallback_name: &str) -> ParsedToolGlobalSkill {
    match parse_tool_global_skill_content(content, fallback_name) {
        Ok(parsed) => parsed,
        Err(message) => ParsedToolGlobalSkill {
            frontmatter_raw: String::new(),
            body: content.to_string(),
            name: fallback_name.to_string(),
            description: message,
            version: None,
            state: SkillState::Broken,
            permissions: PermissionRequest::default(),
        },
    }
}

fn parse_tool_global_skill_content(
    content: &str,
    fallback_name: &str,
) -> Result<ParsedToolGlobalSkill, String> {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .ok_or_else(|| "missing YAML frontmatter".to_string())?;
    let (frontmatter_raw, body) = split_import_frontmatter(rest)?;
    let frontmatter: serde_norway::Value =
        serde_norway::from_str(frontmatter_raw).map_err(|err| err.to_string())?;
    let name = frontmatter
        .get("name")
        .and_then(serde_norway::Value::as_str)
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .unwrap_or(fallback_name)
        .to_string();
    let description = frontmatter
        .get("description")
        .and_then(serde_norway::Value::as_str)
        .map(str::trim)
        .filter(|description| !description.is_empty())
        .map(ToString::to_string)
        .unwrap_or_else(|| first_markdown_paragraph(&body));
    let version = frontmatter
        .get("version")
        .and_then(serde_norway::Value::as_str)
        .map(str::trim)
        .filter(|version| !version.is_empty())
        .map(ToString::to_string);
    let permissions = import_permissions_from_frontmatter(&frontmatter);
    Ok(ParsedToolGlobalSkill {
        frontmatter_raw: frontmatter_raw.to_string(),
        body,
        name,
        description,
        version,
        state: SkillState::Loaded,
        permissions,
    })
}

fn split_import_frontmatter(rest: &str) -> Result<(&str, String), String> {
    if let Some((frontmatter, body)) = rest.split_once("\n---\n") {
        return Ok((frontmatter, body.to_string()));
    }
    if let Some((frontmatter, body)) = rest.split_once("\n---\r\n") {
        return Ok((frontmatter, body.to_string()));
    }
    if let Some(frontmatter) = rest.strip_suffix("\n---") {
        return Ok((frontmatter, String::new()));
    }
    if let Some(frontmatter) = rest.strip_suffix("\r\n---") {
        return Ok((frontmatter, String::new()));
    }
    Err("unterminated YAML frontmatter".to_string())
}

fn first_markdown_paragraph(body: &str) -> String {
    body.split("\n\n")
        .map(str::trim)
        .find(|paragraph| !paragraph.is_empty() && !paragraph.starts_with('#'))
        .unwrap_or_default()
        .lines()
        .map(str::trim)
        .collect::<Vec<_>>()
        .join(" ")
}

fn import_permissions_from_frontmatter(frontmatter: &serde_norway::Value) -> PermissionRequest {
    let tools = yaml_string_list(frontmatter.get("tools"));
    let files = yaml_string_list(
        frontmatter
            .get("permissions")
            .and_then(|permissions| permissions.get("files"))
            .or_else(|| frontmatter.get("files")),
    );
    let network_value = frontmatter
        .get("permissions")
        .and_then(|permissions| permissions.get("network"))
        .or_else(|| frontmatter.get("network"));
    let network_declared = network_value.is_some();
    let network = network_value
        .and_then(serde_norway::Value::as_str)
        .map(|value| match value.trim().to_ascii_lowercase().as_str() {
            "none" => NetworkAccess::None,
            "read-only" | "readonly" | "read_only" => NetworkAccess::ReadOnly,
            "full" => NetworkAccess::Full,
            other => NetworkAccess::Unknown(other.to_string()),
        })
        .unwrap_or(NetworkAccess::None);
    let exec_value = frontmatter
        .get("permissions")
        .and_then(|permissions| permissions.get("exec"))
        .or_else(|| frontmatter.get("exec"));
    let requires_human_value = frontmatter
        .get("permissions")
        .and_then(|permissions| permissions.get("requires_human"))
        .or_else(|| frontmatter.get("requires_human"));

    PermissionRequest {
        tools,
        files,
        network,
        network_declared,
        exec: exec_value
            .and_then(serde_norway::Value::as_bool)
            .unwrap_or(false),
        exec_declared: exec_value.is_some(),
        requires_human: requires_human_value
            .and_then(serde_norway::Value::as_bool)
            .unwrap_or(true),
        requires_human_declared: requires_human_value.is_some(),
    }
}

fn yaml_string_list(value: Option<&serde_norway::Value>) -> Vec<String> {
    match value {
        Some(serde_norway::Value::Sequence(items)) => items
            .iter()
            .filter_map(serde_norway::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect(),
        Some(serde_norway::Value::String(value)) => value
            .split([',', '\n', '\r'])
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

pub(crate) fn stable_tool_global_instance_id(path: &Path) -> String {
    hash_string(&format!(
        "{}|{}|{}",
        AgentId::ToolGlobal.as_str(),
        Scope::ToolGlobal.as_str(),
        path.to_string_lossy()
    ))
}

pub(crate) fn short_hash(value: &str) -> String {
    hash_string(value).chars().take(12).collect()
}

pub(crate) fn import_audit_summary(
    findings: &[RuleFindingRecord],
    conflict_count: usize,
) -> ToolGlobalImportAudit {
    let error_count = findings
        .iter()
        .filter(|finding| finding.severity == "error")
        .count();
    let warn_count = findings
        .iter()
        .filter(|finding| finding.severity == "warn" || finding.severity == "warning")
        .count();
    let info_count = findings
        .iter()
        .filter(|finding| finding.severity == "info")
        .count();
    ToolGlobalImportAudit {
        status: if error_count == 0 {
            "completed"
        } else {
            "issues"
        },
        read_only_preview: true,
        finding_count: findings.len(),
        error_count,
        warn_count,
        info_count,
        conflict_count,
    }
}
