/* ==========================================================================
 * Models module (placeholder)
 * Domain data structures exchanged with the frontend. Only the type surface
 * is defined now; they will be populated by real implementations later.
 * ========================================================================== */

use serde::Serialize;

/// A connected drive. Placeholder model for the drives feature.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct DriveInfo {
  pub id: String,
  pub name: String,
  pub mount_point: Option<String>,
  pub total_bytes: Option<u64>,
  pub free_bytes: Option<u64>,
}

/// An in-flight or queued transfer job. Placeholder model.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct TransferJob {
  pub id: String,
  pub source: String,
  pub destination: String,
  pub status: String,
}

/// A finished transfer recorded in history. Placeholder model.
#[derive(Debug, Clone, Serialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub struct HistoryEntry {
  pub id: String,
  pub transfer_id: String,
  pub started_at: String,
  pub completed_at: Option<String>,
}
