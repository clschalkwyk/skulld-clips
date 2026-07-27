use std::{fs, path::PathBuf};

use chrono::Utc;
use serde::{Deserialize, Serialize};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;
use tauri_plugin_store::StoreExt;

use crate::{
    domain::{AppError, MediaProbe, ProjectV1},
    security::path_policy::PathPolicy,
    services::{probe, projects},
};

const RECENTS_STORE: &str = "settings.json";
const RECENTS_KEY: &str = "recentProjects";
const MAX_RECENTS: usize = 20;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct CreateProjectResponse {
    project_path: String,
    project: ProjectV1,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct LoadProjectResponse {
    project_path: String,
    project: ProjectV1,
    source_status: projects::SourceStatus,
    migration_applied: bool,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SaveProjectResponse {
    saved_at: String,
    project_sha256: String,
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RelinkSourceResponse {
    project: ProjectV1,
    fingerprint_matched: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RecentProject {
    project_path: String,
    name: String,
    source_filename: String,
    last_opened_at: String,
    source_status: projects::SourceStatus,
    duration_ms: u64,
}

#[tauri::command]
pub async fn select_media_file(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
) -> Result<Option<String>, AppError> {
    let dialog_app = app.clone();
    let selected = tauri::async_runtime::spawn_blocking(move || {
        dialog_app
            .dialog()
            .file()
            .add_filter(
                "Gameplay video",
                &["mp4", "mov", "mkv", "webm", "m4v", "avi"],
            )
            .blocking_pick_file()
    })
    .await
    .map_err(|_| AppError::internal("The file dialog did not complete."))?;
    let selected = authorize_selected_file(selected, &paths)?;
    if let Some(path) = &selected {
        app.asset_protocol_scope()
            .allow_file(path)
            .map_err(|_| AppError::internal("The media preview path could not be authorized."))?;
    }
    Ok(selected)
}

#[tauri::command]
pub async fn select_project_file(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
) -> Result<Option<String>, AppError> {
    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("Skull’d Clip Forge project", &["json"])
            .blocking_pick_file()
    })
    .await
    .map_err(|_| AppError::internal("The file dialog did not complete."))?;
    authorize_selected_file(selected, &paths)
}

#[tauri::command]
pub async fn select_projects_folder(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
) -> Result<Option<String>, AppError> {
    let selected =
        tauri::async_runtime::spawn_blocking(move || app.dialog().file().blocking_pick_folder())
            .await
            .map_err(|_| AppError::internal("The folder dialog did not complete."))?;
    match selected {
        Some(path) => {
            let path = path.into_path().map_err(|_| {
                AppError::invalid_argument("The selected folder path is not supported.")
            })?;
            let canonical = paths.authorize_existing_directory(&path)?;
            Ok(Some(path_to_string(canonical)?))
        }
        None => Ok(None),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn probe_media(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
    path: String,
) -> Result<MediaProbe, AppError> {
    let source_path = paths.require_existing_file(PathBuf::from(path).as_path())?;
    let resource_dir = app.path().resource_dir().ok();
    tauri::async_runtime::spawn_blocking(move || {
        probe::probe_media(&source_path, resource_dir.as_deref())
    })
    .await
    .map_err(|_| AppError::internal("The media probe did not complete."))?
}

#[tauri::command(rename_all = "camelCase")]
pub async fn create_project(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
    source_path: String,
    project_name: Option<String>,
    projects_root: Option<String>,
) -> Result<CreateProjectResponse, AppError> {
    let source_path = paths.require_existing_file(PathBuf::from(source_path).as_path())?;
    let projects_root = match projects_root {
        Some(root) => paths.require_existing_directory(PathBuf::from(root).as_path())?,
        None => {
            let root = app
                .path()
                .app_local_data_dir()
                .map_err(|_| AppError::internal("The local project folder is unavailable."))?
                .join("projects");
            fs::create_dir_all(&root).map_err(|_| {
                AppError::io(
                    "The local project folder could not be created.",
                    "Check application storage permissions and try again.",
                )
            })?;
            root
        }
    };
    let resource_dir = app.path().resource_dir().ok();
    let created = tauri::async_runtime::spawn_blocking(move || {
        projects::create_project(
            &source_path,
            &projects_root,
            project_name.as_deref(),
            resource_dir.as_deref(),
        )
    })
    .await
    .map_err(|_| AppError::internal("Project creation did not complete."))??;
    let project_path = paths.authorize_existing_file(&created.project_path)?;
    let response = CreateProjectResponse {
        project_path: path_to_string(project_path)?,
        project: created.project,
    };
    record_recent(
        &app,
        &response.project_path,
        &response.project,
        projects::SourceStatus::Ok,
    )?;
    Ok(response)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn load_project(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
    project_path: String,
) -> Result<LoadProjectResponse, AppError> {
    let project_path = paths.require_existing_file(PathBuf::from(project_path).as_path())?;
    let loaded =
        tauri::async_runtime::spawn_blocking(move || projects::load_project(&project_path))
            .await
            .map_err(|_| AppError::internal("Project loading did not complete."))??;
    let response = LoadProjectResponse {
        project_path: path_to_string(loaded.project_path)?,
        project: loaded.project,
        source_status: loaded.source_status,
        migration_applied: false,
    };
    if response.source_status == projects::SourceStatus::Ok {
        app.asset_protocol_scope()
            .allow_file(&response.project.source.path)
            .map_err(|_| AppError::internal("The media preview path could not be authorized."))?;
    }
    for overlay in &response.project.overlays {
        let (asset, _) = overlay.asset();
        let asset_path = crate::services::assets::resolve_project_asset_path(
            PathBuf::from(&response.project_path).as_path(),
            &asset.relative_path,
        )?;
        app.asset_protocol_scope()
            .allow_file(asset_path)
            .map_err(|_| AppError::internal("A project asset preview could not be authorized."))?;
    }
    record_recent(
        &app,
        &response.project_path,
        &response.project,
        response.source_status,
    )?;
    Ok(response)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn save_project(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
    project_path: String,
    project: ProjectV1,
) -> Result<SaveProjectResponse, AppError> {
    let project_path = paths.require_existing_file(PathBuf::from(project_path).as_path())?;
    let recent_path = path_to_string(project_path.clone())?;
    let recent_project = project.clone();
    let saved = tauri::async_runtime::spawn_blocking(move || {
        projects::save_project(&project_path, project)
    })
    .await
    .map_err(|_| AppError::internal("Project saving did not complete."))??;
    update_recent(
        &app,
        &recent_path,
        &recent_project,
        projects::source_status(&recent_project),
    )?;
    Ok(SaveProjectResponse {
        saved_at: saved.saved_at,
        project_sha256: saved.project_sha256,
    })
}

#[tauri::command(rename_all = "camelCase")]
pub async fn relink_source(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
    project_path: String,
    replacement_path: String,
    accept_fingerprint_mismatch: bool,
) -> Result<RelinkSourceResponse, AppError> {
    let project_path = paths.require_existing_file(PathBuf::from(project_path).as_path())?;
    let replacement_path =
        paths.require_existing_file(PathBuf::from(replacement_path).as_path())?;
    let recent_path = path_to_string(project_path.clone())?;
    let resource_dir = app.path().resource_dir().ok();
    let relinked = tauri::async_runtime::spawn_blocking(move || {
        projects::relink_source(
            &project_path,
            &replacement_path,
            accept_fingerprint_mismatch,
            resource_dir.as_deref(),
        )
    })
    .await
    .map_err(|_| AppError::internal("Source relinking did not complete."))??;
    app.asset_protocol_scope()
        .allow_file(&relinked.project.source.path)
        .map_err(|_| AppError::internal("The media preview path could not be authorized."))?;
    update_recent(
        &app,
        &recent_path,
        &relinked.project,
        projects::SourceStatus::Ok,
    )?;
    Ok(RelinkSourceResponse {
        project: relinked.project,
        fingerprint_matched: relinked.fingerprint_matched,
    })
}

#[tauri::command]
pub async fn list_recent_projects(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
) -> Result<Vec<RecentProject>, AppError> {
    let mut recents = read_recents(&app)?;
    for recent in &mut recents {
        let path = PathBuf::from(&recent.project_path);
        if let Ok(canonical) = paths.authorize_existing_file(&path) {
            if let Ok(loaded) = projects::load_project(&canonical) {
                recent.name = loaded.project.name;
                recent.source_filename = loaded.project.source.filename;
                recent.duration_ms = loaded.project.source.probe.duration_ms;
                recent.source_status = loaded.source_status;
            }
        }
    }
    write_recents(&app, &recents)?;
    Ok(recents)
}

#[tauri::command(rename_all = "camelCase")]
pub fn remove_recent_project(app: AppHandle, project_path: String) -> Result<(), AppError> {
    let mut recents = read_recents(&app)?;
    recents.retain(|recent| recent.project_path != project_path);
    write_recents(&app, &recents)
}

fn authorize_selected_file(
    selected: Option<tauri_plugin_dialog::FilePath>,
    paths: &PathPolicy,
) -> Result<Option<String>, AppError> {
    match selected {
        Some(path) => {
            let path = path.into_path().map_err(|_| {
                AppError::invalid_argument("The selected file path is not supported.")
            })?;
            let canonical = paths.authorize_existing_file(&path)?;
            Ok(Some(path_to_string(canonical)?))
        }
        None => Ok(None),
    }
}

fn record_recent(
    app: &AppHandle,
    project_path: &str,
    project: &ProjectV1,
    source_status: projects::SourceStatus,
) -> Result<(), AppError> {
    let mut recents = read_recents(app)?;
    recents.retain(|recent| recent.project_path != project_path);
    recents.insert(
        0,
        RecentProject {
            project_path: project_path.to_owned(),
            name: project.name.clone(),
            source_filename: project.source.filename.clone(),
            last_opened_at: Utc::now().to_rfc3339(),
            source_status,
            duration_ms: project.source.probe.duration_ms,
        },
    );
    recents.truncate(MAX_RECENTS);
    write_recents(app, &recents)
}

fn update_recent(
    app: &AppHandle,
    project_path: &str,
    project: &ProjectV1,
    source_status: projects::SourceStatus,
) -> Result<(), AppError> {
    let mut recents = read_recents(app)?;
    if let Some(recent) = recents
        .iter_mut()
        .find(|recent| recent.project_path == project_path)
    {
        recent.name = project.name.clone();
        recent.source_filename = project.source.filename.clone();
        recent.source_status = source_status;
        recent.duration_ms = project.source.probe.duration_ms;
    } else {
        recents.push(RecentProject {
            project_path: project_path.to_owned(),
            name: project.name.clone(),
            source_filename: project.source.filename.clone(),
            last_opened_at: Utc::now().to_rfc3339(),
            source_status,
            duration_ms: project.source.probe.duration_ms,
        });
    }
    recents.truncate(MAX_RECENTS);
    write_recents(app, &recents)
}

fn read_recents(app: &AppHandle) -> Result<Vec<RecentProject>, AppError> {
    let store = app
        .store(RECENTS_STORE)
        .map_err(|_| AppError::internal("Recent projects could not be loaded."))?;
    match store.get(RECENTS_KEY) {
        Some(value) => serde_json::from_value(value)
            .map_err(|_| AppError::internal("Recent project data is invalid.")),
        None => Ok(Vec::new()),
    }
}

fn write_recents(app: &AppHandle, recents: &[RecentProject]) -> Result<(), AppError> {
    let store = app
        .store(RECENTS_STORE)
        .map_err(|_| AppError::internal("Recent projects could not be opened."))?;
    let value = serde_json::to_value(recents)
        .map_err(|_| AppError::internal("Recent projects could not be serialized."))?;
    store.set(RECENTS_KEY, value);
    store.save().map_err(|_| {
        AppError::io(
            "Recent projects could not be saved.",
            "Check application storage permissions and try again.",
        )
    })
}

fn path_to_string(path: PathBuf) -> Result<String, AppError> {
    path.into_os_string().into_string().map_err(|_| {
        AppError::invalid_argument("Selected paths must be valid Unicode on this platform.")
    })
}
