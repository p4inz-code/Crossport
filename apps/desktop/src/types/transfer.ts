/* ==========================================================================
 * Transfer domain types
 * Placeholder surface for the transfer feature (implemented in a later phase).
 * ========================================================================== */

export type TransferDirection = "copy" | "move";

export type TransferStatus =
  | "queued"
  | "running"
  | "paused"
  | "completed"
  | "failed"
  | "cancelled";

export interface Transfer {
  id: string;
  sourcePath: string;
  destinationPath: string;
  direction: TransferDirection;
  status: TransferStatus;
  /** Total bytes to transfer, when known. */
  totalBytes: number | null;
  /** Bytes transferred so far. */
  transferredBytes: number;
  startedAt: string | null;
  completedAt: string | null;
}
