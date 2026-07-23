use super::*;
use crate::service_keyset_cursor::{decode_cursor, encode_cursor, KeysetCursor};
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};
use skills_copilot_core::SourceCoverage;

const PREVIEW_METHOD: &str = "session.previewLocalSessions";
const MESSAGE_METHOD: &str = "session.listLocalSessionMessages";
const MAX_SQLITE_SESSIONS: usize = 10_000;
const MAX_SQLITE_PREVIEW_MESSAGES: usize = 2_000;
const MAX_SQLITE_TEXT_BYTES: usize = 32 * 1024 * 1024;
const SQLITE_MESSAGE_SCAN_ROWS: usize = 1_000;
const SQLITE_MESSAGE_SCAN_BYTES: usize = 32 * 1024 * 1024;
const SQLITE_MESSAGE_SNAPSHOT_ROWS: usize = 20_000;
const SQLITE_MESSAGE_SNAPSHOT_BYTES: usize = 32 * 1024 * 1024;

#[derive(Clone, Copy, Eq, PartialEq)]
enum SqliteAgent {
    Opencode,
    Hermes,
    Openclaw,
}

impl SqliteAgent {
    fn from_requested(value: Option<&str>) -> Option<Self> {
        match value?.trim().to_ascii_lowercase().as_str() {
            "opencode" | "open-code" => Some(Self::Opencode),
            "hermes" | "hermes-agent" => Some(Self::Hermes),
            "openclaw" | "open-claw" => Some(Self::Openclaw),
            _ => None,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Opencode => "opencode",
            Self::Hermes => "hermes",
            Self::Openclaw => "openclaw",
        }
    }

    fn source_kind(self) -> &'static str {
        match self {
            Self::Opencode => "opencode-sqlite",
            Self::Hermes => "hermes-sqlite",
            Self::Openclaw => "openclaw-sqlite",
        }
    }

    fn database_paths(self, ctx: &AdapterContext) -> Vec<(PathBuf, Option<String>)> {
        match self {
            Self::Opencode => vec![(opencode_data_dir(ctx).join("opencode.db"), None)],
            Self::Hermes => vec![(hermes_home_dir(ctx).join("state.db"), None)],
            Self::Openclaw => openclaw_database_sources(ctx),
        }
    }
}

fn openclaw_database_sources(ctx: &AdapterContext) -> Vec<(PathBuf, Option<String>)> {
    let state_dir = openclaw_state_dir(ctx);
    let config = std::fs::read_to_string(openclaw_config_path(ctx))
        .ok()
        .and_then(|content| json5::from_str::<serde_json::Value>(&content).ok());
    openclaw_agent_database_paths(&state_dir)
        .into_iter()
        .map(|db_path| {
            let agent_id = db_path
                .parent()
                .and_then(Path::parent)
                .and_then(Path::file_name)
                .and_then(|value| value.to_str())
                .unwrap_or("main");
            let workspace =
                openclaw_workspace_for_agent(config.as_ref(), agent_id, &state_dir, &ctx.user_home);
            (
                db_path,
                workspace.map(|path| path.to_string_lossy().to_string()),
            )
        })
        .collect()
}

fn openclaw_workspace_for_agent(
    config: Option<&serde_json::Value>,
    agent_id: &str,
    state_dir: &Path,
    user_home: &Path,
) -> Option<PathBuf> {
    let listed = config
        .and_then(|config| config.get("agents"))
        .and_then(|agents| agents.get("list"))
        .and_then(serde_json::Value::as_array)
        .and_then(|agents| {
            agents
                .iter()
                .find(|agent| agent.get("id").and_then(serde_json::Value::as_str) == Some(agent_id))
        })
        .and_then(|agent| agent.get("workspace"))
        .and_then(serde_json::Value::as_str);
    let default = config
        .and_then(|config| config.get("agents"))
        .and_then(|agents| agents.get("defaults"))
        .and_then(|defaults| defaults.get("workspace"))
        .and_then(serde_json::Value::as_str);
    let raw = listed.or(default);
    if let Some(raw) = raw {
        let trimmed = raw.trim();
        if trimmed == "~" {
            return Some(user_home.to_path_buf());
        }
        if let Some(relative) = trimmed.strip_prefix("~/") {
            return Some(user_home.join(relative));
        }
        let path = PathBuf::from(trimmed);
        return Some(if path.is_absolute() {
            path
        } else {
            state_dir.join(path)
        });
    }
    Some(if agent_id == "main" {
        state_dir.join("workspace")
    } else {
        state_dir.join(format!("workspace-{agent_id}"))
    })
}

#[derive(Clone)]
struct SqliteSession {
    db_path: PathBuf,
    native_id: String,
    resume_locator: String,
    service_id: String,
    title: String,
    project_root: Option<String>,
    started_at: Option<i64>,
    modified_at: i64,
    ended_at: Option<i64>,
    declared_message_count: usize,
    declared_tool_count: usize,
}

pub(super) struct SqliteSessionResumeSnapshot {
    pub(super) row: LocalSessionPreviewRow,
    pub(super) resume_locator: SessionNativeResumeLocator,
    pub(super) source_revision: String,
    pub(super) coverage: SourceCoverage,
    pub(super) project_matches_selected_context: bool,
}

#[derive(Clone)]
struct SqliteMessage {
    role: String,
    text: String,
    timestamp: Option<i64>,
    kind: String,
}

#[derive(Clone)]
struct SqliteMessageRow {
    message: SqliteMessage,
    raw_digest: [u8; 32],
    raw_bytes: usize,
}

#[allow(clippy::too_many_arguments)]
pub(super) fn preview_sqlite_sessions(
    ctx: &AdapterContext,
    params: &LocalSessionPreviewParams,
    requested_agent: Option<&str>,
    scope: LocalSessionScope,
    sort: LocalSessionSort,
    direction: SortDirection,
    search: Option<&str>,
    include_content_items: bool,
    limit: usize,
    max_excerpt_chars: usize,
    redaction_roots: &[(String, &'static str)],
) -> Result<Option<LocalSessionPreviewResult>, ServiceError> {
    if requested_agent.is_some_and(|agent| agent.eq_ignore_ascii_case(AgentId::Codex.as_str())) {
        // Summary inventory comes from the same local thread index used by
        // `thread/list`. Exact detail and message reads deliberately fall back
        // to the guarded rollout reader so raw conversation content is never
        // copied into another database.
        if params.session_id.is_none() && !include_content_items {
            return preview_codex_state_sessions(
                ctx,
                params,
                scope,
                sort,
                direction,
                search,
                limit,
                max_excerpt_chars,
                redaction_roots,
            );
        }
        return Ok(None);
    }
    let Some(agent) = SqliteAgent::from_requested(requested_agent) else {
        return Ok(None);
    };
    let db_sources = agent
        .database_paths(ctx)
        .into_iter()
        .filter(|(path, _)| path.is_file())
        .collect::<Vec<_>>();
    if db_sources.is_empty() {
        return Ok(None);
    }
    if params.paging_mode.as_deref() == Some("keyset")
        && (sort != LocalSessionSort::ModifiedAt
            || direction != SortDirection::Desc
            || search.is_some()
            || scope != LocalSessionScope::All)
    {
        return Err(ServiceError::InvalidRequest(
            "cursor session pages require all-scope recent order without server search".to_string(),
        ));
    }

    let mut sessions = Vec::new();
    for (db_path, project_root) in &db_sources {
        let connection = open_read_only_database(db_path)?;
        sessions.extend(load_sessions(
            &connection,
            agent,
            db_path,
            project_root.as_deref(),
        )?);
    }
    if let Some(session_id) = params
        .session_id
        .as_deref()
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        sessions.retain(|session| session.service_id == session_id);
    }
    if scope == LocalSessionScope::Project {
        let project_roots = local_session_project_filter_roots(
            ctx,
            params.project_root.as_deref(),
            params.current_cwd.as_deref(),
        );
        sessions.retain(|session| {
            session.project_root.as_ref().is_some_and(|session_root| {
                project_roots
                    .iter()
                    .any(|root| local_session_paths_match(root, Path::new(session_root)))
            })
        });
    }
    if let Some(search) = search {
        sessions.retain(|session| session.title.to_ascii_lowercase().contains(search));
    }
    sort_sqlite_sessions(&mut sessions, sort, direction);

    let source_revision = sqlite_source_revision(&sessions, agent);
    let query_digest = sqlite_preview_query_digest(agent, params, scope, include_content_items);
    let cursor = params
        .cursor
        .as_deref()
        .map(|value| decode_cursor(value, PREVIEW_METHOD, &query_digest))
        .transpose()?;
    if params
        .source_revision
        .as_deref()
        .is_some_and(|value| value != source_revision)
        || cursor
            .as_ref()
            .is_some_and(|value| value.source_revision != source_revision)
    {
        return Err(ServiceError::SourceChanged);
    }
    let start = cursor.as_ref().map_or_else(
        || params.offset.unwrap_or(0).min(sessions.len()),
        |cursor| {
            sessions.partition_point(|session| {
                session.modified_at > cursor.sort_value
                    || (session.modified_at == cursor.sort_value
                        && session.service_id <= cursor.stable_id)
            })
        },
    );
    let end = start.saturating_add(limit).min(sessions.len());
    let page = &sessions[start..end];
    let mut redactor = PromptRedactor::new(redaction_roots);
    let mut rows = Vec::with_capacity(page.len());
    for session in page {
        let connection = open_read_only_database(&session.db_path)?;
        rows.push(sqlite_session_row(
            &connection,
            agent,
            session,
            scope,
            include_content_items,
            max_excerpt_chars,
            &mut redactor,
        )?);
    }
    let has_more = end < sessions.len();
    let next_cursor = if has_more {
        page.last()
            .map(|session| {
                encode_cursor(&KeysetCursor {
                    version: 1,
                    method: PREVIEW_METHOD.to_string(),
                    query_digest: query_digest.clone(),
                    source_revision: source_revision.clone(),
                    sort_value: session.modified_at,
                    stable_id: session.service_id.clone(),
                    tie_breaker_digest: None,
                    accepted_count: Some(end),
                    processed_prefix_digest: None,
                    resolved_start_at: None,
                    resolved_end_at: None,
                })
            })
            .transpose()?
    } else {
        None
    };
    let preview_roots = db_sources
        .iter()
        .map(|(db_path, _)| LocalSessionPreviewRoot {
            root: redactor.redact(&db_path.to_string_lossy()),
            status: "auto-discovered-read-only".to_string(),
            candidate_count: sessions
                .iter()
                .filter(|session| session.db_path.as_path() == db_path.as_path())
                .count(),
            blocker: None,
        })
        .collect();
    let user_message_count = rows.iter().map(|row| row.user_message_count).sum();
    let total_message_count = rows.iter().map(|row| row.total_message_count).sum();
    let tool_call_count = rows.iter().map(|row| row.tool_call_count).sum();
    let count = rows.len();
    Ok(Some(LocalSessionPreviewResult {
        generated_by: "local-v3.00-sqlite",
        authorized: true,
        authorization_required: false,
        roots: preview_roots,
        count,
        total_candidate_count: sessions.len(),
        total_matched_count: sessions.len(),
        offset: start,
        limit,
        has_more,
        next_offset: (params.cursor.is_none() && has_more).then_some(end),
        next_cursor,
        source_revision: Some(source_revision),
        source_completeness: ListSourceCompleteness::Enumerable,
        incomplete_reason: None,
        candidate_set_truncated: false,
        user_message_count,
        total_message_count,
        tool_call_count,
        skill_call_count: 0,
        skill_usage_rows: Vec::new(),
        session_rows: rows,
        gap_notes: Vec::new(),
        blocker_notes: Vec::new(),
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
    }))
}

#[derive(Clone)]
struct CodexIndexedSession {
    service_id: String,
    native_id: String,
    title: String,
    cwd: String,
    rollout_path: PathBuf,
    preview: String,
    started_at: i64,
    modified_at: i64,
}

#[allow(clippy::too_many_arguments)]
fn preview_codex_state_sessions(
    ctx: &AdapterContext,
    params: &LocalSessionPreviewParams,
    scope: LocalSessionScope,
    sort: LocalSessionSort,
    direction: SortDirection,
    search: Option<&str>,
    limit: usize,
    max_excerpt_chars: usize,
    redaction_roots: &[(String, &'static str)],
) -> Result<Option<LocalSessionPreviewResult>, ServiceError> {
    let Some(db_path) = codex_state_database_path(ctx) else {
        return Ok(None);
    };
    if params.paging_mode.as_deref() == Some("keyset")
        && (sort != LocalSessionSort::ModifiedAt
            || direction != SortDirection::Desc
            || search.is_some()
            || scope != LocalSessionScope::All)
    {
        return Err(ServiceError::InvalidRequest(
            "cursor session pages require all-scope recent order without server search".to_string(),
        ));
    }

    let connection = open_read_only_database(&db_path)?;
    let mut sessions = load_codex_indexed_sessions(&connection)?;
    if scope == LocalSessionScope::Project {
        let project_roots = local_session_project_filter_roots(
            ctx,
            params.project_root.as_deref(),
            params.current_cwd.as_deref(),
        );
        sessions.retain(|session| {
            let session_cwd = local_session_normalized_path(Path::new(&session.cwd));
            project_roots
                .iter()
                .any(|root| local_session_normalized_path(root) == session_cwd)
        });
    }
    if let Some(search) = search {
        sessions.retain(|session| {
            session.title.to_ascii_lowercase().contains(search)
                || session.preview.to_ascii_lowercase().contains(search)
        });
    }
    sessions.sort_by(|left, right| {
        let order = match sort {
            LocalSessionSort::ModifiedAt => left.modified_at.cmp(&right.modified_at),
            LocalSessionSort::Title => left.title.to_lowercase().cmp(&right.title.to_lowercase()),
        };
        let order = if direction == SortDirection::Desc {
            order.reverse()
        } else {
            order
        };
        order.then_with(|| left.service_id.cmp(&right.service_id))
    });

    let source_revision = codex_index_source_revision(&sessions, &db_path);
    let query_digest = format!(
        "sha256:{}",
        trace_content_hash(&format!(
            "codex|{}|{}|{}|{}",
            scope.as_str(),
            params.project_root.as_deref().unwrap_or_default(),
            params.current_cwd.as_deref().unwrap_or_default(),
            search.unwrap_or_default()
        ))
    );
    let cursor = params
        .cursor
        .as_deref()
        .map(|value| decode_cursor(value, PREVIEW_METHOD, &query_digest))
        .transpose()?;
    if params
        .source_revision
        .as_deref()
        .is_some_and(|value| value != source_revision)
        || cursor
            .as_ref()
            .is_some_and(|value| value.source_revision != source_revision)
    {
        return Err(ServiceError::SourceChanged);
    }
    let start = cursor.as_ref().map_or_else(
        || params.offset.unwrap_or(0).min(sessions.len()),
        |cursor| {
            sessions.partition_point(|session| {
                session.modified_at > cursor.sort_value
                    || (session.modified_at == cursor.sort_value
                        && session.service_id <= cursor.stable_id)
            })
        },
    );
    let end = start.saturating_add(limit).min(sessions.len());
    let page = &sessions[start..end];
    let mut redactor = PromptRedactor::new(redaction_roots);
    let rows = page
        .iter()
        .map(|session| {
            let title = if session.title.trim().is_empty() {
                truncate_chars(&session.preview, 120)
            } else {
                truncate_chars(&session.title, 120)
            };
            let excerpt = truncate_chars(&redactor.redact(&session.preview), max_excerpt_chars);
            LocalSessionPreviewRow {
                id: session.service_id.clone(),
                title,
                source_kind: "codex-state-index".to_string(),
                scope: scope.as_str().to_string(),
                agent: Some(AgentId::Codex.as_str().to_string()),
                project_root: Some(redactor.redact(&session.cwd)),
                redacted_path: format!(
                    "<codex-session-index>#{}",
                    &session.native_id[..session.native_id.len().min(20)]
                ),
                modified_at: Some(session.modified_at),
                started_at: Some(session.started_at),
                ended_at: Some(session.modified_at),
                excerpt_char_count: excerpt.chars().count(),
                excerpt,
                user_message_count: 0,
                total_message_count: 0,
                tool_call_count: 0,
                skill_call_count: 0,
                content_hash: format!(
                    "sha256:{}",
                    trace_content_hash(&format!("{}|{}", session.native_id, session.modified_at))
                ),
                evidence_refs: vec!["codex:thread-index".to_string()],
                content_included: false,
                content_items: Vec::new(),
            }
        })
        .collect::<Vec<_>>();
    let has_more = end < sessions.len();
    let next_cursor = if has_more {
        page.last()
            .map(|session| {
                encode_cursor(&KeysetCursor {
                    version: 1,
                    method: PREVIEW_METHOD.to_string(),
                    query_digest: query_digest.clone(),
                    source_revision: source_revision.clone(),
                    sort_value: session.modified_at,
                    stable_id: session.service_id.clone(),
                    tie_breaker_digest: None,
                    accepted_count: Some(end),
                    processed_prefix_digest: None,
                    resolved_start_at: None,
                    resolved_end_at: None,
                })
            })
            .transpose()?
    } else {
        None
    };
    let count = rows.len();
    let redacted_db = redactor.redact(&db_path.to_string_lossy());
    Ok(Some(LocalSessionPreviewResult {
        generated_by: "local-v3.01-codex-index",
        authorized: true,
        authorization_required: false,
        roots: vec![LocalSessionPreviewRoot {
            root: redacted_db,
            status: "auto-discovered-read-only".to_string(),
            candidate_count: sessions.len(),
            blocker: None,
        }],
        count,
        total_candidate_count: sessions.len(),
        total_matched_count: sessions.len(),
        offset: start,
        limit,
        has_more,
        next_offset: (params.cursor.is_none() && has_more).then_some(end),
        next_cursor,
        source_revision: Some(source_revision),
        source_completeness: ListSourceCompleteness::Enumerable,
        incomplete_reason: None,
        candidate_set_truncated: false,
        user_message_count: 0,
        total_message_count: 0,
        tool_call_count: 0,
        skill_call_count: 0,
        skill_usage_rows: Vec::new(),
        session_rows: rows,
        gap_notes: Vec::new(),
        blocker_notes: Vec::new(),
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
    }))
}

fn codex_state_database_path(ctx: &AdapterContext) -> Option<PathBuf> {
    let codex_home = codex_home_dir(ctx);
    let preferred = codex_home.join("state_5.sqlite");
    if preferred.is_file() {
        return Some(preferred);
    }
    let mut candidates = fs::read_dir(codex_home)
        .ok()?
        .take(128)
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let name = entry.file_name();
            let name = name.to_str()?;
            let version = name
                .strip_prefix("state_")?
                .strip_suffix(".sqlite")?
                .parse::<u32>()
                .ok()?;
            entry
                .file_type()
                .ok()?
                .is_file()
                .then_some((version, entry.path()))
        })
        .collect::<Vec<_>>();
    candidates.sort_by_key(|(version, _)| *version);
    candidates.pop().map(|(_, path)| path)
}

fn load_codex_indexed_sessions(
    connection: &Connection,
) -> Result<Vec<CodexIndexedSession>, ServiceError> {
    let mut statement = connection
        .prepare(
            "SELECT id, title, cwd, rollout_path, preview, COALESCE(created_at_ms, created_at * 1000), COALESCE(updated_at_ms, updated_at * 1000) FROM threads WHERE archived = 0 AND source IN ('cli', 'vscode', 'appServer', 'unknown') ORDER BY COALESCE(updated_at_ms, updated_at * 1000) DESC, id ASC LIMIT ?1",
        )
        .map_err(sqlite_schema_error)?;
    let rows = statement
        .query_map([MAX_SQLITE_SESSIONS as i64], |row| {
            let native_id: String = row.get(0)?;
            let rollout_path = PathBuf::from(row.get::<_, String>(3)?);
            Ok(CodexIndexedSession {
                service_id: local_session_row_id(&rollout_path),
                native_id,
                title: row.get(1)?,
                cwd: row.get(2)?,
                rollout_path,
                preview: row.get(4)?,
                started_at: row.get(5)?,
                modified_at: row.get(6)?,
            })
        })
        .map_err(sqlite_schema_error)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(sqlite_schema_error)
}

fn codex_index_source_revision(sessions: &[CodexIndexedSession], db_path: &Path) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot.codex-thread-index.v1\0");
    hasher.update(trace_content_hash(&db_path.to_string_lossy()).as_bytes());
    for session in sessions {
        hasher.update([0]);
        hasher.update(session.service_id.as_bytes());
        hasher.update(session.modified_at.to_le_bytes());
        hasher.update(session.rollout_path.to_string_lossy().as_bytes());
    }
    format!("sha256:{}", hex_prefix(&hasher.finalize(), 64))
}

pub(super) fn preview_sqlite_session_resume(
    ctx: &AdapterContext,
    requested_agent: AgentId,
    session_id: &str,
    expected_source_revision: &str,
    project_filter_roots: &[PathBuf],
    redaction_roots: &[(String, &'static str)],
) -> Result<Option<SqliteSessionResumeSnapshot>, SessionResumePreviewError> {
    if requested_agent == AgentId::Codex {
        return preview_codex_session_resume(
            ctx,
            session_id,
            expected_source_revision,
            project_filter_roots,
            redaction_roots,
        );
    }
    let agent = match requested_agent {
        AgentId::Opencode => SqliteAgent::Opencode,
        AgentId::Hermes => SqliteAgent::Hermes,
        AgentId::Openclaw => SqliteAgent::Openclaw,
        _ => return Ok(None),
    };
    let db_sources = agent
        .database_paths(ctx)
        .into_iter()
        .filter(|(path, _)| path.is_file())
        .collect::<Vec<_>>();
    if db_sources.is_empty() {
        return Ok(None);
    }

    let mut sessions = Vec::new();
    let mut source_limited = false;
    for (db_path, project_root) in &db_sources {
        let connection =
            open_read_only_database(db_path).map_err(sqlite_resume_source_unavailable)?;
        let loaded = load_sessions(&connection, agent, db_path, project_root.as_deref())
            .map_err(sqlite_resume_source_unavailable)?;
        source_limited |= loaded.len() == MAX_SQLITE_SESSIONS;
        sessions.extend(loaded);
    }
    sort_sqlite_sessions(
        &mut sessions,
        LocalSessionSort::ModifiedAt,
        SortDirection::Desc,
    );
    let source_revision = sqlite_source_revision(&sessions, agent);
    if source_revision != expected_source_revision {
        return Err(SessionResumePreviewError::SourceChanged);
    }
    let Some(session) = sessions
        .into_iter()
        .find(|session| session.service_id == session_id)
    else {
        return Err(SessionResumePreviewError::SessionNotFound);
    };
    let project_matches_selected_context = session.project_root.as_ref().is_some_and(|root| {
        project_filter_roots
            .iter()
            .any(|project| local_session_paths_match(project, Path::new(root)))
    });
    let connection =
        open_read_only_database(&session.db_path).map_err(sqlite_resume_source_unavailable)?;
    let mut redactor = PromptRedactor::new(redaction_roots);
    let row = sqlite_session_row(
        &connection,
        agent,
        &session,
        LocalSessionScope::All,
        false,
        1_000,
        &mut redactor,
    )
    .map_err(sqlite_resume_source_unavailable)?;
    let source_count = db_sources.len();
    let coverage = if source_limited {
        SourceCoverage::incomplete(
            source_count,
            Some(source_count),
            ListIncompleteReason::SafetyBudget,
        )
    } else {
        SourceCoverage::enumerable(source_count, Some(source_count))
    };
    let resume_locator = match agent {
        SqliteAgent::Openclaw => {
            SessionNativeResumeLocator::OpenClawSessionKey(session.resume_locator)
        }
        _ => SessionNativeResumeLocator::NativeId(session.resume_locator),
    };
    Ok(Some(SqliteSessionResumeSnapshot {
        row,
        resume_locator,
        source_revision,
        coverage,
        project_matches_selected_context,
    }))
}

fn preview_codex_session_resume(
    ctx: &AdapterContext,
    session_id: &str,
    expected_source_revision: &str,
    project_filter_roots: &[PathBuf],
    redaction_roots: &[(String, &'static str)],
) -> Result<Option<SqliteSessionResumeSnapshot>, SessionResumePreviewError> {
    let Some(db_path) = codex_state_database_path(ctx) else {
        return Ok(None);
    };
    let connection = open_read_only_database(&db_path).map_err(sqlite_resume_source_unavailable)?;
    let sessions =
        load_codex_indexed_sessions(&connection).map_err(sqlite_resume_source_unavailable)?;
    let source_limited = sessions.len() == MAX_SQLITE_SESSIONS;
    let source_revision = codex_index_source_revision(&sessions, &db_path);
    if source_revision != expected_source_revision {
        return Err(SessionResumePreviewError::SourceChanged);
    }
    let Some(session) = sessions
        .into_iter()
        .find(|session| session.service_id == session_id)
    else {
        return Err(SessionResumePreviewError::SessionNotFound);
    };
    let project_matches_selected_context = project_filter_roots
        .iter()
        .any(|project| local_session_paths_match(project, Path::new(&session.cwd)));
    let mut redactor = PromptRedactor::new(redaction_roots);
    let title = if session.title.trim().is_empty() {
        truncate_chars(&session.preview, 120)
    } else {
        truncate_chars(&session.title, 120)
    };
    let excerpt = truncate_chars(&redactor.redact(&session.preview), 1_000);
    let row = LocalSessionPreviewRow {
        id: session.service_id,
        title,
        source_kind: "codex-state-index".to_string(),
        scope: LocalSessionScope::All.as_str().to_string(),
        agent: Some(AgentId::Codex.as_str().to_string()),
        project_root: Some(redactor.redact(&session.cwd)),
        redacted_path: format!(
            "<codex-session-index>#{}",
            &session.native_id[..session.native_id.len().min(20)]
        ),
        modified_at: Some(session.modified_at),
        started_at: Some(session.started_at),
        ended_at: Some(session.modified_at),
        excerpt_char_count: excerpt.chars().count(),
        excerpt,
        user_message_count: 0,
        total_message_count: 0,
        tool_call_count: 0,
        skill_call_count: 0,
        content_hash: format!(
            "sha256:{}",
            trace_content_hash(&format!("{}|{}", session.native_id, session.modified_at))
        ),
        evidence_refs: vec!["codex:thread-index".to_string()],
        content_included: false,
        content_items: Vec::new(),
    };
    let coverage = if source_limited {
        SourceCoverage::incomplete(1, Some(1), ListIncompleteReason::SafetyBudget)
    } else {
        SourceCoverage::enumerable(1, Some(1))
    };
    Ok(Some(SqliteSessionResumeSnapshot {
        row,
        resume_locator: SessionNativeResumeLocator::NativeId(session.native_id),
        source_revision,
        coverage,
        project_matches_selected_context,
    }))
}

fn sqlite_resume_source_unavailable(_error: ServiceError) -> SessionResumePreviewError {
    SessionResumePreviewError::SourceUnavailable
}

pub(super) fn list_sqlite_session_messages(
    ctx: &AdapterContext,
    params: &LocalSessionMessagePageParams,
    limit: usize,
    redaction_roots: &[(String, &'static str)],
) -> Result<Option<LocalSessionMessagePageResult>, ServiceError> {
    let Some(agent) = SqliteAgent::from_requested(params.agent.as_deref()) else {
        return Ok(None);
    };
    let db_sources = agent
        .database_paths(ctx)
        .into_iter()
        .filter(|(path, _)| path.is_file())
        .collect::<Vec<_>>();
    if db_sources.is_empty() {
        return Ok(None);
    }
    let mut selected = None;
    for (db_path, project_root) in &db_sources {
        let connection = open_read_only_database(db_path)?;
        if let Some(session) = load_sessions(&connection, agent, db_path, project_root.as_deref())?
            .into_iter()
            .find(|session| session.service_id == params.session_id)
        {
            selected = Some(session);
            break;
        }
    }
    let Some(session) = selected else {
        return Err(ServiceError::InvalidRequest(
            "selected session was not found in the current SQLite store".to_string(),
        ));
    };
    let connection = open_read_only_database(&session.db_path)?;
    connection
        .execute_batch("BEGIN DEFERRED")
        .map_err(sqlite_schema_error)?;
    let query_digest = sqlite_message_query_digest(agent, params);
    let cursor = params
        .cursor
        .as_deref()
        .map(|value| decode_cursor(value, MESSAGE_METHOD, &query_digest))
        .transpose()?;
    if cursor
        .as_ref()
        .is_some_and(|cursor| cursor.stable_id != session.service_id)
    {
        return Err(ServiceError::InvalidRequest(
            "message cursor is outside the selected SQLite session".to_string(),
        ));
    }
    let fixed_end_row = cursor
        .as_ref()
        .and_then(|cursor| cursor.resolved_end_at)
        .map(|value| {
            usize::try_from(value).map_err(|_| {
                ServiceError::InvalidRequest(
                    "message cursor has an invalid SQLite snapshot endpoint".to_string(),
                )
            })
        })
        .transpose()?;
    if cursor.is_some() && fixed_end_row.is_none() {
        return Err(ServiceError::InvalidRequest(
            "message cursor is missing the fixed SQLite snapshot endpoint".to_string(),
        ));
    }
    let snapshot_was_limited = cursor
        .as_ref()
        .and_then(|cursor| cursor.tie_breaker_digest.as_deref())
        == Some("safety_budget");
    let snapshot = sqlite_message_snapshot(
        &connection,
        agent,
        &session,
        fixed_end_row,
        snapshot_was_limited,
    )?;
    let source_revision = snapshot.source_revision.clone();
    if params
        .source_revision
        .as_deref()
        .is_some_and(|value| value != source_revision)
        || cursor
            .as_ref()
            .is_some_and(|value| value.source_revision != source_revision)
    {
        return Err(ServiceError::SourceChanged);
    }
    let start = cursor
        .as_ref()
        .map(|cursor| {
            usize::try_from(cursor.sort_value).map_err(|_| {
                ServiceError::InvalidRequest(
                    "message cursor has an invalid SQLite row offset".to_string(),
                )
            })
        })
        .transpose()?
        .unwrap_or(0);
    if start > snapshot.end_row {
        return Err(ServiceError::InvalidRequest(
            "message cursor is outside the selected SQLite session".to_string(),
        ));
    }
    let scan = load_final_message_page(&snapshot.rows, start, snapshot.end_row, limit);
    let mut redactor = PromptRedactor::new(redaction_roots);
    let items = scan
        .messages
        .iter()
        .map(|(row_index, message)| sqlite_content_item(message, *row_index, &mut redactor))
        .collect::<Vec<_>>();
    let has_more = scan.next_row < snapshot.end_row;
    let next_cursor = has_more
        .then(|| {
            encode_cursor(&KeysetCursor {
                version: 1,
                method: MESSAGE_METHOD.to_string(),
                query_digest,
                source_revision: source_revision.clone(),
                sort_value: i64::try_from(scan.next_row).map_err(|_| {
                    ServiceError::InvalidRequest("SQLite message offset overflow".to_string())
                })?,
                stable_id: session.service_id.clone(),
                tie_breaker_digest: (snapshot.source_completeness
                    == ListSourceCompleteness::Limited)
                    .then_some("safety_budget".to_string()),
                accepted_count: Some(
                    cursor
                        .as_ref()
                        .and_then(|cursor| cursor.accepted_count)
                        .unwrap_or_default()
                        .saturating_add(items.len()),
                ),
                processed_prefix_digest: Some(source_revision.clone()),
                resolved_start_at: None,
                resolved_end_at: Some(i64::try_from(snapshot.end_row).unwrap_or(i64::MAX)),
            })
        })
        .transpose()?;
    let returned_count = items.len();
    Ok(Some(LocalSessionMessagePageResult {
        generated_by: "local-v3.00-sqlite",
        session_id: session.service_id.clone(),
        content_items: items,
        returned_count,
        total_count: (!has_more).then_some(
            cursor
                .as_ref()
                .and_then(|cursor| cursor.accepted_count)
                .unwrap_or_default()
                .saturating_add(returned_count),
        ),
        has_more,
        next_cursor,
        source_revision,
        source_completeness: snapshot.source_completeness,
        incomplete_reason: snapshot.incomplete_reason,
        scanned_bytes: scan.scanned_bytes as u64,
        scanned_through_bytes: snapshot
            .row_sizes
            .iter()
            .take(scan.next_row)
            .fold(0_u64, |total, size| total.saturating_add(*size as u64)),
        snapshot_bytes: snapshot.snapshot_bytes as u64,
        redaction_summary: local_preview_redaction_summary_from(redactor.summary()),
        safety_flags: local_preview_safety_flags(),
        read_only: true,
        provider_request_sent: false,
        raw_prompt_persisted: false,
        raw_response_persisted: false,
        raw_trace_persisted: false,
    }))
}

fn open_read_only_database(path: &Path) -> Result<Connection, ServiceError> {
    Connection::open_with_flags(
        path,
        OpenFlags::SQLITE_OPEN_READ_ONLY | OpenFlags::SQLITE_OPEN_NO_MUTEX,
    )
    .map_err(|_| {
        ServiceError::InvalidRequest(
            "current local session database could not be opened read-only".to_string(),
        )
    })
}

fn load_sessions(
    connection: &Connection,
    agent: SqliteAgent,
    db_path: &Path,
    project_root: Option<&str>,
) -> Result<Vec<SqliteSession>, ServiceError> {
    let sql = match agent {
        SqliteAgent::Opencode if sqlite_table_has_column(connection, "session", "time_archived")? => "SELECT id, title, directory, time_created, time_updated, NULL, 0, 0 FROM session WHERE parent_id IS NULL AND time_archived IS NULL ORDER BY time_updated DESC, id ASC LIMIT ?1",
        SqliteAgent::Opencode => "SELECT id, title, directory, time_created, time_updated, NULL, 0, 0 FROM session WHERE parent_id IS NULL ORDER BY time_updated DESC, id ASC LIMIT ?1",
        SqliteAgent::Hermes => "SELECT id, COALESCE(title, id), NULL, CAST(started_at * 1000 AS INTEGER), CAST(COALESCE(ended_at, started_at) * 1000 AS INTEGER), CAST(ended_at * 1000 AS INTEGER), COALESCE(message_count, 0), COALESCE(tool_call_count, 0) FROM sessions WHERE source NOT IN ('cron', 'batch', 'subagent', 'memory', 'memory_consolidation') ORDER BY COALESCE(ended_at, started_at) DESC, id ASC LIMIT ?1",
        SqliteAgent::Openclaw => "SELECT s.session_id, COALESCE(NULLIF(s.display_name, ''), s.session_key), NULL, s.started_at, s.updated_at, s.ended_at, 0, 0, s.session_key FROM sessions s WHERE COALESCE(s.status, 'active') NOT IN ('archived', 'deleted') AND EXISTS (SELECT 1 FROM session_routes r WHERE r.session_id = s.session_id) AND NOT EXISTS (SELECT 1 FROM session_entries e WHERE e.session_id = s.session_id AND json_extract(e.entry_json, '$.archivedAt') IS NOT NULL) AND lower(s.session_key) NOT LIKE 'cron:%' AND lower(s.session_key) NOT LIKE 'hook:%' AND lower(s.session_key) NOT LIKE '%:subagent:%' AND lower(s.session_key) NOT LIKE '%:cron:%' AND lower(s.session_key) NOT LIKE '%:hook:%' AND lower(s.session_key) NOT LIKE '%:heartbeat:%' AND lower(s.session_key) NOT LIKE '%:acp:%' ORDER BY s.updated_at DESC, s.session_id ASC LIMIT ?1",
    };
    let mut statement = connection.prepare(sql).map_err(sqlite_schema_error)?;
    let rows = statement
        .query_map([MAX_SQLITE_SESSIONS as i64], |row| {
            let native_id: String = row.get(0)?;
            let resume_locator = if agent == SqliteAgent::Openclaw {
                row.get(8)?
            } else {
                native_id.clone()
            };
            Ok(SqliteSession {
                db_path: db_path.to_path_buf(),
                service_id: sqlite_service_id(agent, db_path, &native_id),
                native_id,
                resume_locator,
                title: row.get(1)?,
                project_root: project_root.map(ToString::to_string).or(row.get(2)?),
                started_at: row.get(3)?,
                modified_at: row.get::<_, Option<i64>>(4)?.unwrap_or_default(),
                ended_at: row.get(5)?,
                declared_message_count: row.get::<_, i64>(6)?.max(0) as usize,
                declared_tool_count: row.get::<_, i64>(7)?.max(0) as usize,
            })
        })
        .map_err(sqlite_schema_error)?;
    let sessions = rows
        .collect::<Result<Vec<_>, _>>()
        .map_err(sqlite_schema_error)?;
    Ok(sessions)
}

fn sqlite_table_has_column(
    connection: &Connection,
    table: &str,
    column: &str,
) -> Result<bool, ServiceError> {
    let mut statement = connection
        .prepare(&format!("PRAGMA table_info({table})"))
        .map_err(sqlite_schema_error)?;
    let names = statement
        .query_map([], |row| row.get::<_, String>(1))
        .map_err(sqlite_schema_error)?;
    for name in names {
        if name.map_err(sqlite_schema_error)? == column {
            return Ok(true);
        }
    }
    Ok(false)
}

fn load_messages(
    connection: &Connection,
    agent: SqliteAgent,
    session_id: &str,
) -> Result<Vec<SqliteMessage>, ServiceError> {
    match agent {
        SqliteAgent::Opencode => load_opencode_messages(connection, session_id),
        SqliteAgent::Hermes => load_hermes_messages(connection, session_id),
        SqliteAgent::Openclaw => load_openclaw_messages(connection, session_id),
    }
}

struct SqliteMessagePageScan {
    messages: Vec<(usize, SqliteMessage)>,
    next_row: usize,
    scanned_bytes: usize,
}

struct SqliteMessageSnapshot {
    end_row: usize,
    rows: Vec<SqliteMessageRow>,
    row_sizes: Vec<usize>,
    snapshot_bytes: usize,
    source_revision: String,
    source_completeness: ListSourceCompleteness,
    incomplete_reason: Option<ListIncompleteReason>,
}

fn sqlite_message_snapshot(
    connection: &Connection,
    agent: SqliteAgent,
    session: &SqliteSession,
    fixed_end_row: Option<usize>,
    was_limited: bool,
) -> Result<SqliteMessageSnapshot, ServiceError> {
    let current_rows = sqlite_message_row_count(connection, agent, &session.native_id)?;
    if fixed_end_row.is_some_and(|end_row| current_rows < end_row) {
        return Err(ServiceError::SourceChanged);
    }
    let requested_end = fixed_end_row.unwrap_or(current_rows);
    let row_bounded_end = requested_end.min(SQLITE_MESSAGE_SNAPSHOT_ROWS);
    let mut snapshot_rows = Vec::with_capacity(row_bounded_end);
    let mut row_sizes = Vec::with_capacity(row_bounded_end);
    let mut snapshot_bytes = 0usize;
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot.sqlite-session-message-snapshot.v2\0");
    hasher.update(agent.id().as_bytes());
    hasher.update([0]);
    hasher.update(trace_content_hash(&session.db_path.to_string_lossy()).as_bytes());
    hasher.update([0]);
    hasher.update(session.service_id.as_bytes());

    let mut offset = 0usize;
    let mut byte_limited = false;
    while offset < row_bounded_end {
        let raw_limit = SQLITE_MESSAGE_SCAN_ROWS.min(row_bounded_end.saturating_sub(offset));
        let sizes =
            load_message_row_sizes(connection, agent, &session.native_id, offset, raw_limit)?;
        if sizes.is_empty() {
            return Err(ServiceError::SourceChanged);
        }
        let mut retained = 0usize;
        for size in &sizes {
            if snapshot_bytes.saturating_add(*size) > SQLITE_MESSAGE_SNAPSHOT_BYTES {
                byte_limited = true;
                break;
            }
            snapshot_bytes = snapshot_bytes.saturating_add(*size);
            retained = retained.saturating_add(1);
        }
        if retained == 0 {
            break;
        }
        let rows = load_message_rows(connection, agent, &session.native_id, offset, retained)?;
        if rows.len() != retained {
            return Err(ServiceError::SourceChanged);
        }
        for (row, expected_size) in rows.into_iter().zip(sizes) {
            if row.raw_bytes != expected_size {
                return Err(ServiceError::SourceChanged);
            }
            hasher.update((offset as u64).to_le_bytes());
            hasher.update(row.raw_digest);
            row_sizes.push(row.raw_bytes);
            snapshot_rows.push(row);
            offset = offset.saturating_add(1);
        }
        if byte_limited {
            break;
        }
    }
    let end_row = offset;
    hasher.update((end_row as u64).to_le_bytes());
    hasher.update((snapshot_bytes as u64).to_le_bytes());
    let source_revision = format!("sha256:{}", hex_prefix(&hasher.finalize(), 64));
    let incomplete = was_limited
        || (fixed_end_row.is_none()
            && (current_rows > end_row || requested_end > row_bounded_end || byte_limited));
    Ok(SqliteMessageSnapshot {
        end_row,
        rows: snapshot_rows,
        row_sizes,
        snapshot_bytes,
        source_revision,
        source_completeness: if incomplete {
            ListSourceCompleteness::Limited
        } else {
            ListSourceCompleteness::Enumerable
        },
        incomplete_reason: incomplete.then_some(ListIncompleteReason::SafetyBudget),
    })
}

fn load_final_message_page(
    snapshot_rows: &[SqliteMessageRow],
    start_row: usize,
    end_row: usize,
    limit: usize,
) -> SqliteMessagePageScan {
    let mut next_row = start_row.min(end_row);
    let mut messages = Vec::with_capacity(limit);
    let mut scanned_rows = 0usize;
    let mut scanned_bytes = 0usize;
    let mut byte_budget_reached = false;
    while next_row < end_row
        && messages.len() < limit
        && scanned_rows < SQLITE_MESSAGE_SCAN_ROWS
        && scanned_bytes < SQLITE_MESSAGE_SCAN_BYTES
    {
        let page_end = end_row
            .min(next_row.saturating_add(SQLITE_MESSAGE_SCAN_ROWS.saturating_sub(scanned_rows)));
        for row in &snapshot_rows[next_row..page_end] {
            if scanned_rows > 0
                && scanned_bytes.saturating_add(row.raw_bytes) > SQLITE_MESSAGE_SCAN_BYTES
            {
                byte_budget_reached = true;
                break;
            }
            let row_index = next_row;
            next_row = next_row.saturating_add(1);
            scanned_rows = scanned_rows.saturating_add(1);
            scanned_bytes = scanned_bytes.saturating_add(row.raw_bytes);
            if matches!(row.message.kind.as_str(), "user_message" | "agent_reply") {
                messages.push((row_index, row.message.clone()));
                if messages.len() == limit {
                    break;
                }
            }
        }
        if scanned_rows >= SQLITE_MESSAGE_SCAN_ROWS
            || scanned_bytes >= SQLITE_MESSAGE_SCAN_BYTES
            || messages.len() >= limit
            || byte_budget_reached
        {
            break;
        }
    }
    SqliteMessagePageScan {
        messages,
        next_row,
        scanned_bytes,
    }
}

fn sqlite_message_row_count(
    connection: &Connection,
    agent: SqliteAgent,
    session_id: &str,
) -> Result<usize, ServiceError> {
    let sql = match agent {
        SqliteAgent::Opencode => {
            "SELECT COUNT(*) FROM message m JOIN part p ON p.message_id = m.id WHERE m.session_id = ?1"
        }
        SqliteAgent::Hermes => "SELECT COUNT(*) FROM messages WHERE session_id = ?1",
        SqliteAgent::Openclaw => "SELECT COUNT(*) FROM transcript_events WHERE session_id = ?1",
    };
    let count = connection
        .query_row(sql, [session_id], |row| row.get::<_, i64>(0))
        .map_err(sqlite_schema_error)?;
    Ok(count.max(0) as usize)
}

fn load_message_row_sizes(
    connection: &Connection,
    agent: SqliteAgent,
    session_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<usize>, ServiceError> {
    let sql = match agent {
        SqliteAgent::Opencode => {
            "SELECT length(CAST(m.id AS BLOB)) + length(CAST(p.id AS BLOB)) + length(CAST(m.data AS BLOB)) + length(CAST(p.data AS BLOB)) + 8 FROM message m JOIN part p ON p.message_id = m.id WHERE m.session_id = ?1 ORDER BY m.time_created ASC, p.time_created ASC, p.id ASC LIMIT ?2 OFFSET ?3"
        }
        SqliteAgent::Hermes => {
            "SELECT 16 + length(CAST(role AS BLOB)) + length(CAST(COALESCE(content, '') AS BLOB)) + length(CAST(COALESCE(tool_name, '') AS BLOB)) + length(CAST(COALESCE(tool_calls, '') AS BLOB)) + length(CAST(COALESCE(reasoning, '') AS BLOB)) FROM messages WHERE session_id = ?1 ORDER BY timestamp ASC, id ASC LIMIT ?2 OFFSET ?3"
        }
        SqliteAgent::Openclaw => {
            "SELECT 16 + length(CAST(event_json AS BLOB)) FROM transcript_events WHERE session_id = ?1 ORDER BY seq ASC LIMIT ?2 OFFSET ?3"
        }
    };
    let mut statement = connection.prepare(sql).map_err(sqlite_schema_error)?;
    let rows = statement
        .query_map((session_id, limit as i64, offset as i64), |row| {
            row.get::<_, i64>(0)
        })
        .map_err(sqlite_schema_error)?;
    rows.map(|row| {
        let size = row.map_err(sqlite_schema_error)?.max(0);
        usize::try_from(size).map_err(|_| {
            ServiceError::InvalidRequest(
                "current local session row exceeds the supported size".to_string(),
            )
        })
    })
    .collect()
}

fn load_message_rows(
    connection: &Connection,
    agent: SqliteAgent,
    session_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<SqliteMessageRow>, ServiceError> {
    match agent {
        SqliteAgent::Opencode => load_opencode_message_rows(connection, session_id, offset, limit),
        SqliteAgent::Hermes => load_hermes_message_rows(connection, session_id, offset, limit),
        SqliteAgent::Openclaw => {
            load_openclaw_raw_message_rows(connection, session_id, offset, limit)
        }
    }
}

fn load_openclaw_message_rows(
    connection: &Connection,
    session_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<SqliteMessage>, ServiceError> {
    Ok(
        load_openclaw_raw_message_rows(connection, session_id, offset, limit)?
            .into_iter()
            .map(|row| row.message)
            .collect(),
    )
}

fn load_openclaw_raw_message_rows(
    connection: &Connection,
    session_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<SqliteMessageRow>, ServiceError> {
    let mut statement = connection
        .prepare("SELECT seq, event_json, created_at FROM transcript_events WHERE session_id = ?1 ORDER BY seq ASC LIMIT ?2 OFFSET ?3")
        .map_err(sqlite_schema_error)?;
    let rows = statement
        .query_map((session_id, limit as i64, offset as i64), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, Option<i64>>(2)?,
            ))
        })
        .map_err(sqlite_schema_error)?;
    rows.map(|row| {
        let (seq, event_json, timestamp) = row.map_err(sqlite_schema_error)?;
        Ok(SqliteMessageRow {
            message: openclaw_message_from_json(&event_json, timestamp),
            raw_digest: sqlite_raw_message_digest(&[
                &seq.to_le_bytes(),
                event_json.as_bytes(),
                &timestamp.unwrap_or_default().to_le_bytes(),
            ]),
            raw_bytes: event_json
                .len()
                .saturating_add(2 * std::mem::size_of::<i64>()),
        })
    })
    .collect()
}

fn openclaw_message_from_json(event_json: &str, timestamp: Option<i64>) -> SqliteMessage {
    let event = serde_json::from_str::<serde_json::Value>(event_json).unwrap_or_default();
    let payload = event
        .get("message")
        .or_else(|| {
            event
                .get("payload")
                .and_then(|payload| payload.get("message"))
        })
        .unwrap_or(&event);
    let role = payload
        .get("role")
        .or_else(|| event.get("role"))
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default()
        .to_string();
    let content = payload
        .get("content")
        .or_else(|| payload.get("text"))
        .or_else(|| event.get("content"));
    let (text, has_tool, has_thinking) = openclaw_content_projection(content);
    let kind = if role == "user" && !text.is_empty() {
        "user_message"
    } else if role == "assistant" && !text.is_empty() {
        "agent_reply"
    } else if has_thinking {
        "thinking"
    } else if has_tool {
        "tool_call"
    } else {
        "ignored"
    };
    SqliteMessage {
        role,
        text,
        timestamp,
        kind: kind.to_string(),
    }
}

fn openclaw_content_projection(content: Option<&serde_json::Value>) -> (String, bool, bool) {
    let Some(content) = content else {
        return (String::new(), false, false);
    };
    match content {
        serde_json::Value::String(text) => (text.clone(), false, false),
        serde_json::Value::Array(items) => {
            let mut text = Vec::new();
            let mut has_tool = false;
            let mut has_thinking = false;
            for item in items {
                let kind = item
                    .get("type")
                    .and_then(serde_json::Value::as_str)
                    .unwrap_or_default();
                has_tool |= matches!(
                    kind,
                    "tool" | "tool_call" | "toolCall" | "tool_use" | "toolUse"
                );
                has_thinking |= matches!(kind, "thinking" | "reasoning");
                if matches!(kind, "text" | "output_text" | "thinking" | "reasoning") {
                    if let Some(value) = item
                        .get("text")
                        .or_else(|| item.get("content"))
                        .and_then(serde_json::Value::as_str)
                    {
                        if !value.is_empty() {
                            text.push(value);
                        }
                    }
                }
            }
            (text.join("\n"), has_tool, has_thinking)
        }
        _ => (String::new(), false, false),
    }
}

fn load_opencode_message_rows(
    connection: &Connection,
    session_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<SqliteMessageRow>, ServiceError> {
    let mut statement = connection
        .prepare("SELECT m.id, p.id, m.data, p.data, p.time_created FROM message m JOIN part p ON p.message_id = m.id WHERE m.session_id = ?1 ORDER BY m.time_created ASC, p.time_created ASC, p.id ASC LIMIT ?2 OFFSET ?3")
        .map_err(sqlite_schema_error)?;
    let rows = statement
        .query_map((session_id, limit as i64, offset as i64), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, i64>(4)?,
            ))
        })
        .map_err(sqlite_schema_error)?;
    rows.map(|row| {
        let (message_id, part_id, message_json, part_json, timestamp) =
            row.map_err(sqlite_schema_error)?;
        Ok(SqliteMessageRow {
            message: opencode_message_from_json(&message_json, &part_json, timestamp),
            raw_digest: sqlite_raw_message_digest(&[
                message_id.as_bytes(),
                part_id.as_bytes(),
                message_json.as_bytes(),
                part_json.as_bytes(),
                &timestamp.to_le_bytes(),
            ]),
            raw_bytes: message_id
                .len()
                .saturating_add(part_id.len())
                .saturating_add(message_json.len())
                .saturating_add(part_json.len())
                .saturating_add(std::mem::size_of::<i64>()),
        })
    })
    .collect()
}

fn opencode_message_from_json(
    message_json: &str,
    part_json: &str,
    timestamp: i64,
) -> SqliteMessage {
    let message = serde_json::from_str::<serde_json::Value>(message_json).unwrap_or_default();
    let part = serde_json::from_str::<serde_json::Value>(part_json).unwrap_or_default();
    let role = message
        .get("role")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let part_type = part
        .get("type")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let text = part
        .get("text")
        .and_then(serde_json::Value::as_str)
        .unwrap_or_default();
    let kind = match (role, part_type) {
        ("user", "text") if !text.is_empty() => "user_message",
        ("assistant", "text") if !text.is_empty() => "agent_reply",
        (_, "reasoning") if !text.is_empty() => "thinking",
        (_, "tool") => "tool_call",
        _ => "ignored",
    };
    SqliteMessage {
        role: role.to_string(),
        text: text.to_string(),
        timestamp: Some(timestamp),
        kind: kind.to_string(),
    }
}

fn load_hermes_message_rows(
    connection: &Connection,
    session_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<SqliteMessageRow>, ServiceError> {
    let mut statement = connection
        .prepare("SELECT id, role, COALESCE(content, ''), CAST(timestamp * 1000 AS INTEGER), COALESCE(tool_name, ''), COALESCE(tool_calls, ''), COALESCE(reasoning, '') FROM messages WHERE session_id = ?1 ORDER BY timestamp ASC, id ASC LIMIT ?2 OFFSET ?3")
        .map_err(sqlite_schema_error)?;
    let rows = statement
        .query_map((session_id, limit as i64, offset as i64), |row| {
            Ok((
                row.get::<_, i64>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, String>(2)?,
                row.get::<_, i64>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
                row.get::<_, String>(6)?,
            ))
        })
        .map_err(sqlite_schema_error)?;
    rows.map(|row| {
        let (row_id, role, content, timestamp, tool_name, tool_calls, reasoning) =
            row.map_err(sqlite_schema_error)?;
        let raw_digest = sqlite_raw_message_digest(&[
            &row_id.to_le_bytes(),
            role.as_bytes(),
            content.as_bytes(),
            &timestamp.to_le_bytes(),
            tool_name.as_bytes(),
            tool_calls.as_bytes(),
            reasoning.as_bytes(),
        ]);
        let raw_bytes = std::mem::size_of::<i64>()
            .saturating_add(role.len())
            .saturating_add(content.len())
            .saturating_add(std::mem::size_of::<i64>())
            .saturating_add(tool_name.len())
            .saturating_add(tool_calls.len())
            .saturating_add(reasoning.len());
        let (kind, text) = if role == "user" && !content.is_empty() {
            ("user_message", content)
        } else if role == "assistant" && !content.is_empty() {
            ("agent_reply", content)
        } else if !reasoning.is_empty() {
            ("thinking", reasoning)
        } else if !tool_name.is_empty() || !tool_calls.is_empty() {
            ("tool_call", tool_name)
        } else {
            ("ignored", String::new())
        };
        Ok(SqliteMessageRow {
            message: SqliteMessage {
                role,
                text,
                timestamp: Some(timestamp),
                kind: kind.to_string(),
            },
            raw_digest,
            raw_bytes,
        })
    })
    .collect()
}

fn sqlite_raw_message_digest(parts: &[&[u8]]) -> [u8; 32] {
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot.sqlite-message-row.v1\0");
    for part in parts {
        hasher.update((part.len() as u64).to_le_bytes());
        hasher.update(part);
    }
    hasher.finalize().into()
}

fn load_opencode_messages(
    connection: &Connection,
    session_id: &str,
) -> Result<Vec<SqliteMessage>, ServiceError> {
    let mut statement = connection
        .prepare("SELECT m.data, p.data, p.time_created FROM message m JOIN part p ON p.message_id = m.id WHERE m.session_id = ?1 ORDER BY m.time_created ASC, p.time_created ASC, p.id ASC LIMIT ?2")
        .map_err(sqlite_schema_error)?;
    let rows = statement
        .query_map((session_id, MAX_SQLITE_PREVIEW_MESSAGES as i64), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(sqlite_schema_error)?;
    let mut messages = Vec::new();
    let mut retained_bytes = 0usize;
    for row in rows {
        let (message_json, part_json, timestamp) = row.map_err(sqlite_schema_error)?;
        let message = opencode_message_from_json(&message_json, &part_json, timestamp);
        let role = message.role;
        let text = message.text;
        let kind = message.kind;
        if kind == "ignored" {
            continue;
        }
        if text.is_empty() && kind != "tool_call" {
            continue;
        }
        retained_bytes = retained_bytes.saturating_add(text.len());
        if retained_bytes > MAX_SQLITE_TEXT_BYTES {
            break;
        }
        messages.push(SqliteMessage {
            role: role.to_string(),
            text: text.to_string(),
            timestamp: Some(timestamp),
            kind,
        });
    }
    Ok(messages)
}

fn load_hermes_messages(
    connection: &Connection,
    session_id: &str,
) -> Result<Vec<SqliteMessage>, ServiceError> {
    let mut statement = connection
        .prepare("SELECT role, COALESCE(content, ''), CAST(timestamp * 1000 AS INTEGER), COALESCE(tool_name, ''), COALESCE(tool_calls, ''), COALESCE(reasoning, '') FROM messages WHERE session_id = ?1 ORDER BY timestamp ASC, id ASC LIMIT ?2")
        .map_err(sqlite_schema_error)?;
    let rows = statement
        .query_map((session_id, MAX_SQLITE_PREVIEW_MESSAGES as i64), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
                row.get::<_, String>(3)?,
                row.get::<_, String>(4)?,
                row.get::<_, String>(5)?,
            ))
        })
        .map_err(sqlite_schema_error)?;
    let mut messages = Vec::new();
    let mut retained_bytes = 0usize;
    for row in rows {
        let (role, content, timestamp, tool_name, tool_calls, reasoning) =
            row.map_err(sqlite_schema_error)?;
        let (kind, text) = if role == "user" && !content.is_empty() {
            ("user_message", content)
        } else if role == "assistant" && !content.is_empty() {
            ("agent_reply", content)
        } else if !reasoning.is_empty() {
            ("thinking", reasoning)
        } else if !tool_name.is_empty() || !tool_calls.is_empty() {
            ("tool_call", tool_name)
        } else {
            continue;
        };
        retained_bytes = retained_bytes.saturating_add(text.len());
        if retained_bytes > MAX_SQLITE_TEXT_BYTES {
            break;
        }
        messages.push(SqliteMessage {
            role,
            text,
            timestamp: Some(timestamp),
            kind: kind.to_string(),
        });
    }
    Ok(messages)
}

fn load_openclaw_messages(
    connection: &Connection,
    session_id: &str,
) -> Result<Vec<SqliteMessage>, ServiceError> {
    let rows = load_openclaw_message_rows(connection, session_id, 0, MAX_SQLITE_PREVIEW_MESSAGES)?;
    let mut messages = Vec::new();
    let mut retained_bytes = 0usize;
    for message in rows {
        if message.kind == "ignored" || (message.text.is_empty() && message.kind != "tool_call") {
            continue;
        }
        retained_bytes = retained_bytes.saturating_add(message.text.len());
        if retained_bytes > MAX_SQLITE_TEXT_BYTES {
            break;
        }
        messages.push(message);
    }
    Ok(messages)
}

fn sqlite_session_row(
    connection: &Connection,
    agent: SqliteAgent,
    session: &SqliteSession,
    scope: LocalSessionScope,
    include_content_items: bool,
    max_excerpt_chars: usize,
    redactor: &mut PromptRedactor<'_>,
) -> Result<LocalSessionPreviewRow, ServiceError> {
    if connection.is_autocommit() {
        connection
            .execute_batch("BEGIN DEFERRED")
            .map_err(sqlite_schema_error)?;
    }
    let (user_message_count, _agent_message_count, exact_total_count, exact_tool_count) =
        sqlite_session_counts(connection, agent, &session.native_id)?;
    let excerpt_message = load_first_final_message(connection, agent, &session.native_id)?;
    let excerpt = excerpt_message
        .as_ref()
        .map(|message| truncate_chars(&redactor.redact(&message.text), max_excerpt_chars))
        .unwrap_or_default();
    let content_items = if include_content_items {
        load_messages(connection, agent, &session.native_id)?
            .iter()
            .filter(|message| matches!(message.kind.as_str(), "thinking" | "tool_call"))
            .enumerate()
            .map(|(index, message)| sqlite_content_item(message, index, redactor))
            .collect()
    } else {
        Vec::new()
    };
    let redacted_project = session
        .project_root
        .as_ref()
        .map(|path| redactor.redact(path));
    let total_message_count = exact_total_count.max(session.declared_message_count);
    let tool_call_count = exact_tool_count.max(session.declared_tool_count);
    Ok(LocalSessionPreviewRow {
        id: session.service_id.clone(),
        title: truncate_chars(&session.title, 120),
        source_kind: agent.source_kind().to_string(),
        scope: scope.as_str().to_string(),
        agent: Some(agent.id().to_string()),
        project_root: redacted_project,
        redacted_path: format!(
            "<{}-session-db>#{}",
            agent.id(),
            &session.service_id[..session.service_id.len().min(20)]
        ),
        modified_at: Some(session.modified_at),
        started_at: session.started_at,
        ended_at: session.ended_at.or(Some(session.modified_at)),
        excerpt_char_count: excerpt.chars().count(),
        excerpt,
        user_message_count,
        total_message_count,
        tool_call_count,
        skill_call_count: 0,
        content_hash: sqlite_message_revision(connection, agent, session)?,
        evidence_refs: vec![format!("{}:sqlite-session", agent.id())],
        content_included: include_content_items,
        content_items,
    })
}

fn sqlite_session_counts(
    connection: &Connection,
    agent: SqliteAgent,
    session_id: &str,
) -> Result<(usize, usize, usize, usize), ServiceError> {
    let sql = match agent {
        SqliteAgent::Opencode => {
            "SELECT COUNT(DISTINCT CASE WHEN json_extract(m.data, '$.role') = 'user' AND json_extract(p.data, '$.type') = 'text' AND COALESCE(json_extract(p.data, '$.text'), '') <> '' THEN m.id END), COUNT(DISTINCT CASE WHEN json_extract(m.data, '$.role') = 'assistant' AND json_extract(p.data, '$.type') = 'text' AND COALESCE(json_extract(p.data, '$.text'), '') <> '' THEN m.id END), COUNT(DISTINCT m.id), SUM(CASE WHEN json_extract(p.data, '$.type') = 'tool' THEN 1 ELSE 0 END) FROM message m JOIN part p ON p.message_id = m.id WHERE m.session_id = ?1"
        }
        SqliteAgent::Hermes => {
            "SELECT SUM(CASE WHEN role = 'user' AND COALESCE(content, '') <> '' THEN 1 ELSE 0 END), SUM(CASE WHEN role = 'assistant' AND COALESCE(content, '') <> '' THEN 1 ELSE 0 END), COUNT(*), SUM(CASE WHEN COALESCE(tool_name, '') <> '' OR COALESCE(tool_calls, '') <> '' THEN 1 ELSE 0 END) FROM messages WHERE session_id = ?1"
        }
        SqliteAgent::Openclaw => {
            let messages = load_openclaw_messages(connection, session_id)?;
            let user = messages
                .iter()
                .filter(|message| message.kind == "user_message")
                .count();
            let agent = messages
                .iter()
                .filter(|message| message.kind == "agent_reply")
                .count();
            let tools = messages
                .iter()
                .filter(|message| message.kind == "tool_call")
                .count();
            return Ok((user, agent, messages.len(), tools));
        }
    };
    let counts = connection
        .query_row(sql, [session_id], |row| {
            Ok((
                row.get::<_, Option<i64>>(0)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(1)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(2)?.unwrap_or_default(),
                row.get::<_, Option<i64>>(3)?.unwrap_or_default(),
            ))
        })
        .map_err(sqlite_schema_error)?;
    Ok((
        counts.0.max(0) as usize,
        counts.1.max(0) as usize,
        counts.2.max(0) as usize,
        counts.3.max(0) as usize,
    ))
}

fn load_first_final_message(
    connection: &Connection,
    agent: SqliteAgent,
    session_id: &str,
) -> Result<Option<SqliteMessage>, ServiceError> {
    match agent {
        SqliteAgent::Opencode => {
            let mut statement = connection
                .prepare("SELECT m.data, p.data, p.time_created FROM message m JOIN part p ON p.message_id = m.id WHERE m.session_id = ?1 AND json_extract(m.data, '$.role') IN ('user', 'assistant') AND json_extract(p.data, '$.type') = 'text' AND COALESCE(json_extract(p.data, '$.text'), '') <> '' ORDER BY m.time_created ASC, p.time_created ASC, p.id ASC LIMIT 1")
                .map_err(sqlite_schema_error)?;
            let mut rows = statement.query([session_id]).map_err(sqlite_schema_error)?;
            let Some(row) = rows.next().map_err(sqlite_schema_error)? else {
                return Ok(None);
            };
            let message_json: String = row.get(0).map_err(sqlite_schema_error)?;
            let part_json: String = row.get(1).map_err(sqlite_schema_error)?;
            let timestamp: i64 = row.get(2).map_err(sqlite_schema_error)?;
            Ok(Some(opencode_message_from_json(
                &message_json,
                &part_json,
                timestamp,
            )))
        }
        SqliteAgent::Hermes => {
            let mut statement = connection
                .prepare("SELECT role, content, CAST(timestamp * 1000 AS INTEGER) FROM messages WHERE session_id = ?1 AND role IN ('user', 'assistant') AND COALESCE(content, '') <> '' ORDER BY timestamp ASC, id ASC LIMIT 1")
                .map_err(sqlite_schema_error)?;
            let mut rows = statement.query([session_id]).map_err(sqlite_schema_error)?;
            let Some(row) = rows.next().map_err(sqlite_schema_error)? else {
                return Ok(None);
            };
            let role: String = row.get(0).map_err(sqlite_schema_error)?;
            let text: String = row.get(1).map_err(sqlite_schema_error)?;
            let timestamp: i64 = row.get(2).map_err(sqlite_schema_error)?;
            Ok(Some(SqliteMessage {
                kind: if role == "user" {
                    "user_message".to_string()
                } else {
                    "agent_reply".to_string()
                },
                role,
                text,
                timestamp: Some(timestamp),
            }))
        }
        SqliteAgent::Openclaw => Ok(load_openclaw_message_rows(
            connection,
            session_id,
            0,
            MAX_SQLITE_PREVIEW_MESSAGES,
        )?
        .into_iter()
        .find(|message| matches!(message.kind.as_str(), "user_message" | "agent_reply"))),
    }
}

fn sqlite_content_item(
    message: &SqliteMessage,
    index: usize,
    redactor: &mut PromptRedactor<'_>,
) -> LocalSessionContentItem {
    let text = redactor.redact(&message.text);
    let title = match message.kind.as_str() {
        "user_message" => "User",
        "agent_reply" => "Agent Reply",
        "thinking" => "Thinking",
        "tool_call" => "Tool Call",
        _ => "Message",
    };
    LocalSessionContentItem {
        id: format!("sqlite-message-{index}"),
        kind: message.kind.clone(),
        title: title.to_string(),
        char_count: text.chars().count(),
        text,
        timestamp: message.timestamp,
        evidence_refs: vec![format!("sqlite:{}:{index}", message.role)],
    }
}

fn sort_sqlite_sessions(
    sessions: &mut [SqliteSession],
    sort: LocalSessionSort,
    direction: SortDirection,
) {
    sessions.sort_by(|left, right| {
        let order = match sort {
            LocalSessionSort::ModifiedAt => left.modified_at.cmp(&right.modified_at),
            LocalSessionSort::Title => left.title.to_lowercase().cmp(&right.title.to_lowercase()),
        };
        let order = if direction == SortDirection::Desc {
            order.reverse()
        } else {
            order
        };
        order.then_with(|| left.service_id.cmp(&right.service_id))
    });
}

fn sqlite_service_id(agent: SqliteAgent, db_path: &Path, native_id: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot.sqlite-session.v1\0");
    hasher.update(agent.id().as_bytes());
    hasher.update([0]);
    hasher.update(db_path.to_string_lossy().as_bytes());
    hasher.update([0]);
    hasher.update(native_id.as_bytes());
    format!(
        "sqlite:{}:{}",
        agent.id(),
        hex_prefix(&hasher.finalize(), 40)
    )
}

fn sqlite_source_revision(sessions: &[SqliteSession], agent: SqliteAgent) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot.sqlite-session-source.v1\0");
    hasher.update(agent.id().as_bytes());
    for session in sessions {
        hasher.update([0]);
        hasher.update(trace_content_hash(&session.db_path.to_string_lossy()).as_bytes());
        hasher.update(session.service_id.as_bytes());
        hasher.update(session.modified_at.to_le_bytes());
    }
    format!("sha256:{}", hex_prefix(&hasher.finalize(), 64))
}

fn sqlite_message_revision(
    connection: &Connection,
    agent: SqliteAgent,
    session: &SqliteSession,
) -> Result<String, ServiceError> {
    Ok(sqlite_message_snapshot(connection, agent, session, None, false)?.source_revision)
}

fn sqlite_preview_query_digest(
    agent: SqliteAgent,
    params: &LocalSessionPreviewParams,
    scope: LocalSessionScope,
    include_content_items: bool,
) -> String {
    let value = format!(
        "{}|{}|{}|{}|{}|{}",
        agent.id(),
        scope.as_str(),
        params.project_root.as_deref().unwrap_or_default(),
        params.current_cwd.as_deref().unwrap_or_default(),
        params.session_id.as_deref().unwrap_or_default(),
        include_content_items
    );
    format!("sha256:{}", trace_content_hash(&value))
}

fn sqlite_message_query_digest(
    agent: SqliteAgent,
    params: &LocalSessionMessagePageParams,
) -> String {
    let value = format!(
        "{}|{}|{}|{}",
        agent.id(),
        params.session_id,
        params.project_root.as_deref().unwrap_or_default(),
        params.current_cwd.as_deref().unwrap_or_default()
    );
    format!("sha256:{}", trace_content_hash(&value))
}

fn sqlite_schema_error(_error: rusqlite::Error) -> ServiceError {
    ServiceError::InvalidRequest(
        "current local session database schema is unsupported or unreadable".to_string(),
    )
}
