/* ==========================================================================
 * CrossPort backend entry point
 * Wires the module tree, logging foundation, managed state, and the Tauri
 * command layer. Commands are placeholders until features are implemented.
 * ========================================================================== */

mod commands;
mod config;
mod errors;
mod filesystem;
mod history;
mod logging;
mod models;
mod platform;
mod settings;
mod state;
mod transfer;

use state::AppState;

#[cfg_attr(mobile, tauri::mobile_entry_point)]
pub fn run() {
  tauri::Builder::default()
    .plugin(logging::plugin())
    .manage(AppState::default())
    .invoke_handler(tauri::generate_handler![
      commands::drives::list_drives,
      commands::transfer::start_transfer,
      commands::transfer::cancel_transfer,
      commands::history::list_history,
      commands::history::clear_history,
      commands::settings::get_settings,
      commands::settings::update_settings
    ])
    .setup(|app| {
      logging::init(app.handle());
      Ok(())
    })
    .run(tauri::generate_context!())
    .expect("error while running tauri application");
}
