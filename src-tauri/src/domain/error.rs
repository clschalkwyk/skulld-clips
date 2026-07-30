use serde::Serialize;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[allow(dead_code)] // The stable contract is implemented before later milestones activate every code.
pub enum AppErrorCode {
    #[serde(rename = "E_INVALID_ARGUMENT")]
    InvalidArgument,
    #[serde(rename = "E_MEDIA_UNSUPPORTED")]
    MediaUnsupported,
    #[serde(rename = "E_SOURCE_MISSING")]
    SourceMissing,
    #[serde(rename = "E_SOURCE_CHANGED")]
    SourceChanged,
    #[serde(rename = "E_PROJECT_SCHEMA")]
    ProjectSchema,
    #[serde(rename = "E_ASSET_MISSING")]
    AssetMissing,
    #[serde(rename = "E_DESTINATION_DENIED")]
    DestinationDenied,
    #[serde(rename = "E_OUTPUT_EXISTS")]
    OutputExists,
    #[serde(rename = "E_DISK_SPACE")]
    DiskSpace,
    #[serde(rename = "E_FFPROBE_FAILED")]
    FfprobeFailed,
    #[serde(rename = "E_FFMPEG_FAILED")]
    FfmpegFailed,
    #[serde(rename = "E_EXPORT_ACTIVE")]
    ExportActive,
    #[serde(rename = "E_EXPORT_NOT_FOUND")]
    ExportNotFound,
    #[serde(rename = "E_EXPORT_CANCELLED")]
    ExportCancelled,
    #[serde(rename = "E_ANALYSIS_ACTIVE")]
    AnalysisActive,
    #[serde(rename = "E_ANALYSIS_NOT_FOUND")]
    AnalysisNotFound,
    #[serde(rename = "E_ANALYSIS_FAILED")]
    AnalysisFailed,
    #[serde(rename = "E_INTEGRATION_UNAVAILABLE")]
    IntegrationUnavailable,
    #[serde(rename = "E_AUTH_REQUIRED")]
    AuthRequired,
    #[serde(rename = "E_NETWORK")]
    Network,
    #[serde(rename = "E_YOUTUBE_API")]
    YouTubeApi,
    #[serde(rename = "E_AI_PROVIDER_AUTH")]
    AiProviderAuth,
    #[serde(rename = "E_AI_PROVIDER_API")]
    AiProviderApi,
    #[serde(rename = "E_IO")]
    Io,
    #[serde(rename = "E_INTERNAL")]
    Internal,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AppError {
    pub code: AppErrorCode,
    pub message: String,
    pub safe_detail: Option<String>,
    pub retryable: bool,
}

impl AppError {
    pub fn invalid_argument(safe_detail: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::InvalidArgument,
            message: "The request contains invalid values.".to_owned(),
            safe_detail: Some(safe_detail.into()),
            retryable: false,
        }
    }

    pub fn media_unsupported(safe_detail: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::MediaUnsupported,
            message: "The selected file is not supported media.".to_owned(),
            safe_detail: Some(safe_detail.into()),
            retryable: false,
        }
    }

    pub fn project_schema(safe_detail: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::ProjectSchema,
            message: "The project file is invalid or unsupported.".to_owned(),
            safe_detail: Some(safe_detail.into()),
            retryable: false,
        }
    }

    pub fn source_missing() -> Self {
        Self {
            code: AppErrorCode::SourceMissing,
            message: "The project source file is missing.".to_owned(),
            safe_detail: Some("Choose the moved source file to relink the project.".to_owned()),
            retryable: false,
        }
    }

    pub fn source_changed() -> Self {
        Self {
            code: AppErrorCode::SourceChanged,
            message: "The project source file has changed.".to_owned(),
            safe_detail: Some(
                "Choose the original file or explicitly accept the replacement.".to_owned(),
            ),
            retryable: false,
        }
    }

    pub fn asset_missing(safe_detail: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::AssetMissing,
            message: "A required project asset is missing or invalid.".to_owned(),
            safe_detail: Some(safe_detail.into()),
            retryable: false,
        }
    }

    pub fn destination_denied(safe_detail: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::DestinationDenied,
            message: "The export destination is not writable.".to_owned(),
            safe_detail: Some(safe_detail.into()),
            retryable: false,
        }
    }

    pub fn output_exists() -> Self {
        Self {
            code: AppErrorCode::OutputExists,
            message: "A file already exists at the export destination.".to_owned(),
            safe_detail: Some(
                "Choose another filename or explicitly confirm replacement.".to_owned(),
            ),
            retryable: false,
        }
    }

    pub fn disk_space(safe_detail: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::DiskSpace,
            message: "There is not enough free space for this export.".to_owned(),
            safe_detail: Some(safe_detail.into()),
            retryable: false,
        }
    }

    pub fn export_active() -> Self {
        Self {
            code: AppErrorCode::ExportActive,
            message: "Another export is already active.".to_owned(),
            safe_detail: Some(
                "Wait for it to finish or cancel it before starting another.".to_owned(),
            ),
            retryable: false,
        }
    }

    pub fn export_not_found() -> Self {
        Self {
            code: AppErrorCode::ExportNotFound,
            message: "The export job is no longer active.".to_owned(),
            safe_detail: Some("Refresh the export state before trying again.".to_owned()),
            retryable: false,
        }
    }

    pub fn analysis_active() -> Self {
        Self {
            code: AppErrorCode::AnalysisActive,
            message: "Another clip analysis is already active.".to_owned(),
            safe_detail: Some(
                "Wait for it to finish or cancel it before starting another scan.".to_owned(),
            ),
            retryable: false,
        }
    }

    pub fn analysis_not_found() -> Self {
        Self {
            code: AppErrorCode::AnalysisNotFound,
            message: "The clip analysis job is no longer active.".to_owned(),
            safe_detail: Some("Start a new source scan before trying again.".to_owned()),
            retryable: false,
        }
    }

    pub fn analysis_failed(safe_detail: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: AppErrorCode::AnalysisFailed,
            message: "Clip Forge could not analyze this gameplay clip.".to_owned(),
            safe_detail: Some(safe_detail.into()),
            retryable,
        }
    }

    pub fn ffmpeg_failed(safe_detail: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::FfmpegFailed,
            message: "The video export failed.".to_owned(),
            safe_detail: Some(safe_detail.into()),
            retryable: true,
        }
    }

    pub fn io(message: impl Into<String>, safe_detail: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::Io,
            message: message.into(),
            safe_detail: Some(safe_detail.into()),
            retryable: true,
        }
    }

    pub fn integration_unavailable(safe_detail: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::IntegrationUnavailable,
            message: "YouTube performance is not configured in this build.".to_owned(),
            safe_detail: Some(safe_detail.into()),
            retryable: false,
        }
    }

    pub fn auth_required(safe_detail: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::AuthRequired,
            message: "Connect a YouTube channel to continue.".to_owned(),
            safe_detail: Some(safe_detail.into()),
            retryable: false,
        }
    }

    pub fn network(safe_detail: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::Network,
            message: "YouTube could not be reached.".to_owned(),
            safe_detail: Some(safe_detail.into()),
            retryable: true,
        }
    }

    pub fn youtube_api(safe_detail: impl Into<String>, retryable: bool) -> Self {
        Self {
            code: AppErrorCode::YouTubeApi,
            message: "YouTube could not return channel performance.".to_owned(),
            safe_detail: Some(safe_detail.into()),
            retryable,
        }
    }

    pub fn ai_provider_auth(provider: &str, safe_detail: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::AiProviderAuth,
            message: format!("Save a valid {provider} API key to continue."),
            safe_detail: Some(safe_detail.into()),
            retryable: false,
        }
    }

    pub fn ai_provider_api(
        provider: &str,
        safe_detail: impl Into<String>,
        retryable: bool,
    ) -> Self {
        Self {
            code: AppErrorCode::AiProviderApi,
            message: format!("{provider} could not generate YouTube copy."),
            safe_detail: Some(safe_detail.into()),
            retryable,
        }
    }

    pub fn media_tool_failed(tool: &str, safe_detail: impl Into<String>) -> Self {
        let code = match tool {
            "ffprobe" => AppErrorCode::FfprobeFailed,
            _ => AppErrorCode::FfmpegFailed,
        };

        Self {
            code,
            message: format!("{tool} is unavailable."),
            safe_detail: Some(safe_detail.into()),
            retryable: true,
        }
    }

    pub fn internal(safe_detail: impl Into<String>) -> Self {
        Self {
            code: AppErrorCode::Internal,
            message: "The native application encountered an unexpected error.".to_owned(),
            safe_detail: Some(safe_detail.into()),
            retryable: true,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{AppError, AppErrorCode};

    #[test]
    fn stable_error_codes_match_the_frontend_contract() {
        let expected = [
            (AppErrorCode::InvalidArgument, "\"E_INVALID_ARGUMENT\""),
            (AppErrorCode::MediaUnsupported, "\"E_MEDIA_UNSUPPORTED\""),
            (AppErrorCode::SourceMissing, "\"E_SOURCE_MISSING\""),
            (AppErrorCode::SourceChanged, "\"E_SOURCE_CHANGED\""),
            (AppErrorCode::ProjectSchema, "\"E_PROJECT_SCHEMA\""),
            (AppErrorCode::AssetMissing, "\"E_ASSET_MISSING\""),
            (AppErrorCode::DestinationDenied, "\"E_DESTINATION_DENIED\""),
            (AppErrorCode::OutputExists, "\"E_OUTPUT_EXISTS\""),
            (AppErrorCode::DiskSpace, "\"E_DISK_SPACE\""),
            (AppErrorCode::FfprobeFailed, "\"E_FFPROBE_FAILED\""),
            (AppErrorCode::FfmpegFailed, "\"E_FFMPEG_FAILED\""),
            (AppErrorCode::ExportActive, "\"E_EXPORT_ACTIVE\""),
            (AppErrorCode::ExportNotFound, "\"E_EXPORT_NOT_FOUND\""),
            (AppErrorCode::ExportCancelled, "\"E_EXPORT_CANCELLED\""),
            (AppErrorCode::AnalysisActive, "\"E_ANALYSIS_ACTIVE\""),
            (AppErrorCode::AnalysisNotFound, "\"E_ANALYSIS_NOT_FOUND\""),
            (AppErrorCode::AnalysisFailed, "\"E_ANALYSIS_FAILED\""),
            (
                AppErrorCode::IntegrationUnavailable,
                "\"E_INTEGRATION_UNAVAILABLE\"",
            ),
            (AppErrorCode::AuthRequired, "\"E_AUTH_REQUIRED\""),
            (AppErrorCode::Network, "\"E_NETWORK\""),
            (AppErrorCode::YouTubeApi, "\"E_YOUTUBE_API\""),
            (AppErrorCode::AiProviderAuth, "\"E_AI_PROVIDER_AUTH\""),
            (AppErrorCode::AiProviderApi, "\"E_AI_PROVIDER_API\""),
            (AppErrorCode::Io, "\"E_IO\""),
            (AppErrorCode::Internal, "\"E_INTERNAL\""),
        ];

        for (code, serialized) in expected {
            assert_eq!(serde_json::to_string(&code).unwrap(), serialized);
        }
    }

    #[test]
    fn app_error_uses_the_stable_camel_case_shape() {
        let error = AppError::media_tool_failed("ffprobe", "Configure ffprobe.");
        let json = serde_json::to_value(error).unwrap();

        assert_eq!(json["code"], "E_FFPROBE_FAILED");
        assert_eq!(json["message"], "ffprobe is unavailable.");
        assert_eq!(json["safeDetail"], "Configure ffprobe.");
        assert_eq!(json["retryable"], true);
    }
}
