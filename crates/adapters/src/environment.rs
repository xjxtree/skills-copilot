use std::{
    env,
    path::{Component, Path, PathBuf},
};

pub(crate) fn absolute_env_path(name: &str) -> Option<PathBuf> {
    let value = env::var_os(name)?;
    absolute_path_value(Path::new(&value))
}

pub(crate) fn absolute_path_value(path: &Path) -> Option<PathBuf> {
    path.is_absolute().then(|| normalize_path_lexically(path))
}

pub(crate) fn expand_local_path(
    raw: &str,
    user_home: &Path,
    relative_base: &Path,
) -> Option<PathBuf> {
    let expanded = expand_environment_variables(raw.trim())?;
    if expanded.is_empty() {
        return None;
    }
    let path = if expanded == "~" {
        user_home.to_path_buf()
    } else if let Some(rest) = expanded.strip_prefix("~/") {
        user_home.join(rest)
    } else {
        let path = PathBuf::from(expanded);
        if path.is_absolute() {
            path
        } else {
            relative_base.join(path)
        }
    };
    Some(normalize_path_lexically(&path))
}

fn expand_environment_variables(raw: &str) -> Option<String> {
    let mut output = String::with_capacity(raw.len());
    let mut rest = raw;
    while let Some(start) = rest.find("${") {
        output.push_str(&rest[..start]);
        let suffix = &rest[start + 2..];
        let end = suffix.find('}')?;
        let name = &suffix[..end];
        if name.is_empty()
            || !name
                .bytes()
                .all(|byte| byte.is_ascii_alphanumeric() || byte == b'_')
        {
            return None;
        }
        let value = env::var(name).ok()?;
        output.push_str(&value);
        rest = &suffix[end + 1..];
    }
    output.push_str(rest);
    Some(output)
}

pub(crate) fn normalize_path_lexically(path: &Path) -> PathBuf {
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

pub(crate) fn env_flag(name: &str) -> bool {
    env::var(name).is_ok_and(|value| {
        matches!(
            value.trim().to_ascii_lowercase().as_str(),
            "1" | "true" | "yes" | "on"
        )
    })
}
