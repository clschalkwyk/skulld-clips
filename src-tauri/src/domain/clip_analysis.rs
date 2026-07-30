use serde::Serialize;

use super::AppError;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum ClipEventKind {
    Completion,
    Death,
    BossEncounter,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipCandidate {
    pub id: String,
    pub kind: ClipEventKind,
    pub event_ms: u64,
    pub detected_start_ms: u64,
    pub detected_end_ms: u64,
    pub suggested_in_ms: u64,
    pub suggested_out_ms: u64,
    pub confidence: f64,
    pub evidence: Vec<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct StartClipAnalysisResponse {
    pub job_id: String,
    pub accepted_at: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CancelClipAnalysisResponse {
    pub accepted: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipAnalysisProgressEvent {
    pub event: &'static str,
    pub job_id: String,
    pub progress: f64,
    pub analyzed_ms: u64,
    pub total_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipAnalysisCompletedEvent {
    pub event: &'static str,
    pub job_id: String,
    pub candidates: Vec<ClipCandidate>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipAnalysisFailedEvent {
    pub event: &'static str,
    pub job_id: String,
    pub error: AppError,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ClipAnalysisCancelledEvent {
    pub event: &'static str,
    pub job_id: String,
}
