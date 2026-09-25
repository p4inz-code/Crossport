/* ==========================================================================
 * Settings service
 * The single frontend entry point for settings. Inside Tauri it invokes the
 * backend commands (which own persistence); in a plain browser it falls back
 * to validated localStorage so `pnpm dev` stays usable. Failures are always
 * `IpcError`s with a stable code.
 * ========================================================================== */

import { type AppSettings, settingsSchema } from "@/types";
import { IpcError, invokeCommand, invokeTyped, isDesktopRuntime } from "./ipc";
import { readSettingsFromLocal, writeSettingsToLocal } from "./storage";

/** Loads the current settings. */
export async function getSettings(): Promise<AppSettings> {
  if (!isDesktopRuntime()) {
    return readSettingsFromLocal();
  }
  return invokeTyped("get_settings", settingsSchema);
}

/** Validates and persists updated settings. */
export async function updateSettings(settings: AppSettings): Promise<void> {
  const validated = settingsSchema.safeParse(settings);
  if (!validated.success) {
    throw new IpcError(
      "invalid_input",
      `Settings are invalid: ${validated.error.issues
        .map((issue) => `${issue.path.join(".")} ${issue.message}`)
        .join("; ")}`,
    );
  }

  if (!isDesktopRuntime()) {
    writeSettingsToLocal(validated.data);
    return;
  }

  await invokeCommand("update_settings", { settings: validated.data });
}
