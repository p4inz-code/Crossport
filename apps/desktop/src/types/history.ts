/* ==========================================================================
 * History domain types
 * Placeholder surface for the history feature (implemented in a later phase).
 * ========================================================================== */

import type { TransferStatus } from "./transfer";

export interface HistoryEntry {
  id: string;
  sourcePath: string;
  destinationPath: string;
  status: TransferStatus;
  totalBytes: number | null;
  startedAt: string;
  completedAt: string | null;
}
