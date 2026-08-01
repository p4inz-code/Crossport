/* ==========================================================================
 * Drives commands (placeholder)
 * ========================================================================== */

use crate::errors::{AppError, AppResult};
use crate::models::DriveInfo;

/// Lists connected drives. Placeholder — implemented in a later phase.
#[tauri::command]
pub fn list_drives() -> AppResult<Vec<DriveInfo>> {
  Err(AppError::NotImplemented)
}
