/* ==========================================================================
 * History commands (placeholder)
 * ========================================================================== */

use crate::errors::{AppError, AppResult};
use crate::models::HistoryEntry;

/// Returns the persisted transfer history. Placeholder.
#[tauri::command]
pub fn list_history() -> AppResult<Vec<HistoryEntry>> {
  Err(AppError::NotImplemented)
}

/// Clears the persisted transfer history. Placeholder.
#[tauri::command]
pub fn clear_history() -> AppResult<()> {
  Err(AppError::NotImplemented)
}
