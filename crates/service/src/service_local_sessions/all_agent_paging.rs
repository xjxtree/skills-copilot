use super::*;
use crate::service_keyset_cursor::{decode_cursor, encode_cursor, KeysetCursor};
use sha2::{Digest, Sha256};

const METHOD: &str = "session.previewLocalSessions";
const MAX_AGGREGATE_ACCEPTED_SESSIONS: usize = 10_000;
const AGENTS: [AgentId; 6] = [
    AgentId::ClaudeCode,
    AgentId::Codex,
    AgentId::Opencode,
    AgentId::Pi,
    AgentId::Hermes,
    AgentId::Openclaw,
];

struct AgentSessionStream {
    agent: AgentId,
    rows: Vec<LocalSessionPreviewRow>,
    roots: Vec<LocalSessionPreviewRoot>,
    total_candidate_count: usize,
    total_matched_count: usize,
    has_more: bool,
    source_revision: String,
    limited: bool,
    authorized: bool,
    gap_notes: Vec<String>,
    blocker_notes: Vec<String>,
    redaction_summary: LocalPreviewRedactionSummary,
}

impl ServiceHost {
    /// Merge the six canonical adapter inventories into one stateless keyset.
    ///
    /// Each adapter is read through its normal bounded implementation. To
    /// reconstruct a later global page without persisting session rows, the
    /// service replays only the accepted prefix needed for that page, then
    /// validates the aggregate source revision and processed-prefix identity
    /// before returning new rows.
    pub(super) fn preview_all_agent_sessions_keyset(
        &self,
        params: &LocalSessionPreviewParams,
    ) -> Result<LocalSessionPreviewResult, ServiceError> {
        let query_digest = aggregate_query_digest(params);
        let cursor = params
            .cursor
            .as_deref()
            .map(|value| decode_cursor(value, METHOD, &query_digest))
            .transpose()?;
        let start = cursor
            .as_ref()
            .and_then(|cursor| cursor.accepted_count)
            .unwrap_or_default();
        if start > MAX_AGGREGATE_ACCEPTED_SESSIONS {
            return Err(ServiceError::InvalidRequest(
                "aggregate session cursor exceeds the bounded accepted-row limit".to_string(),
            ));
        }
        let limit = params.limit.unwrap_or(20).clamp(1, 100);
        let requested_end = start.saturating_add(limit);
        let target = requested_end.min(MAX_AGGREGATE_ACCEPTED_SESSIONS);

        let mut streams = Vec::with_capacity(AGENTS.len());
        for agent in AGENTS {
            streams.push(self.collect_agent_session_prefix(params, agent, target)?);
        }

        let source_revision = aggregate_source_revision(&streams);
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

        let mut rows = streams
            .iter_mut()
            .flat_map(|stream| std::mem::take(&mut stream.rows))
            .collect::<Vec<_>>();
        rows.sort_by(|left, right| {
            right
                .modified_at
                .unwrap_or_default()
                .cmp(&left.modified_at.unwrap_or_default())
                .then_with(|| left.id.cmp(&right.id))
                .then_with(|| left.agent.cmp(&right.agent))
        });
        let mut seen = BTreeSet::new();
        rows.retain(|row| seen.insert(row.id.clone()));

        if let Some(cursor) = cursor.as_ref() {
            let prefix_end = start.min(rows.len());
            let prefix = aggregate_prefix_digest(&rows[..prefix_end]);
            if cursor.processed_prefix_digest.as_deref() != Some(prefix.as_str()) {
                return Err(ServiceError::SourceChanged);
            }
        }

        let end = requested_end
            .min(rows.len())
            .min(MAX_AGGREGATE_ACCEPTED_SESSIONS);
        let next_prefix_digest = aggregate_prefix_digest(&rows[..end]);
        let source_has_more = streams.iter().any(|stream| stream.has_more) || rows.len() > end;
        let source_was_limited = streams.iter().any(|stream| stream.limited);
        let aggregate_limit_reached = requested_end > MAX_AGGREGATE_ACCEPTED_SESSIONS
            || (end == MAX_AGGREGATE_ACCEPTED_SESSIONS && source_has_more);
        let limited = source_was_limited || aggregate_limit_reached;
        let page_rows = if start < end {
            rows.drain(start..end).collect::<Vec<_>>()
        } else {
            Vec::new()
        };
        let has_more = !limited && source_has_more;
        let next_cursor = if has_more {
            page_rows
                .last()
                .map(|row| {
                    encode_cursor(&KeysetCursor {
                        version: 1,
                        method: METHOD.to_string(),
                        query_digest: query_digest.clone(),
                        source_revision: source_revision.clone(),
                        sort_value: row.modified_at.unwrap_or_default(),
                        stable_id: row.id.clone(),
                        tie_breaker_digest: None,
                        accepted_count: Some(end),
                        processed_prefix_digest: Some(next_prefix_digest.clone()),
                        resolved_start_at: None,
                        resolved_end_at: None,
                    })
                })
                .transpose()?
        } else {
            None
        };

        let mut roots = Vec::new();
        let mut gap_notes = Vec::new();
        let mut blocker_notes = Vec::new();
        let mut redacted_value_count = 0usize;
        let mut redacted_fields = BTreeSet::new();
        let mut placeholders = BTreeSet::new();
        let mut raw_secret_returned = false;
        for stream in &mut streams {
            roots.append(&mut stream.roots);
            gap_notes.append(&mut stream.gap_notes);
            blocker_notes.append(&mut stream.blocker_notes);
            redacted_value_count =
                redacted_value_count.saturating_add(stream.redaction_summary.redacted_value_count);
            redacted_fields.extend(std::mem::take(
                &mut stream.redaction_summary.redacted_fields,
            ));
            placeholders.extend(std::mem::take(&mut stream.redaction_summary.placeholders));
            raw_secret_returned |= stream.redaction_summary.raw_secret_returned;
        }
        if aggregate_limit_reached {
            gap_notes.push(
                "The all-agent session inventory reached its bounded accepted-row limit."
                    .to_string(),
            );
        }
        gap_notes.sort();
        gap_notes.dedup();
        blocker_notes.sort();
        blocker_notes.dedup();
        let count = page_rows.len();
        let total_candidate_count = streams
            .iter()
            .map(|stream| stream.total_candidate_count)
            .sum();
        let total_matched_count = if has_more {
            streams
                .iter()
                .map(|stream| stream.total_matched_count)
                .sum::<usize>()
                .max(end)
        } else {
            end
        };
        let user_message_count = page_rows.iter().map(|row| row.user_message_count).sum();
        let total_message_count = page_rows.iter().map(|row| row.total_message_count).sum();
        let tool_call_count = page_rows.iter().map(|row| row.tool_call_count).sum();
        let skill_call_count = page_rows.iter().map(|row| row.skill_call_count).sum();

        Ok(LocalSessionPreviewResult {
            generated_by: "local-v3.02-all-agents",
            authorized: streams.iter().any(|stream| stream.authorized),
            authorization_required: false,
            roots,
            count,
            total_candidate_count,
            total_matched_count,
            offset: start,
            limit,
            has_more,
            next_offset: None,
            next_cursor,
            source_revision: Some(source_revision),
            source_completeness: if limited {
                ListSourceCompleteness::Limited
            } else {
                ListSourceCompleteness::Enumerable
            },
            incomplete_reason: limited.then_some(ListIncompleteReason::SafetyBudget),
            candidate_set_truncated: limited,
            user_message_count,
            total_message_count,
            tool_call_count,
            skill_call_count,
            skill_usage_rows: Vec::new(),
            session_rows: page_rows,
            gap_notes,
            blocker_notes,
            redaction_summary: LocalPreviewRedactionSummary {
                status: "redacted-local-only".to_string(),
                redacted_value_count,
                redacted_fields: redacted_fields.into_iter().collect(),
                placeholders: placeholders.into_iter().collect(),
                raw_trace_persisted: false,
                raw_prompt_persisted: false,
                raw_response_persisted: false,
                raw_secret_returned,
            },
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

    fn collect_agent_session_prefix(
        &self,
        base: &LocalSessionPreviewParams,
        agent: AgentId,
        target: usize,
    ) -> Result<AgentSessionStream, ServiceError> {
        let mut rows = Vec::new();
        let mut roots = Vec::new();
        let mut total_candidate_count = 0usize;
        let mut total_matched_count = 0usize;
        let mut cursor = None;
        let mut source_revision = None;
        let mut limited = false;
        let mut authorized = false;
        let mut gap_notes = Vec::new();
        let mut blocker_notes = Vec::new();
        let mut redaction_summary = LocalPreviewRedactionSummary {
            status: "redacted-local-only".to_string(),
            redacted_value_count: 0,
            redacted_fields: Vec::new(),
            placeholders: Vec::new(),
            raw_trace_persisted: false,
            raw_prompt_persisted: false,
            raw_response_persisted: false,
            raw_secret_returned: false,
        };
        let mut has_more = true;

        while rows.len() < target && has_more {
            let previous_cursor = cursor.clone();
            let mut params = base.clone();
            params.agent = Some(agent.as_str().to_string());
            params.limit = Some((target - rows.len()).clamp(1, 100));
            params.cursor = cursor;
            params.source_revision = source_revision.clone();
            let result = self.preview_local_sessions(params)?;
            if roots.is_empty() {
                roots = result.roots;
            }
            authorized |= result.authorized;
            total_candidate_count = result.total_candidate_count;
            total_matched_count = result.total_matched_count;
            limited |= result.candidate_set_truncated
                || result.source_completeness != ListSourceCompleteness::Enumerable;
            gap_notes.extend(result.gap_notes);
            blocker_notes.extend(result.blocker_notes);
            redaction_summary.redacted_value_count = redaction_summary
                .redacted_value_count
                .saturating_add(result.redaction_summary.redacted_value_count);
            redaction_summary
                .redacted_fields
                .extend(result.redaction_summary.redacted_fields);
            redaction_summary
                .placeholders
                .extend(result.redaction_summary.placeholders);
            redaction_summary.raw_secret_returned |= result.redaction_summary.raw_secret_returned;
            rows.extend(result.session_rows);
            has_more = result.has_more && !limited;
            cursor = result.next_cursor;
            source_revision = result.source_revision;
            if has_more && cursor == previous_cursor {
                return Err(ServiceError::InvalidRequest(
                    "agent session keyset made no cursor progress".to_string(),
                ));
            }
        }

        Ok(AgentSessionStream {
            agent,
            rows,
            roots,
            total_candidate_count,
            total_matched_count,
            has_more,
            source_revision: source_revision.unwrap_or_else(|| "none".to_string()),
            limited,
            authorized,
            gap_notes,
            blocker_notes,
            redaction_summary,
        })
    }
}

fn aggregate_query_digest(params: &LocalSessionPreviewParams) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"session.previewLocalSessions.all-agents.query.v1\0");
    for value in [
        params.scope.as_deref().unwrap_or_default(),
        params.project_root.as_deref().unwrap_or_default(),
        params.current_cwd.as_deref().unwrap_or_default(),
        params.sort.as_deref().unwrap_or_default(),
        params.direction.as_deref().unwrap_or_default(),
    ] {
        hasher.update(value.trim().as_bytes());
        hasher.update([0]);
    }
    hasher.update(params.max_excerpt_chars.unwrap_or(1_000).to_le_bytes());
    format!("sha256:{}", hex_prefix(&hasher.finalize(), 64))
}

fn aggregate_source_revision(streams: &[AgentSessionStream]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"session.previewLocalSessions.all-agents.source.v1\0");
    for stream in streams {
        hasher.update(stream.agent.as_str().as_bytes());
        hasher.update([0]);
        hasher.update(stream.source_revision.as_bytes());
        hasher.update([0]);
    }
    format!("sha256:{}", hex_prefix(&hasher.finalize(), 64))
}

fn aggregate_prefix_digest(rows: &[LocalSessionPreviewRow]) -> String {
    let mut ids = rows.iter().map(|row| row.id.as_str()).collect::<Vec<_>>();
    ids.sort_unstable();
    let mut hasher = Sha256::new();
    hasher.update(b"session.previewLocalSessions.all-agents.prefix.v1\0");
    for id in ids {
        hasher.update(id.as_bytes());
        hasher.update([0]);
    }
    format!("sha256:{}", hex_prefix(&hasher.finalize(), 64))
}
