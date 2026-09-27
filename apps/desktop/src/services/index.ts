/* ==========================================================================
 * Services barrel
 * ========================================================================== */

export {
  CLOSE_REQUESTED_EVENT,
  exitApp,
  subscribeToCloseRequests,
} from "./app-service";
export { listDrives } from "./drives-service";
export {
  inspectPath,
  listAncestors,
  listDirectory,
  pickDirectory,
} from "./filesystem-service";
export {
  clearHistory,
  deleteHistoryRecord,
  getArchiveStatus,
  getHistoryRecord,
  listHistory,
} from "./history-service";
export type {
  BackendErrorCode,
  ClientErrorCode,
  IpcErrorCode,
} from "./ipc";
export {
  BACKEND_ERROR_CODES,
  CLIENT_ERROR_CODES,
  IPC_ERROR_CODES,
  IpcError,
  invokeCommand,
  invokeTyped,
  isDesktopRuntime,
  isIpcErrorCode,
  requireDesktopRuntime,
  toIpcError,
} from "./ipc";
export {
  getRecoveryCandidate,
  listRecoveryCandidates,
  recoverTransfer,
} from "./recovery-service";
export { getSettings, updateSettings } from "./settings-service";
export { readSettingsFromLocal, writeSettingsToLocal } from "./storage";
export { getSystemInfo } from "./system-service";
export {
  cancelTransfer,
  clearFinishedTransfers,
  getTransfer,
  listTransfers,
  pauseTransfer,
  planTransfer,
  removeTransfer,
  resumeTransfer,
  startTransfer,
  subscribeToTransferUpdates,
  TRANSFER_EVENT,
} from "./transfer-service";
