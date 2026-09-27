/* ==========================================================================
 * Application lifecycle commands
 * The one place the frontend is asked whether closing the window is really
 * wanted, and the only command that ends the process.
 *
 * Closing with work in flight is not a data-loss event — every file is either
 * committed or still under its temporary name, and the job's state document
 * survives — but it is an outcome the user has to choose. So the window close
 * is held, the frontend is told what would happen, and only an explicit
 * confirmation ends the process.
 * ========================================================================== */

use serde::Serialize;
use tauri::{AppHandle, Manager};

use crate::errors::AppResult;
use crate::state::AppState;
use crate::transfer::TransferStatus;

/// Event telling the frontend that a close was requested and is being held.
pub const CLOSE_REQUESTED_EVENT: &str = "app:close-requested";

/// What closing right now would set aside.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CloseWarning {
    /// Jobs that have not reached a terminal state.
    pub live_jobs: usize,
    /// Interrupted jobs still waiting for a decision.
    pub interrupted_jobs: usize,
    /// One line stating the consequence, in the application's own words.
    pub detail: String,
}

/// Whether closing deserves a confirmation, and why.
///
/// Closing when nothing is in flight and nothing is waiting is not worth a
/// dialog: there is no live work to interrupt and no decision to lose.
/// Everything else warns, because a running transfer really does become an
/// interrupted one and only the user can say whether that is fine.
pub fn close_warning(live_jobs: usize, interrupted_jobs: usize) -> Option<CloseWarning> {
    if live_jobs == 0 && interrupted_jobs == 0 {
        return None;
    }

    let detail = match (live_jobs, interrupted_jobs) {
        (0, interrupted) => format!(
            "{interrupted} interrupted transfer{} will still be waiting for a decision next time.",
            if interrupted == 1 { "" } else { "s" }
        ),
        (live, 0) => format!(
            "{live} transfer{} still running will be stopped and listed under Recovery.",
            if live == 1 { " is" } else { "s are" }
        ),
        (live, interrupted) => format!(
            "{live} transfer{} still running will be stopped and listed under Recovery, \
             joining {interrupted} interrupted transfer{} already waiting.",
            if live == 1 { " is" } else { "s are" },
            if interrupted == 1 { "" } else { "s" }
        ),
    };

    Some(CloseWarning {
        live_jobs,
        interrupted_jobs,
        detail,
    })
}

/// Counts live jobs and interrupted jobs from what the engine and archive hold.
///
/// Kept separate from the `AppHandle` lookup so the counting rule — a terminal
/// job is not live work — is testable on its own.
pub fn live_work_counts<I>(statuses: I, interrupted_jobs: usize) -> (usize, usize)
where
    I: IntoIterator<Item = TransferStatus>,
{
    let live_jobs = statuses
        .into_iter()
        .filter(|status| !status.is_terminal())
        .count();
    (live_jobs, interrupted_jobs)
}

/// What closing the window would set aside, read from the running application.
pub fn close_state(app: &AppHandle) -> Option<CloseWarning> {
    let state = app.state::<AppState>();
    let statuses: Vec<TransferStatus> = state
        .transfers
        .snapshots()
        .into_iter()
        .map(|snapshot| snapshot.status)
        .collect();
    let interrupted = state
        .archive()
        .map(|archive| archive.state().list().len())
        .unwrap_or(0);

    let (live_jobs, interrupted_jobs) = live_work_counts(statuses, interrupted);
    close_warning(live_jobs, interrupted_jobs)
}

/// Ends the application immediately.
///
/// Only reachable from a confirmation: closing while transfers run is a choice
/// the user makes, never something that happens to them.
#[tauri::command]
pub async fn exit_app(app: AppHandle) -> AppResult<()> {
    app.exit(0);
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn closing_with_nothing_in_flight_needs_no_confirmation() {
        assert_eq!(close_warning(0, 0), None);
    }

    #[test]
    fn closing_with_live_work_warns_about_what_it_stops() {
        let warning = close_warning(2, 0).expect("a running transfer is worth a warning");

        assert_eq!(warning.live_jobs, 2);
        assert_eq!(warning.interrupted_jobs, 0);
        assert!(
            warning.detail.contains("Recovery"),
            "the warning says where the job will be listed: {}",
            warning.detail
        );
    }

    #[test]
    fn closing_with_a_waiting_decision_warns_about_that_too() {
        let waiting = close_warning(0, 1).expect("an undecided job is worth a warning");
        assert!(waiting
            .detail
            .contains("1 interrupted transfer will still be waiting"));

        let both = close_warning(1, 2).expect("both cases are worth a warning");
        assert!(both.detail.contains("1 transfer is still running"));
        assert!(both
            .detail
            .contains("2 interrupted transfers already waiting"));
    }

    #[test]
    fn only_non_terminal_jobs_count_as_live_work() {
        let statuses = [
            TransferStatus::Queued,
            TransferStatus::Preparing,
            TransferStatus::Running,
            TransferStatus::Paused,
            TransferStatus::Cancelling,
            TransferStatus::Completed,
            TransferStatus::Failed,
            TransferStatus::Cancelled,
        ];

        let (live, interrupted) = live_work_counts(statuses, 3);

        assert_eq!(
            live, 5,
            "queued, preparing, running, paused, and cancelling jobs are live"
        );
        assert_eq!(interrupted, 3);
        assert_eq!(
            live_work_counts([], 0),
            (0, 0),
            "an empty application closes without a dialog"
        );
    }
}
