/* ==========================================================================
 * Filesystem commands
 * Path validation, metadata inspection, and directory listing for the
 * frontend. All filesystem access stays on the Rust side, so the webview needs
 * no filesystem permissions, and every blocking filesystem call runs on the
 * blocking pool instead of the async runtime.
 * ========================================================================== */

use crate::commands::run_blocking;
use crate::errors::AppResult;
use crate::filesystem::directory::{self, DirectoryListing};
use crate::filesystem::metadata::{self, EntryMetadata};
use crate::filesystem::path::normalize;

/// Normalizes a user-supplied path and reports its metadata.
///
/// Errors are structured: `invalid_input` for a malformed path,
/// `path_not_found` when the path does not exist.
#[tauri::command]
pub async fn inspect_path(path: String) -> AppResult<EntryMetadata> {
    run_blocking("inspect_path", move || {
        metadata::describe(&normalize(&path)?)
    })
    .await
}

/// Lists one directory for the browser surface.
///
/// The path is validated in Rust before anything is read: it must be absolute,
/// must not contain null bytes, must not escape its root, and must be an
/// existing directory. Returns entries with the metadata the platform reports,
/// in a deterministic order (directories first, then files).
#[tauri::command]
pub async fn list_directory(path: String) -> AppResult<DirectoryListing> {
    run_blocking("list_directory", move || directory::list_requested(&path)).await
}
