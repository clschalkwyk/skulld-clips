use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VideoProbe {
    pub stream_index: u32,
    pub codec: String,
    pub raw_width: u32,
    pub raw_height: u32,
    pub display_width: u32,
    pub display_height: u32,
    pub rotation_degrees: u16,
    pub avg_frame_rate: Option<f64>,
    pub real_frame_rate: Option<f64>,
    pub pixel_format: Option<String>,
    pub sample_aspect_ratio: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AudioProbe {
    pub stream_index: u32,
    pub codec: String,
    pub sample_rate: Option<u32>,
    pub channels: Option<u16>,
    pub channel_layout: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct MediaProbe {
    pub duration_ms: u64,
    pub container_name: String,
    pub file_size_bytes: u64,
    pub video: VideoProbe,
    pub has_audio: bool,
    pub audio: Option<AudioProbe>,
    pub warnings: Vec<String>,
}
