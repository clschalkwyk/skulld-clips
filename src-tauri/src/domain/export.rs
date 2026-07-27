use serde::{Deserialize, Serialize};

use crate::domain::{AppError, ExportSettings, ProjectV1};

#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportRequest {
    pub project_path: String,
    pub project_snapshot: ProjectV1,
    pub destination_path: String,
    pub overwrite: bool,
    pub settings: ExportSettings,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportValidation {
    pub valid: bool,
    pub errors: Vec<AppError>,
    pub warnings: Vec<String>,
    pub estimated_bytes: Option<u64>,
    pub free_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartExportResponse {
    pub job_id: String,
    pub accepted_at: String,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelExportResponse {
    pub accepted: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportProgressEvent {
    pub event: &'static str,
    pub job_id: String,
    pub phase: &'static str,
    pub progress: f64,
    pub encoded_ms: u64,
    pub total_ms: u64,
    pub fps: Option<f64>,
    pub speed: Option<f64>,
    pub output_bytes: Option<u64>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCompletedEvent {
    pub event: &'static str,
    pub job_id: String,
    pub output_path: String,
    pub duration_ms: u64,
    pub size_bytes: u64,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportFailedEvent {
    pub event: &'static str,
    pub job_id: String,
    pub error: AppError,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ExportCancelledEvent {
    pub event: &'static str,
    pub job_id: String,
}
