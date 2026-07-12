use std::{
    cmp::Ordering,
    fs::{self, File},
    io::{Read, Take},
    path::{Path, PathBuf},
};

use skills_copilot_core::{AdapterRoot, RootSource, Scope};

const MAX_PUBLISHERS: usize = 64;
const MAX_PACKAGES_PER_PUBLISHER: usize = 512;
const MAX_VERSIONS_PER_PACKAGE: usize = 32;
const MAX_MANIFEST_BYTES: u64 = 1024 * 1024;

pub(super) fn cached_plugin_skill_roots(codex_home: &Path) -> Vec<AdapterRoot> {
    let cache_root = codex_home.join("plugins/cache");
    let Ok(canonical_cache) = cache_root.canonicalize() else {
        return Vec::new();
    };

    let mut roots = Vec::new();
    for publisher in bounded_child_directories(&canonical_cache, MAX_PUBLISHERS) {
        for package in bounded_child_directories(&publisher, MAX_PACKAGES_PER_PUBLISHER) {
            let mut versions = bounded_child_directories(&package, MAX_VERSIONS_PER_PACKAGE);
            versions.sort_by(|left, right| version_path_cmp(right, left));
            if let Some(path) = versions
                .iter()
                .find_map(|version| cached_plugin_skills_root(&canonical_cache, version))
            {
                roots.push(AdapterRoot {
                    scope: Scope::AgentGlobal,
                    path,
                    source: RootSource::Plugin,
                });
            }
        }
    }
    roots
}

fn bounded_child_directories(parent: &Path, limit: usize) -> Vec<PathBuf> {
    let Ok(entries) = fs::read_dir(parent) else {
        return Vec::new();
    };
    let mut paths = entries
        .filter_map(Result::ok)
        .filter(|entry| {
            let name = entry.file_name();
            let Some(name) = name.to_str() else {
                return false;
            };
            !name.starts_with('.') && entry.file_type().is_ok_and(|kind| kind.is_dir())
        })
        .map(|entry| entry.path())
        .collect::<Vec<_>>();
    paths.sort();
    paths.truncate(limit);
    paths
}

fn cached_plugin_skills_root(cache_root: &Path, package_root: &Path) -> Option<PathBuf> {
    let canonical_package = package_root.canonicalize().ok()?;
    if !canonical_package.starts_with(cache_root) {
        return None;
    }

    let manifest_path = canonical_package.join(".codex-plugin/plugin.json");
    let content = read_bounded_manifest(&manifest_path)?;
    let manifest = serde_json::from_str::<serde_json::Value>(&content).ok()?;
    let skills = manifest
        .get("skills")
        .and_then(serde_json::Value::as_str)
        .unwrap_or("./skills/");
    super::resolve_local_marketplace_path(&canonical_package, skills)
        .filter(|path| path.starts_with(&canonical_package) && path.is_dir())
}

fn read_bounded_manifest(path: &Path) -> Option<String> {
    let file = File::open(path).ok()?;
    if file.metadata().ok()?.len() > MAX_MANIFEST_BYTES {
        return None;
    }
    let mut content = String::new();
    let mut bounded: Take<File> = file.take(MAX_MANIFEST_BYTES + 1);
    bounded.read_to_string(&mut content).ok()?;
    if content.len() as u64 > MAX_MANIFEST_BYTES {
        None
    } else {
        Some(content)
    }
}

fn version_path_cmp(left: &Path, right: &Path) -> Ordering {
    let left = left
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    let right = right
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("");
    natural_version_cmp(left, right).then_with(|| left.cmp(right))
}

fn natural_version_cmp(left: &str, right: &str) -> Ordering {
    let mut left = left.split(|character: char| !character.is_ascii_alphanumeric());
    let mut right = right.split(|character: char| !character.is_ascii_alphanumeric());
    loop {
        match (left.next(), right.next()) {
            (Some(left), Some(right)) => {
                let ordering = match (left.parse::<u64>(), right.parse::<u64>()) {
                    (Ok(left), Ok(right)) => left.cmp(&right),
                    _ => left.cmp(right),
                };
                if ordering != Ordering::Equal {
                    return ordering;
                }
            }
            (Some(_), None) => return Ordering::Greater,
            (None, Some(_)) => return Ordering::Less,
            (None, None) => return Ordering::Equal,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::natural_version_cmp;
    use std::cmp::Ordering;

    #[test]
    fn compares_numeric_version_components() {
        assert_eq!(natural_version_cmp("1.10.0", "1.9.0"), Ordering::Greater);
        assert_eq!(
            natural_version_cmp("0.2.8-build", "0.2.8"),
            Ordering::Greater
        );
    }
}
