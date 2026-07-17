use super::*;
use crate::service_keyset_cursor::{decode_cursor, encode_cursor, KeysetCursor};
use rusqlite::{Connection, OpenFlags};
use sha2::{Digest, Sha256};

const PREVIEW_METHOD: &str = "session.previewLocalSessions";
const MESSAGE_METHOD: &str = "session.listLocalSessionMessages";
const MAX_SQLITE_SESSIONS: usize = 10_000;
const MAX_SQLITE_PREVIEW_MESSAGES: usize = 2_000;
const MAX_SQLITE_TEXT_BYTES: usize = 32 * 1024 * 1024;
const SQLITE_MESSAGE_SCAN_ROWS: usize = 1_000;

#[derive(Clone, Copy, Eq, PartialEq)]
enum SqliteAgent {
    Opencode,
    Hermes,
}

impl SqliteAgent {
    fn from_requested(value: Option<&str>) -> Option<Self> {
        match value?.trim().to_ascii_lowercase().as_str() {
            "opencode" | "open-code" => Some(Self::Opencode),
            "hermes" | "hermes-agent" => Some(Self::Hermes),
            _ => None,
        }
    }

    fn id(self) -> &'static str {
        match self {
            Self::Opencode => "opencode",
            Self::Hermes => "hermes",
        }
    }

    fn source_kind(self) -> &'static str {
        match self {
            Self::Opencode => "opencode-sqlite",
            Self::Hermes => "hermes-sqlite",
        }
    }

    fn database_path(self, ctx: &AdapterContext) -> PathBuf {
        match self {
            Self::Opencode => opencode_data_dir(ctx).join("opencode.db"),
            Self::Hermes => hermes_home_dir(ctx).join("state.db"),
        }
    }
}

#[derive(Clone)]
struct SqliteSession {
    native_id: String,
    service_id: String,
    title: String,
    project_root: Option<String>,
    started_at: Option<i64>,
    modified_at: i64,
    ended_at: Option<i64>,
    declared_message_count: usize,
    declared_tool_count: usize,
}

#[derive(Clone)]
struct SqliteMessage {
    role: String,
    text: String,
    timestamp: Option<i64>,
    kind: String,
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
    let Some(agent) = SqliteAgent::from_requested(requested_agent) else {
        return Ok(None);
    };
    let db_path = agent.database_path(ctx);
    if !db_path.is_file() {
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

    let connection = open_read_only_database(&db_path)?;
    let mut sessions = load_sessions(&connection, agent, &db_path)?;
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

    let source_revision = sqlite_source_revision(&sessions, agent, &db_path);
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
    let redacted_db = redactor.redact(&db_path.to_string_lossy());
    let user_message_count = rows.iter().map(|row| row.user_message_count).sum();
    let total_message_count = rows.iter().map(|row| row.total_message_count).sum();
    let tool_call_count = rows.iter().map(|row| row.tool_call_count).sum();
    let count = rows.len();
    Ok(Some(LocalSessionPreviewResult {
        generated_by: "local-v3.00-sqlite",
        authorized: true,
        authorization_required: false,
        roots: vec![LocalSessionPreviewRoot {
            root: redacted_db,
            status: "auto-discovered-read-only".to_string(),
            candidate_count: count,
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

pub(super) fn list_sqlite_session_messages(
    ctx: &AdapterContext,
    params: &LocalSessionMessagePageParams,
    limit: usize,
    redaction_roots: &[(String, &'static str)],
) -> Result<Option<LocalSessionMessagePageResult>, ServiceError> {
    let Some(agent) = SqliteAgent::from_requested(params.agent.as_deref()) else {
        return Ok(None);
    };
    let db_path = agent.database_path(ctx);
    if !db_path.is_file() {
        return Ok(None);
    }
    let connection = open_read_only_database(&db_path)?;
    let sessions = load_sessions(&connection, agent, &db_path)?;
    let Some(session) = sessions
        .iter()
        .find(|session| session.service_id == params.session_id)
    else {
        return Err(ServiceError::InvalidRequest(
            "selected session was not found in the current SQLite store".to_string(),
        ));
    };
    let source_revision = sqlite_message_revision(&connection, agent, session)?;
    let query_digest = sqlite_message_query_digest(agent, params);
    let cursor = params
        .cursor
        .as_deref()
        .map(|value| decode_cursor(value, MESSAGE_METHOD, &query_digest))
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
    if cursor
        .as_ref()
        .is_some_and(|cursor| cursor.stable_id != session.service_id)
    {
        return Err(ServiceError::InvalidRequest(
            "message cursor is outside the selected SQLite session".to_string(),
        ));
    }
    let start = cursor
        .as_ref()
        .and_then(|cursor| usize::try_from(cursor.sort_value).ok())
        .unwrap_or(0);
    let scan = load_final_message_page(&connection, agent, &session.native_id, start, limit)?;
    if start > scan.total_rows {
        return Err(ServiceError::InvalidRequest(
            "message cursor is outside the selected SQLite session".to_string(),
        ));
    }
    let mut redactor = PromptRedactor::new(redaction_roots);
    let items = scan
        .messages
        .iter()
        .enumerate()
        .map(|(index, message)| sqlite_content_item(message, start + index, &mut redactor))
        .collect::<Vec<_>>();
    let has_more = scan.next_row < scan.total_rows;
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
                tie_breaker_digest: None,
                accepted_count: Some(
                    cursor
                        .as_ref()
                        .and_then(|cursor| cursor.accepted_count)
                        .unwrap_or_default()
                        .saturating_add(items.len()),
                ),
                processed_prefix_digest: None,
                resolved_start_at: None,
                resolved_end_at: Some(i64::try_from(scan.total_rows).unwrap_or(i64::MAX)),
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
        source_completeness: ListSourceCompleteness::Enumerable,
        incomplete_reason: None,
        scanned_bytes: 0,
        scanned_through_bytes: scan.next_row as u64,
        snapshot_bytes: scan.total_rows as u64,
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
) -> Result<Vec<SqliteSession>, ServiceError> {
    let sql = match agent {
        SqliteAgent::Opencode => "SELECT id, title, directory, time_created, time_updated, NULL, 0, 0 FROM session WHERE parent_id IS NULL ORDER BY time_updated DESC, id ASC LIMIT ?1",
        SqliteAgent::Hermes => "SELECT id, COALESCE(title, id), NULL, CAST(started_at * 1000 AS INTEGER), CAST(COALESCE(ended_at, started_at) * 1000 AS INTEGER), CAST(ended_at * 1000 AS INTEGER), COALESCE(message_count, 0), COALESCE(tool_call_count, 0) FROM sessions WHERE source NOT IN ('cron', 'batch', 'subagent', 'memory') ORDER BY COALESCE(ended_at, started_at) DESC, id ASC LIMIT ?1",
    };
    let mut statement = connection.prepare(sql).map_err(sqlite_schema_error)?;
    let rows = statement
        .query_map([MAX_SQLITE_SESSIONS as i64], |row| {
            let native_id: String = row.get(0)?;
            Ok(SqliteSession {
                service_id: sqlite_service_id(agent, db_path, &native_id),
                native_id,
                title: row.get(1)?,
                project_root: row.get(2)?,
                started_at: row.get(3)?,
                modified_at: row.get::<_, Option<i64>>(4)?.unwrap_or_default(),
                ended_at: row.get(5)?,
                declared_message_count: row.get::<_, i64>(6)?.max(0) as usize,
                declared_tool_count: row.get::<_, i64>(7)?.max(0) as usize,
            })
        })
        .map_err(sqlite_schema_error)?;
    rows.collect::<Result<Vec<_>, _>>()
        .map_err(sqlite_schema_error)
}

fn load_messages(
    connection: &Connection,
    agent: SqliteAgent,
    session_id: &str,
) -> Result<Vec<SqliteMessage>, ServiceError> {
    match agent {
        SqliteAgent::Opencode => load_opencode_messages(connection, session_id),
        SqliteAgent::Hermes => load_hermes_messages(connection, session_id),
    }
}

struct SqliteMessagePageScan {
    messages: Vec<SqliteMessage>,
    next_row: usize,
    total_rows: usize,
}

fn load_final_message_page(
    connection: &Connection,
    agent: SqliteAgent,
    session_id: &str,
    start_row: usize,
    limit: usize,
) -> Result<SqliteMessagePageScan, ServiceError> {
    let total_rows = sqlite_message_row_count(connection, agent, session_id)?;
    let mut next_row = start_row.min(total_rows);
    let mut messages = Vec::with_capacity(limit);
    while next_row < total_rows && messages.len() < limit {
        let remaining = total_rows.saturating_sub(next_row);
        let raw_limit = SQLITE_MESSAGE_SCAN_ROWS.min(remaining);
        let rows = load_message_rows(connection, agent, session_id, next_row, raw_limit)?;
        if rows.is_empty() {
            next_row = total_rows;
            break;
        }
        for message in rows {
            next_row = next_row.saturating_add(1);
            if matches!(message.kind.as_str(), "user_message" | "agent_reply") {
                messages.push(message);
                if messages.len() == limit {
                    break;
                }
            }
        }
        if messages.len() < limit {
            break;
        }
    }
    Ok(SqliteMessagePageScan {
        messages,
        next_row,
        total_rows,
    })
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
    };
    let count = connection
        .query_row(sql, [session_id], |row| row.get::<_, i64>(0))
        .map_err(sqlite_schema_error)?;
    Ok(count.max(0) as usize)
}

fn load_message_rows(
    connection: &Connection,
    agent: SqliteAgent,
    session_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<SqliteMessage>, ServiceError> {
    match agent {
        SqliteAgent::Opencode => load_opencode_message_rows(connection, session_id, offset, limit),
        SqliteAgent::Hermes => load_hermes_message_rows(connection, session_id, offset, limit),
    }
}

fn load_opencode_message_rows(
    connection: &Connection,
    session_id: &str,
    offset: usize,
    limit: usize,
) -> Result<Vec<SqliteMessage>, ServiceError> {
    let mut statement = connection
        .prepare("SELECT m.data, p.data, p.time_created FROM message m JOIN part p ON p.message_id = m.id WHERE m.session_id = ?1 ORDER BY m.time_created ASC, p.time_created ASC, p.id ASC LIMIT ?2 OFFSET ?3")
        .map_err(sqlite_schema_error)?;
    let rows = statement
        .query_map((session_id, limit as i64, offset as i64), |row| {
            Ok((
                row.get::<_, String>(0)?,
                row.get::<_, String>(1)?,
                row.get::<_, i64>(2)?,
            ))
        })
        .map_err(sqlite_schema_error)?;
    rows.map(|row| {
        let (message_json, part_json, timestamp) = row.map_err(sqlite_schema_error)?;
        Ok(opencode_message_from_json(
            &message_json,
            &part_json,
            timestamp,
        ))
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
) -> Result<Vec<SqliteMessage>, ServiceError> {
    let mut statement = connection
        .prepare("SELECT role, COALESCE(content, ''), CAST(timestamp * 1000 AS INTEGER), COALESCE(tool_name, ''), COALESCE(tool_calls, ''), COALESCE(reasoning, '') FROM messages WHERE session_id = ?1 ORDER BY timestamp ASC, id ASC LIMIT ?2 OFFSET ?3")
        .map_err(sqlite_schema_error)?;
    let rows = statement
        .query_map((session_id, limit as i64, offset as i64), |row| {
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
    rows.map(|row| {
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
            ("ignored", String::new())
        };
        Ok(SqliteMessage {
            role,
            text,
            timestamp: Some(timestamp),
            kind: kind.to_string(),
        })
    })
    .collect()
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

fn sqlite_session_row(
    connection: &Connection,
    agent: SqliteAgent,
    session: &SqliteSession,
    scope: LocalSessionScope,
    include_content_items: bool,
    max_excerpt_chars: usize,
    redactor: &mut PromptRedactor<'_>,
) -> Result<LocalSessionPreviewRow, ServiceError> {
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

fn sqlite_source_revision(
    sessions: &[SqliteSession],
    agent: SqliteAgent,
    db_path: &Path,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot.sqlite-session-source.v1\0");
    hasher.update(agent.id().as_bytes());
    hasher.update(trace_content_hash(&db_path.to_string_lossy()).as_bytes());
    for session in sessions {
        hasher.update([0]);
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
    let (count, latest): (i64, i64) = match agent {
        SqliteAgent::Opencode => connection.query_row(
            "SELECT COUNT(*), COALESCE(MAX(time_updated), 0) FROM part WHERE session_id = ?1",
            [&session.native_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ),
        SqliteAgent::Hermes => connection.query_row(
            "SELECT COUNT(*), COALESCE(CAST(MAX(timestamp) * 1000 AS INTEGER), 0) FROM messages WHERE session_id = ?1",
            [&session.native_id],
            |row| Ok((row.get(0)?, row.get(1)?)),
        ),
    }
    .map_err(sqlite_schema_error)?;
    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot.sqlite-session-messages.v1\0");
    hasher.update(agent.id().as_bytes());
    hasher.update(session.service_id.as_bytes());
    hasher.update(count.to_le_bytes());
    hasher.update(latest.to_le_bytes());
    Ok(format!("sha256:{}", hex_prefix(&hasher.finalize(), 64)))
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
