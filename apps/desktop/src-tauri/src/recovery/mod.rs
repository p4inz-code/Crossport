/* ==========================================================================
 * Crash recovery
 * What CrossPort does about a transfer that was running when the application,
 * or Windows, stopped.
 *
 * The rule that shapes all of it: a transfer that did not prove it finished is
 * treated as unfinished. Nothing here ever looks at a destination file and
 * concludes that a transfer succeeded — the only proof accepted is the
 * archive's own record of the job reaching a terminal state.
 *
 * Because the engine writes every file to a temporary path and renames it into
 * place only when it is complete, an interrupted job leaves a destination that
 * contains only complete files plus `.crossport-<job>-<index>.partial`
 * leftovers. Byte-offset continuation of those leftovers is *not* offered:
 * nothing persisted proves which prefix of a partial file is valid, so a
 * restart starts the affected files from zero after the leftovers are removed.
 * Correctness is worth more than resume speed.
 *
 * Live state is written while a job runs, so after a crash the application
 * knows what was in flight, what it was doing, and how far it had reported
 * getting — and it knows that none of that is proof of completion.
 * ========================================================================== */

pub mod artifacts;

#[cfg(test)]
mod tests;

pub use artifacts::{artifact_directories, discard, scan, ArtifactScan, PartialArtifact};

use std::path::PathBuf;
use std::sync::Mutex;

use serde::{Deserialize, Serialize};

use crate::errors::{AppError, AppResult, StoredError};
use crate::history::{HistoryVerification, RecoveryAction};
use crate::persistence::{read_document, write_document, LoadOutcome, LoadStatus};
use crate::transfer::{ConflictStrategy, TransferOperation, TransferRequest, TransferStatus};
use crate::verification::VerificationPolicy;

/// File name of the persisted in-flight state document.
pub const STATE_FILE: &str = "transfer-state.json";

/// Most interrupted jobs the state file keeps.
///
/// Reaching this means the application died mid-transfer more than a hundred
/// times without the user ever clearing the list, which is not a state worth
/// preserving: the oldest entries are dropped and the drop is logged.
pub const MAX_STATE_JOBS: usize = 100;

/// A job's counters as they were when its state was last written.
///
/// Deliberately smaller than the live progress structure: speed, ETA, and
/// elapsed time describe a process that no longer exists, so persisting them
/// would only invite someone to display them after a restart.
#[derive(Debug, Clone, Copy, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct PersistedProgress {
    pub total_bytes: u64,
    pub transferred_bytes: u64,
    pub total_files: u64,
    pub completed_files: u64,
    pub total_directories: u64,
    pub completed_directories: u64,
    pub skipped_items: u64,
    pub failed_items: u64,
}

impl PersistedProgress {
    /// Whole percent reported before the interruption, when there was anything
    /// to measure. Derived the same way the live progress is.
    pub fn percent(&self) -> Option<u32> {
        if self.total_bytes > 0 {
            let done = self.transferred_bytes.min(self.total_bytes);
            return Some((done.saturating_mul(100) / self.total_bytes) as u32);
        }
        let items = self.total_files.saturating_add(self.total_directories);
        if items == 0 {
            return None;
        }
        let done = self
            .completed_files
            .saturating_add(self.completed_directories)
            .min(items);
        Some((done.saturating_mul(100) / items) as u32)
    }
}

/// One job that was queued or running when its state was last written.
///
/// The stored request is the whole point: it is what a restart replays, so the
/// operation, the conflict strategy, and the verification policy the user chose
/// survive a crash unchanged.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct InterruptedTransfer {
    pub id: String,
    pub request: TransferRequest,
    /// What the job was doing when this was written.
    pub status: TransferStatus,
    pub queued_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub updated_at_ms: u64,
    pub progress: PersistedProgress,
    /// Verification as it stood, when any had run.
    pub verification: Option<HistoryVerification>,
}

impl InterruptedTransfer {
    /// Whether the job had begun moving data (as opposed to still queueing).
    pub fn had_started(&self) -> bool {
        self.started_at_ms.is_some()
    }

    fn validate(&self) -> Result<(), String> {
        if self.id.trim().is_empty() {
            return Err("an interrupted transfer needs an identifier".to_string());
        }
        if self.request.sources.is_empty() {
            return Err("an interrupted transfer needs at least one source".to_string());
        }
        if self.request.destination.trim().is_empty() {
            return Err("an interrupted transfer needs a destination".to_string());
        }
        Ok(())
    }
}

/// The persisted state document: jobs in the order they were accepted.
#[derive(Debug, Clone, Default, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", default)]
pub struct TransferStateDocument {
    pub jobs: Vec<InterruptedTransfer>,
}

/// The in-flight state file.
///
/// Written while transfers run and read once at startup. Nothing else in the
/// application reads it, and every write replaces the whole document
/// atomically, so a crash during a write leaves the previous version intact.
pub struct TransferStateStore {
    path: PathBuf,
    inner: Mutex<StateInner>,
}

struct StateInner {
    jobs: Vec<InterruptedTransfer>,
    /// False when the file belongs to a newer build and must not be replaced.
    writable: bool,
}

impl TransferStateStore {
    /// Opens the store, reading whatever is on disk.
    pub fn open(path: PathBuf) -> (Self, LoadStatus) {
        let outcome: LoadOutcome<TransferStateDocument> = read_document(&path, migrate);
        let status = outcome.status.clone();
        let writable = outcome.writable();
        let mut jobs = sanitize(outcome.value.jobs);
        if jobs.len() > MAX_STATE_JOBS {
            let dropped = jobs.len() - MAX_STATE_JOBS;
            log::warn!(
                "the transfer state file held {dropped} more interrupted jobs than are kept; dropping the oldest"
            );
            jobs.truncate(MAX_STATE_JOBS);
        }

        (
            Self {
                path,
                inner: Mutex::new(StateInner { jobs, writable }),
            },
            status,
        )
    }

    pub fn path(&self) -> &std::path::Path {
        &self.path
    }

    pub fn writable(&self) -> bool {
        lock(&self.inner).writable
    }

    /// Records or replaces one job's state.
    ///
    /// Called on every state transition and on throttled progress, so it is
    /// deliberately cheap: one lock, one vector update, one atomic write.
    pub fn upsert(&self, job: InterruptedTransfer) -> AppResult<()> {
        if let Err(detail) = job.validate() {
            return Err(AppError::InvalidInput(detail));
        }

        let mut inner = lock(&self.inner);
        match inner.jobs.iter().position(|existing| existing.id == job.id) {
            Some(index) => inner.jobs[index] = job,
            None => inner.jobs.push(job),
        }

        if inner.jobs.len() > MAX_STATE_JOBS {
            let dropped = inner.jobs.len() - MAX_STATE_JOBS;
            log::warn!("dropping {dropped} of the oldest interrupted transfer records");
            inner.jobs.drain(..dropped);
        }

        // Written while the lock is held, so a state write can never land out
        // of order with the clear that follows a completed transfer.
        if inner.writable {
            persist(&self.path, &inner.jobs)?;
        }
        Ok(())
    }

    /// Forgets a job, which is what makes it no longer interrupted.
    pub fn forget(&self, id: &str) -> AppResult<bool> {
        let mut inner = lock(&self.inner);
        let before = inner.jobs.len();
        inner.jobs.retain(|job| job.id != id);
        let removed = inner.jobs.len() != before;

        if removed && inner.writable {
            persist(&self.path, &inner.jobs)?;
        }
        Ok(removed)
    }

    pub fn get(&self, id: &str) -> Option<InterruptedTransfer> {
        lock(&self.inner)
            .jobs
            .iter()
            .find(|job| job.id == id)
            .cloned()
    }

    /// Every interrupted job, oldest first.
    pub fn list(&self) -> Vec<InterruptedTransfer> {
        lock(&self.inner).jobs.clone()
    }

    pub fn len(&self) -> usize {
        lock(&self.inner).jobs.len()
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }
}

impl std::fmt::Debug for TransferStateStore {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let inner = lock(&self.inner);
        formatter
            .debug_struct("TransferStateStore")
            .field("path", &self.path)
            .field("jobs", &inner.jobs.len())
            .field("writable", &inner.writable)
            .finish()
    }
}

/// What recovery decided about one interrupted job.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RecoveryOutcome {
    /// The archive itself proves the job reached a terminal state before the
    /// application stopped. Nothing needs to run again.
    CompletedBeforeCrash,
    /// The job did not finish and can be run again from the beginning.
    RestartRequired,
    /// A source the job needs is gone, so it cannot be run again.
    SourceMissing,
    /// The destination is missing, is not a directory, or cannot be written.
    DestinationUnavailable,
    /// The recorded request cannot be planned again.
    Unsupported,
}

impl RecoveryOutcome {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::CompletedBeforeCrash => "completed_before_crash",
            Self::RestartRequired => "restart_required",
            Self::SourceMissing => "source_missing",
            Self::DestinationUnavailable => "destination_unavailable",
            Self::Unsupported => "unsupported",
        }
    }

    /// Whether a restart can be offered at all.
    pub fn can_restart(self) -> bool {
        matches!(self, Self::RestartRequired)
    }

    /// A one-line explanation for the user.
    pub fn explain(self) -> &'static str {
        match self {
            Self::CompletedBeforeCrash => {
                "This transfer finished before the application stopped; the queue entry was never cleared."
            }
            Self::RestartRequired => {
                "This transfer did not finish. It can be run again from the beginning."
            }
            Self::SourceMissing => {
                "A source this transfer needs is no longer there, so it cannot be run again."
            }
            Self::DestinationUnavailable => {
                "The destination cannot be written, so this transfer cannot be run again."
            }
            Self::Unsupported => {
                "This transfer's recorded request can no longer be planned, so it cannot be run again."
            }
        }
    }
}

/// What running an interrupted job again would do to the destination.
///
/// Surfaced before the user commits, because with the `replace` strategy a
/// restart overwrites the entries that are already there.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RestartImpact {
    pub strategy: ConflictStrategy,
    /// Entries that already exist where the transfer would write.
    pub conflicts: u64,
    /// Items the strategy would leave alone.
    pub skipped_items: u64,
    pub total_bytes: u64,
    pub total_files: u64,
    /// True when a restart would overwrite existing entries. The UI must ask
    /// before doing this.
    pub overwrites: bool,
}

/// One interrupted job as the frontend sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryCandidate {
    pub id: String,
    pub operation: TransferOperation,
    pub conflict: ConflictStrategy,
    pub verification: VerificationPolicy,
    pub sources: Vec<String>,
    pub destination: String,
    /// What the job was doing when its state was last written.
    pub status: TransferStatus,
    pub queued_at_ms: u64,
    pub started_at_ms: Option<u64>,
    pub updated_at_ms: u64,
    pub progress: PersistedProgress,
    /// Whole percent reported before the interruption, when measurable.
    pub percent: Option<u32>,
    pub verification_summary: Option<HistoryVerification>,
    pub outcome: RecoveryOutcome,
    /// Why the outcome is what it is, in the backend's own words.
    pub detail: Option<String>,
    /// Partial files this job left behind.
    pub artifacts: Vec<PartialArtifact>,
    pub artifact_bytes: u64,
    /// True when the artifact scan hit its bound and may have missed some.
    pub artifacts_truncated: bool,
    /// Directories the leftovers are in.
    pub artifact_directories: Vec<String>,
    /// What a restart would do, when it can be run again.
    pub restart_impact: Option<RestartImpact>,
    /// Evidence, never proof: whether a re-plan found every planned item in
    /// place with the expected size. `None` when that could not be determined.
    pub destination_looks_complete: Option<bool>,
    /// True when the archive's own record shows the job finished.
    pub confirmed_by_archive: bool,
    pub can_restart: bool,
    pub can_discard: bool,
}

/// Everything a probe learned about an interrupted job, so classification can
/// be a pure function of facts.
#[derive(Debug, Clone, Default, PartialEq, Eq)]
pub struct Probe {
    /// The archive holds a finished record for this job.
    pub archive_confirms_finish: bool,
    /// Whether every source in the request still exists.
    pub sources_present: bool,
    /// Whether the destination is an existing, writable directory.
    pub destination_usable: bool,
    /// Facts from re-planning the recorded request, when it could be planned.
    pub planned: Option<PlanFacts>,
    /// Why re-planning failed, when it did.
    pub plan_error: Option<StoredError>,
    /// Leftovers found in the destination tree.
    pub artifacts: ArtifactScan,
    /// Whether every planned item is present at the expected size.
    pub looks_complete: Option<bool>,
}

/// What re-planning the recorded request produced.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PlanFacts {
    pub total_bytes: u64,
    pub total_files: u64,
    pub conflicts: u64,
    pub skipped_items: u64,
}

/// Decides what should happen to an interrupted job.
///
/// Order matters: archive proof beats everything, then the blocking conditions,
/// then the ordinary case.
pub fn classify(
    job: &InterruptedTransfer,
    strategy: ConflictStrategy,
    probe: &Probe,
) -> (RecoveryOutcome, Option<String>) {
    if probe.archive_confirms_finish {
        return (
            RecoveryOutcome::CompletedBeforeCrash,
            Some(
                "The archive recorded this transfer reaching a terminal state before the application stopped."
                    .to_string(),
            ),
        );
    }

    if !probe.sources_present {
        return (
            RecoveryOutcome::SourceMissing,
            Some(format!(
                "At least one of the {} source(s) is no longer there.",
                job.request.sources.len()
            )),
        );
    }

    if !probe.destination_usable {
        return (
            RecoveryOutcome::DestinationUnavailable,
            Some(format!(
                "'{}' is not a writable directory right now.",
                job.request.destination
            )),
        );
    }

    let Some(planned) = probe.planned else {
        let detail = probe
            .plan_error
            .as_ref()
            .map(|error| error.message.clone())
            .unwrap_or_else(|| "the recorded request could not be planned".to_string());
        return (RecoveryOutcome::Unsupported, Some(detail));
    };

    let mut detail = String::new();
    if job.had_started() {
        detail.push_str(&format!(
            "The transfer stopped after reporting {} of {} bytes. Restarting starts every file from the beginning",
            job.progress.transferred_bytes, job.progress.total_bytes
        ));
        if !probe.artifacts.artifacts.is_empty() {
            detail.push_str(&format!(
                "; {} unfinished file(s) will be removed first",
                probe.artifacts.artifacts.len()
            ));
        }
        detail.push('.');
    } else {
        detail.push_str("The transfer had not started; restarting it repeats nothing.");
    }

    if planned.conflicts > 0 {
        detail.push_str(&format!(
            " {} entr{} already exist at the destination and the '{}' strategy will apply to them.",
            planned.conflicts,
            if planned.conflicts == 1 { "y" } else { "ies" },
            strategy.as_str()
        ));
    }

    if probe.looks_complete == Some(true) {
        detail.push_str(
            " Every planned item is already present at the expected size, but completion was never recorded, so this transfer is still treated as unfinished.",
        );
    }

    (RecoveryOutcome::RestartRequired, Some(detail))
}

fn lock<T>(mutex: &Mutex<T>) -> std::sync::MutexGuard<'_, T> {
    mutex.lock().unwrap_or_else(|error| error.into_inner())
}

fn persist(path: &std::path::Path, jobs: &[InterruptedTransfer]) -> AppResult<()> {
    write_document(
        path,
        &TransferStateDocument {
            jobs: jobs.to_vec(),
        },
    )
}

/// Drops records that cannot be used, keeping the rest.
fn sanitize(jobs: Vec<InterruptedTransfer>) -> Vec<InterruptedTransfer> {
    let mut kept = Vec::with_capacity(jobs.len());
    let mut dropped = 0usize;
    let mut seen = std::collections::HashSet::new();

    for job in jobs {
        if job.validate().is_err() || !seen.insert(job.id.clone()) {
            dropped += 1;
            continue;
        }
        kept.push(job);
    }

    if dropped > 0 {
        log::warn!("dropped {dropped} unusable interrupted-transfer records");
    }
    kept
}

/// Migration for the state document. See [`crate::history`] for the rationale.
fn migrate(
    _from: u32,
    document: serde_json::Map<String, serde_json::Value>,
) -> Result<serde_json::Map<String, serde_json::Value>, String> {
    crate::persistence::migrate_fill_defaults(0, document)
}

/// The action a caller may take on a candidate, validated against its outcome.
pub fn validate_action(candidate: &RecoveryCandidate, action: RecoveryAction) -> AppResult<()> {
    match action {
        RecoveryAction::Restart => {
            if candidate.can_restart {
                Ok(())
            } else {
                Err(AppError::RecoveryUnavailable(format!(
                    "transfer '{}' cannot be restarted: {}",
                    candidate.id,
                    candidate.outcome.explain()
                )))
            }
        }
        RecoveryAction::Discard => {
            if candidate.can_discard {
                Ok(())
            } else {
                Err(AppError::RecoveryUnavailable(format!(
                    "transfer '{}' has nothing that can be discarded",
                    candidate.id
                )))
            }
        }
        RecoveryAction::Confirm => {
            if candidate.confirmed_by_archive {
                Ok(())
            } else {
                Err(AppError::RecoveryUnavailable(format!(
                    "transfer '{}' is not confirmed complete by the archive, so it cannot be marked as such",
                    candidate.id
                )))
            }
        }
        RecoveryAction::Pending => Err(AppError::InvalidInput(
            "no recovery action was chosen".to_string(),
        )),
    }
}
