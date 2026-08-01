/* ==========================================================================
 * Application-wide constants
 * ========================================================================== */

import packageJson from "../../package.json";

export const APP_NAME = "CrossPort";

/** Version sourced from package.json so it can never drift from the build. */
export const APP_VERSION = packageJson.version;

/** Storage keys for the settings persistence foundation. */
export const STORAGE_KEYS = {
  settings: "crossport:settings",
} as const;
