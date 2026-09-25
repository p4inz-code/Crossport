import { describe, expect, it } from "vitest";

import { formatBytes, formatDateTime } from "./format";

describe("formatBytes", () => {
  it("prints raw bytes below one kilobyte", () => {
    expect(formatBytes(0)).toBe("0 B");
    expect(formatBytes(512)).toBe("512 B");
    expect(formatBytes(1023)).toBe("1023 B");
  });

  it("scales into the next unit at each threshold", () => {
    expect(formatBytes(1024)).toBe("1 KB");
    expect(formatBytes(1024 * 1024)).toBe("1 MB");
    expect(formatBytes(1024 ** 3)).toBe("1 GB");
    expect(formatBytes(1024 ** 4)).toBe("1 TB");
  });

  it("rounds to one decimal place", () => {
    expect(formatBytes(1536)).toBe("1.5 KB");
    expect(formatBytes(1024 * 1024 * 2.25)).toBe("2.3 MB");
  });

  it("refuses to invent a value for impossible input", () => {
    expect(formatBytes(-1)).toBe("Unknown");
    expect(formatBytes(Number.NaN)).toBe("Unknown");
    expect(formatBytes(Number.POSITIVE_INFINITY)).toBe("Unknown");
  });
});

describe("formatDateTime", () => {
  it("reports missing timestamps", () => {
    expect(formatDateTime(null)).toBe("Unknown");
    expect(formatDateTime(Number.NaN)).toBe("Unknown");
  });

  it("formats a backend millisecond timestamp", () => {
    const formatted = formatDateTime(1_700_000_000_000);

    expect(formatted).not.toBe("Unknown");
    expect(formatted.length).toBeGreaterThan(0);
  });
});
