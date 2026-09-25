/* ==========================================================================
 * Filesystem service
 * Path inspection and the native folder picker. Both keep filesystem access
 * in Rust: the frontend never touches a path itself and holds no filesystem
 * capability.
 * ========================================================================== */

import {
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
 * Opens the native folder picker.
 *
 * Resolves with the validated absolute path, or `null` when the user
 * cancelled the dialog.
 */
export async function pickDirectory(): Promise<string | null> {
  requireDesktopRuntime("pick_directory");
  return invokeTyped("pick_directory", pickedDirectorySchema);
}
