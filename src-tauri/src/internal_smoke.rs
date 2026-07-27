use std::{
    env, fs,
    path::{Path, PathBuf},
    time::Duration,
};

use uuid::Uuid;

use crate::{
    domain::{AppError, FrameRateMode, QualityMode},
    ffmpeg::args::build_ffmpeg_args,
    services::{export, media_tools, process, projects},
};

const MAX_SMOKE_OUTPUT_BYTES: u64 = 256 * 1024;
const SMOKE_TIMEOUT: Duration = Duration::from_secs(90);

pub fn run() -> Result<(), String> {
    if env::var("SKCF_INTERNAL_EXPORT_SMOKE").as_deref() != Ok("1") {
        return Err("SKCF_INTERNAL_EXPORT_SMOKE must equal 1.".to_owned());
    }
    let source = required_path("SKCF_INTERNAL_SMOKE_SOURCE")?;
    let destination = required_path("SKCF_INTERNAL_SMOKE_DESTINATION")?;
    let projects_root = required_path("SKCF_INTERNAL_SMOKE_PROJECTS_ROOT")?;
    if destination.exists() {
        return Err("The internal smoke destination already exists.".to_owned());
    }
    let destination_parent = destination
        .parent()
        .ok_or_else(|| "The internal smoke destination has no parent.".to_owned())?;
    fs::create_dir_all(destination_parent)
        .map_err(|_| "The internal smoke destination folder could not be created.".to_owned())?;

    let created = projects::create_project(&source, &projects_root, Some("Installed smoke"), None)
        .map_err(app_error)?;
    let mut project = created.project;
    project.export_defaults.quality_mode = QualityMode::Draft;
    project.export_defaults.frame_rate_mode = FrameRateMode::Thirty;
    project.validate().map_err(app_error)?;

    let partial = destination_parent.join(format!(".smoke-{}.partial.mp4", Uuid::new_v4()));
    let ffmpeg = media_tools::resolve_ffmpeg_path(None).map_err(app_error)?;
    let args = build_ffmpeg_args(
        &project,
        &project.export_defaults,
        &[] as &[&Path],
        &partial,
        false,
    )
    .map_err(str::to_owned)?;
    let output = process::run_bounded(&ffmpeg, &args, SMOKE_TIMEOUT, MAX_SMOKE_OUTPUT_BYTES)
        .map_err(|error| format!("The internal smoke FFmpeg process failed: {error:?}"))?;
    if !output.status.success() {
        let _ = fs::remove_file(&partial);
        return Err(format!(
            "The internal smoke FFmpeg process exited with status {}.",
            output.status
        ));
    }
    export::verify_output(&partial, &project, None).map_err(|error| {
        let _ = fs::remove_file(&partial);
        app_error(error)
    })?;
    fs::rename(&partial, &destination).map_err(|_| {
        let _ = fs::remove_file(&partial);
        "The verified internal smoke output could not be published.".to_owned()
    })?;
    Ok(())
}

fn required_path(name: &str) -> Result<PathBuf, String> {
    env::var_os(name)
        .filter(|value| !value.is_empty())
        .map(PathBuf::from)
        .ok_or_else(|| format!("{name} is required for the internal export smoke."))
}

fn app_error(error: AppError) -> String {
    error.safe_detail.unwrap_or(error.message)
}
