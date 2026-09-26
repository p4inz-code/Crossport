/* ==========================================================================
 * CrossPort backend entry point
 * Wires the module tree (platform volumes, filesystem browsing, transfers,
 * settings, errors), the logging foundation, managed state, the native dialog
 * plugin, and the Tauri command layer. Settings are loaded from disk during
 * setup so startup logging reflects real state, and the transfer engine is
 * given the app handle so it can publish typed progress events.
 * ========================================================================== */

mod commands;
mod config;
mod errors;
mod filesystem;
mod logging;
mod platform;
mod settings;
mod state;
pub mod transfer;

use std::sync::Arc;

use tauri::{Emitter, Manager};

use state::AppState;
use transfer::{TransferPublisher, TransferSnapshot};

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
    tauri::Builder::default()
        .plugin(logging::plugin())
        .plugin(tauri_plugin_dialog::init())
        .manage(AppState::default())
        .invoke_handler(tauri::generate_handler![
            commands::dialog::pick_directory,
            commands::drives::list_drives,
            commands::filesystem::inspect_path,
            commands::filesystem::list_directory,
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
            // Progress events need the app handle; transfers started before
            // this point would simply run without publishing progress.
            app.state::<AppState>()
                .transfers
                .attach(Arc::new(FrontendPublisher {
                    app: app.handle().clone(),
                }));
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            eprintln!("failed to run CrossPort application: {error}");
            std::process::exit(1);
        });
}
