use std::{
    collections::HashSet,
    path::{Path, PathBuf},
};

use crate::shared::{
    required_frontmatter_string, split_yaml_frontmatter, stable_path_id, validate_kebab_skill_name,
};

use crate::environment::{absolute_env_path, expand_local_path, normalize_path_lexically};
use skills_copilot_core::{
    AdapterContext, AdapterError, AdapterRoot, AgentAdapter, AgentConfigAdapter,
    AgentConfigDocument, AgentId, PermissionRequest, RootSource, Scope, SkillInstance, SkillState,
};

#[derive(Debug, Default)]
pub struct PiAdapter;

impl AgentAdapter for PiAdapter {
    fn id(&self) -> AgentId {
        AgentId::Pi
    }

    fn display_name(&self) -> &'static str {
        "Pi"
    }

    fn roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
        let agent_dir = pi_agent_dir(ctx);
        let mut roots = pi_package_skill_roots(ctx, &agent_dir);
        roots.extend(pi_configured_skill_roots(ctx, &agent_dir));

        if let Some(project_root) = &ctx.project_root {
            if pi_project_is_trusted(ctx, &agent_dir) {
                roots.extend(pi_project_skill_roots(
                    project_root,
                    ctx.project_cwd.as_deref(),
                ));
            }
        }

        roots.extend([
            AdapterRoot {
                scope: Scope::AgentGlobal,
                path: agent_dir.join("skills"),
                source: RootSource::UserHome,
            },
            AdapterRoot {
                scope: Scope::AgentGlobal,
                path: ctx.user_home.join(".agents/skills"),
                source: RootSource::Compatibility,
            },
        ]);

        dedup_roots(roots)
    }

    fn link_target_roots(&self, ctx: &AdapterContext) -> Vec<AdapterRoot> {
        self.roots(ctx)
            .into_iter()
            .flat_map(|root| pi_declared_skill_link_targets(&root))
            .collect()
    }

    fn parse(&self, path: &Path) -> Result<SkillInstance, AdapterError> {
        let content = std::fs::read_to_string(path)
            .map_err(|err| AdapterError::new(format!("failed to read skill: {err}")))?;
        self.parse_content(path, content)
    }

    fn parse_content(&self, path: &Path, content: String) -> Result<SkillInstance, AdapterError> {
        let fallback_name = fallback_skill_name(path);
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
            id: stable_path_id("pi", path),
            agent: AgentId::Pi,
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
        if relative_path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md") {
            return !relative_path.as_os_str().is_empty();
        }
        root.source != RootSource::Compatibility
            && relative_path.components().count() == 1
            && relative_path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    }

    fn is_skill_file(&self, path: &Path) -> bool {
        path.extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    }

    fn config_paths(&self, ctx: &AdapterContext) -> Vec<PathBuf> {
        let mut paths = vec![pi_agent_dir(ctx).join("settings.json")];
        if let Some(project_root) = &ctx.project_root {
            let cwd = ctx
                .project_cwd
                .as_deref()
                .filter(|cwd| cwd.starts_with(project_root))
                .unwrap_or(project_root);
            paths.push(cwd.join(".pi/settings.json"));
        }
        paths
    }
}

fn pi_declared_skill_link_targets(root: &AdapterRoot) -> Vec<AdapterRoot> {
    let mut targets = Vec::new();
    let mut stack = vec![root.path.clone()];
    let mut visited = 0usize;
    while let Some(directory) = stack.pop() {
        if visited >= 50_000 {
            break;
        }
        visited += 1;
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Ok(metadata) = std::fs::symlink_metadata(&path) else {
                continue;
            };
            if metadata.file_type().is_symlink() {
                if path.is_dir() || path.is_file() {
                    targets.push(AdapterRoot {
                        scope: root.scope,
                        path,
                        source: RootSource::Configured,
                    });
                }
            } else if metadata.is_dir()
                && entry.file_name() != "node_modules"
                && !entry.file_name().to_string_lossy().starts_with('.')
            {
                stack.push(path);
            }
        }
    }
    targets
}

pub fn pi_agent_dir(ctx: &AdapterContext) -> PathBuf {
    absolute_env_path("PI_CODING_AGENT_DIR").unwrap_or_else(|| ctx.user_home.join(".pi/agent"))
}

impl AgentConfigAdapter for PiAdapter {
    fn patch_enabled(
        &self,
        doc: &mut AgentConfigDocument,
        instance: &SkillInstance,
        on: bool,
    ) -> Result<(), AdapterError> {
        doc.text = patch_pi_config(&doc.text, &instance.path, on, instance.scope)?;
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
    let name = required_frontmatter_string(&frontmatter, "name", "Pi")?;
    validate_kebab_skill_name(&name, "Pi")?;
    let description = required_frontmatter_string(&frontmatter, "description", "Pi")?;

    Ok(ParsedSkill {
        frontmatter_raw: frontmatter_raw.to_string(),
        body,
        name,
        description,
    })
}

fn pi_project_skill_roots(project_root: &Path, project_cwd: Option<&Path>) -> Vec<AdapterRoot> {
    let mut roots = Vec::new();
    let cwd = project_cwd
        .filter(|cwd| cwd.starts_with(project_root))
        .unwrap_or(project_root);
    roots.push(AdapterRoot {
        scope: Scope::AgentProject,
        path: cwd.join(".pi/skills"),
        source: RootSource::Project,
    });
    for dir in pi_project_directories(project_root, Some(cwd)) {
        roots.push(AdapterRoot {
            scope: Scope::AgentProject,
            path: dir.join(".agents/skills"),
            source: RootSource::Compatibility,
        });
    }

    roots
}

fn pi_project_directories<'a>(
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

fn pi_settings_sources(ctx: &AdapterContext, agent_dir: &Path) -> Vec<(Scope, PathBuf)> {
    let mut sources = Vec::new();
    if let Some(project_root) = &ctx.project_root {
        if pi_project_is_trusted(ctx, agent_dir) {
            let cwd = ctx
                .project_cwd
                .as_deref()
                .filter(|cwd| cwd.starts_with(project_root))
                .unwrap_or(project_root);
            sources.push((Scope::AgentProject, cwd.join(".pi/settings.json")));
        }
    }
    sources.push((Scope::AgentGlobal, agent_dir.join("settings.json")));
    sources
}

fn pi_project_is_trusted(ctx: &AdapterContext, agent_dir: &Path) -> bool {
    let Some(project_root) = ctx.project_root.as_deref() else {
        return false;
    };
    let cwd = ctx
        .project_cwd
        .as_deref()
        .filter(|cwd| cwd.starts_with(project_root))
        .unwrap_or(project_root);
    let trust_path = agent_dir.join("trust.json");
    let Ok(content) = std::fs::read_to_string(trust_path) else {
        // No persisted decision means Pi will ask interactively. Keep the
        // inventory visible so a session-only acceptance can still be
        // reconciled; an explicit persisted denial is always enforced below.
        return true;
    };
    let Ok(entries) = serde_json::from_str::<serde_json::Map<String, serde_json::Value>>(&content)
    else {
        return true;
    };
    let mut current = cwd
        .canonicalize()
        .unwrap_or_else(|_| normalize_path_lexically(cwd));
    loop {
        if let Some(decision) = entries
            .get(current.to_string_lossy().as_ref())
            .and_then(serde_json::Value::as_bool)
        {
            return decision;
        }
        if !current.pop() {
            break;
        }
    }
    true
}

fn pi_configured_skill_roots(ctx: &AdapterContext, agent_dir: &Path) -> Vec<AdapterRoot> {
    pi_settings_sources(ctx, agent_dir)
        .into_iter()
        .flat_map(|(scope, config_path)| {
            let Ok(content) = std::fs::read_to_string(&config_path) else {
                return Vec::new();
            };
            let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
                return Vec::new();
            };
            let base = if scope == Scope::AgentProject {
                config_path.parent().unwrap_or(agent_dir)
            } else {
                agent_dir
            };
            value
                .get("skills")
                .and_then(serde_json::Value::as_array)
                .into_iter()
                .flatten()
                .filter_map(serde_json::Value::as_str)
                .filter(|raw| {
                    !matches!(raw.as_bytes().first(), Some(b'!') | Some(b'+') | Some(b'-'))
                })
                .filter_map(|raw| expand_local_path(raw, &ctx.user_home, base))
                .map(|path| AdapterRoot {
                    scope,
                    path,
                    source: RootSource::Configured,
                })
                .collect()
        })
        .collect()
}

fn pi_package_skill_roots(ctx: &AdapterContext, agent_dir: &Path) -> Vec<AdapterRoot> {
    let mut roots = Vec::new();
    for (scope, config_path) in pi_settings_sources(ctx, agent_dir) {
        let Ok(content) = std::fs::read_to_string(&config_path) else {
            continue;
        };
        let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
            continue;
        };
        let Some(packages) = value.get("packages").and_then(serde_json::Value::as_array) else {
            continue;
        };
        let config_base = config_path.parent().unwrap_or(agent_dir);
        for package in packages {
            let source = package
                .as_str()
                .or_else(|| package.get("source").and_then(serde_json::Value::as_str));
            if package
                .get("skills")
                .and_then(serde_json::Value::as_array)
                .is_some_and(Vec::is_empty)
            {
                continue;
            }
            let Some(package_root) = source.and_then(|source| {
                pi_installed_package_root(source, agent_dir, config_base, &ctx.user_home, scope)
            }) else {
                continue;
            };
            let manifest_path = package_root.join("package.json");
            let Ok(manifest_text) = std::fs::read_to_string(manifest_path) else {
                continue;
            };
            let Ok(manifest) = serde_json::from_str::<serde_json::Value>(&manifest_text) else {
                continue;
            };
            let declared_skill_paths = manifest
                .get("pi")
                .and_then(|pi| pi.get("skills"))
                .and_then(serde_json::Value::as_array)
                .cloned();
            let skill_paths = declared_skill_paths.unwrap_or_else(|| {
                let convention_path = package_root.join("skills");
                match std::fs::symlink_metadata(&convention_path) {
                    Err(error) if error.kind() == std::io::ErrorKind::NotFound => Vec::new(),
                    _ => vec![serde_json::Value::String("./skills".to_string())],
                }
            });
            roots.extend(
                skill_paths
                    .iter()
                    .filter_map(serde_json::Value::as_str)
                    .filter_map(|raw| expand_local_path(raw, &ctx.user_home, &package_root))
                    .map(|path| AdapterRoot {
                        scope,
                        path,
                        source: RootSource::Plugin,
                    }),
            );
        }
    }
    roots
}

fn pi_installed_package_root(
    source: &str,
    agent_dir: &Path,
    config_base: &Path,
    user_home: &Path,
    scope: Scope,
) -> Option<PathBuf> {
    let source = source.trim();
    if let Some(package) = source.strip_prefix("npm:") {
        if package.is_empty()
            || package
                .split('/')
                .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
        {
            return None;
        }
        let install_root = if scope == Scope::AgentProject {
            config_base
        } else {
            agent_dir
        };
        return Some(install_root.join("npm/node_modules").join(package));
    }
    if source.starts_with("git:") || source.starts_with("http:") || source.starts_with("https:") {
        return None;
    }
    expand_local_path(source, user_home, config_base)
}

fn dedup_roots(roots: Vec<AdapterRoot>) -> Vec<AdapterRoot> {
    let mut seen = HashSet::new();
    roots
        .into_iter()
        .filter(|root| seen.insert((root.scope, root.path.clone())))
        .collect()
}

fn fallback_skill_name(path: &Path) -> String {
    if path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md") {
        return path
            .parent()
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .unwrap_or("unknown")
            .to_string();
    }
    path.file_stem()
        .and_then(|name| name.to_str())
        .unwrap_or("unknown")
        .to_string()
}

fn patch_pi_config(
    content: &str,
    skill_path: &Path,
    enabled: bool,
    scope: Scope,
) -> Result<String, AdapterError> {
    let mut value = if content.trim().is_empty() {
        serde_json::json!({ "skills": [] })
    } else {
        serde_json::from_str(content)
            .map_err(|err| AdapterError::new(format!("invalid Pi settings JSON: {err}")))?
    };

    if scope == Scope::AgentProject && pi_project_explicitly_untrusted(&value) {
        return Err(AdapterError::new(
            "Pi project settings explicitly mark this project untrusted; project/package toggles are blocked",
        ));
    }

    let settings = pi_skill_paths_array_mut(&mut value)?;
    let normalized_skill = normalize_path_lexically(skill_path);
    settings.retain(|value| {
        let Some(raw) = value.as_str() else {
            return true;
        };
        let Some(target) = raw.strip_prefix(['!', '+', '-']) else {
            return true;
        };
        normalize_path_lexically(Path::new(target)) != normalized_skill
    });
    let prefix = if enabled { '+' } else { '-' };
    settings.push(serde_json::Value::String(format!(
        "{prefix}{}",
        portable_path_text(&normalized_skill)
    )));

    let mut text = serde_json::to_string_pretty(&value)
        .map_err(|err| AdapterError::new(format!("failed to serialize Pi settings: {err}")))?;
    text.push('\n');
    Ok(text)
}

fn portable_path_text(path: &Path) -> String {
    path.to_string_lossy().replace('\\', "/")
}

fn pi_project_explicitly_untrusted(value: &serde_json::Value) -> bool {
    value
        .get("project")
        .and_then(|project| project.get("trusted"))
        .and_then(serde_json::Value::as_bool)
        == Some(false)
        || value
            .get("trust")
            .and_then(|trust| trust.get("projectRootTrusted"))
            .and_then(serde_json::Value::as_bool)
            == Some(false)
}

fn pi_skill_paths_array_mut(
    value: &mut serde_json::Value,
) -> Result<&mut Vec<serde_json::Value>, AdapterError> {
    let object = value
        .as_object_mut()
        .ok_or_else(|| AdapterError::new("Pi settings must be a JSON object"))?;
    let skills = object
        .entry("skills")
        .or_insert_with(|| serde_json::json!([]));
    if skills.as_object().is_some_and(|legacy| {
        legacy.len() == 1
            && legacy
                .get("disabled")
                .and_then(serde_json::Value::as_array)
                .is_some()
    }) {
        // Migrate the exact non-Pi shape emitted by early Agent Copilot
        // builds. It never affected Pi runtime loading.
        *skills = serde_json::json!([]);
    }
    skills
        .as_array_mut()
        .ok_or_else(|| AdapterError::new("Pi skills must be an array"))
}

pub fn pi_skill_enabled_by_settings(
    content: &str,
    settings_path: &Path,
    skill_path: &Path,
    user_home: &Path,
) -> Result<bool, AdapterError> {
    let value: serde_json::Value = serde_json::from_str(content)
        .map_err(|err| AdapterError::new(format!("invalid Pi settings JSON: {err}")))?;
    // `disabledSkills` was emitted by early Agent Copilot builds but is not a
    // Pi setting. Deliberately ignore it so diagnostics reflect Pi's runtime.
    let patterns = value
        .get("skills")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    let base = settings_path.parent().unwrap_or(user_home);
    let matches = |raw: &str| {
        expand_local_path(raw, user_home, base).is_some_and(|target| {
            let target = target
                .canonicalize()
                .unwrap_or_else(|_| normalize_path_lexically(&target));
            let skill = skill_path
                .canonicalize()
                .unwrap_or_else(|_| normalize_path_lexically(skill_path));
            target == skill || skill.parent().is_some_and(|parent| target == parent)
        })
    };
    let excluded = patterns
        .iter()
        .filter_map(|raw| raw.strip_prefix('!'))
        .any(matches);
    let included = patterns
        .iter()
        .filter_map(|raw| raw.strip_prefix('+'))
        .any(matches);
    let force_excluded = patterns
        .iter()
        .filter_map(|raw| raw.strip_prefix('-'))
        .any(matches);
    Ok((!excluded || included) && !force_excluded)
}

#[cfg(test)]
mod tests {
    use skills_copilot_core::{AdapterRoot, RootSource};

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
    fn exposes_native_and_agent_compatibility_roots() {
        let adapter = PiAdapter;
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

        assert_eq!(
            roots[0].path,
            PathBuf::from("/tmp/project/nested/deeper/.pi/skills")
        );
        assert_eq!(roots[0].source, RootSource::Project);
        assert_eq!(
            roots[1].path,
            PathBuf::from("/tmp/project/nested/deeper/.agents/skills")
        );
        assert_eq!(roots[1].source, RootSource::Compatibility);
        assert_eq!(
            roots[2].path,
            PathBuf::from("/tmp/project/nested/.agents/skills")
        );
        assert_eq!(roots[3].path, PathBuf::from("/tmp/project/.agents/skills"));
        assert_eq!(roots[4].path, PathBuf::from("/tmp/home/.pi/agent/skills"));
        assert_eq!(roots[4].source, RootSource::UserHome);
        assert_eq!(roots[5].path, PathBuf::from("/tmp/home/.agents/skills"));
        assert_eq!(roots[5].source, RootSource::Compatibility);
        assert_eq!(roots.len(), 6);
    }

    #[test]
    fn package_convention_root_is_optional_but_explicit_manifest_root_is_not() {
        let fixture_root = std::env::temp_dir().join(format!(
            "skills-copilot-pi-package-roots-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system time after epoch")
                .as_nanos()
        ));
        let user_home = fixture_root.join("home");
        let agent_dir = user_home.join(".pi/agent");
        let package_root = agent_dir.join("npm/node_modules/example-extension");
        std::fs::create_dir_all(&package_root).expect("create package root");
        std::fs::write(
            agent_dir.join("settings.json"),
            r#"{"packages":["npm:example-extension"]}"#,
        )
        .expect("write Pi settings");
        std::fs::write(
            package_root.join("package.json"),
            r#"{"name":"example-extension","pi":{"extensions":["./index.js"]}}"#,
        )
        .expect("write package manifest");
        let ctx = AdapterContext {
            user_home,
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        };

        let missing_convention_roots = pi_package_skill_roots(&ctx, &agent_dir);
        assert!(
            missing_convention_roots.is_empty(),
            "a package without an existing conventional skills directory is not a scan source"
        );

        std::fs::create_dir(package_root.join("skills"))
            .expect("create conventional skills directory");
        let convention_roots = pi_package_skill_roots(&ctx, &agent_dir);
        assert_eq!(convention_roots.len(), 1);
        assert_eq!(convention_roots[0].path, package_root.join("skills"));

        std::fs::remove_dir(package_root.join("skills"))
            .expect("remove conventional skills directory");
        std::fs::write(
            package_root.join("package.json"),
            r#"{"name":"example-extension","pi":{"skills":["./declared-skills"]}}"#,
        )
        .expect("write explicit package skill root");
        let explicit_roots = pi_package_skill_roots(&ctx, &agent_dir);
        assert_eq!(explicit_roots.len(), 1);
        assert_eq!(explicit_roots[0].path, package_root.join("declared-skills"));

        std::fs::remove_dir_all(fixture_root).expect("remove Pi package root fixture");
    }

    #[test]
    fn parses_valid_directory_skill_frontmatter() {
        let adapter = PiAdapter;
        let fixture = fixture_path("fixtures/pi/global/agent/skills/global-pdf/SKILL.md");

        let skill = adapter.parse(&fixture).expect("skill parses");

        assert_eq!(skill.agent, AgentId::Pi);
        assert_eq!(skill.name, "global-pdf");
        assert_eq!(
            skill.description,
            "Extracts and reviews PDF text during Pi sessions. Use when a workflow needs PDF inspection steps."
        );
        assert_eq!(skill.state, SkillState::Loaded);
        assert!(skill.enabled);
    }

    #[test]
    fn path_and_content_parsing_are_equivalent() {
        let adapter = PiAdapter;
        let fixture = fixture_path("fixtures/pi/global/agent/skills/global-pdf/SKILL.md");

        crate::assert_parse_equivalent(&adapter, &fixture);
    }

    #[test]
    fn marks_missing_description_as_broken() {
        let adapter = PiAdapter;
        let fixture = fixture_path("fixtures/pi/broken/missing-description/SKILL.md");

        let skill = adapter.parse(&fixture).expect("broken skill is returned");

        assert_eq!(skill.name, "missing-description");
        assert_eq!(skill.state, SkillState::Broken);
        assert!(!skill.enabled);
        assert!(skill.description.contains("description"));
    }

    #[test]
    fn pi_settings_paths_use_portable_separators() {
        let text = patch_pi_config(
            "",
            Path::new(r"C:\fixture\pi-toggle\SKILL.md"),
            false,
            Scope::AgentGlobal,
        )
        .expect("Pi config patch succeeds");

        assert!(text.contains("C:/fixture/pi-toggle/SKILL.md"));
        assert!(!text.contains(r"C:\\fixture"));
    }

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }
}
