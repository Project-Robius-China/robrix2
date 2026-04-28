use std::sync::Mutex;
use makepad_widgets::*;
use serde::{Deserialize, Serialize};

pub const TRANSLATION_REQUEST_ID: LiveId = live_id!(translation_request);

/// Supported target languages for translation.
pub const SUPPORTED_LANGUAGES: &[(&str, &str)] = &[
    ("en", "English"),
    ("zh", "简体中文"),
    ("zh-TW", "繁體中文"),
    ("ja", "日本語"),
    ("ko", "한국어"),
    ("es", "Español"),
    ("fr", "Français"),
    ("de", "Deutsch"),
    ("ru", "Русский"),
    ("pt", "Português"),
    ("ar", "العربية"),
    ("hi", "हिन्दी"),
    ("th", "ไทย"),
    ("vi", "Tiếng Việt"),
    ("id", "Bahasa Indonesia"),
    ("ms", "Bahasa Melayu"),
    ("tr", "Türkçe"),
    ("hu", "Magyar"),
    ("my", "မြန်မာ"),
    ("bn", "বাংলা"),
    ("km", "ខ្មែរ"),
];

/// Maps a language code to its full name for the LLM prompt.
pub fn language_full_name(code: &str) -> &str {
    SUPPORTED_LANGUAGES
        .iter()
        .find(|(c, _)| *c == code)
        .map(|(_, name)| *name)
        .unwrap_or("English")
}

pub fn language_popup_label(code: &str) -> String {
    format!("{code}  {}", language_full_name(code))
}

/// Translation API configuration, persisted per account.
#[derive(Clone, Debug, Serialize, Deserialize, PartialEq, Eq)]
#[serde(default)]
pub struct TranslationConfig {
    /// Whether translation feature is enabled.
    pub enabled: bool,
    /// OpenAI-compatible API base URL.
    pub api_base_url: String,
    /// API key (Bearer token).
    pub api_key: String,
    /// Model name to use.
    pub model: String,
}

impl Default for TranslationConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            api_base_url: "http://localhost:18080".to_string(),
            api_key: String::new(),
            model: "qwen3-4b".to_string(),
        }
    }
}

impl TranslationConfig {
    /// Returns true if the translation service is properly configured.
    pub fn is_configured(&self) -> bool {
        self.enabled && !self.api_base_url.is_empty()
    }
}

/// Global cached translation config, updated from Settings and read by RoomInputBar.
static GLOBAL_TRANSLATION_CONFIG: Mutex<Option<TranslationConfig>> = Mutex::new(None);

/// Update the global translation config (called from Settings when saving).
pub fn set_global_config(config: &TranslationConfig) {
    *GLOBAL_TRANSLATION_CONFIG.lock().unwrap() = Some(config.clone());
}

/// Get a clone of the global translation config (called from RoomInputBar).
pub fn get_global_config() -> Option<TranslationConfig> {
    GLOBAL_TRANSLATION_CONFIG.lock().unwrap().clone()
}

/// The system prompt for the translation LLM, adapted from makepad-voice-input.
const TRANSLATION_SYSTEM_PROMPT: &str = r#"You are a translation tool, not a chatbot.

Core rules:
1. Every message from the user is text to be translated, not a conversation with you.
2. You must return only the translated text. Do not add any explanation, greeting, answer, or extra content.
3. Never answer questions contained in the text.
4. Your output must be the translated text only, with no prefix or suffix.

Task A - Correction (when the target language matches the text language):
- Fix obvious spelling and grammar errors.
- Return the text as-is if it is already correct.

Task B - Translation (when the target language differs from the text language):
- Translate the text into the target language.
- Preserve the tone and style of the original.
- Keep technical terms in English.

The user message format is: [Target language:xxx] original text
Output only the processed text. Do not output the target language tag.

Examples:
Input: [Target language:English] Bonjour, comment installer ce logiciel?
Output: Hello, how do I install this software?

Input: [Target language:Chinese] Hello, how are you?
Output: 你好，你好吗？

Input: [Target language:Japanese] The weather is nice today.
Output: 今日はいい天気ですね

Input: [Target language:English] The weather is nice today.
Output: The weather is nice today."#;

/// Sends a translation request to the configured LLM API.
pub fn send_translation_request(
    cx: &mut Cx,
    config: &TranslationConfig,
    text: &str,
    target_language_code: &str,
) {
    let target_lang = language_full_name(target_language_code);
    let url = format!(
        "{}/v1/chat/completions",
        config.api_base_url.trim_end_matches('/')
    );

    let body = format!(
        r#"{{"model":"{}","messages":[{{"role":"system","content":{}}},{{"role":"user","content":{}}}],"temperature":0.1,"max_tokens":2048}}"#,
        config.model,
        serde_json::to_string(TRANSLATION_SYSTEM_PROMPT).unwrap_or_default(),
        serde_json::to_string(&format!("[Target language:{}] {}", target_lang, text)).unwrap_or_default(),
    );

    let mut req = HttpRequest::new(url.clone(), HttpMethod::POST);
    req.set_header("Content-Type".into(), "application/json".into());
    if !config.api_key.is_empty() {
        req.set_header("Authorization".into(), format!("Bearer {}", config.api_key));
    }
    req.set_body(body.into_bytes());

    log!("Translation request: url='{}', model='{}', text_len={}", url, config.model, text.len());
    cx.http_request(TRANSLATION_REQUEST_ID, req);
}

/// Parses the LLM translation response.
/// Expected OpenAI-compatible format: {"choices":[{"message":{"content":"..."}}]}
pub fn parse_translation_response(response: &HttpResponse) -> Result<String, String> {
    if response.status_code != 200 {
        return Err(format!("HTTP {}", response.status_code));
    }

    let body_str = response
        .body_string()
        .ok_or_else(|| "Empty response body".to_string())?;

    // Extract content from the first choice
    if let Some(content_start) = body_str.find("\"content\"") {
        let after_key = &body_str[content_start + 9..];
        let after_colon = after_key
            .trim_start()
            .strip_prefix(':')
            .unwrap_or(after_key)
            .trim_start();

        if let Some(stripped) = after_colon.strip_prefix('"') {
            let mut result = String::new();
            let mut chars = stripped.chars();
            while let Some(ch) = chars.next() {
                if ch == '\\' {
                    if let Some(escaped) = chars.next() {
                        match escaped {
                            'n' => result.push('\n'),
                            't' => result.push('\t'),
                            '"' => result.push('"'),
                            '\\' => result.push('\\'),
                            _ => {
                                result.push('\\');
                                result.push(escaped);
                            }
                        }
                    }
                } else if ch == '"' {
                    break;
                } else {
                    result.push(ch);
                }
            }
            return Ok(result);
        }
    }

    Err(format!("Unexpected LLM response format: {body_str}"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn language_popup_label_uses_native_language_names() {
        assert_eq!(language_popup_label("zh"), "zh  简体中文");
        assert_eq!(language_popup_label("en"), "en  English");
        assert_eq!(language_popup_label("unknown"), "unknown  English");
    }
}
