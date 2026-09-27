import { describe, expect, it } from "vitest";

import {
  DEFAULT_HISTORY_LIMIT,
  DEFAULT_SETTINGS,
  MAX_HISTORY_LIMIT,
  MIN_HISTORY_LIMIT,
  settingsSchema,
} from "./settings";

/** A settings document with every required field, for focused overrides. */
function settings(overrides: Record<string, unknown> = {}) {
  return { ...DEFAULT_SETTINGS, ...overrides };
}

describe("settingsSchema", () => {
  it("accepts valid settings", () => {
    expect(
      settingsSchema.safeParse(settings({ theme: "dark", locale: "en-US" }))
        .success,
    ).toBe(true);
  });

  it("rejects unknown themes", () => {
    expect(settingsSchema.safeParse(settings({ theme: "neon" })).success).toBe(
      false,
    );
  });

  it("rejects a locale shorter than two characters", () => {
    expect(settingsSchema.safeParse(settings({ locale: "x" })).success).toBe(
      false,
    );
  });

  it("rejects a locale longer than sixteen characters", () => {
    expect(
      settingsSchema.safeParse(settings({ locale: "x".repeat(17) })).success,
    ).toBe(false);
  });

  it("accepts every exported theme mode", () => {
    for (const theme of ["light", "dark", "system"] as const) {
      expect(settingsSchema.safeParse(settings({ theme })).success).toBe(true);
    }
  });

  it("accepts every verification policy the backend publishes", () => {
    for (const verification of ["none", "size", "checksum"] as const) {
      expect(settingsSchema.safeParse(settings({ verification })).success).toBe(
        true,
      );
    }
  });

  it("rejects a verification policy the backend does not know", () => {
    expect(
      settingsSchema.safeParse(settings({ verification: "paranoid" })).success,
    ).toBe(false);
  });

  it("rejects a history limit outside the backend's range", () => {
    expect(
      settingsSchema.safeParse(
        settings({ historyLimit: MIN_HISTORY_LIMIT - 1 }),
      ).success,
    ).toBe(false);
    expect(
      settingsSchema.safeParse(
        settings({ historyLimit: MAX_HISTORY_LIMIT + 1 }),
      ).success,
    ).toBe(false);
    expect(
      settingsSchema.safeParse(settings({ historyLimit: 0 })).success,
    ).toBe(false);
  });

  it("requires the phase four fields rather than defaulting them silently", () => {
    const { verification: _verification, ...withoutVerification } = settings();
    expect(settingsSchema.safeParse(withoutVerification).success).toBe(false);
  });

  it("treats the defaults as valid", () => {
    expect(settingsSchema.safeParse(DEFAULT_SETTINGS).success).toBe(true);
    expect(DEFAULT_SETTINGS.historyLimit).toBe(DEFAULT_HISTORY_LIMIT);
    expect(DEFAULT_SETTINGS.verification).toBe("size");
  });
});
