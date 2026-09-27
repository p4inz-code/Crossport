import { describe, expect, it } from "vitest";

import { makeRecoveryCandidate } from "@/test/fixtures";
import {
  isRecoverable,
  needsDecision,
  RECOVERY_OUTCOMES,
  recoveryCandidateSchema,
  recoveryOutcomeLabel,
} from "./recovery";

describe("recovery contracts", () => {
  it("accepts a candidate the backend produced", () => {
    expect(
      recoveryCandidateSchema.safeParse(makeRecoveryCandidate()).success,
    ).toBe(true);
  });

  it("accepts a candidate with no artifacts and no restart impact", () => {
    const parsed = recoveryCandidateSchema.safeParse(
      makeRecoveryCandidate({
        artifacts: [],
        artifactBytes: 0,
        artifactDirectories: [],
        restartImpact: null,
        canRestart: false,
        outcome: "source_missing",
      }),
    );

    expect(parsed.success).toBe(true);
  });

  it("rejects an outcome recovery never reports", () => {
    expect(
      recoveryCandidateSchema.safeParse(
        makeRecoveryCandidate({ outcome: "probably_fine" as never }),
      ).success,
    ).toBe(false);
  });

  it("rejects a percent outside the range the backend reports", () => {
    expect(
      recoveryCandidateSchema.safeParse(makeRecoveryCandidate({ percent: 120 }))
        .success,
    ).toBe(false);
  });

  it("treats a destination that merely looks complete as evidence, not proof", () => {
    const candidate = makeRecoveryCandidate({
      destinationLooksComplete: true,
      confirmedByArchive: false,
      outcome: "restart_required",
    });

    expect(candidate.confirmedByArchive).toBe(false);
    expect(isRecoverable(candidate.outcome)).toBe(true);
  });

  it("only offers a restart for the outcome that allows one", () => {
    expect(isRecoverable("restart_required")).toBe(true);
    for (const outcome of RECOVERY_OUTCOMES) {
      if (outcome !== "restart_required") {
        expect(isRecoverable(outcome)).toBe(false);
      }
    }
  });

  it("treats every outcome but a proven finish as needing a decision", () => {
    expect(needsDecision("completed_before_crash")).toBe(false);
    expect(needsDecision("restart_required")).toBe(true);
    expect(needsDecision("source_missing")).toBe(true);
    expect(needsDecision("destination_unavailable")).toBe(true);
    expect(needsDecision("unsupported")).toBe(true);
  });

  it("explains every outcome without a fallthrough", () => {
    for (const outcome of RECOVERY_OUTCOMES) {
      expect(recoveryOutcomeLabel(outcome).length).toBeGreaterThan(0);
    }
  });
});
