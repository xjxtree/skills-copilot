use super::*;
use crate::service_keyset_cursor::{decode_cursor, encode_cursor, KeysetCursor};
use std::io::{Read, Seek, SeekFrom};

const METHOD: &str = "session.listLocalSessionMessages";
const DEFAULT_PAGE_ITEMS: usize = 40;
const MAX_PAGE_ITEMS: usize = 100;
const MAX_PAGE_SCAN_BYTES: u64 = 32 * 1024 * 1024;
const MAX_PAGE_TEXT_BYTES: usize = 2 * 1024 * 1024;
const MESSAGE_SCHEMA_PROBE_BYTES: usize = 512 * 1024;
const MAX_RECORD_PROBE_BYTES: usize = 128 * 1024;
const READ_CHUNK_BYTES: usize = 64 * 1024;
const SKIP_LINE_CURSOR_MARKER: &str = "skip-line";

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum MessageRecordProbe {
    PagedMessage,
    NonFinal,
    Unresolved,
}

#[derive(Debug)]
enum MessageRecordState {
    Probing(Vec<u8>),
    Retaining(Vec<u8>),
    Discarding,
}

struct ResolvedMessageSource {
    path: PathBuf,
    guarded_root: GuardedLocalSessionRoot,
}

struct MessagePageScan {
    items: Vec<LocalSessionContentItem>,
    next_offset: u64,
    next_draft_index: usize,
    skip_line: bool,
    last_goal_digest: Option<String>,
    scanned_bytes: u64,
}

struct PagedMessageDraft {
    content: LocalSessionContentDraft,
    goal_digest: Option<String>,
}

impl ServiceHost {
    pub fn list_local_session_messages(
        &self,
        params: LocalSessionMessagePageParams,
    ) -> Result<LocalSessionMessagePageResult, ServiceError> {
        let session_id = params.session_id.trim();
        if session_id.is_empty() {
            return Err(ServiceError::InvalidRequest(
                "session_id is required for local session message paging".to_string(),
            ));
        }
        if params.cursor.is_some() != params.source_revision.is_some() {
            return Err(ServiceError::InvalidRequest(
                "message continuation requires both cursor and source_revision".to_string(),
            ));
        }

        let limit = params
            .limit
            .unwrap_or(DEFAULT_PAGE_ITEMS)
            .clamp(1, MAX_PAGE_ITEMS);
        let requested_roots = normalize_string_list(params.authorized_roots.clone());
        let auto_discover = params.auto_discover.unwrap_or(requested_roots.is_empty());
        let adapter_ctx = self.effective_adapter_ctx()?;
        let project_filter_roots = local_session_project_filter_roots(
            &adapter_ctx,
            params.project_root.as_deref(),
            params.current_cwd.as_deref(),
        );
        let redaction_roots = self.trace_redaction_roots(&adapter_ctx);
        if requested_roots.is_empty() && auto_discover {
            if let Some(result) = sqlite_sessions::list_sqlite_session_messages(
                &adapter_ctx,
                &params,
                limit,
                &redaction_roots,
            )? {
                return Ok(result);
            }
        }
        let source = resolve_message_source(
            &adapter_ctx,
            params.agent.as_deref(),
            session_id,
            requested_roots,
            auto_discover,
            &project_filter_roots,
            &redaction_roots,
        )?;
        let query_digest = message_query_digest(
            session_id,
            params.agent.as_deref(),
            &params.authorized_roots,
            params.project_root.as_deref(),
            params.current_cwd.as_deref(),
            auto_discover,
        );
        let cursor = params
            .cursor
            .as_deref()
            .map(|value| decode_cursor(value, METHOD, &query_digest))
            .transpose()?;

        let (mut file, metadata) = source.guarded_root.open_regular_file(&source.path)?;
        let snapshot_bytes = cursor
            .as_ref()
            .and_then(|cursor| cursor.resolved_end_at)
            .and_then(|value| u64::try_from(value).ok())
            .unwrap_or(metadata.len());
        if metadata.len() < snapshot_bytes {
            return Err(ServiceError::SourceChanged);
        }
        let source_revision = message_source_revision(
            &mut file,
            &source.path,
            &metadata,
            snapshot_bytes,
            session_id,
        )?;
        if params
            .source_revision
            .as_deref()
            .is_some_and(|revision| revision != source_revision)
            || cursor
                .as_ref()
                .is_some_and(|cursor| cursor.source_revision != source_revision)
        {
            return Err(ServiceError::SourceChanged);
        }

        let start_offset = cursor.as_ref().map_or(0, |cursor| {
            u64::try_from(cursor.sort_value).unwrap_or(u64::MAX)
        });
        if start_offset > snapshot_bytes
            || cursor
                .as_ref()
                .is_some_and(|cursor| cursor.stable_id != session_id)
        {
            return Err(ServiceError::InvalidRequest(
                "message cursor is outside the selected session snapshot".to_string(),
            ));
        }
        let initial_draft_index = cursor
            .as_ref()
            .and_then(|cursor| cursor.accepted_count)
            .unwrap_or(0);
        let initial_goal_digest = cursor
            .as_ref()
            .and_then(|cursor| cursor.processed_prefix_digest.clone());
        let accepted_before = cursor
            .as_ref()
            .and_then(|cursor| cursor.resolved_start_at)
            .and_then(|value| usize::try_from(value).ok())
            .unwrap_or(0);
        let skip_line = cursor
            .as_ref()
            .and_then(|cursor| cursor.tie_breaker_digest.as_deref())
            == Some(SKIP_LINE_CURSOR_MARKER);
        if skip_line && initial_draft_index != 0 {
            return Err(ServiceError::InvalidRequest(
                "message cursor line state is invalid".to_string(),
            ));
        }

        let prefer_response_items = source_prefers_response_items(&mut file, snapshot_bytes)?;
        file.seek(SeekFrom::Start(start_offset))?;
        let mut redactor = PromptRedactor::new(&redaction_roots);
        let scan = scan_message_page(
            &mut file,
            session_id,
            start_offset,
            snapshot_bytes,
            initial_draft_index,
            initial_goal_digest,
            skip_line,
            prefer_response_items,
            limit,
            &mut redactor,
        )?;
        let returned_count = scan.items.len();
        let accepted_through = accepted_before.saturating_add(returned_count);
        let has_more = scan.next_offset < snapshot_bytes || scan.next_draft_index > 0;
        let next_cursor = has_more
            .then(|| {
                encode_cursor(&KeysetCursor {
                    version: 1,
                    method: METHOD.to_string(),
                    query_digest: query_digest.clone(),
                    source_revision: source_revision.clone(),
                    sort_value: i64::try_from(scan.next_offset).map_err(|_| {
                        ServiceError::InvalidRequest(
                            "session message offset exceeds supported cursor range".to_string(),
                        )
                    })?,
                    stable_id: session_id.to_string(),
                    tie_breaker_digest: scan.skip_line.then(|| SKIP_LINE_CURSOR_MARKER.to_string()),
                    accepted_count: Some(scan.next_draft_index),
                    processed_prefix_digest: scan.last_goal_digest.clone(),
                    resolved_start_at: Some(i64::try_from(accepted_through).map_err(|_| {
                        ServiceError::InvalidRequest(
                            "session message count exceeds supported cursor range".to_string(),
                        )
                    })?),
                    resolved_end_at: Some(i64::try_from(snapshot_bytes).map_err(|_| {
                        ServiceError::InvalidRequest(
                            "session snapshot exceeds supported cursor range".to_string(),
                        )
                    })?),
                })
            })
            .transpose()?;

        Ok(LocalSessionMessagePageResult {
            generated_by: "local-v2.99",
            session_id: session_id.to_string(),
            content_items: scan.items,
            returned_count,
            total_count: (!has_more).then_some(accepted_through),
            has_more,
            next_cursor,
            source_revision,
            source_completeness: ListSourceCompleteness::Enumerable,
            incomplete_reason: None,
            scanned_bytes: scan.scanned_bytes,
            scanned_through_bytes: scan.next_offset,
            snapshot_bytes,
            redaction_summary: local_preview_redaction_summary_from(redactor.summary()),
            safety_flags: local_preview_safety_flags(),
            read_only: true,
            provider_request_sent: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_trace_persisted: false,
        })
    }
}

fn resolve_message_source(
    adapter_ctx: &AdapterContext,
    requested_agent: Option<&str>,
    session_id: &str,
    requested_roots: Vec<String>,
    auto_discover: bool,
    project_filter_roots: &[PathBuf],
    redaction_roots: &[(String, &'static str)],
) -> Result<ResolvedMessageSource, ServiceError> {
    let mut roots = requested_roots
        .into_iter()
        .map(|root| {
            let requested_path = PathBuf::from(root);
            let guarded_root = GuardedLocalSessionRoot::open(&requested_path);
            let path = guarded_root
                .as_ref()
                .map_or(requested_path, |root| root.path().to_path_buf());
            LocalSessionRootRequest {
                path,
                guarded_root,
                status: "authorized-read-only",
                source_kind: "authorized-local-session",
            }
        })
        .collect::<Vec<_>>();
    if auto_discover {
        let (mut discovered, _) = auto_local_session_roots(
            adapter_ctx,
            requested_agent,
            LocalSessionScope::All,
            project_filter_roots,
        );
        roots.append(&mut discovered);
    }
    dedupe_local_session_root_requests(&mut roots);
    run_scheduled_local_session_root_swap_test_hook(&adapter_ctx.user_home);

    let mut io = LocalSessionIoContext::new(LocalSessionReadLimits::default());
    let mut notes = Vec::new();
    let mut redactor = PromptRedactor::new(redaction_roots);
    let mut inventory_was_truncated = false;
    for root in roots {
        let guarded_root = match root.guarded_root {
            Ok(root) => root,
            Err(_) => continue,
        };
        let inventory = collect_local_session_inventory(
            &guarded_root,
            requested_agent,
            &mut io.inventory_budget,
            &mut notes,
            &mut redactor,
        );
        inventory_was_truncated |= inventory.truncated;
        if let Some(candidate) = inventory
            .candidates
            .into_iter()
            .find(|candidate| local_session_row_id(&candidate.path) == session_id)
        {
            return Ok(ResolvedMessageSource {
                path: candidate.path,
                guarded_root,
            });
        }
    }

    let reason = if inventory_was_truncated {
        "selected session was not found within the bounded authorized inventory"
    } else {
        "selected session was not found in the authorized local stores"
    };
    Err(ServiceError::InvalidRequest(reason.to_string()))
}

fn message_query_digest(
    session_id: &str,
    requested_agent: Option<&str>,
    roots: &[String],
    project_root: Option<&str>,
    current_cwd: Option<&str>,
    auto_discover: bool,
) -> String {
    let mut root_digests = roots
        .iter()
        .map(|root| trace_content_hash(root.trim()))
        .collect::<Vec<_>>();
    root_digests.sort();
    let mut hasher = Sha256::new();
    hasher.update(b"session.listLocalSessionMessages.query.v1\0");
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(requested_agent.unwrap_or_default().trim().as_bytes());
    hasher.update([u8::from(auto_discover)]);
    hasher.update(trace_content_hash(project_root.unwrap_or_default()).as_bytes());
    hasher.update(trace_content_hash(current_cwd.unwrap_or_default()).as_bytes());
    for digest in root_digests {
        hasher.update([0]);
        hasher.update(digest.as_bytes());
    }
    format!("sha256:{}", hex_prefix(&hasher.finalize(), 64))
}

fn message_source_revision(
    file: &mut fs::File,
    path: &Path,
    _metadata: &fs::Metadata,
    snapshot_bytes: u64,
    session_id: &str,
) -> Result<String, ServiceError> {
    const FINGERPRINT_WINDOW_BYTES: usize = 4 * 1024;
    let head_len = usize::try_from(snapshot_bytes)
        .unwrap_or(usize::MAX)
        .min(FINGERPRINT_WINDOW_BYTES);
    let tail_len = usize::try_from(snapshot_bytes.saturating_sub(head_len as u64))
        .unwrap_or(usize::MAX)
        .min(FINGERPRINT_WINDOW_BYTES);
    let mut head = vec![0_u8; head_len];
    file.seek(SeekFrom::Start(0))?;
    file.read_exact(&mut head)?;
    let mut tail = vec![0_u8; tail_len];
    if tail_len > 0 {
        file.seek(SeekFrom::Start(
            snapshot_bytes.saturating_sub(tail_len as u64),
        ))?;
        file.read_exact(&mut tail)?;
    }
    let mut hasher = Sha256::new();
    hasher.update(b"session.listLocalSessionMessages.source.v1\0");
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(trace_content_hash(&local_session_normalized_path(path)).as_bytes());
    hasher.update(snapshot_bytes.to_le_bytes());
    hasher.update(head);
    hasher.update(tail);
    #[cfg(unix)]
    {
        use std::os::unix::fs::MetadataExt;
        hasher.update(_metadata.dev().to_le_bytes());
        hasher.update(_metadata.ino().to_le_bytes());
    }
    Ok(format!("sha256:{}", hex_prefix(&hasher.finalize(), 64)))
}

fn source_prefers_response_items(
    file: &mut fs::File,
    snapshot_bytes: u64,
) -> Result<bool, ServiceError> {
    file.seek(SeekFrom::Start(0))?;
    let probe_bytes = usize::try_from(snapshot_bytes)
        .unwrap_or(usize::MAX)
        .min(MESSAGE_SCHEMA_PROBE_BYTES);
    let mut bytes = vec![0_u8; probe_bytes];
    let mut read = 0usize;
    while read < bytes.len() {
        let count = file.read(&mut bytes[read..])?;
        if count == 0 {
            break;
        }
        read += count;
    }
    bytes.truncate(read);
    Ok(bytes.split(|byte| *byte == b'\n').any(|record| {
        let text = String::from_utf8_lossy(record);
        top_level_scalar_fields_from_prefix(&text)
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(|kind| kind.eq_ignore_ascii_case("response_item"))
    }))
}

#[allow(clippy::too_many_arguments)]
fn scan_message_page(
    file: &mut fs::File,
    session_id: &str,
    start_offset: u64,
    snapshot_bytes: u64,
    initial_draft_index: usize,
    initial_goal_digest: Option<String>,
    skip_line: bool,
    prefer_response_items: bool,
    limit: usize,
    redactor: &mut PromptRedactor<'_>,
) -> Result<MessagePageScan, ServiceError> {
    let mut state = if skip_line {
        MessageRecordState::Discarding
    } else {
        MessageRecordState::Probing(Vec::new())
    };
    let mut record_start = start_offset;
    let mut offset = start_offset;
    let mut first_record = true;
    let mut items = Vec::new();
    let mut text_bytes = 0usize;
    let mut last_goal_digest = initial_goal_digest;
    let mut buffer = [0_u8; READ_CHUNK_BYTES];

    while offset < snapshot_bytes {
        let scanned = offset.saturating_sub(start_offset);
        let at_budget = scanned >= MAX_PAGE_SCAN_BYTES;
        if at_budget {
            if matches!(state, MessageRecordState::Discarding) {
                return Ok(MessagePageScan {
                    items,
                    next_offset: offset,
                    next_draft_index: 0,
                    skip_line: true,
                    last_goal_digest,
                    scanned_bytes: scanned,
                });
            }
            if matches!(state, MessageRecordState::Probing(ref bytes) if bytes.is_empty()) {
                break;
            }
        }

        let remaining_file = snapshot_bytes.saturating_sub(offset);
        let remaining_budget = MAX_PAGE_SCAN_BYTES.saturating_sub(scanned);
        let read_limit = if remaining_budget == 0 {
            READ_CHUNK_BYTES as u64
        } else {
            remaining_budget.min(READ_CHUNK_BYTES as u64)
        }
        .min(remaining_file) as usize;
        let count = file.read(&mut buffer[..read_limit])?;
        if count == 0 {
            return Err(ServiceError::SourceChanged);
        }

        let mut cursor = 0usize;
        while cursor < count {
            let newline = buffer[cursor..count]
                .iter()
                .position(|byte| *byte == b'\n')
                .map(|relative| cursor + relative);
            let end = newline.unwrap_or(count);
            let segment = &buffer[cursor..end];
            append_message_record_segment(&mut state, segment, prefer_response_items);
            offset = offset.saturating_add((end - cursor) as u64);

            if newline.is_none() {
                cursor = count;
                continue;
            }

            offset = offset.saturating_add(1);
            let draft_index = if first_record { initial_draft_index } else { 0 };
            if let Some(record) = retained_message_record(&mut state) {
                let drafts = paged_message_drafts(record, prefer_response_items)?;
                for (index, draft) in drafts.into_iter().enumerate().skip(draft_index) {
                    if draft.goal_digest.as_ref() == last_goal_digest.as_ref()
                        && draft.goal_digest.is_some()
                    {
                        continue;
                    }
                    let goal_digest = draft.goal_digest.clone();
                    let item = paged_message_item(
                        session_id,
                        record_start,
                        index,
                        draft.content,
                        redactor,
                    );
                    let exceeds_count = items.len() >= limit;
                    let exceeds_text = !items.is_empty()
                        && text_bytes.saturating_add(item.text.len()) > MAX_PAGE_TEXT_BYTES;
                    if exceeds_count || exceeds_text {
                        return Ok(MessagePageScan {
                            items,
                            next_offset: record_start,
                            next_draft_index: index,
                            skip_line: false,
                            last_goal_digest,
                            scanned_bytes: offset.saturating_sub(start_offset),
                        });
                    }
                    text_bytes = text_bytes.saturating_add(item.text.len());
                    items.push(item);
                    if goal_digest.is_some() {
                        last_goal_digest = goal_digest;
                    }
                }
            }
            first_record = false;
            state = MessageRecordState::Probing(Vec::new());
            record_start = offset;
            cursor = end + 1;
            if items.len() >= limit || text_bytes >= MAX_PAGE_TEXT_BYTES {
                return Ok(MessagePageScan {
                    items,
                    next_offset: offset,
                    next_draft_index: 0,
                    skip_line: false,
                    last_goal_digest,
                    scanned_bytes: offset.saturating_sub(start_offset),
                });
            }
        }
    }

    if offset == snapshot_bytes {
        let draft_index = if first_record { initial_draft_index } else { 0 };
        if let Some(record) = retained_message_record(&mut state) {
            let drafts = paged_message_drafts(record, prefer_response_items)?;
            for (index, draft) in drafts.into_iter().enumerate().skip(draft_index) {
                if draft.goal_digest.as_ref() == last_goal_digest.as_ref()
                    && draft.goal_digest.is_some()
                {
                    continue;
                }
                let goal_digest = draft.goal_digest.clone();
                let item =
                    paged_message_item(session_id, record_start, index, draft.content, redactor);
                let exceeds_count = items.len() >= limit;
                let exceeds_text = !items.is_empty()
                    && text_bytes.saturating_add(item.text.len()) > MAX_PAGE_TEXT_BYTES;
                if exceeds_count || exceeds_text {
                    return Ok(MessagePageScan {
                        items,
                        next_offset: record_start,
                        next_draft_index: index,
                        skip_line: false,
                        last_goal_digest,
                        scanned_bytes: offset.saturating_sub(start_offset),
                    });
                }
                text_bytes = text_bytes.saturating_add(item.text.len());
                items.push(item);
                if goal_digest.is_some() {
                    last_goal_digest = goal_digest;
                }
            }
        }
    }

    Ok(MessagePageScan {
        items,
        next_offset: offset,
        next_draft_index: 0,
        skip_line: false,
        last_goal_digest,
        scanned_bytes: offset.saturating_sub(start_offset),
    })
}

fn append_message_record_segment(
    state: &mut MessageRecordState,
    segment: &[u8],
    prefer_response_items: bool,
) {
    let next = match state {
        MessageRecordState::Probing(bytes) => {
            bytes.extend_from_slice(segment);
            let probe = if bytes.len() <= MAX_RECORD_PROBE_BYTES {
                probe_message_record(bytes, prefer_response_items)
            } else {
                MessageRecordProbe::Unresolved
            };
            match probe {
                MessageRecordProbe::PagedMessage => {
                    Some(MessageRecordState::Retaining(std::mem::take(bytes)))
                }
                MessageRecordProbe::NonFinal => Some(MessageRecordState::Discarding),
                MessageRecordProbe::Unresolved => None,
            }
        }
        MessageRecordState::Retaining(bytes) => {
            bytes.extend_from_slice(segment);
            None
        }
        MessageRecordState::Discarding => None,
    };
    if let Some(next) = next {
        *state = next;
    }
}

fn retained_message_record(state: &mut MessageRecordState) -> Option<&[u8]> {
    match state {
        MessageRecordState::Probing(bytes) | MessageRecordState::Retaining(bytes)
            if !bytes.is_empty() =>
        {
            Some(bytes)
        }
        MessageRecordState::Probing(_)
        | MessageRecordState::Retaining(_)
        | MessageRecordState::Discarding => None,
    }
}

fn probe_message_record(bytes: &[u8], prefer_response_items: bool) -> MessageRecordProbe {
    let text = String::from_utf8_lossy(bytes);
    let root_fields = top_level_scalar_fields_from_prefix(&text);
    let root_type = root_fields.get("type").and_then(Value::as_str);
    if prefer_response_items && root_type == Some("event_msg") {
        return MessageRecordProbe::NonFinal;
    }
    if root_type.is_some_and(is_tool_result_record_type) {
        return MessageRecordProbe::NonFinal;
    }
    let root_classification = local_session_record_classification(&root_fields, None);
    if let Some(probe) = conclusive_message_probe(root_classification) {
        return probe;
    }
    if let Some(payload) = top_level_object_value_fragment(&text, "payload") {
        let payload_fields = top_level_scalar_fields_from_prefix(payload);
        if payload_fields
            .get("type")
            .and_then(Value::as_str)
            .is_some_and(is_tool_result_record_type)
        {
            return MessageRecordProbe::NonFinal;
        }
        let payload_classification = local_session_record_classification(&payload_fields, None);
        if let Some(probe) = conclusive_message_probe(payload_classification) {
            return probe;
        }
        if let Some(payload_type) = payload_fields.get("type").and_then(Value::as_str) {
            if root_type == Some("response_item") && !payload_type.eq_ignore_ascii_case("message") {
                return MessageRecordProbe::NonFinal;
            }
            if root_type == Some("event_msg") {
                return MessageRecordProbe::NonFinal;
            }
        }
    }
    MessageRecordProbe::Unresolved
}

fn is_tool_result_record_type(value: &str) -> bool {
    let normalized = value.to_ascii_lowercase().replace(['_', '-'], "");
    is_json_tool_result_type(&normalized)
}

fn conclusive_message_probe(
    classification: LocalSessionRecordClassification,
) -> Option<MessageRecordProbe> {
    match classification {
        LocalSessionRecordClassification::User
        | LocalSessionRecordClassification::Assistant
        | LocalSessionRecordClassification::Thinking
        | LocalSessionRecordClassification::Tool => Some(MessageRecordProbe::PagedMessage),
        LocalSessionRecordClassification::Deny => Some(MessageRecordProbe::NonFinal),
        LocalSessionRecordClassification::KnownStructure
        | LocalSessionRecordClassification::Missing
        | LocalSessionRecordClassification::Unproven => None,
    }
}

fn top_level_object_value_fragment<'a>(fragment: &'a str, requested_key: &str) -> Option<&'a str> {
    let bytes = fragment.as_bytes();
    let mut object_depth = 0_i32;
    let mut array_depth = 0_i32;
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        match bytes[cursor] {
            b'"' => {
                let end = json_string_end(bytes, cursor)?;
                if object_depth == 1 && array_depth == 0 {
                    let key = serde_json::from_slice::<String>(&bytes[cursor..=end]).ok()?;
                    let mut value_start = end + 1;
                    while bytes.get(value_start).is_some_and(u8::is_ascii_whitespace) {
                        value_start += 1;
                    }
                    if bytes.get(value_start) == Some(&b':') {
                        value_start += 1;
                        while bytes.get(value_start).is_some_and(u8::is_ascii_whitespace) {
                            value_start += 1;
                        }
                        if key == requested_key && bytes.get(value_start) == Some(&b'{') {
                            return Some(&fragment[value_start..]);
                        }
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
    None
}

fn paged_message_drafts(
    record: &[u8],
    prefer_response_items: bool,
) -> Result<Vec<PagedMessageDraft>, ServiceError> {
    let text = std::str::from_utf8(record).map_err(|_| {
        ServiceError::InvalidRequest(
            "selected session contains a non-UTF-8 message record".to_string(),
        )
    })?;
    let mut drafts = Vec::new();
    match parse_local_session_json_with_raw_classification_bounds(text.trim()) {
        Ok((value, _)) => {
            if prefer_response_items
                && local_session_top_level_record_type(&value) == Some("event_msg")
            {
                return Ok(Vec::new());
            }
            let timestamp = json_session_timestamp_millis(&value);
            collect_json_session_content_drafts(&value, timestamp, &mut drafts);
        }
        Err(LocalSessionJsonParseError::Invalid) => {
            if probe_message_record(record, prefer_response_items)
                == MessageRecordProbe::PagedMessage
            {
                return Err(ServiceError::InvalidRequest(
                    "selected session contains an invalid message record".to_string(),
                ));
            }
            collect_text_session_content_drafts(text.trim(), None, &mut drafts);
        }
        Err(LocalSessionJsonParseError::UnsafeClassification) => {
            return Err(ServiceError::InvalidRequest(
                "selected session contains an unsafe message classification".to_string(),
            ));
        }
    }

    let mut complete_drafts = Vec::new();
    for draft in drafts {
        let skill_invocations = if draft.kind == "skill_call" {
            Vec::new()
        } else {
            extract_skill_invocation_names(&draft.text)
        };
        let timestamp = draft.timestamp;
        complete_drafts.push(draft);
        complete_drafts.extend(skill_invocations.into_iter().map(|name| {
            LocalSessionContentDraft {
                kind: "skill_call".to_string(),
                title: format!("Skill: {name}"),
                text: name,
                timestamp,
                evidence_refs: Vec::new(),
            }
        }));
    }

    Ok(complete_drafts
        .into_iter()
        .filter_map(|mut draft| {
            matches!(
                draft.kind.as_str(),
                "user_message" | "agent_reply" | "thinking" | "tool_call" | "skill_call"
            )
            .then(|| {
                let goal_digest = if draft.kind == "user_message" {
                    if let Some(objective) = displayable_goal_message(&draft.text) {
                        draft.text = objective;
                        Some(trace_content_hash(&format!("session-goal\0{}", draft.text)))
                    } else {
                        draft.text = draft.text.trim().to_string();
                        None
                    }
                } else {
                    None
                };
                PagedMessageDraft {
                    content: draft,
                    goal_digest,
                }
            })
        })
        .filter(|draft| !draft.content.text.trim().is_empty())
        .collect())
}

fn displayable_goal_message(text: &str) -> Option<String> {
    let lower = text.to_ascii_lowercase();
    if !lower.contains("<codex_internal_context") || !lower.contains("source=\"goal\"") {
        return None;
    }
    let start_tag = "<objective>";
    let end_tag = "</objective>";
    let start = lower.find(start_tag)? + start_tag.len();
    let end = lower[start..].find(end_tag)? + start;
    let objective = text.get(start..end)?.trim();
    (!objective.is_empty()).then(|| objective.to_string())
}

fn paged_message_item(
    session_id: &str,
    record_offset: u64,
    draft_index: usize,
    draft: LocalSessionContentDraft,
    redactor: &mut PromptRedactor<'_>,
) -> LocalSessionContentItem {
    let text = redact_local_session_content(redactor, &draft.text);
    let identity = trace_content_hash(&format!(
        "{session_id}\0{record_offset}\0{draft_index}\0{}",
        draft.kind
    ));
    LocalSessionContentItem {
        id: format!("session-message-{}", &identity[..16]),
        kind: draft.kind,
        title: truncate_chars(&redactor.redact(&draft.title), 120),
        char_count: text.chars().count(),
        text,
        timestamp: draft.timestamp,
        evidence_refs: vec![format!("session.message:{}", &identity[..16])],
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn probe_rejects_large_codex_tool_output_from_its_prefix() {
        let prefix = br#"{"timestamp":"2026-07-17T01:00:02Z","type":"response_item","payload":{"type":"custom_tool_call_output","output":"xxxxxxxx"#;
        assert_eq!(
            probe_message_record(prefix, true),
            MessageRecordProbe::NonFinal
        );
    }

    #[test]
    fn probe_retains_tool_calls_but_not_tool_results() {
        let call = br#"{"type":"response_item","payload":{"type":"custom_tool_call","name":"shell","input":"pwd"}}"#;
        let result = br#"{"type":"response_item","payload":{"type":"custom_tool_call_output","output":"/tmp"}}"#;
        assert_eq!(
            probe_message_record(call, true),
            MessageRecordProbe::PagedMessage
        );
        assert_eq!(
            probe_message_record(result, true),
            MessageRecordProbe::NonFinal
        );
    }
}
