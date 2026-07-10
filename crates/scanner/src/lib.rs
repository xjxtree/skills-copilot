use std::{
    collections::{HashMap, HashSet},
    fs,
    path::{Path, PathBuf},
    time::UNIX_EPOCH,
};

use sha2::{Digest, Sha256};
use skills_copilot_core::{
    AdapterContext, AdapterRoot, AgentAdapter, AgentId, RootSource, Scope, SkillInstance,
    SkillState,
};
use thiserror::Error;

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
pub enum ScanIssueKind {
    RootUnavailable,
    RootOutsideAllowlist,
    DirectoryUnreadable,
    EntryUnreadable,
    FileUnreadable,
    FileTooLarge,
    BudgetExceeded,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScanIssue {
    pub path: PathBuf,
    pub kind: ScanIssueKind,
    pub detail: String,
}

#[derive(Debug, Default)]
pub struct ScanReport {
    pub instances: Vec<SkillInstance>,
    pub skipped_roots: Vec<PathBuf>,
    /// Canonical paths of roots that were completely enumerated this round.
    /// Callers (e.g. catalog sweep) should only consider records whose
    /// path is under one of these roots as candidates for cleanup.
    pub scanned_roots: Vec<PathBuf>,
    /// Canonical paths of roots whose traversal encountered a filesystem error.
    pub partial_roots: Vec<PathBuf>,
    pub issues: Vec<ScanIssue>,
    pub stats: ScanStats,
}

#[derive(Debug, Clone, Copy)]
struct ScanLimits {
    max_depth: usize,
    max_directories: usize,
    max_entries: usize,
    max_skill_files: usize,
    max_skill_bytes: u64,
    max_total_skill_bytes: u64,
}

impl Default for ScanLimits {
    fn default() -> Self {
        Self {
            max_depth: 64,
            max_directories: 50_000,
            max_entries: 200_000,
            max_skill_files: 25_000,
            max_skill_bytes: 2 * 1024 * 1024,
            max_total_skill_bytes: 256 * 1024 * 1024,
        }
    }
}

#[derive(Debug, Clone, Copy, Default, Eq, PartialEq)]
pub struct ScanStats {
    pub directories_visited: usize,
    pub entries_seen: usize,
    pub skill_files_seen: usize,
    pub bytes_read: u64,
    pub budget_exhausted: bool,
}

#[derive(Debug, Default)]
struct ScanBudget {
    stats: ScanStats,
}

#[derive(Debug, Clone)]
struct ResolvedScanRoot {
    declared: AdapterRoot,
    canonical: PathBuf,
    declared_is_symlink: bool,
}

#[derive(Debug, Clone)]
struct ResolvedAllowedRoot {
    scope: Scope,
    canonical: PathBuf,
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum RootWalkStatus {
    Complete,
    Partial,
}

#[derive(Debug, Error)]
pub enum ScannerError {
    #[error("failed to read directory {path}: {source}")]
    ReadDir {
        path: PathBuf,
        source: std::io::Error,
    },
    #[error("failed to canonicalize root {path}: {source}")]
    CanonicalizeRoot {
        path: PathBuf,
        source: std::io::Error,
    },
}

pub fn scan_roots(adapter: &dyn AgentAdapter, ctx: &AdapterContext) -> Vec<AdapterRoot> {
    adapter.roots(ctx)
}

pub fn scan_agent(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
) -> Result<ScanReport, ScannerError> {
    scan_agent_with_limits(adapter, ctx, ScanLimits::default())
}

fn scan_agent_with_limits(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
    limits: ScanLimits,
) -> Result<ScanReport, ScannerError> {
    scan_agent_with_limits_and_symlink_inspector(adapter, ctx, limits, |path| {
        fs::symlink_metadata(path).map(|metadata| metadata.file_type().is_symlink())
    })
}

#[cfg(test)]
fn scan_agent_with_symlink_inspector<F>(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
    inspect_declared_symlink: F,
) -> Result<ScanReport, ScannerError>
where
    F: FnMut(&Path) -> std::io::Result<bool>,
{
    scan_agent_with_limits_and_symlink_inspector(
        adapter,
        ctx,
        ScanLimits::default(),
        inspect_declared_symlink,
    )
}

fn scan_agent_with_limits_and_symlink_inspector<F>(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
    limits: ScanLimits,
    mut inspect_declared_symlink: F,
) -> Result<ScanReport, ScannerError>
where
    F: FnMut(&Path) -> std::io::Result<bool>,
{
    let mut report = ScanReport::default();
    let mut budget = ScanBudget::default();
    let roots = adapter.roots(ctx);
    let overrides = SkillConfigOverrides::preload(adapter.id(), ctx, &roots);
    let mut resolved_roots = Vec::new();

    for declared in roots {
        let declared_is_symlink = match inspect_declared_symlink(&declared.path) {
            Ok(declared_is_symlink) => declared_is_symlink,
            Err(error) => {
                report.skipped_roots.push(declared.path.clone());
                report.issues.push(ScanIssue {
                    path: declared.path,
                    kind: ScanIssueKind::RootUnavailable,
                    detail: format!("failed to inspect declared root: {error}"),
                });
                continue;
            }
        };
        let canonical = match resolve_directory_root(&declared.path) {
            Ok(canonical) => canonical,
            Err(detail) => {
                report.skipped_roots.push(declared.path.clone());
                report.issues.push(ScanIssue {
                    path: declared.path,
                    kind: ScanIssueKind::RootUnavailable,
                    detail,
                });
                continue;
            }
        };
        let location_authorization_is_deferred =
            !root_source_self_authorizes(&declared.source) && declared_is_symlink;
        if !location_authorization_is_deferred
            && !is_allowed_canonical_root(adapter.id(), ctx, &declared, &canonical)
        {
            report.skipped_roots.push(declared.path.clone());
            report.issues.push(ScanIssue {
                path: declared.path,
                kind: ScanIssueKind::RootOutsideAllowlist,
                detail: format!(
                    "canonical root {} is outside its declared scope",
                    canonical.display()
                ),
            });
            continue;
        }
        resolved_roots.push(ResolvedScanRoot {
            declared,
            canonical,
            declared_is_symlink,
        });
    }

    let mut allowed_roots = adapter
        .link_target_roots(ctx)
        .into_iter()
        .filter_map(|declared| {
            let canonical = resolve_directory_root(&declared.path).ok()?;
            is_allowed_canonical_root(adapter.id(), ctx, &declared, &canonical).then_some(
                ResolvedAllowedRoot {
                    scope: declared.scope,
                    canonical,
                },
            )
        })
        .collect::<Vec<_>>();

    allowed_roots.extend(
        resolved_roots
            .iter()
            .filter(|root| {
                root_source_self_authorizes(&root.declared.source) || !root.declared_is_symlink
            })
            .map(|root| ResolvedAllowedRoot {
                scope: root.declared.scope,
                canonical: root.canonical.clone(),
            }),
    );

    let mut resolved_root_keys = HashSet::new();
    let mut walked_roots = Vec::new();
    for root in resolved_roots {
        let needs_external_authority =
            !root_source_self_authorizes(&root.declared.source) && root.declared_is_symlink;
        if needs_external_authority
            && !target_is_allowlisted(&root.canonical, root.declared.scope, &allowed_roots)
        {
            report.skipped_roots.push(root.declared.path.clone());
            report.issues.push(ScanIssue {
                path: root.declared.path,
                kind: ScanIssueKind::RootOutsideAllowlist,
                detail: format!(
                    "canonical root {} is not under another declared root with the same scope",
                    root.canonical.display()
                ),
            });
            continue;
        }
        let root_key = format!(
            "{}|{}",
            root.declared.scope.as_str(),
            root.canonical.to_string_lossy()
        );
        if resolved_root_keys.insert(root_key) {
            walked_roots.push(root);
        }
    }

    for root in walked_roots {
        match visit_root(
            adapter,
            ctx,
            &root,
            &allowed_roots,
            &overrides,
            &limits,
            &mut budget,
            &mut report,
        ) {
            RootWalkStatus::Complete => report.scanned_roots.push(root.canonical),
            RootWalkStatus::Partial => report.partial_roots.push(root.canonical),
        }
    }
    report.instances = dedup_instances(report.instances);
    report.stats = budget.stats;
    Ok(report)
}

fn resolve_directory_root(path: &Path) -> Result<PathBuf, String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("failed to canonicalize root: {error}"))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("failed to inspect canonical root: {error}"))?;
    if !metadata.is_dir() {
        return Err("root is not a directory".to_string());
    }
    Ok(canonical)
}

fn root_source_self_authorizes(source: &RootSource) -> bool {
    matches!(
        source,
        RootSource::Configured
            | RootSource::Admin
            | RootSource::Plugin
            | RootSource::System
            | RootSource::Extra
    )
}

fn target_is_allowlisted(path: &Path, scope: Scope, roots: &[ResolvedAllowedRoot]) -> bool {
    roots
        .iter()
        .filter(|root| root.scope == scope)
        .any(|root| path.starts_with(&root.canonical))
}

fn is_allowed_canonical_root(
    agent: AgentId,
    ctx: &AdapterContext,
    root: &AdapterRoot,
    canonical_root: &Path,
) -> bool {
    match root.source {
        RootSource::UserHome => ctx
            .user_home
            .canonicalize()
            .is_ok_and(|base| canonical_root.starts_with(base)),
        RootSource::Project if agent == AgentId::Openclaw => {
            openclaw_workspace_base_for_root_path(&root.path)
                .and_then(|workspace_root| workspace_root.canonicalize().ok())
                .is_some_and(|base| canonical_root.starts_with(base))
        }
        RootSource::Project => ctx
            .project_root
            .as_ref()
            .and_then(|project_root| project_root.canonicalize().ok())
            .is_some_and(|base| canonical_root.starts_with(base)),
        RootSource::Compatibility => match root.scope {
            Scope::AgentGlobal => ctx
                .user_home
                .canonicalize()
                .is_ok_and(|base| canonical_root.starts_with(base)),
            Scope::AgentProject => ctx
                .project_root
                .as_ref()
                .and_then(|project_root| project_root.canonicalize().ok())
                .is_some_and(|base| canonical_root.starts_with(base)),
            Scope::ToolGlobal => false,
            _ => false,
        },
        RootSource::Configured
        | RootSource::Admin
        | RootSource::Plugin
        | RootSource::System
        | RootSource::Extra => true,
    }
}

#[allow(clippy::too_many_arguments)]
fn visit_root(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
    root: &ResolvedScanRoot,
    allowed_roots: &[ResolvedAllowedRoot],
    overrides: &SkillConfigOverrides,
    limits: &ScanLimits,
    budget: &mut ScanBudget,
    report: &mut ScanReport,
) -> RootWalkStatus {
    // Each stack entry is (resolved_dir, display_root, depth).
    // `display_root` is the user-visible path of the directory being scanned.
    // For the initial scan root it is the declared path; for symlinked
    // subdirectories it remains the original link path so that the user sees
    // ~/.claude/skills/foo/SKILL.md rather than ~/.agents/skills/foo/SKILL.md.
    let mut stack: Vec<(PathBuf, PathBuf, usize)> =
        vec![(root.canonical.clone(), root.declared.path.clone(), 0)];
    let mut visited_dirs = HashSet::new();
    let mut partial = false;
    'walk: while let Some((dir, display_root, depth)) = stack.pop() {
        if !visited_dirs.insert(dir.clone()) {
            continue;
        }
        if budget.stats.directories_visited >= limits.max_directories {
            partial = true;
            record_budget_exceeded(
                budget,
                report,
                dir,
                format!("directory limit of {} was reached", limits.max_directories),
            );
            break;
        }
        budget.stats.directories_visited += 1;
        let entries = match fs::read_dir(&dir) {
            Ok(entries) => entries,
            Err(error) => {
                partial = true;
                report.issues.push(ScanIssue {
                    path: dir,
                    kind: ScanIssueKind::DirectoryUnreadable,
                    detail: error.to_string(),
                });
                continue;
            }
        };
        let mut readable_entries = Vec::new();
        let mut entry_budget_exhausted = false;
        for entry in entries {
            if budget.stats.entries_seen >= limits.max_entries {
                partial = true;
                entry_budget_exhausted = true;
                record_budget_exceeded(
                    budget,
                    report,
                    dir.clone(),
                    format!("entry limit of {} was reached", limits.max_entries),
                );
                break;
            }
            budget.stats.entries_seen += 1;
            match entry {
                Ok(entry) => readable_entries.push(entry),
                Err(error) => {
                    partial = true;
                    report.issues.push(ScanIssue {
                        path: dir.clone(),
                        kind: ScanIssueKind::EntryUnreadable,
                        detail: error.to_string(),
                    });
                }
            }
        }
        let mut entries = readable_entries;
        entries.sort_by_key(|entry| entry.file_name());
        for entry in entries {
            let path = entry.path();
            let display_path = display_root.join(entry.file_name());
            let file_type = match entry.file_type() {
                Ok(file_type) => file_type,
                Err(error) => {
                    partial = true;
                    report.issues.push(ScanIssue {
                        path,
                        kind: ScanIssueKind::EntryUnreadable,
                        detail: error.to_string(),
                    });
                    continue;
                }
            };
            if file_type.is_symlink() {
                let resolved = match path.canonicalize() {
                    Ok(resolved) => resolved,
                    Err(error) => {
                        partial = true;
                        report.issues.push(ScanIssue {
                            path,
                            kind: ScanIssueKind::EntryUnreadable,
                            detail: format!("failed to canonicalize entry: {error}"),
                        });
                        continue;
                    }
                };
                if !target_is_allowlisted(&resolved, root.declared.scope, allowed_roots) {
                    report.issues.push(ScanIssue {
                        path,
                        kind: ScanIssueKind::RootOutsideAllowlist,
                        detail: format!(
                            "symlink target {} is outside declared roots with the same scope",
                            resolved.display()
                        ),
                    });
                    continue;
                }
                let metadata = match fs::metadata(&resolved) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        partial = true;
                        report.issues.push(ScanIssue {
                            path: resolved,
                            kind: ScanIssueKind::EntryUnreadable,
                            detail: error.to_string(),
                        });
                        continue;
                    }
                };
                if metadata.is_dir() {
                    if depth >= limits.max_depth {
                        partial = true;
                        record_budget_exceeded(
                            budget,
                            report,
                            resolved,
                            format!("depth limit of {} was reached", limits.max_depth),
                        );
                    } else {
                        stack.push((resolved, display_path, depth + 1));
                    }
                } else if resolved
                    .file_name()
                    .map(|n| n == "SKILL.md")
                    .unwrap_or(false)
                {
                    if depth >= limits.max_depth {
                        partial = true;
                        record_budget_exceeded(
                            budget,
                            report,
                            resolved,
                            format!("depth limit of {} was reached", limits.max_depth),
                        );
                        continue;
                    }
                    if !adapter_accepts_skill_path(adapter.id(), &root.canonical, &resolved) {
                        continue;
                    }
                    if visit_skill_file(
                        adapter,
                        ctx,
                        root,
                        overrides,
                        resolved,
                        display_path,
                        metadata.len(),
                        limits,
                        budget,
                        report,
                    ) == SkillVisitStatus::BudgetExceeded
                    {
                        partial = true;
                        break 'walk;
                    }
                }
                continue;
            }
            if file_type.is_dir() {
                if depth >= limits.max_depth {
                    partial = true;
                    record_budget_exceeded(
                        budget,
                        report,
                        path,
                        format!("depth limit of {} was reached", limits.max_depth),
                    );
                } else {
                    stack.push((path, display_path, depth + 1));
                }
                continue;
            }
            if entry.file_name() == "SKILL.md" {
                let canonical_path = match path.canonicalize() {
                    Ok(canonical_path) => canonical_path,
                    Err(error) => {
                        partial = true;
                        report.issues.push(ScanIssue {
                            path,
                            kind: ScanIssueKind::EntryUnreadable,
                            detail: format!("failed to canonicalize entry: {error}"),
                        });
                        continue;
                    }
                };
                if !target_is_allowlisted(&canonical_path, root.declared.scope, allowed_roots) {
                    report.issues.push(ScanIssue {
                        path,
                        kind: ScanIssueKind::RootOutsideAllowlist,
                        detail: format!(
                            "file target {} is outside declared roots with the same scope",
                            canonical_path.display()
                        ),
                    });
                    continue;
                }
                if !adapter_accepts_skill_path(adapter.id(), &root.canonical, &canonical_path) {
                    continue;
                }
                if depth >= limits.max_depth {
                    partial = true;
                    record_budget_exceeded(
                        budget,
                        report,
                        canonical_path,
                        format!("depth limit of {} was reached", limits.max_depth),
                    );
                    continue;
                }
                let metadata = match fs::metadata(&canonical_path) {
                    Ok(metadata) => metadata,
                    Err(error) => {
                        report.issues.push(ScanIssue {
                            path: canonical_path.clone(),
                            kind: ScanIssueKind::FileUnreadable,
                            detail: error.to_string(),
                        });
                        let mut instance = broken_instance(
                            adapter,
                            &root.declared,
                            canonical_path.clone(),
                            format!("failed to inspect skill file: {error}"),
                        );
                        normalize_instance(
                            ctx,
                            &root.declared,
                            canonical_path,
                            overrides,
                            &mut instance,
                        );
                        instance.display_path = display_path;
                        report.instances.push(instance);
                        continue;
                    }
                };
                if visit_skill_file(
                    adapter,
                    ctx,
                    root,
                    overrides,
                    canonical_path,
                    display_path,
                    metadata.len(),
                    limits,
                    budget,
                    report,
                ) == SkillVisitStatus::BudgetExceeded
                {
                    partial = true;
                    break 'walk;
                }
            }
        }
        if entry_budget_exhausted {
            break;
        }
    }
    if partial {
        RootWalkStatus::Partial
    } else {
        RootWalkStatus::Complete
    }
}

#[derive(Debug, Clone, Copy, Eq, PartialEq)]
enum SkillVisitStatus {
    Complete,
    BudgetExceeded,
}

#[allow(clippy::too_many_arguments)]
fn visit_skill_file(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
    root: &ResolvedScanRoot,
    overrides: &SkillConfigOverrides,
    canonical_path: PathBuf,
    display_path: PathBuf,
    metadata_len: u64,
    limits: &ScanLimits,
    budget: &mut ScanBudget,
    report: &mut ScanReport,
) -> SkillVisitStatus {
    if budget.stats.skill_files_seen >= limits.max_skill_files {
        record_budget_exceeded(
            budget,
            report,
            canonical_path,
            format!("skill file limit of {} was reached", limits.max_skill_files),
        );
        return SkillVisitStatus::BudgetExceeded;
    }
    budget.stats.skill_files_seen += 1;

    if metadata_len > limits.max_skill_bytes {
        let detail = format!(
            "skill file is {metadata_len} bytes; per-file limit is {} bytes",
            limits.max_skill_bytes
        );
        report.issues.push(ScanIssue {
            path: canonical_path.clone(),
            kind: ScanIssueKind::FileTooLarge,
            detail: detail.clone(),
        });
        push_broken_instance(
            adapter,
            ctx,
            root,
            overrides,
            canonical_path,
            display_path,
            detail,
            report,
        );
        return SkillVisitStatus::Complete;
    }

    let Some(next_total) = budget.stats.bytes_read.checked_add(metadata_len) else {
        record_budget_exceeded(
            budget,
            report,
            canonical_path,
            "total skill byte counter overflowed".to_string(),
        );
        return SkillVisitStatus::BudgetExceeded;
    };
    if next_total > limits.max_total_skill_bytes {
        record_budget_exceeded(
            budget,
            report,
            canonical_path,
            format!(
                "total skill byte limit of {} bytes would be exceeded",
                limits.max_total_skill_bytes
            ),
        );
        return SkillVisitStatus::BudgetExceeded;
    }
    budget.stats.bytes_read = next_total;

    let mut instance = match read_skill_content_bounded(&canonical_path, limits.max_skill_bytes) {
        Ok(Some(content)) => adapter
            .parse_content(&canonical_path, content)
            .unwrap_or_else(|err| {
                broken_instance(adapter, &root.declared, canonical_path.clone(), err.message)
            }),
        Ok(None) => {
            let detail = format!(
                "skill file grew beyond the per-file limit of {} bytes while being read",
                limits.max_skill_bytes
            );
            report.issues.push(ScanIssue {
                path: canonical_path.clone(),
                kind: ScanIssueKind::FileTooLarge,
                detail: detail.clone(),
            });
            broken_instance(adapter, &root.declared, canonical_path.clone(), detail)
        }
        Err(error) => {
            let detail = format!("failed to read skill file: {error}");
            report.issues.push(ScanIssue {
                path: canonical_path.clone(),
                kind: ScanIssueKind::FileUnreadable,
                detail: detail.clone(),
            });
            broken_instance(adapter, &root.declared, canonical_path.clone(), detail)
        }
    };
    normalize_instance(
        ctx,
        &root.declared,
        canonical_path,
        overrides,
        &mut instance,
    );
    instance.display_path = display_path;
    report.instances.push(instance);
    SkillVisitStatus::Complete
}

#[allow(clippy::too_many_arguments)]
fn push_broken_instance(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
    root: &ResolvedScanRoot,
    overrides: &SkillConfigOverrides,
    canonical_path: PathBuf,
    display_path: PathBuf,
    detail: String,
    report: &mut ScanReport,
) {
    let mut instance = broken_instance(adapter, &root.declared, canonical_path.clone(), detail);
    normalize_instance(
        ctx,
        &root.declared,
        canonical_path,
        overrides,
        &mut instance,
    );
    instance.display_path = display_path;
    report.instances.push(instance);
}

fn record_budget_exceeded(
    budget: &mut ScanBudget,
    report: &mut ScanReport,
    path: PathBuf,
    detail: String,
) {
    budget.stats.budget_exhausted = true;
    report.issues.push(ScanIssue {
        path,
        kind: ScanIssueKind::BudgetExceeded,
        detail,
    });
}

fn read_skill_content_bounded(path: &Path, max_bytes: u64) -> std::io::Result<Option<String>> {
    use std::io::Read;

    let file = fs::File::open(path)?;
    let mut bytes = Vec::with_capacity((max_bytes.min(64 * 1024)) as usize);
    file.take(max_bytes.saturating_add(1))
        .read_to_end(&mut bytes)?;
    if bytes.len() as u64 > max_bytes {
        return Ok(None);
    }
    String::from_utf8(bytes)
        .map(Some)
        .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
}

fn adapter_accepts_skill_path(
    agent: AgentId,
    canonical_root: &Path,
    canonical_path: &Path,
) -> bool {
    if agent != AgentId::Pi {
        return true;
    }
    let Ok(relative) = canonical_path.strip_prefix(canonical_root) else {
        return false;
    };
    let components = relative.components().collect::<Vec<_>>();
    components.len() == 2
        && canonical_path.file_name().and_then(|name| name.to_str()) == Some("SKILL.md")
}

fn dedup_instances(instances: Vec<SkillInstance>) -> Vec<SkillInstance> {
    let mut seen_paths = HashSet::new();
    let mut deduped = Vec::new();

    for instance in instances {
        let path_key = format!(
            "{}|{}|{}",
            instance.agent.as_str(),
            instance.scope.as_str(),
            instance.path.to_string_lossy()
        );
        if !seen_paths.insert(path_key) {
            continue;
        }

        deduped.push(instance);
    }

    deduped
}

fn openclaw_workspace_base_for_root_path(root_path: &Path) -> Option<PathBuf> {
    if root_path.file_name().and_then(|name| name.to_str()) != Some("skills") {
        return None;
    }
    let parent = root_path.parent()?;
    if parent.file_name().and_then(|name| name.to_str()) == Some(".agents") {
        return parent.parent().map(Path::to_path_buf);
    }
    Some(parent.to_path_buf())
}

fn normalize_instance(
    ctx: &AdapterContext,
    root: &AdapterRoot,
    canonical_path: PathBuf,
    overrides: &SkillConfigOverrides,
    instance: &mut SkillInstance,
) {
    instance.scope = root.scope;
    instance.path = canonical_path.clone();
    instance.project_root = match root.scope {
        Scope::AgentProject if instance.agent == AgentId::Openclaw => {
            openclaw_workspace_base_for_root_path(&root.path).or_else(|| ctx.project_root.clone())
        }
        Scope::AgentProject => ctx.project_root.clone(),
        _ => None,
    };
    instance.id = stable_instance_id(
        instance.agent.as_str(),
        root.scope.as_str(),
        &canonical_path,
    );
    instance.definition_id = canonical_definition_id(&instance.name);
    instance.fingerprint = content_fingerprint(&instance.frontmatter_raw, &instance.body);
    if let Ok(metadata) = fs::metadata(&canonical_path) {
        instance.mtime = metadata
            .modified()
            .ok()
            .and_then(|time| time.duration_since(UNIX_EPOCH).ok())
            .map(|duration| duration.as_millis() as i64)
            .unwrap_or_default();
    }
    instance.first_seen = instance.mtime;
    instance.last_seen = instance.mtime;

    // Agent config overrides are scoped to the current adapter only. Keep the
    // per-scan settings cache outside adapter parsing so one file is not reread
    // for every skill in a root.
    if matches!(instance.state, SkillState::Loaded) && overrides.is_disabled(ctx, root, instance) {
        instance.enabled = false;
        instance.state = SkillState::Disabled;
    }
}

#[derive(Debug)]
struct SkillConfigOverrides {
    agent: AgentId,
    disabled_by_settings_path: HashMap<PathBuf, HashSet<String>>,
}

impl SkillConfigOverrides {
    fn preload(agent: AgentId, ctx: &AdapterContext, roots: &[AdapterRoot]) -> Self {
        let mut disabled_by_settings_path = HashMap::new();
        match agent {
            AgentId::ClaudeCode => {
                for settings_path in roots
                    .iter()
                    .filter_map(|root| claude_settings_path_for(ctx, root))
                    .collect::<HashSet<_>>()
                {
                    if let Some(disabled) = read_disabled_claude_skill_overrides(&settings_path) {
                        disabled_by_settings_path.insert(settings_path, disabled);
                    }
                }
            }
            AgentId::Opencode => {
                for settings_path in roots
                    .iter()
                    .filter_map(|root| opencode_settings_path_for(ctx, root))
                    .collect::<HashSet<_>>()
                {
                    if let Some(disabled) = read_disabled_opencode_skill_permissions(&settings_path)
                    {
                        disabled_by_settings_path.insert(settings_path, disabled);
                    }
                }
            }
            AgentId::Hermes => {
                for settings_path in roots
                    .iter()
                    .filter_map(|root| hermes_settings_path_for(ctx, root))
                    .collect::<HashSet<_>>()
                {
                    if let Some(disabled) = read_disabled_hermes_skills(&settings_path) {
                        disabled_by_settings_path.insert(settings_path, disabled);
                    }
                }
            }
            AgentId::Openclaw => {
                for settings_path in roots
                    .iter()
                    .filter_map(|root| openclaw_settings_path_for(ctx, root))
                    .collect::<HashSet<_>>()
                {
                    if let Some(disabled) = read_disabled_openclaw_skill_entries(&settings_path) {
                        disabled_by_settings_path.insert(settings_path, disabled);
                    }
                }
            }
            _ => {}
        }

        Self {
            agent,
            disabled_by_settings_path,
        }
    }

    fn is_disabled(
        &self,
        ctx: &AdapterContext,
        root: &AdapterRoot,
        instance: &SkillInstance,
    ) -> bool {
        let settings_path = match self.agent {
            AgentId::Opencode => opencode_settings_path_for(ctx, root),
            AgentId::ClaudeCode => claude_settings_path_for(ctx, root),
            AgentId::Hermes => hermes_settings_path_for(ctx, root),
            AgentId::Openclaw => openclaw_settings_path_for(ctx, root),
            _ => None,
        };
        let skill_key = match self.agent {
            AgentId::Openclaw => {
                openclaw_config_key_from_frontmatter(&instance.frontmatter_raw, &instance.name)
            }
            _ => instance.name.clone(),
        };
        settings_path
            .and_then(|settings_path| self.disabled_by_settings_path.get(&settings_path))
            .is_some_and(|disabled| disabled.contains(&skill_key))
    }
}

fn claude_settings_path_for(ctx: &AdapterContext, root: &AdapterRoot) -> Option<PathBuf> {
    match root.scope {
        Scope::AgentGlobal => Some(ctx.user_home.join(".claude/settings.json")),
        Scope::AgentProject => ctx
            .project_root
            .as_ref()
            .map(|p| p.join(".claude/settings.local.json")),
        Scope::ToolGlobal => None,
        // Scope is `#[non_exhaustive]`; future variants have no default path.
        _ => None,
    }
}

fn opencode_settings_path_for(ctx: &AdapterContext, root: &AdapterRoot) -> Option<PathBuf> {
    match root.scope {
        Scope::AgentGlobal => Some(ctx.user_home.join(".config/opencode/opencode.json")),
        Scope::AgentProject => ctx.project_root.as_ref().map(|p| p.join("opencode.json")),
        Scope::ToolGlobal => None,
        _ => None,
    }
}

fn hermes_settings_path_for(ctx: &AdapterContext, root: &AdapterRoot) -> Option<PathBuf> {
    match root.scope {
        Scope::AgentGlobal => Some(ctx.user_home.join(".hermes/config.yaml")),
        Scope::AgentProject | Scope::ToolGlobal => None,
        _ => None,
    }
}

fn openclaw_settings_path_for(ctx: &AdapterContext, root: &AdapterRoot) -> Option<PathBuf> {
    match root.scope {
        Scope::AgentGlobal | Scope::AgentProject => {
            Some(ctx.user_home.join(".openclaw/openclaw.json"))
        }
        Scope::ToolGlobal => None,
        _ => None,
    }
}

fn read_disabled_claude_skill_overrides(settings_path: &Path) -> Option<HashSet<String>> {
    let Ok(content) = fs::read_to_string(settings_path) else {
        return None;
    };
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&content) else {
        return None;
    };
    let overrides = value
        .get("skillOverrides")
        .and_then(serde_json::Value::as_object)?;
    Some(
        overrides
            .iter()
            .filter(|(_, value)| *value == "off")
            .map(|(name, _)| name.clone())
            .collect(),
    )
}

fn read_disabled_opencode_skill_permissions(settings_path: &Path) -> Option<HashSet<String>> {
    let Ok(content) = fs::read_to_string(settings_path) else {
        return None;
    };
    let stripped = strip_json_comments(&content);
    let Ok(value) = serde_json::from_str::<serde_json::Value>(&stripped) else {
        return None;
    };
    let skill_permissions = value
        .get("permission")
        .and_then(|permission| permission.get("skill"))
        .and_then(serde_json::Value::as_object)?;
    Some(
        skill_permissions
            .iter()
            .filter(|(_, value)| value.as_str() == Some("deny"))
            .filter(|(name, _)| !name.contains('*') && !name.contains('?'))
            .map(|(name, _)| name.clone())
            .collect(),
    )
}

fn read_disabled_hermes_skills(settings_path: &Path) -> Option<HashSet<String>> {
    let Ok(content) = fs::read_to_string(settings_path) else {
        return None;
    };
    let value = serde_yaml::from_str::<serde_yaml::Value>(&content).ok()?;
    let disabled = value
        .get("skills")
        .and_then(|skills| skills.get("disabled"))
        .and_then(serde_yaml::Value::as_sequence)?;
    Some(
        disabled
            .iter()
            .filter_map(serde_yaml::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect(),
    )
}

fn read_disabled_openclaw_skill_entries(settings_path: &Path) -> Option<HashSet<String>> {
    let Ok(content) = fs::read_to_string(settings_path) else {
        return None;
    };
    let value = json5::from_str::<serde_json::Value>(&content).ok()?;
    let entries = value
        .get("skills")
        .and_then(|skills| skills.get("entries"))
        .and_then(serde_json::Value::as_object)?;
    Some(
        entries
            .iter()
            .filter(|(_, entry)| {
                entry.get("enabled").and_then(serde_json::Value::as_bool) == Some(false)
            })
            .map(|(key, _)| key.clone())
            .collect(),
    )
}

fn openclaw_config_key_from_frontmatter(frontmatter_raw: &str, fallback_name: &str) -> String {
    serde_yaml::from_str::<serde_yaml::Value>(frontmatter_raw)
        .ok()
        .and_then(|frontmatter| {
            frontmatter
                .get("metadata")
                .and_then(|metadata| metadata.get("openclaw"))
                .and_then(|openclaw| openclaw.get("skillKey"))
                .and_then(serde_yaml::Value::as_str)
                .map(str::trim)
                .filter(|value| !value.is_empty())
                .map(ToString::to_string)
        })
        .unwrap_or_else(|| fallback_name.to_string())
}

fn strip_json_comments(content: &str) -> String {
    let mut output = String::with_capacity(content.len());
    let mut chars = content.chars().peekable();
    let mut in_string = false;
    let mut escaped = false;

    while let Some(ch) = chars.next() {
        if in_string {
            output.push(ch);
            if escaped {
                escaped = false;
            } else if ch == '\\' {
                escaped = true;
            } else if ch == '"' {
                in_string = false;
            }
            continue;
        }

        if ch == '"' {
            in_string = true;
            output.push(ch);
            continue;
        }

        if ch == '/' {
            match chars.peek().copied() {
                Some('/') => {
                    chars.next();
                    for next in chars.by_ref() {
                        if next == '\n' {
                            output.push('\n');
                            break;
                        }
                    }
                    continue;
                }
                Some('*') => {
                    chars.next();
                    let mut previous = '\0';
                    for next in chars.by_ref() {
                        if previous == '*' && next == '/' {
                            break;
                        }
                        previous = next;
                    }
                    continue;
                }
                _ => {}
            }
        }

        output.push(ch);
    }

    output
}

fn broken_instance(
    adapter: &dyn AgentAdapter,
    root: &AdapterRoot,
    path: PathBuf,
    message: String,
) -> SkillInstance {
    let name = path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
        .unwrap_or("broken")
        .to_string();
    SkillInstance {
        id: stable_instance_id(adapter.id().as_str(), root.scope.as_str(), &path),
        agent: adapter.id(),
        scope: root.scope,
        project_root: None,
        path: path.clone(),
        display_path: path,
        definition_id: canonical_definition_id(&name),
        name: name.clone(),
        display_name: name,
        description: message,
        version: None,
        state: SkillState::Broken,
        enabled: false,
        frontmatter_raw: String::new(),
        body: String::new(),
        scripts: Vec::new(),
        permissions: Default::default(),
        fingerprint: String::new(),
        mtime: 0,
        first_seen: 0,
        last_seen: 0,
    }
}

fn stable_instance_id(agent: &str, scope: &str, path: &Path) -> String {
    hash_string(&format!("{}|{}|{}", agent, scope, path.to_string_lossy()))
}

fn canonical_definition_id(name: &str) -> String {
    hash_string(&name.to_ascii_lowercase())
}

fn content_fingerprint(frontmatter: &str, body: &str) -> String {
    hash_string(&format!("{frontmatter}\n---\n{body}"))
}

fn hash_string(value: &str) -> String {
    let digest = Sha256::digest(value.as_bytes());
    format!("{digest:x}")
}

#[cfg(test)]
mod tests {
    use skills_copilot_adapters::{
        ClaudeCodeAdapter, CodexAdapter, HermesAdapter, OpenclawAdapter, OpencodeAdapter, PiAdapter,
    };
    use skills_copilot_core::{AdapterContext, AdapterRoot, RootSource};

    use super::*;

    #[derive(Debug)]
    struct TestAdapter {
        roots: Vec<AdapterRoot>,
        link_target_roots: Vec<AdapterRoot>,
    }

    impl AgentAdapter for TestAdapter {
        fn id(&self) -> AgentId {
            AgentId::ClaudeCode
        }

        fn display_name(&self) -> &'static str {
            "Scanner Test Adapter"
        }

        fn roots(&self, _ctx: &AdapterContext) -> Vec<AdapterRoot> {
            self.roots.clone()
        }

        fn link_target_roots(&self, _ctx: &AdapterContext) -> Vec<AdapterRoot> {
            self.link_target_roots.clone()
        }

        fn parse(&self, path: &Path) -> Result<SkillInstance, skills_copilot_core::AdapterError> {
            ClaudeCodeAdapter.parse(path)
        }

        fn parse_content(
            &self,
            path: &Path,
            content: String,
        ) -> Result<SkillInstance, skills_copilot_core::AdapterError> {
            ClaudeCodeAdapter.parse_content(path, content)
        }

        fn is_enabled(&self, instance: &SkillInstance) -> bool {
            instance.enabled
        }

        fn config_paths(&self, _ctx: &AdapterContext) -> Vec<PathBuf> {
            Vec::new()
        }
    }

    fn test_scan_limits() -> ScanLimits {
        ScanLimits {
            max_depth: 8,
            max_directories: 8,
            max_entries: 8,
            max_skill_files: 8,
            max_skill_bytes: 128,
            max_total_skill_bytes: 1_024,
        }
    }

    fn budget_test_fixture(label: &str) -> (PathBuf, PathBuf, AdapterContext, TestAdapter) {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-scanner-budget-{label}-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("system clock")
                .as_nanos()
        ));
        let scan_root = temp_root.join("skills");
        std::fs::create_dir_all(&scan_root).expect("create scan root");
        let ctx = AdapterContext {
            user_home: temp_root.join("home"),
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        };
        let adapter = TestAdapter {
            roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: scan_root.clone(),
                source: RootSource::Extra,
            }],
            link_target_roots: Vec::new(),
        };
        (temp_root, scan_root, ctx, adapter)
    }

    fn write_budget_skill(root: &Path, name: &str, content: &str) -> PathBuf {
        let skill_dir = root.join(name);
        std::fs::create_dir_all(&skill_dir).expect("create budget skill directory");
        let path = skill_dir.join("SKILL.md");
        std::fs::write(&path, content).expect("write budget skill");
        path
    }

    #[test]
    fn oversized_skill_becomes_broken_without_stopping_scan() {
        let (temp_root, scan_root, ctx, adapter) = budget_test_fixture("oversized");
        let oversized_path = write_budget_skill(&scan_root, "oversized", &"x".repeat(129));
        write_budget_skill(
            &scan_root,
            "valid",
            "---\nname: valid\ndescription: valid fixture\n---\nbody\n",
        );

        let report = scan_agent_with_limits(&adapter, &ctx, test_scan_limits())
            .expect("bounded scan returns a report");
        let oversized = report
            .instances
            .iter()
            .find(|instance| instance.name == "oversized")
            .expect("oversized instance");
        let valid = report
            .instances
            .iter()
            .find(|instance| instance.name == "valid")
            .expect("valid instance");
        let canonical_oversized_path = oversized_path
            .canonicalize()
            .expect("canonical oversized path");

        assert_eq!(oversized.state, SkillState::Broken);
        assert_eq!(valid.state, SkillState::Loaded);
        assert!(report.issues.iter().any(|issue| {
            issue.kind == ScanIssueKind::FileTooLarge && issue.path == canonical_oversized_path
        }));

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn total_byte_budget_marks_root_partial_without_unbounded_read() {
        let (temp_root, scan_root, ctx, adapter) = budget_test_fixture("total-bytes");
        let first_path = write_budget_skill(
            &scan_root,
            "a-first",
            "---\nname: a-first\ndescription: first fixture\n---\nbody\n",
        );
        let second_path = write_budget_skill(
            &scan_root,
            "b-second",
            "---\nname: b-second\ndescription: second fixture\n---\nbody\n",
        );
        let combined_lengths = std::fs::metadata(&first_path)
            .expect("first metadata")
            .len()
            + std::fs::metadata(&second_path)
                .expect("second metadata")
                .len();
        let mut limits = test_scan_limits();
        limits.max_total_skill_bytes = combined_lengths - 1;

        let report =
            scan_agent_with_limits(&adapter, &ctx, limits).expect("bounded scan returns a report");
        let canonical_root = scan_root.canonicalize().expect("canonical scan root");

        assert!(!report.instances.is_empty());
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ScanIssueKind::BudgetExceeded));
        assert!(report.stats.budget_exhausted);
        assert!(report.partial_roots.contains(&canonical_root));
        assert!(!report.scanned_roots.contains(&canonical_root));

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn entry_budget_marks_root_partial_and_records_budget_issue() {
        let (temp_root, scan_root, ctx, adapter) = budget_test_fixture("entries");
        for index in 0..9 {
            std::fs::create_dir_all(scan_root.join(format!("child-{index:02}")))
                .expect("create child directory");
        }

        let report = scan_agent_with_limits(&adapter, &ctx, test_scan_limits())
            .expect("bounded scan returns a report");
        let canonical_root = scan_root.canonicalize().expect("canonical scan root");

        assert_eq!(report.stats.entries_seen, 8);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ScanIssueKind::BudgetExceeded));
        assert!(report.partial_roots.contains(&canonical_root));

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn complete_root_remains_eligible_for_missing_sweep() {
        let (temp_root, scan_root, ctx, adapter) = budget_test_fixture("complete");
        write_budget_skill(
            &scan_root,
            "complete",
            "---\nname: complete\ndescription: complete fixture\n---\nbody\n",
        );

        let report = scan_agent_with_limits(&adapter, &ctx, test_scan_limits())
            .expect("bounded scan returns a report");
        let canonical_root = scan_root.canonicalize().expect("canonical scan root");

        assert!(report.issues.is_empty());
        assert!(report.partial_roots.is_empty());
        assert_eq!(report.scanned_roots, vec![canonical_root]);

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn scans_extra_root_for_skill_files() {
        let ctx = AdapterContext {
            user_home: fixture_path("fixtures/claude-code/empty-home"),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: fixture_path("fixtures/claude-code/personal"),
                source: RootSource::Extra,
            }],
        };

        let report = scan_agent(&ClaudeCodeAdapter, &ctx).expect("scan succeeds");

        assert_eq!(report.instances.len(), 1);
        assert_eq!(report.instances[0].name, "summarize-changes");
        assert_eq!(report.instances[0].scope, Scope::AgentGlobal);
    }

    #[test]
    fn claude_skill_overrides_do_not_disable_other_agents() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-claude-only-overrides-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let claude_skill_dir = home.join(".claude/skills/same-name");
        let codex_skill_dir = home.join(".agents/skills/same-name");
        let opencode_skill_dir = home.join(".config/opencode/skills/same-name");
        std::fs::create_dir_all(&claude_skill_dir).expect("create Claude skill dir");
        std::fs::create_dir_all(&codex_skill_dir).expect("create Codex skill dir");
        std::fs::create_dir_all(&opencode_skill_dir).expect("create opencode skill dir");
        std::fs::write(
            home.join(".claude/settings.json"),
            "{\n  \"skillOverrides\": {\n    \"same-name\": \"off\"\n  }\n}\n",
        )
        .expect("write Claude settings");
        std::fs::write(
            claude_skill_dir.join("SKILL.md"),
            "---\nname: same-name\ndescription: Claude skill\n---\nBody.\n",
        )
        .expect("write Claude skill");
        std::fs::write(
            codex_skill_dir.join("SKILL.md"),
            "---\nname: same-name\ndescription: Codex skill\n---\nBody.\n",
        )
        .expect("write Codex skill");
        std::fs::write(
            opencode_skill_dir.join("SKILL.md"),
            "---\nname: same-name\ndescription: opencode skill\n---\nBody.\n",
        )
        .expect("write opencode skill");

        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let claude = scan_agent(&ClaudeCodeAdapter, &ctx).expect("Claude scan succeeds");
        let codex = scan_agent(&CodexAdapter, &ctx).expect("Codex scan succeeds");
        let opencode = scan_agent(&OpencodeAdapter, &ctx).expect("opencode scan succeeds");

        assert_eq!(claude.instances.len(), 1);
        assert_eq!(claude.instances[0].name, "same-name");
        assert_eq!(claude.instances[0].state, SkillState::Disabled);
        assert!(!claude.instances[0].enabled);

        assert_eq!(codex.instances.len(), 1);
        assert_eq!(codex.instances[0].name, "same-name");
        assert_eq!(codex.instances[0].state, SkillState::Loaded);
        assert!(codex.instances[0].enabled);

        assert_eq!(opencode.instances.len(), 3);
        assert!(opencode
            .instances
            .iter()
            .all(|skill| skill.name == "same-name"
                && skill.state == SkillState::Loaded
                && skill.enabled));

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn hermes_and_openclaw_config_overrides_disable_scanned_skills() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-hermes-openclaw-overrides-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let hermes_skill_dir = home.join(".hermes/skills/research-brief");
        let openclaw_skill_dir = home.join(".openclaw/skills/visible-name");
        std::fs::create_dir_all(&hermes_skill_dir).expect("create Hermes skill dir");
        std::fs::create_dir_all(&openclaw_skill_dir).expect("create OpenClaw skill dir");
        std::fs::write(
            home.join(".hermes/config.yaml"),
            "skills:\n  disabled:\n    - research-brief\n",
        )
        .expect("write Hermes config");
        std::fs::write(
            home.join(".openclaw/openclaw.json"),
            "{\n  skills: { entries: { \"routed-key\": { enabled: false } } },\n}\n",
        )
        .expect("write OpenClaw config");
        std::fs::write(
            hermes_skill_dir.join("SKILL.md"),
            "---\nname: research-brief\ndescription: Hermes skill\n---\nBody.\n",
        )
        .expect("write Hermes skill");
        std::fs::write(
            openclaw_skill_dir.join("SKILL.md"),
            "---\nname: visible-name\ndescription: OpenClaw skill\nmetadata:\n  openclaw:\n    skillKey: routed-key\n---\nBody.\n",
        )
        .expect("write OpenClaw skill");

        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let hermes = scan_agent(&HermesAdapter, &ctx).expect("Hermes scan succeeds");
        let openclaw = scan_agent(&OpenclawAdapter, &ctx).expect("OpenClaw scan succeeds");

        assert_eq!(hermes.instances.len(), 1);
        assert_eq!(hermes.instances[0].state, SkillState::Disabled);
        assert!(!hermes.instances[0].enabled);
        assert!(openclaw
            .instances
            .iter()
            .any(|skill| skill.name == "visible-name"
                && skill.state == SkillState::Disabled
                && !skill.enabled));

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    #[cfg(unix)]
    fn allows_symlink_target_inside_another_declared_root_with_same_scope() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-home-symlink-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let claude_skills_dir = home.join(".claude/skills");
        let real_skill_dir = home.join(".agents/skills/shared");
        std::fs::create_dir_all(&real_skill_dir).expect("create real skill dir");
        std::fs::create_dir_all(&claude_skills_dir).expect("create claude skills dir");
        std::fs::write(
            real_skill_dir.join("SKILL.md"),
            "---\nname: shared\ndescription: follows explicit compatibility-root symlinks\n---\nBody.",
        )
        .expect("write SKILL.md");
        std::os::unix::fs::symlink(&real_skill_dir, claude_skills_dir.join("shared"))
            .expect("create symlink");

        let ctx = AdapterContext {
            user_home: home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&ClaudeCodeAdapter, &ctx).expect("scan succeeds");
        let canonical_skill_path = real_skill_dir
            .join("SKILL.md")
            .canonicalize()
            .expect("canonicalize shared skill");

        assert_eq!(report.instances.len(), 1);
        assert_eq!(report.instances[0].agent, AgentId::ClaudeCode);
        assert_eq!(report.instances[0].name, "shared");
        assert_eq!(report.instances[0].path, canonical_skill_path);
        assert_eq!(
            report.instances[0].display_path,
            claude_skills_dir.join("shared").join("SKILL.md"),
            "display_path should show the original symlink location"
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    #[cfg(unix)]
    fn rejects_symlink_target_outside_declared_adapter_roots() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-undeclared-home-symlink-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let claude_skills_dir = home.join(".claude/skills");
        let private_skill_dir = home.join("private-skill");
        std::fs::create_dir_all(&claude_skills_dir).expect("create Claude skills dir");
        std::fs::create_dir_all(&private_skill_dir).expect("create private skill dir");
        std::fs::write(
            private_skill_dir.join("SKILL.md"),
            "---\nname: private-skill\ndescription: undeclared private skill\n---\nBody.",
        )
        .expect("write private SKILL.md");
        std::os::unix::fs::symlink(&private_skill_dir, claude_skills_dir.join("linked"))
            .expect("create symlink");

        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&ClaudeCodeAdapter, &ctx).expect("scan succeeds");
        let canonical_private_dir = private_skill_dir
            .canonicalize()
            .expect("canonicalize private skill dir");

        assert!(
            report
                .instances
                .iter()
                .all(|instance| !instance.path.starts_with(&canonical_private_dir)),
            "scanner must reject targets outside every declared adapter root"
        );
        assert!(report
            .scanned_roots
            .iter()
            .all(|root| !root.starts_with(&canonical_private_dir)));
        assert!(report
            .partial_roots
            .iter()
            .all(|root| !root.starts_with(&canonical_private_dir)));

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn unavailable_root_is_reported_and_other_roots_continue() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-unavailable-root-{}",
            std::process::id()
        ));
        let unavailable_root = temp_root.join("not-a-directory");
        let valid_root = temp_root.join("valid-skills");
        let valid_skill_dir = valid_root.join("valid-review");
        std::fs::create_dir_all(&valid_skill_dir).expect("create valid skill dir");
        std::fs::write(&unavailable_root, "ordinary file").expect("write ordinary file root");
        std::fs::write(
            valid_skill_dir.join("SKILL.md"),
            "---\nname: valid-review\ndescription: valid Claude fixture\n---\nBody.",
        )
        .expect("write valid SKILL.md");
        let adapter = TestAdapter {
            roots: vec![
                AdapterRoot {
                    scope: Scope::AgentGlobal,
                    path: unavailable_root.clone(),
                    source: RootSource::Extra,
                },
                AdapterRoot {
                    scope: Scope::AgentGlobal,
                    path: valid_root,
                    source: RootSource::Extra,
                },
            ],
            link_target_roots: Vec::new(),
        };
        let ctx = AdapterContext {
            user_home: temp_root.join("home"),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&adapter, &ctx).expect("scan degrades unavailable root");

        assert_eq!(report.instances.len(), 1);
        assert_eq!(report.instances[0].name, "valid-review");
        assert!(report.skipped_roots.contains(&unavailable_root));
        assert_eq!(
            report
                .issues
                .iter()
                .filter(|issue| {
                    issue.path == unavailable_root && issue.kind == ScanIssueKind::RootUnavailable
                })
                .count(),
            1
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn declared_root_inspection_error_is_reported_and_other_roots_continue() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-root-inspection-error-{}",
            std::process::id()
        ));
        let uninspectable_root = temp_root.join("uninspectable");
        let uninspectable_skill_dir = uninspectable_root.join("must-not-scan");
        let valid_root = temp_root.join("valid-skills");
        let valid_skill_dir = valid_root.join("valid-review");
        std::fs::create_dir_all(&uninspectable_skill_dir).expect("create uninspectable skill dir");
        std::fs::create_dir_all(&valid_skill_dir).expect("create valid skill dir");
        std::fs::write(
            uninspectable_skill_dir.join("SKILL.md"),
            "---\nname: must-not-scan\ndescription: unavailable declaration\n---\nBody.",
        )
        .expect("write uninspectable SKILL.md");
        std::fs::write(
            valid_skill_dir.join("SKILL.md"),
            "---\nname: valid-review\ndescription: valid Claude fixture\n---\nBody.",
        )
        .expect("write valid SKILL.md");
        let adapter = TestAdapter {
            roots: vec![
                AdapterRoot {
                    scope: Scope::AgentGlobal,
                    path: uninspectable_root.clone(),
                    source: RootSource::Extra,
                },
                AdapterRoot {
                    scope: Scope::AgentGlobal,
                    path: valid_root.clone(),
                    source: RootSource::Extra,
                },
            ],
            link_target_roots: Vec::new(),
        };
        let ctx = AdapterContext {
            user_home: temp_root.join("home"),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent_with_symlink_inspector(&adapter, &ctx, |path| {
            if path == uninspectable_root {
                Err(std::io::Error::new(
                    std::io::ErrorKind::PermissionDenied,
                    "injected metadata denial",
                ))
            } else {
                Ok(false)
            }
        })
        .expect("scan degrades root inspection error");

        assert_eq!(report.instances.len(), 1);
        assert_eq!(report.instances[0].name, "valid-review");
        assert_eq!(report.skipped_roots, vec![uninspectable_root.clone()]);
        assert_eq!(
            report
                .issues
                .iter()
                .filter(|issue| {
                    issue.path == uninspectable_root
                        && issue.kind == ScanIssueKind::RootUnavailable
                        && issue.detail.contains("injected metadata denial")
                })
                .count(),
            1
        );
        assert_eq!(
            report.scanned_roots,
            vec![valid_root.canonicalize().expect("canonicalize valid root")]
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    #[cfg(unix)]
    fn rejects_extra_root_symlinks_outside_scan_root() {
        let temp_root =
            std::env::temp_dir().join(format!("skills-copilot-symlink-{}", std::process::id()));
        let skills_dir = temp_root.join("skills");
        let real_skill_dir = temp_root.join("real-skill-outside-scan-root");
        std::fs::create_dir_all(&real_skill_dir).expect("create real skill dir");
        std::fs::create_dir_all(&skills_dir).expect("create skills dir");
        std::fs::write(
            real_skill_dir.join("SKILL.md"),
            "---\nname: symlink-test\ndescription: follows symlinks\n---\nBody.",
        )
        .expect("write SKILL.md");
        std::os::unix::fs::symlink(&real_skill_dir, skills_dir.join("symlink-test"))
            .expect("create symlink");

        let ctx = AdapterContext {
            user_home: temp_root.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: skills_dir.clone(),
                source: RootSource::Extra,
            }],
        };

        let report = scan_agent(&ClaudeCodeAdapter, &ctx).expect("scan succeeds");

        assert!(
            report.instances.is_empty(),
            "scanner must reject SKILL.md files whose canonical path escapes the scanned root"
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    #[cfg(unix)]
    fn rejects_builtin_root_symlink_that_escapes_user_home() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-root-symlink-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let claude_dir = home.join(".claude");
        let outside_skills_dir = temp_root.join("outside-skills");
        let outside_skill_dir = outside_skills_dir.join("escaped");
        std::fs::create_dir_all(&claude_dir).expect("create claude dir");
        std::fs::create_dir_all(&outside_skill_dir).expect("create outside skill dir");
        std::fs::write(
            outside_skill_dir.join("SKILL.md"),
            "---\nname: escaped\ndescription: outside root\n---\nBody.",
        )
        .expect("write outside SKILL.md");
        std::os::unix::fs::symlink(&outside_skills_dir, claude_dir.join("skills"))
            .expect("create root symlink");

        let ctx = AdapterContext {
            user_home: home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&ClaudeCodeAdapter, &ctx).expect("scan succeeds");

        assert!(
            report.instances.is_empty(),
            "builtin user-home root symlink must not let the scanner escape user_home"
        );
        assert_eq!(report.skipped_roots, vec![home.join(".claude/skills")]);

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    #[cfg(unix)]
    fn allows_builtin_leaf_symlink_when_same_scope_target_root_is_explicit() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-explicit-root-symlink-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let claude_dir = home.join(".claude");
        let declared_path = claude_dir.join("skills");
        let explicit_target = temp_root.join("explicit-target");
        let skill_dir = explicit_target.join("authorized");
        std::fs::create_dir_all(&claude_dir).expect("create Claude dir");
        std::fs::create_dir_all(&skill_dir).expect("create explicitly allowed skill dir");
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: authorized\ndescription: explicit same-scope target\n---\nBody.",
        )
        .expect("write authorized SKILL.md");
        std::os::unix::fs::symlink(&explicit_target, &declared_path)
            .expect("create built-in root symlink");
        let adapter = TestAdapter {
            roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: declared_path.clone(),
                source: RootSource::UserHome,
            }],
            link_target_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: explicit_target.clone(),
                source: RootSource::Extra,
            }],
        };
        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&adapter, &ctx).expect("scan succeeds");
        let canonical_target = explicit_target
            .canonicalize()
            .expect("canonicalize explicit target");
        let canonical_skill = skill_dir
            .join("SKILL.md")
            .canonicalize()
            .expect("canonicalize authorized skill");

        assert_eq!(report.instances.len(), 1);
        assert_eq!(report.instances[0].path, canonical_skill);
        assert_eq!(
            report.instances[0].display_path,
            declared_path.join("authorized/SKILL.md")
        );
        assert!(!report.skipped_roots.contains(&declared_path));
        assert_eq!(report.scanned_roots, vec![canonical_target]);

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    #[cfg(unix)]
    fn rejects_builtin_leaf_symlink_when_explicit_target_has_different_scope() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-cross-scope-root-symlink-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let claude_dir = home.join(".claude");
        let declared_path = claude_dir.join("skills");
        let explicit_target = temp_root.join("explicit-target");
        let skill_dir = explicit_target.join("cross-scope");
        std::fs::create_dir_all(&claude_dir).expect("create Claude dir");
        std::fs::create_dir_all(&skill_dir).expect("create cross-scope skill dir");
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: cross-scope\ndescription: differently scoped target\n---\nBody.",
        )
        .expect("write cross-scope SKILL.md");
        std::os::unix::fs::symlink(&explicit_target, &declared_path)
            .expect("create built-in root symlink");
        let adapter = TestAdapter {
            roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: declared_path.clone(),
                source: RootSource::UserHome,
            }],
            link_target_roots: vec![AdapterRoot {
                scope: Scope::AgentProject,
                path: explicit_target,
                source: RootSource::Extra,
            }],
        };
        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };
        let mut classifications = [true, false, true].into_iter();

        let report = scan_agent_with_symlink_inspector(&adapter, &ctx, |_path| {
            Ok::<bool, std::io::Error>(
                classifications
                    .next()
                    .expect("scanner must not inspect one declared leaf more than three times"),
            )
        })
        .expect("scan succeeds");

        assert_eq!(
            classifications.next(),
            Some(false),
            "each declared root leaf must be inspected exactly once"
        );
        assert!(report.instances.is_empty());
        assert_eq!(report.skipped_roots, vec![declared_path.clone()]);
        assert!(report.scanned_roots.is_empty());
        assert!(report.partial_roots.is_empty());
        assert_eq!(
            report
                .issues
                .iter()
                .filter(|issue| {
                    issue.path == declared_path && issue.kind == ScanIssueKind::RootOutsideAllowlist
                })
                .count(),
            1
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    #[cfg(unix)]
    fn rejects_project_root_symlinks_outside_project_root() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-project-symlink-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let project = temp_root.join("project");
        let project_skills_dir = project.join(".claude/skills");
        let home_skill_dir = home.join(".agents/skills/home-only");
        std::fs::create_dir_all(&project_skills_dir).expect("create project skills dir");
        std::fs::create_dir_all(&home_skill_dir).expect("create home skill dir");
        std::fs::write(
            home_skill_dir.join("SKILL.md"),
            "---\nname: home-only\ndescription: outside project\n---\nBody.",
        )
        .expect("write home SKILL.md");
        std::os::unix::fs::symlink(&home_skill_dir, project_skills_dir.join("home-only"))
            .expect("create project symlink");

        let ctx = AdapterContext {
            user_home: home,
            project_root: Some(project),
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&ClaudeCodeAdapter, &ctx).expect("scan succeeds");

        assert!(
            report.instances.is_empty(),
            "project roots must not scan symlink targets outside the project root"
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn opencode_permission_skill_deny_marks_exact_skill_disabled() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-opencode-permission-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let skill_dir = home.join(".config/opencode/skills/global-review");
        std::fs::create_dir_all(&skill_dir).expect("create opencode skill dir");
        std::fs::create_dir_all(home.join(".config/opencode")).expect("create opencode config dir");
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: global-review\ndescription: opencode disabled fixture\n---\nBody.",
        )
        .expect("write opencode SKILL.md");
        std::fs::write(
            home.join(".config/opencode/opencode.json"),
            r#"{
              // JSONC comments are accepted for readback.
              "permission": {
                "skill": {
                  "*": "allow",
                  "global-review": "deny"
                }
              }
            }"#,
        )
        .expect("write opencode config");

        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&OpencodeAdapter, &ctx).expect("scan succeeds");

        assert_eq!(report.instances.len(), 1);
        assert_eq!(report.instances[0].name, "global-review");
        assert!(!report.instances[0].enabled);
        assert_eq!(report.instances[0].state, SkillState::Disabled);

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn opencode_scans_claude_and_agent_compatible_roots_with_opencode_permissions() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-opencode-compat-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let claude_skill_dir = home.join(".claude/skills/claude-compatible");
        let agent_skill_dir = home.join(".agents/skills/agent-compatible");
        std::fs::create_dir_all(&claude_skill_dir).expect("create Claude-compatible skill dir");
        std::fs::create_dir_all(&agent_skill_dir).expect("create agent-compatible skill dir");
        std::fs::create_dir_all(home.join(".config/opencode")).expect("create opencode config dir");
        std::fs::write(
            claude_skill_dir.join("SKILL.md"),
            "---\nname: claude-compatible\ndescription: opencode Claude compatibility fixture\n---\nBody.",
        )
        .expect("write Claude-compatible SKILL.md");
        std::fs::write(
            agent_skill_dir.join("SKILL.md"),
            "---\nname: agent-compatible\ndescription: opencode agent compatibility fixture\n---\nBody.",
        )
        .expect("write agent-compatible SKILL.md");
        std::fs::write(
            home.join(".claude/settings.json"),
            "{\n  \"skillOverrides\": {\n    \"claude-compatible\": \"off\"\n  }\n}\n",
        )
        .expect("write Claude settings");
        std::fs::write(
            home.join(".config/opencode/opencode.json"),
            r#"{"permission":{"skill":{"agent-compatible":"deny"}}}"#,
        )
        .expect("write opencode config");

        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&OpencodeAdapter, &ctx).expect("scan succeeds");
        let by_name: HashMap<_, _> = report
            .instances
            .iter()
            .map(|skill| (skill.name.as_str(), (skill.state.clone(), skill.enabled)))
            .collect();

        assert_eq!(report.instances.len(), 2);
        assert_eq!(
            by_name.get("claude-compatible"),
            Some(&(SkillState::Loaded, true)),
            "opencode compatibility roots must not inherit Claude skillOverrides"
        );
        assert_eq!(
            by_name.get("agent-compatible"),
            Some(&(SkillState::Disabled, false)),
            "opencode compatibility roots must honor opencode permission.skill"
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn opencode_scans_configured_local_paths_without_fetching_urls() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-opencode-configured-scan-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let configured_root = temp_root.join("custom-skills");
        let skill_dir = configured_root.join("custom-review");
        std::fs::create_dir_all(&skill_dir).expect("create configured skill dir");
        std::fs::create_dir_all(home.join(".config/opencode")).expect("create opencode config dir");
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: custom-review\ndescription: opencode configured path fixture\n---\nBody.",
        )
        .expect("write configured skill");
        let config = serde_json::json!({
            "skills": {
                "paths": [
                    configured_root.to_string_lossy().to_string(),
                    configured_root.to_string_lossy().to_string()
                ],
                "urls": ["https://example.invalid/.well-known/skills/"]
            },
            "permission": {
                "skill": {
                    "custom-review": "deny"
                }
            }
        });
        std::fs::write(
            home.join(".config/opencode/opencode.json"),
            serde_json::to_string(&config).expect("serialize opencode config"),
        )
        .expect("write opencode config");

        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&OpencodeAdapter, &ctx).expect("scan succeeds");

        assert_eq!(report.instances.len(), 1);
        assert_eq!(report.instances[0].name, "custom-review");
        assert_eq!(report.instances[0].state, SkillState::Disabled);
        assert!(!report.instances[0].enabled);
        assert_eq!(
            report.scanned_roots.len(),
            1,
            "duplicate configured paths should canonicalize and dedupe before scanning"
        );
        assert!(report.scanned_roots[0].ends_with("custom-skills"));
        assert!(
            report
                .skipped_roots
                .iter()
                .all(|root| !root.to_string_lossy().contains("https://example.invalid")),
            "skills.urls must not become skipped filesystem roots or trigger fetch attempts"
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn opencode_keeps_native_and_compatible_roots_for_conflict_analysis() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-opencode-dedup-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let native_skill_dir = home.join(".config/opencode/skills/shared-review");
        let claude_skill_dir = home.join(".claude/skills/shared-review");
        let agents_skill_dir = home.join(".agents/skills/shared-review");
        std::fs::create_dir_all(&native_skill_dir).expect("create native skill dir");
        std::fs::create_dir_all(&claude_skill_dir).expect("create Claude-compatible skill dir");
        std::fs::create_dir_all(&agents_skill_dir).expect("create agents-compatible skill dir");
        for dir in [&native_skill_dir, &claude_skill_dir, &agents_skill_dir] {
            std::fs::write(
                dir.join("SKILL.md"),
                "---\nname: shared-review\ndescription: duplicate opencode fixture\n---\nBody.",
            )
            .expect("write duplicate opencode skill");
        }

        let ctx = AdapterContext {
            user_home: home.clone(),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&OpencodeAdapter, &ctx).expect("scan succeeds");

        assert_eq!(report.instances.len(), 3);
        assert!(report.instances.iter().any(|skill| skill.path
            == native_skill_dir
                .join("SKILL.md")
                .canonicalize()
                .expect("canonical native path")));
        assert!(report
            .instances
            .iter()
            .all(|skill| skill.name == "shared-review"));

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn pi_scans_native_directory_skills_and_ignores_plain_markdown() {
        let temp_root =
            std::env::temp_dir().join(format!("skills-copilot-pi-scan-{}", std::process::id()));
        let home = temp_root.join("home");
        let root = home.join(".pi/agent/skills");
        let dir_skill = root.join("global-pdf");
        std::fs::create_dir_all(&dir_skill).expect("create pi dir skill");
        std::fs::write(
            dir_skill.join("SKILL.md"),
            "---\nname: global-pdf\ndescription: Pi directory fixture\n---\nBody.",
        )
        .expect("write pi dir skill");
        std::fs::write(
            root.join("root-note.md"),
            "---\nname: root-note\ndescription: Pi root markdown fixture\n---\nBody.",
        )
        .expect("write pi root markdown");
        let nested_reference_dir = dir_skill.join("references");
        std::fs::create_dir_all(&nested_reference_dir).expect("create nested reference dir");
        std::fs::write(
            nested_reference_dir.join("implementation.md"),
            "---\nname: implementation\ndescription: This markdown is support material, not a Pi root skill.\n---\nBody.",
        )
        .expect("write nested reference markdown");
        std::fs::write(
            nested_reference_dir.join("SKILL.md"),
            "---\nname: implementation\ndescription: This nested SKILL.md is support material, not a Pi root skill.\n---\nBody.",
        )
        .expect("write nested reference SKILL.md");
        std::fs::write(
            root.join("SKILL.md"),
            "---\nname: root-noise\ndescription: Historical catalog noise, not a Pi directory skill.\n---\nBody.",
        )
        .expect("write root SKILL.md noise");

        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&PiAdapter, &ctx).expect("scan succeeds");
        let names: HashSet<_> = report
            .instances
            .iter()
            .map(|skill| skill.name.as_str())
            .collect();

        assert_eq!(report.instances.len(), 1);
        assert!(names.contains("global-pdf"));
        assert!(!names.contains("root-note"));
        assert!(!names.contains("implementation"));
        assert!(!names.contains("root-noise"));

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn pi_scans_agent_compatibility_roots_without_markdown_noise() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-pi-compat-scan-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let project = temp_root.join("project");
        let global_root = home.join(".agents/skills");
        let project_root = project.join(".agents/skills");
        let global_skill = global_root.join("pi-agent-global");
        let project_skill = project_root.join("pi-agent-project");
        std::fs::create_dir_all(&global_skill).expect("create global compat skill");
        std::fs::create_dir_all(&project_skill).expect("create project compat skill");
        std::fs::write(
            global_skill.join("SKILL.md"),
            "---\nname: pi-agent-global\ndescription: Pi global compatibility fixture\n---\nBody.",
        )
        .expect("write global compat skill");
        std::fs::write(
            project_skill.join("SKILL.md"),
            "---\nname: pi-agent-project\ndescription: Pi project compatibility fixture\n---\nBody.",
        )
        .expect("write project compat skill");
        std::fs::write(
            global_root.join("root-noise.md"),
            "---\nname: root-noise\ndescription: ignored compatibility markdown\n---\nBody.",
        )
        .expect("write root markdown");

        let ctx = AdapterContext {
            user_home: home,
            project_root: Some(project.clone()),
            project_cwd: Some(project),
            extra_roots: vec![],
        };

        let report = scan_agent(&PiAdapter, &ctx).expect("scan succeeds");
        let names: HashSet<_> = report
            .instances
            .iter()
            .map(|skill| skill.name.as_str())
            .collect();

        assert!(names.contains("pi-agent-global"));
        assert!(names.contains("pi-agent-project"));
        assert!(!names.contains("root-noise"));
        assert!(report
            .scanned_roots
            .iter()
            .any(|root| root.ends_with(".agents/skills")));

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn openclaw_scans_documented_global_and_selected_home_workspace_roots() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-openclaw-scan-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let workspace = home.join(".openclaw/workspace");
        let managed_global_path =
            write_openclaw_skill(&home.join(".openclaw/skills"), "managed-global");
        write_openclaw_skill(&home.join(".agents/skills"), "managed-global");
        write_openclaw_skill(&home.join(".agents/skills"), "personal-shared");
        write_openclaw_skill(&workspace.join("skills"), "workspace-local");
        write_openclaw_skill(&workspace.join(".agents/skills"), "workspace-agents");

        let ctx = AdapterContext {
            user_home: home,
            project_root: Some(workspace),
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&OpenclawAdapter, &ctx).expect("scan succeeds");
        let by_name: HashMap<_, _> = report
            .instances
            .iter()
            .map(|skill| (skill.name.as_str(), skill.scope))
            .collect();

        assert_eq!(report.instances.len(), 5);
        assert_eq!(by_name.get("managed-global"), Some(&Scope::AgentGlobal));
        assert_eq!(by_name.get("personal-shared"), Some(&Scope::AgentGlobal));
        assert_eq!(by_name.get("workspace-local"), Some(&Scope::AgentProject));
        assert_eq!(by_name.get("workspace-agents"), Some(&Scope::AgentProject));
        assert_eq!(
            report
                .instances
                .iter()
                .find(|skill| skill.name == "managed-global")
                .expect("managed global")
                .path,
            managed_global_path,
            "OpenClaw native global root wins over shared compatibility root"
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn openclaw_scans_home_workspace_roots_when_selected_project_is_inside_workspace() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-openclaw-nested-workspace-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let workspace = home.join(".openclaw/workspace");
        let nested_project = workspace.join("repo");
        let workspace_skill = write_openclaw_skill(&workspace.join("skills"), "workspace-local");
        let workspace_agents_skill =
            write_openclaw_skill(&workspace.join(".agents/skills"), "workspace-agents");

        let ctx = AdapterContext {
            user_home: home,
            project_root: Some(nested_project.clone()),
            project_cwd: Some(nested_project.join("nested")),
            extra_roots: vec![],
        };

        let report = scan_agent(&OpenclawAdapter, &ctx).expect("scan succeeds");
        let by_name: HashMap<_, _> = report
            .instances
            .iter()
            .map(|skill| (skill.name.as_str(), skill))
            .collect();

        assert_eq!(report.instances.len(), 2);
        assert_eq!(
            by_name.get("workspace-local").map(|skill| &skill.path),
            Some(&workspace_skill)
        );
        assert_eq!(
            by_name.get("workspace-agents").map(|skill| &skill.path),
            Some(&workspace_agents_skill)
        );
        assert!(report.instances.iter().all(|skill| {
            skill.scope == Scope::AgentProject && skill.project_root == Some(workspace.clone())
        }));

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn openclaw_does_not_scan_arbitrary_project_skill_roots() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-openclaw-project-scope-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let project = temp_root.join("repo");
        write_openclaw_skill(&project.join("skills"), "not-workspace-skills");
        write_openclaw_skill(&project.join(".agents/skills"), "not-workspace-agents");

        let ctx = AdapterContext {
            user_home: home,
            project_root: Some(project),
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&OpenclawAdapter, &ctx).expect("scan succeeds");

        assert!(
            report.instances.is_empty(),
            "OpenClaw must not infer arbitrary repo skills or .agents roots as workspace roots"
        );

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn hermes_scans_active_home_and_explicit_external_dirs_only() {
        let temp_root =
            std::env::temp_dir().join(format!("skills-copilot-hermes-scan-{}", std::process::id()));
        let home = temp_root.join("home");
        let hermes_skill_path = write_hermes_skill(
            &home.join(".hermes/skills/nested/research"),
            "hermes-research",
        );
        let external_skill_path = write_hermes_skill(
            &temp_root.join("configured-external/analysis"),
            "external-analysis",
        );
        write_hermes_skill(
            &temp_root.join("repo/skills/project-skill"),
            "project-skill",
        );
        std::fs::write(
            home.join(".hermes/config.yaml"),
            format!(
                "skills:\n  external_dirs:\n    - {}\n",
                temp_root.join("configured-external").display()
            ),
        )
        .expect("write Hermes config");
        std::fs::create_dir_all(home.join(".hermes/cron")).expect("create hermes cron dir");
        std::fs::create_dir_all(home.join(".hermes/logs")).expect("create hermes logs dir");
        std::fs::write(home.join(".hermes/.env"), "HERMES_TOKEN=<redacted>\n")
            .expect("write redacted env fixture");
        std::fs::write(
            home.join(".hermes/auth.json"),
            "{\"token\":\"<redacted>\"}\n",
        )
        .expect("write redacted auth fixture");
        std::fs::write(
            home.join(".hermes/cron/jobs.json"),
            "{\"jobs\":[{\"id\":\"not-a-skill\",\"enabled\":false}]}\n",
        )
        .expect("write cron fixture");
        std::fs::write(home.join(".hermes/logs/session.log"), "<redacted>\n")
            .expect("write log fixture");

        let ctx = AdapterContext {
            user_home: home,
            project_root: Some(temp_root.join("repo")),
            project_cwd: Some(temp_root.join("repo/nested")),
            extra_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: temp_root.join("unverified"),
                source: RootSource::Extra,
            }],
        };

        let report = scan_agent(&HermesAdapter, &ctx).expect("scan succeeds");

        assert_eq!(report.instances.len(), 2);
        assert!(report.instances.iter().any(|instance| {
            instance.agent == AgentId::Hermes
                && instance.scope == Scope::AgentGlobal
                && instance.name == "hermes-research"
                && instance.path == hermes_skill_path
        }));
        assert!(report.instances.iter().any(|instance| {
            instance.agent == AgentId::Hermes
                && instance.scope == Scope::AgentGlobal
                && instance.name == "external-analysis"
                && instance.path == external_skill_path
        }));
        assert!(report
            .instances
            .iter()
            .all(|instance| instance.name != "project-skill"));

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    fn fixture_path(relative: &str) -> PathBuf {
        PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("../..")
            .join(relative)
    }

    fn write_openclaw_skill(root: &Path, name: &str) -> PathBuf {
        let skill_dir = root.join(name);
        std::fs::create_dir_all(&skill_dir).expect("create OpenClaw skill dir");
        let skill_path = skill_dir.join("SKILL.md");
        std::fs::write(
            &skill_path,
            format!("---\nname: {name}\ndescription: {name} fixture\n---\nbody"),
        )
        .expect("write OpenClaw skill");
        skill_path.canonicalize().expect("canonicalize skill path")
    }

    fn write_hermes_skill(root: &Path, name: &str) -> PathBuf {
        std::fs::create_dir_all(root).expect("create Hermes skill dir");
        let skill_path = root.join("SKILL.md");
        std::fs::write(
            &skill_path,
            format!("---\nname: {name}\ndescription: {name} fixture\n---\nbody"),
        )
        .expect("write Hermes skill");
        skill_path.canonicalize().expect("canonicalize skill path")
    }
}
