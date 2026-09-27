/* ==========================================================================
 * Persistence foundation
 * Every document CrossPort keeps on disk (transfer state, transfer history)
 * goes through this module, so durability, versioning, and corruption
 * behaviour are decided once instead of per caller.
 *
 * The rules these functions exist to enforce:
 *
 * - A document is written to a temporary file, flushed to the device, and
 *   only then renamed over the live file. A crash at any point leaves either
 *   the previous document or the new one, never a half-written one.
 * - Every document carries a `schemaVersion`. Reading an older document is a
 *   migration; reading a newer one is refused, because an older build must not
 *   overwrite data a newer build wrote.
 * - A missing file is normal and yields the caller's default. An empty,
 *   malformed, or unreadable file is preserved beside the original for
 *   diagnostics and reported, never silently discarded and never fatal.
 *
 * Documents are JSON: the application's data is small, human-inspectable
 * state, and a database would add a dependency and a failure mode without
 * buying anything the transfer engine needs.
 * ========================================================================== */

use std::path::{Path, PathBuf};

use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use serde_json::{Map, Value};

use crate::errors::{AppError, AppResult};

/// Current on-disk schema version for every document in this application.
///
/// A document written by a faster-moving build can be newer than this; see
/// [`LoadStatus::Unsupported`].
pub const SCHEMA_VERSION: u32 = 1;

/// Key every document stores its version under.
const VERSION_KEY: &str = "schemaVersion";

/// How a document was obtained.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum LoadStatus {
    /// The file was read and parsed as the current schema.
    Loaded,
    /// No file exists yet. Expected on a first run.
    Missing,
    /// The file was written by an older schema and migrated on read.
    Migrated { from: u32 },
    /// The file was unusable and its default value was returned instead. The
    /// original was preserved beside it.
    Recovered { detail: String },
    /// The file was written by a newer schema. Its contents are untouched and
    /// must not be overwritten.
    Unsupported { detail: String },
}

impl LoadStatus {
    /// Whether this document may be written back to disk.
    ///
    /// Only a newer, unrecognized schema forbids writing: the application has
    /// no idea what is in that file, so it must not replace it.
    pub fn writable(&self) -> bool {
        !matches!(self, Self::Unsupported { .. })
    }

    /// The kind of outcome, without the detail string.
    pub fn state(&self) -> LoadState {
        match self {
            Self::Loaded => LoadState::Loaded,
            Self::Missing => LoadState::Missing,
            Self::Migrated { .. } => LoadState::Migrated,
            Self::Recovered { .. } => LoadState::Recovered,
            Self::Unsupported { .. } => LoadState::Unsupported,
        }
    }

    /// The backend's own words about an outcome that needs explaining.
    pub fn detail(&self) -> Option<String> {
        match self {
            Self::Loaded | Self::Missing => None,
            Self::Migrated { from } => Some(format!(
                "the document was written by schema version {from} and was migrated"
            )),
            Self::Recovered { detail } | Self::Unsupported { detail } => Some(detail.clone()),
        }
    }

    /// One identifier of the outcome, for the frontend contract.
    pub fn as_str(&self) -> &'static str {
        self.state().as_str()
    }
}

/// How a durable document was obtained, as a value the frontend can switch on.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LoadState {
    /// Read and parsed as the current schema.
    Loaded,
    /// No file exists yet; the caller's default applies.
    Missing,
    /// Written by an older schema and migrated on read.
    Migrated,
    /// Unusable, preserved for diagnostics, and replaced by the default value.
    Recovered,
    /// Written by a newer schema: read-only until that build is used.
    Unsupported,
}

impl LoadState {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Loaded => "loaded",
            Self::Missing => "missing",
            Self::Migrated => "migrated",
            Self::Recovered => "recovered",
            Self::Unsupported => "unsupported",
        }
    }

    /// Whether the user should be told something about this document.
    pub fn is_noteworthy(self) -> bool {
        matches!(self, Self::Recovered | Self::Unsupported)
    }
}

/// One loaded document, plus how it was obtained.
#[derive(Debug, Clone)]
pub struct LoadOutcome<T> {
    pub value: T,
    pub status: LoadStatus,
}

impl<T> LoadOutcome<T> {
    /// Whether the document may be saved back to the same path.
    pub fn writable(&self) -> bool {
        self.status.writable()
    }
}

/// Migrates a document body from `from` to [`SCHEMA_VERSION`].
///
/// Receives the parsed object rather than bytes so a migration can inspect and
/// rewrite individual fields. Returning `Err` means the document cannot be
/// understood, which is reported exactly like malformed input: the value falls
/// back to the caller's default and the original file is preserved.
pub type Migrator =
    fn(from: u32, document: Map<String, Value>) -> Result<Map<String, Value>, String>;

/// The default migrator: accepts any older version and lets `serde(default)`
/// fill in fields the older document did not have.
///
/// That is the honest migration story for additive changes, which is what this
/// application's documents have had so far.
pub fn migrate_fill_defaults(
    _from: u32,
    document: Map<String, Value>,
) -> Result<Map<String, Value>, String> {
    Ok(document)
}

/// Reads a versioned JSON document from `path`.
///
/// Never fails: a caller that cannot load its document still has to start.
/// Every outcome that is not [`LoadStatus::Loaded`], [`LoadStatus::Missing`],
/// or [`LoadStatus::Migrated`] is logged and described in the returned status
/// so the frontend can tell the user what happened.
pub fn read_document<T>(path: &Path, migrate: Migrator) -> LoadOutcome<T>
where
    T: DeserializeOwned + Default,
{
    let contents = match std::fs::read_to_string(path) {
        Ok(contents) => contents,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return LoadOutcome {
                value: T::default(),
                status: LoadStatus::Missing,
            }
        }
        Err(error) => {
            // Unreadable is not the same as absent, but it is not fatal either:
            // a locked or permission-denied file must not stop the app.
            return recover(
                path,
                T::default(),
                format!("the file could not be read: {error}"),
            );
        }
    };

    if contents.trim().is_empty() {
        return recover(path, T::default(), "the file was empty".to_string());
    }

    let mut document: Value = match serde_json::from_str(&contents) {
        Ok(document) => document,
        Err(error) => {
            return recover(
                path,
                T::default(),
                format!("the file is not valid JSON: {error}"),
            )
        }
    };

    let Some(object) = document.as_object_mut() else {
        return recover(
            path,
            T::default(),
            "the file does not contain a JSON object".to_string(),
        );
    };

    // A document without a version predates versioning, which is version 0.
    let version = match object.remove(VERSION_KEY) {
        Some(Value::Number(number)) => match number.as_u64() {
            Some(version) => version as u32,
            None => {
                return recover(
                    path,
                    T::default(),
                    format!("the schema version is not a whole number: {number}"),
                )
            }
        },
        None => 0,
        Some(other) => {
            return recover(
                path,
                T::default(),
                format!("the schema version is not a number: {other}"),
            )
        }
    };

    if version > SCHEMA_VERSION {
        let detail = format!(
            "the file uses schema version {version}, but this build understands at most {SCHEMA_VERSION}"
        );
        log::warn!(
            "'{}' was written by a newer build and will be left untouched ({detail})",
            path.display()
        );
        return LoadOutcome {
            value: T::default(),
            status: LoadStatus::Unsupported { detail },
        };
    }

    let body = match migrate(version, object.clone()) {
        Ok(body) => body,
        Err(detail) => {
            return recover(
                path,
                T::default(),
                format!("the document could not be migrated from version {version}: {detail}"),
            )
        }
    };

    match serde_json::from_value::<T>(Value::Object(body)) {
        Ok(value) => LoadOutcome {
            value,
            status: if version == SCHEMA_VERSION {
                LoadStatus::Loaded
            } else {
                LoadStatus::Migrated { from: version }
            },
        },
        Err(error) => recover(
            path,
            T::default(),
            format!("the document does not match the expected shape: {error}"),
        ),
    }
}

/// Writes `value` as a versioned JSON document, atomically.
///
/// The value is serialized first, then written to a sibling temporary file and
/// flushed to the device, and only then renamed over the live path. An
/// interrupted write therefore leaves the previous document intact, and a
/// reader never observes a partially written one.
pub fn write_document<T: Serialize>(path: &Path, value: &T) -> AppResult<()> {
    let mut document = match serde_json::to_value(value) {
        Ok(Value::Object(document)) => document,
        Ok(_) => {
            return Err(AppError::Internal(
                "the document must serialize to a JSON object".to_string(),
            ))
        }
        Err(error) => {
            return Err(AppError::Internal(format!(
                "failed to serialize the document: {error}"
            )))
        }
    };
    document.insert(VERSION_KEY.to_string(), Value::from(SCHEMA_VERSION));

    let contents = serde_json::to_string_pretty(&Value::Object(document)).map_err(|error| {
        AppError::Internal(format!("failed to serialize the document: {error}"))
    })?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            AppError::Internal(format!("failed to create {}: {error}", parent.display()))
        })?;
    }

    let temporary = temporary_path(path);
    write_flushed(&temporary, contents.as_bytes()).map_err(|error| {
        AppError::Internal(format!("failed to write {}: {error}", temporary.display()))
    })?;

    std::fs::rename(&temporary, path).map_err(|error| {
        // Leave nothing half-written behind when the rename is refused.
        let _ = std::fs::remove_file(&temporary);
        AppError::Internal(format!("failed to replace {}: {error}", path.display()))
    })?;

    Ok(())
}

/// Writes bytes and flushes them to the device before returning.
///
/// The flush is why this does not use `std::fs::write`: a rename is only
/// atomic with respect to a crash if the bytes it exposes are already durable.
fn write_flushed(path: &Path, bytes: &[u8]) -> std::io::Result<()> {
    use std::io::Write;

    let mut file = std::fs::File::create(path)?;
    file.write_all(bytes)?;
    file.sync_all()
}

/// Sibling path used for the atomic replacement.
///
/// The name carries the process id and a counter, so two writers replacing the
/// same document never share a temporary file. Sharing one would let the second
/// writer rename a file the first already moved away, which fails the write for
/// no reason; unique names make concurrent replacement safe, with the rename
/// still deciding the winner atomically.
fn temporary_path(path: &Path) -> PathBuf {
    static NEXT_TEMPORARY: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let serial = NEXT_TEMPORARY.fetch_add(1, std::sync::atomic::Ordering::Relaxed);

    let mut name = path.file_name().unwrap_or_default().to_os_string();
    name.push(format!(".{}-{serial}.tmp", std::process::id()));
    path.with_file_name(name)
}

/// Preserves an unusable document beside the original and reports why.
///
/// The preserved copy is what makes a corrupt file diagnosable after the fact;
/// the original path is left free for the next write to succeed.
fn recover<T>(path: &Path, value: T, detail: String) -> LoadOutcome<T> {
    log::warn!(
        "'{}' could not be used and will be preserved for inspection: {detail}",
        path.display()
    );

    let preserved = preserved_path(path);
    match std::fs::rename(path, &preserved) {
        Ok(()) => log::warn!("the unusable file was moved to '{}'", preserved.display()),
        Err(error) => log::warn!(
            "the unusable file at '{}' could not be preserved: {error}",
            path.display()
        ),
    }

    LoadOutcome {
        value,
        status: LoadStatus::Recovered { detail },
    }
}

/// Where an unusable document is preserved, e.g. `history.corrupt-1700000000000.json`.
fn preserved_path(path: &Path) -> PathBuf {
    let stamp = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| elapsed.as_millis())
        .unwrap_or(0);

    let stem = path
        .file_stem()
        .map(|stem| stem.to_string_lossy().into_owned())
        .unwrap_or_else(|| "document".to_string());
    let extension = path
        .extension()
        .map(|extension| format!(".{}", extension.to_string_lossy()))
        .unwrap_or_default();

    path.with_file_name(format!("{stem}.corrupt-{stamp}{extension}"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::test_support::unique_temp_dir;
    use serde::Deserialize;

    #[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
    #[serde(rename_all = "camelCase", default)]
    struct Sample {
        app: String,
        limit: u32,
    }

    fn sample_path(label: &str) -> (PathBuf, PathBuf) {
        let dir = unique_temp_dir(label);
        let path = dir.join("state").join("sample.json");
        (dir, path)
    }

    fn load(path: &Path) -> LoadOutcome<Sample> {
        read_document(path, migrate_fill_defaults)
    }

    #[test]
    fn a_missing_file_is_expected_and_yields_defaults() {
        let (dir, path) = sample_path("persistence-missing");

        let outcome = load(&path);

        assert_eq!(outcome.status, LoadStatus::Missing);
        assert_eq!(outcome.value, Sample::default());
        assert!(outcome.writable(), "a missing file must be writable");

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_document_round_trips_on_disk() {
        let (dir, path) = sample_path("persistence-round-trip");
        let value = Sample {
            app: "crossport".to_string(),
            limit: 200,
        };

        write_document(&path, &value).expect("the document is writable");
        let outcome = load(&path);

        assert_eq!(outcome.status, LoadStatus::Loaded);
        assert_eq!(outcome.value, value);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn every_document_records_its_schema_version() {
        let (dir, path) = sample_path("persistence-version");

        write_document(&path, &Sample::default()).expect("the document is writable");

        let raw: Value =
            serde_json::from_str(&std::fs::read_to_string(&path).expect("readable")).expect("json");
        assert_eq!(raw[VERSION_KEY], serde_json::json!(SCHEMA_VERSION));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn writing_leaves_no_temporary_file_behind() {
        let (dir, path) = sample_path("persistence-temp");

        write_document(&path, &Sample::default()).expect("the document is writable");

        assert!(path.exists());
        let leftovers: Vec<String> = std::fs::read_dir(path.parent().expect("has a parent"))
            .expect("the directory is readable")
            .filter_map(Result::ok)
            .map(|entry| entry.file_name().to_string_lossy().into_owned())
            .filter(|name| name.ends_with(".tmp"))
            .collect();
        assert!(
            leftovers.is_empty(),
            "no temporary file may survive a write: {leftovers:?}"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn writing_replaces_an_existing_document() {
        let (dir, path) = sample_path("persistence-replace");
        write_document(
            &path,
            &Sample {
                app: "first".to_string(),
                limit: 1,
            },
        )
        .expect("first write");

        let second = Sample {
            app: "second".to_string(),
            limit: 2,
        };
        write_document(&path, &second).expect("second write");

        assert_eq!(load(&path).value, second);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn writing_creates_missing_directories() {
        let (dir, path) = sample_path("persistence-mkdir");

        assert!(!path.parent().expect("has a parent").exists());
        write_document(&path, &Sample::default()).expect("the document is writable");

        assert!(path.parent().expect("has a parent").is_dir());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_write_failure_is_a_structured_error() {
        let (dir, _path) = sample_path("persistence-write-failure");
        // A file where the directory should be: creating the parent must fail.
        std::fs::create_dir_all(&dir).expect("the workspace exists");
        let blocked = dir.join("blocked");
        std::fs::write(&blocked, b"not a directory").expect("the blocker is writable");

        let error = write_document(&blocked.join("sample.json"), &Sample::default())
            .expect_err("writing under a file must fail");

        assert_eq!(error.code(), "internal");
        assert_eq!(
            load(&blocked.join("sample.json")).status,
            LoadStatus::Missing
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_empty_file_is_recovered_and_preserved() {
        let (dir, path) = sample_path("persistence-empty");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
        std::fs::write(&path, b"").expect("the file is writable");

        let outcome = load(&path);

        assert!(matches!(outcome.status, LoadStatus::Recovered { .. }));
        assert_eq!(outcome.value, Sample::default());
        assert_eq!(
            corrupt_files(&dir).len(),
            1,
            "the unusable file must be preserved"
        );
        assert!(
            !path.exists(),
            "the original path is freed for the next write"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn malformed_json_is_recovered_and_preserved() {
        let (dir, path) = sample_path("persistence-malformed");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
        std::fs::write(&path, b"{ \"app\": \"crossport\",").expect("the file is writable");

        let outcome = load(&path);

        assert!(matches!(outcome.status, LoadStatus::Recovered { .. }));
        assert_eq!(outcome.value, Sample::default());
        assert_eq!(corrupt_files(&dir).len(), 1);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn json_that_is_not_an_object_is_recovered() {
        let (dir, path) = sample_path("persistence-not-object");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
        std::fs::write(&path, b"[1, 2, 3]").expect("the file is writable");

        let outcome = load(&path);

        assert!(matches!(outcome.status, LoadStatus::Recovered { .. }));
        assert_eq!(outcome.value, Sample::default());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_document_of_the_wrong_shape_is_recovered() {
        let (dir, path) = sample_path("persistence-wrong-shape");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
        std::fs::write(&path, br#"{"schemaVersion":1,"limit":"many"}"#)
            .expect("the file is writable");

        let outcome = load(&path);

        assert!(
            matches!(outcome.status, LoadStatus::Recovered { .. }),
            "a field of the wrong type is not a usable document: {:?}",
            outcome.status
        );
        assert_eq!(outcome.value, Sample::default());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_document_without_a_version_is_treated_as_the_oldest_schema() {
        let (dir, path) = sample_path("persistence-unversioned");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
        std::fs::write(&path, br#"{"app":"crossport"}"#).expect("the file is writable");

        let outcome = load(&path);

        assert_eq!(outcome.status, LoadStatus::Migrated { from: 0 });
        assert_eq!(outcome.value.app, "crossport");
        assert_eq!(outcome.value.limit, Sample::default().limit);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn unknown_fields_from_other_builds_are_ignored() {
        let (dir, path) = sample_path("persistence-unknown-fields");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
        std::fs::write(
            &path,
            br#"{"schemaVersion":1,"app":"crossport","limit":5,"futureField":true}"#,
        )
        .expect("the file is writable");

        let outcome = load(&path);

        assert_eq!(outcome.status, LoadStatus::Loaded);
        assert_eq!(outcome.value.limit, 5);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_newer_schema_is_refused_and_left_untouched() {
        let (dir, path) = sample_path("persistence-newer");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
        let contents = br#"{"schemaVersion":99,"app":"from the future","limit":7}"#;
        std::fs::write(&path, contents).expect("the file is writable");

        let outcome = load(&path);

        assert!(matches!(outcome.status, LoadStatus::Unsupported { .. }));
        assert_eq!(outcome.value, Sample::default());
        assert!(
            !outcome.writable(),
            "an unrecognized document must never be overwritten"
        );
        assert_eq!(
            std::fs::read(&path).expect("still there"),
            contents,
            "the newer document must survive untouched"
        );
        assert!(corrupt_files(&dir).is_empty());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_version_that_is_not_a_whole_number_is_recovered() {
        let (dir, path) = sample_path("persistence-bad-version");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
        std::fs::write(&path, br#"{"schemaVersion":"one"}"#).expect("the file is writable");

        assert!(matches!(load(&path).status, LoadStatus::Recovered { .. }));

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_failed_migration_is_recovered_rather_than_guessed_at() {
        fn refusing(
            _from: u32,
            _document: Map<String, Value>,
        ) -> Result<Map<String, Value>, String> {
            Err("this migration is not implemented".to_string())
        }

        let (dir, path) = sample_path("persistence-failed-migration");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
        std::fs::write(&path, br#"{"app":"crossport"}"#).expect("the file is writable");

        let outcome = read_document::<Sample>(&path, refusing);

        assert!(matches!(outcome.status, LoadStatus::Recovered { .. }));
        assert_eq!(outcome.value, Sample::default());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_migration_can_rewrite_an_older_document() {
        fn rename_app(
            from: u32,
            mut document: Map<String, Value>,
        ) -> Result<Map<String, Value>, String> {
            if from == 0 {
                if let Some(value) = document.remove("oldApp") {
                    document.insert("app".to_string(), value);
                }
            }
            Ok(document)
        }

        let (dir, path) = sample_path("persistence-migration");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
        std::fs::write(&path, br#"{"oldApp":"crossport","limit":9}"#)
            .expect("the file is writable");

        let outcome = read_document::<Sample>(&path, rename_app);

        assert_eq!(outcome.status, LoadStatus::Migrated { from: 0 });
        assert_eq!(outcome.value.app, "crossport");
        assert_eq!(outcome.value.limit, 9);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_recovered_document_can_be_written_back() {
        let (dir, path) = sample_path("persistence-recover-then-write");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
        std::fs::write(&path, b"not json").expect("the file is writable");

        let recovered = load(&path);
        assert!(recovered.writable());

        write_document(&path, &recovered.value).expect("the recovered document saved");

        assert_eq!(load(&path).status, LoadStatus::Loaded);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn preserved_files_do_not_collide_with_each_other() {
        let (dir, path) = sample_path("persistence-preserve-twice");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");

        std::fs::write(&path, b"first corruption").expect("the file is writable");
        load(&path);
        std::fs::write(&path, b"second corruption").expect("the file is writable");
        load(&path);

        assert!(
            corrupt_files(&dir).len() >= 2,
            "each preserved copy needs its own name"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn documents_are_stored_as_readable_json() {
        let (dir, path) = sample_path("persistence-readable");
        write_document(
            &path,
            &Sample {
                app: "crossport".to_string(),
                limit: 3,
            },
        )
        .expect("the document is writable");

        let contents = std::fs::read_to_string(&path).expect("readable");
        assert!(contents.contains('\n'), "a document should be formatted");
        assert!(contents.contains("\"app\": \"crossport\""));

        let _ = std::fs::remove_dir_all(dir);
    }

    /// Every preserved copy under `dir`, wherever the document lived inside it.
    fn corrupt_files(dir: &Path) -> Vec<PathBuf> {
        let mut found = Vec::new();
        collect_corrupt(dir, &mut found);
        found
    }

    fn collect_corrupt(dir: &Path, found: &mut Vec<PathBuf>) {
        let Ok(entries) = std::fs::read_dir(dir) else {
            return;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                collect_corrupt(&path, found);
            } else if path
                .file_name()
                .map(|name| name.to_string_lossy().contains(".corrupt-"))
                .unwrap_or(false)
            {
                found.push(path);
            }
        }
    }
}
