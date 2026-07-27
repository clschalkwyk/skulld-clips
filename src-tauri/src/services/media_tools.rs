use std::{
    env,
    ffi::OsString,
    fs,
    path::{Path, PathBuf},
    time::Duration,
};

use crate::{
    domain::{AppError, RuntimeInfo},
    services::process::{self, ProcessError},
};

const PROJECT_SCHEMA_VERSION: u32 = 1;
const TOOL_TIMEOUT: Duration = Duration::from_secs(5);
const MAX_TOOL_OUTPUT_BYTES: u64 = 64 * 1024;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum MediaTool {
    Ffmpeg,
    Ffprobe,
}

impl MediaTool {
    fn command_name(self) -> &'static str {
        match self {
            Self::Ffmpeg => "ffmpeg",
            Self::Ffprobe => "ffprobe",
        }
    }

    fn override_name(self) -> &'static str {
        match self {
            Self::Ffmpeg => "SKCF_FFMPEG_PATH",
            Self::Ffprobe => "SKCF_FFPROBE_PATH",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
struct ResolvedTool {
    path: PathBuf,
    bundled: bool,
}

pub fn collect_runtime_info(
    app_version: String,
    resource_dir: Option<&Path>,
) -> Result<RuntimeInfo, AppError> {
    let ffmpeg = resolve_media_tool(MediaTool::Ffmpeg, resource_dir)?;
    let ffprobe = resolve_media_tool(MediaTool::Ffprobe, resource_dir)?;
    let ffmpeg_version = read_tool_version(MediaTool::Ffmpeg, &ffmpeg.path)?;
    let ffprobe_version = read_tool_version(MediaTool::Ffprobe, &ffprobe.path)?;

    Ok(RuntimeInfo {
        app_version,
        project_schema_version: PROJECT_SCHEMA_VERSION,
        os: env::consts::OS.to_owned(),
        arch: env::consts::ARCH.to_owned(),
        ffmpeg_version,
        ffprobe_version,
        bundled_sidecars: ffmpeg.bundled && ffprobe.bundled,
    })
}

pub fn resolve_ffprobe_path(resource_dir: Option<&Path>) -> Result<PathBuf, AppError> {
    resolve_media_tool(MediaTool::Ffprobe, resource_dir).map(|tool| tool.path)
}

pub fn resolve_ffmpeg_path(resource_dir: Option<&Path>) -> Result<PathBuf, AppError> {
    resolve_media_tool(MediaTool::Ffmpeg, resource_dir).map(|tool| tool.path)
}

fn resolve_media_tool(
    tool: MediaTool,
    resource_dir: Option<&Path>,
) -> Result<ResolvedTool, AppError> {
    if cfg!(debug_assertions) {
        if let Some(path) = env::var_os(tool.override_name()) {
            return validate_tool_path(tool, PathBuf::from(path), false);
        }

        if let Some(path) = find_on_path(tool.command_name(), env::var_os("PATH").as_ref()) {
            return Ok(ResolvedTool {
                path,
                bundled: false,
            });
        }
    }

    if let Some(resource_dir) = resource_dir {
        let path = resource_dir
            .join("binaries")
            .join(bundled_filename(tool.command_name()));
        if path.is_file() {
            return validate_tool_path(tool, path, true);
        }
    }

    Err(AppError::media_tool_failed(
        tool.command_name(),
        format!(
            "Install {} for development or set {} to an absolute executable path.",
            tool.command_name(),
            tool.override_name()
        ),
    ))
}

fn validate_tool_path(
    tool: MediaTool,
    path: PathBuf,
    bundled: bool,
) -> Result<ResolvedTool, AppError> {
    if !path.is_absolute() || !path.is_file() {
        return Err(AppError::media_tool_failed(
            tool.command_name(),
            format!(
                "{} must reference an existing absolute file.",
                tool.override_name()
            ),
        ));
    }

    let canonical = fs::canonicalize(path).map_err(|_| {
        AppError::media_tool_failed(
            tool.command_name(),
            "The configured media-tool path could not be resolved.",
        )
    })?;

    Ok(ResolvedTool {
        path: canonical,
        bundled,
    })
}

fn find_on_path(program: &str, path_value: Option<&OsString>) -> Option<PathBuf> {
    let path_value = path_value?;

    env::split_paths(path_value)
        .flat_map(|directory| executable_candidates(&directory, program))
        .find(|candidate| candidate.is_file())
        .and_then(|candidate| fs::canonicalize(candidate).ok())
}

fn executable_candidates(directory: &Path, program: &str) -> Vec<PathBuf> {
    #[cfg(target_os = "windows")]
    {
        vec![
            directory.join(format!("{program}.exe")),
            directory.join(program),
        ]
    }

    #[cfg(not(target_os = "windows"))]
    {
        vec![directory.join(program)]
    }
}

fn bundled_filename(program: &str) -> String {
    let extension = if cfg!(target_os = "windows") {
        ".exe"
    } else {
        ""
    };

    format!("{program}-{}{extension}", target_triple())
}

fn target_triple() -> &'static str {
    #[cfg(all(target_os = "windows", target_arch = "x86_64"))]
    {
        "x86_64-pc-windows-msvc"
    }
    #[cfg(all(target_os = "macos", target_arch = "aarch64"))]
    {
        "aarch64-apple-darwin"
    }
    #[cfg(all(target_os = "macos", target_arch = "x86_64"))]
    {
        "x86_64-apple-darwin"
    }
    #[cfg(not(any(
        all(target_os = "windows", target_arch = "x86_64"),
        all(target_os = "macos", target_arch = "aarch64"),
        all(target_os = "macos", target_arch = "x86_64")
    )))]
    {
        "unsupported-target"
    }
}

fn read_tool_version(tool: MediaTool, path: &Path) -> Result<String, AppError> {
    let output = process::run_bounded(
        path,
        &[OsString::from("-version")],
        TOOL_TIMEOUT,
        MAX_TOOL_OUTPUT_BYTES,
    )
    .map_err(|error| map_process_error(tool, error))?;

    if !output.status.success() {
        return Err(AppError::media_tool_failed(
            tool.command_name(),
            format!("The version check exited with status {}.", output.status),
        ));
    }

    parse_version(tool, &output.stdout)
        .or_else(|| parse_version(tool, &output.stderr))
        .ok_or_else(|| {
            AppError::media_tool_failed(
                tool.command_name(),
                "The version output was not recognized.",
            )
        })
}

fn map_process_error(tool: MediaTool, error: ProcessError) -> AppError {
    let detail = match error {
        ProcessError::Timeout => "The version check timed out.",
        ProcessError::OutputLimit => "The version output exceeded the safety limit.",
        ProcessError::Spawn => "The configured media tool could not be started.",
        ProcessError::MissingPipe
        | ProcessError::Wait
        | ProcessError::OutputRead
        | ProcessError::ReaderStopped => "The version check could not be completed.",
    };

    AppError::media_tool_failed(tool.command_name(), detail)
}

fn parse_version(tool: MediaTool, bytes: &[u8]) -> Option<String> {
    let line = String::from_utf8_lossy(bytes).lines().next()?.to_owned();
    let prefix = format!("{} version ", tool.command_name());

    line.strip_prefix(&prefix)?
        .split_whitespace()
        .next()
        .map(ToOwned::to_owned)
}

#[cfg(test)]
mod tests {
    use std::{
        ffi::OsString,
        fs,
        time::{SystemTime, UNIX_EPOCH},
    };

    use super::{find_on_path, parse_version, MediaTool};

    #[test]
    fn parses_ffmpeg_and_ffprobe_versions() {
        assert_eq!(
            parse_version(
                MediaTool::Ffmpeg,
                b"ffmpeg version 8.1 Copyright (c) the FFmpeg developers\n"
            ),
            Some("8.1".to_owned())
        );
        assert_eq!(
            parse_version(
                MediaTool::Ffprobe,
                b"ffprobe version n7.1.1-static Copyright (c)\n"
            ),
            Some("n7.1.1-static".to_owned())
        );
    }

    #[test]
    fn rejects_unrecognized_version_output() {
        assert_eq!(
            parse_version(MediaTool::Ffmpeg, b"not an ffmpeg response\n"),
            None
        );
    }

    #[test]
    fn resolves_a_tool_from_an_explicit_path_list() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let directory = std::env::temp_dir().join(format!("skcf-path-test-{unique}"));
        fs::create_dir_all(&directory).unwrap();

        #[cfg(target_os = "windows")]
        let executable = directory.join("ffprobe.exe");
        #[cfg(not(target_os = "windows"))]
        let executable = directory.join("ffprobe");

        fs::write(&executable, b"fixture").unwrap();
        let path_value = OsString::from(directory.as_os_str());

        let resolved = find_on_path("ffprobe", Some(&path_value));

        assert_eq!(resolved, Some(fs::canonicalize(&executable).unwrap()));
        fs::remove_file(executable).unwrap();
        fs::remove_dir(directory).unwrap();
    }
}
