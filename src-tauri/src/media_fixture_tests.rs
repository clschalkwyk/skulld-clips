use std::{
    ffi::OsStr,
    fs,
    path::{Path, PathBuf},
    process::{Command, Output},
};

use uuid::Uuid;

use crate::{
    domain::{
        centered_crop, FrameRateMode, NormalizedRect, Overlay, OverlayBase, ProjectV1, QualityMode,
        StingPreset,
    },
    ffmpeg::args::build_ffmpeg_args,
    ffmpeg::filter_graph::resolved_frame_rate,
    services::{assets, export::verify_output, media_tools, probe, projects},
};

fn integration_enabled() -> bool {
    std::env::var_os("SKCF_RUN_MEDIA_INTEGRATION").is_some()
}

fn fixture_root(name: &str) -> PathBuf {
    let root = std::env::temp_dir().join(format!("skcf-{name}-fixtures-{}", Uuid::new_v4()));
    fs::create_dir_all(&root).unwrap();
    root
}

fn run(program: &Path, args: &[&OsStr]) -> Output {
    let output = Command::new(program).args(args).output().unwrap();
    assert!(
        output.status.success(),
        "{} failed: {}",
        program.display(),
        String::from_utf8_lossy(&output.stderr)
    );
    output
}

fn generate_video(
    ffmpeg: &Path,
    output: &Path,
    source: &str,
    duration: &str,
    audio: bool,
    codec: &str,
) {
    let mut command = Command::new(ffmpeg);
    command.args([
        "-hide_banner",
        "-loglevel",
        "error",
        "-f",
        "lavfi",
        "-i",
        source,
    ]);
    if audio {
        command.args(["-f", "lavfi", "-i", "sine=frequency=440:sample_rate=48000"]);
    }
    command.args(["-t", duration, "-c:v", codec, "-pix_fmt", "yuv420p"]);
    if codec == "libx265" {
        command.args(["-x265-params", "log-level=error", "-tag:v", "hvc1"]);
    }
    if audio {
        command.args(["-c:a", "aac", "-shortest"]);
    }
    let output_result = command.arg(output).output().unwrap();
    assert!(
        output_result.status.success(),
        "fixture generation failed: {}",
        String::from_utf8_lossy(&output_result.stderr)
    );
}

fn supports_encoder(ffmpeg: &Path, encoder: &str) -> bool {
    Command::new(ffmpeg)
        .args(["-hide_banner", "-encoders"])
        .output()
        .is_ok_and(|output| {
            output.status.success() && String::from_utf8_lossy(&output.stdout).contains(encoder)
        })
}

fn project_for_source(source: &Path, source_probe: &crate::domain::MediaProbe) -> ProjectV1 {
    let mut project: ProjectV1 =
        serde_json::from_str(include_str!("../../examples/example-project.skcf.json")).unwrap();
    project.source.path = source.to_string_lossy().into_owned();
    project.source.filename = source.file_name().unwrap().to_string_lossy().into_owned();
    project.source.probe = source_probe.clone();
    project.source.fingerprint.size_bytes = source_probe.file_size_bytes;
    project.timeline.in_ms = 0;
    project.timeline.out_ms = source_probe.duration_ms;
    project.crop = centered_crop(source_probe);
    project.overlays.clear();
    project.export_defaults.quality_mode = QualityMode::Draft;
    project.export_defaults.frame_rate_mode = FrameRateMode::SourceCapped60;
    project
}

fn export_fixture(ffmpeg: &Path, project: &ProjectV1, output: &Path) {
    let args = build_ffmpeg_args(
        project,
        &project.export_defaults,
        &[] as &[&Path],
        output,
        false,
    )
    .unwrap();
    let export = Command::new(ffmpeg).args(args).output().unwrap();
    assert!(
        export.status.success(),
        "fixture export failed: {}",
        String::from_utf8_lossy(&export.stderr)
    );
    if let Err(error) = verify_output(output, project, None) {
        let output_probe = probe::probe_media(output, None).ok();
        panic!(
            "verification failed for {:?}: {error:?}; output metadata: {output_probe:?}",
            output.file_name()
        );
    }
}

fn assert_audio_drift_within_one_frame(ffprobe: &Path, output: &Path, frame_rate: u32) {
    let metadata = run(
        ffprobe,
        &[
            "-v".as_ref(),
            "error".as_ref(),
            "-show_entries".as_ref(),
            "stream=codec_type,duration".as_ref(),
            "-of".as_ref(),
            "json".as_ref(),
            output.as_os_str(),
        ],
    );
    let metadata: serde_json::Value = serde_json::from_slice(&metadata.stdout).unwrap();
    let streams = metadata["streams"].as_array().unwrap();
    let duration = |codec_type: &str| {
        streams
            .iter()
            .find(|stream| stream["codec_type"] == codec_type)
            .and_then(|stream| stream["duration"].as_str())
            .and_then(|duration| duration.parse::<f64>().ok())
            .unwrap()
    };
    let drift = (duration("video") - duration("audio")).abs();
    assert!(
        drift <= 0.020 + 1.0 / f64::from(frame_rate),
        "audio/video drift was {drift:.6} seconds"
    );
}

#[test]
fn generated_probe_fixture_matrix_covers_boundaries_and_variants() {
    if !integration_enabled() {
        return;
    }
    let ffmpeg = media_tools::resolve_ffmpeg_path(None).unwrap();
    let root = fixture_root("probe");
    let landscape = root.join("landscape-h264-aac.mp4");
    let portrait = root.join("portrait-silent.mp4");
    let minimum = root.join("minimum ünicode 250ms.mp4");
    let rotated = root.join("rotated.mov");
    let variable_rate = root.join("variable frame rate with audio.mp4");
    let cover_art = root.join("cover.png");
    let attached_cover = root.join("attached-cover.mp4");
    let corrupt = root.join("corrupt-header.mp4");

    generate_video(
        &ffmpeg,
        &landscape,
        "testsrc=size=640x360:rate=30",
        "1",
        true,
        "libx264",
    );
    generate_video(
        &ffmpeg,
        &portrait,
        "testsrc=size=360x640:rate=30",
        "1",
        false,
        "libx264",
    );
    generate_video(
        &ffmpeg,
        &minimum,
        "color=c=blue:size=320x240:rate=60",
        "0.25",
        false,
        "libx264",
    );
    run(
        &ffmpeg,
        &[
            "-hide_banner".as_ref(),
            "-loglevel".as_ref(),
            "error".as_ref(),
            "-display_rotation:v:0".as_ref(),
            "90".as_ref(),
            "-i".as_ref(),
            landscape.as_os_str(),
            "-c".as_ref(),
            "copy".as_ref(),
            rotated.as_os_str(),
        ],
    );
    run(
        &ffmpeg,
        &[
            "-hide_banner".as_ref(),
            "-loglevel".as_ref(),
            "error".as_ref(),
            "-f".as_ref(),
            "lavfi".as_ref(),
            "-i".as_ref(),
            "testsrc=size=320x240:rate=60:duration=1".as_ref(),
            "-f".as_ref(),
            "lavfi".as_ref(),
            "-i".as_ref(),
            "sine=frequency=440:sample_rate=48000:duration=1".as_ref(),
            "-vf".as_ref(),
            "select='if(lt(t,0.5),not(mod(n,2)),not(mod(n,4)))'".as_ref(),
            "-fps_mode".as_ref(),
            "vfr".as_ref(),
            "-c:v".as_ref(),
            "libx264".as_ref(),
            "-pix_fmt".as_ref(),
            "yuv420p".as_ref(),
            "-c:a".as_ref(),
            "aac".as_ref(),
            "-ar".as_ref(),
            "48000".as_ref(),
            "-t".as_ref(),
            "1".as_ref(),
            variable_rate.as_os_str(),
        ],
    );
    run(
        &ffmpeg,
        &[
            "-hide_banner".as_ref(),
            "-loglevel".as_ref(),
            "error".as_ref(),
            "-f".as_ref(),
            "lavfi".as_ref(),
            "-i".as_ref(),
            "color=c=yellow:size=32x32".as_ref(),
            "-frames:v".as_ref(),
            "1".as_ref(),
            cover_art.as_os_str(),
        ],
    );
    run(
        &ffmpeg,
        &[
            "-hide_banner".as_ref(),
            "-loglevel".as_ref(),
            "error".as_ref(),
            "-i".as_ref(),
            landscape.as_os_str(),
            "-i".as_ref(),
            cover_art.as_os_str(),
            "-map".as_ref(),
            "0".as_ref(),
            "-map".as_ref(),
            "1".as_ref(),
            "-c".as_ref(),
            "copy".as_ref(),
            "-disposition:v:1".as_ref(),
            "attached_pic".as_ref(),
            attached_cover.as_os_str(),
        ],
    );
    fs::write(&corrupt, b"not a media container").unwrap();

    let landscape_probe = probe::probe_media(&landscape, None).unwrap();
    assert_eq!(
        (
            landscape_probe.video.display_width,
            landscape_probe.video.display_height,
            landscape_probe.has_audio,
        ),
        (640, 360, true)
    );
    let portrait_probe = probe::probe_media(&portrait, None).unwrap();
    assert_eq!(
        (
            portrait_probe.video.display_width,
            portrait_probe.video.display_height,
            portrait_probe.has_audio,
        ),
        (360, 640, false)
    );
    let minimum_probe = probe::probe_media(&minimum, None).unwrap();
    assert!(minimum_probe.duration_ms >= 250);
    let rotated_probe = probe::probe_media(&rotated, None).unwrap();
    assert_eq!(rotated_probe.video.rotation_degrees, 270);
    assert_eq!(
        (
            rotated_probe.video.display_width,
            rotated_probe.video.display_height,
        ),
        (360, 640)
    );
    let variable_rate_probe = probe::probe_media(&variable_rate, None).unwrap();
    assert!(variable_rate_probe.has_audio);
    assert!(variable_rate_probe
        .warnings
        .iter()
        .any(|warning| warning.contains("variable or non-uniform")));
    let attached_cover_probe = probe::probe_media(&attached_cover, None).unwrap();
    assert_eq!(attached_cover_probe.video.stream_index, 0);
    assert_eq!(attached_cover_probe.video.codec, "h264");
    assert_eq!(
        (
            attached_cover_probe.video.display_width,
            attached_cover_probe.video.display_height,
        ),
        (640, 360)
    );
    assert!(probe::probe_media(&corrupt, None).is_err());

    let rotated_project = project_for_source(&rotated, &rotated_probe);
    export_fixture(
        &ffmpeg,
        &rotated_project,
        &root.join("rotated Skull'd ü.partial.mp4"),
    );
    let variable_rate_project = project_for_source(&variable_rate, &variable_rate_probe);
    let expected_frame_rate = resolved_frame_rate(
        &variable_rate_project,
        variable_rate_project.export_defaults.frame_rate_mode,
    );
    let variable_rate_output = root.join("variable-to-cfr.partial.mp4");
    export_fixture(&ffmpeg, &variable_rate_project, &variable_rate_output);
    let normalized_probe = probe::probe_media(&variable_rate_output, None).unwrap();
    assert!(normalized_probe
        .video
        .avg_frame_rate
        .is_some_and(|rate| (rate - f64::from(expected_frame_rate)).abs() <= 0.05));
    let ffprobe = media_tools::resolve_ffprobe_path(None).unwrap();
    assert_audio_drift_within_one_frame(&ffprobe, &variable_rate_output, expected_frame_rate);

    if supports_encoder(&ffmpeg, "libx265") {
        let hevc = root.join("decoder-variation-hevc.mov");
        generate_video(
            &ffmpeg,
            &hevc,
            "color=c=green:size=320x240:rate=30",
            "0.5",
            false,
            "libx265",
        );
        assert_eq!(probe::probe_media(&hevc, None).unwrap().video.codec, "hevc");
    }

    let projects_root = root.join("projects");
    let created =
        projects::create_project(&landscape, &projects_root, Some("Relink fixture"), None).unwrap();
    let mut edited = created.project;
    edited.timeline.out_ms = 750;
    let expected_crop = edited.crop.clone();
    projects::save_project(&created.project_path, edited).unwrap();
    let moved = root.join("moved landscape.mp4");
    fs::copy(&landscape, &moved).unwrap();
    fs::remove_file(&landscape).unwrap();
    assert_eq!(
        projects::load_project(&created.project_path)
            .unwrap()
            .source_status,
        projects::SourceStatus::Missing
    );
    let relinked = projects::relink_source(&created.project_path, &moved, false, None).unwrap();
    assert!(relinked.fingerprint_matched);
    assert_eq!(relinked.project.timeline.out_ms, 750);
    assert_eq!(relinked.project.crop, expected_crop);

    fs::remove_dir_all(root).unwrap();
}

#[test]
fn golden_export_frame_matches_the_shared_crop_and_overlay_mapping() {
    if !integration_enabled() {
        return;
    }
    let ffmpeg = media_tools::resolve_ffmpeg_path(None).unwrap();
    let root = fixture_root("golden");
    let source = root.join("blue landscape.mp4");
    let overlay_path = root.join("red overlay.png");
    let caption_path = root.join("green caption ü.png");
    let output_path = root.join("Skull'd golden ü.partial.mp4");
    generate_video(
        &ffmpeg,
        &source,
        "color=c=blue:size=640x360:rate=30",
        "1",
        false,
        "libx264",
    );
    run(
        &ffmpeg,
        &[
            "-hide_banner".as_ref(),
            "-loglevel".as_ref(),
            "error".as_ref(),
            "-f".as_ref(),
            "lavfi".as_ref(),
            "-i".as_ref(),
            "color=c=red:size=64x64".as_ref(),
            "-frames:v".as_ref(),
            "1".as_ref(),
            overlay_path.as_os_str(),
        ],
    );
    run(
        &ffmpeg,
        &[
            "-hide_banner".as_ref(),
            "-loglevel".as_ref(),
            "error".as_ref(),
            "-f".as_ref(),
            "lavfi".as_ref(),
            "-i".as_ref(),
            "color=c=lime:size=64x64".as_ref(),
            "-frames:v".as_ref(),
            "1".as_ref(),
            caption_path.as_os_str(),
        ],
    );

    let source_probe = probe::probe_media(&source, None).unwrap();
    let mut project: ProjectV1 =
        serde_json::from_str(include_str!("../../examples/example-project.skcf.json")).unwrap();
    project.source.path = source.to_string_lossy().into_owned();
    project.source.filename = "blue landscape.mp4".to_owned();
    project.source.probe = source_probe.clone();
    project.source.fingerprint.size_bytes = source_probe.file_size_bytes;
    project.timeline.in_ms = 0;
    project.timeline.out_ms = 1_000;
    project.crop = centered_crop(&source_probe);
    let mut caption = project
        .overlays
        .iter()
        .find(|overlay| matches!(overlay, Overlay::Caption { .. }))
        .unwrap()
        .clone();
    let caption_base = match &mut caption {
        Overlay::Caption { base, .. } => base,
        Overlay::Image { .. } | Overlay::Sting { .. } => unreachable!(),
    };
    caption_base.position.x = 0.55;
    caption_base.position.y = 0.65;
    caption_base.position.width = 0.3;
    caption_base.position.height = 0.1;
    caption_base.opacity = 1.0;
    caption_base.start_ms = 0;
    caption_base.end_ms = 1_000;
    let mut image = project
        .overlays
        .iter()
        .find(|overlay| matches!(overlay, Overlay::Image { .. }))
        .unwrap()
        .clone();
    let base = match &mut image {
        Overlay::Image { base, .. } => base,
        Overlay::Caption { .. } | Overlay::Sting { .. } => unreachable!(),
    };
    base.position.x = 0.1;
    base.position.y = 0.2;
    base.position.width = 0.25;
    base.position.height = 0.1;
    base.opacity = 1.0;
    base.start_ms = 0;
    base.end_ms = 1_000;
    project.overlays = vec![caption, image];
    project.export_defaults.quality_mode = QualityMode::Draft;
    project.export_defaults.frame_rate_mode = FrameRateMode::Thirty;

    let args = build_ffmpeg_args(
        &project,
        &project.export_defaults,
        &[&caption_path, &overlay_path],
        &output_path,
        false,
    )
    .unwrap();
    let export = Command::new(&ffmpeg).args(args).output().unwrap();
    assert!(
        export.status.success(),
        "golden export failed: {}",
        String::from_utf8_lossy(&export.stderr)
    );
    verify_output(&output_path, &project, None).unwrap();

    let frame = run(
        &ffmpeg,
        &[
            "-hide_banner".as_ref(),
            "-loglevel".as_ref(),
            "error".as_ref(),
            "-i".as_ref(),
            output_path.as_os_str(),
            "-frames:v".as_ref(),
            "1".as_ref(),
            "-f".as_ref(),
            "rawvideo".as_ref(),
            "-pix_fmt".as_ref(),
            "rgb24".as_ref(),
            "pipe:1".as_ref(),
        ],
    )
    .stdout;
    assert_eq!(frame.len(), 1080 * 1920 * 3);
    let overlay_pixel = rgb_at(&frame, 1080, 243, 480);
    let caption_pixel = rgb_at(&frame, 1080, 650, 1_300);
    let background_pixel = rgb_at(&frame, 1080, 900, 1_800);
    assert!(
        overlay_pixel[0] > 220 && overlay_pixel[1] < 35 && overlay_pixel[2] < 35,
        "overlay pixel was {overlay_pixel:?}"
    );
    assert!(
        caption_pixel[1] > 220 && caption_pixel[0] < 35 && caption_pixel[2] < 35,
        "caption pixel was {caption_pixel:?}"
    );
    assert!(
        background_pixel[2] > 220 && background_pixel[0] < 35 && background_pixel[1] < 35,
        "background pixel was {background_pixel:?}"
    );

    fs::remove_dir_all(root).unwrap();
}

fn rgb_at(frame: &[u8], width: usize, x: usize, y: usize) -> [u8; 3] {
    let offset = (y * width + x) * 3;
    [frame[offset], frame[offset + 1], frame[offset + 2]]
}

#[test]
fn constrained_sting_keys_moves_mixes_and_does_not_freeze() {
    if !integration_enabled() {
        return;
    }
    let ffmpeg = media_tools::resolve_ffmpeg_path(None).unwrap();
    let root = fixture_root("sting");
    let source = root.join("blue source with audio.mp4");
    let sting = root.join("green Skull'd sting.mp4");
    let output = root.join("sting result.partial.mp4");
    generate_video(
        &ffmpeg,
        &source,
        "color=c=blue:size=640x360:rate=30",
        "2",
        true,
        "libx264",
    );
    let generated_sting = Command::new(&ffmpeg)
        .args([
            "-hide_banner",
            "-loglevel",
            "error",
            "-f",
            "lavfi",
            "-i",
            "color=c=0x06EE11:size=256x256:rate=30:duration=3,drawbox=x=80:y=80:w=96:h=96:color=red:t=fill",
            "-f",
            "lavfi",
            "-i",
            "sine=frequency=880:sample_rate=48000:duration=3",
            "-t",
            "3",
            "-c:v",
            "libx264",
            "-pix_fmt",
            "yuv420p",
            "-c:a",
            "aac",
            "-shortest",
        ])
        .arg(&sting)
        .output()
        .unwrap();
    assert!(
        generated_sting.status.success(),
        "sting generation failed: {}",
        String::from_utf8_lossy(&generated_sting.stderr)
    );

    let source_probe = probe::probe_media(&source, None).unwrap();
    let sting_probe = probe::probe_media(&sting, None).unwrap();
    let project_directory = root.join(Uuid::new_v4().to_string());
    fs::create_dir_all(&project_directory).unwrap();
    let project_path = project_directory.join("project.skcf.json");
    fs::write(&project_path, b"{}").unwrap();
    let (sting_asset, stored_sting, preview_path) =
        assets::import_sting_asset(&project_path, &sting, &sting_probe, &ffmpeg).unwrap();
    assert!(stored_sting.is_file());
    assert!(preview_path.is_file());
    assert_eq!(
        (
            sting_asset.preview.width,
            sting_asset.preview.height,
            sting_asset.preview.frame_count,
        ),
        (768, 576, 12)
    );
    let preview_rgba = run(
        &ffmpeg,
        &[
            "-hide_banner".as_ref(),
            "-loglevel".as_ref(),
            "error".as_ref(),
            "-i".as_ref(),
            preview_path.as_os_str(),
            "-frames:v".as_ref(),
            "1".as_ref(),
            "-f".as_ref(),
            "rawvideo".as_ref(),
            "-pix_fmt".as_ref(),
            "rgba".as_ref(),
            "pipe:1".as_ref(),
        ],
    )
    .stdout;
    let transparent_offset = (10 * 768 + 10) * 4;
    let subject_offset = (90 * 768 + 90) * 4;
    assert!(preview_rgba[transparent_offset + 3] < 10);
    assert!(preview_rgba[subject_offset] > 180 && preview_rgba[subject_offset + 3] > 200);
    let mut project = project_for_source(&source, &source_probe);
    project.timeline.out_ms = 2_000;
    project.overlays = vec![Overlay::Sting {
        base: OverlayBase {
            id: Uuid::new_v4().to_string(),
            name: "Skull'd sting".to_owned(),
            position: NormalizedRect {
                x: 0.5,
                y: 0.5,
                width: 0.25,
                height: 0.140_625,
            },
            opacity: 1.0,
            start_ms: 500,
            end_ms: 1_500,
            z_index: 0,
        },
        asset: sting_asset,
        preset: StingPreset::ToastyRight,
        include_audio: true,
    }];
    project.validate().unwrap();
    assets::validate_project_assets(&project_path, &project, true).unwrap();

    let args = build_ffmpeg_args(
        &project,
        &project.export_defaults,
        &[&stored_sting],
        &output,
        false,
    )
    .unwrap();
    let exported = Command::new(&ffmpeg).args(args).output().unwrap();
    assert!(
        exported.status.success(),
        "sting export failed: {}",
        String::from_utf8_lossy(&exported.stderr)
    );
    verify_output(&output, &project, None).unwrap();

    let frame_at = |seconds: &str| {
        run(
            &ffmpeg,
            &[
                "-hide_banner".as_ref(),
                "-loglevel".as_ref(),
                "error".as_ref(),
                "-ss".as_ref(),
                seconds.as_ref(),
                "-i".as_ref(),
                output.as_os_str(),
                "-frames:v".as_ref(),
                "1".as_ref(),
                "-f".as_ref(),
                "rawvideo".as_ref(),
                "-pix_fmt".as_ref(),
                "rgb24".as_ref(),
                "pipe:1".as_ref(),
            ],
        )
        .stdout
    };
    let active = frame_at("1.0");
    let after = frame_at("1.7");
    let keyed_background = rgb_at(&active, 1080, 550, 970);
    let sting_subject = rgb_at(&active, 1080, 675, 1_095);
    let cleared_subject = rgb_at(&after, 1080, 675, 1_095);
    assert!(
        keyed_background[2] > 200 && keyed_background[0] < 50 && keyed_background[1] < 50,
        "keyed background was {keyed_background:?}"
    );
    assert!(
        sting_subject[0] > 180 && sting_subject[1] < 80 && sting_subject[2] < 80,
        "sting subject was {sting_subject:?}"
    );
    assert!(
        cleared_subject[2] > 200 && cleared_subject[0] < 50 && cleared_subject[1] < 50,
        "post-sting frame froze at {cleared_subject:?}"
    );

    let volume = Command::new(&ffmpeg)
        .args([
            "-hide_banner",
            "-i",
            output.to_str().unwrap(),
            "-af",
            "volumedetect",
            "-f",
            "null",
            "-",
        ])
        .output()
        .unwrap();
    assert!(volume.status.success());
    let volume_report = String::from_utf8_lossy(&volume.stderr);
    assert!(
        volume_report.contains("max_volume: -"),
        "sting mix was not kept below full scale: {volume_report}"
    );

    fs::remove_dir_all(root).unwrap();
}
