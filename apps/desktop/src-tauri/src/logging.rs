/* ==========================================================================
 * Logging foundation
 * Owns the Tauri log plugin configuration and startup logging.
 * ========================================================================== */

use tauri::Manager;

/// Builds the log plugin registered on the application builder.
pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
  tauri_plugin_log::Builder::default()
    .level(log::LevelFilter::Info)
    .build()
}

/// Logs startup information after plugins are registered and state is managed.
pub fn init(app: &tauri::AppHandle) {
  let state = app.state::<crate::state::AppState>();
  log::info!(
    "{} v{} starting (theme={}, locale={})",
    state.config.app_name,
    state.config.app_version,
    state.settings.theme,
    state.settings.locale
  );
}
