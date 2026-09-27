import { describe, expect, it } from "vitest";

import { IpcError } from "@/services/ipc";
import {
  makeTransferIssue,
  makeTransferProgress,
  makeTransferSnapshot,
  makeVerificationSummary,
} from "@/test/fixtures";
import {
  CONFLICT_STRATEGIES,
  TRANSFER_ISSUE_REASONS,
  TRANSFER_OPERATIONS,
  TRANSFER_STATUSES,
  VERIFICATION_STATUSES,
} from "@/types";
import {
  basename,
  CONFLICT_STRATEGY_DESCRIPTIONS,
  CONFLICT_STRATEGY_LABELS,
  canCancel,
  canPause,
  canResume,
  formatDuration,
  formatEta,
  formatSpeed,
  isFinished,
  isMoving,
  issueError,
  issueLabel,
  isVerifying,
  jobError,
  keyedIssues,
  metadataNote,
  TRANSFER_ISSUE_ICONS,
  TRANSFER_ISSUE_LABELS,
  TRANSFER_OPERATION_ICONS,
  TRANSFER_OPERATION_LABELS,
  TRANSFER_STATUS_ICONS,
  TRANSFER_STATUS_LABELS,
  TRANSFER_STATUS_TONES,
  transferBytesLabel,
  transferCountsLabel,
  transferOutcomeLabel,
  transferStatusLabel,
  transferTitle,
  VERIFICATION_ICONS,
  VERIFICATION_TONES,
  verificationLine,
} from "./presentation";

describe("transfer wording", () => {
  it("has a label, a tone, and an icon for every published status", () => {
    for (const status of TRANSFER_STATUSES) {
      expect(TRANSFER_STATUS_LABELS[status]).toBeTruthy();
      expect(TRANSFER_STATUS_TONES[status]).toBeTruthy();
      expect(TRANSFER_STATUS_ICONS[status]).toBeTruthy();
    }
  });

  it("has a label and an icon for every operation", () => {
    for (const operation of TRANSFER_OPERATIONS) {
      expect(TRANSFER_OPERATION_LABELS[operation]).toBeTruthy();
      expect(TRANSFER_OPERATION_ICONS[operation]).toBeTruthy();
    }
  });

  it("explains every conflict strategy", () => {
    for (const strategy of CONFLICT_STRATEGIES) {
      expect(CONFLICT_STRATEGY_LABELS[strategy]).toBeTruthy();
      expect(CONFLICT_STRATEGY_DESCRIPTIONS[strategy]).toBeTruthy();
    }
  });

  it("has a label and an icon for every issue reason", () => {
    for (const reason of TRANSFER_ISSUE_REASONS) {
      expect(TRANSFER_ISSUE_LABELS[reason]).toBeTruthy();
      expect(TRANSFER_ISSUE_ICONS[reason]).toBeTruthy();
    }
  });
});

describe("job controls", () => {
  it("offers pause, resume, and cancel exactly where the engine accepts them", () => {
    const pause = TRANSFER_STATUSES.filter((status) =>
      canPause(makeTransferSnapshot({ status })),
    );
    const resume = TRANSFER_STATUSES.filter((status) =>
      canResume(makeTransferSnapshot({ status })),
    );
    const cancel = TRANSFER_STATUSES.filter((status) =>
      canCancel(makeTransferSnapshot({ status })),
    );

    expect(pause).toEqual(["queued", "preparing", "running"]);
    expect(resume).toEqual(["paused"]);
    expect(cancel).toEqual(["queued", "preparing", "running", "paused"]);
  });

  it("treats only completed, failed, and cancelled jobs as finished", () => {
    const finished = TRANSFER_STATUSES.filter((status) =>
      isFinished(makeTransferSnapshot({ status })),
    );

    expect(finished).toEqual(["completed", "failed", "cancelled"]);
  });

  it("only calls a job moving while data is actually moving", () => {
    const moving = TRANSFER_STATUSES.filter((status) =>
      isMoving(makeTransferSnapshot({ status })),
    );

    expect(moving).toEqual(["preparing", "running"]);
  });
});

describe("verification wording", () => {
  it("has a tone and an icon for every published verification status", () => {
    for (const status of VERIFICATION_STATUSES) {
      expect(VERIFICATION_TONES[status]).toBeTruthy();
      expect(VERIFICATION_ICONS[status]).toBeTruthy();
    }
  });

  it("calls a job that is checking its output Verifying", () => {
    const checking = makeTransferSnapshot({
      status: "running",
      progress: makeTransferProgress({ activity: "verifying" }),
    });

    expect(isVerifying(checking)).toBe(true);
    expect(transferStatusLabel(checking)).toBe("Verifying");
    expect(transferStatusLabel(makeTransferSnapshot())).toBe("Transferring");
    expect(
      transferStatusLabel(makeTransferSnapshot({ status: "paused" })),
    ).toBe("Paused");
  });

  it("never calls a job verifying unless it is still running", () => {
    for (const status of TRANSFER_STATUSES) {
      expect(
        isVerifying(
          makeTransferSnapshot({
            status,
            progress: makeTransferProgress({ activity: "verifying" }),
          }),
        ),
      ).toBe(status === "running");
    }
  });

  it("states both the policy and the backend's own verdict", () => {
    const summary = makeVerificationSummary({
      status: "verified",
      policy: "checksum",
      method: "size_and_checksum",
      checksumAlgorithm: "sha256",
      verdict: "verified (size_and_checksum, 2 files, 4096 bytes)",
    });

    expect(verificationLine(summary)).toBe(
      "SHA-256 verification — verified (size_and_checksum, 2 files, 4096 bytes)",
    );
  });

  it("says what was not preserved instead of letting a badge imply it", () => {
    expect(metadataNote(makeVerificationSummary())).toBe(
      "Modified times and read-only attributes are not reapplied.",
    );
    expect(
      metadataNote(
        makeVerificationSummary({
          coverage: {
            ...makeVerificationSummary().coverage,
            modifiedTimePreserved: true,
            readonlyPreserved: true,
          },
        }),
      ),
    ).toBeNull();
    expect(
      metadataNote(
        makeVerificationSummary({
          coverage: {
            ...makeVerificationSummary().coverage,
            modifiedTimePreserved: true,
          },
        }),
      ),
    ).toBe("Read-only attributes are not reapplied.");
  });
});

describe("naming a job", () => {
  it("names the first source and counts the rest", () => {
    expect(transferTitle(makeTransferSnapshot())).toBe("Photos");
    expect(
      transferTitle(
        makeTransferSnapshot({
          sources: ["D:\\Photos", "D:\\Music", "D:\\Videos"],
        }),
      ),
    ).toBe("Photos + 2 more");
  });

  it("takes the last segment of either separator", () => {
    expect(basename("D:\\Photos\\trip.jpg")).toBe("trip.jpg");
    expect(basename("/home/user/notes.txt")).toBe("notes.txt");
    expect(basename("D:\\Photos\\")).toBe("Photos");
    expect(basename("C:")).toBe("C:");
  });
});

describe("progress wording", () => {
  it("counts files and folders that the plan actually has", () => {
    expect(transferCountsLabel(makeTransferProgress())).toBe(
      "1 of 2 files · 1 of 1 folders",
    );
    expect(
      transferCountsLabel(
        makeTransferProgress({
          totalFiles: 0,
          completedFiles: 0,
          totalDirectories: 2,
          completedDirectories: 0,
        }),
      ),
    ).toBe("0 of 2 folders");
    expect(
      transferCountsLabel(
        makeTransferProgress({
          totalFiles: 0,
          completedFiles: 0,
          totalDirectories: 0,
          completedDirectories: 0,
        }),
      ),
    ).toBe("No items to move");
  });

  it("says there is no file data instead of claiming zero bytes moved", () => {
    expect(transferBytesLabel(makeTransferProgress())).toBe("1 KB of 4 KB");
    expect(
      transferBytesLabel(
        makeTransferProgress({
          totalBytes: 0,
          transferredBytes: 0,
          percent: null,
        }),
      ),
    ).toBe("No file data");
  });

  it("formats speed and countdowns", () => {
    expect(formatSpeed(1024)).toBe("1 KB/s");
    expect(formatSpeed(0)).toBe("0 B/s");

    expect(formatEta(null)).toBe("Unknown");
    expect(formatEta(45)).toBe("45s left");
    expect(formatEta(60)).toBe("1m left");
    expect(formatEta(80)).toBe("1m 20s left");
    expect(formatEta(3600)).toBe("1h left");
    expect(formatEta(3900)).toBe("1h 5m left");
  });

  it("formats elapsed time", () => {
    expect(formatDuration(0)).toBe("0s");
    expect(formatDuration(45_000)).toBe("45s");
    expect(formatDuration(80_000)).toBe("1m 20s");
    expect(formatDuration(3_600_000)).toBe("1h");
    expect(formatDuration(3_900_000)).toBe("1h 5m");
  });

  it("summarizes what a finished job did not manage", () => {
    expect(transferOutcomeLabel(makeTransferSnapshot())).toBe(
      "No problems reported",
    );
    expect(
      transferOutcomeLabel(
        makeTransferSnapshot({
          progress: makeTransferProgress({ failedItems: 1, skippedItems: 2 }),
        }),
      ),
    ).toBe("1 failed · 2 skipped");
  });
});

describe("issue wording", () => {
  it("uses the backend message for a failure", () => {
    const issue = makeTransferIssue({
      reason: "failed",
      detail: null,
      error: { code: "disk_full", message: "disk full: D:\\" },
    });

    expect(issueLabel(issue)).toBe("disk full: D:\\");
    expect(issueError(issue)).toBeInstanceOf(IpcError);
    expect(issueError(issue)?.code).toBe("disk_full");
  });

  it("keeps an unknown failure code readable", () => {
    const issue = makeTransferIssue({
      reason: "failed",
      detail: null,
      error: { code: "quota_exceeded", message: "the share is full" },
    });

    const error = issueError(issue);

    expect(error?.code).toBe("unknown");
    expect(error?.unsupportedCode).toBe("quota_exceeded");
    expect(issueLabel(issue)).toBe("the share is full");
  });

  it("uses the engine's explanation for skipped and unsupported items", () => {
    expect(
      issueLabel(
        makeTransferIssue({ reason: "skipped", detail: "already there" }),
      ),
    ).toBe("already there");
    expect(
      issueLabel(
        makeTransferIssue({
          reason: "unsupported",
          detail: "symbolic links are not copied",
        }),
      ),
    ).toBe("symbolic links are not copied");
  });

  it("never renders an empty detail", () => {
    expect(
      issueLabel(makeTransferIssue({ reason: "skipped", detail: "" })),
    ).toContain("already");
    expect(
      issueLabel(makeTransferIssue({ reason: "unsupported", detail: null })),
    ).toContain("cannot be copied");
    expect(
      issueLabel(
        makeTransferIssue({ reason: "failed", detail: null, error: null }),
      ),
    ).toContain("could not be transferred");
  });

  it("reads a job-level failure", () => {
    expect(jobError(makeTransferSnapshot())).toBeNull();
    expect(
      jobError(
        makeTransferSnapshot({
          error: { code: "transfer_failed", message: "3 items failed" },
        }),
      )?.code,
    ).toBe("transfer_failed");
  });
});

describe("issue keys", () => {
  it("identifies an issue by its content, not by its row", () => {
    const first = makeTransferIssue({ path: "D:\\a.txt" });
    const second = makeTransferIssue({ path: "D:\\b.txt" });

    const keys = keyedIssues([first, second]).map((entry) => entry.key);
    // The same issues in the reversed order keep their own keys.
    const reversed = keyedIssues([second, first]).map((entry) => entry.key);

    expect(keys[0]).not.toBe(keys[1]);
    expect(reversed[1]).toBe(keys[0]);
    expect(reversed[0]).toBe(keys[1]);
  });

  it("separates identical issues so no two rows share a key", () => {
    const issue = makeTransferIssue({ path: "D:\\same.txt" });

    const keys = keyedIssues([issue, issue, issue]).map((entry) => entry.key);

    expect(new Set(keys).size).toBe(3);
  });

  it("keeps keys stable when later issues arrive", () => {
    const first = makeTransferIssue({ path: "D:\\a.txt" });
    const second = makeTransferIssue({ path: "D:\\b.txt" });

    const before = keyedIssues([first]).map((entry) => entry.key);
    const after = keyedIssues([first, second]).map((entry) => entry.key);

    expect(after[0]).toBe(before[0]);
  });

  it("treats the same path with different reasons as different issues", () => {
    const skipped = makeTransferIssue({ path: "D:\\x", reason: "skipped" });
    const failed = makeTransferIssue({
      path: "D:\\x",
      reason: "failed",
      error: { code: "io_error", message: "read failed" },
    });

    const keys = keyedIssues([skipped, failed]).map((entry) => entry.key);

    expect(keys[0]).not.toBe(keys[1]);
  });
});
