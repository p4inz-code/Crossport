/* ==========================================================================
 * CrossPort backend entry point
 * Wires the module tree (platform volumes, filesystem browsing, settings,
 * errors), the logging foundation, managed state, the native dialog plugin,
 * and the Tauri command layer. Settings are loaded from disk during setup so
 * startup logging reflects real state.
 * ========================================================================== */

mod commands;
mod config;
mod errors;
mod filesystem;
mod logging;
mod platform;
mod settings;
mod state;

use state::AppState;
use tauri::Manager;

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
            Ok(())
        })
        .run(tauri::generate_context!())
        .unwrap_or_else(|error| {
            eprintln!("failed to run CrossPort application: {error}");
            std::process::exit(1);
        });
}
