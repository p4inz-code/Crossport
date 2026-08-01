/* ==========================================================================
 * Config foundation
 * Static application configuration available through managed state.
 * Future phases may load user/runtime config here as well.
 * ========================================================================== */

use serde::{Deserialize, Serialize};

/// Immutable application configuration.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AppConfig {
  pub app_name: String,
  pub app_version: String,
}

impl Default for AppConfig {
  fn default() -> Self {
    Self {
      app_name: "CrossPort".to_string(),
      app_version: env!("CARGO_PKG_VERSION").to_string(),
    }
  }
}
