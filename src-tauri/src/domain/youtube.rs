use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeChannel {
    pub channel_id: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeConnectionStatus {
    pub configured: bool,
    pub authenticated: bool,
    pub channel: Option<YouTubeChannel>,
    pub last_synced_at: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeVideoCandidate {
    pub video_id: String,
    pub title: String,
    pub published_at: String,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubePerformanceMetrics {
    pub engaged_views: f64,
    pub views: f64,
    pub estimated_minutes_watched: f64,
    pub average_view_duration_seconds: f64,
    pub average_view_percentage: f64,
    pub likes: f64,
    pub comments: f64,
    pub shares: f64,
    pub subscribers_gained: f64,
    pub subscribers_lost: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeDailyPerformance {
    pub date: String,
    pub metrics: YouTubePerformanceMetrics,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubePerformanceSnapshot {
    pub start_date: String,
    pub end_date: String,
    pub synced_at: String,
    pub metrics: YouTubePerformanceMetrics,
    pub daily: Vec<YouTubeDailyPerformance>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct YouTubeProjectPerformance {
    pub project_id: String,
    pub project_name: String,
    pub video_id: String,
    pub video_title: String,
    pub published_at: String,
    pub linked_at: String,
    pub performance: Option<YouTubePerformanceSnapshot>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct AuthorizedChannel {
    pub channel: YouTubeChannel,
    pub uploads_playlist_id: String,
}
