/* ==========================================================================
 * Error foundation
 * Central error type for the CrossPort backend. Every command returns
 * `AppResult<T>` so failures are typed, serializable, and consistent.
 * ========================================================================== */

use serde::ser::{Serialize, Serializer};

/// Application-level error type shared across backend modules.
///
/// `InvalidInput` and `Internal` are reserved variants of the designed error
/// surface; they are not constructed until business logic lands in a later
/// phase. Kept now so downstream modules never need to reshape this type.
#[derive(Debug, Clone, PartialEq, Eq)]
#[allow(dead_code)]
pub enum AppError {
  /// The requested capability is a placeholder and not yet implemented.
  NotImplemented,
  /// Input received from the frontend failed validation.
  InvalidInput(String),
  /// An internal operation failed with a contextual message.
  Internal(String),
}

impl std::fmt::Display for AppError {
  fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
    match self {
      Self::NotImplemented => write!(f, "not implemented"),
      Self::InvalidInput(message) => write!(f, "invalid input: {message}"),
      Self::Internal(message) => write!(f, "internal error: {message}"),
    }
  }
}

impl std::error::Error for AppError {}

impl Serialize for AppError {
  fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
    serializer.serialize_str(&self.to_string())
  }
}

/// Convenience alias used by every command signature.
pub type AppResult<T> = Result<T, AppError>;
