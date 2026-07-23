use std::{
    collections::{HashMap, HashSet},
    path::{Component, Path, PathBuf},
};

use skills_copilot_core::{
    adapter_logical_source_token, AdapterContext, AdapterError, AdapterRoot, AgentAdapter,
    AgentConfigAdapter, AgentConfigDocument, AgentId, PermissionRequest, RootSource, Scope,
    SkillInstance, SkillState,
};

use crate::environment::absolute_env_path;

#[derive(Debug, Default)]
pub struct ClaudeCodeAdapter;

impl AgentAdapter for ClaudeCodeAdapter {
    fn id(&self) -> AgentId {
        AgentId::ClaudeCode
    }

    fn display_name(&self) -> &'static str {
        "Claude Code"
    }

    fn roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
        let personal_root = AdapterRoot {
            scope: Scope::AgentGlobal,
            path: claude_config_dir(ctx).join("skills"),
            source: RootSource::UserHome,
            logical_source_id: Some("claude-user-skills".to_string()),
        };
        let mut roots = vec![personal_root.clone()];
        roots.extend(claude_declared_skill_link_roots(&personal_root));
        push_claude_commands_root(
            &mut roots,
            Scope::AgentGlobal,
            claude_config_dir(ctx).join("commands"),
            0,
        );

        if let Some(project_root) = &ctx.project_root {
            for (level, directory) in claude_project_directories(ctx, project_root).enumerate() {
                let root = AdapterRoot {
                    scope: Scope::AgentProject,
                    path: directory.join(".claude/skills"),
                    source: RootSource::Project,
                    logical_source_id: Some(format!("claude-project-skills:{level}")),
                };
                roots.push(root.clone());
                roots.extend(claude_declared_skill_link_roots(&root));
                push_claude_commands_root(
                    &mut roots,
                    Scope::AgentProject,
                    directory.join(".claude/commands"),
                    level,
                );
            }
        }

        roots.extend(claude_enabled_plugin_skill_roots(ctx));
        roots.extend(ctx.extra_roots.clone());
        dedup_roots(roots)
    }

    fn link_target_roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
        let mut roots = vec![AdapterRoot {
            scope: Scope::AgentGlobal,
            path: ctx.user_home.join(".agents/skills"),
            source: RootSource::Compatibility,
            logical_source_id: Some("agents-shared-skills".to_string()),
        }];
        if let Some(project_root) = &ctx.project_root {
            roots.extend(
                claude_project_directories(ctx, project_root)
                    .enumerate()
                    .map(|(level, directory)| AdapterRoot {
                        scope: Scope::AgentProject,
                        path: directory.join(".agents/skills"),
                        source: RootSource::Compatibility,
                        logical_source_id: Some(format!("agents-project-skills:{level}")),
                    }),
            );
        }
        roots
    }

    fn parse(&self, path: &Path) -> Result<SkillInstance, AdapterError> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| AdapterError::new(format!("failed to read skill: {err}")))?;
        self.parse_content(path, content)
    }

    fn parse_content(&self, path: &Path, content: String) -> Result<SkillInstance, AdapterError> {
        let is_command = is_claude_command_path(path);
        let display_name = (if is_command {
            path.file_stem()
        } else {
            path.parent().and_then(Path::file_name)
        })
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string();
        let parsed = parse_skill_content(&content);
        let (frontmatter_raw, body, name, description, permissions, state, enabled) = match parsed {
            Ok(parsed) => (
                parsed.frontmatter_raw,
                parsed.body,
                if is_command {
                    display_name.clone()
                } else {
                    parsed.name.unwrap_or_else(|| display_name.clone())
                },
                parsed.description,
                parsed.permissions,
                SkillState::Loaded,
                true,
            ),
            Err(_message) if is_command && !starts_with_yaml_frontmatter(&content) => (
                String::new(),
                content.clone(),
                display_name.clone(),
                first_markdown_paragraph(&content),
                PermissionRequest::default(),
                SkillState::Loaded,
                true,
            ),
            Err(_message) => (
                String::new(),
                content,
                display_name.clone(),
                String::new(),
                PermissionRequest::default(),
                SkillState::Broken,
                false,
            ),
        };

        Ok(SkillInstance {
            id: stable_path_id(path),
            agent: AgentId::ClaudeCode,
            scope: Scope::AgentProject,
            project_root: None,
            path: PathBuf::from(path),
            display_path: PathBuf::from(path),
            definition_id: name.clone(),
            name: name.clone(),
            display_name,
            description,
            version: None,
            state,
            enabled,
            frontmatter_raw,
            body,
            scripts: Vec::new(),
            permissions,
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
        if is_claude_commands_root(root) {
            return components == 1
                && relative_path
                    .extension()
                    .and_then(|extension| extension.to_str())
                    == Some("md")
                && relative_path
                    .file_stem()
                    .and_then(|name| name.to_str())
                    .is_some_and(|name| !name.is_empty() && !name.starts_with('.'));
        }
        relative_path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md")
            && if root.source == RootSource::Plugin {
                (1..=2).contains(&components)
            } else {
                components == 2
            }
    }

    fn is_skill_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    }

    fn config_paths(&self, ctx: &AdapterContext) -> Vec<PathBuf> {
        let mut paths = vec![claude_config_dir(ctx).join("settings.json")];
        if let Some(project_root) = &ctx.project_root {
            paths.push(project_root.join(".claude/settings.json"));
            paths.push(project_root.join(".claude/settings.local.json"));
        }
        paths.push(PathBuf::from(
            "/Library/Application Support/ClaudeCode/managed-settings.json",
        ));
        paths.push(PathBuf::from("/etc/claude-code/managed-settings.json"));
        paths.dedup();
        paths
    }
}

pub fn claude_config_dir(ctx: &AdapterContext) -> PathBuf {
    absolute_env_path("CLAUDE_CONFIG_DIR").unwrap_or_else(|| ctx.user_home.join(".claude"))
}

fn claude_project_directories<'a>(
    ctx: &'a AdapterContext,
    project_root: &'a Path,
) -> impl Iterator<Item = &'a Path> {
    let start = ctx
        .project_cwd
        .as_deref()
        .filter(|cwd| cwd.starts_with(project_root))
        .unwrap_or(project_root);
    std::iter::successors(Some(start), move |directory| {
        (*directory != project_root)
            .then(|| directory.parent())
            .flatten()
            .filter(|parent| parent.starts_with(project_root))
    })
}

fn push_claude_commands_root(
    roots: &mut Vec<AdapterRoot>,
    scope: Scope,
    path: PathBuf,
    level: usize,
) {
    if path.is_dir() {
        roots.push(AdapterRoot {
            scope,
            path,
            source: match scope {
                Scope::AgentGlobal => RootSource::UserHome,
                Scope::AgentProject => RootSource::Project,
                _ => RootSource::Project,
            },
            logical_source_id: Some(if scope == Scope::AgentGlobal {
                "claude-user-commands".to_string()
            } else {
                format!("claude-project-commands:{level}")
            }),
        });
    }
}

fn is_claude_commands_root(root: &AdapterRoot) -> bool {
    root.path.file_name().and_then(|name| name.to_str()) == Some("commands")
        && matches!(root.source, RootSource::UserHome | RootSource::Project)
}

fn is_claude_command_path(path: &Path) -> bool {
    path.extension().and_then(|extension| extension.to_str()) == Some("md")
        && path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            == Some("commands")
}

fn starts_with_yaml_frontmatter(content: &str) -> bool {
    content.starts_with("---\n") || content.starts_with("---\r\n")
}

fn claude_declared_skill_link_roots(root: &AdapterRoot) -> Vec<AdapterRoot> {
    let Ok(entries) = std::fs::read_dir(&root.path) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path();
            let metadata = std::fs::symlink_metadata(&path).ok()?;
            if !metadata.file_type().is_symlink() || !path.join("SKILL.md").is_file() {
                return None;
            }
            Some(AdapterRoot {
                scope: root.scope,
                path,
                // Claude 2.1.203+ treats an immediate skill-directory symlink
                // as an explicit declaration. The resolved target is therefore
                // a narrow authorized root, never a reason to allow a broad
                // directory outside the normal Claude locations.
                source: RootSource::Configured,
                logical_source_id: root
                    .logical_source_id
                    .as_ref()
                    .map(|logical| format!("{logical}:declared-link")),
            })
        })
        .collect()
}

fn claude_enabled_plugin_skill_roots(ctx: &AdapterContext) -> Vec<AdapterRoot> {
    let config_dir = claude_config_dir(ctx);
    let enabled = claude_effective_enabled_plugins(ctx);
    if enabled.is_empty() {
        return Vec::new();
    }
    let installed_path = config_dir.join("plugins/installed_plugins.json");
    let Ok(content) = std::fs::read_to_string(installed_path) else {
        return Vec::new();
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return Vec::new();
    };
    let Some(installed) = value.get("plugins").and_then(serde_json::Value::as_object) else {
        return Vec::new();
    };
    let plugins_root = config_dir.join("plugins");
    let canonical_plugins_root = plugins_root
        .canonicalize()
        .unwrap_or_else(|_| plugins_root.clone());
    let mut roots = Vec::new();
    for (plugin_id, installations) in installed {
        if !enabled.contains(plugin_id) {
            continue;
        }
        let Some(installations) = installations.as_array() else {
            continue;
        };
        let Some(install_path) = installations.iter().rev().find_map(|installation| {
            installation
                .get("installPath")
                .and_then(serde_json::Value::as_str)
                .map(PathBuf::from)
                .filter(|path| path.is_absolute())
        }) else {
            continue;
        };
        let Ok(canonical_install) = install_path.canonicalize() else {
            continue;
        };
        if !canonical_install.starts_with(&canonical_plugins_root) {
            continue;
        }
        let manifest_path = canonical_install.join(".claude-plugin/plugin.json");
        let manifest = std::fs::read_to_string(&manifest_path)
            .ok()
            .and_then(|text| serde_json::from_str::<serde_json::Value>(&text).ok());
        let manifest_skills = manifest
            .as_ref()
            .and_then(|manifest| manifest.get("skills"));
        let logical_source_id = adapter_logical_source_token("claude-plugin", plugin_id);

        let default_skills = canonical_install.join("skills");
        if default_skills.is_dir() {
            roots.push(AdapterRoot {
                scope: Scope::AgentGlobal,
                path: default_skills,
                source: RootSource::Plugin,
                logical_source_id: logical_source_id.clone(),
            });
        }
        for raw in manifest_skills.into_iter().flat_map(json_string_or_array) {
            let Some(candidate) = safe_plugin_relative_path(&canonical_install, raw) else {
                continue;
            };
            roots.push(AdapterRoot {
                scope: Scope::AgentGlobal,
                path: candidate,
                source: RootSource::Plugin,
                logical_source_id: logical_source_id.clone(),
            });
        }
        if manifest_skills.is_none()
            && !canonical_install.join("skills").exists()
            && canonical_install.join("SKILL.md").is_file()
        {
            roots.push(AdapterRoot {
                scope: Scope::AgentGlobal,
                path: canonical_install.join("SKILL.md"),
                source: RootSource::Plugin,
                logical_source_id: logical_source_id.clone(),
            });
        }
    }
    dedup_roots(roots)
}

fn claude_effective_enabled_plugins(ctx: &AdapterContext) -> HashSet<String> {
    let mut states = HashMap::<String, bool>::new();
    for settings_path in ClaudeCodeAdapter.config_paths(ctx) {
        let Ok(content) = std::fs::read_to_string(settings_path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
            continue;
        };
        let Some(entries) = value
            .get("enabledPlugins")
            .and_then(serde_json::Value::as_object)
        else {
            continue;
        };
        for (id, enabled) in entries {
            if let Some(enabled) = enabled.as_bool() {
                states.insert(id.clone(), enabled);
            }
        }
    }
    states
        .into_iter()
        .filter_map(|(id, enabled)| enabled.then_some(id))
        .collect()
}

fn json_string_or_array(value: &serde_json::Value) -> Vec<&str> {
    match value {
        serde_json::Value::String(value) => vec![value.as_str()],
        serde_json::Value::Array(values) => values
            .iter()
            .filter_map(serde_json::Value::as_str)
            .collect(),
        _ => Vec::new(),
    }
}

fn safe_plugin_relative_path(plugin_root: &Path, raw: &str) -> Option<PathBuf> {
    if !raw.starts_with("./") {
        return None;
    }
    let relative = Path::new(raw);
    if relative.components().any(|component| {
        matches!(
            component,
            Component::ParentDir | Component::RootDir | Component::Prefix(_)
        )
    }) {
        return None;
    }
    let candidate = plugin_root.join(relative);
    let canonical = candidate.canonicalize().ok()?;
    canonical.starts_with(plugin_root).then_some(canonical)
}

fn dedup_roots(roots: Vec<AdapterRoot>) -> Vec<AdapterRoot> {
    let mut seen = HashSet::new();
    roots
        .into_iter()
        .filter(|root| seen.insert((root.scope, root.path.clone())))
        .collect()
}

impl AgentConfigAdapter for ClaudeCodeAdapter {
    fn patch_enabled(
        &self,
        doc: &mut AgentConfigDocument,
        instance: &SkillInstance,
        on: bool,
    ) -> Result<(), AdapterError> {
        let mut root = if doc.text.trim().is_empty() {
            serde_json::json!({})
        } else {
            serde_json::from_str(&doc.text)
                .map_err(|err| AdapterError::new(format!("invalid Claude settings JSON: {err}")))?
        };

        let root_obj = root
            .as_object_mut()
            .ok_or_else(|| AdapterError::new("Claude settings must be a JSON object"))?;
        let overrides = root_obj
            .entry("skillOverrides")
            .or_insert_with(|| serde_json::json!({}));
        let overrides_obj = overrides
            .as_object_mut()
            .ok_or_else(|| AdapterError::new("skillOverrides must be a JSON object"))?;

        if on {
            overrides_obj.remove(&instance.name);
        } else {
            overrides_obj.insert(instance.name.clone(), serde_json::json!("off"));
        }

        doc.text = serde_json::to_string_pretty(&root)
            .map_err(|err| AdapterError::new(format!("failed to serialize settings: {err}")))?;
        doc.text.push('\n');
        Ok(())
    }
}

struct ParsedSkill {
    frontmatter_raw: String,
    body: String,
    name: Option<String>,
    description: String,
    permissions: PermissionRequest,
}

fn parse_skill_content(content: &str) -> Result<ParsedSkill, String> {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .ok_or_else(|| "missing YAML frontmatter".to_string())?;
    let (frontmatter_raw, body) = split_frontmatter(rest)?;
    let frontmatter: serde_norway::Value =
        serde_norway::from_str(frontmatter_raw).map_err(|err| err.to_string())?;
    let name = frontmatter
        .get("name")
        .and_then(serde_norway::Value::as_str)
        .map(ToString::to_string);
    let description = frontmatter
        .get("description")
        .and_then(serde_norway::Value::as_str)
        .map(ToString::to_string)
        .unwrap_or_else(|| first_markdown_paragraph(&body));
    let permissions = PermissionRequest {
        tools: parse_allowed_tools(frontmatter.get("allowed-tools")),
        ..PermissionRequest::default()
    };

    Ok(ParsedSkill {
        frontmatter_raw: frontmatter_raw.to_string(),
        body,
        name,
        description,
        permissions,
    })
}

fn split_frontmatter(rest: &str) -> Result<(&str, String), String> {
    let mut line_start = 0;
    for line in rest.split_inclusive('\n') {
        let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
        let line_content = line_without_newline
            .strip_suffix('\r')
            .unwrap_or(line_without_newline);
        if line_content == "---" {
            let frontmatter = rest[..line_start]
                .strip_suffix("\r\n")
                .or_else(|| rest[..line_start].strip_suffix('\n'))
                .unwrap_or(&rest[..line_start]);
            return Ok((frontmatter, rest[line_start + line.len()..].to_string()));
        }
        line_start += line.len();
    }
    Err("unterminated YAML frontmatter".to_string())
}

fn parse_allowed_tools(value: Option<&serde_norway::Value>) -> Vec<String> {
    match value {
        Some(serde_norway::Value::String(raw)) => raw
            .split(|ch: char| ch.is_whitespace() || ch == ',')
            .filter(|part| !part.is_empty())
            .map(ToString::to_string)
            .collect(),
        Some(serde_norway::Value::Sequence(items)) => items
            .iter()
            .filter_map(serde_norway::Value::as_str)
            .map(ToString::to_string)
            .collect(),
        _ => Vec::new(),
    }
}

fn first_markdown_paragraph(body: &str) -> String {
    body.split("\n\n")
        .map(str::trim)
        .find(|part| !part.is_empty() && !part.starts_with('#'))
        .unwrap_or_default()
        .to_string()
}

fn stable_path_id(path: &Path) -> String {
    format!("claude-code:{}", path.display())
}

#[cfg(test)]
mod tests {
    use skills_copilot_core::{AgentConfigDocument, ConfigFormat, SkillState};

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
        assert_eq!(parsed.name.as_deref(), Some("sample-skill"));
        assert_eq!(parsed.description, "Sample");
        assert_eq!(parsed.permissions.tools, vec!["Read", "Search"]);
    }

    #[test]
    fn yaml_contract_malformed_frontmatter_returns_error() {
        let result =
            parse_skill_content("---\nname: [unterminated\ndescription: Sample\n---\nBody.\n");

        assert!(result.is_err());
    }

    #[test]
    fn project_agents_root_authorizes_same_scope_claude_skill_links() {
        let adapter = ClaudeCodeAdapter;
        let ctx = AdapterContext {
            user_home: PathBuf::from("/tmp/home"),
            project_root: Some(PathBuf::from("/tmp/project")),
            project_cwd: Some(PathBuf::from("/tmp/project")),
            extra_roots: vec![],
        };

        let roots = adapter.link_target_roots(&ctx);

        assert!(roots.iter().any(|root| {
            root.scope == Scope::AgentProject
                && root.source == RootSource::Compatibility
                && root.path == Path::new("/tmp/project/.agents/skills")
        }));
    }

    #[test]
    fn project_commands_are_exposed_as_legacy_claude_skills() {
        let root = temp_test_root("skills-copilot-claude-commands");
        let home = root.join("home");
        let project = root.join("project");
        let commands = project.join(".claude/commands");
        std::fs::create_dir_all(&commands).expect("create commands dir");
        let command = commands.join("gan-build.md");
        std::fs::write(
            &command,
            "# GAN Build\n\nBuild with a generator/evaluator loop.\n",
        )
        .expect("write command");

        let ctx = AdapterContext {
            user_home: home,
            project_root: Some(project.clone()),
            project_cwd: Some(project),
            extra_roots: Vec::new(),
        };
        let roots = ClaudeCodeAdapter.roots(&ctx);
        let command_root = roots
            .iter()
            .find(|candidate| candidate.path == commands)
            .expect("Claude project commands root");

        assert_eq!(command_root.scope, Scope::AgentProject);
        assert_eq!(command_root.source, RootSource::Project);
        assert!(ClaudeCodeAdapter.accepts_skill_path(command_root, Path::new("gan-build.md")));
        assert!(
            !ClaudeCodeAdapter.accepts_skill_path(command_root, Path::new("nested/gan-build.md"))
        );

        let skill = ClaudeCodeAdapter
            .parse(&command)
            .expect("legacy command parses");
        assert_eq!(skill.name, "gan-build");
        assert_eq!(skill.display_name, "gan-build");
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.enabled);

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn plugin_cache_is_not_a_root_and_only_enabled_manifest_skills_are_exposed() {
        let root = temp_test_root("skills-copilot-claude-plugins");
        let home = root.join("home");
        let config = home.join(".claude");
        let enabled = config.join("plugins/cache/market/enabled/1.0.0");
        let disabled = config.join("plugins/cache/market/disabled/1.0.0");
        for plugin in [&enabled, &disabled] {
            std::fs::create_dir_all(plugin.join(".claude-plugin"))
                .expect("create plugin manifest dir");
            std::fs::create_dir_all(plugin.join("skills/example"))
                .expect("create plugin skill dir");
        }
        std::fs::write(
            enabled.join(".claude-plugin/plugin.json"),
            r#"{"name":"enabled-plugin","skills":["./extra"]}"#,
        )
        .expect("write enabled manifest");
        std::fs::create_dir_all(enabled.join("extra/custom")).expect("create custom skill root");
        std::fs::write(
            disabled.join(".claude-plugin/plugin.json"),
            r#"{"name":"disabled-plugin"}"#,
        )
        .expect("write disabled manifest");
        std::fs::create_dir_all(&config).expect("create Claude config");
        std::fs::write(
            config.join("settings.json"),
            r#"{"enabledPlugins":{"enabled@market":true,"disabled@market":false}}"#,
        )
        .expect("write Claude settings");
        std::fs::create_dir_all(config.join("plugins")).expect("create plugins dir");
        std::fs::write(
            config.join("plugins/installed_plugins.json"),
            serde_json::json!({
                "version": 2,
                "plugins": {
                    "enabled@market": [{"installPath": enabled}],
                    "disabled@market": [{"installPath": disabled}]
                }
            })
            .to_string(),
        )
        .expect("write installed plugin index");
        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        };

        let roots = ClaudeCodeAdapter.roots(&ctx);
        let canonical_enabled = enabled.canonicalize().expect("canonicalize enabled plugin");
        let canonical_disabled = disabled
            .canonicalize()
            .expect("canonicalize disabled plugin");

        assert!(roots
            .iter()
            .any(|root| root.path == canonical_enabled.join("skills")));
        assert!(roots
            .iter()
            .any(|root| root.path == canonical_enabled.join("extra")));
        assert!(roots
            .iter()
            .all(|root| !root.path.starts_with(&canonical_disabled)));
        assert!(roots
            .iter()
            .all(|root| root.path != config.join("plugins/cache")));

        let _ = std::fs::remove_dir_all(root);
    }

    #[test]
    fn parses_valid_skill_frontmatter() {
        let adapter = ClaudeCodeAdapter;
        let fixture = fixture_path("fixtures/claude-code/personal/valid-summarize/SKILL.md");

        let skill = adapter.parse(&fixture).expect("skill parses");

        assert_eq!(skill.name, "summarize-changes");
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.permissions.tools.iter().any(|tool| tool == "Read"));
    }

    #[test]
    fn path_and_content_parsing_are_equivalent() {
        let adapter = ClaudeCodeAdapter;
        let fixture = fixture_path("fixtures/claude-code/personal/valid-summarize/SKILL.md");

        crate::assert_parse_equivalent(&adapter, &fixture);
    }

    #[test]
    fn marks_missing_frontmatter_as_broken() {
        let adapter = ClaudeCodeAdapter;
        let fixture =
            fixture_path("fixtures/claude-code/project/broken-missing-frontmatter/SKILL.md");

        let skill = adapter.parse(&fixture).expect("broken skill is returned");

        assert_eq!(skill.state, SkillState::Broken);
        assert!(!skill.enabled);
    }

    #[test]
    fn parses_crlf_frontmatter() {
        let adapter = ClaudeCodeAdapter;
        let root = temp_test_root("skills-copilot-claude-crlf");
        let skill_dir = root.join("crlf-skill");
        std::fs::create_dir_all(&skill_dir).expect("create skill dir");
        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\r\nname: crlf-skill\r\ndescription: CRLF frontmatter\r\nallowed-tools: Read\r\n---\r\nBody.\r\n",
        )
        .expect("write skill");

        let skill = adapter.parse(&skill_path).expect("skill parses");

        assert_eq!(skill.name, "crlf-skill");
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.enabled);
        assert!(skill.permissions.tools.iter().any(|tool| tool == "Read"));

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn rejects_garbage_frontmatter_closing_delimiter() {
        let adapter = ClaudeCodeAdapter;
        let root = temp_test_root("skills-copilot-claude-garbage-closing");
        let skill_dir = root.join("garbage-closing");
        std::fs::create_dir_all(&skill_dir).expect("create skill dir");
        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(
            &skill_path,
            "---\nname: garbage-closing\ndescription: invalid delimiter\n---garbage\nBody.\n",
        )
        .expect("write skill");

        let skill = adapter
            .parse(&skill_path)
            .expect("broken skill is returned");

        assert_eq!(skill.state, SkillState::Broken);
        assert!(!skill.enabled);

        let _ = std::fs::remove_dir_all(&root);
    }

    #[test]
    fn patches_skill_overrides_off_and_on() {
        let adapter = ClaudeCodeAdapter;
        let mut doc = AgentConfigDocument {
            path: PathBuf::from(".claude/settings.local.json"),
            format: ConfigFormat::Json,
            text: "{}".to_string(),
        };
        let mut skill = adapter
            .parse(&fixture_path(
                "fixtures/claude-code/project/valid-review/SKILL.md",
            ))
            .expect("skill parses");
        skill.name = "review-pr".to_string();

        adapter
            .patch_enabled(&mut doc, &skill, false)
            .expect("disable patch succeeds");
        assert!(doc.text.contains("\"review-pr\": \"off\""));

        adapter
            .patch_enabled(&mut doc, &skill, true)
            .expect("enable patch succeeds");
        assert!(!doc.text.contains("review-pr"));
    }

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }

    fn temp_test_root(prefix: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!("{prefix}-{}", std::process::id()));
        let _ = std::fs::remove_dir_all(&root);
        root
    }
}
