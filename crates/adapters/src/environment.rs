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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn absolute_path_values_reject_relative_inputs_and_normalize_parent_components() {
        assert_eq!(absolute_path_value(Path::new("relative/path")), None);
        #[cfg(not(windows))]
        let (input, expected) = (
            Path::new("/tmp/adapter-root/../skills"),
            PathBuf::from("/tmp/skills"),
        );
        #[cfg(windows)]
        let (input, expected) = (
            Path::new(r"C:\tmp\adapter-root\..\skills"),
            PathBuf::from(r"C:\tmp\skills"),
        );
        assert_eq!(absolute_path_value(input), Some(expected));
    }

    #[test]
    fn local_path_expansion_is_lexically_normalized_against_its_declared_base() {
        let home = Path::new("/tmp/home");
        let base = Path::new("/tmp/project/.agent");

        assert_eq!(
            expand_local_path("~/skills/../shared", home, base),
            Some(PathBuf::from("/tmp/home/shared"))
        );
        assert_eq!(
            expand_local_path("./skills/../review", home, base),
            Some(PathBuf::from("/tmp/project/.agent/review"))
        );
        assert_eq!(
            expand_local_path("/tmp/global/../shared", home, base),
            Some(PathBuf::from("/tmp/shared"))
        );
    }

    #[test]
    fn malformed_environment_tokens_never_become_local_paths() {
        let home = Path::new("/tmp/home");
        let base = Path::new("/tmp/project");

        assert_eq!(expand_local_path("${UNCLOSED", home, base), None);
        assert_eq!(expand_local_path("${BAD-NAME}", home, base), None);
        assert_eq!(expand_local_path("${}", home, base), None);
    }

    #[test]
    fn lexical_normalization_preserves_absolute_root_when_parent_components_overflow() {
        assert_eq!(
            normalize_path_lexically(Path::new("/../../tmp/skills")),
            PathBuf::from("/tmp/skills")
        );
    }
}
