/* ==========================================================================
 * Verification tests
 * Every case here writes real files and runs the real verifier: this module's
 * whole purpose is to report what is actually on disk, so a mocked filesystem
 * would test nothing.
 * ========================================================================== */

use super::*;
use crate::filesystem::test_support::unique_temp_dir;

/// A checkpoint that never stops.
fn run() -> bool {
    true
}

/// A checkpoint that stops immediately.
fn stop() -> bool {
    false
}

fn write(path: &Path, bytes: &[u8]) {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent).expect("the parent is creatable");
    }
    std::fs::write(path, bytes).expect("the file is writable");
}

/// A digest of `bytes`, as the copy engine would have computed while streaming.
fn digest_of(bytes: &[u8]) -> String {
    let mut hasher = StreamHasher::new();
    hasher.update(bytes);
    hasher.finish()
}

fn request<'a>(
    source: &'a Path,
    destination: &'a Path,
    expected_bytes: u64,
    streamed_checksum: Option<String>,
) -> ItemVerification<'a> {
    ItemVerification {
        source,
        destination,
        expected_bytes,
        streamed_checksum,
    }
}

#[test]
fn the_default_policy_verifies_size() {
    assert_eq!(VerificationPolicy::default(), VerificationPolicy::Size);
}

#[test]
fn policies_map_to_the_method_they_run() {
    assert_eq!(VerificationPolicy::None.method(), VerificationMethod::None);
    assert_eq!(VerificationPolicy::Size.method(), VerificationMethod::Size);
    assert_eq!(
        VerificationPolicy::Checksum.method(),
        VerificationMethod::SizeAndChecksum
    );
}

#[test]
fn policies_round_trip_through_their_identifiers() {
    for policy in [
        VerificationPolicy::None,
        VerificationPolicy::Size,
        VerificationPolicy::Checksum,
    ] {
        assert_eq!(
            VerificationPolicy::parse(policy.as_str()).expect("its own identifier parses"),
            policy
        );
    }
}

#[test]
fn an_unknown_policy_identifier_is_rejected_rather_than_downgraded() {
    let error = VerificationPolicy::parse("sha512").expect_err("unknown policies are rejected");

    assert_eq!(error.code(), "invalid_input");
    assert!(error.to_string().contains("sha512"));
}

#[test]
fn only_the_checksum_policy_names_an_algorithm() {
    assert_eq!(VerificationPolicy::None.algorithm(), None);
    assert_eq!(VerificationPolicy::Size.algorithm(), None);
    assert_eq!(
        VerificationPolicy::Checksum.algorithm(),
        Some(ChecksumAlgorithm::Sha256)
    );
    assert_eq!(ChecksumAlgorithm::Sha256.as_str(), "sha256");
}

#[test]
fn a_policy_of_none_reports_skipped_rather_than_verified() {
    let dir = unique_temp_dir("verify-none");
    let source = dir.join("source.txt");
    let destination = dir.join("destination.txt");
    write(&source, b"hello");
    write(&destination, b"hello");

    let result = verify_item(
        request(&source, &destination, 5, None),
        VerificationPolicy::None,
        &mut run,
    )
    .expect("verification runs");

    assert_eq!(result.status, VerificationStatus::Skipped);
    assert_eq!(result.method, VerificationMethod::None);
    assert!(!result.status.is_proven(), "nothing was checked");
    assert!(!result.status.is_failure(), "skipping is not a failure");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_matching_size_verifies() {
    let dir = unique_temp_dir("verify-size-match");
    let source = dir.join("source.bin");
    let destination = dir.join("destination.bin");
    let bytes = vec![0x5Au8; 4096];
    write(&source, &bytes);
    write(&destination, &bytes);

    let result = verify_item(
        request(&source, &destination, bytes.len() as u64, None),
        VerificationPolicy::Size,
        &mut run,
    )
    .expect("verification runs");

    assert_eq!(result.status, VerificationStatus::Verified);
    assert_eq!(result.method, VerificationMethod::Size);
    assert_eq!(result.expected_bytes, Some(4096));
    assert_eq!(result.actual_bytes, Some(4096));
    assert!(result.mismatch.is_none());
    assert!(result.error.is_none());
    assert!(result.status.is_proven());

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_truncated_destination_is_a_size_mismatch() {
    let dir = unique_temp_dir("verify-size-mismatch");
    let source = dir.join("source.bin");
    let destination = dir.join("destination.bin");
    write(&source, &vec![1u8; 1000]);
    write(&destination, &vec![1u8; 400]);

    let result = verify_item(
        request(&source, &destination, 1000, None),
        VerificationPolicy::Size,
        &mut run,
    )
    .expect("verification runs");

    assert_eq!(result.status, VerificationStatus::Mismatch);
    let mismatch = result.mismatch.expect("a mismatch is described");
    assert_eq!(mismatch.reason, VerificationMismatchReason::SizeMismatch);
    assert_eq!(mismatch.expected, "1000 bytes");
    assert_eq!(mismatch.actual, "400 bytes");
    assert_eq!(mismatch.path, destination.display().to_string());
    assert!(result.status.is_failure());

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_destination_that_does_not_exist_is_a_mismatch_not_a_failure_to_run() {
    let dir = unique_temp_dir("verify-missing");
    let source = dir.join("source.bin");
    let destination = dir.join("gone.bin");
    write(&source, &[2u8; 10]);

    let result = verify_item(
        request(&source, &destination, 10, None),
        VerificationPolicy::Size,
        &mut run,
    )
    .expect("verification runs");

    assert_eq!(result.status, VerificationStatus::Mismatch);
    let mismatch = result.mismatch.expect("a mismatch is described");
    assert_eq!(
        mismatch.reason,
        VerificationMismatchReason::DestinationMissing
    );
    assert_eq!(mismatch.actual, "nothing");

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_directory_where_a_file_was_written_is_a_mismatch() {
    let dir = unique_temp_dir("verify-directory");
    let source = dir.join("source.bin");
    let destination = dir.join("destination");
    write(&source, &[3u8; 10]);
    std::fs::create_dir_all(&destination).expect("the destination directory is creatable");

    let result = verify_item(
        request(&source, &destination, 10, None),
        VerificationPolicy::Size,
        &mut run,
    )
    .expect("verification runs");

    assert_eq!(result.status, VerificationStatus::Mismatch);
    let mismatch = result.mismatch.expect("a mismatch is described");
    assert_eq!(
        mismatch.reason,
        VerificationMismatchReason::DestinationNotAFile
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_checksum_that_matches_verifies_with_both_digests_recorded() {
    let dir = unique_temp_dir("verify-checksum-match");
    let source = dir.join("source.bin");
    let destination = dir.join("destination.bin");
    let bytes: Vec<u8> = (0..=255u8).cycle().take(70_000).collect();
    write(&source, &bytes);
    write(&destination, &bytes);

    let result = verify_item(
        request(
            &source,
            &destination,
            bytes.len() as u64,
            Some(digest_of(&bytes)),
        ),
        VerificationPolicy::Checksum,
        &mut run,
    )
    .expect("verification runs");

    assert_eq!(result.status, VerificationStatus::Verified);
    assert_eq!(result.method, VerificationMethod::SizeAndChecksum);
    assert_eq!(result.checksum_algorithm, Some(ChecksumAlgorithm::Sha256));
    assert_eq!(result.expected_checksum, result.actual_checksum);
    assert!(result
        .expected_checksum
        .expect("digest")
        .starts_with("sha256:"));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn corrupted_bytes_of_the_same_size_are_a_checksum_mismatch() {
    let dir = unique_temp_dir("verify-checksum-mismatch");
    let source = dir.join("source.bin");
    let destination = dir.join("destination.bin");
    let bytes = vec![0x11u8; 8192];
    write(&source, &bytes);
    // Same length, one byte different: only a checksum can see this.
    let mut corrupted = bytes.clone();
    corrupted[4000] = 0x22;
    write(&destination, &corrupted);

    let result = verify_item(
        request(
            &source,
            &destination,
            bytes.len() as u64,
            Some(digest_of(&bytes)),
        ),
        VerificationPolicy::Checksum,
        &mut run,
    )
    .expect("verification runs");

    assert_eq!(result.status, VerificationStatus::Mismatch);
    let mismatch = result.mismatch.expect("a mismatch is described");
    assert_eq!(
        mismatch.reason,
        VerificationMismatchReason::ChecksumMismatch
    );
    assert!(
        mismatch.expected.contains("sha256"),
        "the mismatch must name the algorithm: {}",
        mismatch.expected
    );
    assert_ne!(result.expected_checksum, result.actual_checksum);
    assert_eq!(result.expected_bytes, result.actual_bytes);

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_checksum_policy_without_a_source_digest_fails_instead_of_claiming_success() {
    let dir = unique_temp_dir("verify-checksum-missing-digest");
    let source = dir.join("source.bin");
    let destination = dir.join("destination.bin");
    write(&source, b"payload");
    write(&destination, b"payload");

    let result = verify_item(
        request(&source, &destination, 7, None),
        VerificationPolicy::Checksum,
        &mut run,
    )
    .expect("verification runs");

    assert_eq!(
        result.status,
        VerificationStatus::Failed,
        "a comparison that cannot be made is never a success"
    );
    assert_eq!(result.failure_error().code(), "verification_failed");
    assert!(result.actual_checksum.is_none());

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_stopped_verification_reports_no_verdict() {
    let dir = unique_temp_dir("verify-stopped");
    let source = dir.join("source.bin");
    let destination = dir.join("destination.bin");
    let bytes = vec![9u8; 5000];
    write(&source, &bytes);
    write(&destination, &bytes);

    let outcome = verify_item(
        request(&source, &destination, 5000, Some(digest_of(&bytes))),
        VerificationPolicy::Checksum,
        &mut stop,
    );

    assert_eq!(outcome, Err(VerificationAborted));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_changed_size_between_the_checks_is_reported_as_a_size_mismatch() {
    let dir = unique_temp_dir("verify-grew");
    let source = dir.join("source.bin");
    let destination = dir.join("destination.bin");
    write(&source, &[4u8; 100]);
    // Verified against a promise of 100 bytes, but the file holds 200: the
    // size check catches it before any hashing is needed.
    write(&destination, &[4u8; 200]);

    let result = verify_item(
        request(&source, &destination, 100, Some(digest_of(&[4u8; 100]))),
        VerificationPolicy::Checksum,
        &mut run,
    )
    .expect("verification runs");

    assert_eq!(result.status, VerificationStatus::Mismatch);
    assert_eq!(
        result.mismatch.expect("a mismatch").reason,
        VerificationMismatchReason::SizeMismatch
    );

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_failure_error_names_the_reason_it_failed_for() {
    let dir = unique_temp_dir("verify-failure-error");
    let source = dir.join("source.bin");
    let destination = dir.join("destination.bin");
    write(&source, &[1u8; 50]);
    write(&destination, &[1u8; 20]);

    let result = verify_item(
        request(&source, &destination, 50, None),
        VerificationPolicy::Size,
        &mut run,
    )
    .expect("verification runs");

    let error = result.failure_error();

    assert_eq!(error.code(), "verification_failed");
    assert!(error.to_string().contains("size_mismatch"));
    assert!(error.to_string().contains("destination.bin"));

    let _ = std::fs::remove_dir_all(dir);
}

#[test]
fn a_link_where_a_file_was_written_is_a_mismatch_and_is_never_followed() {
    let dir = unique_temp_dir("verify-symlink");
    let source = dir.join("source.bin");
    let outside = dir.join("elsewhere.bin");
    let destination = dir.join("destination.bin");
    write(&source, &[6u8; 32]);

    // A link that happens to point at content of the right size: following it
    // would report a match, which is exactly the mistake this test exists to
    // catch.
    write(&outside, &[6u8; 32]);
    if !try_symlink_file(&outside, &destination) {
        // Some Windows configurations refuse to create links without developer
        // mode. The case cannot be set up here rather than passing falsely.
        let _ = std::fs::remove_dir_all(dir);
        return;
    }

    let result = verify_item(
        request(&source, &destination, 32, None),
        VerificationPolicy::Size,
        &mut run,
    )
    .expect("verification runs");

    assert_eq!(result.status, VerificationStatus::Mismatch);
    let mismatch = result.mismatch.expect("a mismatch is described");
    assert_eq!(
        mismatch.reason,
        VerificationMismatchReason::DestinationNotAFile
    );
    assert_eq!(mismatch.actual, "a link");
    assert!(
        !result.status.is_proven(),
        "a link must never be reported as verified"
    );

    let _ = std::fs::remove_dir_all(dir);
}

/// Creates a file symlink, returning `false` when the platform refuses to.
fn try_symlink_file(target: &Path, link: &Path) -> bool {
    #[cfg(unix)]
    {
        std::os::unix::fs::symlink(target, link).is_ok()
    }
    #[cfg(windows)]
    {
        std::os::windows::fs::symlink_file(target, link).is_ok()
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = (target, link);
        false
    }
}

#[test]
fn coverage_claims_only_what_the_policy_ran() {
    let size = VerificationCoverage::for_policy(VerificationPolicy::Size);
    assert!(size.size);
    assert!(size.structure);
    assert!(!size.checksum);

    let checksum = VerificationCoverage::for_policy(VerificationPolicy::Checksum);
    assert!(checksum.size);
    assert!(checksum.checksum);

    let none = VerificationCoverage::for_policy(VerificationPolicy::None);
    assert!(!none.size);
    assert!(!none.structure);
    assert!(!none.checksum);

    // Metadata this engine does not reapply must never be claimed.
    for coverage in [size, checksum, none] {
        assert!(
            !coverage.modified_time_preserved,
            "modified time is not preserved, so it must not be claimed"
        );
        assert!(
            !coverage.readonly_preserved,
            "the read-only attribute is not preserved, so it must not be claimed"
        );
    }
}

fn verified(destination: &Path, bytes: u64) -> FileVerification {
    FileVerification {
        path: destination.display().to_string(),
        method: VerificationMethod::Size,
        status: VerificationStatus::Verified,
        expected_bytes: Some(bytes),
        actual_bytes: Some(bytes),
        checksum_algorithm: None,
        expected_checksum: None,
        actual_checksum: None,
        mismatch: None,
        error: None,
        duration_ms: 1,
    }
}

fn mismatched(destination: &Path) -> FileVerification {
    FileVerification {
        mismatch: Some(VerificationMismatch {
            reason: VerificationMismatchReason::SizeMismatch,
            path: destination.display().to_string(),
            expected: "10 bytes".to_string(),
            actual: "4 bytes".to_string(),
            detail: "the destination holds a different number of bytes than the source did"
                .to_string(),
        }),
        status: VerificationStatus::Mismatch,
        expected_bytes: Some(10),
        actual_bytes: Some(4),
        ..verified(destination, 10)
    }
}

#[test]
fn a_log_with_nothing_recorded_reports_pending() {
    let log = VerificationLog::new(VerificationPolicy::Size);
    let summary = log.summary(false);

    assert_eq!(summary.status, VerificationStatus::Pending);
    assert_eq!(summary.verified_files, 0);
    assert!(!summary.is_proven());
}

#[test]
fn a_log_reports_progress_while_the_job_is_still_running() {
    let mut log = VerificationLog::new(VerificationPolicy::Size);
    log.set_planned_files(4);
    log.record(&verified(Path::new("C:\\out\\a.bin"), 100));

    let live = log.summary(false);
    assert_eq!(live.status, VerificationStatus::Verifying);
    assert!(
        !live.is_proven(),
        "an unfinished job has not proven anything yet"
    );

    log.finish();
    let settled = log.summary(true);
    assert_eq!(settled.status, VerificationStatus::Verified);
    assert!(settled.is_proven());
}

#[test]
fn a_log_counts_what_it_verified_and_what_it_did_not() {
    let mut log = VerificationLog::new(VerificationPolicy::Size);
    log.set_planned_files(5);
    log.record(&verified(Path::new("a"), 100));
    log.record(&verified(Path::new("b"), 250));
    log.record(&mismatched(Path::new("c")));
    log.record(&FileVerification::skipped(Path::new("d")));
    log.finish();

    let summary = log.summary(true);

    assert_eq!(summary.status, VerificationStatus::Mismatch);
    assert_eq!(summary.planned_files, 5);
    assert_eq!(summary.checked_files, 3);
    assert_eq!(summary.verified_files, 2);
    assert_eq!(summary.mismatched_files, 1);
    assert_eq!(summary.skipped_files, 1);
    assert_eq!(
        summary.unverified_files, 1,
        "one planned file produced no result at all"
    );
    assert_eq!(summary.verified_bytes, 350);
    assert_eq!(summary.mismatches.len(), 1);
    assert!(summary.mismatches[0]
        .detail
        .contains("different number of bytes"));
    assert!(log.has_failures());
}

#[test]
fn a_single_mismatch_fails_the_whole_job_verdict() {
    let mut log = VerificationLog::new(VerificationPolicy::Checksum);
    log.set_planned_files(3);
    log.record(&verified(Path::new("a"), 10));
    log.record(&verified(Path::new("b"), 10));
    log.record(&mismatched(Path::new("c")));
    log.finish();

    let summary = log.summary(true);

    assert_eq!(summary.status, VerificationStatus::Mismatch);
    assert!(!summary.is_proven());
    assert!(log.has_failures());
    assert!(summary.verdict.contains("did not verify"));
}

#[test]
fn a_log_that_only_skipped_reports_skipped_not_verified() {
    let mut log = VerificationLog::new(VerificationPolicy::None);
    log.set_planned_files(2);
    log.record(&FileVerification::skipped(Path::new("a")));
    log.record(&FileVerification::skipped(Path::new("b")));
    log.finish();

    let summary = log.summary(true);

    assert_eq!(summary.status, VerificationStatus::Skipped);
    assert_eq!(summary.skipped_files, 2);
    assert_eq!(summary.verified_files, 0);
    assert!(!summary.is_proven());
    assert_eq!(summary.verdict, "not verified");
}

#[test]
fn a_verified_job_never_reports_a_skipped_status() {
    let mut log = VerificationLog::new(VerificationPolicy::None);
    log.set_planned_files(1);
    log.record(&verified(Path::new("a"), 1));
    log.finish();

    assert_eq!(log.summary(true).status, VerificationStatus::Verified);
}

#[test]
fn an_error_raised_during_verification_is_kept_and_fails_the_job() {
    let mut log = VerificationLog::new(VerificationPolicy::Checksum);
    log.record_error(AppError::PermissionDenied("C:\\out\\a.bin".to_string()));
    log.finish();

    let summary = log.summary(true);

    assert_eq!(summary.status, VerificationStatus::Failed);
    assert_eq!(summary.failed_files, 1);
    assert_eq!(
        summary.error.as_ref().map(|error| error.code.as_str()),
        Some("permission_denied")
    );
    assert_eq!(
        summary
            .error
            .as_ref()
            .map(StoredError::to_error)
            .map(|error| error.code()),
        Some("permission_denied"),
        "a stored error must map back to the same category"
    );
    assert!(!summary.is_proven());
}

#[test]
fn a_log_records_the_duration_it_actually_took() {
    let mut log = VerificationLog::new(VerificationPolicy::Size);
    log.record(&verified(Path::new("a"), 1));
    std::thread::sleep(Duration::from_millis(5));
    log.finish();

    assert!(log.summary(true).duration_ms >= 5);
}

#[test]
fn mismatch_lists_are_bounded_and_the_truncation_is_reported() {
    let mut log = VerificationLog::new(VerificationPolicy::Size);
    log.set_planned_files(MAX_MISMATCHES as u64 + 50);
    for index in 0..MAX_MISMATCHES + 50 {
        log.record(&mismatched(Path::new(&format!("file-{index}"))));
    }
    log.finish();

    let summary = log.summary(true);

    assert_eq!(summary.mismatches.len(), MAX_MISMATCHES);
    assert!(summary.mismatches_truncated);
    assert_eq!(summary.mismatched_files as usize, MAX_MISMATCHES + 50);
}

#[test]
fn a_resumed_summary_keeps_the_digest_algorithm_on_the_wire() {
    let mut log = VerificationLog::new(VerificationPolicy::Checksum);
    log.record(&verified(Path::new("a"), 1));
    log.finish();

    let summary = log.summary(true);
    let json = serde_json::to_value(&summary).expect("the summary serializes");

    assert_eq!(json["policy"], serde_json::json!("checksum"));
    assert_eq!(json["method"], serde_json::json!("size_and_checksum"));
    assert_eq!(json["checksumAlgorithm"], serde_json::json!("sha256"));
    assert_eq!(json["status"], serde_json::json!("verified"));
    assert_eq!(
        json["coverage"]["modifiedTimePreserved"],
        serde_json::json!(false)
    );
    // The frontend contract requires this field: a summary that crosses the
    // wire without it is rejected by the queue and the transfer never starts.
    assert_eq!(
        json["verdict"],
        serde_json::json!(summary.verdict),
        "the verdict line travels with the summary, not just its raw fields"
    );
    assert!(
        summary.verdict.starts_with("verified ("),
        "the verdict states the method that ran: {}",
        summary.verdict
    );
}

#[test]
fn the_summary_verdict_always_describes_the_real_state() {
    assert!(VerificationSummary::pending(VerificationPolicy::Size)
        .verdict
        .contains("not started"));

    let mut log = VerificationLog::new(VerificationPolicy::Checksum);
    log.set_planned_files(2);
    log.record(&verified(Path::new("a"), 2048));
    log.finish();
    let verdict = log.summary(true).verdict;
    assert!(verdict.contains("verified"), "{verdict}");
    assert!(verdict.contains("size_and_checksum"), "{verdict}");
    assert!(verdict.contains("2048 bytes"), "{verdict}");
}

#[test]
fn verification_of_a_file_larger_than_the_buffer_streams_it() {
    let dir = unique_temp_dir("verify-large");
    let source = dir.join("source.bin");
    let destination = dir.join("destination.bin");
    let bytes: Vec<u8> = (0..=255u8)
        .cycle()
        .take(HASH_BUFFER_BYTES * 3 + 7)
        .collect();
    write(&source, &bytes);
    write(&destination, &bytes);

    let result = verify_item(
        request(
            &source,
            &destination,
            bytes.len() as u64,
            Some(digest_of(&bytes)),
        ),
        VerificationPolicy::Checksum,
        &mut run,
    )
    .expect("verification runs");

    assert_eq!(result.status, VerificationStatus::Verified);
    assert!(bytes.len() > HASH_BUFFER_BYTES);

    let _ = std::fs::remove_dir_all(dir);
}
