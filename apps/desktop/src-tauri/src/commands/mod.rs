/* ==========================================================================
 * Commands module
 * Tauri command layer. Each submodule mirrors a backend domain and is wired
 * into the invoke_handler from lib.rs. Command names are the frontend's IPC
 * contract: rename them only together with the matching service.
 * ========================================================================== */

pub mod dialog;
pub mod drives;
pub mod filesystem;
pub mod settings;
pub mod system;
pub mod transfer;

use crate::errors::{AppError, AppResult};

/// Runs a blocking filesystem operation on the blocking thread pool so the
/// async runtime and the Tauri event loop stay free while the OS walks a
/// directory or queries a volume.
///
/// A panic inside `work` is reported as an internal error instead of taking
/// the runtime down with it.
pub(crate) async fn run_blocking<T, F>(command: &'static str, work: F) -> AppResult<T>
where
    T: Send + 'static,
    F: FnOnce() -> AppResult<T> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(work)
        .await
        .map_err(|error| AppError::Internal(format!("{command} failed to run: {error}")))?
}
