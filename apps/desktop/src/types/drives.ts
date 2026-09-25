/* ==========================================================================
 * Drive types
 * Mirrors `DriveInfo` in `src-tauri/src/platform/drives.rs`.
 * ========================================================================== */

import { z } from "zod";

/** One storage root the user can browse. */
export const driveInfoSchema = z.object({
  root: z.string().min(1),
  label: z.string().min(1),
});

export type DriveInfo = z.infer<typeof driveInfoSchema>;

/** Payload of the `list_drives` command. */
export const driveListSchema = z.array(driveInfoSchema);
