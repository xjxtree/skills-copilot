use std::{
    collections::BTreeSet,
    fs,
    path::{Component, Path, PathBuf},
};

use serde::{Deserialize, Serialize};
use skills_copilot_core::{AdapterContext, AgentAdapter};

use super::{supported_scan_adapters, tool_global_staging_skills_root};

pub const MAX_AUTHORIZED_FILE_WATCH_ROOTS: usize = 256;

/// A bounded set of existing local directories that a native client may watch.
///
/// The paths are protocol-internal capability data. Clients must never log or
/// render them and must treat filesystem events as invalidation signals only.
#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq, Eq)]
pub struct AuthorizedFileWatchPlan {
    pub roots: Vec<PathBuf>,
    pub total_count: usize,
    pub truncated: bool,
}

pub fn authorized_file_watch_plan(
    ctx: &AdapterContext,
    app_data_dir: &Path,
) -> AuthorizedFileWatchPlan {
    let mut candidates = Vec::new();
    for adapter in supported_scan_adapters() {
        collect_adapter_candidates(adapter.as_ref(), ctx, &mut candidates);
    }
    candidates.push(tool_global_staging_skills_root(app_data_dir));
    build_authorized_file_watch_plan(
        candidates,
        ctx,
        app_data_dir,
        MAX_AUTHORIZED_FILE_WATCH_ROOTS,
    )
}

fn collect_adapter_candidates(
    adapter: &dyn AgentAdapter,
    ctx: &AdapterContext,
    candidates: &mut Vec<PathBuf>,
) {
    candidates.extend(adapter.roots(ctx).into_iter().map(|root| root.path));
    candidates.extend(
        adapter
            .link_target_roots(ctx)
            .into_iter()
            .map(|root| root.path),
    );
    candidates.extend(
        adapter
            .config_paths(ctx)
            .into_iter()
            .filter_map(|path| path.parent().map(Path::to_path_buf)),
    );
}

fn build_authorized_file_watch_plan(
    candidates: impl IntoIterator<Item = PathBuf>,
    ctx: &AdapterContext,
    app_data_dir: &Path,
    limit: usize,
) -> AuthorizedFileWatchPlan {
    let accepted = candidates
        .into_iter()
        .filter_map(|path| accepted_watch_root(&path, ctx, app_data_dir))
        .collect::<BTreeSet<_>>();
    let total_count = accepted.len();
    let roots = accepted.into_iter().take(limit).collect::<Vec<_>>();
    AuthorizedFileWatchPlan {
        truncated: total_count > roots.len(),
        total_count,
        roots,
    }
}

fn accepted_watch_root(path: &Path, ctx: &AdapterContext, app_data_dir: &Path) -> Option<PathBuf> {
    let path = normalize_absolute_path(path)?;
    let user_home = normalize_absolute_path(&ctx.user_home)?;
    let project_root = ctx
        .project_root
        .as_deref()
        .and_then(normalize_absolute_path);
    let app_data_dir = normalize_absolute_path(app_data_dir)?;

    if is_broad_watch_root(&path, &user_home, project_root.as_deref(), &app_data_dir)
        || has_symlink_or_unreadable_component(&path)
    {
        return None;
    }

    let metadata = fs::symlink_metadata(&path).ok()?;
    (!metadata.file_type().is_symlink() && metadata.is_dir()).then_some(path)
}

fn normalize_absolute_path(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }

    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::Normal(part) => normalized.push(part),
            Component::CurDir | Component::ParentDir => return None,
        }
    }
    Some(normalized)
}

fn is_broad_watch_root(
    path: &Path,
    user_home: &Path,
    project_root: Option<&Path>,
    app_data_dir: &Path,
) -> bool {
    let normal_component_count = path
        .components()
        .filter(|component| matches!(component, Component::Normal(_)))
        .count();
    normal_component_count < 2
        || path == user_home
        || project_root.is_some_and(|root| path == root)
        || path == app_data_dir
}

fn has_symlink_or_unreadable_component(path: &Path) -> bool {
    let mut current = PathBuf::new();
    for component in path.components() {
        current.push(component.as_os_str());
        if matches!(component, Component::Prefix(_) | Component::RootDir) {
            continue;
        }
        match fs::symlink_metadata(&current) {
            Ok(metadata) if metadata.file_type().is_symlink() => return true,
            Ok(_) => {}
            Err(_) => return true,
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use std::{
        env, fs,
        path::{Path, PathBuf},
        time::{SystemTime, UNIX_EPOCH},
    };

    use skills_copilot_core::AdapterContext;

    use super::{build_authorized_file_watch_plan, MAX_AUTHORIZED_FILE_WATCH_ROOTS};

    struct TempTree {
        root: PathBuf,
    }

    impl TempTree {
        fn new(label: &str) -> Self {
            let nonce = SystemTime::now()
                .duration_since(UNIX_EPOCH)
                .expect("clock after epoch")
                .as_nanos();
            let root = env::temp_dir().join(format!(
                "agent-copilot-watch-plan-{label}-{}-{nonce}",
                std::process::id()
            ));
            fs::create_dir_all(&root).expect("create temp tree");
            let root = root.canonicalize().expect("canonicalize temp tree");
            Self { root }
        }

        fn directory(&self, relative: impl AsRef<Path>) -> PathBuf {
            let path = self.root.join(relative);
            fs::create_dir_all(&path).expect("create directory");
            path
        }

        fn context(&self) -> AdapterContext {
            AdapterContext {
                user_home: self.root.join("home"),
                project_cwd: Some(self.root.join("project/work")),
                project_root: Some(self.root.join("project")),
                extra_roots: Vec::new(),
            }
        }
    }

    impl Drop for TempTree {
        fn drop(&mut self) {
            let _ = fs::remove_dir_all(&self.root);
        }
    }

    #[test]
    fn watch_plan_keeps_only_existing_deduplicated_directories() {
        let tree = TempTree::new("dedupe");
        let accepted = tree.directory("home/.claude/skills");
        let file = tree.root.join("home/.claude/settings.json");
        fs::write(&file, "{}").expect("write file");
        let missing = tree.root.join("home/.agents/skills");
        let ctx = tree.context();

        let plan = build_authorized_file_watch_plan(
            [
                accepted.clone(),
                accepted.clone(),
                file,
                missing,
                PathBuf::from("relative"),
            ],
            &ctx,
            &tree.root.join("app-data"),
            MAX_AUTHORIZED_FILE_WATCH_ROOTS,
        );

        assert_eq!(plan.roots, vec![accepted]);
        assert_eq!(plan.total_count, 1);
        assert!(!plan.truncated);
    }

    #[test]
    fn watch_plan_rejects_home_project_app_data_and_shallow_roots() {
        let tree = TempTree::new("broad");
        let ctx = tree.context();
        fs::create_dir_all(&ctx.user_home).expect("create home");
        fs::create_dir_all(ctx.project_root.as_ref().expect("project root"))
            .expect("create project");
        let app_data = tree.directory("app-data");
        let filesystem_root = tree
            .root
            .ancestors()
            .last()
            .expect("filesystem root")
            .to_path_buf();
        let shallow_root = tree
            .root
            .ancestors()
            .find(|path| {
                path.components()
                    .filter(|component| matches!(component, std::path::Component::Normal(_)))
                    .count()
                    == 1
            })
            .expect("existing shallow root")
            .to_path_buf();

        let plan = build_authorized_file_watch_plan(
            [
                filesystem_root,
                shallow_root,
                ctx.user_home.clone(),
                ctx.project_root.clone().expect("project root"),
                app_data.clone(),
            ],
            &ctx,
            &app_data,
            MAX_AUTHORIZED_FILE_WATCH_ROOTS,
        );

        assert!(plan.roots.is_empty());
        assert_eq!(plan.total_count, 0);
    }

    #[cfg(unix)]
    #[test]
    fn watch_plan_rejects_any_symlink_component() {
        use std::os::unix::fs::symlink;

        let tree = TempTree::new("symlink");
        let target = tree.directory("target/skills");
        let link_parent = tree.directory("home/.claude");
        let link = link_parent.join("skills");
        symlink(&target, &link).expect("create symlink");
        let ctx = tree.context();

        let plan = build_authorized_file_watch_plan(
            [link],
            &ctx,
            &tree.root.join("app-data"),
            MAX_AUTHORIZED_FILE_WATCH_ROOTS,
        );

        assert!(plan.roots.is_empty());
    }

    #[test]
    fn watch_plan_is_deterministic_and_bounded() {
        let tree = TempTree::new("bounded");
        let ctx = tree.context();
        let candidates = (0..=MAX_AUTHORIZED_FILE_WATCH_ROOTS)
            .map(|index| tree.directory(format!("home/.skills/root-{index:03}")))
            .rev()
            .collect::<Vec<_>>();

        let plan = build_authorized_file_watch_plan(
            candidates,
            &ctx,
            &tree.root.join("app-data"),
            MAX_AUTHORIZED_FILE_WATCH_ROOTS,
        );

        assert_eq!(plan.roots.len(), MAX_AUTHORIZED_FILE_WATCH_ROOTS);
        assert_eq!(plan.total_count, MAX_AUTHORIZED_FILE_WATCH_ROOTS + 1);
        assert!(plan.truncated);
        assert!(plan.roots.windows(2).all(|pair| pair[0] < pair[1]));
    }
}
