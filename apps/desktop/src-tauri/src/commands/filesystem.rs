/* ==========================================================================
 * Filesystem commands
 * Path validation and metadata inspection for the frontend. All filesystem
 * access stays on the Rust side, so the webview needs no filesystem
 * permissions.
 * ========================================================================== */

use crate::errors::AppResult;
use crate::filesystem::metadata::{self, EntryMetadata};
use crate::filesystem::path::normalize;

/// Normalizes a user-supplied path and reports its metadata.
///
/// Errors are structured: `invalid_input` for a malformed path,
/// `path_not_found` when the path does not exist.
#[tauri::command]
pub async fn inspect_path(path: String) -> AppResult<EntryMetadata> {
    metadata::describe(&normalize(&path)?)
}
