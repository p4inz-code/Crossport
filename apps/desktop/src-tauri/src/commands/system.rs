/* ==========================================================================
 * System commands
 * Exposes the platform abstraction to the frontend so the UI renders real
 * host facts instead of guessing from the webview.
 * ========================================================================== */

use tauri::State;

use crate::platform::SystemInfo;
use crate::state::AppState;

/// Reports the platform the backend is running on.
#[tauri::command]
pub fn get_system_info(state: State<'_, AppState>) -> SystemInfo {
    SystemInfo::current(state.platform)
}
