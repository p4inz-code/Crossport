/* ==========================================================================
 * Dialog commands
 * Hosts the native folder picker. The dialog runs in Rust and the webview
 * never receives dialog or filesystem plugin permissions; the command returns
 * the selected folder only after it has been normalized and validated.
 * ========================================================================== */

use tauri_plugin_dialog::DialogExt;

use crate::errors::{AppError, AppResult};
use crate::filesystem::path::must_be_directory;

/// Opens the native folder picker.
///
/// Returns the validated absolute path, or `None` when the user cancelled.
#[tauri::command]
pub async fn pick_directory(app: tauri::AppHandle) -> AppResult<Option<String>> {
    let (sender, receiver) = std::sync::mpsc::channel();

    // Non-blocking variant: the plugin dispatches the dialog to the main thread
    // itself and fires the callback from its own thread, so the command never
    // blocks the event loop or the main thread.
    app.dialog()
        .file()
        .set_title("Select a folder")
        .pick_folder(move |selection| {
            let _ = sender.send(selection);
        });

    let selection = tauri::async_runtime::spawn_blocking(move || receiver.recv().ok().flatten())
        .await
        .map_err(|error| AppError::Internal(format!("folder dialog failed: {error}")))?;

    let Some(selection) = selection else {
        return Ok(None);
    };

    let selected_path = selection.into_path().map_err(|error| {
        AppError::InvalidInput(format!("unsupported folder selection: {error}"))
    })?;

    let validated = must_be_directory(selected_path)?;
    Ok(Some(validated.display().to_string()))
}
