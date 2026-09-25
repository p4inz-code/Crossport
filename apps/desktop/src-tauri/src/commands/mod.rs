/* ==========================================================================
 * Commands module
 * Tauri command layer. Each submodule mirrors a backend domain and is wired
 * into the invoke_handler from lib.rs. Command names are the frontend's IPC
 * contract: rename them only together with the matching service.
 * ========================================================================== */

pub mod dialog;
pub mod drives;
pub mod filesystem;
pub mod settings;
pub mod system;
