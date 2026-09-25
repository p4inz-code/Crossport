/* ==========================================================================
 * Shared TypeScript types — public barrel
 * Every schema and type mirrors a Rust payload; only the names consumed
 * outside this module are re-exported here.
 * ========================================================================== */

export type { DriveInfo } from "./drives";
export { driveListSchema } from "./drives";
export type { EntryMetadata } from "./filesystem";
export { entryMetadataSchema, pickedDirectorySchema } from "./filesystem";
export type { AppSettings, ThemeMode } from "./settings";
export { DEFAULT_SETTINGS, settingsSchema, THEME_MODES } from "./settings";
export type { LoadStatus, Platform, SaveStatus, SystemInfo } from "./system";
export { systemInfoSchema } from "./system";
