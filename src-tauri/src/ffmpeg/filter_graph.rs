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

pub fn output_has_audio(project: &ProjectV1) -> bool {
    project.source.probe.has_audio
        || project.overlays.iter().any(|overlay| {
            overlay
                .sting()
                .is_some_and(|(asset, include)| include && asset.has_audio)
        })
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
        let input_index = index + 1;
        if matches!(overlay, Overlay::Sting { .. }) {
            let relative_start_ms = base.start_ms - project.timeline.in_ms;
            let source_window_ms = (base.end_ms - base.start_ms) * 3;
            filters.push(format!(
                "[{input_index}:v:0]trim=start=0:end={},setpts=(PTS-STARTPTS)/3+{}/TB,chromakey=0x06EE11:0.160000:0.050000,format=rgba,scale={}:{}:flags=lanczos,colorchannelmixer=aa={}[{asset_label}]",
                seconds(source_window_ms),
                seconds(relative_start_ms),
                position.width,
                position.height,
                decimal(base.opacity)
            ));
        } else {
            filters.push(format!(
                "[{input_index}:v]format=rgba,scale={}:{}:flags=lanczos,colorchannelmixer=aa={}[{asset_label}]",
                position.width,
                position.height,
                decimal(base.opacity)
            ));
        }
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
        let relative_start_ms = base.start_ms - project.timeline.in_ms;
        let relative_end_ms = base.end_ms - project.timeline.in_ms;
        if matches!(overlay, Overlay::Sting { .. }) {
            let x_expression = sting_x_expression(position.x, relative_start_ms, relative_end_ms);
            filters.push(format!(
                "[{input_label}][{asset_label}]overlay=x='{x_expression}':y={}:enable='between(t,{},{})':eof_action=pass:shortest=0:repeatlast=0[{output_label}]",
                position.y,
                seconds(relative_start_ms),
                seconds(relative_end_ms),
            ));
        } else {
            filters.push(format!(
                "[{input_label}][{asset_label}]overlay={}:{}:enable='between(t,{},{})':eof_action=pass:shortest=0:repeatlast=1[{output_label}]",
                position.x,
                position.y,
                seconds(relative_start_ms),
                seconds(relative_end_ms),
            ));
        }
    }

    let sting_audio = overlays
        .iter()
        .enumerate()
        .find_map(|(index, overlay)| match overlay {
            Overlay::Sting {
                base,
                asset,
                include_audio: true,
                ..
            } if asset.has_audio => Some((index + 1, base)),
            Overlay::Image { .. } | Overlay::Caption { .. } | Overlay::Sting { .. } => None,
        });
    match (&probe.audio, sting_audio) {
        (Some(audio), Some((input_index, base))) => {
            let relative_start_ms = base.start_ms - project.timeline.in_ms;
            let relative_end_ms = base.end_ms - project.timeline.in_ms;
            let source_window_ms = (base.end_ms - base.start_ms) * 3;
            filters.push(format!(
                "[0:{}]atrim=start={}:end={},asetpts=PTS-STARTPTS,volume=0.720000:enable='between(t,{},{})'[amain]",
                audio.stream_index,
                seconds(project.timeline.in_ms),
                seconds(project.timeline.out_ms),
                seconds(relative_start_ms),
                seconds(relative_end_ms),
            ));
            filters.push(format!(
                "[{input_index}:a:0]atrim=start=0:end={},asetpts=PTS-STARTPTS,atempo=1.5,atempo=2.0,adelay={relative_start_ms}:all=1[asting]",
                seconds(source_window_ms),
            ));
            filters.push(
                "[amain][asting]amix=inputs=2:duration=longest:dropout_transition=0:normalize=0,alimiter=limit=0.950000[aout]"
                    .to_owned(),
            );
        }
        (Some(audio), None) => filters.push(format!(
            "[0:{}]atrim=start={}:end={},asetpts=PTS-STARTPTS[aout]",
            audio.stream_index,
            seconds(project.timeline.in_ms),
            seconds(project.timeline.out_ms)
        )),
        (None, Some((input_index, base))) => {
            let relative_start_ms = base.start_ms - project.timeline.in_ms;
            let source_window_ms = (base.end_ms - base.start_ms) * 3;
            filters.push(format!(
                "[{input_index}:a:0]atrim=start=0:end={},asetpts=PTS-STARTPTS,atempo=1.5,atempo=2.0,adelay={relative_start_ms}:all=1,apad=whole_dur={},atrim=end={}[aout]",
                seconds(source_window_ms),
                seconds(project.timeline.out_ms - project.timeline.in_ms),
                seconds(project.timeline.out_ms - project.timeline.in_ms),
            ));
        }
        (None, None) => {}
    }
    filters.join(";")
}

fn sting_x_expression(x: u32, start_ms: u64, end_ms: u64) -> String {
    let entry_end_ms = start_ms + 180;
    let exit_start_ms = end_ms - 120;
    format!(
        "if(lt(t,{}),W-(t-{})/0.180000*(W-{x}),if(gt(t,{}),{x}+(t-{})/0.120000*(W-{x}),{x}))",
        seconds(entry_end_ms),
        seconds(start_ms),
        seconds(exit_start_ms),
        seconds(exit_start_ms),
    )
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
