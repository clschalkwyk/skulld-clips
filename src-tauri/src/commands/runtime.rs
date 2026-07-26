use tauri::{AppHandle, Manager};

use crate::{
    domain::{AppError, RuntimeInfo},
    services::media_tools,
};

#[tauri::command]
pub async fn get_runtime_info(app: AppHandle) -> Result<RuntimeInfo, AppError> {
    let app_version = app.package_info().version.to_string();
    let resource_dir = app.path().resource_dir().ok();

    tauri::async_runtime::spawn_blocking(move || {
        media_tools::collect_runtime_info(app_version, resource_dir.as_deref())
    })
    .await
    .map_err(|_| AppError::internal("The runtime check did not complete."))?
}
