use serde::Serialize;

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RuntimeInfo {
    pub app_version: String,
    pub project_schema_version: u32,
    pub os: String,
    pub arch: String,
    pub ffmpeg_version: String,
    pub ffprobe_version: String,
    pub bundled_sidecars: bool,
}
