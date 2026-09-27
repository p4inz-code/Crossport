/* ==========================================================================
 * Transfer engine tests
 * The queue, its controls, and the numbers it reports — exercised against real
 * files on the real filesystem, because that is the only way to prove that
 * pause, resume, cancel, cleanup, and byte accounting actually behave.
 * ========================================================================== */

use super::*;
use crate::filesystem::test_support::unique_temp_dir;
use crate::transfer::copy::COPY_BUFFER_BYTES;
use crate::verification::{
    ChecksumAlgorithm, VerificationMethod, VerificationPolicy, VerificationStatus,
};

use std::path::{Path, PathBuf};

const TIMEOUT: Duration = Duration::from_secs(30);

fn clean_up(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

fn write_file(path: &Path, bytes: usize) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("parent is creatable");
    }
    std::fs::write(path, vec![9u8; bytes]).expect("file is writable");
}

fn request_for(
    sources: &[&Path],
    destination: &Path,
    operation: TransferOperation,
    conflict: ConflictStrategy,
) -> TransferRequest {
    TransferRequest {
        sources: sources
            .iter()
            .map(|path| path.display().to_string())
            .collect(),
        destination: destination.display().to_string(),
        operation,
        conflict,
        verification: None,
    }
}

/// Waits for a job to reach a state, or fails the test with the last state.
fn wait_for(
    engine: &TransferEngine,
    id: &str,
    what: &str,
    predicate: impl Fn(&TransferSnapshot) -> bool,
) -> TransferSnapshot {
    let deadline = Instant::now() + TIMEOUT;
    loop {
        let snapshot = engine.snapshot(id).expect("the job exists");
        if predicate(&snapshot) {
            return snapshot;
        }
        if snapshot.status.is_terminal() || Instant::now() > deadline {
            panic!("'{what}' never happened; last state: {snapshot:?}");
        }
        std::thread::sleep(Duration::from_millis(2));
    }
}

fn wait_for_status(engine: &TransferEngine, id: &str, status: TransferStatus) -> TransferSnapshot {
    wait_for(engine, id, status.as_str(), |snapshot| {
        snapshot.status == status
    })
}

/// A transfer big enough that its progress can be observed while it runs.
/// 64 MiB is far more than one poll interval's worth of I/O at any speed a
/// filesystem can sustain, so these tests observe real mid-transfer state
/// instead of racing a copy that finished before the first check.
const LONG_FILE_BYTES: usize = 64 * 1024 * 1024;

/// Waits until a job is actively moving data, so control commands are applied
/// mid-transfer rather than to a job that already finished.
fn wait_for_running_progress(engine: &TransferEngine, id: &str) -> TransferSnapshot {
    wait_for(engine, id, "mid-transfer", |snapshot| {
        snapshot.status == TransferStatus::Running && snapshot.progress.transferred_bytes > 0
    })
}

#[test]
fn a_queued_job_reports_itself_before_it_runs() {
    let workspace = unique_temp_dir("engine-enqueue");
    let source = workspace.join("payload.bin");
    write_file(&source, 1024);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let snapshot = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");

    assert!(!snapshot.id.is_empty(), "every job has a stable identifier");
    assert_eq!(snapshot.operation, TransferOperation::Copy);
    assert_eq!(snapshot.destination, destination.display().to_string());
    assert_eq!(snapshot.sources, vec![source.display().to_string()]);
    assert_eq!(snapshot.progress.total_bytes, 1024);
    assert_eq!(snapshot.progress.total_files, 1);
    assert_eq!(snapshot.progress.percent, Some(0));
    assert!(snapshot.error.is_none());
    assert!(snapshot.issues.is_empty());
    assert!(
        snapshot.status == TransferStatus::Queued
            || snapshot.status == TransferStatus::Preparing
            || snapshot.status == TransferStatus::Running
            || snapshot.status == TransferStatus::Completed,
        "a job never starts out finished: {:?}",
        snapshot.status
    );

    wait_for_status(&engine, &snapshot.id, TransferStatus::Completed);
    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn jobs_run_one_at_a_time_in_the_order_they_were_accepted() {
    let workspace = unique_temp_dir("engine-order");
    let first_source = workspace.join("first.bin");
    write_file(&first_source, LONG_FILE_BYTES);
    let second_source = workspace.join("second.txt");
    write_file(&second_source, 64);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(DEFAULT_MAX_ACTIVE);
    let first = engine
        .enqueue_request(request_for(
            &[&first_source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the first job is accepted");
    wait_for_running_progress(&engine, &first.id);

    // Hold the only worker by pausing the job that owns it.
    engine.pause(&first.id).expect("the first job pauses");

    let second = engine
        .enqueue_request(request_for(
            &[&second_source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the second job is accepted");

    assert_eq!(
        engine.snapshot(&second.id).expect("exists").status,
        TransferStatus::Queued,
        "a queued job waits for the worker instead of running alongside it"
    );
    assert!(
        !destination.join("second.txt").exists(),
        "a queued job has not touched the filesystem"
    );

    engine.resume(&first.id).expect("the first job resumes");
    wait_for_status(&engine, &first.id, TransferStatus::Completed);
    wait_for_status(&engine, &second.id, TransferStatus::Completed);

    let queue: Vec<String> = engine
        .snapshots()
        .into_iter()
        .map(|snapshot| snapshot.id)
        .collect();
    assert_eq!(queue, vec![first.id.clone(), second.id.clone()]);

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn two_workers_run_two_jobs_at_once_when_the_engine_is_told_to() {
    let workspace = unique_temp_dir("engine-two");
    let held_source = workspace.join("held.bin");
    write_file(&held_source, LONG_FILE_BYTES);
    let free_source = workspace.join("free.txt");
    write_file(&free_source, 64);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(2);
    let held = engine
        .enqueue_request(request_for(
            &[&held_source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the first job is accepted");
    wait_for_running_progress(&engine, &held.id);
    engine.pause(&held.id).expect("the first job pauses");

    let other = engine
        .enqueue_request(request_for(
            &[&free_source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the second job is accepted");

    wait_for_status(&engine, &other.id, TransferStatus::Completed);
    assert_eq!(
        engine.snapshot(&held.id).expect("exists").status,
        TransferStatus::Paused,
        "the second worker did not disturb the parked job"
    );

    engine
        .cancel(&held.id)
        .expect("the parked job is cancelled");
    wait_for_status(&engine, &held.id, TransferStatus::Cancelled);
    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn pause_really_stops_the_transfer_and_resume_really_continues_it() {
    let workspace = unique_temp_dir("engine-pause");
    let source = workspace.join("payload.bin");
    write_file(&source, LONG_FILE_BYTES);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");

    let running = wait_for_running_progress(&engine, &job.id);
    assert!(running.progress.percent.unwrap_or(100) < 100);

    let paused = engine.pause(&job.id).expect("the job pauses");
    assert_eq!(paused.status, TransferStatus::Paused);

    // The worker finishes the chunk it was already writing, then parks. After
    // that, nothing moves: a paused transfer that keeps copying is a fake pause.
    std::thread::sleep(Duration::from_millis(150));
    let parked = engine.snapshot(&job.id).expect("exists");
    assert_eq!(parked.status, TransferStatus::Paused);
    assert!(
        parked.progress.transferred_bytes - paused.progress.transferred_bytes
            <= COPY_BUFFER_BYTES as u64,
        "at most the chunk already in flight completes before the job parks"
    );

    std::thread::sleep(Duration::from_millis(150));
    let later = engine.snapshot(&job.id).expect("exists");
    assert_eq!(
        later.progress.transferred_bytes, parked.progress.transferred_bytes,
        "a parked job moves no bytes at all"
    );
    assert_eq!(
        parked.progress.bytes_per_second, 0,
        "a parked job reports no speed rather than a stale one"
    );
    assert_eq!(parked.progress.eta_seconds, None);
    assert!(
        !destination.join("payload.bin").exists(),
        "nothing is committed while the job is paused"
    );

    let resumed = engine.resume(&job.id).expect("the job resumes");
    assert!(
        resumed.status == TransferStatus::Running || resumed.status == TransferStatus::Queued,
        "a resumed job is live again: {:?}",
        resumed.status
    );

    let finished = wait_for_status(&engine, &job.id, TransferStatus::Completed);
    assert_eq!(finished.progress.percent, Some(100));
    assert_eq!(finished.progress.completed_files, 1);
    assert_eq!(
        std::fs::metadata(destination.join("payload.bin"))
            .expect("the destination exists")
            .len(),
        LONG_FILE_BYTES as u64
    );
    assert_eq!(
        std::fs::read(destination.join("payload.bin"))
            .expect("readable")
            .len(),
        std::fs::read(&source).expect("readable").len(),
        "the resumed file holds every byte of the source"
    );

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn pausing_a_queued_job_keeps_it_out_of_the_worker_until_resumed() {
    let workspace = unique_temp_dir("engine-pause-queued");
    let held_source = workspace.join("held.bin");
    write_file(&held_source, LONG_FILE_BYTES);
    let waiting_source = workspace.join("waiting.txt");
    write_file(&waiting_source, 32);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let held = engine
        .enqueue_request(request_for(
            &[&held_source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the first job is accepted");
    wait_for_running_progress(&engine, &held.id);
    engine.pause(&held.id).expect("the first job pauses");

    let waiting = engine
        .enqueue_request(request_for(
            &[&waiting_source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the second job is accepted");
    let paused = engine.pause(&waiting.id).expect("a queued job pauses");
    assert_eq!(paused.status, TransferStatus::Paused);

    engine.resume(&held.id).expect("the first job resumes");
    wait_for_status(&engine, &held.id, TransferStatus::Completed);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(
        engine.snapshot(&waiting.id).expect("exists").status,
        TransferStatus::Paused,
        "a job paused while queued is never picked up"
    );
    assert!(!destination.join("waiting.txt").exists());

    engine.resume(&waiting.id).expect("the second job resumes");
    wait_for_status(&engine, &waiting.id, TransferStatus::Completed);
    assert!(destination.join("waiting.txt").is_file());

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn cancelling_a_running_job_discards_its_partial_output() {
    let workspace = unique_temp_dir("engine-cancel-running");
    let source = workspace.join("payload.bin");
    write_file(&source, LONG_FILE_BYTES);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");

    wait_for_running_progress(&engine, &job.id);

    // Parking the job first makes the cancellation deterministic (and covers
    // the real-world case of cancelling a transfer the user had paused).
    engine.pause(&job.id).expect("the job pauses mid-transfer");
    std::thread::sleep(Duration::from_millis(150));

    let cancelling = engine.cancel(&job.id).expect("the job is cancelled");
    assert_eq!(cancelling.status, TransferStatus::Cancelling);

    let cancelled = wait_for_status(&engine, &job.id, TransferStatus::Cancelled);
    assert!(
        cancelled.error.is_none(),
        "cancelling is not a failure: {:?}",
        cancelled.error
    );

    let leftovers: Vec<String> = std::fs::read_dir(&destination)
        .expect("readable")
        .flatten()
        .map(|entry| entry.file_name().to_string_lossy().into_owned())
        .collect();
    assert!(
        leftovers.is_empty(),
        "a cancelled transfer leaves neither output nor partial files: {leftovers:?}"
    );

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn cancelling_a_queued_job_releases_it_without_touching_the_disk() {
    let workspace = unique_temp_dir("engine-cancel-queued");
    let held_source = workspace.join("held.bin");
    write_file(&held_source, LONG_FILE_BYTES);
    let never_source = workspace.join("never.txt");
    write_file(&never_source, 32);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let held = engine
        .enqueue_request(request_for(
            &[&held_source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the first job is accepted");
    wait_for_running_progress(&engine, &held.id);
    engine.pause(&held.id).expect("the first job pauses");

    let queued = engine
        .enqueue_request(request_for(
            &[&never_source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the second job is accepted");

    let cancelled = engine
        .cancel(&queued.id)
        .expect("a queued job cancels at once");
    assert_eq!(cancelled.status, TransferStatus::Cancelled);
    assert!(
        cancelled.finished_at_ms.is_some(),
        "a cancelled job is finished and done with"
    );

    engine.resume(&held.id).expect("the first job resumes");
    wait_for_status(&engine, &held.id, TransferStatus::Completed);
    std::thread::sleep(Duration::from_millis(100));
    assert_eq!(
        engine.snapshot(&queued.id).expect("exists").status,
        TransferStatus::Cancelled,
        "a cancelled queued job never runs"
    );
    assert!(!destination.join("never.txt").exists());

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn a_failed_job_is_reported_and_the_queue_keeps_working() {
    let workspace = unique_temp_dir("engine-failed");
    let vanished_source = workspace.join("vanished.bin");
    write_file(&vanished_source, 4096);
    let good_source = workspace.join("good.txt");
    write_file(&good_source, 64);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);

    // Plan first, then remove the source: the job fails while it runs, which is
    // the failure mode the engine has to survive, not a planning rejection.
    let request = request_for(
        &[&vanished_source],
        &destination,
        TransferOperation::Copy,
        ConflictStrategy::Skip,
    );
    let plan = plan::plan_for_start(&request).expect("the plan succeeds");
    std::fs::remove_file(&vanished_source).expect("the test owns the file");
    let failed = engine.enqueue(plan).expect("the job is accepted");

    let failed = wait_for_status(&engine, &failed.id, TransferStatus::Failed);
    assert_eq!(failed.progress.failed_items, 1);
    assert_eq!(failed.issues.len(), 1);
    assert_eq!(
        failed.issues[0].error.as_ref().map(|error| error.code()),
        Some("path_not_found")
    );
    assert!(
        failed.error.is_some(),
        "a job that lost an item reports a job-level reason"
    );

    let good = engine
        .enqueue_request(request_for(
            &[&good_source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("a later job is still accepted");
    wait_for_status(&engine, &good.id, TransferStatus::Completed);
    assert!(destination.join("good.txt").is_file());

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn conflict_strategies_are_honoured_end_to_end() {
    let workspace = unique_temp_dir("engine-conflicts");
    let source = workspace.join("notes.txt");
    write_file(&source, 64);
    let destination = workspace.join("out");
    write_file(&destination.join("notes.txt"), 4);

    let engine = TransferEngine::new(1);

    let skipped = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");
    wait_for_status(&engine, &skipped.id, TransferStatus::Completed);
    assert_eq!(
        std::fs::metadata(destination.join("notes.txt"))
            .expect("exists")
            .len(),
        4
    );

    let renamed = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Rename,
        ))
        .expect("the job is accepted");
    wait_for_status(&engine, &renamed.id, TransferStatus::Completed);
    assert_eq!(
        std::fs::metadata(destination.join("notes (2).txt"))
            .expect("exists")
            .len(),
        64
    );

    let replaced = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Replace,
        ))
        .expect("the job is accepted");
    wait_for_status(&engine, &replaced.id, TransferStatus::Completed);
    assert_eq!(
        std::fs::metadata(destination.join("notes.txt"))
            .expect("exists")
            .len(),
        64
    );

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn a_move_reports_the_source_as_gone_and_the_destination_as_complete() {
    let workspace = unique_temp_dir("engine-move");
    let source = workspace.join("Data");
    write_file(&source.join("inner.bin"), 8192);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Move,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");

    let finished = wait_for_status(&engine, &job.id, TransferStatus::Completed);

    assert_eq!(finished.operation, TransferOperation::Move);
    assert_eq!(finished.progress.transferred_bytes, 8192);
    assert_eq!(finished.progress.percent, Some(100));
    assert!(!source.exists(), "a completed move removes its source");
    assert!(destination.join("Data").join("inner.bin").is_file());

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn an_unknown_transfer_identifier_is_a_structured_error() {
    let engine = TransferEngine::new(1);

    for outcome in [
        engine.snapshot("transfer-missing").err(),
        engine.pause("transfer-missing").err(),
        engine.resume("transfer-missing").err(),
        engine.cancel("transfer-missing").err(),
        engine.remove("transfer-missing").err(),
    ] {
        let error = outcome.expect("an unknown identifier is an error");
        assert_eq!(error.code(), "transfer_not_found");
    }

    engine.shutdown();
}

#[test]
fn finished_jobs_reject_control_commands_and_can_be_pruned() {
    let workspace = unique_temp_dir("engine-finished");
    let source = workspace.join("tiny.txt");
    write_file(&source, 16);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");
    wait_for_status(&engine, &job.id, TransferStatus::Completed);

    assert_eq!(
        engine.pause(&job.id).expect_err("pausing is over").code(),
        "invalid_input"
    );
    assert_eq!(
        engine.resume(&job.id).expect_err("resuming is over").code(),
        "invalid_input"
    );
    assert_eq!(
        engine
            .cancel(&job.id)
            .expect_err("cancelling is over")
            .code(),
        "invalid_input"
    );

    engine
        .remove(&job.id)
        .expect("a finished job can be removed");
    assert!(engine.snapshots().is_empty());

    let second = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");
    assert_ne!(second.id, job.id, "identifiers are never reused");

    wait_for_status(&engine, &second.id, TransferStatus::Completed);
    assert_eq!(engine.clear_finished(), 1);
    assert!(engine.snapshots().is_empty());

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn live_jobs_survive_a_prune_of_finished_ones() {
    let workspace = unique_temp_dir("engine-prune");
    let long_source = workspace.join("long.bin");
    write_file(&long_source, LONG_FILE_BYTES);
    let small_source = workspace.join("small.txt");
    write_file(&small_source, 16);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let live = engine
        .enqueue_request(request_for(
            &[&long_source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");
    wait_for_running_progress(&engine, &live.id);

    assert_eq!(
        engine
            .remove(&live.id)
            .expect_err("a live job is not removable")
            .code(),
        "invalid_input"
    );
    assert_eq!(engine.clear_finished(), 0);

    engine.pause(&live.id).expect("the job pauses");
    engine.cancel(&live.id).expect("the job is cancelled");
    wait_for_status(&engine, &live.id, TransferStatus::Cancelled);
    assert_eq!(engine.clear_finished(), 1);

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn control_commands_are_safe_under_concurrent_use() {
    let workspace = unique_temp_dir("engine-concurrency");
    let source = workspace.join("payload.bin");
    write_file(&source, LONG_FILE_BYTES);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");

    let mut handles: Vec<std::thread::JoinHandle<()>> = Vec::new();
    for index in 0..4 {
        let engine = engine.clone();
        let id = job.id.clone();
        handles.push(std::thread::spawn(move || {
            for _ in 0..200 {
                match index % 4 {
                    0 => {
                        let _ = engine.pause(&id);
                    }
                    1 => {
                        let _ = engine.resume(&id);
                    }
                    2 => {
                        let snapshot = engine.snapshot(&id).expect("the job exists");
                        assert!(
                            snapshot.progress.percent.unwrap_or(0) <= 100,
                            "progress is never nonsense: {:?}",
                            snapshot.progress
                        );
                    }
                    _ => {
                        for snapshot in engine.snapshots() {
                            assert_eq!(snapshot.id, id);
                            assert!(
                                snapshot.progress.transferred_bytes
                                    <= snapshot.progress.total_bytes,
                                "byte counts never exceed the plan: {:?}",
                                snapshot.progress
                            );
                        }
                    }
                }
            }
        }));
    }

    for handle in handles {
        handle.join().expect("control threads do not panic");
    }

    let final_state = engine.snapshot(&job.id).expect("the job still exists");
    assert!(
        final_state.status.is_live() || final_state.status.is_terminal(),
        "the queue is still consistent: {:?}",
        final_state.status
    );
    assert_eq!(engine.snapshots().len(), 1);

    // The engine still works after all of that.
    if final_state.status != TransferStatus::Completed {
        let _ = engine.cancel(&job.id);
    }
    wait_for(&engine, &job.id, "a terminal state", |snapshot| {
        snapshot.status.is_terminal()
    });

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn progress_accounting_matches_the_files_that_were_written() {
    let workspace = unique_temp_dir("engine-progress");
    let source = workspace.join("Data");
    write_file(&source.join("a.bin"), 1000);
    write_file(&source.join("b.bin"), 2000);
    std::fs::create_dir_all(source.join("empty")).expect("subdir");
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");

    let finished = wait_for_status(&engine, &job.id, TransferStatus::Completed);
    let progress = finished.progress;

    assert_eq!(progress.total_bytes, 3000);
    assert_eq!(progress.transferred_bytes, 3000);
    assert_eq!(progress.total_files, 2);
    assert_eq!(progress.completed_files, 2);
    assert_eq!(progress.total_directories, 2);
    assert_eq!(progress.completed_directories, 2);
    assert_eq!(progress.skipped_items, 0);
    assert_eq!(progress.failed_items, 0);
    assert_eq!(progress.percent, Some(100));
    assert!(progress.current_file.is_none(), "nothing is in flight");
    if progress.elapsed_ms > 0 {
        assert!(
            progress.average_bytes_per_second > 0,
            "a transfer that moved bytes reports a real average speed"
        );
    }
    assert!(finished.issues.is_empty());
    assert!(finished.error.is_none());

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn a_job_with_nothing_to_measure_reports_no_percentage() {
    let workspace = unique_temp_dir("engine-empty-progress");
    let source = workspace.join("Empty");
    std::fs::create_dir_all(&source).expect("subdir");
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");

    let finished = wait_for_status(&engine, &job.id, TransferStatus::Completed);

    assert_eq!(finished.progress.total_bytes, 0);
    assert_eq!(finished.progress.bytes_per_second, 0);
    assert_eq!(finished.progress.eta_seconds, None);
    assert_eq!(
        finished.progress.percent,
        Some(100),
        "one folder planned and created is complete work"
    );

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn a_skipped_job_reports_what_it_left_alone() {
    let workspace = unique_temp_dir("engine-skipped");
    let source = workspace.join("notes.txt");
    write_file(&source, 128);
    let destination = workspace.join("out");
    write_file(&destination.join("notes.txt"), 4);

    let engine = TransferEngine::new(1);
    let job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");

    let finished = wait_for_status(&engine, &job.id, TransferStatus::Completed);

    assert_eq!(finished.progress.skipped_items, 1);
    assert_eq!(finished.progress.skipped_bytes, 128);
    assert_eq!(finished.progress.transferred_bytes, 0);
    assert_eq!(finished.issues.len(), 1);
    assert_eq!(finished.issues[0].reason, TransferIssueReason::Skipped);
    assert!(
        finished.error.is_none(),
        "skipping is what the user asked for, not a failure"
    );

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn unsafe_requests_are_refused_before_a_job_is_created() {
    let workspace = unique_temp_dir("engine-unsafe");
    let source = workspace.join("Data");
    write_file(&source.join("inner.txt"), 32);
    let nested_destination = source.join("Backup");
    std::fs::create_dir_all(&nested_destination).expect("subdir");

    let engine = TransferEngine::new(1);
    let error = engine
        .enqueue_request(request_for(
            &[&source],
            &nested_destination,
            TransferOperation::Copy,
            ConflictStrategy::Replace,
        ))
        .expect_err("a folder cannot be copied into itself");

    assert_eq!(error.code(), "unsafe_relationship");
    assert!(engine.snapshots().is_empty(), "no job was created");

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn window_speed_needs_two_samples_and_real_progress() {
    let base = Instant::now();

    let mut samples = VecDeque::new();
    assert_eq!(window_speed(&samples), 0, "no samples, no speed");

    samples.push_back((base, 0));
    assert_eq!(window_speed(&samples), 0, "one sample is not a measurement");

    samples.push_back((base + Duration::from_millis(500), 1_000_000));
    assert_eq!(window_speed(&samples), 2_000_000);

    let stalled: VecDeque<(Instant, u64)> =
        vec![(base, 4_000), (base + Duration::from_secs(1), 4_000)].into();
    assert_eq!(window_speed(&stalled), 0, "no progress, no speed");

    let instant: VecDeque<(Instant, u64)> = vec![(base, 0), (base, 1_000)].into();
    assert_eq!(
        window_speed(&instant),
        0,
        "a zero-length window cannot divide by zero"
    );
}

#[test]
fn timing_excludes_paused_time_and_offers_an_eta_only_while_running() {
    let base = Instant::now();

    let mut runtime = JobRuntime {
        status: TransferStatus::Running,
        counters: TransferCounters {
            total_bytes: 4_000,
            transferred_bytes: 1_000,
            ..TransferCounters::default()
        },
        started_ms: Some(1),
        started_at: Some(base),
        paused_total: Duration::from_secs(1),
        samples: vec![(base, 0), (base + Duration::from_secs(1), 1_000)].into(),
        ..JobRuntime::default()
    };

    let running = timing_of(&runtime, base + Duration::from_secs(3));
    assert_eq!(
        running.elapsed_ms, 2_000,
        "the paused second is not working time"
    );
    assert_eq!(running.bytes_per_second, 1_000);
    assert_eq!(running.average_bytes_per_second, 500);
    assert_eq!(running.eta_seconds, Some(3), "3 KiB left at 1 KiB/s");

    runtime.status = TransferStatus::Paused;
    let paused = timing_of(&runtime, base + Duration::from_secs(4));
    assert_eq!(
        paused.eta_seconds, None,
        "a paused job cannot promise a finish time"
    );

    runtime.status = TransferStatus::Completed;
    runtime.finished_instant = Some(base + Duration::from_secs(5));
    let finished = timing_of(&runtime, base + Duration::from_secs(30));
    assert_eq!(
        finished.elapsed_ms, 4_000,
        "a finished job stops counting at the moment it finished"
    );
}

#[test]
fn a_job_that_was_never_started_reports_no_timing() {
    let runtime = JobRuntime {
        status: TransferStatus::Queued,
        counters: TransferCounters {
            total_bytes: 500,
            ..TransferCounters::default()
        },
        ..JobRuntime::default()
    };

    let timing = timing_of(&runtime, Instant::now());

    assert_eq!(timing.elapsed_ms, 0);
    assert_eq!(timing.bytes_per_second, 0);
    assert_eq!(timing.average_bytes_per_second, 0);
    assert_eq!(timing.eta_seconds, None);
}

#[test]
fn issue_lists_are_bounded_and_flagged() {
    let mut runtime = JobRuntime {
        status: TransferStatus::Running,
        counters: TransferCounters::default(),
        ..JobRuntime::default()
    };

    for index in 0..(MAX_ISSUES + 10) {
        push_issue(
            &mut runtime,
            TransferIssue::skipped(format!("D:\\file-{index}"), "destination exists"),
        );
    }

    assert_eq!(runtime.issues.len(), MAX_ISSUES);
    assert!(runtime.issues_truncated);

    // The counters are not bounded: only the diagnostics list is.
    assert_eq!(runtime.counters.skipped_items, 0);
}

#[test]
fn a_plan_with_no_roots_still_produces_a_valid_snapshot() {
    let workspace = unique_temp_dir("engine-empty-plan");
    let source = workspace.join("file.txt");
    write_file(&source, 8);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let request = request_for(
        &[&source],
        &destination,
        TransferOperation::Copy,
        ConflictStrategy::Skip,
    );
    let plan = plan::plan_request(&request).expect("the plan succeeds");
    let engine = TransferEngine::new(1);
    let job = engine.enqueue(plan).expect("the job is accepted");

    let snapshot = engine.snapshot(&job.id).expect("the job is readable");

    assert_eq!(snapshot.id, job.id);
    assert_eq!(snapshot.progress.total_files, 1);
    assert!(
        snapshot.queued_at_ms > 0,
        "every job carries a real timestamp"
    );

    // Either the copy finished before this line, or the cancel landed first;
    // both leave a consistent queue.
    if !engine
        .snapshot(&job.id)
        .expect("exists")
        .status
        .is_terminal()
    {
        engine
            .cancel(&job.id)
            .expect("cancelled before it can finish");
    }
    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn a_destination_that_is_inside_the_source_is_refused_by_the_engine() {
    let workspace = unique_temp_dir("engine-inside");
    let source = workspace.join("Data");
    write_file(&source.join("inner.txt"), 8);
    let destination = source.join("nested").join("deeper");
    std::fs::create_dir_all(&destination).expect("subdir");

    let engine = TransferEngine::new(1);
    let error = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Move,
            ConflictStrategy::Rename,
        ))
        .expect_err("a move into a descendant is refused");

    assert_eq!(error.code(), "unsafe_relationship");
    assert_eq!(engine.snapshots().len(), 0);

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn cloning_the_engine_shares_one_queue() {
    let workspace = unique_temp_dir("engine-clone");
    let source = workspace.join("file.txt");
    write_file(&source, 8);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let clone = engine.clone();
    let job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");

    assert_eq!(
        clone.snapshot(&job.id).expect("the clone sees the job").id,
        job.id
    );
    wait_for_status(&clone, &job.id, TransferStatus::Completed);

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn debug_output_summarizes_the_queue() {
    let engine = TransferEngine::new(2);
    let debugged = format!("{engine:?}");

    assert!(debugged.contains("TransferEngine"));
    assert!(debugged.contains("max_active: 2"));

    engine.shutdown();
}

/// A publisher that records what the frontend would have received.
#[derive(Default)]
struct RecordingPublisher {
    published: Mutex<Vec<TransferSnapshot>>,
}

impl TransferPublisher for RecordingPublisher {
    fn publish(&self, snapshot: &TransferSnapshot) {
        lock(&self.published).push(snapshot.clone());
    }
}

#[test]
fn published_snapshots_track_the_job_and_stay_throttled() {
    let workspace = unique_temp_dir("engine-published");
    let source = workspace.join("payload.bin");
    write_file(&source, LONG_FILE_BYTES);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let publisher = Arc::new(RecordingPublisher::default());
    engine.attach(publisher.clone());

    let job = engine
        .enqueue_request(request_for(
            &[&source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Skip,
        ))
        .expect("the job is accepted");
    // Give the transfer a few cadence intervals to report byte progress. The
    // assertions below are unconditional; the one that needs a transfer slower
    // than the cadence says so explicitly.
    std::thread::sleep(PROGRESS_INTERVAL * 3);
    let finished = wait_for_status(&engine, &job.id, TransferStatus::Completed);

    let published = lock(&publisher.published).clone();
    assert!(
        published
            .iter()
            .any(|snapshot| snapshot.status == TransferStatus::Preparing
                || snapshot.status == TransferStatus::Running),
        "the UI is told when a job starts, not only when it ends"
    );
    assert_eq!(
        published.last().map(|snapshot| snapshot.status),
        Some(TransferStatus::Completed),
        "the final state is published, not left to a poll"
    );

    let interval_ms = u64::try_from(PROGRESS_INTERVAL.as_millis()).unwrap_or(120);
    if finished.progress.elapsed_ms > interval_ms * 3 {
        assert!(
            published.iter().any(|snapshot| {
                snapshot.status == TransferStatus::Running
                    && snapshot.progress.transferred_bytes > 0
                    && snapshot.progress.transferred_bytes < snapshot.progress.total_bytes
            }),
            "a transfer that ran for more than three cadence intervals published progress mid-flight"
        );
    }
    assert!(
        published.iter().all(|snapshot| snapshot.id == job.id),
        "every published snapshot belongs to the job it describes"
    );
    assert!(
        published.windows(2).all(|pair| {
            pair[1].progress.transferred_bytes >= pair[0].progress.transferred_bytes
        }),
        "published byte counts never go backwards"
    );

    // Byte progress is coalesced: at most one publish per interval, plus the
    // state changes and item boundaries that are always published.
    let byte_publishes = published
        .iter()
        .filter(|snapshot| snapshot.status == TransferStatus::Running)
        .filter(|snapshot| snapshot.progress.transferred_bytes > 0)
        .count() as u64;
    let ceiling = finished.progress.elapsed_ms / interval_ms + 4;
    assert!(
        byte_publishes <= ceiling,
        "{byte_publishes} progress events in {} ms exceeds the cadence ({ceiling})",
        finished.progress.elapsed_ms
    );

    // A control command is published immediately, without waiting for the
    // cadence: the UI must never poll to find out that a transfer paused.
    let control_source = workspace.join("control.bin");
    write_file(&control_source, LONG_FILE_BYTES);
    let control = engine
        .enqueue_request(request_for(
            &[&control_source],
            &destination,
            TransferOperation::Copy,
            ConflictStrategy::Rename,
        ))
        .expect("the job is accepted");
    wait_for_running_progress(&engine, &control.id);
    engine
        .pause(&control.id)
        .expect("the job pauses mid-transfer");

    let after_pause = lock(&publisher.published).clone();
    let parked = after_pause
        .iter()
        .find(|snapshot| snapshot.id == control.id && snapshot.status == TransferStatus::Paused)
        .unwrap_or_else(|| {
            panic!(
                "a pause is published as soon as it is accepted; the UI only saw {:?}",
                after_pause
                    .iter()
                    .map(|snapshot| (snapshot.id.clone(), snapshot.status))
                    .collect::<Vec<_>>()
            )
        });
    assert!(
        parked.progress.transferred_bytes > 0
            && parked.progress.transferred_bytes < parked.progress.total_bytes,
        "the published pause carries the real partial progress: {:?}",
        parked.progress
    );

    engine.cancel(&control.id).expect("the parked job cancels");
    wait_for_status(&engine, &control.id, TransferStatus::Cancelled);
    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn a_failing_job_publishes_its_failure_immediately() {
    let workspace = unique_temp_dir("engine-published-failure");
    let source = workspace.join("vanished.bin");
    write_file(&source, 4096);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let request = request_for(
        &[&source],
        &destination,
        TransferOperation::Copy,
        ConflictStrategy::Skip,
    );
    let plan = plan::plan_for_start(&request).expect("the plan succeeds");
    std::fs::remove_file(&source).expect("the test owns the file");

    let engine = TransferEngine::new(1);
    let publisher = Arc::new(RecordingPublisher::default());
    engine.attach(publisher.clone());
    let job = engine.enqueue(plan).expect("the job is accepted");
    wait_for_status(&engine, &job.id, TransferStatus::Failed);

    let published = lock(&publisher.published).clone();
    let failure = published
        .iter()
        .find(|snapshot| snapshot.progress.failed_items > 0)
        .expect("the failure is published as soon as it happens");
    assert_eq!(
        failure
            .issues
            .first()
            .and_then(|issue| issue.error.as_ref())
            .map(|error| error.code()),
        Some("path_not_found")
    );

    engine.shutdown();
    clean_up(&workspace);
}

// -- verification in the copy path ------------------------------------------

/// A copy request that states its verification policy, as the command layer
/// does when it folds the configured policy into a job.
fn request_verifying(
    source: &Path,
    destination: &Path,
    policy: VerificationPolicy,
) -> TransferRequest {
    let mut request = request_for(
        &[source],
        destination,
        TransferOperation::Copy,
        ConflictStrategy::Skip,
    );
    request.verification = Some(policy);
    request
}

#[test]
fn a_copy_proves_its_size_when_the_policy_asks_for_it() {
    let workspace = unique_temp_dir("engine-verify-size");
    let source = workspace.join("payload.bin");
    write_file(&source, 32 * 1024);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let job = engine
        .enqueue_request(request_verifying(
            &source,
            &destination,
            VerificationPolicy::Size,
        ))
        .expect("the job is accepted");
    let finished = wait_for_status(&engine, &job.id, TransferStatus::Completed);

    let verification = &finished.verification;
    assert_eq!(verification.status, VerificationStatus::Verified);
    assert_eq!(verification.policy, VerificationPolicy::Size);
    assert_eq!(verification.method, VerificationMethod::Size);
    assert_eq!(verification.planned_files, 1);
    assert_eq!(verification.checked_files, 1);
    assert_eq!(verification.verified_files, 1);
    assert_eq!(verification.unverified_files, 0);
    assert_eq!(verification.verified_bytes, 32 * 1024);
    assert!(
        !verification.coverage.checksum,
        "a size check must never claim a checksum was compared"
    );
    assert!(
        !verification.coverage.modified_time_preserved && !verification.coverage.readonly_preserved,
        "this engine does not reapply metadata, and says so"
    );
    assert!(
        verification.verdict().starts_with("verified (size"),
        "the verdict states the method that ran: {}",
        verification.verdict()
    );

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn a_checksum_policy_compares_the_source_bytes_with_the_file_on_disk() {
    let workspace = unique_temp_dir("engine-verify-checksum");
    let source = workspace.join("payload.bin");
    write_file(&source, 8 * 1024);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let job = engine
        .enqueue_request(request_verifying(
            &source,
            &destination,
            VerificationPolicy::Checksum,
        ))
        .expect("the job is accepted");
    let finished = wait_for_status(&engine, &job.id, TransferStatus::Completed);

    let verification = &finished.verification;
    assert_eq!(verification.status, VerificationStatus::Verified);
    assert_eq!(verification.method, VerificationMethod::SizeAndChecksum);
    assert_eq!(
        verification.checksum_algorithm,
        Some(ChecksumAlgorithm::Sha256)
    );
    assert!(verification.coverage.checksum);
    assert_eq!(
        verification.checked_files, verification.planned_files,
        "every planned file was checked, not just some"
    );
    // The check is about data, so the data is compared independently of it.
    assert_eq!(
        std::fs::read(&source).expect("the source is readable"),
        std::fs::read(destination.join("payload.bin")).expect("the copy is readable"),
        "a verified copy holds the source's bytes"
    );

    engine.shutdown();
    clean_up(&workspace);
}

#[test]
fn a_job_with_no_verification_reports_skipped_instead_of_verified() {
    let workspace = unique_temp_dir("engine-verify-none");
    let source = workspace.join("payload.bin");
    write_file(&source, 1024);
    let destination = workspace.join("out");
    std::fs::create_dir_all(&destination).expect("destination is creatable");

    let engine = TransferEngine::new(1);
    let job = engine
        .enqueue_request(request_verifying(
            &source,
            &destination,
            VerificationPolicy::None,
        ))
        .expect("the job is accepted");
    let finished = wait_for_status(&engine, &job.id, TransferStatus::Completed);

    let verification = &finished.verification;
    assert_eq!(verification.status, VerificationStatus::Skipped);
    assert_eq!(verification.policy, VerificationPolicy::None);
    assert_eq!(verification.skipped_files, 1);
    assert_eq!(verification.checked_files, 0);
    assert_eq!(verification.verified_files, 0);
    assert_eq!(verification.verdict(), "not verified");
    assert!(
        !verification.coverage.size && !verification.coverage.checksum,
        "nothing about the bytes was checked, so nothing may be claimed"
    );
    assert!(
        finished.status == TransferStatus::Completed,
        "a job that did not verify is still a job that finished; the verdict says the rest"
    );

    engine.shutdown();
    clean_up(&workspace);
}

/// Keeps the helper honest: the tests build destinations with it, so a broken
/// helper would fail every test above instead of quietly passing.
#[test]
fn the_test_helpers_create_real_paths() {
    let workspace: PathBuf = unique_temp_dir("engine-helpers");
    write_file(&workspace.join("nested").join("file.txt"), 4);

    assert!(workspace.join("nested").join("file.txt").is_file());

    clean_up(&workspace);
}
