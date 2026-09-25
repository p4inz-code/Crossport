/* ==========================================================================
 * Platform and system types
 * ========================================================================== */

import { z } from "zod";

/** Platforms the Rust `Platform` enum can report. */
export const PLATFORMS = ["windows", "macos", "linux", "other"] as const;
export type Platform = (typeof PLATFORMS)[number];

/** Payload of the `get_system_info` command. */
export const systemInfoSchema = z.object({
  platform: z.enum(PLATFORMS),
  os: z.string().min(1),
  arch: z.string().min(1),
  family: z.string().min(1),
});

export type SystemInfo = z.infer<typeof systemInfoSchema>;

/** Async state of anything loaded over IPC. */
export type LoadStatus = "idle" | "loading" | "ready" | "error";

/** State of a settings write. */
export type SaveStatus = "idle" | "saving" | "saved" | "error";
