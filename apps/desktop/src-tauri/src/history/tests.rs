/* ==========================================================================
 * History tests
 * Retention, filtering, and durability are the whole value of this module, so
 * every test here writes real files through the real document layer.
 * ========================================================================== */

use super::*;
use std::path::PathBuf;

use crate::filesystem::test_support::unique_temp_dir;
use crate::transfer::{TransferIssue, TransferIssueReason};
use crate::verification::{ChecksumAlgorithm, VerificationPolicy, VerificationStatus};

fn record(id: &str, status: HistoryStatus) -> HistoryRecord {
    HistoryRecord {
        id: id.to_string(),
        sources: vec![format!("C:\\src\\{id}")],
        destination: "D:\\backup".to_string(),
        status,
        total_bytes: 1_000,
        transferred_bytes: 1_000,
        finished_at_ms: 1_700_000_000_000,
        ..HistoryRecord::default()
    }
}

fn store(label: &str, limit: u32) -> (PathBuf, HistoryStore) {
    let dir = unique_temp_dir(label);
    let path = dir.join("config").join(HISTORY_FILE);
    let (store, status) = HistoryStore::open(path.clone(), limit);
    assert_eq!(status, LoadStatus::Missing, "a fresh store has no file");
    (dir, store)
}

#[test]
fn a_fresh_store_is_empty_and_writable() {
    let (dir, store) = store("history-empty", DEFAULT_HISTORY_LIMIT);

    assert!(store.is_empty());
    assert!(store.writable());
    assert_eq!(store.list(HistoryFilter::All), Vec::new());

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn records_are_listed_newest_first() {
    let (dir, store) = store("history-order", DEFAULT_HISTORY_LIMIT);

    for id in ["one", "two", "three"] {
        store
            .record(record(id, HistoryStatus::Completed))
            .expect("the record stores");
    }

    let ids: Vec<String> = store
        .list(HistoryFilter::All)
        .into_iter()
        .map(|entry| entry.id)
        .collect();

    assert_eq!(ids, vec!["three", "two", "one"]);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_record_survives_being_written_and_read_again() {
    let (dir, store) = store("history-round-trip", DEFAULT_HISTORY_LIMIT);
    let mut entry = record("kept", HistoryStatus::Failed);
    entry.issues.push(HistoryIssue::from(&TransferIssue::failed(
        "D:\\backup\\a.txt",
        crate::errors::AppError::DiskFull("the volume is full".to_string()),
    )));
    entry.error = Some(StoredError::from(&crate::errors::AppError::DiskFull(
        "the volume is full".to_string(),
    )));
    entry.verification = Some(HistoryVerification {
        status: VerificationStatus::Verified,
        method: crate::verification::VerificationMethod::Size,
        policy: VerificationPolicy::Size,
        checksum_algorithm: None,
        checked_files: 4,
        verified_files: 4,
        mismatched_files: 0,
        failed_files: 0,
        verified_bytes: 4_096,
        verdict: "verified (size, 4 files, 4096 bytes)".to_string(),
    });
    store.record(entry.clone()).expect("the record stores");

    let (reopened, status) = HistoryStore::open(store.path().to_path_buf(), DEFAULT_HISTORY_LIMIT);

    assert_eq!(status, LoadStatus::Loaded);
    assert_eq!(reopened.list(HistoryFilter::All), vec![entry]);
    assert_eq!(
        reopened.list(HistoryFilter::All)[0].issues[0]
            .error
            .as_ref()
            .map(|error| error.code.as_str()),
        Some("disk_full"),
        "a stored issue error must survive the round trip"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn every_status_round_trips_through_the_identifier() {
    for status in [
        HistoryStatus::Completed,
        HistoryStatus::Failed,
        HistoryStatus::Cancelled,
        HistoryStatus::Interrupted,
        HistoryStatus::Recovered,
    ] {
        let json = serde_json::to_value(status).expect("serializes");
        assert_eq!(json, serde_json::json!(status.as_str()));
    }
}

#[test]
fn live_statuses_map_onto_history_statuses() {
    assert_eq!(
        HistoryStatus::from_transfer_status(TransferStatus::Completed),
        Some(HistoryStatus::Completed)
    );
    assert_eq!(
        HistoryStatus::from_transfer_status(TransferStatus::Failed),
        Some(HistoryStatus::Failed)
    );
    assert_eq!(
        HistoryStatus::from_transfer_status(TransferStatus::Cancelled),
        Some(HistoryStatus::Cancelled)
    );
    for live in [
        TransferStatus::Queued,
        TransferStatus::Preparing,
        TransferStatus::Running,
        TransferStatus::Paused,
        TransferStatus::Cancelling,
    ] {
        assert_eq!(
            HistoryStatus::from_transfer_status(live),
            None,
            "{live:?} has not finished, so it has no history status"
        );
    }
}

#[test]
fn only_successful_statuses_count_as_success() {
    assert!(HistoryStatus::Completed.succeeded());
    assert!(HistoryStatus::Recovered.succeeded());
    assert!(!HistoryStatus::Failed.succeeded());
    assert!(!HistoryStatus::Cancelled.succeeded());
    assert!(!HistoryStatus::Interrupted.succeeded());
}

#[test]
fn filters_select_exactly_their_status() {
    let (dir, store) = store("history-filter", DEFAULT_HISTORY_LIMIT);
    for (id, status) in [
        ("done", HistoryStatus::Completed),
        ("bad", HistoryStatus::Failed),
        ("stopped", HistoryStatus::Cancelled),
        ("killed", HistoryStatus::Interrupted),
        ("saved", HistoryStatus::Recovered),
    ] {
        store.record(record(id, status)).expect("the record stores");
    }

    let ids = |filter| {
        let mut ids: Vec<String> = store
            .list(filter)
            .into_iter()
            .map(|entry| entry.id)
            .collect();
        ids.sort();
        ids
    };

    assert_eq!(ids(HistoryFilter::All).len(), 5);
    assert_eq!(ids(HistoryFilter::Completed), vec!["done"]);
    assert_eq!(ids(HistoryFilter::Failed), vec!["bad"]);
    assert_eq!(ids(HistoryFilter::Cancelled), vec!["stopped"]);
    assert_eq!(
        ids(HistoryFilter::Interrupted),
        vec!["killed", "saved"],
        "the interrupted filter covers recovered episodes too"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn filter_identifiers_round_trip_and_reject_typos() {
    for filter in HistoryFilter::ALL {
        assert_eq!(
            HistoryFilter::parse(filter.as_str()).expect("its own identifier parses"),
            filter
        );
    }

    let error = HistoryFilter::parse("everything").expect_err("unknown filters are rejected");
    assert!(error.contains("everything"));
}

#[test]
fn a_duplicate_identifier_is_refused_rather_than_overwritten() {
    let (dir, store) = store("history-duplicate", DEFAULT_HISTORY_LIMIT);
    store
        .record(record("one", HistoryStatus::Completed))
        .expect("the first record stores");

    let error = store
        .record(record("one", HistoryStatus::Failed))
        .expect_err("a duplicate must be refused");

    assert_eq!(error.code(), "invalid_input");
    assert_eq!(store.len(), 1);
    assert_eq!(
        store.get("one").expect("still there").status,
        HistoryStatus::Completed
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_record_that_could_never_be_displayed_is_refused() {
    let (dir, store) = store("history-invalid", DEFAULT_HISTORY_LIMIT);

    let mut no_id = record("x", HistoryStatus::Completed);
    no_id.id = String::new();
    assert_eq!(
        store
            .record(no_id)
            .expect_err("an identifier is required")
            .code(),
        "invalid_input"
    );

    let mut no_sources = record("y", HistoryStatus::Completed);
    no_sources.sources.clear();
    assert_eq!(
        store
            .record(no_sources)
            .expect_err("a source is required")
            .code(),
        "invalid_input"
    );

    let mut no_destination = record("z", HistoryStatus::Completed);
    no_destination.destination = "  ".to_string();
    assert_eq!(
        store
            .record(no_destination)
            .expect_err("a destination is required")
            .code(),
        "invalid_input"
    );

    assert!(store.is_empty(), "nothing invalid reached the store");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn the_newest_records_survive_retention() {
    let (dir, store) = store("history-retention", MIN_HISTORY_LIMIT);
    let limit = MIN_HISTORY_LIMIT as usize;

    for index in 0..limit + 25 {
        store
            .record(record(&format!("job-{index:03}"), HistoryStatus::Completed))
            .expect("the record stores");
    }

    let kept = store.list(HistoryFilter::All);
    assert_eq!(kept.len(), limit);
    assert_eq!(
        kept[0].id,
        format!("job-{:03}", limit + 24),
        "the newest record is still first"
    );
    assert_eq!(
        kept[limit - 1].id,
        "job-025",
        "the oldest records are the ones removed"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn retention_is_reported_to_the_caller() {
    let (dir, store) = store("history-retention-report", MIN_HISTORY_LIMIT);
    for index in 0..MIN_HISTORY_LIMIT {
        let outcome = store
            .record(record(&format!("job-{index:03}"), HistoryStatus::Completed))
            .expect("the record stores");
        assert_eq!(outcome.removed, 0, "nothing is removed before the bound");
    }

    let outcome = store
        .record(record("one-more", HistoryStatus::Completed))
        .expect("the record stores");

    assert_eq!(outcome.removed, 1);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn lowering_the_limit_prunes_immediately_and_persists_it() {
    let (dir, store) = store("history-lower-limit", DEFAULT_HISTORY_LIMIT);
    for index in 0..50 {
        store
            .record(record(&format!("job-{index:03}"), HistoryStatus::Completed))
            .expect("the record stores");
    }

    let outcome = store
        .set_limit(MIN_HISTORY_LIMIT)
        .expect("the limit applies");

    assert_eq!(outcome.removed, 50 - MIN_HISTORY_LIMIT as usize);
    assert_eq!(store.len(), MIN_HISTORY_LIMIT as usize);

    // The pruned list is what was written, not just what is in memory.
    let (reopened, _) = HistoryStore::open(store.path().to_path_buf(), DEFAULT_HISTORY_LIMIT);
    assert_eq!(
        reopened.len(),
        MIN_HISTORY_LIMIT as usize,
        "a reopen sees the pruned list even with a larger limit"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_limit_outside_the_accepted_range_is_clamped_not_obeyed() {
    assert_eq!(clamp_limit(0), MIN_HISTORY_LIMIT as usize);
    assert_eq!(clamp_limit(1), MIN_HISTORY_LIMIT as usize);
    assert_eq!(clamp_limit(100), 100);
    assert_eq!(clamp_limit(u32::MAX), MAX_HISTORY_LIMIT as usize);
}

#[test]
fn setting_validation_rejects_a_limit_that_would_be_clamped() {
    assert!(validate_limit(MIN_HISTORY_LIMIT).is_ok());
    assert!(validate_limit(MAX_HISTORY_LIMIT).is_ok());
    assert!(validate_limit(MIN_HISTORY_LIMIT - 1).is_err());
    assert!(validate_limit(MAX_HISTORY_LIMIT + 1).is_err());
}

#[test]
fn a_zero_limit_cannot_erase_the_history() {
    let (dir, store) = store("history-zero-limit", DEFAULT_HISTORY_LIMIT);
    store
        .record(record("kept", HistoryStatus::Completed))
        .expect("the record stores");

    store.set_limit(0).expect("the limit is clamped");

    assert_eq!(store.len(), 1, "a zero limit must not empty the history");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn records_can_be_fetched_deleted_and_cleared() {
    let (dir, store) = store("history-delete", DEFAULT_HISTORY_LIMIT);
    store
        .record(record("one", HistoryStatus::Completed))
        .expect("the record stores");
    store
        .record(record("two", HistoryStatus::Cancelled))
        .expect("the record stores");

    assert_eq!(store.get("two").expect("present").id, "two");
    assert!(store.delete("two").expect("deletion works"));
    assert!(!store
        .delete("two")
        .expect("a second deletion is not an error"));
    assert_eq!(store.len(), 1);

    assert_eq!(store.clear().expect("clearing works"), 1);
    assert!(store.is_empty());

    let (reopened, _) = HistoryStore::open(store.path().to_path_buf(), DEFAULT_HISTORY_LIMIT);
    assert!(
        reopened.is_empty(),
        "clearing must be persisted, not just done in memory"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_corrupt_document_is_preserved_and_reported() {
    let dir = unique_temp_dir("history-corrupt");
    let path = dir.join("config").join(HISTORY_FILE);
    std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
    std::fs::write(&path, b"{\"records\": [ this is not json").expect("writable");

    let (store, status) = HistoryStore::open(path.clone(), DEFAULT_HISTORY_LIMIT);

    assert!(
        matches!(status, LoadStatus::Recovered { .. }),
        "a corrupt document must be reported: {status:?}"
    );
    assert!(store.is_empty());
    assert!(
        store.writable(),
        "the path is free again for the next write"
    );
    assert!(!path.exists(), "the unusable file was moved aside");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_document_from_a_newer_build_is_never_overwritten() {
    let dir = unique_temp_dir("history-newer");
    let path = dir.join("config").join(HISTORY_FILE);
    std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
    let contents = br#"{"schemaVersion":99,"records":[]}"#;
    std::fs::write(&path, contents).expect("writable");

    let (store, status) = HistoryStore::open(path.clone(), DEFAULT_HISTORY_LIMIT);

    assert!(matches!(status, LoadStatus::Unsupported { .. }));
    assert!(
        !store.writable(),
        "an unknown document must not be replaced"
    );

    // Recording still works in memory, but nothing is written over that file.
    store
        .record(record("session-only", HistoryStatus::Completed))
        .expect("the record is kept for this session");
    assert_eq!(store.len(), 1);
    assert_eq!(
        std::fs::read(&path).expect("still there"),
        contents,
        "the newer document must survive untouched"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn unusable_records_inside_a_readable_document_do_not_cost_the_rest() {
    let dir = unique_temp_dir("history-partial");
    let path = dir.join("config").join(HISTORY_FILE);
    std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
    std::fs::write(
        &path,
        br#"{
            "schemaVersion": 1,
            "records": [
                {"id": "good", "sources": ["C:\\a"], "destination": "D:\\b", "status": "completed"},
                {"id": "", "sources": ["C:\\a"], "destination": "D:\\b"},
                {"id": "good", "sources": ["C:\\a"], "destination": "D:\\b"},
                {"id": "also-good", "sources": ["C:\\a"], "destination": "D:\\b", "status": "failed"}
            ]
        }"#,
    )
    .expect("writable");

    let (store, status) = HistoryStore::open(path, DEFAULT_HISTORY_LIMIT);

    assert_eq!(status, LoadStatus::Loaded);
    let ids: Vec<String> = store
        .list(HistoryFilter::All)
        .into_iter()
        .map(|entry| entry.id)
        .collect();
    assert_eq!(ids, vec!["good", "also-good"], "the valid records survive");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_document_without_a_version_is_migrated() {
    let dir = unique_temp_dir("history-migration");
    let path = dir.join("config").join(HISTORY_FILE);
    std::fs::create_dir_all(path.parent().expect("has a parent")).expect("directory");
    std::fs::write(
        &path,
        br#"{"records":[{"id":"old","sources":["C:\\a"],"destination":"D:\\b"}]}"#,
    )
    .expect("writable");

    let (store, status) = HistoryStore::open(path, DEFAULT_HISTORY_LIMIT);

    assert_eq!(status, LoadStatus::Migrated { from: 0 });
    assert_eq!(store.len(), 1);
    assert_eq!(
        store.get("old").expect("present").status,
        HistoryStatus::Completed,
        "an older document's missing fields take their defaults"
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_record_summarizes_its_sources_without_inventing_any() {
    let mut entry = record("one", HistoryStatus::Completed);
    assert_eq!(entry.source_summary(), "C:\\src\\one");

    entry.sources = vec![
        "C:\\a".to_string(),
        "C:\\b".to_string(),
        "C:\\c".to_string(),
    ];
    assert_eq!(entry.source_summary(), "C:\\a and 2 more");

    entry.sources.clear();
    assert_eq!(entry.source_summary(), "(no source)");
}

#[test]
fn recovery_actions_round_trip_and_reject_typos() {
    for action in [
        RecoveryAction::Pending,
        RecoveryAction::Discard,
        RecoveryAction::Restart,
        RecoveryAction::Confirm,
    ] {
        assert_eq!(
            RecoveryAction::parse(action.as_str()).expect("its own identifier parses"),
            action
        );
    }

    assert!(RecoveryAction::parse("resume").is_err());
}

#[test]
fn a_verification_summary_is_condensed_for_history() {
    let mut log = crate::verification::VerificationLog::new(VerificationPolicy::Checksum);
    log.set_planned_files(2);
    log.finish();
    let summary = log.summary(true);

    let stored = HistoryVerification::from(&summary);

    assert_eq!(stored.policy, VerificationPolicy::Checksum);
    assert_eq!(stored.checksum_algorithm, Some(ChecksumAlgorithm::Sha256));
    assert_eq!(stored.status, VerificationStatus::Skipped);
    assert!(
        !stored.verdict.is_empty(),
        "history keeps the verdict line, not just the fields"
    );
}

#[test]
fn history_issue_reasons_survive_persistence() {
    let issue = TransferIssue {
        path: "D:\\b\\link".to_string(),
        reason: TransferIssueReason::Unsupported,
        error: None,
        detail: Some("symbolic links are not copied".to_string()),
    };

    let stored = HistoryIssue::from(&issue);
    let json = serde_json::to_string(&stored).expect("serializes");
    let parsed: HistoryIssue = serde_json::from_str(&json).expect("parses");

    assert_eq!(parsed, stored);
    assert_eq!(parsed.reason, TransferIssueReason::Unsupported);
    assert_eq!(
        parsed.detail.as_deref(),
        Some("symbolic links are not copied")
    );
}
