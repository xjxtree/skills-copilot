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

impl ServiceHost {
    #[allow(clippy::too_many_arguments)]
    pub(super) fn preview_local_sessions_keyset(
        &self,
        params: &LocalSessionPreviewParams,
        root_requests: Vec<LocalSessionRootRequest>,
        requested_agent: Option<&str>,
        project_filter_roots: &[PathBuf],
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
        let query_digest = local_session_keyset_query_digest(
            requested_agent,
            &root_requests,
            project_filter_roots,
            scope,
            include_content_items,
            max_excerpt_chars,
        );
        let cursor = params
            .cursor
            .as_deref()
            .map(|text| decode_cursor(text, METHOD, &query_digest))
            .transpose()?;
        let mut roots = Vec::<KeysetLocalSessionRoot>::new();
        let mut candidates = Vec::<KeysetLocalSessionCandidate>::new();
        let mut total_candidate_count = 0usize;
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
            total_candidate_count += inventory.total_candidate_count;
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
        let mut selected = candidates
            .into_iter()
            .skip(start)
            .take(limit + 1)
            .collect::<Vec<_>>();
        let has_more = !candidate_set_was_truncated && selected.len() > limit;
        if selected.len() > limit {
            selected.truncate(limit);
        }
        let next_cursor = if has_more {
            selected
                .last()
                .map(|candidate| {
                    encode_cursor(&KeysetCursor {
                        version: 1,
                        method: METHOD.to_string(),
                        query_digest: query_digest.clone(),
                        source_revision: source_revision.clone(),
                        sort_value: candidate.file.modified_at,
                        stable_id: candidate.row_id.clone(),
                        tie_breaker_digest: Some(candidate.path_digest.clone()),
                    })
                })
                .transpose()?
        } else {
            None
        };

        let skill_matchers = self.local_session_skill_matchers(requested_agent)?;
        let mut session_rows = Vec::new();
        let mut seen_session_row_ids = BTreeSet::new();
        let mut skill_usage = BTreeMap::<String, LocalSessionSkillUsageAccumulator>::new();
        for candidate in selected {
            let root = &roots[candidate.root_index];
            let options = LocalSessionPreviewRowOptions {
                requested_agent,
                max_excerpt_chars,
                source_kind: root.source_kind,
                skill_matchers: &skill_matchers,
                scope,
                project_filter_roots,
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
                Ok(Some(entry)) if seen_session_row_ids.insert(entry.row.id.clone()) => {
                    roots[candidate.root_index].accepted_count += 1;
                    update_local_session_skill_usage(&mut skill_usage, &entry);
                    session_rows.push(entry.row);
                }
                Ok(Some(_)) | Ok(None) => {}
                Err(error) => gap_notes.push(format!(
                    "{}: {}",
                    redactor.redact(&candidate.file.path.to_string_lossy()),
                    redactor.redact(&error.to_string())
                )),
            }
        }
        let count = session_rows.len();
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
            total_matched_count: total_candidate_count,
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
            skill_usage_rows: local_session_skill_usage_rows(skill_usage, limit),
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

fn local_session_keyset_query_digest(
    requested_agent: Option<&str>,
    roots: &[LocalSessionRootRequest],
    project_filter_roots: &[PathBuf],
    scope: LocalSessionScope,
    include_content_items: bool,
    max_excerpt_chars: usize,
) -> String {
    let root_digests = roots
        .iter()
        .map(|root| local_session_path_digest(&root.path));
    let project_digests = project_filter_roots
        .iter()
        .map(|root| local_session_path_digest(root));
    let mut hasher = Sha256::new();
    hasher.update(b"session.previewLocalSessions.query.v1\0");
    hasher.update(requested_agent.unwrap_or_default().trim().as_bytes());
    hasher.update([0]);
    hasher.update(scope.as_str().as_bytes());
    hasher.update([u8::from(include_content_items)]);
    hasher.update(max_excerpt_chars.to_le_bytes());
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
    hasher.update(b"session.previewLocalSessions.source.v1\0");
    for candidate in candidates {
        hasher.update(candidate.row_id.as_bytes());
        hasher.update([0]);
        hasher.update(candidate.file.modified_at.to_le_bytes());
        hasher.update(candidate.file.file_size.to_le_bytes());
        hasher.update(scope.as_str().as_bytes());
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
