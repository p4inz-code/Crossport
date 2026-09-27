/* ==========================================================================
 * Filesystem types
 * Mirrors `EntryMetadata` in `src-tauri/src/filesystem/metadata.rs` and
 * `DirectoryListing` in `src-tauri/src/filesystem/directory.rs`.
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

/** Entry kinds `list_directory` reports. Links are never resolved. */
export const ENTRY_KINDS = ["directory", "file", "symlink", "other"] as const;
export type EntryKind = (typeof ENTRY_KINDS)[number];

/** One child of a listed directory. */
export const directoryEntrySchema = z.object({
  name: z.string().min(1),
  /** Absolute path, built by Rust; the frontend never joins paths itself. */
  path: z.string().min(1),
  /** `null` for directories, links, and entries without metadata. */
  sizeBytes: z.number().int().nonnegative().nullable(),
  modifiedMs: z.number().int().nonnegative().nullable(),
  readonly: z.boolean().nullable(),
  kind: z.enum(ENTRY_KINDS),
});

export type DirectoryEntry = z.infer<typeof directoryEntrySchema>;

/** Payload of the `list_directory` command. */
export const directoryListingSchema = z.object({
  /** Absolute, normalized directory that was listed. */
  path: z.string().min(1),
  name: z.string().min(1),
  /** Parent directory, or `null` at a filesystem root. */
  parent: z.string().min(1).nullable(),
  entries: z.array(directoryEntrySchema),
  truncated: z.boolean(),
});

export type DirectoryListing = z.infer<typeof directoryListingSchema>;

/**
 * One step of a breadcrumb trail, from `list_ancestors`.
 *
 * The label is the directory's own name (or a root's own form), and the path is
 * the absolute path Rust resolved — the frontend never assembles one, so these
 * are the only locations a breadcrumb can offer.
 */
export const pathAncestorSchema = z.object({
  path: z.string().min(1),
  label: z.string().min(1),
});

export type PathAncestor = z.infer<typeof pathAncestorSchema>;

export const pathAncestorsSchema = z.array(pathAncestorSchema);

/** Payload of the `pick_directory` command: a path, or `null` on cancel. */
export const pickedDirectorySchema = z.string().min(1).nullable();
