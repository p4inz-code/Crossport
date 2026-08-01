/* ==========================================================================
 * Transfer store — initial state only
 * Placeholder state for active transfers (implemented in a later phase).
 * ========================================================================== */

import { create } from "zustand";

import type { Transfer } from "@/types";

interface TransferState {
  transfers: Transfer[];
  /** Identifier of the transfer currently shown as active, if any. */
  activeTransferId: string | null;
}

export const useTransferStore = create<TransferState>(() => ({
  transfers: [],
  activeTransferId: null,
}));
