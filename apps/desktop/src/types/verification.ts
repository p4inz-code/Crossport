/* ==========================================================================
 * Verification types
 * Mirrors the verification domain in `src-tauri/src/verification/mod.rs`: the
 * policy a transfer is verified under, the policy's method, the per-file
 * result, and the job-level summary the queue and history both carry.
 *
 * The contract deliberately states what was *not* checked: `coverage` names
 * each claim (size, structure, checksum) and reports metadata preservation as
 * false, because this engine does not reapply modified times or read-only
 * attributes. Nothing here infers a claim from a status.
 * ========================================================================== */

import { z } from "zod";

/** How thoroughly a transfer verifies what it wrote. */
export const VERIFICATION_POLICIES = ["none", "size", "checksum"] as const;
export type VerificationPolicy = (typeof VERIFICATION_POLICIES)[number];

/** The policy used when a request does not name one. */
export const DEFAULT_VERIFICATION_POLICY: VerificationPolicy = "size";

/** What a policy actually does when it runs. */
export const VERIFICATION_METHODS = [
  "none",
  "size",
  "size_and_checksum",
] as const;
export type VerificationMethod = (typeof VERIFICATION_METHODS)[number];

/** How one verification (or a whole job's) ended up. */
export const VERIFICATION_STATUSES = [
  "pending",
  "verifying",
  "verified",
  "mismatch",
  "failed",
  "skipped",
] as const;
export type VerificationStatus = (typeof VERIFICATION_STATUSES)[number];

/** Why a verification did not match. */
export const VERIFICATION_MISMATCH_REASONS = [
  "size_mismatch",
  "checksum_mismatch",
  "destination_missing",
  "destination_not_a_file",
] as const;
export type VerificationMismatchReason =
  (typeof VERIFICATION_MISMATCH_REASONS)[number];

/** Checksum algorithms the backend can compute. */
export const CHECKSUM_ALGORITHMS = ["sha256"] as const;
export type ChecksumAlgorithm = (typeof CHECKSUM_ALGORITHMS)[number];

/** Named schemas, so other type modules reuse one definition per closed set. */
export const verificationPolicySchema = z.enum(VERIFICATION_POLICIES);
export const verificationMethodSchema = z.enum(VERIFICATION_METHODS);
export const verificationStatusSchema = z.enum(VERIFICATION_STATUSES);
export const verificationMismatchReasonSchema = z.enum(
  VERIFICATION_MISMATCH_REASONS,
);
export const checksumAlgorithmSchema = z.enum(CHECKSUM_ALGORITHMS);

/** A structured failure attached to a verification result. */
export const verificationErrorSchema = z.object({
  code: z.string().min(1),
  message: z.string(),
});

export type VerificationError = z.infer<typeof verificationErrorSchema>;

/** One provable discrepancy, with both sides of it. */
export const verificationMismatchSchema = z.object({
  reason: verificationMismatchReasonSchema,
  /** Destination path the mismatch is about. */
  path: z.string().min(1),
  /** What the transfer promised, rendered by the backend. */
  expected: z.string(),
  /** What the destination actually holds. */
  actual: z.string(),
  detail: z.string(),
});

export type VerificationMismatch = z.infer<typeof verificationMismatchSchema>;

/** What a job's verification covered. Claims, not implications. */
export const verificationCoverageSchema = z.object({
  size: z.boolean(),
  structure: z.boolean(),
  checksum: z.boolean(),
  /** Always false: this engine does not reapply modified times. */
  modifiedTimePreserved: z.boolean(),
  /** Always false: this engine does not reapply the read-only attribute. */
  readonlyPreserved: z.boolean(),
});

export type VerificationCoverage = z.infer<typeof verificationCoverageSchema>;

/** The result of verifying one copied file. */
export const fileVerificationSchema = z.object({
  path: z.string().min(1),
  method: verificationMethodSchema,
  status: verificationStatusSchema,
  /** Size the plan measured, when there is one to compare against. */
  expectedBytes: z.number().int().nonnegative().nullable(),
  /** Size the destination reported, when it could be read. */
  actualBytes: z.number().int().nonnegative().nullable(),
  checksumAlgorithm: checksumAlgorithmSchema.nullable(),
  /** Digest of the bytes that came from the source. */
  expectedChecksum: z.string().nullable(),
  /** Digest of the file on disk. */
  actualChecksum: z.string().nullable(),
  mismatch: verificationMismatchSchema.nullable(),
  error: verificationErrorSchema.nullable(),
  durationMs: z.number().int().nonnegative(),
});

export type FileVerification = z.infer<typeof fileVerificationSchema>;

/** The verification verdict for a whole job. */
export const verificationSummarySchema = z.object({
  status: verificationStatusSchema,
  policy: verificationPolicySchema,
  method: verificationMethodSchema,
  checksumAlgorithm: checksumAlgorithmSchema.nullable(),
  plannedFiles: z.number().int().nonnegative(),
  checkedFiles: z.number().int().nonnegative(),
  verifiedFiles: z.number().int().nonnegative(),
  mismatchedFiles: z.number().int().nonnegative(),
  failedFiles: z.number().int().nonnegative(),
  skippedFiles: z.number().int().nonnegative(),
  /** Planned files with no result recorded; never implied to be checked. */
  unverifiedFiles: z.number().int().nonnegative(),
  verifiedBytes: z.number().int().nonnegative(),
  durationMs: z.number().int().nonnegative(),
  coverage: verificationCoverageSchema,
  /** Bounded list of discrepancies, newest last. */
  mismatches: z.array(verificationMismatchSchema),
  mismatchesTruncated: z.boolean(),
  error: verificationErrorSchema.nullable(),
  /** One line the backend rendered, so the UI never reinterprets the fields. */
  verdict: z.string(),
});

export type VerificationSummary = z.infer<typeof verificationSummarySchema>;

/** Whether a verification status means the data was not proven to arrive. */
export function isVerificationFailure(status: VerificationStatus): boolean {
  return status === "mismatch" || status === "failed";
}

/** Whether a verification status means the data was proven to arrive. */
export function isVerificationProven(status: VerificationStatus): boolean {
  return status === "verified";
}

/** Short label for a verification status, for badges and notifications. */
export function verificationStatusLabel(status: VerificationStatus): string {
  switch (status) {
    case "pending":
      return "Verification pending";
    case "verifying":
      return "Verifying";
    case "verified":
      return "Verified";
    case "mismatch":
      return "Verification mismatch";
    case "failed":
      return "Verification failed";
    case "skipped":
      return "Not verified";
  }
}

/** Short label for a verification policy. */
export function verificationPolicyLabel(policy: VerificationPolicy): string {
  switch (policy) {
    case "none":
      return "No verification";
    case "size":
      return "Size verification";
    case "checksum":
      return "SHA-256 verification";
  }
}

/**
 * Whether a summary covers everything the plan intended to verify.
 *
 * A job that checked fewer files than it planned is not a job whose data was
 * proven, and the UI says so rather than showing a green badge.
 */
export function verificationIsComplete(summary: VerificationSummary): boolean {
  return (
    summary.plannedFiles > 0 &&
    summary.checkedFiles + summary.skippedFiles >= summary.plannedFiles
  );
}
