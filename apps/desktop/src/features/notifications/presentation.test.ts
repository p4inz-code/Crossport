import { describe, expect, it } from "vitest";

import {
  makeRecoveryCandidate,
  makeTransferSnapshot,
  makeVerificationSummary,
} from "@/test/fixtures";
import {
  notificationForRecovery,
  notificationForTransfer,
} from "./presentation";

describe("transfer notifications", () => {
  it("says nothing while a job is still running", () => {
    expect(
      notificationForTransfer(makeTransferSnapshot({ status: "running" })),
    ).toBeNull();
    expect(
      notificationForTransfer(makeTransferSnapshot({ status: "queued" })),
    ).toBeNull();
  });

  it("reports a completed transfer with its verification verdict", () => {
    const notification = notificationForTransfer(
      makeTransferSnapshot({
        status: "completed",
        verification: makeVerificationSummary({
          status: "verified",
          verdict: "verified (size, 2 files, 4096 bytes)",
        }),
      }),
    );

    expect(notification?.kind).toBe("success");
    expect(notification?.title).toBe("Transfer completed");
    expect(notification?.message).toContain("verified (size");
    expect(notification?.event).toBe("completed");
  });

  it("warns when the conflict strategy left items alone", () => {
    const notification = notificationForTransfer(
      makeTransferSnapshot({
        status: "completed",
        progress: {
          ...makeTransferSnapshot().progress,
          skippedItems: 2,
        },
      }),
    );

    expect(notification?.kind).toBe("warning");
    expect(notification?.title).toContain("skipped");
    expect(notification?.event).toBe("completed-with-skips");
  });

  it("calls a verification mismatch what it is", () => {
    const notification = notificationForTransfer(
      makeTransferSnapshot({
        status: "failed",
        verification: makeVerificationSummary({
          status: "mismatch",
          mismatchedFiles: 1,
          verdict: "1 of 2 files did not verify",
        }),
        error: {
          code: "verification_failed",
          message: "1 of 2 files did not verify",
        },
      }),
    );

    expect(notification?.kind).toBe("error");
    expect(notification?.title).toBe("Verification mismatch");
    expect(notification?.event).toBe("verification-failed");
    expect(notification?.message).toContain("1 of 2 files did not verify");
  });

  it("reports a failure with the backend's own message", () => {
    const notification = notificationForTransfer(
      makeTransferSnapshot({
        status: "failed",
        error: {
          code: "permission_denied",
          message: "permission denied: D:\\Backup\\report.txt",
        },
      }),
    );

    expect(notification?.kind).toBe("error");
    expect(notification?.title).toBe("Transfer failed");
    expect(notification?.message).toContain("permission denied");
    expect(notification?.event).toBe("failed");
  });

  it("reports a cancellation without calling it a failure", () => {
    const notification = notificationForTransfer(
      makeTransferSnapshot({ status: "cancelled" }),
    );

    expect(notification?.kind).toBe("info");
    expect(notification?.title).toBe("Transfer cancelled");
    expect(notification?.event).toBe("cancelled");
  });
});

describe("recovery notifications", () => {
  it("says nothing when nothing was interrupted", () => {
    expect(notificationForRecovery([])).toBeNull();
  });

  it("prompts once for a single interrupted transfer", () => {
    const notification = notificationForRecovery([makeRecoveryCandidate()]);

    expect(notification?.kind).toBe("warning");
    expect(notification?.title).toBe("A transfer was interrupted");
    expect(notification?.message).toContain("can be run again");
    expect(notification?.event).toBe("recovery-required");
  });

  it("counts several interrupted transfers and says what can be done", () => {
    const notification = notificationForRecovery([
      makeRecoveryCandidate(),
      makeRecoveryCandidate({
        id: "transfer-2",
        outcome: "source_missing",
        canRestart: false,
      }),
    ]);

    expect(notification?.title).toBe("2 transfers were interrupted");
    expect(notification?.message).toContain("1 can be run again");
  });

  it("explains that nothing can be rerun when no candidate can", () => {
    const notification = notificationForRecovery([
      makeRecoveryCandidate({
        outcome: "destination_unavailable",
        canRestart: false,
      }),
    ]);

    expect(notification?.message).toContain("cannot be run again");
    expect(notification?.message).toContain("inspected or discarded");
  });
});
