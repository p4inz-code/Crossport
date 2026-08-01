/* ==========================================================================
 * Transfer commands (placeholder)
 * ========================================================================== */

use crate::errors::{AppError, AppResult};
use crate::models::TransferJob;

/// Starts a transfer between two paths. Placeholder.
#[tauri::command]
pub fn start_transfer(_source: String, _destination: String) -> AppResult<TransferJob> {
  Err(AppError::NotImplemented)
}

/// Cancels an in-flight transfer. Placeholder.
#[tauri::command]
pub fn cancel_transfer(_transfer_id: String) -> AppResult<()> {
  Err(AppError::NotImplemented)
}
