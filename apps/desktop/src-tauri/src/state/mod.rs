/* ==========================================================================
 * State module
 * Managed application state registered with Tauri and shared with commands
 * via `tauri::State`. Holds immutable config, the resolved platform, and the
 * current settings.
 * ========================================================================== */

use std::sync::{Arc, Mutex};

use crate::archive::TransferArchive;
use crate::config::AppConfig;
use crate::errors::{AppError, AppResult};
use crate::platform::Platform;
use crate::settings::AppSettings;
use crate::transfer::TransferEngine;

/// Application state managed by the Tauri runtime.
pub struct AppState {
    pub config: AppConfig,
    /// Resolved once at startup; platform checks never re-read the environment.
    pub platform: Platform,
    /// The transfer queue. Cheap to clone: every clone shares one queue and
    /// one set of workers.
    pub transfers: TransferEngine,
    settings: Mutex<AppSettings>,
    /// History, interrupted-transfer state, and recovery. Installed during
    /// setup, once the platform directories are known; until then every
    /// archive command reports a structured error instead of guessing at a
    /// location.
    archive: Mutex<Option<Arc<TransferArchive>>>,
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            platform: Platform::current(),
            transfers: TransferEngine::default(),
            settings: Mutex::new(AppSettings::default()),
            archive: Mutex::new(None),
        }
    }
}

impl AppState {
    /// Returns a copy of the current settings.
    pub fn settings_snapshot(&self) -> AppResult<AppSettings> {
        self.settings
            .lock()
            .map(|guard| guard.clone())
            .map_err(|_| AppError::Internal("settings lock poisoned".to_string()))
    }

    /// Replaces the current settings.
    pub fn replace_settings(&self, next: AppSettings) -> AppResult<()> {
        let mut guard = self
            .settings
            .lock()
            .map_err(|_| AppError::Internal("settings lock poisoned".to_string()))?;
        *guard = next;
        Ok(())
    }

    /// Installs the durable archive. Called once during application setup.
    pub fn install_archive(&self, archive: Arc<TransferArchive>) -> AppResult<()> {
        let mut guard = self
            .archive
            .lock()
            .map_err(|_| AppError::Internal("archive lock poisoned".to_string()))?;
        *guard = Some(archive);
        Ok(())
    }

    /// The durable archive, or a structured error while it is unavailable.
    ///
    /// Shared rather than borrowed so a command can drop the state lock before
    /// it does any filesystem work.
    pub fn archive(&self) -> AppResult<Arc<TransferArchive>> {
        self.archive
            .lock()
            .map_err(|_| AppError::Internal("archive lock poisoned".to_string()))?
            .clone()
            .ok_or_else(|| {
                AppError::Internal(
                    "the transfer archive is not available yet; it is opened at startup"
                        .to_string(),
                )
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::settings::AppSettings;

    #[test]
    fn starts_with_default_settings() {
        let state = AppState::default();

        assert_eq!(
            state.settings_snapshot().expect("lock is available"),
            AppSettings::default()
        );
    }

    #[test]
    fn replace_settings_is_visible_to_the_next_snapshot() {
        let state = AppState::default();
        let next = AppSettings {
            theme: "dark".to_string(),
            locale: "de".to_string(),
            ..AppSettings::default()
        };

        state
            .replace_settings(next.clone())
            .expect("lock is available");

        assert_eq!(state.settings_snapshot().expect("lock is available"), next);
    }

    #[test]
    fn resolved_platform_matches_the_host() {
        assert_eq!(AppState::default().platform, Platform::current());
    }

    #[test]
    fn starts_with_an_empty_transfer_queue() {
        let state = AppState::default();

        assert!(state.transfers.snapshots().is_empty());

        state.transfers.shutdown();
    }

    #[test]
    fn the_archive_is_unavailable_until_it_is_installed() {
        let state = AppState::default();

        let error = state
            .archive()
            .expect_err("an archive command before startup must fail cleanly");
        assert_eq!(error.code(), "internal");
        assert!(error.to_string().contains("not available yet"));

        let workspace = crate::filesystem::test_support::unique_temp_dir("state-archive");
        let (archive, _, _) = crate::archive::TransferArchive::open(
            &workspace,
            crate::history::DEFAULT_HISTORY_LIMIT,
        );
        state
            .install_archive(Arc::clone(&archive))
            .expect("installing works");
        assert!(Arc::ptr_eq(
            &state.archive().expect("the archive is available"),
            &archive
        ));

        state.transfers.shutdown();
        let _ = std::fs::remove_dir_all(workspace);
    }
}
