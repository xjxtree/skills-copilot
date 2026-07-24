use std::{
    fs,
    path::{Path, PathBuf},
};

use crate::CommandError;

#[derive(Debug, Clone, Eq, PartialEq)]
pub(super) enum ManagerSourceResolution {
    Local(PathBuf),
    Network,
}

pub(super) fn resolve_manager_source(
    source: &str,
    manager_cwd: &Path,
) -> Result<ManagerSourceResolution, CommandError> {
    let source = source.trim();
    if source.contains('\0') {
        return Err(CommandError::InvalidSkillManagerRequest(
            "skill manager source contains an invalid character".to_string(),
        ));
    }
    if source.contains("://") {
        if is_file_url(source) {
            return resolve_file_url(source);
        }
        validate_remote_repository_url(source)?;
        return Ok(ManagerSourceResolution::Network);
    }

    let path = PathBuf::from(source);
    let explicit_local = path.is_absolute() || source.starts_with('.') || source.starts_with('/');
    let candidate = if path.is_absolute() {
        path
    } else {
        manager_cwd.join(path)
    };
    if candidate.exists() {
        return resolve_local_manager_source(candidate);
    }
    if explicit_local {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local skill manager source does not exist at the selected manager scope".to_string(),
        ));
    }
    if source.starts_with("git@") || looks_like_scp_git_source(source) {
        validate_scp_git_source(source)?;
        return Ok(ManagerSourceResolution::Network);
    }
    Ok(ManagerSourceResolution::Network)
}

fn resolve_file_url(source: &str) -> Result<ManagerSourceResolution, CommandError> {
    if source.contains('%') || source.contains('\\') {
        return Err(invalid_remote_source());
    }
    let url = url::Url::parse(source).map_err(|_| invalid_remote_source())?;
    if !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        return Err(invalid_remote_source());
    }
    let path = url.to_file_path().map_err(|_| {
        CommandError::InvalidSkillManagerRequest(
            "skill manager file URL is not a valid local path".to_string(),
        )
    })?;
    resolve_local_manager_source(path)
}

fn validate_remote_repository_url(source: &str) -> Result<(), CommandError> {
    if source.contains('%') || source.contains('\\') {
        return Err(invalid_remote_source());
    }
    let Some((scheme, authority_and_path)) = source.split_once("://") else {
        return Err(invalid_remote_source());
    };
    let is_https = scheme.eq_ignore_ascii_case("https");
    let is_ssh = scheme.eq_ignore_ascii_case("ssh");
    if (!is_https && !is_ssh) || authority_and_path.starts_with('/') {
        return Err(invalid_remote_source());
    }
    let url = url::Url::parse(source).map_err(|_| invalid_remote_source())?;
    let valid_username = url.username().is_empty() || (is_ssh && url.username() == "git");
    if !valid_username
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
        || url.host_str().is_none_or(str::is_empty)
    {
        return Err(invalid_remote_source());
    }
    let raw_path = authority_and_path
        .find('/')
        .map(|index| &authority_and_path[index..])
        .unwrap_or_default();
    if !valid_repository_path(raw_path) {
        return Err(invalid_remote_source());
    }
    Ok(())
}

fn validate_scp_git_source(source: &str) -> Result<(), CommandError> {
    if !source.starts_with("git@")
        || source.contains('?')
        || source.contains('#')
        || source.contains('%')
        || source.contains('\\')
    {
        return Err(invalid_remote_source());
    }
    let Some((authority, path)) = source.split_once(':') else {
        return Err(invalid_remote_source());
    };
    let host = authority.strip_prefix("git@").unwrap_or_default();
    if host.is_empty()
        || host.contains('@')
        || host.contains('/')
        || host.chars().any(char::is_whitespace)
        || !valid_repository_path(path)
    {
        return Err(invalid_remote_source());
    }
    Ok(())
}

fn valid_repository_path(path: &str) -> bool {
    let path = path.strip_prefix('/').unwrap_or(path);
    !path.is_empty()
        && !path.ends_with('/')
        && !path.contains('\\')
        && path
            .split('/')
            .all(|segment| !segment.is_empty() && segment != "." && segment != "..")
}

pub(super) fn looks_like_scp_git_source(source: &str) -> bool {
    if source.contains("://") {
        return false;
    }
    let Some(colon) = source.find(':') else {
        return false;
    };
    if colon == 0 || colon + 1 >= source.len() {
        return false;
    }
    let authority = &source[..colon];
    let path = &source[colon + 1..];
    let valid_authority = authority
        .split_once('@')
        .map_or(!authority.is_empty(), |(user, host)| {
            !user.is_empty() && !host.is_empty() && !host.contains('@')
        });
    valid_authority
        && !authority.contains('/')
        && !authority.contains('\\')
        && !authority.chars().any(char::is_whitespace)
        && !path.chars().any(char::is_whitespace)
        && colon + 1 < source.len()
}

fn resolve_local_manager_source(path: PathBuf) -> Result<ManagerSourceResolution, CommandError> {
    let canonical = path.canonicalize().map_err(|_| {
        CommandError::InvalidSkillManagerRequest(
            "local skill manager source cannot be resolved".to_string(),
        )
    })?;
    let metadata = fs::metadata(&canonical)?;
    if !metadata.is_file() && !metadata.is_dir() {
        return Err(CommandError::InvalidSkillManagerRequest(
            "local skill manager source is not a regular file or directory".to_string(),
        ));
    }
    Ok(ManagerSourceResolution::Local(canonical))
}

pub(super) fn normalized_remote_manager_install_source(
    source: &str,
    manager_cwd: &Path,
) -> Result<String, CommandError> {
    if is_file_url(source.trim()) {
        return Err(CommandError::LocalSkillManagerSourceUnsupported);
    }
    match resolve_manager_source(source, manager_cwd)? {
        ManagerSourceResolution::Local(_) => {
            return Err(CommandError::LocalSkillManagerSourceUnsupported);
        }
        ManagerSourceResolution::Network => {}
    }

    let source = source.trim();
    if source.contains("://") || looks_like_scp_git_source(source) {
        return Ok(source.to_string());
    }

    let shorthand = source.trim_matches('/');
    let components = shorthand.split('/').collect::<Vec<_>>();
    if components.len() != 2
        || components
            .iter()
            .any(|component| !valid_github_shorthand_component(component))
    {
        return Err(invalid_remote_source());
    }
    let owner = components[0];
    let repository = components[1].strip_suffix(".git").unwrap_or(components[1]);
    if !valid_github_shorthand_component(repository) {
        return Err(invalid_remote_source());
    }
    Ok(format!("https://github.com/{owner}/{repository}.git"))
}

fn is_file_url(source: &str) -> bool {
    source
        .get(..7)
        .is_some_and(|prefix| prefix.eq_ignore_ascii_case("file://"))
}

fn valid_github_shorthand_component(component: &str) -> bool {
    !component.is_empty()
        && component != "."
        && component != ".."
        && component.len() <= 128
        && component
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_' | b'.'))
}

fn invalid_remote_source() -> CommandError {
    CommandError::InvalidSkillManagerRequest(
        "remote skill manager sources must use a credential-free HTTPS or SSH repository URL, git SCP form, or owner/repository GitHub shorthand"
            .to_string(),
    )
}
