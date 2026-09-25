/* ==========================================================================
 * Drives service
 * Drive enumeration is a backend concern: the host is inspected in Rust and
 * the webview only receives the resulting list.
 * ========================================================================== */

import { type DriveInfo, driveListSchema } from "@/types";
import { invokeTyped, requireDesktopRuntime } from "./ipc";

/** Lists the storage roots the user can currently reach. */
export async function listDrives(): Promise<DriveInfo[]> {
  requireDesktopRuntime("list_drives");
  return invokeTyped("list_drives", driveListSchema);
}
