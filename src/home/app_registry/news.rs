//! L1 news card factory. Second concrete mini-app type proving that
//! `SplashHost` can render a new capability without bespoke Rust UI
//! widgets.

use serde_json::Value as JsonValue;
use unicode_segmentation::UnicodeSegmentation;

use crate::i18n::AppLanguage;

use super::{AppFactory, RenderFailure, RenderedApp, ValidationError};
use super::capability_descriptors;
use super::splash_host::{CapabilitySchema, HostError};

pub const TYPE_KEY: &str = "news";

const MAX_TEXT_GRAPHEMES: usize = 160;
const MAX_ITEMS: usize = 3;
const NEWS_CARD_BG_COLOR: &str = "#x0B5F79";

pub static FACTORY: NewsFactory = NewsFactory;
pub(crate) static NEWS_CAPABILITY_SCHEMA: NewsCapabilitySchema = NewsCapabilitySchema;

pub(crate) struct NewsCapabilitySchema;

pub struct NewsFactory;

impl AppFactory for NewsFactory {
    fn supported_version(&self) -> u32 {
        1
    }

    fn init(&self, initial_state: &JsonValue) -> Result<Box<dyn RenderedApp>, ValidationError> {
        let obj = initial_state
            .as_object()
            .ok_or_else(|| ValidationError::new("initial_state", "must be a JSON object"))?;

        let topic = parse_required_text(obj, "topic")?;
        let headline = parse_required_text(obj, "headline")?;
        let summary = parse_optional_text(obj, "summary")?
            .unwrap_or_else(|| headline.clone());
        let time_range = parse_time_range(obj)?;
        let focus = parse_focus(obj)?;
        let updated_at = obj
            .get("updated_at")
            .and_then(JsonValue::as_str)
            .map(str::to_string);
        let language_override = parse_optional_language(obj)?;
        let items = parse_items(obj)?;

        Ok(Box::new(RenderedNews {
            topic,
            time_range,
            focus,
            headline,
            summary,
            updated_at,
            language_override,
            items,
        }))
    }
}

impl CapabilitySchema for NewsCapabilitySchema {
    fn app_type(&self) -> &'static str {
        TYPE_KEY
    }

    fn app_version(&self) -> u32 {
        FACTORY.supported_version()
    }

    fn contains_path(&self, path: &str) -> bool {
        matches!(
            path,
            "$state.hero.bg_color"
                | "$state.topic_label"
                | "$state.headline"
                | "$state.summary"
                | "$state.updated.visible"
                | "$state.updated.text"
                | "$state.item_1.visible"
                | "$state.item_1.title"
                | "$state.item_1.source"
                | "$state.item_2.visible"
                | "$state.item_2.title"
                | "$state.item_2.source"
                | "$state.item_3.visible"
                | "$state.item_3.title"
                | "$state.item_3.source"
        )
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewsTimeRange {
    Today,
    ThisWeek,
}

impl NewsTimeRange {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "today" => Some(Self::Today),
            "this_week" => Some(Self::ThisWeek),
            _ => None,
        }
    }

    fn bg_color(self) -> &'static str {
        NEWS_CARD_BG_COLOR
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NewsFocus {
    Headlines,
    Digest,
}

impl NewsFocus {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "headlines" => Some(Self::Headlines),
            "digest" => Some(Self::Digest),
            _ => None,
        }
    }

    fn template_id(self) -> &'static str {
        match self {
            Self::Headlines => "headlines_card",
            Self::Digest => "digest_card",
        }
    }
}

#[derive(Debug, Clone)]
pub struct NewsItem {
    title: String,
    source: String,
}

#[derive(Debug, Clone)]
pub struct RenderedNews {
    topic: String,
    time_range: NewsTimeRange,
    focus: NewsFocus,
    headline: String,
    summary: String,
    updated_at: Option<String>,
    language_override: Option<AppLanguage>,
    items: Vec<NewsItem>,
}

impl RenderedApp for RenderedNews {
    fn app_type(&self) -> &'static str {
        TYPE_KEY
    }

    fn render(&self, app_language: AppLanguage) -> Result<String, RenderFailure> {
        render_news(self, self.language_override.unwrap_or(app_language))
    }
}

#[derive(Debug, Clone, Default)]
struct NewsTextSlot {
    visible: bool,
    text: String,
}

#[derive(Debug, Clone, Default)]
struct NewsItemSlot {
    visible: bool,
    title: String,
    source: String,
}

#[derive(Debug, Clone)]
struct NewsTemplateViewModel {
    bg_color: String,
    topic_label: String,
    headline: String,
    summary: String,
    updated: NewsTextSlot,
    item_1: NewsItemSlot,
    item_2: NewsItemSlot,
    item_3: NewsItemSlot,
}

impl NewsTemplateViewModel {
    fn from_news(state: &RenderedNews, app_language: AppLanguage) -> Self {
        let topic_prefix = copy(app_language, "News", "新闻");
        let time_label = match (state.time_range, app_language) {
            (NewsTimeRange::Today, AppLanguage::English) => "Today",
            (NewsTimeRange::Today, AppLanguage::ChineseSimplified) => "今日",
            (NewsTimeRange::ThisWeek, AppLanguage::English) => "This week",
            (NewsTimeRange::ThisWeek, AppLanguage::ChineseSimplified) => "本周",
        };
        let item_1 = item_slot(state.items.first());
        let item_2 = item_slot(state.items.get(1));
        let item_3 = item_slot(state.items.get(2));

        Self {
            bg_color: state.time_range.bg_color().to_string(),
            topic_label: format!("{topic_prefix} · {} · {time_label}", state.topic),
            headline: state.headline.clone(),
            summary: state.summary.clone(),
            updated: NewsTextSlot {
                visible: state.updated_at.is_some(),
                text: state.updated_at.clone().unwrap_or_default(),
            },
            item_1,
            item_2,
            item_3,
        }
    }

    fn template_state(&self) -> JsonValue {
        serde_json::json!({
            "hero": {
                "bg_color": self.bg_color,
            },
            "topic_label": self.topic_label,
            "headline": self.headline,
            "summary": self.summary,
            "updated": {
                "visible": self.updated.visible,
                "text": self.updated.text,
            },
            "item_1": {
                "visible": self.item_1.visible,
                "title": self.item_1.title,
                "source": self.item_1.source,
            },
            "item_2": {
                "visible": self.item_2.visible,
                "title": self.item_2.title,
                "source": self.item_2.source,
            },
            "item_3": {
                "visible": self.item_3.visible,
                "title": self.item_3.title,
                "source": self.item_3.source,
            },
        })
    }
}

fn render_news(
    state: &RenderedNews,
    app_language: AppLanguage,
) -> Result<String, RenderFailure> {
    let view_model = NewsTemplateViewModel::from_news(state, app_language);
    let host = super::splash_host::splash_host();
    let chrome = capability_descriptors::chrome_for(TYPE_KEY).ok_or_else(|| {
        RenderFailure::Internal {
            reason: format!("missing capability descriptor for {TYPE_KEY}"),
        }
    })?;
    let handle = host
        .load_template("news_guidance", state.focus.template_id())
        .map_err(news_host_error_to_render_failure)?;
    host.render_to_splash(&handle, &view_model.template_state(), &chrome)
        .map_err(news_host_error_to_render_failure)
}

/// See `weather::host_error_to_render_failure` — mirror classifier for
/// the news capability. Kept per-module (not shared) to avoid a
/// premature abstraction; the classification logic is the same shape
/// but the two consumers stay independent.
fn news_host_error_to_render_failure(err: HostError) -> RenderFailure {
    match err {
        HostError::TemplateNotFound {
            capability_id,
            template_id,
        } => RenderFailure::TemplateMissing {
            capability_id,
            template_id,
        },
        HostError::ParseError { .. }
        | HostError::WidgetNotAllowed { .. }
        | HostError::LocalFunctionNotAllowed { .. }
        | HostError::AttributionFieldInTemplate { .. }
        | HostError::BindingPathNotInSchema { .. } => RenderFailure::HostRejected {
            reason: err.to_string(),
        },
        HostError::BindingError { .. }
        | HostError::UpdateOpNotYetSupported { .. }
        | HostError::GeneratedTemplateNotYetSupported => RenderFailure::HostError {
            reason: err.to_string(),
        },
    }
}

fn item_slot(item: Option<&NewsItem>) -> NewsItemSlot {
    item.map(|item| NewsItemSlot {
        visible: true,
        title: item.title.clone(),
        source: item.source.clone(),
    }).unwrap_or_default()
}

fn parse_required_text(
    obj: &serde_json::Map<String, JsonValue>,
    key: &'static str,
) -> Result<String, ValidationError> {
    let raw = obj
        .get(key)
        .and_then(JsonValue::as_str)
        .ok_or_else(|| ValidationError::new(key, "required field missing or not a string"))?;
    let value = truncate_text(raw.trim());
    if value.is_empty() {
        return Err(ValidationError::new(key, "must not be empty"));
    }
    Ok(value)
}

fn parse_optional_text(
    obj: &serde_json::Map<String, JsonValue>,
    key: &'static str,
) -> Result<Option<String>, ValidationError> {
    match obj.get(key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => {
            let raw = value
                .as_str()
                .ok_or_else(|| ValidationError::new(key, "must be a string"))?;
            let value = truncate_text(raw.trim());
            Ok((!value.is_empty()).then_some(value))
        }
    }
}

fn parse_time_range(
    obj: &serde_json::Map<String, JsonValue>,
) -> Result<NewsTimeRange, ValidationError> {
    match obj.get("time_range") {
        None | Some(JsonValue::Null) => Ok(NewsTimeRange::Today),
        Some(value) => {
            let raw = value
                .as_str()
                .ok_or_else(|| ValidationError::new("time_range", "must be a string"))?;
            NewsTimeRange::parse(raw).ok_or_else(|| {
                ValidationError::new("time_range", "must be today or this_week")
            })
        }
    }
}

fn parse_focus(
    obj: &serde_json::Map<String, JsonValue>,
) -> Result<NewsFocus, ValidationError> {
    match obj.get("focus") {
        None | Some(JsonValue::Null) => Ok(NewsFocus::Headlines),
        Some(value) => {
            let raw = value
                .as_str()
                .ok_or_else(|| ValidationError::new("focus", "must be a string"))?;
            NewsFocus::parse(raw).ok_or_else(|| {
                ValidationError::new("focus", "must be headlines or digest")
            })
        }
    }
}

fn parse_optional_language(
    obj: &serde_json::Map<String, JsonValue>,
) -> Result<Option<AppLanguage>, ValidationError> {
    match obj.get("language") {
        None | Some(JsonValue::Null) => Ok(None),
        Some(value) => {
            let raw = value
                .as_str()
                .ok_or_else(|| ValidationError::new("language", "must be a string"))?;
            match raw {
                "en" => Ok(Some(AppLanguage::English)),
                "zh-CN" => Ok(Some(AppLanguage::ChineseSimplified)),
                _ => Err(ValidationError::new("language", "must be en or zh-CN")),
            }
        }
    }
}

fn parse_items(
    obj: &serde_json::Map<String, JsonValue>,
) -> Result<Vec<NewsItem>, ValidationError> {
    let raw_items = obj
        .get("items")
        .and_then(JsonValue::as_array)
        .ok_or_else(|| ValidationError::new("items", "required field missing or not an array"))?;
    if raw_items.is_empty() {
        return Err(ValidationError::new("items", "must contain at least one item"));
    }

    let mut items = Vec::new();
    for (index, raw_item) in raw_items.iter().take(MAX_ITEMS).enumerate() {
        let item = raw_item.as_object().ok_or_else(|| {
            ValidationError::new("items", format!("entry {index} must be an object"))
        })?;
        let title = parse_required_text(item, "title")?;
        let source = parse_optional_text(item, "source")?
            .unwrap_or_else(|| copy(AppLanguage::English, "Unknown source", "未知来源").to_string());
        items.push(NewsItem {
            title,
            source,
        });
    }
    Ok(items)
}

fn truncate_text(raw: &str) -> String {
    let graphemes: Vec<&str> = raw.graphemes(true).collect();
    if graphemes.len() <= MAX_TEXT_GRAPHEMES {
        raw.to_string()
    } else {
        let mut truncated: String = graphemes[..MAX_TEXT_GRAPHEMES].concat();
        truncated.push('\u{2026}');
        truncated
    }
}

fn copy<'a>(app_language: AppLanguage, en: &'a str, zh: &'a str) -> &'a str {
    match app_language {
        AppLanguage::English => en,
        AppLanguage::ChineseSimplified => zh,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use makepad_widgets::ScriptNew;
    use serde_json::json;

    fn init_from(value: JsonValue) -> Result<Box<dyn RenderedApp>, ValidationError> {
        NewsFactory.init(&value)
    }

    #[test]
    fn valid_payload_succeeds_and_renders() {
        let rendered = init_from(json!({
            "topic": "AI",
            "time_range": "today",
            "headline": "AI funding rounds accelerate",
            "summary": "Three major AI infrastructure startups announced new funding today.",
            "items": [
                { "title": "Compute startup raises new round", "source": "Tech Ledger" }
            ],
            "updated_at": "2026-04-23T08:00:00Z"
        })).unwrap();

        let splash = rendered.render(AppLanguage::English).expect("render should succeed in this test");

        assert!(splash.contains("AI funding rounds accelerate"), "{splash}");
        assert!(splash.contains("Tech Ledger"), "{splash}");
        assert!(splash.contains("news_guidance"), "{splash}");
        assert!(splash.contains(NEWS_CARD_BG_COLOR), "{splash}");
        assert!(!splash.contains("$state."), "{splash}");
    }

    #[test]
    fn digest_focus_uses_digest_template() {
        let rendered = init_from(json!({
            "topic": "AI",
            "time_range": "this_week",
            "focus": "digest",
            "headline": "AI weekly digest",
            "summary": "A weekly digest of AI developments.",
            "items": [
                { "title": "Compute startup raises new round", "source": "Tech Ledger" }
            ]
        })).unwrap();

        let splash = rendered.render(AppLanguage::English).expect("render should succeed in this test");

        assert!(splash.contains("AI weekly digest"), "{splash}");
        assert!(splash.contains("#xEFF7F3"), "digest template marker missing: {splash}");
    }

    #[test]
    fn focus_rejects_unknown_values() {
        let Err(err) = init_from(json!({
            "topic": "AI",
            "time_range": "today",
            "focus": "deep_dive",
            "headline": "AI funding rounds accelerate",
            "items": [
                { "title": "Compute startup raises new round" }
            ]
        })) else {
            panic!("expected validation error for invalid focus");
        };
        assert_eq!(err.field, "focus");
    }

    #[test]
    fn missing_items_fails_closed() {
        let Err(err) = init_from(json!({
            "topic": "AI",
            "headline": "AI funding rounds accelerate",
        })) else {
            panic!("expected validation error for missing items");
        };
        assert_eq!(err.field, "items");
    }

    #[test]
    fn time_range_rejects_unknown_values() {
        let Err(err) = init_from(json!({
            "topic": "AI",
            "time_range": "tomorrow",
            "headline": "AI funding rounds accelerate",
            "items": [
                { "title": "Compute startup raises new round" }
            ]
        })) else {
            panic!("expected validation error for invalid time_range");
        };
        assert_eq!(err.field, "time_range");
    }

    #[test]
    fn rendered_news_splash_eval_parses_in_makepad_vm() {
        let rendered = init_from(json!({
            "topic": "AI",
            "time_range": "today",
            "headline": "AI funding rounds accelerate",
            "summary": "Three major AI infrastructure startups announced new funding today.",
            "items": [
                { "title": "Compute startup raises new round", "source": "Tech Ledger" },
                { "title": "Open-source tooling gains enterprise adoption", "source": "Dev Weekly" }
            ],
            "updated_at": "2026-04-23T08:00:00Z"
        })).unwrap();
        let splash = rendered.render(AppLanguage::English).expect("render should succeed in this test");

        let mut cx = makepad_widgets::Cx::new(Box::new(|_, _| {}));
        cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);

            let script_mod = makepad_widgets::makepad_platform::ScriptMod {
                cargo_manifest_path: String::new(),
                module_path: String::new(),
                file: "news_card_test".to_string(),
                line: 1,
                column: 0,
                code: String::new(),
                values: vec![],
            };
            let code = format!("use mod.prelude.widgets.*View{{height:Fit, {splash}");
            let value = vm.eval_with_append_source(
                script_mod,
                &code,
                makepad_widgets::makepad_script::NIL.into(),
            );

            assert!(!value.is_err(), "Splash eval errored for news card: {splash}");
            assert!(!value.is_nil(), "Splash eval returned nil for news card: {splash}");

            let _view = makepad_widgets::View::script_from_value(vm, value);
        });
    }
}
