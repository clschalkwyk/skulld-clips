use std::{
    ffi::OsString,
    fs::{self, OpenOptions},
    io::{BufRead, BufReader, Read},
    path::{Path, PathBuf},
    process::{Child, Command, ExitStatus, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;

use crate::{
    domain::{
        AppError, ExportCancelledEvent, ExportCompletedEvent, ExportFailedEvent,
        ExportProgressEvent, ExportRequest, ExportValidation, ProjectV1,
    },
    ffmpeg::{
        args::build_ffmpeg_args,
        filter_graph::{ordered_overlays, resolved_frame_rate},
        progress::{ProgressParser, ProgressSnapshot},
    },
    services::{assets, media_tools, probe, projects},
};

const GIBIBYTE: u64 = 1024 * 1024 * 1024;
const PROGRESS_INTERVAL: Duration = Duration::from_millis(100);
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(50);

#[derive(Clone, Default)]
pub struct ExportRegistry {
    active: Arc<Mutex<Option<ActiveExport>>>,
}

#[derive(Clone)]
struct ActiveExport {
    job_id: String,
    cancel_requested: Arc<AtomicBool>,
}

pub struct PreparedExport {
    pub project: ProjectV1,
    pub destination_path: PathBuf,
    pub partial_path: PathBuf,
    pub asset_paths: Vec<PathBuf>,
    pub ffmpeg_path: PathBuf,
    pub resource_dir: Option<PathBuf>,
    pub overwrite: bool,
}

pub struct VerifiedOutput {
    pub duration_ms: u64,
    pub size_bytes: u64,
}

impl ExportRegistry {
    pub fn reserve(
        &self,
        job_id: String,
        cancel_requested: Arc<AtomicBool>,
    ) -> Result<(), AppError> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| AppError::internal("Export state is unavailable."))?;
        if active.is_some() {
            return Err(AppError::export_active());
        }
        *active = Some(ActiveExport {
            job_id,
            cancel_requested,
        });
        Ok(())
    }

    pub fn is_active(&self) -> bool {
        self.active
            .lock()
            .map(|active| active.is_some())
            .unwrap_or(true)
    }

    pub fn cancel(&self, job_id: &str) -> Result<bool, AppError> {
        let active = self
            .active
            .lock()
            .map_err(|_| AppError::internal("Export state is unavailable."))?;
        let Some(active) = active.as_ref() else {
            return Err(AppError::export_not_found());
        };
        if active.job_id != job_id {
            return Err(AppError::export_not_found());
        }
        active.cancel_requested.store(true, Ordering::SeqCst);
        Ok(true)
    }

    pub fn cancel_active(&self) {
        if let Ok(active) = self.active.lock() {
            if let Some(active) = active.as_ref() {
                active.cancel_requested.store(true, Ordering::SeqCst);
            }
        }
    }

    pub fn finish(&self, job_id: &str) {
        if let Ok(mut active) = self.active.lock() {
            if active
                .as_ref()
                .is_some_and(|active| active.job_id == job_id)
            {
                *active = None;
            }
        }
    }
}

pub fn validate_export(
    request: &ExportRequest,
    project_path: &Path,
    destination_path: &Path,
    resource_dir: Option<&Path>,
    registry: &ExportRegistry,
) -> ExportValidation {
    let mut errors = Vec::new();
    let mut warnings = Vec::new();
    let estimated_bytes = Some(estimate_output_bytes(
        request.project_snapshot.timeline.out_ms - request.project_snapshot.timeline.in_ms,
        request.project_snapshot.source.probe.has_audio,
        request.settings.quality_mode,
        request.settings.audio_bitrate_kbps,
    ));
    let free_bytes = destination_path
        .parent()
        .and_then(|parent| fs2::available_space(parent).ok());

    if registry.is_active() {
        errors.push(AppError::export_active());
    }
    let mut snapshot = request.project_snapshot.clone();
    snapshot.export_defaults = request.settings.clone();
    if let Err(error) = snapshot.validate() {
        errors.push(error);
    }
    match projects::load_project(project_path) {
        Ok(persisted) => {
            if persisted.project.id != snapshot.id
                || persisted.project.created_at != snapshot.created_at
                || persisted.project.schema_version != snapshot.schema_version
                || persisted.project.source != snapshot.source
            {
                errors.push(AppError::project_schema(
                    "The export snapshot does not match the native-owned project identity.",
                ));
            }
            if persisted.source_status != projects::SourceStatus::Ok {
                errors.push(match persisted.source_status {
                    projects::SourceStatus::Missing => AppError::source_missing(),
                    projects::SourceStatus::Changed => AppError::source_changed(),
                    projects::SourceStatus::Ok => unreachable!(),
                });
            }
        }
        Err(error) => errors.push(error),
    }
    if let Err(error) = assets::validate_project_assets(project_path, &snapshot, true) {
        errors.push(error);
    }
    if destination_path.exists() && !request.overwrite {
        errors.push(AppError::output_exists());
    } else if destination_path.exists() {
        warnings
            .push("The existing destination file will be replaced after verification.".to_owned());
    }
    if let Err(error) = verify_destination_parent(destination_path) {
        errors.push(error);
    }
    if let (Some(estimate), Some(free)) = (estimated_bytes, free_bytes) {
        let required = estimate
            .saturating_mul(2)
            .max(estimate.saturating_add(GIBIBYTE));
        if free < required {
            errors.push(AppError::disk_space(format!(
                "At least {} MiB is required, including export headroom.",
                required.div_ceil(1024 * 1024)
            )));
        }
    }
    if let Err(error) = media_tools::resolve_ffmpeg_path(resource_dir) {
        errors.push(error);
    }
    if let Err(error) = media_tools::resolve_ffprobe_path(resource_dir) {
        errors.push(error);
    }

    ExportValidation {
        valid: errors.is_empty(),
        errors,
        warnings,
        estimated_bytes,
        free_bytes,
    }
}

pub fn prepare_export(
    request: ExportRequest,
    project_path: PathBuf,
    destination_path: PathBuf,
    resource_dir: Option<PathBuf>,
    registry: &ExportRegistry,
) -> Result<PreparedExport, AppError> {
    let validation = validate_export(
        &request,
        &project_path,
        &destination_path,
        resource_dir.as_deref(),
        registry,
    );
    if let Some(error) = validation.errors.into_iter().next() {
        return Err(error);
    }
    let asset_paths = ordered_overlays(&request.project_snapshot)
        .into_iter()
        .map(|overlay| {
            let (asset, _) = overlay.asset();
            assets::resolve_project_asset_path(&project_path, &asset.relative_path)
        })
        .collect::<Result<Vec<_>, _>>()?;
    let partial_path = partial_output_path(&destination_path, Uuid::new_v4());
    let ffmpeg_path = media_tools::resolve_ffmpeg_path(resource_dir.as_deref())?;
    let mut project = request.project_snapshot;
    project.export_defaults = request.settings;
    Ok(PreparedExport {
        project,
        destination_path,
        partial_path,
        asset_paths,
        ffmpeg_path,
        resource_dir,
        overwrite: request.overwrite,
    })
}

pub fn run_export(
    app: AppHandle,
    registry: ExportRegistry,
    job_id: String,
    cancel_requested: Arc<AtomicBool>,
    prepared: PreparedExport,
) {
    app.state::<crate::services::diagnostics::Diagnostics>()
        .record("info", "export_started", None);
    let total_ms = prepared.project.timeline.out_ms - prepared.project.timeline.in_ms;
    emit_progress(
        &app,
        &job_id,
        ProgressUpdate {
            phase: "preparing-assets",
            progress: 0.0,
            encoded_ms: 0,
            total_ms,
            fps: None,
            speed: None,
            output_bytes: None,
        },
    );
    let result = run_export_inner(&app, &job_id, &cancel_requested, &prepared);
    registry.finish(&job_id);
    match result {
        Ok(verified) => {
            app.state::<crate::services::diagnostics::Diagnostics>()
                .record("info", "export_completed", None);
            let _ = app.emit(
                "export://completed",
                ExportCompletedEvent {
                    event: "export://completed",
                    job_id,
                    output_path: prepared.destination_path.to_string_lossy().into_owned(),
                    duration_ms: verified.duration_ms,
                    size_bytes: verified.size_bytes,
                },
            );
        }
        Err(ExportRunError::Cancelled) => {
            app.state::<crate::services::diagnostics::Diagnostics>()
                .record("info", "export_cancelled", None);
            let _ = app.emit(
                "export://cancelled",
                ExportCancelledEvent {
                    event: "export://cancelled",
                    job_id,
                },
            );
        }
        Err(ExportRunError::Failed(error)) => {
            app.state::<crate::services::diagnostics::Diagnostics>()
                .record("error", "export_failed", Some(&error));
            let _ = app.emit(
                "export://failed",
                ExportFailedEvent {
                    event: "export://failed",
                    job_id,
                    error,
                },
            );
        }
    }
}

enum ExportRunError {
    Cancelled,
    Failed(AppError),
}

fn run_export_inner(
    app: &AppHandle,
    job_id: &str,
    cancel_requested: &AtomicBool,
    prepared: &PreparedExport,
) -> Result<VerifiedOutput, ExportRunError> {
    let _ = fs::remove_file(&prepared.partial_path);
    let args = build_ffmpeg_args(
        &prepared.project,
        &prepared.project.export_defaults,
        &prepared.asset_paths,
        &prepared.partial_path,
        prepared.overwrite,
    )
    .map_err(|detail| ExportRunError::Failed(AppError::invalid_argument(detail)))?;
    let total_ms = prepared.project.timeline.out_ms - prepared.project.timeline.in_ms;
    let mut child =
        ManagedChild::spawn(&prepared.ffmpeg_path, &args).map_err(ExportRunError::Failed)?;
    let stdout = match child.child.stdout.take() {
        Some(stdout) => stdout,
        None => {
            child.terminate_tree();
            cleanup_partial(&prepared.partial_path);
            return Err(ExportRunError::Failed(AppError::ffmpeg_failed(
                "Progress output was unavailable.",
            )));
        }
    };
    let stderr = match child.child.stderr.take() {
        Some(stderr) => stderr,
        None => {
            child.terminate_tree();
            cleanup_partial(&prepared.partial_path);
            return Err(ExportRunError::Failed(AppError::ffmpeg_failed(
                "Diagnostic output was unavailable.",
            )));
        }
    };
    let (progress_sender, progress_receiver) = mpsc::channel();
    let progress_reader = thread::spawn(move || read_progress(stdout, progress_sender));
    let stderr_reader = thread::spawn(move || drain_output(stderr));
    let timeout = Duration::from_secs(
        total_ms
            .saturating_div(1000)
            .saturating_mul(20)
            .saturating_add(120)
            .max(180),
    );
    let deadline = Instant::now() + timeout;
    let mut last_emit = Instant::now() - PROGRESS_INTERVAL;
    let status: ExitStatus;

    loop {
        while let Ok(progress) = progress_receiver.try_recv() {
            if last_emit.elapsed() >= PROGRESS_INTERVAL || progress.terminal {
                let encoded_ms = progress.encoded_ms.min(total_ms);
                emit_progress(
                    app,
                    job_id,
                    ProgressUpdate {
                        phase: "encoding",
                        progress: (encoded_ms as f64 / total_ms as f64 * 0.98).clamp(0.0, 0.98),
                        encoded_ms,
                        total_ms,
                        fps: progress.fps,
                        speed: progress.speed,
                        output_bytes: progress.output_bytes,
                    },
                );
                last_emit = Instant::now();
            }
        }
        if cancel_requested.load(Ordering::SeqCst) {
            child.terminate_tree();
            let _ = progress_reader.join();
            let _ = stderr_reader.join();
            cleanup_partial(&prepared.partial_path);
            return Err(ExportRunError::Cancelled);
        }
        if Instant::now() >= deadline {
            child.terminate_tree();
            let _ = progress_reader.join();
            let _ = stderr_reader.join();
            cleanup_partial(&prepared.partial_path);
            return Err(ExportRunError::Failed(AppError::ffmpeg_failed(
                "The export exceeded its bounded processing time.",
            )));
        }
        match child.child.try_wait() {
            Ok(Some(exit_status)) => {
                status = exit_status;
                break;
            }
            Ok(None) => thread::sleep(PROCESS_POLL_INTERVAL),
            Err(_) => {
                child.terminate_tree();
                let _ = progress_reader.join();
                let _ = stderr_reader.join();
                cleanup_partial(&prepared.partial_path);
                return Err(ExportRunError::Failed(AppError::ffmpeg_failed(
                    "The export process status could not be read.",
                )));
            }
        }
    }
    let _ = progress_reader.join();
    let _ = stderr_reader.join();
    if !status.success() {
        cleanup_partial(&prepared.partial_path);
        return Err(ExportRunError::Failed(AppError::ffmpeg_failed(format!(
            "FFmpeg exited with status {status}. Review the source and overlay formats, then retry."
        ))));
    }
    if cancel_requested.load(Ordering::SeqCst) {
        cleanup_partial(&prepared.partial_path);
        return Err(ExportRunError::Cancelled);
    }

    emit_progress(
        app,
        job_id,
        ProgressUpdate {
            phase: "verifying",
            progress: 0.99,
            encoded_ms: total_ms,
            total_ms,
            fps: None,
            speed: None,
            output_bytes: fs::metadata(&prepared.partial_path)
                .ok()
                .map(|metadata| metadata.len()),
        },
    );
    let verified = match verify_output(
        &prepared.partial_path,
        &prepared.project,
        prepared.resource_dir.as_deref(),
    ) {
        Ok(verified) => verified,
        Err(error) => {
            cleanup_partial(&prepared.partial_path);
            return Err(ExportRunError::Failed(error));
        }
    };
    if cancel_requested.load(Ordering::SeqCst) {
        cleanup_partial(&prepared.partial_path);
        return Err(ExportRunError::Cancelled);
    }
    if let Err(error) = publish_output(
        &prepared.partial_path,
        &prepared.destination_path,
        prepared.overwrite,
    ) {
        cleanup_partial(&prepared.partial_path);
        return Err(ExportRunError::Failed(error));
    }
    Ok(verified)
}

pub fn verify_output(
    path: &Path,
    project: &ProjectV1,
    resource_dir: Option<&Path>,
) -> Result<VerifiedOutput, AppError> {
    let metadata = fs::metadata(path)
        .map_err(|_| AppError::ffmpeg_failed("The partial output was not created."))?;
    if !metadata.is_file() || metadata.len() == 0 {
        return Err(AppError::ffmpeg_failed(
            "The partial output is empty or not a regular file.",
        ));
    }
    let details = probe::probe_media_details(path, resource_dir).map_err(|_| {
        AppError::ffmpeg_failed("The completed partial output could not be verified.")
    })?;
    let output = &details.probe;
    let expected_duration = project.timeline.out_ms - project.timeline.in_ms;
    let frame_rate = resolved_frame_rate(project, project.export_defaults.frame_rate_mode);
    let tolerance_ms = 20 + u64::from(1000_u32.div_ceil(frame_rate));
    let duration_difference = output.duration_ms.abs_diff(expected_duration);
    let video_valid = details.usable_video_streams == 1
        && output.video.display_width == 1080
        && output.video.display_height == 1920
        && output.video.codec == "h264"
        && output.video.rotation_degrees == 0
        && output
            .video
            .pixel_format
            .as_deref()
            .is_some_and(|format| format.starts_with("yuv420p"))
        && output.video.sample_aspect_ratio.as_deref() == Some("1:1");
    let fps_valid = output
        .video
        .avg_frame_rate
        .or(output.video.real_frame_rate)
        .is_some_and(|actual| (actual - f64::from(frame_rate)).abs() <= 0.05);
    let audio_valid = if project.source.probe.has_audio {
        details.audio_streams == 1
            && output
                .audio
                .as_ref()
                .is_some_and(|audio| audio.codec == "aac" && audio.sample_rate == Some(48_000))
    } else {
        details.audio_streams == 0 && !output.has_audio
    };
    let container_valid = output.container_name.split(',').any(|name| name == "mp4");
    if !video_valid
        || !fps_valid
        || !audio_valid
        || !container_valid
        || duration_difference > tolerance_ms
    {
        return Err(AppError::ffmpeg_failed(
            "Output verification rejected its streams, dimensions, duration, or codecs.",
        ));
    }
    Ok(VerifiedOutput {
        duration_ms: output.duration_ms,
        size_bytes: metadata.len(),
    })
}

fn estimate_output_bytes(
    duration_ms: u64,
    has_audio: bool,
    quality: crate::domain::QualityMode,
    audio_bitrate_kbps: u16,
) -> u64 {
    let video_kbps: u64 = match quality {
        crate::domain::QualityMode::Draft => 8_000,
        crate::domain::QualityMode::Balanced => 12_000,
        crate::domain::QualityMode::High => 20_000,
    };
    let total_kbps = video_kbps
        + if has_audio {
            u64::from(audio_bitrate_kbps)
        } else {
            0
        };
    total_kbps
        .saturating_mul(duration_ms)
        .saturating_div(8_000)
        .max(1024 * 1024)
}

fn verify_destination_parent(destination_path: &Path) -> Result<(), AppError> {
    let parent = destination_path.parent().ok_or_else(|| {
        AppError::destination_denied("The selected destination has no parent folder.")
    })?;
    let probe_path = parent.join(format!(".skcf-write-test-{}", Uuid::new_v4()));
    match OpenOptions::new()
        .create_new(true)
        .write(true)
        .open(&probe_path)
    {
        Ok(file) => {
            drop(file);
            fs::remove_file(&probe_path).map_err(|_| {
                AppError::destination_denied(
                    "The destination folder could not clean up a write check.",
                )
            })
        }
        Err(_) => Err(AppError::destination_denied(
            "Choose a folder where this application can create files.",
        )),
    }
}

fn partial_output_path(destination_path: &Path, job_id: Uuid) -> PathBuf {
    let filename = destination_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("export.mp4");
    destination_path.with_file_name(format!(".{filename}.{job_id}.partial.mp4"))
}

fn cleanup_partial(path: &Path) {
    if path.is_file() {
        let _ = fs::remove_file(path);
    }
}

fn publish_output(partial: &Path, destination: &Path, overwrite: bool) -> Result<(), AppError> {
    if overwrite {
        replace_output(partial, destination)
    } else {
        fs::hard_link(partial, destination).map_err(|_| {
            if destination.exists() {
                AppError::output_exists()
            } else {
                AppError::destination_denied(
                    "The verified output could not be published in the destination folder.",
                )
            }
        })?;
        if fs::remove_file(partial).is_err() {
            let _ = fs::remove_file(destination);
            return Err(AppError::destination_denied(
                "The verified output could not be finalized cleanly.",
            ));
        }
        Ok(())
    }
}

#[cfg(not(windows))]
fn replace_output(partial: &Path, destination: &Path) -> Result<(), AppError> {
    fs::rename(partial, destination).map_err(|_| {
        AppError::destination_denied(
            "The verified output could not replace the selected destination.",
        )
    })
}

#[cfg(windows)]
fn replace_output(partial: &Path, destination: &Path) -> Result<(), AppError> {
    use std::{iter, os::windows::ffi::OsStrExt};
    use windows_sys::Win32::Storage::FileSystem::{
        MoveFileExW, MOVEFILE_REPLACE_EXISTING, MOVEFILE_WRITE_THROUGH,
    };

    let source: Vec<u16> = partial
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let target: Vec<u16> = destination
        .as_os_str()
        .encode_wide()
        .chain(iter::once(0))
        .collect();
    let result = unsafe {
        MoveFileExW(
            source.as_ptr(),
            target.as_ptr(),
            MOVEFILE_REPLACE_EXISTING | MOVEFILE_WRITE_THROUGH,
        )
    };
    if result == 0 {
        Err(AppError::destination_denied(
            "The verified output could not replace the selected destination.",
        ))
    } else {
        Ok(())
    }
}

struct ProgressUpdate {
    phase: &'static str,
    progress: f64,
    encoded_ms: u64,
    total_ms: u64,
    fps: Option<f64>,
    speed: Option<f64>,
    output_bytes: Option<u64>,
}

fn emit_progress(app: &AppHandle, job_id: &str, update: ProgressUpdate) {
    let _ = app.emit(
        "export://progress",
        ExportProgressEvent {
            event: "export://progress",
            job_id: job_id.to_owned(),
            phase: update.phase,
            progress: update.progress,
            encoded_ms: update.encoded_ms,
            total_ms: update.total_ms,
            fps: update.fps,
            speed: update.speed,
            output_bytes: update.output_bytes,
        },
    );
}

fn read_progress(reader: impl Read, sender: mpsc::Sender<ProgressSnapshot>) {
    let mut reader = BufReader::new(reader);
    let mut parser = ProgressParser::default();
    let mut line = String::new();
    loop {
        line.clear();
        match reader.read_line(&mut line) {
            Ok(0) | Err(_) => break,
            Ok(_) if line.len() > 4096 => continue,
            Ok(_) => {
                if let Some(snapshot) = parser.push_line(&line) {
                    let _ = sender.send(snapshot);
                }
            }
        }
    }
}

fn drain_output(mut reader: impl Read) {
    let mut buffer = [0_u8; 8192];
    loop {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(_) => {}
        }
    }
}

struct ManagedChild {
    child: Child,
    #[cfg(windows)]
    job_handle: windows_sys::Win32::Foundation::HANDLE,
}

impl ManagedChild {
    fn spawn(program: &Path, args: &[OsString]) -> Result<Self, AppError> {
        let mut command = Command::new(program);
        command
            .args(args)
            .stdin(Stdio::null())
            .stdout(Stdio::piped())
            .stderr(Stdio::piped());
        #[cfg(unix)]
        {
            use std::os::unix::process::CommandExt;
            command.process_group(0);
        }
        #[cfg(windows)]
        {
            use std::os::windows::process::CommandExt;
            use windows_sys::Win32::System::Threading::CREATE_NEW_PROCESS_GROUP;
            command.creation_flags(CREATE_NEW_PROCESS_GROUP);
        }
        #[allow(unused_mut)]
        let mut child = command.spawn().map_err(|_| {
            AppError::ffmpeg_failed("The configured FFmpeg process could not be started.")
        })?;

        #[cfg(windows)]
        {
            use std::{ffi::c_void, mem::size_of, os::windows::io::AsRawHandle, ptr};
            use windows_sys::Win32::{
                Foundation::CloseHandle,
                System::JobObjects::{
                    AssignProcessToJobObject, CreateJobObjectW, JobObjectExtendedLimitInformation,
                    SetInformationJobObject, JOBOBJECT_EXTENDED_LIMIT_INFORMATION,
                    JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE,
                },
            };
            let job_handle = unsafe { CreateJobObjectW(ptr::null(), ptr::null()) };
            if job_handle.is_null() {
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::ffmpeg_failed(
                    "A Windows export process group could not be created.",
                ));
            }
            let mut limits = JOBOBJECT_EXTENDED_LIMIT_INFORMATION::default();
            limits.BasicLimitInformation.LimitFlags = JOB_OBJECT_LIMIT_KILL_ON_JOB_CLOSE;
            let configured = unsafe {
                SetInformationJobObject(
                    job_handle,
                    JobObjectExtendedLimitInformation,
                    &limits as *const _ as *const c_void,
                    size_of::<JOBOBJECT_EXTENDED_LIMIT_INFORMATION>() as u32,
                )
            };
            let assigned = unsafe { AssignProcessToJobObject(job_handle, child.as_raw_handle()) };
            if configured == 0 || assigned == 0 {
                unsafe {
                    CloseHandle(job_handle);
                }
                let _ = child.kill();
                let _ = child.wait();
                return Err(AppError::ffmpeg_failed(
                    "FFmpeg could not be assigned to its Windows process group.",
                ));
            }
            Ok(Self { child, job_handle })
        }

        #[cfg(not(windows))]
        Ok(Self { child })
    }

    fn terminate_tree(&mut self) {
        #[cfg(unix)]
        {
            let process_group = -(self.child.id() as i32);
            unsafe {
                libc::kill(process_group, libc::SIGTERM);
            }
            let deadline = Instant::now() + Duration::from_secs(1);
            while Instant::now() < deadline {
                if self.child.try_wait().ok().flatten().is_some() {
                    return;
                }
                thread::sleep(Duration::from_millis(50));
            }
            unsafe {
                libc::kill(process_group, libc::SIGKILL);
            }
            let _ = self.child.wait();
        }
        #[cfg(windows)]
        {
            use windows_sys::Win32::System::JobObjects::TerminateJobObject;
            unsafe {
                TerminateJobObject(self.job_handle, 1);
            }
            let _ = self.child.wait();
        }
    }
}

#[cfg(windows)]
impl Drop for ManagedChild {
    fn drop(&mut self) {
        use windows_sys::Win32::Foundation::CloseHandle;
        unsafe {
            CloseHandle(self.job_handle);
        }
    }
}

#[cfg(test)]
mod tests {
    use std::{
        fs,
        process::Command,
        sync::{
            atomic::{AtomicBool, Ordering},
            Arc,
        },
    };

    use uuid::Uuid;

    use crate::{
        domain::{AudioProbe, FrameRateMode, Overlay, ProjectV1, QualityMode},
        ffmpeg::args::build_ffmpeg_args,
        services::media_tools,
    };

    use super::{
        cleanup_partial, partial_output_path, publish_output, verify_output, ExportRegistry,
        ManagedChild,
    };

    #[test]
    fn publishing_without_overwrite_never_replaces_an_existing_file() {
        let root = std::env::temp_dir().join(format!("skcf-publish-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let destination = root.join("clip.mp4");
        let partial = partial_output_path(&destination, Uuid::new_v4());
        fs::write(&destination, b"existing").unwrap();
        fs::write(&partial, b"verified").unwrap();

        assert!(publish_output(&partial, &destination, false).is_err());
        assert_eq!(fs::read(&destination).unwrap(), b"existing");
        assert!(partial.is_file());

        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn shutdown_cancellation_marks_the_active_job_only() {
        let registry = ExportRegistry::default();
        let cancelled = Arc::new(AtomicBool::new(false));
        registry
            .reserve("job-1".to_owned(), cancelled.clone())
            .unwrap();

        registry.cancel_active();

        assert!(cancelled.load(Ordering::SeqCst));
        registry.finish("stale-job");
        assert!(registry.is_active());
        registry.finish("job-1");
        assert!(!registry.is_active());
    }

    #[test]
    fn cancellation_terminates_the_media_process_and_removes_its_partial_when_enabled() {
        if std::env::var_os("SKCF_RUN_MEDIA_INTEGRATION").is_none() {
            return;
        }
        let ffmpeg = media_tools::resolve_ffmpeg_path(None).unwrap();
        let root = std::env::temp_dir().join(format!("skcf-cancel-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let partial = root.join(".cancel-test.partial.mp4");
        let args = vec![
            "-hide_banner".into(),
            "-loglevel".into(),
            "error".into(),
            "-nostdin".into(),
            "-re".into(),
            "-f".into(),
            "lavfi".into(),
            "-i".into(),
            "testsrc=size=640x360:rate=30".into(),
            "-t".into(),
            "10".into(),
            "-c:v".into(),
            "libx264".into(),
            "-pix_fmt".into(),
            "yuv420p".into(),
            partial.as_os_str().to_owned(),
        ];
        let mut child = ManagedChild::spawn(&ffmpeg, &args).unwrap();
        std::thread::sleep(std::time::Duration::from_millis(250));

        child.terminate_tree();
        cleanup_partial(&partial);

        assert!(child.child.try_wait().unwrap().is_some());
        assert!(!partial.exists());
        fs::remove_dir_all(root).unwrap();
    }

    #[test]
    fn exports_and_verifies_a_real_media_fixture_when_enabled() {
        if std::env::var_os("SKCF_RUN_MEDIA_INTEGRATION").is_none() {
            return;
        }

        let ffmpeg = media_tools::resolve_ffmpeg_path(None).unwrap();
        let root = std::env::temp_dir().join(format!("skcf-media-export-{}", Uuid::new_v4()));
        fs::create_dir_all(&root).unwrap();
        let source = root.join("source fixture.mp4");
        let overlay = root.join("overlay fixture.png");
        let output = root.join("verified.partial.mp4");
        fs::write(&overlay, include_bytes!("../../icons/32x32.png")).unwrap();

        let fixture_status = Command::new(&ffmpeg)
            .args([
                "-hide_banner",
                "-loglevel",
                "error",
                "-f",
                "lavfi",
                "-i",
                "testsrc=size=640x360:rate=30",
                "-f",
                "lavfi",
                "-i",
                "sine=frequency=440:sample_rate=48000",
                "-t",
                "2",
                "-c:v",
                "libx264",
                "-pix_fmt",
                "yuv420p",
                "-c:a",
                "aac",
                "-shortest",
            ])
            .arg(&source)
            .status()
            .unwrap();
        assert!(fixture_status.success());

        let mut project: ProjectV1 =
            serde_json::from_str(include_str!("../../../examples/example-project.skcf.json"))
                .unwrap();
        project.source.path = source.to_string_lossy().into_owned();
        project.source.filename = "source fixture.mp4".to_owned();
        project.source.probe.duration_ms = 2_000;
        project.source.probe.container_name = "mov,mp4,m4a,3gp,3g2,mj2".to_owned();
        project.source.probe.file_size_bytes = fs::metadata(&source).unwrap().len();
        project.source.fingerprint.size_bytes = project.source.probe.file_size_bytes;
        project.source.probe.video.raw_width = 640;
        project.source.probe.video.raw_height = 360;
        project.source.probe.video.display_width = 640;
        project.source.probe.video.display_height = 360;
        project.source.probe.video.rotation_degrees = 0;
        project.source.probe.video.avg_frame_rate = Some(30.0);
        project.source.probe.video.real_frame_rate = Some(30.0);
        project.source.probe.video.pixel_format = Some("yuv420p".to_owned());
        project.source.probe.video.sample_aspect_ratio = Some("1:1".to_owned());
        project.source.probe.has_audio = true;
        project.source.probe.audio = Some(AudioProbe {
            stream_index: 1,
            codec: "aac".to_owned(),
            sample_rate: Some(48_000),
            channels: Some(1),
            channel_layout: Some("mono".to_owned()),
        });
        project.timeline.in_ms = 0;
        project.timeline.out_ms = 1_000;
        for item in &mut project.overlays {
            match item {
                Overlay::Image { base, .. } | Overlay::Caption { base, .. } => {
                    base.start_ms = 0;
                    base.end_ms = 1_000;
                }
            }
        }
        project.export_defaults.quality_mode = QualityMode::Draft;
        project.export_defaults.frame_rate_mode = FrameRateMode::Thirty;

        let args = build_ffmpeg_args(
            &project,
            &project.export_defaults,
            &[&overlay, &overlay],
            &output,
            false,
        )
        .unwrap();
        let export = Command::new(&ffmpeg).args(args).output().unwrap();
        assert!(
            export.status.success(),
            "FFmpeg export failed: {}",
            String::from_utf8_lossy(&export.stderr)
        );

        let verified = verify_output(&output, &project, None).unwrap();
        assert_eq!(verified.duration_ms, 1_000);
        assert!(verified.size_bytes > 0);

        fs::remove_dir_all(root).unwrap();
    }
}
