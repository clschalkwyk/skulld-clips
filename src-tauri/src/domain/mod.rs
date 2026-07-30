mod ai_post;
mod clip_analysis;
mod error;
mod export;
mod media;
mod project;
mod runtime;
mod youtube;

pub use ai_post::{
    AiModelOption, AiPostProvider, AiProviderCredentialStatus, AiYouTubePostBrief,
    AiYouTubePostDraft, AiYouTubeTitleOption,
};
#[cfg(test)]
pub use ai_post::{AiYouTubePostFormat, AiYouTubePostMomentType};
pub use clip_analysis::{
    CancelClipAnalysisResponse, ClipAnalysisCancelledEvent, ClipAnalysisCompletedEvent,
    ClipAnalysisFailedEvent, ClipAnalysisProgressEvent, ClipCandidate, ClipEventKind,
    StartClipAnalysisResponse,
};
pub use error::{AppError, AppErrorCode};
pub use export::{
    CancelExportResponse, ExportCancelledEvent, ExportCompletedEvent, ExportFailedEvent,
    ExportProgressEvent, ExportRequest, ExportValidation, StartExportResponse,
};
pub use media::{AudioProbe, MediaProbe, VideoProbe};
pub use project::{
    centered_crop, AssetRef, Canvas, ExportSettings, FrameRateMode, NormalizedRect, Overlay,
    ProjectAssetKind, ProjectSource, ProjectV1, QualityMode, SourceFingerprint, StingAssetRef,
    StingPreviewRef, Timeline, MIN_TRIM_DURATION_MS, PROJECT_FILENAME, PROJECT_SCHEMA_VERSION,
};
#[cfg(test)]
pub use project::{OverlayBase, StingPreset};
pub use runtime::RuntimeInfo;
pub use youtube::{
    AuthorizedChannel, YouTubeChannel, YouTubeConnectionPhase, YouTubeConnectionStatus,
    YouTubeDailyPerformance, YouTubePerformanceMetrics, YouTubePerformanceSnapshot,
    YouTubeProjectPerformance, YouTubeVideoCandidate,
};
