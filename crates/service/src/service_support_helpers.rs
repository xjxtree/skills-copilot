use super::*;
use std::io::{self, Write};
use std::path::Component;
use url::Url;

pub(crate) fn scan_all_label(agent_reports: &[AgentCatalogScanReport]) -> String {
    let labels: Vec<&str> = agent_reports
        .iter()
        .map(|report| report.display_name)
        .collect();
    display_label_list(&labels).unwrap_or_else(|| "supported agents".to_string())
}

pub(crate) fn display_label_list(labels: &[&str]) -> Option<String> {
    match labels {
        [] => None,
        [one] => Some((*one).to_string()),
        [first, second] => Some(format!("{first} and {second}")),
        _ => {
            let mut label = labels[..labels.len() - 1].join(", ");
            label.push_str(", and ");
            label.push_str(labels[labels.len() - 1]);
            Some(label)
        }
    }
}

pub(crate) fn skipped_roots_detail(roots_skipped: &[String]) -> String {
    if roots_skipped.is_empty() {
        return String::new();
    }
    let mut detail = format!("; root-error skipped-root path(s): {}", roots_skipped[0]);
    if roots_skipped.len() > 1 {
        detail.push_str(&format!(" (+{} more)", roots_skipped.len() - 1));
    }
    detail
}

pub fn handle_request_json(input: &str) -> String {
    let response = match serde_json::from_str::<ServiceRequest>(input) {
        Ok(request) => match ServiceHost::from_env() {
            Ok(host) => host.handle(request),
            Err(error) => ServiceResponse {
                id: None,
                ok: false,
                result: None,
                error: Some(ServiceErrorRecord {
                    code: error.code().to_string(),
                    message: error.to_string(),
                    details: error.details(),
                }),
            },
        },
        Err(error) => ServiceResponse {
            id: None,
            ok: false,
            result: None,
            error: Some(ServiceErrorRecord {
                code: "parse_error".to_string(),
                message: error.to_string(),
                details: None,
            }),
        },
    };
    serde_json::to_string(&response).unwrap_or_else(|error| {
        json!({
            "id": null,
            "ok": false,
            "error": {
                "code": "serialize_error",
                "message": error.to_string()
            }
        })
        .to_string()
    })
}

pub(crate) fn default_app_data_dir(user_home: &Path) -> PathBuf {
    app_data_dir_for_bundle_id(user_home, DEFAULT_BUNDLE_ID)
}

pub(crate) fn legacy_app_data_dir(user_home: &Path) -> PathBuf {
    app_data_dir_for_bundle_id(user_home, LEGACY_BUNDLE_ID)
}

pub(crate) fn resolve_default_app_data_dir(user_home: &Path) -> Result<PathBuf, ServiceError> {
    let preferred = default_app_data_dir(user_home);
    let legacy = legacy_app_data_dir(user_home);
    crate::app_data_migration::migrate_legacy_app_data_dir(&legacy, &preferred)?;
    Ok(preferred)
}

fn app_data_dir_for_bundle_id(user_home: &Path, bundle_id: &str) -> PathBuf {
    if cfg!(target_os = "macos") {
        user_home
            .join("Library")
            .join("Application Support")
            .join(bundle_id)
    } else {
        user_home.join(".skills-copilot").join(bundle_id)
    }
}

pub(crate) fn infer_project_root(cwd: &Path) -> PathBuf {
    let mut current = Some(cwd);
    while let Some(dir) = current {
        if dir.join(".git").exists() {
            return dir.to_path_buf();
        }
        current = dir.parent();
    }
    cwd.to_path_buf()
}

pub(crate) fn display_path(path: &Path) -> String {
    path.to_string_lossy().to_string()
}

pub(crate) fn supported_methods() -> Vec<&'static str> {
    SUPPORTED_METHODS.to_vec()
}

pub(crate) fn normalize_string_list(values: Vec<String>) -> Vec<String> {
    let mut normalized = values
        .into_iter()
        .map(|value| value.trim().to_string())
        .filter(|value| !value.is_empty())
        .collect::<Vec<_>>();
    normalized.sort();
    normalized.dedup();
    normalized
}

pub(crate) fn hex_prefix(bytes: &[u8], chars: usize) -> String {
    bytes
        .iter()
        .flat_map(|byte| {
            let high = b"0123456789abcdef"[(byte >> 4) as usize] as char;
            let low = b"0123456789abcdef"[(byte & 0x0f) as usize] as char;
            [high, low]
        })
        .take(chars)
        .collect()
}

pub(crate) fn parse_agent_param(agent: &str) -> Result<AgentId, ServiceError> {
    match agent {
        "claude-code" => Ok(AgentId::ClaudeCode),
        "codex" => Ok(AgentId::Codex),
        "opencode" => Ok(AgentId::Opencode),
        "pi" => Ok(AgentId::Pi),
        "hermes" => Ok(AgentId::Hermes),
        "openclaw" => Ok(AgentId::Openclaw),
        other => Err(ServiceError::InvalidRequest(format!(
            "unsupported target_agent '{other}'"
        ))),
    }
}

pub(crate) fn parse_scope_param(scope: &str) -> Result<Scope, ServiceError> {
    match scope {
        "agent-global" => Ok(Scope::AgentGlobal),
        "agent-project" => Ok(Scope::AgentProject),
        "tool-global" => Ok(Scope::ToolGlobal),
        other => Err(ServiceError::InvalidRequest(format!(
            "unsupported target_scope '{other}'"
        ))),
    }
}

pub(crate) fn create_private_dir_all(path: &Path) -> io::Result<()> {
    create_private_directory_tree_no_follow(path)
}

pub(crate) fn write_private_text_file(path: &Path, content: &str) -> io::Result<()> {
    write_private_bytes_file(path, content.as_bytes())
}

pub(crate) fn write_private_bytes_file(path: &Path, content: &[u8]) -> io::Result<()> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "private file has no parent"))?;
    create_private_dir_all(parent)?;
    reject_symlink(path, "private file")?;

    let tmp = private_tmp_path(path)?;
    reject_symlink(&tmp, "private temp file")?;
    let mut options = fs::OpenOptions::new();
    options.write(true).create_new(true);
    #[cfg(unix)]
    {
        use std::os::unix::fs::OpenOptionsExt;
        options.mode(0o600);
        options.custom_flags(rustix::fs::OFlags::NOFOLLOW.bits() as i32);
    }
    let write_result = (|| -> io::Result<()> {
        let mut file = options.open(&tmp)?;
        set_private_file_permissions(&file)?;
        file.write_all(content)?;
        file.sync_all()?;
        drop(file);

        reject_symlink(path, "private file")?;
        fs::rename(&tmp, path)?;
        set_private_path_permissions(path)?;
        sync_parent_dir(parent);
        Ok(())
    })();
    if write_result.is_err() {
        let _ = fs::remove_file(&tmp);
    }
    write_result
}

fn private_tmp_path(path: &Path) -> io::Result<PathBuf> {
    let parent = path
        .parent()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "private file has no parent"))?;
    let file_name = path
        .file_name()
        .ok_or_else(|| io::Error::new(io::ErrorKind::InvalidInput, "private file has no name"))?
        .to_string_lossy();
    Ok(parent.join(format!(
        ".{file_name}.{}.{}.tmp",
        std::process::id(),
        unix_timestamp_millis()
    )))
}

fn reject_symlink(path: &Path, label: &str) -> io::Result<()> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.file_type().is_symlink() => Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            format!("{label} is a symlink: {}", path.display()),
        )),
        Ok(_) => Ok(()),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
        Err(error) => Err(error),
    }
}

#[cfg(unix)]
fn create_private_directory_tree_no_follow(path: &Path) -> io::Result<()> {
    use rustix::fs::{fchmod, Mode};

    let directory = open_private_directory_tree_no_follow(path, true)?;
    fchmod(&directory, Mode::from_bits_truncate(0o700)).map_err(io::Error::from)
}

#[cfg(unix)]
fn open_private_directory_tree_no_follow(
    path: &Path,
    create_missing: bool,
) -> io::Result<fs::File> {
    use rustix::fs::{mkdirat, open, openat, Mode, OFlags};
    use rustix::io::Errno;

    let path = normalize_private_root_alias(path)?;
    let flags = OFlags::RDONLY | OFlags::DIRECTORY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let mut current = if path.is_absolute() {
        open("/", flags, Mode::empty()).map_err(io::Error::from)?
    } else {
        open(".", flags, Mode::empty()).map_err(io::Error::from)?
    };
    let mut saw_name = false;
    for component in path.components() {
        let name = match component {
            Component::RootDir | Component::CurDir => continue,
            Component::Normal(name) => name,
            Component::ParentDir | Component::Prefix(_) => return Err(unsafe_private_path()),
        };
        saw_name = true;
        match openat(&current, name, flags, Mode::empty()) {
            Ok(next) => current = next,
            Err(error) if error == Errno::NOENT && create_missing => {
                let mode = Mode::from_bits_truncate(0o700);
                match mkdirat(&current, name, mode) {
                    Ok(()) | Err(Errno::EXIST) => {}
                    Err(error) => return Err(io::Error::from(error)),
                }
                current = openat(&current, name, flags, Mode::empty()).map_err(|error| {
                    if error == Errno::LOOP || error == Errno::NOTDIR {
                        unsafe_private_path()
                    } else {
                        io::Error::from(error)
                    }
                })?;
            }
            Err(error) if error == Errno::LOOP || error == Errno::NOTDIR => {
                return Err(unsafe_private_path())
            }
            Err(error) => return Err(io::Error::from(error)),
        }
    }
    if !saw_name {
        return Err(unsafe_private_path());
    }
    Ok(fs::File::from(current))
}

#[cfg(not(unix))]
fn create_private_directory_tree_no_follow(path: &Path) -> io::Result<()> {
    walk_private_directory_tree_no_follow(path, true)
}

#[cfg(not(unix))]
fn walk_private_directory_tree_no_follow(path: &Path, create_missing: bool) -> io::Result<()> {
    let mut current = PathBuf::new();
    let mut saw_name = false;
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => current.push(prefix.as_os_str()),
            Component::RootDir => current.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => return Err(unsafe_private_path()),
            Component::Normal(name) => {
                saw_name = true;
                current.push(name);
                match fs::symlink_metadata(&current) {
                    Ok(metadata) if metadata.file_type().is_symlink() || !metadata.is_dir() => {
                        return Err(unsafe_private_path())
                    }
                    Ok(_) => {}
                    Err(error) if error.kind() == io::ErrorKind::NotFound && create_missing => {
                        fs::create_dir(&current)?;
                    }
                    Err(error) => return Err(error),
                }
            }
        }
    }
    if !saw_name {
        return Err(unsafe_private_path());
    }
    Ok(())
}

#[cfg(unix)]
fn normalize_private_root_alias(path: &Path) -> io::Result<PathBuf> {
    use std::os::unix::fs::MetadataExt;

    if !path.is_absolute() {
        return Ok(path.to_path_buf());
    }
    let mut components = path.components();
    if components.next() != Some(Component::RootDir) {
        return Err(unsafe_private_path());
    }
    let Some(Component::Normal(first)) = components.next() else {
        return Ok(path.to_path_buf());
    };
    let root_entry = Path::new("/").join(first);
    let metadata = match fs::symlink_metadata(&root_entry) {
        Ok(metadata) => metadata,
        Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(path.to_path_buf()),
        Err(error) => return Err(error),
    };
    if !metadata.file_type().is_symlink() {
        return Ok(path.to_path_buf());
    }
    if metadata.uid() != 0 {
        return Err(unsafe_private_path());
    }
    let mut normalized = fs::canonicalize(&root_entry)?;
    for component in components {
        match component {
            Component::Normal(name) => normalized.push(name),
            Component::CurDir => {}
            Component::RootDir | Component::ParentDir | Component::Prefix(_) => {
                return Err(unsafe_private_path())
            }
        }
    }
    Ok(normalized)
}

fn unsafe_private_path() -> io::Error {
    io::Error::new(
        io::ErrorKind::InvalidInput,
        "private directory path must not contain symlinks or parent traversal",
    )
}

#[cfg(unix)]
fn set_private_file_permissions(file: &fs::File) -> io::Result<()> {
    use std::os::unix::fs::PermissionsExt;

    file.set_permissions(fs::Permissions::from_mode(0o600))
}

#[cfg(not(unix))]
fn set_private_file_permissions(_file: &fs::File) -> io::Result<()> {
    Ok(())
}

#[cfg(unix)]
fn set_private_path_permissions(path: &Path) -> io::Result<()> {
    use rustix::fs::{fchmod, open, Mode, OFlags};

    let flags = OFlags::RDONLY | OFlags::NOFOLLOW | OFlags::CLOEXEC;
    let file = fs::File::from(open(path, flags, Mode::empty()).map_err(io::Error::from)?);
    if !file.metadata()?.is_file() {
        return Err(io::Error::new(
            io::ErrorKind::InvalidInput,
            "private file destination is not a regular file",
        ));
    }
    fchmod(&file, Mode::from_bits_truncate(0o600)).map_err(io::Error::from)
}

#[cfg(not(unix))]
fn set_private_path_permissions(_path: &Path) -> io::Result<()> {
    Ok(())
}

fn sync_parent_dir(parent: &Path) {
    if let Ok(parent_dir) = fs::File::open(parent) {
        let _ = parent_dir.sync_all();
    }
}

pub(crate) fn is_pi_plain_markdown_catalog_noise(skill: &SkillRecord) -> bool {
    skill.agent == AgentId::Pi.as_str()
        && skill
            .path
            .extension()
            .and_then(|extension| extension.to_str())
            == Some("md")
        && skill.path.file_name().and_then(|name| name.to_str()) != Some("SKILL.md")
}

pub(crate) fn unix_timestamp_millis() -> i64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .map(|duration| i64::try_from(duration.as_millis()).unwrap_or(i64::MAX))
        .unwrap_or(0)
}

pub(crate) fn estimate_tokens(parts: &[&str]) -> u32 {
    let chars = parts.iter().map(|part| part.chars().count()).sum::<usize>();
    let estimated = chars.div_ceil(4).saturating_add(120);
    u32::try_from(estimated).unwrap_or(u32::MAX)
}

#[derive(Debug, Clone)]
pub(crate) struct BuiltLlmPrompt {
    pub(crate) prompt_preview: String,
    pub(crate) prompt_scope: Vec<String>,
    pub(crate) included_fields: Vec<String>,
    pub(crate) excluded_fields: Vec<String>,
    pub(crate) redaction: LlmPromptRedactionSummary,
    pub(crate) estimated_output_tokens: u32,
}

pub(crate) struct PromptRedactor<'a> {
    roots: &'a [(String, &'static str)],
    redacted_value_count: usize,
    redacted_fields: BTreeMap<String, ()>,
}

impl<'a> PromptRedactor<'a> {
    pub(crate) fn new(roots: &'a [(String, &'static str)]) -> Self {
        Self {
            roots,
            redacted_value_count: 0,
            redacted_fields: BTreeMap::new(),
        }
    }

    pub(crate) fn redact(&mut self, value: &str) -> String {
        let (path_redacted, path_count) = redact_with_count(value, self.roots);
        if path_count > 0 {
            self.redacted_value_count += path_count;
            self.redacted_fields.insert("local paths".to_string(), ());
        }
        let mut token_count = 0usize;
        let mut redact_next_token = false;
        let redacted = path_redacted
            .split_whitespace()
            .map(|token| {
                let trimmed = token.trim_matches(|ch: char| {
                    matches!(ch, '"' | '\'' | ',' | ';' | ')' | '(' | '[' | ']')
                });
                let lower = trimmed.to_lowercase();
                if redact_next_token {
                    redact_next_token = lower == "bearer";
                    token_count += 1;
                    "<redacted>"
                } else if lower.contains("key")
                    || lower.contains("token")
                    || lower.contains("secret")
                    || lower.contains("credential")
                    || lower.contains("password")
                    || lower == "authorization:"
                    || lower == "bearer"
                {
                    redact_next_token = !trimmed.contains('=');
                    token_count += 1;
                    "<redacted>"
                } else if lower.starts_with("http://") || lower.starts_with("https://") {
                    token_count += 1;
                    "<redacted-url>"
                } else if looks_like_high_entropy_secret(trimmed) {
                    token_count += 1;
                    "<redacted-secret>"
                } else {
                    token
                }
            })
            .collect::<Vec<_>>()
            .join(" ");
        if token_count > 0 {
            self.redacted_value_count += token_count;
            self.redacted_fields
                .insert("secret-like tokens and private URLs".to_string(), ());
        }
        redacted
    }

    pub(crate) fn summary(self) -> LlmPromptRedactionSummary {
        LlmPromptRedactionSummary {
            status: "redacted-preview-confirmed-required".to_string(),
            redacted_value_count: self.redacted_value_count,
            redacted_fields: self.redacted_fields.into_keys().collect(),
            placeholders: vec![
                "$HOME",
                "<project-root>",
                "<project-cwd>",
                "<app-data-dir>",
                "<redacted>",
                "<redacted-url>",
            ],
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_secret_returned: false,
        }
    }
}

pub(crate) fn looks_like_high_entropy_secret(value: &str) -> bool {
    let token = value.trim_matches(|ch: char| {
        matches!(
            ch,
            '"' | '\'' | ',' | ';' | ')' | '(' | '[' | ']' | '{' | '}' | ':' | '.'
        )
    });
    let len = token.chars().count();
    if len < 32 || token.contains('/') || token.contains('\\') {
        return false;
    }
    let allowed = token
        .chars()
        .all(|ch| ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '=' | '+'));
    if !allowed {
        return false;
    }
    let has_upper = token.chars().any(|ch| ch.is_ascii_uppercase());
    let has_lower = token.chars().any(|ch| ch.is_ascii_lowercase());
    let has_digit = token.chars().any(|ch| ch.is_ascii_digit());
    let has_symbol = token.chars().any(|ch| matches!(ch, '-' | '_' | '=' | '+'));
    let class_count = [has_upper, has_lower, has_digit, has_symbol]
        .into_iter()
        .filter(|flag| *flag)
        .count();
    let unique_count = token.chars().collect::<BTreeSet<_>>().len();
    class_count >= 3 && unique_count >= 16
}

pub(crate) fn redact_with_count(value: &str, roots: &[(String, &'static str)]) -> (String, usize) {
    let mut redacted = value.to_string();
    let mut count = 0usize;
    for (root, placeholder) in roots {
        if !root.is_empty() && redacted.contains(root) {
            count += redacted.matches(root).count();
            redacted = redacted.replace(root, placeholder);
        }
    }
    if count > 0 {
        redacted = redacted.replace('\\', "/");
    }
    (redacted, count)
}

pub(crate) fn llm_preview_id(
    params: &LlmPreviewPromptParams,
    profile: Option<&ProviderProfileRecord>,
    prompt_preview: &str,
    estimated_input_tokens: u32,
    estimated_output_tokens: u32,
) -> String {
    let profile_fingerprint = profile
        .map(|profile| {
            format!(
                "{}\x1f{}\x1f{}\x1f{}",
                profile.id,
                profile.provider_type.as_str(),
                profile.base_url,
                profile.model
            )
        })
        .unwrap_or_else(|| "no-profile".to_string());
    let source = serde_json::json!({
        "version": "v2.42",
        "profile": profile_fingerprint,
        "action": params.action.as_str(),
        "skill_instance_id": params.skill_instance_id,
        "instance_ids": params.instance_ids,
        "user_intent": params.user_intent.as_deref(),
        "prompt": prompt_preview,
        "estimated_input_tokens": estimated_input_tokens,
        "estimated_output_tokens": estimated_output_tokens
    });
    let digest = Sha256::digest(source.to_string().as_bytes());
    format!("prompt-preview-{digest:x}")
}

pub(crate) fn llm_prompt_action_type(params: &LlmPreviewPromptParams) -> String {
    params.action.as_str().to_string()
}

pub(crate) fn inferred_llm_prompt_scope(params: &LlmPreviewPromptParams) -> Option<String> {
    if params.instance_ids.len() > 1 {
        Some("visible".to_string())
    } else if params.skill_instance_id.is_some() || params.instance_ids.len() == 1 {
        Some("selected".to_string())
    } else {
        None
    }
}

pub(crate) fn destination_host_for_url(base_url: &str) -> String {
    let Ok(url) = Url::parse(base_url) else {
        return "<unknown>".to_string();
    };
    let Some(host) = url.host_str() else {
        return "<unknown>".to_string();
    };
    let host = if host.contains(':') {
        format!("[{host}]")
    } else {
        host.to_string()
    };
    match url.port() {
        Some(port) => format!("{host}:{port}"),
        None => host,
    }
}
