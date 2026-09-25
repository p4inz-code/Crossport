/* ==========================================================================
 * Logging foundation
 * Owns the Tauri log plugin configuration and startup logging. Logs always
 * land in a per-app log directory as well as stdout, because a Windows
 * release build has no console a user can read.
 * ========================================================================== */

use log::LevelFilter;
use tauri::Manager;
use tauri_plugin_log::{FileOpenStrategy, RotationStrategy, Target, TargetKind, TimezoneStrategy};

/// Base file name of the log file; the plugin appends the date on rotation.
const LOG_FILE_NAME: &str = "crossport";

/// 5 MiB per file, three files kept: enough history to diagnose a user report
/// without letting logs grow without bound.
const MAX_LOG_FILE_BYTES: u128 = 5 * 1024 * 1024;
const MAX_LOG_FILES: usize = 3;

/// Builds the log plugin registered on the application builder.
///
/// - development: `debug` level, useful while working
/// - production: `info` level, quiet enough to keep files readable
/// - dependency crates are pinned to `warn` so their chatter cannot fill the
///   file the user is asked to share
pub fn plugin() -> tauri::plugin::TauriPlugin<tauri::Wry> {
    let level = if cfg!(debug_assertions) {
        LevelFilter::Debug
    } else {
        LevelFilter::Info
    };

    tauri_plugin_log::Builder::default()
        .level(level)
        .level_for("tauri", LevelFilter::Warn)
        .level_for("tao", LevelFilter::Warn)
        .level_for("wry", LevelFilter::Warn)
        .max_file_size(MAX_LOG_FILE_BYTES)
        .rotation_strategy(RotationStrategy::KeepSome(MAX_LOG_FILES))
        .file_open_strategy(FileOpenStrategy::Rotate)
        .timezone_strategy(TimezoneStrategy::UseLocal)
        .targets([
            Target::new(TargetKind::Stdout),
            // Resolves to `AppPaths::log_dir`; both come from Tauri's path
            // resolver, so diagnostics point at the directory users see.
            Target::new(TargetKind::LogDir {
                file_name: Some(LOG_FILE_NAME.to_string()),
            }),
        ])
        .build()
}

/// Logs startup context once plugins are registered and settings are loaded.
///
/// This runs on every launch and is the entry point of any bug report, so it
/// records the version, the platform, the loaded preferences, and the
/// directories that hold settings and logs.
pub fn init(app: &tauri::AppHandle) {
    let state = app.state::<crate::state::AppState>();

    log::info!(
        "{} v{} starting on {} ({}/{})",
        state.config.app_name,
        state.config.app_version,
        state.platform.as_str(),
        std::env::consts::OS,
        std::env::consts::ARCH,
    );

    match state.settings_snapshot() {
        Ok(settings) => log::debug!(
            "settings loaded: theme={}, locale={}",
            settings.theme,
            settings.locale
        ),
        Err(error) => log::error!("could not read the loaded settings: {error}"),
    }

    match crate::platform::AppPaths::resolve(app) {
        Ok(paths) => log::info!(
            "config dir: {} | log dir: {}",
            paths.config_dir.display(),
            paths.log_dir.display()
        ),
        Err(error) => log::warn!("could not resolve application directories: {error}"),
    }
}
