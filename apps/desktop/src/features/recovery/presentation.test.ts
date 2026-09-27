import { describe, expect, it } from "vitest";

import { makeRecoveryCandidate } from "@/test/fixtures";
import {
  actionLabel,
  artifactSummary,
  candidateBadge,
  candidateOutcome,
  candidateProgress,
  candidateProof,
  candidateTitle,
  restartImpactLine,
} from "./presentation";

describe("recovery presentation", () => {
  it("names the operation, the source, and the destination", () => {
    const title = candidateTitle(
      makeRecoveryCandidate({
        operation: "move",
        sources: ["D:\\Photos", "D:\\Videos"],
      }),
    );

    expect(title).toContain("Move");
    expect(title).toContain("D:\\Photos");
    expect(title).toContain("and 1 more");
    expect(title).toContain("D:\\Backup");
  });

  it("reports progress in bytes with the reported percentage", () => {
    expect(candidateProgress(makeRecoveryCandidate())).toBe(
      "1 KB of 4 KB (25%)",
    );
  });

  it("falls back to item counts when no bytes were reported", () => {
    const progress = candidateProgress(
      makeRecoveryCandidate({
        percent: null,
        progress: {
          totalBytes: 0,
          transferredBytes: 0,
          totalFiles: 4,
          completedFiles: 1,
          totalDirectories: 2,
          completedDirectories: 1,
          skippedItems: 0,
          failedItems: 0,
        },
      }),
    );

    expect(progress).toBe("2 of 6 items");
  });

  it("says so when nothing was reported at all", () => {
    const progress = candidateProgress(
      makeRecoveryCandidate({
        percent: null,
        progress: {
          totalBytes: 0,
          transferredBytes: 0,
          totalFiles: 0,
          completedFiles: 0,
          totalDirectories: 0,
          completedDirectories: 0,
          skippedItems: 0,
          failedItems: 0,
        },
      }),
    );

    expect(progress).toContain("No progress was reported");
  });

  it("describes the leftovers a job left behind", () => {
    const summary = artifactSummary(makeRecoveryCandidate());

    expect(summary).toContain("1 unfinished file(s)");
    expect(summary).toContain("1 KB");
    expect(summary).toContain("in 1 directory");
  });

  it("reports a truncated artifact scan instead of implying completeness", () => {
    const summary = artifactSummary(
      makeRecoveryCandidate({ artifactsTruncated: true }),
    );

    expect(summary).toContain("may be more");
  });

  it("says nothing about leftovers when there are none", () => {
    expect(
      artifactSummary(makeRecoveryCandidate({ artifacts: [] })),
    ).toBeNull();
  });

  it("warns when a restart would overwrite existing entries", () => {
    const line = restartImpactLine(
      makeRecoveryCandidate({
        restartImpact: {
          strategy: "replace",
          conflicts: 2,
          skippedItems: 0,
          totalBytes: 4096,
          totalFiles: 2,
          overwrites: true,
        },
      }),
    );

    expect(line).toContain("overwrites 2 existing entries");
    expect(line).toContain("'replace'");
  });

  it("says a restart writes fresh data when nothing collides", () => {
    const line = restartImpactLine(makeRecoveryCandidate());

    expect(line).toContain("Nothing exists at the destination yet");
  });

  it("describes the archive's proof, and its absence, honestly", () => {
    expect(
      candidateProof(makeRecoveryCandidate({ confirmedByArchive: true })),
    ).toContain("archive recorded");

    expect(
      candidateProof(makeRecoveryCandidate({ destinationLooksComplete: true })),
    ).toContain("evidence, not proof");

    expect(candidateProof(makeRecoveryCandidate())).toBeNull();
  });

  it("labels outcomes and actions without a fallthrough", () => {
    expect(candidateOutcome(makeRecoveryCandidate())).toBe(
      "Interrupted — can be run again",
    );
    expect(actionLabel("restart")).toContain("Restart");
    expect(actionLabel("discard")).toContain("Discard");
    expect(actionLabel("confirm")).toContain("recovered");
  });

  it("says whether a candidate can be restarted", () => {
    expect(candidateBadge(makeRecoveryCandidate())).toBe("Can restart");
    expect(candidateBadge(makeRecoveryCandidate({ canRestart: false }))).toBe(
      "Cannot restart",
    );
  });
});
