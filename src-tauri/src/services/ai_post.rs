use std::{collections::HashSet, io::Read, time::Duration};

use keyring::Entry;
use reqwest::{
    blocking::{Client, Response},
    StatusCode,
};
use serde::{de::DeserializeOwned, Deserialize};
use serde_json::{json, Value};

use crate::domain::{
    AiModelOption, AiPostProvider, AiProviderCredentialStatus, AiYouTubePostBrief,
    AiYouTubePostDraft, AiYouTubeTitleOption, AppError,
};

const KEYRING_SERVICE: &str = "com.skulld.clipforge.ai-provider";
const OPENAI_MODELS_ENDPOINT: &str = "https://api.openai.com/v1/models";
const OPENAI_RESPONSES_ENDPOINT: &str = "https://api.openai.com/v1/responses";
const OPENROUTER_KEY_ENDPOINT: &str = "https://openrouter.ai/api/v1/key";
const OPENROUTER_MODELS_ENDPOINT: &str =
    "https://openrouter.ai/api/v1/models?output_modalities=text&limit=500";
const OPENROUTER_CHAT_ENDPOINT: &str = "https://openrouter.ai/api/v1/chat/completions";
const HTTP_TIMEOUT: Duration = Duration::from_secs(30);
const MAX_RESPONSE_BYTES: u64 = 2 * 1024 * 1024;
const MAX_MODELS: usize = 500;
const MAX_MODEL_ID_CHARACTERS: usize = 200;
const MAX_TITLE_CHARACTERS: usize = 100;
const TARGET_TITLE_CHARACTERS: usize = 72;
const MAX_DESCRIPTION_CHARACTERS: usize = 5_000;
const MAX_HASHTAGS: usize = 3;

#[derive(Debug, Default)]
pub struct AiPostService;

impl AiPostService {
    pub fn credential_statuses(&self) -> Result<Vec<AiProviderCredentialStatus>, AppError> {
        [AiPostProvider::Openai, AiPostProvider::Openrouter]
            .into_iter()
            .map(|provider| {
                Ok(AiProviderCredentialStatus {
                    provider,
                    configured: key_exists(provider)?,
                })
            })
            .collect()
    }

    pub fn save_api_key(
        &self,
        provider: AiPostProvider,
        api_key: &str,
    ) -> Result<AiProviderCredentialStatus, AppError> {
        validate_api_key(provider, api_key)?;
        validate_remote_credential(provider, api_key)?;
        keyring_entry(provider)?
            .set_password(api_key)
            .map_err(|_| credential_store_error("saved"))?;
        Ok(AiProviderCredentialStatus {
            provider,
            configured: true,
        })
    }

    pub fn clear_api_key(
        &self,
        provider: AiPostProvider,
    ) -> Result<AiProviderCredentialStatus, AppError> {
        match keyring_entry(provider)?.delete_credential() {
            Ok(()) | Err(keyring::Error::NoEntry) => Ok(AiProviderCredentialStatus {
                provider,
                configured: false,
            }),
            Err(_) => Err(credential_store_error("removed")),
        }
    }

    pub fn list_models(&self, provider: AiPostProvider) -> Result<Vec<AiModelOption>, AppError> {
        let api_key = load_api_key(provider)?;
        list_models_with_key(provider, &api_key)
    }

    pub fn generate(
        &self,
        provider: AiPostProvider,
        model: &str,
        brief: &AiYouTubePostBrief,
    ) -> Result<AiYouTubePostDraft, AppError> {
        validate_model_id(model)?;
        validate_brief(brief)?;
        let api_key = load_api_key(provider)?;
        let content = match provider {
            AiPostProvider::Openai => generate_with_openai(&api_key, model, brief)?,
            AiPostProvider::Openrouter => generate_with_openrouter(&api_key, model, brief)?,
        };
        build_draft(content, brief, provider)
    }
}

fn credential_store_error(action: &str) -> AppError {
    AppError::io(
        "The AI provider credential could not be updated.",
        format!(
            "The API key could not be {action}. Unlock the operating-system credential store and retry."
        ),
    )
}

fn keyring_entry(provider: AiPostProvider) -> Result<Entry, AppError> {
    let account = match provider {
        AiPostProvider::Openai => "openai-api-key",
        AiPostProvider::Openrouter => "openrouter-api-key",
    };
    Entry::new(KEYRING_SERVICE, account).map_err(|_| {
        AppError::io(
            "The operating-system credential store is unavailable.",
            "Unlock the credential store and retry.",
        )
    })
}

fn key_exists(provider: AiPostProvider) -> Result<bool, AppError> {
    match keyring_entry(provider)?.get_password() {
        Ok(value) => Ok(!value.is_empty()),
        Err(keyring::Error::NoEntry) => Ok(false),
        Err(_) => Err(AppError::io(
            "The AI provider credential state is unavailable.",
            "Unlock the operating-system credential store and retry.",
        )),
    }
}

fn load_api_key(provider: AiPostProvider) -> Result<String, AppError> {
    keyring_entry(provider)?
        .get_password()
        .map_err(|error| match error {
            keyring::Error::NoEntry => AppError::ai_provider_auth(
                provider.label(),
                "Enter and save this provider's API key before loading models or generating copy.",
            ),
            _ => AppError::io(
                "The saved AI provider credential is unavailable.",
                "Unlock the operating-system credential store and retry.",
            ),
        })
}

fn validate_api_key(provider: AiPostProvider, api_key: &str) -> Result<(), AppError> {
    let trimmed = api_key.trim();
    if trimmed != api_key
        || !(20..=512).contains(&api_key.len())
        || api_key.chars().any(char::is_whitespace)
    {
        return Err(AppError::ai_provider_auth(
            provider.label(),
            "Paste the complete API key without leading, trailing, or embedded whitespace.",
        ));
    }
    Ok(())
}

fn validate_remote_credential(provider: AiPostProvider, api_key: &str) -> Result<(), AppError> {
    let endpoint = match provider {
        AiPostProvider::Openai => OPENAI_MODELS_ENDPOINT,
        AiPostProvider::Openrouter => OPENROUTER_KEY_ENDPOINT,
    };
    let response = Client::new()
        .get(endpoint)
        .bearer_auth(api_key)
        .timeout(HTTP_TIMEOUT)
        .send()
        .map_err(|_| {
            AppError::ai_provider_api(
                provider.label(),
                "The API key could not be validated because the provider is unreachable.",
                true,
            )
        })?;
    decode_empty_response(response, provider, ProviderOperation::CredentialValidation)
}

fn list_models_with_key(
    provider: AiPostProvider,
    api_key: &str,
) -> Result<Vec<AiModelOption>, AppError> {
    let endpoint = match provider {
        AiPostProvider::Openai => OPENAI_MODELS_ENDPOINT,
        AiPostProvider::Openrouter => OPENROUTER_MODELS_ENDPOINT,
    };
    let response = Client::new()
        .get(endpoint)
        .bearer_auth(api_key)
        .timeout(HTTP_TIMEOUT)
        .send()
        .map_err(|_| {
            AppError::ai_provider_api(
                provider.label(),
                "The model catalog could not be loaded because the provider is unreachable.",
                true,
            )
        })?;
    match provider {
        AiPostProvider::Openai => {
            let catalog: OpenAiModelsResponse =
                decode_json_response(response, provider, ProviderOperation::ModelList)?;
            Ok(openai_model_options(catalog))
        }
        AiPostProvider::Openrouter => {
            let catalog: OpenRouterModelsResponse =
                decode_json_response(response, provider, ProviderOperation::ModelList)?;
            Ok(openrouter_model_options(catalog))
        }
    }
}

fn validate_model_id(model: &str) -> Result<(), AppError> {
    let length = model.chars().count();
    if length == 0
        || length > MAX_MODEL_ID_CHARACTERS
        || model.trim() != model
        || model.chars().any(char::is_whitespace)
    {
        return Err(AppError::invalid_argument(
            "Choose a model from the selected provider's current model list.",
        ));
    }
    Ok(())
}

fn validate_brief(brief: &AiYouTubePostBrief) -> Result<(), AppError> {
    let valid = (2..=60).contains(&brief.game.trim().chars().count())
        && (10..=280).contains(&brief.content_summary.trim().chars().count())
        && (3..=80).contains(&brief.primary_search_phrase.trim().chars().count())
        && brief.supporting_keywords.chars().count() <= 240
        && brief.call_to_action.chars().count() <= 240;
    if !valid {
        return Err(AppError::invalid_argument(
            "Complete the bounded YouTube content brief before generating copy.",
        ));
    }
    Ok(())
}

fn generate_with_openai(
    api_key: &str,
    model: &str,
    brief: &AiYouTubePostBrief,
) -> Result<GeneratedPostContent, AppError> {
    let body = json!({
        "model": model,
        "store": false,
        "max_output_tokens": 1200,
        "instructions": system_prompt(),
        "input": user_prompt(brief)?,
        "text": {
            "format": {
                "type": "json_schema",
                "name": "youtube_post",
                "strict": true,
                "schema": output_schema()
            }
        }
    });
    let response = Client::new()
        .post(OPENAI_RESPONSES_ENDPOINT)
        .bearer_auth(api_key)
        .json(&body)
        .timeout(HTTP_TIMEOUT)
        .send()
        .map_err(|_| provider_unreachable(AiPostProvider::Openai))?;
    let payload: OpenAiResponse = decode_json_response(
        response,
        AiPostProvider::Openai,
        ProviderOperation::Generation,
    )?;
    let text = payload
        .output
        .into_iter()
        .flat_map(|item| item.content)
        .find_map(|part| (part.kind == "output_text").then_some(part.text))
        .ok_or_else(|| invalid_generation(AiPostProvider::Openai))?;
    parse_generated_content(&text, AiPostProvider::Openai)
}

fn generate_with_openrouter(
    api_key: &str,
    model: &str,
    brief: &AiYouTubePostBrief,
) -> Result<GeneratedPostContent, AppError> {
    let body = json!({
        "model": model,
        "messages": [
            {"role": "system", "content": system_prompt()},
            {"role": "user", "content": user_prompt(brief)?}
        ],
        "response_format": {
            "type": "json_schema",
            "json_schema": {
                "name": "youtube_post",
                "strict": true,
                "schema": output_schema()
            }
        },
        "stream": false,
        "max_tokens": 1200
    });
    let response = Client::new()
        .post(OPENROUTER_CHAT_ENDPOINT)
        .bearer_auth(api_key)
        .header(
            "HTTP-Referer",
            "https://github.com/clschalkwyk/skulld-clips",
        )
        .header("X-OpenRouter-Title", "Skull'd Clip Forge")
        .json(&body)
        .timeout(HTTP_TIMEOUT)
        .send()
        .map_err(|_| provider_unreachable(AiPostProvider::Openrouter))?;
    let payload: OpenRouterChatResponse = decode_json_response(
        response,
        AiPostProvider::Openrouter,
        ProviderOperation::Generation,
    )?;
    let content = payload
        .choices
        .into_iter()
        .next()
        .and_then(|choice| message_text(choice.message.content))
        .ok_or_else(|| invalid_generation(AiPostProvider::Openrouter))?;
    parse_generated_content(&content, AiPostProvider::Openrouter)
}

fn system_prompt() -> &'static str {
    "You write accurate, catchy, search-aware YouTube metadata for gameplay videos. \
Never invent events, builds, outcomes, enemies, rewards, or claims absent from the factual brief. \
Return exactly three distinct title angles: search-first, hook-first, and moment-first. \
Each title must contain the exact primary search phrase, stay at or below 72 characters where possible, \
and never exceed 100 characters. Avoid clickbait, all-caps shouting, and repeated punctuation. \
The description must open with the exact primary search phrase, use the supplied keywords naturally, \
stay below 5,000 characters, and exclude hashtags. Return no more than three relevant hashtags separately."
}

fn user_prompt(brief: &AiYouTubePostBrief) -> Result<String, AppError> {
    serde_json::to_string(brief)
        .map_err(|_| AppError::internal("The bounded YouTube content brief could not be encoded."))
}

fn output_schema() -> Value {
    json!({
        "type": "object",
        "properties": {
            "titles": {
                "type": "array",
                "items": {"type": "string", "minLength": 1, "maxLength": 100},
                "minItems": 3,
                "maxItems": 3
            },
            "description": {"type": "string", "minLength": 1, "maxLength": 4800},
            "hashtags": {
                "type": "array",
                "items": {"type": "string", "maxLength": 80},
                "maxItems": 3
            }
        },
        "required": ["titles", "description", "hashtags"],
        "additionalProperties": false
    })
}

fn build_draft(
    generated: GeneratedPostContent,
    brief: &AiYouTubePostBrief,
    provider: AiPostProvider,
) -> Result<AiYouTubePostDraft, AppError> {
    if generated.titles.len() != 3 {
        return Err(invalid_generation(provider));
    }
    let phrase = brief.primary_search_phrase.trim();
    let mut distinct = HashSet::new();
    let titles: Vec<String> = generated
        .titles
        .into_iter()
        .map(|title| ensure_phrase_in_title(&title, phrase))
        .collect();
    if titles.iter().any(|title| {
        title.is_empty()
            || title.chars().count() > MAX_TITLE_CHARACTERS
            || !distinct.insert(title.to_lowercase())
    }) {
        return Err(invalid_generation(provider));
    }

    let hashtags = normalize_hashtags(generated.hashtags);
    let mut description = generated.description.trim().to_owned();
    if !starts_with_phrase(&description, phrase) {
        description = format!("{phrase}: {description}");
    }
    let hashtag_line = hashtags.join(" ");
    let suffix = if hashtag_line.is_empty() {
        String::new()
    } else {
        format!("\n\n{hashtag_line}")
    };
    description = truncate_with_ellipsis(
        &description,
        MAX_DESCRIPTION_CHARACTERS.saturating_sub(suffix.chars().count()),
    );
    description.push_str(&suffix);

    let labels = [
        ("searchFirst", "Search-first"),
        ("hookFirst", "Hook-first"),
        ("momentFirst", "Moment-first"),
    ];
    let title_options = titles
        .iter()
        .zip(labels)
        .map(|(title, (id, label))| AiYouTubeTitleOption {
            id: id.to_owned(),
            label: label.to_owned(),
            title: title.clone(),
        })
        .collect();
    let title = titles
        .first()
        .cloned()
        .ok_or_else(|| invalid_generation(provider))?;
    Ok(AiYouTubePostDraft {
        title_options,
        title,
        description,
        hashtags,
    })
}

fn ensure_phrase_in_title(title: &str, phrase: &str) -> String {
    let clean = title.split_whitespace().collect::<Vec<_>>().join(" ");
    if clean.to_lowercase().contains(&phrase.to_lowercase()) {
        return truncate_with_ellipsis(&clean, MAX_TITLE_CHARACTERS);
    }
    let separator = ": ";
    let available = TARGET_TITLE_CHARACTERS
        .max(phrase.chars().count() + separator.chars().count() + 1)
        .min(MAX_TITLE_CHARACTERS);
    let remainder_limit =
        available.saturating_sub(phrase.chars().count() + separator.chars().count());
    format!(
        "{phrase}{separator}{}",
        truncate_with_ellipsis(&clean, remainder_limit)
    )
}

fn starts_with_phrase(value: &str, phrase: &str) -> bool {
    value
        .trim_start()
        .to_lowercase()
        .starts_with(&phrase.to_lowercase())
}

fn truncate_with_ellipsis(value: &str, max_characters: usize) -> String {
    if value.chars().count() <= max_characters {
        return value.to_owned();
    }
    if max_characters <= 1 {
        return "…".chars().take(max_characters).collect();
    }
    let mut output = value
        .chars()
        .take(max_characters.saturating_sub(1))
        .collect::<String>();
    output = output.trim_end().to_owned();
    output.push('…');
    output
}

fn normalize_hashtags(values: Vec<String>) -> Vec<String> {
    let mut seen = HashSet::new();
    values
        .into_iter()
        .filter_map(|value| {
            let body = value
                .trim()
                .trim_start_matches('#')
                .chars()
                .filter(|character| character.is_alphanumeric() || *character == '_')
                .collect::<String>();
            if body.is_empty() {
                return None;
            }
            let hashtag = format!("#{body}");
            seen.insert(hashtag.to_lowercase()).then_some(hashtag)
        })
        .take(MAX_HASHTAGS)
        .collect()
}

fn parse_generated_content(
    text: &str,
    provider: AiPostProvider,
) -> Result<GeneratedPostContent, AppError> {
    let clean = text
        .trim()
        .strip_prefix("```json")
        .or_else(|| text.trim().strip_prefix("```"))
        .unwrap_or(text.trim())
        .strip_suffix("```")
        .unwrap_or(text.trim())
        .trim();
    serde_json::from_str(clean).map_err(|_| invalid_generation(provider))
}

fn message_text(content: Value) -> Option<String> {
    if let Some(text) = content.as_str() {
        return Some(text.to_owned());
    }
    content.as_array().map(|parts| {
        parts
            .iter()
            .filter_map(|part| part.get("text").and_then(Value::as_str))
            .collect::<Vec<_>>()
            .join("")
    })
}

fn provider_unreachable(provider: AiPostProvider) -> AppError {
    AppError::ai_provider_api(
        provider.label(),
        "The provider is unreachable. Check the network connection and retry.",
        true,
    )
}

fn invalid_generation(provider: AiPostProvider) -> AppError {
    AppError::ai_provider_api(
        provider.label(),
        "The selected model returned incomplete metadata. Retry or choose another text model.",
        true,
    )
}

#[derive(Debug, Clone, Copy)]
enum ProviderOperation {
    CredentialValidation,
    ModelList,
    Generation,
}

fn decode_empty_response(
    response: Response,
    provider: AiPostProvider,
    operation: ProviderOperation,
) -> Result<(), AppError> {
    let status = response.status();
    let content_length = response.content_length();
    let _: Value = decode_provider_body(status, content_length, response, provider, operation)?;
    Ok(())
}

fn decode_json_response<T: DeserializeOwned>(
    response: Response,
    provider: AiPostProvider,
    operation: ProviderOperation,
) -> Result<T, AppError> {
    let status = response.status();
    let content_length = response.content_length();
    decode_provider_body(status, content_length, response, provider, operation)
}

fn decode_provider_body<T: DeserializeOwned, R: Read>(
    status: StatusCode,
    content_length: Option<u64>,
    reader: R,
    provider: AiPostProvider,
    operation: ProviderOperation,
) -> Result<T, AppError> {
    if content_length.is_some_and(|length| length > MAX_RESPONSE_BYTES) {
        return Err(AppError::ai_provider_api(
            provider.label(),
            "The provider returned an oversized response.",
            true,
        ));
    }
    let mut bytes = Vec::new();
    reader
        .take(MAX_RESPONSE_BYTES + 1)
        .read_to_end(&mut bytes)
        .map_err(|_| provider_unreachable(provider))?;
    if bytes.len() as u64 > MAX_RESPONSE_BYTES {
        return Err(AppError::ai_provider_api(
            provider.label(),
            "The provider returned an oversized response.",
            true,
        ));
    }
    if !status.is_success() {
        return Err(provider_status_error(status, provider, operation));
    }
    serde_json::from_slice(&bytes).map_err(|_| {
        AppError::ai_provider_api(
            provider.label(),
            "The provider returned an invalid response.",
            true,
        )
    })
}

fn provider_status_error(
    status: StatusCode,
    provider: AiPostProvider,
    operation: ProviderOperation,
) -> AppError {
    match status {
        StatusCode::UNAUTHORIZED | StatusCode::FORBIDDEN => AppError::ai_provider_auth(
            provider.label(),
            "The provider rejected this API key. Replace it and retry.",
        ),
        StatusCode::TOO_MANY_REQUESTS => AppError::ai_provider_api(
            provider.label(),
            "The provider rate limit or account credit limit was reached. Retry later.",
            true,
        ),
        StatusCode::BAD_REQUEST if matches!(operation, ProviderOperation::Generation) => {
            AppError::ai_provider_api(
                provider.label(),
                "The selected model does not support this structured post request. Choose another text model.",
                false,
            )
        }
        status if status.is_server_error() => AppError::ai_provider_api(
            provider.label(),
            "The provider is temporarily unavailable. Retry later.",
            true,
        ),
        _ => AppError::ai_provider_api(
            provider.label(),
            match operation {
                ProviderOperation::CredentialValidation => {
                    "The provider could not validate this API key."
                }
                ProviderOperation::ModelList => "The provider could not return its model list.",
                ProviderOperation::Generation => {
                    "The provider rejected the selected model or generation request."
                }
            },
            false,
        ),
    }
}

#[derive(Debug, Deserialize)]
struct OpenAiModelsResponse {
    #[serde(default)]
    data: Vec<OpenAiModel>,
}

#[derive(Debug, Deserialize)]
struct OpenAiModel {
    id: String,
}

fn openai_model_options(response: OpenAiModelsResponse) -> Vec<AiModelOption> {
    let excluded = [
        "audio",
        "realtime",
        "transcribe",
        "tts",
        "image",
        "search",
        "moderation",
        "embedding",
        "codex",
        "computer-use",
        "deep-research",
        "instruct",
    ];
    let mut models: Vec<_> = response
        .data
        .into_iter()
        .filter(|model| {
            (model.id.starts_with("gpt-5")
                || model.id.starts_with("gpt-4.1")
                || model.id.starts_with("gpt-4o")
                || model.id.starts_with('o'))
                && !excluded.iter().any(|term| model.id.contains(term))
        })
        .map(|model| AiModelOption {
            provider: AiPostProvider::Openai,
            name: model.id.clone(),
            id: model.id,
            context_length: None,
        })
        .collect();
    models.sort_by(|left, right| {
        let left_family = usize::from(!left.id.starts_with("gpt-"));
        let right_family = usize::from(!right.id.starts_with("gpt-"));
        left_family
            .cmp(&right_family)
            .then_with(|| right.id.cmp(&left.id))
    });
    models.dedup_by(|left, right| left.id == right.id);
    models.truncate(MAX_MODELS);
    models
}

#[derive(Debug, Deserialize)]
struct OpenRouterModelsResponse {
    #[serde(default)]
    data: Vec<OpenRouterModel>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterModel {
    id: String,
    name: Option<String>,
    context_length: Option<u64>,
    architecture: Option<OpenRouterArchitecture>,
    #[serde(default)]
    supported_parameters: Vec<String>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterArchitecture {
    #[serde(default)]
    output_modalities: Vec<String>,
}

fn openrouter_model_options(response: OpenRouterModelsResponse) -> Vec<AiModelOption> {
    let mut models: Vec<_> = response
        .data
        .into_iter()
        .filter(|model| {
            let outputs_text = match model.architecture.as_ref() {
                None => true,
                Some(architecture) => {
                    architecture.output_modalities.is_empty()
                        || architecture
                            .output_modalities
                            .iter()
                            .any(|modality| modality == "text")
                }
            };
            let supports_structured_output = model.supported_parameters.is_empty()
                || model
                    .supported_parameters
                    .iter()
                    .any(|parameter| parameter == "response_format");
            outputs_text && supports_structured_output
        })
        .map(|model| AiModelOption {
            provider: AiPostProvider::Openrouter,
            name: model.name.unwrap_or_else(|| model.id.clone()),
            id: model.id,
            context_length: model.context_length,
        })
        .collect();
    models.sort_by(|left, right| left.name.cmp(&right.name).then(left.id.cmp(&right.id)));
    models.dedup_by(|left, right| left.id == right.id);
    models.truncate(MAX_MODELS);
    models
}

#[derive(Debug, Deserialize)]
struct OpenAiResponse {
    #[serde(default)]
    output: Vec<OpenAiOutputItem>,
}

#[derive(Debug, Deserialize)]
struct OpenAiOutputItem {
    #[serde(default)]
    content: Vec<OpenAiOutputPart>,
}

#[derive(Debug, Deserialize)]
struct OpenAiOutputPart {
    #[serde(rename = "type")]
    kind: String,
    #[serde(default)]
    text: String,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChatResponse {
    #[serde(default)]
    choices: Vec<OpenRouterChoice>,
}

#[derive(Debug, Deserialize)]
struct OpenRouterChoice {
    message: OpenRouterMessage,
}

#[derive(Debug, Deserialize)]
struct OpenRouterMessage {
    content: Value,
}

#[derive(Debug, Deserialize)]
struct GeneratedPostContent {
    titles: Vec<String>,
    description: String,
    #[serde(default)]
    hashtags: Vec<String>,
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use reqwest::StatusCode;

    use super::{
        build_draft, decode_provider_body, openai_model_options, openrouter_model_options,
        parse_generated_content, validate_api_key, AiPostProvider, AiYouTubePostBrief,
        GeneratedPostContent, OpenAiModelsResponse, OpenRouterModelsResponse, ProviderOperation,
        MAX_RESPONSE_BYTES,
    };
    use crate::domain::{AiYouTubePostFormat, AiYouTubePostMomentType};

    fn brief() -> AiYouTubePostBrief {
        AiYouTubePostBrief {
            game: "Diablo IV".to_owned(),
            format: AiYouTubePostFormat::Short,
            moment_type: AiYouTubePostMomentType::BossEncounter,
            content_summary:
                "The Butcher ambushed my Whirlwind Barbarian and the fight ended on the final hit."
                    .to_owned(),
            primary_search_phrase: "Diablo 4 Butcher fight".to_owned(),
            supporting_keywords: "Whirlwind Barbarian, Season 14".to_owned(),
            call_to_action: "Subscribe for more Diablo IV boss fights.".to_owned(),
        }
    }

    #[test]
    fn api_key_validation_rejects_whitespace_and_short_values() {
        assert!(validate_api_key(AiPostProvider::Openai, "short").is_err());
        assert!(validate_api_key(
            AiPostProvider::Openrouter,
            "sk-or-v1-this key contains whitespace"
        )
        .is_err());
        assert!(validate_api_key(
            AiPostProvider::Openrouter,
            "sk-or-v1-valid-looking-key-123456789"
        )
        .is_ok());
    }

    #[test]
    fn provider_model_catalogs_keep_text_generation_models_only() {
        let openai: OpenAiModelsResponse = serde_json::from_value(serde_json::json!({
            "data": [
                {"id": "gpt-5.6-terra"},
                {"id": "gpt-image-2"},
                {"id": "text-embedding-3-small"},
                {"id": "o4-mini"}
            ]
        }))
        .unwrap();
        assert_eq!(
            openai_model_options(openai)
                .into_iter()
                .map(|model| model.id)
                .collect::<Vec<_>>(),
            vec!["gpt-5.6-terra", "o4-mini"]
        );

        let openrouter: OpenRouterModelsResponse = serde_json::from_value(serde_json::json!({
            "data": [
                {
                    "id": "openai/gpt-5.6-terra",
                    "name": "GPT-5.6 Terra",
                    "context_length": 200000,
                    "architecture": {"output_modalities": ["text"]},
                    "supported_parameters": ["response_format"]
                },
                {
                    "id": "example/image-only",
                    "name": "Image only",
                    "architecture": {"output_modalities": ["image"]},
                    "supported_parameters": ["response_format"]
                },
                {
                    "id": "example/no-structured-output",
                    "name": "No structured output",
                    "architecture": {"output_modalities": ["text"]},
                    "supported_parameters": ["temperature"]
                }
            ]
        }))
        .unwrap();
        let models = openrouter_model_options(openrouter);
        assert_eq!(models.len(), 1);
        assert_eq!(models[0].id, "openai/gpt-5.6-terra");
        assert_eq!(models[0].context_length, Some(200_000));
    }

    #[test]
    fn generated_copy_is_bounded_and_normalized() {
        let draft = build_draft(
            GeneratedPostContent {
                titles: vec![
                    "The Butcher Nearly Ended This Run".to_owned(),
                    "Final-Hit Butcher Ambush".to_owned(),
                    "Whirlwind Barbarian vs The Butcher".to_owned(),
                ],
                description: "The ambush came down to one final hit.".to_owned(),
                hashtags: vec![
                    "Diablo IV".to_owned(),
                    "#BossFight".to_owned(),
                    "Diablo IV".to_owned(),
                    "#Shorts".to_owned(),
                ],
            },
            &brief(),
            AiPostProvider::Openai,
        )
        .unwrap();

        assert_eq!(draft.title_options.len(), 3);
        assert!(draft
            .title_options
            .iter()
            .all(|option| option.title.contains("Diablo 4 Butcher fight")
                && option.title.chars().count() <= 100));
        assert!(draft.description.starts_with("Diablo 4 Butcher fight:"));
        assert_eq!(draft.hashtags, vec!["#DiabloIV", "#BossFight", "#Shorts"]);
        assert!(draft.description.ends_with("#DiabloIV #BossFight #Shorts"));
    }

    #[test]
    fn structured_output_parser_accepts_bounded_code_fences() {
        let parsed = parse_generated_content(
            "```json\n{\"titles\":[\"a\",\"b\",\"c\"],\"description\":\"d\",\"hashtags\":[]}\n```",
            AiPostProvider::Openrouter,
        )
        .unwrap();
        assert_eq!(parsed.titles, vec!["a", "b", "c"]);
    }

    #[test]
    fn provider_response_decoder_bounds_and_sanitizes_failures() {
        let unauthorized = decode_provider_body::<serde_json::Value, _>(
            StatusCode::UNAUTHORIZED,
            None,
            Cursor::new(br#"{"error":"private provider detail"}"#),
            AiPostProvider::Openai,
            ProviderOperation::Generation,
        )
        .unwrap_err();
        assert_eq!(
            serde_json::to_value(unauthorized).unwrap()["code"],
            "E_AI_PROVIDER_AUTH"
        );

        let oversized = decode_provider_body::<serde_json::Value, _>(
            StatusCode::OK,
            Some(MAX_RESPONSE_BYTES + 1),
            Cursor::new(Vec::<u8>::new()),
            AiPostProvider::Openrouter,
            ProviderOperation::ModelList,
        )
        .unwrap_err();
        assert_eq!(
            serde_json::to_value(oversized).unwrap()["code"],
            "E_AI_PROVIDER_API"
        );
    }
}
