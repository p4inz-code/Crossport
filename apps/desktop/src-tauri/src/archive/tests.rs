/* ==========================================================================
 * Archive tests
 * The archive is where the transfer engine, durable state, recovery, and
 * history meet, so these tests drive the real engine against real files and
 * then inspect what a restart would see.
 *
 * The properties being defended here:
 *
 * - a finished job is recorded in history and leaves no interrupted state;
 * - a job that stopped mid-flight is reported as interrupted, with an outcome
 *   that never claims success;
 * - restarting removes the leftovers and replays the recorded request;
 * - discarding accounts for the episode without running anything;
 * - a corrupt document is reported and set aside instead of taking the
 *   application down.
 * ========================================================================== */

use super::*;
use crate::filesystem::test_support::unique_temp_dir;
use crate::history::HistoryStatus;
use crate::recovery::{InterruptedTransfer, PersistedProgress, RecoveryOutcome};
use crate::transfer::{TransferOperation, TransferRequest, TransferStatus};
use crate::verification::{VerificationPolicy, VerificationStatus};

use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const TIMEOUT: Duration = Duration::from_secs(30);

fn clean_up(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

fn write_file(path: &Path, bytes: usize) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent is creatable");
    }
    std::fs::write(path, vec![7u8; bytes]).expect("file is writable");
}

fn request_for(
    sources: &[&Path],
    destination: &Path,
    operation: TransferOperation,
    conflict: ConflictStrategy,
    verification: VerificationPolicy,
) -> TransferRequest {
    TransferRequest {
        sources: sources
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        destination: destination.display().to_string(),
        operation,
        conflict,
        verification: Some(verification),
    }
}

fn wait_for_status(engine: &TransferEngine, id: &str, status: TransferStatus) -> TransferSnapshot {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let snapshot = engine.snapshot(id).expect("the job exists");
        if snapshot.status == status {
            return snapshot;
        }
        if snapshot.status.is_terminal() || Instant::now() > deadline {
            panic!("'{id}' never reached {status:?}; last state: {snapshot:?}");
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Waits until the archive records the job, which is what a client observes
/// after the completion event. The engine sets the terminal status just before
/// it writes history, so "the job says completed" is not yet "the record is on
/// disk" — a race a real crash can also produce, which recovery handles.
fn wait_for_history(archive: &TransferArchive, id: &str) -> HistoryRecord {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if let Ok(record) = archive.history_record(id) {
            return record;
        }
        if Instant::now() > deadline {
            panic!("'{id}' was never recorded in history");
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Waits until a document exists on disk, so a test that reopens the archive
/// cannot race the write that `record` performs after its in-memory insert.
fn wait_for_file(path: &Path) {
    let deadline = Instant::now() + TIMEOUT;
    while !path.is_file() {
        if Instant::now() > deadline {
            panic!("'{}' was never written", path.display());
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Waits until the state document on disk no longer lists a job.
///
/// The store releases its lock before it writes the document, so waiting for
/// the in-memory list to empty is not the same as waiting for the write to
/// land. A test that wants to prove what a restart would see must wait for the
/// document itself.
fn wait_for_state_file_clear(path: &Path, job_id: &str) {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let cleared = match std::fs::read_to_string(path) {
            Ok(contents) => serde_json::from_str::<serde_json::Value>(&contents)
                .ok()
                .map(|document| {
                    document
                        .get("jobs")
                        .and_then(serde_json::Value::as_array)
                        .map(|jobs| {
                            jobs.iter().all(|job| {
                                job.get("id").and_then(serde_json::Value::as_str) != Some(job_id)
                            })
                        })
                        // A document without a job list records nothing.
                        .unwrap_or(true)
                })
                // An unusable document is not proof of anything; keep waiting.
                .unwrap_or(false),
            // No file at all is the same as nothing recorded.
            Err(_) => true,
        };
        if cleared {
            return;
        }
        if Instant::now() > deadline {
            panic!("'{job_id}' is still listed as an interrupted transfer on disk");
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// Waits until the live state of a finished job is released. History is written
/// before the state is cleared, so this is the last step of a completion.
fn wait_for_state_clear(archive: &TransferArchive, id: &str) {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        if archive.state().get(id).is_none() {
            return;
        }
        if Instant::now() > deadline {
            panic!("'{id}' is still recorded as interrupted");
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

/// An archive and an engine wired to it, inside a fresh temporary directory.
fn archive(label: &str) -> (PathBuf, Arc<TransferArchive>, TransferEngine) {
    let workspace = unique_temp_dir(label);
    let (archive, history_status, state_status) = TransferArchive::open(
        &workspace.join("config"),
        crate::history::DEFAULT_HISTORY_LIMIT,
    );
    assert_eq!(history_status, LoadStatus::Missing);
    assert_eq!(state_status, LoadStatus::Missing);

    let engine = TransferEngine::new(1);
    engine.attach_journal(Arc::clone(&archive) as Arc<dyn TransferJournal>);
    (workspace, archive, engine)
}

/// A record of a job that was running when the application stopped.
fn interrupted_job(
    id: &str,
    sources: &[&Path],
    destination: &Path,
    operation: TransferOperation,
    conflict: ConflictStrategy,
    transferred_bytes: u64,
) -> InterruptedTransfer {
    InterruptedTransfer {
        id: id.to_string(),
        request: request_for(
            sources,
            destination,
            operation,
            conflict,
            VerificationPolicy::Size,
        ),
        status: TransferStatus::Running,
        queued_at_ms: 1_700_000_000_000,
        started_at_ms: Some(1_700_000_000_100),
        updated_at_ms: 1_700_000_000_900,
        progress: PersistedProgress {
            total_bytes: 1_024,
            transferred_bytes,
            total_files: 1,
            completed_files: 0,
            total_directories: 0,
            completed_directories: 0,
            skipped_items: 0,
            failed_items: 0,
        },
        verification: None,
    }
}

// -- finished jobs ----------------------------------------------------------

#[test]
fn a_finished_job_lands_in_history_and_leaves_no_interrupted_state() {
    let (workspace, archive, engine) = archive("archive-finished");
    let source = workspace.join("in").join("report.txt");
    write_file(&source, 4_096);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination");

    let queued = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
            VerificationPolicy::Size,
        ))
        .expect("the request is valid");
    wait_for_status(&engine, &queued.id, TransferStatus::Completed);

    let record = wait_for_history(&archive, &queued.id);
    assert_eq!(record.status, HistoryStatus::Completed);
    assert_eq!(record.operation, TransferOperation::Copy);
    assert_eq!(record.sources, vec![source.display().to_string()]);
    assert_eq!(record.destination, destination.display().to_string());
    assert_eq!(record.total_bytes, 4_096);
    assert_eq!(record.transferred_bytes, 4_096);
    assert_eq!(record.completed_files, 1);
    assert!(record.finished_at_ms >= record.queued_at_ms);
    assert!(record.error.is_none());

    let verification = record.verification.expect("verification is recorded");
    assert_eq!(verification.policy, VerificationPolicy::Size);
    assert_eq!(verification.status, VerificationStatus::Verified);
    assert_eq!(verification.verified_files, 1);

    wait_for_state_clear(&archive, &queued.id);
    wait_for_state_file_clear(
        &workspace.join("config").join(crate::recovery::STATE_FILE),
        &queued.id,
    );
    assert!(
        archive.state().is_empty(),
        "a finished job must not be left in the interrupted list"
    );

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn a_completed_transfer_survives_a_restart_of_the_archive() {
    let (workspace, archive, engine) = archive("archive-restart");
    let source = workspace.join("in").join("data.bin");
    write_file(&source, 2_048);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination");

    let queued = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
            VerificationPolicy::Checksum,
        ))
        .expect("the request is valid");
    wait_for_status(&engine, &queued.id, TransferStatus::Completed);
    wait_for_history(&archive, &queued.id);
    wait_for_state_clear(&archive, &queued.id);
    wait_for_state_file_clear(
        &workspace.join("config").join(crate::recovery::STATE_FILE),
        &queued.id,
    );
    wait_for_file(&workspace.join("config").join(crate::history::HISTORY_FILE));

    let (reopened, history_status, state_status) = TransferArchive::open(
        &workspace.join("config"),
        crate::history::DEFAULT_HISTORY_LIMIT,
    );
    assert_eq!(history_status, LoadStatus::Loaded);
    assert!(
        state_status == LoadStatus::Missing
            || (matches!(state_status, LoadStatus::Loaded) && reopened.state().is_empty()),
        "nothing may be left running after a finished transfer: {state_status:?}"
    );

    let record = reopened
        .history_record(&queued.id)
        .expect("history survived the restart");
    assert_eq!(record.status, HistoryStatus::Completed);
    assert_eq!(
        record.verification.expect("verification recorded").status,
        VerificationStatus::Verified
    );

    engine.shutdown();
    clean_up(&workspace);
}

/// A file big enough that a cancellation lands while it is still copying.
const CANCELLATION_BYTES: usize = 32 * 1024 * 1024;

#[test]
fn a_cancelled_transfer_is_recorded_as_cancelled_and_leaves_nothing_running() {
    let (workspace, archive, engine) = archive("archive-cancelled");
    let source = workspace.join("in").join("payload.bin");
    write_file(&source, CANCELLATION_BYTES);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination");

    let queued = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
            VerificationPolicy::Size,
        ))
        .expect("the request is valid");

    // Wait until it is actually moving data, so the cancel is a real mid-flight
    // cancellation rather than a cancelled queued job.
    let deadline = Instant::now() + TIMEOUT;
    while engine
        .snapshot(&queued.id)
        .expect("the job exists")
        .progress
        .transferred_bytes
        == 0
    {
        assert!(Instant::now() < deadline, "the transfer never started");
        std::thread::sleep(Duration::from_millis(2));
    }
    engine.cancel(&queued.id).expect("the job is cancellable");
    wait_for_status(&engine, &queued.id, TransferStatus::Cancelled);

    let record = wait_for_history(&archive, &queued.id);
    assert_eq!(record.status, HistoryStatus::Cancelled);
    assert!(
        record.error.is_none(),
        "cancelling is not a failure: {:?}",
        record.error
    );
    wait_for_state_file_clear(
        &workspace.join("config").join(crate::recovery::STATE_FILE),
        &queued.id,
    );
    assert!(
        archive.state().get(&queued.id).is_none(),
        "a cancelled job must not be left as an interrupted one"
    );

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn a_failed_transfer_is_recorded_with_the_reason_it_failed_for() {
    let (workspace, archive, engine) = archive("archive-failed");
    let source = workspace.join("in").join("report.txt");
    write_file(&source, 1_024);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination");

    // Plan first, then remove the source, so the job fails while it runs rather
    // than being turned away at planning time. Blocking the destination with a
    // read-only file was the earlier way to fail this job, but that only fails
    // on Windows: POSIX lets the owner of a writable directory unlink a
    // read-only file, so the same setup completes normally there. A source that
    // vanishes after planning fails identically on every platform — the same
    // mechanism the engine's own failure tests use.
    let plan = crate::transfer::plan::plan_for_start(&request_for(
        &[&source],
        &destination,
        TransferOperation::Copy,
        ConflictStrategy::Replace,
        VerificationPolicy::Size,
    ))
    .expect("the plan succeeds");
    std::fs::remove_file(&source).expect("the test owns the source");
    let queued = engine.enqueue(plan).expect("the job is accepted");
    wait_for_status(&engine, &queued.id, TransferStatus::Failed);

    let record = wait_for_history(&archive, &queued.id);
    assert_eq!(record.status, HistoryStatus::Failed);
    assert_eq!(record.failed_items, 1);
    let summary = record.error.expect("a failed record keeps its reason");
    assert!(
        summary.message.contains("did not transfer"),
        "the job-level reason is the summary, got '{}'",
        summary.message
    );

    // The cause lives on the item that failed, with the backend's own code.
    let issue = record
        .issues
        .iter()
        .find(|issue| issue.reason == crate::transfer::TransferIssueReason::Failed)
        .expect("the failed item is recorded");
    let cause = issue.error.clone().expect("the item keeps its error");
    assert_eq!(
        cause.code, "path_not_found",
        "the cause names what went wrong, got '{}'",
        cause.code
    );
    assert!(!cause.message.is_empty());
    wait_for_state_file_clear(
        &workspace.join("config").join(crate::recovery::STATE_FILE),
        &queued.id,
    );

    engine.shutdown();
    clean_up(&workspace);
}

// -- interrupted jobs -------------------------------------------------------

#[test]
fn a_job_that_was_running_is_reported_as_an_interrupted_candidate() {
    let (workspace, archive, engine) = archive("archive-candidate");
    let source = workspace.join("in").join("report.txt");
    write_file(&source, 1_024);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination");

    archive
        .state()
        .upsert(interrupted_job(
            "transfer-interrupted",
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
            256,
        ))
        .expect("state is writable");

    let candidates = archive.recovery_candidates();
    assert_eq!(candidates.len(), 1);
    let candidate = &candidates[0];

    assert_eq!(candidate.id, "transfer-interrupted");
    assert_eq!(candidate.outcome, RecoveryOutcome::RestartRequired);
    assert!(candidate.can_restart);
    assert!(candidate.can_discard);
    assert_eq!(candidate.percent, Some(25));
    assert_eq!(candidate.operation, TransferOperation::Copy);
    assert_eq!(candidate.destination, destination.display().to_string());
    assert!(!candidate.confirmed_by_archive);
    assert!(
        candidate
            .detail
            .as_ref()
            .expect("a reason is given")
            .contains("256 of 1024 bytes"),
        "{:?}",
        candidate.detail
    );

    assert_eq!(
        archive.recovery_candidate("transfer-interrupted").unwrap(),
        *candidate
    );
    let missing = archive
        .recovery_candidate("transfer-nope")
        .expect_err("an unknown job is a structured error");
    assert_eq!(missing.code(), "recovery_unavailable");

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn a_missing_source_blocks_a_restart_without_claiming_success() {
    let (workspace, archive, engine) = archive("archive-source-missing");
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination");

    archive
        .state()
        .upsert(interrupted_job(
            "transfer-interrupted",
            &[&workspace.join("in").join("gone.txt")],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
            0,
        ))
        .expect("state is writable");

    let candidate = archive.recovery_candidate("transfer-interrupted").unwrap();
    assert_eq!(candidate.outcome, RecoveryOutcome::SourceMissing);
    assert!(!candidate.can_restart);

    let error = archive
        .recover(
            &engine,
            "transfer-interrupted",
            crate::history::RecoveryAction::Restart,
        )
        .expect_err("a job whose source is gone cannot be restarted");
    assert_eq!(error.code(), "recovery_unavailable");
    assert!(
        archive.state().get("transfer-interrupted").is_some(),
        "a refused action must leave the episode available"
    );

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn an_unusable_destination_blocks_a_restart() {
    let (workspace, archive, engine) = archive("archive-destination-gone");
    let source = workspace.join("in").join("report.txt");
    write_file(&source, 512);

    archive
        .state()
        .upsert(interrupted_job(
            "transfer-interrupted",
            &[&source],
            &workspace.join("never-created"),
            TransferOperation::Copy,
            ConflictStrategy::Skip,
            0,
        ))
        .expect("state is writable");

    let candidate = archive.recovery_candidate("transfer-interrupted").unwrap();
    assert_eq!(candidate.outcome, RecoveryOutcome::DestinationUnavailable);
    assert!(!candidate.can_restart);

    engine.shutdown();
    clean_up(&workspace);
}

// -- recovery actions -------------------------------------------------------

#[test]
fn restarting_removes_the_leftovers_and_queues_the_recorded_request() {
    let (workspace, archive, engine) = archive("archive-restart-action");
    let source = workspace.join("in").join("report.txt");
    write_file(&source, 1_024);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination");

    // What an interrupted job actually leaves behind: a partial file and a
    // persisted record that does not prove anything about the destination.
    let leftover = destination.join(".crossport-transfer-interrupted-0.partial");
    std::fs::write(&leftover, vec![0u8; 600]).expect("partial artifact");
    archive
        .state()
        .upsert(interrupted_job(
            "transfer-interrupted",
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
            600,
        ))
        .expect("state is writable");

    let report = archive
        .recover(
            &engine,
            "transfer-interrupted",
            crate::history::RecoveryAction::Restart,
        )
        .expect("the restart is allowed");

    assert!(!leftover.exists(), "the leftovers are removed first");
    assert_eq!(report.artifacts_removed, 1);

    let restarted = report.restarted_as.expect("a new job was queued");
    assert_ne!(restarted, "transfer-interrupted");
    wait_for_status(&engine, &restarted, TransferStatus::Completed);

    assert_eq!(
        std::fs::read(destination.join("report.txt"))
            .expect("the file arrived")
            .len(),
        1_024
    );

    assert!(
        archive.state().get("transfer-interrupted").is_none(),
        "the episode is cleared once it is accounted for"
    );
    let episode = archive
        .history_record("transfer-interrupted")
        .expect("the episode is recorded");
    assert_eq!(episode.status, HistoryStatus::Interrupted);
    assert_eq!(
        episode.recovery,
        Some(crate::history::RecoveryAction::Restart)
    );
    assert_eq!(episode.transferred_bytes, 600);
    assert!(
        episode
            .error
            .expect("the episode keeps its reason")
            .message
            .contains("stopped"),
        "the recorded reason explains what happened"
    );

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn discarding_accounts_for_the_episode_without_running_anything() {
    let (workspace, archive, engine) = archive("archive-discard");
    let source = workspace.join("in").join("report.txt");
    write_file(&source, 1_024);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination");
    let leftover = destination.join(".crossport-transfer-interrupted-0.partial");
    std::fs::write(&leftover, vec![0u8; 100]).expect("partial artifact");
    std::fs::write(destination.join("keep.txt"), b"user data").expect("user file");

    archive
        .state()
        .upsert(interrupted_job(
            "transfer-interrupted",
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
            100,
        ))
        .expect("state is writable");

    let report = archive
        .recover(
            &engine,
            "transfer-interrupted",
            crate::history::RecoveryAction::Discard,
        )
        .expect("discarding is always allowed");

    assert!(report.restarted_as.is_none(), "nothing runs again");
    assert!(!leftover.exists(), "the leftovers are removed");
    assert!(
        destination.join("keep.txt").is_file(),
        "user data is never removed by a discard"
    );
    assert!(archive.state().is_empty());
    assert!(engine.snapshots().is_empty(), "no job was queued");

    let episode = archive
        .history_record("transfer-interrupted")
        .expect("the episode is recorded");
    assert_eq!(episode.status, HistoryStatus::Interrupted);
    assert_eq!(
        episode.recovery,
        Some(crate::history::RecoveryAction::Discard)
    );

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn the_archive_refuses_to_confirm_a_job_it_did_not_prove_finished() {
    let (workspace, archive, engine) = archive("archive-confirm-unproven");
    let source = workspace.join("in").join("report.txt");
    write_file(&source, 1_024);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination");

    // The file is physically there, which is exactly the situation the rule
    // exists for: presence is not proof.
    std::fs::write(destination.join("report.txt"), vec![7u8; 1_024]).expect("file in place");
    archive
        .state()
        .upsert(interrupted_job(
            "transfer-interrupted",
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
            0,
        ))
        .expect("state is writable");

    let candidate = archive.recovery_candidate("transfer-interrupted").unwrap();
    assert_eq!(
        candidate.destination_looks_complete,
        Some(true),
        "the destination really does look complete"
    );
    assert_eq!(
        candidate.outcome,
        RecoveryOutcome::RestartRequired,
        "looking complete is still not proof"
    );
    assert!(!candidate.confirmed_by_archive);

    let error = archive
        .recover(
            &engine,
            "transfer-interrupted",
            crate::history::RecoveryAction::Confirm,
        )
        .expect_err("a job the archive did not confirm cannot be marked complete");
    assert_eq!(error.code(), "recovery_unavailable");

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn a_job_the_archive_proved_finished_is_confirmed_rather_than_restarted() {
    let (workspace, archive, engine) = archive("archive-confirm-proven");
    let source = workspace.join("in").join("report.txt");
    write_file(&source, 1_024);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination");
    let id = "transfer-finished-before-crash";

    // The job finished and its history record was written, but the application
    // stopped before its live state could be cleared. This is the only shape in
    // which a job may be declared complete by recovery, and it is written
    // directly here so the test cannot race the engine's own completion.
    archive
        .history()
        .record(crate::history::HistoryRecord {
            id: id.to_string(),
            status: HistoryStatus::Completed,
            sources: vec![source.display().to_string()],
            destination: destination.display().to_string(),
            ..crate::history::HistoryRecord::default()
        })
        .expect("the finished record is stored");
    archive
        .state()
        .upsert(interrupted_job(
            id,
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
            0,
        ))
        .expect("state is writable");

    let candidate = archive.recovery_candidate(id).unwrap();
    assert!(candidate.confirmed_by_archive);
    assert_eq!(candidate.outcome, RecoveryOutcome::CompletedBeforeCrash);
    assert!(!candidate.can_restart);

    let report = archive
        .recover(&engine, id, crate::history::RecoveryAction::Confirm)
        .expect("the archive proved it");
    assert!(report.restarted_as.is_none());
    assert!(archive.state().is_empty());
    assert_eq!(
        archive.history_record(id).unwrap().status,
        HistoryStatus::Recovered,
        "the episode is recorded as recovered"
    );

    engine.shutdown();
    clean_up(&workspace);
}

// -- hostile persistence ----------------------------------------------------

#[test]
fn a_corrupt_state_document_starts_clean_and_is_reported() {
    let workspace = unique_temp_dir("archive-corrupt-state");
    let config = workspace.join("config");
    std::fs::create_dir_all(&config).expect("config dir");
    std::fs::write(config.join(crate::recovery::STATE_FILE), b"not json at all").expect("write");

    let (archive, history_status, state_status) =
        TransferArchive::open(&config, crate::history::DEFAULT_HISTORY_LIMIT);

    assert_eq!(history_status, LoadStatus::Missing);
    assert!(matches!(state_status, LoadStatus::Recovered { .. }));
    assert!(archive.degraded(), "the degraded document is reported");
    assert!(archive.recovery_candidates().is_empty());
    assert!(
        archive.state().writable(),
        "the application must be able to record state again"
    );

    clean_up(&workspace);
}

#[test]
fn a_corrupt_history_document_starts_clean_and_still_records_new_transfers() {
    let workspace = unique_temp_dir("archive-corrupt-history");
    let config = workspace.join("config");
    std::fs::create_dir_all(&config).expect("config dir");
    std::fs::write(config.join(crate::history::HISTORY_FILE), b"[1, 2, 3]").expect("write");

    let (archive, history_status, state_status) =
        TransferArchive::open(&config, crate::history::DEFAULT_HISTORY_LIMIT);

    assert!(matches!(history_status, LoadStatus::Recovered { .. }));
    assert_eq!(state_status, LoadStatus::Missing);
    assert!(archive.degraded());
    assert!(archive.history_records(HistoryFilter::All).is_empty());

    // The store must still work: start a real transfer and watch it land.
    let engine = TransferEngine::new(1);
    engine.attach_journal(Arc::clone(&archive) as Arc<dyn TransferJournal>);
    let source = workspace.join("in").join("report.txt");
    write_file(&source, 256);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination");

    let queued = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
            VerificationPolicy::Size,
        ))
        .expect("the request is valid");
    wait_for_status(&engine, &queued.id, TransferStatus::Completed);

    assert_eq!(
        archive.history_record(&queued.id).unwrap().status,
        HistoryStatus::Completed
    );

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn a_newer_history_schema_is_never_overwritten() {
    let workspace = unique_temp_dir("archive-newer-schema");
    let config = workspace.join("config");
    std::fs::create_dir_all(&config).expect("config dir");
    std::fs::write(
        config.join(crate::history::HISTORY_FILE),
        br#"{"schemaVersion":99,"records":[{"id":"future"}]}"#,
    )
    .expect("write");

    let (archive, history_status, _state_status) =
        TransferArchive::open(&config, crate::history::DEFAULT_HISTORY_LIMIT);

    assert!(matches!(history_status, LoadStatus::Unsupported { .. }));
    assert!(!archive.writable(), "a newer document is read-only");
    assert!(archive.degraded(), "an unusable document must be reported");

    let original = std::fs::read(config.join(crate::history::HISTORY_FILE)).expect("readable");
    archive
        .history()
        .record(crate::history::HistoryRecord {
            id: "transfer-new".to_string(),
            status: HistoryStatus::Completed,
            sources: vec!["C:\\source\\report.txt".to_string()],
            destination: "C:\\destination".to_string(),
            ..crate::history::HistoryRecord::default()
        })
        .expect("the in-memory view still works");
    assert_eq!(
        std::fs::read(config.join(crate::history::HISTORY_FILE)).expect("readable"),
        original,
        "the newer document on disk must not be replaced"
    );

    clean_up(&workspace);
}

#[test]
fn history_records_are_filtered_newest_first() {
    let (workspace, archive, engine) = archive("archive-filter");
    let config = workspace.join("config");
    let source = workspace.join("in").join("report.txt");
    write_file(&source, 128);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination");

    let first = archive
        .history()
        .record(crate::history::HistoryRecord {
            id: "transfer-old".to_string(),
            status: HistoryStatus::Completed,
            sources: vec![source.display().to_string()],
            destination: destination.display().to_string(),
            total_bytes: 128,
            transferred_bytes: 128,
            finished_at_ms: 10,
            ..crate::history::HistoryRecord::default()
        })
        .expect("a valid record is stored");
    assert_eq!(first.removed, 0);

    let queued = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
            VerificationPolicy::Size,
        ))
        .expect("the request is valid");
    wait_for_history(&archive, &queued.id);

    let all = archive.history_records(HistoryFilter::All);
    assert_eq!(all.len(), 2);
    assert_eq!(all[0].id, queued.id, "newest first");
    assert_eq!(all[1].id, "transfer-old");

    let completed = archive.history_records(HistoryFilter::Completed);
    assert_eq!(completed.len(), 2);

    let failed = archive.history_records(HistoryFilter::Failed);
    assert!(failed.is_empty());

    assert!(archive.delete_history_record("transfer-old").unwrap());
    assert!(!archive.delete_history_record("transfer-old").unwrap());
    assert_eq!(archive.history_records(HistoryFilter::All).len(), 1);

    // What was deleted must stay deleted across a reopen.
    let (reopened, _, _) = TransferArchive::open(&config, crate::history::DEFAULT_HISTORY_LIMIT);
    assert_eq!(reopened.history_records(HistoryFilter::All).len(), 1);

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn retention_keeps_the_newest_records() {
    let (workspace, archive, engine) = archive("archive-retention");
    let limit = crate::history::MIN_HISTORY_LIMIT;

    archive
        .set_history_limit(limit)
        .expect("the limit is applied");

    for index in 0..(limit + 5) {
        archive
            .history()
            .record(crate::history::HistoryRecord {
                id: format!("transfer-{index}"),
                status: HistoryStatus::Completed,
                sources: vec!["C:\\source\\report.txt".to_string()],
                destination: "C:\\destination".to_string(),
                finished_at_ms: index as u64,
                ..crate::history::HistoryRecord::default()
            })
            .expect("the record is stored");
    }

    let records = archive.history_records(HistoryFilter::All);
    assert_eq!(records.len() as u32, limit);
    assert_eq!(
        records[0].id,
        format!("transfer-{}", limit + 4),
        "the newest record is kept"
    );
    assert!(
        !records.iter().any(|record| record.id == "transfer-0"),
        "the oldest record is the one pruned"
    );

    engine.shutdown();
    clean_up(&workspace);
}
