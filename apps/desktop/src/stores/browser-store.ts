/* ==========================================================================
 * Browser store
 * Navigation state for the directory browser: the current location, the
 * history behind it, and the listing the backend returned for it.
 *
 * The frontend never builds a path itself. It asks the backend to list a path
 * (a volume root, an entry path the backend reported, or the parent the
 * backend reported) and adopts the normalized path that comes back, so a
 * location shown in the UI is always one Rust validated.
 *
 * Two guards keep the surface honest:
 * - a request token, so a slow listing can never overwrite a newer location;
 * - a listing is dropped on failure instead of being left on screen, so a
 *   disconnected volume shows an error instead of stale entries.
 * ========================================================================== */

import { create } from "zustand";

import { listAncestors, listDirectory } from "@/services/filesystem-service";
import { type IpcError, toIpcError } from "@/services/ipc";
import type { DirectoryListing, LoadStatus, PathAncestor } from "@/types";

/** How many locations are remembered in each direction. */
const HISTORY_LIMIT = 50;

interface BrowserState {
  /** Absolute directory currently shown, or `null` when nothing is open. */
  location: string | null;
  /** Previous locations, oldest first; the last entry is "back". */
  history: string[];
  /** Locations ahead of the current one, oldest first; the last is "forward". */
  future: string[];
  /**
   * The current location and every directory above it, oldest first, as the
   * backend resolved them. This is what a breadcrumb shows and navigates by;
   * the frontend never assembles a path itself.
   */
  trail: PathAncestor[];
  /** Listing for the current location. Never kept across a navigation. */
  listing: DirectoryListing | null;
  status: LoadStatus;
  error: IpcError | null;

  /** Opens a directory, pushing the current location onto history. */
  open: (path: string) => Promise<void>;
  /** Reloads the current location without touching history. */
  refresh: () => Promise<void>;
  /** Returns to the previous location. No-op when there is none. */
  goBack: () => Promise<void>;
  /** Returns to a location that was navigated back from. */
  goForward: () => Promise<void>;
  /** Opens the parent the backend reported. No-op at a filesystem root. */
  goUp: () => Promise<void>;
  /** Leaves the browser (drive disconnected, or back to the volume list). */
  close: () => void;
}

/** Monotonic token identifying the newest in-flight listing request. */
let activeRequest = 0;

export const useBrowserStore = create<BrowserState>((set, get) => {
  /** Loads `path` and adopts it as the current location. */
  async function load(path: string): Promise<void> {
    const token = activeRequest + 1;
    activeRequest = token;

    try {
      const listing = await listDirectory(path);
      if (token !== activeRequest) {
        // A newer navigation started while this listing was in flight.
        return;
      }
      set({
        location: listing.path,
        listing,
        status: "ready",
        error: null,
      });
    } catch (error) {
      if (token !== activeRequest) {
        return;
      }
      const ipcError = toIpcError(error);
      console.warn("[browser] directory listing failed", ipcError);
      set({ listing: null, status: "error", error: ipcError });
      return;
    }

    await loadTrail(path, token);
  }

  /**
   * Loads the breadcrumb trail for a location the listing already accepted.
   *
   * A trail that cannot be read leaves the crumbs empty rather than failing the
   * browser: the directory itself is open, which is what the user asked for.
   */
  async function loadTrail(path: string, token: number): Promise<void> {
    try {
      const trail = await listAncestors(path);
      if (token !== activeRequest) {
        return;
      }
      set({ trail });
    } catch (error) {
      if (token !== activeRequest) {
        return;
      }
      console.warn("[browser] the breadcrumb trail could not be read", error);
      set({ trail: [] });
    }
  }

  return {
    location: null,
    history: [],
    future: [],
    trail: [],
    listing: null,
    status: "idle",
    error: null,

    open: async (path: string) => {
      const { location, history } = get();
      const nextHistory =
        location === null || location === path
          ? history
          : [...history, location].slice(-HISTORY_LIMIT);

      set({
        location: path,
        history: nextHistory,
        // A fresh navigation is a new branch: whatever was ahead is gone.
        future: [],
        trail: [],
        listing: null,
        status: "loading",
        error: null,
      });
      await load(path);
    },

    refresh: async () => {
      const { location } = get();
      if (location === null) {
        return;
      }

      set({ status: "loading", error: null });
      await load(location);
    },

    goBack: async () => {
      const { history, future, location } = get();
      const previous = history.at(-1);
      if (previous === undefined) {
        return;
      }

      set({
        location: previous,
        history: history.slice(0, -1),
        // Where we came from is now ahead of us, so "forward" can return to it.
        future:
          location === null
            ? future
            : [...future, location].slice(-HISTORY_LIMIT),
        trail: [],
        listing: null,
        status: "loading",
        error: null,
      });
      await load(previous);
    },

    goForward: async () => {
      const { history, future, location } = get();
      const next = future.at(-1);
      if (next === undefined) {
        return;
      }

      set({
        location: next,
        future: future.slice(0, -1),
        history:
          location === null
            ? history
            : [...history, location].slice(-HISTORY_LIMIT),
        trail: [],
        listing: null,
        status: "loading",
        error: null,
      });
      await load(next);
    },

    goUp: async () => {
      const parent = get().listing?.parent ?? null;
      if (parent === null) {
        return;
      }
      await get().open(parent);
    },

    close: () => {
      // Any in-flight listing is abandoned rather than applied to a browser
      // the user has already left.
      activeRequest += 1;
      set({
        location: null,
        history: [],
        future: [],
        trail: [],
        listing: null,
        status: "idle",
        error: null,
      });
    },
  };
});
