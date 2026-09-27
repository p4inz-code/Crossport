/* ==========================================================================
 * useCloseGuard hook
 * Hears that the window was asked to close while work is in flight, and holds
 * the warning until the user answers it.
 *
 * Rust is what actually prevents the close: this hook only learns that it
 * happened, so a browser (where there is no window event) behaves exactly as
 * before and no decision is ever invented here.
 * ========================================================================== */

import { useEffect, useState } from "react";

import { subscribeToCloseRequests } from "@/services/app-service";
import type { CloseWarning } from "@/types";

export function useCloseGuard(): {
  /** What closing would set aside, or `null` while nothing is being asked. */
  warning: CloseWarning | null;
  /** Clears the question once the user has answered it. */
  clear: () => void;
} {
  const [warning, setWarning] = useState<CloseWarning | null>(null);

  useEffect(() => {
    let stopped = false;
    let unsubscribe: (() => void) | null = null;

    void subscribeToCloseRequests((next) => {
      if (!stopped) {
        setWarning(next);
      }
    })
      .then((stop) => {
        if (stopped) {
          // The component went away while the listener was being registered.
          stop();
          return;
        }
        unsubscribe = stop;
      })
      .catch((error: unknown) => {
        console.warn("[app] close requests could not be observed", error);
      });

    return () => {
      stopped = true;
      unsubscribe?.();
    };
  }, []);

  return { warning, clear: () => setWarning(null) };
}
