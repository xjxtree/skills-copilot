use super::{
    auto_local_session_roots, collect_local_session_inventory, dedupe_local_session_root_requests,
    local_session_normalized_path, local_session_preview_row, local_session_project_filter_roots,
    local_session_row_id, normalize_string_list, sqlite_sessions, GuardedLocalSessionRoot,
    LocalSessionIoContext, LocalSessionPreviewRowOptions, LocalSessionReadLimits,
    LocalSessionRootRequest, LocalSessionScope, PromptRedactor, ServiceError, ServiceHost,
};
use serde::Deserialize;
use sha2::{Digest, Sha256};
use skills_copilot_commands::action_descriptor;
use skills_copilot_core::{
    ActionImpact, ActionIntent, ActionKind, ActionNetworkPosture, ActionReadbackDomain,
    ActionTargetKind, ActionTargetRef, AgentId, EvidenceKind, EvidenceRef, ResumeCapability,
    ResumeUnsupportedReason, Scope, SessionContinuationRecord, SourceCoverage,
};
use std::collections::BTreeSet;
use std::path::{Path, PathBuf};
use thiserror::Error;

const PREVIEW_METHOD: &str = "session.previewResume";
const MAX_NATIVE_LOCATOR_BYTES: usize = 512;

#[derive(Debug, Clone, Deserialize)]
pub(crate) struct SessionResumePreviewParams {
    #[serde(alias = "authorized_dirs", alias = "authorized_paths")]
    pub authorized_roots: Vec<String>,
    pub auto_discover: bool,
    pub agent: String,
    pub project_root: String,
    pub current_cwd: String,
    pub session_id: String,
    pub expected_source_revision: String,
    pub expected_snapshot_revision: String,
}

/// A locator whose adapter ownership has already been established while
/// collecting the accepted local-session snapshot.
///
/// The product never derives one of these values from a title, redacted path,
/// or stable service row id. OpenClaw is intentionally distinct because its
/// TUI resumes a routing key rather than a transcript/session id.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) enum SessionNativeResumeLocator {
    NativeId(String),
    OpenClawSessionKey(String),
}

/// Adapter evidence accepted into one immutable product snapshot.
///
/// Callers must populate this while the native session id (or OpenClaw session
/// key) is still available from the authorized adapter source. The locator is
/// deliberately absent from the returned evidence references; it appears only
/// as the final copy-only argv when resume is supported.
#[derive(Debug, Clone, Eq, PartialEq)]
pub(crate) struct SessionResumeEvidence {
    pub id: String,
    pub agent: AgentId,
    pub project_id: Option<String>,
    pub project_match: bool,
    pub title: String,
    pub intent: Option<String>,
    pub started_at: Option<i64>,
    pub ended_at: Option<i64>,
    pub modified_at: i64,
    pub source_kind: String,
    pub source_revision: String,
    pub snapshot_revision: String,
    pub coverage: SourceCoverage,
    pub native_locator: Option<SessionNativeResumeLocator>,
    pub adapter_unsupported_reason: Option<ResumeUnsupportedReason>,
    pub evidence_summary: String,
}

#[derive(Debug, Clone, Eq, PartialEq, Error)]
pub(crate) enum SessionResumePreviewError {
    #[error("invalid session resume request: {0}")]
    InvalidRequest(String),
    #[error("session_id is required")]
    MissingSessionId,
    #[error("expected revision must not be empty")]
    EmptyExpectedRevision,
    #[error("requested project context does not match the active product project")]
    InvalidProjectContext,
    #[error("selected session was not found in the accepted product snapshot")]
    SessionNotFound,
    #[error("selected session evidence changed")]
    SourceChanged,
    #[error("invalid accepted session evidence: {0}")]
    InvalidEvidence(String),
    #[error("local session source is unavailable")]
    SourceUnavailable,
}

impl SessionResumePreviewError {
    #[cfg(test)]
    pub(crate) fn code(&self) -> &'static str {
        match self {
            Self::InvalidRequest(_)
            | Self::MissingSessionId
            | Self::EmptyExpectedRevision
            | Self::InvalidProjectContext
            | Self::InvalidEvidence(_)
            | Self::SourceUnavailable => "invalid_request",
            Self::SessionNotFound => "session_not_found",
            Self::SourceChanged => "source_changed",
        }
    }
}

impl From<SessionResumePreviewError> for ServiceError {
    fn from(error: SessionResumePreviewError) -> Self {
        match error {
            SessionResumePreviewError::SessionNotFound => Self::SessionNotFound,
            SessionResumePreviewError::SourceChanged => Self::SourceChanged,
            other => Self::InvalidRequest(other.to_string()),
        }
    }
}

impl ServiceHost {
    /// Rebuild the bounded, adapter-owned session inventory and return one
    /// deterministic copy-only continuation preview.
    ///
    /// The accepted source revision must come from the canonical all-scope,
    /// no-search, recent-order summary inventory for this exact agent/root
    /// selection. Filtered/detail revisions are not interchangeable.
    ///
    /// `accepted_snapshot_revision` and `project_id` come from the same
    /// server-owned product snapshot used by project/skill reads. Snapshot
    /// mismatch is rejected before adapter context or session-source I/O.
    pub(crate) fn preview_session_resume(
        &self,
        params: SessionResumePreviewParams,
        accepted_snapshot_revision: &str,
        project_id: Option<String>,
    ) -> Result<SessionContinuationRecord, SessionResumePreviewError> {
        let session_id = required_value(
            &params.session_id,
            SessionResumePreviewError::MissingSessionId,
        )?;
        let expected_source_revision = required_value(
            &params.expected_source_revision,
            SessionResumePreviewError::EmptyExpectedRevision,
        )?;
        let expected_snapshot_revision = required_value(
            &params.expected_snapshot_revision,
            SessionResumePreviewError::EmptyExpectedRevision,
        )?;
        let accepted_snapshot_revision = required_value(
            accepted_snapshot_revision,
            SessionResumePreviewError::InvalidEvidence(
                "accepted product snapshot revision is empty".to_string(),
            ),
        )?;
        if expected_snapshot_revision != accepted_snapshot_revision {
            return Err(SessionResumePreviewError::SourceChanged);
        }
        let agent = parse_resume_agent(&params.agent)?;
        let project_root = required_value(
            &params.project_root,
            SessionResumePreviewError::InvalidRequest("project_root is required".to_string()),
        )?;
        let current_cwd = required_value(
            &params.current_cwd,
            SessionResumePreviewError::InvalidRequest("current_cwd is required".to_string()),
        )?;

        let adapter_ctx = self
            .effective_adapter_ctx()
            .map_err(map_session_source_error)?;
        validate_active_project_context(&adapter_ctx, project_root, current_cwd)?;
        let project_filter_roots = local_session_project_filter_roots(&adapter_ctx, None, None);
        let requested_roots = normalize_string_list(params.authorized_roots.clone());
        let auto_discover = params.auto_discover;
        let use_native_sqlite = requested_roots.is_empty()
            && auto_discover
            && matches!(
                agent,
                AgentId::Codex | AgentId::Opencode | AgentId::Hermes | AgentId::Openclaw
            );
        if use_native_sqlite {
            if let Some(snapshot) = sqlite_sessions::preview_sqlite_session_resume(
                &adapter_ctx,
                agent,
                session_id,
                expected_source_revision,
                &project_filter_roots,
                &self.trace_redaction_roots(&adapter_ctx),
            )? {
                return project_session_continuation(resume_evidence_from_sqlite(
                    snapshot,
                    agent,
                    project_id,
                    accepted_snapshot_revision,
                ));
            }
        }

        let snapshot = collect_file_resume_snapshot(
            &adapter_ctx,
            agent,
            requested_roots,
            auto_discover,
            &project_filter_roots,
            session_id,
            expected_source_revision,
            &self.trace_redaction_roots(&adapter_ctx),
        )?;
        let Some(selected) = snapshot.selected else {
            return Err(SessionResumePreviewError::SessionNotFound);
        };
        project_session_continuation(resume_evidence_from_file(
            selected,
            agent,
            project_id,
            accepted_snapshot_revision,
            &snapshot.source_revision,
            snapshot.coverage,
        ))
    }
}

fn validate_active_project_context(
    adapter_ctx: &skills_copilot_core::AdapterContext,
    requested_project_root: &str,
    requested_current_cwd: &str,
) -> Result<(), SessionResumePreviewError> {
    let Some(active_project_root) = adapter_ctx.project_root.as_deref() else {
        return Err(SessionResumePreviewError::InvalidProjectContext);
    };
    let requested_project_root = Path::new(requested_project_root);
    let requested_current_cwd = Path::new(requested_current_cwd);
    if !requested_project_root.is_absolute() || !requested_current_cwd.is_absolute() {
        return Err(SessionResumePreviewError::InvalidProjectContext);
    }

    let active_root = local_session_normalized_path(active_project_root);
    let requested_root = local_session_normalized_path(requested_project_root);
    if requested_root != active_root {
        return Err(SessionResumePreviewError::InvalidProjectContext);
    }

    let active_cwd = local_session_normalized_path(
        adapter_ctx
            .project_cwd
            .as_deref()
            .unwrap_or(active_project_root),
    );
    let requested_cwd = local_session_normalized_path(requested_current_cwd);
    if requested_cwd != active_cwd
        || !super::local_session_path_is_within(active_project_root, requested_current_cwd)
    {
        return Err(SessionResumePreviewError::InvalidProjectContext);
    }
    Ok(())
}

struct FileResumeRoot {
    path: PathBuf,
    guarded_root: GuardedLocalSessionRoot,
    source_kind: &'static str,
}

struct FileResumeCandidate {
    root_index: usize,
    path: PathBuf,
    modified_at: i64,
    row_id: String,
    path_digest: String,
}

struct FileResumeSelected {
    entry: super::LocalSessionPreviewEntry,
}

struct FileResumeSnapshot {
    source_revision: String,
    coverage: SourceCoverage,
    selected: Option<FileResumeSelected>,
}

#[allow(clippy::too_many_arguments)]
fn collect_file_resume_snapshot(
    adapter_ctx: &skills_copilot_core::AdapterContext,
    agent: AgentId,
    requested_roots: Vec<String>,
    auto_discover: bool,
    project_filter_roots: &[PathBuf],
    session_id: &str,
    expected_source_revision: &str,
    redaction_roots: &[(String, &'static str)],
) -> Result<FileResumeSnapshot, SessionResumePreviewError> {
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
    if auto_discover {
        let (mut discovered, discovery_notes) = auto_local_session_roots(
            adapter_ctx,
            Some(agent.as_str()),
            LocalSessionScope::All,
            project_filter_roots,
        );
        root_requests.append(&mut discovered);
        gap_notes.extend(discovery_notes);
    }
    dedupe_local_session_root_requests(&mut root_requests);

    let mut io = LocalSessionIoContext::new(LocalSessionReadLimits::default());
    let mut redactor = PromptRedactor::new(redaction_roots);
    let mut roots = Vec::new();
    let mut candidates = Vec::new();
    let mut truncated = false;
    for root_request in root_requests {
        let LocalSessionRootRequest {
            path,
            guarded_root,
            source_kind,
            ..
        } = root_request;
        if !path.is_absolute() {
            continue;
        }
        let Ok(guarded_root) = guarded_root else {
            continue;
        };
        let inventory = collect_local_session_inventory(
            &guarded_root,
            Some(agent.as_str()),
            &mut io.inventory_budget,
            &mut gap_notes,
            &mut redactor,
        );
        truncated |= inventory.truncated;
        let root_index = roots.len();
        roots.push(FileResumeRoot {
            path,
            guarded_root,
            source_kind,
        });
        candidates.extend(inventory.candidates.into_iter().map(|candidate| {
            let row_id = local_session_row_id(&candidate.path);
            let path_digest = session_path_digest(&candidate.path);
            FileResumeCandidate {
                root_index,
                path: candidate.path,
                modified_at: candidate.modified_at,
                row_id,
                path_digest,
            }
        }));
    }
    candidates.sort_by(|left, right| {
        right
            .modified_at
            .cmp(&left.modified_at)
            .then_with(|| left.row_id.cmp(&right.row_id))
            .then_with(|| left.path_digest.cmp(&right.path_digest))
    });
    let mut seen_ids = BTreeSet::new();
    candidates.retain(|candidate| seen_ids.insert(candidate.row_id.clone()));

    let source_revision = file_session_source_revision(&candidates);
    if source_revision != expected_source_revision {
        return Err(SessionResumePreviewError::SourceChanged);
    }
    let inspected_sources = roots.len();
    let expected_sources = Some(inspected_sources);
    let mut coverage = if truncated {
        SourceCoverage::incomplete(
            inspected_sources,
            expected_sources,
            skills_copilot_core::ListIncompleteReason::SafetyBudget,
        )
    } else {
        SourceCoverage::enumerable(inspected_sources, expected_sources)
    };
    let Some(candidate) = candidates
        .into_iter()
        .find(|candidate| candidate.row_id == session_id)
    else {
        return Ok(FileResumeSnapshot {
            source_revision,
            coverage,
            selected: None,
        });
    };
    let root = &roots[candidate.root_index];
    let codex_home = skills_copilot_adapters::codex_home_dir(adapter_ctx);
    let options = LocalSessionPreviewRowOptions {
        requested_agent: Some(agent.as_str()),
        max_excerpt_chars: 1_000,
        source_kind: root.source_kind,
        skill_matchers: &[],
        scope: LocalSessionScope::All,
        project_filter_roots,
        codex_home: &codex_home,
        search: None,
        include_content_items: false,
    };
    let outcome = local_session_preview_row(
        &candidate.path,
        &root.path,
        &root.guarded_root,
        options,
        &mut io,
        &mut gap_notes,
        &mut redactor,
    )
    .map_err(map_session_source_error)?;
    if outcome.budget_exhausted && coverage.is_complete() {
        coverage = SourceCoverage::incomplete(
            inspected_sources,
            expected_sources,
            skills_copilot_core::ListIncompleteReason::SafetyBudget,
        );
    }
    Ok(FileResumeSnapshot {
        source_revision,
        coverage,
        selected: outcome.entry.map(|entry| FileResumeSelected { entry }),
    })
}

fn resume_evidence_from_file(
    selected: FileResumeSelected,
    agent: AgentId,
    project_id: Option<String>,
    snapshot_revision: &str,
    source_revision: &str,
    coverage: SourceCoverage,
) -> SessionResumeEvidence {
    let row = selected.entry.row;
    let project_match = selected.entry.project_matches_selected_context;
    SessionResumeEvidence {
        id: row.id,
        agent,
        project_id: project_id.filter(|_| project_match),
        project_match,
        title: row.title,
        intent: None,
        started_at: row.started_at,
        ended_at: row.ended_at,
        modified_at: row.modified_at.unwrap_or_default(),
        source_kind: row.source_kind,
        source_revision: source_revision.to_string(),
        snapshot_revision: snapshot_revision.to_string(),
        coverage,
        native_locator: selected.entry.resume_locator,
        adapter_unsupported_reason: None,
        evidence_summary: format!(
            "Verified {} local session metadata from an authorized source.",
            agent.as_str()
        ),
    }
}

fn resume_evidence_from_sqlite(
    snapshot: sqlite_sessions::SqliteSessionResumeSnapshot,
    agent: AgentId,
    project_id: Option<String>,
    snapshot_revision: &str,
) -> SessionResumeEvidence {
    let row = snapshot.row;
    let project_match = snapshot.project_matches_selected_context;
    SessionResumeEvidence {
        id: row.id,
        agent,
        project_id: project_id.filter(|_| project_match),
        project_match,
        title: row.title,
        intent: None,
        started_at: row.started_at,
        ended_at: row.ended_at,
        modified_at: row.modified_at.unwrap_or_default(),
        source_kind: row.source_kind,
        source_revision: snapshot.source_revision,
        snapshot_revision: snapshot_revision.to_string(),
        coverage: snapshot.coverage,
        native_locator: Some(snapshot.resume_locator),
        adapter_unsupported_reason: None,
        evidence_summary: format!(
            "Verified {} local session metadata from its canonical index.",
            agent.as_str()
        ),
    }
}

fn parse_resume_agent(value: &str) -> Result<AgentId, SessionResumePreviewError> {
    let value = required_value(
        value,
        SessionResumePreviewError::InvalidRequest("agent is required".to_string()),
    )?;
    match value {
        "claude-code" => Ok(AgentId::ClaudeCode),
        "codex" => Ok(AgentId::Codex),
        "opencode" => Ok(AgentId::Opencode),
        "pi" => Ok(AgentId::Pi),
        "hermes" => Ok(AgentId::Hermes),
        "openclaw" => Ok(AgentId::Openclaw),
        _ => Err(SessionResumePreviewError::InvalidRequest(
            "agent does not support local session continuation".to_string(),
        )),
    }
}

fn map_session_source_error(error: ServiceError) -> SessionResumePreviewError {
    match error {
        ServiceError::SourceChanged => SessionResumePreviewError::SourceChanged,
        _ => SessionResumePreviewError::SourceUnavailable,
    }
}

fn session_path_digest(path: &Path) -> String {
    super::trace_content_hash(&local_session_normalized_path(path))
}

fn file_session_source_revision(candidates: &[FileResumeCandidate]) -> String {
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
        hasher.update(LocalSessionScope::All.as_str().as_bytes());
        hasher.update([0]);
    }
    format!("sha256:{}", super::hex_prefix(&hasher.finalize(), 64))
}

/// Select a continuation from one already accepted product snapshot.
///
/// This function performs no I/O. In particular, it validates the stable id
/// and both optional revisions against server-owned records before returning a
/// copyable command. A caller must not substitute client-supplied records for
/// `records`.
#[cfg(test)]
pub(crate) fn preview_session_resume_from_snapshot(
    records: &[SessionContinuationRecord],
    session_id: &str,
    expected_source_revision: Option<&str>,
    expected_snapshot_revision: Option<&str>,
) -> Result<SessionContinuationRecord, SessionResumePreviewError> {
    let session_id = required_value(session_id, SessionResumePreviewError::MissingSessionId)?;
    let expected_source_revision = optional_expected_revision(expected_source_revision)?;
    let expected_snapshot_revision = optional_expected_revision(expected_snapshot_revision)?;

    let record = records
        .iter()
        .find(|record| record.id == session_id)
        .ok_or(SessionResumePreviewError::SessionNotFound)?;
    if expected_source_revision.is_some_and(|revision| revision != record.source_revision)
        || expected_snapshot_revision.is_some_and(|revision| revision != record.snapshot_revision)
    {
        return Err(SessionResumePreviewError::SourceChanged);
    }
    record
        .validate()
        .map_err(|error| SessionResumePreviewError::InvalidEvidence(error.to_string()))?;
    Ok(record.clone())
}

/// Convert adapter-owned session evidence into the shared product record.
///
/// Command construction is an allowlist. Missing, incomplete, mismatched, or
/// undocumented evidence yields a typed unsupported capability and no argv.
pub(crate) fn project_session_continuation(
    evidence: SessionResumeEvidence,
) -> Result<SessionContinuationRecord, SessionResumePreviewError> {
    validate_evidence_identity(&evidence)?;
    evidence
        .coverage
        .validate()
        .map_err(|error| SessionResumePreviewError::InvalidEvidence(error.to_string()))?;

    let evidence_ref = EvidenceRef {
        id: stable_session_evidence_id(&evidence.id, &evidence.source_revision),
        kind: EvidenceKind::Session,
        source_revision: evidence.source_revision.clone(),
        summary: evidence.evidence_summary.clone(),
        agent: Some(evidence.agent),
        target_id: Some(evidence.id.clone()),
    };
    evidence_ref
        .validate()
        .map_err(|error| SessionResumePreviewError::InvalidEvidence(error.to_string()))?;

    let resume = projected_resume_capability(&evidence);
    let actions = if resume.argv.is_empty() {
        Vec::new()
    } else {
        vec![action_descriptor(
            ActionKind::ResumeSession,
            ActionIntent::ResumeSession,
            ActionTargetRef {
                kind: ActionTargetKind::Session,
                id: evidence.id.clone(),
                agent: Some(evidence.agent),
                scope: evidence.project_id.as_ref().map(|_| Scope::AgentProject),
            },
            evidence.project_id.clone(),
            vec![ActionImpact::ReadOnly],
            PREVIEW_METHOD,
            None,
            evidence.snapshot_revision.clone(),
            false,
            ActionNetworkPosture::None,
            vec![ActionReadbackDomain::SessionContinuation],
            vec![evidence_ref.id.clone()],
        )
        .map_err(|error| SessionResumePreviewError::InvalidEvidence(error.to_string()))?]
    };

    let record = SessionContinuationRecord {
        id: evidence.id,
        agent: evidence.agent,
        project_id: evidence.project_id,
        title: evidence.title,
        intent: evidence.intent,
        started_at: evidence.started_at,
        ended_at: evidence.ended_at,
        modified_at: evidence.modified_at,
        source_kind: evidence.source_kind,
        source_revision: evidence.source_revision,
        snapshot_revision: evidence.snapshot_revision,
        coverage: evidence.coverage,
        resume,
        evidence: vec![evidence_ref],
        actions,
    };
    record
        .validate()
        .map_err(|error| SessionResumePreviewError::InvalidEvidence(error.to_string()))?;
    Ok(record)
}

pub(crate) fn native_resume_locator_from_file_content(
    agent: &str,
    content: &str,
) -> Option<SessionNativeResumeLocator> {
    let agent = parse_resume_agent(agent).ok()?;
    let mut native_id = None;
    for value in session_json_records(content) {
        let Some(candidate) = adapter_native_id(agent, &value) else {
            continue;
        };
        if !set_consistent_native_id(&mut native_id, candidate) {
            return None;
        }
    }
    native_id.map(SessionNativeResumeLocator::NativeId)
}

fn session_json_records(content: &str) -> Vec<serde_json::Value> {
    if let Ok(value) = serde_json::from_str::<serde_json::Value>(content.trim()) {
        return match value {
            serde_json::Value::Array(values) => values,
            value => vec![value],
        };
    }
    content
        .lines()
        .filter_map(|line| serde_json::from_str::<serde_json::Value>(line.trim()).ok())
        .collect()
}

fn adapter_native_id(agent: AgentId, value: &serde_json::Value) -> Option<&str> {
    let map = value.as_object()?;
    match agent {
        AgentId::ClaudeCode => map
            .get("sessionId")
            .and_then(serde_json::Value::as_str)
            .filter(|value| !value.is_empty()),
        AgentId::Codex
            if map.get("type").and_then(serde_json::Value::as_str) == Some("session_meta") =>
        {
            map.get("payload")
                .and_then(serde_json::Value::as_object)
                .and_then(|payload| payload.get("id"))
                .and_then(serde_json::Value::as_str)
                .or_else(|| map.get("id").and_then(serde_json::Value::as_str))
                .filter(|value| !value.is_empty())
        }
        AgentId::Pi if map.get("type").and_then(serde_json::Value::as_str) == Some("session") => {
            map.get("id")
                .and_then(serde_json::Value::as_str)
                .filter(|value| !value.is_empty())
        }
        _ => None,
    }
}

fn set_consistent_native_id(slot: &mut Option<String>, candidate: &str) -> bool {
    match slot {
        Some(value) => value == candidate,
        None => {
            *slot = Some(candidate.to_string());
            true
        }
    }
}

fn projected_resume_capability(evidence: &SessionResumeEvidence) -> ResumeCapability {
    if !evidence.coverage.is_complete() {
        return ResumeCapability::unsupported(ResumeUnsupportedReason::SourceIncomplete);
    }
    if !evidence.project_match {
        return ResumeCapability::unsupported(ResumeUnsupportedReason::InvalidProjectContext);
    }
    if let Some(reason) = evidence.adapter_unsupported_reason {
        return ResumeCapability::unsupported(reason);
    }
    if evidence.agent == AgentId::ToolGlobal {
        return ResumeCapability::unsupported(ResumeUnsupportedReason::AgentUnsupported);
    }
    let Some(locator) = evidence.native_locator.as_ref() else {
        return ResumeCapability::unsupported(ResumeUnsupportedReason::MissingNativeId);
    };
    let Ok(locator_value) = validated_locator(locator) else {
        return ResumeCapability::unsupported(ResumeUnsupportedReason::MissingNativeId);
    };

    let argv = match (evidence.agent, locator) {
        (AgentId::ClaudeCode, SessionNativeResumeLocator::NativeId(_)) => {
            vec!["claude", "--resume", locator_value]
        }
        (AgentId::Codex, SessionNativeResumeLocator::NativeId(_)) => {
            vec!["codex", "resume", locator_value]
        }
        (AgentId::Opencode, SessionNativeResumeLocator::NativeId(_)) => {
            vec!["opencode", "--session", locator_value]
        }
        (AgentId::Pi, SessionNativeResumeLocator::NativeId(_)) => {
            vec!["pi", "--session", locator_value]
        }
        (AgentId::Hermes, SessionNativeResumeLocator::NativeId(_)) => {
            vec!["hermes", "--resume", locator_value]
        }
        (AgentId::Openclaw, SessionNativeResumeLocator::OpenClawSessionKey(_)) => {
            vec!["openclaw", "tui", "--session", locator_value]
        }
        (AgentId::ToolGlobal, _) => unreachable!("tool-global was rejected before locator mapping"),
        _ => {
            return ResumeCapability::unsupported(ResumeUnsupportedReason::SessionUnsupported);
        }
    };
    ResumeCapability::supported(argv.into_iter().map(str::to_string).collect())
}

fn validated_locator(
    locator: &SessionNativeResumeLocator,
) -> Result<&str, SessionResumePreviewError> {
    let value = match locator {
        SessionNativeResumeLocator::NativeId(value)
        | SessionNativeResumeLocator::OpenClawSessionKey(value) => value.as_str(),
    };
    if value.is_empty()
        || value != value.trim()
        || value.len() > MAX_NATIVE_LOCATOR_BYTES
        || value.starts_with('-')
        || value
            .chars()
            .any(|character| character.is_control() || character.is_whitespace())
    {
        return Err(SessionResumePreviewError::InvalidEvidence(
            "native resume locator is not a safe CLI argument".to_string(),
        ));
    }
    Ok(value)
}

fn validate_evidence_identity(
    evidence: &SessionResumeEvidence,
) -> Result<(), SessionResumePreviewError> {
    for (value, label) in [
        (evidence.id.as_str(), "session id"),
        (evidence.title.as_str(), "session title"),
        (evidence.source_kind.as_str(), "session source kind"),
        (evidence.source_revision.as_str(), "session source revision"),
        (
            evidence.snapshot_revision.as_str(),
            "session snapshot revision",
        ),
        (
            evidence.evidence_summary.as_str(),
            "session evidence summary",
        ),
    ] {
        if value.trim().is_empty() || value.chars().any(char::is_control) {
            return Err(SessionResumePreviewError::InvalidEvidence(format!(
                "{label} is empty or contains control characters"
            )));
        }
    }
    if evidence
        .project_id
        .as_deref()
        .is_some_and(|value| value.trim().is_empty() || value.chars().any(char::is_control))
    {
        return Err(SessionResumePreviewError::InvalidEvidence(
            "session project id is empty or contains control characters".to_string(),
        ));
    }
    Ok(())
}

fn required_value(
    value: &str,
    error: SessionResumePreviewError,
) -> Result<&str, SessionResumePreviewError> {
    if value.is_empty() || value != value.trim() || value.chars().any(char::is_control) {
        return Err(error);
    }
    Ok(value)
}

#[cfg(test)]
fn optional_expected_revision(
    value: Option<&str>,
) -> Result<Option<&str>, SessionResumePreviewError> {
    value
        .map(|value| required_value(value, SessionResumePreviewError::EmptyExpectedRevision))
        .transpose()
}

fn stable_session_evidence_id(session_id: &str, source_revision: &str) -> String {
    use sha2::{Digest, Sha256};

    let mut hasher = Sha256::new();
    hasher.update(b"agent-copilot.session-resume-evidence.v1\0");
    hasher.update(session_id.as_bytes());
    hasher.update([0]);
    hasher.update(source_revision.as_bytes());
    format!("evidence:session-resume:{:x}", hasher.finalize())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LocalSessionPreviewParams;
    use rusqlite::{params, Connection};
    use skills_copilot_core::{
        AdapterContext, ListIncompleteReason, ResumeCapabilityState, ResumeUnsupportedReason,
    };
    use std::fs;
    use std::sync::atomic::{AtomicU64, Ordering};

    const SOURCE_REVISION: &str = "sha256:session-source";
    const SNAPSHOT_REVISION: &str = "sha256:product-snapshot";
    static NEXT_FIXTURE: AtomicU64 = AtomicU64::new(0);

    #[test]
    fn wire_params_require_every_context_and_revision_field() {
        let complete = serde_json::json!({
            "authorized_roots": [],
            "auto_discover": true,
            "agent": "codex",
            "project_root": "/tmp/project",
            "current_cwd": "/tmp/project",
            "session_id": "session-1",
            "expected_source_revision": "sha256:source",
            "expected_snapshot_revision": "sha256:snapshot"
        });
        serde_json::from_value::<SessionResumePreviewParams>(complete.clone())
            .expect("complete request");

        for field in [
            "authorized_roots",
            "auto_discover",
            "agent",
            "project_root",
            "current_cwd",
            "session_id",
            "expected_source_revision",
            "expected_snapshot_revision",
        ] {
            let mut incomplete = complete.clone();
            incomplete
                .as_object_mut()
                .expect("request object")
                .remove(field);
            assert!(
                serde_json::from_value::<SessionResumePreviewParams>(incomplete).is_err(),
                "missing {field} must fail deserialization"
            );
        }

        assert_eq!(
            parse_resume_agent("Claude")
                .expect_err("aliases are not accepted")
                .code(),
            "invalid_request"
        );

        let filesystem_root = AdapterContext {
            user_home: PathBuf::from("/"),
            project_root: Some(PathBuf::from("/")),
            project_cwd: Some(PathBuf::from("/")),
            extra_roots: Vec::new(),
        };
        validate_active_project_context(&filesystem_root, "/", "/")
            .expect("filesystem root remains a valid project boundary");

        for error in [
            SessionResumePreviewError::InvalidProjectContext,
            SessionResumePreviewError::InvalidEvidence("bad evidence".to_string()),
            SessionResumePreviewError::SourceUnavailable,
        ] {
            assert_eq!(error.code(), "invalid_request");
        }
    }

    fn evidence(
        agent: AgentId,
        locator: Option<SessionNativeResumeLocator>,
    ) -> SessionResumeEvidence {
        SessionResumeEvidence {
            id: format!("session-{}", agent.as_str()),
            agent,
            project_id: Some("project:test".to_string()),
            project_match: true,
            title: format!("{} session", agent.as_str()),
            intent: Some("Continue verified work".to_string()),
            started_at: Some(10),
            ended_at: Some(20),
            modified_at: 20,
            source_kind: format!("{}-native", agent.as_str()),
            source_revision: SOURCE_REVISION.to_string(),
            snapshot_revision: SNAPSHOT_REVISION.to_string(),
            coverage: SourceCoverage::enumerable(1, Some(1)),
            native_locator: locator,
            adapter_unsupported_reason: None,
            evidence_summary: format!("Verified {} local session", agent.as_str()),
        }
    }

    #[test]
    fn projects_each_documented_native_resume_argv_without_reordering() {
        let cases = [
            (
                AgentId::ClaudeCode,
                SessionNativeResumeLocator::NativeId("claude-id".to_string()),
                vec!["claude", "--resume", "claude-id"],
            ),
            (
                AgentId::Codex,
                SessionNativeResumeLocator::NativeId(
                    "019f0000-0000-7000-8000-000000000001".to_string(),
                ),
                vec!["codex", "resume", "019f0000-0000-7000-8000-000000000001"],
            ),
            (
                AgentId::Opencode,
                SessionNativeResumeLocator::NativeId("ses_opencode".to_string()),
                vec!["opencode", "--session", "ses_opencode"],
            ),
            (
                AgentId::Pi,
                SessionNativeResumeLocator::NativeId(
                    "019f0000-0000-7000-8000-000000000002".to_string(),
                ),
                vec!["pi", "--session", "019f0000-0000-7000-8000-000000000002"],
            ),
            (
                AgentId::Hermes,
                SessionNativeResumeLocator::NativeId("20260724_091523_a1b2c3".to_string()),
                vec!["hermes", "--resume", "20260724_091523_a1b2c3"],
            ),
            (
                AgentId::Openclaw,
                SessionNativeResumeLocator::OpenClawSessionKey(
                    "agent:main:tui:project".to_string(),
                ),
                vec!["openclaw", "tui", "--session", "agent:main:tui:project"],
            ),
        ];

        for (agent, locator, expected) in cases {
            let record = project_session_continuation(evidence(agent, Some(locator)))
                .expect("valid continuation");
            assert_eq!(record.resume.state, ResumeCapabilityState::Supported);
            assert_eq!(record.resume.argv, expected);
            assert!(record.resume.copy_only);
            assert_eq!(record.actions.len(), 1);
            assert_eq!(record.actions[0].source_revision, SNAPSHOT_REVISION);
            assert_eq!(
                record.actions[0].evidence_refs,
                vec![record.evidence[0].id.clone()]
            );
            assert_eq!(record.evidence[0].source_revision, SOURCE_REVISION);
        }
    }

    #[test]
    fn openclaw_transcript_id_is_not_treated_as_a_session_key() {
        let record = project_session_continuation(evidence(
            AgentId::Openclaw,
            Some(SessionNativeResumeLocator::NativeId(
                "019f0000-0000-7000-8000-000000000003".to_string(),
            )),
        ))
        .expect("valid unsupported continuation");

        assert_eq!(record.resume.state, ResumeCapabilityState::Unsupported);
        assert_eq!(
            record.resume.unsupported_reason,
            Some(ResumeUnsupportedReason::SessionUnsupported)
        );
        assert!(record.resume.argv.is_empty());
        assert!(record.actions.is_empty());
    }

    #[test]
    fn incomplete_or_missing_evidence_never_exposes_argv() {
        let mut incomplete = evidence(
            AgentId::Codex,
            Some(SessionNativeResumeLocator::NativeId("codex-id".to_string())),
        );
        incomplete.coverage =
            SourceCoverage::incomplete(0, Some(1), ListIncompleteReason::SafetyBudget);
        let incomplete =
            project_session_continuation(incomplete).expect("valid incomplete continuation");
        assert_eq!(
            incomplete.resume.unsupported_reason,
            Some(ResumeUnsupportedReason::SourceIncomplete)
        );
        assert!(incomplete.resume.argv.is_empty());
        assert!(incomplete.actions.is_empty());

        let missing = project_session_continuation(evidence(AgentId::ClaudeCode, None))
            .expect("valid missing-id continuation");
        assert_eq!(
            missing.resume.unsupported_reason,
            Some(ResumeUnsupportedReason::MissingNativeId)
        );
        assert!(missing.resume.argv.is_empty());
    }

    #[test]
    fn project_and_adapter_limitations_remain_typed() {
        let mut mismatched = evidence(
            AgentId::Pi,
            Some(SessionNativeResumeLocator::NativeId("pi-id".to_string())),
        );
        mismatched.project_match = false;
        let mismatched =
            project_session_continuation(mismatched).expect("valid unsupported continuation");
        assert_eq!(
            mismatched.resume.unsupported_reason,
            Some(ResumeUnsupportedReason::InvalidProjectContext)
        );

        let mut unsupported = evidence(
            AgentId::Hermes,
            Some(SessionNativeResumeLocator::NativeId(
                "hermes-id".to_string(),
            )),
        );
        unsupported.adapter_unsupported_reason = Some(ResumeUnsupportedReason::SessionUnsupported);
        let unsupported =
            project_session_continuation(unsupported).expect("valid unsupported continuation");
        assert_eq!(
            unsupported.resume.unsupported_reason,
            Some(ResumeUnsupportedReason::SessionUnsupported)
        );
        assert!(unsupported.resume.argv.is_empty());
    }

    #[test]
    fn unsafe_or_wrong_locator_kinds_fail_closed() {
        for locator in [
            SessionNativeResumeLocator::NativeId("-last".to_string()),
            SessionNativeResumeLocator::NativeId("has space".to_string()),
            SessionNativeResumeLocator::NativeId("line\nbreak".to_string()),
        ] {
            let record = project_session_continuation(evidence(AgentId::Codex, Some(locator)))
                .expect("unsafe locator produces unsupported record");
            assert_eq!(
                record.resume.unsupported_reason,
                Some(ResumeUnsupportedReason::MissingNativeId)
            );
            assert!(record.resume.argv.is_empty());
        }

        let wrong_kind = project_session_continuation(evidence(
            AgentId::ClaudeCode,
            Some(SessionNativeResumeLocator::OpenClawSessionKey(
                "agent:main:main".to_string(),
            )),
        ))
        .expect("wrong locator kind produces unsupported record");
        assert_eq!(
            wrong_kind.resume.unsupported_reason,
            Some(ResumeUnsupportedReason::SessionUnsupported)
        );
    }

    #[test]
    fn cached_preview_rejects_unknown_and_stale_inputs_before_returning_command() {
        let record = project_session_continuation(evidence(
            AgentId::Codex,
            Some(SessionNativeResumeLocator::NativeId("codex-id".to_string())),
        ))
        .expect("valid continuation");
        let records = vec![record.clone()];

        let selected = preview_session_resume_from_snapshot(
            &records,
            &record.id,
            Some(SOURCE_REVISION),
            Some(SNAPSHOT_REVISION),
        )
        .expect("matching preview");
        assert_eq!(selected, record);

        assert_eq!(
            preview_session_resume_from_snapshot(&records, "unknown", None, None)
                .expect_err("unknown session"),
            SessionResumePreviewError::SessionNotFound
        );
        assert_eq!(
            preview_session_resume_from_snapshot(&records, &record.id, Some("sha256:stale"), None,)
                .expect_err("stale source"),
            SessionResumePreviewError::SourceChanged
        );
        assert_eq!(
            preview_session_resume_from_snapshot(&records, &record.id, None, Some("sha256:stale"),)
                .expect_err("stale snapshot"),
            SessionResumePreviewError::SourceChanged
        );
    }

    #[test]
    fn cached_preview_validates_request_shape_without_lookup() {
        assert_eq!(
            preview_session_resume_from_snapshot(&[], " \n", None, None).expect_err("missing id"),
            SessionResumePreviewError::MissingSessionId
        );
        assert_eq!(
            preview_session_resume_from_snapshot(&[], "session", Some("  "), None)
                .expect_err("empty expected revision"),
            SessionResumePreviewError::EmptyExpectedRevision
        );
        assert_eq!(
            SessionResumePreviewError::SessionNotFound.code(),
            "session_not_found"
        );
        assert_eq!(
            SessionResumePreviewError::SourceChanged.code(),
            "source_changed"
        );
    }

    #[test]
    fn bounded_file_resolver_matches_keyset_revision_and_projects_claude_resume() {
        let fixture = session_fixture("claude-resume");
        let root = fixture.join("sessions");
        let project = fixture.join("project");
        fs::create_dir_all(&root).expect("create session root");
        fs::create_dir_all(&project).expect("create project root");
        let path = root.join("session.jsonl");
        fs::write(
            &path,
            serde_json::json!({
                "type": "user",
                "sessionId": "claude-native-id",
                "cwd": project,
                "message": {"role": "user", "content": "Continue the task"}
            })
            .to_string(),
        )
        .expect("write Claude session");
        let host = session_host(&fixture, Some(project.clone()));
        let list = host
            .preview_local_sessions(LocalSessionPreviewParams {
                authorized_roots: vec![root.to_string_lossy().to_string()],
                auto_discover: Some(false),
                agent: Some("claude-code".to_string()),
                scope: Some("all".to_string()),
                include_content_items: Some(false),
                limit: Some(100),
                paging_mode: Some("keyset".to_string()),
                ..LocalSessionPreviewParams::default()
            })
            .expect("keyset summary");
        let source_revision = list.source_revision.expect("source revision");
        let session_id = list.session_rows[0].id.clone();
        let project_id = Some("project:test".to_string());

        let record = host
            .preview_session_resume(
                SessionResumePreviewParams {
                    authorized_roots: vec![root.to_string_lossy().to_string()],
                    auto_discover: false,
                    agent: "claude-code".to_string(),
                    project_root: project.to_string_lossy().to_string(),
                    current_cwd: project.to_string_lossy().to_string(),
                    session_id,
                    expected_source_revision: source_revision.clone(),
                    expected_snapshot_revision: SNAPSHOT_REVISION.to_string(),
                },
                SNAPSHOT_REVISION,
                project_id,
            )
            .expect("resume preview");
        assert_eq!(
            record.resume.argv,
            vec!["claude", "--resume", "claude-native-id"]
        );
        assert_eq!(record.source_revision, source_revision);
        assert_eq!(record.snapshot_revision, SNAPSHOT_REVISION);
        assert!(record.coverage.is_complete());
        assert_eq!(record.actions.len(), 1);
        assert_eq!(record.actions[0].source_revision, record.snapshot_revision);
        let _ = fs::remove_dir_all(fixture);
    }

    #[test]
    fn bounded_file_resolver_rejects_stale_or_unknown_selection() {
        let fixture = session_fixture("stale-resume");
        let root = fixture.join("sessions");
        fs::create_dir_all(&root).expect("create session root");
        fs::write(
            root.join("session.jsonl"),
            serde_json::json!({
                "type": "user",
                "sessionId": "claude-native-id",
                "cwd": fixture.join("project"),
                "message": {"role": "user", "content": "Claude session"}
            })
            .to_string(),
        )
        .expect("write Claude session");
        let host = session_host(&fixture, Some(fixture.join("project")));
        let list = host
            .preview_local_sessions(LocalSessionPreviewParams {
                authorized_roots: vec![root.to_string_lossy().to_string()],
                auto_discover: Some(false),
                agent: Some("claude-code".to_string()),
                include_content_items: Some(false),
                paging_mode: Some("keyset".to_string()),
                ..LocalSessionPreviewParams::default()
            })
            .expect("keyset summary");
        let source_revision = list.source_revision.expect("source revision");
        let base = SessionResumePreviewParams {
            authorized_roots: vec![root.to_string_lossy().to_string()],
            auto_discover: false,
            agent: "claude-code".to_string(),
            project_root: fixture.join("project").to_string_lossy().to_string(),
            current_cwd: fixture.join("project").to_string_lossy().to_string(),
            session_id: list.session_rows[0].id.clone(),
            expected_source_revision: source_revision,
            expected_snapshot_revision: SNAPSHOT_REVISION.to_string(),
        };

        let mut stale_snapshot = base.clone();
        stale_snapshot.expected_snapshot_revision = "sha256:stale".to_string();
        assert_eq!(
            host.preview_session_resume(stale_snapshot, SNAPSHOT_REVISION, None)
                .expect_err("stale product snapshot"),
            SessionResumePreviewError::SourceChanged
        );
        let mut stale_source = base.clone();
        stale_source.expected_source_revision = "sha256:stale".to_string();
        assert_eq!(
            host.preview_session_resume(stale_source, SNAPSHOT_REVISION, None)
                .expect_err("stale source"),
            SessionResumePreviewError::SourceChanged
        );
        let mut unknown = base.clone();
        unknown.session_id = "local-session-unknown".to_string();
        assert_eq!(
            host.preview_session_resume(unknown, SNAPSHOT_REVISION, None)
                .expect_err("unknown session"),
            SessionResumePreviewError::SessionNotFound
        );

        let outside = fixture.join("outside");
        fs::create_dir_all(&outside).expect("create outside project");
        let mut forged_root = base.clone();
        forged_root.project_root = outside.to_string_lossy().to_string();
        forged_root.current_cwd = outside.to_string_lossy().to_string();
        assert_eq!(
            host.preview_session_resume(forged_root, SNAPSHOT_REVISION, None)
                .expect_err("client project root cannot replace active project"),
            SessionResumePreviewError::InvalidProjectContext
        );

        let mut escaped_cwd = base;
        escaped_cwd.current_cwd = outside.to_string_lossy().to_string();
        assert_eq!(
            host.preview_session_resume(escaped_cwd, SNAPSHOT_REVISION, None)
                .expect_err("current cwd must match active cwd and stay within project"),
            SessionResumePreviewError::InvalidProjectContext
        );
        let _ = fs::remove_dir_all(fixture);
    }

    #[test]
    fn native_file_parser_is_adapter_specific_and_rejects_conflicts() {
        let nested_only = serde_json::json!({
            "type": "user",
            "payload": {"sessionId": "nested"}
        })
        .to_string();
        assert_eq!(
            native_resume_locator_from_file_content("claude-code", &nested_only),
            None
        );

        let codex = [
            serde_json::json!({
                "type": "session_meta",
                "payload": {"id": "codex-native"}
            })
            .to_string(),
            serde_json::json!({
                "type": "response_item",
                "payload": {"id": "unrelated"}
            })
            .to_string(),
        ]
        .join("\n");
        assert_eq!(
            native_resume_locator_from_file_content("codex", &codex),
            Some(SessionNativeResumeLocator::NativeId(
                "codex-native".to_string()
            ))
        );

        let conflicting = [
            serde_json::json!({"type":"user","sessionId":"one"}).to_string(),
            serde_json::json!({"type":"assistant","sessionId":"two"}).to_string(),
        ]
        .join("\n");
        assert_eq!(
            native_resume_locator_from_file_content("claude-code", &conflicting),
            None
        );
    }

    #[test]
    fn hermes_without_project_identity_remains_typed_unsupported() {
        let fixture = session_fixture("hermes-unassigned-resume");
        let project = fixture.join("project");
        let home = fixture.join("home");
        let db_path = home.join(".hermes/state.db");
        fs::create_dir_all(&project).expect("create project");
        fs::create_dir_all(db_path.parent().expect("Hermes database parent"))
            .expect("create Hermes database directory");
        let connection = Connection::open(&db_path).expect("open Hermes database");
        connection
            .execute_batch(
                "CREATE TABLE sessions (id TEXT PRIMARY KEY, source TEXT, parent_session_id TEXT, started_at REAL, ended_at REAL, message_count INTEGER, tool_call_count INTEGER, title TEXT);\
                 CREATE TABLE messages (id INTEGER PRIMARY KEY, session_id TEXT, role TEXT, content TEXT, timestamp REAL, tool_name TEXT, tool_calls TEXT, reasoning TEXT);",
            )
            .expect("create Hermes schema");
        connection
            .execute(
                "INSERT INTO sessions (id, source, started_at, ended_at, message_count, tool_call_count, title) VALUES (?1, 'cli', 10.0, 20.0, 0, 0, ?2)",
                params!["20260724_091523_a1b2c3", "Hermes unassigned session"],
            )
            .expect("insert Hermes session");
        drop(connection);

        let host = ServiceHost {
            app_data_dir: fixture.join("app-data"),
            adapter_ctx: AdapterContext {
                user_home: home,
                project_root: Some(project.clone()),
                project_cwd: Some(project.clone()),
                extra_roots: Vec::new(),
            },
        };
        let list = host
            .preview_local_sessions(LocalSessionPreviewParams {
                auto_discover: Some(true),
                agent: Some("hermes".to_string()),
                scope: Some("all".to_string()),
                include_content_items: Some(false),
                paging_mode: Some("keyset".to_string()),
                limit: Some(20),
                ..LocalSessionPreviewParams::default()
            })
            .expect("Hermes summary");
        assert_eq!(list.session_rows.len(), 1);
        assert!(list.session_rows[0].project_root.is_none());
        assert!(list
            .gap_notes
            .iter()
            .any(|note| note.contains("remain unassigned")));

        let record = host
            .preview_session_resume(
                SessionResumePreviewParams {
                    authorized_roots: Vec::new(),
                    auto_discover: true,
                    agent: "hermes".to_string(),
                    project_root: project.to_string_lossy().to_string(),
                    current_cwd: project.to_string_lossy().to_string(),
                    session_id: list.session_rows[0].id.clone(),
                    expected_source_revision: list.source_revision.expect("source revision"),
                    expected_snapshot_revision: SNAPSHOT_REVISION.to_string(),
                },
                SNAPSHOT_REVISION,
                Some("project:test".to_string()),
            )
            .expect("typed unsupported Hermes preview");
        assert_eq!(record.resume.state, ResumeCapabilityState::Unsupported);
        assert_eq!(
            record.resume.unsupported_reason,
            Some(ResumeUnsupportedReason::InvalidProjectContext)
        );
        assert!(record.project_id.is_none());
        assert!(record.resume.argv.is_empty());
        assert!(record.actions.is_empty());

        let project_only = host
            .preview_local_sessions(LocalSessionPreviewParams {
                auto_discover: Some(true),
                agent: Some("hermes".to_string()),
                scope: Some("project".to_string()),
                include_content_items: Some(false),
                limit: Some(20),
                ..LocalSessionPreviewParams::default()
            })
            .expect("Hermes project summary");
        assert!(project_only.session_rows.is_empty());
        let _ = fs::remove_dir_all(fixture);
    }

    #[test]
    fn sqlite_resolver_uses_openclaw_session_key_instead_of_transcript_id() {
        let fixture = session_fixture("openclaw-resume");
        let workspace = fixture.join("workspace");
        let home = fixture.join("home");
        let state = home.join(".openclaw");
        let db_path = state.join("agents/main/agent/openclaw-agent.sqlite");
        fs::create_dir_all(&workspace).expect("create workspace");
        fs::create_dir_all(db_path.parent().expect("database parent"))
            .expect("create database parent");
        fs::create_dir_all(&state).expect("create state");
        fs::write(
            state.join("openclaw.json"),
            format!(
                "{{agents: {{defaults: {{workspace: {:?}}}}}}}",
                workspace.to_string_lossy()
            ),
        )
        .expect("write OpenClaw config");
        let connection = Connection::open(&db_path).expect("open database");
        connection
            .execute_batch(
                "CREATE TABLE sessions (session_id TEXT PRIMARY KEY, session_key TEXT NOT NULL, session_scope TEXT, created_at INTEGER NOT NULL, updated_at INTEGER NOT NULL, started_at INTEGER, ended_at INTEGER, status TEXT, chat_type TEXT, channel TEXT, account_id TEXT, primary_conversation_id TEXT, model_provider TEXT, model TEXT, agent_harness_id TEXT, parent_session_key TEXT, spawned_by TEXT, display_name TEXT);\
                 CREATE TABLE session_routes (session_key TEXT NOT NULL, session_id TEXT NOT NULL, updated_at INTEGER NOT NULL);\
                 CREATE TABLE session_entries (session_id TEXT NOT NULL, session_key TEXT NOT NULL, entry_json TEXT NOT NULL, updated_at INTEGER NOT NULL);\
                 CREATE TABLE transcript_events (session_id TEXT NOT NULL, seq INTEGER NOT NULL, event_json TEXT NOT NULL, created_at INTEGER NOT NULL);",
            )
            .expect("create schema");
        connection
            .execute(
                "INSERT INTO sessions (session_id, session_key, created_at, updated_at, started_at, ended_at, status, display_name) VALUES (?1, ?2, 1000, 2000, 1000, 2000, 'active', 'OpenClaw session')",
                params!["transcript-native-id", "agent:main:tui:project"],
            )
            .expect("insert session");
        connection
            .execute(
                "INSERT INTO session_routes (session_key, session_id, updated_at) VALUES (?1, ?2, 2000)",
                params!["agent:main:tui:project", "transcript-native-id"],
            )
            .expect("insert route");
        connection
            .execute(
                "INSERT INTO session_entries (session_id, session_key, entry_json, updated_at) VALUES (?1, ?2, '{}', 2000)",
                params!["transcript-native-id", "agent:main:tui:project"],
            )
            .expect("insert entry");
        connection
            .execute(
                "INSERT INTO transcript_events (session_id, seq, event_json, created_at) VALUES (?1, 1, ?2, 2000)",
                params![
                    "transcript-native-id",
                    serde_json::json!({
                        "type":"message",
                        "message":{"role":"user","content":[{"type":"text","text":"Continue"}]}
                    })
                    .to_string()
                ],
            )
            .expect("insert event");
        drop(connection);

        let host = ServiceHost {
            app_data_dir: fixture.join("app-data"),
            adapter_ctx: AdapterContext {
                user_home: home,
                project_root: Some(workspace.clone()),
                project_cwd: Some(workspace.clone()),
                extra_roots: Vec::new(),
            },
        };
        let list = host
            .preview_local_sessions(LocalSessionPreviewParams {
                auto_discover: Some(true),
                agent: Some("openclaw".to_string()),
                scope: Some("all".to_string()),
                include_content_items: Some(false),
                limit: Some(20),
                ..LocalSessionPreviewParams::default()
            })
            .expect("OpenClaw summary");
        let record = host
            .preview_session_resume(
                SessionResumePreviewParams {
                    authorized_roots: Vec::new(),
                    auto_discover: true,
                    agent: "openclaw".to_string(),
                    project_root: workspace.to_string_lossy().to_string(),
                    current_cwd: workspace.to_string_lossy().to_string(),
                    session_id: list.session_rows[0].id.clone(),
                    expected_source_revision: list.source_revision.expect("source revision"),
                    expected_snapshot_revision: SNAPSHOT_REVISION.to_string(),
                },
                SNAPSHOT_REVISION,
                Some("project:test".to_string()),
            )
            .expect("OpenClaw resume");
        assert_eq!(
            record.resume.argv,
            vec!["openclaw", "tui", "--session", "agent:main:tui:project"]
        );
        assert!(!record
            .resume
            .argv
            .contains(&"transcript-native-id".to_string()));
        let _ = fs::remove_dir_all(fixture);
    }

    fn session_fixture(label: &str) -> PathBuf {
        std::env::temp_dir().join(format!(
            "skills-copilot-session-resume-{label}-{}-{}",
            std::process::id(),
            NEXT_FIXTURE.fetch_add(1, Ordering::Relaxed)
        ))
    }

    fn session_host(root: &Path, project_root: Option<PathBuf>) -> ServiceHost {
        ServiceHost {
            app_data_dir: root.join("app-data"),
            adapter_ctx: AdapterContext {
                user_home: root.join("home"),
                project_root: project_root.clone(),
                project_cwd: project_root,
                extra_roots: Vec::new(),
            },
        }
    }
}
