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
