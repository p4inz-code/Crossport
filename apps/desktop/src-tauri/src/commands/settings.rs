/* ==========================================================================
 * Settings commands (placeholder)
 * ========================================================================== */

use crate::errors::{AppError, AppResult};
use crate::settings::AppSettings;

/// Returns the current application settings. Placeholder.
#[tauri::command]
pub fn get_settings() -> AppResult<AppSettings> {
  Err(AppError::NotImplemented)
}

/// Persists updated application settings. Placeholder.
#[tauri::command]
pub fn update_settings(_settings: AppSettings) -> AppResult<()> {
  Err(AppError::NotImplemented)
}
