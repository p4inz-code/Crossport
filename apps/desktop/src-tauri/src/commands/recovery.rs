/* ==========================================================================
 * Recovery commands
 * What the application found unfinished, what it can prove, and what the user
 * decided to do about it.
 *
 * A recovery action is never carried out implicitly: the list is a read, and
 * every action goes through one explicit command that validates the action
 * against the candidate's own outcome first. A caller that asks to restart a
 * job whose source has disappeared gets a structured refusal, not a guess.
 *
 * Actions run on the blocking pool because they walk the destination tree,
 * remove leftovers, and re-plan the recorded request.
 * ========================================================================== */

use tauri::State;

use crate::archive::DocumentStatus;
use crate::commands::run_blocking;
use crate::errors::{AppError, AppResult};
use crate::history::RecoveryAction;
use crate::recovery::RecoveryCandidate;
use crate::state::AppState;
use crate::transfer::TransferEngine;

/// What recovery currently finds, plus the state document's health.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryListing {
    pub candidates: Vec<RecoveryCandidate>,
    /// How the interrupted-state document loaded.
    pub status: DocumentStatus,
    /// False only while the document belongs to a newer build.
    pub writable: bool,
}

/// Every job that was in flight when the application stopped.
///
/// Read-only: inspecting a candidate scans its destination tree for leftovers
/// and re-plans its request, but touches nothing.
#[tauri::command]
pub async fn list_recovery_candidates(state: State<'_, AppState>) -> AppResult<RecoveryListing> {
    let archive = state.archive()?;
    run_blocking("list_recovery_candidates", move || {
        Ok(RecoveryListing {
            candidates: archive.recovery_candidates(),
            status: DocumentStatus::from(archive.state_status()),
            writable: archive.state_status().writable(),
        })
    })
    .await
}

/// One interrupted job, or a structured error naming what is missing.
#[tauri::command]
pub async fn get_recovery_candidate(
    state: State<'_, AppState>,
    id: String,
) -> AppResult<RecoveryCandidate> {
    let archive = state.archive()?;
    run_blocking("get_recovery_candidate", move || {
        archive.recovery_candidate(&id)
    })
    .await
}

/// Carries out a recovery decision: `restart`, `discard`, or `confirm`.
///
/// The action is validated against the candidate before anything happens, so
/// an impossible restart is refused rather than half-applied.
#[tauri::command]
pub async fn recover_transfer(
    state: State<'_, AppState>,
    id: String,
    action: String,
) -> AppResult<crate::archive::RecoveryReport> {
    let action = RecoveryAction::parse(&action).map_err(AppError::InvalidInput)?;
    let archive = state.archive()?;
    let engine: TransferEngine = state.transfers.clone();

    run_blocking("recover_transfer", move || {
        archive.recover(&engine, &id, action)
    })
    .await
}
