/* ==========================================================================
 * Settings domain types
 * Mirrors `AppSettings` in `src-tauri/src/settings/mod.rs`: appearance, region,
 * the verification policy transfers run under when they do not name one, and
 * how many finished transfers history keeps.
 *
 * The retention range is the backend's: a value outside it is rejected there
 * too, so the UI can never store a bound the archive would refuse to honour.
 * ========================================================================== */

import { z } from "zod";

import { verificationPolicySchema } from "./verification";

/** Supported theme modes. 'system' follows the OS preference. */
export const THEME_MODES = ["light", "dark", "system"] as const;
export type ThemeMode = (typeof THEME_MODES)[number];

/** Smallest retention limit the backend accepts. */
export const MIN_HISTORY_LIMIT = 20;

/** Largest retention limit the backend accepts. */
export const MAX_HISTORY_LIMIT = 2000;

/** How many finished transfers history keeps by default. */
export const DEFAULT_HISTORY_LIMIT = 200;

/** Schema used to validate persisted settings loaded from storage. */
export const settingsSchema = z.object({
  theme: z.enum(THEME_MODES),
  locale: z.string().min(2).max(16),
  verification: verificationPolicySchema,
  historyLimit: z.number().int().min(MIN_HISTORY_LIMIT).max(MAX_HISTORY_LIMIT),
});

/** Fully validated application settings. */
export type AppSettings = z.infer<typeof settingsSchema>;

/** Fallback settings used before hydration or when storage is empty. */
export const DEFAULT_SETTINGS: AppSettings = {
  theme: "system",
  locale: "en",
  verification: "size",
  historyLimit: DEFAULT_HISTORY_LIMIT,
};
