use std::{ffi::OsString, path::Path};

use crate::{
    domain::{ExportSettings, ProjectV1, QualityMode},
    ffmpeg::filter_graph::{build_filter_graph, ordered_overlays, resolved_frame_rate, seconds},
};

pub fn build_ffmpeg_args(
    project: &ProjectV1,
    settings: &ExportSettings,
    asset_paths: &[impl AsRef<Path>],
    partial_path: &Path,
    overwrite: bool,
) -> Result<Vec<OsString>, &'static str> {
    if asset_paths.len() != ordered_overlays(project).len() {
        return Err("Overlay asset paths do not match the project snapshot.");
    }
    let frame_rate = resolved_frame_rate(project, settings.frame_rate_mode);
    let (preset, crf) = quality_args(settings.quality_mode);
    let mut args = vec![
        "-hide_banner".into(),
        "-loglevel".into(),
        "error".into(),
        "-nostdin".into(),
        "-noautorotate".into(),
        "-i".into(),
        project.source.path.as_str().into(),
    ];
    for asset in asset_paths {
        args.extend([
            "-loop".into(),
            "1".into(),
            "-framerate".into(),
            "1".into(),
            "-i".into(),
            asset.as_ref().as_os_str().to_owned(),
        ]);
    }
    args.extend([
        "-filter_complex".into(),
        build_filter_graph(project, frame_rate).into(),
        "-map".into(),
        "[vout]".into(),
    ]);
    if project.source.probe.has_audio {
        args.extend(["-map".into(), "[aout]".into()]);
    }
    args.extend([
        "-map_metadata".into(),
        "-1".into(),
        "-map_chapters".into(),
        "-1".into(),
        "-c:v".into(),
        "libx264".into(),
        "-preset".into(),
        preset.into(),
        "-crf".into(),
        crf.into(),
        "-profile:v".into(),
        "high".into(),
        "-pix_fmt".into(),
        "yuv420p".into(),
    ]);
    if project.source.probe.has_audio {
        args.extend([
            "-c:a".into(),
            "aac".into(),
            "-b:a".into(),
            format!("{}k", settings.audio_bitrate_kbps).into(),
            "-ar".into(),
            "48000".into(),
        ]);
    }
    args.extend([
        "-movflags".into(),
        "+faststart".into(),
        "-t".into(),
        seconds(project.timeline.out_ms - project.timeline.in_ms).into(),
        "-progress".into(),
        "pipe:1".into(),
        "-nostats".into(),
        if overwrite { "-y" } else { "-n" }.into(),
        partial_path.as_os_str().to_owned(),
    ]);
    Ok(args)
}

fn quality_args(mode: QualityMode) -> (&'static str, &'static str) {
    match mode {
        QualityMode::Draft => ("veryfast", "24"),
        QualityMode::Balanced => ("medium", "20"),
        QualityMode::High => ("slow", "18"),
    }
}

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use crate::domain::ProjectV1;

    use super::build_ffmpeg_args;

    #[test]
    fn special_character_paths_remain_single_arguments() {
        let project: ProjectV1 =
            serde_json::from_str(include_str!("../../../examples/example-project.skcf.json"))
                .unwrap();
        let paths = [
            PathBuf::from("/tmp/caption ü.png"),
            PathBuf::from("/tmp/logo's image.png"),
        ];
        let output = PathBuf::from("/tmp/Skull'd result ü.partial.mp4");
        let args =
            build_ffmpeg_args(&project, &project.export_defaults, &paths, &output, false).unwrap();

        assert!(args.contains(&paths[0].as_os_str().to_owned()));
        assert!(args.contains(&paths[1].as_os_str().to_owned()));
        assert_eq!(args.last(), Some(&output.as_os_str().to_owned()));
        assert!(args.contains(&OsString::from("-n")));
    }

    use std::ffi::OsString;
}
