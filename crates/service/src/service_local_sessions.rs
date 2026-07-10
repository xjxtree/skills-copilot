use super::service_local_session_io::{
    read_bounded_text, BoundedReadSpec, BoundedText, LocalSessionIoContext, LocalSessionReadLimits,
};
use super::*;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

impl ServiceHost {
    pub fn preview_local_sessions(
        &self,
        params: LocalSessionPreviewParams,
    ) -> Result<LocalSessionPreviewResult, ServiceError> {
        let limit = params.limit.unwrap_or(20).clamp(1, 100);
        let max_files = params.max_files.unwrap_or(200).clamp(1, 1_000);
        let max_excerpt_chars = params.max_excerpt_chars.unwrap_or(1_000).clamp(120, 4_000);
        let requested_roots = normalize_string_list(params.authorized_roots);
        let auto_discover = params.auto_discover.unwrap_or(requested_roots.is_empty());
        let adapter_ctx = self.effective_adapter_ctx()?;
        let scope = LocalSessionScope::from_param(params.scope.as_deref());
        let project_filter_roots = local_session_project_filter_roots(
            &adapter_ctx,
            params.project_root.as_deref(),
            params.current_cwd.as_deref(),
        );
        let search = params
            .search
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase());
        let redaction_roots = self.trace_redaction_roots(&adapter_ctx);
        let mut redactor = PromptRedactor::new(&redaction_roots);
        let requested_agent = params.agent.as_deref();

        let mut root_requests = requested_roots
            .iter()
            .map(|root| LocalSessionRootRequest {
                path: PathBuf::from(root),
                status: "authorized-read-only",
                source_kind: "authorized-local-session",
            })
            .collect::<Vec<_>>();
        let mut gap_notes = Vec::new();
        let mut blocker_notes = Vec::new();
        if auto_discover {
            let (mut discovered_roots, discovery_notes) = auto_local_session_roots(
                &adapter_ctx,
                requested_agent,
                scope,
                &project_filter_roots,
            );
            root_requests.append(&mut discovered_roots);
            gap_notes.extend(discovery_notes);
        }
        dedupe_local_session_root_requests(&mut root_requests);

        if root_requests.is_empty() {
            if gap_notes.is_empty() {
                gap_notes.push(
                    "No supported local agent session store was found for the selected agent."
                        .to_string(),
                );
            }
            return Ok(LocalSessionPreviewResult {
                generated_by: "local-v2.98",
                authorized: false,
                authorization_required: false,
                roots: Vec::new(),
                count: 0,
                total_candidate_count: 0,
                total_matched_count: 0,
                offset: 0,
                limit,
                has_more: false,
                next_offset: None,
                user_message_count: 0,
                total_message_count: 0,
                tool_call_count: 0,
                skill_call_count: 0,
                skill_usage_rows: Vec::new(),
                session_rows: Vec::new(),
                gap_notes,
                blocker_notes,
                redaction_summary: local_preview_redaction_summary_from(redactor.summary()),
                safety_flags: local_preview_safety_flags(),
                read_only: true,
                provider_request_sent: false,
                skill_files_mutated: false,
                agent_config_mutated: false,
                snapshot_created: false,
                triage_mutated: false,
                raw_prompt_persisted: false,
                raw_response_persisted: false,
                raw_trace_persisted: false,
            });
        }

        let mut root_rows = Vec::new();
        let mut session_rows = Vec::new();
        let mut seen_session_row_ids = BTreeSet::new();
        let mut skill_usage = BTreeMap::<String, LocalSessionSkillUsageAccumulator>::new();
        let skill_matchers = self.local_session_skill_matchers(requested_agent)?;
        let mut total_candidate_count = 0usize;
        let mut io = LocalSessionIoContext::new(LocalSessionReadLimits::default());

        for root_request in root_requests {
            let root = root_request.path.to_string_lossy().to_string();
            let redacted_root = redactor.redact(&root);
            let root_path = root_request.path;
            if !root_path.is_absolute() {
                let blocker = "Authorized session roots must be absolute paths.".to_string();
                blocker_notes.push(format!("{redacted_root}: {blocker}"));
                root_rows.push(LocalSessionPreviewRoot {
                    root: redacted_root,
                    status: "blocked".to_string(),
                    candidate_count: 0,
                    blocker: Some(blocker),
                });
                continue;
            }
            if !root_path.exists() {
                let blocker = "Authorized session root does not exist.".to_string();
                blocker_notes.push(format!("{redacted_root}: {blocker}"));
                root_rows.push(LocalSessionPreviewRoot {
                    root: redacted_root,
                    status: "blocked".to_string(),
                    candidate_count: 0,
                    blocker: Some(blocker),
                });
                continue;
            }
            if !root_path.is_dir() {
                let blocker = "Authorized session root is not a directory.".to_string();
                blocker_notes.push(format!("{redacted_root}: {blocker}"));
                root_rows.push(LocalSessionPreviewRoot {
                    root: redacted_root,
                    status: "blocked".to_string(),
                    candidate_count: 0,
                    blocker: Some(blocker),
                });
                continue;
            }

            let canonical_root = match root_path.canonicalize() {
                Ok(path) => path,
                Err(error) => {
                    let blocker = format!("Authorized session root could not be resolved: {error}");
                    blocker_notes.push(format!("{redacted_root}: {}", redactor.redact(&blocker)));
                    root_rows.push(LocalSessionPreviewRoot {
                        root: redacted_root,
                        status: "blocked".to_string(),
                        candidate_count: 0,
                        blocker: Some(redactor.redact(&blocker)),
                    });
                    continue;
                }
            };

            let files = collect_local_session_files(
                &canonical_root,
                max_files,
                &mut gap_notes,
                &mut redactor,
            );
            total_candidate_count += files.len();
            let mut root_candidate_count = 0usize;
            for file in files {
                let options = LocalSessionPreviewRowOptions {
                    requested_agent,
                    max_excerpt_chars,
                    source_kind: root_request.source_kind,
                    skill_matchers: &skill_matchers,
                    scope,
                    project_filter_roots: &project_filter_roots,
                    search: search.as_deref(),
                };
                match local_session_preview_row(
                    &file,
                    &canonical_root,
                    options,
                    &mut io,
                    &mut redactor,
                ) {
                    Ok(Some(entry)) => {
                        if seen_session_row_ids.insert(entry.row.id.clone()) {
                            root_candidate_count += 1;
                            update_local_session_skill_usage(&mut skill_usage, &entry);
                            session_rows.push(entry.row);
                        }
                    }
                    Ok(None) => {}
                    Err(error) => {
                        gap_notes.push(format!(
                            "{}: {}",
                            redactor.redact(&file.to_string_lossy()),
                            redactor.redact(&error.to_string())
                        ));
                    }
                }
            }

            root_rows.push(LocalSessionPreviewRoot {
                root: redacted_root,
                status: root_request.status.to_string(),
                candidate_count: root_candidate_count,
                blocker: None,
            });
        }

        session_rows.sort_by(|left, right| {
            right
                .modified_at
                .cmp(&left.modified_at)
                .then_with(|| left.title.cmp(&right.title))
                .then_with(|| left.id.cmp(&right.id))
        });
        let total_matched_count = session_rows.len();
        let offset = params.offset.unwrap_or(0).min(total_matched_count);
        let page_end = offset.saturating_add(limit).min(total_matched_count);
        let has_more = page_end < total_matched_count;
        session_rows = session_rows.into_iter().skip(offset).take(limit).collect();
        let count = session_rows.len();
        let user_message_count = session_rows
            .iter()
            .map(|row| row.user_message_count)
            .sum::<usize>();
        let total_message_count = session_rows
            .iter()
            .map(|row| row.total_message_count)
            .sum::<usize>();
        let tool_call_count = session_rows
            .iter()
            .map(|row| row.tool_call_count)
            .sum::<usize>();
        let skill_call_count = session_rows
            .iter()
            .map(|row| row.skill_call_count)
            .sum::<usize>();
        if total_matched_count == 0 && blocker_notes.is_empty() {
            gap_notes.push(
                "Discovered local session stores did not contain supported session files (.jsonl, .json, .txt, .log)."
                    .to_string(),
            );
        }
        let skill_usage_rows = local_session_skill_usage_rows(skill_usage, limit);

        Ok(LocalSessionPreviewResult {
            generated_by: "local-v2.98",
            authorized: root_rows.iter().any(|root| {
                root.status == "authorized-read-only" || root.status == "auto-discovered-read-only"
            }),
            authorization_required: false,
            roots: root_rows,
            count,
            total_candidate_count,
            total_matched_count,
            offset,
            limit,
            has_more,
            next_offset: has_more.then_some(page_end),
            user_message_count,
            total_message_count,
            tool_call_count,
            skill_call_count,
            skill_usage_rows,
            session_rows,
            gap_notes,
            blocker_notes,
            redaction_summary: local_preview_redaction_summary_from(redactor.summary()),
            safety_flags: local_preview_safety_flags(),
            read_only: true,
            provider_request_sent: false,
            skill_files_mutated: false,
            agent_config_mutated: false,
            snapshot_created: false,
            triage_mutated: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
        })
    }

    fn local_session_skill_matchers(
        &self,
        requested_agent: Option<&str>,
    ) -> Result<Vec<LocalSessionSkillMatcher>, ServiceError> {
        let Some(catalog) = self.open_existing_catalog_read_only()? else {
            return Ok(Vec::new());
        };
        let skills = self.list_visible_skill_records(&catalog)?;
        Ok(skills
            .into_iter()
            .filter(|skill| {
                requested_agent.is_none_or(|agent| {
                    agent.eq_ignore_ascii_case(AgentId::ToolGlobal.as_str())
                        || skill.agent.eq_ignore_ascii_case(agent)
                        || skill
                            .agent
                            .eq_ignore_ascii_case(AgentId::ToolGlobal.as_str())
                })
            })
            .map(LocalSessionSkillMatcher::from)
            .collect())
    }
}

struct LocalSessionRootRequest {
    path: PathBuf,
    status: &'static str,
    source_kind: &'static str,
}

#[derive(Debug, Clone)]
struct LocalSessionPreviewEntry {
    row: LocalSessionPreviewRow,
    skill_mentions: Vec<LocalSessionSkillMention>,
}

#[derive(Debug, Clone, Copy)]
struct LocalSessionPreviewRowOptions<'a> {
    requested_agent: Option<&'a str>,
    max_excerpt_chars: usize,
    source_kind: &'static str,
    skill_matchers: &'a [LocalSessionSkillMatcher],
    scope: LocalSessionScope,
    project_filter_roots: &'a [PathBuf],
    search: Option<&'a str>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum LocalSessionScope {
    Project,
    All,
}

impl LocalSessionScope {
    fn from_param(value: Option<&str>) -> Self {
        match value
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(|value| value.to_ascii_lowercase())
            .as_deref()
        {
            Some("project")
            | Some("current")
            | Some("current_project")
            | Some("current-folder")
            | Some("current_folder") => Self::Project,
            _ => Self::All,
        }
    }

    fn as_str(self) -> &'static str {
        match self {
            Self::Project => "project",
            Self::All => "all",
        }
    }
}

#[derive(Debug, Clone)]
struct LocalSessionContentDraft {
    kind: String,
    title: String,
    text: String,
    timestamp: Option<i64>,
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Copy, Default)]
struct LocalSessionTimeBounds {
    started_at: Option<i64>,
    ended_at: Option<i64>,
}

impl LocalSessionTimeBounds {
    fn push(&mut self, timestamp: Option<i64>) {
        let Some(timestamp) = timestamp else {
            return;
        };
        self.started_at = Some(
            self.started_at
                .map_or(timestamp, |current| current.min(timestamp)),
        );
        self.ended_at = Some(
            self.ended_at
                .map_or(timestamp, |current| current.max(timestamp)),
        );
    }
}

#[derive(Debug, Clone)]
struct LocalSessionSkillMatcher {
    skill_id: String,
    skill_name: String,
    agent: String,
    needles: Vec<String>,
}

impl From<SkillRecord> for LocalSessionSkillMatcher {
    fn from(skill: SkillRecord) -> Self {
        let mut needles = Vec::new();
        push_session_skill_needle(&mut needles, &skill.name);
        push_session_skill_needle(&mut needles, &skill.definition_id);
        push_session_skill_needle(&mut needles, &skill.id);
        Self {
            skill_id: skill.id,
            skill_name: skill.name,
            agent: skill.agent,
            needles,
        }
    }
}

#[derive(Debug, Clone)]
struct LocalSessionSkillMention {
    skill_id: String,
    skill_name: String,
    agent: String,
    count: usize,
    matched_invocations: Vec<String>,
    evidence_ref: String,
}

#[derive(Debug, Default)]
struct LocalSessionSkillUsageAccumulator {
    skill_id: String,
    skill_name: String,
    agent: String,
    call_count: usize,
    session_count: usize,
    latest_modified_at: Option<i64>,
    evidence_refs: Vec<String>,
}

#[derive(Debug, Clone, Default)]
struct LocalSessionParsedMetadata {
    title: Option<String>,
    project_root: Option<String>,
    session_id: Option<String>,
}

fn auto_local_session_roots(
    adapter_ctx: &AdapterContext,
    requested_agent: Option<&str>,
    scope: LocalSessionScope,
    project_roots: &[PathBuf],
) -> (Vec<LocalSessionRootRequest>, Vec<String>) {
    let mut roots = Vec::new();
    let mut notes = Vec::new();
    let home = &adapter_ctx.user_home;

    if local_session_agent_matches(requested_agent, AgentId::ClaudeCode.as_str()) {
        let claude_projects = home.join(".claude/projects");
        let mut pushed_project_root = false;
        if scope == LocalSessionScope::Project {
            for project in project_roots {
                let encoded = encode_claude_project_session_dir(project);
                pushed_project_root |= push_existing_session_root(
                    &mut roots,
                    claude_projects.join(encoded),
                    "auto-discovered-read-only",
                    "auto-local-session",
                );
            }
        }
        push_existing_session_root(
            &mut roots,
            home.join(".claude/sessions"),
            "auto-discovered-read-only",
            "auto-local-session",
        );
        if scope == LocalSessionScope::All || project_roots.is_empty() || !pushed_project_root {
            push_existing_session_root(
                &mut roots,
                claude_projects,
                "auto-discovered-read-only",
                "auto-local-session",
            );
        }
    }

    if local_session_agent_matches(requested_agent, AgentId::Codex.as_str()) {
        push_existing_session_root(
            &mut roots,
            home.join(".codex/sessions"),
            "auto-discovered-read-only",
            "auto-local-session",
        );
    }

    if local_session_agent_matches(requested_agent, AgentId::Opencode.as_str()) {
        push_existing_session_root(
            &mut roots,
            home.join(".local/share/opencode/storage"),
            "auto-discovered-read-only",
            "auto-local-session",
        );
    }

    if local_session_agent_matches(requested_agent, AgentId::Pi.as_str()) {
        let pi_sessions = home.join(".pi/agent/sessions");
        let mut pushed_project_root = false;
        if scope == LocalSessionScope::Project {
            for project in project_roots {
                for encoded in encode_pi_project_session_dirs(project) {
                    pushed_project_root |= push_existing_session_root(
                        &mut roots,
                        pi_sessions.join(encoded),
                        "auto-discovered-read-only",
                        "auto-local-session",
                    );
                }
            }
        }
        if scope == LocalSessionScope::All || project_roots.is_empty() || !pushed_project_root {
            push_existing_session_root(
                &mut roots,
                pi_sessions,
                "auto-discovered-read-only",
                "auto-local-session",
            );
        }
        push_existing_session_root(
            &mut roots,
            home.join(".pi/context-mode/sessions"),
            "auto-discovered-read-only",
            "auto-local-session",
        );
    }

    if local_session_agent_matches(requested_agent, AgentId::Hermes.as_str()) {
        let state_db = home.join(".hermes/state.db");
        if state_db.exists() {
            notes.push(
                "Hermes session storage is SQLite-backed; automatic session parsing is deferred until the schema is confirmed."
                    .to_string(),
            );
        }
    }

    if local_session_agent_matches(requested_agent, AgentId::Openclaw.as_str()) {
        let openclaw_root = home.join(".openclaw");
        if openclaw_root.exists() {
            notes.push(
                "OpenClaw session storage is not yet format-confirmed for automatic local parsing."
                    .to_string(),
            );
        }
    }

    if roots.is_empty() && notes.is_empty() {
        notes.push(
            "No supported local session store was detected for Claude Code, Codex, opencode, or Pi."
                .to_string(),
        );
    }

    (roots, notes)
}

fn local_session_project_filter_roots(
    adapter_ctx: &AdapterContext,
    requested_project_root: Option<&str>,
    requested_current_cwd: Option<&str>,
) -> Vec<PathBuf> {
    let explicit_candidates = [requested_project_root, requested_current_cwd]
        .into_iter()
        .flatten()
        .map(str::to_string)
        .collect::<Vec<_>>();
    if !explicit_candidates.is_empty() {
        return normalized_local_session_project_roots(explicit_candidates);
    }

    let mut candidates = Vec::new();
    if let Some(value) = adapter_ctx.project_root.as_ref() {
        candidates.push(value.to_string_lossy().to_string());
    }
    if let Some(value) = adapter_ctx.project_cwd.as_ref() {
        candidates.push(value.to_string_lossy().to_string());
    }
    normalized_local_session_project_roots(candidates)
}

fn normalized_local_session_project_roots(candidates: Vec<String>) -> Vec<PathBuf> {
    let mut roots: Vec<PathBuf> = Vec::new();
    for candidate in candidates {
        let trimmed = candidate.trim();
        if trimmed.is_empty() {
            continue;
        }
        let path = PathBuf::from(trimmed);
        if !path.is_absolute() {
            continue;
        }
        let normalized = local_session_normalized_path(&path);
        if !roots
            .iter()
            .any(|root| local_session_normalized_path(root) == normalized)
        {
            roots.push(path);
        }
    }
    roots
}

fn local_session_agent_matches(requested_agent: Option<&str>, agent: &str) -> bool {
    requested_agent
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .is_none_or(|requested| {
            requested.eq_ignore_ascii_case(agent)
                || requested.eq_ignore_ascii_case(AgentId::ToolGlobal.as_str())
                || requested.eq_ignore_ascii_case("all")
        })
}

fn push_existing_session_root(
    roots: &mut Vec<LocalSessionRootRequest>,
    path: PathBuf,
    status: &'static str,
    source_kind: &'static str,
) -> bool {
    if path.is_dir() {
        roots.push(LocalSessionRootRequest {
            path,
            status,
            source_kind,
        });
        true
    } else {
        false
    }
}

fn dedupe_local_session_root_requests(roots: &mut Vec<LocalSessionRootRequest>) {
    let mut seen = BTreeSet::new();
    roots.retain(|root| {
        let key = root
            .path
            .canonicalize()
            .unwrap_or_else(|_| root.path.clone())
            .to_string_lossy()
            .to_string();
        seen.insert(key)
    });
}

fn encode_claude_project_session_dir(project: &Path) -> String {
    encode_project_path_session_component(project)
}

fn encode_pi_project_session_dirs(project: &Path) -> Vec<String> {
    let dash_path = encode_project_path_session_component(project);
    let trimmed = dash_path.trim_matches('-');
    let mut candidates = vec![
        dash_path.clone(),
        format!("{dash_path}-"),
        format!("-{trimmed}-"),
        format!("--{trimmed}--"),
    ];
    candidates.sort();
    candidates.dedup();
    candidates
}

fn encode_project_path_session_component(project: &Path) -> String {
    project
        .to_string_lossy()
        .chars()
        .map(|ch| match ch {
            '/' | '\\' | ':' => '-',
            other => other,
        })
        .collect()
}

fn collect_local_session_files(
    root: &Path,
    max_files: usize,
    gap_notes: &mut Vec<String>,
    redactor: &mut PromptRedactor<'_>,
) -> Vec<PathBuf> {
    let mut files = Vec::new();
    let mut directories = vec![root.to_path_buf()];

    while let Some(directory) = directories.pop() {
        let entries = match fs::read_dir(&directory) {
            Ok(entries) => entries,
            Err(error) => {
                gap_notes.push(format!(
                    "{}: {}",
                    redactor.redact(&directory.to_string_lossy()),
                    redactor.redact(&error.to_string())
                ));
                continue;
            }
        };
        for entry in entries.flatten() {
            if files.len() >= max_files {
                gap_notes.push(format!(
                    "Local session preview stopped after {} candidate file(s) for bounded read latency.",
                    max_files
                ));
                return files;
            }
            let path = entry.path();
            let Ok(file_type) = entry.file_type() else {
                continue;
            };
            if file_type.is_dir() {
                directories.push(path);
            } else if file_type.is_file()
                && is_supported_local_session_file(&path)
                && !is_ignored_local_session_file(&path)
            {
                match path.canonicalize() {
                    Ok(canonical) if canonical.starts_with(root) => files.push(canonical),
                    Ok(canonical) => gap_notes.push(format!(
                        "{}: skipped because it resolves outside the authorized root.",
                        redactor.redact(&canonical.to_string_lossy())
                    )),
                    Err(error) => gap_notes.push(format!(
                        "{}: {}",
                        redactor.redact(&path.to_string_lossy()),
                        redactor.redact(&error.to_string())
                    )),
                }
            }
        }
    }

    files
}

fn is_supported_local_session_file(path: &Path) -> bool {
    path.extension()
        .and_then(|extension| extension.to_str())
        .map(|extension| {
            matches!(
                extension.to_ascii_lowercase().as_str(),
                "jsonl" | "json" | "txt" | "log"
            )
        })
        .unwrap_or(false)
}

fn is_ignored_local_session_file(path: &Path) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".meta.json"))
    {
        return true;
    }
    path.components().any(|component| {
        component.as_os_str().to_str().is_some_and(|name| {
            matches!(
                name,
                "memory" | "subagents" | "message" | "part" | "tool-results"
            )
        })
    })
}

fn local_session_preview_row(
    path: &Path,
    root: &Path,
    options: LocalSessionPreviewRowOptions<'_>,
    io: &mut LocalSessionIoContext,
    redactor: &mut PromptRedactor<'_>,
) -> Result<Option<LocalSessionPreviewEntry>, ServiceError> {
    if !path.starts_with(root) {
        return Ok(None);
    }
    let file_content = read_local_session_file_content(path, io)?;
    if file_content.is_empty() {
        return Ok(None);
    }
    let content = enrich_local_session_content(path, root, &file_content);
    let mut metadata = local_session_parsed_metadata(path, &file_content, &content);
    if let Some(project_root) =
        local_session_storage_project_root(path, root, options.project_filter_roots)
    {
        metadata.project_root = Some(project_root);
    }
    if metadata.title.is_none() {
        metadata.title = metadata
            .session_id
            .as_deref()
            .and_then(|session_id| codex_session_index_title(path, session_id));
    }
    if !local_session_matches_scope(
        options.scope,
        options.project_filter_roots,
        metadata.project_root.as_deref(),
    ) {
        return Ok(None);
    }
    let excerpt = truncate_chars(
        &redact_local_session_content(redactor, &content),
        options.max_excerpt_chars,
    );
    let excerpt_char_count = excerpt.chars().count();
    let content_hash = trace_content_hash(&content);
    let short_hash = content_hash.chars().take(12).collect::<String>();
    let row_id = local_session_row_id(path);
    let redacted_path = redactor.redact(&path.to_string_lossy());
    let title = metadata
        .title
        .as_deref()
        .map(|title| truncate_chars(&redactor.redact(title), 120))
        .filter(|title| !title.trim().is_empty())
        .unwrap_or_else(|| local_session_title(path, &short_hash));
    if !local_session_matches_search(
        options.search,
        &title,
        metadata.project_root.as_deref(),
        &redacted_path,
        &excerpt,
    ) {
        return Ok(None);
    }
    let skill_mentions = detect_local_session_skill_mentions(
        &content,
        options.skill_matchers,
        &format!("session.content_hash:{short_hash}"),
    );
    let content_items = local_session_content_items(
        &content,
        &short_hash,
        options.max_excerpt_chars,
        &skill_mentions,
        redactor,
    );
    let modified_at = local_session_modified_at(path);
    let (started_at, ended_at) = local_session_time_bounds(&content, &content_items, modified_at);
    let metrics = local_session_metrics(&content, &content_items);
    let agent = options
        .requested_agent
        .map(str::trim)
        .filter(|agent| !agent.is_empty())
        .map(|agent| truncate_chars(&redactor.redact(agent), 80))
        .or_else(|| infer_local_session_agent(path));
    let redacted_project_root = metadata
        .project_root
        .as_deref()
        .map(|project| truncate_chars(&redactor.redact(project), 180));
    Ok(Some(LocalSessionPreviewEntry {
        row: LocalSessionPreviewRow {
            id: row_id,
            title,
            source_kind: options.source_kind.to_string(),
            scope: options.scope.as_str().to_string(),
            agent,
            project_root: redacted_project_root,
            redacted_path: redacted_path.clone(),
            modified_at,
            started_at,
            ended_at,
            excerpt,
            excerpt_char_count,
            user_message_count: metrics.user_message_count,
            total_message_count: metrics.total_message_count,
            tool_call_count: metrics.tool_call_count,
            skill_call_count: metrics.skill_call_count,
            content_hash,
            evidence_refs: vec![
                format!("session.path:{redacted_path}"),
                format!("session.content_hash:{short_hash}"),
            ],
            content_items,
        },
        skill_mentions,
    }))
}

fn read_local_session_file_content(
    path: &Path,
    io: &mut LocalSessionIoContext,
) -> Result<String, ServiceError> {
    let bounded = read_bounded_text(
        path,
        BoundedReadSpec {
            head_bytes: io.limits.primary_head_bytes,
            tail_bytes: io.limits.primary_tail_bytes,
            line_fragment_bytes: io.limits.max_line_fragment_bytes,
        },
        &mut io.budget,
    )?;
    debug_assert!(bounded.bytes_read <= io.limits.max_preview_read_bytes);
    Ok(compact_bounded_local_session_content(
        &bounded,
        io.limits.max_line_fragment_bytes,
    ))
}

fn compact_bounded_local_session_content(
    bounded: &BoundedText,
    max_line_fragment_bytes: usize,
) -> String {
    if bounded.head.is_empty() && bounded.tail.is_empty() {
        return String::new();
    }
    if !bounded.truncated {
        let mut contiguous = String::with_capacity(bounded.head.len() + bounded.tail.len());
        contiguous.push_str(&bounded.head);
        contiguous.push_str(&bounded.tail);
        return compact_local_session_records(&contiguous, max_line_fragment_bytes);
    }

    let complete_head_end = bounded.head.rfind('\n').map_or(0, |newline| newline + 1);
    if complete_head_end == 0 && !bounded.tail.contains('\n') {
        let mut fields = supported_scalar_fragment(&bounded.head, max_line_fragment_bytes);
        for (key, value) in supported_scalar_fragment(&bounded.tail, max_line_fragment_bytes) {
            fields.insert(key, value);
        }
        let mut compacted = String::new();
        append_supported_scalar_fields(&mut compacted, fields);
        if compacted.is_empty() {
            return compacted;
        }
        let mut content = String::from("{\"type\":\"skills-copilot-truncation-marker\"}\n");
        content.push_str(&compacted);
        return content;
    }

    let mut head =
        compact_local_session_records(&bounded.head[..complete_head_end], max_line_fragment_bytes);
    append_supported_scalar_fragment(
        &mut head,
        &bounded.head[complete_head_end..],
        max_line_fragment_bytes,
    );
    let tail = compact_local_session_records(&bounded.tail, max_line_fragment_bytes);
    if head.is_empty() && tail.is_empty() {
        return String::new();
    }

    let mut content = head;
    content.push_str("{\"type\":\"skills-copilot-truncation-marker\"}\n");
    content.push_str(&tail);
    content
}

fn compact_local_session_records(text: &str, max_line_fragment_bytes: usize) -> String {
    let mut content = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || should_skip_local_session_sidecar_line(trimmed) {
            continue;
        }
        if trimmed.len() > max_line_fragment_bytes {
            append_supported_scalar_fragment(&mut content, trimmed, max_line_fragment_bytes);
            continue;
        }
        if looks_like_json_fragment(trimmed) && serde_json::from_str::<Value>(trimmed).is_err() {
            append_supported_scalar_fragment(&mut content, trimmed, max_line_fragment_bytes);
            continue;
        }
        content.push_str(trimmed);
        content.push('\n');
    }
    content
}

fn append_supported_scalar_fragment(content: &mut String, fragment: &str, max_bytes: usize) {
    append_supported_scalar_fields(content, supported_scalar_fragment(fragment, max_bytes));
}

fn supported_scalar_fragment(fragment: &str, max_bytes: usize) -> serde_json::Map<String, Value> {
    let mut fields = serde_json::Map::new();
    for key in [
        "type",
        "role",
        "text",
        "content",
        "title",
        "aiTitle",
        "timestamp",
        "sessionId",
        "id",
        "cwd",
    ] {
        if let Some(value) = last_json_scalar_for_key(fragment, key, max_bytes) {
            fields.insert(key.to_string(), value);
        }
    }
    fields
}

fn append_supported_scalar_fields(content: &mut String, fields: serde_json::Map<String, Value>) {
    if fields.is_empty() {
        return;
    }
    let compacted = Value::Object(fields).to_string();
    if should_skip_local_session_sidecar_line(&compacted) {
        return;
    }
    content.push_str(&compacted);
    content.push('\n');
}

fn last_json_scalar_for_key(fragment: &str, key: &str, max_bytes: usize) -> Option<Value> {
    let needle = format!("\"{key}\"");
    let mut latest = None;
    for (offset, _) in fragment.match_indices(&needle) {
        let mut cursor = offset + needle.len();
        cursor += fragment[cursor..]
            .bytes()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count();
        if fragment.as_bytes().get(cursor) != Some(&b':') {
            continue;
        }
        cursor += 1;
        cursor += fragment[cursor..]
            .bytes()
            .take_while(|byte| byte.is_ascii_whitespace())
            .count();
        let mut values =
            serde_json::Deserializer::from_str(&fragment[cursor..]).into_iter::<Value>();
        let Some(Ok(value)) = values.next() else {
            continue;
        };
        let value = match value {
            Value::String(text) => Value::String(truncate_utf8_bytes(&text, max_bytes)),
            Value::Number(_) | Value::Bool(_) | Value::Null => value,
            Value::Array(_) | Value::Object(_) => continue,
        };
        latest = Some(value);
    }
    latest
}

fn truncate_utf8_bytes(text: &str, max_bytes: usize) -> String {
    if text.len() <= max_bytes {
        return text.to_string();
    }
    let mut end = max_bytes.min(text.len());
    while end > 0 && !text.is_char_boundary(end) {
        end -= 1;
    }
    text[..end].to_string()
}

fn looks_like_json_fragment(text: &str) -> bool {
    text.starts_with('{') || text.starts_with('[') || text.ends_with('}') || text.ends_with(']')
}

fn should_skip_local_session_sidecar_line(line: &str) -> bool {
    let prefix = line.chars().take(4_096).collect::<String>();
    [
        "attachment",
        "file-history-snapshot",
        "last-prompt",
        "mode",
        "permission-mode",
        "queue-operation",
    ]
    .iter()
    .any(|record_type| {
        prefix.contains(&format!("\"type\":\"{record_type}\""))
            || prefix.contains(&format!("\"type\": \"{record_type}\""))
    })
}

fn local_session_row_id(path: &Path) -> String {
    let path_key = local_session_normalized_path(path);
    let path_hash = trace_content_hash(&path_key)
        .chars()
        .take(16)
        .collect::<String>();
    format!("local-session-{path_hash}")
}

fn enrich_local_session_content(path: &Path, root: &Path, file_content: &str) -> String {
    let Some(agent) = infer_local_session_agent(path) else {
        return file_content.to_string();
    };
    if agent != AgentId::Opencode.as_str() && agent != "opencode" {
        return file_content.to_string();
    }
    let Ok(value) = serde_json::from_str::<Value>(file_content) else {
        return file_content.to_string();
    };
    let Some(session_id) = value.get("id").and_then(Value::as_str) else {
        return file_content.to_string();
    };
    let Some(storage_root) = opencode_storage_root(path) else {
        return file_content.to_string();
    };

    let mut chunks = vec![file_content.to_string()];
    let message_root = storage_root.join("message").join(session_id);
    if let Some(message_root) = authorized_local_session_extra_dir(root, &message_root) {
        let Ok(entries) = fs::read_dir(&message_root) else {
            return chunks.join("\n");
        };
        let mut message_paths = entries
            .flatten()
            .filter_map(|entry| {
                entry
                    .file_type()
                    .ok()
                    .filter(|file_type| file_type.is_file())
                    .map(|_| entry.path())
            })
            .collect::<Vec<_>>();
        message_paths.sort();
        for message_path in message_paths.into_iter().take(240) {
            let Some(message_path) = authorized_local_session_extra_file(root, &message_path)
            else {
                continue;
            };
            if let Ok(message) = fs::read_to_string(&message_path) {
                chunks.push(message.clone());
                if let Ok(message_value) = serde_json::from_str::<Value>(&message) {
                    if let Some(message_id) = message_value.get("id").and_then(Value::as_str) {
                        append_opencode_parts(&storage_root, root, message_id, &mut chunks);
                    }
                }
            }
        }
    }
    chunks.join("\n")
}

fn append_opencode_parts(
    storage_root: &Path,
    root: &Path,
    message_id: &str,
    chunks: &mut Vec<String>,
) {
    let part_root = storage_root.join("part").join(message_id);
    let Some(part_root) = authorized_local_session_extra_dir(root, &part_root) else {
        return;
    };
    let Ok(entries) = fs::read_dir(&part_root) else {
        return;
    };
    let mut part_paths = entries
        .flatten()
        .filter_map(|entry| {
            entry
                .file_type()
                .ok()
                .filter(|file_type| file_type.is_file())
                .map(|_| entry.path())
        })
        .collect::<Vec<_>>();
    part_paths.sort();
    for part_path in part_paths.into_iter().take(240) {
        let Some(part_path) = authorized_local_session_extra_file(root, &part_path) else {
            continue;
        };
        if let Ok(part) = fs::read_to_string(part_path) {
            chunks.push(part);
        }
    }
}

fn authorized_local_session_extra_dir(root: &Path, path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    canonical.starts_with(root).then_some(canonical)
}

fn authorized_local_session_extra_file(root: &Path, path: &Path) -> Option<PathBuf> {
    let canonical = path.canonicalize().ok()?;
    canonical.starts_with(root).then_some(canonical)
}

fn opencode_storage_root(path: &Path) -> Option<PathBuf> {
    let mut current = path.parent();
    while let Some(directory) = current {
        if directory.file_name().and_then(|name| name.to_str()) == Some("storage") {
            return Some(directory.to_path_buf());
        }
        current = directory.parent();
    }
    None
}

fn local_session_parsed_metadata(
    path: &Path,
    file_content: &str,
    content: &str,
) -> LocalSessionParsedMetadata {
    let mut metadata = LocalSessionParsedMetadata::default();
    let mut parsed_json = false;
    if path
        .file_stem()
        .and_then(|name| name.to_str())
        .is_some_and(|stem| stem.starts_with("ses_"))
    {
        metadata.session_id = path
            .file_stem()
            .and_then(|name| name.to_str())
            .map(str::to_string);
    }

    for text in [file_content, content] {
        for line in text.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                if metadata.title.is_none() && !parsed_json {
                    metadata.title = local_session_text_title_candidate(line);
                }
                continue;
            };
            parsed_json = true;
            merge_local_session_metadata(&value, &mut metadata);
        }
    }
    if metadata.title.is_none() && !parsed_json {
        metadata.title = local_session_text_title_candidate(content);
    }
    metadata
}

fn merge_local_session_metadata(value: &Value, metadata: &mut LocalSessionParsedMetadata) {
    match value {
        Value::Array(items) => {
            for item in items {
                merge_local_session_metadata(item, metadata);
            }
        }
        Value::Object(map) => {
            if let Some(title) = json_session_title_candidate(map) {
                let is_ai_title = map.get("type").and_then(Value::as_str) == Some("ai-title");
                if metadata.title.is_none() || is_ai_title {
                    metadata.title = Some(title);
                }
            }
            if metadata.project_root.is_none() {
                metadata.project_root = json_session_project_candidate(map);
            }
            if metadata.session_id.is_none() {
                metadata.session_id = json_session_id_candidate(map);
            }
            for (key, nested) in map {
                if matches!(
                    key.as_str(),
                    "content" | "text" | "arguments" | "output" | "description"
                ) {
                    continue;
                }
                merge_local_session_metadata(nested, metadata);
            }
        }
        _ => {}
    }
}

fn json_session_title_candidate(map: &serde_json::Map<String, Value>) -> Option<String> {
    if map.get("type").and_then(Value::as_str) == Some("ai-title") {
        return map
            .get("aiTitle")
            .and_then(Value::as_str)
            .and_then(local_session_text_title_candidate);
    }
    for key in ["aiTitle", "title", "display", "task", "thread_name"] {
        if let Some(title) = map.get(key).and_then(Value::as_str) {
            if let Some(title) = local_session_text_title_candidate(title) {
                return Some(title);
            }
        }
    }
    if let Some(payload) = map.get("payload").and_then(Value::as_object) {
        if payload.get("type").and_then(Value::as_str) == Some("user_message") {
            if let Some(message) = payload.get("message").and_then(Value::as_str) {
                return local_session_text_title_candidate(message);
            }
        }
    }
    let role = json_session_role(map);
    if role.is_some_and(|role| matches!(role, "user" | "human" | "customer")) {
        if let Some(text) = json_session_text_for_kind(map, "user_message") {
            return local_session_text_title_candidate(&text);
        }
    }
    None
}

fn local_session_text_title_candidate(text: &str) -> Option<String> {
    let candidates = normalize_local_session_title_candidates(text);
    if candidates
        .first()
        .is_some_and(|candidate| is_internal_local_session_title_block(candidate))
    {
        return None;
    }
    for candidate in candidates {
        if !candidate.is_empty() && !is_unhelpful_local_session_title(&candidate) {
            return Some(truncate_chars(&candidate, 120));
        }
    }
    None
}

fn normalize_local_session_title_candidates(text: &str) -> Vec<String> {
    let mut value = text.trim().replace('\r', "\n");
    for prefix in [
        "<command-message>",
        "</command-message>",
        "<command-name>",
        "</command-name>",
        "<command-args>",
        "</command-args>",
    ] {
        value = value.replace(prefix, " ");
    }
    if let Some(stripped) = value.strip_prefix("Task:") {
        value = stripped.trim().to_string();
    }
    value
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .map(|line| line.trim_matches(['"', '\'', '`', ' ']).to_string())
        .filter(|line| !line.is_empty())
        .collect()
}

fn is_internal_local_session_title_block(value: &str) -> bool {
    let lower = value.trim().to_ascii_lowercase();
    lower.starts_with("# agents.md instructions")
        || lower.starts_with("<permissions instructions>")
        || lower.starts_with("<environment_context>")
        || lower.starts_with("<local-command-caveat>")
        || lower.starts_with("<command-")
        || lower.starts_with("<skill name=")
        || lower.starts_with("<turn_")
        || lower.starts_with("you are a delegated subagent")
        || lower.starts_with("you are codex")
        || lower.starts_with("shared instruction entrypoint")
}

fn is_unhelpful_local_session_title(value: &str) -> bool {
    let lower = value.to_ascii_lowercase();
    let trimmed = value.trim();
    lower.starts_with("# agents.md instructions")
        || lower.starts_with("<permissions instructions>")
        || lower.starts_with("<environment_context>")
        || lower.starts_with("<local-command-caveat>")
        || lower.starts_with("<command-")
        || lower.starts_with("<skill name=")
        || lower.starts_with("<turn_")
        || lower.starts_with("you are a delegated subagent")
        || lower.starts_with("you are codex")
        || lower.starts_with("shared instruction entrypoint")
        || lower == "normal"
        || lower == "head"
        || lower == "main"
        || lower == "null"
        || lower == "clear"
        || lower == "cls"
        || is_image_placeholder_local_session_title(trimmed)
        || trimmed.starts_with("$HOME")
        || trimmed.starts_with('/')
        || is_version_like_local_session_title(trimmed)
}

fn is_image_placeholder_local_session_title(value: &str) -> bool {
    let trimmed = value.trim();
    if !trimmed.starts_with("[Image #") {
        return false;
    }
    let remainder = trimmed
        .replace("[Image #", "")
        .replace(']', "")
        .replace(char::is_whitespace, "");
    !remainder.is_empty()
        && remainder
            .chars()
            .all(|character| character.is_ascii_digit())
}

fn is_version_like_local_session_title(value: &str) -> bool {
    let value = value.trim();
    value
        .chars()
        .next()
        .is_some_and(|character| character.is_ascii_digit())
        && value.contains('.')
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '.' | '-'))
}

fn is_internal_local_session_message(kind: &str, text: &str) -> bool {
    matches!(kind, "user_message" | "agent_reply")
        && local_session_text_title_candidate(text).is_none()
}

fn local_session_storage_project_root(
    path: &Path,
    root: &Path,
    project_filter_roots: &[PathBuf],
) -> Option<String> {
    if project_filter_roots.is_empty() {
        return None;
    }
    let root_text = root.to_string_lossy();
    let path_text = path.to_string_lossy();
    for project in project_filter_roots {
        let project_text = local_session_normalized_path(project);
        let claude_marker = format!(
            "/.claude/projects/{}",
            encode_claude_project_session_dir(project)
        );
        if path_or_root_contains_session_marker(&path_text, &root_text, &claude_marker) {
            return Some(project_text);
        }
        for encoded in encode_pi_project_session_dirs(project) {
            let pi_marker = format!("/.pi/agent/sessions/{encoded}");
            if path_or_root_contains_session_marker(&path_text, &root_text, &pi_marker) {
                return Some(project_text);
            }
        }
    }
    None
}

fn path_or_root_contains_session_marker(path_text: &str, root_text: &str, marker: &str) -> bool {
    [path_text, root_text]
        .into_iter()
        .map(|value| value.replace('\\', "/"))
        .any(|value| value.ends_with(marker) || value.contains(&format!("{marker}/")))
}

fn codex_session_index_title(path: &Path, session_id: &str) -> Option<String> {
    let codex_root = local_session_agent_store_root(path, ".codex")?;
    for index_file_name in ["session_index.jsonl", "history.jsonl"] {
        let index_path = codex_root.join(index_file_name);
        let Ok(index_content) = fs::read_to_string(index_path) else {
            continue;
        };
        for line in index_content.lines().filter(|line| !line.trim().is_empty()) {
            let Ok(value) = serde_json::from_str::<Value>(line) else {
                continue;
            };
            let Some(map) = value.as_object() else {
                continue;
            };
            let row_id = map
                .get("id")
                .or_else(|| map.get("session_id"))
                .and_then(Value::as_str);
            if row_id != Some(session_id) {
                continue;
            }
            for key in ["thread_name", "title", "text"] {
                if let Some(title) = map.get(key).and_then(Value::as_str) {
                    if let Some(title) = local_session_text_title_candidate(title) {
                        return Some(title);
                    }
                }
            }
        }
    }
    None
}

fn local_session_agent_store_root(path: &Path, directory_name: &str) -> Option<PathBuf> {
    path.ancestors()
        .find(|ancestor| {
            ancestor.file_name().and_then(|name| name.to_str()) == Some(directory_name)
        })
        .map(Path::to_path_buf)
}

fn json_session_project_candidate(map: &serde_json::Map<String, Value>) -> Option<String> {
    for key in [
        "cwd",
        "current_cwd",
        "current_dir",
        "directory",
        "worktree",
        "workspace",
        "project",
        "projectRoot",
        "project_root",
    ] {
        if let Some(value) = map.get(key).and_then(Value::as_str) {
            if Path::new(value).is_absolute() {
                return Some(value.to_string());
            }
        }
    }
    None
}

fn json_session_id_candidate(map: &serde_json::Map<String, Value>) -> Option<String> {
    for key in ["sessionId", "session_id", "sessionID", "id"] {
        if let Some(value) = map.get(key).and_then(Value::as_str) {
            if !value.trim().is_empty() {
                return Some(value.trim().to_string());
            }
        }
    }
    None
}

fn local_session_matches_scope(
    scope: LocalSessionScope,
    project_filter_roots: &[PathBuf],
    session_project_root: Option<&str>,
) -> bool {
    if scope == LocalSessionScope::All {
        return true;
    }
    let Some(session_project_root) = session_project_root else {
        return false;
    };
    if project_filter_roots.is_empty() {
        return false;
    }
    let session_path = PathBuf::from(session_project_root);
    project_filter_roots
        .iter()
        .any(|project| local_session_paths_match(project, &session_path))
}

fn local_session_paths_match(project: &Path, session_path: &Path) -> bool {
    let left = local_session_normalized_path(project);
    let right = local_session_normalized_path(session_path);
    left == right || right.starts_with(&(left + "/"))
}

fn local_session_normalized_path(path: &Path) -> String {
    let normalized = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .replace('\\', "/");
    normalized
        .strip_prefix("//?/")
        .unwrap_or(&normalized)
        .trim_end_matches('/')
        .to_string()
}

fn local_session_matches_search(
    search: Option<&str>,
    title: &str,
    project_root: Option<&str>,
    redacted_path: &str,
    excerpt: &str,
) -> bool {
    let Some(search) = search else {
        return true;
    };
    let search = search.trim();
    if search.is_empty() {
        return true;
    }
    title.to_ascii_lowercase().contains(search)
        || project_root.is_some_and(|project| project.to_ascii_lowercase().contains(search))
        || redacted_path.to_ascii_lowercase().contains(search)
        || excerpt.to_ascii_lowercase().contains(search)
}

fn push_session_skill_needle(needles: &mut Vec<String>, value: &str) {
    let normalized = value.trim().to_ascii_lowercase();
    if normalized.len() < 3 {
        return;
    }
    if needles.iter().any(|needle| needle == &normalized) {
        return;
    }
    needles.push(normalized);
}

fn detect_local_session_skill_mentions(
    content: &str,
    skill_matchers: &[LocalSessionSkillMatcher],
    evidence_ref: &str,
) -> Vec<LocalSessionSkillMention> {
    if skill_matchers.is_empty() {
        return Vec::new();
    }
    let invocations = extract_skill_invocation_names(content);
    if invocations.is_empty() {
        return Vec::new();
    }
    skill_matchers
        .iter()
        .filter_map(|matcher| {
            let matched_invocations = invocations
                .iter()
                .filter(|invocation| {
                    matcher
                        .needles
                        .iter()
                        .any(|needle| invocation.as_str() == needle)
                })
                .cloned()
                .collect::<Vec<_>>();
            let count = matched_invocations.len();
            (count > 0).then(|| LocalSessionSkillMention {
                skill_id: matcher.skill_id.clone(),
                skill_name: matcher.skill_name.clone(),
                agent: matcher.agent.clone(),
                count,
                matched_invocations,
                evidence_ref: evidence_ref.to_string(),
            })
        })
        .collect()
}

fn extract_skill_invocation_names(text: &str) -> Vec<String> {
    let lower = text.to_ascii_lowercase();
    let mut names = Vec::new();
    for pattern in ["/skill:", "/skill ", "skill:"] {
        let mut start = 0usize;
        while let Some(relative) = lower[start..].find(pattern) {
            let pattern_start = start + relative;
            let offset = pattern_start + pattern.len();
            if !is_skill_invocation_left_boundary(&lower, pattern_start) {
                start = next_skill_invocation_search_start(&lower, offset);
                continue;
            }
            if pattern == "skill:"
                && pattern_start > 0
                && lower.as_bytes().get(pattern_start - 1) == Some(&b'/')
            {
                start = next_skill_invocation_search_start(&lower, offset);
                continue;
            }
            let name = read_skill_invocation_name(&lower[offset..]);
            if !name.is_empty() {
                names.push(name);
            }
            start = next_skill_invocation_search_start(&lower, offset);
            if start >= lower.len() {
                break;
            }
        }
    }
    names.sort();
    names
}

fn next_skill_invocation_search_start(value: &str, offset: usize) -> usize {
    if offset >= value.len() {
        return value.len();
    }
    value[offset..]
        .char_indices()
        .nth(1)
        .map(|(relative, _)| offset + relative)
        .unwrap_or(value.len())
}

fn is_skill_invocation_left_boundary(value: &str, pattern_start: usize) -> bool {
    if pattern_start == 0 {
        return true;
    }
    value[..pattern_start]
        .chars()
        .next_back()
        .is_none_or(|ch| !(ch.is_ascii_alphanumeric() || matches!(ch, '-' | '_' | '.')))
}

fn read_skill_invocation_name(value: &str) -> String {
    let mut name = String::new();
    for character in value.trim_start().chars() {
        if character.is_ascii_alphanumeric() || matches!(character, '-' | '_' | ':' | '.') {
            name.push(character);
        } else {
            break;
        }
    }
    name.trim_matches(|character: char| {
        matches!(
            character,
            '-' | '_' | ':' | '.' | '/' | '\\' | '"' | '\'' | '`'
        )
    })
    .to_string()
}

fn update_local_session_skill_usage(
    usage: &mut BTreeMap<String, LocalSessionSkillUsageAccumulator>,
    entry: &LocalSessionPreviewEntry,
) {
    for mention in &entry.skill_mentions {
        let accumulator = usage.entry(mention.skill_id.clone()).or_insert_with(|| {
            LocalSessionSkillUsageAccumulator {
                skill_id: mention.skill_id.clone(),
                skill_name: mention.skill_name.clone(),
                agent: mention.agent.clone(),
                ..Default::default()
            }
        });
        accumulator.call_count += mention.count;
        accumulator.session_count += 1;
        accumulator.latest_modified_at =
            max_optional_millis(accumulator.latest_modified_at, entry.row.modified_at);
        if !accumulator
            .evidence_refs
            .iter()
            .any(|reference| reference == &mention.evidence_ref)
            && accumulator.evidence_refs.len() < 6
        {
            accumulator.evidence_refs.push(mention.evidence_ref.clone());
        }
    }
}

fn max_optional_millis(left: Option<i64>, right: Option<i64>) -> Option<i64> {
    match (left, right) {
        (Some(left), Some(right)) => Some(left.max(right)),
        (Some(left), None) => Some(left),
        (None, Some(right)) => Some(right),
        (None, None) => None,
    }
}

fn local_session_skill_usage_rows(
    usage: BTreeMap<String, LocalSessionSkillUsageAccumulator>,
    limit: usize,
) -> Vec<LocalSessionSkillUsageRow> {
    let mut rows = usage
        .into_values()
        .map(|row| LocalSessionSkillUsageRow {
            skill_id: row.skill_id,
            skill_name: row.skill_name,
            agent: row.agent,
            call_count: row.call_count,
            session_count: row.session_count,
            latest_modified_at: row.latest_modified_at,
            evidence_refs: row.evidence_refs,
        })
        .collect::<Vec<_>>();
    rows.sort_by(|left, right| {
        right
            .call_count
            .cmp(&left.call_count)
            .then_with(|| right.session_count.cmp(&left.session_count))
            .then_with(|| right.latest_modified_at.cmp(&left.latest_modified_at))
            .then_with(|| left.skill_name.cmp(&right.skill_name))
    });
    rows.truncate(limit);
    rows
}

fn local_session_content_items(
    content: &str,
    short_hash: &str,
    max_item_chars: usize,
    skill_mentions: &[LocalSessionSkillMention],
    redactor: &mut PromptRedactor<'_>,
) -> Vec<LocalSessionContentItem> {
    const MAX_SESSION_CONTENT_ITEMS: usize = 240;
    let mut drafts = Vec::new();

    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match serde_json::from_str::<Value>(trimmed) {
            Ok(value) => {
                let timestamp = json_session_timestamp_millis(&value);
                collect_json_session_content_drafts(&value, timestamp, &mut drafts);
            }
            Err(_) => collect_text_session_content_drafts(trimmed, None, &mut drafts),
        }
        if drafts.len() >= MAX_SESSION_CONTENT_ITEMS {
            break;
        }
    }

    let mut matched_invocations = BTreeSet::new();
    for mention in skill_mentions {
        for invocation in &mention.matched_invocations {
            matched_invocations.insert(invocation.clone());
        }
        let text = if mention.count > 1 {
            format!("{} ({} calls)", mention.skill_name, mention.count)
        } else {
            mention.skill_name.clone()
        };
        drafts.push(LocalSessionContentDraft {
            kind: "skill_call".to_string(),
            title: format!("Skill: {}", mention.skill_name),
            text,
            timestamp: None,
            evidence_refs: vec![mention.evidence_ref.clone()],
        });
    }
    let mut unmatched_invocations = BTreeMap::<String, usize>::new();
    for invocation in extract_skill_invocation_names(content) {
        if matched_invocations.contains(&invocation) {
            continue;
        }
        *unmatched_invocations.entry(invocation).or_default() += 1;
    }
    for (invocation, count) in unmatched_invocations {
        let text = if count > 1 {
            format!("{invocation} ({count} calls)")
        } else {
            invocation.clone()
        };
        drafts.push(LocalSessionContentDraft {
            kind: "skill_call".to_string(),
            title: format!("Skill: {invocation}"),
            text,
            timestamp: None,
            evidence_refs: vec![format!("session.content_hash:{short_hash}")],
        });
    }

    if drafts.is_empty() {
        drafts.push(LocalSessionContentDraft {
            kind: "agent_reply".to_string(),
            title: "Session excerpt".to_string(),
            text: content.to_string(),
            timestamp: None,
            evidence_refs: vec![format!("session.content_hash:{short_hash}")],
        });
    }

    drafts
        .into_iter()
        .take(MAX_SESSION_CONTENT_ITEMS)
        .enumerate()
        .map(|(index, draft)| {
            let redacted = truncate_chars(
                &redact_local_session_content(redactor, &draft.text),
                max_item_chars,
            );
            LocalSessionContentItem {
                id: format!("session-item-{short_hash}-{index}"),
                kind: draft.kind,
                title: truncate_chars(&redactor.redact(&draft.title), 120),
                char_count: redacted.chars().count(),
                text: redacted,
                timestamp: draft.timestamp,
                evidence_refs: draft.evidence_refs,
            }
        })
        .collect()
}

fn local_session_time_bounds(
    content: &str,
    content_items: &[LocalSessionContentItem],
    fallback_timestamp: Option<i64>,
) -> (Option<i64>, Option<i64>) {
    let mut bounds = LocalSessionTimeBounds::default();
    for item in content_items {
        bounds.push(item.timestamp);
    }

    if bounds.started_at.is_none() || bounds.ended_at.is_none() {
        for line in content.lines() {
            let trimmed = line.trim();
            if trimmed.is_empty() {
                continue;
            }
            if let Ok(value) = serde_json::from_str::<Value>(trimmed) {
                bounds.push(json_session_timestamp_millis(&value));
            }
        }
    }

    let started_at = bounds.started_at.or(fallback_timestamp);
    let ended_at = bounds.ended_at.or(started_at).or(fallback_timestamp);
    (started_at, ended_at)
}

fn json_session_timestamp_millis(value: &Value) -> Option<i64> {
    let Value::Object(map) = value else {
        return json_session_timestamp_value_millis(value);
    };
    for key in [
        "timestamp",
        "created_at",
        "createdAt",
        "updated_at",
        "updatedAt",
        "completed_at",
        "completedAt",
        "time",
    ] {
        if let Some(timestamp) = map.get(key).and_then(json_session_timestamp_value_millis) {
            return Some(timestamp);
        }
    }
    None
}

fn json_session_timestamp_value_millis(value: &Value) -> Option<i64> {
    match value {
        Value::String(text) => parse_local_session_timestamp_millis(text),
        Value::Number(number) => number
            .as_i64()
            .and_then(normalize_local_session_epoch_millis)
            .or_else(|| {
                number
                    .as_f64()
                    .and_then(normalize_local_session_epoch_millis_from_float)
            }),
        _ => None,
    }
}

fn parse_local_session_timestamp_millis(value: &str) -> Option<i64> {
    let trimmed = value.trim();
    if trimmed.is_empty() {
        return None;
    }
    if let Ok(epoch) = trimmed.parse::<i64>() {
        return normalize_local_session_epoch_millis(epoch);
    }
    if let Ok(epoch) = trimmed.parse::<f64>() {
        return normalize_local_session_epoch_millis_from_float(epoch);
    }
    let parsed = OffsetDateTime::parse(trimmed, &Rfc3339).ok()?;
    let millis = parsed.unix_timestamp_nanos() / 1_000_000;
    i64::try_from(millis).ok()
}

fn normalize_local_session_epoch_millis(value: i64) -> Option<i64> {
    let magnitude = value.checked_abs().unwrap_or(i64::MAX);
    if magnitude >= 10_000_000_000 {
        Some(value)
    } else {
        value.checked_mul(1_000)
    }
}

fn normalize_local_session_epoch_millis_from_float(value: f64) -> Option<i64> {
    if !value.is_finite() {
        return None;
    }
    let millis = if value.abs() >= 10_000_000_000.0 {
        value
    } else {
        value * 1_000.0
    };
    if millis < i64::MIN as f64 || millis > i64::MAX as f64 {
        return None;
    }
    Some(millis.round() as i64)
}

fn redact_local_session_content(redactor: &mut PromptRedactor<'_>, value: &str) -> String {
    let owner_redacted = redact_unix_listing_owners(value);
    redactor.redact(&owner_redacted)
}

fn redact_unix_listing_owners(value: &str) -> String {
    value
        .lines()
        .map(redact_unix_listing_owner_line_with_escaped_newlines)
        .collect::<Vec<_>>()
        .join("\n")
}

fn redact_unix_listing_owner_line_with_escaped_newlines(line: &str) -> String {
    line.split("\\n")
        .map(redact_unix_listing_owner_line)
        .collect::<Vec<_>>()
        .join("\\n")
}

fn redact_unix_listing_owner_line(line: &str) -> String {
    let leading_len = line.len() - line.trim_start().len();
    let leading = &line[..leading_len];
    let tokens = line[leading_len..].split_whitespace().collect::<Vec<_>>();
    if tokens.len() < 4
        || !is_unix_listing_mode(tokens[0])
        || !tokens[1].chars().all(|ch| ch.is_ascii_digit())
    {
        return line.to_string();
    }

    let mut redacted = tokens
        .iter()
        .map(|token| (*token).to_string())
        .collect::<Vec<_>>();
    redacted[2] = "<user>".to_string();
    redacted[3] = "<group>".to_string();
    format!("{leading}{}", redacted.join(" "))
}

fn is_unix_listing_mode(token: &str) -> bool {
    let chars = token.chars().collect::<Vec<_>>();
    if chars.len() < 10 {
        return false;
    }
    if !matches!(chars[0], '-' | 'd' | 'l' | 'b' | 'c' | 'p' | 's') {
        return false;
    }
    chars[1..10]
        .iter()
        .all(|ch| matches!(ch, 'r' | 'w' | 'x' | 's' | 'S' | 't' | 'T' | '-'))
}

fn collect_json_session_content_drafts(
    value: &Value,
    inherited_timestamp: Option<i64>,
    drafts: &mut Vec<LocalSessionContentDraft>,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_json_session_content_drafts(item, inherited_timestamp, drafts);
            }
        }
        Value::Object(map) => {
            let timestamp = json_session_timestamp_millis(value).or(inherited_timestamp);
            let role = json_session_role(map).map(str::to_string);
            let mut pushed_direct_tool = false;
            if let Some(kind) = json_session_content_kind(map, role.as_deref()) {
                let text = json_session_text_for_kind(map, kind);
                if let Some(text) = text {
                    if !text.trim().is_empty() && !is_internal_local_session_message(kind, &text) {
                        let resolved_kind = local_session_content_kind_for_text(kind, &text, map);
                        pushed_direct_tool = resolved_kind == "tool_call";
                        push_local_session_text_drafts(
                            drafts,
                            resolved_kind,
                            json_session_title(map, resolved_kind),
                            text,
                            timestamp,
                            Vec::new(),
                        );
                    }
                }
            }

            if !pushed_direct_tool {
                collect_json_tool_call_drafts(map, timestamp, drafts);
            }

            for (key, nested) in map {
                if matches!(
                    key.as_str(),
                    "author"
                        | "content"
                        | "text"
                        | "message"
                        | "delta"
                        | "tool_calls"
                        | "toolCalls"
                        | "tool_use"
                        | "toolUse"
                        | "function_call"
                        | "parts"
                ) {
                    continue;
                }
                collect_json_session_content_drafts(nested, timestamp, drafts);
            }
        }
        Value::String(text) => {
            collect_text_session_content_drafts(text, inherited_timestamp, drafts)
        }
        _ => {}
    }
}

fn collect_json_tool_call_drafts(
    map: &serde_json::Map<String, Value>,
    timestamp: Option<i64>,
    drafts: &mut Vec<LocalSessionContentDraft>,
) {
    for key in ["content", "parts"] {
        if let Some(nested) = map.get(key) {
            collect_json_tool_part_drafts(nested, timestamp, drafts);
        }
    }
    if let Some(message) = map.get("message").and_then(Value::as_object) {
        for key in ["content", "parts"] {
            if let Some(nested) = message.get(key) {
                collect_json_tool_part_drafts(nested, timestamp, drafts);
            }
        }
    }
    for key in [
        "tool_calls",
        "toolCalls",
        "tool_use",
        "toolUse",
        "function_call",
    ] {
        let Some(value) = map.get(key) else {
            continue;
        };
        match value {
            Value::Array(items) => {
                for item in items {
                    drafts.push(LocalSessionContentDraft {
                        kind: "tool_call".to_string(),
                        title: json_tool_title(item),
                        text: compact_json_session_text(item),
                        timestamp,
                        evidence_refs: Vec::new(),
                    });
                }
            }
            Value::Object(_) | Value::String(_) => {
                drafts.push(LocalSessionContentDraft {
                    kind: "tool_call".to_string(),
                    title: json_tool_title(value),
                    text: compact_json_session_text(value),
                    timestamp,
                    evidence_refs: Vec::new(),
                });
            }
            _ => {}
        }
    }
}

fn collect_json_tool_part_drafts(
    value: &Value,
    inherited_timestamp: Option<i64>,
    drafts: &mut Vec<LocalSessionContentDraft>,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_json_tool_part_drafts(item, inherited_timestamp, drafts);
            }
        }
        Value::Object(map) => {
            let timestamp = json_session_timestamp_millis(value).or(inherited_timestamp);
            if let Some(kind) = map.get("type").and_then(Value::as_str) {
                let normalized = kind.to_ascii_lowercase().replace(['_', '-'], "");
                if is_json_tool_type(&normalized) {
                    drafts.push(LocalSessionContentDraft {
                        kind: "tool_call".to_string(),
                        title: json_tool_title(value),
                        text: json_tool_payload_text(value),
                        timestamp,
                        evidence_refs: Vec::new(),
                    });
                    return;
                }
            }
            for key in ["content", "parts"] {
                if let Some(nested) = map.get(key) {
                    collect_json_tool_part_drafts(nested, timestamp, drafts);
                }
            }
            if let Some(message) = map.get("message").and_then(Value::as_object) {
                for key in ["content", "parts"] {
                    if let Some(nested) = message.get(key) {
                        collect_json_tool_part_drafts(nested, timestamp, drafts);
                    }
                }
            }
        }
        _ => {}
    }
}

fn collect_text_session_content_drafts(
    line: &str,
    timestamp: Option<i64>,
    drafts: &mut Vec<LocalSessionContentDraft>,
) {
    let lower = line.to_ascii_lowercase();
    let (kind, title, text) = if let Some(text) =
        strip_session_line_prefix(line, &["user:", "human:", "用户：", "用户:"])
    {
        ("user_message", "User", text)
    } else if let Some(text) =
        strip_session_line_prefix(line, &["assistant:", "agent:", "助手：", "助手:"])
    {
        ("agent_reply", "Agent", text)
    } else if let Some(text) = strip_session_line_prefix(
        line,
        &["thinking:", "reasoning:", "thought:", "思考：", "思考:"],
    ) {
        ("thinking", "Thinking", text)
    } else if let Some(text) =
        strip_session_line_prefix(line, &["tool:", "function:", "工具：", "工具:"])
    {
        ("tool_call", "Tool", text)
    } else if is_tool_result_text(line)
        || lower.contains("tool_call")
        || lower.contains("tool_use")
        || lower.contains("function_call")
    {
        ("tool_call", "Tool", line)
    } else if !extract_skill_invocation_names(line).is_empty() {
        ("skill_call", "Skill", line)
    } else {
        return;
    };
    push_local_session_text_drafts(
        drafts,
        kind,
        title.to_string(),
        text.trim().to_string(),
        timestamp,
        Vec::new(),
    );
}

fn strip_session_line_prefix<'a>(line: &'a str, prefixes: &[&str]) -> Option<&'a str> {
    let lower = line.to_ascii_lowercase();
    for prefix in prefixes {
        if lower.starts_with(&prefix.to_ascii_lowercase()) {
            return Some(line[prefix.len()..].trim());
        }
    }
    None
}

fn push_local_session_text_drafts(
    drafts: &mut Vec<LocalSessionContentDraft>,
    kind: &str,
    title: String,
    text: String,
    timestamp: Option<i64>,
    evidence_refs: Vec<String>,
) {
    for (segment_kind, segment_text) in split_inline_thinking_segments(kind, &text) {
        drafts.push(LocalSessionContentDraft {
            kind: segment_kind.to_string(),
            title: if segment_kind == kind {
                title.clone()
            } else {
                json_fallback_session_title(segment_kind)
            },
            text: segment_text,
            timestamp,
            evidence_refs: evidence_refs.clone(),
        });
    }
}

fn split_inline_thinking_segments(default_kind: &str, text: &str) -> Vec<(&'static str, String)> {
    let mut segments = Vec::new();
    let mut cursor = 0usize;
    let lower = text.to_ascii_lowercase();

    while let Some(start_offset) = lower[cursor..].find("<think>") {
        let start = cursor + start_offset;
        let before = text[cursor..start].trim();
        if !before.is_empty() {
            segments.push((stable_session_kind(default_kind), before.to_string()));
        }

        let inner_start = start + "<think>".len();
        if let Some(end_offset) = lower[inner_start..].find("</think>") {
            let end = inner_start + end_offset;
            let thinking = text[inner_start..end].trim();
            if !thinking.is_empty() {
                segments.push(("thinking", thinking.to_string()));
            }
            cursor = end + "</think>".len();
        } else {
            let thinking = text[inner_start..].trim();
            if !thinking.is_empty() {
                segments.push(("thinking", thinking.to_string()));
            }
            cursor = text.len();
            break;
        }
    }

    let after = text[cursor..].trim();
    if !after.is_empty() {
        segments.push((stable_session_kind(default_kind), after.to_string()));
    }

    if segments.is_empty() && !text.trim().is_empty() {
        segments.push((stable_session_kind(default_kind), text.trim().to_string()));
    }
    segments
}

fn stable_session_kind(kind: &str) -> &'static str {
    match kind {
        "user_message" => "user_message",
        "agent_reply" => "agent_reply",
        "thinking" => "thinking",
        "tool_call" => "tool_call",
        "skill_call" => "skill_call",
        _ => "agent_reply",
    }
}

fn json_fallback_session_title(kind: &str) -> String {
    match kind {
        "user_message" => "User".to_string(),
        "agent_reply" => "Agent".to_string(),
        "thinking" => "Thinking".to_string(),
        "tool_call" => "Tool".to_string(),
        "skill_call" => "Skill".to_string(),
        _ => "Session".to_string(),
    }
}

fn json_session_content_kind(
    map: &serde_json::Map<String, Value>,
    role: Option<&str>,
) -> Option<&'static str> {
    if let Some(kind) = map.get("type").and_then(Value::as_str) {
        let normalized = kind.to_ascii_lowercase().replace(['_', '-'], "");
        match normalized.as_str() {
            "user" | "human" => return Some("user_message"),
            value if is_json_thinking_type(value) => return Some("thinking"),
            value if is_json_tool_type(value) => return Some("tool_call"),
            _ => {}
        }
    }
    if json_thinking_payload_text(&Value::Object(map.clone())).is_some() {
        return Some("thinking");
    }
    if json_session_text(map)
        .as_deref()
        .is_some_and(is_tool_result_text)
    {
        return Some("tool_call");
    }
    role.and_then(|role| match role.to_ascii_lowercase().as_str() {
        "user" | "human" | "customer" => Some("user_message"),
        "assistant" | "agent" | "model" => Some("agent_reply"),
        "tool" | "function" | "toolresult" | "tool_result" => Some("tool_call"),
        "system" | "developer" => None,
        _ => None,
    })
}

fn json_session_title(map: &serde_json::Map<String, Value>, kind: &str) -> String {
    for key in ["name", "tool_name", "function_name", "title"] {
        if let Some(value) = map.get(key).and_then(Value::as_str) {
            if !value.trim().is_empty() {
                return value.trim().to_string();
            }
        }
    }
    match kind {
        "user_message" => "User".to_string(),
        "agent_reply" => "Agent".to_string(),
        "thinking" => "Thinking".to_string(),
        "tool_call" => "Tool".to_string(),
        "skill_call" => "Skill".to_string(),
        _ => "Session".to_string(),
    }
}

fn json_session_text(map: &serde_json::Map<String, Value>) -> Option<String> {
    for key in [
        "content",
        "text",
        "message",
        "delta",
        "thinking",
        "reasoning",
        "summary",
        "result",
    ] {
        if let Some(value) = map.get(key).and_then(json_value_text) {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn json_session_text_for_kind(map: &serde_json::Map<String, Value>, kind: &str) -> Option<String> {
    if kind == "tool_call" || is_json_tool_object(map) {
        return json_tool_payload_text(&Value::Object(map.clone())).into();
    }
    if kind == "thinking" {
        if let Some(text) = json_thinking_payload_text(&Value::Object(map.clone())) {
            return Some(text);
        }
    }

    for key in [
        "content",
        "text",
        "message",
        "delta",
        "thinking",
        "reasoning",
        "summary",
        "result",
    ] {
        if let Some(value) = map.get(key).and_then(json_non_tool_message_text) {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
    }

    json_session_text(map)
        .filter(|text| !is_tool_result_text(text))
        .filter(|_| !json_session_contains_tool_payload(map))
}

fn json_session_contains_tool_payload(map: &serde_json::Map<String, Value>) -> bool {
    map.values().any(json_value_contains_tool_payload)
}

fn json_value_contains_tool_payload(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(json_value_contains_tool_payload),
        Value::Object(map) => {
            is_json_tool_object(map)
                || [
                    "tool_calls",
                    "toolCalls",
                    "tool_use",
                    "toolUse",
                    "function_call",
                ]
                .iter()
                .any(|key| map.contains_key(*key))
                || map.values().any(json_value_contains_tool_payload)
        }
        _ => false,
    }
}

fn local_session_content_kind_for_text<'a>(
    kind: &'a str,
    text: &str,
    map: &serde_json::Map<String, Value>,
) -> &'a str {
    if kind == "agent_reply"
        && (json_session_has_tool_process_signal(map) || is_agent_process_note_text(text))
    {
        "thinking"
    } else {
        kind
    }
}

fn json_session_has_tool_process_signal(map: &serde_json::Map<String, Value>) -> bool {
    map.values().any(json_value_has_tool_process_signal)
}

fn json_value_has_tool_process_signal(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(json_value_has_tool_process_signal),
        Value::Object(map) => {
            for key in [
                "stop_reason",
                "stopReason",
                "finish_reason",
                "finishReason",
                "finish_details",
                "finishDetails",
            ] {
                if let Some(value) = map.get(key) {
                    if json_value_is_tool_process_signal(value) {
                        return true;
                    }
                }
            }
            map.values().any(json_value_has_tool_process_signal)
        }
        _ => false,
    }
}

fn json_value_is_tool_process_signal(value: &Value) -> bool {
    match value {
        Value::String(text) => is_tool_process_signal_text(text),
        Value::Object(map) => map.values().any(json_value_is_tool_process_signal),
        Value::Array(items) => items.iter().any(json_value_is_tool_process_signal),
        _ => false,
    }
}

fn is_tool_process_signal_text(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase().replace(['_', '-', ' '], "");
    matches!(
        normalized.as_str(),
        "tool"
            | "tooluse"
            | "toolcall"
            | "toolcalls"
            | "functioncall"
            | "functioncalls"
            | "requiresaction"
    )
}

fn is_agent_process_note_text(text: &str) -> bool {
    let trimmed = text.trim();
    if trimmed.is_empty() || trimmed.chars().count() > 320 {
        return false;
    }

    let lower = trimmed.to_ascii_lowercase();
    let english_starters = [
        "i need to",
        "i'll ",
        "i will ",
        "i should ",
        "i'm going to",
        "i apologize",
        "let me ",
        "sorry",
    ];
    let english_tool_terms = [
        "tool",
        "websearch",
        "webfetch",
        "search",
        "research",
        "investigate",
        "load",
        "gather",
    ];
    if english_starters
        .iter()
        .any(|starter| lower.starts_with(starter))
        && english_tool_terms.iter().any(|term| lower.contains(term))
    {
        return true;
    }

    let chinese_starters = ["我需要", "我会", "我先", "我来", "让我", "抱歉"];
    let chinese_tool_terms = ["工具", "搜索", "联网", "调研", "加载", "调用", "查询"];
    chinese_starters
        .iter()
        .any(|starter| trimmed.starts_with(starter))
        && chinese_tool_terms.iter().any(|term| trimmed.contains(term))
}

fn json_thinking_payload_text(value: &Value) -> Option<String> {
    match value {
        Value::Array(items) => {
            let texts = items
                .iter()
                .filter_map(json_thinking_payload_text)
                .collect::<Vec<_>>();
            (!texts.is_empty()).then(|| texts.join("\n"))
        }
        Value::Object(map) => {
            let is_thinking_object = map
                .get("type")
                .and_then(Value::as_str)
                .map(|kind| {
                    is_json_thinking_type(&kind.to_ascii_lowercase().replace(['_', '-'], ""))
                })
                .unwrap_or(false);

            if is_thinking_object {
                for key in [
                    "thinking",
                    "reasoning",
                    "thought",
                    "text",
                    "content",
                    "summary",
                    "message",
                    "delta",
                ] {
                    if let Some(text) = map.get(key).and_then(json_value_text) {
                        if !text.trim().is_empty() {
                            return Some(text);
                        }
                    }
                }
            }

            for key in ["thinking", "reasoning", "thought"] {
                if let Some(text) = map.get(key).and_then(json_value_text) {
                    if !text.trim().is_empty() {
                        return Some(text);
                    }
                }
            }

            for key in ["content", "parts", "message", "delta"] {
                if let Some(text) = map.get(key).and_then(json_thinking_payload_text) {
                    if !text.trim().is_empty() {
                        return Some(text);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn json_non_tool_message_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => (!is_tool_result_text(text)).then(|| text.clone()),
        Value::Array(items) => {
            let texts = items
                .iter()
                .filter_map(json_non_tool_message_text)
                .collect::<Vec<_>>();
            (!texts.is_empty()).then(|| texts.join("\n"))
        }
        Value::Object(map) => {
            if is_json_tool_object(map) {
                return None;
            }
            for key in [
                "text",
                "input",
                "summary",
                "content",
                "message",
                "delta",
                "thinking",
                "reasoning",
            ] {
                if let Some(text) = map.get(key).and_then(json_non_tool_message_text) {
                    if !text.trim().is_empty() {
                        return Some(text);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn is_json_tool_object(map: &serde_json::Map<String, Value>) -> bool {
    if map
        .get("type")
        .and_then(Value::as_str)
        .map(|kind| is_json_tool_type(&kind.to_ascii_lowercase().replace(['_', '-'], "")))
        .unwrap_or(false)
    {
        return true;
    }
    ["tool_use_id", "toolUseId", "tool_call_id", "toolCallId"]
        .iter()
        .any(|key| map.contains_key(*key))
}

fn is_json_thinking_type(normalized: &str) -> bool {
    matches!(
        normalized,
        "thinking" | "thinkingtext" | "reasoning" | "reasoningtext" | "thought"
    )
}

fn is_json_tool_type(normalized: &str) -> bool {
    matches!(
        normalized,
        "toolcall"
            | "tooluse"
            | "functioncall"
            | "toolresult"
            | "tooluseresult"
            | "tooluseerror"
            | "functionresult"
    )
}

fn is_tool_result_text(text: &str) -> bool {
    let lower = text.trim_start().to_ascii_lowercase();
    [
        "<tool_use_error>",
        "</tool_use_error>",
        "<tooluseerror>",
        "</tooluseerror>",
        "<tool_result>",
        "</tool_result>",
        "<toolresult>",
        "</toolresult>",
    ]
    .iter()
    .any(|marker| lower.starts_with(marker) || lower.contains(marker))
}

fn json_tool_payload_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        Value::Object(map) => {
            for key in [
                "content",
                "text",
                "message",
                "result",
                "error",
                "output",
                "input",
                "arguments",
            ] {
                if let Some(text) = map.get(key).and_then(json_value_text) {
                    if !text.trim().is_empty() {
                        return text;
                    }
                }
            }
            compact_json_session_text(value)
        }
        _ => compact_json_session_text(value),
    }
}

fn json_value_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => Some(text.clone()),
        Value::Array(items) => {
            let texts = items.iter().filter_map(json_value_text).collect::<Vec<_>>();
            (!texts.is_empty()).then(|| texts.join("\n"))
        }
        Value::Object(map) => {
            for key in [
                "text",
                "content",
                "message",
                "input",
                "arguments",
                "name",
                "thinking",
                "summary",
            ] {
                if let Some(text) = map.get(key).and_then(json_value_text) {
                    if !text.trim().is_empty() {
                        return Some(text);
                    }
                }
            }
            None
        }
        _ => None,
    }
}

fn json_tool_title(value: &Value) -> String {
    if let Value::Object(map) = value {
        for key in ["name", "tool_name", "function_name"] {
            if let Some(name) = map.get(key).and_then(Value::as_str) {
                if !name.trim().is_empty() {
                    return name.trim().to_string();
                }
            }
        }
        if let Some(function) = map.get("function").and_then(Value::as_object) {
            if let Some(name) = function.get("name").and_then(Value::as_str) {
                if !name.trim().is_empty() {
                    return name.trim().to_string();
                }
            }
        }
        if let Some(kind) = map.get("type").and_then(Value::as_str) {
            let normalized = kind.to_ascii_lowercase().replace(['_', '-'], "");
            return match normalized.as_str() {
                "toolresult" | "tooluseresult" | "functionresult" => "Tool result".to_string(),
                "tooluseerror" => "Tool error".to_string(),
                _ => "Tool".to_string(),
            };
        }
    }
    "Tool".to_string()
}

fn compact_json_session_text(value: &Value) -> String {
    match value {
        Value::String(text) => text.clone(),
        _ => serde_json::to_string(value).unwrap_or_default(),
    }
}

#[derive(Debug, Clone, Copy, Default)]
struct LocalSessionMetrics {
    user_message_count: usize,
    total_message_count: usize,
    tool_call_count: usize,
    skill_call_count: usize,
}

fn local_session_metrics(
    content: &str,
    content_items: &[LocalSessionContentItem],
) -> LocalSessionMetrics {
    let mut metrics = LocalSessionMetrics::default();
    for item in content_items {
        match item.kind.as_str() {
            "user_message" => {
                metrics.user_message_count += 1;
                metrics.total_message_count += 1;
            }
            "agent_reply" | "thinking" => {
                metrics.total_message_count += 1;
            }
            "tool_call" => {
                metrics.tool_call_count += 1;
            }
            "skill_call" => {}
            _ => {}
        }
    }

    if metrics.total_message_count == 0 {
        metrics.total_message_count = content
            .lines()
            .filter(|line| !line.trim().is_empty())
            .count();
    }
    metrics.skill_call_count = count_skill_invocation_mentions(content);
    metrics
}

fn json_session_role(map: &serde_json::Map<String, Value>) -> Option<&str> {
    map.get("role")
        .and_then(Value::as_str)
        .or_else(|| map.get("sender").and_then(Value::as_str))
        .or_else(|| {
            map.get("message")
                .and_then(Value::as_object)
                .and_then(|message| message.get("role"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            map.get("payload")
                .and_then(Value::as_object)
                .and_then(|payload| payload.get("role"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            map.get("payload")
                .and_then(Value::as_object)
                .and_then(|payload| payload.get("item"))
                .and_then(Value::as_object)
                .and_then(|item| item.get("role"))
                .and_then(Value::as_str)
        })
        .or_else(|| {
            map.get("author")
                .and_then(Value::as_object)
                .and_then(|author| author.get("role"))
                .and_then(Value::as_str)
        })
}

fn count_skill_invocation_mentions(text: &str) -> usize {
    extract_skill_invocation_names(text).len()
}

fn local_session_title(path: &Path, short_hash: &str) -> String {
    path.file_stem()
        .and_then(|name| name.to_str())
        .map(|name| truncate_chars(name, 120))
        .filter(|name| !name.is_empty())
        .unwrap_or_else(|| format!("Local session {short_hash}"))
}

fn infer_local_session_agent(path: &Path) -> Option<String> {
    let normalized = path.to_string_lossy().to_ascii_lowercase();
    if normalized.contains(".claude") {
        Some("claude-code".to_string())
    } else if normalized.contains(".codex") {
        Some("codex".to_string())
    } else if normalized.contains("opencode") {
        Some("opencode".to_string())
    } else if normalized.contains(".pi/") {
        Some("pi".to_string())
    } else {
        None
    }
}

fn local_session_modified_at(path: &Path) -> Option<i64> {
    let modified = fs::metadata(path).ok()?.modified().ok()?;
    modified
        .duration_since(UNIX_EPOCH)
        .ok()
        .map(|duration| duration.as_millis() as i64)
}

fn local_preview_redaction_summary_from(
    summary: LlmPromptRedactionSummary,
) -> LocalPreviewRedactionSummary {
    LocalPreviewRedactionSummary {
        status: "redacted-local-only".to_string(),
        redacted_value_count: summary.redacted_value_count,
        redacted_fields: summary.redacted_fields,
        placeholders: summary
            .placeholders
            .into_iter()
            .map(str::to_string)
            .collect(),
        raw_trace_persisted: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_secret_returned: summary.raw_secret_returned,
    }
}

fn local_preview_safety_flags() -> LocalPreviewSafetyFlags {
    LocalPreviewSafetyFlags {
        read_only: true,
        app_local_only: true,
        provider_request_sent: false,
        write_back_allowed: false,
        write_actions_available: false,
        skill_files_mutated: false,
        agent_config_mutated: false,
        script_execution_allowed: false,
        execution_actions_available: false,
        config_mutation_allowed: false,
        snapshot_created: false,
        triage_mutation_allowed: false,
        credential_accessed: false,
        raw_secret_returned: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
        cloud_sync_performed: false,
        telemetry_emitted: false,
    }
}
