/* ==========================================================================
 * CrossPort backend entry point
 * Wires the module tree (platform volumes, filesystem browsing, transfers,
 * settings, errors), the logging foundation, managed state, the native dialog
 * plugin, and the Tauri command layer. Settings are loaded from disk during
 * setup so startup logging reflects real state, and the transfer engine is
 * given the app handle so it can publish typed progress events.
 * ========================================================================== */

pub mod archive;
mod commands;
mod config;
mod errors;
mod filesystem;
mod history;
mod logging;
#[cfg(test)]
mod measure;
mod persistence;
mod platform;
mod recovery;
mod settings;
mod state;
pub mod transfer;
pub mod verification;

use std::sync::Arc;

use tauri::{Emitter, Manager};

use archive::TransferArchive;
use platform::AppPaths;
use state::AppState;
use transfer::{TransferJournal, TransferPublisher, TransferSnapshot};

/// Publishes transfer progress to the frontend as typed `transfer:update`
/// events.
///
/// This is the only place that pairs the transfer engine with the windowing
/// layer: the engine hands snapshots to whatever publisher is installed, so
/// the backend stays testable without a running application.
struct FrontendPublisher {
    app: tauri::AppHandle,
}

impl TransferPublisher for FrontendPublisher {
    fn publish(&self, snapshot: &TransferSnapshot) {
        if let Err(error) = self.app.emit(transfer::TRANSFER_EVENT, snapshot.clone()) {
            log::debug!("transfer progress could not be published: {error}");
        }
    }
}

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
    // A release build has no console: a panic would otherwise stop the app
    // with nothing on screen. Installed before anything else can fail.
    errors::install_panic_hook();

    let app = tauri::Builder::default()
        .plugin(logging::plugin())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::app::exit_app,
            commands::dialog::pick_directory,
            commands::drives::list_drives,
            commands::filesystem::inspect_path,
            commands::filesystem::list_ancestors,
            commands::filesystem::list_directory,
            commands::history::clear_history,
            commands::history::delete_history_record,
            commands::history::get_archive_status,
            commands::history::get_history_record,
            commands::history::list_history,
            commands::recovery::get_recovery_candidate,
            commands::recovery::list_recovery_candidates,
            commands::recovery::recover_transfer,
            commands::settings::get_settings,
            commands::settings::update_settings,
            commands::system::get_system_info,
            commands::transfer::cancel_transfer,
            commands::transfer::clear_finished_transfers,
            commands::transfer::get_transfer,
            commands::transfer::list_transfers,
            commands::transfer::pause_transfer,
            commands::transfer::plan_transfer,
            commands::transfer::remove_transfer,
            commands::transfer::resume_transfer,
            commands::transfer::start_transfer,
        ])
        .setup(|app| {
            // Load persisted settings before startup logging runs.
            match settings::load(app.handle()) {
                Ok(loaded) => {
                    if let Err(error) = app.state::<AppState>().replace_settings(loaded) {
                        log::error!("failed to store loaded settings: {error}");
                    }
                }
                Err(error) => log::error!("failed to load settings: {error}"),
            }
            logging::init(app.handle());

            // The durable archive: transfer history, interrupted jobs, and
            // recovery. Opened after settings so the retention bound is the
            // user's, and inside setup so every command that needs it finds it.
            let limit = app
                .state::<AppState>()
                .settings_snapshot()
                .map(|settings| settings.history_limit)
                .unwrap_or(history::DEFAULT_HISTORY_LIMIT);
            match AppPaths::resolve(app.handle()) {
                Ok(paths) => {
                    let (archive, history_status, state_status) =
                        TransferArchive::open(&paths.config_dir, limit);
                    if archive.degraded() {
                        log::warn!(
                            "the transfer archive loaded with a document it could not use: \
                             history={history_status:?}, state={state_status:?}"
                        );
                    }
                    app.state::<AppState>()
                        .transfers
                        .attach_journal(Arc::clone(&archive) as Arc<dyn TransferJournal>);
                    if let Err(error) = app.state::<AppState>().install_archive(archive) {
                        log::error!("failed to install the transfer archive: {error}");
                    }
                }
                Err(error) => {
                    log::error!("failed to resolve the application directories: {error}");
                }
            }

            // Progress events need the app handle; transfers started before
            // this point would simply run without publishing progress.
            app.state::<AppState>()
                .transfers
                .attach(Arc::new(FrontendPublisher {
                    app: app.handle().clone(),
                }));
            Ok(())
        })
        .build(tauri::generate_context!())
        .unwrap_or_else(|error| {
            // The log is written first (inside `show_fatal`), then the user is
            // told why no window appeared instead of the process vanishing.
            errors::show_fatal(
                "CrossPort could not start",
                &format!(
                    "CrossPort could not start and has to stop.\n\n{error}\n\n\
                     The details are in CrossPort's log folder."
                ),
            );
            std::process::exit(1);
        });

    app.run(|handle, event| {
        // Closing while work is in flight is the user's decision, not an
        // accident: the close is held and the frontend is asked. Every file is
        // either committed or still under its temporary name, and the job's
        // state document survives, so nothing is lost either way.
        if let tauri::RunEvent::WindowEvent {
            event: tauri::WindowEvent::CloseRequested { api, .. },
            ..
        } = event
        {
            if let Some(warning) = commands::app::close_state(handle) {
                api.prevent_close();
                if let Err(error) = handle.emit(commands::app::CLOSE_REQUESTED_EVENT, warning) {
                    log::warn!("a close request could not be published: {error}");
                }
            }
        }
    });
}
