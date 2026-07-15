use std::path::{Path, PathBuf};

use crate::shared::{optional_frontmatter_string, split_yaml_frontmatter, stable_path_id};
use skills_copilot_core::{
    AdapterContext, AdapterError, AdapterRoot, AgentAdapter, AgentConfigAdapter,
    AgentConfigDocument, AgentId, PermissionRequest, RootSource, Scope, SkillInstance, SkillState,
};

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
        let mut roots = vec![
            AdapterRoot {
                scope: Scope::AgentGlobal,
                path: ctx.user_home.join(".openclaw/skills"),
                source: RootSource::UserHome,
            },
            AdapterRoot {
                scope: Scope::AgentGlobal,
                path: ctx.user_home.join(".agents/skills"),
                source: RootSource::UserHome,
            },
        ];

        roots.extend(openclaw_bundled_skill_roots());

        if let Some(workspace_root) = openclaw_selected_workspace_root(ctx) {
            roots.push(AdapterRoot {
                scope: Scope::AgentProject,
                path: workspace_root.join("skills"),
                source: RootSource::Project,
            });
            roots.push(AdapterRoot {
                scope: Scope::AgentProject,
                path: workspace_root.join(".agents/skills"),
                source: RootSource::Project,
            });
        }

        roots
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

fn openclaw_config_path(ctx: &AdapterContext) -> PathBuf {
    ctx.user_home.join(".openclaw/openclaw.json")
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

fn openclaw_selected_workspace_root(ctx: &AdapterContext) -> Option<PathBuf> {
    let selected_paths = [ctx.project_root.as_ref(), ctx.project_cwd.as_ref()]
        .into_iter()
        .flatten()
        .flat_map(|selected| normalized_path_variants(selected))
        .collect::<Vec<_>>();
    openclaw_home_workspace_candidates(ctx)
        .into_iter()
        .find(|candidate| {
            let candidate_paths = normalized_path_variants(candidate);
            selected_paths.iter().any(|selected| {
                candidate_paths
                    .iter()
                    .any(|candidate| selected == candidate || selected.starts_with(candidate))
            })
        })
}

fn openclaw_home_workspace_candidates(ctx: &AdapterContext) -> [PathBuf; 2] {
    [
        ctx.user_home.join(".openclaw/workspace"),
        ctx.user_home.join("openclaw/workspace"),
    ]
}

fn openclaw_bundled_skill_roots() -> Vec<AdapterRoot> {
    [
        "/usr/lib/node_modules/openclaw/skills",
        "/usr/local/lib/node_modules/openclaw/skills",
        "/opt/jvs-claw/base/lib/node_modules/openclaw/skills",
    ]
    .into_iter()
    .map(|path| AdapterRoot {
        scope: Scope::AgentGlobal,
        path: PathBuf::from(path),
        source: RootSource::System,
    })
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

fn normalize_path_lexically(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
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
            }],
        };

        let roots = adapter.roots(&ctx);

        assert_eq!(roots[0].path, PathBuf::from("/tmp/home/.openclaw/skills"));
        assert_eq!(roots[0].scope, Scope::AgentGlobal);
        assert_eq!(roots[0].source, RootSource::UserHome);
        assert_eq!(roots[1].path, PathBuf::from("/tmp/home/.agents/skills"));
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
                && root.source == RootSource::Project
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
                && root.source == RootSource::Project
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
