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
#[tauri::command]
pub fn update_settings(
    app: tauri::AppHandle,
    state: State<'_, AppState>,
    settings: AppSettings,
) -> AppResult<()> {
    settings.validate()?;
    settings::save(&app, &settings)?;
    state.replace_settings(settings)
}
