use std::{
    cmp::Ordering,
    collections::{BTreeMap, BTreeSet, HashSet},
    fs,
    io::Read,
    path::{Path, PathBuf},
};

use crate::shared::{required_frontmatter_string, split_yaml_frontmatter, stable_path_id};
use skills_copilot_core::{
    AdapterContext, AdapterError, AdapterRoot, AgentAdapter, AgentConfigAdapter,
    AgentConfigDocument, AgentId, PermissionRequest, RootSource, Scope, SkillInstance, SkillState,
};

mod paths;
pub use paths::codex_home_dir;
#[cfg(test)]
use paths::resolved_codex_home;

#[derive(Debug, Default)]
pub struct CodexAdapter;

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct CodexSkillConfigEntry {
    pub path: Option<String>,
    pub enabled: Option<bool>,
}

const MAX_PLUGIN_MARKETPLACES: usize = 128;
const MAX_PLUGINS_PER_MARKETPLACE: usize = 1_024;
const MAX_VERSIONS_PER_PLUGIN: usize = 128;
const MAX_PLUGIN_MANIFEST_BYTES: u64 = 256 * 1024;

pub fn codex_plugin_cache_id(codex_home: &Path, skill_path: &Path) -> Option<String> {
    let cache_root = codex_home.join("plugins/cache");
    let relative = match skill_path.strip_prefix(&cache_root) {
        Ok(relative) => relative.to_path_buf(),
        Err(_) => {
            let canonical_cache_root = cache_root.canonicalize().ok()?;
            let canonical_skill_path = skill_path.canonicalize().ok()?;
            canonical_skill_path
                .strip_prefix(canonical_cache_root)
                .ok()?
                .to_path_buf()
        }
    };
    let mut components = relative.components();
    let publisher = normal_component(components.next()?)?;
    let package = normal_component(components.next()?)?;
    normal_component(components.next()?)?;
    let remainder = components
        .map(normal_component)
        .collect::<Option<Vec<_>>>()?;
    if remainder.len() < 2 || remainder.last() != Some(&"SKILL.md") {
        return None;
    }
    Some(format!("{package}@{publisher}"))
}

fn normal_component(component: std::path::Component<'_>) -> Option<&str> {
    match component {
        std::path::Component::Normal(value) => value.to_str(),
        _ => None,
    }
}

pub fn parse_codex_enabled_plugin_ids(text: &str) -> BTreeSet<String> {
    parse_codex_plugin_states(text)
        .into_iter()
        .filter_map(|(plugin_id, enabled)| enabled.then_some(plugin_id))
        .collect()
}

pub fn parse_codex_plugin_states(text: &str) -> BTreeMap<String, bool> {
    let mut plugin_states = BTreeMap::new();
    let mut current_plugin = None;

    for raw_line in text.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            current_plugin = parse_codex_plugin_section_id(line);
            continue;
        }
        let Some(plugin_id) = current_plugin.as_ref() else {
            continue;
        };
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "enabled" {
            if let Ok(enabled) = parse_toml_bool(value.trim()) {
                plugin_states.insert(plugin_id.clone(), enabled);
            }
        }
    }

    plugin_states
}

pub fn codex_plugin_is_effectively_enabled(
    plugin_id: &str,
    plugin_states: &BTreeMap<String, bool>,
) -> bool {
    plugin_states.get(plugin_id).copied().unwrap_or(false)
}

fn parse_codex_plugin_section_id(line: &str) -> Option<String> {
    let header = line.strip_prefix("[plugins.")?;
    let closing = header.find(']')?;
    validate_toml_trailing(&header[closing + 1..]).ok()?;
    let inner = header[..closing].trim();
    if inner.starts_with('\'') || inner.starts_with('"') {
        parse_toml_string(inner).ok()
    } else if !inner.is_empty()
        && inner
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '-' | '_'))
    {
        Some(inner.to_string())
    } else {
        None
    }
}

impl AgentAdapter for CodexAdapter {
    fn id(&self) -> AgentId {
        AgentId::Codex
    }

    fn display_name(&self) -> &'static str {
        "Codex"
    }

    fn roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
        let shared_root = AdapterRoot {
            scope: Scope::AgentGlobal,
            path: ctx.user_home.join(".agents/skills"),
            source: RootSource::UserHome,
        };
        let mut roots = vec![shared_root.clone()];
        roots.extend(codex_declared_symlink_roots(&shared_root));

        roots.push(AdapterRoot {
            scope: Scope::AgentGlobal,
            path: codex_home_dir(ctx).join("skills"),
            source: RootSource::Compatibility,
        });

        roots.extend(codex_plugin_skill_roots(ctx));

        if let Some(project_root) = &ctx.project_root {
            let project_roots = codex_project_skill_roots(project_root, ctx.project_cwd.as_deref());
            for project_root in &project_roots {
                roots.extend(codex_declared_symlink_roots(project_root));
            }
            roots.extend(project_roots);
        }

        roots.push(AdapterRoot {
            scope: Scope::AgentGlobal,
            path: PathBuf::from("/etc/codex/skills"),
            source: RootSource::Admin,
        });
        dedup_roots(roots)
    }

    fn link_target_roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
        let mut roots = codex_declared_symlink_roots(&AdapterRoot {
            scope: Scope::AgentGlobal,
            path: ctx.user_home.join(".agents/skills"),
            source: RootSource::Compatibility,
        });
        if let Some(project_root) = &ctx.project_root {
            for root in codex_project_skill_roots(project_root, ctx.project_cwd.as_deref()) {
                roots.extend(codex_declared_symlink_roots(&root));
            }
        }
        for root in &mut roots {
            root.source = RootSource::Configured;
        }
        roots
    }

    fn parse(&self, path: &Path) -> Result<SkillInstance, AdapterError> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| AdapterError::new(format!("failed to read skill: {err}")))?;
        self.parse_content(path, content)
    }

    fn parse_content(&self, path: &Path, content: String) -> Result<SkillInstance, AdapterError> {
        let fallback_name = path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
        let parsed = parse_skill_content(&content);
        let (frontmatter_raw, body, name, description, state, enabled) = match parsed {
            Ok(parsed) => (
                parsed.frontmatter_raw,
                parsed.body,
                parsed.name.clone(),
                parsed.description,
                SkillState::Loaded,
                true,
            ),
            Err(message) => (
                String::new(),
                content,
                fallback_name.clone(),
                message,
                SkillState::Broken,
                false,
            ),
        };

        Ok(SkillInstance {
            id: stable_path_id("codex", path),
            agent: AgentId::Codex,
            scope: Scope::AgentProject,
            project_root: None,
            path: PathBuf::from(path),
            display_path: PathBuf::from(path),
            definition_id: name.clone(),
            name: name.clone(),
            display_name: name,
            description,
            version: None,
            state,
            enabled,
            frontmatter_raw,
            body,
            scripts: Vec::new(),
            permissions: PermissionRequest::default(),
            fingerprint: String::new(),
            mtime: 0,
            first_seen: 0,
            last_seen: 0,
        })
    }

    fn is_enabled(&self, instance: &SkillInstance) -> bool {
        instance.enabled
    }

    fn accepts_skill_path(&self, root: &AdapterRoot, relative_path: &Path) -> bool {
        let components = relative_path
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .collect::<Vec<_>>();
        (components.len() == 2
            || (components.len() == 1
                && (root.source == RootSource::Plugin
                    || fs::symlink_metadata(&root.path)
                        .is_ok_and(|metadata| metadata.file_type().is_symlink())))
            || (root.source == RootSource::Compatibility
                && components.len() == 3
                && components.first() == Some(&".system")))
            && components.last() == Some(&"SKILL.md")
    }

    fn config_paths(&self, ctx: &AdapterContext) -> Vec<PathBuf> {
        let mut paths = vec![codex_user_config_path(ctx)];
        if let Some(project_root) = &ctx.project_root {
            paths.extend(
                codex_project_directories(project_root, ctx.project_cwd.as_deref())
                    .map(|directory| directory.join(".codex/config.toml")),
            );
        }
        if let Some(profile) = active_codex_profile(&paths[0]) {
            paths.push(codex_home_dir(ctx).join(format!("{profile}.config.toml")));
        }
        paths.push(PathBuf::from("/etc/codex/config.toml"));
        paths.dedup();
        paths
    }
}

fn codex_declared_symlink_roots(parent: &AdapterRoot) -> Vec<AdapterRoot> {
    let Ok(entries) = fs::read_dir(&parent.path) else {
        return Vec::new();
    };
    let mut roots = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_type()
                .is_ok_and(|file_type| file_type.is_symlink())
                && fs::metadata(entry.path()).is_ok_and(|metadata| metadata.is_dir())
        })
        .map(|entry| AdapterRoot {
            scope: parent.scope,
            path: entry.path(),
            source: RootSource::Compatibility,
        })
        .collect::<Vec<_>>();
    roots.sort_by(|left, right| left.path.cmp(&right.path));
    roots
}

fn codex_plugin_skill_roots(ctx: &AdapterContext) -> Vec<AdapterRoot> {
    let cache_root = codex_home_dir(ctx).join("plugins/cache");
    let plugin_states = fs::read_to_string(codex_user_config_path(ctx))
        .map(|content| parse_codex_plugin_states(&content))
        .unwrap_or_default();
    let mut roots = Vec::new();
    for marketplace_root in bounded_child_directories(&cache_root, MAX_PLUGIN_MARKETPLACES) {
        let Some(publisher) = marketplace_root.file_name().and_then(|name| name.to_str()) else {
            continue;
        };
        for plugin_root in bounded_child_directories(&marketplace_root, MAX_PLUGINS_PER_MARKETPLACE)
        {
            let Some(package) = plugin_root.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let plugin_id = format!("{package}@{publisher}");
            if !codex_plugin_is_effectively_enabled(&plugin_id, &plugin_states) {
                continue;
            }
            let selected = bounded_child_directories(&plugin_root, MAX_VERSIONS_PER_PLUGIN)
                .into_iter()
                .filter_map(plugin_manifest_skill_root)
                .max_by(compare_version_names);
            let Some((_, skill_root)) = selected else {
                continue;
            };
            roots.push(AdapterRoot {
                scope: Scope::AgentGlobal,
                path: skill_root,
                source: RootSource::Plugin,
            });
        }
    }
    roots
}

fn bounded_child_directories(parent: &Path, limit: usize) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut directories = entries
        .take(limit.saturating_add(1))
        .filter_map(Result::ok)
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| !name.is_empty() && !name.starts_with('.'))
                && entry.file_type().is_ok_and(|file_type| file_type.is_dir())
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    directories.sort();
    directories.truncate(limit);
    directories
}

fn plugin_manifest_skill_root(version_root: PathBuf) -> Option<(String, PathBuf)> {
    let version = version_root.file_name()?.to_str()?.to_string();
    let manifest_path = version_root.join(".codex-plugin/plugin.json");
    let metadata = fs::symlink_metadata(&manifest_path).ok()?;
    if !metadata.file_type().is_file() || metadata.len() > MAX_PLUGIN_MANIFEST_BYTES {
        return None;
    }
    let mut bytes = Vec::with_capacity(metadata.len().try_into().ok()?);
    fs::File::open(manifest_path)
        .ok()?
        .take(MAX_PLUGIN_MANIFEST_BYTES.saturating_add(1))
        .read_to_end(&mut bytes)
        .ok()?;
    if bytes.len() as u64 > MAX_PLUGIN_MANIFEST_BYTES {
        return None;
    }
    let manifest: serde_json::Value = serde_json::from_slice(&bytes).ok()?;
    let raw_skills = manifest.get("skills")?.as_str()?;
    let skill_root = safe_plugin_manifest_path(&version_root, raw_skills)?;
    Some((version, skill_root))
}

fn safe_plugin_manifest_path(version_root: &Path, raw_path: &str) -> Option<PathBuf> {
    let relative = Path::new(raw_path.trim());
    if relative.as_os_str().is_empty() || relative.is_absolute() {
        return None;
    }
    let mut resolved = version_root.to_path_buf();
    for component in relative.components() {
        match component {
            std::path::Component::CurDir => {}
            std::path::Component::Normal(name) => resolved.push(name),
            std::path::Component::ParentDir
            | std::path::Component::RootDir
            | std::path::Component::Prefix(_) => return None,
        }
    }
    if resolved == version_root {
        return None;
    }
    if let (Ok(canonical_version), Ok(canonical_resolved)) =
        (version_root.canonicalize(), resolved.canonicalize())
    {
        if !canonical_resolved.starts_with(canonical_version) {
            return None;
        }
    }
    Some(resolved)
}

fn compare_version_names(left: &(String, PathBuf), right: &(String, PathBuf)) -> Ordering {
    compare_natural_version(&left.0, &right.0).then_with(|| left.0.cmp(&right.0))
}

fn compare_natural_version(left: &str, right: &str) -> Ordering {
    let left_parts = left.split(['.', '-', '_']).collect::<Vec<_>>();
    let right_parts = right.split(['.', '-', '_']).collect::<Vec<_>>();
    for index in 0..left_parts.len().max(right_parts.len()) {
        let left_part = left_parts.get(index).copied().unwrap_or_default();
        let right_part = right_parts.get(index).copied().unwrap_or_default();
        let ordering = match (left_part.parse::<u64>(), right_part.parse::<u64>()) {
            (Ok(left_number), Ok(right_number)) => left_number.cmp(&right_number),
            _ => left_part.cmp(right_part),
        };
        if ordering != Ordering::Equal {
            return ordering;
        }
    }
    Ordering::Equal
}

impl AgentConfigAdapter for CodexAdapter {
    fn patch_enabled(
        &self,
        doc: &mut AgentConfigDocument,
        instance: &SkillInstance,
        on: bool,
    ) -> Result<(), AdapterError> {
        doc.text = patch_codex_config(&doc.text, &instance.path, on)?;
        Ok(())
    }
}

struct ParsedSkill {
    frontmatter_raw: String,
    body: String,
    name: String,
    description: String,
}

fn parse_skill_content(content: &str) -> Result<ParsedSkill, String> {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .ok_or_else(|| "missing YAML frontmatter".to_string())?;
    let (frontmatter_raw, body) = split_yaml_frontmatter(rest)?;
    let frontmatter: serde_norway::Value =
        serde_norway::from_str(frontmatter_raw).map_err(|err| err.to_string())?;
    let name = required_frontmatter_string(&frontmatter, "name", "Codex")?;
    let description = required_frontmatter_string(&frontmatter, "description", "Codex")?;

    Ok(ParsedSkill {
        frontmatter_raw: frontmatter_raw.to_string(),
        body,
        name,
        description,
    })
}

fn codex_user_config_path(ctx: &AdapterContext) -> PathBuf {
    codex_home_dir(ctx).join("config.toml")
}

fn codex_project_skill_roots(project_root: &Path, project_cwd: Option<&Path>) -> Vec<AdapterRoot> {
    codex_project_directories(project_root, project_cwd)
        .map(|dir| AdapterRoot {
            scope: Scope::AgentProject,
            path: dir.join(".agents/skills"),
            source: RootSource::Project,
        })
        .collect()
}

fn codex_project_directories<'a>(
    project_root: &'a Path,
    project_cwd: Option<&'a Path>,
) -> impl Iterator<Item = &'a Path> {
    let start = project_cwd
        .filter(|cwd| cwd.starts_with(project_root))
        .unwrap_or(project_root);
    std::iter::successors(Some(start), move |directory| {
        (*directory != project_root)
            .then(|| directory.parent())
            .flatten()
            .filter(|parent| parent.starts_with(project_root))
    })
}

fn active_codex_profile(config_path: &Path) -> Option<String> {
    let content = std::fs::read_to_string(config_path).ok()?;
    let mut inside_section = false;
    for raw_line in content.lines() {
        let line = raw_line.trim();
        if line.starts_with('[') {
            inside_section = true;
            continue;
        }
        if inside_section || line.starts_with('#') {
            continue;
        }
        let Some((key, value)) = line.split_once('=') else {
            continue;
        };
        if key.trim() == "profile" {
            let profile = parse_toml_string(value.trim()).ok()?;
            if !profile.is_empty()
                && profile
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
            {
                return Some(profile);
            }
        }
    }
    None
}

fn dedup_roots(roots: Vec<AdapterRoot>) -> Vec<AdapterRoot> {
    let mut seen = HashSet::new();
    let mut deduped = Vec::new();
    for root in roots {
        let key = format!(
            "{}|{}|{}",
            root.scope.as_str(),
            root_source_key(&root.source),
            root.path.to_string_lossy()
        );
        if seen.insert(key) {
            deduped.push(root);
        }
    }
    deduped
}

fn root_source_key(source: &RootSource) -> &'static str {
    match source {
        RootSource::UserHome => "user-home",
        RootSource::Project => "project",
        RootSource::Extra => "extra",
        RootSource::Compatibility => "compatibility",
        RootSource::Configured => "configured",
        RootSource::Admin => "admin",
        RootSource::Plugin => "plugin",
        RootSource::System => "system",
    }
}

fn patch_codex_config(content: &str, skill_path: &Path, on: bool) -> Result<String, AdapterError> {
    let skill_path = skill_path.to_string_lossy();
    let mut output = String::new();
    let mut cursor = 0;

    while cursor < content.len() {
        let remaining = &content[cursor..];
        let Some(relative_start) = find_skills_config_header(remaining) else {
            output.push_str(remaining);
            break;
        };
        let block_start = cursor + relative_start;
        output.push_str(&content[cursor..block_start]);

        let after_header = next_line_start(content, block_start);
        let block_end = next_table_header(content, after_header).unwrap_or(content.len());
        let block = &content[block_start..block_end];
        if classify_skills_config_block(block, &skill_path)? != SkillsConfigBlock::Target {
            output.push_str(block);
        }
        cursor = block_end;
    }

    if !on {
        append_disabled_entry(&mut output, &skill_path);
    }

    if output.is_empty() {
        return Ok(output);
    }
    if !output.ends_with('\n') {
        output.push('\n');
    }
    Ok(output)
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SkillsConfigBlock {
    Target,
    NonTarget,
}

fn find_skills_config_header(content: &str) -> Option<usize> {
    let mut offset = 0;
    for line in content.split_inclusive('\n') {
        if line.trim() == "[[skills.config]]" {
            return Some(offset);
        }
        offset += line.len();
    }
    if !content.ends_with('\n') && content[offset..].trim() == "[[skills.config]]" {
        return Some(offset);
    }
    None
}

fn next_line_start(content: &str, line_start: usize) -> usize {
    content[line_start..]
        .find('\n')
        .map(|offset| line_start + offset + 1)
        .unwrap_or(content.len())
}

fn next_table_header(content: &str, start: usize) -> Option<usize> {
    let mut offset = start;
    for line in content[start..].split_inclusive('\n') {
        if line.trim_start().starts_with('[') {
            return Some(offset);
        }
        offset += line.len();
    }
    None
}

pub fn parse_codex_skill_config_entries(text: &str) -> Vec<CodexSkillConfigEntry> {
    let mut entries = Vec::new();
    let mut cursor = 0;

    while cursor < text.len() {
        let remaining = &text[cursor..];
        let Some(relative_start) = find_skills_config_header(remaining) else {
            break;
        };
        let block_start = cursor + relative_start;
        let after_header = next_line_start(text, block_start);
        let block_end = next_table_header(text, after_header).unwrap_or(text.len());
        let block = &text[block_start..block_end];

        let mut entry = CodexSkillConfigEntry {
            path: None,
            enabled: None,
        };
        for raw_line in block.lines() {
            let line = raw_line.trim();
            if line.is_empty() || line.starts_with('#') || line == "[[skills.config]]" {
                continue;
            }
            let Some((key, value)) = line.split_once('=') else {
                continue;
            };
            match key.trim() {
                "path" => {
                    if let Ok(path) = parse_toml_string(value.trim()) {
                        entry.path = Some(path);
                    }
                }
                "enabled" => {
                    if let Ok(enabled) = parse_toml_bool(value.trim()) {
                        entry.enabled = Some(enabled);
                    }
                }
                _ => {}
            }
        }
        entries.push(entry);
        cursor = block_end;
    }

    entries
}

fn classify_skills_config_block(
    block: &str,
    skill_path: &str,
) -> Result<SkillsConfigBlock, AdapterError> {
    let mut valid_paths = Vec::new();
    let mut path_line_count = 0;
    let mut target_path_line_is_malformed = false;
    let mut target_schema_errors = Vec::new();

    for (line_number, raw_line) in block.lines().enumerate() {
        let line = raw_line.trim();
        if line.is_empty() || line.starts_with('#') || line == "[[skills.config]]" {
            continue;
        }

        let Some((key, value)) = line.split_once('=') else {
            if looks_like_key(line, "path") && raw_line.contains(skill_path) {
                target_path_line_is_malformed = true;
            }
            if looks_like_key(line, "enabled") && raw_line.contains(skill_path) {
                target_schema_errors.push(format!(
                    "line {} has an invalid `enabled` assignment",
                    line_number + 1
                ));
            }
            continue;
        };

        let key = key.trim();
        let value = value.trim();
        if key == "path" {
            path_line_count += 1;
            match parse_toml_string(value) {
                Ok(path) => valid_paths.push(path),
                Err(err) => {
                    if raw_line.contains(skill_path) {
                        target_path_line_is_malformed = true;
                        target_schema_errors.push(format!(
                            "line {} has an invalid `path` assignment: {err}",
                            line_number + 1
                        ));
                    }
                }
            }
        } else if key == "enabled" && parse_toml_bool(value).is_err() {
            target_schema_errors.push(format!(
                "line {} has an invalid `enabled` assignment",
                line_number + 1
            ));
        }
    }

    let is_target = valid_paths.iter().any(|path| path == skill_path);
    if target_path_line_is_malformed && !is_target {
        return Err(AdapterError::new(format!(
            "invalid Codex skills.config path entry for target `{skill_path}`"
        )));
    }
    if is_target && path_line_count > 1 {
        return Err(AdapterError::new(format!(
            "malformed Codex skills.config block for target `{skill_path}`: duplicate path entries"
        )));
    }
    if is_target && !target_schema_errors.is_empty() {
        return Err(AdapterError::new(format!(
            "malformed Codex skills.config block for target `{skill_path}`: {}",
            target_schema_errors.join("; ")
        )));
    }

    if is_target {
        Ok(SkillsConfigBlock::Target)
    } else {
        Ok(SkillsConfigBlock::NonTarget)
    }
}

fn looks_like_key(line: &str, key: &str) -> bool {
    let rest = line.strip_prefix(key).unwrap_or_default();
    !rest.is_empty()
        && rest
            .chars()
            .next()
            .is_some_and(|ch| ch.is_whitespace() || ch == '=')
}

fn parse_toml_string(value: &str) -> Result<String, String> {
    if value.starts_with("\"\"\"") || value.starts_with("'''") {
        return Err("multi-line strings are not supported for Codex skill paths".to_string());
    }
    if value.starts_with('"') {
        parse_basic_toml_string(value)
    } else if value.starts_with('\'') {
        parse_literal_toml_string(value)
    } else {
        Err("path must be a TOML basic or literal string".to_string())
    }
}

fn parse_basic_toml_string(value: &str) -> Result<String, String> {
    let mut chars = value.char_indices();
    if chars.next().map(|(_, ch)| ch) != Some('"') {
        return Err("path must start with a basic string quote".to_string());
    }
    let mut parsed = String::new();
    let mut escaped = false;
    while let Some((offset, ch)) = chars.next() {
        if escaped {
            let unescaped = match ch {
                'b' => '\u{0008}',
                't' => '\t',
                'n' => '\n',
                'f' => '\u{000c}',
                'r' => '\r',
                '"' => '"',
                '\\' => '\\',
                'u' => parse_hex_escape(&mut chars, 4)?,
                'U' => parse_hex_escape(&mut chars, 8)?,
                other => return Err(format!("unsupported TOML escape `\\{other}`")),
            };
            parsed.push(unescaped);
            escaped = false;
            continue;
        }
        match ch {
            '\\' => escaped = true,
            '"' => {
                validate_toml_trailing(&value[offset + ch.len_utf8()..])?;
                return Ok(parsed);
            }
            other => parsed.push(other),
        }
    }
    Err("unterminated TOML basic string".to_string())
}

fn parse_hex_escape(chars: &mut std::str::CharIndices<'_>, width: usize) -> Result<char, String> {
    let mut hex = String::new();
    for _ in 0..width {
        let Some((_, ch)) = chars.next() else {
            return Err("incomplete TOML unicode escape".to_string());
        };
        if !ch.is_ascii_hexdigit() {
            return Err("invalid TOML unicode escape".to_string());
        }
        hex.push(ch);
    }
    let codepoint = u32::from_str_radix(&hex, 16)
        .map_err(|err| format!("invalid TOML unicode escape: {err}"))?;
    char::from_u32(codepoint).ok_or_else(|| "invalid TOML unicode scalar value".to_string())
}

fn parse_literal_toml_string(value: &str) -> Result<String, String> {
    let rest = value
        .strip_prefix('\'')
        .ok_or_else(|| "path must start with a literal string quote".to_string())?;
    let end = rest
        .find('\'')
        .ok_or_else(|| "unterminated TOML literal string".to_string())?;
    validate_toml_trailing(&rest[end + 1..])?;
    Ok(rest[..end].to_string())
}

fn parse_toml_bool(value: &str) -> Result<bool, String> {
    let value = value.trim_start();
    if let Some(rest) = value.strip_prefix("true") {
        validate_toml_trailing(rest)?;
        return Ok(true);
    }
    if let Some(rest) = value.strip_prefix("false") {
        validate_toml_trailing(rest)?;
        return Ok(false);
    }
    Err("enabled must be true or false".to_string())
}

fn validate_toml_trailing(rest: &str) -> Result<(), String> {
    let trailing = rest.trim_start();
    if trailing.is_empty() || trailing.starts_with('#') {
        Ok(())
    } else {
        Err(format!("unexpected trailing content `{trailing}`"))
    }
}

fn append_disabled_entry(output: &mut String, skill_path: &str) {
    if !output.is_empty() && !output.ends_with("\n\n") {
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push('\n');
    }
    output.push_str("[[skills.config]]\npath = \"");
    output.push_str(&escape_toml_basic_string(skill_path));
    output.push_str("\"\nenabled = false\n");
}

fn escape_toml_basic_string(value: &str) -> String {
    let mut escaped = String::new();
    for ch in value.chars() {
        match ch {
            '\u{0008}' => escaped.push_str("\\b"),
            '\t' => escaped.push_str("\\t"),
            '\n' => escaped.push_str("\\n"),
            '\u{000c}' => escaped.push_str("\\f"),
            '\r' => escaped.push_str("\\r"),
            '"' => escaped.push_str("\\\""),
            '\\' => escaped.push_str("\\\\"),
            other => escaped.push(other),
        }
    }
    escaped
}

#[cfg(test)]
mod tests {
    use skills_copilot_core::{AgentConfigDocument, ConfigFormat, RootSource, SkillState};

    use super::*;

    #[test]
    fn yaml_contract_preserves_scalar_sequence_bool_and_nested_mapping() {
        let raw = "name: sample-skill\ndescription: Sample\nenabled: true\nallowed-tools:\n  - Read\n  - Search\nmetadata:\n  openclaw:\n    skillKey: routed-key\n";
        let value: serde_norway::Value = serde_norway::from_str(raw).expect("yaml parses");

        assert_eq!(
            value.get("name").and_then(serde_norway::Value::as_str),
            Some("sample-skill")
        );
        assert_eq!(
            value.get("enabled").and_then(serde_norway::Value::as_bool),
            Some(true)
        );
        assert_eq!(
            value
                .get("allowed-tools")
                .and_then(serde_norway::Value::as_sequence)
                .map(Vec::len),
            Some(2)
        );
        assert_eq!(
            value
                .get("metadata")
                .and_then(|item| item.get("openclaw"))
                .and_then(|item| item.get("skillKey"))
                .and_then(serde_norway::Value::as_str),
            Some("routed-key")
        );

        let parsed = parse_skill_content(&format!("---\n{raw}---\nBody.\n"))
            .expect("adapter frontmatter parses");
        assert_eq!(parsed.name, "sample-skill");
        assert_eq!(parsed.description, "Sample");
    }

    #[test]
    fn yaml_contract_malformed_frontmatter_returns_error() {
        let result =
            parse_skill_content("---\nname: [unterminated\ndescription: Sample\n---\nBody.\n");

        assert!(result.is_err());
    }

    #[test]
    fn parses_explicitly_enabled_codex_plugin_ids() {
        let enabled = parse_codex_enabled_plugin_ids(
            r#"
[plugins."pdf@openai-primary-runtime"]
enabled = true

[plugins.'browser@openai-bundled']
enabled = false

[plugins.local_plugin]
enabled = true # comment

[mcp_servers.browser]
enabled = true
"#,
        );

        assert_eq!(
            enabled,
            BTreeSet::from([
                "local_plugin".to_string(),
                "pdf@openai-primary-runtime".to_string(),
            ])
        );
        assert_eq!(
            parse_codex_plugin_states("[plugins.\"browser@openai-bundled\"]\nenabled = false\n"),
            BTreeMap::from([("browser@openai-bundled".to_string(), false)])
        );
        assert_eq!(
            parse_codex_plugin_states(
                "[plugins.\"browser@openai-bundled\"] # installed by desktop\nenabled = true # active\n"
            ),
            BTreeMap::from([("browser@openai-bundled".to_string(), true)])
        );
        assert!(!codex_plugin_is_effectively_enabled(
            "sales@openai-curated-remote",
            &BTreeMap::new()
        ));
        assert!(!codex_plugin_is_effectively_enabled(
            "browser@openai-bundled",
            &BTreeMap::new()
        ));
        assert!(!codex_plugin_is_effectively_enabled(
            "sales@openai-curated-remote",
            &BTreeMap::from([("sales@openai-curated-remote".to_string(), false)])
        ));
    }

    #[test]
    fn derives_plugin_cache_id_only_from_valid_cache_skill_paths() {
        let codex_home = Path::new("/tmp/home/.codex");
        assert_eq!(
            codex_plugin_cache_id(
                codex_home,
                Path::new(
                    "/tmp/home/.codex/plugins/cache/openai-primary-runtime/pdf/1.2.3/skills/pdf/SKILL.md",
                ),
            )
            .as_deref(),
            Some("pdf@openai-primary-runtime")
        );
        assert_eq!(
            codex_plugin_cache_id(
                codex_home,
                Path::new(
                    "/tmp/home/.codex/plugins/cache/personal/workflows/2.0.0/playbooks/review/SKILL.md",
                ),
            )
            .as_deref(),
            Some("workflows@personal"),
            "the manifest-declared root does not have to be named skills"
        );
        assert!(codex_plugin_cache_id(
            codex_home,
            Path::new(
                "/tmp/home/.codex/plugins/cache/openai-primary-runtime/pdf/1.2.3/docs/readme.md"
            ),
        )
        .is_none());
        assert!(codex_plugin_cache_id(
            codex_home,
            Path::new("/tmp/home/.agents/plugins/pdf/skills/pdf/SKILL.md"),
        )
        .is_none());
    }

    #[test]
    fn exposes_native_and_read_only_expanded_roots() {
        let adapter = CodexAdapter;
        let ctx = AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: Some(PathBuf::from("/tmp/project")),
            project_cwd: Some(PathBuf::from("/tmp/project/nested/deeper")),
            extra_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: PathBuf::from("/tmp/unverified"),
                source: RootSource::Extra,
            }],
        };

        let roots = adapter.roots(&ctx);

        assert_eq!(roots.len(), 6);
        assert_eq!(roots[0].path, PathBuf::from("/tmp/home/.agents/skills"));
        assert_eq!(roots[0].scope, Scope::AgentGlobal);
        assert_eq!(roots[0].source, RootSource::UserHome);
        assert_eq!(roots[1].path, PathBuf::from("/tmp/home/.codex/skills"));
        assert_eq!(roots[1].scope, Scope::AgentGlobal);
        assert_eq!(roots[1].source, RootSource::Compatibility);
        assert_eq!(
            roots[2].path,
            PathBuf::from("/tmp/project/nested/deeper/.agents/skills")
        );
        assert_eq!(
            roots[3].path,
            PathBuf::from("/tmp/project/nested/.agents/skills")
        );
        assert_eq!(roots[4].path, PathBuf::from("/tmp/project/.agents/skills"));
        for root in &roots[2..5] {
            assert_eq!(root.scope, Scope::AgentProject);
            assert_eq!(root.source, RootSource::Project);
        }
        assert_eq!(roots[5].path, PathBuf::from("/etc/codex/skills"));
        assert_eq!(roots[5].scope, Scope::AgentGlobal);
        assert_eq!(roots[5].source, RootSource::Admin);
    }

    #[test]
    fn includes_system_review_agent_reported_by_current_runtime() {
        let adapter = CodexAdapter;
        let root = AdapterRoot {
            scope: Scope::AgentGlobal,
            path: PathBuf::from("/tmp/home/.codex/skills"),
            source: RootSource::Compatibility,
        };

        assert!(adapter.accepts_skill_path(&root, Path::new(".system/imagegen/SKILL.md")));
        assert!(adapter.accepts_skill_path(&root, Path::new(".system/review-agent/SKILL.md")));
    }

    #[test]
    fn codex_home_override_rejects_lexical_escape_and_relative_paths() {
        let home = std::env::temp_dir().join("codex-home-boundary/home");
        let ctx = AdapterContext {
            user_home: home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        assert_eq!(
            resolved_codex_home(&ctx, Some(home.join("profiles/work"))),
            home.join("profiles/work")
        );
        assert_eq!(
            resolved_codex_home(&ctx, Some(home.join("../outside"))),
            home.join(".codex")
        );
        assert_eq!(
            resolved_codex_home(&ctx, Some(PathBuf::from("relative-codex-home"))),
            home.join(".codex")
        );
    }

    #[test]
    fn local_plugin_marketplace_is_not_a_filesystem_scan_source() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-codex-plugin-roots-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let plugin_root = home.join(".codex/plugins/local-review");
        let skills_root = plugin_root.join("skills");
        std::fs::create_dir_all(skills_root.join("review-helper"))
            .expect("create plugin skill dir");
        std::fs::create_dir_all(plugin_root.join(".codex-plugin"))
            .expect("create plugin manifest dir");
        std::fs::create_dir_all(home.join(".agents/plugins")).expect("create marketplace parent");
        std::fs::write(
            plugin_root.join(".codex-plugin/plugin.json"),
            "{\n  \"name\": \"local-review\",\n  \"skills\": \"./skills/\"\n}\n",
        )
        .expect("write plugin manifest");
        std::fs::write(
            skills_root.join("review-helper/SKILL.md"),
            "---\nname: review-helper\ndescription: Plugin fixture\n---\nBody.\n",
        )
        .expect("write plugin skill");
        std::fs::write(
            home.join(".agents/plugins/marketplace.json"),
            "{\n  \"plugins\": [\n    {\"source\": {\"source\": \"local\", \"path\": \"./.codex/plugins/local-review\"}},\n    {\"source\": {\"source\": \"local\", \"path\": \"./../outside\"}},\n    {\"source\": \"https://example.invalid/plugin.tgz\"}\n  ]\n}\n",
        )
        .expect("write marketplace");

        let adapter = CodexAdapter;
        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };
        let roots = adapter.roots(&ctx);

        assert_eq!(
            roots
                .iter()
                .filter(|root| root.source == RootSource::Plugin)
                .count(),
            0,
            "marketplace and cache layouts are runtime implementation details, not scan roots"
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn plugin_cache_exposes_only_explicitly_enabled_skill_roots() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-codex-plugin-cache-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let cache = home.join(".codex/plugins/cache");
        for version in ["1.9.0", "1.10.0"] {
            let package = cache.join("openai-bundled/browser").join(version);
            std::fs::create_dir_all(package.join(".codex-plugin"))
                .expect("create plugin manifest dir");
            std::fs::create_dir_all(package.join("skills/browser-control"))
                .expect("create plugin skills dir");
            std::fs::write(
                package.join(".codex-plugin/plugin.json"),
                format!(
                    "{{\"name\":\"browser\",\"version\":\"{version}\",\"skills\":\"./skills/\"}}"
                ),
            )
            .expect("write plugin manifest");
        }
        let stale = cache.join("personal/stale-cache/9.0.0");
        std::fs::create_dir_all(stale.join(".codex-plugin"))
            .expect("create stale plugin manifest dir");
        std::fs::create_dir_all(stale.join("skills/stale-cache"))
            .expect("create stale plugin skills dir");
        std::fs::write(
            stale.join(".codex-plugin/plugin.json"),
            r#"{"name":"stale-cache","version":"9.0.0","skills":"./skills/"}"#,
        )
        .expect("write stale plugin manifest");
        let unconfigured_remote = cache.join("openai-curated-remote/unconfigured-remote/1.0.0");
        std::fs::create_dir_all(unconfigured_remote.join(".codex-plugin"))
            .expect("create unconfigured remote manifest dir");
        std::fs::create_dir_all(unconfigured_remote.join("skills/unconfigured-remote"))
            .expect("create unconfigured remote skill dir");
        std::fs::write(
            unconfigured_remote.join(".codex-plugin/plugin.json"),
            r#"{"name":"unconfigured-remote","version":"1.0.0","skills":"./skills/"}"#,
        )
        .expect("write unconfigured remote manifest");
        let unconfigured_local = cache.join("personal/unconfigured-local/1.0.0");
        std::fs::create_dir_all(unconfigured_local.join(".codex-plugin"))
            .expect("create unconfigured local manifest dir");
        std::fs::create_dir_all(unconfigured_local.join("skills/unconfigured-local"))
            .expect("create unconfigured local skill dir");
        std::fs::write(
            unconfigured_local.join(".codex-plugin/plugin.json"),
            r#"{"name":"unconfigured-local","version":"1.0.0","skills":"./skills/"}"#,
        )
        .expect("write unconfigured local manifest");
        let escaping = cache.join("personal/escaping/1.0.0");
        std::fs::create_dir_all(escaping.join(".codex-plugin"))
            .expect("create escaping manifest dir");
        std::fs::write(
            escaping.join(".codex-plugin/plugin.json"),
            r#"{"name":"escaping","skills":"../../../../outside"}"#,
        )
        .expect("write escaping manifest");
        let staging = cache.join(".remote-plugin-install-staging/staged/9.0.0");
        std::fs::create_dir_all(staging.join(".codex-plugin"))
            .expect("create staging manifest dir");
        std::fs::create_dir_all(staging.join("skills/staged")).expect("create staging skills dir");
        std::fs::write(
            staging.join(".codex-plugin/plugin.json"),
            r#"{"name":"staged","skills":"./skills/"}"#,
        )
        .expect("write staging manifest");
        std::fs::create_dir_all(home.join(".codex")).expect("create Codex home");
        std::fs::write(
            home.join(".codex/config.toml"),
            "[plugins.\"browser@openai-bundled\"]\nenabled = true\n\n[plugins.\"stale-cache@personal\"]\nenabled = false\n",
        )
        .expect("write enabled plugin config");

        let adapter = CodexAdapter;
        let roots = adapter.roots(&AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        });
        let cache_roots = roots
            .iter()
            .filter(|root| root.path.starts_with(&cache))
            .map(|root| root.path.clone())
            .collect::<Vec<_>>();

        assert_eq!(
            cache_roots,
            vec![cache.join("openai-bundled/browser/1.10.0/skills")],
            "only explicitly enabled plugin copies should be read from their manifest-declared roots"
        );
        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn enabled_remote_plugins_expose_every_manifest_declared_skill_root() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-codex-remote-plugin-entries-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let cache = home.join(".codex/plugins/cache/openai-curated-remote");

        let build_macos = cache.join("build-macos-apps/1.0.0");
        let investment_banking = cache.join("investment-banking/1.0.0");
        let templates = cache.join("openai-templates/1.0.0");
        for package in [&build_macos, &investment_banking, &templates] {
            std::fs::create_dir_all(package.join(".codex-plugin"))
                .expect("create plugin manifest dir");
            std::fs::write(
                package.join(".codex-plugin/plugin.json"),
                r#"{"name":"fixture","version":"1.0.0","skills":"./skills/"}"#,
            )
            .expect("write plugin manifest");
        }
        std::fs::create_dir_all(build_macos.join("skills/build-run-debug"))
            .expect("create public macOS skill");
        std::fs::create_dir_all(investment_banking.join("skills/investment-banking"))
            .expect("create investment router skill");
        std::fs::create_dir_all(investment_banking.join("skills/dcf-model-builder"))
            .expect("create investment internal skill");
        std::fs::create_dir_all(templates.join("skills/artifact-template"))
            .expect("create default template skill");
        std::fs::write(
            home.join(".codex/config.toml"),
            "[plugins.\"build-macos-apps@openai-curated-remote\"]\nenabled = true\n\n[plugins.\"investment-banking@openai-curated-remote\"]\nenabled = true\n\n[plugins.\"openai-templates@openai-curated-remote\"]\nenabled = true\n",
        )
        .expect("write enabled remote plugin config");

        let roots = CodexAdapter.roots(&AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        });
        let plugin_roots = roots
            .iter()
            .filter(|root| root.source == RootSource::Plugin)
            .map(|root| root.path.clone())
            .collect::<Vec<_>>();

        assert!(plugin_roots.contains(&build_macos.join("skills")));
        assert!(plugin_roots.contains(&investment_banking.join("skills")));
        assert!(plugin_roots.contains(&templates.join("skills")));
        assert_eq!(plugin_roots.len(), 3);
        assert!(CodexAdapter.accepts_skill_path(
            roots
                .iter()
                .find(|root| root.path == investment_banking.join("skills"))
                .expect("investment plugin skill root"),
            Path::new("dcf-model-builder/SKILL.md")
        ));

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn path_and_content_parsing_are_equivalent() {
        let adapter = CodexAdapter;
        let valid_frontmatter =
            "---\nname: codex-alpha\ndescription: Alpha Codex skill.\n---\nBody.\n";
        let fixture = write_skill("valid-codex", valid_frontmatter);

        let skill = adapter.parse(&fixture).expect("skill parses");

        assert_eq!(skill.agent, AgentId::Codex);
        assert_eq!(skill.name, "codex-alpha");
        assert_eq!(skill.description, "Alpha Codex skill.");
        assert_eq!(
            skill.frontmatter_raw,
            "name: codex-alpha\ndescription: Alpha Codex skill."
        );
        assert_eq!(skill.body, "Body.\n");
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.enabled);
        assert!(skill.permissions.tools.is_empty());

        crate::assert_parse_equivalent(&adapter, &fixture);
    }

    #[test]
    fn marks_missing_description_as_broken() {
        let adapter = CodexAdapter;
        let fixture = fixture_path("fixtures/codex/broken/missing-description/SKILL.md");

        let skill = adapter.parse(&fixture).expect("broken skill is returned");

        assert_eq!(skill.name, "missing-description");
        assert_eq!(skill.state, SkillState::Broken);
        assert!(!skill.enabled);
        assert!(skill.description.contains("description"));
    }

    #[test]
    fn marks_missing_name_as_broken() {
        let adapter = CodexAdapter;
        let fixture = write_skill(
            "missing-name",
            "---\ndescription: Missing name fixture.\n---\nBody.\n",
        );

        let skill = adapter.parse(&fixture).expect("broken skill is returned");

        assert_eq!(skill.name, "missing-name");
        assert_eq!(skill.state, SkillState::Broken);
        assert!(!skill.enabled);
        assert!(skill.description.contains("name"));
    }

    #[test]
    fn disable_adds_one_disabled_entry() {
        let adapter = CodexAdapter;
        let skill_path = PathBuf::from("/tmp/codex/alpha/SKILL.md");
        let mut doc = config_doc("profile = \"default\"\n");
        let skill = skill_for_path(&skill_path);

        adapter
            .patch_enabled(&mut doc, &skill, false)
            .expect("disable patch succeeds");

        assert!(doc.text.contains("profile = \"default\""));
        assert_eq!(doc.text.matches("[[skills.config]]").count(), 1);
        assert!(doc.text.contains("path = \"/tmp/codex/alpha/SKILL.md\""));
        assert!(doc.text.contains("enabled = false"));
    }

    #[test]
    fn disable_preserves_comments_and_non_target_config() {
        let adapter = CodexAdapter;
        let skill_path = PathBuf::from("/tmp/codex/alpha/SKILL.md");
        let mut doc = config_doc(
            "# Codex user config\nprofile = \"default\"\n\n[model]\nname = \"gpt-5\"\n\n# keep beta disabled\n[[skills.config]]\n# literal string path must remain intact\npath = '/tmp/codex/beta/SKILL.md'\nenabled = false\n\n[profiles.work]\nmodel = \"gpt-5-codex\"\n\n[[skills.config]]\npath = \"/tmp/codex/alpha/SKILL.md\"\nenabled = true\n",
        );
        let skill = skill_for_path(&skill_path);

        adapter
            .patch_enabled(&mut doc, &skill, false)
            .expect("disable patch succeeds");

        assert!(doc.text.contains("# Codex user config"));
        assert!(doc.text.contains("[model]\nname = \"gpt-5\""));
        assert!(doc.text.contains("# keep beta disabled"));
        assert!(doc.text.contains("path = '/tmp/codex/beta/SKILL.md'"));
        assert!(doc
            .text
            .contains("[profiles.work]\nmodel = \"gpt-5-codex\""));
        assert_eq!(doc.text.matches("/tmp/codex/alpha/SKILL.md").count(), 1);
        assert!(doc.text.contains("path = \"/tmp/codex/alpha/SKILL.md\""));
        assert!(doc.text.contains("enabled = false"));
        assert!(!doc
            .text
            .contains("path = \"/tmp/codex/alpha/SKILL.md\"\nenabled = true"));
    }

    #[test]
    fn enable_removes_all_matching_entries_and_leaves_non_targets() {
        let adapter = CodexAdapter;
        let skill_path = PathBuf::from("/tmp/codex/alpha/SKILL.md");
        let mut doc = config_doc(
            "profile = \"default\"\n\n[[skills.config]]\npath = \"/tmp/codex/alpha/SKILL.md\"\nenabled = false\n\n[[skills.config]]\npath = '/tmp/codex/beta/SKILL.md'\nenabled = false\n\n[[skills.config]]\npath = \"/tmp/codex/alpha/SKILL.md\"\nenabled = true\n",
        );
        let skill = skill_for_path(&skill_path);

        adapter
            .patch_enabled(&mut doc, &skill, true)
            .expect("enable patch succeeds");

        assert!(doc.text.contains("profile = \"default\""));
        assert!(!doc.text.contains("/tmp/codex/alpha/SKILL.md"));
        assert!(doc.text.contains("/tmp/codex/beta/SKILL.md"));
        assert_eq!(doc.text.matches("[[skills.config]]").count(), 1);
    }

    #[test]
    fn disable_normalizes_duplicate_matching_entries() {
        let adapter = CodexAdapter;
        let skill_path = PathBuf::from("/tmp/codex/alpha/SKILL.md");
        let mut doc = config_doc(
            "[[skills.config]]\npath = \"/tmp/codex/alpha/SKILL.md\"\nenabled = true\n\n[[skills.config]]\npath = \"/tmp/codex/alpha/SKILL.md\"\nenabled = false\n",
        );
        let skill = skill_for_path(&skill_path);

        adapter
            .patch_enabled(&mut doc, &skill, false)
            .expect("disable patch succeeds");

        assert_eq!(doc.text.matches("[[skills.config]]").count(), 1);
        assert_eq!(doc.text.matches("/tmp/codex/alpha/SKILL.md").count(), 1);
        assert!(doc.text.contains("enabled = false"));
        assert!(!doc.text.contains("enabled = true"));
    }

    #[test]
    fn disable_matches_basic_and_literal_target_strings() {
        let adapter = CodexAdapter;
        let skill_path = PathBuf::from("/tmp/codex/alpha/SKILL.md");
        let mut doc = config_doc(
            "[[skills.config]]\npath = \"/tmp/codex/alpha/SKILL.md\" # basic string\nenabled = true\n\n[[skills.config]]\npath = '/tmp/codex/alpha/SKILL.md' # literal string\nenabled = false\n",
        );
        let skill = skill_for_path(&skill_path);

        adapter
            .patch_enabled(&mut doc, &skill, false)
            .expect("disable patch succeeds");

        assert_eq!(doc.text.matches("[[skills.config]]").count(), 1);
        assert_eq!(doc.text.matches("/tmp/codex/alpha/SKILL.md").count(), 1);
        assert!(doc.text.contains("path = \"/tmp/codex/alpha/SKILL.md\""));
        assert!(doc.text.contains("enabled = false"));
        assert!(!doc.text.contains("# basic string"));
        assert!(!doc.text.contains("# literal string"));
    }

    #[test]
    fn parses_skill_config_entries_with_toml_comments_and_literal_strings() {
        let entries = parse_codex_skill_config_entries(
            r#"
profile = "default"

[[skills.config]]
path = '/tmp/codex/alpha/SKILL.md' # literal path
enabled = false # trailing comment

[[skills.config]]
path = "/tmp/codex/beta/SKILL.md" # basic path
enabled = true
"#,
        );

        assert_eq!(entries.len(), 2);
        assert_eq!(
            entries[0].path.as_deref(),
            Some("/tmp/codex/alpha/SKILL.md")
        );
        assert_eq!(entries[0].enabled, Some(false));
        assert_eq!(entries[1].path.as_deref(), Some("/tmp/codex/beta/SKILL.md"));
        assert_eq!(entries[1].enabled, Some(true));
    }

    #[test]
    fn invalid_target_path_line_returns_error() {
        let adapter = CodexAdapter;
        let skill_path = PathBuf::from("/tmp/codex/alpha/SKILL.md");
        let mut doc =
            config_doc("[[skills.config]]\npath = /tmp/codex/alpha/SKILL.md\nenabled = false\n");
        let skill = skill_for_path(&skill_path);

        let err = adapter
            .patch_enabled(&mut doc, &skill, false)
            .expect_err("invalid target path should fail");

        assert!(err.message.contains("invalid Codex skills.config path"));
        assert!(doc.text.contains("path = /tmp/codex/alpha/SKILL.md"));
    }

    #[test]
    fn malformed_target_block_returns_error() {
        let adapter = CodexAdapter;
        let skill_path = PathBuf::from("/tmp/codex/alpha/SKILL.md");
        let mut doc = config_doc(
            "[[skills.config]]\npath = \"/tmp/codex/alpha/SKILL.md\"\nenabled = \"false\"\n",
        );
        let skill = skill_for_path(&skill_path);

        let err = adapter
            .patch_enabled(&mut doc, &skill, true)
            .expect_err("malformed target block should fail");

        assert!(err.message.contains("malformed Codex skills.config block"));
        assert!(err.message.contains("enabled"));
        assert!(doc.text.contains("enabled = \"false\""));
    }

    #[test]
    fn same_frontmatter_name_keeps_path_distinct_instances() {
        let adapter = CodexAdapter;
        let path_a = write_skill(
            "conflict-a",
            "---\nname: shared-codex\ndescription: First.\n---\nBody A.\n",
        );
        let path_b = write_skill(
            "conflict-b",
            "---\nname: shared-codex\ndescription: Second.\n---\nBody B.\n",
        );

        let skill_a = adapter.parse(&path_a).expect("skill a parses");
        let skill_b = adapter.parse(&path_b).expect("skill b parses");

        assert_eq!(skill_a.name, skill_b.name);
        assert_ne!(skill_a.id, skill_b.id);
    }

    fn config_doc(text: &str) -> AgentConfigDocument {
        AgentConfigDocument {
            path: PathBuf::from("/tmp/home/.codex/config.toml"),
            format: ConfigFormat::Toml,
            text: text.to_string(),
        }
    }

    fn skill_for_path(path: &Path) -> SkillInstance {
        SkillInstance {
            id: "codex-alpha-id".to_string(),
            agent: AgentId::Codex,
            scope: Scope::AgentGlobal,
            project_root: None,
            path: path.to_path_buf(),
            display_path: path.to_path_buf(),
            definition_id: "codex-alpha".to_string(),
            name: "codex-alpha".to_string(),
            display_name: "codex-alpha".to_string(),
            description: "Alpha".to_string(),
            version: None,
            state: SkillState::Loaded,
            enabled: true,
            frontmatter_raw: String::new(),
            body: String::new(),
            scripts: Vec::new(),
            permissions: PermissionRequest::default(),
            fingerprint: String::new(),
            mtime: 0,
            first_seen: 0,
            last_seen: 0,
        }
    }

    fn write_skill(name: &str, content: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "skills-copilot-codex-adapter-{}-{}",
            std::process::id(),
            name
        ));
        let skill_dir = root.join(name);
        std::fs::create_dir_all(&skill_dir).expect("create skill dir");
        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(&skill_path, content).expect("write skill");
        skill_path
            .canonicalize()
            .expect("canonicalize temp skill path")
    }

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }
}
