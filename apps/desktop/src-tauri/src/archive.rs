/* ==========================================================================
 * Transfer archive
 * The durable side of transfers: what happened (history), what did not finish
 * (interrupted-job state), and what to do about it (recovery).
 *
 * This module is the only place that knows both sides. The transfer engine
 * hands it snapshots through the `TransferJournal` port and never reads a file;
 * the command layer asks it for history and recovery answers. Everything it
 * writes goes through the atomic, versioned document layer in `persistence`.
 *
 * Write order is the correctness property that matters here:
 *
 * 1. while a job runs, its live state is replaced on a cadence;
 * 2. when it finishes, its history record is written **first**;
 * 3. only then is the live state forgotten.
 *
 * A crash between 2 and 3 leaves a job that recovery can recognise as finished
 * from the archive's own record, and it can never leave a finished job looking
 * unfinished.
 * ========================================================================== */

#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::Arc;

use crate::errors::{AppError, AppResult, StoredError};
use crate::history::{
    HistoryFilter, HistoryIssue, HistoryRecord, HistoryStatus, HistoryStore, HistoryVerification,
    PruneOutcome, RecoveryAction,
};
use crate::persistence::LoadStatus;
use crate::recovery::{
    artifact_directories, classify, discard, scan, InterruptedTransfer, PersistedProgress, Probe,
    RecoveryCandidate, RestartImpact, TransferStateStore, STATE_FILE,
};
use crate::transfer::{
    model::{ItemAction, ItemKind, TransferItem},
    plan, safety, ConflictStrategy, TransferEngine, TransferJournal, TransferRequest,
    TransferSnapshot,
};
use serde::Serialize;

/// How many issues a history record keeps for its details view.
///
/// The live queue already bounds its own list; history keeps a shorter one,
/// because a record is read long after the fact and its value is the summary.
const HISTORY_ISSUES: usize = 40;

/// How many planned items are checked before "does the destination look
/// complete" gives up and answers "unknown".
///
/// The question is only ever asked about an interrupted job, and it is evidence
/// rather than proof, so it must never turn startup into a long walk.
const LOOKS_COMPLETE_BUDGET: usize = 20_000;

/// The durable side of the transfer engine.
pub struct TransferArchive {
    history: Arc<HistoryStore>,
    state: Arc<TransferStateStore>,
    /// Status of each document as it was loaded, for the UI to report.
    history_status: LoadStatus,
    state_status: LoadStatus,
}

impl TransferArchive {
    /// Opens the archive inside an application directory.
    ///
    /// Never fails: a document that cannot be used is reported through the
    /// returned statuses and its default is used instead, because an
    /// application that cannot read its own history still has to start.
    pub fn open(directory: &Path, history_limit: u32) -> (Arc<Self>, LoadStatus, LoadStatus) {
        let (history, history_status) =
            HistoryStore::open(directory.join(crate::history::HISTORY_FILE), history_limit);
        let (state, state_status) = TransferStateStore::open(directory.join(STATE_FILE));

        let archive = Arc::new(Self {
            history: Arc::new(history),
            state: Arc::new(state),
            history_status,
            state_status,
        });
        let history_status = archive.history_status.clone();
        let state_status = archive.state_status.clone();
        (archive, history_status, state_status)
    }

    pub fn history(&self) -> &HistoryStore {
        &self.history
    }

    pub fn state(&self) -> &TransferStateStore {
        &self.state
    }

    /// How the history document loaded, for the UI to explain a degraded list.
    pub fn history_status(&self) -> &LoadStatus {
        &self.history_status
    }

    /// How the interrupted-state document loaded.
    pub fn state_status(&self) -> &LoadStatus {
        &self.state_status
    }

    /// Whether a document could not be used as written.
    ///
    /// Two shapes of trouble count: a document that was unusable and had to be
    /// set aside, and one written by a newer build that this build refuses to
    /// replace. Both mean the archive is running on less than it should be, and
    /// the user is told rather than left guessing.
    pub fn degraded(&self) -> bool {
        [&self.history_status, &self.state_status]
            .iter()
            .any(|status| {
                matches!(
                    status,
                    LoadStatus::Recovered { .. } | LoadStatus::Unsupported { .. }
                )
            })
    }

    /// The archive's health and current sizes.
    pub fn status(&self) -> ArchiveStatus {
        ArchiveStatus {
            history: DocumentStatus::from(&self.history_status),
            state: DocumentStatus::from(&self.state_status),
            degraded: self.degraded(),
            writable: self.writable(),
            history_records: u32::try_from(self.history.len()).unwrap_or(u32::MAX),
            history_limit: self.history.limit(),
            interrupted_jobs: u32::try_from(self.state.len()).unwrap_or(u32::MAX),
        }
    }

    /// Whether every document may be written back to disk.
    ///
    /// False only when a document belongs to a newer build, which is also the
    /// only condition under which the archive keeps working in memory while
    /// refusing to replace what it cannot understand.
    pub fn writable(&self) -> bool {
        self.history_status.writable() && self.state_status.writable()
    }

    // -- history ------------------------------------------------------------

    pub fn history_records(&self, filter: HistoryFilter) -> Vec<HistoryRecord> {
        self.history.list(filter)
    }

    pub fn history_record(&self, id: &str) -> AppResult<HistoryRecord> {
        self.history.get(id).ok_or_else(|| {
            AppError::InvalidInput(format!("no history entry with the identifier '{id}'"))
        })
    }

    pub fn delete_history_record(&self, id: &str) -> AppResult<bool> {
        self.history.delete(id)
    }

    pub fn clear_history(&self) -> AppResult<usize> {
        self.history.clear()
    }

    /// Applies a new retention bound, pruning older records if it shrank.
    pub fn set_history_limit(&self, limit: u32) -> AppResult<PruneOutcome> {
        self.history.set_limit(limit)
    }

    // -- recovery -----------------------------------------------------------

    /// Every job that was in flight when the application stopped, with what
    /// recovery makes of it.
    pub fn recovery_candidates(&self) -> Vec<RecoveryCandidate> {
        self.state
            .list()
            .iter()
            .map(|job| self.candidate(job))
            .collect()
    }

    /// One candidate, or a structured error naming what is missing.
    pub fn recovery_candidate(&self, id: &str) -> AppResult<RecoveryCandidate> {
        let job = self.state.get(id).ok_or_else(|| {
            AppError::RecoveryUnavailable(format!(
                "'{id}' is not an interrupted transfer this archive knows about"
            ))
        })?;
        Ok(self.candidate(&job))
    }

    /// Carries out a recovery decision.
    ///
    /// `Restart` removes the leftovers, queues the recorded request again, and
    /// accounts for the interrupted episode in history. `Discard` removes the
    /// leftovers and records the episode as interrupted. `Confirm` revises the
    /// history entry that proved the job finished, because that entry already
    /// exists — writing a second one for the same transfer would either be
    /// refused or hide the real result.
    ///
    /// The interrupted episode is only forgotten once its own history entry is
    /// on disk, so a failure at any point leaves the state that lets the user
    /// try again.
    pub fn recover(
        &self,
        engine: &TransferEngine,
        id: &str,
        action: RecoveryAction,
    ) -> AppResult<RecoveryReport> {
        let candidate = self.recovery_candidate(id)?;
        crate::recovery::validate_action(&candidate, action)?;

        // Leftovers go first: a restart must not leave another job's unfinished
        // files behind, and a discard exists to remove them.
        let removed = discard(&candidate.artifacts)?;

        let restarted_as = match action {
            RecoveryAction::Restart => {
                let job = self.state.get(id).ok_or_else(|| {
                    AppError::RecoveryUnavailable(format!("'{id}' disappeared during recovery"))
                })?;
                // The recorded request is replayed as-is: same sources, same
                // destination, same operation, same conflict strategy, same
                // verification policy the user originally chose.
                let snapshot = engine.enqueue_request(job.request.clone())?;
                Some(snapshot.id)
            }
            RecoveryAction::Discard | RecoveryAction::Confirm | RecoveryAction::Pending => None,
        };

        let status = match action {
            RecoveryAction::Restart | RecoveryAction::Discard => HistoryStatus::Interrupted,
            RecoveryAction::Confirm => HistoryStatus::Recovered,
            RecoveryAction::Pending => unreachable!("validated above"),
        };

        let pruned = match action {
            RecoveryAction::Confirm => {
                // The proven record is the job's real result, so it is kept
                // field for field; only the fact that recovery had to account
                // for the episode is added.
                let mut record = self.history_record(id)?;
                record.status = HistoryStatus::Recovered;
                record.recovery = Some(RecoveryAction::Confirm);
                self.history.update(record)?;
                PruneOutcome {
                    removed: 0,
                    invalid: 0,
                }
            }
            RecoveryAction::Restart | RecoveryAction::Discard => {
                let mut record = self.episode_record(&candidate, status, action);
                record.recovered_from = None;
                self.history.record(record)?
            }
            RecoveryAction::Pending => unreachable!("validated above"),
        };
        self.state.forget(id)?;

        Ok(RecoveryReport {
            candidate,
            action,
            restarted_as,
            recorded: pruned,
            artifacts_removed: removed.len(),
        })
    }

    /// The history entry for an interrupted episode.
    fn episode_record(
        &self,
        candidate: &RecoveryCandidate,
        status: HistoryStatus,
        action: RecoveryAction,
    ) -> HistoryRecord {
        HistoryRecord {
            id: candidate.id.clone(),
            operation: candidate.operation,
            conflict: candidate.conflict,
            status,
            sources: candidate.sources.clone(),
            destination: candidate.destination.clone(),
            total_bytes: candidate.progress.total_bytes,
            transferred_bytes: candidate.progress.transferred_bytes,
            total_files: candidate.progress.total_files,
            completed_files: candidate.progress.completed_files,
            total_directories: candidate.progress.total_directories,
            completed_directories: candidate.progress.completed_directories,
            skipped_items: candidate.progress.skipped_items,
            failed_items: candidate.progress.failed_items,
            queued_at_ms: candidate.queued_at_ms,
            started_at_ms: candidate.started_at_ms,
            finished_at_ms: candidate.updated_at_ms,
            duration_ms: 0,
            error: Some(StoredError::from(&AppError::TransferFailed(
                interrupted_note(candidate),
            ))),
            issues_truncated: false,
            issues: Vec::new(),
            verification: candidate.verification_summary.clone(),
            recovery: Some(action),
            recovered_from: None,
        }
    }

    /// Builds a candidate by inspecting the world the job left behind.
    fn candidate(&self, job: &InterruptedTransfer) -> RecoveryCandidate {
        let probe = self.probe(job);
        let (outcome, detail) = classify(job, job.request.conflict, &probe);

        let artifact_bytes = probe.artifacts.bytes();
        let restart_impact = probe.planned.map(|planned| RestartImpact {
            strategy: job.request.conflict,
            conflicts: planned.conflicts,
            skipped_items: planned.skipped_items,
            total_bytes: planned.total_bytes,
            total_files: planned.total_files,
            // Only `replace` overwrites what is already there; the other two
            // strategies leave existing entries alone.
            overwrites: job.request.conflict == ConflictStrategy::Replace && planned.conflicts > 0,
        });

        RecoveryCandidate {
            id: job.id.clone(),
            operation: job.request.operation,
            conflict: job.request.conflict,
            verification: job.request.verification_policy(),
            sources: job.request.sources.clone(),
            destination: job.request.destination.clone(),
            status: job.status,
            queued_at_ms: job.queued_at_ms,
            started_at_ms: job.started_at_ms,
            updated_at_ms: job.updated_at_ms,
            progress: job.progress,
            percent: job.progress.percent(),
            verification_summary: job.verification.clone(),
            outcome,
            detail,
            artifacts: probe.artifacts.artifacts.clone(),
            artifact_bytes,
            artifacts_truncated: probe.artifacts.truncated,
            artifact_directories: artifact_directories(&probe.artifacts.artifacts),
            restart_impact,
            destination_looks_complete: probe.looks_complete,
            confirmed_by_archive: probe.archive_confirms_finish,
            can_restart: outcome.can_restart(),
            // Discarding is always allowed: it is how an interrupted episode is
            // accounted for and cleared, whether or not it left leftovers.
            can_discard: true,
        }
    }

    /// Inspects the filesystem and the archive for one interrupted job.
    fn probe(&self, job: &InterruptedTransfer) -> Probe {
        let archive_confirms_finish = self.history.get(&job.id).is_some();
        let sources_present = job
            .request
            .sources
            .iter()
            .all(|source| Path::new(source).exists());

        // The same destination check the engine performs before queueing work:
        // it must exist, be a directory, and accept a write.
        let destination_usable =
            safety::validate_destination(&job.request.destination, true).is_ok();

        let artifacts = match Path::new(&job.request.destination) {
            destination if destination.is_dir() => scan(destination, &job.id),
            _ => Default::default(),
        };

        let planned = plan::plan_request(&job.request);
        let (planned_facts, plan_error, looks_complete) = match &planned {
            Ok(plan) => (
                Some(crate::recovery::PlanFacts {
                    total_bytes: plan.total_bytes,
                    total_files: plan.total_files,
                    conflicts: plan.conflicts,
                    skipped_items: plan.skipped_items,
                }),
                None,
                destination_looks_complete(plan, &artifacts),
            ),
            Err(error) => (None, Some(StoredError::from(error)), None),
        };

        Probe {
            archive_confirms_finish,
            sources_present,
            destination_usable,
            planned: planned_facts,
            plan_error,
            artifacts,
            looks_complete,
        }
    }
}

impl TransferJournal for TransferArchive {
    fn record_live(&self, snapshot: &TransferSnapshot) {
        // A terminal job belongs in history, not in the interrupted list. The
        // engine does not call this for one, and if it ever did, recording it
        // here would make a finished job look unfinished after a crash.
        if snapshot.status.is_terminal() {
            return;
        }

        let job = live_record(snapshot);
        if let Err(error) = self.state.upsert(job) {
            log::warn!("could not record the state of '{}': {error}", snapshot.id);
        }
    }

    fn record_finished(&self, snapshot: &TransferSnapshot) {
        let record = record_from_snapshot(snapshot);

        if let Err(error) = self.history.record(record) {
            // History first, live state second: if the record cannot be stored,
            // the interrupted state stays so nothing about this job is lost.
            log::error!(
                "could not record the result of '{}' in history: {error}",
                snapshot.id
            );
            return;
        }

        if let Err(error) = self.state.forget(&snapshot.id) {
            log::warn!(
                "could not clear the live state of '{}': {error}",
                snapshot.id
            );
        }
    }

    fn forget(&self, job_id: &str) {
        if let Err(error) = self.state.forget(job_id) {
            log::warn!("could not clear the live state of '{job_id}': {error}");
        }
    }
}

impl std::fmt::Debug for TransferArchive {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter
            .debug_struct("TransferArchive")
            .field("history", &self.history)
            .field("state", &self.state)
            .finish()
    }
}

/// How one durable document loaded, as the frontend sees it.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DocumentStatus {
    pub state: crate::persistence::LoadState,
    /// The backend's own words when the outcome needs explaining.
    pub detail: Option<String>,
}

impl From<&LoadStatus> for DocumentStatus {
    fn from(status: &LoadStatus) -> Self {
        Self {
            state: status.state(),
            detail: status.detail(),
        }
    }
}

/// The archive's own health, so a degraded history or recovery list can be
/// explained instead of silently looking empty.
#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ArchiveStatus {
    pub history: DocumentStatus,
    pub state: DocumentStatus,
    pub degraded: bool,
    pub writable: bool,
    /// Records currently kept.
    pub history_records: u32,
    /// The retention bound in force.
    pub history_limit: u32,
    /// Jobs currently listed as interrupted.
    pub interrupted_jobs: u32,
}

/// What a recovery decision did.
#[derive(Debug, Clone, PartialEq, Eq, serde::Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RecoveryReport {
    pub candidate: RecoveryCandidate,
    pub action: RecoveryAction,
    /// Identifier of the job the restart produced, when it was restarted.
    pub restarted_as: Option<String>,
    /// How many records the history prune removed, if any.
    pub recorded: PruneOutcome,
    pub artifacts_removed: usize,
}

/// The live-state record for a snapshot.
///
/// The request is reconstructed from the snapshot, which carries every field of
/// it, so a restart replays exactly what the user asked for.
pub fn live_record(snapshot: &TransferSnapshot) -> InterruptedTransfer {
    InterruptedTransfer {
        id: snapshot.id.clone(),
        request: request_from_snapshot(snapshot),
        status: snapshot.status,
        queued_at_ms: snapshot.queued_at_ms,
        started_at_ms: snapshot.started_at_ms,
        updated_at_ms: now_ms(),
        progress: PersistedProgress {
            total_bytes: snapshot.progress.total_bytes,
            transferred_bytes: snapshot.progress.transferred_bytes,
            total_files: snapshot.progress.total_files,
            completed_files: snapshot.progress.completed_files,
            total_directories: snapshot.progress.total_directories,
            completed_directories: snapshot.progress.completed_directories,
            skipped_items: snapshot.progress.skipped_items,
            failed_items: snapshot.progress.failed_items,
        },
        verification: Some(HistoryVerification::from(&snapshot.verification)),
    }
}

/// The request a snapshot is running, rebuilt field for field.
pub fn request_from_snapshot(snapshot: &TransferSnapshot) -> TransferRequest {
    TransferRequest {
        sources: snapshot.sources.clone(),
        destination: snapshot.destination.clone(),
        operation: snapshot.operation,
        conflict: snapshot.conflict,
        verification: Some(snapshot.verification.policy),
    }
}

/// The durable record of a finished job.
pub fn record_from_snapshot(snapshot: &TransferSnapshot) -> HistoryRecord {
    let status = HistoryStatus::from_transfer_status(snapshot.status).unwrap_or(
        // The engine only reports finished jobs here; anything else is a
        // failed transfer rather than a silently missing record.
        HistoryStatus::Failed,
    );

    let issues_truncated = snapshot.issues_truncated || snapshot.issues.len() > HISTORY_ISSUES;

    HistoryRecord {
        id: snapshot.id.clone(),
        operation: snapshot.operation,
        conflict: snapshot.conflict,
        status,
        sources: snapshot.sources.clone(),
        destination: snapshot.destination.clone(),
        total_bytes: snapshot.progress.total_bytes,
        transferred_bytes: snapshot.progress.transferred_bytes,
        total_files: snapshot.progress.total_files,
        completed_files: snapshot.progress.completed_files,
        total_directories: snapshot.progress.total_directories,
        completed_directories: snapshot.progress.completed_directories,
        skipped_items: snapshot.progress.skipped_items,
        failed_items: snapshot.progress.failed_items,
        queued_at_ms: snapshot.queued_at_ms,
        started_at_ms: snapshot.started_at_ms,
        finished_at_ms: snapshot.finished_at_ms.unwrap_or_else(now_ms),
        duration_ms: snapshot.progress.elapsed_ms,
        error: snapshot.error.as_ref().map(StoredError::from),
        issues_truncated,
        issues: snapshot
            .issues
            .iter()
            .take(HISTORY_ISSUES)
            .map(HistoryIssue::from)
            .collect(),
        verification: Some(HistoryVerification::from(&snapshot.verification)),
        recovery: None,
        recovered_from: None,
    }
}

/// Whether every planned file is already present at the size the plan measured.
///
/// Evidence only. This answer is never treated as proof that a transfer
/// finished, and its absence (`None`) is not evidence of anything either.
fn destination_looks_complete(
    plan: &crate::transfer::TransferPlan,
    artifacts: &crate::recovery::ArtifactScan,
) -> Option<bool> {
    if !artifacts.artifacts.is_empty() {
        // A leftover partial file means the job was interrupted mid-file.
        return Some(false);
    }

    let files = plan
        .items
        .iter()
        .filter(|item| item.kind == ItemKind::File && item.action == ItemAction::Transfer);

    let mut checked = 0usize;
    for item in files {
        checked += 1;
        if checked > LOOKS_COMPLETE_BUDGET {
            return None;
        }
        if !file_matches(item) {
            return Some(false);
        }
    }

    // The destinations of the directories themselves are checked too, so a job
    // that only ever created folders is not reported as incomplete.
    let directories = plan
        .items
        .iter()
        .filter(|item| item.kind == ItemKind::Directory && item.action == ItemAction::Transfer);
    for item in directories {
        checked += 1;
        if checked > LOOKS_COMPLETE_BUDGET {
            return None;
        }
        if !item.destination.is_dir() {
            return Some(false);
        }
    }

    Some(true)
}

fn file_matches(item: &TransferItem) -> bool {
    match std::fs::symlink_metadata(&item.destination) {
        Ok(metadata) => metadata.file_type().is_file() && metadata.len() == item.size_bytes,
        Err(_) => false,
    }
}

/// One line explaining an interrupted episode to a reader of history.
fn interrupted_note(candidate: &RecoveryCandidate) -> String {
    format!(
        "the application stopped while this transfer was {}; {}",
        candidate.status.as_str(),
        candidate.outcome.explain()
    )
}

fn now_ms() -> u64 {
    std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|elapsed| u64::try_from(elapsed.as_millis()).unwrap_or(u64::MAX))
        .unwrap_or(0)
}
