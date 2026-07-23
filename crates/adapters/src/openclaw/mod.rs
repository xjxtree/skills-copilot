use std::{
    collections::HashSet,
    env,
    path::{Path, PathBuf},
};

use crate::shared::{optional_frontmatter_string, split_yaml_frontmatter, stable_path_id};
use skills_copilot_core::{
    adapter_logical_source_token, AdapterContext, AdapterError, AdapterRoot, AgentAdapter,
    AgentConfigAdapter, AgentConfigDocument, AgentId, PermissionRequest, RootSource, Scope,
    SkillInstance, SkillState,
};

use crate::environment::{absolute_env_path, expand_local_path, normalize_path_lexically};

#[derive(Debug, Default)]
pub struct OpenclawAdapter;

impl AgentAdapter for OpenclawAdapter {
    fn id(&self) -> AgentId {
        AgentId::Openclaw
    }

    fn display_name(&self) -> &'static str {
        "OpenClaw"
    }

    fn roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
        let state_dir = openclaw_state_dir(ctx);
        let mut roots = Vec::new();
        for (level, workspace_root) in openclaw_selected_workspace_roots(ctx, &state_dir)
            .into_iter()
            .enumerate()
        {
            roots.push(AdapterRoot {
                scope: Scope::AgentProject,
                path: workspace_root.join("skills"),
                source: RootSource::Project,
                logical_source_id: Some(format!("openclaw-workspace-skills:{level}")),
            });
            roots.push(AdapterRoot {
                scope: Scope::AgentProject,
                path: workspace_root.join(".agents/skills"),
                source: RootSource::Compatibility,
                logical_source_id: Some(format!("agents-workspace-skills:{level}")),
            });
        }

        roots.push(AdapterRoot {
            scope: Scope::AgentGlobal,
            path: ctx.user_home.join(".agents/skills"),
            source: RootSource::Compatibility,
            logical_source_id: Some("agents-shared-skills".to_string()),
        });
        roots.push(AdapterRoot {
            scope: Scope::AgentGlobal,
            path: state_dir.join("skills"),
            source: RootSource::UserHome,
            logical_source_id: Some("openclaw-user-skills".to_string()),
        });
        roots.extend(openclaw_bundled_skill_roots());
        roots.extend(openclaw_plugin_skill_roots(ctx, &state_dir));
        roots.extend(openclaw_extra_skill_roots(ctx, &state_dir));

        dedup_roots(roots)
    }

    fn link_target_roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
        let state_dir = openclaw_state_dir(ctx);
        let mut roots = Vec::new();
        for declared in self.roots(ctx) {
            let unconditional = declared.path == ctx.user_home.join(".agents/skills")
                || declared.path == state_dir.join("skills");
            if unconditional {
                roots.extend(openclaw_declared_skill_link_targets(&declared));
            }
        }
        if let Ok(content) = std::fs::read_to_string(openclaw_config_path(ctx)) {
            if let Ok(config) = json5::from_str::<serde_json::Value>(&content) {
                for (declaration_index, path) in config
                    .get("skills")
                    .and_then(|skills| skills.get("load"))
                    .and_then(|load| load.get("allowSymlinkTargets"))
                    .and_then(serde_json::Value::as_array)
                    .into_iter()
                    .flatten()
                    .filter_map(serde_json::Value::as_str)
                    .filter_map(|raw| expand_local_path(raw, &ctx.user_home, &state_dir))
                    .enumerate()
                {
                    for scope in [Scope::AgentGlobal, Scope::AgentProject] {
                        roots.push(AdapterRoot {
                            scope,
                            path: path.clone(),
                            source: RootSource::Configured,
                            logical_source_id: Some(format!(
                                "symlink-target:{declaration_index}:{}",
                                scope.as_str()
                            )),
                        });
                    }
                }
            }
        }
        dedup_roots(roots)
    }

    fn parse(&self, path: &Path) -> Result<SkillInstance, AdapterError> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| AdapterError::new(format!("failed to read skill: {err}")))?;
        self.parse_content(path, content)
    }

    fn parse_content(&self, path: &Path, content: String) -> Result<SkillInstance, AdapterError> {
        let fallback_name = containing_dir_name(path);
        let parsed = parse_skill_content(&content, &fallback_name);
        let (frontmatter_raw, body, name, description, version, state, enabled) = match parsed {
            Ok(parsed) => (
                parsed.frontmatter_raw,
                parsed.body,
                parsed.name,
                parsed.description,
                parsed.version,
                SkillState::Loaded,
                true,
            ),
            Err(message) => (
                String::new(),
                content,
                fallback_name,
                message,
                None,
                SkillState::Broken,
                false,
            ),
        };

        Ok(SkillInstance {
            id: stable_path_id("openclaw", path),
            agent: AgentId::Openclaw,
            scope: Scope::AgentProject,
            project_root: None,
            path: PathBuf::from(path),
            display_path: PathBuf::from(path),
            definition_id: name.clone(),
            name: name.clone(),
            display_name: name,
            description,
            version,
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
        let components = relative_path.components().count();
        let depth_is_valid = if root.source == RootSource::Plugin {
            (1..=7).contains(&components)
        } else {
            (2..=7).contains(&components)
        };
        depth_is_valid
            && relative_path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md")
    }

    fn config_paths(&self, ctx: &AdapterContext) -> Vec<PathBuf> {
        vec![openclaw_config_path(ctx)]
    }
}

impl AgentConfigAdapter for OpenclawAdapter {
    fn patch_enabled(
        &self,
        doc: &mut AgentConfigDocument,
        instance: &SkillInstance,
        on: bool,
    ) -> Result<(), AdapterError> {
        let key = openclaw_config_key_for_instance(instance);
        doc.text = patch_openclaw_config(&doc.text, &key, on)?;
        Ok(())
    }
}

struct ParsedSkill {
    frontmatter_raw: String,
    body: String,
    name: String,
    description: String,
    version: Option<String>,
}

fn parse_skill_content(content: &str, fallback_name: &str) -> Result<ParsedSkill, String> {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .ok_or_else(|| "missing YAML frontmatter".to_string())?;
    let (frontmatter_raw, body) = split_yaml_frontmatter(rest)?;
    let frontmatter: serde_norway::Value =
        serde_norway::from_str(frontmatter_raw).map_err(|err| err.to_string())?;
    let name = optional_frontmatter_string(&frontmatter, "name")
        .unwrap_or_else(|| fallback_name.to_string());
    let description = optional_frontmatter_string(&frontmatter, "description").unwrap_or_default();
    let version = optional_frontmatter_string(&frontmatter, "version");

    Ok(ParsedSkill {
        frontmatter_raw: frontmatter_raw.to_string(),
        body,
        name,
        description,
        version,
    })
}

pub fn openclaw_config_path(ctx: &AdapterContext) -> PathBuf {
    absolute_env_path("OPENCLAW_CONFIG_PATH")
        .unwrap_or_else(|| openclaw_state_dir(ctx).join("openclaw.json"))
}

pub fn openclaw_state_dir(ctx: &AdapterContext) -> PathBuf {
    if let Some(path) = absolute_env_path("OPENCLAW_STATE_DIR") {
        return path;
    }
    let profile = env::var("OPENCLAW_PROFILE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| {
            !value.is_empty()
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        });
    profile.map_or_else(
        || ctx.user_home.join(".openclaw"),
        |profile| ctx.user_home.join(format!(".openclaw-{profile}")),
    )
}

pub fn openclaw_disabled_skill_keys(config_text: &str) -> Vec<String> {
    openclaw_config_json(config_text)
        .ok()
        .and_then(|value| {
            value
                .get("skills")
                .and_then(|skills| skills.get("entries"))
                .and_then(serde_json::Value::as_object)
                .map(|entries| {
                    entries
                        .iter()
                        .filter(|(_, entry)| {
                            entry.get("enabled").and_then(serde_json::Value::as_bool) == Some(false)
                        })
                        .map(|(key, _)| key.clone())
                        .collect::<Vec<_>>()
                })
        })
        .unwrap_or_default()
}

fn patch_openclaw_config(
    config_text: &str,
    skill_key: &str,
    enabled: bool,
) -> Result<String, AdapterError> {
    let mut value = openclaw_config_json(config_text)?;
    let entries = openclaw_entries_object_mut(&mut value)?;
    let entry = entries
        .entry(skill_key.to_string())
        .or_insert_with(|| serde_json::json!({}));
    let Some(entry) = entry.as_object_mut() else {
        return Err(AdapterError::new(format!(
            "OpenClaw config `skills.entries.{skill_key}` must be an object before it can be patched"
        )));
    };
    entry.insert("enabled".to_string(), serde_json::Value::Bool(enabled));

    let mut text = serde_json::to_string_pretty(&value)
        .map_err(|err| AdapterError::new(format!("failed to serialize OpenClaw config: {err}")))?;
    text.push('\n');
    Ok(text)
}

fn openclaw_config_json(config_text: &str) -> Result<serde_json::Value, AdapterError> {
    if config_text.trim().is_empty() {
        return Ok(serde_json::json!({
            "skills": {
                "entries": {}
            }
        }));
    }
    json5::from_str(config_text)
        .map_err(|err| AdapterError::new(format!("invalid OpenClaw JSON5 config: {err}")))
}

fn openclaw_entries_object_mut(
    value: &mut serde_json::Value,
) -> Result<&mut serde_json::Map<String, serde_json::Value>, AdapterError> {
    let Some(root) = value.as_object_mut() else {
        return Err(AdapterError::new(
            "OpenClaw config must be an object before it can be patched",
        ));
    };
    let skills = root
        .entry("skills".to_string())
        .or_insert_with(|| serde_json::json!({}));
    let Some(skills) = skills.as_object_mut() else {
        return Err(AdapterError::new(
            "OpenClaw config `skills` must be an object before it can be patched",
        ));
    };
    let entries = skills
        .entry("entries".to_string())
        .or_insert_with(|| serde_json::json!({}));
    entries.as_object_mut().ok_or_else(|| {
        AdapterError::new(
            "OpenClaw config `skills.entries` must be an object before it can be patched",
        )
    })
}

pub fn openclaw_config_key_from_frontmatter(frontmatter_raw: &str, fallback_name: &str) -> String {
    serde_norway::from_str::<serde_norway::Value>(frontmatter_raw)
        .ok()
        .and_then(|frontmatter| {
            frontmatter
                .get("metadata")
                .and_then(|metadata| metadata.get("openclaw"))
                .and_then(|openclaw| openclaw.get("skillKey"))
                .and_then(serde_norway::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| fallback_name.to_string())
}

fn openclaw_config_key_for_instance(instance: &SkillInstance) -> String {
    if !instance.frontmatter_raw.is_empty() {
        return openclaw_config_key_from_frontmatter(&instance.frontmatter_raw, &instance.name);
    }
    let Ok(content) = std::fs::read_to_string(&instance.path) else {
        return instance.name.clone();
    };
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"));
    let Some(rest) = rest else {
        return instance.name.clone();
    };
    let Ok((frontmatter_raw, _)) = split_yaml_frontmatter(rest) else {
        return instance.name.clone();
    };
    openclaw_config_key_from_frontmatter(frontmatter_raw, &instance.name)
}

fn openclaw_selected_workspace_roots(ctx: &AdapterContext, state_dir: &Path) -> Vec<PathBuf> {
    let selected_paths = [ctx.project_root.as_ref(), ctx.project_cwd.as_ref()]
        .into_iter()
        .flatten()
        .flat_map(|selected| normalized_path_variants(selected))
        .collect::<Vec<_>>();
    openclaw_workspace_candidates(ctx, state_dir)
        .into_iter()
        .filter(|candidate| {
            let candidate_paths = normalized_path_variants(candidate);
            selected_paths.iter().any(|selected| {
                candidate_paths
                    .iter()
                    .any(|candidate| selected == candidate || selected.starts_with(candidate))
            })
        })
        .collect()
}

fn openclaw_workspace_candidates(ctx: &AdapterContext, state_dir: &Path) -> Vec<PathBuf> {
    let mut candidates = Vec::new();
    if let Some(workspace) = absolute_env_path("OPENCLAW_WORKSPACE_DIR") {
        candidates.push(workspace);
    }
    if let Ok(content) = std::fs::read_to_string(openclaw_config_path(ctx)) {
        if let Ok(config) = json5::from_str::<serde_json::Value>(&content) {
            let default_workspace = config
                .get("agents")
                .and_then(|agents| agents.get("defaults"))
                .and_then(|defaults| defaults.get("workspace"))
                .and_then(serde_json::Value::as_str);
            if let Some(path) =
                default_workspace.and_then(|raw| expand_local_path(raw, &ctx.user_home, state_dir))
            {
                candidates.push(path);
            }
            if let Some(agents) = config
                .get("agents")
                .and_then(|agents| agents.get("list"))
                .and_then(serde_json::Value::as_array)
            {
                candidates.extend(agents.iter().filter_map(|agent| {
                    agent
                        .get("workspace")
                        .and_then(serde_json::Value::as_str)
                        .and_then(|raw| expand_local_path(raw, &ctx.user_home, state_dir))
                }));
            }
        }
    }
    if candidates.is_empty() {
        candidates.push(state_dir.join("workspace"));
    }
    let mut seen = HashSet::new();
    candidates
        .into_iter()
        .filter(|path| seen.insert(path.clone()))
        .collect()
}

fn openclaw_bundled_skill_roots() -> Vec<AdapterRoot> {
    openclaw_package_root_from_path()
        .into_iter()
        .map(|package_root| AdapterRoot {
            scope: Scope::AgentGlobal,
            path: package_root.join("skills"),
            source: RootSource::System,
            logical_source_id: Some("system".to_string()),
        })
        .collect()
}

fn openclaw_package_root_from_path() -> Option<PathBuf> {
    let executable = env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| env::split_paths(&paths).collect::<Vec<_>>())
        .map(|directory| directory.join("openclaw"))
        .find(|candidate| candidate.is_file())?
        .canonicalize()
        .ok()?;
    executable.ancestors().find_map(|ancestor| {
        let manifest = ancestor.join("package.json");
        let content = std::fs::read_to_string(manifest).ok()?;
        let value = serde_json::from_str::<serde_json::Value>(&content).ok()?;
        (value.get("name").and_then(serde_json::Value::as_str) == Some("openclaw"))
            .then(|| ancestor.to_path_buf())
    })
}

fn openclaw_extra_skill_roots(ctx: &AdapterContext, state_dir: &Path) -> Vec<AdapterRoot> {
    let Ok(content) = std::fs::read_to_string(openclaw_config_path(ctx)) else {
        return Vec::new();
    };
    let Ok(config) = json5::from_str::<serde_json::Value>(&content) else {
        return Vec::new();
    };
    config
        .get("skills")
        .and_then(|skills| skills.get("load"))
        .and_then(|load| load.get("extraDirs"))
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .filter_map(|raw| expand_local_path(raw, &ctx.user_home, state_dir))
        .enumerate()
        .map(|(index, path)| AdapterRoot {
            scope: Scope::AgentGlobal,
            path,
            source: RootSource::Configured,
            logical_source_id: Some(format!("configured-extra:{index}")),
        })
        .collect()
}

fn openclaw_plugin_skill_roots(ctx: &AdapterContext, state_dir: &Path) -> Vec<AdapterRoot> {
    let config = std::fs::read_to_string(openclaw_config_path(ctx))
        .ok()
        .and_then(|content| json5::from_str::<serde_json::Value>(&content).ok());
    let mut candidates = Vec::<(PathBuf, bool)>::new();
    if let Some(config) = config.as_ref() {
        candidates.extend(
            config
                .get("plugins")
                .and_then(|plugins| plugins.get("load"))
                .and_then(|load| load.get("paths"))
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .filter_map(|raw| expand_local_path(raw, &ctx.user_home, state_dir))
                .map(|path| (path, false)),
        );
    }
    candidates.extend(
        openclaw_plugin_directories(&state_dir.join("extensions")).map(|path| (path, false)),
    );
    for workspace in openclaw_selected_workspace_roots(ctx, state_dir) {
        candidates.extend(
            openclaw_plugin_directories(&workspace.join(".openclaw/extensions"))
                .map(|path| (path, true)),
        );
    }

    let allow = config
        .as_ref()
        .and_then(|config| config.get("plugins"))
        .and_then(|plugins| plugins.get("allow"))
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<HashSet<_>>()
        });
    let deny = config
        .as_ref()
        .and_then(|config| config.get("plugins"))
        .and_then(|plugins| plugins.get("deny"))
        .and_then(serde_json::Value::as_array)
        .map(|items| {
            items
                .iter()
                .filter_map(serde_json::Value::as_str)
                .collect::<HashSet<_>>()
        })
        .unwrap_or_default();
    let entries = config
        .as_ref()
        .and_then(|config| config.get("plugins"))
        .and_then(|plugins| plugins.get("entries"))
        .and_then(serde_json::Value::as_object);

    let mut roots = Vec::new();
    for (candidate, workspace_origin) in candidates {
        let Ok(plugin_root) = candidate.canonicalize() else {
            continue;
        };
        let manifest_path = plugin_root.join("openclaw.plugin.json");
        let Ok(content) = std::fs::read_to_string(manifest_path) else {
            continue;
        };
        let Ok(manifest) = json5::from_str::<serde_json::Value>(&content) else {
            continue;
        };
        let Some(id) = manifest
            .get("id")
            .or_else(|| manifest.get("name"))
            .and_then(serde_json::Value::as_str)
        else {
            continue;
        };
        let explicitly_enabled = entries
            .and_then(|entries| entries.get(id))
            .and_then(|entry| entry.get("enabled"))
            .and_then(serde_json::Value::as_bool);
        if deny.contains(id)
            || allow.as_ref().is_some_and(|allow| !allow.contains(id))
            || explicitly_enabled == Some(false)
            || (workspace_origin && explicitly_enabled != Some(true))
        {
            continue;
        }
        let skills = manifest
            .get("skills")
            .into_iter()
            .flat_map(json_string_or_array);
        for raw in skills {
            let Some(path) = safe_openclaw_plugin_path(&plugin_root, raw) else {
                continue;
            };
            roots.push(AdapterRoot {
                scope: if workspace_origin {
                    Scope::AgentProject
                } else {
                    Scope::AgentGlobal
                },
                path,
                source: RootSource::Plugin,
                logical_source_id: adapter_logical_source_token("openclaw-plugin", id),
            });
        }
    }
    dedup_roots(roots)
}

fn openclaw_plugin_directories(root: &Path) -> impl Iterator<Item = PathBuf> {
    std::fs::read_dir(root)
        .into_iter()
        .flatten()
        .flatten()
        .filter_map(|entry| entry.path().is_dir().then(|| entry.path()))
}

fn json_string_or_array(value: &serde_json::Value) -> Vec<&str> {
    match value {
        serde_json::Value::String(value) => vec![value],
        serde_json::Value::Array(values) => values
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect(),
        _ => Vec::new(),
    }
}

fn safe_openclaw_plugin_path(plugin_root: &Path, raw: &str) -> Option<PathBuf> {
    let relative = Path::new(raw);
    if relative.is_absolute()
        || relative
            .components()
            .any(|component| matches!(component, std::path::Component::ParentDir))
    {
        return None;
    }
    let canonical = plugin_root.join(relative).canonicalize().ok()?;
    canonical.starts_with(plugin_root).then_some(canonical)
}

fn openclaw_declared_skill_link_targets(root: &AdapterRoot) -> Vec<AdapterRoot> {
    let mut targets = Vec::new();
    let mut stack = vec![(root.path.clone(), 0usize)];
    while let Some((directory, depth)) = stack.pop() {
        if depth >= 6 {
            continue;
        }
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if metadata.file_type().is_symlink() {
                if path.is_dir() {
                    targets.push(AdapterRoot {
                        scope: root.scope,
                        path,
                        source: RootSource::Configured,
                        logical_source_id: root
                            .logical_source_id
                            .as_ref()
                            .map(|logical| format!("{logical}:declared-link")),
                    });
                }
            } else if metadata.is_dir() {
                stack.push((path, depth + 1));
            }
        }
    }
    targets
}

fn dedup_roots(roots: Vec<AdapterRoot>) -> Vec<AdapterRoot> {
    let mut seen = HashSet::new();
    roots
        .into_iter()
        .filter(|root| seen.insert((root.scope, root.path.clone())))
        .collect()
}

fn containing_dir_name(path: &Path) -> String {
    path.parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn normalized_path_variants(path: &Path) -> Vec<PathBuf> {
    let lexical = normalize_path_lexically(path);
    match path.canonicalize() {
        Ok(canonical) if canonical != lexical => vec![lexical, canonical],
        Ok(_) | Err(_) => vec![lexical],
    }
}

#[cfg(test)]
mod tests {
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

        let parsed = parse_skill_content(&format!("---\n{raw}---\nBody.\n"), "fallback")
            .expect("adapter frontmatter parses");
        assert_eq!(parsed.name, "sample-skill");
        assert_eq!(parsed.description, "Sample");
        assert_eq!(
            openclaw_config_key_from_frontmatter(&parsed.frontmatter_raw, "fallback"),
            "routed-key"
        );
    }

    #[test]
    fn yaml_contract_malformed_frontmatter_returns_error() {
        let result = parse_skill_content(
            "---\nname: [unterminated\ndescription: Sample\n---\nBody.\n",
            "fallback",
        );

        assert!(result.is_err());
    }

    #[test]
    fn exposes_documented_read_only_roots_without_generic_project_roots() {
        let adapter = OpenclawAdapter;
        let ctx = AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: Some(PathBuf::from("/tmp/project")),
            project_cwd: Some(PathBuf::from("/tmp/project/nested")),
            extra_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: PathBuf::from("/tmp/unverified"),
                source: RootSource::Extra,
                logical_source_id: None,
            }],
        };

        let roots = adapter.roots(&ctx);

        assert_eq!(roots[0].path, PathBuf::from("/tmp/home/.agents/skills"));
        assert_eq!(roots[0].scope, Scope::AgentGlobal);
        assert_eq!(roots[0].source, RootSource::Compatibility);
        assert_eq!(roots[1].path, PathBuf::from("/tmp/home/.openclaw/skills"));
        assert_eq!(roots[1].scope, Scope::AgentGlobal);
        assert_eq!(roots[1].source, RootSource::UserHome);
        assert!(
            roots
                .iter()
                .all(|root| !root.path.starts_with("/tmp/project")),
            "OpenClaw must not infer arbitrary repository roots as project workspaces"
        );
        assert!(
            roots
                .iter()
                .all(|root| !root.path.starts_with("/tmp/unverified")),
            "OpenClaw must not consume generic extra roots as configured extraDirs"
        );
        assert!(roots
            .iter()
            .filter(|root| { root.path.starts_with("/usr") || root.path.starts_with("/opt") })
            .all(|root| root.source == RootSource::System));
    }

    #[test]
    fn exposes_home_openclaw_workspace_roots_when_project_is_workspace() {
        let adapter = OpenclawAdapter;
        let ctx = AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: Some(PathBuf::from("/tmp/home/.openclaw/workspace")),
            project_cwd: None,
            extra_roots: vec![],
        };

        let roots = adapter.roots(&ctx);

        assert!(roots.iter().any(|root| {
            root.scope == Scope::AgentProject
                && root.source == RootSource::Project
                && root.path == Path::new("/tmp/home/.openclaw/workspace/skills")
        }));
        assert!(roots.iter().any(|root| {
            root.scope == Scope::AgentProject
                && root.source == RootSource::Compatibility
                && root.path == Path::new("/tmp/home/.openclaw/workspace/.agents/skills")
        }));
    }

    #[test]
    fn exposes_home_openclaw_workspace_roots_when_selection_is_inside_workspace() {
        let adapter = OpenclawAdapter;
        let ctx = AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: Some(PathBuf::from("/tmp/home/.openclaw/workspace/repo")),
            project_cwd: Some(PathBuf::from("/tmp/home/.openclaw/workspace/repo/nested")),
            extra_roots: vec![],
        };

        let roots = adapter.roots(&ctx);

        assert!(roots.iter().any(|root| {
            root.scope == Scope::AgentProject
                && root.source == RootSource::Project
                && root.path == Path::new("/tmp/home/.openclaw/workspace/skills")
        }));
        assert!(roots.iter().any(|root| {
            root.scope == Scope::AgentProject
                && root.source == RootSource::Compatibility
                && root.path == Path::new("/tmp/home/.openclaw/workspace/.agents/skills")
        }));
        assert!(
            roots
                .iter()
                .all(|root| !root.path.starts_with("/tmp/home/.openclaw/workspace/repo")),
            "OpenClaw must scan the confirmed workspace roots, not infer nested repo roots"
        );
    }

    #[test]
    fn plugin_manifest_roots_are_enabled_explicitly_and_never_scan_extension_cache_broadly() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-openclaw-plugins-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let state = home.join(".openclaw");
        let enabled = state.join("extensions/enabled-plugin");
        let disabled = state.join("extensions/disabled-plugin");
        let unlisted = state.join("extensions/unlisted-plugin");
        for plugin in [&enabled, &disabled, &unlisted] {
            std::fs::create_dir_all(plugin.join("declared-skill"))
                .expect("create declared plugin skill root");
            std::fs::create_dir_all(plugin.join("cache-noise/hidden"))
                .expect("create plugin cache noise");
        }
        std::fs::write(
            enabled.join("openclaw.plugin.json"),
            r#"{"id":"enabled","skills":["declared-skill"]}"#,
        )
        .expect("write enabled plugin manifest");
        std::fs::write(
            disabled.join("openclaw.plugin.json"),
            r#"{"id":"disabled","skills":["declared-skill"]}"#,
        )
        .expect("write disabled plugin manifest");
        std::fs::write(
            unlisted.join("openclaw.plugin.json"),
            r#"{"id":"unlisted","skills":["declared-skill"]}"#,
        )
        .expect("write unlisted plugin manifest");
        std::fs::write(
            state.join("openclaw.json"),
            r#"{"plugins":{"allow":["enabled","disabled"],"entries":{"enabled":{"enabled":true},"disabled":{"enabled":false}}}}"#,
        )
        .expect("write OpenClaw config");
        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        };

        let roots = OpenclawAdapter.roots(&ctx);

        assert!(roots.iter().any(|root| {
            root.source == RootSource::Plugin
                && root.path == enabled.join("declared-skill").canonicalize().unwrap()
        }));
        assert!(roots.iter().all(|root| !root.path.starts_with(&disabled)));
        assert!(roots.iter().all(|root| !root.path.starts_with(&unlisted)));
        assert!(roots
            .iter()
            .all(|root| root.path != state.join("extensions")));

        let plugin_root = AdapterRoot {
            scope: Scope::AgentGlobal,
            path: enabled.join("declared-skill"),
            source: RootSource::Plugin,
            logical_source_id: None,
        };
        let managed_root = AdapterRoot {
            scope: Scope::AgentGlobal,
            path: state.join("skills"),
            source: RootSource::UserHome,
            logical_source_id: Some("native-user".to_string()),
        };
        assert!(OpenclawAdapter.accepts_skill_path(&plugin_root, Path::new("SKILL.md")));
        assert!(!OpenclawAdapter.accepts_skill_path(&managed_root, Path::new("SKILL.md")));

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn parses_valid_openclaw_skill_frontmatter() {
        let adapter = OpenclawAdapter;
        let fixture =
            fixture_path("fixtures/openclaw/skill-evidence/sample-openclaw-skill/SKILL.md");

        let skill = adapter.parse(&fixture).expect("skill parses");

        assert_eq!(skill.agent, AgentId::Openclaw);
        assert_eq!(skill.name, "sample-openclaw-skill");
        assert_eq!(
            skill.description,
            "Evidence sample only for an OpenClaw skill directory containing SKILL.md."
        );
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.enabled);
    }

    #[test]
    fn path_and_content_parsing_are_equivalent() {
        let adapter = OpenclawAdapter;
        let fixture =
            fixture_path("fixtures/openclaw/skill-evidence/sample-openclaw-skill/SKILL.md");

        crate::assert_parse_equivalent(&adapter, &fixture);
    }

    #[test]
    fn falls_back_to_directory_name_when_name_is_missing() {
        let adapter = OpenclawAdapter;
        let fixture = fixture_path("fixtures/openclaw/broken/missing-name/SKILL.md");

        let skill = adapter.parse(&fixture).expect("skill parses with fallback");

        assert_eq!(skill.name, "missing-name");
        assert_eq!(skill.description, "Missing name fallback fixture.");
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.enabled);
    }

    #[test]
    fn keeps_missing_description_loaded_with_empty_description() {
        let adapter = OpenclawAdapter;
        let fixture = fixture_path("fixtures/openclaw/broken/missing-description/SKILL.md");

        let skill = adapter.parse(&fixture).expect("skill parses");

        assert_eq!(skill.name, "missing-description");
        assert_eq!(skill.description, "");
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.enabled);
    }

    #[test]
    fn patch_enabled_accepts_json5_and_writes_entries_enabled() {
        let mut doc = AgentConfigDocument {
            path: PathBuf::from("/tmp/home/.openclaw/openclaw.json"),
            format: skills_copilot_core::ConfigFormat::Json,
            text: "{\n  skills: {\n    entries: {\n      \"image-lab\": { enabled: true, apiKey: { source: \"env\", id: \"KEY\" } },\n    },\n  },\n}\n".to_string(),
        };
        let skill = SkillInstance {
            id: "openclaw:test".to_string(),
            agent: AgentId::Openclaw,
            scope: Scope::AgentGlobal,
            project_root: None,
            path: PathBuf::from("/tmp/home/.openclaw/skills/image-lab/SKILL.md"),
            display_path: PathBuf::from("/tmp/home/.openclaw/skills/image-lab/SKILL.md"),
            definition_id: "image-lab".to_string(),
            name: "image-lab".to_string(),
            display_name: "image-lab".to_string(),
            description: String::new(),
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
        };

        OpenclawAdapter
            .patch_enabled(&mut doc, &skill, false)
            .expect("disable succeeds");
        assert!(openclaw_disabled_skill_keys(&doc.text).contains(&"image-lab".to_string()));
        assert!(doc.text.contains("\"apiKey\""));

        OpenclawAdapter
            .patch_enabled(&mut doc, &skill, true)
            .expect("enable succeeds");
        assert!(!openclaw_disabled_skill_keys(&doc.text).contains(&"image-lab".to_string()));
    }

    #[test]
    fn config_key_prefers_openclaw_skill_key_metadata() {
        let frontmatter = "name: visible-name\nmetadata:\n  openclaw:\n    skillKey: routed-key\n";
        assert_eq!(
            openclaw_config_key_from_frontmatter(frontmatter, "visible-name"),
            "routed-key"
        );
    }

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }
}
