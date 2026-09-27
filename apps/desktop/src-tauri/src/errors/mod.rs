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
    /// A transfer request was rejected before it started because the source
    /// and destination relationship is unsafe (the destination is the source,
    /// or it lives inside it).
    UnsafeRelationship(String),
    /// The destination volume cannot hold the requested transfer.
    NotEnoughSpace(String),
    /// The destination ran out of space while data was being written.
    DiskFull(String),
    /// A transfer request covers more entries than the engine will plan.
    TooManyItems(String),
    /// No transfer job with that identifier is known to the engine.
    TransferNotFound(String),
    /// A transfer job finished without completing every item.
    TransferFailed(String),
    /// A destination did not match its source, or could not be checked when the
    /// configured policy required it.
    VerificationFailed(String),
    /// Persisted state could not be written or used, so history or recovery is
    /// running degraded. The message says what was affected.
    StateUnavailable(String),
    /// An interrupted transfer cannot be restarted or discarded as asked.
    RecoveryUnavailable(String),
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
            Self::UnsafeRelationship(_) => "unsafe_relationship",
            Self::NotEnoughSpace(_) => "not_enough_space",
            Self::DiskFull(_) => "disk_full",
            Self::TooManyItems(_) => "too_many_items",
            Self::TransferNotFound(_) => "transfer_not_found",
            Self::TransferFailed(_) => "transfer_failed",
            Self::VerificationFailed(_) => "verification_failed",
            Self::StateUnavailable(_) => "state_unavailable",
            Self::RecoveryUnavailable(_) => "recovery_unavailable",
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
            Self::UnsafeRelationship(message) => {
                write!(f, "unsafe source/destination relationship: {message}")
            }
            Self::NotEnoughSpace(message) => write!(f, "not enough space: {message}"),
            Self::DiskFull(message) => write!(f, "the destination ran out of space: {message}"),
            Self::TooManyItems(message) => write!(f, "too many items: {message}"),
            Self::TransferNotFound(message) => write!(f, "transfer not found: {message}"),
            Self::TransferFailed(message) => write!(f, "transfer failed: {message}"),
            Self::VerificationFailed(message) => {
                write!(f, "verification failed: {message}")
            }
            Self::StateUnavailable(message) => write!(f, "state unavailable: {message}"),
            Self::RecoveryUnavailable(message) => {
                write!(f, "recovery unavailable: {message}")
            }
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

/// An error as it is stored on disk and sent across the wire.
///
/// `AppError` is deliberately serialize-only: it carries one variant per
/// failure category, and those variants are a backend implementation detail.
/// A document stores the two facts a reader can rely on — the stable `code`
/// and the human-readable `message` — and [`StoredError::to_error`] maps the
/// code back onto a variant when the backend needs an `AppError` again.
///
/// The serialized shape is identical to `AppError`'s, so both can travel in the
/// same payloads.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StoredError {
    pub code: String,
    pub message: String,
}

impl StoredError {
    /// Rebuilds a typed error from its stored form.
    ///
    /// A stored message is the *rendered* message — the same string the
    /// frontend displays — so the category's prefix is removed before it is
    /// handed to the variant, which puts that prefix back when displayed. The
    /// prefix is obtained by rendering the variant itself, so this cannot drift
    /// out of step with `Display`.
    ///
    /// An unrecognized code becomes [`AppError::Internal`], because a code this
    /// build does not know is a fact it cannot act on. The message is still
    /// preserved verbatim in that case.
    pub fn to_error(&self) -> AppError {
        let variant = empty_variant(&self.code);
        let prefix = variant.to_string();
        let message = match self.message.strip_prefix(&prefix) {
            Some(message) => message.to_string(),
            None => self.message.clone(),
        };

        match variant {
            AppError::InvalidInput(_) => AppError::InvalidInput(message),
            AppError::PathNotFound(_) => AppError::PathNotFound(message),
            AppError::PathNotDirectory(_) => AppError::PathNotDirectory(message),
            AppError::PermissionDenied(_) => AppError::PermissionDenied(message),
            AppError::Io(_) => AppError::Io(message),
            AppError::UnsafeRelationship(_) => AppError::UnsafeRelationship(message),
            AppError::NotEnoughSpace(_) => AppError::NotEnoughSpace(message),
            AppError::DiskFull(_) => AppError::DiskFull(message),
            AppError::TooManyItems(_) => AppError::TooManyItems(message),
            AppError::TransferNotFound(_) => AppError::TransferNotFound(message),
            AppError::TransferFailed(_) => AppError::TransferFailed(message),
            AppError::VerificationFailed(_) => AppError::VerificationFailed(message),
            AppError::StateUnavailable(_) => AppError::StateUnavailable(message),
            AppError::RecoveryUnavailable(_) => AppError::RecoveryUnavailable(message),
            AppError::Internal(_) => AppError::Internal(message),
        }
    }
}

/// The variant a stored `code` names, with an empty message.
///
/// Rendering it yields exactly the prefix `Display` adds for that category.
fn empty_variant(code: &str) -> AppError {
    match code {
        "invalid_input" => AppError::InvalidInput(String::new()),
        "path_not_found" => AppError::PathNotFound(String::new()),
        "path_not_directory" => AppError::PathNotDirectory(String::new()),
        "permission_denied" => AppError::PermissionDenied(String::new()),
        "io" => AppError::Io(String::new()),
        "unsafe_relationship" => AppError::UnsafeRelationship(String::new()),
        "not_enough_space" => AppError::NotEnoughSpace(String::new()),
        "disk_full" => AppError::DiskFull(String::new()),
        "too_many_items" => AppError::TooManyItems(String::new()),
        "transfer_not_found" => AppError::TransferNotFound(String::new()),
        "transfer_failed" => AppError::TransferFailed(String::new()),
        "verification_failed" => AppError::VerificationFailed(String::new()),
        "state_unavailable" => AppError::StateUnavailable(String::new()),
        "recovery_unavailable" => AppError::RecoveryUnavailable(String::new()),
        _ => AppError::Internal(String::new()),
    }
}

impl From<&AppError> for StoredError {
    fn from(error: &AppError) -> Self {
        Self {
            code: error.code().to_string(),
            message: error.to_string(),
        }
    }
}

impl From<AppError> for StoredError {
    fn from(error: AppError) -> Self {
        Self::from(&error)
    }
}

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
        assert_eq!(
            AppError::UnsafeRelationship("x".into()).code(),
            "unsafe_relationship"
        );
        assert_eq!(
            AppError::NotEnoughSpace("x".into()).code(),
            "not_enough_space"
        );
        assert_eq!(AppError::DiskFull("x".into()).code(), "disk_full");
        assert_eq!(AppError::TooManyItems("x".into()).code(), "too_many_items");
        assert_eq!(
            AppError::TransferNotFound("x".into()).code(),
            "transfer_not_found"
        );
        assert_eq!(
            AppError::TransferFailed("x".into()).code(),
            "transfer_failed"
        );
        assert_eq!(
            AppError::VerificationFailed("x".into()).code(),
            "verification_failed"
        );
        assert_eq!(
            AppError::StateUnavailable("x".into()).code(),
            "state_unavailable"
        );
        assert_eq!(
            AppError::RecoveryUnavailable("x".into()).code(),
            "recovery_unavailable"
        );
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
            AppError::UnsafeRelationship("x".into()),
            AppError::NotEnoughSpace("x".into()),
            AppError::DiskFull("x".into()),
            AppError::TooManyItems("x".into()),
            AppError::TransferNotFound("x".into()),
            AppError::TransferFailed("x".into()),
            AppError::VerificationFailed("x".into()),
            AppError::StateUnavailable("x".into()),
            AppError::RecoveryUnavailable("x".into()),
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
    fn every_code_round_trips_through_its_stored_form() {
        for error in [
            AppError::InvalidInput("x".into()),
            AppError::PathNotFound("x".into()),
            AppError::PathNotDirectory("x".into()),
            AppError::PermissionDenied("x".into()),
            AppError::Io("x".into()),
            AppError::UnsafeRelationship("x".into()),
            AppError::NotEnoughSpace("x".into()),
            AppError::DiskFull("x".into()),
            AppError::TooManyItems("x".into()),
            AppError::TransferNotFound("x".into()),
            AppError::TransferFailed("x".into()),
            AppError::VerificationFailed("x".into()),
            AppError::StateUnavailable("x".into()),
            AppError::RecoveryUnavailable("x".into()),
        ] {
            let stored = StoredError::from(&error);
            let restored = stored.to_error();

            assert_eq!(restored.code(), error.code(), "the code must survive");
            assert_eq!(restored.to_string(), error.to_string());
        }
    }

    #[test]
    fn a_stored_error_keeps_a_code_this_build_does_not_know() {
        let stored = StoredError {
            code: "future_category".to_string(),
            message: "something new happened".to_string(),
        };

        let error = stored.to_error();

        assert_eq!(error.code(), "internal");
        assert!(error.to_string().contains("something new happened"));
    }

    #[test]
    fn a_stored_error_serializes_like_the_error_it_came_from() {
        let error = AppError::DiskFull("D:\\ is full".to_string());

        assert_eq!(
            serde_json::to_value(StoredError::from(&error)).expect("stored errors serialize"),
            serde_json::to_value(&error).expect("errors serialize")
        );
    }

    #[test]
    fn keeps_unmapped_io_errors_under_the_io_code() {
        let error = AppError::from(std::io::Error::new(std::io::ErrorKind::WouldBlock, "busy"));
        assert_eq!(error.code(), "io");
        assert!(error.to_string().contains("busy"));
    }
}
