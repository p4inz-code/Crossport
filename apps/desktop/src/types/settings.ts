/* ==========================================================================
 * Settings domain types
 * ========================================================================== */

import { z } from "zod";

/** Supported theme modes. 'system' follows the OS preference. */
export const THEME_MODES = ["light", "dark", "system"] as const;
export type ThemeMode = (typeof THEME_MODES)[number];

/** Schema used to validate persisted settings loaded from storage. */
export const settingsSchema = z.object({
  theme: z.enum(THEME_MODES),
  locale: z.string().min(2).max(16),
});

/** Fully validated application settings. */
export type AppSettings = z.infer<typeof settingsSchema>;

/** Fallback settings used before hydration or when storage is empty. */
export const DEFAULT_SETTINGS: AppSettings = {
  theme: "system",
  locale: "en",
};
