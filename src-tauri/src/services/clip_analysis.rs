use std::{
    ffi::OsString,
    io::{self, Read},
    path::{Path, PathBuf},
    process::{Child, Command, Stdio},
    sync::{
        atomic::{AtomicBool, Ordering},
        mpsc, Arc, Mutex,
    },
    thread,
    time::{Duration, Instant},
};

use tauri::{AppHandle, Emitter, Manager};
use uuid::Uuid;
use wait_timeout::ChildExt;

use crate::{
    domain::{
        AppError, ClipAnalysisCancelledEvent, ClipAnalysisCompletedEvent, ClipAnalysisFailedEvent,
        ClipAnalysisProgressEvent, ClipCandidate, ClipEventKind,
    },
    services::{media_tools, probe},
};

const SAMPLE_WIDTH: usize = 320;
const SAMPLE_HEIGHT: usize = 180;
const SAMPLE_FPS: u64 = 2;
const FRAME_BYTES: usize = SAMPLE_WIDTH * SAMPLE_HEIGHT * 3;
const PROCESS_POLL_INTERVAL: Duration = Duration::from_millis(50);
const PROGRESS_INTERVAL: Duration = Duration::from_millis(250);
const PROCESS_EXIT_TIMEOUT: Duration = Duration::from_secs(5);
const ANALYSIS_TIMEOUT: Duration = Duration::from_secs(10 * 60);
const MAX_CANDIDATES: usize = 50;

#[derive(Clone, Default)]
pub struct ClipAnalysisRegistry {
    active: Arc<Mutex<Option<ActiveAnalysis>>>,
}

#[derive(Clone)]
struct ActiveAnalysis {
    job_id: String,
    cancel_requested: Arc<AtomicBool>,
}

pub struct PreparedClipAnalysis {
    source_path: PathBuf,
    ffmpeg_path: PathBuf,
    video_stream_index: u32,
    duration_ms: u64,
}

#[derive(Debug, Clone, Copy)]
struct FrameObservation {
    time_ms: u64,
    completion_score: f64,
    death_score: f64,
    boss_score: f64,
}

#[derive(Debug, Default, Clone, Copy)]
struct RegionStats {
    bright_density: f64,
    red_density: f64,
    gold_density: f64,
    dark_density: f64,
    edge_density: f64,
    max_bright_row_ratio: f64,
    max_red_row_ratio: f64,
    longest_red_run_ratio: f64,
    wide_red_row_density: f64,
}

impl ClipAnalysisRegistry {
    pub fn reserve(
        &self,
        job_id: String,
        cancel_requested: Arc<AtomicBool>,
    ) -> Result<(), AppError> {
        let mut active = self
            .active
            .lock()
            .map_err(|_| AppError::internal("Clip analysis state is unavailable."))?;
        if active.is_some() {
            return Err(AppError::analysis_active());
        }
        *active = Some(ActiveAnalysis {
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
            .map_err(|_| AppError::internal("Clip analysis state is unavailable."))?;
        let Some(active) = active.as_ref() else {
            return Err(AppError::analysis_not_found());
        };
        if active.job_id != job_id {
            return Err(AppError::analysis_not_found());
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
                .is_some_and(|analysis| analysis.job_id == job_id)
            {
                *active = None;
            }
        }
    }
}

pub fn prepare_clip_analysis(
    source_path: PathBuf,
    resource_dir: Option<PathBuf>,
) -> Result<PreparedClipAnalysis, AppError> {
    let details = probe::probe_media_details(&source_path, resource_dir.as_deref())?;
    let ffmpeg_path = media_tools::resolve_ffmpeg_path(resource_dir.as_deref())?;
    Ok(PreparedClipAnalysis {
        source_path,
        ffmpeg_path,
        video_stream_index: details.probe.video.stream_index,
        duration_ms: details.probe.duration_ms,
    })
}

pub fn run_clip_analysis(
    app: AppHandle,
    registry: ClipAnalysisRegistry,
    job_id: String,
    cancel_requested: Arc<AtomicBool>,
    prepared: PreparedClipAnalysis,
) {
    app.state::<crate::services::diagnostics::Diagnostics>()
        .record("info", "clip_analysis_started", None);
    let result = run_clip_analysis_inner(&cancel_requested, &prepared, |analyzed_ms, total_ms| {
        emit_progress(&app, &job_id, analyzed_ms, total_ms);
    });
    registry.finish(&job_id);
    match result {
        Ok(candidates) => {
            app.state::<crate::services::diagnostics::Diagnostics>()
                .record("info", "clip_analysis_completed", None);
            let _ = app.emit(
                "clip-analysis://completed",
                ClipAnalysisCompletedEvent {
                    event: "clip-analysis://completed",
                    job_id,
                    candidates,
                },
            );
        }
        Err(AnalysisRunError::Cancelled) => {
            app.state::<crate::services::diagnostics::Diagnostics>()
                .record("info", "clip_analysis_cancelled", None);
            let _ = app.emit(
                "clip-analysis://cancelled",
                ClipAnalysisCancelledEvent {
                    event: "clip-analysis://cancelled",
                    job_id,
                },
            );
        }
        Err(AnalysisRunError::Failed(error)) => {
            app.state::<crate::services::diagnostics::Diagnostics>()
                .record("error", "clip_analysis_failed", Some(&error));
            let _ = app.emit(
                "clip-analysis://failed",
                ClipAnalysisFailedEvent {
                    event: "clip-analysis://failed",
                    job_id,
                    error,
                },
            );
        }
    }
}

enum AnalysisRunError {
    Cancelled,
    Failed(AppError),
}

enum FrameMessage {
    Frame(Vec<u8>),
    Finished,
    Failed,
}

fn run_clip_analysis_inner(
    cancel_requested: &AtomicBool,
    prepared: &PreparedClipAnalysis,
    mut report_progress: impl FnMut(u64, u64),
) -> Result<Vec<ClipCandidate>, AnalysisRunError> {
    let mut child = spawn_sampler(prepared).map_err(AnalysisRunError::Failed)?;
    let stdout = child.child.stdout.take().ok_or_else(|| {
        AnalysisRunError::Failed(AppError::analysis_failed(
            "FFmpeg did not expose sampled video frames.",
            true,
        ))
    })?;
    let stderr = child.child.stderr.take().ok_or_else(|| {
        AnalysisRunError::Failed(AppError::analysis_failed(
            "FFmpeg diagnostic output was unavailable.",
            true,
        ))
    })?;
    let (frame_sender, frame_receiver) = mpsc::sync_channel(2);
    let frame_reader = thread::spawn(move || read_frames(stdout, frame_sender));
    let stderr_reader = thread::spawn(move || drain_output(stderr));
    let started = Instant::now();
    let mut last_progress = Instant::now() - PROGRESS_INTERVAL;
    let mut observations = Vec::new();
    let mut previous_frame: Option<Vec<u8>> = None;
    let mut frame_index = 0_u64;

    loop {
        if cancel_requested.load(Ordering::SeqCst) {
            child.terminate();
            let _ = frame_reader.join();
            let _ = stderr_reader.join();
            return Err(AnalysisRunError::Cancelled);
        }
        if started.elapsed() > ANALYSIS_TIMEOUT {
            child.terminate();
            let _ = frame_reader.join();
            let _ = stderr_reader.join();
            return Err(AnalysisRunError::Failed(AppError::analysis_failed(
                "The source scan exceeded ten minutes. Use a shorter source clip and retry.",
                true,
            )));
        }
        match frame_receiver.recv_timeout(PROCESS_POLL_INTERVAL) {
            Ok(FrameMessage::Frame(frame)) => {
                let time_ms = frame_index.saturating_mul(1_000) / SAMPLE_FPS;
                observations.push(analyze_frame(
                    &frame,
                    previous_frame.as_deref(),
                    time_ms.min(prepared.duration_ms),
                ));
                previous_frame = Some(frame);
                frame_index = frame_index.saturating_add(1);
                if last_progress.elapsed() >= PROGRESS_INTERVAL {
                    report_progress(time_ms, prepared.duration_ms);
                    last_progress = Instant::now();
                }
            }
            Ok(FrameMessage::Finished) => break,
            Ok(FrameMessage::Failed) => {
                child.terminate();
                let _ = frame_reader.join();
                let _ = stderr_reader.join();
                return Err(AnalysisRunError::Failed(AppError::analysis_failed(
                    "FFmpeg returned an incomplete sampled frame.",
                    true,
                )));
            }
            Err(mpsc::RecvTimeoutError::Timeout) => {
                if child.has_exited().map_err(AnalysisRunError::Failed)? {
                    break;
                }
            }
            Err(mpsc::RecvTimeoutError::Disconnected) => break,
        }
    }

    let status = child.wait().map_err(AnalysisRunError::Failed)?;
    let _ = frame_reader.join();
    let _ = stderr_reader.join();
    if !status.success() {
        return Err(AnalysisRunError::Failed(AppError::analysis_failed(
            "FFmpeg could not decode the source for scene analysis.",
            true,
        )));
    }
    report_progress(prepared.duration_ms, prepared.duration_ms);
    Ok(build_candidates(&observations, prepared.duration_ms))
}

struct SamplerChild {
    child: Child,
}

impl SamplerChild {
    fn terminate(&mut self) {
        let _ = self.child.kill();
        let _ = self.child.wait();
    }

    fn has_exited(&mut self) -> Result<bool, AppError> {
        self.child
            .try_wait()
            .map(|status| status.is_some())
            .map_err(|_| {
                AppError::analysis_failed("The FFmpeg analysis process became unavailable.", true)
            })
    }

    fn wait(&mut self) -> Result<std::process::ExitStatus, AppError> {
        match self.child.wait_timeout(PROCESS_EXIT_TIMEOUT) {
            Ok(Some(status)) => Ok(status),
            Ok(None) => {
                self.terminate();
                Err(AppError::analysis_failed(
                    "The FFmpeg analysis process did not finish cleanly.",
                    true,
                ))
            }
            Err(_) => Err(AppError::analysis_failed(
                "The FFmpeg analysis process could not be finalized.",
                true,
            )),
        }
    }
}

impl Drop for SamplerChild {
    fn drop(&mut self) {
        if self.child.try_wait().ok().flatten().is_none() {
            self.terminate();
        }
    }
}

fn spawn_sampler(prepared: &PreparedClipAnalysis) -> Result<SamplerChild, AppError> {
    let args = sampler_args(
        &prepared.source_path,
        prepared.video_stream_index,
        SAMPLE_WIDTH,
        SAMPLE_HEIGHT,
        SAMPLE_FPS,
    );
    let child = Command::new(&prepared.ffmpeg_path)
        .args(args)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|_| AppError::analysis_failed("FFmpeg could not start the source scan.", true))?;
    Ok(SamplerChild { child })
}

fn sampler_args(
    source_path: &Path,
    stream_index: u32,
    width: usize,
    height: usize,
    fps: u64,
) -> Vec<OsString> {
    vec![
        OsString::from("-hide_banner"),
        OsString::from("-loglevel"),
        OsString::from("error"),
        OsString::from("-nostdin"),
        OsString::from("-i"),
        source_path.as_os_str().to_owned(),
        OsString::from("-map"),
        OsString::from(format!("0:{stream_index}")),
        OsString::from("-vf"),
        OsString::from(format!(
            "fps={fps},scale={width}:{height}:flags=fast_bilinear,format=rgb24"
        )),
        OsString::from("-an"),
        OsString::from("-sn"),
        OsString::from("-dn"),
        OsString::from("-f"),
        OsString::from("rawvideo"),
        OsString::from("-pix_fmt"),
        OsString::from("rgb24"),
        OsString::from("pipe:1"),
    ]
}

fn read_frames(mut stdout: impl Read, sender: mpsc::SyncSender<FrameMessage>) {
    loop {
        let mut frame = vec![0_u8; FRAME_BYTES];
        let mut offset = 0;
        while offset < frame.len() {
            match stdout.read(&mut frame[offset..]) {
                Ok(0) if offset == 0 => {
                    let _ = sender.send(FrameMessage::Finished);
                    return;
                }
                Ok(0) => {
                    let _ = sender.send(FrameMessage::Failed);
                    return;
                }
                Ok(read) => offset += read,
                Err(error) if error.kind() == io::ErrorKind::Interrupted => continue,
                Err(_) => {
                    let _ = sender.send(FrameMessage::Failed);
                    return;
                }
            }
        }
        if sender.send(FrameMessage::Frame(frame)).is_err() {
            return;
        }
    }
}

fn drain_output(mut reader: impl Read) {
    let mut buffer = [0_u8; 4096];
    let mut total = 0_usize;
    while total <= 64 * 1024 {
        match reader.read(&mut buffer) {
            Ok(0) | Err(_) => break,
            Ok(read) => total += read,
        }
    }
}

fn emit_progress(app: &AppHandle, job_id: &str, analyzed_ms: u64, total_ms: u64) {
    let progress = if total_ms == 0 {
        0.0
    } else {
        (analyzed_ms as f64 / total_ms as f64).clamp(0.0, 1.0)
    };
    let _ = app.emit(
        "clip-analysis://progress",
        ClipAnalysisProgressEvent {
            event: "clip-analysis://progress",
            job_id: job_id.to_owned(),
            progress,
            analyzed_ms: analyzed_ms.min(total_ms),
            total_ms,
        },
    );
}

fn analyze_frame(frame: &[u8], previous: Option<&[u8]>, time_ms: u64) -> FrameObservation {
    let emblem = region_stats(frame, 0.42, 0.58, 0.08, 0.24);
    let completion_gold_line = region_stats(frame, 0.34, 0.66, 0.15, 0.26);
    let completion_title_line = region_stats(frame, 0.30, 0.70, 0.20, 0.34);
    let death_banner = region_stats(frame, 0.30, 0.70, 0.08, 0.38);
    let central_field = region_stats(frame, 0.28, 0.72, 0.03, 0.75);
    let boss_hud = region_stats(frame, 0.25, 0.75, 0.04, 0.14);
    let frame_delta = previous
        .map(|previous| frame_difference(frame, previous))
        .unwrap_or(0.0);

    let gold = ramp(completion_gold_line.gold_density, 0.004, 0.025);
    let emblem_bright = ramp(emblem.bright_density, 0.025, 0.16);
    let title_line = ramp(completion_title_line.max_bright_row_ratio, 0.045, 0.16);
    let title_edges = ramp(completion_title_line.edge_density, 0.03, 0.12);
    let completion_dark = ramp(central_field.dark_density, 0.24, 0.62);
    let completion_emblem_red = ramp(emblem.red_density, 0.04, 0.18);
    let scene_change = ramp(frame_delta, 0.08, 0.28);
    let completion_score = clamp_score(
        (0.30 * emblem_bright
            + 0.25 * gold
            + 0.20 * title_line
            + 0.10 * title_edges
            + 0.10 * completion_dark
            + 0.05 * scene_change)
            * (0.45 + 0.55 * completion_emblem_red),
    );

    let death_title = ramp(death_banner.max_bright_row_ratio, 0.05, 0.17);
    let death_red = ramp(death_banner.red_density, 0.012, 0.08);
    let death_edges = ramp(death_banner.edge_density, 0.11, 0.30);
    let death_dark = ramp(central_field.dark_density, 0.32, 0.72);
    let death_gold_penalty = ramp(death_banner.gold_density, 0.008, 0.045);
    let death_emblem_penalty = ramp(emblem.bright_density, 0.03, 0.12);
    let death_score = clamp_score(
        (0.39 * death_title
            + 0.28 * death_red
            + 0.16 * death_edges
            + 0.12 * death_dark
            + 0.05 * scene_change)
            * (1.0 - 0.65 * death_gold_penalty)
            * (1.0 - 0.45 * death_emblem_penalty),
    );

    let boss_run = ramp(boss_hud.longest_red_run_ratio, 0.45, 0.78);
    let boss_row = ramp(boss_hud.max_red_row_ratio, 0.40, 0.72);
    let boss_red = ramp(boss_hud.red_density, 0.018, 0.12);
    let boss_thickness = ramp(boss_hud.wide_red_row_density, 0.025, 0.10)
        * (1.0 - ramp(boss_hud.wide_red_row_density, 0.24, 0.55));
    let broad_red_penalty = ramp(boss_hud.wide_red_row_density, 0.24, 0.70);
    let boss_score = clamp_score(
        (0.50 * boss_run + 0.25 * boss_row + 0.15 * boss_red + 0.10 * boss_thickness)
            * (1.0 - 0.80 * broad_red_penalty),
    );

    FrameObservation {
        time_ms,
        completion_score,
        death_score,
        boss_score,
    }
}

fn region_stats(frame: &[u8], left: f64, right: f64, top: f64, bottom: f64) -> RegionStats {
    let x0 = normalized_pixel(left, SAMPLE_WIDTH);
    let x1 = normalized_pixel(right, SAMPLE_WIDTH).max(x0 + 1);
    let y0 = normalized_pixel(top, SAMPLE_HEIGHT);
    let y1 = normalized_pixel(bottom, SAMPLE_HEIGHT).max(y0 + 1);
    let width = x1 - x0;
    let height = y1 - y0;
    let total = (width * height) as f64;
    let mut bright = 0_usize;
    let mut red = 0_usize;
    let mut gold = 0_usize;
    let mut dark = 0_usize;
    let mut edges = 0_usize;
    let mut max_bright_row = 0_usize;
    let mut max_red_row = 0_usize;
    let mut longest_red_run = 0_usize;
    let mut wide_red_rows = 0_usize;

    for y in y0..y1 {
        let mut row_bright = 0_usize;
        let mut row_red = 0_usize;
        let mut red_run = 0_usize;
        for x in x0..x1 {
            let (r, g, b) = pixel(frame, x, y);
            let luminance = luma(r, g, b);
            let is_bright = luminance > 190 && channel_spread(r, g, b) < 80;
            let is_red = r > 72 && (r as f64) > (g as f64 * 1.35) && (r as f64) > (b as f64 * 1.20);
            let is_gold = r > 125
                && g > 62
                && b < 115
                && (r as f64) > (g as f64 * 1.05)
                && (g as f64) > (b as f64 * 1.15);
            bright += usize::from(is_bright);
            red += usize::from(is_red);
            gold += usize::from(is_gold);
            dark += usize::from(luminance < 62);
            row_bright += usize::from(is_bright);
            row_red += usize::from(is_red);
            if is_red {
                red_run += 1;
                longest_red_run = longest_red_run.max(red_run);
            } else {
                red_run = 0;
            }
            if x > x0 {
                let (left_r, left_g, left_b) = pixel(frame, x - 1, y);
                if luminance.abs_diff(luma(left_r, left_g, left_b)) > 52 {
                    edges += 1;
                }
            }
        }
        max_bright_row = max_bright_row.max(row_bright);
        max_red_row = max_red_row.max(row_red);
        wide_red_rows += usize::from(row_red as f64 / width as f64 >= 0.30);
    }

    RegionStats {
        bright_density: bright as f64 / total,
        red_density: red as f64 / total,
        gold_density: gold as f64 / total,
        dark_density: dark as f64 / total,
        edge_density: edges as f64 / total,
        max_bright_row_ratio: max_bright_row as f64 / width as f64,
        max_red_row_ratio: max_red_row as f64 / width as f64,
        longest_red_run_ratio: longest_red_run as f64 / width as f64,
        wide_red_row_density: wide_red_rows as f64 / height as f64,
    }
}

fn build_candidates(observations: &[FrameObservation], duration_ms: u64) -> Vec<ClipCandidate> {
    let mut candidates = Vec::new();
    candidates.extend(group_candidates(
        observations,
        ClipEventKind::Completion,
        0.56,
        2,
        1_500,
        duration_ms,
    ));
    candidates.extend(group_candidates(
        observations,
        ClipEventKind::Death,
        0.60,
        2,
        1_500,
        duration_ms,
    ));
    candidates.extend(group_candidates(
        observations,
        ClipEventKind::BossEncounter,
        0.58,
        4,
        1_500,
        duration_ms,
    ));
    candidates.sort_by_key(|candidate| candidate.event_ms);
    if candidates.len() > MAX_CANDIDATES {
        candidates.sort_by(|left, right| {
            right
                .confidence
                .total_cmp(&left.confidence)
                .then_with(|| left.event_ms.cmp(&right.event_ms))
        });
        candidates.truncate(MAX_CANDIDATES);
        candidates.sort_by_key(|candidate| candidate.event_ms);
    }
    candidates
}

fn group_candidates(
    observations: &[FrameObservation],
    kind: ClipEventKind,
    threshold: f64,
    minimum_hits: usize,
    maximum_gap_ms: u64,
    duration_ms: u64,
) -> Vec<ClipCandidate> {
    let mut hits = observations
        .iter()
        .filter_map(|observation| {
            let score = match kind {
                ClipEventKind::Completion => observation.completion_score,
                ClipEventKind::Death => observation.death_score,
                ClipEventKind::BossEncounter => observation.boss_score,
            };
            (score >= threshold).then_some((observation.time_ms, score))
        })
        .peekable();
    let mut groups: Vec<Vec<(u64, f64)>> = Vec::new();
    while let Some(hit) = hits.next() {
        let mut group = vec![hit];
        while let Some(next) = hits.peek() {
            if next
                .0
                .saturating_sub(group.last().expect("group is not empty").0)
                > maximum_gap_ms
            {
                break;
            }
            group.push(hits.next().expect("peeked hit exists"));
        }
        if group.len() >= minimum_hits {
            groups.push(group);
        }
    }
    groups
        .into_iter()
        .map(|group| candidate_from_group(kind, &group, duration_ms))
        .collect()
}

fn candidate_from_group(
    kind: ClipEventKind,
    group: &[(u64, f64)],
    duration_ms: u64,
) -> ClipCandidate {
    let (event_ms, maximum_score) = group
        .iter()
        .copied()
        .max_by(|left, right| left.1.total_cmp(&right.1))
        .expect("candidate groups are never empty");
    let average_score = group.iter().map(|(_, score)| score).sum::<f64>() / group.len() as f64;
    let confidence = clamp_score(maximum_score * 0.7 + average_score * 0.3);
    let first_ms = group.first().expect("candidate groups are never empty").0;
    let last_ms = group.last().expect("candidate groups are never empty").0;
    let (suggested_in_ms, suggested_out_ms, evidence) = match kind {
        ClipEventKind::Completion => (
            event_ms.saturating_sub(20_000),
            event_ms.saturating_add(5_000).min(duration_ms),
            vec![
                "Large centered title treatment".to_owned(),
                "Bright emblem and gold UI signature".to_owned(),
            ],
        ),
        ClipEventKind::Death => (
            event_ms.saturating_sub(15_000),
            event_ms.saturating_add(5_000).min(duration_ms),
            vec![
                "Wide pale title over red death treatment".to_owned(),
                "Darkened gameplay field".to_owned(),
            ],
        ),
        ClipEventKind::BossEncounter => (
            first_ms.saturating_sub(5_000),
            last_ms.saturating_add(5_000).min(duration_ms),
            vec!["Persistent wide red health bar near the top HUD".to_owned()],
        ),
    };
    ClipCandidate {
        id: Uuid::new_v4().to_string(),
        kind,
        event_ms,
        suggested_in_ms,
        suggested_out_ms: suggested_out_ms.max(suggested_in_ms.saturating_add(250)),
        confidence,
        evidence,
    }
}

fn frame_difference(current: &[u8], previous: &[u8]) -> f64 {
    let mut total = 0_u64;
    let mut samples = 0_u64;
    for index in (0..FRAME_BYTES).step_by(12) {
        total = total.saturating_add(current[index].abs_diff(previous[index]) as u64);
        samples += 1;
    }
    if samples == 0 {
        0.0
    } else {
        total as f64 / (samples as f64 * 255.0)
    }
}

fn pixel(frame: &[u8], x: usize, y: usize) -> (u8, u8, u8) {
    let index = (y * SAMPLE_WIDTH + x) * 3;
    (frame[index], frame[index + 1], frame[index + 2])
}

fn luma(r: u8, g: u8, b: u8) -> u8 {
    (((r as u16 * 54) + (g as u16 * 183) + (b as u16 * 19)) >> 8) as u8
}

fn channel_spread(r: u8, g: u8, b: u8) -> u8 {
    r.max(g).max(b).saturating_sub(r.min(g).min(b))
}

fn normalized_pixel(value: f64, size: usize) -> usize {
    ((value.clamp(0.0, 1.0) * size as f64).round() as usize).min(size)
}

fn ramp(value: f64, low: f64, high: f64) -> f64 {
    ((value - low) / (high - low)).clamp(0.0, 1.0)
}

fn clamp_score(score: f64) -> f64 {
    score.clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::{
        analyze_frame, build_candidates, prepare_clip_analysis, run_clip_analysis_inner,
        sampler_args, ClipEventKind, FrameObservation, FRAME_BYTES, SAMPLE_HEIGHT, SAMPLE_WIDTH,
    };
    use std::{
        env,
        path::{Path, PathBuf},
        sync::atomic::AtomicBool,
    };

    #[test]
    fn sampler_uses_fixed_safe_arguments() {
        let args = sampler_args(Path::new("/media/source.mp4"), 2, 320, 180, 2);
        let rendered = args
            .iter()
            .map(|value| value.to_string_lossy())
            .collect::<Vec<_>>();
        assert_eq!(rendered.last().map(|value| value.as_ref()), Some("pipe:1"));
        assert!(rendered.contains(&"-nostdin".into()));
        assert!(rendered.contains(&"0:2".into()));
        assert!(rendered
            .iter()
            .any(|value| value == "fps=2,scale=320:180:flags=fast_bilinear,format=rgb24"));
        assert_eq!(
            rendered.iter().filter(|value| value.contains(';')).count(),
            0
        );
    }

    #[test]
    fn wide_top_red_bar_produces_a_boss_signal() {
        let mut frame = vec![20_u8; FRAME_BYTES];
        fill_rect(&mut frame, 40, 12, 280, 18, (180, 16, 20));
        let observation = analyze_frame(&frame, None, 1_000);
        assert!(observation.boss_score > 0.8);
        assert!(observation.completion_score < 0.56);
    }

    #[test]
    fn gold_title_and_bright_emblem_produce_a_completion_signal() {
        let mut frame = vec![34_u8; FRAME_BYTES];
        fill_rect(&mut frame, 85, 8, 235, 138, (96, 18, 28));
        fill_rect(&mut frame, 130, 18, 190, 40, (245, 240, 226));
        striped_text(&mut frame, 70, 48, 250, 58, (222, 145, 42));
        striped_text(&mut frame, 58, 66, 262, 78, (238, 232, 218));
        let observation = analyze_frame(&frame, None, 2_000);
        assert!(observation.completion_score >= 0.56);
        assert!(observation.death_score < observation.completion_score);
    }

    #[test]
    fn red_death_treatment_produces_a_death_signal() {
        let mut frame = vec![24_u8; FRAME_BYTES];
        fill_rect(&mut frame, 74, 20, 246, 64, (70, 15, 20));
        striped_text(&mut frame, 52, 45, 268, 57, (236, 232, 222));
        fill_rect(&mut frame, 92, 28, 228, 33, (155, 20, 30));
        let observation = analyze_frame(&frame, None, 3_000);
        assert!(observation.death_score >= 0.56);
        assert!(observation.death_score > observation.completion_score);
    }

    #[test]
    fn candidates_require_persistence_and_return_safe_windows() {
        let observations = vec![
            observation(10_000, 0.70, 0.1, 0.1),
            observation(10_500, 0.76, 0.1, 0.1),
            observation(30_000, 0.1, 0.1, 0.82),
            observation(30_500, 0.1, 0.1, 0.84),
            observation(31_000, 0.1, 0.1, 0.86),
            observation(31_500, 0.1, 0.1, 0.80),
        ];
        let candidates = build_candidates(&observations, 40_000);
        assert_eq!(candidates.len(), 2);
        assert_eq!(candidates[0].kind, ClipEventKind::Completion);
        assert_eq!(candidates[0].suggested_in_ms, 0);
        assert_eq!(candidates[1].kind, ClipEventKind::BossEncounter);
        assert!(candidates[1].suggested_out_ms <= 40_000);
    }

    #[test]
    fn analyzes_real_media_source_when_enabled() {
        if env::var("SKCF_RUN_CLIP_ANALYSIS_INTEGRATION").as_deref() != Ok("1") {
            return;
        }
        let source_path = env::var_os("SKCF_CLIP_ANALYSIS_SOURCE")
            .map(PathBuf::from)
            .expect("SKCF_CLIP_ANALYSIS_SOURCE must point to an authorized local media fixture");
        let prepared = prepare_clip_analysis(source_path, None)
            .unwrap_or_else(|_| panic!("the real clip analysis fixture could not be prepared"));
        let cancel_requested = AtomicBool::new(false);
        let mut analyzed_ms = 0_u64;
        let candidates =
            run_clip_analysis_inner(&cancel_requested, &prepared, |current_ms, _total_ms| {
                analyzed_ms = current_ms
            })
            .unwrap_or_else(|_| panic!("the real clip analysis fixture could not be analyzed"));
        assert_eq!(analyzed_ms, prepared.duration_ms);

        let completion_count = candidates
            .iter()
            .filter(|candidate| candidate.kind == ClipEventKind::Completion)
            .count();
        let death_count = candidates
            .iter()
            .filter(|candidate| candidate.kind == ClipEventKind::Death)
            .count();
        let boss_count = candidates
            .iter()
            .filter(|candidate| candidate.kind == ClipEventKind::BossEncounter)
            .count();
        println!(
            "real clip analysis completed: {} completion, {} death, {} boss candidates",
            completion_count, death_count, boss_count
        );
    }

    fn observation(
        time_ms: u64,
        completion_score: f64,
        death_score: f64,
        boss_score: f64,
    ) -> FrameObservation {
        FrameObservation {
            time_ms,
            completion_score,
            death_score,
            boss_score,
        }
    }

    fn fill_rect(
        frame: &mut [u8],
        left: usize,
        top: usize,
        right: usize,
        bottom: usize,
        color: (u8, u8, u8),
    ) {
        for y in top..bottom.min(SAMPLE_HEIGHT) {
            for x in left..right.min(SAMPLE_WIDTH) {
                set_pixel(frame, x, y, color);
            }
        }
    }

    fn striped_text(
        frame: &mut [u8],
        left: usize,
        top: usize,
        right: usize,
        bottom: usize,
        color: (u8, u8, u8),
    ) {
        for x in (left..right).step_by(5) {
            fill_rect(frame, x, top, (x + 3).min(right), bottom, color);
        }
    }

    fn set_pixel(frame: &mut [u8], x: usize, y: usize, color: (u8, u8, u8)) {
        let index = (y * SAMPLE_WIDTH + x) * 3;
        frame[index] = color.0;
        frame[index + 1] = color.1;
        frame[index + 2] = color.2;
    }
}
