/* ==========================================================================
 * Drive commands
 * Async on purpose: enumeration probes the filesystem, which must never block
 * the Tauri event loop.
 * ========================================================================== */

use crate::platform::drives::{self, DriveInfo};

/// Enumerates the storage roots the user can currently reach.
#[tauri::command]
pub async fn list_drives() -> Vec<DriveInfo> {
    drives::list_drives()
}
