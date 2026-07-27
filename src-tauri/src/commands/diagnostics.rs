use std::{
    path::{Path, PathBuf},
    process::{Command, Stdio},
};

use serde::Serialize;
use serde_json::{json, to_value};
use tauri::{AppHandle, Manager, State};
use tauri_plugin_dialog::DialogExt;

use crate::{
    domain::AppError,
    security::path_policy::PathPolicy,
    services::{diagnostics::Diagnostics, media_tools},
};

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RevealResponse {
    opened: bool,
}

#[tauri::command(rename_all = "camelCase")]
pub async fn select_diagnostic_destination(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
    suggested_name: String,
) -> Result<Option<String>, AppError> {
    let filename = safe_diagnostic_filename(&suggested_name);
    let selected = tauri::async_runtime::spawn_blocking(move || {
        app.dialog()
            .file()
            .add_filter("ZIP archive", &["zip"])
            .set_file_name(filename)
            .blocking_save_file()
    })
    .await
    .map_err(|_| AppError::internal("The diagnostic destination dialog did not complete."))?;
    match selected {
        Some(path) => {
            let mut path = path.into_path().map_err(|_| {
                AppError::destination_denied("The selected diagnostic path is not supported.")
            })?;
            if path.extension().is_none() {
                path.set_extension("zip");
            }
            let authorized = paths.authorize_diagnostic_file(&path)?;
            Ok(Some(path_to_string(&authorized)?))
        }
        None => Ok(None),
    }
}

#[tauri::command(rename_all = "camelCase")]
pub async fn create_diagnostic_bundle(
    app: AppHandle,
    paths: State<'_, PathPolicy>,
    diagnostics: State<'_, Diagnostics>,
    destination_zip_path: String,
    project_path: Option<String>,
) -> Result<crate::services::diagnostics::DiagnosticBundle, AppError> {
    let destination =
        paths.require_diagnostic_file(PathBuf::from(destination_zip_path).as_path())?;
    let project = project_path
        .map(PathBuf::from)
        .map(|project| paths.require_existing_file(&project))
        .transpose()?;
    let resource_dir = app.path().resource_dir().ok();
    let runtime = match media_tools::collect_runtime_info(
        app.package_info().version.to_string(),
        resource_dir.as_deref(),
    ) {
        Ok(runtime) => to_value(runtime)
            .unwrap_or_else(|_| json!({"status": "unavailable", "code": "E_INTERNAL"})),
        Err(error) => json!({
            "status": "unavailable",
            "error": error
        }),
    };
    diagnostics.record("info", "diagnostic_bundle_requested", None);
    let bundle = diagnostics.create_bundle(&destination, project.as_deref(), &runtime)?;
    diagnostics.record("info", "diagnostic_bundle_created", None);
    Ok(bundle)
}

#[tauri::command(rename_all = "camelCase")]
pub async fn reveal_in_folder(
    paths: State<'_, PathPolicy>,
    path: String,
) -> Result<RevealResponse, AppError> {
    let path = paths.require_reveal_file(PathBuf::from(path).as_path())?;
    tauri::async_runtime::spawn_blocking(move || reveal(&path))
        .await
        .map_err(|_| AppError::internal("The reveal request did not complete."))??;
    Ok(RevealResponse { opened: true })
}

fn reveal(path: &Path) -> Result<(), AppError> {
    let mut command = if cfg!(target_os = "windows") {
        let mut command = Command::new("explorer.exe");
        command.arg("/select,").arg(path);
        command
    } else if cfg!(target_os = "macos") {
        let mut command = Command::new("/usr/bin/open");
        command.arg("-R").arg(path);
        command
    } else {
        return Err(AppError::invalid_argument(
            "Reveal in folder is not available on this platform.",
        ));
    };
    let status = command
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .map_err(|_| {
            AppError::io(
                "The system file browser could not be opened.",
                "Open the selected destination manually.",
            )
        })?;
    if status.success() {
        Ok(())
    } else {
        Err(AppError::io(
            "The system file browser did not accept the reveal request.",
            "Open the selected destination manually.",
        ))
    }
}

fn safe_diagnostic_filename(suggested_name: &str) -> String {
    let stem: String = suggested_name
        .chars()
        .filter(|character| {
            character.is_alphanumeric() || matches!(character, ' ' | '-' | '_' | '.')
        })
        .take(100)
        .collect();
    let stem = stem
        .trim()
        .trim_end_matches(".zip")
        .trim()
        .trim_matches('.');
    format!(
        "{}.zip",
        if stem.is_empty() {
            "skulld-clip-diagnostics"
        } else {
            stem
        }
    )
}

fn path_to_string(path: &Path) -> Result<String, AppError> {
    path.to_str().map(str::to_owned).ok_or_else(|| {
        AppError::destination_denied("Diagnostic paths must be valid Unicode on this platform.")
    })
}

#[cfg(test)]
mod tests {
    use super::safe_diagnostic_filename;

    #[test]
    fn diagnostic_filenames_cannot_inject_paths() {
        assert_eq!(
            safe_diagnostic_filename("../../Private:Clip?.zip"),
            "PrivateClip.zip"
        );
        assert_eq!(safe_diagnostic_filename(" "), "skulld-clip-diagnostics.zip");
    }
}
