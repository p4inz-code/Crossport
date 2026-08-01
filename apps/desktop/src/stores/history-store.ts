/* ==========================================================================
 * History store — initial state only
 * Placeholder state for transfer history (implemented in a later phase).
 * ========================================================================== */

import { create } from "zustand";

import type { HistoryEntry } from "@/types";

interface HistoryState {
  entries: HistoryEntry[];
  isLoading: boolean;
}

export const useHistoryStore = create<HistoryState>(() => ({
  entries: [],
  isLoading: false,
}));
