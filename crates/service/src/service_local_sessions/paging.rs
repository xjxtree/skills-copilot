use super::*;
use crate::service_keyset_cursor::{decode_cursor, encode_cursor, KeysetCursor};

struct KeysetLocalSessionRoot {
    path: PathBuf,
    guarded_root: GuardedLocalSessionRoot,
    status: &'static str,
    source_kind: &'static str,
    redacted_root: String,
    accepted_count: usize,
}

struct KeysetLocalSessionCandidate {
    root_index: usize,
    file: crate::service_local_session_io::LocalSessionFileCandidate,
    row_id: String,
    path_digest: String,
}

struct KeysetLocalSessionPageRead {
    session_rows: Vec<LocalSessionPreviewRow>,
    skill_usage: BTreeMap<String, LocalSessionSkillUsageAccumulator>,
    last_processed_index: Option<usize>,
    next_index: usize,
    budget_exhausted: bool,
}

struct LocalSessionKeysetQuery<'a> {
    requested_agent: Option<&'a str>,
    roots: &'a [LocalSessionRootRequest],
    project_filter_roots: &'a [PathBuf],
    scope: LocalSessionScope,
    sort: LocalSessionSort,
    direction: SortDirection,
    has_search: bool,
    include_content_items: bool,
    max_excerpt_chars: usize,
}

pub(super) fn validate_local_session_keyset_shape(
    params: &LocalSessionPreviewParams,
) -> Result<(), ServiceError> {
    if params.session_id.is_some()
        || params.offset.is_some()
        || params.max_files.is_some()
        || params.include_content_items != Some(false)
    {
        return Err(ServiceError::InvalidRequest(
            "keyset session pages reject session_id, offset, max_files, and content detail fields"
                .to_string(),
        ));
    }
    if params.cursor.is_some() != params.source_revision.is_some() {
        return Err(ServiceError::InvalidRequest(
            "keyset continuation requires both cursor and source_revision".to_string(),
        ));
    }
    Ok(())
}

impl ServiceHost {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn preview_local_sessions_keyset(
        &self,
        params: &LocalSessionPreviewParams,
        root_requests: Vec<LocalSessionRootRequest>,
        requested_agent: Option<&str>,
        project_filter_roots: &[PathBuf],
        codex_home: &Path,
        scope: LocalSessionScope,
        sort: LocalSessionSort,
        direction: SortDirection,
        has_search: bool,
        limit: usize,
        max_excerpt_chars: usize,
        include_content_items: bool,
        io: &mut LocalSessionIoContext,
        mut gap_notes: Vec<String>,
        mut blocker_notes: Vec<String>,
        mut redactor: PromptRedactor<'_>,
    ) -> Result<LocalSessionPreviewResult, ServiceError> {
        if sort != LocalSessionSort::ModifiedAt
            || direction != SortDirection::Desc
            || has_search
            || scope != LocalSessionScope::All
        {
            return Err(ServiceError::InvalidRequest(
                "cursor session pages require all-scope recent order without server search"
                    .to_string(),
            ));
        }
        const METHOD: &str = "session.previewLocalSessions";
        let query_digest = local_session_keyset_query_digest(LocalSessionKeysetQuery {
            requested_agent,
            roots: &root_requests,
            project_filter_roots,
            scope,
            sort,
            direction,
            has_search,
            include_content_items,
            max_excerpt_chars,
        });
        let cursor = params
            .cursor
            .as_deref()
            .map(|text| decode_cursor(text, METHOD, &query_digest))
            .transpose()?;
        let mut roots = Vec::<KeysetLocalSessionRoot>::new();
        let mut candidates = Vec::<KeysetLocalSessionCandidate>::new();
        let mut candidate_set_was_truncated = false;
        for root_request in root_requests {
            let LocalSessionRootRequest {
                path,
                guarded_root,
                status,
                source_kind,
            } = root_request;
            let redacted_root = redactor.redact(&path.to_string_lossy());
            if !path.is_absolute() {
                blocker_notes.push(format!(
                    "{redacted_root}: Authorized session roots must be absolute paths."
                ));
                continue;
            }
            let guarded_root = match guarded_root {
                Ok(root) => root,
                Err(error) => {
                    blocker_notes.push(format!(
                        "{redacted_root}: {}",
                        redactor.redact(&format!(
                            "Authorized session root could not be opened safely: {error}"
                        ))
                    ));
                    continue;
                }
            };
            let inventory = collect_local_session_inventory(
                &guarded_root,
                &mut io.inventory_budget,
                &mut gap_notes,
                &mut redactor,
            );
            candidate_set_was_truncated |= inventory.truncated;
            let root_index = roots.len();
            roots.push(KeysetLocalSessionRoot {
                path,
                guarded_root,
                status,
                source_kind,
                redacted_root,
                accepted_count: 0,
            });
            candidates.extend(inventory.candidates.into_iter().map(|file| {
                let row_id = local_session_row_id(&file.path);
                let path_digest = local_session_path_digest(&file.path);
                KeysetLocalSessionCandidate {
                    root_index,
                    file,
                    row_id,
                    path_digest,
                }
            }));
        }
        candidates.sort_by(|left, right| {
            right
                .file
                .modified_at
                .cmp(&left.file.modified_at)
                .then_with(|| left.row_id.cmp(&right.row_id))
                .then_with(|| left.path_digest.cmp(&right.path_digest))
        });
        let mut seen_candidate_ids = BTreeSet::new();
        candidates.retain(|candidate| seen_candidate_ids.insert(candidate.row_id.clone()));
        let total_candidate_count = candidates.len();
        let source_revision = local_session_source_revision(&candidates, scope);
        let cursor_revision = cursor
            .as_ref()
            .map(|cursor| cursor.source_revision.as_str());
        if params
            .source_revision
            .as_deref()
            .is_some_and(|revision| revision != source_revision)
            || cursor_revision.is_some_and(|revision| revision != source_revision)
        {
            return Err(ServiceError::SourceChanged);
        }
        let start = cursor.as_ref().map_or(0, |cursor| {
            candidates.partition_point(|candidate| {
                !local_session_candidate_is_after_cursor(candidate, cursor)
            })
        });
        if let Some(cursor) = cursor.as_ref() {
            let current_prefix_digest = local_session_processed_prefix_digest(&candidates[..start]);
            if cursor.processed_prefix_digest.as_deref() != Some(&current_prefix_digest) {
                return Err(ServiceError::SourceChanged);
            }
        }
        let accepted_before = match cursor.as_ref() {
            Some(cursor) => match cursor.accepted_count {
                Some(accepted_count) if accepted_count <= start => accepted_count,
                _ => {
                    return Err(ServiceError::InvalidRequest(
                        "cursor accepted count is invalid".to_string(),
                    ));
                }
            },
            None => 0,
        };
        let skill_matchers = self.local_session_skill_matchers(requested_agent)?;
        let mut session_rows = Vec::new();
        let mut seen_session_row_ids = BTreeSet::new();
        let mut skill_usage = BTreeMap::<String, LocalSessionSkillUsageAccumulator>::new();
        let mut last_processed_index = None;
        let mut next_index = start;
        let mut page_budget_exhausted = false;
        for (candidate_index, candidate) in candidates.iter().enumerate().skip(start) {
            let root = &roots[candidate.root_index];
            let options = LocalSessionPreviewRowOptions {
                requested_agent,
                max_excerpt_chars,
                source_kind: root.source_kind,
                skill_matchers: &skill_matchers,
                scope,
                project_filter_roots,
                codex_home,
                search: None,
                include_content_items,
            };
            match local_session_preview_row(
                &candidate.file.path,
                &root.path,
                &root.guarded_root,
                options,
                io,
                &mut gap_notes,
                &mut redactor,
            ) {
                Ok(outcome) => {
                    page_budget_exhausted |= outcome.budget_exhausted;
                    if let Some(entry) = outcome.entry {
                        if seen_session_row_ids.insert(entry.row.id.clone()) {
                            roots[candidate.root_index].accepted_count += 1;
                            update_local_session_skill_usage(&mut skill_usage, &entry);
                            session_rows.push(entry.row);
                        }
                    }
                }
                Err(error) => gap_notes.push(format!(
                    "{}: {}",
                    redactor.redact(&candidate.file.path.to_string_lossy()),
                    redactor.redact(&error.to_string())
                )),
            }
            last_processed_index = Some(candidate_index);
            next_index = candidate_index.saturating_add(1);
            if page_budget_exhausted || next_index.saturating_sub(start) == limit {
                break;
            }
        }
        let page_read = KeysetLocalSessionPageRead {
            session_rows,
            skill_usage,
            last_processed_index,
            next_index,
            budget_exhausted: page_budget_exhausted,
        };
        let KeysetLocalSessionPageRead {
            session_rows,
            skill_usage,
            last_processed_index,
            next_index,
            budget_exhausted,
        } = page_read;
        candidate_set_was_truncated |= budget_exhausted;
        let has_more = !candidate_set_was_truncated && next_index < candidates.len();
        let accepted_through = accepted_before.saturating_add(session_rows.len());
        let next_cursor = if has_more {
            last_processed_index
                .and_then(|index| candidates.get(index))
                .map(|candidate| {
                    encode_cursor(&KeysetCursor {
                        version: 1,
                        method: METHOD.to_string(),
                        query_digest: query_digest.clone(),
                        source_revision: source_revision.clone(),
                        sort_value: candidate.file.modified_at,
                        stable_id: candidate.row_id.clone(),
                        tie_breaker_digest: Some(candidate.path_digest.clone()),
                        accepted_count: Some(accepted_through),
                        processed_prefix_digest: Some(local_session_processed_prefix_digest(
                            &candidates[..next_index],
                        )),
                        resolved_start_at: None,
                        resolved_end_at: None,
                    })
                })
                .transpose()?
        } else {
            None
        };
        let count = session_rows.len();
        let total_matched_count = if has_more {
            accepted_through.saturating_add(candidates.len().saturating_sub(next_index))
        } else {
            accepted_through
        };
        let user_message_count = session_rows.iter().map(|row| row.user_message_count).sum();
        let total_message_count = session_rows.iter().map(|row| row.total_message_count).sum();
        let tool_call_count = session_rows.iter().map(|row| row.tool_call_count).sum();
        let skill_call_count = session_rows.iter().map(|row| row.skill_call_count).sum();
        if candidate_set_was_truncated {
            gap_notes.push(
                "Local session candidate set was truncated by bounded inventory limits."
                    .to_string(),
            );
        }
        let source_completeness = if candidate_set_was_truncated {
            ListSourceCompleteness::Limited
        } else {
            ListSourceCompleteness::Enumerable
        };
        let incomplete_reason =
            candidate_set_was_truncated.then_some(ListIncompleteReason::SafetyBudget);
        let root_rows = roots
            .into_iter()
            .map(|root| LocalSessionPreviewRoot {
                root: root.redacted_root,
                status: root.status.to_string(),
                candidate_count: root.accepted_count,
                blocker: None,
            })
            .collect::<Vec<_>>();
        Ok(LocalSessionPreviewResult {
            generated_by: "local-v2.98",
            authorized: !root_rows.is_empty(),
            authorization_required: false,
            roots: root_rows,
            count,
            total_candidate_count,
            total_matched_count,
            offset: 0,
            limit,
            has_more,
            next_offset: None,
            next_cursor,
            source_revision: Some(source_revision),
            source_completeness,
            incomplete_reason,
            candidate_set_truncated: candidate_set_was_truncated,
            user_message_count,
            total_message_count,
            tool_call_count,
            skill_call_count,
            skill_usage_rows: local_session_skill_usage_rows(skill_usage),
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
}

fn local_session_path_digest(path: &Path) -> String {
    trace_content_hash(&local_session_normalized_path(path))
}

fn local_session_keyset_query_digest(query: LocalSessionKeysetQuery<'_>) -> String {
    let root_digests = query
        .roots
        .iter()
        .map(|root| local_session_path_digest(&root.path));
    let project_digests = query
        .project_filter_roots
        .iter()
        .map(|root| local_session_path_digest(root));
    let mut hasher = Sha256::new();
    hasher.update(b"session.previewLocalSessions.query.v1\0");
    hasher.update(query.requested_agent.unwrap_or_default().trim().as_bytes());
    hasher.update([0]);
    hasher.update(query.scope.as_str().as_bytes());
    hasher.update(match query.sort {
        LocalSessionSort::ModifiedAt => b"modified_at".as_slice(),
        LocalSessionSort::Title => b"title".as_slice(),
    });
    hasher.update(match query.direction {
        SortDirection::Asc => b"asc".as_slice(),
        SortDirection::Desc => b"desc".as_slice(),
    });
    hasher.update([u8::from(query.has_search)]);
    hasher.update([u8::from(query.include_content_items)]);
    hasher.update(query.max_excerpt_chars.to_le_bytes());
    for digest in root_digests.chain(project_digests) {
        hasher.update([0]);
        hasher.update(digest.as_bytes());
    }
    format!("sha256:{}", hex_prefix(&hasher.finalize(), 64))
}

fn local_session_source_revision(
    candidates: &[KeysetLocalSessionCandidate],
    scope: LocalSessionScope,
) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"session.previewLocalSessions.source.v2\0");
    let mut identities = candidates
        .iter()
        .map(|candidate| (candidate.row_id.as_str(), candidate.path_digest.as_str()))
        .collect::<Vec<_>>();
    identities.sort_unstable();
    for (row_id, path_digest) in identities {
        hasher.update(row_id.as_bytes());
        hasher.update([0]);
        hasher.update(path_digest.as_bytes());
        hasher.update([0]);
        hasher.update(scope.as_str().as_bytes());
        hasher.update([0]);
    }
    format!("sha256:{}", hex_prefix(&hasher.finalize(), 64))
}

fn local_session_processed_prefix_digest(candidates: &[KeysetLocalSessionCandidate]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(b"session.previewLocalSessions.processed-prefix.v1\0");
    let mut identities = candidates
        .iter()
        .map(|candidate| (candidate.row_id.as_str(), candidate.path_digest.as_str()))
        .collect::<Vec<_>>();
    identities.sort_unstable();
    for (row_id, path_digest) in identities {
        hasher.update(row_id.as_bytes());
        hasher.update([0]);
        hasher.update(path_digest.as_bytes());
        hasher.update([0]);
    }
    format!("sha256:{}", hex_prefix(&hasher.finalize(), 64))
}

fn local_session_candidate_is_after_cursor(
    candidate: &KeysetLocalSessionCandidate,
    cursor: &KeysetCursor,
) -> bool {
    candidate.file.modified_at < cursor.sort_value
        || (candidate.file.modified_at == cursor.sort_value
            && (candidate.row_id > cursor.stable_id
                || (candidate.row_id == cursor.stable_id
                    && cursor
                        .tie_breaker_digest
                        .as_deref()
                        .is_some_and(|digest| candidate.path_digest.as_str() > digest))))
}
