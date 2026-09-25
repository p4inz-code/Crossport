/* ==========================================================================
 * Filesystem types
 * Mirrors `EntryMetadata` in `src-tauri/src/filesystem/metadata.rs`.
 * ========================================================================== */

import { z } from "zod";

/** Read-only metadata for one path, reported by `inspect_path`. */
export const entryMetadataSchema = z.object({
  path: z.string().min(1),
  name: z.string().min(1),
  isDir: z.boolean(),
  isFile: z.boolean(),
  isSymlink: z.boolean(),
  sizeBytes: z.number().int().nonnegative(),
  /** Milliseconds since the UNIX epoch; `null` when the OS cannot report it. */
  modifiedMs: z.number().int().nonnegative().nullable(),
  readonly: z.boolean(),
});

export type EntryMetadata = z.infer<typeof entryMetadataSchema>;

/** Payload of the `pick_directory` command: a path, or `null` on cancel. */
export const pickedDirectorySchema = z.string().min(1).nullable();
