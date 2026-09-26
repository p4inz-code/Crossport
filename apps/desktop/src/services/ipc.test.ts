import { invoke, isTauri } from "@tauri-apps/api/core";
import { beforeEach, describe, expect, it, vi } from "vitest";
import { z } from "zod";

import {
  BACKEND_ERROR_CODES,
  CLIENT_ERROR_CODES,
  IPC_ERROR_CODES,
  IpcError,
  invokeCommand,
  invokeTyped,
  isIpcErrorCode,
  requireDesktopRuntime,
  toIpcError,
} from "./ipc";

vi.mock("@tauri-apps/api/core", () => ({
  invoke: vi.fn(),
  isTauri: vi.fn(() => true),
}));

const mockedInvoke = vi.mocked(invoke);
const mockedIsTauri = vi.mocked(isTauri);

describe("error contract", () => {
  beforeEach(() => {
    vi.clearAllMocks();
    mockedIsTauri.mockReturnValue(true);
  });

  it("lists exactly the codes the Rust AppError can produce", () => {
    expect([...BACKEND_ERROR_CODES]).toEqual([
      "invalid_input",
      "path_not_found",
      "path_not_directory",
      "permission_denied",
      "io",
      "unsafe_relationship",
      "not_enough_space",
      "disk_full",
      "too_many_items",
      "transfer_not_found",
      "transfer_failed",
      "internal",
    ]);
  });

  it("keeps client-side codes separate from backend codes", () => {
    for (const code of CLIENT_ERROR_CODES) {
      expect(BACKEND_ERROR_CODES as readonly string[]).not.toContain(code);
    }
    expect([...IPC_ERROR_CODES]).toEqual([
      ...BACKEND_ERROR_CODES,
      ...CLIENT_ERROR_CODES,
    ]);
  });

  it("recognises published codes only", () => {
    expect(isIpcErrorCode("invalid_input")).toBe(true);
    expect(isIpcErrorCode("invalid_response")).toBe(true);
    expect(isIpcErrorCode("rate_limited")).toBe(false);
    expect(isIpcErrorCode("INVALID_INPUT")).toBe(false);
  });
});

describe("toIpcError", () => {
  it("keeps the backend code and message", () => {
    const error = toIpcError({
      code: "path_not_found",
      message: "path not found: C:\\nope",
    });

    expect(error).toBeInstanceOf(IpcError);
    expect(error.code).toBe("path_not_found");
    expect(error.message).toBe("path not found: C:\\nope");
    expect(error.unsupportedCode).toBeNull();
  });

  it("maps every backend code verbatim", () => {
    for (const code of BACKEND_ERROR_CODES) {
      expect(toIpcError({ code, message: "detail" }).code).toBe(code);
    }
  });

  it("preserves an unrecognised backend code for diagnostics", () => {
    const error = toIpcError({ code: "rate_limited", message: "slow down" });

    expect(error.code).toBe("unknown");
    expect(error.unsupportedCode).toBe("rate_limited");
    expect(error.message).toBe("slow down");
  });

  it("uses a stable message when the backend omits one", () => {
    expect(toIpcError({ code: "internal" }).message).toBe(
      "The desktop backend reported an error.",
    );
    expect(toIpcError({}).message).toBe(
      "The desktop backend reported an error.",
    );
  });

  it("normalises strings, Errors, and empty values", () => {
    expect(toIpcError("permission denied").message).toBe("permission denied");
    expect(toIpcError(new Error("boom")).message).toBe("boom");
    expect(toIpcError(undefined).code).toBe("unknown");
    expect(toIpcError(null).code).toBe("unknown");
    expect(toIpcError(42).code).toBe("unknown");
  });

  it("passes an existing IpcError through unchanged", () => {
    const original = new IpcError("io", "the device disappeared");
    expect(toIpcError(original)).toBe(original);
  });
});

describe("invokeCommand", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns the backend payload", async () => {
    mockedInvoke.mockResolvedValue({ theme: "dark", locale: "en" });

    await expect(invokeCommand("get_settings")).resolves.toEqual({
      theme: "dark",
      locale: "en",
    });
    expect(mockedInvoke).toHaveBeenCalledWith("get_settings", undefined);
  });

  it("forwards command arguments", async () => {
    mockedInvoke.mockResolvedValue(undefined);

    await invokeCommand("update_settings", { settings: { theme: "dark" } });

    expect(mockedInvoke).toHaveBeenCalledWith("update_settings", {
      settings: { theme: "dark" },
    });
  });

  it("rethrows a structured failure as an IpcError", async () => {
    mockedInvoke.mockRejectedValue({
      code: "invalid_input",
      message: "invalid input: unknown theme",
    });

    const error = await invokeCommand("update_settings").catch(
      (failure: unknown) => failure,
    );

    expect(error).toBeInstanceOf(IpcError);
    expect((error as IpcError).code).toBe("invalid_input");
  });
});

describe("invokeTyped", () => {
  const schema = z.object({ root: z.string(), label: z.string() });

  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("returns schema-validated data", async () => {
    mockedInvoke.mockResolvedValue({ root: "C:\\", label: "C:", extra: true });

    await expect(invokeTyped("list_drives", schema)).resolves.toEqual({
      root: "C:\\",
      label: "C:",
    });
  });

  it("rejects a payload that breaks the contract", async () => {
    mockedInvoke.mockResolvedValue({ root: 42, label: "C:" });

    const error = await invokeTyped("list_drives", schema).catch(
      (failure: unknown) => failure,
    );

    expect(error).toBeInstanceOf(IpcError);
    expect((error as IpcError).code).toBe("invalid_response");
    expect((error as IpcError).message).toContain("list_drives");
  });

  it("still surfaces backend errors", async () => {
    mockedInvoke.mockRejectedValue({
      code: "permission_denied",
      message: "no",
    });

    const error = await invokeTyped("list_drives", schema).catch(
      (failure: unknown) => failure,
    );

    expect((error as IpcError).code).toBe("permission_denied");
  });
});

describe("requireDesktopRuntime", () => {
  beforeEach(() => {
    vi.clearAllMocks();
  });

  it("allows calls inside the desktop app", () => {
    mockedIsTauri.mockReturnValue(true);

    expect(() => requireDesktopRuntime("list_drives")).not.toThrow();
  });

  it("fails with the unavailable code in a browser", () => {
    mockedIsTauri.mockReturnValue(false);

    expect(() => requireDesktopRuntime("list_drives")).toThrowError(IpcError);

    try {
      requireDesktopRuntime("list_drives");
      throw new Error("expected requireDesktopRuntime to throw");
    } catch (error) {
      expect(error).toBeInstanceOf(IpcError);
      expect((error as IpcError).code).toBe("unavailable");
      expect((error as IpcError).message).toContain("list_drives");
    }
  });
});
