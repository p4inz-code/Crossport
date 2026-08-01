/* ==========================================================================
 * Application-level types
 * ========================================================================== */

/** Runtime platform the frontend is executing in. */
export type AppPlatform = "desktop" | "web";

/** Top-level application status used by the app store and status bar. */
export type AppStatus = "ready" | "busy" | "error";
