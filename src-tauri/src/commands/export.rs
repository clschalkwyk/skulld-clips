use std::{
    path::{Path, PathBuf},
    sync::{atomic::AtomicBool, Arc},
    thread,
};

use chrono::Utc;
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use uuid::Uuid;

use crate::{
    domain::{
        AppError, CancelExportResponse, ExportRequest, ExportValidation, StartExportResponse,
    },
    security::path_policy::PathPolicy,
    services::export::{self, ExportRegistry},
};

#[tauri::command(rename_all = "camelCase")]
pub async fn select_export_destination(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
    suggested_name: String,
) -> Result<Option<String>, AppError> {
    let filename = safe_export_filename(&suggested_name);
    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("MPEG-4 video", &["mp4"])
            .set_file_name(filename)
            .blocking_save_file()
    })
    .await
    .map_err(|_| AppError::internal("The export destination dialog did not complete."))?;
    match selected {
        Some(path) => {
            let mut path = path.into_path().map_err(|_| {
                AppError::destination_denied("The selected export path is not supported.")
            })?;
            if path.extension().is_none() {
                path.set_extension("mp4");
            }
            let authorized = paths.authorize_output_file(&path)?;
            Ok(Some(path_to_string(&authorized)?))
        }
        None => Ok(None),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn validate_export(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
    registry: State<'_, ExportRegistry>,
    request: ExportRequest,
) -> Result<ExportValidation, AppError> {
    let project_path =
        paths.require_existing_file(PathBuf::from(&request.project_path).as_path())?;
    let destination_path =
        paths.require_output_file(PathBuf::from(&request.destination_path).as_path())?;
    let resource_dir = app.path().resource_dir().ok();
    let registry = registry.inner().clone();
    tauri::async_runtime::spawn_blocking(move || {
        export::validate_export(
            &request,
            &project_path,
            &destination_path,
            resource_dir.as_deref(),
            &registry,
        )
    })
    .await
    .map_err(|_| AppError::internal("Export validation did not complete."))
}

#[tauri::command(rename_all = "camelCase")]
pub async fn start_export(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
    registry: State<'_, ExportRegistry>,
    request: ExportRequest,
) -> Result<StartExportResponse, AppError> {
    let project_path =
        paths.require_existing_file(PathBuf::from(&request.project_path).as_path())?;
    let destination_path =
        paths.require_output_file(PathBuf::from(&request.destination_path).as_path())?;
    let resource_dir = app.path().resource_dir().ok();
    let registry = registry.inner().clone();
    let prepare_registry = registry.clone();
    let prepared = tauri::async_runtime::spawn_blocking(move || {
        export::prepare_export(
            request,
            project_path,
            destination_path,
            resource_dir,
            &prepare_registry,
        )
    })
    .await
    .map_err(|_| AppError::internal("Export preparation did not complete."))??;

    let job_id = Uuid::new_v4().to_string();
    let cancel_requested = Arc::new(AtomicBool::new(false));
    registry.reserve(job_id.clone(), cancel_requested.clone())?;
    let worker_registry = registry.clone();
    let worker_job_id = job_id.clone();
    let worker_app = app.clone();
    if thread::Builder::new()
        .name(format!("skcf-export-{job_id}"))
        .spawn(move || {
            export::run_export(
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
            "The export worker could not be started.",
        ));
    }
    Ok(StartExportResponse {
        job_id,
        accepted_at: Utc::now().to_rfc3339(),
    })
}

#[tauri::command(rename_all = "camelCase")]
pub fn cancel_export(
    registry: State<'_, ExportRegistry>,
    job_id: String,
) -> Result<CancelExportResponse, AppError> {
    Ok(CancelExportResponse {
        accepted: registry.cancel(&job_id)?,
    })
}

fn safe_export_filename(suggested_name: &str) -> String {
    let stem: String = suggested_name
        .chars()
        .filter(|character| {
            character.is_alphanumeric() || matches!(character, ' ' | '-' | '_' | '.')
        })
        .take(100)
        .collect();
    let stem = stem
        .trim()
        .trim_end_matches(".mp4")
        .trim()
        .trim_matches('.');
    format!("{}.mp4", if stem.is_empty() { "skulld-clip" } else { stem })
}

fn path_to_string(path: &Path) -> Result<String, AppError> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        AppError::destination_denied("Export paths must be valid Unicode on this platform.")
    })
}

#[cfg(test)]
mod tests {
    use super::safe_export_filename;

    #[test]
    fn suggested_export_names_cannot_inject_paths() {
        assert_eq!(
            safe_export_filename("../../Boss:Fight?.mp4"),
            "BossFight.mp4"
        );
        assert_eq!(safe_export_filename("  "), "skulld-clip.mp4");
    }
}
