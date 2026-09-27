/* ==========================================================================
 * History commands
 * The read side of the archive: what has happened, and how to trim it.
 *
 * History is durable state that can be partially degraded (a document written
 * by a newer build, or one that had to be set aside), so the listing carries
 * the document's own status alongside the records. A client can therefore say
 * "3 records, but the file could not be used" instead of showing an empty page
 * and implying nothing ever happened.
 *
 * Nothing here touches the transfer queue: history and the live queue are
 * separate by design, and a filtered list is a pure read.
 * ========================================================================== */

use tauri::State;

use crate::archive::DocumentStatus;
use crate::errors::{AppError, AppResult};
use crate::history::{HistoryFilter, HistoryRecord};
use crate::state::AppState;

/// Every record that matches a filter, newest first, with the document's
/// health and the bound in force.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HistoryListing {
    pub records: Vec<HistoryRecord>,
    /// How many records are kept in total, whatever the filter selects.
    pub total: u32,
    /// The retention bound currently in force.
    pub limit: u32,
    /// The filter the records were selected with.
    pub filter: HistoryFilter,
    /// How the history document loaded.
    pub status: DocumentStatus,
    /// False only while the document belongs to a newer build.
    pub writable: bool,
}

/// Lists finished transfers, newest first.
#[tauri::command]
pub fn list_history(
    state: State<'_, AppState>,
    filter: Option<String>,
) -> AppResult<HistoryListing> {
    let filter = match filter {
        Some(value) => HistoryFilter::parse(&value).map_err(AppError::InvalidInput)?,
        None => HistoryFilter::All,
    };
    let archive = state.archive()?;

    Ok(HistoryListing {
        records: archive.history_records(filter),
        total: u32::try_from(archive.history().len()).unwrap_or(u32::MAX),
        limit: archive.history().limit(),
        filter,
        status: DocumentStatus::from(archive.history_status()),
        writable: archive.history_status().writable(),
    })
}

/// One transfer's record, with everything history kept about it.
#[tauri::command]
pub fn get_history_record(state: State<'_, AppState>, id: String) -> AppResult<HistoryRecord> {
    state.archive()?.history_record(&id)
}

/// Removes one record. `false` means there was nothing to remove.
#[tauri::command]
pub fn delete_history_record(state: State<'_, AppState>, id: String) -> AppResult<bool> {
    state.archive()?.delete_history_record(&id)
}

/// Removes every record and reports how many were removed.
///
/// Explicit, and never automatic: history is the only account of what this
/// application did.
#[tauri::command]
pub fn clear_history(state: State<'_, AppState>) -> AppResult<u32> {
    let removed = state.archive()?.clear_history()?;
    Ok(u32::try_from(removed).unwrap_or(u32::MAX))
}

/// The archive's own health, so a degraded history or recovery list can be
/// explained rather than silently looking empty.
#[tauri::command]
pub fn get_archive_status(state: State<'_, AppState>) -> AppResult<crate::archive::ArchiveStatus> {
    let archive = state.archive()?;
    if archive.history().path().as_os_str().is_empty() {
        return Err(AppError::Internal(
            "the transfer archive has no document path".to_string(),
        ));
    }
    Ok(archive.status())
}
