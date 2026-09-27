/* ==========================================================================
 * Settings commands
 * The single owner of user preferences across the IPC boundary: the backend
 * validates, persists, and mirrors settings into managed state.
 * ========================================================================== */

use tauri::State;

use crate::errors::AppResult;
use crate::settings::{self, AppSettings};
use crate::state::AppState;

/// Returns the current application settings.
#[tauri::command]
pub fn get_settings(state: State<'_, AppState>) -> AppResult<AppSettings> {
    state.settings_snapshot()
}

/// Validates and persists updated application settings.
///
/// The retention bound takes effect immediately: shrinking it prunes the
/// oldest history entries rather than waiting for the next transfer, so the
/// count the settings page shows is the count the user has. A failure to apply
/// it is logged rather than hidden — the settings themselves are already
/// stored, and the archive will apply the bound when it next records a
/// transfer.
#[tauri::command]
pub fn update_settings(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> AppResult<()> {
    settings.validate()?;
    settings::save(&app, &settings)?;
    state.replace_settings(settings.clone())?;

    match state.archive() {
        Ok(archive) => {
            if let Err(error) = archive.set_history_limit(settings.history_limit) {
                log::warn!("could not apply the history retention limit: {error}");
            }
        }
        Err(error) => log::warn!("could not reach the transfer archive: {error}"),
    }

    Ok(())
}
