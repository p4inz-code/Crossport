import { describe, expect, it } from "vitest";

import { makeVerificationSummary } from "@/test/fixtures";
import {
  isVerificationFailure,
  isVerificationProven,
  VERIFICATION_POLICIES,
  VERIFICATION_STATUSES,
  verificationIsComplete,
  verificationPolicyLabel,
  verificationStatusLabel,
  verificationSummarySchema,
} from "./verification";

describe("verification contracts", () => {
  it("accepts the summary the backend produces", () => {
    expect(
      verificationSummarySchema.safeParse(makeVerificationSummary()).success,
    ).toBe(true);
  });

  it("rejects a status the backend does not publish", () => {
    expect(
      verificationSummarySchema.safeParse(
        makeVerificationSummary({ status: "maybe" as never }),
      ).success,
    ).toBe(false);
  });

  it("keeps the coverage claims explicit", () => {
    const summary = makeVerificationSummary({
      policy: "checksum",
      method: "size_and_checksum",
      checksumAlgorithm: "sha256",
      coverage: {
        size: true,
        structure: true,
        checksum: true,
        modifiedTimePreserved: false,
        readonlyPreserved: false,
      },
    });

    const parsed = verificationSummarySchema.parse(summary);

    expect(parsed.coverage.checksum).toBe(true);
    expect(parsed.coverage.modifiedTimePreserved).toBe(false);
    expect(parsed.coverage.readonlyPreserved).toBe(false);
  });

  it("never calls a skipped verification a success or a failure", () => {
    expect(isVerificationProven("skipped")).toBe(false);
    expect(isVerificationFailure("skipped")).toBe(false);
  });

  it("treats mismatch and failure as failures and nothing else", () => {
    for (const status of VERIFICATION_STATUSES) {
      const expected = status === "mismatch" || status === "failed";
      expect(isVerificationFailure(status)).toBe(expected);
    }
  });

  it("does not call a job verified when files went unchecked", () => {
    const partial = makeVerificationSummary({
      status: "verified",
      plannedFiles: 4,
      checkedFiles: 2,
      verifiedFiles: 2,
    });

    expect(verificationIsComplete(partial)).toBe(false);
  });

  it("calls a job verified only when the plan was covered", () => {
    const complete = makeVerificationSummary({
      status: "verified",
      plannedFiles: 2,
      checkedFiles: 2,
      verifiedFiles: 2,
    });

    expect(verificationIsComplete(complete)).toBe(true);
  });

  it("labels every policy and status without a fallthrough", () => {
    for (const policy of VERIFICATION_POLICIES) {
      expect(verificationPolicyLabel(policy).length).toBeGreaterThan(0);
    }
    for (const status of VERIFICATION_STATUSES) {
      expect(verificationStatusLabel(status).length).toBeGreaterThan(0);
    }
  });
});
