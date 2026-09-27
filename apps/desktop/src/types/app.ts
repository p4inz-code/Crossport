/* ==========================================================================
 * Application lifecycle types
 * Mirrors `CloseWarning` in `src-tauri/src/commands/app.rs`: what closing the
 * window right now would set aside, in the backend's own words.
 *
 * The window close is held by Rust while work is in flight, so this payload is
 * the only thing that turns a close request into a decision the user can make.
 * ========================================================================== */

import { z } from "zod";

/** What closing the application would interrupt. */
export const closeWarningSchema = z.object({
  /** Jobs that have not reached a terminal state. */
  liveJobs: z.number().int().nonnegative(),
  /** Interrupted jobs still waiting for a decision. */
  interruptedJobs: z.number().int().nonnegative(),
  /** One line stating the consequence. */
  detail: z.string().min(1),
});

export type CloseWarning = z.infer<typeof closeWarningSchema>;

/** Whether a close request is worth asking about at all. */
export function closeNeedsConfirmation(warning: CloseWarning): boolean {
  return warning.liveJobs > 0 || warning.interruptedJobs > 0;
}
