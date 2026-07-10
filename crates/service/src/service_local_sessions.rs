use super::service_local_session_io::{
    read_bounded_text, BoundedReadSpec, BoundedText, LocalSessionIoContext, LocalSessionReadLimits,
    MAX_PROVENANCE_TOKEN_BYTES,
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
    let compacted_file_content = read_local_session_file_content(path, io)?;
    let file_content = strip_internal_local_session_records(&compacted_file_content);
    if file_content.is_empty() {
        return Ok(None);
    }
    let accepted_file_content = accepted_local_session_content(&file_content);
    let enriched_content = enrich_local_session_content(path, root, &accepted_file_content);
    let content = if enriched_content == accepted_file_content {
        accepted_file_content
    } else {
        accepted_local_session_content(&enriched_content)
    };
    let mut metadata = local_session_parsed_metadata(path, &content);
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
    let skill_invocations = extract_skill_invocation_names(&content);
    let skill_mentions = detect_local_session_skill_mentions(
        &skill_invocations,
        options.skill_matchers,
        &format!("session.content_hash:{short_hash}"),
    );
    let content_items = local_session_content_items(
        &content,
        &short_hash,
        options.max_excerpt_chars,
        &skill_mentions,
        &skill_invocations,
        redactor,
    );
    let modified_at = local_session_modified_at(path);
    let (started_at, ended_at) = local_session_time_bounds(&content, &content_items, modified_at);
    let metrics = local_session_metrics(&content_items, skill_invocations.len());
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
    let head = bounded
        .head
        .strip_prefix('\u{feff}')
        .unwrap_or(bounded.head.as_str());
    if !bounded.truncated && bounded.retained_head_end == bounded.retained_tail_start {
        let mut contiguous = String::with_capacity(head.len() + bounded.tail.len());
        contiguous.push_str(head);
        contiguous.push_str(&bounded.tail);
        return compact_local_session_records(&contiguous, max_line_fragment_bytes);
    }

    let complete_head_end = head.rfind('\n').map_or(0, |newline| newline + 1);
    let head_fragment = head[complete_head_end..].trim();
    let (tail_leading_fragment, tail_remainder) = split_tail_leading_fragment(&bounded.tail);
    let tail_leading_fragment = tail_leading_fragment.trim_end_matches('\r').trim();

    let mut retained_head =
        compact_local_session_records(&head[..complete_head_end], max_line_fragment_bytes);
    let mut recovered = String::new();
    let consumed_tail_leading =
        !bounded.tail_starts_at_line_boundary && !tail_leading_fragment.is_empty();
    let recoverable_provenance = bounded
        .record_provenance
        .as_ref()
        .filter(|_| !head_fragment.is_empty() && bounded.gap_stays_on_same_line);

    if let Some(provenance) = recoverable_provenance {
        let mut fields = top_level_scalar_fields_from_prefix(head_fragment);
        for (key, value) in top_level_scalar_fields_from_suffix(tail_leading_fragment) {
            fields.insert(key, value);
        }
        provenance.merge_into(&mut fields);
        append_supported_scalar_fields(&mut recovered, fields, max_line_fragment_bytes);
    }
    let retained_tail = if consumed_tail_leading {
        compact_local_session_records(tail_remainder, max_line_fragment_bytes)
    } else {
        compact_local_session_records(&bounded.tail, max_line_fragment_bytes)
    };
    if retained_head.is_empty() && recovered.is_empty() && retained_tail.is_empty() {
        return String::new();
    }

    retained_head.push_str(LOCAL_SESSION_TRUNCATION_MARKER_LINE);
    retained_head.push_str(&recovered);
    retained_head.push_str(&retained_tail);
    retained_head
}

fn compact_local_session_records(text: &str, max_line_fragment_bytes: usize) -> String {
    let mut content = String::new();
    for line in text.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() || should_skip_local_session_sidecar_line(trimmed) {
            continue;
        }
        if is_complete_json_record(trimmed)
            && trimmed.len().saturating_add(1) <= max_line_fragment_bytes
        {
            content.push_str(trimmed);
            content.push('\n');
            continue;
        }
        if looks_like_json_fragment(trimmed) && is_complete_json_record(trimmed) {
            append_compacted_record(&mut content, trimmed, max_line_fragment_bytes);
            continue;
        }
        if looks_like_incomplete_json_record(trimmed) {
            continue;
        }
        content.push_str(&truncate_utf8_bytes(
            trimmed,
            max_line_fragment_bytes.saturating_sub(1),
        ));
        content.push('\n');
    }
    content
}

fn append_supported_scalar_fragment(content: &mut String, fragment: &str, max_bytes: usize) {
    append_supported_scalar_fields(content, supported_scalar_fragment(fragment), max_bytes);
}

fn supported_scalar_fragment(fragment: &str) -> serde_json::Map<String, Value> {
    let mut fields = top_level_scalar_fields_from_prefix(fragment);
    for (key, value) in top_level_scalar_fields_from_suffix(fragment) {
        fields.insert(key, value);
    }
    fields
}

fn append_supported_scalar_fields(
    content: &mut String,
    fields: serde_json::Map<String, Value>,
    max_bytes: usize,
) {
    if supported_scalar_fields_are_non_message(&fields) {
        return;
    }
    let Some(compacted) = bounded_supported_scalar_object(fields, max_bytes) else {
        return;
    };
    if should_skip_local_session_sidecar_line(&compacted) {
        return;
    }
    content.push_str(&compacted);
    content.push('\n');
}

fn supported_scalar_fields_are_non_message(fields: &serde_json::Map<String, Value>) -> bool {
    matches!(
        local_session_record_classification(fields, None),
        LocalSessionRecordClassification::Deny | LocalSessionRecordClassification::Tool
    )
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum LocalSessionRecordClassification {
    User,
    Assistant,
    Thinking,
    Tool,
    Deny,
    Unknown,
}

fn local_session_object_is_denied(fields: &serde_json::Map<String, Value>) -> bool {
    let fallback_role = json_session_role(fields).map(str::to_string);
    local_session_record_classification(fields, fallback_role.as_deref())
        == LocalSessionRecordClassification::Deny
}

fn local_session_value_is_denied(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(local_session_object_is_denied)
}

fn local_session_record_classification(
    fields: &serde_json::Map<String, Value>,
    fallback_role: Option<&str>,
) -> LocalSessionRecordClassification {
    let type_classification = local_session_type_classification(fields.get("type"));
    let role_classification = fields.get("role").map_or_else(
        || {
            fallback_role.map_or(LocalSessionRecordClassification::Unknown, |role| {
                local_session_role_classification(role)
            })
        },
        |role| {
            role.as_str().map_or(
                LocalSessionRecordClassification::Deny,
                local_session_role_classification,
            )
        },
    );

    if matches!(type_classification, LocalSessionRecordClassification::Deny)
        || matches!(role_classification, LocalSessionRecordClassification::Deny)
    {
        return LocalSessionRecordClassification::Deny;
    }
    if matches!(type_classification, LocalSessionRecordClassification::Tool)
        || matches!(role_classification, LocalSessionRecordClassification::Tool)
    {
        return LocalSessionRecordClassification::Tool;
    }
    if type_classification != LocalSessionRecordClassification::Unknown {
        return type_classification;
    }
    role_classification
}

fn local_session_type_classification(value: Option<&Value>) -> LocalSessionRecordClassification {
    let Some(value) = value else {
        return LocalSessionRecordClassification::Unknown;
    };
    let Some(record_type) = value.as_str() else {
        return LocalSessionRecordClassification::Deny;
    };
    let normalized = record_type.to_ascii_lowercase().replace(['_', '-'], "");
    if is_hidden_local_session_record_type(record_type) || is_json_non_message_type(&normalized) {
        LocalSessionRecordClassification::Deny
    } else if is_json_tool_type(&normalized) {
        LocalSessionRecordClassification::Tool
    } else if is_json_thinking_type(&normalized) {
        LocalSessionRecordClassification::Thinking
    } else {
        match normalized.as_str() {
            "user" | "human" => LocalSessionRecordClassification::User,
            "assistant" | "agent" | "model" => LocalSessionRecordClassification::Assistant,
            _ => LocalSessionRecordClassification::Unknown,
        }
    }
}

fn local_session_role_classification(role: &str) -> LocalSessionRecordClassification {
    let normalized = role.to_ascii_lowercase().replace(['_', '-'], "");
    match normalized.as_str() {
        "user" | "human" | "customer" => LocalSessionRecordClassification::User,
        "assistant" | "agent" | "model" => LocalSessionRecordClassification::Assistant,
        "tool" | "function" | "toolresult" => LocalSessionRecordClassification::Tool,
        "system" | "developer" | "summary" => LocalSessionRecordClassification::Deny,
        _ => LocalSessionRecordClassification::Unknown,
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum LocalSessionJsonParseError {
    Invalid,
    UnsafeClassification,
}

fn parse_local_session_json_with_raw_classification_bounds(
    text: &str,
) -> Result<(Value, bool), LocalSessionJsonParseError> {
    let parsed =
        serde_json::from_str::<Value>(text).map_err(|_| LocalSessionJsonParseError::Invalid)?;
    let overflow_ranges = raw_classification_token_overflow_ranges(text.as_bytes())
        .ok_or(LocalSessionJsonParseError::UnsafeClassification)?;
    if overflow_ranges.is_empty() {
        return Ok((parsed, false));
    }

    let mut bounded = String::with_capacity(text.len());
    let mut copied_end = 0usize;
    for (start, end) in overflow_ranges {
        if start < copied_end || end > text.len() {
            return Err(LocalSessionJsonParseError::UnsafeClassification);
        }
        bounded.push_str(&text[copied_end..start]);
        bounded.push_str("null");
        copied_end = end;
    }
    bounded.push_str(&text[copied_end..]);
    serde_json::from_str(&bounded)
        .map(|value| (value, true))
        .map_err(|_| LocalSessionJsonParseError::UnsafeClassification)
}

fn raw_classification_token_overflow_ranges(bytes: &[u8]) -> Option<Vec<(usize, usize)>> {
    let mut cursor = 0usize;
    let mut ranges = Vec::new();
    scan_raw_json_value(bytes, &mut cursor, 0, &mut ranges)?;
    skip_raw_json_whitespace(bytes, &mut cursor);
    ranges.sort_unstable_by_key(|(start, _)| *start);
    (cursor == bytes.len()).then_some(ranges)
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RawClassificationKey {
    Type,
    Role,
    Other,
}

fn scan_raw_json_value(
    bytes: &[u8],
    cursor: &mut usize,
    depth: usize,
    ranges: &mut Vec<(usize, usize)>,
) -> Option<()> {
    const MAX_SCAN_NESTING: usize = 256;
    if depth > MAX_SCAN_NESTING {
        return None;
    }
    skip_raw_json_whitespace(bytes, cursor);
    match bytes.get(*cursor)? {
        b'{' => scan_raw_json_object(bytes, cursor, depth + 1, ranges),
        b'[' => scan_raw_json_array(bytes, cursor, depth + 1, ranges),
        b'"' => {
            let end = json_string_end(bytes, *cursor)?;
            *cursor = end + 1;
            Some(())
        }
        _ => {
            let start = *cursor;
            while bytes.get(*cursor).is_some_and(|byte| {
                !byte.is_ascii_whitespace() && !matches!(byte, b',' | b']' | b'}')
            }) {
                *cursor += 1;
            }
            (*cursor > start).then_some(())
        }
    }
}

fn scan_raw_json_object(
    bytes: &[u8],
    cursor: &mut usize,
    depth: usize,
    ranges: &mut Vec<(usize, usize)>,
) -> Option<()> {
    *cursor += 1;
    let mut final_type_overflow = None;
    let mut final_role_overflow = None;
    skip_raw_json_whitespace(bytes, cursor);
    if bytes.get(*cursor) == Some(&b'}') {
        *cursor += 1;
        return Some(());
    }

    loop {
        let key_start = *cursor;
        if bytes.get(key_start) != Some(&b'"') {
            return None;
        }
        let key_end = json_string_end(bytes, key_start)?;
        let key_token = &bytes[key_start..=key_end];
        let classification_key = (key_token.len() <= 64)
            .then(|| serde_json::from_slice::<String>(key_token).ok())
            .flatten()
            .map_or(RawClassificationKey::Other, |key| match key.as_str() {
                "type" => RawClassificationKey::Type,
                "role" => RawClassificationKey::Role,
                _ => RawClassificationKey::Other,
            });
        *cursor = key_end + 1;
        skip_raw_json_whitespace(bytes, cursor);
        if bytes.get(*cursor) != Some(&b':') {
            return None;
        }
        *cursor += 1;
        skip_raw_json_whitespace(bytes, cursor);

        if classification_key != RawClassificationKey::Other && bytes.get(*cursor) == Some(&b'"') {
            let value_start = *cursor;
            let value_end = json_string_end(bytes, value_start)? + 1;
            let overflow = (value_end.saturating_sub(value_start) > MAX_PROVENANCE_TOKEN_BYTES)
                .then_some((value_start, value_end));
            match classification_key {
                RawClassificationKey::Type => final_type_overflow = overflow,
                RawClassificationKey::Role => final_role_overflow = overflow,
                RawClassificationKey::Other => {}
            }
            *cursor = value_end;
        } else {
            match classification_key {
                RawClassificationKey::Type => final_type_overflow = None,
                RawClassificationKey::Role => final_role_overflow = None,
                RawClassificationKey::Other => {}
            }
            scan_raw_json_value(bytes, cursor, depth, ranges)?;
        }

        skip_raw_json_whitespace(bytes, cursor);
        match bytes.get(*cursor) {
            Some(b',') => {
                *cursor += 1;
                skip_raw_json_whitespace(bytes, cursor);
            }
            Some(b'}') => {
                *cursor += 1;
                ranges.extend(final_type_overflow);
                ranges.extend(final_role_overflow);
                return Some(());
            }
            _ => return None,
        }
    }
}

fn scan_raw_json_array(
    bytes: &[u8],
    cursor: &mut usize,
    depth: usize,
    ranges: &mut Vec<(usize, usize)>,
) -> Option<()> {
    *cursor += 1;
    skip_raw_json_whitespace(bytes, cursor);
    if bytes.get(*cursor) == Some(&b']') {
        *cursor += 1;
        return Some(());
    }

    loop {
        scan_raw_json_value(bytes, cursor, depth, ranges)?;
        skip_raw_json_whitespace(bytes, cursor);
        match bytes.get(*cursor) {
            Some(b',') => {
                *cursor += 1;
                skip_raw_json_whitespace(bytes, cursor);
            }
            Some(b']') => {
                *cursor += 1;
                return Some(());
            }
            _ => return None,
        }
    }
}

fn skip_raw_json_whitespace(bytes: &[u8], cursor: &mut usize) {
    while bytes.get(*cursor).is_some_and(u8::is_ascii_whitespace) {
        *cursor += 1;
    }
}

fn split_tail_leading_fragment(tail: &str) -> (&str, &str) {
    tail.find('\n').map_or((tail, ""), |newline| {
        (&tail[..newline], &tail[newline + 1..])
    })
}

fn is_complete_json_record(fragment: &str) -> bool {
    !fragment.is_empty() && serde_json::from_str::<Value>(fragment).is_ok()
}

fn append_compacted_record(content: &mut String, record: &str, max_bytes: usize) {
    match parse_local_session_json_with_raw_classification_bounds(record) {
        Ok((mut value, _)) => {
            if local_session_top_level_record_type(&value)
                .is_some_and(is_hidden_local_session_record_type)
            {
                return;
            }
            prune_omitted_local_session_values(&mut value);
            let compacted = value.to_string();
            if compacted.len().saturating_add(1) <= max_bytes {
                if !should_skip_local_session_sidecar_line(&compacted) {
                    content.push_str(&compacted);
                    content.push('\n');
                }
                return;
            }
            append_supported_scalar_fields(
                content,
                top_level_scalar_fields_from_prefix(&compacted),
                max_bytes,
            );
        }
        Err(LocalSessionJsonParseError::Invalid) => {
            append_supported_scalar_fragment(content, record, max_bytes);
        }
        Err(LocalSessionJsonParseError::UnsafeClassification) => {}
    }
}

fn prune_omitted_local_session_values(value: &mut Value) {
    match value {
        Value::Array(items) => {
            for item in items {
                prune_omitted_local_session_values(item);
            }
        }
        Value::Object(map) => {
            let omitted = map
                .keys()
                .filter(|key| is_omitted_local_session_key(key))
                .cloned()
                .collect::<Vec<_>>();
            for key in omitted {
                map.remove(&key);
            }
            for nested in map.values_mut() {
                prune_omitted_local_session_values(nested);
            }
        }
        Value::String(text) if text.len() > 4_096 => {
            *text = truncate_utf8_bytes(text, 4_096);
        }
        _ => {}
    }
}

fn is_omitted_local_session_key(key: &str) -> bool {
    matches!(
        key,
        "base64" | "blob" | "bytes" | "data" | "image" | "image_data"
    )
}

fn is_supported_local_session_scalar_key(key: &str) -> bool {
    matches!(
        key,
        "type"
            | "role"
            | "text"
            | "content"
            | "title"
            | "aiTitle"
            | "timestamp"
            | "sessionId"
            | "id"
            | "cwd"
    )
}

fn top_level_scalar_fields_from_prefix(fragment: &str) -> serde_json::Map<String, Value> {
    let bytes = fragment.as_bytes();
    let mut fields = serde_json::Map::new();
    let mut object_depth = 0_i32;
    let mut array_depth = 0_i32;
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'"' => {
                let Some(end) = json_string_end(bytes, cursor) else {
                    break;
                };
                if object_depth == 1 && array_depth == 0 {
                    if let Some((key, value)) = scalar_field_at(fragment, cursor, end) {
                        fields.insert(key, value);
                    }
                }
                cursor = end + 1;
            }
            b'{' => {
                object_depth += 1;
                cursor += 1;
            }
            b'}' => {
                object_depth -= 1;
                cursor += 1;
            }
            b'[' => {
                array_depth += 1;
                cursor += 1;
            }
            b']' => {
                array_depth -= 1;
                cursor += 1;
            }
            _ => cursor += 1,
        }
    }
    fields
}

fn top_level_scalar_fields_from_suffix(fragment: &str) -> serde_json::Map<String, Value> {
    let bytes = fragment.as_bytes();
    let mut fields = serde_json::Map::new();
    let mut object_depth = 0_i32;
    let mut array_depth = 0_i32;
    let mut string_end = None;
    let mut cursor = bytes.len();
    while cursor > 0 {
        cursor -= 1;
        let byte = bytes[cursor];
        if let Some(end) = string_end {
            if byte == b'"' && !json_quote_is_escaped(bytes, cursor) {
                if object_depth == 1 && array_depth == 0 {
                    if let Some((key, value)) = scalar_field_at(fragment, cursor, end) {
                        fields.entry(key).or_insert(value);
                    }
                }
                string_end = None;
            }
            continue;
        }
        match byte {
            b'"' if !json_quote_is_escaped(bytes, cursor) => string_end = Some(cursor),
            b'}' => object_depth += 1,
            b'{' => object_depth -= 1,
            b']' => array_depth += 1,
            b'[' => array_depth -= 1,
            _ => {}
        }
    }
    fields
}

fn json_string_end(bytes: &[u8], start: usize) -> Option<usize> {
    let mut escaped = false;
    for (offset, byte) in bytes[start + 1..].iter().enumerate() {
        if escaped {
            escaped = false;
            continue;
        }
        match byte {
            b'\\' => escaped = true,
            b'"' => return Some(start + 1 + offset),
            _ => {}
        }
    }
    None
}

fn json_quote_is_escaped(bytes: &[u8], quote: usize) -> bool {
    let mut backslashes = 0usize;
    let mut cursor = quote;
    while cursor > 0 && bytes[cursor - 1] == b'\\' {
        backslashes += 1;
        cursor -= 1;
    }
    backslashes % 2 == 1
}

fn scalar_field_at(fragment: &str, start: usize, end: usize) -> Option<(String, Value)> {
    let key = serde_json::from_str::<String>(&fragment[start..=end]).ok()?;
    if !is_supported_local_session_scalar_key(&key) {
        return None;
    }
    let bytes = fragment.as_bytes();
    let mut cursor = end + 1;
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    if bytes.get(cursor) != Some(&b':') {
        return None;
    }
    cursor += 1;
    while bytes.get(cursor).is_some_and(u8::is_ascii_whitespace) {
        cursor += 1;
    }
    if matches!(bytes.get(cursor), Some(b'{') | Some(b'[')) {
        return None;
    }
    if matches!(key.as_str(), "type" | "role") && bytes.get(cursor) == Some(&b'"') {
        let value_end = json_string_end(bytes, cursor)?;
        if value_end.saturating_sub(cursor).saturating_add(1) > MAX_PROVENANCE_TOKEN_BYTES {
            return Some((key, Value::Null));
        }
    }
    let value = serde_json::Deserializer::from_str(&fragment[cursor..])
        .into_iter::<Value>()
        .next()?
        .ok()?;
    matches!(
        value,
        Value::String(_) | Value::Number(_) | Value::Bool(_) | Value::Null
    )
    .then_some((key, value))
}

fn bounded_supported_scalar_object(
    mut fields: serde_json::Map<String, Value>,
    max_bytes: usize,
) -> Option<String> {
    let maximum_object_bytes = max_bytes.checked_sub(1)?;
    let mut retained = serde_json::Map::new();
    for key in [
        "type",
        "role",
        "timestamp",
        "sessionId",
        "id",
        "cwd",
        "title",
        "aiTitle",
        "text",
        "content",
    ] {
        let Some(value) = fields.remove(key) else {
            continue;
        };
        if let Some(value) =
            value_fitting_scalar_object(&retained, key, value, maximum_object_bytes)
        {
            retained.insert(key.to_string(), value);
        }
    }
    if retained.is_empty() {
        return None;
    }
    let compacted = Value::Object(retained).to_string();
    (compacted.len() <= maximum_object_bytes).then_some(compacted)
}

fn value_fitting_scalar_object(
    retained: &serde_json::Map<String, Value>,
    key: &str,
    value: Value,
    maximum_object_bytes: usize,
) -> Option<Value> {
    let Value::String(text) = value else {
        let mut candidate = retained.clone();
        candidate.insert(key.to_string(), value.clone());
        return (Value::Object(candidate).to_string().len() <= maximum_object_bytes)
            .then_some(value);
    };
    let mut boundaries = text
        .char_indices()
        .map(|(index, _)| index)
        .collect::<Vec<_>>();
    boundaries.push(text.len());
    let mut low = 0usize;
    let mut high = boundaries.len();
    while low < high {
        let middle = low + (high - low).div_ceil(2);
        let mut candidate = retained.clone();
        candidate.insert(
            key.to_string(),
            Value::String(text[..boundaries[middle - 1]].to_string()),
        );
        if Value::Object(candidate).to_string().len() <= maximum_object_bytes {
            low = middle;
        } else {
            high = middle - 1;
        }
    }
    (low > 0).then(|| Value::String(text[..boundaries[low - 1]].to_string()))
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

fn looks_like_incomplete_json_record(text: &str) -> bool {
    let text = text.trim_start_matches('\u{feff}').trim_start();
    let Some((opening, remainder)) = text
        .chars()
        .next()
        .map(|opening| (opening, &text[opening.len_utf8()..]))
    else {
        return false;
    };
    let next = remainder.trim_start().chars().next();
    match opening {
        '{' => next.is_none() || matches!(next, Some('"' | '}')),
        '[' => {
            next.is_none()
                || matches!(
                    next,
                    Some('"' | '{' | '[' | ']' | '-' | '0'..='9' | 't' | 'f' | 'n')
                )
        }
        _ => false,
    }
}

#[cfg(test)]
mod bounded_content_tests {
    use super::*;

    #[test]
    fn incomplete_json_detection_preserves_plaintext_with_json_punctuation() {
        for incomplete in [r#"{"type":"user""#, r#"["partial""#, "{   ", "[true"] {
            assert!(
                looks_like_incomplete_json_record(incomplete),
                "expected incomplete JSON: {incomplete}"
            );
        }
        for plaintext in ["{plain visible", "[INFO] visible", "visible ]"] {
            assert!(
                !looks_like_incomplete_json_record(plaintext),
                "expected plaintext: {plaintext}"
            );
        }
    }

    #[test]
    fn complete_but_unproven_head_fragment_is_dropped() {
        let bounded = BoundedText {
            head: concat!(
                "{\"type\":\"mode\"}\n",
                "{\"data\":{\"type\":\"user\",\"text\":\"head-hello\"}}"
            )
            .to_string(),
            tail: String::new(),
            retained_head_end: 80,
            retained_tail_start: 100,
            tail_starts_at_line_boundary: false,
            gap_stays_on_same_line: false,
            record_provenance: None,
            truncated: true,
            bytes_read: 80,
        };

        let compacted = compact_bounded_local_session_content(&bounded, 1_024);

        assert!(!compacted.contains("\"data\""), "{compacted}");
        assert!(!compacted.contains("head-hello"), "{compacted}");
        assert!(compacted.is_empty(), "{compacted}");
    }

    #[test]
    fn incomplete_tail_without_root_type_is_not_attributed_after_complete_head() {
        let bounded = BoundedText {
            head: "{\"type\":\"user\",\"role\":\"user\",\"text\":\"head-visible\"}\n".to_string(),
            tail: "private-tail\",\"text\":\"untyped-tail-must-not-surface\"}\n".to_string(),
            retained_head_end: 64,
            retained_tail_start: 128,
            tail_starts_at_line_boundary: false,
            gap_stays_on_same_line: false,
            record_provenance: None,
            truncated: true,
            bytes_read: 128,
        };

        let compacted = compact_bounded_local_session_content(&bounded, 1_024);

        assert!(compacted.contains("head-visible"), "{compacted}");
        assert!(!compacted.contains("untyped-tail-must-not-surface"));
    }

    #[test]
    fn recovered_scalar_record_respects_aggregate_line_fragment_limit() {
        let max_bytes = 1_024;
        let fragment = format!(
            "{{\"type\":\"user\",\"role\":\"user\",\"text\":{},\"content\":{}}}",
            serde_json::to_string(&"t".repeat(max_bytes)).expect("serialize text"),
            serde_json::to_string(&"c".repeat(max_bytes)).expect("serialize content")
        );
        let mut compacted = String::new();

        append_supported_scalar_fragment(&mut compacted, &fragment, max_bytes);

        let line = compacted.trim_end_matches('\n');
        assert!(
            compacted.len() <= max_bytes,
            "retained line and delimiter use {} bytes",
            compacted.len()
        );
        assert!(line.len() <= max_bytes, "retained {} bytes", line.len());
        assert!(serde_json::from_str::<Value>(line).is_ok());
    }

    #[test]
    fn prefix_scalar_recovery_ignores_nested_omitted_fields() {
        let fragment = r#"{"type":"user","role":"user","data":{"text":"nested-private""#;

        let fields = top_level_scalar_fields_from_prefix(fragment);

        assert_eq!(fields.get("type").and_then(Value::as_str), Some("user"));
        assert_eq!(fields.get("role").and_then(Value::as_str), Some("user"));
        assert!(!fields.contains_key("text"));
    }

    #[test]
    fn suffix_scalar_recovery_ignores_nested_data_and_keeps_outer_text() {
        let fragment = concat!(
            "private-tail\",\"text\":\"nested-private\",\"type\":\"assistant\"},",
            "\"text\":\"outer-visible\",\"timestamp\":\"2026-07-10T08:09:10Z\"}"
        );

        let fields = top_level_scalar_fields_from_suffix(fragment);

        assert_eq!(
            fields.get("text").and_then(Value::as_str),
            Some("outer-visible")
        );
        assert_eq!(
            fields.get("timestamp").and_then(Value::as_str),
            Some("2026-07-10T08:09:10Z")
        );
        assert!(!fields.contains_key("type"));
    }
}

fn should_skip_local_session_sidecar_line(line: &str) -> bool {
    serde_json::from_str::<Value>(line)
        .ok()
        .as_ref()
        .and_then(local_session_top_level_record_type)
        .is_some_and(is_hidden_local_session_record_type)
}

const LOCAL_SESSION_TRUNCATION_MARKER_TYPE: &str = "skills-copilot-truncation-marker";
const LOCAL_SESSION_TRUNCATION_MARKER_LINE: &str =
    "{\"type\":\"skills-copilot-truncation-marker\"}\n";

const SKIPPED_LOCAL_SESSION_RECORD_TYPES: [&str; 6] = [
    "attachment",
    "file-history-snapshot",
    "last-prompt",
    "mode",
    "permission-mode",
    "queue-operation",
];

fn is_skipped_local_session_record_type(record_type: &str) -> bool {
    SKIPPED_LOCAL_SESSION_RECORD_TYPES.contains(&record_type)
}

fn is_hidden_local_session_record_type(record_type: &str) -> bool {
    record_type == LOCAL_SESSION_TRUNCATION_MARKER_TYPE
        || is_skipped_local_session_record_type(record_type)
}

fn local_session_top_level_record_type(value: &Value) -> Option<&str> {
    value
        .as_object()
        .and_then(|map| map.get("type"))
        .and_then(Value::as_str)
}

fn strip_internal_local_session_records(content: &str) -> String {
    let mut visible = String::new();
    for line in content.lines() {
        let trimmed = line.trim();
        let is_marker = serde_json::from_str::<Value>(trimmed)
            .ok()
            .as_ref()
            .and_then(local_session_top_level_record_type)
            == Some(LOCAL_SESSION_TRUNCATION_MARKER_TYPE);
        if trimmed.is_empty() || is_marker {
            continue;
        }
        visible.push_str(line);
        visible.push('\n');
    }
    visible
}

fn accepted_local_session_content(content: &str) -> String {
    let mut accepted = String::with_capacity(content.len());
    for line in content.lines() {
        let trimmed = line.trim();
        if trimmed.is_empty() {
            continue;
        }
        match parse_local_session_json_with_raw_classification_bounds(trimmed) {
            Ok((mut value, classification_changed)) => {
                let (retained, tree_changed) = retain_accepted_local_session_value(&mut value);
                if retained {
                    if classification_changed || tree_changed {
                        accepted.push_str(&value.to_string());
                    } else {
                        accepted.push_str(trimmed);
                    }
                    accepted.push('\n');
                }
            }
            Err(LocalSessionJsonParseError::Invalid) => {
                accepted.push_str(line);
                accepted.push('\n');
            }
            Err(LocalSessionJsonParseError::UnsafeClassification) => {}
        }
    }
    accepted
}

fn retain_accepted_local_session_value(value: &mut Value) -> (bool, bool) {
    match value {
        Value::Array(items) => {
            let mut changed = false;
            items.retain_mut(|nested| {
                let (retained, nested_changed) = retain_accepted_local_session_value(nested);
                changed |= nested_changed || !retained;
                retained
            });
            (true, changed)
        }
        Value::Object(map) => {
            if local_session_object_is_denied(map) {
                return (false, true);
            }
            let mut changed = false;
            map.retain(|_, nested| {
                let (retained, nested_changed) = retain_accepted_local_session_value(nested);
                changed |= nested_changed || !retained;
                retained
            });
            (true, changed)
        }
        _ => (true, false),
    }
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

fn local_session_parsed_metadata(path: &Path, content: &str) -> LocalSessionParsedMetadata {
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

    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            if metadata.title.is_none() && !parsed_json {
                metadata.title = local_session_text_title_candidate(line);
            }
            continue;
        };
        parsed_json = true;
        merge_local_session_metadata(&value, &mut metadata);
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
    invocations: &[String],
    skill_matchers: &[LocalSessionSkillMatcher],
    evidence_ref: &str,
) -> Vec<LocalSessionSkillMention> {
    if skill_matchers.is_empty() {
        return Vec::new();
    }
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
    skill_invocations: &[String],
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
    for invocation in skill_invocations {
        if matched_invocations.contains(invocation) {
            continue;
        }
        *unmatched_invocations.entry(invocation.clone()).or_default() += 1;
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
        if let Some(text) = local_session_plain_text_fallback(content) {
            drafts.push(LocalSessionContentDraft {
                kind: "agent_reply".to_string(),
                title: "Session excerpt".to_string(),
                text,
                timestamp: None,
                evidence_refs: vec![format!("session.content_hash:{short_hash}")],
            });
        }
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

fn local_session_plain_text_fallback(content: &str) -> Option<String> {
    let text = content
        .lines()
        .map(str::trim)
        .filter(|line| !line.is_empty())
        .filter_map(|line| match serde_json::from_str::<Value>(line) {
            Ok(value) => json_non_tool_message_text(&value),
            Err(_) => Some(line.to_string()),
        })
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n");
    (!text.is_empty()).then_some(text)
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
            if local_session_object_is_denied(map) {
                return;
            }
            let direct_kind = json_session_content_kind(map, role.as_deref());
            let direct_tool = direct_kind == Some("tool_call");
            let mut pushed_direct_tool = false;
            if let Some(kind) = direct_kind {
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
                if is_json_session_structure_key(key)
                    || matches!(
                        key.as_str(),
                        "content"
                            | "text"
                            | "message"
                            | "delta"
                            | "tool_calls"
                            | "toolCalls"
                            | "tool_use"
                            | "toolUse"
                            | "function_call"
                            | "parts"
                    )
                {
                    continue;
                }
                if direct_tool
                    && matches!(
                        key.as_str(),
                        "result"
                            | "error"
                            | "output"
                            | "input"
                            | "arguments"
                            | "payload"
                            | "data"
                            | "function"
                    )
                {
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
                    if local_session_value_is_denied(item) {
                        continue;
                    }
                    if let Some(text) = json_tool_payload_text(item) {
                        drafts.push(LocalSessionContentDraft {
                            kind: "tool_call".to_string(),
                            title: json_tool_title(item),
                            text,
                            timestamp,
                            evidence_refs: Vec::new(),
                        });
                    }
                }
            }
            Value::Object(_) | Value::String(_) => {
                if local_session_value_is_denied(value) {
                    continue;
                }
                if let Some(text) = json_tool_payload_text(value) {
                    drafts.push(LocalSessionContentDraft {
                        kind: "tool_call".to_string(),
                        title: json_tool_title(value),
                        text,
                        timestamp,
                        evidence_refs: Vec::new(),
                    });
                }
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
            if local_session_object_is_denied(map) {
                return;
            }
            let timestamp = json_session_timestamp_millis(value).or(inherited_timestamp);
            if let Some(kind) = map.get("type").and_then(Value::as_str) {
                let normalized = kind.to_ascii_lowercase().replace(['_', '-'], "");
                if is_json_tool_type(&normalized) {
                    if let Some(text) = json_tool_payload_text(value) {
                        drafts.push(LocalSessionContentDraft {
                            kind: "tool_call".to_string(),
                            title: json_tool_title(value),
                            text,
                            timestamp,
                            evidence_refs: Vec::new(),
                        });
                    }
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
    match local_session_record_classification(map, role) {
        LocalSessionRecordClassification::User => return Some("user_message"),
        LocalSessionRecordClassification::Assistant => return Some("agent_reply"),
        LocalSessionRecordClassification::Thinking => return Some("thinking"),
        LocalSessionRecordClassification::Tool => return Some("tool_call"),
        LocalSessionRecordClassification::Deny => return None,
        LocalSessionRecordClassification::Unknown => {}
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
    None
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
        return json_tool_payload_text(&Value::Object(map.clone()));
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
            if is_json_tool_object(map) || json_session_blocks_plain_text_fallback(map) {
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

fn json_session_blocks_plain_text_fallback(map: &serde_json::Map<String, Value>) -> bool {
    matches!(
        local_session_record_classification(map, json_session_role(map)),
        LocalSessionRecordClassification::Deny | LocalSessionRecordClassification::Tool
    )
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
        "tool"
            | "toolcall"
            | "tooluse"
            | "functioncall"
            | "toolresult"
            | "tooluseresult"
            | "tooluseerror"
            | "functionresult"
    )
}

fn is_json_non_message_type(normalized: &str) -> bool {
    matches!(
        normalized,
        "developer" | "system" | "summary" | "compaction" | "context" | "metadata"
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

fn json_tool_payload_text(value: &Value) -> Option<String> {
    match value {
        Value::String(text) => (!text.trim().is_empty()).then(|| text.clone()),
        Value::Object(map) => {
            if local_session_object_is_denied(map) {
                return None;
            }
            for key in [
                "content",
                "text",
                "message",
                "result",
                "error",
                "output",
                "input",
                "arguments",
                "payload",
                "data",
            ] {
                let Some(value) = map.get(key) else {
                    continue;
                };
                if is_meaningful_empty_tool_payload(key, value) {
                    return Some(compact_json_session_text(value));
                }
                if let Some(text) = json_tool_payload_value_text(value) {
                    return Some(text);
                }
            }
            map.get("function").and_then(json_tool_payload_text)
        }
        _ => None,
    }
}

fn is_meaningful_empty_tool_payload(key: &str, value: &Value) -> bool {
    matches!(key, "arguments" | "input" | "result" | "output")
        && match value {
            Value::Array(items) => items.is_empty(),
            Value::Object(map) => map.is_empty(),
            _ => false,
        }
}

fn json_tool_payload_value_text(value: &Value) -> Option<String> {
    if !json_tool_payload_value_has_content(value) {
        return None;
    }
    match value {
        Value::Null => None,
        Value::String(text) => Some(text.clone()),
        Value::Array(_) | Value::Object(_) | Value::Bool(_) | Value::Number(_) => {
            Some(compact_json_session_text(value))
        }
    }
}

fn json_tool_payload_value_has_content(value: &Value) -> bool {
    match value {
        Value::Null => false,
        Value::String(text) => !text.trim().is_empty(),
        Value::Array(items) => items.iter().any(json_tool_payload_value_has_content),
        Value::Object(map) => map.iter().any(|(key, nested)| {
            !is_json_session_structure_key(key) && json_tool_payload_value_has_content(nested)
        }),
        Value::Bool(_) | Value::Number(_) => true,
    }
}

fn is_json_session_structure_key(key: &str) -> bool {
    matches!(
        key,
        "type"
            | "kind"
            | "role"
            | "sender"
            | "author"
            | "id"
            | "name"
            | "title"
            | "tool_name"
            | "function_name"
            | "tool_use_id"
            | "toolUseId"
            | "tool_call_id"
            | "toolCallId"
            | "session_id"
            | "sessionId"
            | "conversation_id"
            | "conversationId"
            | "cwd"
            | "timestamp"
            | "created_at"
            | "createdAt"
            | "updated_at"
            | "updatedAt"
    )
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
    content_items: &[LocalSessionContentItem],
    skill_call_count: usize,
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

    metrics.skill_call_count = skill_call_count;
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
