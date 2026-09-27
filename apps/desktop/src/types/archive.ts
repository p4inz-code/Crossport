/* ==========================================================================
 * Archive status types
 * Mirrors `LoadState`, `DocumentStatus`, and `ArchiveStatus` in
 * `src-tauri/src/persistence/mod.rs` and `src-tauri/src/archive.rs`.
 *
 * A durable document can load well, be absent, be migrated from an older
 * schema, be set aside because it was unusable, or belong to a newer build. The
 * last two are states the UI must not present as "empty": they mean data exists
 * that this build cannot use, and the backend says so in its own words.
 * ========================================================================== */

import { z } from "zod";

/** How a durable document loaded. */
export const ARCHIVE_LOAD_STATES = [
  "loaded",
  "missing",
  "migrated",
  "recovered",
  "unsupported",
] as const;
export type ArchiveLoadState = (typeof ARCHIVE_LOAD_STATES)[number];

/** How one durable document loaded, with the backend's explanation. */
export const documentStatusSchema = z.object({
  state: z.enum(ARCHIVE_LOAD_STATES),
  detail: z.string().nullable(),
});

export type DocumentStatus = z.infer<typeof documentStatusSchema>;

/** The archive's own health, so a degraded list can be explained. */
export const archiveStatusSchema = z.object({
  history: documentStatusSchema,
  state: documentStatusSchema,
  degraded: z.boolean(),
  writable: z.boolean(),
  historyRecords: z.number().int().nonnegative(),
  historyLimit: z.number().int().nonnegative(),
  interruptedJobs: z.number().int().nonnegative(),
});

export type ArchiveStatus = z.infer<typeof archiveStatusSchema>;

/** Whether a document needs explaining to the user. */
export function isDocumentNoteworthy(status: DocumentStatus): boolean {
  return status.state === "recovered" || status.state === "unsupported";
}

/** One line describing a document's state, without the backend's detail. */
export function archiveLoadStateLabel(state: ArchiveLoadState): string {
  switch (state) {
    case "loaded":
      return "Loaded";
    case "missing":
      return "Not created yet";
    case "migrated":
      return "Migrated from an older version";
    case "recovered":
      return "Unusable and set aside";
    case "unsupported":
      return "Written by a newer version";
  }
}
