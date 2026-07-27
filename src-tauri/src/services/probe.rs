use std::{ffi::OsString, fs, path::Path, time::Duration};

use serde::Deserialize;

use crate::{
    domain::{AppError, AudioProbe, MediaProbe, VideoProbe, MIN_TRIM_DURATION_MS},
    services::{
        media_tools,
        process::{self, ProcessError},
    },
};

const PROBE_TIMEOUT: Duration = Duration::from_secs(20);
const MAX_PROBE_OUTPUT_BYTES: u64 = 4 * 1024 * 1024;

pub struct ProbedMedia {
    pub probe: MediaProbe,
    pub usable_video_streams: usize,
    pub audio_streams: usize,
}

#[derive(Debug, Deserialize)]
struct RawProbe {
    #[serde(default)]
    streams: Vec<RawStream>,
    format: Option<RawFormat>,
    error: Option<RawProbeError>,
}

#[derive(Debug, Deserialize)]
struct RawProbeError {
    string: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawFormat {
    format_name: Option<String>,
    duration: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawStream {
    index: u32,
    codec_type: Option<String>,
    codec_name: Option<String>,
    width: Option<u32>,
    height: Option<u32>,
    duration: Option<String>,
    avg_frame_rate: Option<String>,
    r_frame_rate: Option<String>,
    pix_fmt: Option<String>,
    sample_aspect_ratio: Option<String>,
    sample_rate: Option<String>,
    channels: Option<u16>,
    channel_layout: Option<String>,
    disposition: Option<RawDisposition>,
    tags: Option<RawTags>,
    side_data_list: Option<Vec<RawSideData>>,
}

#[derive(Debug, Default, Deserialize)]
struct RawDisposition {
    #[serde(default)]
    default: u8,
    #[serde(default)]
    attached_pic: u8,
}

#[derive(Debug, Deserialize)]
struct RawTags {
    rotate: Option<String>,
}

#[derive(Debug, Deserialize)]
struct RawSideData {
    rotation: Option<f64>,
}

pub fn probe_media(
    source_path: &Path,
    resource_dir: Option<&Path>,
) -> Result<MediaProbe, AppError> {
    probe_media_details(source_path, resource_dir).map(|details| details.probe)
}

pub fn probe_media_details(
    source_path: &Path,
    resource_dir: Option<&Path>,
) -> Result<ProbedMedia, AppError> {
    let metadata = fs::metadata(source_path).map_err(|_| AppError::source_missing())?;
    if !metadata.is_file() {
        return Err(AppError::invalid_argument(
            "The selected source must be a readable file.",
        ));
    }

    let ffprobe_path = media_tools::resolve_ffprobe_path(resource_dir)?;
    let args = [
        OsString::from("-v"),
        OsString::from("error"),
        OsString::from("-print_format"),
        OsString::from("json"),
        OsString::from("-show_format"),
        OsString::from("-show_streams"),
        OsString::from("-show_chapters"),
        OsString::from("-show_error"),
        source_path.as_os_str().to_owned(),
    ];
    let output = process::run_bounded(&ffprobe_path, &args, PROBE_TIMEOUT, MAX_PROBE_OUTPUT_BYTES)
        .map_err(map_process_error)?;

    if !output.status.success() {
        return Err(AppError::media_unsupported(
            "ffprobe could not read a usable media stream from the selected file.",
        ));
    }

    let raw: RawProbe = serde_json::from_slice(&output.stdout).map_err(|_| {
        AppError::media_tool_failed(
            "ffprobe",
            "ffprobe returned malformed metadata instead of valid JSON.",
        )
    })?;
    let usable_video_streams = raw
        .streams
        .iter()
        .filter(|stream| is_usable_video_stream(stream))
        .count();
    let audio_streams = raw
        .streams
        .iter()
        .filter(|stream| stream.codec_type.as_deref() == Some("audio"))
        .count();
    let probe = normalize_raw_probe_checked(raw, metadata.len())?;
    Ok(ProbedMedia {
        probe,
        usable_video_streams,
        audio_streams,
    })
}

#[cfg(test)]
pub fn normalize_probe_json(bytes: &[u8], file_size_bytes: u64) -> Result<MediaProbe, AppError> {
    let raw: RawProbe = serde_json::from_slice(bytes).map_err(|_| {
        AppError::media_tool_failed(
            "ffprobe",
            "ffprobe returned malformed metadata instead of valid JSON.",
        )
    })?;

    normalize_raw_probe_checked(raw, file_size_bytes)
}

fn normalize_raw_probe_checked(
    raw: RawProbe,
    file_size_bytes: u64,
) -> Result<MediaProbe, AppError> {
    if raw
        .error
        .as_ref()
        .and_then(|error| error.string.as_ref())
        .is_some()
    {
        return Err(AppError::media_unsupported(
            "The selected file contains a media error.",
        ));
    }

    normalize_raw_probe(raw, file_size_bytes)
}

fn normalize_raw_probe(raw: RawProbe, file_size_bytes: u64) -> Result<MediaProbe, AppError> {
    let video_stream = select_video_stream(&raw.streams).ok_or_else(|| {
        AppError::media_unsupported("No usable video stream was found in the selected file.")
    })?;
    let audio_stream = select_audio_stream(&raw.streams);
    let rotation_degrees = rotation_degrees(video_stream);
    let raw_width = video_stream.width.unwrap_or_default();
    let raw_height = video_stream.height.unwrap_or_default();
    let (display_width, display_height) = if matches!(rotation_degrees, 90 | 270) {
        (raw_height, raw_width)
    } else {
        (raw_width, raw_height)
    };
    let duration_ms = parse_duration_ms(
        raw.format
            .as_ref()
            .and_then(|format| format.duration.as_deref())
            .or(video_stream.duration.as_deref()),
    )
    .ok_or_else(|| {
        AppError::media_unsupported("The selected video does not report a usable duration.")
    })?;
    if duration_ms < MIN_TRIM_DURATION_MS {
        return Err(AppError::media_unsupported(
            "The selected video must be at least 250 milliseconds long.",
        ));
    }

    let avg_frame_rate = parse_ratio(video_stream.avg_frame_rate.as_deref());
    let real_frame_rate = parse_ratio(video_stream.r_frame_rate.as_deref());
    let audio = audio_stream.map(normalize_audio);
    let mut warnings = Vec::new();
    if audio.is_none() {
        warnings.push("Source has no audio stream; silent export is supported.".to_owned());
    }
    if rotation_degrees != 0 {
        warnings.push(format!(
            "Source orientation uses {rotation_degrees}-degree rotation metadata."
        ));
    }
    if matches!(
        (avg_frame_rate, real_frame_rate),
        (Some(avg), Some(real)) if (avg - real).abs() > 0.01
    ) {
        warnings.push("Source may use a variable or non-uniform frame rate.".to_owned());
    }

    Ok(MediaProbe {
        duration_ms,
        container_name: raw
            .format
            .and_then(|format| format.format_name)
            .unwrap_or_else(|| "unknown".to_owned()),
        file_size_bytes,
        video: VideoProbe {
            stream_index: video_stream.index,
            codec: video_stream
                .codec_name
                .clone()
                .unwrap_or_else(|| "unknown".to_owned()),
            raw_width,
            raw_height,
            display_width,
            display_height,
            rotation_degrees,
            avg_frame_rate,
            real_frame_rate,
            pixel_format: video_stream.pix_fmt.clone(),
            sample_aspect_ratio: video_stream.sample_aspect_ratio.clone(),
        },
        has_audio: audio.is_some(),
        audio,
        warnings,
    })
}

fn select_video_stream(streams: &[RawStream]) -> Option<&RawStream> {
    streams
        .iter()
        .filter(|stream| is_usable_video_stream(stream))
        .find(|stream| {
            stream
                .disposition
                .as_ref()
                .is_some_and(|disposition| disposition.default == 1)
        })
        .or_else(|| streams.iter().find(|stream| is_usable_video_stream(stream)))
}

fn is_usable_video_stream(stream: &RawStream) -> bool {
    stream.codec_type.as_deref() == Some("video")
        && stream.width.unwrap_or_default() > 0
        && stream.height.unwrap_or_default() > 0
        && stream
            .disposition
            .as_ref()
            .map_or(true, |disposition| disposition.attached_pic == 0)
}

fn select_audio_stream(streams: &[RawStream]) -> Option<&RawStream> {
    streams
        .iter()
        .filter(|stream| stream.codec_type.as_deref() == Some("audio"))
        .find(|stream| {
            stream
                .disposition
                .as_ref()
                .is_some_and(|disposition| disposition.default == 1)
        })
        .or_else(|| {
            streams
                .iter()
                .find(|stream| stream.codec_type.as_deref() == Some("audio"))
        })
}

fn normalize_audio(stream: &RawStream) -> AudioProbe {
    AudioProbe {
        stream_index: stream.index,
        codec: stream
            .codec_name
            .clone()
            .unwrap_or_else(|| "unknown".to_owned()),
        sample_rate: stream
            .sample_rate
            .as_deref()
            .and_then(|value| value.parse().ok()),
        channels: stream.channels,
        channel_layout: stream.channel_layout.clone(),
    }
}

fn rotation_degrees(stream: &RawStream) -> u16 {
    if let Some(rotation) = stream
        .tags
        .as_ref()
        .and_then(|tags| tags.rotate.as_deref())
        .and_then(|value| value.parse::<f64>().ok())
    {
        return normalize_rotation(rotation);
    }

    stream
        .side_data_list
        .as_ref()
        .and_then(|list| list.iter().find_map(|side_data| side_data.rotation))
        .map(|rotation| normalize_rotation(-rotation))
        .unwrap_or(0)
}

fn normalize_rotation(rotation: f64) -> u16 {
    let rounded = (rotation / 90.0).round() as i32 * 90;
    rounded.rem_euclid(360) as u16
}

fn parse_duration_ms(value: Option<&str>) -> Option<u64> {
    let seconds = value?.parse::<f64>().ok()?;
    if !seconds.is_finite() || seconds <= 0.0 {
        return None;
    }
    Some((seconds * 1000.0).round() as u64)
}

fn parse_ratio(value: Option<&str>) -> Option<f64> {
    let (numerator, denominator) = value?.split_once('/')?;
    let numerator = numerator.parse::<f64>().ok()?;
    let denominator = denominator.parse::<f64>().ok()?;
    if denominator == 0.0 {
        return None;
    }
    let ratio = numerator / denominator;
    (ratio.is_finite() && ratio > 0.0).then_some(ratio)
}

fn map_process_error(error: ProcessError) -> AppError {
    let detail = match error {
        ProcessError::Timeout => "ffprobe exceeded the 20-second safety timeout.",
        ProcessError::OutputLimit => "ffprobe output exceeded the 4 MiB safety limit.",
        ProcessError::Spawn => "ffprobe could not be started.",
        ProcessError::MissingPipe
        | ProcessError::Wait
        | ProcessError::OutputRead
        | ProcessError::ReaderStopped => "ffprobe did not complete safely.",
    };

    AppError::media_tool_failed("ffprobe", detail)
}

#[cfg(test)]
mod tests {
    use super::normalize_probe_json;

    #[test]
    fn normalizes_default_video_and_audio_streams() {
        let probe = normalize_probe_json(
            include_bytes!("../../../fixtures/ffprobe/landscape-h264-aac.json"),
            12_345,
        )
        .unwrap();

        assert_eq!(probe.duration_ms, 15_000);
        assert_eq!(probe.video.stream_index, 0);
        assert_eq!(probe.video.display_width, 1920);
        assert_eq!(probe.video.display_height, 1080);
        assert_eq!(probe.video.rotation_degrees, 0);
        assert_eq!(probe.audio.unwrap().stream_index, 1);
        assert!(probe.has_audio);
    }

    #[test]
    fn ignores_attached_pictures_and_normalizes_rotation() {
        let probe = normalize_probe_json(
            include_bytes!("../../../fixtures/ffprobe/rotated-with-cover.json"),
            44,
        )
        .unwrap();

        assert_eq!(probe.video.stream_index, 2);
        assert_eq!(probe.video.raw_width, 1920);
        assert_eq!(probe.video.raw_height, 1080);
        assert_eq!(probe.video.display_width, 1080);
        assert_eq!(probe.video.display_height, 1920);
        assert_eq!(probe.video.rotation_degrees, 90);
        assert!(!probe.has_audio);
        assert_eq!(
            probe.warnings,
            [
                "Source has no audio stream; silent export is supported.",
                "Source orientation uses 90-degree rotation metadata.",
                "Source may use a variable or non-uniform frame rate.",
            ]
        );
    }

    #[test]
    fn rejects_malformed_and_video_less_probe_data() {
        assert!(normalize_probe_json(b"{not-json", 0).is_err());
        assert!(normalize_probe_json(br#"{"streams":[],"format":{"duration":"4.0"}}"#, 0).is_err());
    }
}
