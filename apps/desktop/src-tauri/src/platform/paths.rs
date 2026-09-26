/* ==========================================================================
 * Platform paths
 * Resolves the directories the application owns through Tauri's public
 * `PathResolver`. Nothing else in the backend should call `tauri::path`
 * directly, so a future target only needs to be validated here.
 * ========================================================================== */

use std::path::{Component, Path, PathBuf};

use tauri::Manager;

use crate::errors::{AppError, AppResult};

/// Application-owned directories on the current platform.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AppPaths {
    /// Per-user configuration (`%APPDATA%\com.crossport.app` on Windows).
    pub config_dir: PathBuf,
    /// Per-user logs (`%LOCALAPPDATA%\com.crossport.app\logs` on Windows).
    /// This is the directory the log plugin's `LogDir` target resolves to.
    pub log_dir: PathBuf,
}

impl AppPaths {
    /// Resolves every application directory for the running app.
    pub fn resolve(app: &tauri::AppHandle) -> AppResult<Self> {
        let resolver = app.path();
        Ok(Self {
            config_dir: resolve_one("app config directory", resolver.app_config_dir())?,
            log_dir: resolve_one("app log directory", resolver.app_log_dir())?,
        })
    }

    /// Path of a file inside the configuration directory.
    pub fn config_file(&self, file_name: &str) -> PathBuf {
        self.config_dir.join(file_name)
    }
}

fn resolve_one(what: &str, result: tauri::Result<PathBuf>) -> AppResult<PathBuf> {
    result.map_err(|error| AppError::Internal(format!("failed to resolve {what}: {error}")))
}

/// Comparison key for one path component.
///
/// Windows paths are case-insensitive, so the key is folded there; on every
/// other target the component is compared exactly. The folded form is only
/// ever used for comparison, never to build a path.
fn component_key(component: Component<'_>) -> String {
    let value = component.as_os_str().to_string_lossy().into_owned();
    if cfg!(windows) {
        value.to_lowercase()
    } else {
        value
    }
}

fn components(path: &Path) -> Vec<String> {
    path.components().map(component_key).collect()
}

/// Whether `candidate` is `parent` itself or lives inside it.
///
/// Purely lexical and component-wise: no filesystem access, and both paths must
/// already be normalized (absolute, no `.`, no `..`, no redundant separators).
/// Case-insensitive on Windows, so a path the user typed as `c:\data` still
/// matches the `C:\Data` the backend resolved.
pub fn path_contains(parent: &Path, candidate: &Path) -> bool {
    let parent = components(parent);
    let candidate = components(candidate);
    parent.len() <= candidate.len() && parent.iter().zip(candidate.iter()).all(|(a, b)| a == b)
}

/// Whether two normalized paths name the same location on this platform.
pub fn same_path(left: &Path, right: &Path) -> bool {
    components(left) == components(right)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn config_file_joins_inside_the_config_directory() {
        let paths = AppPaths {
            config_dir: PathBuf::from("root").join("config"),
            log_dir: PathBuf::from("root").join("logs"),
        };
        assert_eq!(
            paths.config_file("settings.json"),
            PathBuf::from("root").join("config").join("settings.json")
        );
    }

    #[test]
    fn config_and_log_directories_are_distinct() {
        let paths = AppPaths {
            config_dir: PathBuf::from("root").join("config"),
            log_dir: PathBuf::from("root").join("logs"),
        };
        assert_ne!(paths.config_dir, paths.log_dir);
    }

    #[test]
    fn path_contains_accepts_the_path_itself_and_its_children() {
        let base = if cfg!(windows) { "C:\\Data" } else { "/data" };
        let child = Path::new(base).join("nested").join("file.txt");

        assert!(path_contains(Path::new(base), Path::new(base)));
        assert!(path_contains(Path::new(base), &child));
        assert!(!path_contains(&child, Path::new(base)));
    }

    #[test]
    fn path_contains_rejects_sibling_prefixes() {
        let base = if cfg!(windows) { "C:\\Data" } else { "/data" };
        let sibling = if cfg!(windows) {
            "C:\\Database"
        } else {
            "/database"
        };

        assert!(
            !path_contains(Path::new(base), Path::new(sibling)),
            "a shared name prefix is not containment"
        );
    }

    #[test]
    fn same_path_follows_platform_case_rules() {
        if cfg!(windows) {
            assert!(same_path(Path::new("C:\\Data"), Path::new("c:\\data")));
            assert!(!same_path(Path::new("C:\\Data"), Path::new("C:\\Data2")));
        } else {
            assert!(same_path(Path::new("/data"), Path::new("/data")));
            assert!(!same_path(Path::new("/data"), Path::new("/Data")));
        }
    }
}
