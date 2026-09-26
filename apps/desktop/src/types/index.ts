/* ==========================================================================
 * Shared TypeScript types — public barrel
 * Every schema and type mirrors a Rust payload; only the names consumed
 * outside this module are re-exported here.
 * ========================================================================== */

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
export type { AppSettings, ThemeMode } from "./settings";
export { DEFAULT_SETTINGS, settingsSchema, THEME_MODES } from "./settings";
export type { LoadStatus, Platform, SaveStatus, SystemInfo } from "./system";
export { systemInfoSchema } from "./system";
export type {
  ConflictStrategy,
  ItemKind,
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
  DEFAULT_CONFLICT_STRATEGY,
  hasStarted,
  ITEM_KINDS,
  isLiveTransferStatus,
  isTerminalTransferStatus,
  TRANSFER_ISSUE_REASONS,
  TRANSFER_OPERATIONS,
  TRANSFER_STATUSES,
  transferErrorSchema,
  transferIssueSchema,
  transferPreviewRootSchema,
  transferPreviewSchema,
  transferProgressSchema,
  transferRequestSchema,
  transferSnapshotSchema,
} from "./transfer";
