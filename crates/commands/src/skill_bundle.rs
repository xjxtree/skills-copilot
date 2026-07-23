use super::*;

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct ExportedSkillMetadata {
    pub name: String,
    pub description: String,
    pub skill_path: String,
    pub source_agent: String,
    pub source_scope: String,
    pub version: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportedSkillManifest {
    pub manifest_version: u32,
    pub bundle_format: String,
    pub metadata: ExportedSkillMetadata,
    pub fingerprint: String,
    pub permissions: serde_json::Value,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ExportedSkillBundle {
    pub manifest_path: PathBuf,
    pub bundle_path: PathBuf,
    pub fingerprint: String,
    pub metadata: ExportedSkillMetadata,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ReimportedSkillBundle {
    pub fingerprint: String,
    pub metadata: ExportedSkillMetadata,
    pub permissions: serde_json::Value,
}

struct ExportSkillSource {
    agent: String,
    scope: String,
    frontmatter_raw: String,
    body: String,
    name: String,
    description: String,
    version: Option<String>,
    permissions: serde_json::Value,
    fingerprint: String,
}

pub fn export_skill_bundle(
    catalog: &Catalog,
    instance_id: &str,
    output_dir: &Path,
) -> Result<ExportedSkillBundle, CommandError> {
    let detail = get_skill(catalog, instance_id)?;
    let version = version_from_frontmatter(&detail.frontmatter_raw);
    let source = ExportSkillSource {
        agent: detail.agent,
        scope: detail.scope,
        frontmatter_raw: detail.frontmatter_raw,
        body: detail.body,
        name: detail.name,
        description: detail.description,
        version,
        permissions: stable_permissions(detail.permissions),
        fingerprint: detail.fingerprint,
    };
    write_export_bundle(source, output_dir)
}

pub fn export_staging_skill_bundle(
    source_path: &Path,
    output_dir: &Path,
) -> Result<ExportedSkillBundle, CommandError> {
    let source = read_staging_skill_source(source_path)?;
    write_export_bundle(source, output_dir)
}

pub fn reimport_skill_bundle(bundle_path: &Path) -> Result<ReimportedSkillBundle, CommandError> {
    let manifest_path = bundle_path.join("manifest.json");
    let manifest_text = fs::read_to_string(&manifest_path)?;
    let manifest: ExportedSkillManifest = serde_json::from_str(&manifest_text)?;
    validate_relative_bundle_path(&manifest.metadata.skill_path)?;
    let skill_path = bundle_path.join(&manifest.metadata.skill_path);
    let parsed = parse_export_skill_file(&skill_path)?;
    let fingerprint = content_fingerprint(&parsed.frontmatter_raw, &parsed.body);
    if fingerprint != manifest.fingerprint {
        return Err(CommandError::InvalidSkillBundle(format!(
            "manifest fingerprint {} does not match bundle content {}",
            manifest.fingerprint, fingerprint
        )));
    }
    Ok(ReimportedSkillBundle {
        fingerprint,
        metadata: manifest.metadata,
        permissions: manifest.permissions,
    })
}

fn write_export_bundle(
    source: ExportSkillSource,
    output_dir: &Path,
) -> Result<ExportedSkillBundle, CommandError> {
    let bundle_path = output_dir.join(safe_bundle_dir_name(&source.name));
    let skill_relative_path = "skill/SKILL.md";
    let skill_dir = bundle_path.join("skill");
    fs::create_dir_all(&skill_dir)?;
    fs::write(
        skill_dir.join("SKILL.md"),
        skill_file_content(&source.frontmatter_raw, &source.body),
    )?;

    let metadata = ExportedSkillMetadata {
        name: source.name,
        description: source.description,
        skill_path: skill_relative_path.to_string(),
        source_agent: source.agent,
        source_scope: source.scope,
        version: source.version,
    };
    let manifest = ExportedSkillManifest {
        manifest_version: 1,
        bundle_format: "skills-copilot.tool-global.v2.9".to_string(),
        metadata: metadata.clone(),
        fingerprint: source.fingerprint.clone(),
        permissions: source.permissions,
    };
    let manifest_path = bundle_path.join("manifest.json");
    let manifest_text = serde_json::to_string_pretty(&manifest)?;
    fs::write(&manifest_path, format!("{manifest_text}\n"))?;

    Ok(ExportedSkillBundle {
        manifest_path,
        bundle_path,
        fingerprint: source.fingerprint,
        metadata,
    })
}

struct ParsedExportSkill {
    frontmatter_raw: String,
    body: String,
    name: String,
    description: String,
    version: Option<String>,
    permissions: serde_json::Value,
}

fn read_staging_skill_source(path: &Path) -> Result<ExportSkillSource, CommandError> {
    let parsed = parse_export_skill_file(path)?;
    let fingerprint = content_fingerprint(&parsed.frontmatter_raw, &parsed.body);
    Ok(ExportSkillSource {
        agent: "skills-copilot".to_string(),
        scope: Scope::ToolGlobal.as_str().to_string(),
        frontmatter_raw: parsed.frontmatter_raw,
        body: parsed.body,
        name: parsed.name,
        description: parsed.description,
        version: parsed.version,
        permissions: parsed.permissions,
        fingerprint,
    })
}

fn parse_export_skill_file(path: &Path) -> Result<ParsedExportSkill, CommandError> {
    let skill_path = if path.is_dir() {
        path.join("SKILL.md")
    } else {
        path.to_path_buf()
    };
    if skill_path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md") {
        return Err(CommandError::InvalidSkillSource(format!(
            "expected a skill directory or SKILL.md path, got {}",
            path.display()
        )));
    }
    let content = fs::read_to_string(&skill_path)?;
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .ok_or_else(|| {
            CommandError::InvalidSkillSource(format!(
                "{} is missing YAML frontmatter",
                skill_path.display()
            ))
        })?;
    let (frontmatter_raw, body) = split_export_frontmatter(rest).ok_or_else(|| {
        CommandError::InvalidSkillSource(format!(
            "{} has unterminated YAML frontmatter",
            skill_path.display()
        ))
    })?;
    let frontmatter: serde_norway::Value = serde_norway::from_str(frontmatter_raw)
        .map_err(|err| CommandError::InvalidSkillSource(err.to_string()))?;
    let name = yaml_string(&frontmatter, "name")
        .ok_or_else(|| CommandError::InvalidSkillSource("missing skill name".to_string()))?;
    let description = yaml_string(&frontmatter, "description").unwrap_or_else(|| {
        body.lines()
            .map(str::trim)
            .find(|line| !line.is_empty() && !line.starts_with('#'))
            .unwrap_or_default()
            .to_string()
    });
    let version = yaml_string(&frontmatter, "version");
    let permissions = permissions_from_frontmatter(&frontmatter);
    Ok(ParsedExportSkill {
        frontmatter_raw: frontmatter_raw.to_string(),
        body,
        name,
        description,
        version,
        permissions,
    })
}

fn split_export_frontmatter(rest: &str) -> Option<(&str, String)> {
    if let Some((frontmatter, body)) = rest.split_once("\n---\n") {
        return Some((frontmatter, body.to_string()));
    }
    if let Some((frontmatter, body)) = rest.split_once("\n---\r\n") {
        return Some((frontmatter, body.to_string()));
    }
    rest.strip_suffix("\n---")
        .or_else(|| rest.strip_suffix("\r\n---"))
        .map(|frontmatter| (frontmatter, String::new()))
}

fn skill_file_content(frontmatter_raw: &str, body: &str) -> String {
    let mut content = String::from("---\n");
    content.push_str(frontmatter_raw);
    content.push_str("\n---\n");
    content.push_str(body);
    content
}

fn version_from_frontmatter(frontmatter_raw: &str) -> Option<String> {
    serde_norway::from_str::<serde_norway::Value>(frontmatter_raw)
        .ok()
        .and_then(|value| yaml_string(&value, "version"))
}

fn yaml_string(value: &serde_norway::Value, key: &str) -> Option<String> {
    value
        .get(key)
        .and_then(serde_norway::Value::as_str)
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

pub(crate) fn permissions_from_frontmatter(frontmatter: &serde_norway::Value) -> serde_json::Value {
    let mut permissions = serde_json::Map::new();
    if let Some(tools) = yaml_string_vec(frontmatter.get("tools"))
        .or_else(|| yaml_string_vec(frontmatter.get("allowed-tools")))
    {
        permissions.insert("tools".to_string(), string_array_value(tools));
    }
    if let Some(files) = yaml_string_vec(frontmatter.get("files")) {
        permissions.insert("files".to_string(), string_array_value(files));
    }
    if let Some(network) = yaml_string(frontmatter, "network")
        .or_else(|| yaml_nested_string(frontmatter, &["permissions", "network"]))
    {
        permissions.insert("network".to_string(), serde_json::Value::String(network));
    }
    if let Some(exec) = yaml_bool(frontmatter, "exec")
        .or_else(|| yaml_nested_bool(frontmatter, &["permissions", "exec"]))
    {
        permissions.insert("exec".to_string(), exec.into());
    }
    if let Some(requires_human) = yaml_bool(frontmatter, "requires_human")
        .or_else(|| yaml_nested_bool(frontmatter, &["permissions", "requires_human"]))
    {
        permissions.insert("requires_human".to_string(), requires_human.into());
    }
    serde_json::Value::Object(permissions)
}

fn yaml_string_vec(value: Option<&serde_norway::Value>) -> Option<Vec<String>> {
    match value? {
        serde_norway::Value::Sequence(items) => items
            .iter()
            .map(|item| item.as_str().map(ToString::to_string))
            .collect(),
        serde_norway::Value::String(raw) => Some(
            raw.split(',')
                .map(str::trim)
                .filter(|item| !item.is_empty())
                .map(ToString::to_string)
                .collect(),
        ),
        _ => None,
    }
}

fn yaml_bool(value: &serde_norway::Value, key: &str) -> Option<bool> {
    value.get(key).and_then(serde_norway::Value::as_bool)
}

fn yaml_nested_string(value: &serde_norway::Value, path: &[&str]) -> Option<String> {
    yaml_nested_value(value, path)?
        .as_str()
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToString::to_string)
}

fn yaml_nested_bool(value: &serde_norway::Value, path: &[&str]) -> Option<bool> {
    yaml_nested_value(value, path)?.as_bool()
}

fn yaml_nested_value<'a>(
    value: &'a serde_norway::Value,
    path: &[&str],
) -> Option<&'a serde_norway::Value> {
    let mut current = value;
    for key in path {
        current = current.get(*key)?;
    }
    Some(current)
}

fn string_array_value(values: Vec<String>) -> serde_json::Value {
    serde_json::Value::Array(values.into_iter().map(serde_json::Value::String).collect())
}

fn stable_permissions(value: serde_json::Value) -> serde_json::Value {
    match value {
        serde_json::Value::Object(map) => {
            let mut stable = serde_json::Map::new();
            for key in ["tools", "files", "network", "exec", "requires_human"] {
                if let Some(value) = map.get(key) {
                    stable.insert(key.to_string(), value.clone());
                }
            }
            serde_json::Value::Object(stable)
        }
        _ => serde_json::json!({}),
    }
}

fn validate_relative_bundle_path(path: &str) -> Result<(), CommandError> {
    let candidate = Path::new(path);
    if candidate.is_absolute() || path.contains("..") {
        return Err(CommandError::InvalidSkillBundle(format!(
            "manifest skill_path must be relative and contained: {path}"
        )));
    }
    Ok(())
}

fn safe_bundle_dir_name(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .map(|ch| {
            if ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_') {
                ch
            } else {
                '-'
            }
        })
        .collect();
    let trimmed = sanitized.trim_matches('-');
    if trimmed.is_empty() {
        "skill".to_string()
    } else {
        trimmed.to_string()
    }
}

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

pub fn import_local_skill_to_tool_global(
    catalog: &Catalog,
    ctx: &AdapterContext,
    staging_root: &Path,
    source_path: &Path,
) -> Result<ToolGlobalImportResult, CommandError> {
    reject_symlink(source_path, "import source")?;
    let source_dir = canonical_import_source(source_path)?;
    let source_skill_path = source_dir.join("SKILL.md");
    if !source_skill_path.is_file() {
        return Err(CommandError::InvalidImportSource(format!(
            "{} does not contain SKILL.md",
            source_dir.display()
        )));
    }
    reject_symlink(&source_dir, "import source")?;
    reject_symlink(&source_skill_path, "import SKILL.md")?;

    let source_content = fs::read_to_string(&source_skill_path)?;
    let parsed = parse_tool_global_skill(
        &source_content,
        source_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("imported-skill"),
    );
    fs::create_dir_all(staging_root)?;
    reject_symlink(staging_root, "tool-global staging root")?;
    let staging_skills_root = staging_root.join("skills");
    fs::create_dir_all(&staging_skills_root)?;
    reject_symlink(&staging_skills_root, "tool-global staging skills root")?;
    let canonical_staging_skills_root = staging_skills_root.canonicalize()?;
    let destination_dir = canonical_staging_skills_root.join(format!(
        "{}-{}",
        canonical_skill_name_suggestion(&parsed.name),
        short_hash(&source_dir.to_string_lossy())
    ));
    ensure_path_inside(
        &destination_dir,
        &canonical_staging_skills_root,
        "staging destination",
    )?;

    let temp_dir = canonical_staging_skills_root.join(format!(
        ".{}.tmp-{}",
        destination_dir
            .file_name()
            .and_then(|name| name.to_str())
            .unwrap_or("import"),
        current_time_ms()
    ));
    ensure_path_inside(
        &temp_dir,
        &canonical_staging_skills_root,
        "staging temp destination",
    )?;
    if temp_dir.exists() {
        fs::remove_dir_all(&temp_dir)?;
    }
    if let Err(error) =
        copy_skill_dir_to_staging(&source_dir, &temp_dir, &canonical_staging_skills_root)
    {
        let _ = fs::remove_dir_all(&temp_dir);
        return Err(error);
    }
    if destination_dir.exists() {
        fs::remove_dir_all(&destination_dir)?;
    }
    fs::rename(&temp_dir, &destination_dir)?;

    let staged_skill_path = destination_dir.join("SKILL.md").canonicalize()?;
    ensure_path_inside(
        &staged_skill_path,
        &canonical_staging_skills_root,
        "staged skill path",
    )?;
    register_tool_global_staged_skill(catalog, ctx, &source_dir, &staged_skill_path)
}

pub fn import_github_skill_to_tool_global_deferred(url: &str) -> Result<(), CommandError> {
    Err(CommandError::UnsupportedImportSource(format!(
        "GitHub repo import is explicitly deferred; provide a local source_path after cloning or unpacking the repo yourself. Requested URL: {url}"
    )))
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

fn canonical_import_source(source_path: &Path) -> Result<PathBuf, CommandError> {
    if !source_path.exists() {
        return Err(CommandError::InvalidImportSource(format!(
            "{} does not exist",
            source_path.display()
        )));
    }
    let source_dir = source_path.canonicalize()?;
    if !source_dir.is_dir() {
        return Err(CommandError::InvalidImportSource(format!(
            "{} is not a directory",
            source_dir.display()
        )));
    }
    Ok(source_dir)
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

fn copy_skill_dir_to_staging(
    source_dir: &Path,
    destination_dir: &Path,
    staging_root: &Path,
) -> Result<(), CommandError> {
    ensure_path_inside(destination_dir, staging_root, "staging destination")?;
    fs::create_dir_all(destination_dir)?;
    for entry in fs::read_dir(source_dir)? {
        let entry = entry?;
        let file_type = entry.file_type()?;
        let file_name = entry.file_name();
        let source = entry.path();
        let destination = destination_dir.join(file_name);
        ensure_path_inside(&destination, staging_root, "staging copy target")?;
        if file_type.is_symlink() {
            return Err(CommandError::InvalidImportSource(format!(
                "{} is a symlink; tool-global import does not follow source symlinks",
                source.display()
            )));
        }
        if file_type.is_dir() {
            copy_skill_dir_to_staging(&source, &destination, staging_root)?;
        } else if file_type.is_file() {
            fs::copy(&source, &destination)?;
        }
    }
    Ok(())
}

pub(crate) fn ensure_path_inside(
    path: &Path,
    root: &Path,
    label: &str,
) -> Result<(), CommandError> {
    let normalized_path = normalize_path_lexically(path);
    let normalized_root = normalize_path_lexically(root);
    if !normalized_path.starts_with(&normalized_root) {
        return Err(CommandError::UnsafeConfigPath(format!(
            "{label} {} resolves outside staging root {}",
            path.display(),
            root.display()
        )));
    }
    Ok(())
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
