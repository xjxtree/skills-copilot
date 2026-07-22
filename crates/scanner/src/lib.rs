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
    DanglingSymlink,
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
    /// Complete roots with their original adapter scope preserved. Catalog
    /// missing-sweep must use this field instead of the path-only compatibility
    /// field above.
    pub scoped_scanned_roots: Vec<ScopedScanRoot>,
    /// Canonical paths of roots whose traversal encountered a filesystem error.
    pub partial_roots: Vec<PathBuf>,
    /// Partial roots with their original adapter scope preserved.
    pub scoped_partial_roots: Vec<ScopedScanRoot>,
    pub issues: Vec<ScanIssue>,
    /// Immutable declared/canonical aliases captured while roots were
    /// resolved. Service diagnostics use this snapshot and never re-resolve a
    /// path after the scan.
    pub root_aliases: Vec<ScanRootAlias>,
    pub stats: ScanStats,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScopedScanRoot {
    pub scope: Scope,
    pub path: PathBuf,
}

#[derive(Debug, Clone, Eq, PartialEq)]
pub struct ScanRootAlias {
    pub declared: PathBuf,
    pub canonical: PathBuf,
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
    is_file: bool,
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
    inspect_declared_symlink: F,
) -> Result<ScanReport, ScannerError>
where
    F: FnMut(&Path) -> std::io::Result<bool>,
{
    scan_agent_with_limits_and_inspectors(adapter, ctx, limits, inspect_declared_symlink, |path| {
        fs::metadata(path).map(|metadata| metadata.len())
    })
}

fn scan_agent_with_limits_and_inspectors<F, G>(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
    limits: ScanLimits,
    mut inspect_declared_symlink: F,
    mut inspect_skill_len: G,
) -> Result<ScanReport, ScannerError>
where
    F: FnMut(&Path) -> std::io::Result<bool>,
    G: FnMut(&Path) -> std::io::Result<u64>,
{
    let mut report = ScanReport::default();
    let mut budget = ScanBudget::default();
    let roots = adapter.roots(ctx);
    let overrides = SkillConfigOverrides::preload(adapter, ctx);
    let mut resolved_roots = Vec::new();

    for declared in roots {
        let declared_is_symlink = match inspect_declared_symlink(&declared.path) {
            Ok(declared_is_symlink) => declared_is_symlink,
            Err(error)
                if error.kind() == std::io::ErrorKind::NotFound
                    && root_source_is_optional(&declared.source) =>
            {
                continue;
            }
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
        let (canonical, is_file) = match resolve_scan_root(&declared.path) {
            Ok(resolved) => resolved,
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
        if is_file && !adapter.is_skill_file(&canonical) {
            report.skipped_roots.push(declared.path.clone());
            report.issues.push(ScanIssue {
                path: declared.path,
                kind: ScanIssueKind::RootUnavailable,
                detail: "root file is not a supported skill file for this adapter".to_string(),
            });
            continue;
        }
        report.root_aliases.push(ScanRootAlias {
            declared: declared.path.clone(),
            canonical: canonical.clone(),
        });
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
            is_file,
        });
    }

    let mut allowed_roots = Vec::new();
    for declared in adapter.link_target_roots(ctx) {
        let Ok(canonical) = resolve_directory_root(&declared.path) else {
            continue;
        };
        if !is_allowed_canonical_root(adapter.id(), ctx, &declared, &canonical) {
            continue;
        }
        report.root_aliases.push(ScanRootAlias {
            declared: declared.path,
            canonical: canonical.clone(),
        });
        allowed_roots.push(ResolvedAllowedRoot {
            scope: declared.scope,
            canonical,
        });
    }

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
        let status = visit_root(
            adapter,
            ctx,
            &root,
            &allowed_roots,
            &overrides,
            &limits,
            &mut budget,
            &mut inspect_skill_len,
            &mut report,
        );
        let scoped_root = ScopedScanRoot {
            scope: root.declared.scope,
            path: root.canonical.clone(),
        };
        match status {
            RootWalkStatus::Complete => {
                report.scanned_roots.push(root.canonical);
                report.scoped_scanned_roots.push(scoped_root);
            }
            RootWalkStatus::Partial => {
                report.partial_roots.push(root.canonical);
                report.scoped_partial_roots.push(scoped_root);
            }
        }
    }
    report.instances = dedup_instances(report.instances);
    if matches!(
        adapter.id(),
        AgentId::ClaudeCode | AgentId::Pi | AgentId::Openclaw
    ) {
        let mut effective_names = HashSet::new();
        report
            .instances
            .retain(|instance| effective_names.insert(instance.name.clone()));
    }
    let mut alias_keys = HashSet::new();
    report
        .root_aliases
        .retain(|alias| alias_keys.insert((alias.declared.clone(), alias.canonical.clone())));
    report.stats = budget.stats;
    Ok(report)
}

fn resolve_scan_root(path: &Path) -> Result<(PathBuf, bool), String> {
    let canonical = path
        .canonicalize()
        .map_err(|error| format!("failed to canonicalize root: {error}"))?;
    let metadata = fs::metadata(&canonical)
        .map_err(|error| format!("failed to inspect canonical root: {error}"))?;
    if !metadata.is_dir() && !metadata.is_file() {
        return Err("root is neither a directory nor a regular file".to_string());
    }
    Ok((canonical, metadata.is_file()))
}

fn resolve_directory_root(path: &Path) -> Result<PathBuf, String> {
    let (canonical, is_file) = resolve_scan_root(path)?;
    if is_file {
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

fn root_source_is_optional(source: &RootSource) -> bool {
    matches!(
        source,
        RootSource::UserHome
            | RootSource::Project
            | RootSource::Compatibility
            | RootSource::Admin
            | RootSource::System
    )
}

fn target_is_allowlisted(path: &Path, scope: Scope, roots: &[ResolvedAllowedRoot]) -> bool {
    roots
        .iter()
        .filter(|root| root.scope == scope)
        .any(|root| path.starts_with(&root.canonical))
}

fn unavailable_symlink_target_outside_allowlist(
    link: &Path,
    scope: Scope,
    roots: &[ResolvedAllowedRoot],
) -> Option<PathBuf> {
    let raw_target = fs::read_link(link).ok()?;
    let target = if raw_target.is_absolute() {
        raw_target
    } else {
        link.parent()?.join(raw_target)
    };
    let comparable_target = canonicalize_missing_path_for_comparison(&target)?;
    (!target_is_allowlisted(&comparable_target, scope, roots)).then_some(comparable_target)
}

fn canonicalize_missing_path_for_comparison(path: &Path) -> Option<PathBuf> {
    let normalized = normalize_path_lexically(path);
    let mut ancestor = normalized.clone();
    let mut suffix = Vec::new();
    loop {
        match ancestor.canonicalize() {
            Ok(mut canonical) => {
                for component in suffix.iter().rev() {
                    canonical.push(component);
                }
                return Some(canonical);
            }
            Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
                suffix.push(ancestor.file_name()?.to_os_string());
                if !ancestor.pop() {
                    return None;
                }
            }
            Err(_) => return None,
        }
    }
}

fn normalize_path_lexically(path: &Path) -> PathBuf {
    use std::path::Component;

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    normalized.push(component.as_os_str());
                }
            }
            Component::Prefix(_) | Component::RootDir | Component::Normal(_) => {
                normalized.push(component.as_os_str());
            }
        }
    }
    normalized
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
        RootSource::Compatibility
            if agent == AgentId::Openclaw && root.scope == Scope::AgentProject =>
        {
            openclaw_workspace_base_for_root_path(&root.path)
                .and_then(|workspace_root| workspace_root.canonicalize().ok())
                .is_some_and(|base| canonical_root.starts_with(base))
        }
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
    inspect_skill_len: &mut dyn FnMut(&Path) -> std::io::Result<u64>,
    report: &mut ScanReport,
) -> RootWalkStatus {
    if root.is_file {
        let relative = root
            .canonical
            .file_name()
            .map(PathBuf::from)
            .unwrap_or_else(|| root.canonical.clone());
        if !adapter.is_skill_file(&root.canonical)
            || !adapter.accepts_skill_path(&root.declared, &relative)
        {
            return RootWalkStatus::Complete;
        }
        budget.stats.entries_seen = budget.stats.entries_seen.saturating_add(1);
        let metadata_len = inspect_skill_len(&root.canonical);
        return match visit_skill_file(
            adapter,
            ctx,
            root,
            overrides,
            root.canonical.clone(),
            root.declared.path.clone(),
            metadata_len,
            limits,
            budget,
            report,
        ) {
            SkillVisitStatus::Complete => RootWalkStatus::Complete,
            SkillVisitStatus::BudgetExceeded => RootWalkStatus::Partial,
        };
    }
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
                        if let Some(unavailable_target) =
                            unavailable_symlink_target_outside_allowlist(
                                &path,
                                root.declared.scope,
                                allowed_roots,
                            )
                        {
                            report.issues.push(ScanIssue {
                                path,
                                kind: ScanIssueKind::DanglingSymlink,
                                detail: format!(
                                    "unavailable symlink target {} is outside declared roots with the same scope",
                                    unavailable_target.display()
                                ),
                            });
                            continue;
                        }
                        report.issues.push(ScanIssue {
                            path,
                            kind: ScanIssueKind::DanglingSymlink,
                            detail: format!(
                                "symlink target is unavailable; skipped this link without degrading the surrounding root: {error}"
                            ),
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
                    let relative = display_path
                        .strip_prefix(&root.declared.path)
                        .unwrap_or(display_path.as_path());
                    if !adapter.should_descend(&root.declared, relative) {
                        continue;
                    }
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
                } else if adapter.is_skill_file(&resolved) {
                    let relative = display_path
                        .strip_prefix(&root.declared.path)
                        .unwrap_or(display_path.as_path());
                    if !adapter.accepts_skill_path(&root.declared, relative) {
                        continue;
                    }
                    let metadata_len = inspect_skill_len(&resolved);
                    if visit_skill_file(
                        adapter,
                        ctx,
                        root,
                        overrides,
                        resolved,
                        display_path,
                        metadata_len,
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
                let relative = display_path
                    .strip_prefix(&root.declared.path)
                    .unwrap_or(display_path.as_path());
                if !adapter.should_descend(&root.declared, relative) {
                    continue;
                }
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
            if adapter.is_skill_file(&path) {
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
                let relative = display_path
                    .strip_prefix(&root.declared.path)
                    .unwrap_or(display_path.as_path());
                if !adapter.accepts_skill_path(&root.declared, relative) {
                    continue;
                }
                let metadata_len = inspect_skill_len(&canonical_path);
                if visit_skill_file(
                    adapter,
                    ctx,
                    root,
                    overrides,
                    canonical_path,
                    display_path,
                    metadata_len,
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
    metadata_len: std::io::Result<u64>,
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

    if let Err(error) = metadata_len {
        let detail = format!("failed to inspect skill file: {error}");
        report.issues.push(ScanIssue {
            path: canonical_path.clone(),
            kind: ScanIssueKind::FileUnreadable,
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

    let total_remaining = limits
        .max_total_skill_bytes
        .saturating_sub(budget.stats.bytes_read);
    if total_remaining == 0 {
        record_budget_exceeded(
            budget,
            report,
            canonical_path,
            format!(
                "total skill byte limit of {} bytes was reached",
                limits.max_total_skill_bytes
            ),
        );
        return SkillVisitStatus::BudgetExceeded;
    }

    let read = read_skill_content_bounded(&canonical_path, limits.max_skill_bytes, total_remaining);
    debug_assert!(read.bytes_read <= total_remaining);
    budget.stats.bytes_read = budget
        .stats
        .bytes_read
        .checked_add(read.bytes_read)
        .expect("bounded byte accounting cannot overflow");

    let mut instance = match read.outcome {
        Ok(BoundedSkillContent::Content(content)) => adapter
            .parse_content(&canonical_path, content)
            .unwrap_or_else(|err| {
                broken_instance(adapter, &root.declared, canonical_path.clone(), err.message)
            }),
        Ok(BoundedSkillContent::FileTooLarge) => {
            let detail = format!(
                "skill file exceeded the per-file limit of {} bytes while being read",
                limits.max_skill_bytes
            );
            report.issues.push(ScanIssue {
                path: canonical_path.clone(),
                kind: ScanIssueKind::FileTooLarge,
                detail: detail.clone(),
            });
            broken_instance(adapter, &root.declared, canonical_path.clone(), detail)
        }
        Ok(BoundedSkillContent::TotalBudgetExceeded) => {
            record_budget_exceeded(
                budget,
                report,
                canonical_path,
                format!(
                    "total skill byte limit of {} bytes was reached while reading a skill file",
                    limits.max_total_skill_bytes
                ),
            );
            return SkillVisitStatus::BudgetExceeded;
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
        canonical_path.clone(),
        overrides,
        &mut instance,
    );
    instance.display_path = normalized_display_path(
        instance.agent,
        &root.declared,
        &canonical_path,
        display_path,
    );
    finalize_discovered_instance(ctx, &root.declared, overrides, &mut instance);
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
        canonical_path.clone(),
        overrides,
        &mut instance,
    );
    instance.display_path = normalized_display_path(
        instance.agent,
        &root.declared,
        &canonical_path,
        display_path,
    );
    finalize_discovered_instance(ctx, &root.declared, overrides, &mut instance);
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

#[derive(Debug)]
enum BoundedSkillContent {
    Content(String),
    FileTooLarge,
    TotalBudgetExceeded,
}

#[derive(Debug)]
struct BoundedSkillRead {
    bytes_read: u64,
    outcome: std::io::Result<BoundedSkillContent>,
}

fn read_skill_content_bounded(
    path: &Path,
    max_file_bytes: u64,
    total_remaining: u64,
) -> BoundedSkillRead {
    use std::io::Read;

    let mut file = match fs::File::open(path) {
        Ok(file) => file,
        Err(error) => {
            return BoundedSkillRead {
                bytes_read: 0,
                outcome: Err(error),
            };
        }
    };
    let read_cap = max_file_bytes.saturating_add(1).min(total_remaining);
    if read_cap == 0 {
        return BoundedSkillRead {
            bytes_read: 0,
            outcome: Ok(BoundedSkillContent::TotalBudgetExceeded),
        };
    }
    let read_cap = usize::try_from(read_cap).unwrap_or(usize::MAX);
    let mut bytes = Vec::with_capacity(read_cap);
    let mut chunk = [0_u8; 8 * 1024];
    while bytes.len() < read_cap {
        let remaining = read_cap - bytes.len();
        let chunk_len = remaining.min(chunk.len());
        match file.read(&mut chunk[..chunk_len]) {
            Ok(0) => break,
            Ok(count) => bytes.extend_from_slice(&chunk[..count]),
            Err(error) => {
                return BoundedSkillRead {
                    bytes_read: bytes.len() as u64,
                    outcome: Err(error),
                };
            }
        }
    }
    let bytes_read = bytes.len() as u64;
    let outcome = if bytes.len() == read_cap {
        if bytes_read > max_file_bytes {
            Ok(BoundedSkillContent::FileTooLarge)
        } else {
            Ok(BoundedSkillContent::TotalBudgetExceeded)
        }
    } else {
        String::from_utf8(bytes)
            .map(BoundedSkillContent::Content)
            .map_err(|error| std::io::Error::new(std::io::ErrorKind::InvalidData, error))
    };
    BoundedSkillRead {
        bytes_read,
        outcome,
    }
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
    _overrides: &SkillConfigOverrides,
    instance: &mut SkillInstance,
) {
    instance.scope = root.scope;
    instance.path = canonical_path.clone();
    instance.display_path = canonical_path.clone();
    if instance.agent == AgentId::Codex {
        if let Some(namespace) = codex_skill_namespace(root) {
            if !instance.name.starts_with(&format!("{namespace}:")) {
                instance.name = format!("{namespace}:{}", instance.name);
                instance.display_name = instance.name.clone();
            }
        }
    }
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
}

fn finalize_discovered_instance(
    ctx: &AdapterContext,
    root: &AdapterRoot,
    overrides: &SkillConfigOverrides,
    instance: &mut SkillInstance,
) {
    if instance.agent == AgentId::ClaudeCode {
        normalize_claude_runtime_identity(ctx, root, instance);
        instance.definition_id = canonical_definition_id(&instance.name);
    }
    // Agent config overrides are scoped to the current adapter only. Keep the
    // per-scan settings cache outside adapter parsing so one file is not reread
    // for every skill in a root. Claude plugin skills are controlled by plugin
    // enablement and are intentionally unaffected by `skillOverrides`.
    if matches!(instance.state, SkillState::Loaded) && overrides.is_disabled(ctx, root, instance) {
        instance.enabled = false;
        instance.state = SkillState::Disabled;
    }
}

fn normalize_claude_runtime_identity(
    ctx: &AdapterContext,
    root: &AdapterRoot,
    instance: &mut SkillInstance,
) {
    if root.source == RootSource::Plugin {
        if let Some(namespace) = claude_plugin_namespace(&instance.path) {
            instance.name = format!("{namespace}:{}", instance.name);
            instance.display_name = instance.name.clone();
        }
        return;
    }
    if root.path.file_name().and_then(|name| name.to_str()) == Some("commands")
        && instance
            .display_path
            .extension()
            .and_then(|extension| extension.to_str())
            .is_some_and(|extension| extension.eq_ignore_ascii_case("md"))
    {
        return;
    }

    let Some(directory_name) = instance
        .display_path
        .parent()
        .and_then(Path::file_name)
        .and_then(|name| name.to_str())
    else {
        return;
    };
    let mut runtime_name = directory_name.to_string();
    if root.scope == Scope::AgentProject {
        let skill_owner_directory = instance.display_path.ancestors().nth(4);
        if let (Some(project_root), Some(skill_owner_directory)) =
            (ctx.project_root.as_deref(), skill_owner_directory)
        {
            if skill_owner_directory != project_root {
                if let Ok(relative) = skill_owner_directory.strip_prefix(project_root) {
                    let qualifier = relative
                        .components()
                        .filter_map(|component| component.as_os_str().to_str())
                        .collect::<Vec<_>>()
                        .join("/");
                    if !qualifier.is_empty() {
                        runtime_name = format!("{qualifier}:{runtime_name}");
                    }
                }
            }
        }
    }
    instance.name = runtime_name.clone();
    instance.display_name = runtime_name;
}

fn claude_plugin_namespace(path: &Path) -> Option<String> {
    path.ancestors().find_map(|ancestor| {
        let manifest = ancestor.join(".claude-plugin/plugin.json");
        let content = fs::read_to_string(manifest).ok()?;
        let value = serde_json::from_str::<serde_json::Value>(&content).ok()?;
        value
            .get("name")
            .and_then(serde_json::Value::as_str)
            .map(str::trim)
            .filter(|name| !name.is_empty())
            .map(ToString::to_string)
    })
}

fn normalized_display_path(
    agent: AgentId,
    root: &AdapterRoot,
    canonical_path: &Path,
    discovered_display_path: PathBuf,
) -> PathBuf {
    if root.source == RootSource::Plugin {
        match agent {
            AgentId::Codex => {
                return codex_plugin_logical_display_path(canonical_path)
                    .unwrap_or(discovered_display_path);
            }
            AgentId::ClaudeCode => {
                return claude_plugin_logical_display_path(canonical_path)
                    .unwrap_or(discovered_display_path);
            }
            _ => {}
        }
    }
    discovered_display_path
}

fn claude_plugin_logical_display_path(path: &Path) -> Option<PathBuf> {
    let plugin_root = path
        .ancestors()
        .find(|ancestor| ancestor.join(".claude-plugin/plugin.json").is_file())?;
    let namespace = claude_plugin_namespace(path)?;
    let relative = path.strip_prefix(plugin_root).ok()?;
    Some(
        PathBuf::from("$CLAUDE_CONFIG_DIR")
            .join("plugins")
            .join(namespace)
            .join(relative),
    )
}

fn codex_skill_namespace(root: &AdapterRoot) -> Option<String> {
    if root.source == RootSource::Plugin {
        let components = root
            .path
            .components()
            .filter_map(|component| component.as_os_str().to_str())
            .collect::<Vec<_>>();
        if let Some(cache_index) = components
            .windows(2)
            .position(|window| window == ["plugins", "cache"])
        {
            return components
                .get(cache_index + 3)
                .map(|package| (*package).to_string());
        }
        return root
            .path
            .ancestors()
            .find(|ancestor| ancestor.join(".codex-plugin/plugin.json").is_file())
            .and_then(Path::parent)
            .and_then(Path::file_name)
            .and_then(|name| name.to_str())
            .map(ToString::to_string);
    }
    let metadata = fs::symlink_metadata(&root.path).ok()?;
    if !metadata.file_type().is_symlink() || root.path.join("SKILL.md").is_file() {
        return None;
    }
    root.path
        .file_name()
        .and_then(|name| name.to_str())
        .map(ToString::to_string)
}

fn codex_plugin_logical_display_path(path: &Path) -> Option<PathBuf> {
    let components = path
        .components()
        .filter_map(|component| component.as_os_str().to_str())
        .collect::<Vec<_>>();
    let cache_index = components
        .windows(2)
        .position(|window| window == ["plugins", "cache"])?;
    let publisher = *components.get(cache_index + 2)?;
    let package = *components.get(cache_index + 3)?;
    let payload_index = cache_index + 5;
    let payload = components.get(payload_index..)?;
    if payload.len() < 2 || payload.last() != Some(&"SKILL.md") {
        return None;
    }
    let mut logical = PathBuf::from("$CODEX_HOME")
        .join("plugins")
        .join(format!("{package}@{publisher}"));
    for component in payload {
        logical.push(component);
    }
    Some(logical)
}

#[derive(Debug)]
struct SkillConfigOverrides {
    agent: AgentId,
    disabled: HashSet<String>,
    claude_skill_overrides: HashMap<String, bool>,
    opencode_skill_permissions: Vec<(String, bool)>,
    pi_settings: Vec<(Scope, PathBuf, String)>,
    openclaw_config: Option<serde_json::Value>,
    openclaw_allowed_skills: Option<HashSet<String>>,
    openclaw_allow_bundled: Option<HashSet<String>>,
}

impl SkillConfigOverrides {
    fn preload(adapter: &dyn AgentAdapter, ctx: &AdapterContext) -> Self {
        let agent = adapter.id();
        let mut disabled = HashSet::new();
        let mut claude_skill_overrides = HashMap::new();
        let mut opencode_skill_permissions = Vec::new();
        let mut pi_settings = Vec::new();
        let mut openclaw_config = None;
        let mut openclaw_allowed_skills = None;
        let mut openclaw_allow_bundled = None;
        let config_paths = adapter.config_paths(ctx);
        match agent {
            AgentId::ClaudeCode => {
                for settings_path in config_paths {
                    if let Some(entries) = read_claude_skill_overrides(&settings_path) {
                        claude_skill_overrides.extend(entries);
                    }
                }
            }
            AgentId::Opencode => {
                for settings_path in config_paths {
                    if let Some(permissions) = read_opencode_skill_permissions(&settings_path) {
                        opencode_skill_permissions.extend(permissions);
                    }
                }
            }
            AgentId::Hermes => {
                for settings_path in config_paths {
                    if let Some(names) = read_disabled_hermes_skills(&settings_path) {
                        disabled.extend(names);
                    }
                }
            }
            AgentId::Pi => {
                for settings_path in config_paths {
                    let Ok(content) = fs::read_to_string(&settings_path) else {
                        continue;
                    };
                    let scope = if settings_path
                        .components()
                        .any(|component| component.as_os_str() == ".pi")
                        && !settings_path.starts_with(ctx.user_home.join(".pi/agent"))
                    {
                        Scope::AgentProject
                    } else {
                        Scope::AgentGlobal
                    };
                    pi_settings.push((scope, settings_path, content));
                }
            }
            AgentId::Openclaw => {
                for settings_path in config_paths {
                    if let Some(config) = read_openclaw_config(&settings_path) {
                        if let Some(names) = disabled_openclaw_skill_entries(&config) {
                            disabled.extend(names);
                        }
                        openclaw_allowed_skills = effective_openclaw_agent_skill_allowlist(
                            ctx,
                            &config,
                            &openclaw_state_dir(ctx),
                        );
                        openclaw_allow_bundled = config
                            .get("skills")
                            .and_then(|skills| skills.get("allowBundled"))
                            .and_then(serde_json::Value::as_array)
                            .map(|items| {
                                items
                                    .iter()
                                    .filter_map(serde_json::Value::as_str)
                                    .map(ToString::to_string)
                                    .collect()
                            });
                        openclaw_config = Some(config);
                    } else if let Some(names) = read_disabled_openclaw_skill_entries(&settings_path)
                    {
                        disabled.extend(names);
                    }
                }
            }
            _ => {}
        }

        Self {
            agent,
            disabled,
            claude_skill_overrides,
            opencode_skill_permissions,
            pi_settings,
            openclaw_config,
            openclaw_allowed_skills,
            openclaw_allow_bundled,
        }
    }

    fn is_disabled(
        &self,
        ctx: &AdapterContext,
        root: &AdapterRoot,
        instance: &SkillInstance,
    ) -> bool {
        if self.agent == AgentId::Pi {
            return self
                .pi_settings
                .iter()
                .filter(|(scope, _, _)| *scope == root.scope)
                .any(|(_, path, content)| {
                    !pi_skill_enabled_by_settings(content, path, &instance.path, &ctx.user_home)
                });
        }
        if self.agent == AgentId::Opencode {
            return self
                .opencode_skill_permissions
                .iter()
                .filter(|(pattern, _)| wildcard_matches(pattern, &instance.name))
                .map(|(_, denied)| *denied)
                .next_back()
                .unwrap_or(false);
        }
        if self.agent == AgentId::ClaudeCode {
            if root.source == RootSource::Plugin {
                return false;
            }
            return self
                .claude_skill_overrides
                .get(&instance.name)
                .copied()
                .unwrap_or(false);
        }
        let skill_key = match self.agent {
            AgentId::Openclaw => {
                openclaw_config_key_from_frontmatter(&instance.frontmatter_raw, &instance.name)
            }
            _ => instance.name.clone(),
        };
        if self.agent == AgentId::Openclaw
            && (self
                .openclaw_allowed_skills
                .as_ref()
                .is_some_and(|allowed| !allowed.contains(&instance.name))
                || (root.source == RootSource::System
                    && self
                        .openclaw_allow_bundled
                        .as_ref()
                        .is_some_and(|allowed| !allowed.contains(&instance.name)))
                || self.openclaw_config.as_ref().is_some_and(|config| {
                    !openclaw_skill_is_eligible(config, instance, &skill_key)
                }))
        {
            return true;
        }
        self.disabled.contains(&skill_key)
    }
}

fn pi_skill_enabled_by_settings(
    content: &str,
    settings_path: &Path,
    skill_path: &Path,
    user_home: &Path,
) -> bool {
    let Ok(value) = serde_json::from_str::<serde_json::Value>(content) else {
        return true;
    };
    let patterns = value
        .get("skills")
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .filter_map(serde_json::Value::as_str)
        .collect::<Vec<_>>();
    let base = settings_path.parent().unwrap_or(user_home);
    let matches = |raw: &str| {
        let raw = raw.trim();
        let target = if raw == "~" {
            user_home.to_path_buf()
        } else if let Some(relative) = raw.strip_prefix("~/") {
            user_home.join(relative)
        } else {
            let raw_path = PathBuf::from(raw);
            if raw_path.is_absolute() {
                raw_path
            } else {
                base.join(raw_path)
            }
        };
        let target = target.canonicalize().unwrap_or(target);
        let skill = skill_path
            .canonicalize()
            .unwrap_or_else(|_| normalize_path_lexically(skill_path));
        target == skill || skill.parent().is_some_and(|parent| target == parent)
    };
    let excluded = patterns
        .iter()
        .filter_map(|raw| raw.strip_prefix('!'))
        .any(matches);
    let included = patterns
        .iter()
        .filter_map(|raw| raw.strip_prefix('+'))
        .any(matches);
    let force_excluded = patterns
        .iter()
        .filter_map(|raw| raw.strip_prefix('-'))
        .any(matches);
    (!excluded || included) && !force_excluded
}

fn read_claude_skill_overrides(settings_path: &Path) -> Option<HashMap<String, bool>> {
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
            .filter_map(|(name, value)| match value.as_str() {
                Some("off") => Some((name.clone(), true)),
                Some("on" | "name-only" | "user-invocable-only") => Some((name.clone(), false)),
                _ => None,
            })
            .collect(),
    )
}

fn read_opencode_skill_permissions(settings_path: &Path) -> Option<Vec<(String, bool)>> {
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
    let mut permissions = skill_permissions
        .iter()
        .filter_map(|(pattern, value)| match value.as_str() {
            Some("deny") => Some((pattern.clone(), true)),
            Some("allow") | Some("ask") => Some((pattern.clone(), false)),
            _ => None,
        })
        .collect::<Vec<_>>();
    permissions.sort_by_key(|(pattern, _)| {
        pattern
            .chars()
            .filter(|ch| !matches!(ch, '*' | '?'))
            .count()
    });
    Some(permissions)
}

fn wildcard_matches(pattern: &str, value: &str) -> bool {
    let pattern = pattern.as_bytes();
    let value = value.as_bytes();
    let (mut pattern_index, mut value_index) = (0usize, 0usize);
    let (mut star_index, mut star_value_index) = (None, 0usize);
    while value_index < value.len() {
        if pattern_index < pattern.len()
            && (pattern[pattern_index] == b'?' || pattern[pattern_index] == value[value_index])
        {
            pattern_index += 1;
            value_index += 1;
        } else if pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
            star_index = Some(pattern_index);
            pattern_index += 1;
            star_value_index = value_index;
        } else if let Some(star) = star_index {
            pattern_index = star + 1;
            star_value_index += 1;
            value_index = star_value_index;
        } else {
            return false;
        }
    }
    while pattern_index < pattern.len() && pattern[pattern_index] == b'*' {
        pattern_index += 1;
    }
    pattern_index == pattern.len()
}

fn read_disabled_hermes_skills(settings_path: &Path) -> Option<HashSet<String>> {
    let Ok(content) = fs::read_to_string(settings_path) else {
        return None;
    };
    let value = serde_norway::from_str::<serde_norway::Value>(&content).ok()?;
    let disabled = value
        .get("skills")
        .and_then(|skills| skills.get("disabled"))
        .and_then(serde_norway::Value::as_sequence)?;
    Some(
        disabled
            .iter()
            .filter_map(serde_norway::Value::as_str)
            .map(str::trim)
            .filter(|value| !value.is_empty())
            .map(ToString::to_string)
            .collect(),
    )
}

fn read_disabled_openclaw_skill_entries(settings_path: &Path) -> Option<HashSet<String>> {
    let value = read_openclaw_config(settings_path)?;
    disabled_openclaw_skill_entries(&value)
}

fn read_openclaw_config(settings_path: &Path) -> Option<serde_json::Value> {
    let Ok(content) = fs::read_to_string(settings_path) else {
        return None;
    };
    json5::from_str::<serde_json::Value>(&content).ok()
}

fn disabled_openclaw_skill_entries(value: &serde_json::Value) -> Option<HashSet<String>> {
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

fn effective_openclaw_agent_skill_allowlist(
    ctx: &AdapterContext,
    config: &serde_json::Value,
    state_dir: &Path,
) -> Option<HashSet<String>> {
    let defaults = config
        .get("agents")
        .and_then(|agents| agents.get("defaults"))
        .and_then(|defaults| defaults.get("skills"))
        .and_then(json_string_set);
    let selected = [ctx.project_cwd.as_deref(), ctx.project_root.as_deref()]
        .into_iter()
        .flatten()
        .collect::<Vec<_>>();
    let matching_agent = config
        .get("agents")
        .and_then(|agents| agents.get("list"))
        .and_then(serde_json::Value::as_array)
        .into_iter()
        .flatten()
        .find(|agent| {
            agent
                .get("workspace")
                .and_then(serde_json::Value::as_str)
                .and_then(|raw| openclaw_config_path_value(raw, &ctx.user_home, state_dir))
                .is_some_and(|workspace| {
                    selected.iter().any(|selected| {
                        let selected = normalize_path_lexically(selected);
                        let workspace = normalize_path_lexically(&workspace);
                        selected == workspace || selected.starts_with(&workspace)
                    })
                })
        });
    matching_agent
        .and_then(|agent| agent.get("skills"))
        .and_then(json_string_set)
        .or(defaults)
}

fn openclaw_state_dir(ctx: &AdapterContext) -> PathBuf {
    if let Some(path) = std::env::var_os("OPENCLAW_STATE_DIR")
        .map(PathBuf::from)
        .filter(|path| path.is_absolute())
    {
        return normalize_path_lexically(&path);
    }
    let profile = std::env::var("OPENCLAW_PROFILE")
        .ok()
        .map(|value| value.trim().to_string())
        .filter(|value| {
            !value.is_empty()
                && value
                    .bytes()
                    .all(|byte| byte.is_ascii_alphanumeric() || matches!(byte, b'-' | b'_'))
        });
    profile.map_or_else(
        || ctx.user_home.join(".openclaw"),
        |profile| ctx.user_home.join(format!(".openclaw-{profile}")),
    )
}

fn json_string_set(value: &serde_json::Value) -> Option<HashSet<String>> {
    value.as_array().map(|items| {
        items
            .iter()
            .filter_map(serde_json::Value::as_str)
            .map(ToString::to_string)
            .collect()
    })
}

fn openclaw_config_path_value(raw: &str, user_home: &Path, base: &Path) -> Option<PathBuf> {
    let trimmed = raw.trim();
    if trimmed.is_empty() {
        return None;
    }
    if trimmed == "~" {
        return Some(user_home.to_path_buf());
    }
    if let Some(relative) = trimmed.strip_prefix("~/") {
        return Some(user_home.join(relative));
    }
    let path = PathBuf::from(trimmed);
    Some(if path.is_absolute() {
        path
    } else {
        base.join(path)
    })
}

fn openclaw_skill_is_eligible(
    config: &serde_json::Value,
    instance: &SkillInstance,
    skill_key: &str,
) -> bool {
    let Ok(frontmatter) = serde_norway::from_str::<serde_norway::Value>(&instance.frontmatter_raw)
    else {
        return true;
    };
    let Some(requires) = frontmatter
        .get("metadata")
        .and_then(|metadata| metadata.get("openclaw"))
        .and_then(|openclaw| openclaw.get("requires"))
    else {
        return true;
    };
    let strings = |key: &str| {
        requires
            .get(key)
            .and_then(serde_norway::Value::as_sequence)
            .into_iter()
            .flatten()
            .filter_map(serde_norway::Value::as_str)
            .collect::<Vec<_>>()
    };
    let bins = strings("bins");
    if bins.iter().any(|bin| !command_exists(bin)) {
        return false;
    }
    let any_bins = strings("anyBins");
    if !any_bins.is_empty() && !any_bins.iter().any(|bin| command_exists(bin)) {
        return false;
    }
    let os = strings("os");
    if !os.is_empty()
        && !os
            .iter()
            .any(|value| openclaw_os_matches(value, std::env::consts::OS))
    {
        return false;
    }
    let configured_env = config
        .get("skills")
        .and_then(|skills| skills.get("entries"))
        .and_then(|entries| entries.get(skill_key));
    for name in strings("env") {
        let process_has_value = std::env::var_os(name).is_some_and(|value| !value.is_empty());
        let entry_has_value = configured_env.is_some_and(|entry| {
            entry
                .get("env")
                .and_then(|env| env.get(name))
                .is_some_and(|value| !value.is_null())
                || entry.get("apiKey").is_some_and(|value| !value.is_null())
        });
        if !process_has_value && !entry_has_value {
            return false;
        }
    }
    strings("config")
        .iter()
        .all(|path| json_dotted_path_exists(config, path))
}

fn command_exists(command: &str) -> bool {
    if command.trim().is_empty() || command.contains(std::path::MAIN_SEPARATOR) {
        return false;
    }
    std::env::var_os("PATH")
        .into_iter()
        .flat_map(|paths| std::env::split_paths(&paths).collect::<Vec<_>>())
        .any(|directory| directory.join(command).is_file())
}

fn openclaw_os_matches(required: &str, actual: &str) -> bool {
    matches!(
        (required, actual),
        ("darwin" | "macos", "macos") | ("win32" | "windows", "windows")
    ) || required == actual
}

fn json_dotted_path_exists(value: &serde_json::Value, path: &str) -> bool {
    path.split('.')
        .filter(|part| !part.is_empty())
        .try_fold(value, |current, part| current.get(part))
        .is_some_and(|value| !value.is_null())
}

fn openclaw_config_key_from_frontmatter(frontmatter_raw: &str, fallback_name: &str) -> String {
    serde_norway::from_str::<serde_norway::Value>(frontmatter_raw)
        .ok()
        .and_then(|frontmatter| {
            frontmatter
                .get("metadata")
                .and_then(|metadata| metadata.get("openclaw"))
                .and_then(|openclaw| openclaw.get("skillKey"))
                .and_then(serde_norway::Value::as_str)
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
    use std::collections::HashMap;

    use skills_copilot_adapters::{
        ClaudeCodeAdapter, CodexAdapter, HermesAdapter, OpenclawAdapter, OpencodeAdapter, PiAdapter,
    };
    use skills_copilot_core::{AdapterContext, AdapterRoot, RootSource};

    use super::*;

    #[test]
    fn codex_plugin_display_path_identifies_install_without_exposing_cache_layout() {
        assert_eq!(
            codex_plugin_logical_display_path(Path::new(
                "/home/.codex/plugins/cache/openai-bundled/browser/1.2.3/skills/control/SKILL.md"
            )),
            Some(PathBuf::from(
                "$CODEX_HOME/plugins/browser@openai-bundled/skills/control/SKILL.md"
            ))
        );
        assert_eq!(
            codex_plugin_logical_display_path(Path::new(
                "/home/.codex/plugins/cache/personal/workflows/2.0.0/playbooks/review/SKILL.md"
            )),
            Some(PathBuf::from(
                "$CODEX_HOME/plugins/workflows@personal/playbooks/review/SKILL.md"
            )),
            "a nonstandard manifest skills root must stay logical and cache-free"
        );
    }

    #[test]
    fn codex_plugin_namespace_matches_runtime_skill_identity() {
        assert_eq!(
            codex_skill_namespace(&AdapterRoot {
                scope: Scope::AgentGlobal,
                path: PathBuf::from(
                    "/home/.codex/plugins/cache/openai-bundled/browser/1.2.3/skills"
                ),
                source: RootSource::Plugin,
            })
            .as_deref(),
            Some("browser")
        );
        assert_eq!(
            codex_skill_namespace(&AdapterRoot {
                scope: Scope::AgentGlobal,
                path: PathBuf::from(
                    "/home/.codex/plugins/cache/openai-curated-remote/product-design/0.1.52/skills"
                ),
                source: RootSource::Plugin,
            })
            .as_deref(),
            Some("product-design"),
            "a manifest-declared plugin root must retain its package namespace"
        );
    }

    #[cfg(unix)]
    #[test]
    fn codex_scans_declared_shared_skill_symlinks_with_runtime_namespaces() {
        use std::os::unix::fs::symlink;

        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-codex-shared-links-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let shared = home.join(".agents/skills");
        let grouped = temp_root.join("grouped-skills");
        let direct = temp_root.join("direct-skill");
        std::fs::create_dir_all(grouped.join("android-native-dev")).expect("create grouped skill");
        std::fs::create_dir_all(&direct).expect("create direct skill");
        std::fs::create_dir_all(&shared).expect("create shared skills root");
        std::fs::write(
            grouped.join("android-native-dev/SKILL.md"),
            "---\nname: android-native-dev\ndescription: grouped fixture\n---\nBody.",
        )
        .expect("write grouped skill");
        std::fs::write(
            direct.join("SKILL.md"),
            "---\nname: short-drama-pro\ndescription: direct fixture\n---\nBody.",
        )
        .expect("write direct skill");
        symlink(&grouped, shared.join("minimax-skills")).expect("link grouped root");
        symlink(&direct, shared.join("short-drama-pro")).expect("link direct root");

        let report = scan_agent(
            &CodexAdapter,
            &AdapterContext {
                user_home: home,
                project_root: None,
                project_cwd: None,
                extra_roots: vec![],
            },
        )
        .expect("scan succeeds");
        let names = report
            .instances
            .iter()
            .map(|skill| skill.name.as_str())
            .collect::<HashSet<_>>();

        assert!(
            names.contains("minimax-skills:android-native-dev"),
            "names={names:?}; issues={:?}",
            report.issues
        );
        assert!(names.contains("short-drama-pro"), "names={names:?}");
        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn codex_scans_only_enabled_plugin_and_hides_cache_storage_path() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-codex-plugin-display-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let plugin = home.join(".codex/plugins/cache/openai-bundled/browser/1.2.3");
        std::fs::create_dir_all(plugin.join(".codex-plugin"))
            .expect("create plugin manifest directory");
        std::fs::create_dir_all(plugin.join("skills/control"))
            .expect("create plugin skill directory");
        std::fs::write(
            plugin.join(".codex-plugin/plugin.json"),
            r#"{"name":"browser","version":"1.2.3","skills":"./skills/"}"#,
        )
        .expect("write plugin manifest");
        std::fs::write(
            plugin.join("skills/control/SKILL.md"),
            "---\nname: control\ndescription: browser control fixture\n---\nBody.",
        )
        .expect("write plugin skill");
        std::fs::write(
            home.join(".codex/config.toml"),
            "[plugins.\"browser@openai-bundled\"]\nenabled = true\n",
        )
        .expect("write plugin enablement");

        let report = scan_agent(
            &CodexAdapter,
            &AdapterContext {
                user_home: home,
                project_root: None,
                project_cwd: None,
                extra_roots: vec![],
            },
        )
        .expect("scan succeeds");

        assert_eq!(report.instances.len(), 1, "issues={:?}", report.issues);
        assert_eq!(report.instances[0].name, "browser:control");
        assert_eq!(
            report.instances[0].display_path,
            PathBuf::from("$CODEX_HOME/plugins/browser@openai-bundled/skills/control/SKILL.md")
        );
        assert!(!report.instances[0]
            .display_path
            .to_string_lossy()
            .contains("/plugins/cache/"));

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn yaml_contract_scanner_preserves_disabled_sequence_and_nested_metadata() {
        let root = std::env::temp_dir().join(format!(
            "skills-copilot-scanner-yaml-contract-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .expect("time after epoch")
                .as_nanos()
        ));
        let config_path = root.join("config.yaml");
        std::fs::create_dir_all(&root).expect("create yaml contract root");
        std::fs::write(
            &config_path,
            "skills:\n  disabled:\n    - alpha\n    - beta\n  metadata:\n    source: local\nenabled: true\n",
        )
        .expect("write yaml contract config");

        let disabled = read_disabled_hermes_skills(&config_path).expect("disabled skills");
        assert_eq!(
            disabled,
            HashSet::from(["alpha".to_string(), "beta".to_string()])
        );
        assert_eq!(
            openclaw_config_key_from_frontmatter(
                "name: visible\nmetadata:\n  openclaw:\n    skillKey: routed-key\n",
                "fallback",
            ),
            "routed-key"
        );

        std::fs::write(&config_path, "skills: [unterminated\n")
            .expect("write malformed yaml contract config");
        assert!(read_disabled_hermes_skills(&config_path).is_none());

        let _ = std::fs::remove_dir_all(root);
    }

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
    fn stale_metadata_growth_never_reads_past_total_byte_budget() {
        let (temp_root, scan_root, ctx, adapter) = budget_test_fixture("stale-metadata");
        let skill_path = write_budget_skill(
            &scan_root,
            "growing",
            "---\nname: growing\ndescription: content larger than the injected metadata\n---\nbody that exceeds the total byte budget\n",
        );
        let canonical_skill = skill_path.canonicalize().expect("canonical skill path");
        let mut limits = test_scan_limits();
        limits.max_total_skill_bytes = 32;

        let report = scan_agent_with_limits_and_inspectors(
            &adapter,
            &ctx,
            limits,
            |path| fs::symlink_metadata(path).map(|metadata| metadata.file_type().is_symlink()),
            |path| {
                if path == canonical_skill {
                    Ok(1)
                } else {
                    fs::metadata(path).map(|metadata| metadata.len())
                }
            },
        )
        .expect("bounded scan returns a report");
        let canonical_root = scan_root.canonicalize().expect("canonical scan root");

        assert_eq!(report.stats.bytes_read, limits.max_total_skill_bytes);
        assert!(report.stats.budget_exhausted);
        assert!(report.partial_roots.contains(&canonical_root));
        assert!(!report.scanned_roots.contains(&canonical_root));
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ScanIssueKind::BudgetExceeded));

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
    fn same_canonical_root_keeps_complete_and_partial_scopes_distinct() {
        let (temp_root, scan_root, ctx, mut adapter) = budget_test_fixture("cross-scope-root");
        write_budget_skill(
            &scan_root,
            "shared",
            "---\nname: shared\ndescription: shared scope fixture\n---\nbody\n",
        );
        adapter.roots = vec![
            AdapterRoot {
                scope: Scope::AgentGlobal,
                path: scan_root.clone(),
                source: RootSource::Extra,
            },
            AdapterRoot {
                scope: Scope::AgentProject,
                path: scan_root.clone(),
                source: RootSource::Extra,
            },
        ];
        let mut limits = test_scan_limits();
        limits.max_directories = 2;

        let report =
            scan_agent_with_limits(&adapter, &ctx, limits).expect("bounded scan returns a report");
        let canonical_root = scan_root.canonicalize().expect("canonical scan root");

        assert_eq!(
            report.scoped_scanned_roots,
            vec![ScopedScanRoot {
                scope: Scope::AgentGlobal,
                path: canonical_root.clone(),
            }]
        );
        assert_eq!(
            report.scoped_partial_roots,
            vec![ScopedScanRoot {
                scope: Scope::AgentProject,
                path: canonical_root,
            }]
        );

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn depth_limit_accepts_skill_at_cap_and_marks_cap_plus_one_partial() {
        let (temp_root, scan_root, ctx, adapter) = budget_test_fixture("depth-cap");
        write_budget_skill(
            &scan_root,
            "at-cap",
            "---\nname: at-cap\ndescription: depth cap fixture\n---\nbody\n",
        );
        write_budget_skill(
            &scan_root.join("too-deep-parent"),
            "too-deep",
            "---\nname: too-deep\ndescription: beyond depth cap fixture\n---\nbody\n",
        );
        let mut limits = test_scan_limits();
        limits.max_depth = 1;

        let report =
            scan_agent_with_limits(&adapter, &ctx, limits).expect("bounded scan returns a report");
        let canonical_root = scan_root.canonicalize().expect("canonical root");

        assert!(report
            .instances
            .iter()
            .any(|instance| instance.name == "at-cap"));
        assert!(!report
            .instances
            .iter()
            .any(|instance| instance.name == "too-deep"));
        assert!(report.stats.budget_exhausted);
        assert!(report.scoped_partial_roots.contains(&ScopedScanRoot {
            scope: Scope::AgentGlobal,
            path: canonical_root,
        }));

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn directory_limit_stops_at_cap_and_marks_root_partial() {
        let (temp_root, scan_root, ctx, adapter) = budget_test_fixture("directory-cap");
        std::fs::create_dir_all(scan_root.join("a")).expect("create first directory");
        std::fs::create_dir_all(scan_root.join("b")).expect("create second directory");
        let mut limits = test_scan_limits();
        limits.max_directories = 2;

        let report =
            scan_agent_with_limits(&adapter, &ctx, limits).expect("bounded scan returns a report");

        assert_eq!(report.stats.directories_visited, 2);
        assert!(report.stats.budget_exhausted);
        assert_eq!(report.scoped_partial_roots.len(), 1);

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn skill_file_limit_counts_exact_cap_and_stops_before_cap_plus_one() {
        let (temp_root, scan_root, ctx, adapter) = budget_test_fixture("skill-file-cap");
        for name in ["a", "b", "c"] {
            write_budget_skill(
                &scan_root,
                name,
                &format!("---\nname: {name}\ndescription: cap fixture\n---\nbody\n"),
            );
        }
        let mut limits = test_scan_limits();
        limits.max_directories = 8;
        limits.max_entries = 16;
        limits.max_skill_files = 2;

        let report =
            scan_agent_with_limits(&adapter, &ctx, limits).expect("bounded scan returns a report");

        assert_eq!(report.stats.skill_files_seen, 2);
        assert_eq!(report.instances.len(), 2);
        assert!(report.stats.budget_exhausted);
        assert_eq!(report.scoped_partial_roots.len(), 1);

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn failed_skill_metadata_still_counts_candidate_and_returns_broken_instance() {
        let (temp_root, scan_root, ctx, adapter) = budget_test_fixture("metadata-failure");
        let skill_path = write_budget_skill(
            &scan_root,
            "metadata-failure",
            "---\nname: metadata-failure\ndescription: metadata failure fixture\n---\nbody\n",
        )
        .canonicalize()
        .expect("canonical skill");

        let report = scan_agent_with_limits_and_inspectors(
            &adapter,
            &ctx,
            test_scan_limits(),
            |path| fs::symlink_metadata(path).map(|metadata| metadata.file_type().is_symlink()),
            |path| {
                if path == skill_path {
                    Err(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        "injected metadata failure",
                    ))
                } else {
                    fs::metadata(path).map(|metadata| metadata.len())
                }
            },
        )
        .expect("scan returns degraded instance");

        assert_eq!(report.stats.skill_files_seen, 1);
        assert_eq!(report.instances.len(), 1);
        assert_eq!(report.instances[0].state, SkillState::Broken);
        assert!(report
            .issues
            .iter()
            .any(|issue| issue.kind == ScanIssueKind::FileUnreadable));

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
    fn generated_cache_and_vcs_directories_are_never_skill_sources() {
        let (temp_root, scan_root, ctx, adapter) = budget_test_fixture("generated-roots");
        write_budget_skill(
            &scan_root,
            "active",
            "---\nname: active\ndescription: active fixture\n---\nbody\n",
        );
        for generated in ["cache", ".cache", "dist", "build", "target", ".git"] {
            write_budget_skill(
                &scan_root.join(generated),
                "stale",
                "---\nname: stale\ndescription: generated fixture\n---\nbody\n",
            );
        }

        let report = scan_agent(&adapter, &ctx).expect("scan succeeds");
        assert_eq!(
            report
                .instances
                .iter()
                .map(|instance| instance.name.as_str())
                .collect::<Vec<_>>(),
            vec!["active"]
        );

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
        assert_eq!(report.instances[0].name, "valid-summarize");
        assert_eq!(report.instances[0].scope, Scope::AgentGlobal);
    }

    #[test]
    fn claude_scans_only_enabled_plugin_skills_with_namespaced_logical_provenance() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-claude-plugin-scan-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let config = home.join(".claude");
        let plugin = config.join("plugins/cache/market/review-kit/1.2.3");
        let skill = plugin.join("skills/review");
        std::fs::create_dir_all(plugin.join(".claude-plugin"))
            .expect("create Claude plugin manifest dir");
        std::fs::create_dir_all(&skill).expect("create Claude plugin skill dir");
        std::fs::write(
            plugin.join(".claude-plugin/plugin.json"),
            r#"{"name":"review-kit"}"#,
        )
        .expect("write Claude plugin manifest");
        std::fs::write(
            skill.join("SKILL.md"),
            "---\nname: review\ndescription: plugin fixture\n---\nBody.\n",
        )
        .expect("write Claude plugin skill");
        std::fs::write(
            config.join("settings.json"),
            r#"{"enabledPlugins":{"review-kit@market":true},"skillOverrides":{"review-kit:review":"off"}}"#,
        )
        .expect("write Claude settings");
        std::fs::create_dir_all(config.join("plugins")).expect("create Claude plugins dir");
        std::fs::write(
            config.join("plugins/installed_plugins.json"),
            serde_json::json!({
                "version": 2,
                "plugins": {
                    "review-kit@market": [{"installPath": plugin}]
                }
            })
            .to_string(),
        )
        .expect("write Claude installed plugin index");

        let report = scan_agent(
            &ClaudeCodeAdapter,
            &AdapterContext {
                user_home: home,
                project_root: None,
                project_cwd: None,
                extra_roots: Vec::new(),
            },
        )
        .expect("scan succeeds");

        assert_eq!(report.instances.len(), 1);
        let instance = &report.instances[0];
        assert_eq!(instance.name, "review-kit:review");
        assert_eq!(instance.state, SkillState::Loaded);
        assert!(instance.enabled, "plugin enablement is not a skillOverride");
        assert_eq!(
            instance.display_path,
            PathBuf::from("$CLAUDE_CONFIG_DIR/plugins/review-kit/skills/review/SKILL.md")
        );
        assert!(!instance.display_path.to_string_lossy().contains("cache"));

        let _ = std::fs::remove_dir_all(temp_root);
    }

    #[test]
    fn claude_project_command_keeps_its_runtime_command_name() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-claude-project-command-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let project = temp_root.join("project");
        let command = project.join(".claude/commands/gan-build.md");
        std::fs::create_dir_all(command.parent().expect("command parent"))
            .expect("create Claude project commands dir");
        std::fs::write(&command, "Build the GAN feature end to end.\n")
            .expect("write Claude project command");

        let report = scan_agent(
            &ClaudeCodeAdapter,
            &AdapterContext {
                user_home: home,
                project_root: Some(project.clone()),
                project_cwd: Some(project),
                extra_roots: vec![],
            },
        )
        .expect("Claude scan succeeds");
        let instance = report
            .instances
            .iter()
            .find(|instance| instance.display_path == command)
            .unwrap_or_else(|| {
                panic!(
                    "project command is scanned; instances={:?}; issues={:?}",
                    report.instances, report.issues
                )
            });

        assert_eq!(instance.name, "gan-build");
        assert_eq!(instance.display_name, "gan-build");
        assert_eq!(instance.scope, Scope::AgentProject);
        let _ = std::fs::remove_dir_all(temp_root);
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
    fn claude_personal_skill_symlink_explicitly_authorizes_its_exact_target() {
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

        assert_eq!(report.instances.len(), 1);
        assert!(report.instances[0].path.starts_with(&canonical_private_dir));
        assert_eq!(report.instances[0].name, "linked");
        assert_eq!(
            report.instances[0].display_path,
            claude_skills_dir.join("linked/SKILL.md")
        );

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
    fn missing_optional_project_root_does_not_degrade_scan() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-missing-optional-project-root-{}",
            std::process::id()
        ));
        let missing_project_root = temp_root.join("project/.opencode/skills");
        let valid_root = temp_root.join("valid-skills");
        let valid_skill_dir = valid_root.join("valid-review");
        std::fs::create_dir_all(&valid_skill_dir).expect("create valid skill dir");
        std::fs::write(
            valid_skill_dir.join("SKILL.md"),
            "---\nname: valid-review\ndescription: valid Claude fixture\n---\nBody.",
        )
        .expect("write valid SKILL.md");
        let adapter = TestAdapter {
            roots: vec![
                AdapterRoot {
                    scope: Scope::AgentProject,
                    path: missing_project_root,
                    source: RootSource::Project,
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
            project_root: Some(temp_root.join("project")),
            project_cwd: Some(temp_root.join("project")),
            extra_roots: vec![],
        };

        let report = scan_agent(&adapter, &ctx).expect("scan succeeds");

        assert_eq!(report.instances.len(), 1);
        assert_eq!(report.instances[0].name, "valid-review");
        assert!(report.skipped_roots.is_empty());
        assert!(report.issues.is_empty());

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn missing_explicit_root_is_reported() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-missing-explicit-root-{}",
            std::process::id()
        ));
        let missing_root = temp_root.join("configured-skills");
        let adapter = TestAdapter {
            roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: missing_root.clone(),
                source: RootSource::Extra,
            }],
            link_target_roots: Vec::new(),
        };
        let ctx = AdapterContext {
            user_home: temp_root.join("home"),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&adapter, &ctx).expect("scan degrades missing explicit root");

        assert_eq!(report.skipped_roots, vec![missing_root.clone()]);
        assert_eq!(report.issues.len(), 1);
        assert_eq!(report.issues[0].path, missing_root);
        assert_eq!(report.issues[0].kind, ScanIssueKind::RootUnavailable);

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
    fn dangling_symlink_outside_allowlist_does_not_make_root_partial() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-dangling-outside-root-{}",
            std::process::id()
        ));
        let scan_root = temp_root.join("skills");
        let dangling_link = scan_root.join("removed-external-skill");
        let missing_target = temp_root.join("external/removed-skill");
        std::fs::create_dir_all(&scan_root).expect("create scan root");
        std::os::unix::fs::symlink(&missing_target, &dangling_link)
            .expect("create dangling external skill link");
        let adapter = TestAdapter {
            roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: scan_root.clone(),
                source: RootSource::Extra,
            }],
            link_target_roots: Vec::new(),
        };
        let ctx = AdapterContext {
            user_home: temp_root.join("home"),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&adapter, &ctx).expect("scan succeeds");
        let reported_link = scan_root
            .canonicalize()
            .expect("canonical scan root")
            .join("removed-external-skill");

        assert_eq!(
            report.scanned_roots,
            vec![scan_root.canonicalize().expect("canonical scan root")]
        );
        assert!(report.partial_roots.is_empty());
        assert!(report.issues.iter().any(|issue| {
            issue.path == reported_link && issue.kind == ScanIssueKind::DanglingSymlink
        }));

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    #[cfg(unix)]
    fn dangling_symlink_inside_allowlist_does_not_make_root_partial() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-dangling-inside-root-{}",
            std::process::id()
        ));
        let scan_root = temp_root.join("skills");
        let dangling_link = scan_root.join("removed-local-skill");
        let missing_target = scan_root.join("removed-target");
        std::fs::create_dir_all(&scan_root).expect("create scan root");
        std::os::unix::fs::symlink(&missing_target, &dangling_link)
            .expect("create dangling local skill link");
        let adapter = TestAdapter {
            roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: scan_root.clone(),
                source: RootSource::Extra,
            }],
            link_target_roots: Vec::new(),
        };
        let ctx = AdapterContext {
            user_home: temp_root.join("home"),
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };

        let report = scan_agent(&adapter, &ctx).expect("scan succeeds");
        let reported_link = scan_root
            .canonicalize()
            .expect("canonical scan root")
            .join("removed-local-skill");

        assert!(report.partial_roots.is_empty());
        assert_eq!(
            report.scanned_roots,
            vec![scan_root.canonicalize().expect("canonical scan root")]
        );
        assert!(report.issues.iter().any(|issue| {
            issue.path == reported_link && issue.kind == ScanIssueKind::DanglingSymlink
        }));

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
    fn report_snapshots_declared_and_canonical_root_aliases() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-root-alias-snapshot-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let declared_path = home.join(".claude/skills");
        let target = temp_root.join("external-target");
        write_budget_skill(
            &target,
            "aliased",
            "---\nname: aliased\ndescription: aliased root fixture\n---\nbody\n",
        );
        std::fs::create_dir_all(declared_path.parent().expect("declared parent"))
            .expect("create declared parent");
        std::os::unix::fs::symlink(&target, &declared_path).expect("create root symlink");
        let adapter = TestAdapter {
            roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: declared_path.clone(),
                source: RootSource::UserHome,
            }],
            link_target_roots: vec![AdapterRoot {
                scope: Scope::AgentGlobal,
                path: target.clone(),
                source: RootSource::Extra,
            }],
        };
        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: Vec::new(),
        };

        let report = scan_agent(&adapter, &ctx).expect("scan succeeds");
        let canonical_target = target.canonicalize().expect("canonical target");
        std::fs::remove_file(&declared_path).expect("remove declared symlink after scan");

        assert!(report.root_aliases.iter().any(|alias| {
            alias.declared == declared_path && alias.canonical == canonical_target
        }));

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
    fn claude_project_skill_symlink_explicitly_authorizes_its_exact_target() {
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

        assert_eq!(report.instances.len(), 1);
        assert_eq!(report.instances[0].name, "home-only");
        assert_eq!(report.instances[0].scope, Scope::AgentProject);

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
    fn opencode_permission_skill_wildcard_marks_matching_skill_disabled() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-opencode-wildcard-permission-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let skill_dir = home.join(".config/opencode/skills/global-review");
        std::fs::create_dir_all(&skill_dir).expect("create opencode skill dir");
        std::fs::write(
            skill_dir.join("SKILL.md"),
            "---\nname: global-review\ndescription: wildcard-disabled fixture\n---\nBody.",
        )
        .expect("write opencode SKILL.md");
        std::fs::write(
            home.join(".config/opencode/opencode.json"),
            r#"{"permission":{"skill":{"*":"allow","global-*":"deny"}}}"#,
        )
        .expect("write opencode wildcard permission");

        let report = scan_agent(
            &OpencodeAdapter,
            &AdapterContext {
                user_home: home,
                project_root: None,
                project_cwd: None,
                extra_roots: vec![],
            },
        )
        .expect("scan succeeds");

        assert_eq!(report.instances.len(), 1);
        assert_eq!(report.instances[0].state, SkillState::Disabled);
        assert!(!report.instances[0].enabled);
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
        let skill_dir = configured_root.join("vendor/custom-review");
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
    fn pi_scans_recursive_directory_skills_and_native_root_markdown() {
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

        assert_eq!(report.instances.len(), 4);
        assert!(names.contains("global-pdf"));
        assert!(names.contains("root-note"));
        assert!(names.contains("implementation"));
        assert!(names.contains("root-noise"));

        let _ = std::fs::remove_dir_all(&temp_root);
    }

    #[test]
    fn pi_honors_exact_skill_path_overrides_without_scanning_override_tokens() {
        let temp_root = std::env::temp_dir().join(format!(
            "skills-copilot-pi-path-override-{}",
            std::process::id()
        ));
        let home = temp_root.join("home");
        let skill_path = home.join(".pi/agent/skills/review/SKILL.md");
        let configured_path = temp_root.join("configured/extra.md");
        std::fs::create_dir_all(skill_path.parent().expect("skill parent"))
            .expect("create Pi skill");
        std::fs::create_dir_all(configured_path.parent().expect("configured parent"))
            .expect("create configured parent");
        for (path, name) in [(&skill_path, "review"), (&configured_path, "extra")] {
            std::fs::write(
                path,
                format!("---\nname: {name}\ndescription: Pi settings fixture\n---\nBody."),
            )
            .expect("write Pi skill");
        }
        let settings_path = home.join(".pi/agent/settings.json");
        std::fs::create_dir_all(settings_path.parent().expect("settings parent"))
            .expect("create settings parent");
        std::fs::write(
            &settings_path,
            serde_json::json!({
                "skills": [
                    configured_path.to_string_lossy(),
                    format!("-{}", skill_path.display())
                ]
            })
            .to_string(),
        )
        .expect("write Pi settings");
        let settings = std::fs::read_to_string(&settings_path).expect("read Pi settings");
        assert!(
            !pi_skill_enabled_by_settings(&settings, &settings_path, &skill_path, &home),
            "the official exact -path override should disable the target"
        );

        let ctx = AdapterContext {
            user_home: home,
            project_root: None,
            project_cwd: None,
            extra_roots: vec![],
        };
        let overrides = SkillConfigOverrides::preload(&PiAdapter, &ctx);
        assert_eq!(overrides.pi_settings.len(), 1);
        assert_eq!(overrides.pi_settings[0].0, Scope::AgentGlobal);
        let native_root = PiAdapter
            .roots(&ctx)
            .into_iter()
            .find(|root| root.path.ends_with(".pi/agent/skills"))
            .expect("native root");
        assert_eq!(native_root.scope, Scope::AgentGlobal);
        let parsed = PiAdapter
            .parse(&skill_path.canonicalize().expect("canonical skill"))
            .expect("parse skill");
        assert!(!pi_skill_enabled_by_settings(
            &overrides.pi_settings[0].2,
            &overrides.pi_settings[0].1,
            &parsed.path,
            &ctx.user_home,
        ));
        assert!(overrides.is_disabled(&ctx, &native_root, &parsed));

        let report = scan_agent(&PiAdapter, &ctx).expect("scan succeeds");
        let review = report
            .instances
            .iter()
            .find(|skill| skill.name == "review")
            .expect("native skill remains visible as disabled");
        assert_eq!(review.state, SkillState::Disabled);
        assert!(!review.enabled);
        assert!(report.instances.iter().any(|skill| skill.name == "extra"));
        assert!(report
            .skipped_roots
            .iter()
            .all(|root| !root.to_string_lossy().starts_with('-')));

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
        write_openclaw_skill(&home.join(".openclaw/skills"), "managed-global");
        let personal_global_path =
            write_openclaw_skill(&home.join(".agents/skills"), "managed-global");
        write_openclaw_skill(&home.join(".agents/skills"), "personal-shared");
        let grouped_workspace_path =
            write_openclaw_skill(&workspace.join("skills/group/organized"), "workspace-local");
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

        assert_eq!(report.instances.len(), 4);
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
            personal_global_path,
            "OpenClaw personal .agents root wins over managed ~/.openclaw/skills"
        );
        assert_eq!(
            report
                .instances
                .iter()
                .find(|skill| skill.name == "workspace-local")
                .expect("grouped workspace skill")
                .path,
            grouped_workspace_path,
            "OpenClaw discovers grouped skills recursively within its six-level bound"
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
        std::fs::write(
            home.join(".openclaw/openclaw.json"),
            serde_json::json!({
                "agents": {
                    "defaults": {"skills": ["workspace-agents"]},
                    "list": [{
                        "id": "main",
                        "workspace": workspace,
                        "skills": ["workspace-local"]
                    }]
                }
            })
            .to_string(),
        )
        .expect("write workspace-specific OpenClaw allowlist");

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
        assert!(by_name
            .get("workspace-local")
            .is_some_and(|skill| skill.enabled));
        assert!(by_name
            .get("workspace-agents")
            .is_some_and(|skill| !skill.enabled && skill.state == SkillState::Disabled));
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
