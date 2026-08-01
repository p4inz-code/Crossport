/* ==========================================================================
 * Drives domain types
 * Placeholder surface for the drives feature (implemented in a later phase).
 * ========================================================================== */

export type DriveKind =
  | "fixed"
  | "removable"
  | "network"
  | "optical"
  | "ram"
  | "unknown";

export type DriveStatus = "connected" | "disconnected" | "error";

export interface Drive {
  id: string;
  name: string;
  mountPoint: string;
  kind: DriveKind;
  status: DriveStatus;
  /** Total capacity in bytes, when known. */
  totalBytes: number | null;
  /** Free space in bytes, when known. */
  freeBytes: number | null;
  filesystem: string | null;
}
