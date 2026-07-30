use tauri::{AppHandle, Manager};

use crate::{
    domain::{
        AiModelOption, AiPostProvider, AiProviderCredentialStatus, AiYouTubePostBrief,
        AiYouTubePostDraft, AppError,
    },
    services::ai_post::AiPostService,
};

#[tauri::command]
pub async fn get_ai_provider_credential_statuses(
    app: AppHandle,
) -> Result<Vec<AiProviderCredentialStatus>, AppError> {
    run_blocking(app, |service| service.credential_statuses()).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn save_ai_provider_api_key(
    app: AppHandle,
    provider: AiPostProvider,
    api_key: String,
) -> Result<AiProviderCredentialStatus, AppError> {
    run_blocking(app, move |service| service.save_api_key(provider, &api_key)).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn clear_ai_provider_api_key(
    app: AppHandle,
    provider: AiPostProvider,
) -> Result<AiProviderCredentialStatus, AppError> {
    run_blocking(app, move |service| service.clear_api_key(provider)).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn list_ai_provider_models(
    app: AppHandle,
    provider: AiPostProvider,
) -> Result<Vec<AiModelOption>, AppError> {
    run_blocking(app, move |service| service.list_models(provider)).await
}

#[tauri::command(rename_all = "camelCase")]
pub async fn generate_ai_youtube_post(
    app: AppHandle,
    provider: AiPostProvider,
    model: String,
    brief: AiYouTubePostBrief,
) -> Result<AiYouTubePostDraft, AppError> {
    run_blocking(app, move |service| {
        service.generate(provider, &model, &brief)
    })
    .await
}

async fn run_blocking<T, F>(app: AppHandle, operation: F) -> Result<T, AppError>
where
    T: Send + 'static,
    F: FnOnce(&AiPostService) -> Result<T, AppError> + Send + 'static,
{
    tauri::async_runtime::spawn_blocking(move || {
        let service = app.state::<AiPostService>();
        operation(service.inner())
    })
    .await
    .map_err(|_| AppError::internal("The AI provider operation did not complete."))?
}
