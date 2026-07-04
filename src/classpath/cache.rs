use crate::classpath::AutoConfigBean;
use std::path::{Path, PathBuf};

/// Compute a deterministic cache key from the project's build files.
///
/// Hashes the *contents* of recognized build files so that the cache is
/// invalidated when dependencies change.
pub fn compute_cache_key(project_path: &Path) -> String {
    use std::hash::{Hash, Hasher};
    let mut hasher = std::collections::hash_map::DefaultHasher::new();

    let build_files = [
        "build.gradle.kts",
        "build.gradle",
        "settings.gradle.kts",
        "settings.gradle",
        "gradle/libs.versions.toml",
        "pom.xml",
    ];

    for file in &build_files {
        let path = project_path.join(file);
        if path.exists() {
            if let Ok(content) = std::fs::read_to_string(&path) {
                content.hash(&mut hasher);
            }
        }
    }

    format!("{:x}", hasher.finish())
}

/// Path to the on-disk cache file for a given key.
fn cache_path(key: &str) -> PathBuf {
    let home = std::env::var("HOME").unwrap_or_default();
    let dir = PathBuf::from(&home).join(".doctor/cache");
    let _ = std::fs::create_dir_all(&dir);
    dir.join(format!("auto-config-beans-{key}.json"))
}

/// Load cached auto-configuration bean definitions, if available.
pub fn load_cache(key: &str) -> Option<Vec<AutoConfigBean>> {
    let path = cache_path(key);
    if path.exists() {
        if let Ok(content) = std::fs::read_to_string(&path) {
            if let Ok(beans) = serde_json::from_str(&content) {
                return Some(beans);
            }
        }
    }
    None
}

/// Persist auto-configuration bean definitions to the cache.
pub fn save_cache(key: &str, beans: &[AutoConfigBean]) -> std::io::Result<()> {
    let path = cache_path(key);
    let json = serde_json::to_string_pretty(beans)?;
    std::fs::write(&path, json)
}
