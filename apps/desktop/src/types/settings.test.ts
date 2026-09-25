import { describe, expect, it } from "vitest";

import { DEFAULT_SETTINGS, settingsSchema } from "./settings";

describe("settingsSchema", () => {
  it("accepts valid settings", () => {
    expect(
      settingsSchema.safeParse({ theme: "dark", locale: "en-US" }).success,
    ).toBe(true);
  });

  it("rejects unknown themes", () => {
    expect(
      settingsSchema.safeParse({ theme: "neon", locale: "en" }).success,
    ).toBe(false);
  });

  it("rejects a locale shorter than two characters", () => {
    expect(
      settingsSchema.safeParse({ theme: "light", locale: "x" }).success,
    ).toBe(false);
  });

  it("rejects a locale longer than sixteen characters", () => {
    expect(
      settingsSchema.safeParse({ theme: "light", locale: "x".repeat(17) })
        .success,
    ).toBe(false);
  });

  it("accepts every exported theme mode", () => {
    for (const theme of ["light", "dark", "system"] as const) {
      expect(settingsSchema.safeParse({ theme, locale: "en" }).success).toBe(
        true,
      );
    }
  });

  it("treats the defaults as valid", () => {
    expect(settingsSchema.safeParse(DEFAULT_SETTINGS).success).toBe(true);
  });
});
