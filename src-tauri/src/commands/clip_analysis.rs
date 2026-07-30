use std::{
    path::PathBuf,
    sync::{atomic::AtomicBool, Arc},
    thread,
};

use chrono::Utc;
use tauri::{AppHandle, Manager, State};
use uuid::Uuid;

use crate::{
    domain::{AppError, CancelClipAnalysisResponse, StartClipAnalysisResponse},
    security::path_policy::PathPolicy,
    services::clip_analysis::{self, ClipAnalysisRegistry},
};

#[tauri::command(rename_all = "camelCase")]
pub async fn start_clip_analysis(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
    registry: State<'_, ClipAnalysisRegistry>,
    source_path: String,
) -> Result<StartClipAnalysisResponse, AppError> {
    let source_path = paths.require_existing_file(PathBuf::from(source_path).as_path())?;
    let resource_dir = app.path().resource_dir().ok();
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        clip_analysis::prepare_clip_analysis(source_path, resource_dir)
    })
    .await
    .map_err(|_| AppError::internal("Clip analysis preparation did not complete."))??;

    let job_id = Uuid::new_v4().to_string();
    let cancel_requested = Arc::new(AtomicBool::new(false));
    let registry = registry.inner().clone();
    registry.reserve(job_id.clone(), cancel_requested.clone())?;
    let worker_job_id = job_id.clone();
    let worker_registry = registry.clone();
    let worker_app = app.clone();
    if thread::Builder::new()
        .name(format!("skcf-clip-analysis-{job_id}"))
        .spawn(move || {
            clip_analysis::run_clip_analysis(
                worker_app,
                worker_registry,
                worker_job_id,
                cancel_requested,
                prepared,
            );
        })
        .is_err()
    {
        registry.finish(&job_id);
        return Err(AppError::internal(
            "The clip analysis worker could not be started.",
        ));
    }

    Ok(StartClipAnalysisResponse {
        job_id,
        accepted_at: Utc::now().to_rfc3339(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn cancel_clip_analysis(
    registry: State<'_, ClipAnalysisRegistry>,
    job_id: String,
) -> Result<CancelClipAnalysisResponse, AppError> {
    Ok(CancelClipAnalysisResponse {
        accepted: registry.cancel(&job_id)?,
    })
}
