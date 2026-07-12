use std::path::{Component, Path, PathBuf};

use skills_copilot_core::AdapterContext;

pub fn codex_home_dir(ctx: &AdapterContext) -> PathBuf {
    resolved_codex_home(ctx, std::env::var_os("CODEX_HOME").map(PathBuf::from))
}

pub(crate) fn resolved_codex_home(ctx: &AdapterContext, override_path: Option<PathBuf>) -> PathBuf {
    let default = ctx.user_home.join(".codex");
    let Some(home) = normalize_absolute(&ctx.user_home) else {
        return default;
    };
    override_path
        .and_then(|path| normalize_absolute(&path))
        .filter(|path| path.starts_with(&home))
        .unwrap_or(default)
}

fn normalize_absolute(path: &Path) -> Option<PathBuf> {
    if !path.is_absolute() {
        return None;
    }
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(component.as_os_str()),
            Component::CurDir => {}
            Component::ParentDir => {
                if !normalized.pop() {
                    return None;
                }
            }
            Component::Normal(name) => normalized.push(name),
        }
    }
    Some(normalized)
}
