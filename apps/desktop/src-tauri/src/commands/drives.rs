/* ==========================================================================
 * Drive and volume commands
 * Enumeration probes the filesystem and the Win32 volume APIs for every
 * detected volume, so it runs on the blocking pool: the event loop never
 * waits on a slow or disconnected volume.
 * ========================================================================== */

use crate::commands::run_blocking;
use crate::platform::{drives, DriveInfo};

/// Enumerates the storage volumes the user can currently reach, with the
/// metadata the platform can report for each one.
#[tauri::command]
pub async fn list_drives() -> Vec<DriveInfo> {
    run_blocking("list_drives", || Ok(drives::list_drives()))
        .await
        .unwrap_or_else(|error| {
            // Enumeration is infallible by design; reaching this point means
            // the blocking task itself failed. An empty list is the honest
            // answer, and the failure is logged rather than hidden.
            log::error!("volume enumeration failed: {error}");
            Vec::new()
        })
}
