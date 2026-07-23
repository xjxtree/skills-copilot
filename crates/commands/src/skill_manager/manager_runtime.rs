use super::*;

pub(super) const DEFAULT_MANAGER_TOOL: &str = "npx-skills";
pub(super) const SKILLS_NPM_TOOL: &str = "skills-npm";
pub(super) const SKILLS_CLI_VERSION: &str = "1.5.20";
pub(super) const SKILLS_CLI_BINARY: &str = "skills@1.5.20";
pub(super) const NPX_BINARY: &str = "npx";
pub const SUPPORTED_MANAGER_AGENTS: [&str; 6] = [
    "claude-code",
    "pi",
    "opencode",
    "codex",
    "hermes-agent",
    "openclaw",
];

pub(super) fn manager_env(ctx: &AdapterContext) -> Vec<SkillManagerEnvPreview> {
    let npm_cache = manager_isolated_npm_cache(ctx);
    vec![
        env_preview("HOME", &ctx.user_home.to_string_lossy()),
        env_preview("LANG", "en_US.UTF-8"),
        env_preview("LC_ALL", "en_US.UTF-8"),
        env_preview("DISABLE_TELEMETRY", "1"),
        env_preview("DO_NOT_TRACK", "1"),
        env_preview("CI", "1"),
        env_preview("npm_config_audit", "false"),
        env_preview("npm_config_fund", "false"),
        env_preview("npm_config_update_notifier", "false"),
        env_preview("npm_config_ignore_scripts", "true"),
        env_preview("npm_config_registry", "https://registry.npmjs.org/"),
        env_preview("npm_config_userconfig", "/dev/null"),
        env_preview(
            "npm_config_globalconfig",
            "/.agent-copilot-no-global-npmrc",
        ),
        env_preview("npm_config_cache", &npm_cache.to_string_lossy()),
        env_preview("GIT_CONFIG_NOSYSTEM", "1"),
        env_preview("GIT_CONFIG_GLOBAL", "/dev/null"),
        env_preview("GIT_TERMINAL_PROMPT", "0"),
        env_preview("GCM_INTERACTIVE", "never"),
        env_preview("GIT_ASKPASS", "/usr/bin/false"),
        env_preview("SSH_ASKPASS", "/usr/bin/false"),
        env_preview(
            "GIT_SSH_COMMAND",
            "/usr/bin/ssh -F /dev/null -oBatchMode=yes -oIdentityAgent=none -oIdentitiesOnly=yes -oIdentityFile=/dev/null -oStrictHostKeyChecking=yes",
        ),
        env_preview("XDG_CONFIG_HOME", "/.agent-copilot-no-user-config"),
    ]
}

pub(super) fn manager_isolated_npm_cache(ctx: &AdapterContext) -> PathBuf {
    if cfg!(target_os = "macos") {
        ctx.user_home
            .join("Library/Application Support/dev.agent-copilot.native/external-manager/npm-cache")
    } else {
        ctx.user_home
            .join(".skills-copilot/dev.agent-copilot.native/external-manager/npm-cache")
    }
}

pub(super) fn manager_command_env(
    ctx: &AdapterContext,
    executable: &str,
) -> Vec<SkillManagerEnvPreview> {
    let mut env_vars = manager_env(ctx);
    env_vars.push(env_preview(
        "PATH",
        &manager_command_path(ctx, Path::new(executable)),
    ));
    env_vars
}

pub(super) fn manager_command_path(ctx: &AdapterContext, executable: &Path) -> String {
    let path_var = env::var_os("PATH");
    let fallback_dirs = fallback_binary_search_dirs_for_home(Some(&ctx.user_home));
    manager_command_path_from_sources(Some(executable), path_var.as_deref(), &fallback_dirs)
}

pub(super) fn manager_command_path_from_sources(
    executable: Option<&Path>,
    path_var: Option<&std::ffi::OsStr>,
    fallback_dirs: &[PathBuf],
) -> String {
    let mut dirs = Vec::new();
    let mut seen = BTreeSet::new();

    if let Some(parent) = executable
        .and_then(Path::parent)
        .filter(|path| !path.as_os_str().is_empty())
    {
        push_path_dir(&mut dirs, &mut seen, parent.to_path_buf());
    }
    for dir in path_var.into_iter().flat_map(env::split_paths) {
        push_path_dir(&mut dirs, &mut seen, dir);
    }
    for dir in fallback_dirs {
        push_path_dir(&mut dirs, &mut seen, dir.clone());
    }

    env::join_paths(&dirs)
        .ok()
        .filter(|path| !path.as_os_str().is_empty())
        .unwrap_or_else(|| std::ffi::OsString::from("/usr/bin:/bin"))
        .to_string_lossy()
        .to_string()
}

pub(super) fn push_path_dir(dirs: &mut Vec<PathBuf>, seen: &mut BTreeSet<PathBuf>, dir: PathBuf) {
    if dir.as_os_str().is_empty() {
        return;
    }
    if seen.insert(dir.clone()) {
        dirs.push(dir);
    }
}

pub(super) fn env_preview(key: &str, value: &str) -> SkillManagerEnvPreview {
    SkillManagerEnvPreview {
        key: key.to_string(),
        value: value.to_string(),
    }
}

pub(super) fn npx_executable() -> Result<PathBuf, CommandError> {
    let path =
        resolve_binary(env::var_os("SKILLS_COPILOT_NPX_PATH"), NPX_BINARY).ok_or_else(|| {
            CommandError::SkillManagerUnavailable(
                "npx executable was not found; install Node/npm or set SKILLS_COPILOT_NPX_PATH"
                    .to_string(),
            )
        })?;
    let canonical = path.canonicalize().map_err(|_| {
        CommandError::SkillManagerUnavailable(format!(
            "npx executable could not be resolved safely: {}",
            path.display()
        ))
    })?;
    let metadata = fs::metadata(&canonical)?;
    if !metadata.is_file() {
        return Err(CommandError::SkillManagerUnavailable(
            "npx executable must resolve to a regular file".to_string(),
        ));
    }
    #[cfg(unix)]
    {
        use std::os::unix::fs::PermissionsExt;
        if metadata.permissions().mode() & 0o111 == 0 {
            return Err(CommandError::SkillManagerUnavailable(
                "npx executable is not executable".to_string(),
            ));
        }
    }
    Ok(canonical)
}

pub(super) fn resolve_binary(
    override_path: Option<std::ffi::OsString>,
    binary_name: &str,
) -> Option<PathBuf> {
    let path_var = env::var_os("PATH");
    let fallback_dirs = fallback_binary_search_dirs();
    resolve_binary_from_sources(
        override_path,
        binary_name,
        path_var.as_deref(),
        &fallback_dirs,
    )
}

pub(super) fn resolve_binary_from_sources(
    override_path: Option<std::ffi::OsString>,
    binary_name: &str,
    path_var: Option<&std::ffi::OsStr>,
    fallback_dirs: &[PathBuf],
) -> Option<PathBuf> {
    if let Some(path) = override_path
        .map(PathBuf::from)
        .filter(|path| !path.as_os_str().is_empty())
    {
        return Some(path);
    }

    path_var
        .into_iter()
        .flat_map(env::split_paths)
        .chain(fallback_dirs.iter().cloned())
        .map(|dir| dir.join(binary_name))
        .find(|candidate| candidate.is_file())
}

pub(super) fn fallback_binary_search_dirs() -> Vec<PathBuf> {
    let home = env::var_os("HOME").map(PathBuf::from);
    fallback_binary_search_dirs_for_home(home.as_deref())
}

pub(super) fn fallback_binary_search_dirs_for_home(home: Option<&Path>) -> Vec<PathBuf> {
    let mut dirs = vec![
        PathBuf::from("/opt/homebrew/bin"),
        PathBuf::from("/opt/homebrew/sbin"),
        PathBuf::from("/usr/local/bin"),
        PathBuf::from("/usr/local/sbin"),
        PathBuf::from("/usr/bin"),
        PathBuf::from("/bin"),
        PathBuf::from("/usr/sbin"),
        PathBuf::from("/sbin"),
    ];

    if let Some(home) = home.filter(|path| !path.as_os_str().is_empty()) {
        dirs.extend([
            home.join(".volta/bin"),
            home.join(".asdf/shims"),
            home.join(".local/bin"),
            home.join(".npm-global/bin"),
            home.join(".bun/bin"),
        ]);
        dirs.extend(nvm_node_bin_dirs(home));
    }

    dirs
}

pub(super) fn nvm_node_bin_dirs(home: &Path) -> Vec<PathBuf> {
    let versions_dir = home.join(".nvm/versions/node");
    let mut dirs = fs::read_dir(versions_dir)
        .ok()
        .into_iter()
        .flat_map(|entries| entries.filter_map(Result::ok))
        .map(|entry| entry.path().join("bin"))
        .filter(|path| path.is_dir())
        .collect::<Vec<_>>();
    dirs.sort();
    dirs
}

pub(super) fn default_agent_targets() -> Vec<String> {
    SUPPORTED_MANAGER_AGENTS
        .into_iter()
        .map(ToOwned::to_owned)
        .collect()
}

pub(super) fn normalize_manager_agents(agents: &[String]) -> Result<Vec<String>, CommandError> {
    let source = if agents.is_empty() {
        default_agent_targets()
    } else {
        agents
            .iter()
            .map(|agent| manager_agent_alias(agent))
            .collect::<Result<Vec<_>, _>>()?
    };
    let mut seen = BTreeSet::new();
    Ok(source
        .into_iter()
        .filter(|agent| seen.insert(agent.clone()))
        .collect())
}

pub(super) fn required_manager_agents(agents: &[String]) -> Result<Vec<String>, CommandError> {
    if agents.is_empty() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skill manager mutation requires at least one explicit agent target".to_string(),
        ));
    }
    normalize_manager_agents(agents)
}

pub(super) fn manager_agent_alias(agent: &str) -> Result<String, CommandError> {
    let normalized = agent.trim().to_ascii_lowercase().replace([' ', '_'], "-");
    let mapped = match normalized.as_str() {
        "claude" | "claude-code" => "claude-code",
        "pi" => "pi",
        "opencode" | "open-code" => "opencode",
        "codex" => "codex",
        "hermes" | "hermes-agent" => "hermes-agent",
        "openclaw" | "open-claw" => "openclaw",
        _ => {
            return Err(CommandError::InvalidSkillManagerRequest(format!(
                "unsupported skill manager agent target: {agent}"
            )))
        }
    };
    Ok(mapped.to_string())
}

pub(super) fn normalized_skill_names(skills: &[String]) -> Result<Vec<String>, CommandError> {
    let mut names = Vec::new();
    for skill in skills {
        let trimmed = skill.trim();
        if trimmed.is_empty() {
            continue;
        }
        if trimmed.contains('\0') {
            return Err(CommandError::InvalidSkillManagerRequest(
                "skill name contains NUL".to_string(),
            ));
        }
        names.push(trimmed.to_string());
    }
    Ok(names)
}

pub(super) fn append_agent_args(args: &mut Vec<String>, agents: &[String]) {
    for agent in agents {
        args.push("--agent".to_string());
        args.push(agent.clone());
    }
}

pub(super) fn append_scope_args(
    args: &mut Vec<String>,
    scope: Option<&str>,
) -> Result<(), CommandError> {
    match normalize_manager_scope(scope)?.as_deref() {
        Some("global") => args.push("--global".to_string()),
        Some("project") | None => {}
        Some(_) => unreachable!(),
    }
    Ok(())
}

pub(super) fn normalize_manager_scope(scope: Option<&str>) -> Result<Option<String>, CommandError> {
    match scope.map(str::trim).filter(|scope| !scope.is_empty()) {
        None => Ok(None),
        Some(scope)
            if scope.eq_ignore_ascii_case("project") || scope == Scope::AgentProject.as_str() =>
        {
            Ok(Some("project".to_string()))
        }
        Some(scope)
            if scope.eq_ignore_ascii_case("global") || scope == Scope::AgentGlobal.as_str() =>
        {
            Ok(Some("global".to_string()))
        }
        Some(other) => Err(CommandError::InvalidSkillManagerRequest(format!(
            "unsupported skill manager scope: {other}"
        ))),
    }
}

pub(super) fn manager_cwd(
    ctx: &AdapterContext,
    scope: Option<&str>,
) -> Result<PathBuf, CommandError> {
    if normalize_manager_scope(scope)?.as_deref() == Some("global") {
        return Ok(ctx.user_home.clone());
    }
    Ok(ctx
        .project_cwd
        .clone()
        .or_else(|| ctx.project_root.clone())
        .unwrap_or_else(|| ctx.user_home.clone()))
}
