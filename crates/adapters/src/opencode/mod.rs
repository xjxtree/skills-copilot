use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::shared::{required_frontmatter_string, split_yaml_frontmatter, stable_path_id};
use skills_copilot_core::{
    AdapterContext, AdapterError, AdapterRoot, AgentAdapter, AgentConfigAdapter,
    AgentConfigDocument, AgentId, PermissionRequest, RootSource, Scope, SkillInstance, SkillState,
};

#[derive(Debug, Default)]
pub struct OpencodeAdapter;

impl AgentAdapter for OpencodeAdapter {
    fn id(&self) -> AgentId {
        AgentId::Opencode
    }

    fn display_name(&self) -> &'static str {
        "opencode"
    }

    fn roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
        let mut roots = vec![
            AdapterRoot {
                scope: Scope::AgentGlobal,
                path: ctx.user_home.join(".config/opencode/skills"),
                source: RootSource::UserHome,
            },
            AdapterRoot {
                scope: Scope::AgentGlobal,
                path: ctx.user_home.join(".claude/skills"),
                source: RootSource::UserHome,
            },
            AdapterRoot {
                scope: Scope::AgentGlobal,
                path: ctx.user_home.join(".agents/skills"),
                source: RootSource::UserHome,
            },
        ];

        if let Some(project_root) = &ctx.project_root {
            roots.extend(opencode_project_skill_roots(
                project_root,
                ctx.project_cwd.as_deref(),
            ));
        }

        roots.extend(opencode_configured_skill_roots(ctx));
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
        let (frontmatter_raw, body, name, description, state, enabled) = match parsed {
            Ok(parsed) => (
                parsed.frontmatter_raw,
                parsed.body,
                parsed.name,
                parsed.description,
                SkillState::Loaded,
                true,
            ),
            Err(message) => (
                String::new(),
                content,
                fallback_name,
                message,
                SkillState::Broken,
                false,
            ),
        };

        Ok(SkillInstance {
            id: stable_path_id("opencode", path),
            agent: AgentId::Opencode,
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

    fn config_paths(&self, ctx: &AdapterContext) -> Vec<PathBuf> {
        opencode_config_sources(ctx)
            .into_iter()
            .map(|source| source.path)
            .collect()
    }
}

impl AgentConfigAdapter for OpencodeAdapter {
    fn patch_enabled(
        &self,
        doc: &mut AgentConfigDocument,
        instance: &SkillInstance,
        on: bool,
    ) -> Result<(), AdapterError> {
        doc.text = patch_opencode_config(&doc.text, &instance.name, on)?;
        Ok(())
    }
}

struct ParsedSkill {
    frontmatter_raw: String,
    body: String,
    name: String,
    description: String,
}

fn parse_skill_content(content: &str, directory_name: &str) -> Result<ParsedSkill, String> {
    let rest = content
        .strip_prefix("---\n")
        .or_else(|| content.strip_prefix("---\r\n"))
        .ok_or_else(|| "missing YAML frontmatter".to_string())?;
    let (frontmatter_raw, body) = split_yaml_frontmatter(rest)?;
    let frontmatter: serde_norway::Value =
        serde_norway::from_str(frontmatter_raw).map_err(|err| err.to_string())?;
    let name = required_frontmatter_string(&frontmatter, "name", "opencode")?;
    validate_opencode_skill_name(&name)?;
    if !opencode_skill_name_matches_directory(&name, directory_name) {
        return Err(format!(
            "opencode skill name `{name}` must match containing directory `{directory_name}` or its colon-normalized form"
        ));
    }
    let description = required_frontmatter_string(&frontmatter, "description", "opencode")?;

    Ok(ParsedSkill {
        frontmatter_raw: frontmatter_raw.to_string(),
        body,
        name,
        description,
    })
}

fn opencode_project_skill_roots(
    project_root: &Path,
    project_cwd: Option<&Path>,
) -> Vec<AdapterRoot> {
    let start = project_cwd
        .filter(|cwd| cwd.starts_with(project_root))
        .unwrap_or(project_root);
    let mut roots = Vec::new();
    let mut current = Some(start);

    while let Some(dir) = current {
        roots.push(AdapterRoot {
            scope: Scope::AgentProject,
            path: dir.join(".opencode/skills"),
            source: RootSource::Project,
        });
        roots.push(AdapterRoot {
            scope: Scope::AgentProject,
            path: dir.join(".claude/skills"),
            source: RootSource::Project,
        });
        roots.push(AdapterRoot {
            scope: Scope::AgentProject,
            path: dir.join(".agents/skills"),
            source: RootSource::Project,
        });
        if dir == project_root {
            break;
        }
        current = dir
            .parent()
            .filter(|parent| parent.starts_with(project_root));
    }

    roots
}

#[derive(Debug)]
struct OpencodeConfigSource {
    path: PathBuf,
    scope: Scope,
}

fn opencode_config_sources(ctx: &AdapterContext) -> Vec<OpencodeConfigSource> {
    let mut sources = vec![
        OpencodeConfigSource {
            path: ctx.user_home.join(".config/opencode/opencode.json"),
            scope: Scope::AgentGlobal,
        },
        OpencodeConfigSource {
            path: ctx.user_home.join(".config/opencode/opencode.jsonc"),
            scope: Scope::AgentGlobal,
        },
    ];
    if let Some(project_root) = &ctx.project_root {
        sources.push(OpencodeConfigSource {
            path: project_root.join("opencode.json"),
            scope: Scope::AgentProject,
        });
        sources.push(OpencodeConfigSource {
            path: project_root.join("opencode.jsonc"),
            scope: Scope::AgentProject,
        });
    }
    sources
}

fn opencode_configured_skill_roots(ctx: &AdapterContext) -> Vec<AdapterRoot> {
    let relative_base = opencode_relative_path_base(ctx);
    opencode_config_sources(ctx)
        .into_iter()
        .flat_map(|source| {
            read_opencode_config_skill_paths(&source.path)
                .into_iter()
                .filter_map({
                    let relative_base = relative_base.clone();
                    move |raw_path| {
                        let expanded =
                            expand_opencode_skill_path(&raw_path, &ctx.user_home, &relative_base)?;
                        if !opencode_configured_path_allowed(ctx, source.scope, &expanded) {
                            return None;
                        }
                        Some(AdapterRoot {
                            scope: opencode_configured_scope(ctx, source.scope, &raw_path),
                            path: expanded,
                            source: RootSource::Configured,
                        })
                    }
                })
                .collect::<Vec<_>>()
        })
        .collect()
}

fn opencode_relative_path_base(ctx: &AdapterContext) -> PathBuf {
    ctx.project_cwd
        .as_ref()
        .filter(|cwd| {
            ctx.project_root
                .as_ref()
                .is_some_and(|project_root| cwd.starts_with(project_root))
        })
        .or(ctx.project_root.as_ref())
        .unwrap_or(&ctx.user_home)
        .to_path_buf()
}

fn opencode_configured_scope(ctx: &AdapterContext, config_scope: Scope, raw_path: &str) -> Scope {
    if config_scope == Scope::AgentGlobal
        && !opencode_path_is_home_or_absolute(raw_path)
        && ctx.project_root.is_some()
    {
        Scope::AgentProject
    } else {
        config_scope
    }
}

fn opencode_path_is_home_or_absolute(raw_path: &str) -> bool {
    raw_path == "~" || raw_path.starts_with("~/") || Path::new(raw_path).is_absolute()
}

fn opencode_configured_path_allowed(
    ctx: &AdapterContext,
    config_scope: Scope,
    expanded: &Path,
) -> bool {
    if config_scope != Scope::AgentProject {
        return true;
    }
    let Some(project_root) = &ctx.project_root else {
        return false;
    };
    if let Ok(canonical_path) = expanded.canonicalize() {
        return project_root
            .canonicalize()
            .is_ok_and(|canonical_project| canonical_path.starts_with(canonical_project));
    }
    expanded.starts_with(project_root)
}

fn expand_opencode_skill_path(
    raw_path: &str,
    user_home: &Path,
    relative_base: &Path,
) -> Option<PathBuf> {
    let trimmed = raw_path.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed == "~" {
        return Some(user_home.to_path_buf());
    }
    if let Some(rest) = trimmed.strip_prefix("~/") {
        return Some(user_home.join(rest));
    }
    let path = PathBuf::from(trimmed);
    if path.is_absolute() {
        Some(path)
    } else {
        Some(relative_base.join(path))
    }
}

fn read_opencode_config_skill_paths(config_path: &Path) -> Vec<String> {
    let Ok(content) = std::fs::read_to_string(config_path) else {
        return Vec::new();
    };
    let normalized = normalize_opencode_jsonc_for_read(&content);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&normalized) else {
        return Vec::new();
    };
    value
        .get("skills")
        .and_then(|skills| skills.get("paths"))
        .and_then(serde_json::Value::as_array)
        .map(|paths| {
            paths
                .iter()
                .filter_map(serde_json::Value::as_str)
                .map(str::trim)
                .filter(|path| !path.is_empty())
                .map(ToString::to_string)
                .collect()
        })
        .unwrap_or_default()
}

fn normalize_opencode_jsonc_for_read(content: &str) -> String {
    strip_json_trailing_commas(&strip_json_comments(content))
}

fn strip_json_comments(content: &str) -> String {
    let mut output = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            output.push(ch);
            continue;
        }

        if ch == '/' {
            match chars.peek().copied() {
                Some('/') => {
                    chars.next();
                    for next in chars.by_ref() {
                        if next == '\n' {
                            output.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    let mut previous = '\0';
                    for next in chars.by_ref() {
                        if previous == '*' && next == '/' {
                            break;
                        }
                        previous = next;
                    }
                    continue;
                }
                _ => {}
            }
        }

        output.push(ch);
    }

    output
}

fn strip_json_trailing_commas(content: &str) -> String {
    let chars = content.chars().collect::<Vec<_>>();
    let mut output = String::with_capacity(content.len());
    let mut in_string = false;
    let mut escaped = false;
    for (index, ch) in chars.iter().copied().enumerate() {
        if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            output.push(ch);
            continue;
        }

        if ch == ',' {
            let next_non_ws = chars[index + 1..].iter().find(|next| !next.is_whitespace());
            if matches!(next_non_ws, Some('}' | ']')) {
                continue;
            }
        }

        output.push(ch);
    }
    output
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

fn containing_dir_name(path: &Path) -> String {
    path.parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn validate_opencode_skill_name(name: &str) -> Result<(), String> {
    if name.is_empty() || name.len() > 64 {
        return Err(format!(
            "invalid opencode skill name `{name}`: expected 1-64 characters"
        ));
    }

    if name.starts_with('-')
        || name.ends_with('-')
        || name.starts_with(':')
        || name.ends_with(':')
        || name.contains("--")
        || name.contains("::")
    {
        return Err(format!(
            "invalid opencode skill name `{name}`: use non-empty lowercase segments separated by single colons; segment characters may be alphanumeric or hyphen"
        ));
    }

    if !name.bytes().all(|byte| {
        byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'-' || byte == b':'
    }) {
        return Err(format!(
            "invalid opencode skill name `{name}`: use lowercase alphanumeric characters, hyphens, and optional colon namespaces"
        ));
    }

    for segment in name.split(':') {
        if segment.is_empty()
            || segment.starts_with('-')
            || segment.ends_with('-')
            || segment.contains("--")
        {
            return Err(format!(
                "invalid opencode skill name `{name}`: use non-empty lowercase segments separated by single colons; segment characters may be alphanumeric or hyphen"
            ));
        }
    }

    Ok(())
}

fn opencode_skill_name_matches_directory(name: &str, directory_name: &str) -> bool {
    name == directory_name || name.replace(':', "-") == directory_name
}

fn patch_opencode_config(
    content: &str,
    skill_name: &str,
    on: bool,
) -> Result<String, AdapterError> {
    validate_opencode_skill_name(skill_name).map_err(AdapterError::new)?;
    let mut value = parse_opencode_config(content)?;
    let root = object_mut(&mut value, "opencode config root")?;
    let permission = root
        .entry("permission".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    if let Some(existing) = permission.as_str().map(str::to_string) {
        *permission = serde_json::json!({ "*": existing });
    }
    let permission = object_mut(permission, "`permission` config")?;
    let skill = permission
        .entry("skill".to_string())
        .or_insert_with(|| serde_json::Value::Object(serde_json::Map::new()));

    if let Some(existing) = skill.as_str().map(str::to_string) {
        *skill = serde_json::json!({ "*": existing });
    }
    let skill = object_mut(skill, "`permission.skill` config")?;

    if on {
        if skill
            .get(skill_name)
            .and_then(serde_json::Value::as_str)
            .is_some_and(|rule| rule == "deny")
        {
            skill.remove(skill_name);
        }
    } else {
        skill.insert(
            skill_name.to_string(),
            serde_json::Value::String("deny".to_string()),
        );
    }

    Ok(format!(
        "{}\n",
        serde_json::to_string_pretty(&value).map_err(|err| {
            AdapterError::new(format!("failed to serialize opencode config: {err}"))
        })?
    ))
}

fn parse_opencode_config(content: &str) -> Result<serde_json::Value, AdapterError> {
    let trimmed = content.trim();
    if trimmed.is_empty() {
        return Ok(serde_json::Value::Object(serde_json::Map::new()));
    }
    serde_json::from_str(trimmed).map_err(|err| {
        AdapterError::new(format!(
            "failed to parse opencode JSON config for writable patch: {err}; JSONC/commented configs are not rewritten because comments cannot be preserved"
        ))
    })
}

fn object_mut<'a>(
    value: &'a mut serde_json::Value,
    label: &str,
) -> Result<&'a mut serde_json::Map<String, serde_json::Value>, AdapterError> {
    value
        .as_object_mut()
        .ok_or_else(|| AdapterError::new(format!("{label} must be a JSON object")))
}

#[cfg(test)]
mod tests {
    use skills_copilot_core::{AdapterRoot, RootSource, SkillState};

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

        let parsed = parse_skill_content(&format!("---\n{raw}---\nBody.\n"), "sample-skill")
            .expect("adapter frontmatter parses");
        assert_eq!(parsed.name, "sample-skill");
        assert_eq!(parsed.description, "Sample");
    }

    #[test]
    fn yaml_contract_malformed_frontmatter_returns_error() {
        let result = parse_skill_content(
            "---\nname: [unterminated\ndescription: Sample\n---\nBody.\n",
            "sample-skill",
        );

        assert!(result.is_err());
    }

    #[test]
    fn exposes_documented_native_and_compatibility_roots() {
        let adapter = OpencodeAdapter;
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

        assert_eq!(roots.len(), 12);
        assert_eq!(
            roots[0].path,
            PathBuf::from("/tmp/home/.config/opencode/skills")
        );
        assert_eq!(roots[1].path, PathBuf::from("/tmp/home/.claude/skills"));
        assert_eq!(roots[2].path, PathBuf::from("/tmp/home/.agents/skills"));
        assert_eq!(roots[0].scope, Scope::AgentGlobal);
        assert_eq!(roots[0].source, RootSource::UserHome);
        assert_eq!(
            roots[3].path,
            PathBuf::from("/tmp/project/nested/deeper/.opencode/skills")
        );
        assert_eq!(
            roots[4].path,
            PathBuf::from("/tmp/project/nested/deeper/.claude/skills")
        );
        assert_eq!(
            roots[5].path,
            PathBuf::from("/tmp/project/nested/deeper/.agents/skills")
        );
        assert_eq!(
            roots[6].path,
            PathBuf::from("/tmp/project/nested/.opencode/skills")
        );
        assert_eq!(
            roots[7].path,
            PathBuf::from("/tmp/project/nested/.claude/skills")
        );
        assert_eq!(
            roots[8].path,
            PathBuf::from("/tmp/project/nested/.agents/skills")
        );
        assert_eq!(
            roots[9].path,
            PathBuf::from("/tmp/project/.opencode/skills")
        );
        assert_eq!(roots[10].path, PathBuf::from("/tmp/project/.claude/skills"));
        assert_eq!(roots[11].path, PathBuf::from("/tmp/project/.agents/skills"));
        for root in &roots[3..] {
            assert_eq!(root.scope, Scope::AgentProject);
            assert_eq!(root.source, RootSource::Project);
        }
        assert!(
            roots
                .iter()
                .all(|root| !root.path.starts_with("/tmp/unverified")),
            "opencode must not scan unverified extra roots"
        );
    }

    #[test]
    #[cfg(unix)]
    fn keeps_project_compatibility_root_that_authorizes_native_symlink() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-opencode-project-link-roots-{}",
            std::process::id()
        ));
        let project = temp_root.join("project");
        std::fs::create_dir_all(project.join(".agents/skills"))
            .expect("create project compatibility root");
        std::fs::create_dir_all(project.join(".claude")).expect("create Claude project dir");
        std::os::unix::fs::symlink("../.agents/skills", project.join(".claude/skills"))
            .expect("link Claude project skills");
        let adapter = OpencodeAdapter;
        let ctx = AdapterContext {
            user_home: temp_root.join("home"),
            project_root: Some(project.clone()),
            project_cwd: Some(project.clone()),
            extra_roots: vec![],
        };

        let roots = adapter.roots(&ctx);

        assert!(roots.iter().any(|root| {
            root.source == RootSource::Project && root.path == project.join(".claude/skills")
        }));
        assert!(roots.iter().any(|root| {
            root.source == RootSource::Project && root.path == project.join(".agents/skills")
        }));

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn exposes_configured_skills_paths_as_read_only_roots() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-opencode-configured-roots-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let project = temp_root.join("project");
        let configured_global = home.join("custom-opencode-skills");
        let configured_project = project.join("custom/project-skills");
        let outside_project = temp_root.join("outside-project");
        std::fs::create_dir_all(home.join(".config/opencode")).expect("create config dir");
        std::fs::create_dir_all(&configured_global).expect("create global configured root");
        std::fs::create_dir_all(&configured_project).expect("create project configured root");
        std::fs::create_dir_all(&outside_project).expect("create outside configured root");
        std::fs::write(
            home.join(".config/opencode/opencode.jsonc"),
            r#"{
              // OpenCode accepts JSONC for read-only discovery.
              "skills": {
                "paths": [
                  "~/custom-opencode-skills",
                  "~/custom-opencode-skills",
                ],
                "urls": ["https://example.invalid/.well-known/skills/"]
              },
            }"#,
        )
        .expect("write global opencode config");
        let project_config = serde_json::json!({
            "skills": {
                "paths": [
                    "custom/project-skills",
                    outside_project.to_string_lossy().to_string(),
                ],
            },
        });
        std::fs::write(project.join("opencode.json"), project_config.to_string())
            .expect("write project opencode config");
        let adapter = OpencodeAdapter;
        let ctx = AdapterContext {
            user_home: home.clone(),
            project_root: Some(project.clone()),
            project_cwd: Some(project.clone()),
            extra_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: temp_root.join("unverified-extra"),
                source: RootSource::Extra,
            }],
        };

        let roots = adapter.roots(&ctx);

        assert_eq!(
            roots
                .iter()
                .filter(|root| root.source == RootSource::Configured)
                .count(),
            2,
            "configured paths are deduped and project config cannot add an outside-project root"
        );
        assert!(roots.iter().any(|root| {
            root.scope == Scope::AgentGlobal
                && root.source == RootSource::Configured
                && root.path == configured_global
        }));
        assert!(roots.iter().any(|root| {
            root.scope == Scope::AgentProject
                && root.source == RootSource::Configured
                && root.path == configured_project
        }));
        assert!(
            roots.iter().all(|root| {
                !root.path.starts_with(&outside_project)
                    && !root
                        .path
                        .to_string_lossy()
                        .contains("https://example.invalid")
            }),
            "skills.urls must not become filesystem roots"
        );
        assert!(
            roots
                .iter()
                .all(|root| !root.path.starts_with(temp_root.join("unverified-extra"))),
            "ctx.extra_roots must remain ignored by opencode"
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn parses_valid_skill_frontmatter() {
        let adapter = OpencodeAdapter;
        let fixture = fixture_path(
            "fixtures/opencode/user-home/.config/opencode/skills/global-review/SKILL.md",
        );

        let skill = adapter.parse(&fixture).expect("skill parses");

        assert_eq!(skill.agent, AgentId::Opencode);
        assert_eq!(skill.name, "global-review");
        assert_eq!(
            skill.description,
            "Reviews repository changes for maintainability and risk. Use when preparing an opencode review workflow."
        );
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.enabled);
        assert!(skill.permissions.tools.is_empty());
        assert!(skill.frontmatter_raw.contains("name: global-review"));
        assert!(skill.body.contains("# Global Review"));
    }

    #[test]
    fn path_and_content_parsing_are_equivalent() {
        let adapter = OpencodeAdapter;
        let fixture = fixture_path(
            "fixtures/opencode/user-home/.config/opencode/skills/global-review/SKILL.md",
        );

        crate::assert_parse_equivalent(&adapter, &fixture);
    }

    #[test]
    fn accepts_runtime_colon_skill_names_from_compat_roots() {
        let adapter = OpencodeAdapter;
        let fixture = write_skill(
            "ce-compound",
            "---\nname: ce:compound\ndescription: Document recent work.\n---\nBody.\n",
        );

        let skill = adapter.parse(&fixture).expect("skill parses");

        assert_eq!(skill.name, "ce:compound");
        assert_eq!(skill.definition_id, "ce:compound");
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.enabled);
    }

    #[test]
    fn marks_name_mismatch_as_broken() {
        let adapter = OpencodeAdapter;
        let fixture = fixture_path("fixtures/opencode/broken/name-mismatch/SKILL.md");

        let skill = adapter.parse(&fixture).expect("broken skill is returned");

        assert_eq!(skill.name, "name-mismatch");
        assert_eq!(skill.state, SkillState::Broken);
        assert!(!skill.enabled);
        assert!(skill
            .description
            .contains("must match containing directory"));
    }

    #[test]
    fn marks_missing_required_fields_as_broken() {
        let adapter = OpencodeAdapter;
        let fixture = write_skill(
            "missing-description",
            "---\nname: missing-description\n---\nBody.\n",
        );

        let skill = adapter.parse(&fixture).expect("broken skill is returned");

        assert_eq!(skill.name, "missing-description");
        assert_eq!(skill.state, SkillState::Broken);
        assert!(!skill.enabled);
        assert!(skill.description.contains("description"));
    }

    #[test]
    fn marks_invalid_name_as_broken() {
        let adapter = OpencodeAdapter;
        let fixture = write_skill(
            "bad--name",
            "---\nname: bad--name\ndescription: Invalid name fixture.\n---\nBody.\n",
        );

        let skill = adapter.parse(&fixture).expect("broken skill is returned");

        assert_eq!(skill.name, "bad--name");
        assert_eq!(skill.state, SkillState::Broken);
        assert!(!skill.enabled);
        assert!(skill.description.contains("non-empty lowercase segments"));
    }

    #[test]
    fn patch_enabled_adds_exact_skill_deny_and_preserves_wildcard() {
        let mut doc = AgentConfigDocument {
            path: PathBuf::from("/tmp/opencode.json"),
            format: skills_copilot_core::ConfigFormat::Json,
            text: r#"{"permission":{"skill":{"*":"allow","internal-*":"deny"}}}"#.to_string(),
        };
        let skill = minimal_skill("global-review");

        OpencodeAdapter
            .patch_enabled(&mut doc, &skill, false)
            .expect("disable patch");

        let value: serde_json::Value = serde_json::from_str(&doc.text).expect("patched json");
        assert_eq!(value["permission"]["skill"]["*"], "allow");
        assert_eq!(value["permission"]["skill"]["internal-*"], "deny");
        assert_eq!(value["permission"]["skill"]["global-review"], "deny");
    }

    #[test]
    fn patch_enabled_adds_exact_colon_skill_deny() {
        let mut doc = AgentConfigDocument {
            path: PathBuf::from("/tmp/opencode.json"),
            format: skills_copilot_core::ConfigFormat::Json,
            text: r#"{"permission":{"skill":{"*":"allow"}}}"#.to_string(),
        };
        let skill = minimal_skill("ce:compound");

        OpencodeAdapter
            .patch_enabled(&mut doc, &skill, false)
            .expect("disable patch");

        let value: serde_json::Value = serde_json::from_str(&doc.text).expect("patched json");
        assert_eq!(value["permission"]["skill"]["*"], "allow");
        assert_eq!(value["permission"]["skill"]["ce:compound"], "deny");
    }

    #[test]
    fn patch_enabled_removes_only_exact_deny() {
        let mut doc = AgentConfigDocument {
            path: PathBuf::from("/tmp/opencode.json"),
            format: skills_copilot_core::ConfigFormat::Json,
            text: r#"{"permission":{"skill":{"*":"ask","global-review":"deny","other":"ask"}}}"#
                .to_string(),
        };
        let skill = minimal_skill("global-review");

        OpencodeAdapter
            .patch_enabled(&mut doc, &skill, true)
            .expect("enable patch");

        let value: serde_json::Value = serde_json::from_str(&doc.text).expect("patched json");
        assert!(value["permission"]["skill"].get("global-review").is_none());
        assert_eq!(value["permission"]["skill"]["*"], "ask");
        assert_eq!(value["permission"]["skill"]["other"], "ask");
    }

    #[test]
    fn patch_enabled_accepts_string_permission_defaults() {
        let mut doc = AgentConfigDocument {
            path: PathBuf::from("/tmp/opencode.json"),
            format: skills_copilot_core::ConfigFormat::Json,
            text: r#"{"permission":"ask"}"#.to_string(),
        };
        let skill = minimal_skill("global-review");

        OpencodeAdapter
            .patch_enabled(&mut doc, &skill, false)
            .expect("disable patch");

        let value: serde_json::Value = serde_json::from_str(&doc.text).expect("patched json");
        assert_eq!(value["permission"]["*"], "ask");
        assert_eq!(value["permission"]["skill"]["global-review"], "deny");
    }

    #[test]
    fn patch_enabled_rejects_commented_jsonc_without_rewriting() {
        let original = r#"{
                // OpenCode accepts JSONC, but the managed write should not drop comments.
                "permission": "ask"
            }"#;
        let mut doc = AgentConfigDocument {
            path: PathBuf::from("/tmp/opencode.json"),
            format: skills_copilot_core::ConfigFormat::Json,
            text: original.to_string(),
        };
        let skill = minimal_skill("global-review");

        let err = OpencodeAdapter
            .patch_enabled(&mut doc, &skill, false)
            .expect_err("commented JSONC should not be rewritten");

        assert!(err
            .message
            .contains("JSONC/commented configs are not rewritten"));
        assert_eq!(doc.text, original);
    }

    fn write_skill(name: &str, content: &str) -> PathBuf {
        let root = std::env::temp_dir().join(format!(
            "skills-copilot-opencode-adapter-{}-{}",
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

    fn minimal_skill(name: &str) -> SkillInstance {
        SkillInstance {
            id: name.to_string(),
            agent: AgentId::Opencode,
            scope: Scope::AgentGlobal,
            project_root: None,
            path: PathBuf::from(format!("/tmp/{name}/SKILL.md")),
            display_path: PathBuf::from(format!("/tmp/{name}/SKILL.md")),
            definition_id: name.to_string(),
            name: name.to_string(),
            display_name: name.to_string(),
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
        }
    }
}
