/* ==========================================================================
 * State module
 * Managed application state registered with Tauri and shared with commands
 * via `tauri::State`. Holds immutable config, the resolved platform, and the
 * current settings.
 * ========================================================================== */

use std::sync::Mutex;

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
}

impl Default for AppState {
    fn default() -> Self {
        Self {
            config: AppConfig::default(),
            platform: Platform::current(),
            transfers: TransferEngine::default(),
            settings: Mutex::new(AppSettings::default()),
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
}
