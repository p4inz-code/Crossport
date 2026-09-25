import { describe, expect, it } from "vitest";

import { driveInfoSchema, driveListSchema } from "./drives";
import { entryMetadataSchema, pickedDirectorySchema } from "./filesystem";
import { systemInfoSchema } from "./system";

describe("drive schemas", () => {
  it("accepts a backend drive entry", () => {
    expect(
      driveInfoSchema.safeParse({ root: "C:\\", label: "C:" }).success,
    ).toBe(true);
    expect(driveInfoSchema.safeParse({ root: "/", label: "/" }).success).toBe(
      true,
    );
  });

  it("rejects empty fields", () => {
    expect(driveInfoSchema.safeParse({ root: "", label: "C:" }).success).toBe(
      false,
    );
    expect(driveInfoSchema.safeParse({ root: "/", label: "" }).success).toBe(
      false,
    );
  });

  it("accepts an empty drive list but not a malformed one", () => {
    expect(driveListSchema.safeParse([]).success).toBe(true);
    expect(driveListSchema.safeParse(null).success).toBe(false);
    expect(driveListSchema.safeParse(["C:\\"]).success).toBe(false);
  });
});

describe("entry metadata schema", () => {
  const metadata = {
    path: "/home/user",
    name: "user",
    isDir: true,
    isFile: false,
    isSymlink: false,
    sizeBytes: 4096,
    modifiedMs: 1_700_000_000_000,
    readonly: false,
  };

  it("accepts a backend metadata payload", () => {
    expect(entryMetadataSchema.safeParse(metadata).success).toBe(true);
  });

  it("accepts a null modification time", () => {
    expect(
      entryMetadataSchema.safeParse({ ...metadata, modifiedMs: null }).success,
    ).toBe(true);
  });

  it("rejects impossible sizes and missing flags", () => {
    expect(
      entryMetadataSchema.safeParse({ ...metadata, sizeBytes: -1 }).success,
    ).toBe(false);
    expect(
      entryMetadataSchema.safeParse({ ...metadata, sizeBytes: 1.5 }).success,
    ).toBe(false);

    const { isDir: _isDir, ...withoutFlag } = metadata;
    expect(entryMetadataSchema.safeParse(withoutFlag).success).toBe(false);
  });

  it("treats a picked directory as a path or a cancellation", () => {
    expect(pickedDirectorySchema.safeParse("C:\\Users").success).toBe(true);
    expect(pickedDirectorySchema.safeParse(null).success).toBe(true);
    expect(pickedDirectorySchema.safeParse("").success).toBe(false);
  });
});

describe("system info schema", () => {
  it("accepts the backend contract", () => {
    expect(
      systemInfoSchema.safeParse({
        platform: "windows",
        os: "windows",
        arch: "x86_64",
        family: "windows",
      }).success,
    ).toBe(true);
  });

  it("rejects unknown platforms and missing fields", () => {
    expect(
      systemInfoSchema.safeParse({
        platform: "solaris",
        os: "solaris",
        arch: "sparc",
        family: "unix",
      }).success,
    ).toBe(false);

    expect(systemInfoSchema.safeParse({ platform: "linux" }).success).toBe(
      false,
    );
  });
});
