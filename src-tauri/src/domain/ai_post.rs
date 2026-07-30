use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AiPostProvider {
    Openai,
    Openrouter,
}

impl AiPostProvider {
    pub fn label(self) -> &'static str {
        match self {
            Self::Openai => "OpenAI",
            Self::Openrouter => "OpenRouter",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiProviderCredentialStatus {
    pub provider: AiPostProvider,
    pub configured: bool,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiModelOption {
    pub provider: AiPostProvider,
    pub id: String,
    pub name: String,
    pub context_length: Option<u64>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "lowercase")]
pub enum AiYouTubePostFormat {
    Short,
    Video,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum AiYouTubePostMomentType {
    Completion,
    Death,
    BossEncounter,
    BuildShowcase,
    GameplayHighlight,
    Guide,
}

#[derive(Debug, Clone, PartialEq, Eq, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiYouTubePostBrief {
    pub game: String,
    pub format: AiYouTubePostFormat,
    pub moment_type: AiYouTubePostMomentType,
    pub content_summary: String,
    pub primary_search_phrase: String,
    pub supporting_keywords: String,
    pub call_to_action: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiYouTubeTitleOption {
    pub id: String,
    pub label: String,
    pub title: String,
}

#[derive(Debug, Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AiYouTubePostDraft {
    pub title_options: Vec<AiYouTubeTitleOption>,
    pub title: String,
    pub description: String,
    pub hashtags: Vec<String>,
}
