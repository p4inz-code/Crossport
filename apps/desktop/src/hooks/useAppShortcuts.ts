/* ==========================================================================
 * useAppShortcuts hook
 * The application's keyboard journeys: Ctrl/Cmd + a digit moves between the
 * routes in the navigation table.
 *
 * Deliberately small. Anything that changes what is on disk — starting a
 * transfer, cancelling one, clearing history — is a button the user presses,
 * never a keystroke that can be hit by accident. The hook also stays out of the
 * way while the user is typing, so a shortcut can never eat a character.
 * ========================================================================== */

import { useEffect } from "react";
import { useNavigate } from "react-router-dom";

import { APP_ROUTES } from "@/lib/navigation";

/** Whether a keystroke belongs to whatever the user is typing in. */
function isTyping(target: EventTarget | null): boolean {
  if (!(target instanceof HTMLElement)) {
    return false;
  }
  if (target.isContentEditable) {
    return true;
  }
  return (
    target.tagName === "INPUT" ||
    target.tagName === "TEXTAREA" ||
    target.tagName === "SELECT"
  );
}

export function useAppShortcuts(): void {
  const navigate = useNavigate();

  useEffect(() => {
    function onKeyDown(event: KeyboardEvent): void {
      // Ctrl+digit on Windows and Linux, Cmd+digit on macOS. Alt and Shift are
      // excluded so the combination stays unambiguous.
      if (!(event.ctrlKey || event.metaKey) || event.altKey || event.shiftKey) {
        return;
      }
      if (isTyping(event.target)) {
        return;
      }

      const route = APP_ROUTES.find((entry) => entry.key === event.key);
      if (route === undefined) {
        return;
      }

      event.preventDefault();
      navigate(route.path);
    }

    document.addEventListener("keydown", onKeyDown);
    return () => document.removeEventListener("keydown", onKeyDown);
  }, [navigate]);
}
