use std::{
    ffi::OsString,
    fs::{self, File, OpenOptions},
    io::{Read, Write},
    path::{Path, PathBuf},
    time::Duration,
};

use base64::{engine::general_purpose::STANDARD, Engine as _};
use sha2::{Digest, Sha256};
use uuid::Uuid;

use crate::{
    domain::{
        AppError, AssetRef, MediaProbe, ProjectAssetKind, ProjectV1, StingAssetRef, StingPreviewRef,
    },
    services::process,
};

const MAX_OVERLAY_BYTES: u64 = 25 * 1024 * 1024;
const MAX_STING_BYTES: u64 = 50 * 1024 * 1024;
const MAX_STING_PREVIEW_BYTES: u64 = 10 * 1024 * 1024;
const MAX_CAPTION_BYTES: usize = 10 * 1024 * 1024;
const IMAGE_HEADER_BYTES: u64 = 256 * 1024;
const MAX_IMAGE_DIMENSION: u32 = 16_384;
const MAX_IMAGE_PIXELS: u64 = 100_000_000;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct ImageInfo {
    width: u32,
    height: u32,
    mime_type: &'static str,
    extension: &'static str,
}

pub fn import_overlay_asset(
    project_path: &Path,
    source_path: &Path,
) -> Result<(AssetRef, PathBuf), AppError> {
    let bytes = read_bounded(source_path, MAX_OVERLAY_BYTES)?;
    let info = inspect_image(&bytes)?;
    let content_hash = sha256_hex(&bytes);
    let filename = format!("{content_hash}.{}", info.extension);
    let relative_path = PathBuf::from("assets").join("overlays").join(filename);
    let destination = project_asset_destination(project_path, &relative_path)?;
    let stored = materialize_content_addressed(&destination, &bytes, MAX_OVERLAY_BYTES)?;
    if sha256_hex(&stored) != content_hash {
        return Err(AppError::asset_missing(
            "An existing content-addressed overlay asset has changed.",
        ));
    }
    let stored_info = inspect_image(&stored)?;
    let original_filename = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned);

    Ok((
        AssetRef {
            relative_path: path_to_slash_string(&relative_path)?,
            sha256: content_hash,
            width: stored_info.width,
            height: stored_info.height,
            mime_type: stored_info.mime_type.to_owned(),
            original_filename,
        },
        destination,
    ))
}

pub fn write_caption_asset(
    project_path: &Path,
    content_hash: &str,
    png_bytes_base64: &str,
    width: u32,
    height: u32,
) -> Result<(AssetRef, PathBuf), AppError> {
    if !is_sha256(content_hash) {
        return Err(AppError::invalid_argument(
            "Caption content hash must be lowercase SHA-256.",
        ));
    }
    if png_bytes_base64.len() > ((MAX_CAPTION_BYTES * 4) / 3) + 8 {
        return Err(AppError::invalid_argument(
            "Rasterized caption payload exceeds the 10 MiB limit.",
        ));
    }
    let bytes = STANDARD.decode(png_bytes_base64).map_err(|_| {
        AppError::invalid_argument("Rasterized caption payload must be valid base64.")
    })?;
    if bytes.len() > MAX_CAPTION_BYTES {
        return Err(AppError::invalid_argument(
            "Rasterized caption payload exceeds the 10 MiB limit.",
        ));
    }
    let info = inspect_image(&bytes)?;
    if info.mime_type != "image/png" || info.width != width || info.height != height {
        return Err(AppError::invalid_argument(
            "Rasterized caption metadata does not match its PNG payload.",
        ));
    }

    let relative_path = PathBuf::from("assets")
        .join("captions")
        .join(format!("{content_hash}.png"));
    let destination = project_asset_destination(project_path, &relative_path)?;
    let stored = materialize_content_addressed(&destination, &bytes, MAX_CAPTION_BYTES as u64)?;
    let stored_info = inspect_image(&stored)?;
    if stored_info.mime_type != "image/png" {
        return Err(AppError::asset_missing(
            "An existing caption asset is not a valid PNG.",
        ));
    }

    Ok((
        AssetRef {
            relative_path: path_to_slash_string(&relative_path)?,
            sha256: content_hash.to_owned(),
            width: stored_info.width,
            height: stored_info.height,
            mime_type: "image/png".to_owned(),
            original_filename: None,
        },
        destination,
    ))
}

pub fn import_sting_asset(
    project_path: &Path,
    source_path: &Path,
    probe: &MediaProbe,
    ffmpeg_path: &Path,
) -> Result<(StingAssetRef, PathBuf, PathBuf), AppError> {
    let is_mp4 = probe
        .container_name
        .split(',')
        .any(|container| container == "mp4");
    if !is_mp4
        || !(1_500..=10_000).contains(&probe.duration_ms)
        || probe.video.display_width == 0
        || probe.video.display_height == 0
        || probe.video.display_width > 4096
        || probe.video.display_height > 4096
        || probe.video.rotation_degrees != 0
        || !(0.9..=1.1).contains(
            &(f64::from(probe.video.display_width) / f64::from(probe.video.display_height)),
        )
        || probe.file_size_bytes > MAX_STING_BYTES
    {
        return Err(AppError::media_unsupported(
            "Choose a square MP4 sting no longer than 10 seconds, larger than 4096 pixels, or over 50 MiB.",
        ));
    }
    let (asset, destination) = store_sting_asset(project_path, source_path, probe)?;
    let (preview, preview_destination) = generate_sting_preview(
        project_path,
        &destination,
        &asset.sha256,
        probe,
        ffmpeg_path,
    )?;

    Ok((
        StingAssetRef {
            asset,
            duration_ms: probe.duration_ms,
            has_audio: probe.has_audio,
            preview,
        },
        destination,
        preview_destination,
    ))
}

fn store_sting_asset(
    project_path: &Path,
    source_path: &Path,
    probe: &MediaProbe,
) -> Result<(AssetRef, PathBuf), AppError> {
    let bytes = read_sting_bounded(source_path)?;
    if bytes.len() as u64 != probe.file_size_bytes {
        return Err(AppError::media_unsupported(
            "The selected sting changed while it was being imported.",
        ));
    }
    let content_hash = sha256_hex(&bytes);
    let relative_path = PathBuf::from("assets")
        .join("stings")
        .join(format!("{content_hash}.mp4"));
    let destination = project_asset_destination(project_path, &relative_path)?;
    let stored = materialize_content_addressed(&destination, &bytes, MAX_STING_BYTES)?;
    if sha256_hex(&stored) != content_hash {
        return Err(AppError::asset_missing(
            "An existing content-addressed sting asset has changed.",
        ));
    }
    let original_filename = source_path
        .file_name()
        .and_then(|name| name.to_str())
        .map(str::to_owned);

    Ok((
        AssetRef {
            relative_path: path_to_slash_string(&relative_path)?,
            sha256: content_hash,
            width: probe.video.display_width,
            height: probe.video.display_height,
            mime_type: "video/mp4".to_owned(),
            original_filename,
        },
        destination,
    ))
}

fn generate_sting_preview(
    project_path: &Path,
    sting_path: &Path,
    sting_hash: &str,
    probe: &MediaProbe,
    ffmpeg_path: &Path,
) -> Result<(StingPreviewRef, PathBuf), AppError> {
    let frame_count = u32::try_from(probe.duration_ms.saturating_mul(4) / 1_000)
        .unwrap_or(40)
        .clamp(6, 40);
    let columns = f64::from(frame_count).sqrt().ceil() as u32;
    let rows = frame_count.div_ceil(columns);
    let relative_path = PathBuf::from("assets")
        .join("stings")
        .join(format!("{sting_hash}.preview.png"));
    let destination = project_asset_destination(project_path, &relative_path)?;
    if !destination.is_file() {
        let parent = destination
            .parent()
            .ok_or_else(|| AppError::project_schema("Sting preview folder is unavailable."))?;
        let temporary = parent.join(format!(".sting-preview-{}.png", Uuid::new_v4()));
        let filter = format!(
            "trim=start=0:end={:.3},setpts=(PTS-STARTPTS)/3,fps=12,chromakey=0x06EE11:0.160000:0.050000,format=rgba,scale=192:192:flags=lanczos,tile={}x{}:nb_frames={frame_count}:padding=0:margin=0:color=0x00000000",
            probe.duration_ms as f64 / 1_000.0,
            columns,
            rows,
        );
        let args: Vec<OsString> = vec![
            "-hide_banner".into(),
            "-loglevel".into(),
            "error".into(),
            "-nostdin".into(),
            "-i".into(),
            sting_path.as_os_str().to_owned(),
            "-vf".into(),
            filter.into(),
            "-frames:v".into(),
            "1".into(),
            "-an".into(),
            "-y".into(),
            temporary.as_os_str().to_owned(),
        ];
        let generated =
            process::run_bounded(ffmpeg_path, &args, Duration::from_secs(30), 64 * 1024);
        let successful = generated.is_ok_and(|output| output.status.success());
        if !successful {
            let _ = fs::remove_file(&temporary);
            return Err(AppError::media_unsupported(
                "The Skull'd sting preview could not be generated with the fixed green-key preset.",
            ));
        }
        fs::rename(&temporary, &destination).map_err(|_| {
            let _ = fs::remove_file(&temporary);
            AppError::io(
                "The Skull'd sting preview could not be saved.",
                "Check project folder permissions and available disk space.",
            )
        })?;
    }

    let bytes = read_preview_bounded(&destination)?;
    let info = inspect_image(&bytes)?;
    let expected_width = columns * 192;
    let expected_height = rows * 192;
    if info.mime_type != "image/png"
        || info.width != expected_width
        || info.height != expected_height
    {
        return Err(AppError::asset_missing(
            "The generated Skull'd sting preview no longer matches its metadata.",
        ));
    }
    Ok((
        StingPreviewRef {
            relative_path: path_to_slash_string(&relative_path)?,
            sha256: sha256_hex(&bytes),
            width: info.width,
            height: info.height,
            frame_width: 192,
            frame_height: 192,
            columns,
            rows,
            frame_count,
            frames_per_second: 12,
        },
        destination,
    ))
}

pub fn validate_project_assets(
    project_path: &Path,
    project: &ProjectV1,
    verify_overlay_hashes: bool,
) -> Result<(), AppError> {
    let project_dir = canonical_project_dir(project_path)?;
    for overlay in &project.overlays {
        let (asset, kind) = overlay.asset();
        let candidate = project_dir.join(&asset.relative_path);
        let canonical = fs::canonicalize(&candidate).map_err(|_| {
            AppError::asset_missing("Restore the missing project asset or remove its overlay.")
        })?;
        if !canonical.starts_with(&project_dir) || !canonical.is_file() {
            return Err(AppError::asset_missing(
                "A project asset resolves outside the project folder.",
            ));
        }
        let stem_matches = canonical
            .file_stem()
            .and_then(|stem| stem.to_str())
            .is_some_and(|stem| stem == asset.sha256);
        if !stem_matches {
            return Err(AppError::asset_missing(
                "A project asset no longer matches its saved metadata.",
            ));
        }

        if kind == ProjectAssetKind::Sting {
            let metadata = fs::metadata(&canonical).map_err(|_| {
                AppError::asset_missing("The project sting asset could not be inspected.")
            })?;
            if asset.mime_type != "video/mp4"
                || canonical
                    .extension()
                    .and_then(|extension| extension.to_str())
                    != Some("mp4")
                || metadata.len() == 0
                || metadata.len() > MAX_STING_BYTES
            {
                return Err(AppError::asset_missing(
                    "A project sting no longer matches its saved metadata.",
                ));
            }
            if verify_overlay_hashes {
                let bytes = read_sting_bounded(&canonical)?;
                if sha256_hex(&bytes) != asset.sha256 {
                    return Err(AppError::asset_missing(
                        "An imported sting asset has changed.",
                    ));
                }
            }
            let preview = overlay
                .sting()
                .map(|(sting, _)| &sting.preview)
                .ok_or_else(|| AppError::project_schema("Sting preview metadata is missing."))?;
            let preview_path =
                fs::canonicalize(project_dir.join(&preview.relative_path)).map_err(|_| {
                    AppError::asset_missing("Restore or re-import the missing sting preview.")
                })?;
            if !preview_path.starts_with(&project_dir) || !preview_path.is_file() {
                return Err(AppError::asset_missing(
                    "A sting preview resolves outside the project folder.",
                ));
            }
            let preview_bytes = read_preview_bounded(&preview_path)?;
            let preview_info = inspect_image(&preview_bytes)?;
            if preview_info.mime_type != "image/png"
                || preview_info.width != preview.width
                || preview_info.height != preview.height
                || sha256_hex(&preview_bytes) != preview.sha256
            {
                return Err(AppError::asset_missing(
                    "A sting preview no longer matches its saved metadata.",
                ));
            }
        } else {
            let verify_hash = verify_overlay_hashes && kind == ProjectAssetKind::Overlay;
            let bytes = if verify_hash {
                read_bounded(&canonical, MAX_OVERLAY_BYTES)?
            } else {
                read_prefix(&canonical, IMAGE_HEADER_BYTES)?
            };
            let info = inspect_image(&bytes)?;
            if info.width != asset.width
                || info.height != asset.height
                || info.mime_type != asset.mime_type
            {
                return Err(AppError::asset_missing(
                    "A project asset no longer matches its saved metadata.",
                ));
            }
            if verify_hash && sha256_hex(&bytes) != asset.sha256 {
                return Err(AppError::asset_missing(
                    "An imported overlay asset has changed.",
                ));
            }
        }
    }
    Ok(())
}

pub fn resolve_project_asset_path(
    project_path: &Path,
    relative_path: &str,
) -> Result<PathBuf, AppError> {
    let project_dir = canonical_project_dir(project_path)?;
    let canonical = fs::canonicalize(project_dir.join(relative_path)).map_err(|_| {
        AppError::asset_missing("Restore the missing project asset or remove its overlay.")
    })?;
    if !canonical.starts_with(&project_dir) || !canonical.is_file() {
        return Err(AppError::asset_missing(
            "A project asset resolves outside the project folder.",
        ));
    }
    Ok(canonical)
}

fn project_asset_destination(
    project_path: &Path,
    relative_path: &Path,
) -> Result<PathBuf, AppError> {
    let project_dir = canonical_project_dir(project_path)?;
    let parent_relative = relative_path.parent().ok_or_else(|| {
        AppError::project_schema("Project asset path does not have a parent folder.")
    })?;
    let parent = project_dir.join(parent_relative);
    fs::create_dir_all(&parent).map_err(|_| {
        AppError::io(
            "The project asset folder could not be created.",
            "Check project folder permissions and try again.",
        )
    })?;
    let canonical_parent = fs::canonicalize(&parent).map_err(|_| {
        AppError::io(
            "The project asset folder could not be opened.",
            "Check project folder permissions and try again.",
        )
    })?;
    if !canonical_parent.starts_with(&project_dir) {
        return Err(AppError::project_schema(
            "Project asset paths may not escape the project folder.",
        ));
    }
    Ok(canonical_parent.join(
        relative_path
            .file_name()
            .ok_or_else(|| AppError::project_schema("Project asset filename is missing."))?,
    ))
}

fn canonical_project_dir(project_path: &Path) -> Result<PathBuf, AppError> {
    let canonical_project = fs::canonicalize(project_path).map_err(|_| {
        AppError::io(
            "The project file could not be opened.",
            "Choose an existing readable Skull’d Clip Forge project.",
        )
    })?;
    canonical_project
        .parent()
        .map(Path::to_path_buf)
        .ok_or_else(|| AppError::project_schema("The project folder is unavailable."))
}

fn read_bounded(path: &Path, maximum: u64) -> Result<Vec<u8>, AppError> {
    let file = File::open(path)
        .map_err(|_| AppError::asset_missing("The selected image asset could not be opened."))?;
    let mut bytes = Vec::new();
    file.take(maximum + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AppError::asset_missing("The selected image asset could not be read."))?;
    if bytes.len() as u64 > maximum {
        return Err(AppError::invalid_argument(format!(
            "Image assets may not exceed {} MiB.",
            maximum / 1024 / 1024
        )));
    }
    Ok(bytes)
}

fn read_prefix(path: &Path, maximum: u64) -> Result<Vec<u8>, AppError> {
    let file = File::open(path)
        .map_err(|_| AppError::asset_missing("The selected image asset could not be opened."))?;
    let mut bytes = Vec::new();
    file.take(maximum)
        .read_to_end(&mut bytes)
        .map_err(|_| AppError::asset_missing("The selected image asset could not be read."))?;
    Ok(bytes)
}

fn read_sting_bounded(path: &Path) -> Result<Vec<u8>, AppError> {
    let file = File::open(path)
        .map_err(|_| AppError::asset_missing("The selected sting could not be opened."))?;
    let mut bytes = Vec::new();
    file.take(MAX_STING_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AppError::asset_missing("The selected sting could not be read."))?;
    if bytes.len() as u64 > MAX_STING_BYTES {
        return Err(AppError::invalid_argument(
            "Skull'd sting assets may not exceed 50 MiB.",
        ));
    }
    Ok(bytes)
}

fn read_preview_bounded(path: &Path) -> Result<Vec<u8>, AppError> {
    let file = File::open(path)
        .map_err(|_| AppError::asset_missing("The sting preview could not be opened."))?;
    let mut bytes = Vec::new();
    file.take(MAX_STING_PREVIEW_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| AppError::asset_missing("The sting preview could not be read."))?;
    if bytes.len() as u64 > MAX_STING_PREVIEW_BYTES {
        return Err(AppError::invalid_argument(
            "Generated sting previews may not exceed 10 MiB.",
        ));
    }
    Ok(bytes)
}

fn materialize_content_addressed(
    path: &Path,
    bytes: &[u8],
    maximum: u64,
) -> Result<Vec<u8>, AppError> {
    if let Ok(metadata) = fs::symlink_metadata(path) {
        if metadata.file_type().is_file() {
            return read_bounded(path, maximum);
        }
        return Err(AppError::project_schema(
            "Project asset destinations must be regular files.",
        ));
    }
    let parent = path
        .parent()
        .ok_or_else(|| AppError::project_schema("Project asset folder is unavailable."))?;
    let temporary = parent.join(format!(".asset-{}.tmp", Uuid::new_v4()));
    let result = (|| {
        let mut file = OpenOptions::new()
            .create_new(true)
            .write(true)
            .open(&temporary)?;
        file.write_all(bytes)?;
        file.sync_all()?;
        match fs::rename(&temporary, path) {
            Ok(()) => Ok(()),
            Err(_) if path.is_file() => {
                fs::remove_file(&temporary)?;
                Ok(())
            }
            Err(error) => Err(error),
        }
    })();
    if result.is_err() {
        let _ = fs::remove_file(&temporary);
    }
    result.map_err(|_| {
        AppError::io(
            "The project asset could not be saved.",
            "Check available disk space and project folder permissions.",
        )
    })?;
    read_bounded(path, maximum)
}

fn inspect_image(bytes: &[u8]) -> Result<ImageInfo, AppError> {
    let info = inspect_png(bytes)
        .or_else(|| inspect_jpeg(bytes))
        .or_else(|| inspect_webp(bytes))
        .ok_or_else(|| {
            AppError::media_unsupported(
                "Choose a valid PNG, JPEG, or WebP static image under 25 MiB.",
            )
        })?;
    let pixels = u64::from(info.width) * u64::from(info.height);
    if info.width == 0
        || info.height == 0
        || info.width > MAX_IMAGE_DIMENSION
        || info.height > MAX_IMAGE_DIMENSION
        || pixels > MAX_IMAGE_PIXELS
    {
        return Err(AppError::invalid_argument(
            "Image dimensions exceed the supported safety limits.",
        ));
    }
    Ok(info)
}

fn inspect_png(bytes: &[u8]) -> Option<ImageInfo> {
    if bytes.len() < 24 || &bytes[..8] != b"\x89PNG\r\n\x1a\n" || &bytes[12..16] != b"IHDR" {
        return None;
    }
    Some(ImageInfo {
        width: u32::from_be_bytes(bytes[16..20].try_into().ok()?),
        height: u32::from_be_bytes(bytes[20..24].try_into().ok()?),
        mime_type: "image/png",
        extension: "png",
    })
}

fn inspect_jpeg(bytes: &[u8]) -> Option<ImageInfo> {
    if bytes.len() < 4 || bytes[..2] != [0xff, 0xd8] {
        return None;
    }
    let mut offset = 2;
    while offset + 4 <= bytes.len() {
        while offset < bytes.len() && bytes[offset] == 0xff {
            offset += 1;
        }
        if offset >= bytes.len() {
            break;
        }
        let marker = bytes[offset];
        offset += 1;
        if matches!(marker, 0xd8 | 0xd9) || (0xd0..=0xd7).contains(&marker) {
            continue;
        }
        if offset + 2 > bytes.len() {
            break;
        }
        let length = usize::from(u16::from_be_bytes([bytes[offset], bytes[offset + 1]]));
        if length < 2 || offset + length > bytes.len() {
            break;
        }
        if matches!(
            marker,
            0xc0 | 0xc1
                | 0xc2
                | 0xc3
                | 0xc5
                | 0xc6
                | 0xc7
                | 0xc9
                | 0xca
                | 0xcb
                | 0xcd
                | 0xce
                | 0xcf
        ) && length >= 7
        {
            return Some(ImageInfo {
                height: u32::from(u16::from_be_bytes([bytes[offset + 3], bytes[offset + 4]])),
                width: u32::from(u16::from_be_bytes([bytes[offset + 5], bytes[offset + 6]])),
                mime_type: "image/jpeg",
                extension: "jpg",
            });
        }
        offset += length;
    }
    None
}

fn inspect_webp(bytes: &[u8]) -> Option<ImageInfo> {
    if bytes.len() < 30 || &bytes[..4] != b"RIFF" || &bytes[8..12] != b"WEBP" {
        return None;
    }
    let chunk = &bytes[12..16];
    let data = &bytes[20..];
    let (width, height) = match chunk {
        b"VP8X" if data.len() >= 10 => (
            1 + u32::from(data[4]) + (u32::from(data[5]) << 8) + (u32::from(data[6]) << 16),
            1 + u32::from(data[7]) + (u32::from(data[8]) << 8) + (u32::from(data[9]) << 16),
        ),
        b"VP8 " if data.len() >= 10 && data[3..6] == [0x9d, 0x01, 0x2a] => (
            u32::from(u16::from_le_bytes([data[6], data[7]]) & 0x3fff),
            u32::from(u16::from_le_bytes([data[8], data[9]]) & 0x3fff),
        ),
        b"VP8L" if data.len() >= 5 && data[0] == 0x2f => (
            1 + u32::from(data[1]) + ((u32::from(data[2]) & 0x3f) << 8),
            1 + (u32::from(data[2]) >> 6)
                + (u32::from(data[3]) << 2)
                + ((u32::from(data[4]) & 0x0f) << 10),
        ),
        _ => return None,
    };
    Some(ImageInfo {
        width,
        height,
        mime_type: "image/webp",
        extension: "webp",
    })
}

fn sha256_hex(bytes: &[u8]) -> String {
    format!("{:x}", Sha256::digest(bytes))
}

fn is_sha256(value: &str) -> bool {
    value.len() == 64
        && value
            .bytes()
            .all(|byte| byte.is_ascii_digit() || (b'a'..=b'f').contains(&byte))
}

fn path_to_slash_string(path: &Path) -> Result<String, AppError> {
    let pieces: Option<Vec<&str>> = path
        .components()
        .map(|component| component.as_os_str().to_str())
        .collect();
    pieces.map(|parts| parts.join("/")).ok_or_else(|| {
        AppError::invalid_argument("Project asset paths must be valid Unicode text.")
    })
}

#[cfg(test)]
mod tests {
    use std::fs;

    use base64::{engine::general_purpose::STANDARD, Engine as _};
    use uuid::Uuid;

    use crate::domain::{AudioProbe, MediaProbe, VideoProbe};

    use super::{
        import_overlay_asset, inspect_image, inspect_webp, store_sting_asset, write_caption_asset,
    };

    #[test]
    fn reads_png_dimensions_from_the_ihdr() {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&1080_u32.to_be_bytes());
        png.extend_from_slice(&1920_u32.to_be_bytes());
        let info = inspect_image(&png).unwrap();
        assert_eq!(
            (info.width, info.height, info.mime_type),
            (1080, 1920, "image/png")
        );
    }

    #[test]
    fn reads_jpeg_dimensions_from_a_start_of_frame_segment() {
        let jpeg = [
            0xff, 0xd8, 0xff, 0xc0, 0x00, 0x11, 0x08, 0x02, 0xd0, 0x05, 0x00, 0x03, 0x01, 0x11,
            0x00, 0x02, 0x11, 0x00, 0x03, 0x11, 0x00,
        ];
        let info = inspect_image(&jpeg).unwrap();
        assert_eq!(
            (info.width, info.height, info.mime_type),
            (1280, 720, "image/jpeg")
        );
    }

    #[test]
    fn reads_extended_webp_dimensions() {
        let mut webp = b"RIFF\0\0\0\0WEBPVP8X\0\0\0\0".to_vec();
        webp.extend_from_slice(&[0, 0, 0, 0, 0xff, 0x03, 0, 0xff, 0x01, 0]);
        let info = inspect_webp(&webp).unwrap();
        assert_eq!((info.width, info.height), (1024, 512));
    }

    #[test]
    fn copies_hashes_and_reuses_project_owned_assets() {
        let root = std::env::temp_dir().join(format!("skcf-assets-{}", Uuid::new_v4()));
        let project_dir = root.join(Uuid::new_v4().to_string());
        fs::create_dir_all(&project_dir).unwrap();
        let project_path = project_dir.join("project.skcf.json");
        fs::write(&project_path, b"{}").unwrap();
        let source_path = root.join("brand image.png");
        let png = png_header(320, 180);
        fs::write(&source_path, &png).unwrap();

        let (imported, imported_path) = import_overlay_asset(&project_path, &source_path).unwrap();
        assert_eq!(
            imported.relative_path,
            format!("assets/overlays/{}.png", imported.sha256)
        );
        assert_eq!((imported.width, imported.height), (320, 180));
        assert_eq!(fs::read(&imported_path).unwrap(), png);

        let content_hash = "c".repeat(64);
        let encoded = STANDARD.encode(png_header(900, 220));
        let (caption, caption_path) =
            write_caption_asset(&project_path, &content_hash, &encoded, 900, 220).unwrap();
        assert_eq!(
            caption.relative_path,
            format!("assets/captions/{content_hash}.png")
        );
        assert_eq!((caption.width, caption.height), (900, 220));
        assert!(caption_path.is_file());

        let sting_source = root.join("skulld sting.mp4");
        fs::write(&sting_source, b"bounded mp4 fixture").unwrap();
        let sting_probe = MediaProbe {
            duration_ms: 5_000,
            container_name: "mov,mp4,m4a,3gp,3g2,mj2".to_owned(),
            file_size_bytes: fs::metadata(&sting_source).unwrap().len(),
            video: VideoProbe {
                stream_index: 0,
                codec: "h264".to_owned(),
                raw_width: 832,
                raw_height: 832,
                display_width: 832,
                display_height: 832,
                rotation_degrees: 0,
                avg_frame_rate: Some(24.0),
                real_frame_rate: Some(24.0),
                pixel_format: Some("yuv420p".to_owned()),
                sample_aspect_ratio: Some("1:1".to_owned()),
            },
            has_audio: true,
            audio: Some(AudioProbe {
                stream_index: 1,
                codec: "aac".to_owned(),
                sample_rate: Some(48_000),
                channels: Some(2),
                channel_layout: Some("stereo".to_owned()),
            }),
            warnings: Vec::new(),
        };
        let (sting, sting_path) =
            store_sting_asset(&project_path, &sting_source, &sting_probe).unwrap();
        assert_eq!(
            sting.relative_path,
            format!("assets/stings/{}.mp4", sting.sha256)
        );
        assert!(sting_path.is_file());

        fs::remove_dir_all(root).unwrap();
    }

    fn png_header(width: u32, height: u32) -> Vec<u8> {
        let mut png = b"\x89PNG\r\n\x1a\n\0\0\0\rIHDR".to_vec();
        png.extend_from_slice(&width.to_be_bytes());
        png.extend_from_slice(&height.to_be_bytes());
        png
    }
}
