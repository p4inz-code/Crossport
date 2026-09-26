/* ==========================================================================
 * IPC transport
 * The only module that talks to the Rust backend. It owns three things:
 *
 * 1. runtime detection (`isTauri()`), so services can offer an honest web
 *    fallback instead of calling commands that cannot exist in a browser;
 * 2. schema validation of every payload before it reaches application state;
 * 3. normalization of backend failures into `IpcError`, whose `code` mirrors
 *    `AppError::code` in `src-tauri/src/errors/mod.rs`.
 *
 * Errors are never reduced to a bare string: callers switch on `code` and use
 * `message` for display only.
 * ========================================================================== */

import { invoke, isTauri } from "@tauri-apps/api/core";
import type { ZodType } from "zod";

/** Error codes the Rust backend returns. Mirrors `AppError::code`. */
export const BACKEND_ERROR_CODES = [
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
] as const;

/** Error codes produced on this side of the boundary. */
export const CLIENT_ERROR_CODES = [
  /** The backend answered with a payload that does not match the contract. */
  "invalid_response",
  /** The command only exists in the desktop app, and we are in a browser. */
  "unavailable",
  /** Failure with no structured information; `message` carries the detail. */
  "unknown",
] as const;

export const IPC_ERROR_CODES = [
  ...BACKEND_ERROR_CODES,
  ...CLIENT_ERROR_CODES,
] as const;

export type BackendErrorCode = (typeof BACKEND_ERROR_CODES)[number];
export type ClientErrorCode = (typeof CLIENT_ERROR_CODES)[number];
export type IpcErrorCode = (typeof IPC_ERROR_CODES)[number];

/** True when the frontend runs inside the Tauri shell. */
export function isDesktopRuntime(): boolean {
  return isTauri();
}

/** A structured failure from the IPC boundary. */
export class IpcError extends Error {
  readonly code: IpcErrorCode;
  /**
   * Code reported by the backend when it is not part of the published
   * contract. Kept so diagnostics never lose information, while `code` stays
   * exhaustively switchable.
   */
  readonly unsupportedCode: string | null;

  constructor(
    code: IpcErrorCode,
    message: string,
    unsupportedCode: string | null = null,
  ) {
    super(message);
    this.name = "IpcError";
    this.code = code;
    this.unsupportedCode = unsupportedCode;
  }
}

export function isIpcErrorCode(code: string): code is IpcErrorCode {
  return (IPC_ERROR_CODES as readonly string[]).includes(code);
}

/**
 * Normalizes anything Tauri can reject with — a serialized `AppError`, a
 * plugin permission failure, a string, or an `Error` — into an `IpcError`.
 */
export function toIpcError(error: unknown): IpcError {
  if (error instanceof IpcError) {
    return error;
  }
  if (error instanceof Error) {
    return new IpcError("unknown", error.message);
  }

  const record = asRecord(error);
  if (record !== null) {
    const message =
      typeof record.message === "string" && record.message.length > 0
        ? record.message
        : "The desktop backend reported an error.";
    if (typeof record.code === "string") {
      return isIpcErrorCode(record.code)
        ? new IpcError(record.code, message)
        : new IpcError("unknown", message, record.code);
    }
    return new IpcError("unknown", message);
  }

  if (typeof error === "string" && error.length > 0) {
    return new IpcError("unknown", error);
  }
  return new IpcError("unknown", "The desktop backend reported an error.");
}

/**
 * Invokes a backend command, converting rejections into `IpcError`.
 *
 * Prefer [`invokeTyped`] for anything that flows into application state.
 */
export async function invokeCommand<T>(
  command: string,
  args?: Record<string, unknown>,
): Promise<T> {
  try {
    return await invoke<T>(command, args);
  } catch (error) {
    throw toIpcError(error);
  }
}

/** Invokes a command and validates its payload against `schema`. */
export async function invokeTyped<T>(
  command: string,
  schema: ZodType<T>,
  args?: Record<string, unknown>,
): Promise<T> {
  const raw = await invokeCommand<unknown>(command, args);
  const parsed = schema.safeParse(raw);
  if (!parsed.success) {
    throw new IpcError(
      "invalid_response",
      `The '${command}' command returned an unexpected payload.`,
    );
  }
  return parsed.data;
}

/**
 * Fails with `unavailable` when a command that only exists inside the desktop
 * app is called from a browser. Used by services that have no honest web
 * fallback, so dev mode shows a real explanation instead of fake data.
 */
export function requireDesktopRuntime(command: string): void {
  if (!isDesktopRuntime()) {
    throw new IpcError(
      "unavailable",
      `The '${command}' feature is only available in the CrossPort desktop app.`,
    );
  }
}

function asRecord(value: unknown): Record<string, unknown> | null {
  return typeof value === "object" && value !== null
    ? (value as Record<string, unknown>)
    : null;
}
