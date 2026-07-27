use crate::{
    domain::{FrameRateMode, Overlay, ProjectV1},
    ffmpeg::coordinates::{crop_to_display_pixels, overlay_to_canvas_pixels},
};

pub fn ordered_overlays(project: &ProjectV1) -> Vec<&Overlay> {
    let mut overlays: Vec<_> = project.overlays.iter().collect();
    overlays.sort_by_key(|overlay| overlay.base().z_index);
    overlays
}

pub fn resolved_frame_rate(project: &ProjectV1, mode: FrameRateMode) -> u32 {
    match mode {
        FrameRateMode::Thirty => 30,
        FrameRateMode::Sixty => 60,
        FrameRateMode::SourceCapped60 => project
            .source
            .probe
            .video
            .avg_frame_rate
            .or(project.source.probe.video.real_frame_rate)
            .unwrap_or(30.0)
            .round()
            .clamp(1.0, 60.0) as u32,
    }
}

pub fn build_filter_graph(project: &ProjectV1, frame_rate: u32) -> String {
    let probe = &project.source.probe;
    let crop = crop_to_display_pixels(
        &project.crop,
        probe.video.display_width,
        probe.video.display_height,
    );
    let overlays = ordered_overlays(project);
    let base_output = if overlays.is_empty() { "vout" } else { "v0" };
    let mut filters = Vec::with_capacity(overlays.len() * 2 + 2);
    let mut video_steps = Vec::new();
    if let Some(orientation) = orientation_filter(probe.video.rotation_degrees) {
        video_steps.push(orientation);
    }
    video_steps.extend([
        format!(
            "trim=start={}:end={}",
            seconds(project.timeline.in_ms),
            seconds(project.timeline.out_ms)
        ),
        "setpts=PTS-STARTPTS".to_owned(),
        format!("crop={}:{}:{}:{}", crop.width, crop.height, crop.x, crop.y),
        "scale=1080:1920:flags=lanczos".to_owned(),
        "setsar=1".to_owned(),
        format!("fps={frame_rate}"),
    ]);
    filters.push(format!(
        "[0:{}]{}[{base_output}]",
        probe.video.stream_index,
        video_steps.join(",")
    ));

    for (index, overlay) in overlays.iter().enumerate() {
        let base = overlay.base();
        let position = overlay_to_canvas_pixels(&base.position);
        let asset_label = format!("overlay{index}");
        filters.push(format!(
            "[{}:v]format=rgba,scale={}:{}:flags=lanczos,colorchannelmixer=aa={}[{asset_label}]",
            index + 1,
            position.width,
            position.height,
            decimal(base.opacity)
        ));
        let input_label = if index == 0 {
            "v0".to_owned()
        } else {
            format!("v{index}")
        };
        let output_label = if index + 1 == overlays.len() {
            "vout".to_owned()
        } else {
            format!("v{}", index + 1)
        };
        filters.push(format!(
            "[{input_label}][{asset_label}]overlay={}:{}:enable='between(t,{},{})':eof_action=pass:shortest=0:repeatlast=1[{output_label}]",
            position.x,
            position.y,
            seconds(base.start_ms - project.timeline.in_ms),
            seconds(base.end_ms - project.timeline.in_ms),
        ));
    }

    if let Some(audio) = &probe.audio {
        filters.push(format!(
            "[0:{}]atrim=start={}:end={},asetpts=PTS-STARTPTS[aout]",
            audio.stream_index,
            seconds(project.timeline.in_ms),
            seconds(project.timeline.out_ms)
        ));
    }
    filters.join(";")
}

fn orientation_filter(rotation_degrees: u16) -> Option<String> {
    match rotation_degrees {
        0 => None,
        90 => Some("transpose=clock".to_owned()),
        180 => Some("hflip,vflip".to_owned()),
        270 => Some("transpose=cclock".to_owned()),
        _ => None,
    }
}

pub fn seconds(milliseconds: u64) -> String {
    format!("{}.{:03}", milliseconds / 1000, milliseconds % 1000)
}

fn decimal(value: f64) -> String {
    format!("{value:.6}")
}

#[cfg(test)]
mod tests {
    use crate::domain::{FrameRateMode, Overlay, ProjectV1};

    use super::{build_filter_graph, resolved_frame_rate};

    #[test]
    fn graph_uses_numeric_values_and_raster_assets_only() {
        let mut project: ProjectV1 =
            serde_json::from_str(include_str!("../../../examples/example-project.skcf.json"))
                .unwrap();
        project.source.probe.video.rotation_degrees = 90;
        let graph = build_filter_graph(&project, 60);

        assert!(graph.contains("transpose=clock,trim=start=8.400:end=23.600"));
        assert!(graph.contains("crop=608:1080:656:0"));
        assert!(graph.contains("scale=1080:1920:flags=lanczos"));
        assert!(graph.contains("overlay="));
        assert!(!graph.contains("CLUTCH"));
        assert!(!graph.contains("Skull"));

        project.source.probe.video.avg_frame_rate = Some(119.88);
        assert_eq!(
            resolved_frame_rate(&project, FrameRateMode::SourceCapped60),
            60
        );
        assert_eq!(project.overlays.len(), 2);
        assert!(matches!(project.overlays[0], Overlay::Caption { .. }));
    }
}
