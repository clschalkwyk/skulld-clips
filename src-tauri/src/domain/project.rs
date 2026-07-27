use std::{
    collections::HashSet,
    path::{Component, Path},
};

use chrono::DateTime;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::domain::{AppError, MediaProbe};

pub const PROJECT_SCHEMA_VERSION: u32 = 1;
pub const PROJECT_FILENAME: &str = "project.skcf.json";
pub const MIN_TRIM_DURATION_MS: u64 = 250;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct NormalizedRect {
    pub x: f64,
    pub y: f64,
    pub width: f64,
    pub height: f64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SourceFingerprint {
    pub size_bytes: u64,
    pub modified_at_ms: u64,
    pub first_chunk_sha256: String,
    pub last_chunk_sha256: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectSource {
    pub path: String,
    pub filename: String,
    pub fingerprint: SourceFingerprint,
    pub probe: MediaProbe,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Timeline {
    pub in_ms: u64,
    pub out_ms: u64,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Canvas {
    pub width: u32,
    pub height: u32,
    pub background: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AssetRef {
    pub relative_path: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub mime_type: String,
    pub original_filename: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OverlayBase {
    pub id: String,
    pub name: String,
    pub position: NormalizedRect,
    pub opacity: f64,
    pub start_ms: u64,
    pub end_ms: u64,
    pub z_index: u16,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum Overlay {
    Image {
        #[serde(flatten)]
        base: OverlayBase,
        asset: AssetRef,
    },
    Caption {
        #[serde(flatten)]
        base: OverlayBase,
        caption: CaptionStyle,
        #[serde(rename = "generatedAsset")]
        generated_asset: AssetRef,
    },
}

impl Overlay {
    pub fn asset(&self) -> (&AssetRef, ProjectAssetKind) {
        match self {
            Self::Image { asset, .. } => (asset, ProjectAssetKind::Overlay),
            Self::Caption {
                generated_asset, ..
            } => (generated_asset, ProjectAssetKind::Caption),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ProjectAssetKind {
    Overlay,
    Caption,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CaptionStyle {
    pub text: String,
    pub font_family: String,
    pub font_size_px: f64,
    pub font_weight: u16,
    pub align: TextAlign,
    pub line_height: f64,
    pub max_width_px: f64,
    pub fill: String,
    pub outline_width_px: f64,
    pub outline_color: String,
    pub background_enabled: bool,
    pub background_color: String,
    pub padding_px: f64,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TextAlign {
    Left,
    Center,
    Right,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum PresetId {
    #[serde(rename = "vertical-generic")]
    VerticalGeneric,
    #[serde(rename = "youtube-shorts")]
    YoutubeShorts,
    #[serde(rename = "instagram-reels")]
    InstagramReels,
    #[serde(rename = "tiktok")]
    Tiktok,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum QualityMode {
    Draft,
    Balanced,
    High,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
pub enum FrameRateMode {
    #[serde(rename = "source-capped-60")]
    SourceCapped60,
    #[serde(rename = "30")]
    Thirty,
    #[serde(rename = "60")]
    Sixty,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ExportSettings {
    pub preset_id: PresetId,
    pub quality_mode: QualityMode,
    pub frame_rate_mode: FrameRateMode,
    pub video_codec: String,
    pub pixel_format: String,
    pub audio_codec: String,
    pub audio_bitrate_kbps: u16,
}

impl Default for ExportSettings {
    fn default() -> Self {
        Self {
            preset_id: PresetId::VerticalGeneric,
            quality_mode: QualityMode::Balanced,
            frame_rate_mode: FrameRateMode::SourceCapped60,
            video_codec: "h264".to_owned(),
            pixel_format: "yuv420p".to_owned(),
            audio_codec: "aac".to_owned(),
            audio_bitrate_kbps: 192,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProjectV1 {
    pub schema_version: u32,
    pub id: String,
    pub name: String,
    pub created_at: String,
    pub updated_at: String,
    pub source: ProjectSource,
    pub timeline: Timeline,
    pub canvas: Canvas,
    pub crop: NormalizedRect,
    pub overlays: Vec<Overlay>,
    pub export_defaults: ExportSettings,
}

impl ProjectV1 {
    pub fn validate(&self) -> Result<(), AppError> {
        if self.schema_version != PROJECT_SCHEMA_VERSION {
            return Err(AppError::project_schema(format!(
                "Project schema version {} is not supported.",
                self.schema_version
            )));
        }
        if Uuid::parse_str(&self.id).is_err() {
            return Err(AppError::project_schema("Project ID must be a UUID."));
        }
        if self.name.trim().is_empty() || self.name.chars().count() > 120 {
            return Err(AppError::invalid_argument(
                "Project name must contain 1 to 120 characters.",
            ));
        }
        let created_at = DateTime::parse_from_rfc3339(&self.created_at).map_err(|_| {
            AppError::project_schema("Project timestamps must use RFC 3339 date-time values.")
        })?;
        let updated_at = DateTime::parse_from_rfc3339(&self.updated_at).map_err(|_| {
            AppError::project_schema("Project timestamps must use RFC 3339 date-time values.")
        })?;
        if updated_at < created_at {
            return Err(AppError::project_schema(
                "Project update time cannot precede its creation time.",
            ));
        }
        validate_probe(&self.source.probe)?;
        validate_fingerprint(&self.source.fingerprint)?;
        if self.source.probe.file_size_bytes != self.source.fingerprint.size_bytes {
            return Err(AppError::project_schema(
                "Source probe and fingerprint sizes do not match.",
            ));
        }
        if self.source.path.is_empty() || self.source.filename.is_empty() {
            return Err(AppError::project_schema(
                "Project source path and filename are required.",
            ));
        }
        if self.timeline.in_ms >= self.timeline.out_ms
            || self.timeline.out_ms > self.source.probe.duration_ms
            || self.timeline.out_ms - self.timeline.in_ms < MIN_TRIM_DURATION_MS
        {
            return Err(AppError::invalid_argument(
                "Trim must be inside the source and at least 250 milliseconds.",
            ));
        }
        if self.canvas.width != 1080
            || self.canvas.height != 1920
            || self.canvas.background != "crop-fill"
        {
            return Err(AppError::project_schema(
                "Canvas must remain 1080 by 1920 with crop-fill.",
            ));
        }
        validate_rect(&self.crop, true)?;
        validate_crop_ratio(&self.crop, &self.source.probe)?;
        if self.overlays.len() > 100 {
            return Err(AppError::invalid_argument(
                "A project may contain at most 100 overlays.",
            ));
        }
        let mut overlay_ids = HashSet::with_capacity(self.overlays.len());
        for overlay in &self.overlays {
            validate_overlay(overlay, self.timeline.in_ms, self.timeline.out_ms)?;
            let id = match overlay {
                Overlay::Image { base, .. } | Overlay::Caption { base, .. } => &base.id,
            };
            if !overlay_ids.insert(id) {
                return Err(AppError::invalid_argument(
                    "Overlay IDs must be unique within a project.",
                ));
            }
        }
        validate_export_settings(&self.export_defaults)
    }
}

pub fn centered_crop(probe: &MediaProbe) -> NormalizedRect {
    let display_width = f64::from(probe.video.display_width);
    let display_height = f64::from(probe.video.display_height);
    let source_ratio = display_width / display_height;
    let output_ratio = 9.0 / 16.0;

    if source_ratio >= output_ratio {
        let width = output_ratio / source_ratio;
        NormalizedRect {
            x: round_six((1.0 - width) / 2.0),
            y: 0.0,
            width: round_six(width),
            height: 1.0,
        }
    } else {
        let height = source_ratio / output_ratio;
        NormalizedRect {
            x: 0.0,
            y: round_six((1.0 - height) / 2.0),
            width: 1.0,
            height: round_six(height),
        }
    }
}

fn validate_probe(probe: &MediaProbe) -> Result<(), AppError> {
    if probe.duration_ms < MIN_TRIM_DURATION_MS
        || probe.video.raw_width == 0
        || probe.video.raw_height == 0
        || probe.video.display_width == 0
        || probe.video.display_height == 0
        || probe.container_name.is_empty()
        || probe.video.codec.is_empty()
    {
        return Err(AppError::project_schema(
            "Project probe metadata is incomplete.",
        ));
    }
    if probe.has_audio != probe.audio.is_some() {
        return Err(AppError::project_schema(
            "Audio presence does not match audio metadata.",
        ));
    }
    if !matches!(probe.video.rotation_degrees, 0 | 90 | 180 | 270) {
        return Err(AppError::project_schema(
            "Source rotation must be 0, 90, 180, or 270 degrees.",
        ));
    }
    if probe
        .video
        .avg_frame_rate
        .into_iter()
        .chain(probe.video.real_frame_rate)
        .any(|rate| !rate.is_finite() || rate <= 0.0)
    {
        return Err(AppError::project_schema(
            "Source frame-rate metadata must be positive.",
        ));
    }
    if let Some(audio) = &probe.audio {
        if audio.codec.is_empty() || audio.sample_rate == Some(0) || audio.channels == Some(0) {
            return Err(AppError::project_schema(
                "Project audio metadata is incomplete.",
            ));
        }
    }
    Ok(())
}

fn validate_fingerprint(fingerprint: &SourceFingerprint) -> Result<(), AppError> {
    if !is_sha256(&fingerprint.first_chunk_sha256)
        || fingerprint
            .last_chunk_sha256
            .as_ref()
            .is_some_and(|hash| !is_sha256(hash))
    {
        return Err(AppError::project_schema(
            "Source fingerprint hashes must be lowercase SHA-256.",
        ));
    }
    Ok(())
}

fn validate_rect(rect: &NormalizedRect, require_inside: bool) -> Result<(), AppError> {
    let values = [rect.x, rect.y, rect.width, rect.height];
    if values.iter().any(|value| !value.is_finite())
        || rect.x < 0.0
        || rect.y < 0.0
        || rect.width <= 0.0
        || rect.height <= 0.0
        || rect.width > 1.0
        || rect.height > 1.0
        || (require_inside && (rect.x + rect.width > 1.000_001 || rect.y + rect.height > 1.000_001))
    {
        return Err(AppError::invalid_argument(
            "Normalized rectangles must remain inside the source.",
        ));
    }
    Ok(())
}

fn validate_crop_ratio(crop: &NormalizedRect, probe: &MediaProbe) -> Result<(), AppError> {
    let pixel_width = crop.width * f64::from(probe.video.display_width);
    let pixel_height = crop.height * f64::from(probe.video.display_height);
    let ratio = pixel_width / pixel_height;
    if (ratio - (9.0 / 16.0)).abs() > 0.002 {
        return Err(AppError::invalid_argument(
            "Crop must preserve the locked 9:16 aspect ratio.",
        ));
    }
    Ok(())
}

fn validate_overlay(
    overlay: &Overlay,
    timeline_in_ms: u64,
    timeline_out_ms: u64,
) -> Result<(), AppError> {
    let (base, asset, expected_parent, expected_mime): (
        &OverlayBase,
        &AssetRef,
        &str,
        Option<&str>,
    ) = match overlay {
        Overlay::Image { base, asset } => (base, asset, "assets/overlays", None),
        Overlay::Caption {
            base,
            caption,
            generated_asset,
            ..
        } => {
            validate_caption(caption)?;
            (base, generated_asset, "assets/captions", Some("image/png"))
        }
    };

    if Uuid::parse_str(&base.id).is_err()
        || base.name.trim().is_empty()
        || base.name.chars().count() > 120
        || !base.opacity.is_finite()
        || !(0.0..=1.0).contains(&base.opacity)
        || base.start_ms >= base.end_ms
        || base.start_ms < timeline_in_ms
        || base.end_ms > timeline_out_ms
        || base.z_index > 999
    {
        return Err(AppError::invalid_argument(
            "Overlay properties are outside their allowed ranges.",
        ));
    }
    validate_rect(&base.position, true)?;
    validate_asset(asset)?;
    if !Path::new(&asset.relative_path).starts_with(expected_parent)
        || expected_mime.is_some_and(|mime| asset.mime_type != mime)
    {
        return Err(AppError::project_schema(
            "Overlay assets must remain in their matching project asset folder.",
        ));
    }
    Ok(())
}

fn validate_asset(asset: &AssetRef) -> Result<(), AppError> {
    let path = Path::new(&asset.relative_path);
    let contained = !path.is_absolute()
        && path
            .components()
            .all(|component| matches!(component, Component::Normal(_) | Component::CurDir));
    if asset.relative_path.is_empty()
        || !contained
        || asset.width == 0
        || asset.height == 0
        || asset.mime_type.is_empty()
        || !is_sha256(&asset.sha256)
    {
        return Err(AppError::project_schema(
            "Project asset references must be valid project-relative files.",
        ));
    }
    Ok(())
}

fn validate_caption(caption: &CaptionStyle) -> Result<(), AppError> {
    let numbers = [
        caption.font_size_px,
        caption.line_height,
        caption.max_width_px,
        caption.outline_width_px,
        caption.padding_px,
    ];
    if caption.text.chars().count() > 500
        || caption.font_family.is_empty()
        || numbers.iter().any(|value| !value.is_finite())
        || !(12.0..=300.0).contains(&caption.font_size_px)
        || !(100..=900).contains(&caption.font_weight)
        || !(0.8..=3.0).contains(&caption.line_height)
        || !(50.0..=1080.0).contains(&caption.max_width_px)
        || !(0.0..=30.0).contains(&caption.outline_width_px)
        || !(0.0..=100.0).contains(&caption.padding_px)
    {
        return Err(AppError::invalid_argument(
            "Caption properties are outside their allowed ranges.",
        ));
    }
    Ok(())
}

fn validate_export_settings(settings: &ExportSettings) -> Result<(), AppError> {
    if settings.video_codec != "h264"
        || settings.pixel_format != "yuv420p"
        || settings.audio_codec != "aac"
        || !matches!(settings.audio_bitrate_kbps, 128 | 160 | 192 | 256)
    {
        return Err(AppError::project_schema(
            "Export settings do not match the supported baseline.",
        ));
    }
    Ok(())
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn round_six(value: f64) -> f64 {
    (value * 1_000_000.0).round() / 1_000_000.0
}

#[cfg(test)]
mod tests {
    use super::{centered_crop, NormalizedRect, ProjectV1};
    use crate::domain::{MediaProbe, VideoProbe};

    fn probe(width: u32, height: u32) -> MediaProbe {
        MediaProbe {
            duration_ms: 10_000,
            container_name: "mov,mp4".to_owned(),
            file_size_bytes: 100,
            video: VideoProbe {
                stream_index: 0,
                codec: "h264".to_owned(),
                raw_width: width,
                raw_height: height,
                display_width: width,
                display_height: height,
                rotation_degrees: 0,
                avg_frame_rate: Some(30.0),
                real_frame_rate: Some(30.0),
                pixel_format: Some("yuv420p".to_owned()),
                sample_aspect_ratio: Some("1:1".to_owned()),
            },
            has_audio: false,
            audio: None,
            warnings: Vec::new(),
        }
    }

    #[test]
    fn centers_a_nine_by_sixteen_crop_for_landscape_video() {
        assert_eq!(
            centered_crop(&probe(1920, 1080)),
            NormalizedRect {
                x: 0.341_797,
                y: 0.0,
                width: 0.316_406,
                height: 1.0,
            }
        );
    }

    #[test]
    fn centers_a_nine_by_sixteen_crop_for_narrow_portrait_video() {
        assert_eq!(
            centered_crop(&probe(720, 1600)),
            NormalizedRect {
                x: 0.0,
                y: 0.1,
                width: 1.0,
                height: 0.8,
            }
        );
    }

    #[test]
    fn authoritative_example_parses_and_passes_cross_field_validation() {
        let project: ProjectV1 =
            serde_json::from_str(include_str!("../../../examples/example-project.skcf.json"))
                .unwrap();
        project.validate().unwrap();
    }
}
