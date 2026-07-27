use std::path::PathBuf;

use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::{
    domain::{AppError, AssetRef},
    security::path_policy::PathPolicy,
    services::{assets, projects},
};

#[tauri::command]
pub async fn select_overlay_file(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
) -> Result<Option<String>, AppError> {
    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("Static image", &["png", "jpg", "jpeg", "webp"])
            .blocking_pick_file()
    })
    .await
    .map_err(|_| AppError::internal("The image file dialog did not complete."))?;
    match selected {
        Some(path) => {
            let path = path.into_path().map_err(|_| {
                AppError::invalid_argument("The selected image path is not supported.")
            })?;
            let canonical = paths.authorize_existing_file(&path)?;
            Ok(Some(path_to_string(canonical)?))
        }
        None => Ok(None),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn import_overlay_asset(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
    project_path: String,
    source_asset_path: String,
) -> Result<AssetRef, AppError> {
    let project_path = paths.require_existing_file(PathBuf::from(project_path).as_path())?;
    let source_asset_path =
        paths.require_existing_file(PathBuf::from(source_asset_path).as_path())?;
    let validation_path = project_path.clone();
    tauri::async_runtime::spawn_blocking(move || projects::load_project(&validation_path))
        .await
        .map_err(|_| AppError::internal("Project asset validation did not complete."))??;
    let (asset, destination) = tauri::async_runtime::spawn_blocking(move || {
        assets::import_overlay_asset(&project_path, &source_asset_path)
    })
    .await
    .map_err(|_| AppError::internal("Image import did not complete."))??;
    app.asset_protocol_scope()
        .allow_file(destination)
        .map_err(|_| AppError::internal("The image preview path could not be authorized."))?;
    Ok(asset)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn write_caption_asset(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
    project_path: String,
    content_hash: String,
    png_bytes_base64: String,
    width: u32,
    height: u32,
) -> Result<AssetRef, AppError> {
    let project_path = paths.require_existing_file(PathBuf::from(project_path).as_path())?;
    let validation_path = project_path.clone();
    tauri::async_runtime::spawn_blocking(move || projects::load_project(&validation_path))
        .await
        .map_err(|_| AppError::internal("Project asset validation did not complete."))??;
    let (asset, destination) = tauri::async_runtime::spawn_blocking(move || {
        assets::write_caption_asset(
            &project_path,
            &content_hash,
            &png_bytes_base64,
            width,
            height,
        )
    })
    .await
    .map_err(|_| AppError::internal("Caption rasterization save did not complete."))??;
    app.asset_protocol_scope()
        .allow_file(destination)
        .map_err(|_| AppError::internal("The caption preview path could not be authorized."))?;
    Ok(asset)
}

fn path_to_string(path: PathBuf) -> Result<String, AppError> {
    path.into_os_string().into_string().map_err(|_| {
        AppError::invalid_argument("Selected paths must be valid Unicode on this platform.")
    })
}
