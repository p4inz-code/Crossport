/* ==========================================================================
 * Application service
 * The two lifecycle calls: end the application, and hear that a window close
 * was held because work is in flight.
 *
 * The close request comes from Rust, which refuses to end the process until it
 * has been told to. Nothing here decides anything: the frontend renders the
 * decision the user makes.
 * ========================================================================== */

import { listen } from "@tauri-apps/api/event";

import { type CloseWarning, closeWarningSchema } from "@/types";
import { invokeCommand, isDesktopRuntime } from "./ipc";

/** Event Rust emits when a close was requested and is being held. */
export const CLOSE_REQUESTED_EVENT = "app:close-requested";

/**
 * Ends the application now.
 *
 * Only reachable from a confirmation the user made; the backend journals
 * transfer state as it runs, so an exit mid-transfer is recoverable rather
 * than lost.
 */
export async function exitApp(): Promise<void> {
  await invokeCommand<null>("exit_app");
}

/**
 * Listens for held close requests.
 *
 * Outside the desktop app there is no window to close, so this resolves with a
 * no-op unsubscribe and the shell's effect stays identical in both runtimes.
 */
export async function subscribeToCloseRequests(
  handler: (warning: CloseWarning) => void,
): Promise<() => void> {
  if (!isDesktopRuntime()) {
    return () => {};
  }

  return listen<unknown>(CLOSE_REQUESTED_EVENT, (event) => {
    const parsed = closeWarningSchema.safeParse(event.payload);
    if (!parsed.success) {
      console.warn("[app] ignored a close request with an unexpected payload");
      return;
    }
    handler(parsed.data);
  });
}
