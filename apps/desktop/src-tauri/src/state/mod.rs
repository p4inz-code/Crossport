/* ==========================================================================
 * State module
 * Managed application state registered with Tauri and shared with commands
 * via `tauri::State`. Currently holds config and settings placeholders.
 * ========================================================================== */

use crate::config::AppConfig;
use crate::settings::AppSettings;

/// Application state managed by the Tauri runtime.
#[derive(Debug)]
pub struct AppState {
  pub config: AppConfig,
  pub settings: AppSettings,
}

impl Default for AppState {
  fn default() -> Self {
    Self {
      config: AppConfig::default(),
      settings: AppSettings::default(),
    }
  }
}
