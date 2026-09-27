import { describe, expect, it } from "vitest";

import { makeHistoryRecord } from "@/test/fixtures";
import {
  conflictSummary,
  failureReason,
  historyDetails,
  historyRowSummary,
  historyTone,
  operationLabel,
  verificationFailed,
  verificationLine,
} from "./presentation";

describe("history presentation", () => {
  it("maps each status to the right tone", () => {
    expect(historyTone(makeHistoryRecord({ status: "completed" }))).toBe(
      "success",
    );
    expect(historyTone(makeHistoryRecord({ status: "recovered" }))).toBe(
      "success",
    );
    expect(historyTone(makeHistoryRecord({ status: "interrupted" }))).toBe(
      "warning",
    );
    expect(historyTone(makeHistoryRecord({ status: "failed" }))).toBe("danger");
    expect(historyTone(makeHistoryRecord({ status: "cancelled" }))).toBe(
      "neutral",
    );
  });

  it("labels operations the way the UI says them", () => {
    expect(operationLabel("copy")).toBe("Copy");
    expect(operationLabel("move")).toBe("Move");
  });

  it("summarizes what moved out of what was planned", () => {
    const summary = historyRowSummary(
      makeHistoryRecord({
        totalFiles: 3,
        completedFiles: 2,
        totalBytes: 4096,
        transferredBytes: 2048,
      }),
    );

    expect(summary).toContain("2 of 3 files");
    expect(summary).toContain("2 KB");
    expect(summary).toContain("of 4 KB");
  });

  it("says a record predates verification rather than implying a check", () => {
    expect(verificationLine(null)).toBe("No verification was recorded");
  });

  it("shows a verification verdict with the policy that produced it", () => {
    const line = verificationLine(
      makeHistoryRecord({ verification: null }).verification ?? {
        status: "verified",
        method: "size_and_checksum",
        policy: "checksum",
        checksumAlgorithm: "sha256",
        checkedFiles: 1,
        verifiedFiles: 1,
        mismatchedFiles: 0,
        failedFiles: 0,
        verifiedBytes: 10,
        verdict: "verified (size_and_checksum, 1 files, 10 bytes)",
      },
    );

    expect(line).toContain("SHA-256 verification");
    expect(line).toContain("verified (size_and_checksum");
  });

  it("flags a failed verification from the summary alone", () => {
    const failed = makeHistoryRecord({
      verification: {
        status: "mismatch",
        method: "size_and_checksum",
        policy: "checksum",
        checksumAlgorithm: "sha256",
        checkedFiles: 1,
        verifiedFiles: 0,
        mismatchedFiles: 1,
        failedFiles: 0,
        verifiedBytes: 0,
        verdict: "1 of 1 files did not verify",
      },
    });

    expect(verificationFailed(failed.verification)).toBe(true);
    expect(verificationFailed(makeHistoryRecord().verification)).toBe(false);
    expect(verificationFailed(null)).toBe(false);
  });

  it("prefers the record's structured error over anything generic", () => {
    const reason = failureReason(
      makeHistoryRecord({
        status: "failed",
        error: {
          code: "permission_denied",
          message: "permission denied: D:\\Backup\\report.txt",
        },
      }),
    );

    expect(reason).toEqual({
      code: "permission_denied",
      message: "permission denied: D:\\Backup\\report.txt",
    });
  });

  it("falls back to the first failed item's error when the job has none", () => {
    const reason = failureReason(
      makeHistoryRecord({
        status: "failed",
        error: null,
        issues: [
          {
            path: "D:\\Backup\\report.txt",
            reason: "failed",
            error: {
              code: "io",
              message: "the device is not ready",
            },
            detail: null,
          },
        ],
      }),
    );

    expect(reason?.code).toBe("io");
  });

  it("reports no reason when a failure really has none", () => {
    expect(
      failureReason(makeHistoryRecord({ status: "failed", error: null })),
    ).toBeNull();
  });

  it("describes the conflict outcome only when there was one", () => {
    expect(conflictSummary(makeHistoryRecord())).toBeNull();
    expect(
      conflictSummary(makeHistoryRecord({ skippedItems: 3, failedItems: 1 })),
    ).toContain("3 left alone");
    expect(
      conflictSummary(makeHistoryRecord({ skippedItems: 0, failedItems: 2 })),
    ).toBe("2 failed");
  });

  it("builds a details model that never claims more than the record holds", () => {
    const details = historyDetails(
      makeHistoryRecord({
        status: "interrupted",
        recovery: "restart",
        skippedItems: 1,
        finishedAtMs: 1_700_000_002_500,
      }),
    );

    expect(details.statusLabel).toBe("Interrupted");
    expect(details.tone).toBe("warning");
    expect(details.operation).toBe("Copy");
    expect(details.conflicts).toContain("1 left alone");
    expect(details.recovery).toContain("restarted from the beginning");
    expect(details.failure).toBeNull();
    expect(details.issues).toEqual([]);
  });

  it("explains a discarded episode without pretending it succeeded", () => {
    const details = historyDetails(
      makeHistoryRecord({ status: "interrupted", recovery: "discard" }),
    );

    expect(details.recovery).toContain("discarded");
  });
});
