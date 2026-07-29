use tauri::{AppHandle, Manager};

use crate::{
    domain::{AppError, YouTubeConnectionStatus, YouTubeProjectPerformance, YouTubeVideoCandidate},
    services::youtube::YouTubePerformanceService,
};

#[tauri::command]
pub async fn get_youtube_connection_status(
    app: AppHandle,
) -> Result<YouTubeConnectionStatus, AppError> {
    run_blocking(app, |service| service.connection_status()).await
}

#[tauri::command]
pub async fn connect_youtube_channel(app: AppHandle) -> Result<YouTubeConnectionStatus, AppError> {
    run_blocking(app, |service| service.connect()).await
}

#[tauri::command]
pub async fn disconnect_youtube_channel(
    app: AppHandle,
) -> Result<YouTubeConnectionStatus, AppError> {
    run_blocking(app, |service| service.disconnect()).await
}

#[tauri::command]
pub async fn list_recent_youtube_uploads(
    app: AppHandle,
) -> Result<Vec<YouTubeVideoCandidate>, AppError> {
    run_blocking(app, |service| service.list_recent_uploads()).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn link_project_to_youtube_video(
    app: AppHandle,
    project_id: String,
    project_name: String,
    video_id_or_url: String,
) -> Result<YouTubeProjectPerformance, AppError> {
    run_blocking(app, move |service| {
        service.link_project(&project_id, &project_name, &video_id_or_url)
    })
    .await
}

#[tauri::command]
pub async fn list_youtube_performance(
    app: AppHandle,
) -> Result<Vec<YouTubeProjectPerformance>, AppError> {
    run_blocking(app, |service| service.list_performance()).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn sync_youtube_performance(
    app: AppHandle,
    project_id: Option<String>,
) -> Result<Vec<YouTubeProjectPerformance>, AppError> {
    run_blocking(app, move |service| {
        service.sync_performance(project_id.as_deref())
    })
    .await
}

async fn run_blocking<T, F>(app: AppHandle, operation: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&YouTubePerformanceService) -> Result<T, AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let service = app.state::<YouTubePerformanceService>();
        operation(service.inner())
    })
    .await
    .map_err(|_| AppError::internal("The YouTube operation did not complete."))?
}
