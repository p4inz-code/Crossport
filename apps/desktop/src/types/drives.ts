/* ==========================================================================
 * Drive and volume types
 * Mirrors `DriveInfo` in `src-tauri/src/platform/volume.rs`. Every optional
 * field is optional because the backend only reports what the platform can
 * actually answer for that volume.
 * ========================================================================== */

import { z } from "zod";

/** Volume kinds the Rust `VolumeKind` enum can report. */
export const VOLUME_KINDS = [
  "fixed",
  "removable",
  "network",
  "optical",
  "ram",
  "unknown",
] as const;
export type VolumeKind = (typeof VOLUME_KINDS)[number];

/** One storage volume the user can browse. */
export const driveInfoSchema = z.object({
  /** Stable identifier derived from the root (`C:`, `/mnt/usb`). */
  id: z.string().min(1),
  /** Absolute mount root, e.g. `C:\`. */
  root: z.string().min(1),
  /** Display label: volume name when reported, otherwise the root label. */
  label: z.string().min(1),
  /** Raw volume name, or `null` when the volume is unnamed. */
  name: z.string().min(1).nullable(),
  kind: z.enum(VOLUME_KINDS),
  /** Filesystem type (`NTFS`, `APFS`, …), or `null` when unreported. */
  filesystem: z.string().min(1).nullable(),
  totalBytes: z.number().int().nonnegative().nullable(),
  freeBytes: z.number().int().nonnegative().nullable(),
  usedBytes: z.number().int().nonnegative().nullable(),
  readonly: z.boolean().nullable(),
  /** False when the root exists but its media is not reachable. */
  mounted: z.boolean(),
});

export type DriveInfo = z.infer<typeof driveInfoSchema>;

/** Payload of the `list_drives` command. */
export const driveListSchema = z.array(driveInfoSchema);
