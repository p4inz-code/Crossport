/* ==========================================================================
 * Settings domain
 * The single source of truth for user preferences. The backend owns
 * persistence: settings are stored as JSON in the platform app-config
 * directory and exposed to the frontend over typed Tauri commands.
 * ========================================================================== */

use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

use crate::errors::{AppError, AppResult};
use crate::platform::AppPaths;

/// Supported theme modes. Mirrors `THEME_MODES` in the frontend.
pub const THEMES: [&str; 3] = ["light", "dark", "system"];

use crate::history::{self, DEFAULT_HISTORY_LIMIT};
use crate::verification::VerificationPolicy;

/// File name of the persisted settings document.
const SETTINGS_FILE: &str = "settings.json";

/// User preferences. Field names map to the frontend `AppSettings` shape.
///
/// `serde(default)` is the migration story for Phase 1: a settings file
/// written by an older build (missing a field, or carrying fields a newer
/// build added) still loads, filling absent fields with their defaults.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase", default)]
pub struct AppSettings {
    pub theme: String,
    pub locale: String,
    /// How thoroughly a transfer verifies what it wrote, applied whenever a
    /// request does not name its own policy.
    ///
    /// The default is size verification: cheap, always meaningful, and it
    /// catches what disks actually do. Hashing every file is available, but it
    /// is a choice rather than an imposition.
    pub verification: VerificationPolicy,
    /// How many finished transfers history keeps, oldest pruned first.
    pub history_limit: u32,
}

impl Default for AppSettings {
    fn default() -> Self {
        Self {
            theme: "system".to_string(),
            locale: "en".to_string(),
            verification: VerificationPolicy::default(),
            history_limit: DEFAULT_HISTORY_LIMIT,
        }
    }
}

impl AppSettings {
    /// Validates that settings received from the frontend are acceptable.
    pub fn validate(&self) -> AppResult<()> {
        if !THEMES.contains(&self.theme.as_str()) {
            return Err(AppError::InvalidInput(format!(
                "theme must be one of [{}], got '{}'",
                THEMES.join(", "),
                self.theme
            )));
        }
        let locale_len = self.locale.chars().count();
        if !(2..=16).contains(&locale_len) {
            return Err(AppError::InvalidInput(
                "locale must be between 2 and 16 characters".to_string(),
            ));
        }
        history::validate_limit(self.history_limit).map_err(AppError::InvalidInput)?;
        Ok(())
    }
}

/// Resolves the settings file path inside the platform app-config directory.
fn settings_file(app: &tauri::AppHandle) -> AppResult<PathBuf> {
    Ok(AppPaths::resolve(app)?.config_file(SETTINGS_FILE))
}

/// Loads persisted settings, falling back to defaults when no file exists.
///
/// A missing file is expected and yields defaults. A corrupt or invalid file
/// is logged loudly and replaced by defaults; the next `save` overwrites it.
pub fn load(app: &tauri::AppHandle) -> AppResult<AppSettings> {
    read_settings(&settings_file(app)?)
}

/// Persists settings to disk.
pub fn save(app: &tauri::AppHandle, settings: &AppSettings) -> AppResult<()> {
    write_settings(&settings_file(app)?, settings)
}

/// Reads and validates a settings document from `path`.
fn read_settings(path: &Path) -> AppResult<AppSettings> {
    match std::fs::read_to_string(path) {
        Ok(contents) => match serde_json::from_str::<AppSettings>(&contents) {
            Ok(settings) if settings.validate().is_ok() => Ok(settings),
            Ok(_) | Err(_) => {
                log::warn!(
                    "settings file at {} is corrupt or invalid; using defaults",
                    path.display()
                );
                Ok(AppSettings::default())
            }
        },
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => Ok(AppSettings::default()),
        Err(error) => Err(AppError::Internal(format!(
            "failed to read settings file {}: {error}",
            path.display()
        ))),
    }
}

/// Writes a settings document to `path`, creating the directory when needed.
///
/// The value is validated first, then written to a temp file and renamed, so a
/// crash mid-write cannot truncate the live settings file.
fn write_settings(path: &Path, settings: &AppSettings) -> AppResult<()> {
    settings.validate()?;

    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).map_err(|error| {
            AppError::Internal(format!(
                "failed to create settings directory {}: {error}",
                parent.display()
            ))
        })?;
    }

    let contents = serde_json::to_string_pretty(settings)
        .map_err(|error| AppError::Internal(format!("failed to serialize settings: {error}")))?;

    let temp_path = path.with_extension("json.tmp");
    std::fs::write(&temp_path, contents).map_err(|error| {
        AppError::Internal(format!(
            "failed to write settings file {}: {error}",
            temp_path.display()
        ))
    })?;
    std::fs::rename(&temp_path, path).map_err(|error| {
        AppError::Internal(format!(
            "failed to finalize settings file {}: {error}",
            path.display()
        ))
    })?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::filesystem::test_support::unique_temp_dir;

    fn settings_path(label: &str) -> (PathBuf, PathBuf) {
        let dir = unique_temp_dir(label);
        let path = dir.join("config").join("settings.json");
        (dir, path)
    }

    #[test]
    fn defaults_are_valid() {
        assert!(AppSettings::default().validate().is_ok());
    }

    #[test]
    fn accepts_every_supported_theme() {
        for theme in THEMES {
            let settings = AppSettings {
                theme: theme.to_string(),
                locale: "en".to_string(),
                ..AppSettings::default()
            };
            assert!(
                settings.validate().is_ok(),
                "theme '{theme}' should be valid"
            );
        }
    }

    #[test]
    fn rejects_unknown_theme() {
        let settings = AppSettings {
            theme: "neon".to_string(),
            locale: "en".to_string(),
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn rejects_too_short_locale() {
        let settings = AppSettings {
            theme: "light".to_string(),
            locale: "x".to_string(),
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn rejects_too_long_locale() {
        let settings = AppSettings {
            theme: "light".to_string(),
            locale: "x".repeat(17),
            ..AppSettings::default()
        };
        assert!(settings.validate().is_err());
    }

    #[test]
    fn round_trips_through_json() {
        let settings = AppSettings {
            theme: "dark".to_string(),
            locale: "fr".to_string(),
            verification: VerificationPolicy::Checksum,
            history_limit: 50,
        };
        let json = serde_json::to_string(&settings).expect("settings should serialize");
        let parsed: AppSettings = serde_json::from_str(&json).expect("settings should parse");
        assert_eq!(settings, parsed);
    }

    #[test]
    fn default_settings_round_trip() {
        let json = serde_json::to_string(&AppSettings::default()).expect("should serialize");
        let parsed: AppSettings = serde_json::from_str(&json).expect("should parse");
        assert_eq!(AppSettings::default(), parsed);
    }

    #[test]
    fn serializes_with_camel_case_keys_for_the_frontend() {
        let value = serde_json::to_value(AppSettings::default()).expect("should serialize");
        assert_eq!(
            value,
            serde_json::json!({
                "theme": "system",
                "locale": "en",
                "verification": "size",
                "historyLimit": DEFAULT_HISTORY_LIMIT,
            })
        );
    }

    #[test]
    fn fills_missing_fields_from_older_settings_files() {
        let parsed: AppSettings =
            serde_json::from_str("{\"theme\":\"dark\"}").expect("partial files must load");
        assert_eq!(
            parsed,
            AppSettings {
                theme: "dark".to_string(),
                locale: AppSettings::default().locale,
                ..AppSettings::default()
            }
        );
    }

    #[test]
    fn ignores_fields_written_by_newer_builds() {
        let parsed: AppSettings =
            serde_json::from_str("{\"theme\":\"light\",\"locale\":\"en\",\"future\":true}")
                .expect("unknown fields must not break loading");
        assert_eq!(parsed.theme, "light");
        assert_eq!(parsed.locale, "en");
    }

    #[test]
    fn an_empty_document_falls_back_to_defaults() {
        let parsed: AppSettings = serde_json::from_str("{}").expect("should parse");
        assert_eq!(parsed, AppSettings::default());
    }

    #[test]
    fn the_default_verification_policy_is_size() {
        assert_eq!(
            AppSettings::default().verification,
            VerificationPolicy::Size,
            "a fresh install must verify what it writes"
        );
    }

    #[test]
    fn every_verification_policy_round_trips() {
        for policy in [
            VerificationPolicy::None,
            VerificationPolicy::Size,
            VerificationPolicy::Checksum,
        ] {
            let settings = AppSettings {
                verification: policy,
                ..AppSettings::default()
            };
            let json = serde_json::to_string(&settings).expect("settings serialize");
            let parsed: AppSettings = serde_json::from_str(&json).expect("settings parse");
            assert_eq!(parsed.verification, policy);
        }
    }

    #[test]
    fn a_history_limit_outside_the_range_is_rejected() {
        let too_small = AppSettings {
            history_limit: history::MIN_HISTORY_LIMIT - 1,
            ..AppSettings::default()
        };
        assert!(too_small.validate().is_err());

        let too_large = AppSettings {
            history_limit: history::MAX_HISTORY_LIMIT + 1,
            ..AppSettings::default()
        };
        assert!(too_large.validate().is_err());

        let accepted = AppSettings {
            history_limit: history::MAX_HISTORY_LIMIT,
            ..AppSettings::default()
        };
        assert!(accepted.validate().is_ok());
    }

    #[test]
    fn a_settings_file_without_phase_four_fields_still_loads() {
        let parsed: AppSettings = serde_json::from_str(r#"{"theme":"dark","locale":"en"}"#)
            .expect("older files must load");
        assert_eq!(parsed.verification, VerificationPolicy::Size);
        assert_eq!(parsed.history_limit, DEFAULT_HISTORY_LIMIT);
    }

    #[test]
    fn a_missing_file_loads_defaults() {
        let (dir, path) = settings_path("settings-missing");

        assert_eq!(
            read_settings(&path).expect("a missing file is not an error"),
            AppSettings::default()
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn persisted_settings_survive_a_round_trip_on_disk() {
        let (dir, path) = settings_path("settings-round-trip");
        let settings = AppSettings {
            theme: "dark".to_string(),
            locale: "pt-BR".to_string(),
            ..AppSettings::default()
        };

        write_settings(&path, &settings).expect("settings should be writable");

        assert!(path.is_file(), "settings.json should exist after a write");
        assert!(
            !path.with_extension("json.tmp").exists(),
            "the temp file must be renamed away"
        );
        assert_eq!(read_settings(&path).expect("file should load"), settings);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn writing_creates_the_configuration_directory() {
        let (dir, path) = settings_path("settings-mkdir");

        assert!(!path.parent().expect("has a parent").exists());
        write_settings(&path, &AppSettings::default()).expect("settings should be writable");

        assert!(path.parent().expect("has a parent").is_dir());

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn an_existing_file_is_overwritten() {
        let (dir, path) = settings_path("settings-overwrite");
        write_settings(
            &path,
            &AppSettings {
                theme: "light".to_string(),
                locale: "en".to_string(),
                ..AppSettings::default()
            },
        )
        .expect("first write");

        let next = AppSettings {
            theme: "dark".to_string(),
            locale: "de".to_string(),
            ..AppSettings::default()
        };
        write_settings(&path, &next).expect("second write");

        assert_eq!(read_settings(&path).expect("file should load"), next);

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn invalid_settings_never_reach_the_disk() {
        let (dir, path) = settings_path("settings-invalid-write");
        let invalid = AppSettings {
            theme: "neon".to_string(),
            locale: "en".to_string(),
            ..AppSettings::default()
        };

        let error = write_settings(&path, &invalid).expect_err("invalid settings are rejected");

        assert_eq!(error.code(), "invalid_input");
        assert!(
            !path.exists(),
            "nothing may be written for invalid settings"
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_corrupt_file_falls_back_to_defaults() {
        let (dir, path) = settings_path("settings-corrupt");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("config dir");
        std::fs::write(&path, b"{ this is not json").expect("file should be writable");

        assert_eq!(
            read_settings(&path).expect("a corrupt file must not fail the app"),
            AppSettings::default()
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_semantically_invalid_file_falls_back_to_defaults() {
        let (dir, path) = settings_path("settings-invalid-read");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("config dir");
        std::fs::write(&path, br#"{"theme":"neon","locale":"en"}"#)
            .expect("file should be writable");

        assert_eq!(
            read_settings(&path).expect("invalid values must not fail the app"),
            AppSettings::default()
        );

        let _ = std::fs::remove_dir_all(dir);
    }

    #[test]
    fn a_recovered_file_can_be_rewritten_with_defaults() {
        let (dir, path) = settings_path("settings-recover");
        std::fs::create_dir_all(path.parent().expect("has a parent")).expect("config dir");
        std::fs::write(&path, b"not json at all").expect("file should be writable");
        assert_eq!(
            read_settings(&path).expect("corrupt file loads defaults"),
            AppSettings::default()
        );

        write_settings(&path, &AppSettings::default()).expect("defaults should persist");

        assert_eq!(
            read_settings(&path).expect("rewritten file loads"),
            AppSettings::default()
        );

        let _ = std::fs::remove_dir_all(dir);
    }
}
