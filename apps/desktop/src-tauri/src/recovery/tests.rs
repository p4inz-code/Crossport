/* ==========================================================================
 * Recovery tests
 * Three things have to hold for recovery to be trustworthy, and each is tested
 * against real files rather than in-memory stand-ins:
 *
 * - the in-flight state survives a restart, and a document that cannot be used
 *   never takes the application down with it;
 * - a partial artifact is identified by name exactly, so a cleanup can never
 *   remove something the user owns;
 * - classification never upgrades an unfinished job to finished without the
 *   archive proving it.
 * ========================================================================== */

use super::*;
use crate::filesystem::test_support::unique_temp_dir;
use crate::recovery::artifacts::is_artifact_of;
use crate::transfer::{ConflictStrategy, TransferOperation, TransferRequest, TransferStatus};
use std::path::{Path, PathBuf};

fn request(destination: &Path) -> TransferRequest {
    TransferRequest {
        sources: vec!["C:\\source\\report.txt".to_string()],
        destination: destination.display().to_string(),
        operation: TransferOperation::Copy,
        conflict: ConflictStrategy::Skip,
        verification: Some(VerificationPolicy::Size),
    }
}

fn job(id: &str, destination: &Path) -> InterruptedTransfer {
    InterruptedTransfer {
        id: id.to_string(),
        request: request(destination),
        status: TransferStatus::Running,
        queued_at_ms: 1_700_000_000_000,
        started_at_ms: Some(1_700_000_000_100),
        updated_at_ms: 1_700_000_000_900,
        progress: PersistedProgress {
            total_bytes: 4_000,
            transferred_bytes: 1_000,
            total_files: 4,
            completed_files: 1,
            total_directories: 1,
            completed_directories: 1,
            skipped_items: 0,
            failed_items: 0,
        },
        verification: None,
    }
}

/// A state store inside a fresh temporary directory.
fn store(label: &str) -> (PathBuf, TransferStateStore) {
    let dir = unique_temp_dir(label);
    let path = dir.join("config").join(STATE_FILE);
    let (store, status) = TransferStateStore::open(path);
    assert_eq!(status, LoadStatus::Missing, "a fresh store has no file");
    (dir, store)
}

fn cleanup(dir: &Path) {
    let _ = std::fs::remove_dir_all(dir);
}

// -- durable state ----------------------------------------------------------

#[test]
fn a_fresh_state_store_is_empty_and_writable() {
    let (dir, store) = store("recovery-empty");

    assert!(store.is_empty());
    assert!(store.writable());
    assert_eq!(store.len(), 0);
    assert_eq!(store.list(), Vec::new());
    assert!(store.get("transfer-1").is_none());

    cleanup(&dir);
}

#[test]
fn an_interrupted_job_survives_a_round_trip_on_disk() {
    let (dir, store) = store("recovery-round-trip");
    let destination = dir.join("destination");
    std::fs::create_dir_all(&destination).expect("destination");

    let interrupted = job("transfer-1", &destination);
    store
        .upsert(interrupted.clone())
        .expect("state is writable");

    // Re-open the store, which is what a restart does.
    let (reopened, status) = TransferStateStore::open(store.path().to_path_buf());
    assert_eq!(status, LoadStatus::Loaded);

    let restored = reopened.get("transfer-1").expect("the job is still there");
    assert_eq!(
        restored, interrupted,
        "every field must survive the restart"
    );
    assert_eq!(restored.progress.percent(), Some(25));
    assert!(restored.had_started());
    // The stored request is what a restart replays, so it must be intact.
    assert_eq!(restored.request.operation, TransferOperation::Copy);
    assert_eq!(restored.request.conflict, ConflictStrategy::Skip);
    assert_eq!(
        restored.request.verification_policy(),
        VerificationPolicy::Size
    );

    cleanup(&dir);
}

#[test]
fn upsert_replaces_the_same_job_rather_than_duplicating_it() {
    let (dir, store) = store("recovery-upsert");
    let destination = dir.join("destination");
    std::fs::create_dir_all(&destination).expect("destination");

    store
        .upsert(job("transfer-1", &destination))
        .expect("first");
    let mut advanced = job("transfer-1", &destination);
    advanced.progress.transferred_bytes = 3_500;
    advanced.status = TransferStatus::Running;
    store.upsert(advanced.clone()).expect("second");

    assert_eq!(store.len(), 1);
    assert_eq!(store.get("transfer-1"), Some(advanced));

    cleanup(&dir);
}

#[test]
fn forget_removes_only_the_named_job() {
    let (dir, store) = store("recovery-forget");
    let destination = dir.join("destination");
    std::fs::create_dir_all(&destination).expect("destination");

    store
        .upsert(job("transfer-1", &destination))
        .expect("first");
    store
        .upsert(job("transfer-2", &destination))
        .expect("second");

    assert!(store.forget("transfer-1").expect("forget is writable"));
    assert!(!store.forget("transfer-1").expect("forget is writable"));
    assert_eq!(store.len(), 1);
    assert!(store.get("transfer-2").is_some());

    cleanup(&dir);
}

#[test]
fn the_state_file_is_bounded_to_the_maximum() {
    let (dir, store) = store("recovery-bounded");
    let destination = dir.join("destination");
    std::fs::create_dir_all(&destination).expect("destination");

    for index in 0..(MAX_STATE_JOBS + 5) {
        store
            .upsert(job(&format!("transfer-{index}"), &destination))
            .expect("state is writable");
    }

    assert_eq!(store.len(), MAX_STATE_JOBS);
    assert!(
        store.get("transfer-0").is_none(),
        "the oldest entries are the ones dropped"
    );
    assert!(store
        .get(&format!("transfer-{}", MAX_STATE_JOBS + 4))
        .is_some());

    cleanup(&dir);
}

#[test]
fn a_queued_job_that_never_started_has_not_started() {
    let (dir, _store) = store("recovery-not-started");
    let destination = dir.join("destination");
    std::fs::create_dir_all(&destination).expect("destination");

    let mut queued = job("transfer-1", &destination);
    queued.started_at_ms = None;
    queued.status = TransferStatus::Queued;

    assert!(!queued.had_started());

    cleanup(&dir);
}

#[test]
fn percent_is_derived_from_bytes_and_falls_back_to_items() {
    let by_bytes = PersistedProgress {
        total_bytes: 1_000,
        transferred_bytes: 250,
        ..PersistedProgress::default()
    };
    assert_eq!(by_bytes.percent(), Some(25));

    let by_items = PersistedProgress {
        total_files: 3,
        completed_files: 1,
        total_directories: 1,
        completed_directories: 1,
        ..PersistedProgress::default()
    };
    assert_eq!(by_items.percent(), Some(50));

    let nothing_known = PersistedProgress::default();
    assert_eq!(nothing_known.percent(), None);

    // More transferred than the plan measured must clamp, not overflow the bar.
    let over = PersistedProgress {
        total_bytes: 100,
        transferred_bytes: 400,
        ..PersistedProgress::default()
    };
    assert_eq!(over.percent(), Some(100));
}

// -- persistence safety -----------------------------------------------------

#[test]
fn a_malformed_state_file_is_preserved_and_the_store_starts_empty() {
    let dir = unique_temp_dir("recovery-malformed");
    let path = dir.join("config").join(STATE_FILE);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("config dir");
    std::fs::write(&path, b"{ this is not json").expect("write");

    let (store, status) = TransferStateStore::open(path.clone());

    assert!(matches!(status, LoadStatus::Recovered { .. }));
    assert!(store.is_empty(), "an unusable file yields an empty state");
    assert!(store.writable(), "the store must be usable again");
    assert!(
        !path.exists(),
        "the original path is freed for the next write"
    );
    let preserved = std::fs::read_dir(path.parent().expect("parent"))
        .expect("config dir")
        .filter_map(Result::ok)
        .filter(|entry| entry.file_name().to_string_lossy().contains(".corrupt-"))
        .count();
    assert_eq!(
        preserved, 1,
        "the unusable document is kept for diagnostics"
    );

    cleanup(&dir);
}

#[test]
fn an_empty_state_file_is_recovered() {
    let dir = unique_temp_dir("recovery-empty-file");
    let path = dir.join("config").join(STATE_FILE);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("config dir");
    std::fs::write(&path, b"").expect("write");

    let (store, status) = TransferStateStore::open(path);

    assert!(matches!(status, LoadStatus::Recovered { .. }));
    assert!(store.is_empty());

    cleanup(&dir);
}

#[test]
fn a_newer_schema_is_left_untouched_and_read_only() {
    let dir = unique_temp_dir("recovery-newer-schema");
    let path = dir.join("config").join(STATE_FILE);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("config dir");
    std::fs::write(&path, br#"{"schemaVersion":99,"jobs":[{"id":"future"}]}"#).expect("write");

    let (store, status) = TransferStateStore::open(path.clone());

    assert!(matches!(status, LoadStatus::Unsupported { .. }));
    assert!(!store.writable(), "a newer document must never be replaced");
    assert!(store.is_empty(), "nothing may be guessed from it");
    assert!(path.exists(), "the file itself is untouched");

    cleanup(&dir);
}

#[test]
fn an_older_schema_is_migrated() {
    let dir = unique_temp_dir("recovery-older-schema");
    let path = dir.join("config").join(STATE_FILE);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("config dir");
    std::fs::write(
        &path,
        br#"{"jobs":[{"id":"transfer-9","request":{"sources":["C:\\a"],"destination":"C:\\b","operation":"copy","conflict":"skip","verification":"size"},"status":"running","queuedAtMs":1,"updatedAtMs":2,"progress":{}}]}"#,
    )
    .expect("write");

    let (store, status) = TransferStateStore::open(path);

    assert!(matches!(status, LoadStatus::Migrated { .. }));
    assert!(store.writable());
    let restored = store.get("transfer-9").expect("the job was migrated");
    assert_eq!(restored.queued_at_ms, 1);
    assert_eq!(restored.progress, PersistedProgress::default());

    cleanup(&dir);
}

#[test]
fn a_document_without_a_version_is_treated_as_the_oldest_schema() {
    let dir = unique_temp_dir("recovery-unversioned");
    let path = dir.join("config").join(STATE_FILE);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("config dir");
    std::fs::write(&path, br#"{"jobs":[]}"#).expect("write");

    let (store, status) = TransferStateStore::open(path);

    assert!(matches!(status, LoadStatus::Migrated { .. }));
    assert!(store.is_empty());

    cleanup(&dir);
}

#[test]
fn a_document_of_the_wrong_shape_is_recovered_rather_than_fatal() {
    let dir = unique_temp_dir("recovery-wrong-shape");
    let path = dir.join("config").join(STATE_FILE);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("config dir");
    std::fs::write(&path, br#"{"jobs":"not a list"}"#).expect("write");

    let (store, status) = TransferStateStore::open(path.clone());

    assert!(matches!(status, LoadStatus::Recovered { .. }));
    assert!(store.is_empty());
    assert!(
        !path.exists(),
        "the unusable document is moved aside so the next write can land"
    );

    cleanup(&dir);
}

#[test]
fn a_leftover_temporary_file_never_shadows_the_real_document() {
    let (dir, store) = store("recovery-leftover-temp");
    let destination = dir.join("destination");
    std::fs::create_dir_all(&destination).expect("destination");
    store
        .upsert(job("transfer-1", &destination))
        .expect("write");

    // Simulate a write that was interrupted after the temp file was created
    // but before the rename: the real document must still be the one read.
    let mut temporary = store.path().as_os_str().to_os_string();
    temporary.push(".tmp");
    std::fs::write(PathBuf::from(&temporary), b"{ half a written doc").expect("write temp");

    let (reopened, status) = TransferStateStore::open(store.path().to_path_buf());

    assert_eq!(status, LoadStatus::Loaded);
    assert!(reopened.get("transfer-1").is_some());

    cleanup(&dir);
}

#[test]
fn jobs_that_cannot_be_used_are_dropped_without_losing_the_rest() {
    let dir = unique_temp_dir("recovery-sanitize");
    let path = dir.join("config").join(STATE_FILE);
    std::fs::create_dir_all(path.parent().expect("parent")).expect("config dir");
    std::fs::write(
        &path,
        br#"{"schemaVersion":1,"jobs":[
            {"id":"","request":{"sources":["C:\\a"],"destination":"C:\\b","operation":"copy"},"status":"running","queuedAtMs":1,"updatedAtMs":2,"progress":{}},
            {"id":"transfer-1","request":{"sources":["C:\\a"],"destination":"C:\\b","operation":"copy"},"status":"running","queuedAtMs":1,"updatedAtMs":2,"progress":{}},
            {"id":"transfer-1","request":{"sources":["C:\\a"],"destination":"C:\\b","operation":"copy"},"status":"running","queuedAtMs":1,"updatedAtMs":2,"progress":{}}
        ]}"#,
    )
    .expect("write");

    let (store, status) = TransferStateStore::open(path);

    assert_eq!(status, LoadStatus::Loaded);
    assert_eq!(store.len(), 1, "one usable, non-duplicated job is kept");
    assert!(store.get("transfer-1").is_some());

    cleanup(&dir);
}

// -- partial artifacts ------------------------------------------------------

#[test]
fn a_partial_artifact_name_matches_only_its_own_job() {
    assert!(is_artifact_of(
        ".crossport-transfer-1-0.partial",
        "transfer-1"
    ));
    assert!(is_artifact_of(
        ".crossport-transfer-1-42.partial",
        "transfer-1"
    ));

    assert!(
        !is_artifact_of(".crossport-transfer-10-0.partial", "transfer-1"),
        "a longer job identifier is a different job's artifact"
    );
    assert!(
        !is_artifact_of(".crossport-transfer-1-0.partial", "transfer-10"),
        "the identifier must match in full"
    );
    assert!(!is_artifact_of("report.txt", "transfer-1"));
    assert!(
        !is_artifact_of(".crossport-notes.txt", "transfer-1"),
        "a user file that merely starts with the prefix is not an artifact"
    );
    assert!(
        !is_artifact_of(".crossport-transfer-1.partial", "transfer-1"),
        "the index separator is required"
    );
    assert!(
        !is_artifact_of(".crossport-transfer-1-.partial", "transfer-1"),
        "the index must be present"
    );
    assert!(
        !is_artifact_of(".crossport-transfer-1-abc.partial", "transfer-1"),
        "the index must be digits"
    );
    assert!(
        !is_artifact_of(".crossport-transfer-1-0.partial.old", "transfer-1"),
        "the suffix must be exact"
    );
}

#[test]
fn a_scan_finds_only_the_jobs_leftovers() {
    let dir = unique_temp_dir("recovery-scan");
    let destination = dir.join("destination");
    let nested = destination.join("sub");
    std::fs::create_dir_all(&nested).expect("destination tree");

    let owned = [
        destination.join(".crossport-transfer-1-0.partial"),
        nested.join(".crossport-transfer-1-3.partial"),
    ];
    for path in &owned {
        std::fs::write(path, vec![0u8; 32]).expect("artifact");
    }

    // Everything here belongs to someone else, or to no one.
    std::fs::write(destination.join("keep.txt"), b"user data").expect("user file");
    std::fs::write(nested.join("keep.txt"), b"user data").expect("user file");
    std::fs::write(
        destination.join(".crossport-transfer-2-0.partial"),
        b"another job",
    )
    .expect("other job artifact");
    std::fs::write(destination.join(".crossport-notes.txt"), b"notes").expect("user file");
    std::fs::write(
        destination.join(".crossport-transfer-1-abc.partial"),
        b"not mine",
    )
    .expect("lookalike");

    let scan = scan(&destination, "transfer-1");

    assert!(!scan.truncated);
    assert!(scan.unreadable.is_empty());
    assert_eq!(scan.artifacts.len(), 2);
    assert_eq!(scan.bytes(), 64);
    let found: Vec<&str> = scan
        .artifacts
        .iter()
        .map(|artifact| artifact.path.as_str())
        .collect();
    assert!(found.contains(&owned[0].to_string_lossy().as_ref()));
    assert!(found.contains(&owned[1].to_string_lossy().as_ref()));

    // Discarding removes exactly those two files and nothing else.
    let removed = discard(&scan.artifacts).expect("discard succeeds");
    assert_eq!(removed.len(), 2);
    for path in &owned {
        assert!(!path.exists(), "the job's leftovers are gone");
    }
    assert!(
        destination.join("keep.txt").is_file(),
        "user data is untouched"
    );
    assert!(nested.join("keep.txt").is_file(), "user data is untouched");
    assert!(
        destination
            .join(".crossport-transfer-2-0.partial")
            .is_file(),
        "another job's leftovers are never touched"
    );
    assert!(destination.join(".crossport-notes.txt").is_file());
    assert!(destination
        .join(".crossport-transfer-1-abc.partial")
        .is_file());

    cleanup(&dir);
}

#[test]
fn a_directory_named_like_an_artifact_is_not_reported() {
    let dir = unique_temp_dir("recovery-artifact-dir");
    let destination = dir.join("destination");
    let lookalike = destination.join(".crossport-transfer-1-0.partial");
    std::fs::create_dir_all(&lookalike).expect("directory whose name looks like an artifact");

    let scan = scan(&destination, "transfer-1");

    assert!(
        scan.artifacts.is_empty(),
        "a directory is never a removable artifact"
    );
    assert!(lookalike.is_dir(), "the directory is untouched by the scan");

    cleanup(&dir);
}

#[test]
fn a_scan_of_a_missing_root_reports_it_instead_of_failing() {
    let dir = unique_temp_dir("recovery-scan-missing");

    let scan = scan(&dir.join("gone"), "transfer-1");

    assert!(scan.artifacts.is_empty());
    assert_eq!(scan.unreadable.len(), 1);
    assert!(!scan.truncated);

    cleanup(&dir);
}

#[test]
fn discard_tolerates_an_artifact_that_is_already_gone() {
    let dir = unique_temp_dir("recovery-discard-gone");
    let destination = dir.join("destination");
    std::fs::create_dir_all(&destination).expect("destination");
    let path = destination.join(".crossport-transfer-1-0.partial");
    std::fs::write(&path, b"partial").expect("artifact");

    let removed = discard(&[PartialArtifact {
        path: path.display().to_string(),
        bytes: 7,
    }])
    .expect("a vanished artifact is not a failure");
    assert_eq!(removed.len(), 1);

    // Removing it a second time is equally fine: cleanup is idempotent.
    discard(&[PartialArtifact {
        path: path.display().to_string(),
        bytes: 7,
    }])
    .expect("discard is idempotent");

    cleanup(&dir);
}

#[test]
fn artifact_directories_lists_each_directory_once_deepest_first() {
    let artifacts = vec![
        PartialArtifact {
            path: "C:\\dest\\a\\.crossport-j-0.partial".to_string(),
            bytes: 1,
        },
        PartialArtifact {
            path: "C:\\dest\\a\\.crossport-j-1.partial".to_string(),
            bytes: 1,
        },
        PartialArtifact {
            path: "C:\\dest\\b\\.crossport-j-2.partial".to_string(),
            bytes: 1,
        },
    ];

    let directories = artifact_directories(&artifacts);

    assert_eq!(directories, vec!["C:\\dest\\b", "C:\\dest\\a"]);
}

// -- classification ---------------------------------------------------------

fn probe() -> Probe {
    Probe {
        archive_confirms_finish: false,
        sources_present: true,
        destination_usable: true,
        planned: Some(PlanFacts {
            total_bytes: 4_000,
            total_files: 4,
            conflicts: 0,
            skipped_items: 0,
        }),
        plan_error: None,
        artifacts: ArtifactScan::default(),
        looks_complete: None,
    }
}

#[test]
fn classify_trusts_the_archive_over_everything_else() {
    let destination = PathBuf::from("C:\\destination");
    let interrupted = job("transfer-1", &destination);

    let mut facts = probe();
    facts.archive_confirms_finish = true;
    // Even if the world looks broken, proof of completion wins.
    facts.sources_present = false;
    facts.destination_usable = false;
    facts.planned = None;

    let (outcome, detail) = classify(&interrupted, ConflictStrategy::Skip, &facts);

    assert_eq!(outcome, RecoveryOutcome::CompletedBeforeCrash);
    assert!(!outcome.can_restart());
    assert!(detail
        .expect("a reason is given")
        .contains("terminal state"));
}

#[test]
fn classify_reports_a_missing_source() {
    let destination = PathBuf::from("C:\\destination");
    let interrupted = job("transfer-1", &destination);

    let mut facts = probe();
    facts.sources_present = false;

    let (outcome, detail) = classify(&interrupted, ConflictStrategy::Skip, &facts);

    assert_eq!(outcome, RecoveryOutcome::SourceMissing);
    assert!(!outcome.can_restart());
    assert!(detail.expect("a reason is given").contains("source"));
}

#[test]
fn classify_reports_an_unusable_destination() {
    let destination = PathBuf::from("C:\\destination");
    let interrupted = job("transfer-1", &destination);

    let mut facts = probe();
    facts.destination_usable = false;

    let (outcome, detail) = classify(&interrupted, ConflictStrategy::Skip, &facts);

    assert_eq!(outcome, RecoveryOutcome::DestinationUnavailable);
    assert!(!outcome.can_restart());
    assert!(detail.expect("a reason is given").contains("writable"));
}

#[test]
fn classify_reports_a_request_that_cannot_be_planned() {
    let destination = PathBuf::from("C:\\destination");
    let interrupted = job("transfer-1", &destination);

    let mut facts = probe();
    facts.planned = None;
    facts.plan_error = Some(StoredError::from(&AppError::InvalidInput(
        "the source is inside the destination".to_string(),
    )));

    let (outcome, detail) = classify(&interrupted, ConflictStrategy::Skip, &facts);

    assert_eq!(outcome, RecoveryOutcome::Unsupported);
    assert!(!outcome.can_restart());
    assert!(
        detail
            .expect("a reason is given")
            .contains("the source is inside the destination"),
        "the backend's own words are passed through"
    );
}

#[test]
fn classify_requires_a_restart_for_an_unfinished_job() {
    let destination = PathBuf::from("C:\\destination");
    let interrupted = job("transfer-1", &destination);

    let (outcome, detail) = classify(&interrupted, ConflictStrategy::Skip, &probe());

    assert_eq!(outcome, RecoveryOutcome::RestartRequired);
    assert!(outcome.can_restart());
    let detail = detail.expect("a reason is given");
    assert!(
        detail.contains("1000 of 4000 bytes"),
        "progress is reported in the backend's own words: {detail}"
    );
    assert!(
        detail.contains("from the beginning"),
        "the honest restart semantics are stated: {detail}"
    );
}

#[test]
fn classify_says_that_a_job_that_never_started_repeats_nothing() {
    let destination = PathBuf::from("C:\\destination");
    let mut queued = job("transfer-1", &destination);
    queued.started_at_ms = None;
    queued.status = TransferStatus::Queued;

    let (outcome, detail) = classify(&queued, ConflictStrategy::Skip, &probe());

    assert_eq!(outcome, RecoveryOutcome::RestartRequired);
    assert!(detail
        .expect("a reason is given")
        .contains("had not started"));
}

#[test]
fn classify_notes_conflicts_when_the_destination_already_has_entries() {
    let destination = PathBuf::from("C:\\destination");
    let interrupted = job("transfer-1", &destination);

    let mut facts = probe();
    facts.planned = Some(PlanFacts {
        total_bytes: 4_000,
        total_files: 4,
        conflicts: 2,
        skipped_items: 0,
    });

    let (outcome, detail) = classify(&interrupted, ConflictStrategy::Replace, &facts);

    assert_eq!(outcome, RecoveryOutcome::RestartRequired);
    let detail = detail.expect("a reason is given");
    assert!(detail.contains("2 entries already exist"), "{detail}");
    assert!(detail.contains("'replace' strategy"), "{detail}");
}

#[test]
fn classify_treats_a_destination_that_looks_complete_as_still_unfinished() {
    let destination = PathBuf::from("C:\\destination");
    let interrupted = job("transfer-1", &destination);

    let mut facts = probe();
    facts.looks_complete = Some(true);

    let (outcome, detail) = classify(&interrupted, ConflictStrategy::Skip, &facts);

    assert_eq!(
        outcome,
        RecoveryOutcome::RestartRequired,
        "a destination that merely looks complete is never marked completed"
    );
    let detail = detail.expect("a reason is given");
    assert!(detail.contains("still treated as unfinished"), "{detail}");
}

#[test]
fn classify_counts_the_artifacts_that_a_restart_will_remove() {
    let destination = PathBuf::from("C:\\destination");
    let interrupted = job("transfer-1", &destination);

    let mut facts = probe();
    facts.artifacts.artifacts.push(PartialArtifact {
        path: "C:\\destination\\.crossport-transfer-1-0.partial".to_string(),
        bytes: 512,
    });

    let (outcome, detail) = classify(&interrupted, ConflictStrategy::Skip, &facts);

    assert_eq!(outcome, RecoveryOutcome::RestartRequired);
    assert!(detail
        .expect("a reason is given")
        .contains("1 unfinished file(s) will be removed"));
}

// -- action validation ------------------------------------------------------

fn candidate(outcome: RecoveryOutcome, confirmed: bool) -> RecoveryCandidate {
    RecoveryCandidate {
        id: "transfer-1".to_string(),
        operation: TransferOperation::Copy,
        conflict: ConflictStrategy::Skip,
        verification: VerificationPolicy::Size,
        sources: vec!["C:\\source".to_string()],
        destination: "C:\\destination".to_string(),
        status: TransferStatus::Running,
        queued_at_ms: 1,
        started_at_ms: Some(2),
        updated_at_ms: 3,
        progress: PersistedProgress::default(),
        percent: None,
        verification_summary: None,
        outcome,
        detail: None,
        artifacts: Vec::new(),
        artifact_bytes: 0,
        artifacts_truncated: false,
        artifact_directories: Vec::new(),
        restart_impact: None,
        destination_looks_complete: None,
        confirmed_by_archive: confirmed,
        can_restart: outcome.can_restart(),
        can_discard: true,
    }
}

#[test]
fn validate_action_allows_a_restart_only_when_it_is_possible() {
    let restartable = candidate(RecoveryOutcome::RestartRequired, false);
    assert!(validate_action(&restartable, RecoveryAction::Restart).is_ok());

    let blocked = candidate(RecoveryOutcome::SourceMissing, false);
    let error = validate_action(&blocked, RecoveryAction::Restart)
        .expect_err("a job with a missing source cannot be restarted");
    assert_eq!(error.code(), "recovery_unavailable");
}

#[test]
fn validate_action_allows_discard_for_every_interrupted_episode() {
    for outcome in [
        RecoveryOutcome::RestartRequired,
        RecoveryOutcome::SourceMissing,
        RecoveryOutcome::DestinationUnavailable,
        RecoveryOutcome::Unsupported,
        RecoveryOutcome::CompletedBeforeCrash,
    ] {
        let candidate = candidate(outcome, false);
        assert!(
            validate_action(&candidate, RecoveryAction::Discard).is_ok(),
            "{outcome:?} must still be clearable"
        );
    }
}

#[test]
fn validate_action_refuses_to_confirm_a_job_the_archive_did_not_prove() {
    let unproven = candidate(RecoveryOutcome::RestartRequired, false);
    let error = validate_action(&unproven, RecoveryAction::Confirm)
        .expect_err("only the archive can confirm a finish");
    assert_eq!(error.code(), "recovery_unavailable");

    let proven = candidate(RecoveryOutcome::CompletedBeforeCrash, true);
    assert!(validate_action(&proven, RecoveryAction::Confirm).is_ok());
}

#[test]
fn validate_action_rejects_the_pending_placeholder() {
    let candidate = candidate(RecoveryOutcome::RestartRequired, false);
    let error = validate_action(&candidate, RecoveryAction::Pending)
        .expect_err("pending is not a decision");
    assert_eq!(error.code(), "invalid_input");
}

#[test]
fn recovery_outcomes_are_honest_about_what_they_mean() {
    assert!(!RecoveryOutcome::CompletedBeforeCrash.can_restart());
    assert!(RecoveryOutcome::RestartRequired.can_restart());
    for outcome in [
        RecoveryOutcome::CompletedBeforeCrash,
        RecoveryOutcome::RestartRequired,
        RecoveryOutcome::SourceMissing,
        RecoveryOutcome::DestinationUnavailable,
        RecoveryOutcome::Unsupported,
    ] {
        assert!(
            !outcome.explain().is_empty(),
            "{outcome:?} must explain itself to the user"
        );
    }
}
