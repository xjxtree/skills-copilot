use super::service_local_session_io::{
    read_bounded_sidecar_text, read_bounded_text, select_newest_candidates, BoundedReadSpec,
    BoundedText, GuardedLocalSessionRoot, LocalSessionInventory, LocalSessionInventoryBudget,
    LocalSessionIoContext, LocalSessionReadBudget, LocalSessionReadLimits, SessionSidecarBudget,
    MAX_PROVENANCE_TOKEN_BYTES,
};
use super::*;
use skills_copilot_adapters::{
    claude_config_dir, codex_home_dir, hermes_home_dir, openclaw_config_path, openclaw_state_dir,
    opencode_data_dir, pi_agent_dir,
};
use std::collections::HashMap;
use time::{format_description::well_known::Rfc3339, OffsetDateTime};

mod classification;
mod message_paging;
mod paging;
mod sqlite_sessions;

use classification::{
    is_internal_local_session_title_block, is_json_thinking_type, is_json_tool_result_type,
    is_json_tool_type, is_supported_local_session_file, is_unhelpful_local_session_title,
    json_session_has_nonfinal_process_signal, local_session_metadata_is_internal,
    local_session_phase_classification, local_session_phase_name_classification,
    local_session_role_classification, local_session_type_name_classification,
};

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum LocalSessionSort {
    ModifiedAt,
    Title,
}

impl LocalSessionSort {
    fn parse(value: Option<&str>) -> Result<Self, ServiceError> {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            None | Some("recent") | Some("modified_at") => Ok(Self::ModifiedAt),
            Some("title") => Ok(Self::Title),
            Some(value) => Err(ServiceError::InvalidRequest(format!(
                "unsupported local session sort '{value}'"
            ))),
        }
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SortDirection {
    Asc,
    Desc,
}

impl SortDirection {
    fn parse(value: Option<&str>, default: Self) -> Result<Self, ServiceError> {
        match value.map(str::trim).filter(|value| !value.is_empty()) {
            None => Ok(default),
            Some("asc") => Ok(Self::Asc),
            Some("desc") => Ok(Self::Desc),
            Some(value) => Err(ServiceError::InvalidRequest(format!(
                "unsupported local session direction '{value}'"
            ))),
        }
    }
}

impl ServiceHost {
    pub fn preview_local_sessions(
        &self,
        params: LocalSessionPreviewParams,
    ) -> Result<LocalSessionPreviewResult, ServiceError> {
        let mut limits = LocalSessionReadLimits::default();
        if params
            .session_id
            .as_deref()
            .is_some_and(|session_id| !session_id.trim().is_empty())
        {
            // A selected detail reads one exact candidate, so it can spend more
            // of the existing request budget on a representative Codex JSONL
            // head without multiplying work across the summary inventory.
            limits.primary_head_bytes = 4 * 1024 * 1024;
            limits.primary_tail_bytes = 512 * 1024;
        }
        let mut io = LocalSessionIoContext::new(limits);
        self.preview_local_sessions_with_io(params, &mut io)
    }

    #[cfg(all(test, unix))]
    pub(crate) fn preview_local_sessions_with_test_limits(
        &self,
        params: LocalSessionPreviewParams,
        limits: LocalSessionReadLimits,
    ) -> Result<LocalSessionPreviewResult, ServiceError> {
        let mut io = LocalSessionIoContext::new(limits);
        self.preview_local_sessions_with_io(params, &mut io)
    }

    pub(crate) fn preview_local_sessions_with_io(
        &self,
        params: LocalSessionPreviewParams,
        io: &mut LocalSessionIoContext,
    ) -> Result<LocalSessionPreviewResult, ServiceError> {
        let include_content_items = params.include_content_items.unwrap_or(true);
        let requested_session_id = params
            .session_id
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToOwned::to_owned);
        let paging_mode = params
            .paging_mode
            .as_deref()
            .map(str::trim)
            .filter(|value| !value.is_empty());
        if paging_mode.is_some_and(|mode| mode != "keyset") {
            return Err(ServiceError::InvalidRequest(
                "unsupported local session paging_mode".to_string(),
            ));
        }
        let sort = LocalSessionSort::parse(params.sort.as_deref())?;
        let default_direction = match sort {
            LocalSessionSort::ModifiedAt => SortDirection::Desc,
            LocalSessionSort::Title => SortDirection::Asc,
        };
        let direction = SortDirection::parse(params.direction.as_deref(), default_direction)?;
        let limit = params.limit.unwrap_or(20).clamp(1, 100);
        let max_files = params.max_files.unwrap_or(200).clamp(1, 1_000);
        let max_excerpt_chars = params.max_excerpt_chars.unwrap_or(1_000).clamp(120, 4_000);
        let requested_roots = normalize_string_list(params.authorized_roots.clone());
        let auto_discover = params.auto_discover.unwrap_or(requested_roots.is_empty());
        let adapter_ctx = self.effective_adapter_ctx()?;
        let codex_home = codex_home_dir(&adapter_ctx);
        let codex_home = codex_home.canonicalize().unwrap_or(codex_home);
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
        let uses_keyset_paging = paging_mode == Some("keyset") || params.cursor.is_some();
        if uses_keyset_paging {
            paging::validate_local_session_keyset_shape(&params)?;
        } else if params.source_revision.is_some() {
            return Err(ServiceError::InvalidRequest(
                "source_revision requires keyset paging".to_string(),
            ));
        }

        if requested_roots.is_empty() && auto_discover {
            if let Some(result) = sqlite_sessions::preview_sqlite_sessions(
                &adapter_ctx,
                &params,
                io,
                requested_agent,
                scope,
                sort,
                direction,
                search.as_deref(),
                include_content_items,
                limit,
                max_excerpt_chars,
                &redaction_roots,
            )? {
                return Ok(result);
            }
        }

        let mut root_requests = requested_roots
            .iter()
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
        run_scheduled_local_session_root_swap_test_hook(&adapter_ctx.user_home);

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
                next_cursor: None,
                source_revision: None,
                source_completeness: ListSourceCompleteness::Enumerable,
                incomplete_reason: None,
                candidate_set_truncated: false,
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

        if uses_keyset_paging {
            return self.preview_local_sessions_keyset(
                &params,
                root_requests,
                requested_agent,
                &project_filter_roots,
                &codex_home,
                scope,
                sort,
                direction,
                search.is_some(),
                limit,
                max_excerpt_chars,
                include_content_items,
                io,
                gap_notes,
                blocker_notes,
                redactor,
            );
        }

        let mut root_rows = Vec::new();
        let mut session_rows = Vec::new();
        let mut seen_session_row_ids = BTreeSet::new();
        let mut skill_usage = BTreeMap::<String, LocalSessionSkillUsageAccumulator>::new();
        let skill_matchers = self.local_session_skill_matchers(requested_agent)?;
        let mut total_candidate_count = 0usize;
        let mut candidate_set_was_truncated = false;
        for root_request in root_requests {
            let LocalSessionRootRequest {
                path: root_path,
                guarded_root,
                status,
                source_kind,
            } = root_request;
            let root = root_path.to_string_lossy().to_string();
            let redacted_root = redactor.redact(&root);
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
            let guarded_root = match guarded_root {
                Ok(root) => root,
                Err(error) => {
                    let blocker =
                        format!("Authorized session root could not be opened safely: {error}");
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

            let inventory = collect_local_session_inventory(
                &guarded_root,
                requested_agent,
                &mut io.inventory_budget,
                &mut gap_notes,
                &mut redactor,
            );
            total_candidate_count += inventory.total_candidate_count;
            let inventory_truncated = inventory.truncated;
            let inventory_candidate_count = inventory.total_candidate_count;
            let files = if let Some(session_id) = requested_session_id.as_deref() {
                inventory
                    .candidates
                    .into_iter()
                    .filter(|candidate| local_session_row_id(&candidate.path) == session_id)
                    .collect::<Vec<_>>()
            } else {
                select_newest_candidates(inventory.candidates, max_files)
            };
            candidate_set_was_truncated |= inventory_truncated
                || (requested_session_id.is_none() && inventory_candidate_count > files.len());
            let mut root_candidate_count = 0usize;
            for candidate in files {
                let file = candidate.path;
                let options = LocalSessionPreviewRowOptions {
                    requested_agent,
                    max_excerpt_chars,
                    source_kind,
                    skill_matchers: &skill_matchers,
                    scope,
                    project_filter_roots: &project_filter_roots,
                    codex_home: &codex_home,
                    search: search.as_deref(),
                    include_content_items,
                };
                match local_session_preview_row(
                    &file,
                    &root_path,
                    &guarded_root,
                    options,
                    io,
                    &mut gap_notes,
                    &mut redactor,
                ) {
                    Ok(outcome) => {
                        candidate_set_was_truncated |= outcome.budget_exhausted;
                        if let Some(entry) = outcome.entry {
                            if seen_session_row_ids.insert(entry.row.id.clone()) {
                                root_candidate_count += 1;
                                update_local_session_skill_usage(&mut skill_usage, &entry);
                                session_rows.push(entry.row);
                            }
                        }
                    }
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
                status: status.to_string(),
                candidate_count: root_candidate_count,
                blocker: None,
            });
        }

        sort_local_session_rows(&mut session_rows, sort, direction);
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
        if candidate_set_was_truncated {
            gap_notes.push(
                "Local session candidate set was truncated by bounded inventory or max-files limits."
                    .to_string(),
            );
        }
        if total_matched_count == 0 && blocker_notes.is_empty() {
            gap_notes.push(
                "Discovered local session stores did not contain supported session files (.jsonl, .json, .txt, .log)."
                    .to_string(),
            );
        }
        let skill_usage_rows = local_session_skill_usage_rows(skill_usage);

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
            next_cursor: None,
            source_revision: None,
            source_completeness: if candidate_set_was_truncated || !blocker_notes.is_empty() {
                ListSourceCompleteness::Limited
            } else {
                ListSourceCompleteness::Enumerable
            },
            incomplete_reason: if candidate_set_was_truncated {
                Some(ListIncompleteReason::SafetyBudget)
            } else if !blocker_notes.is_empty() {
                Some(ListIncompleteReason::UnreadableSource)
            } else {
                None
            },
            candidate_set_truncated: candidate_set_was_truncated,
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

fn sort_local_session_rows(
    rows: &mut [LocalSessionPreviewRow],
    sort: LocalSessionSort,
    direction: SortDirection,
) {
    rows.sort_by(|left, right| {
        let primary = match sort {
            LocalSessionSort::ModifiedAt => left.modified_at.cmp(&right.modified_at),
            LocalSessionSort::Title => left.title.to_lowercase().cmp(&right.title.to_lowercase()),
        };
        let primary = match direction {
            SortDirection::Asc => primary,
            SortDirection::Desc => primary.reverse(),
        };
        primary
            .then_with(|| left.title.cmp(&right.title))
            .then_with(|| right.modified_at.cmp(&left.modified_at))
            .then_with(|| left.id.cmp(&right.id))
    });
}

struct LocalSessionRootRequest {
    path: PathBuf,
    guarded_root: std::io::Result<GuardedLocalSessionRoot>,
    status: &'static str,
    source_kind: &'static str,
}

#[cfg(all(test, unix))]
struct ScheduledLocalSessionRootSwapTestHook {
    user_home: PathBuf,
    action: Box<dyn FnOnce() + Send>,
}

#[cfg(all(test, unix))]
static SCHEDULED_LOCAL_SESSION_ROOT_SWAP_TEST_HOOK: std::sync::Mutex<
    Option<ScheduledLocalSessionRootSwapTestHook>,
> = std::sync::Mutex::new(None);

#[cfg(all(test, unix))]
pub(crate) fn install_scheduled_local_session_root_swap_test_hook(
    user_home: PathBuf,
    action: impl FnOnce() + Send + 'static,
) {
    let mut hook = SCHEDULED_LOCAL_SESSION_ROOT_SWAP_TEST_HOOK
        .lock()
        .expect("lock scheduled local-session root-swap test hook");
    assert!(hook.is_none(), "scheduled root-swap test hook already set");
    *hook = Some(ScheduledLocalSessionRootSwapTestHook {
        user_home,
        action: Box::new(action),
    });
}

#[cfg(all(test, unix))]
fn run_scheduled_local_session_root_swap_test_hook(user_home: &Path) {
    let action = {
        let mut hook = SCHEDULED_LOCAL_SESSION_ROOT_SWAP_TEST_HOOK
            .lock()
            .expect("lock scheduled local-session root-swap test hook");
        if hook
            .as_ref()
            .is_some_and(|scheduled| scheduled.user_home == user_home)
        {
            hook.take().map(|scheduled| scheduled.action)
        } else {
            None
        }
    };
    if let Some(action) = action {
        action();
    }
}

#[cfg(not(all(test, unix)))]
fn run_scheduled_local_session_root_swap_test_hook(_user_home: &Path) {}

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
    codex_home: &'a Path,
    search: Option<&'a str>,
    include_content_items: bool,
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
    is_internal_thread: bool,
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
        let claude_projects = claude_config_dir(adapter_ctx).join("projects");
        let mut pushed_project_root = false;
        if scope == LocalSessionScope::Project {
            for project in project_roots {
                let encoded = encode_claude_project_session_dir(project);
                pushed_project_root |= push_existing_session_root(
                    &mut roots,
                    home,
                    claude_projects.join(encoded),
                    "auto-discovered-read-only",
                    "auto-local-session",
                );
            }
        }
        if scope == LocalSessionScope::All || project_roots.is_empty() || !pushed_project_root {
            push_existing_session_root(
                &mut roots,
                home,
                claude_projects,
                "auto-discovered-read-only",
                "auto-local-session",
            );
        }
    }

    if local_session_agent_matches(requested_agent, AgentId::Codex.as_str()) {
        let codex_home = codex_home_dir(adapter_ctx);
        push_existing_session_root(
            &mut roots,
            home,
            codex_home.join("sessions"),
            "auto-discovered-read-only",
            "auto-local-session",
        );
    }

    if local_session_agent_matches(requested_agent, AgentId::Opencode.as_str())
        && opencode_data_dir(adapter_ctx).join("opencode.db").exists()
    {
        notes.push("OpenCode sessions are loaded from its current SQLite database.".to_string());
    }

    if local_session_agent_matches(requested_agent, AgentId::Pi.as_str()) {
        let pi_sessions = pi_session_dir(adapter_ctx);
        let mut pushed_project_root = false;
        if scope == LocalSessionScope::Project {
            for project in project_roots {
                for encoded in encode_pi_project_session_dirs(project) {
                    pushed_project_root |= push_existing_session_root(
                        &mut roots,
                        home,
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
                home,
                pi_sessions,
                "auto-discovered-read-only",
                "auto-local-session",
            );
        }
    }

    if local_session_agent_matches(requested_agent, AgentId::Hermes.as_str()) {
        let state_db = hermes_home_dir(adapter_ctx).join("state.db");
        if state_db.exists() {
            notes
                .push("Hermes sessions are loaded from its canonical SQLite database.".to_string());
        }
    }

    if local_session_agent_matches(requested_agent, AgentId::Openclaw.as_str()) {
        let openclaw_root = openclaw_state_dir(adapter_ctx);
        if !openclaw_agent_database_paths(&openclaw_root).is_empty() {
            notes.push(
                "OpenClaw sessions are loaded from each agent's canonical SQLite database."
                    .to_string(),
            );
        } else if openclaw_has_legacy_session_storage(&openclaw_root) {
            notes.push(
                "Legacy OpenClaw JSON/JSONL files were detected but are not active session storage; no canonical per-agent SQLite database was found."
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

fn pi_session_dir(adapter_ctx: &AdapterContext) -> PathBuf {
    if let Some(path) = std::env::var_os("PI_CODING_AGENT_SESSION_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        return path;
    }
    let agent_dir = pi_agent_dir(adapter_ctx);
    let settings_path = agent_dir.join("settings.json");
    if let Ok(content) = std::fs::read_to_string(settings_path) {
        if let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) {
            if let Some(raw) = value.get("sessionDir").and_then(serde_json::Value::as_str) {
                let path = PathBuf::from(raw);
                if path.is_absolute() {
                    return path;
                }
                if let Some(rest) = raw.strip_prefix("~/") {
                    return adapter_ctx.user_home.join(rest);
                }
                return agent_dir.join(path);
            }
        }
    }
    agent_dir.join("sessions")
}

fn openclaw_agent_database_paths(state_dir: &Path) -> Vec<PathBuf> {
    let agents_dir = state_dir.join("agents");
    let Ok(entries) = std::fs::read_dir(agents_dir) else {
        return Vec::new();
    };
    entries
        .filter_map(Result::ok)
        .filter_map(|entry| {
            let path = entry.path().join("agent/openclaw-agent.sqlite");
            path.is_file().then_some(path)
        })
        .collect()
}

fn openclaw_has_legacy_session_storage(state_dir: &Path) -> bool {
    std::fs::read_dir(state_dir.join("agents"))
        .into_iter()
        .flatten()
        .flatten()
        .any(|entry| entry.path().join("sessions").is_dir())
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
    authorization_anchor: &Path,
    path: PathBuf,
    status: &'static str,
    source_kind: &'static str,
) -> bool {
    let Ok(guarded_root) = GuardedLocalSessionRoot::open_beneath(authorization_anchor, &path)
    else {
        return false;
    };
    roots.push(LocalSessionRootRequest {
        path: guarded_root.path().to_path_buf(),
        guarded_root: Ok(guarded_root),
        status,
        source_kind,
    });
    true
}

fn dedupe_local_session_root_requests(roots: &mut Vec<LocalSessionRootRequest>) {
    let mut seen = BTreeSet::new();
    roots.retain(|root| seen.insert(root.path.clone()));
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

fn collect_local_session_inventory(
    root: &GuardedLocalSessionRoot,
    requested_agent: Option<&str>,
    budget: &mut LocalSessionInventoryBudget,
    gap_notes: &mut Vec<String>,
    redactor: &mut PromptRedactor<'_>,
) -> LocalSessionInventory {
    let collected = match root.collect_regular_files(budget, |path| {
        is_supported_local_session_file(path)
            && !is_ignored_local_session_file(path, requested_agent)
    }) {
        Ok(inventory) => inventory,
        Err(error) => {
            gap_notes.push(format!(
                "{}: {}",
                redactor.redact(&root.path().to_string_lossy()),
                redactor.redact(&error.to_string())
            ));
            return LocalSessionInventory::default();
        }
    };
    for (directory, error) in collected.directory_errors {
        gap_notes.push(format!(
            "{}: {}",
            redactor.redact(&directory.to_string_lossy()),
            redactor.redact(&error.to_string())
        ));
    }
    collected.inventory
}

fn is_ignored_local_session_file(path: &Path, requested_agent: Option<&str>) -> bool {
    if path
        .file_name()
        .and_then(|name| name.to_str())
        .is_some_and(|name| name.ends_with(".meta.json"))
    {
        return true;
    }
    let agent = requested_agent.unwrap_or_default();
    let has_component = |candidates: &[&str]| {
        path.components().any(|component| {
            component
                .as_os_str()
                .to_str()
                .is_some_and(|name| candidates.contains(&name))
        })
    };
    let all_agents = agent.is_empty() || agent.eq_ignore_ascii_case("all");
    has_component(&["memory", "subagent", "subagents", "subagent-artifacts"])
        || ((all_agents || agent.eq_ignore_ascii_case(AgentId::Openclaw.as_str()))
            && path.file_name().and_then(|name| name.to_str()) == Some("sessions.json"))
        || ((all_agents || agent.eq_ignore_ascii_case(AgentId::ClaudeCode.as_str()))
            && has_component(&["tool-results"]))
        || ((all_agents || agent.eq_ignore_ascii_case(AgentId::Opencode.as_str()))
            && has_component(&["message", "part", "project", "session_diff"]))
        || (agent.eq_ignore_ascii_case(AgentId::Pi.as_str())
            && path.file_name().and_then(|name| name.to_str()) == Some("session.jsonl"))
}

#[rustfmt::skip]
struct LocalSessionPreviewReadOutcome { entry: Option<LocalSessionPreviewEntry>, budget_exhausted: bool }
impl LocalSessionPreviewReadOutcome {
    #[rustfmt::skip]
    fn rejected(budget_exhausted: bool) -> Self {
        Self { entry: None, budget_exhausted }
    }
}
#[rustfmt::skip]
struct LocalSessionPrimaryRead { content: String, modified_at: Option<i64>, budget_exhausted: bool }

fn local_session_preview_row(
    path: &Path,
    root: &Path,
    guarded_root: &GuardedLocalSessionRoot,
    options: LocalSessionPreviewRowOptions<'_>,
    io: &mut LocalSessionIoContext,
    gap_notes: &mut Vec<String>,
    redactor: &mut PromptRedactor<'_>,
) -> Result<LocalSessionPreviewReadOutcome, ServiceError> {
    if !path.starts_with(root) {
        return Ok(LocalSessionPreviewReadOutcome::rejected(false));
    }
    let primary_read = read_local_session_file_content(path, guarded_root, io)?;
    let mut budget_exhausted = primary_read.budget_exhausted;
    let file_content = strip_internal_local_session_records(&primary_read.content);
    if file_content.is_empty() {
        return Ok(LocalSessionPreviewReadOutcome::rejected(budget_exhausted));
    }
    let accepted_file_content = accepted_local_session_content(&file_content);
    let enriched_content =
        enrich_local_session_content(path, guarded_root, &accepted_file_content, io);
    if enriched_content.sidecars_truncated {
        budget_exhausted = true;
        gap_notes.push(
            "OpenCode sidecar enrichment was truncated by bounded file, session-byte, or session-file limits."
                .to_string(),
        );
    }
    let enriched_content = enriched_content.content;
    let content = if enriched_content == accepted_file_content {
        accepted_file_content
    } else {
        accepted_local_session_content(&enriched_content)
    };
    if content.trim().is_empty() {
        return Ok(LocalSessionPreviewReadOutcome::rejected(budget_exhausted));
    }
    let mut metadata = local_session_parsed_metadata(path, &content, options.requested_agent);
    if metadata.is_internal_thread {
        return Ok(LocalSessionPreviewReadOutcome::rejected(budget_exhausted));
    }
    if let Some(project_root) =
        local_session_storage_project_root(path, root, options.project_filter_roots)
    {
        metadata.project_root = Some(project_root);
    }
    if path.starts_with(options.codex_home.join("sessions")) {
        metadata.title = metadata
            .session_id
            .as_deref()
            .and_then(|session_id| codex_session_index_title(io, options.codex_home, session_id))
            .or(metadata.title);
    }
    if !local_session_matches_scope(
        options.scope,
        options.project_filter_roots,
        metadata.project_root.as_deref(),
    ) {
        return Ok(LocalSessionPreviewReadOutcome::rejected(budget_exhausted));
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
        return Ok(LocalSessionPreviewReadOutcome::rejected(budget_exhausted));
    }
    let skill_invocations = extract_local_session_skill_invocation_names(&content);
    let skill_mentions = detect_local_session_skill_mentions(
        &skill_invocations,
        options.skill_matchers,
        &format!("session.content_hash:{short_hash}"),
    );
    let content_drafts =
        local_session_content_drafts(&content, &short_hash, &skill_mentions, &skill_invocations);
    let (started_at, ended_at) =
        local_session_time_bounds(&content, &content_drafts, primary_read.modified_at);
    let metrics = local_session_metrics(&content_drafts, skill_invocations.len());
    let content_items = if options.include_content_items {
        local_session_content_items(
            content_drafts,
            &short_hash,
            options.max_excerpt_chars,
            redactor,
        )
    } else {
        Vec::new()
    };
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
    Ok(LocalSessionPreviewReadOutcome {
        entry: Some(LocalSessionPreviewEntry {
            row: LocalSessionPreviewRow {
                id: row_id,
                title,
                source_kind: options.source_kind.to_string(),
                scope: options.scope.as_str().to_string(),
                agent,
                project_root: redacted_project_root,
                redacted_path: redacted_path.clone(),
                modified_at: primary_read.modified_at,
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
                content_included: options.include_content_items,
                content_items,
            },
            skill_mentions,
        }),
        budget_exhausted,
    })
}

fn read_local_session_file_content(
    path: &Path,
    root: &GuardedLocalSessionRoot,
    io: &mut LocalSessionIoContext,
) -> Result<LocalSessionPrimaryRead, ServiceError> {
    #[cfg(test)]
    io.primary_paths_read.push(path.to_path_buf());
    let bounded = read_bounded_text(
        root,
        path,
        BoundedReadSpec {
            head_bytes: io.limits.primary_head_bytes,
            tail_bytes: io.limits.primary_tail_bytes,
            line_fragment_bytes: io.limits.max_line_fragment_bytes,
        },
        &mut io.budget,
    )?;
    debug_assert!(bounded.bytes_read <= io.limits.max_preview_read_bytes);
    Ok(LocalSessionPrimaryRead {
        content: compact_bounded_local_session_content(&bounded, io.limits.max_line_fragment_bytes),
        modified_at: bounded.modified_at_millis,
        budget_exhausted: bounded.request_budget_exhausted,
    })
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
    let complete_document = text
        .trim()
        .strip_prefix('\u{feff}')
        .unwrap_or_else(|| text.trim())
        .trim();
    if !complete_document.is_empty()
        && parse_local_session_json_with_raw_classification_bounds(complete_document).is_ok()
    {
        if should_skip_local_session_sidecar_line(complete_document) {
            return String::new();
        }
        let mut content = String::new();
        if complete_document.len().saturating_add(1) <= max_line_fragment_bytes {
            content.push_str(complete_document);
            content.push('\n');
        } else {
            append_compacted_record(&mut content, complete_document, max_line_fragment_bytes);
        }
        return content;
    }
    if looks_like_multiline_json_document(text) {
        return String::new();
    }

    let lines = text.lines().collect::<Vec<_>>();
    let mut content = String::new();
    for (index, line) in lines.iter().enumerate() {
        let trimmed = line.trim();
        if trimmed.is_empty() || should_skip_local_session_sidecar_line(trimmed) {
            continue;
        }
        let string_is_container_fragment = serde_json::from_str::<Value>(trimmed)
            .ok()
            .is_some_and(|value| value.is_string())
            && lines[index + 1..]
                .iter()
                .map(|line| line.trim())
                .find(|line| !line.is_empty())
                .is_some_and(|next| next.starts_with(']') || next.starts_with('}'));
        if string_is_container_fragment {
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
        if looks_like_json_member_or_array_fragment(trimmed) {
            continue;
        }
        if trimmed.len().saturating_add(1) > max_line_fragment_bytes
            && malformed_json_shaped_record_is_denied(trimmed)
        {
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
        LocalSessionRecordClassification::Deny
            | LocalSessionRecordClassification::Tool
            | LocalSessionRecordClassification::Unproven
    )
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum LocalSessionRecordClassification {
    User,
    Assistant,
    Thinking,
    Tool,
    KnownStructure,
    Deny,
    Missing,
    Unproven,
}

#[derive(Debug, Clone, Copy)]
struct LocalSessionRoleEvidence<'a> {
    classification: LocalSessionRecordClassification,
    role: Option<&'a str>,
}

fn local_session_object_is_denied(fields: &serde_json::Map<String, Value>) -> bool {
    local_session_classification_is_rejected(local_session_record_classification(fields, None))
}

fn local_session_value_is_denied(value: &Value) -> bool {
    value
        .as_object()
        .is_some_and(local_session_object_is_denied)
}

fn local_session_record_classification(
    fields: &serde_json::Map<String, Value>,
    inherited_role: Option<&str>,
) -> LocalSessionRecordClassification {
    let type_classification = local_session_type_classification(fields.get("type"));
    let phase_classification = local_session_phase_classification(fields.get("phase"));
    let role_evidence = local_session_role_evidence(fields);
    let role_classification =
        if role_evidence.classification == LocalSessionRecordClassification::Missing {
            inherited_role.map_or(LocalSessionRecordClassification::Missing, |role| {
                local_session_role_classification(role)
            })
        } else {
            role_evidence.classification
        };

    for classification in [
        type_classification,
        role_classification,
        phase_classification,
    ] {
        if local_session_classification_is_rejected(classification) {
            return classification;
        }
    }
    if matches!(type_classification, LocalSessionRecordClassification::Tool)
        || matches!(role_classification, LocalSessionRecordClassification::Tool)
    {
        return LocalSessionRecordClassification::Tool;
    }
    if phase_classification != LocalSessionRecordClassification::Missing {
        if type_classification == LocalSessionRecordClassification::User
            || role_classification == LocalSessionRecordClassification::User
            || (type_classification == LocalSessionRecordClassification::Thinking
                && phase_classification != LocalSessionRecordClassification::Thinking)
        {
            return LocalSessionRecordClassification::Unproven;
        }
        return phase_classification;
    }
    if matches!(
        type_classification,
        LocalSessionRecordClassification::User
            | LocalSessionRecordClassification::Assistant
            | LocalSessionRecordClassification::Thinking
    ) {
        return type_classification;
    }
    if role_classification != LocalSessionRecordClassification::Missing {
        return role_classification;
    }
    if type_classification == LocalSessionRecordClassification::KnownStructure {
        return LocalSessionRecordClassification::KnownStructure;
    }
    LocalSessionRecordClassification::Missing
}

fn local_session_role_evidence(
    map: &serde_json::Map<String, Value>,
) -> LocalSessionRoleEvidence<'_> {
    let nested_role = |outer: &str| {
        map.get(outer)
            .and_then(Value::as_object)
            .and_then(|nested| nested.get("role"))
    };
    let payload_item_role = map
        .get("payload")
        .and_then(Value::as_object)
        .and_then(|payload| payload.get("item"))
        .and_then(Value::as_object)
        .and_then(|item| item.get("role"));
    let role_values = [
        map.get("role"),
        map.get("sender"),
        nested_role("message"),
        nested_role("payload"),
        payload_item_role,
        nested_role("author"),
    ];
    let mut evidence = LocalSessionRoleEvidence {
        classification: LocalSessionRecordClassification::Missing,
        role: None,
    };

    for value in role_values.into_iter().flatten() {
        let Some(role) = value.as_str() else {
            return LocalSessionRoleEvidence {
                classification: LocalSessionRecordClassification::Unproven,
                role: None,
            };
        };
        let classification = local_session_role_classification(role);
        if local_session_classification_is_rejected(classification) {
            return LocalSessionRoleEvidence {
                classification,
                role: None,
            };
        }
        if evidence.classification == LocalSessionRecordClassification::Missing {
            evidence = LocalSessionRoleEvidence {
                classification,
                role: Some(role),
            };
        } else if evidence.classification != classification {
            return LocalSessionRoleEvidence {
                classification: LocalSessionRecordClassification::Unproven,
                role: None,
            };
        }
    }
    evidence
}

fn local_session_type_classification(value: Option<&Value>) -> LocalSessionRecordClassification {
    let Some(value) = value else {
        return LocalSessionRecordClassification::Missing;
    };
    let Some(record_type) = value.as_str() else {
        return LocalSessionRecordClassification::Unproven;
    };
    local_session_type_name_classification(record_type)
}

fn local_session_classification_is_rejected(
    classification: LocalSessionRecordClassification,
) -> bool {
    matches!(
        classification,
        LocalSessionRecordClassification::Deny | LocalSessionRecordClassification::Unproven
    )
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
    Phase,
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
    let mut final_phase_overflow = None;
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
                "phase" => RawClassificationKey::Phase,
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
                RawClassificationKey::Phase => final_phase_overflow = overflow,
                RawClassificationKey::Other => {}
            }
            *cursor = value_end;
        } else {
            match classification_key {
                RawClassificationKey::Type => final_type_overflow = None,
                RawClassificationKey::Role => final_role_overflow = None,
                RawClassificationKey::Phase => final_phase_overflow = None,
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
                ranges.extend(final_phase_overflow);
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
            | "phase"
            | "finish"
            | "finish_reason"
            | "finishReason"
            | "stop_reason"
            | "stopReason"
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

fn looks_like_multiline_json_document(text: &str) -> bool {
    let mut lines = text.lines().map(str::trim).filter(|line| !line.is_empty());
    let Some(first) = lines.next() else {
        return false;
    };
    if lines.next().is_none() {
        return false;
    }
    matches!(first, "{" | "[")
        || ((first.starts_with('{') || first.starts_with('['))
            && !is_complete_json_record(first)
            && looks_like_incomplete_json_record(first))
}

fn looks_like_json_member_or_array_fragment(text: &str) -> bool {
    let text = text.trim();
    if matches!(text, "{" | "}" | "[" | "]" | "}," | "],") {
        return true;
    }
    let bytes = text.as_bytes();
    if bytes.first() != Some(&b'"') {
        return false;
    }
    let Some(end) = json_string_end(bytes, 0) else {
        return true;
    };
    let remainder = text[end + 1..].trim_start();
    remainder.starts_with(':') || remainder.starts_with(',')
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

const MAX_MALFORMED_JSON_CLASSIFICATION_SCAN_BYTES: usize = 64 * 1024;

#[derive(Debug, Clone, Eq, PartialEq)]
enum MalformedJsonLikeToken {
    Open,
    Close,
    Colon,
    Comma,
    Scalar(Option<String>),
    Other,
}

struct MalformedJsonLikeTokenScanner<'a> {
    bytes: &'a [u8],
    cursor: usize,
    scan_truncated: bool,
}

impl<'a> MalformedJsonLikeTokenScanner<'a> {
    fn new(text: &'a str) -> Self {
        let mut end = text.len().min(MAX_MALFORMED_JSON_CLASSIFICATION_SCAN_BYTES);
        while end > 0 && !text.is_char_boundary(end) {
            end -= 1;
        }
        Self {
            bytes: &text.as_bytes()[..end],
            cursor: 0,
            scan_truncated: end < text.len(),
        }
    }

    fn exhausted_truncated_scan(&self) -> bool {
        self.scan_truncated && self.cursor >= self.bytes.len()
    }

    fn next_token(&mut self) -> Option<MalformedJsonLikeToken> {
        loop {
            while self
                .bytes
                .get(self.cursor)
                .is_some_and(u8::is_ascii_whitespace)
            {
                self.cursor += 1;
            }
            match (
                self.bytes.get(self.cursor),
                self.bytes.get(self.cursor.saturating_add(1)),
            ) {
                (Some(b'/'), Some(b'*')) => {
                    self.cursor += 2;
                    while self.cursor < self.bytes.len()
                        && !matches!(
                            (
                                self.bytes.get(self.cursor),
                                self.bytes.get(self.cursor.saturating_add(1))
                            ),
                            (Some(b'*'), Some(b'/'))
                        )
                    {
                        self.cursor += 1;
                    }
                    self.cursor = self.cursor.saturating_add(2).min(self.bytes.len());
                    continue;
                }
                (Some(b'/'), Some(b'/')) => {
                    self.cursor = self.bytes.len();
                    return None;
                }
                _ => {}
            }
            break;
        }

        let byte = *self.bytes.get(self.cursor)?;
        self.cursor += 1;
        match byte {
            b'{' | b'[' => Some(MalformedJsonLikeToken::Open),
            b'}' | b']' => Some(MalformedJsonLikeToken::Close),
            b':' => Some(MalformedJsonLikeToken::Colon),
            b',' => Some(MalformedJsonLikeToken::Comma),
            b'"' | b'\'' | b'`' => Some(MalformedJsonLikeToken::Scalar(
                self.scan_quoted_scalar(byte),
            )),
            _ if is_malformed_json_like_bare_scalar_byte(byte) => {
                let start = self.cursor - 1;
                while self
                    .bytes
                    .get(self.cursor)
                    .is_some_and(|byte| is_malformed_json_like_bare_scalar_byte(*byte))
                {
                    self.cursor += 1;
                }
                let token = &self.bytes[start..self.cursor];
                let value = (token.len() <= MAX_PROVENANCE_TOKEN_BYTES)
                    .then(|| String::from_utf8_lossy(token).into_owned());
                Some(MalformedJsonLikeToken::Scalar(value))
            }
            _ => Some(MalformedJsonLikeToken::Other),
        }
    }

    fn scan_quoted_scalar(&mut self, quote: u8) -> Option<String> {
        let token_start = self.cursor - 1;
        let content_start = self.cursor;
        let mut escaped = false;
        while let Some(byte) = self.bytes.get(self.cursor).copied() {
            self.cursor += 1;
            if escaped {
                escaped = false;
                continue;
            }
            if byte == b'\\' {
                escaped = true;
                continue;
            }
            if byte != quote {
                continue;
            }
            let token_end = self.cursor;
            if token_end.saturating_sub(token_start) > MAX_PROVENANCE_TOKEN_BYTES {
                return None;
            }
            if quote == b'"' {
                return serde_json::from_slice::<String>(&self.bytes[token_start..token_end]).ok();
            }
            return decode_relaxed_quoted_scalar(&self.bytes[content_start..token_end - 1]);
        }
        None
    }
}

fn is_malformed_json_like_bare_scalar_byte(byte: u8) -> bool {
    !byte.is_ascii_whitespace()
        && !matches!(
            byte,
            b'{' | b'}' | b'[' | b']' | b':' | b',' | b'"' | b'\'' | b'`' | b'/'
        )
}

fn decode_relaxed_quoted_scalar(bytes: &[u8]) -> Option<String> {
    if bytes.contains(&b'\\') {
        return None;
    }
    let mut decoded = Vec::with_capacity(bytes.len().min(MAX_PROVENANCE_TOKEN_BYTES));
    let mut cursor = 0usize;
    while cursor < bytes.len() {
        if decoded.len() >= MAX_PROVENANCE_TOKEN_BYTES {
            return None;
        }
        decoded.push(bytes[cursor]);
        cursor += 1;
    }
    String::from_utf8(decoded).ok()
}

fn malformed_json_shaped_record_is_denied(text: &str) -> bool {
    let mut scanner =
        MalformedJsonLikeTokenScanner::new(text.trim_start_matches('\u{feff}').trim_start());
    if scanner.next_token() != Some(MalformedJsonLikeToken::Open) {
        return false;
    }

    let mut depth = 1usize;
    let mut last_scalar: Option<Option<String>> = None;
    let mut classification_key: Option<String> = None;
    let mut saw_colon = false;
    while let Some(token) = scanner.next_token() {
        match token {
            MalformedJsonLikeToken::Open => {
                if classification_key.take().is_some() {
                    return true;
                }
                depth = depth.saturating_add(1);
                last_scalar = None;
            }
            MalformedJsonLikeToken::Close => {
                if classification_key.take().is_some() {
                    return true;
                }
                depth = depth.saturating_sub(1);
                last_scalar = None;
                if depth == 0 {
                    break;
                }
            }
            MalformedJsonLikeToken::Comma => {
                if classification_key.take().is_some() {
                    return true;
                }
                last_scalar = None;
            }
            MalformedJsonLikeToken::Colon => {
                saw_colon = true;
                let Some(key) = last_scalar.take() else {
                    classification_key = None;
                    continue;
                };
                let Some(key) = key else {
                    return true;
                };
                let normalized_key = normalized_malformed_json_token(&key);
                if malformed_json_deny_label(&normalized_key) {
                    return true;
                }
                classification_key = matches!(normalized_key.as_str(), "type" | "role" | "phase")
                    .then_some(normalized_key);
            }
            MalformedJsonLikeToken::Scalar(value) => {
                if let Some(key) = classification_key.take() {
                    let Some(value) = value.as_deref() else {
                        return true;
                    };
                    if malformed_json_classification_value_is_denied(&key, value) {
                        return true;
                    }
                }
                last_scalar = Some(value);
            }
            MalformedJsonLikeToken::Other => {
                if classification_key.take().is_some() {
                    return true;
                }
                last_scalar = None;
            }
        }
    }
    classification_key.is_some() || (depth > 0 && saw_colon && scanner.exhausted_truncated_scan())
}

fn normalized_malformed_json_token(value: &str) -> String {
    value.trim().to_ascii_lowercase().replace(['_', '-'], "")
}

fn malformed_json_deny_label(normalized: &str) -> bool {
    matches!(
        normalized,
        "system" | "developer" | "summary" | "compaction" | "context" | "metadata"
    )
}

fn malformed_json_classification_value_is_denied(key: &str, value: &str) -> bool {
    let classification = match key {
        "type" => local_session_type_name_classification(value),
        "role" => local_session_role_classification(value),
        "phase" => local_session_phase_name_classification(value),
        _ => return false,
    };
    local_session_classification_is_rejected(classification)
}

fn plain_local_session_record_is_denied(text: &str) -> bool {
    if malformed_json_shaped_record_is_denied(text) {
        return true;
    }
    let normalized = text.trim().to_ascii_lowercase();
    let label_candidate = normalized.trim_start_matches(['{', '[']).trim_start();
    if let Some(delimiter) = label_candidate
        .char_indices()
        .find_map(|(index, ch)| matches!(ch, ':' | '：').then_some(index))
    {
        let label = label_candidate[..delimiter].trim();
        if matches!(
            label,
            "system"
                | "developer"
                | "summary"
                | "compaction"
                | "context"
                | "metadata"
                | "系统"
                | "开发者"
                | "摘要"
        ) {
            return true;
        }
    }
    [
        "[system]",
        "[developer]",
        "[summary]",
        "<system>",
        "<developer>",
        "<summary>",
    ]
    .iter()
    .any(|prefix| normalized.starts_with(prefix))
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
            modified_at_millis: None,
            truncated: true,
            request_budget_exhausted: false,
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
            modified_at_millis: None,
            truncated: true,
            request_budget_exhausted: false,
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

    #[cfg(unix)]
    #[test]
    fn request_budget_fault_preserves_primary_and_sidecar_truncation_gap() {
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let fixture = PathBuf::from("/tmp").join(format!(
            "sc-request-budget-fault-{}-{unique}",
            std::process::id()
        ));
        let user_home = fixture.join("home");
        let storage_root = user_home.join(".local/share/opencode/storage");
        let session_root = storage_root.join("session");
        let message_root = storage_root.join("message/ses_request_fault");
        fs::create_dir_all(&session_root).expect("create opencode session root");
        fs::create_dir_all(&message_root).expect("create opencode message root");
        fs::write(
            session_root.join("ses_request_fault.json"),
            r#"{"id":"ses_request_fault","title":"Primary title"}"#,
        )
        .expect("write primary session");
        fs::write(
            message_root.join("000-fault.json"),
            r#"{"id":"msg_fault","role":"assistant","content":"faulting sidecar"}"#,
        )
        .expect("write faulting sidecar");
        fs::write(
            message_root.join("001-later.json"),
            r#"{"id":"msg_later","role":"assistant","content":"skill:must-not-read"}"#,
        )
        .expect("write later sidecar");
        let fault_path = message_root
            .join("000-fault.json")
            .canonicalize()
            .expect("canonicalize faulting sidecar");
        super::super::service_local_session_io::install_scheduled_sidecar_read_fault(fault_path, 3);
        let host = ServiceHost {
            app_data_dir: fixture.join("app-data"),
            adapter_ctx: AdapterContext {
                user_home,
                project_root: None,
                project_cwd: None,
                extra_roots: Vec::new(),
            },
        };
        let result = host
            .preview_local_sessions_with_test_limits(
                LocalSessionPreviewParams {
                    agent: Some("opencode".to_string()),
                    authorized_roots: vec![storage_root.to_string_lossy().to_string()],
                    auto_discover: Some(false),
                    limit: Some(10),
                    ..LocalSessionPreviewParams::default()
                },
                LocalSessionReadLimits {
                    max_preview_read_bytes: 128,
                    ..LocalSessionReadLimits::default()
                },
            )
            .expect("preview bounded opencode session");
        let _ = fs::remove_dir_all(&fixture);

        assert_eq!(result.count, 1, "primary row must remain");
        assert_eq!(result.session_rows[0].title, "Primary title");
        assert_eq!(result.skill_call_count, 0, "later sidecar must be omitted");
        assert!(result.gap_notes.iter().any(|note| {
            let note = note.to_ascii_lowercase();
            note.contains("sidecar") && note.contains("truncat")
        }));
    }

    #[cfg(unix)]
    #[test]
    fn row_id_does_not_reresolve_a_checked_candidate_after_path_swap() {
        use std::os::unix::fs::symlink;
        use std::sync::atomic::{AtomicU64, Ordering};

        static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);
        let unique = NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed);
        let fixture =
            PathBuf::from("/tmp").join(format!("sc-row-id-swap-{}-{unique}", std::process::id()));
        let root = fixture.join("authorized");
        let candidate = root.join("record.jsonl");
        let parked = root.join("record.parked");
        let outside = fixture.join("outside.jsonl");
        fs::create_dir_all(&root).expect("create row-id fixture root");
        fs::write(&candidate, "user: SAFE_CHECKED_PATH\n").expect("write checked candidate");
        fs::write(&outside, "user: OUTSIDE_PATH\n").expect("write outside target");
        let checked_candidate = candidate
            .canonicalize()
            .expect("canonical checked candidate");
        let before_swap = local_session_row_id(&checked_candidate);
        fs::rename(&candidate, parked).expect("park checked candidate");
        symlink(&outside, &candidate).expect("replace candidate with outside symlink");

        let after_swap = local_session_row_id(&checked_candidate);
        let _ = fs::remove_dir_all(&fixture);

        assert_eq!(before_swap, after_swap);
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
        let parsed = serde_json::from_str::<Value>(trimmed).ok();
        let is_marker = parsed
            .as_ref()
            .and_then(local_session_top_level_record_type)
            == Some(LOCAL_SESSION_TRUNCATION_MARKER_TYPE);
        let is_internal_injection = parsed
            .as_ref()
            .is_some_and(local_session_json_record_is_internal_injection);
        if trimmed.is_empty() || is_marker || is_internal_injection {
            continue;
        }
        visible.push_str(line);
        visible.push('\n');
    }
    visible
}

fn local_session_json_record_is_internal_injection(value: &Value) -> bool {
    match value {
        Value::Array(items) => {
            !items.is_empty()
                && items
                    .iter()
                    .all(local_session_json_record_is_internal_injection)
        }
        Value::Object(map) => {
            if is_codex_session_wrapper(map) {
                return map
                    .get("payload")
                    .is_some_and(local_session_json_record_is_internal_injection);
            }
            json_session_content_kind(map, None)
                .and_then(|kind| {
                    json_session_text_for_kind(map, kind)
                        .map(|text| is_internal_local_session_message(kind, &text))
                })
                .unwrap_or(false)
        }
        _ => false,
    }
}

fn accepted_local_session_content(content: &str) -> String {
    let complete_document = content
        .trim()
        .strip_prefix('\u{feff}')
        .unwrap_or_else(|| content.trim())
        .trim();
    if let Ok((mut value, _)) =
        parse_local_session_json_with_raw_classification_bounds(complete_document)
    {
        let (retained, _) = retain_accepted_local_session_value(&mut value);
        if !retained {
            return String::new();
        }
        let mut accepted = value.to_string();
        accepted.push('\n');
        return accepted;
    }

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
                if !plain_local_session_record_is_denied(trimmed)
                    && !looks_like_incomplete_json_record(trimmed)
                    && !looks_like_json_member_or_array_fragment(trimmed)
                {
                    accepted.push_str(line);
                    accepted.push('\n');
                }
            }
            Err(LocalSessionJsonParseError::UnsafeClassification) => {}
        }
    }
    accepted
}

fn retain_accepted_local_session_value(value: &mut Value) -> (bool, bool) {
    retain_accepted_local_session_value_at_boundary(value, true, true, false)
}

fn retain_accepted_local_session_value_at_boundary(
    value: &mut Value,
    plain_string_is_record: bool,
    scalar_content_allowed: bool,
    inherited_semantic: bool,
) -> (bool, bool) {
    match value {
        Value::Array(items) => {
            let mut changed = false;
            items.retain_mut(|nested| {
                let (retained, nested_changed) = retain_accepted_local_session_value_at_boundary(
                    nested,
                    plain_string_is_record,
                    scalar_content_allowed,
                    inherited_semantic,
                );
                changed |= nested_changed || !retained;
                retained
            });
            let retained = !items.is_empty() || inherited_semantic;
            (retained, changed || !retained)
        }
        Value::Object(map) => {
            let classification = local_session_record_classification(map, None);
            if local_session_classification_is_rejected(classification) {
                return (false, true);
            }
            let own_semantic = matches!(
                classification,
                LocalSessionRecordClassification::User
                    | LocalSessionRecordClassification::Assistant
                    | LocalSessionRecordClassification::Thinking
                    | LocalSessionRecordClassification::Tool
            );
            let semantic_body = own_semantic
                || (inherited_semantic
                    && classification == LocalSessionRecordClassification::KnownStructure);
            let wrapper_only = !semantic_body
                && matches!(
                    classification,
                    LocalSessionRecordClassification::KnownStructure
                        | LocalSessionRecordClassification::Missing
                );
            let mut changed = false;
            map.retain(|key, nested| {
                let nested_scalar_allowed = semantic_body || !is_json_session_scalar_body_key(key);
                let nested_inherited_semantic = semantic_body
                    && (is_json_session_scalar_body_key(key)
                        || (classification == LocalSessionRecordClassification::Tool
                            && is_json_tool_payload_key(key)));
                let (mut retained, nested_changed) =
                    retain_accepted_local_session_value_at_boundary(
                        nested,
                        false,
                        nested_scalar_allowed,
                        nested_inherited_semantic,
                    );
                if retained
                    && wrapper_only
                    && is_json_session_scalar_body_key(key)
                    && !local_session_value_contains_proven_event(nested)
                {
                    retained = false;
                }
                changed |= nested_changed || !retained;
                retained
            });
            let retained = !map.is_empty() || inherited_semantic;
            (retained, changed || !retained)
        }
        Value::String(_) if !scalar_content_allowed => (false, true),
        Value::String(text)
            if plain_string_is_record && plain_local_session_record_is_denied(text) =>
        {
            (false, true)
        }
        _ => (true, false),
    }
}

fn local_session_value_contains_proven_event(value: &Value) -> bool {
    match value {
        Value::Array(items) => items.iter().any(local_session_value_contains_proven_event),
        Value::Object(map) => {
            matches!(
                local_session_record_classification(map, None),
                LocalSessionRecordClassification::User
                    | LocalSessionRecordClassification::Assistant
                    | LocalSessionRecordClassification::Thinking
                    | LocalSessionRecordClassification::Tool
            ) || map.values().any(local_session_value_contains_proven_event)
        }
        _ => false,
    }
}

fn is_json_tool_payload_key(key: &str) -> bool {
    matches!(
        key,
        "arguments" | "input" | "result" | "output" | "error" | "payload" | "data" | "function"
    )
}

fn is_json_session_scalar_body_key(key: &str) -> bool {
    matches!(
        key,
        "content"
            | "parts"
            | "text"
            | "message"
            | "delta"
            | "thinking"
            | "reasoning"
            | "thought"
            | "summary"
            | "result"
            | "input"
            | "output"
    )
}

pub(crate) fn local_session_row_id(path: &Path) -> String {
    let path_key = path.to_string_lossy().replace('\\', "/");
    let path_key = path_key
        .strip_prefix("//?/")
        .unwrap_or(&path_key)
        .trim_end_matches('/');
    let path_hash = trace_content_hash(path_key)
        .chars()
        .take(16)
        .collect::<String>();
    format!("local-session-{path_hash}")
}

fn enrich_local_session_content(
    path: &Path,
    guarded_root: &GuardedLocalSessionRoot,
    file_content: &str,
    io: &mut LocalSessionIoContext,
) -> LocalSessionEnrichment {
    let unchanged = || LocalSessionEnrichment {
        content: file_content.to_string(),
        sidecars_truncated: false,
    };
    let Some(agent) = infer_local_session_agent(path) else {
        return unchanged();
    };
    if agent != AgentId::Opencode.as_str() && agent != "opencode" {
        return unchanged();
    }
    let Ok(value) = serde_json::from_str::<Value>(file_content) else {
        return unchanged();
    };
    let Some(session_id) = value.get("id").and_then(Value::as_str) else {
        return unchanged();
    };
    let Some(storage_root) = opencode_storage_root(path) else {
        return unchanged();
    };

    let mut chunks = vec![file_content.to_string()];
    let mut sidecar_state = OpencodeSidecarReadState {
        budget: SessionSidecarBudget::new(
            io.limits.max_sidecar_files,
            io.limits.max_sidecar_session_bytes,
        ),
        io,
        truncated: false,
    };
    let message_root = storage_root.join("message").join(session_id);
    {
        let Ok(message_inventory) = guarded_root.collect_regular_files_in_directory(
            &message_root,
            sidecar_state.budget.remaining_files(),
            &mut sidecar_state.io.inventory_budget,
        ) else {
            return LocalSessionEnrichment {
                content: chunks.join("\n"),
                sidecars_truncated: sidecar_state.truncated,
            };
        };
        sidecar_state.truncated |= message_inventory.truncated;
        for message_path in message_inventory.files {
            let Some(message) =
                read_opencode_sidecar(guarded_root, &message_path, &mut sidecar_state)
            else {
                if sidecar_state.budget.remaining_files() == 0
                    || sidecar_state.budget.remaining_bytes() == 0
                    || sidecar_state.io.budget.remaining_bytes() == 0
                {
                    sidecar_state.truncated = true;
                    break;
                }
                continue;
            };
            {
                let message = accepted_local_session_content(&compact_local_session_records(
                    &message,
                    sidecar_state.io.limits.max_line_fragment_bytes,
                ));
                if message.is_empty() {
                    continue;
                }
                let message_value = serde_json::from_str::<Value>(message.trim()).ok();
                let message_id = message_value
                    .as_ref()
                    .and_then(|value| value.get("id"))
                    .and_then(Value::as_str);
                let inherited_role = message_value
                    .as_ref()
                    .and_then(Value::as_object)
                    .and_then(json_session_role);
                let inherited_finish = message_value
                    .as_ref()
                    .and_then(Value::as_object)
                    .and_then(|message| message.get("finish"))
                    .and_then(Value::as_str);
                chunks.push(message);
                if let Some(message_id) = message_id {
                    append_opencode_parts(
                        &storage_root,
                        guarded_root,
                        message_id,
                        inherited_role,
                        inherited_finish,
                        &mut sidecar_state,
                        &mut chunks,
                    );
                }
            }
        }
    }
    LocalSessionEnrichment {
        content: chunks.join("\n"),
        sidecars_truncated: sidecar_state.truncated,
    }
}

struct LocalSessionEnrichment {
    content: String,
    sidecars_truncated: bool,
}

struct OpencodeSidecarReadState<'a> {
    io: &'a mut LocalSessionIoContext,
    budget: SessionSidecarBudget,
    truncated: bool,
}

fn append_opencode_parts(
    storage_root: &Path,
    guarded_root: &GuardedLocalSessionRoot,
    message_id: &str,
    inherited_role: Option<&str>,
    inherited_finish: Option<&str>,
    state: &mut OpencodeSidecarReadState<'_>,
    chunks: &mut Vec<String>,
) {
    let part_root = storage_root.join("part").join(message_id);
    let Ok(part_inventory) = guarded_root.collect_regular_files_in_directory(
        &part_root,
        state.budget.remaining_files(),
        &mut state.io.inventory_budget,
    ) else {
        return;
    };
    state.truncated |= part_inventory.truncated;
    for part_path in part_inventory.files {
        let Some(part) = read_opencode_sidecar(guarded_root, &part_path, state) else {
            if state.budget.remaining_files() == 0
                || state.budget.remaining_bytes() == 0
                || state.io.budget.remaining_bytes() == 0
            {
                state.truncated = true;
                break;
            }
            continue;
        };
        let compacted =
            compact_local_session_records(&part, state.io.limits.max_line_fragment_bytes);
        let attributed =
            opencode_part_with_inherited_message(&compacted, inherited_role, inherited_finish);
        let part = accepted_local_session_content(&attributed);
        if !part.is_empty() {
            chunks.push(part);
        }
    }
}

fn read_opencode_sidecar(
    guarded_root: &GuardedLocalSessionRoot,
    path: &Path,
    state: &mut OpencodeSidecarReadState<'_>,
) -> Option<String> {
    if !state.budget.claim_file() {
        state.truncated = true;
        return None;
    }
    let half_file_limit = state.io.limits.max_sidecar_file_bytes / 2;
    let bounded = read_bounded_sidecar_text(
        guarded_root,
        path,
        BoundedReadSpec {
            head_bytes: half_file_limit,
            tail_bytes: half_file_limit,
            line_fragment_bytes: state.io.limits.max_line_fragment_bytes,
        },
        &mut state.budget,
        &mut state.io.budget,
    )
    .ok()?;
    state.truncated |= bounded.truncated;
    Some(compact_bounded_local_session_content(
        &bounded,
        state.io.limits.max_line_fragment_bytes,
    ))
}

fn opencode_part_with_inherited_message(
    content: &str,
    inherited_role: Option<&str>,
    inherited_finish: Option<&str>,
) -> String {
    if inherited_role.is_none() && inherited_finish.is_none() {
        return content.to_string();
    }
    let Ok(mut value) = serde_json::from_str::<Value>(content.trim()) else {
        return content.to_string();
    };
    let Some(map) = value.as_object_mut() else {
        return content.to_string();
    };
    if !map
        .get("type")
        .and_then(Value::as_str)
        .map(|record_type| record_type.to_ascii_lowercase().replace(['_', '-'], ""))
        .is_some_and(|record_type| {
            matches!(record_type.as_str(), "text" | "inputtext" | "outputtext")
        })
    {
        return content.to_string();
    }
    if json_session_text(map)
        .as_deref()
        .is_some_and(plain_local_session_record_is_denied)
    {
        return content.to_string();
    }
    if local_session_role_evidence(map).classification == LocalSessionRecordClassification::Missing
    {
        if let Some(inherited_role) = inherited_role {
            map.insert(
                "role".to_string(),
                Value::String(inherited_role.to_string()),
            );
        }
    }
    if !map.contains_key("finish") {
        if let Some(inherited_finish) = inherited_finish {
            map.insert(
                "finish".to_string(),
                Value::String(inherited_finish.to_string()),
            );
        }
    }
    let mut attributed = value.to_string();
    attributed.push('\n');
    attributed
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
    content: &str,
    requested_agent: Option<&str>,
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

    for line in content.lines().filter(|line| !line.trim().is_empty()) {
        let Ok(value) = serde_json::from_str::<Value>(line) else {
            if metadata.title.is_none() && !parsed_json {
                metadata.title = local_session_text_title_candidate(line);
            }
            continue;
        };
        parsed_json = true;
        if local_session_metadata_is_internal(requested_agent, &value) {
            metadata.is_internal_thread = true;
        }
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

fn codex_session_index_title(
    io: &mut LocalSessionIoContext,
    codex_root: &Path,
    session_id: &str,
) -> Option<String> {
    let limits = io.limits;
    let (cache, request_budget) = (&mut io.cache, &mut io.budget);
    cache
        .codex_titles_or_load(codex_root.to_path_buf(), || {
            load_codex_session_index_titles(codex_root, limits, request_budget)
        })
        .get(session_id)
        .cloned()
}

fn load_codex_session_index_titles(
    codex_root: &Path,
    limits: LocalSessionReadLimits,
    request_budget: &mut LocalSessionReadBudget,
) -> HashMap<String, String> {
    let Some(anchor) = codex_root.parent() else {
        return HashMap::new();
    };
    let Ok(guarded_root) = GuardedLocalSessionRoot::open_beneath(anchor, codex_root) else {
        return HashMap::new();
    };
    let mut titles = HashMap::new();
    // Codex appends title changes to each index. Read the fallback history
    // first, then let later records and the dedicated session index win.
    for index_file_name in ["history.jsonl", "session_index.jsonl"] {
        let index_path = codex_root.join(index_file_name);
        let Ok(bounded) = read_bounded_text(
            &guarded_root,
            &index_path,
            BoundedReadSpec {
                head_bytes: 32 * 1024,
                tail_bytes: 32 * 1024,
                line_fragment_bytes: limits.max_line_fragment_bytes,
            },
            request_budget,
        ) else {
            continue;
        };
        for line in complete_bounded_index_lines(&bounded) {
            let Ok(value) = serde_json::from_str::<Value>(&line) else {
                continue;
            };
            let Some(map) = value.as_object() else {
                continue;
            };
            let row_id = map
                .get("id")
                .or_else(|| map.get("session_id"))
                .and_then(Value::as_str);
            let Some(row_id) = row_id else {
                continue;
            };
            for key in ["thread_name", "title", "text"] {
                if let Some(title) = map.get(key).and_then(Value::as_str) {
                    if let Some(title) = local_session_text_title_candidate(title) {
                        titles.insert(row_id.to_string(), title);
                        break;
                    }
                }
            }
        }
    }
    titles
}

fn complete_bounded_index_lines(bounded: &BoundedText) -> Vec<String> {
    if !bounded.truncated && bounded.retained_head_end == bounded.retained_tail_start {
        let mut contiguous = String::with_capacity(bounded.head.len() + bounded.tail.len());
        contiguous.push_str(&bounded.head);
        contiguous.push_str(&bounded.tail);
        return contiguous
            .lines()
            .filter(|line| !line.trim().is_empty())
            .map(str::to_string)
            .collect();
    }

    let head = bounded
        .head
        .rfind('\n')
        .map_or("", |newline| &bounded.head[..newline]);
    let tail = if bounded.tail_starts_at_line_boundary {
        bounded.tail.as_str()
    } else {
        bounded
            .tail
            .find('\n')
            .map_or("", |newline| &bounded.tail[newline + 1..])
    };
    head.lines()
        .chain(tail.lines())
        .filter(|line| !line.trim().is_empty())
        .map(str::to_string)
        .collect()
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

fn extract_local_session_skill_invocation_names(content: &str) -> Vec<String> {
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
    }

    let mut names = drafts
        .iter()
        .flat_map(|draft| extract_skill_invocation_names(&draft.text))
        .collect::<Vec<_>>();
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
    rows
}

fn local_session_content_drafts(
    content: &str,
    short_hash: &str,
    skill_mentions: &[LocalSessionSkillMention],
    skill_invocations: &[String],
) -> Vec<LocalSessionContentDraft> {
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

    drafts.into_iter().take(MAX_SESSION_CONTENT_ITEMS).collect()
}

fn local_session_content_items(
    drafts: Vec<LocalSessionContentDraft>,
    short_hash: &str,
    max_item_chars: usize,
    redactor: &mut PromptRedactor<'_>,
) -> Vec<LocalSessionContentItem> {
    drafts
        .into_iter()
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
    content_drafts: &[LocalSessionContentDraft],
    fallback_timestamp: Option<i64>,
) -> (Option<i64>, Option<i64>) {
    let mut bounds = LocalSessionTimeBounds::default();
    for draft in content_drafts {
        bounds.push(draft.timestamp);
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
            if is_codex_session_wrapper(map) {
                if let Some(payload) = map.get("payload") {
                    collect_nested_json_session_content_drafts(payload, timestamp, drafts);
                }
                return;
            }
            if local_session_object_is_denied(map) {
                return;
            }
            let classification = local_session_record_classification(map, None);
            let direct_kind = json_session_content_kind(map, None);
            let direct_tool = direct_kind == Some("tool_call");
            let mut pushed_direct_tool = false;
            if direct_kind != Some("thinking") {
                if let Some(thinking) = json_thinking_payload_text(value) {
                    if !thinking.trim().is_empty() {
                        push_local_session_text_drafts(
                            drafts,
                            "thinking",
                            json_fallback_session_title("thinking"),
                            thinking,
                            timestamp,
                            Vec::new(),
                        );
                    }
                }
            }
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

            if direct_kind.is_none()
                && matches!(
                    classification,
                    LocalSessionRecordClassification::KnownStructure
                        | LocalSessionRecordClassification::Missing
                )
            {
                for key in ["content", "text", "message", "delta", "parts"] {
                    if let Some(nested @ (Value::Array(_) | Value::Object(_))) = map.get(key) {
                        collect_nested_json_session_content_drafts(nested, timestamp, drafts);
                    }
                }
            }

            for (key, nested) in map {
                if matches!(key.as_str(), "metadata" | "meta") {
                    collect_json_metadata_tool_drafts(nested, timestamp, drafts);
                    continue;
                }
                if is_json_session_content_structure_key(key)
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
                collect_nested_json_session_content_drafts(nested, timestamp, drafts);
            }
        }
        Value::String(text) => {
            collect_text_session_content_drafts(text, inherited_timestamp, drafts)
        }
        _ => {}
    }
}

fn collect_nested_json_session_content_drafts(
    value: &Value,
    inherited_timestamp: Option<i64>,
    drafts: &mut Vec<LocalSessionContentDraft>,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_nested_json_session_content_drafts(item, inherited_timestamp, drafts);
            }
        }
        Value::Object(_) => collect_json_session_content_drafts(value, inherited_timestamp, drafts),
        _ => {}
    }
}

fn is_json_session_content_structure_key(key: &str) -> bool {
    is_json_session_structure_key(key)
        || matches!(
            key,
            "path" | "file_path" | "filePath" | "project_root" | "projectRoot"
        )
}

fn collect_json_metadata_tool_drafts(
    value: &Value,
    inherited_timestamp: Option<i64>,
    drafts: &mut Vec<LocalSessionContentDraft>,
) {
    match value {
        Value::Array(items) => {
            for item in items {
                collect_json_metadata_tool_drafts(item, inherited_timestamp, drafts);
            }
        }
        Value::Object(map) => {
            if local_session_object_is_denied(map) {
                return;
            }
            if local_session_record_classification(map, None)
                == LocalSessionRecordClassification::Tool
            {
                collect_json_session_content_drafts(value, inherited_timestamp, drafts);
                return;
            }
            for nested in map.values() {
                collect_json_metadata_tool_drafts(nested, inherited_timestamp, drafts);
            }
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
    } else if is_tool_result_text(line) {
        return;
    } else if lower.contains("tool_call")
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
        LocalSessionRecordClassification::Deny | LocalSessionRecordClassification::Unproven => {
            return None
        }
        LocalSessionRecordClassification::KnownStructure
        | LocalSessionRecordClassification::Missing => {}
    }
    if json_thinking_payload_text(&Value::Object(map.clone())).is_some() {
        return Some("thinking");
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

    for key in ["content", "text", "message", "delta", "summary", "result"] {
        if let Some(value) = map.get(key).and_then(json_non_tool_message_text) {
            if !value.trim().is_empty() {
                return Some(value);
            }
        }
    }
    None
}

fn local_session_content_kind_for_text<'a>(
    kind: &'a str,
    text: &str,
    map: &serde_json::Map<String, Value>,
) -> &'a str {
    if kind == "agent_reply"
        && (json_session_has_nonfinal_process_signal(map) || is_agent_process_note_text(text))
    {
        "thinking"
    } else {
        kind
    }
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
            if let Some(record_type) = map.get("type").and_then(Value::as_str) {
                let normalized = record_type.to_ascii_lowercase().replace(['_', '-'], "");
                if is_json_thinking_type(&normalized) || is_json_tool_type(&normalized) {
                    return None;
                }
                if matches!(normalized.as_str(), "text" | "inputtext" | "outputtext") {
                    for key in ["text", "content", "message", "delta"] {
                        if let Some(text) = map.get(key).and_then(json_non_tool_message_text) {
                            if !text.trim().is_empty() {
                                return Some(text);
                            }
                        }
                    }
                    return None;
                }
            }
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
        local_session_record_classification(map, None),
        LocalSessionRecordClassification::Deny
            | LocalSessionRecordClassification::Tool
            | LocalSessionRecordClassification::KnownStructure
            | LocalSessionRecordClassification::Missing
            | LocalSessionRecordClassification::Unproven
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

fn is_codex_session_wrapper(map: &serde_json::Map<String, Value>) -> bool {
    matches!(
        map.get("type").and_then(Value::as_str),
        Some("response_item" | "event_msg")
    ) && map.get("payload").is_some_and(Value::is_object)
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
            | "phase"
            | "finish"
            | "finish_reason"
            | "finishReason"
            | "stop_reason"
            | "stopReason"
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

include!("service_local_sessions/row_helpers.rs");
