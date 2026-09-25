import { invoke, isTauri } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";

import { inspectPath, pickDirectory } from "./filesystem-service";
import { IpcError } from "./ipc";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedIsTauri = vi.mocked(isTauri);

const METADATA = {
  path: "C:\\Users",
  name: "Users",
  isDir: true,
  isFile: false,
  isSymlink: false,
  sizeBytes: 4096,
  modifiedMs: 1_700_000_000_000,
  readonly: false,
};

describe("filesystem service", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedIsTauri.mockReturnValue(true);
  });

  it("sends the path to the backend and returns its metadata", async () => {
    mockedInvoke.mockResolvedValue(METADATA);

    await expect(inspectPath("C:\\Users")).resolves.toEqual(METADATA);
    expect(mockedInvoke).toHaveBeenCalledWith("inspect_path", {
      path: "C:\\Users",
    });
  });

  it("accepts a null modification time", async () => {
    mockedInvoke.mockResolvedValue({ ...METADATA, modifiedMs: null });

    await expect(inspectPath("C:\\Users")).resolves.toMatchObject({
      modifiedMs: null,
    });
  });

  it("rejects metadata that breaks the contract", async () => {
    mockedInvoke.mockResolvedValue({ ...METADATA, sizeBytes: -1 });

    const error = await inspectPath("C:\\Users").catch(
      (failure: unknown) => failure,
    );

    expect(error).toBeInstanceOf(IpcError);
    expect((error as IpcError).code).toBe("invalid_response");
  });

  it("surfaces path_not_found from the backend", async () => {
    mockedInvoke.mockRejectedValue({
      code: "path_not_found",
      message: "path not found: C:\\nope",
    });

    const error = await inspectPath("C:\\nope").catch(
      (failure: unknown) => failure,
    );

    expect((error as IpcError).code).toBe("path_not_found");
  });

  it("returns the validated folder chosen in the native dialog", async () => {
    mockedInvoke.mockResolvedValue("D:\\Media");

    await expect(pickDirectory()).resolves.toBe("D:\\Media");
    expect(mockedInvoke).toHaveBeenCalledWith("pick_directory", undefined);
  });

  it("returns null when the dialog is cancelled", async () => {
    mockedInvoke.mockResolvedValue(null);

    await expect(pickDirectory()).resolves.toBeNull();
  });

  it("rejects a selection that breaks the contract", async () => {
    mockedInvoke.mockResolvedValue("");

    const error = await pickDirectory().catch((failure: unknown) => failure);

    expect((error as IpcError).code).toBe("invalid_response");
  });

  it("reports the browser runtime as unavailable", async () => {
    mockedIsTauri.mockReturnValue(false);

    const pickError = await pickDirectory().catch(
      (failure: unknown) => failure,
    );
    const inspectError = await inspectPath("C:\\").catch(
      (failure: unknown) => failure,
    );

    expect((pickError as IpcError).code).toBe("unavailable");
    expect((inspectError as IpcError).code).toBe("unavailable");
    expect(mockedInvoke).not.toHaveBeenCalled();
  });
});
