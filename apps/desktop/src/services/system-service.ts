/* ==========================================================================
 * System service
 * Platform facts reported by the backend platform abstraction.
 * ========================================================================== */

import { type SystemInfo, systemInfoSchema } from "@/types";
import { invokeTyped, requireDesktopRuntime } from "./ipc";

/** Reads the platform the Rust backend is running on. */
export async function getSystemInfo(): Promise<SystemInfo> {
  requireDesktopRuntime("get_system_info");
  return invokeTyped("get_system_info", systemInfoSchema);
}
