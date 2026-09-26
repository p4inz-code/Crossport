/* ==========================================================================
 * Transfer commands
 * The transfer engine's IPC surface. Planning and queueing walk the source
 * trees, so both run on the blocking pool; control commands only touch engine
 * state and return the updated snapshot immediately, which is what lets the UI
 * stay responsive while a multi-gigabyte transfer is running.
 *
 * Progress is published as typed `transfer:update` events carrying a
 * `TransferSnapshot`. Commands return snapshots too, so a client that misses an
 * event — or a frontend that just opened the page — can resynchronize with one
 * call instead of guessing.
 * ========================================================================== */

use tauri::State;

use crate::commands::run_blocking;
use crate::errors::AppResult;
use crate::platform::drives;
use crate::state::AppState;
use crate::transfer::model::{TransferPreview, TransferRequest, TransferSnapshot};
use crate::transfer::plan;

/// Plans a transfer without starting one: what would move, where it would
/// land, what collides, and whether the destination has room.
///
/// Read-only. Unsafe source/destination relationships and unreadable sources
/// are reported here with the same structured errors `start_transfer` uses, so
/// a UI can warn before the user commits.
#[tauri::command]
pub async fn plan_transfer(request: TransferRequest) -> AppResult<TransferPreview> {
    run_blocking("plan_transfer", move || {
        let planned = plan::plan_request(&request)?;
        let available = drives::available_bytes(&planned.destination);
        Ok(TransferPreview::from_plan(&planned, available))
    })
    .await
}

/// Plans a request and queues it as a job.
///
/// Rejects (rather than queues) anything unsafe: a missing source, a
/// destination that is inside its own source, a destination that cannot be
/// written, a plan larger than the engine will enumerate, or a destination
/// without enough free space.
#[tauri::command]
pub async fn start_transfer(
    state: State<'_, AppState>,
    request: TransferRequest,
) -> AppResult<TransferSnapshot> {
    let transfers = state.transfers.clone();
    run_blocking("start_transfer", move || transfers.enqueue_request(request)).await
}

/// Every job in queue order, oldest first.
#[tauri::command]
pub fn list_transfers(state: State<'_, AppState>) -> Vec<TransferSnapshot> {
    state.transfers.snapshots()
}

/// One job by identifier.
#[tauri::command]
pub fn get_transfer(state: State<'_, AppState>, id: String) -> AppResult<TransferSnapshot> {
    state.transfers.snapshot(&id)
}

/// Parks a job. Byte progress stops, the destination stays untouched, and the
/// job stays resumable.
#[tauri::command]
pub fn pause_transfer(state: State<'_, AppState>, id: String) -> AppResult<TransferSnapshot> {
    state.transfers.pause(&id)
}

/// Continues a paused job from where it stopped.
#[tauri::command]
pub fn resume_transfer(state: State<'_, AppState>, id: String) -> AppResult<TransferSnapshot> {
    state.transfers.resume(&id)
}

/// Stops a job and discards whatever partial output it had written.
#[tauri::command]
pub fn cancel_transfer(state: State<'_, AppState>, id: String) -> AppResult<TransferSnapshot> {
    state.transfers.cancel(&id)
}

/// Drops one finished job from the queue.
#[tauri::command]
pub fn remove_transfer(state: State<'_, AppState>, id: String) -> AppResult<()> {
    state.transfers.remove(&id)
}

/// Drops every finished job and reports how many were removed.
#[tauri::command]
pub fn clear_finished_transfers(state: State<'_, AppState>) -> u32 {
    u32::try_from(state.transfers.clear_finished()).unwrap_or(u32::MAX)
}
