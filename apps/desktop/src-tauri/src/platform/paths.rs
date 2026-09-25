/* ==========================================================================
 * Platform paths
 * Resolves the directories the application owns through Tauri's public
 * `PathResolver`. Nothing else in the backend should call `tauri::path`
 * directly, so a future target only needs to be validated here.
 * ========================================================================== */

use std::path::PathBuf;

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
}
