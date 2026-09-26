import { describe, expect, it } from "vitest";

import {
  makeTransferIssue,
  makeTransferPreview,
  makeTransferProgress,
  makeTransferSnapshot,
} from "@/test/fixtures";
import {
  CONFLICT_STRATEGIES,
  hasStarted,
  isLiveTransferStatus,
  isTerminalTransferStatus,
  TRANSFER_STATUSES,
  transferIssueSchema,
  transferPreviewSchema,
  transferProgressSchema,
  transferRequestSchema,
  transferSnapshotSchema,
} from "./transfer";

describe("transfer snapshot schema", () => {
  it("accepts the snapshot the backend publishes", () => {
    expect(
      transferSnapshotSchema.safeParse(makeTransferSnapshot()).success,
    ).toBe(true);
  });

  it("accepts a queued job that has not started and has no progress yet", () => {
    const queued = makeTransferSnapshot({
      status: "queued",
      startedAtMs: null,
      progress: makeTransferProgress({
        transferredBytes: 0,
        completedFiles: 0,
        completedDirectories: 0,
        currentFile: null,
        currentFileBytes: 0,
        currentFileTotalBytes: 0,
        percent: 0,
        bytesPerSecond: 0,
        averageBytesPerSecond: 0,
        etaSeconds: null,
        elapsedMs: 0,
      }),
    });

    expect(transferSnapshotSchema.safeParse(queued).success).toBe(true);
  });

  it("accepts a job-level failure and issues with a code this build knows", () => {
    const failed = makeTransferSnapshot({
      status: "failed",
      error: { code: "disk_full", message: "disk full: D:\\" },
      issues: [
        makeTransferIssue({
          path: "D:\\Backup\\Photos\\trip.jpg",
          reason: "failed",
          error: { code: "disk_full", message: "disk full: D:\\" },
          detail: null,
        }),
      ],
    });

    expect(transferSnapshotSchema.safeParse(failed).success).toBe(true);
  });

  it("keeps an unrecognised error code instead of rejecting the payload", () => {
    // Codes are normalized where they are displayed, so a newer backend
    // cannot make an entire snapshot unreadable.
    const snapshot = makeTransferSnapshot({
      error: { code: "quota_exceeded", message: "the share is full" },
    });

    const parsed = transferSnapshotSchema.safeParse(snapshot);

    expect(parsed.success).toBe(true);
  });

  it("rejects statuses, operations, and conflicts outside the published set", () => {
    expect(
      transferSnapshotSchema.safeParse({
        ...makeTransferSnapshot(),
        status: "retrying",
      }).success,
    ).toBe(false);
    expect(
      transferSnapshotSchema.safeParse({
        ...makeTransferSnapshot(),
        operation: "delete",
      }).success,
    ).toBe(false);
    expect(
      transferSnapshotSchema.safeParse({
        ...makeTransferSnapshot(),
        conflict: "overwrite",
      }).success,
    ).toBe(false);
  });

  it("rejects a job without its destination or its sources", () => {
    const { destination: _destination, ...withoutDestination } =
      makeTransferSnapshot();
    expect(transferSnapshotSchema.safeParse(withoutDestination).success).toBe(
      false,
    );
    expect(
      transferSnapshotSchema.safeParse({
        ...makeTransferSnapshot(),
        sources: [],
      }).success,
    ).toBe(false);
  });
});

describe("transfer progress schema", () => {
  it("rejects impossible counters and percentages", () => {
    expect(
      transferProgressSchema.safeParse(
        makeTransferProgress({ transferredBytes: -1 }),
      ).success,
    ).toBe(false);
    expect(
      transferProgressSchema.safeParse(makeTransferProgress({ percent: 101 }))
        .success,
    ).toBe(false);
    expect(
      transferProgressSchema.safeParse(makeTransferProgress({ percent: 25.5 }))
        .success,
    ).toBe(false);
    expect(
      transferProgressSchema.safeParse(
        makeTransferProgress({ totalFiles: 1.5 }),
      ).success,
    ).toBe(false);
  });

  it("accepts an unknown percentage, which is not the same as zero", () => {
    expect(
      transferProgressSchema.safeParse(makeTransferProgress({ percent: null }))
        .success,
    ).toBe(true);
    expect(
      transferProgressSchema.safeParse(
        makeTransferProgress({ etaSeconds: null, currentFile: null }),
      ).success,
    ).toBe(true);
  });
});

describe("transfer request schema", () => {
  const request = {
    sources: ["D:\\Photos"],
    destination: "D:\\Backup",
    operation: "copy",
    conflict: "skip",
  };

  it("accepts the request the UI sends", () => {
    expect(transferRequestSchema.safeParse(request).success).toBe(true);
  });

  it("requires at least one source and a destination", () => {
    expect(
      transferRequestSchema.safeParse({ ...request, sources: [] }).success,
    ).toBe(false);
    expect(
      transferRequestSchema.safeParse({ ...request, sources: [""] }).success,
    ).toBe(false);
    expect(
      transferRequestSchema.safeParse({ ...request, destination: "" }).success,
    ).toBe(false);
  });

  it("accepts every published conflict strategy", () => {
    for (const conflict of CONFLICT_STRATEGIES) {
      expect(
        transferRequestSchema.safeParse({ ...request, conflict }).success,
      ).toBe(true);
    }
    expect(
      transferRequestSchema.safeParse({ ...request, conflict: "ask" }).success,
    ).toBe(false);
  });
});

describe("transfer preview schema", () => {
  it("accepts a plan with unreported free space", () => {
    expect(
      transferPreviewSchema.safeParse(
        makeTransferPreview({ availableBytes: null }),
      ).success,
    ).toBe(true);
  });

  it("accepts a preview root that the conflict strategy would skip", () => {
    const preview = makeTransferPreview({
      conflicts: 1,
      skippedItems: 1,
      skippedBytes: 2048,
      roots: [
        {
          source: "D:\\Photos\\trip.jpg",
          destination: "D:\\Backup\\trip.jpg",
          kind: "file",
          skipped: true,
          files: 0,
          directories: 0,
          bytes: 0,
        },
      ],
    });

    expect(transferPreviewSchema.safeParse(preview).success).toBe(true);
  });

  it("rejects a malformed root", () => {
    expect(
      transferPreviewSchema.safeParse(
        makeTransferPreview({
          roots: [{ source: "D:\\Photos" } as never],
        }),
      ).success,
    ).toBe(false);
  });
});

describe("transfer issue schema", () => {
  it("accepts an unsupported item that carries a human-readable detail", () => {
    expect(
      transferIssueSchema.safeParse(
        makeTransferIssue({
          reason: "unsupported",
          detail: "links are not copied",
        }),
      ).success,
    ).toBe(true);
  });

  it("rejects an issue with an unknown reason", () => {
    expect(
      transferIssueSchema.safeParse({
        ...makeTransferIssue(),
        reason: "ignored",
      }).success,
    ).toBe(false);
  });

  it("rejects an empty path", () => {
    expect(
      transferIssueSchema.safeParse(makeTransferIssue({ path: "" })).success,
    ).toBe(false);
  });
});

describe("status helpers", () => {
  it("splits every published status into live or terminal", () => {
    for (const status of TRANSFER_STATUSES) {
      expect(isTerminalTransferStatus(status)).toBe(
        !isLiveTransferStatus(status),
      );
    }
  });

  it("treats only completed, failed, and cancelled as finished", () => {
    expect(TRANSFER_STATUSES.filter(isTerminalTransferStatus)).toEqual([
      "completed",
      "failed",
      "cancelled",
    ]);
  });

  it("reports whether a job reached a worker", () => {
    expect(hasStarted(makeTransferSnapshot({ startedAtMs: null }))).toBe(false);
    expect(hasStarted(makeTransferSnapshot())).toBe(true);
  });
});
