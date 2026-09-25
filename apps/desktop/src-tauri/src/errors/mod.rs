/* ==========================================================================
 * Error foundation
 * Central error type for the CrossPort backend. Every command returns
 * `AppResult<T>` so failures are typed, serializable, and consistent.
 *
 * Errors are serialized to the frontend as a structured object:
 *
 *   { "code": "path_not_found", "message": "path not found: C:\\nope" }
 *
 * The `code` field is a stable machine-readable identifier mirrored by
 * `IPC_ERROR_CODES` in the frontend (`src/services/ipc.ts`). The frontend
 * switches on `code` and must never string-match `message`.
 * ========================================================================== */

use serde::ser::{Serialize, SerializeMap, Serializer};

/// Application-level error type shared across backend modules.
///
/// Keep the variants aligned with the frontend `IpcErrorCode` union: adding a
/// variant here means adding a code there.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AppError {
    /// Input received from the frontend failed validation.
    InvalidInput(String),
    /// A filesystem path does not exist.
    PathNotFound(String),
    /// A filesystem path exists but is not a directory.
    PathNotDirectory(String),
    /// The process is not allowed to access the requested resource.
    PermissionDenied(String),
    /// A platform or filesystem operation failed.
    Io(String),
    /// An internal operation failed with a contextual message.
    Internal(String),
}

impl AppError {
    /// Stable machine-readable identifier sent across the IPC boundary.
    pub fn code(&self) -> &'static str {
        match self {
            Self::InvalidInput(_) => "invalid_input",
            Self::PathNotFound(_) => "path_not_found",
            Self::PathNotDirectory(_) => "path_not_directory",
            Self::PermissionDenied(_) => "permission_denied",
            Self::Io(_) => "io",
            Self::Internal(_) => "internal",
        }
    }
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(f, "invalid input: {message}"),
            Self::PathNotFound(message) => write!(f, "path not found: {message}"),
            Self::PathNotDirectory(message) => write!(f, "not a directory: {message}"),
            Self::PermissionDenied(message) => write!(f, "permission denied: {message}"),
            Self::Io(message) => write!(f, "i/o error: {message}"),
            Self::Internal(message) => write!(f, "internal error: {message}"),
        }
    }
}

impl std::error::Error for AppError {}

/// Maps OS I/O failures onto the closest structured category.
///
/// Every mapped call site in this crate is a filesystem path operation, so an
/// `ErrorKind::NotFound` is reported as `PathNotFound` rather than a bare
/// `Io`. Unmapped kinds keep their message under the generic `io` code so the
/// frontend still receives actionable detail.
impl From<std::io::Error> for AppError {
    fn from(error: std::io::Error) -> Self {
        let message = error.to_string();
        match error.kind() {
            std::io::ErrorKind::NotFound => Self::PathNotFound(message),
            std::io::ErrorKind::PermissionDenied => Self::PermissionDenied(message),
            std::io::ErrorKind::InvalidInput => Self::InvalidInput(message),
            _ => Self::Io(message),
        }
    }
}

impl Serialize for AppError {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut map = serializer.serialize_map(Some(2))?;
        map.serialize_entry("code", self.code())?;
        map.serialize_entry("message", &self.to_string())?;
        map.end()
    }
}

/// Convenience alias used by every command signature.
pub type AppResult<T> = Result<T, AppError>;

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn serializes_as_structured_object() {
        let error = AppError::InvalidInput("unknown theme".to_string());
        let value = serde_json::to_value(&error).expect("error should serialize");
        assert_eq!(
            value,
            json!({ "code": "invalid_input", "message": "invalid input: unknown theme" })
        );
    }

    #[test]
    fn internal_errors_serialize_with_own_code() {
        let error = AppError::Internal("boom".to_string());
        let value = serde_json::to_value(&error).expect("error should serialize");
        assert_eq!(value["code"], json!("internal"));
        assert_eq!(value["message"], json!("internal error: boom"));
    }

    #[test]
    fn codes_are_stable_identifiers() {
        assert_eq!(AppError::InvalidInput("x".into()).code(), "invalid_input");
        assert_eq!(AppError::PathNotFound("x".into()).code(), "path_not_found");
        assert_eq!(
            AppError::PathNotDirectory("x".into()).code(),
            "path_not_directory"
        );
        assert_eq!(
            AppError::PermissionDenied("x".into()).code(),
            "permission_denied"
        );
        assert_eq!(AppError::Io("x".into()).code(), "io");
        assert_eq!(AppError::Internal("x".into()).code(), "internal");
    }

    #[test]
    fn codes_are_unique_per_variant() {
        let variants = [
            AppError::InvalidInput("x".into()),
            AppError::PathNotFound("x".into()),
            AppError::PathNotDirectory("x".into()),
            AppError::PermissionDenied("x".into()),
            AppError::Io("x".into()),
            AppError::Internal("x".into()),
        ];
        let mut codes: Vec<&str> = variants.iter().map(AppError::code).collect();
        codes.sort_unstable();
        let unique = codes.len();
        codes.dedup();
        assert_eq!(
            codes.len(),
            unique,
            "every error category needs its own code"
        );
    }

    #[test]
    fn maps_not_found_io_errors_to_path_not_found() {
        let error = AppError::from(std::io::Error::new(
            std::io::ErrorKind::NotFound,
            "nope.txt",
        ));
        assert_eq!(error.code(), "path_not_found");
    }

    #[test]
    fn maps_permission_io_errors_to_permission_denied() {
        let error = AppError::from(std::io::Error::new(
            std::io::ErrorKind::PermissionDenied,
            "denied",
        ));
        assert_eq!(error.code(), "permission_denied");
    }

    #[test]
    fn keeps_unmapped_io_errors_under_the_io_code() {
        let error = AppError::from(std::io::Error::new(std::io::ErrorKind::WouldBlock, "busy"));
        assert_eq!(error.code(), "io");
        assert!(error.to_string().contains("busy"));
    }
}
