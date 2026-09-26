/* ==========================================================================
 * Filesystem service
 * Path inspection, directory listing, and the native folder picker. All three
 * keep filesystem access in Rust: the frontend never touches a path itself and
 * holds no filesystem capability.
 * ========================================================================== */

import {
  type DirectoryListing,
  directoryListingSchema,
  type EntryMetadata,
  entryMetadataSchema,
  pickedDirectorySchema,
} from "@/types";
import { invokeTyped, requireDesktopRuntime } from "./ipc";

/**
 * Normalizes and describes a path.
 *
 * Rejects with an `IpcError`: `invalid_input` when the path is malformed
 * (relative, empty, escapes its root) and `path_not_found` when it does not
 * exist.
 */
export async function inspectPath(path: string): Promise<EntryMetadata> {
  requireDesktopRuntime("inspect_path");
  return invokeTyped("inspect_path", entryMetadataSchema, { path });
}

/**
 * Lists one directory.
 *
 * The backend validates the path before reading anything, so a rejection is
 * always a structured `IpcError`: `invalid_input` for a malformed path,
 * `path_not_found` when the directory is gone (an unmounted or disconnected
 * volume lands here), `path_not_directory` when the path is a file, and
 * `permission_denied` when the OS refuses access.
 */
export async function listDirectory(path: string): Promise<DirectoryListing> {
  requireDesktopRuntime("list_directory");
  return invokeTyped("list_directory", directoryListingSchema, { path });
}

/**
 * Opens the native folder picker.
 *
 * Resolves with the validated absolute path, or `null` when the user
 * cancelled the dialog.
 */
export async function pickDirectory(): Promise<string | null> {
  requireDesktopRuntime("pick_directory");
  return invokeTyped("pick_directory", pickedDirectorySchema);
}
