/* ==========================================================================
 * Settings module (placeholder)
 * User-configurable settings that will be persisted and exposed via commands
 * in a later phase. Only the type surface is defined for now.
 * ========================================================================== */

use serde::{Deserialize, Serialize};

/// User preferences. Field names map to the frontend `AppSettings` shape.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct AppSettings {
  pub theme: String,
  pub locale: String,
}

impl Default for AppSettings {
  fn default() -> Self {
    Self {
      theme: "system".to_string(),
      locale: "en".to_string(),
    }
  }
}
