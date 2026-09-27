/* ==========================================================================
 * Shared TypeScript types — public barrel
 * Every schema and type mirrors a Rust payload; only the names consumed
 * outside this module are re-exported here.
 * ========================================================================== */

export type {
  ArchiveLoadState,
  ArchiveStatus,
  DocumentStatus,
} from "./archive";
export {
  ARCHIVE_LOAD_STATES,
  archiveLoadStateLabel,
  archiveStatusSchema,
  documentStatusSchema,
  isDocumentNoteworthy,
} from "./archive";
export type { DriveInfo, VolumeKind } from "./drives";
export { driveListSchema, VOLUME_KINDS } from "./drives";
export type {
  DirectoryEntry,
  DirectoryListing,
  EntryKind,
  EntryMetadata,
} from "./filesystem";
export {
  directoryEntrySchema,
  directoryListingSchema,
  ENTRY_KINDS,
  entryMetadataSchema,
  pickedDirectorySchema,
} from "./filesystem";
export type {
  HistoryFilter,
  HistoryIssue,
  HistoryListing,
  HistoryRecord,
  HistoryStatus,
  HistoryVerification,
  RecoveryAction,
} from "./history";
export {
  HISTORY_FILTERS,
  HISTORY_STATUSES,
  historyFilterLabel,
  historyFilterSchema,
  historyIssueSchema,
  historyListingSchema,
  historyRecordSchema,
  historyStatusLabel,
  historyStatusSchema,
  historyStatusSucceeded,
  historyVerificationSchema,
  RECOVERY_ACTIONS,
  recoveryActionSchema,
} from "./history";
export type {
  PartialArtifact,
  PersistedProgress,
  RecoveryCandidate,
  RecoveryListing,
  RecoveryOutcome,
  RecoveryReport,
  RestartImpact,
} from "./recovery";
export {
  isRecoverable,
  needsDecision,
  partialArtifactSchema,
  persistedProgressSchema,
  RECOVERY_OUTCOMES,
  recoveryCandidateSchema,
  recoveryListingSchema,
  recoveryOutcomeLabel,
  recoveryReportSchema,
  restartImpactSchema,
} from "./recovery";
export type { AppSettings, ThemeMode } from "./settings";
export {
  DEFAULT_HISTORY_LIMIT,
  DEFAULT_SETTINGS,
  MAX_HISTORY_LIMIT,
  MIN_HISTORY_LIMIT,
  settingsSchema,
  THEME_MODES,
} from "./settings";
export type { LoadStatus, Platform, SaveStatus, SystemInfo } from "./system";
export { systemInfoSchema } from "./system";
export type {
  ConflictStrategy,
  ItemKind,
  TransferActivity,
  TransferError,
  TransferIssue,
  TransferIssueReason,
  TransferOperation,
  TransferPreview,
  TransferPreviewRoot,
  TransferProgress,
  TransferRequest,
  TransferSnapshot,
  TransferStatus,
} from "./transfer";
export {
  CONFLICT_STRATEGIES,
  conflictStrategySchema,
  DEFAULT_CONFLICT_STRATEGY,
  hasStarted,
  ITEM_KINDS,
  isLiveTransferStatus,
  isTerminalTransferStatus,
  TRANSFER_ACTIVITIES,
  TRANSFER_ISSUE_REASONS,
  TRANSFER_OPERATIONS,
  TRANSFER_STATUSES,
  transferActivitySchema,
  transferErrorSchema,
  transferIssueReasonSchema,
  transferIssueSchema,
  transferOperationSchema,
  transferPreviewRootSchema,
  transferPreviewSchema,
  transferProgressSchema,
  transferRequestSchema,
  transferSnapshotSchema,
} from "./transfer";
export type {
  ChecksumAlgorithm,
  FileVerification,
  VerificationCoverage,
  VerificationMethod,
  VerificationMismatch,
  VerificationMismatchReason,
  VerificationPolicy,
  VerificationStatus,
  VerificationSummary,
} from "./verification";
export {
  CHECKSUM_ALGORITHMS,
  checksumAlgorithmSchema,
  DEFAULT_VERIFICATION_POLICY,
  fileVerificationSchema,
  isVerificationFailure,
  isVerificationProven,
  VERIFICATION_METHODS,
  VERIFICATION_MISMATCH_REASONS,
  VERIFICATION_POLICIES,
  VERIFICATION_STATUSES,
  verificationCoverageSchema,
  verificationIsComplete,
  verificationMethodSchema,
  verificationMismatchSchema,
  verificationPolicyLabel,
  verificationPolicySchema,
  verificationStatusLabel,
  verificationStatusSchema,
  verificationSummarySchema,
} from "./verification";
