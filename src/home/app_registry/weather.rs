//! L1 weather card factory. First concrete mini-app type.
//!
//! Contract: `specs/task-agent-to-app-l1-weather-card.spec.md`.
//!
//! This is a pure presentational component: `init` validates the
//! weather JSON payload and produces a `RenderedWeather` state;
//! `render` produces a Canvas eval-path Splash DSL string that
//! the caller injects into the message's `splash_card` slot.

#[cfg(test)]
use std::cmp::Reverse;

use serde_json::Value as JsonValue;
use unicode_segmentation::UnicodeSegmentation;

use crate::i18n::{tr_key, AppLanguage};

use super::{AppFactory, RenderFailure, RenderedApp, ValidationError};
use super::capability_descriptors;
use super::splash_host::{CapabilitySchema, HostError};

/// Type key for this factory in the app registry.
pub const TYPE_KEY: &str = "weather";

/// Max length of `location` in grapheme clusters, per spec §校验规则.
const MAX_LOCATION_GRAPHEMES: usize = 64;

/// Max number of forecast entries, per spec §校验规则.
const MAX_FORECAST_ENTRIES: usize = 7;

/// Plausible-range bounds for temperature in Celsius.
const MIN_TEMP_C: f64 = -80.0;
const MAX_TEMP_C: f64 = 80.0;

/// Static factory instance registered into the app registry.
pub static FACTORY: WeatherFactory = WeatherFactory;
pub(crate) static WEATHER_CAPABILITY_SCHEMA: WeatherCapabilitySchema = WeatherCapabilitySchema;

pub(crate) struct WeatherCapabilitySchema;

pub struct WeatherFactory;

impl AppFactory for WeatherFactory {
    fn supported_version(&self) -> u32 {
        2
    }

    fn supports_version(&self, version: u32) -> bool {
        matches!(version, 1 | 2)
    }

    fn init(&self, initial_state: &JsonValue) -> Result<Box<dyn RenderedApp>, ValidationError> {
        let obj = initial_state
            .as_object()
            .ok_or_else(|| ValidationError::new("initial_state", "must be a JSON object"))?;

        let location = parse_location(obj)?;
        let temp_c = parse_required_temperature(obj, "temp_c")?;
        let condition = parse_condition(obj);

        let feels_like_c = parse_optional_temperature(obj, "feels_like_c")?;
        let humidity = parse_optional_humidity(obj)?;
        let wind_kph = parse_optional_wind_kph(obj)?;
        let language_override = parse_optional_language(obj, "language")?;
        let high_c = parse_optional_temperature(obj, "high_c")?;
        let low_c = parse_optional_temperature(obj, "low_c")?;
        let uv_index_max = parse_optional_nonnegative_number(obj, "uv_index_max")?;
        let precipitation_probability_max =
            parse_optional_percentage_number(obj, "precipitation_probability_max")?;
        let updated_at = obj
            .get("updated_at")
            .and_then(|v| v.as_str())
            .and_then(parse_rfc3339_or_none)
            .map(str::to_string);
        let periods = parse_periods(obj)?;
        let forecast = parse_forecast(obj)?;

        Ok(Box::new(RenderedWeather {
            location,
            temp_c,
            condition,
            language_override,
            high_c,
            low_c,
            feels_like_c,
            humidity,
            wind_kph,
            uv_index_max,
            precipitation_probability_max,
            updated_at,
            periods,
            forecast,
        }))
    }
}

impl CapabilitySchema for WeatherCapabilitySchema {
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
                | "$state.location"
                | "$state.hero.symbol"
                | "$state.hero.temp_text"
                | "$state.range.visible"
                | "$state.range.text"
                | "$state.condition_summary"
                | "$state.updated.visible"
                | "$state.updated.text"
                | "$state.feels_like.visible"
                | "$state.feels_like.text"
                | "$state.humidity.visible"
                | "$state.humidity.text"
                | "$state.wind.visible"
                | "$state.wind.text"
                | "$state.guidance_header"
                | "$state.headline"
                | "$state.summary"
                | "$state.periods_section.visible"
                | "$state.period_1.visible"
                | "$state.period_1.label"
                | "$state.period_1.advice"
                | "$state.period_1.temp_text"
                | "$state.period_2.visible"
                | "$state.period_2.label"
                | "$state.period_2.advice"
                | "$state.period_2.temp_text"
                | "$state.period_3.visible"
                | "$state.period_3.label"
                | "$state.period_3.advice"
                | "$state.period_3.temp_text"
                | "$state.chips_section.visible"
                | "$state.chip_1.visible"
                | "$state.chip_1.text"
                | "$state.chip_2.visible"
                | "$state.chip_2.text"
                | "$state.chip_3.visible"
                | "$state.chip_3.text"
                | "$state.chip_4.visible"
                | "$state.chip_4.text"
        )
    }
}

/// Six-way enum of visual weather conditions supported by v1.
///
/// Unknown values received in the payload are not rejected — they
/// are normalized to `Sunny` with a warning. See
/// `test_unknown_condition_falls_back_to_sunny`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum WeatherCondition {
    Sunny,
    Cloudy,
    Rainy,
    Snowy,
    Stormy,
    Foggy,
}

impl WeatherCondition {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "sunny" => Some(Self::Sunny),
            "cloudy" => Some(Self::Cloudy),
            "rainy" => Some(Self::Rainy),
            "snowy" => Some(Self::Snowy),
            "stormy" => Some(Self::Stormy),
            "foggy" => Some(Self::Foggy),
            _ => None,
        }
    }

    /// Background color for the card, keyed on condition. Values
    /// are hex color strings without the `#x` prefix. Contrast ratio
    /// against white text is at least 4.5:1 for all six.
    fn bg_color_hex(self) -> &'static str {
        match self {
            Self::Sunny => "F5A623",   // warm orange
            Self::Cloudy => "6F8AA4",  // soft blue-gray
            Self::Rainy => "3C78C8",   // mid blue
            Self::Snowy => "6B92B9",   // cool blue
            Self::Stormy => "415982",  // deep storm blue
            Self::Foggy => "8099AD",   // misty blue-gray
        }
    }

    /// BMP-safe symbol character. Per the Makepad emoji lesson,
    /// symbols outside the BMP may render as boxes. Everything
    /// here is ≤ U+2744.
    fn symbol(self) -> &'static str {
        match self {
            Self::Sunny => "\u{2600}",  // ☀
            Self::Cloudy => "\u{2601}", // ☁
            Self::Rainy => "\u{2602}",  // ☂
            Self::Snowy => "\u{2744}",  // ❄
            Self::Stormy => "\u{26A1}", // ⚡
            Self::Foggy => "\u{2261}",  // ≡
        }
    }
}

/// A validated, parsed weather payload. This is the `RenderedApp`
/// impl handed back from `init`.
#[derive(Debug, Clone)]
pub struct RenderedWeather {
    pub location: String,
    pub temp_c: f64,
    pub condition: WeatherCondition,
    pub language_override: Option<AppLanguage>,
    pub high_c: Option<f64>,
    pub low_c: Option<f64>,
    pub feels_like_c: Option<f64>,
    pub humidity: Option<u32>,
    pub wind_kph: Option<f64>,
    pub uv_index_max: Option<f64>,
    pub precipitation_probability_max: Option<f64>,
    pub updated_at: Option<String>,
    pub periods: Vec<DayPeriodWeather>,
    pub forecast: Vec<ForecastEntry>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayPeriodSlot {
    Morning,
    Noon,
    Night,
}

impl DayPeriodSlot {
    fn parse(raw: &str) -> Option<Self> {
        match raw {
            "morning" => Some(Self::Morning),
            "noon" => Some(Self::Noon),
            "night" => Some(Self::Night),
            _ => None,
        }
    }
}

#[derive(Debug, Clone)]
pub struct DayPeriodWeather {
    pub slot: DayPeriodSlot,
    pub temp_c: f64,
    pub condition: WeatherCondition,
    pub precipitation_probability: Option<u32>,
}

#[derive(Debug, Clone)]
pub struct ForecastEntry {
    pub day: String,
    pub high_c: f64,
    pub low_c: f64,
    pub condition: WeatherCondition,
}

impl RenderedApp for RenderedWeather {
    fn app_type(&self) -> &'static str {
        TYPE_KEY
    }

    fn render(&self, app_language: AppLanguage) -> Result<String, RenderFailure> {
        render_weather(self, self.language_override.unwrap_or(app_language))
    }
}

// ----- validation helpers -----

fn parse_location(
    obj: &serde_json::Map<String, JsonValue>,
) -> Result<String, ValidationError> {
    let raw = obj
        .get("location")
        .and_then(JsonValue::as_str)
        .ok_or_else(|| ValidationError::new("location", "required field missing or not a string"))?;

    if raw.is_empty() {
        return Err(ValidationError::new("location", "must not be empty"));
    }

    // Truncate at grapheme cluster boundaries per spec §校验规则.
    let graphemes: Vec<&str> = raw.graphemes(true).collect();
    if graphemes.len() <= MAX_LOCATION_GRAPHEMES {
        Ok(raw.to_string())
    } else {
        let mut truncated: String = graphemes[..MAX_LOCATION_GRAPHEMES].concat();
        truncated.push('\u{2026}'); // horizontal ellipsis
        makepad_widgets::log!(
            "org.octos.app weather: location truncated from {} to {} grapheme clusters (+ ellipsis)",
            graphemes.len(),
            MAX_LOCATION_GRAPHEMES,
        );
        Ok(truncated)
    }
}

fn parse_required_temperature(
    obj: &serde_json::Map<String, JsonValue>,
    key: &'static str,
) -> Result<f64, ValidationError> {
    let v = obj
        .get(key)
        .ok_or_else(|| ValidationError::new(key, "required field missing"))?;
    let n = v
        .as_f64()
        .ok_or_else(|| ValidationError::new(key, "must be a number"))?;
    if !(MIN_TEMP_C..=MAX_TEMP_C).contains(&n) {
        return Err(ValidationError::new(
            key,
            format!("out of plausible range ({MIN_TEMP_C}..={MAX_TEMP_C}): got {n}"),
        ));
    }
    Ok(n)
}

fn parse_optional_temperature(
    obj: &serde_json::Map<String, JsonValue>,
    key: &'static str,
) -> Result<Option<f64>, ValidationError> {
    match obj.get(key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(v) => {
            let n = v
                .as_f64()
                .ok_or_else(|| ValidationError::new(key, "must be a number"))?;
            if !(MIN_TEMP_C..=MAX_TEMP_C).contains(&n) {
                return Err(ValidationError::new(
                    key,
                    format!("out of plausible range ({MIN_TEMP_C}..={MAX_TEMP_C}): got {n}"),
                ));
            }
            Ok(Some(n))
        }
    }
}

fn parse_optional_humidity(
    obj: &serde_json::Map<String, JsonValue>,
) -> Result<Option<u32>, ValidationError> {
    match obj.get("humidity") {
        None | Some(JsonValue::Null) => Ok(None),
        Some(v) => {
            let n = v
                .as_i64()
                .ok_or_else(|| ValidationError::new("humidity", "must be an integer"))?;
            if !(0..=100).contains(&n) {
                return Err(ValidationError::new(
                    "humidity",
                    format!("must be 0..=100, got {n}"),
                ));
            }
            Ok(Some(n as u32))
        }
    }
}

fn parse_optional_wind_kph(
    obj: &serde_json::Map<String, JsonValue>,
) -> Result<Option<f64>, ValidationError> {
    match obj.get("wind_kph") {
        None | Some(JsonValue::Null) => Ok(None),
        Some(v) => {
            let n = v
                .as_f64()
                .ok_or_else(|| ValidationError::new("wind_kph", "must be a number"))?;
            if n < 0.0 {
                return Err(ValidationError::new(
                    "wind_kph",
                    format!("must be >= 0, got {n}"),
                ));
            }
            Ok(Some(n))
        }
    }
}

fn parse_optional_language(
    obj: &serde_json::Map<String, JsonValue>,
    key: &'static str,
) -> Result<Option<AppLanguage>, ValidationError> {
    match obj.get(key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(v) => {
            let raw = v
                .as_str()
                .ok_or_else(|| ValidationError::new(key, "must be a string"))?;
            match raw {
                "en" | "English" => Ok(Some(AppLanguage::English)),
                "zh" | "zh-CN" | "ChineseSimplified" => Ok(Some(AppLanguage::ChineseSimplified)),
                _ => Err(ValidationError::new(
                    key,
                    format!("unsupported language {raw:?}; expected en or zh-CN"),
                )),
            }
        }
    }
}

fn parse_optional_nonnegative_number(
    obj: &serde_json::Map<String, JsonValue>,
    key: &'static str,
) -> Result<Option<f64>, ValidationError> {
    match obj.get(key) {
        None | Some(JsonValue::Null) => Ok(None),
        Some(v) => {
            let n = v
                .as_f64()
                .or_else(|| v.as_i64().map(|raw| raw as f64))
                .ok_or_else(|| ValidationError::new(key, "must be a number"))?;
            if n < 0.0 {
                return Err(ValidationError::new(
                    key,
                    format!("must be >= 0, got {n}"),
                ));
            }
            Ok(Some(n))
        }
    }
}

fn parse_optional_percentage_number(
    obj: &serde_json::Map<String, JsonValue>,
    key: &'static str,
) -> Result<Option<f64>, ValidationError> {
    let Some(n) = parse_optional_nonnegative_number(obj, key)? else {
        return Ok(None);
    };
    if n > 100.0 {
        return Err(ValidationError::new(
            key,
            format!("must be 0..=100, got {n}"),
        ));
    }
    Ok(Some(n))
}

fn parse_condition(obj: &serde_json::Map<String, JsonValue>) -> WeatherCondition {
    let Some(raw) = obj.get("condition").and_then(JsonValue::as_str) else {
        makepad_widgets::log!("org.octos.app weather: condition field missing, defaulting to sunny");
        return WeatherCondition::Sunny;
    };
    match WeatherCondition::parse(raw) {
        Some(c) => c,
        None => {
            makepad_widgets::log!(
                "org.octos.app weather: unknown condition {:?}, defaulting to sunny",
                raw,
            );
            WeatherCondition::Sunny
        }
    }
}

fn parse_forecast(
    obj: &serde_json::Map<String, JsonValue>,
) -> Result<Vec<ForecastEntry>, ValidationError> {
    let Some(raw) = obj.get("forecast") else {
        return Ok(Vec::new());
    };
    let array = raw
        .as_array()
        .ok_or_else(|| ValidationError::new("forecast", "must be an array"))?;

    let over_capacity = array.len() > MAX_FORECAST_ENTRIES;
    let limit = MAX_FORECAST_ENTRIES.min(array.len());
    let mut out = Vec::with_capacity(limit);

    for (i, entry) in array.iter().take(limit).enumerate() {
        let e = entry.as_object().ok_or_else(|| {
            ValidationError::new("forecast", format!("entry {i} is not an object"))
        })?;
        let day = e
            .get("day")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| ValidationError::new("forecast", format!("entry {i} missing `day`")))?
            .to_string();
        let high_c = e
            .get("high_c")
            .and_then(JsonValue::as_f64)
            .ok_or_else(|| {
                ValidationError::new("forecast", format!("entry {i} missing or invalid `high_c`"))
            })?;
        let low_c = e
            .get("low_c")
            .and_then(JsonValue::as_f64)
            .ok_or_else(|| {
                ValidationError::new("forecast", format!("entry {i} missing or invalid `low_c`"))
            })?;
        if !(MIN_TEMP_C..=MAX_TEMP_C).contains(&high_c)
            || !(MIN_TEMP_C..=MAX_TEMP_C).contains(&low_c)
        {
            return Err(ValidationError::new(
                "forecast",
                format!("entry {i} temperatures out of plausible range"),
            ));
        }
        let condition = e
            .get("condition")
            .and_then(JsonValue::as_str)
            .and_then(WeatherCondition::parse)
            .unwrap_or(WeatherCondition::Sunny);
        out.push(ForecastEntry {
            day,
            high_c,
            low_c,
            condition,
        });
    }

    if over_capacity {
        makepad_widgets::log!(
            "org.octos.app weather: forecast truncated from {} to {} entries",
            array.len(),
            MAX_FORECAST_ENTRIES,
        );
    }

    Ok(out)
}

fn parse_periods(
    obj: &serde_json::Map<String, JsonValue>,
) -> Result<Vec<DayPeriodWeather>, ValidationError> {
    let Some(raw) = obj.get("periods") else {
        return Ok(Vec::new());
    };
    let array = raw
        .as_array()
        .ok_or_else(|| ValidationError::new("periods", "must be an array"))?;

    let mut out = Vec::with_capacity(array.len().min(3));
    for (i, entry) in array.iter().take(3).enumerate() {
        let e = entry.as_object().ok_or_else(|| {
            ValidationError::new("periods", format!("entry {i} is not an object"))
        })?;
        let slot_raw = e
            .get("slot")
            .and_then(JsonValue::as_str)
            .ok_or_else(|| ValidationError::new("periods", format!("entry {i} missing `slot`")))?;
        let slot = DayPeriodSlot::parse(slot_raw).ok_or_else(|| {
            ValidationError::new("periods", format!("entry {i} has invalid `slot`"))
        })?;
        let temp_c = e
            .get("temp_c")
            .and_then(JsonValue::as_f64)
            .or_else(|| e.get("temp_c").and_then(JsonValue::as_i64).map(|v| v as f64))
            .ok_or_else(|| {
                ValidationError::new("periods", format!("entry {i} missing or invalid `temp_c`"))
            })?;
        if !(MIN_TEMP_C..=MAX_TEMP_C).contains(&temp_c) {
            return Err(ValidationError::new(
                "periods",
                format!("entry {i} temp_c out of plausible range"),
            ));
        }
        let condition = e
            .get("condition")
            .and_then(JsonValue::as_str)
            .and_then(WeatherCondition::parse)
            .unwrap_or(WeatherCondition::Sunny);
        let precipitation_probability = match e.get("precipitation_probability") {
            None | Some(JsonValue::Null) => None,
            Some(v) => {
                let n = v
                    .as_i64()
                    .or_else(|| v.as_f64().map(|raw| raw.round() as i64))
                    .ok_or_else(|| {
                        ValidationError::new(
                            "periods",
                            format!("entry {i} invalid `precipitation_probability`"),
                        )
                    })?;
                if !(0..=100).contains(&n) {
                    return Err(ValidationError::new(
                        "periods",
                        format!("entry {i} precipitation_probability must be 0..=100"),
                    ));
                }
                Some(n as u32)
            }
        };
        out.push(DayPeriodWeather {
            slot,
            temp_c,
            condition,
            precipitation_probability,
        });
    }
    Ok(out)
}

fn parse_rfc3339_or_none(raw: &str) -> Option<&str> {
    // v1 does minimal validation: just sanity-check RFC 3339 shape
    // (YYYY-MM-DDTHH:MM:SS...Z or with +HH:MM offset). If the shape
    // doesn't match, drop the field and let render skip the label.
    let looks_like_date = raw.len() >= 20
        && raw.as_bytes().get(4) == Some(&b'-')
        && raw.as_bytes().get(7) == Some(&b'-')
        && raw.as_bytes().get(10) == Some(&b'T');
    if looks_like_date {
        Some(raw)
    } else {
        makepad_widgets::log!("org.octos.app weather: updated_at {:?} does not look like RFC 3339, ignoring", raw);
        None
    }
}

// ----- Splash DSL render -----

/// Escape a string for safe embedding inside a Splash DSL double-quoted string literal.
///
/// Rules (per spec §校验规则 Splash-safe escape):
///  - `"`  → `\"`
///  - `\`  → `\\`
///  - `\n` → single space
///  - `\r` → single space
///  - `\t` → single space
///  - other C0 control chars (U+0000..U+001F) → single space
pub fn splash_escape(input: &str) -> String {
    let mut out = String::with_capacity(input.len());
    for ch in input.chars() {
        match ch {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' | '\r' | '\t' => out.push(' '),
            c if (c as u32) < 0x20 => out.push(' '),
            c => out.push(c),
        }
    }
    out
}

/// Format a temperature with the original spec semantics: whole
/// integers stay as integers, fractional values get one decimal.
/// Independent of Splash float literal rules — this is user-facing
/// display text, not DSL source.
fn fmt_temp(t: f64) -> String {
    if (t.round() - t).abs() < 1e-6 {
        format!("{}", t.round() as i64)
    } else {
        format!("{t:.1}")
    }
}

fn copy<'a>(app_language: AppLanguage, en: &'a str, zh: &'a str) -> &'a str {
    match app_language {
        AppLanguage::English => en,
        AppLanguage::ChineseSimplified => zh,
    }
}

fn has_guidance_layout(state: &RenderedWeather) -> bool {
    state.high_c.is_some() || state.low_c.is_some() || !state.periods.is_empty()
}

fn is_wet_period(period: &DayPeriodWeather) -> bool {
    matches!(period.condition, WeatherCondition::Rainy | WeatherCondition::Stormy)
        || period.precipitation_probability.unwrap_or(0) >= 45
}

fn has_rain_risk(state: &RenderedWeather) -> bool {
    state
        .periods
        .iter()
        .any(is_wet_period)
        || state.precipitation_probability_max.unwrap_or(0.0) >= 45.0
        || matches!(state.condition, WeatherCondition::Rainy | WeatherCondition::Stormy)
}

fn day_swing(state: &RenderedWeather) -> f64 {
    match (state.high_c, state.low_c) {
        (Some(high), Some(low)) => high - low,
        _ => 0.0,
    }
}

fn guidance_header(app_language: AppLanguage) -> &'static str {
    copy(app_language, "Today's guidance", "今天怎么穿")
}

fn period_label(slot: DayPeriodSlot, app_language: AppLanguage) -> &'static str {
    match (slot, app_language) {
        (DayPeriodSlot::Morning, AppLanguage::English) => "Morning",
        (DayPeriodSlot::Morning, AppLanguage::ChineseSimplified) => "早上",
        (DayPeriodSlot::Noon, AppLanguage::English) => "Noon",
        (DayPeriodSlot::Noon, AppLanguage::ChineseSimplified) => "中午",
        (DayPeriodSlot::Night, AppLanguage::English) => "Night",
        (DayPeriodSlot::Night, AppLanguage::ChineseSimplified) => "晚上",
    }
}

fn headline_advice(state: &RenderedWeather, app_language: AppLanguage) -> &'static str {
    let high = state.high_c.unwrap_or(state.temp_c);
    let low = state.low_c.unwrap_or(state.temp_c);

    if has_rain_risk(state) {
        copy(
            app_language,
            "Umbrella + light waterproof layer",
            "雨具 + 轻薄防水外套",
        )
    } else if high >= 29.0 {
        copy(app_language, "T-shirt + sun protection", "短袖 + 做好防晒")
    } else if low <= 5.0 {
        copy(app_language, "Warm coat + thermal layer", "厚外套 + 保暖内层")
    } else if low < 14.0 {
        copy(app_language, "Light jacket + long pants", "薄外套 + 长裤")
    } else if high >= 24.0 && low >= 18.0 {
        copy(app_language, "Single layer is enough", "单层上衣就够")
    } else {
        copy(app_language, "Light layers work best", "轻薄叠穿最稳妥")
    }
}

fn summary_advice(state: &RenderedWeather, app_language: AppLanguage) -> &'static str {
    let high = state.high_c.unwrap_or(state.temp_c);
    let low = state.low_c.unwrap_or(state.temp_c);
    let swing = day_swing(state);

    if has_rain_risk(state) && swing >= 8.0 {
        copy(
            app_language,
            "Showers and a temperature swing mean a layer and an umbrella are both worth carrying.",
            "有雨且温差明显，外套和雨具都别省。",
        )
    } else if has_rain_risk(state) {
        copy(
            app_language,
            "Rain chances are high today, so keep a compact umbrella within reach.",
            "今天有雨，通勤最好备一把伞。",
        )
    } else if swing >= 8.0 {
        copy(
            app_language,
            "Cool morning, warmer noon. Keep a light outer layer with you.",
            "早晚偏凉，中午回暖，外套建议随身带。",
        )
    } else if high >= 29.0 {
        copy(
            app_language,
            "The afternoon runs hot, so lighter clothes and hydration will feel better.",
            "白天会偏热，注意补水和防晒。",
        )
    } else if low <= 5.0 {
        copy(
            app_language,
            "It stays chilly for most of the day, so don't skip the warm outer layer.",
            "全天偏冷，保暖层要穿足。",
        )
    } else {
        copy(
            app_language,
            "Conditions stay fairly comfortable through the day, with no major outfit changes needed.",
            "体感总体舒服，出门穿搭不用太折腾。",
        )
    }
}

fn period_advice(period: &DayPeriodWeather, app_language: AppLanguage) -> &'static str {
    if is_wet_period(period) {
        if period.temp_c <= 8.0 {
            copy(app_language, "Warm coat + umbrella", "厚外套和雨具一起带")
        } else if period.temp_c < 18.0 {
            copy(app_language, "Bring an umbrella and keep a jacket on", "带伞，外套别省")
        } else {
            copy(app_language, "Light top is fine, but keep rain gear", "带伞，轻便上衣即可")
        }
    } else if period.temp_c <= 5.0 {
        copy(app_language, "Heavy outer layer", "厚外套更稳妥")
    } else if period.temp_c <= 12.0 {
        copy(app_language, "Keep a jacket on", "外套别省")
    } else if period.temp_c <= 18.0 {
        copy(app_language, "Light jacket works best", "薄外套更舒服")
    } else if period.temp_c <= 25.0 {
        copy(app_language, "Single layer feels good", "单穿会比较舒适")
    } else {
        copy(app_language, "Short sleeves are comfortable", "短袖会更轻松")
    }
}

fn chip_labels(state: &RenderedWeather, app_language: AppLanguage) -> Vec<&'static str> {
    let mut chips = Vec::new();
    let swing = day_swing(state);
    let has_rain = has_rain_risk(state);
    let high = state.high_c.unwrap_or(state.temp_c);
    let low = state.low_c.unwrap_or(state.temp_c);

    if swing >= 8.0 {
        chips.push(copy(app_language, "Large temp swing", "温差较大"));
    }
    if has_rain {
        chips.push(copy(app_language, "Bring an umbrella", "记得带伞"));
    } else {
        if high <= 28.0 {
            chips.push(copy(app_language, "Commute-friendly", "适合通勤"));
        }
        if high <= 26.0 && low >= 12.0 {
            chips.push(copy(app_language, "Good for outdoors", "适合户外"));
        }
    }
    if state.uv_index_max.unwrap_or(0.0) >= 6.0 || high >= 29.0 {
        chips.push(copy(app_language, "Sun protection", "注意防晒"));
    }
    if state.wind_kph.unwrap_or(0.0) >= 20.0 {
        chips.push(copy(app_language, "A bit breezy", "风会有点大"));
    }
    if chips.len() < 4 && state.humidity.unwrap_or(0) >= 85 {
        chips.push(copy(app_language, "Humid air", "空气偏潮"));
    }
    chips.truncate(4);
    chips
}

fn condition_summary(state: &RenderedWeather, app_language: AppLanguage) -> &'static str {
    copy(
        app_language,
        match state.condition {
            WeatherCondition::Sunny => "Bright and clear",
            WeatherCondition::Cloudy => "Cloud cover",
            WeatherCondition::Rainy => "Rain likely",
            WeatherCondition::Snowy => "Snowy",
            WeatherCondition::Stormy => "Storm risk",
            WeatherCondition::Foggy => "Low visibility",
        },
        match state.condition {
            WeatherCondition::Sunny => "晴朗舒展",
            WeatherCondition::Cloudy => "云层较多",
            WeatherCondition::Rainy => "降雨概率高",
            WeatherCondition::Snowy => "有降雪迹象",
            WeatherCondition::Stormy => "对流偏强",
            WeatherCondition::Foggy => "能见度偏低",
        },
    )
}

#[derive(Debug, Clone, Default)]
struct GuidanceTextSlot {
    visible: bool,
    text: String,
}

#[derive(Debug, Clone, Default)]
struct GuidancePeriodSlot {
    visible: bool,
    label: String,
    advice: String,
    temp_text: String,
}

#[derive(Debug, Clone, Default)]
struct GuidanceChipSlot {
    visible: bool,
    text: String,
}

#[derive(Debug, Clone)]
struct GuidanceTemplateViewModel {
    bg_color: String,
    location: String,
    symbol: String,
    temp_text: String,
    range: GuidanceTextSlot,
    condition_summary: String,
    updated: GuidanceTextSlot,
    feels_like: GuidanceTextSlot,
    humidity: GuidanceTextSlot,
    wind: GuidanceTextSlot,
    guidance_header: String,
    headline: String,
    summary: String,
    periods_section_visible: bool,
    period_1: GuidancePeriodSlot,
    period_2: GuidancePeriodSlot,
    period_3: GuidancePeriodSlot,
    chips_section_visible: bool,
    chip_1: GuidanceChipSlot,
    chip_2: GuidanceChipSlot,
    chip_3: GuidanceChipSlot,
    chip_4: GuidanceChipSlot,
}

impl GuidanceTemplateViewModel {
    fn from_weather(state: &RenderedWeather, app_language: AppLanguage) -> Self {
        let updated_text = state.updated_at.as_ref().map(|updated| {
            let prefix = tr_key(app_language, "agent_to_app.weather.updated_at_prefix");
            format!("{prefix} {updated}")
        });
        let feels_like_text = state.feels_like_c.map(|feels| {
            let label = tr_key(app_language, "agent_to_app.weather.feels_like");
            format!("{label} {}\u{00B0}", fmt_temp(feels))
        });
        let humidity_text = state.humidity.map(|humidity| {
            let label = tr_key(app_language, "agent_to_app.weather.humidity");
            format!("{label} {humidity}%")
        });
        let wind_text = state.wind_kph.map(|wind_kph| {
            let label = tr_key(app_language, "agent_to_app.weather.wind");
            format!("{label} {} km/h", fmt_temp(wind_kph))
        });
        let period_1 = state.periods.first().map(|period| GuidancePeriodSlot {
            visible: true,
            label: period_label(period.slot, app_language).to_string(),
            advice: period_advice(period, app_language).to_string(),
            temp_text: format!("{}\u{00B0}", fmt_temp(period.temp_c)),
        }).unwrap_or_default();
        let period_2 = state.periods.get(1).map(|period| GuidancePeriodSlot {
            visible: true,
            label: period_label(period.slot, app_language).to_string(),
            advice: period_advice(period, app_language).to_string(),
            temp_text: format!("{}\u{00B0}", fmt_temp(period.temp_c)),
        }).unwrap_or_default();
        let period_3 = state.periods.get(2).map(|period| GuidancePeriodSlot {
            visible: true,
            label: period_label(period.slot, app_language).to_string(),
            advice: period_advice(period, app_language).to_string(),
            temp_text: format!("{}\u{00B0}", fmt_temp(period.temp_c)),
        }).unwrap_or_default();
        let chips = chip_labels(state, app_language);
        let chip_1 = chips.first().map(|text| GuidanceChipSlot {
            visible: true,
            text: (*text).to_string(),
        }).unwrap_or_default();
        let chip_2 = chips.get(1).map(|text| GuidanceChipSlot {
            visible: true,
            text: (*text).to_string(),
        }).unwrap_or_default();
        let chip_3 = chips.get(2).map(|text| GuidanceChipSlot {
            visible: true,
            text: (*text).to_string(),
        }).unwrap_or_default();
        let chip_4 = chips.get(3).map(|text| GuidanceChipSlot {
            visible: true,
            text: (*text).to_string(),
        }).unwrap_or_default();

        Self {
            bg_color: format!("#x{}", state.condition.bg_color_hex()),
            location: state.location.clone(),
            symbol: state.condition.symbol().to_string(),
            temp_text: format!("{}\u{00B0}", fmt_temp(state.temp_c)),
            range: GuidanceTextSlot {
                visible: state.high_c.is_some() && state.low_c.is_some(),
                text: match (state.high_c, state.low_c) {
                    (Some(high), Some(low)) => {
                        format!("{}\u{00B0} / {}\u{00B0}", fmt_temp(high), fmt_temp(low))
                    }
                    _ => String::new(),
                },
            },
            condition_summary: condition_summary(state, app_language).to_string(),
            updated: GuidanceTextSlot {
                visible: updated_text.is_some(),
                text: updated_text.unwrap_or_default(),
            },
            feels_like: GuidanceTextSlot {
                visible: feels_like_text.is_some(),
                text: feels_like_text.unwrap_or_default(),
            },
            humidity: GuidanceTextSlot {
                visible: humidity_text.is_some(),
                text: humidity_text.unwrap_or_default(),
            },
            wind: GuidanceTextSlot {
                visible: wind_text.is_some(),
                text: wind_text.unwrap_or_default(),
            },
            guidance_header: guidance_header(app_language).to_string(),
            headline: headline_advice(state, app_language).to_string(),
            summary: summary_advice(state, app_language).to_string(),
            periods_section_visible: !state.periods.is_empty(),
            period_1,
            period_2,
            period_3,
            chips_section_visible: !chips.is_empty(),
            chip_1,
            chip_2,
            chip_3,
            chip_4,
        }
    }

    #[cfg(test)]
    fn bindings(&self) -> Vec<(&'static str, String)> {
        let mut bindings = vec![
            ("$state.hero.bg_color", self.bg_color.clone()),
            ("$state.location", splash_escape(&self.location)),
            ("$state.hero.symbol", splash_escape(&self.symbol)),
            ("$state.hero.temp_text", splash_escape(&self.temp_text)),
            ("$state.range.visible", self.range.visible.to_string()),
            ("$state.range.text", splash_escape(&self.range.text)),
            ("$state.condition_summary", splash_escape(&self.condition_summary)),
            ("$state.updated.visible", self.updated.visible.to_string()),
            ("$state.updated.text", splash_escape(&self.updated.text)),
            ("$state.feels_like.visible", self.feels_like.visible.to_string()),
            ("$state.feels_like.text", splash_escape(&self.feels_like.text)),
            ("$state.humidity.visible", self.humidity.visible.to_string()),
            ("$state.humidity.text", splash_escape(&self.humidity.text)),
            ("$state.wind.visible", self.wind.visible.to_string()),
            ("$state.wind.text", splash_escape(&self.wind.text)),
            ("$state.guidance_header", splash_escape(&self.guidance_header)),
            ("$state.headline", splash_escape(&self.headline)),
            ("$state.summary", splash_escape(&self.summary)),
            ("$state.periods_section.visible", self.periods_section_visible.to_string()),
            ("$state.period_1.visible", self.period_1.visible.to_string()),
            ("$state.period_1.label", splash_escape(&self.period_1.label)),
            ("$state.period_1.advice", splash_escape(&self.period_1.advice)),
            ("$state.period_1.temp_text", splash_escape(&self.period_1.temp_text)),
            ("$state.period_2.visible", self.period_2.visible.to_string()),
            ("$state.period_2.label", splash_escape(&self.period_2.label)),
            ("$state.period_2.advice", splash_escape(&self.period_2.advice)),
            ("$state.period_2.temp_text", splash_escape(&self.period_2.temp_text)),
            ("$state.period_3.visible", self.period_3.visible.to_string()),
            ("$state.period_3.label", splash_escape(&self.period_3.label)),
            ("$state.period_3.advice", splash_escape(&self.period_3.advice)),
            ("$state.period_3.temp_text", splash_escape(&self.period_3.temp_text)),
            ("$state.chips_section.visible", self.chips_section_visible.to_string()),
            ("$state.chip_1.visible", self.chip_1.visible.to_string()),
            ("$state.chip_1.text", splash_escape(&self.chip_1.text)),
            ("$state.chip_2.visible", self.chip_2.visible.to_string()),
            ("$state.chip_2.text", splash_escape(&self.chip_2.text)),
            ("$state.chip_3.visible", self.chip_3.visible.to_string()),
            ("$state.chip_3.text", splash_escape(&self.chip_3.text)),
            ("$state.chip_4.visible", self.chip_4.visible.to_string()),
            ("$state.chip_4.text", splash_escape(&self.chip_4.text)),
        ];
        bindings.sort_by_key(|(token, _)| Reverse(token.len()));
        bindings
    }

    fn template_state(&self) -> JsonValue {
        serde_json::json!({
            "hero": {
                "bg_color": self.bg_color,
                "symbol": self.symbol,
                "temp_text": self.temp_text,
            },
            "location": self.location,
            "range": {
                "visible": self.range.visible,
                "text": self.range.text,
            },
            "condition_summary": self.condition_summary,
            "updated": {
                "visible": self.updated.visible,
                "text": self.updated.text,
            },
            "feels_like": {
                "visible": self.feels_like.visible,
                "text": self.feels_like.text,
            },
            "humidity": {
                "visible": self.humidity.visible,
                "text": self.humidity.text,
            },
            "wind": {
                "visible": self.wind.visible,
                "text": self.wind.text,
            },
            "guidance_header": self.guidance_header,
            "headline": self.headline,
            "summary": self.summary,
            "periods_section": {
                "visible": self.periods_section_visible,
            },
            "period_1": {
                "visible": self.period_1.visible,
                "label": self.period_1.label,
                "advice": self.period_1.advice,
                "temp_text": self.period_1.temp_text,
            },
            "period_2": {
                "visible": self.period_2.visible,
                "label": self.period_2.label,
                "advice": self.period_2.advice,
                "temp_text": self.period_2.temp_text,
            },
            "period_3": {
                "visible": self.period_3.visible,
                "label": self.period_3.label,
                "advice": self.period_3.advice,
                "temp_text": self.period_3.temp_text,
            },
            "chips_section": {
                "visible": self.chips_section_visible,
            },
            "chip_1": {
                "visible": self.chip_1.visible,
                "text": self.chip_1.text,
            },
            "chip_2": {
                "visible": self.chip_2.visible,
                "text": self.chip_2.text,
            },
            "chip_3": {
                "visible": self.chip_3.visible,
                "text": self.chip_3.text,
            },
            "chip_4": {
                "visible": self.chip_4.visible,
                "text": self.chip_4.text,
            },
        })
    }
}

/// **Test-only helper**. Production code MUST NOT call this — it performs
/// a direct `str::replace` binding against the raw template source,
/// bypassing the SplashHost W5 / W7 / attribution guards. Exposing this
/// as a production fallback would let the system emit Splash that a
/// guard had already rejected, defeating the core safety boundary.
///
/// Gated behind `#[cfg(test)]` so the bypass cannot accidentally land
/// on a hot path; existing tests that want to compare bypass output
/// against host output (to detect drift between the two binders) can
/// still call it.
#[cfg(test)]
fn guidance_template_source() -> &'static str {
    // Use the single source of truth from `templates::` so test-only
    // bypass output cannot drift away from what the SplashHost sees in
    // production (P1a single-source invariant).
    super::templates::WEATHER_CARD_STANDARD
}

#[cfg(test)]
fn bind_guidance_template(view_model: &GuidanceTemplateViewModel) -> String {
    let mut rendered = guidance_template_source().to_string();
    for (token, value) in view_model.bindings() {
        rendered = rendered.replace(token, &value);
    }
    rendered
}

fn render_legacy_weather(state: &RenderedWeather, app_language: AppLanguage) -> String {
    let mut out = String::with_capacity(1024);

    out.push_str("RoundedView {\n");
    out.push_str("  width: Fill height: Fit flow: Down\n");
    out.push_str("  padding: Inset{left: 20. right: 20. top: 16. bottom: 16.}\n");
    out.push_str("  spacing: 10.\n");
    out.push_str(&format!("  draw_bg.color: #x{}\n", state.condition.bg_color_hex()));
    out.push_str("  draw_bg.border_radius: 12.\n");

    out.push_str("  View { width: Fill height: Fit flow: Right spacing: 8. align: Align{y: 0.5}\n");
    out.push_str(&format!(
        "    Label {{ width: Fill text: \"{}\" draw_text.color: #xffffff draw_text.text_style.font_size: 18. }}\n",
        splash_escape(&state.location),
    ));
    if let Some(updated) = &state.updated_at {
        let prefix = tr_key(app_language, "agent_to_app.weather.updated_at_prefix");
        out.push_str(&format!(
            "    Label {{ text: \"{} {}\" draw_text.color: #xffffffaa draw_text.text_style.font_size: 10. }}\n",
            splash_escape(prefix),
            splash_escape(updated),
        ));
    }
    out.push_str("  }\n");

    out.push_str("  View { width: Fill height: Fit flow: Right spacing: 12. align: Align{y: 0.5}\n");
    out.push_str(&format!(
        "    Label {{ text: \"{}\" draw_text.color: #ffffff draw_text.text_style.font_size: 36. }}\n",
        splash_escape(state.condition.symbol()),
    ));
    out.push_str(&format!(
        "    Label {{ text: \"{}\u{00B0}\" draw_text.color: #ffffff draw_text.text_style.font_size: 44. }}\n",
        fmt_temp(state.temp_c),
    ));
    out.push_str(
        "    View { width: Fill height: Fit flow: Down spacing: 2. align: Align{x: 1.0}\n",
    );
    if let Some(feels) = state.feels_like_c {
        let label = tr_key(app_language, "agent_to_app.weather.feels_like");
        out.push_str(&format!(
            "      Label {{ text: \"{} {}\u{00B0}\" draw_text.color: #xffffffcc draw_text.text_style.font_size: 11. }}\n",
            splash_escape(label),
            fmt_temp(feels),
        ));
    }
    if let Some(h) = state.humidity {
        let label = tr_key(app_language, "agent_to_app.weather.humidity");
        out.push_str(&format!(
            "      Label {{ text: \"{} {}%\" draw_text.color: #xffffffcc draw_text.text_style.font_size: 11. }}\n",
            splash_escape(label),
            h,
        ));
    }
    if let Some(w) = state.wind_kph {
        let label = tr_key(app_language, "agent_to_app.weather.wind");
        out.push_str(&format!(
            "      Label {{ text: \"{} {} km/h\" draw_text.color: #xffffffcc draw_text.text_style.font_size: 11. }}\n",
            splash_escape(label),
            fmt_temp(w),
        ));
    }
    out.push_str("    }\n");
    out.push_str("  }\n");

    if !state.forecast.is_empty() {
        let forecast_label = tr_key(app_language, "agent_to_app.weather.forecast");
        out.push_str(&format!(
            "  Label {{ text: \"{}\" draw_text.color: #xffffffaa draw_text.text_style.font_size: 10. }}\n",
            splash_escape(forecast_label),
        ));
        out.push_str("  View { width: Fill height: Fit flow: Right spacing: 6.\n");
        for entry in &state.forecast {
            out.push_str("    RoundedView { width: Fit height: Fit flow: Down spacing: 2.\n");
            out.push_str("      padding: Inset{left: 8. right: 8. top: 6. bottom: 6.}\n");
            out.push_str("      draw_bg.color: #xffffff22\n");
            out.push_str("      draw_bg.border_radius: 6.\n");
            out.push_str(&format!(
                "      Label {{ text: \"{}\" draw_text.color: #ffffff draw_text.text_style.font_size: 10. }}\n",
                splash_escape(&entry.day),
            ));
            out.push_str(&format!(
                "      Label {{ text: \"{}\" draw_text.color: #ffffff draw_text.text_style.font_size: 16. }}\n",
                splash_escape(entry.condition.symbol()),
            ));
            out.push_str(&format!(
                "      Label {{ text: \"{}\u{00B0}/{}\u{00B0}\" draw_text.color: #xffffffcc draw_text.text_style.font_size: 10. }}\n",
                fmt_temp(entry.high_c),
                fmt_temp(entry.low_c),
            ));
            out.push_str("    }\n");
        }
        out.push_str("  }\n");
    }

    out.push_str("}\n");
    out
}

#[allow(dead_code)]
fn render_guidance_weather_inline(state: &RenderedWeather, app_language: AppLanguage) -> String {
    let mut out = String::with_capacity(2300);
    let chips = chip_labels(state, app_language);

    out.push_str("RoundedView {\n");
    out.push_str("  width: Fill height: Fit flow: Down\n");
    out.push_str("  padding: Inset{left: 18. right: 18. top: 16. bottom: 16.}\n");
    out.push_str("  spacing: 14.\n");
    out.push_str(&format!("  draw_bg.color: #x{}\n", state.condition.bg_color_hex()));
    out.push_str("  draw_bg.border_radius: 14.\n");

    out.push_str("  View { width: Fill height: Fit flow: Right spacing: 8. align: Align{y: 0.2}\n");
    out.push_str("    View { width: Fill height: Fit flow: Down spacing: 10.\n");
    out.push_str(&format!(
        "      Label {{ text: \"{}\" draw_text.color: #xffffff draw_text.text_style.font_size: 18. }}\n",
        splash_escape(&state.location),
    ));
    out.push_str("      View { width: Fill height: Fit flow: Right spacing: 12. align: Align{y: 0.5}\n");
    out.push_str(&format!(
        "        Label {{ text: \"{}\" draw_text.color: #xffffff draw_text.text_style.font_size: 32. }}\n",
        splash_escape(state.condition.symbol()),
    ));
    out.push_str(&format!(
        "        Label {{ text: \"{}\u{00B0}\" draw_text.color: #xffffff draw_text.text_style.font_size: 46. }}\n",
        fmt_temp(state.temp_c),
    ));
    out.push_str("        View { width: Fit height: Fit flow: Down spacing: 2.\n");
    if let (Some(high), Some(low)) = (state.high_c, state.low_c) {
        out.push_str(&format!(
            "          Label {{ text: \"{}\u{00B0} / {}\u{00B0}\" draw_text.color: #ffffffdd draw_text.text_style.font_size: 13. }}\n",
            fmt_temp(high),
            fmt_temp(low),
        ));
    }
    out.push_str(&format!(
        "          Label {{ text: \"{}\" draw_text.color: #xffffffbb draw_text.text_style.font_size: 11. }}\n",
        splash_escape(copy(
            app_language,
            match state.condition {
                WeatherCondition::Sunny => "Bright and clear",
                WeatherCondition::Cloudy => "Cloud cover",
                WeatherCondition::Rainy => "Rain likely",
                WeatherCondition::Snowy => "Snowy",
                WeatherCondition::Stormy => "Storm risk",
                WeatherCondition::Foggy => "Low visibility",
            },
            match state.condition {
                WeatherCondition::Sunny => "晴朗舒展",
                WeatherCondition::Cloudy => "云层较多",
                WeatherCondition::Rainy => "降雨概率高",
                WeatherCondition::Snowy => "有降雪迹象",
                WeatherCondition::Stormy => "对流偏强",
                WeatherCondition::Foggy => "能见度偏低",
            }
        )),
    ));
    out.push_str("        }\n");
    out.push_str("      }\n");
    out.push_str("    }\n");

    out.push_str("    View { width: Fit height: Fit flow: Down spacing: 3. align: Align{x: 1.0}\n");
    if let Some(updated) = &state.updated_at {
        let prefix = tr_key(app_language, "agent_to_app.weather.updated_at_prefix");
        out.push_str(&format!(
            "      Label {{ text: \"{} {}\" draw_text.color: #xffffffbb draw_text.text_style.font_size: 10. }}\n",
            splash_escape(prefix),
            splash_escape(updated),
        ));
    }
    if let Some(feels) = state.feels_like_c {
        let label = tr_key(app_language, "agent_to_app.weather.feels_like");
        out.push_str(&format!(
            "      Label {{ text: \"{} {}\u{00B0}\" draw_text.color: #ffffff draw_text.text_style.font_size: 11. }}\n",
            splash_escape(label),
            fmt_temp(feels),
        ));
    }
    if let Some(h) = state.humidity {
        let label = tr_key(app_language, "agent_to_app.weather.humidity");
        out.push_str(&format!(
            "      Label {{ text: \"{} {}%\" draw_text.color: #ffffff draw_text.text_style.font_size: 11. }}\n",
            splash_escape(label),
            h,
        ));
    }
    if let Some(w) = state.wind_kph {
        let label = tr_key(app_language, "agent_to_app.weather.wind");
        out.push_str(&format!(
            "      Label {{ text: \"{} {} km/h\" draw_text.color: #ffffff draw_text.text_style.font_size: 11. }}\n",
            splash_escape(label),
            fmt_temp(w),
        ));
    }
    out.push_str("    }\n");
    out.push_str("  }\n");

    out.push_str("  RoundedView {\n");
    out.push_str("    width: Fill height: Fit flow: Down spacing: 10.\n");
    out.push_str("    padding: Inset{left: 14. right: 14. top: 14. bottom: 14.}\n");
    out.push_str("    draw_bg.color: #xffffff\n");
    out.push_str("    draw_bg.border_radius: 12.\n");
    out.push_str(&format!(
        "    Label {{ text: \"{}\" draw_text.color: #x43516A draw_text.text_style.font_size: 12. }}\n",
        splash_escape(guidance_header(app_language)),
    ));
    out.push_str(&format!(
        "    Label {{ text: \"{}\" draw_text.color: #x182133 draw_text.text_style.font_size: 22. }}\n",
        splash_escape(headline_advice(state, app_language)),
    ));
    out.push_str(&format!(
        "    Label {{ text: \"{}\" draw_text.color: #x526076 draw_text.text_style.font_size: 12. }}\n",
        splash_escape(summary_advice(state, app_language)),
    ));

    if !state.periods.is_empty() {
        out.push_str("    View { width: Fill height: Fit flow: Down spacing: 6.\n");
        for period in &state.periods {
            out.push_str("      RoundedView {\n");
            out.push_str("        width: Fill height: Fit flow: Right spacing: 10.\n");
            out.push_str("        padding: Inset{left: 10. right: 10. top: 9. bottom: 9.}\n");
            out.push_str("        draw_bg.color: #xEEF3F8\n");
            out.push_str("        draw_bg.border_radius: 10.\n");
            out.push_str("        View { width: Fill height: Fit flow: Down spacing: 2.\n");
            out.push_str(&format!(
                "          Label {{ text: \"{}\" draw_text.color: #x4E627C draw_text.text_style.font_size: 11. }}\n",
                splash_escape(period_label(period.slot, app_language)),
            ));
            out.push_str(&format!(
                "          Label {{ text: \"{}\" draw_text.color: #x182133 draw_text.text_style.font_size: 13. }}\n",
                splash_escape(period_advice(period, app_language)),
            ));
            out.push_str("        }\n");
            out.push_str(&format!(
                "        Label {{ text: \"{}\u{00B0}\" draw_text.color: #x182133 draw_text.text_style.font_size: 18. }}\n",
                fmt_temp(period.temp_c),
            ));
            out.push_str("      }\n");
        }
        out.push_str("    }\n");
    }

    if !chips.is_empty() {
        out.push_str("    View { width: Fill height: Fit flow: Right spacing: 8.\n");
        for chip in chips {
            out.push_str("      RoundedView {\n");
            out.push_str("        width: Fit height: Fit flow: Right\n");
            out.push_str("        padding: Inset{left: 9. right: 9. top: 6. bottom: 6.}\n");
            out.push_str("        draw_bg.color: #xE7EEF7\n");
            out.push_str("        draw_bg.border_radius: 999.\n");
            out.push_str(&format!(
                "        Label {{ text: \"{}\" draw_text.color: #x516377 draw_text.text_style.font_size: 11. }}\n",
                splash_escape(chip),
            ));
            out.push_str("      }\n");
        }
        out.push_str("    }\n");
    }

    out.push_str("  }\n");
    out.push_str("}\n");
    out
}

fn render_guidance_weather(
    state: &RenderedWeather,
    app_language: AppLanguage,
) -> Result<String, RenderFailure> {
    let view_model = GuidanceTemplateViewModel::from_weather(state, app_language);
    let host = super::splash_host::splash_host();
    let chrome = capability_descriptors::chrome_for(TYPE_KEY).ok_or_else(|| {
        RenderFailure::Internal {
            reason: format!("missing capability descriptor for {TYPE_KEY}"),
        }
    })?;
    let handle = host
        .load_template("weather_guidance", "card_standard")
        .map_err(host_error_to_render_failure)?;
    host.render_to_splash(&handle, &view_model.template_state(), &chrome)
        .map_err(host_error_to_render_failure)
}

/// Classify a `HostError` into a `RenderFailure` that the dispatcher
/// can act on. Preflight-guard rejections (W5 / W7 / attribution /
/// schema / parse) become `HostRejected` — this is the safety boundary
/// the dispatcher must honor. Runtime binding / op-unsupported errors
/// become `HostError` (still not a valid Splash output, but not a
/// guard violation). `TemplateNotFound` becomes `TemplateMissing`.
pub(crate) fn host_error_to_render_failure(err: HostError) -> RenderFailure {
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

/// Top-level render function. Outputs a Splash DSL string that
/// conforms to the Canvas eval-path syntax (per
/// `makepad-2.0-splash` skill + spec §Splash 输出约束).
///
/// Returns `Err(RenderFailure)` when the SplashHost guard fires
/// (W5 / W7 / attribution / schema / parse), when render-time binding
/// fails, or when capability infrastructure is missing. The legacy
/// v1 code path (`render_legacy_weather`) remains infallible —
/// pre-dates SplashHost and renders its string directly.
pub fn render_weather(
    state: &RenderedWeather,
    app_language: AppLanguage,
) -> Result<String, RenderFailure> {
    if has_guidance_layout(state) {
        render_guidance_weather(state, app_language)
    } else {
        Ok(render_legacy_weather(state, app_language))
    }
}

// ----- tests -----

#[cfg(test)]
mod tests {
    use super::*;
    use makepad_widgets::ScriptNew;
    use serde_json::json;

    fn init_from(value: JsonValue) -> Result<Box<dyn RenderedApp>, ValidationError> {
        WeatherFactory.init(&value)
    }

    // P0 safety-boundary tests — classifier locks HostError → RenderFailure mapping.
    // Preflight-guard variants MUST map to `HostRejected` (they are the safety
    // boundary the dispatcher treats as "fall back to plain text; never
    // reconstruct Splash via bypass"). Runtime variants map to `HostError`.

    #[test]
    fn host_error_parse_error_maps_to_rejected() {
        let err = HostError::ParseError {
            message: "unterminated".into(),
            line: 42,
        };
        matches!(
            host_error_to_render_failure(err),
            RenderFailure::HostRejected { .. }
        )
        .then_some(())
        .expect("ParseError must map to HostRejected");
    }

    #[test]
    fn host_error_widget_not_allowed_maps_to_rejected() {
        let err = HostError::WidgetNotAllowed {
            name: "EvilWidget".into(),
            trust_level: Some("Sensitive".into()),
        };
        matches!(
            host_error_to_render_failure(err),
            RenderFailure::HostRejected { .. }
        )
        .then_some(())
        .expect("WidgetNotAllowed must map to HostRejected — never bypass");
    }

    #[test]
    fn host_error_local_function_not_allowed_maps_to_rejected() {
        let err = HostError::LocalFunctionNotAllowed {
            name: "exec_shell".into(),
        };
        matches!(
            host_error_to_render_failure(err),
            RenderFailure::HostRejected { .. }
        )
        .then_some(())
        .expect("LocalFunctionNotAllowed must map to HostRejected");
    }

    #[test]
    fn host_error_attribution_override_maps_to_rejected() {
        let err = HostError::AttributionFieldInTemplate {
            field: "capability_id".into(),
        };
        matches!(
            host_error_to_render_failure(err),
            RenderFailure::HostRejected { .. }
        )
        .then_some(())
        .expect("AttributionFieldInTemplate must map to HostRejected");
    }

    #[test]
    fn host_error_binding_path_not_in_schema_maps_to_rejected() {
        let err = HostError::BindingPathNotInSchema {
            path: "$state.hack".into(),
            app_type: "weather".into(),
            app_version: 2,
        };
        matches!(
            host_error_to_render_failure(err),
            RenderFailure::HostRejected { .. }
        )
        .then_some(())
        .expect("BindingPathNotInSchema must map to HostRejected");
    }

    #[test]
    fn host_error_binding_error_maps_to_host_error_not_rejected() {
        // BindingError is a runtime resolution failure, NOT a preflight
        // guard. It must not mask as HostRejected (which carries
        // security-boundary semantics).
        let err = HostError::BindingError {
            path: "$state.x".into(),
            message: "missing".into(),
        };
        matches!(
            host_error_to_render_failure(err),
            RenderFailure::HostError { .. }
        )
        .then_some(())
        .expect("BindingError must map to HostError (runtime), not HostRejected (guard)");
    }

    #[test]
    fn host_error_template_not_found_maps_to_template_missing() {
        let err = HostError::TemplateNotFound {
            capability_id: "weather_guidance".into(),
            template_id: "nonexistent".into(),
        };
        matches!(
            host_error_to_render_failure(err),
            RenderFailure::TemplateMissing { .. }
        )
        .then_some(())
        .expect("TemplateNotFound must map to TemplateMissing");
    }

    #[test]
    fn host_error_generated_slot_maps_to_host_error() {
        let err = HostError::GeneratedTemplateNotYetSupported;
        matches!(
            host_error_to_render_failure(err),
            RenderFailure::HostError { .. }
        )
        .then_some(())
        .expect("GeneratedTemplateNotYetSupported must map to HostError");
    }

    #[test]
    fn valid_payload_succeeds_and_renders() {
        let rendered = init_from(json!({
            "location": "Beijing",
            "temp_c": 22,
            "condition": "sunny",
            "humidity": 65
        }))
        .unwrap();
        let splash = rendered.render(AppLanguage::English).expect("render should succeed in this test");
        assert!(splash.contains("Beijing"));
        assert!(splash.contains("22"));
        assert!(splash.contains("Humidity"));
    }

    #[test]
    fn missing_required_location_fails_closed() {
        let Err(err) = init_from(json!({
            "temp_c": 22,
            "condition": "sunny"
        })) else {
            panic!("expected validation error for missing location");
        };
        assert_eq!(err.field, "location");
    }

    #[test]
    fn temperature_out_of_range_fails_closed() {
        let Err(err) = init_from(json!({
            "location": "Nowhere",
            "temp_c": -100,
            "condition": "sunny"
        })) else {
            panic!("expected validation error for out-of-range temp_c");
        };
        assert_eq!(err.field, "temp_c");
    }

    #[test]
    fn unknown_condition_falls_back_to_sunny_without_error() {
        let rendered = init_from(json!({
            "location": "Nowhere",
            "temp_c": 10,
            "condition": "alien_storm"
        }))
        .unwrap();
        // Should render successfully (no error); the symbol should be sunny's.
        let splash = rendered.render(AppLanguage::English).expect("render should succeed in this test");
        assert!(splash.contains("\u{2600}"));
    }

    #[test]
    fn optional_fields_absent_renders_minimum_card() {
        let rendered = init_from(json!({
            "location": "X",
            "temp_c": 0,
            "condition": "cloudy"
        }))
        .unwrap();
        let splash = rendered.render(AppLanguage::English).expect("render should succeed in this test");
        assert!(!splash.contains("Feels like"));
        assert!(!splash.contains("Humidity"));
        assert!(!splash.contains("Wind"));
        assert!(!splash.contains("Forecast"));
        assert!(!splash.contains("Updated"));
    }

    #[test]
    fn long_location_is_truncated_with_grapheme_ellipsis() {
        // 200 'A' characters; each 'A' is one grapheme and one byte.
        let long = "A".repeat(200);
        let rendered = init_from(json!({
            "location": long,
            "temp_c": 10,
            "condition": "sunny"
        }))
        .unwrap();
        let splash = rendered.render(AppLanguage::English).expect("render should succeed in this test");
        // The displayed location should be 64 A's + an ellipsis.
        let expected = "A".repeat(MAX_LOCATION_GRAPHEMES) + "\u{2026}";
        assert!(
            splash.contains(&expected),
            "expected truncated location in output; got: {}",
            splash,
        );
    }

    #[test]
    fn forecast_over_seven_truncated_to_seven() {
        let mut forecast = Vec::new();
        for i in 0..10 {
            forecast.push(json!({
                "day": format!("D{i}"),
                "high_c": 20,
                "low_c": 10,
                "condition": "sunny"
            }));
        }
        let rendered = init_from(json!({
            "location": "Nowhere",
            "temp_c": 15,
            "condition": "sunny",
            "forecast": forecast
        }))
        .unwrap();
        let splash = rendered.render(AppLanguage::English).expect("render should succeed in this test");
        // Only D0..D6 should appear; D7, D8, D9 should not.
        for i in 0..MAX_FORECAST_ENTRIES {
            assert!(splash.contains(&format!("D{i}")), "missing D{i}");
        }
        for i in MAX_FORECAST_ENTRIES..10 {
            assert!(!splash.contains(&format!("D{i}")), "unexpected D{i}");
        }
    }

    #[test]
    fn splash_escape_neutralizes_quotes_and_backslashes() {
        assert_eq!(splash_escape("Beijing\""), "Beijing\\\"");
        assert_eq!(splash_escape("a\\b"), "a\\\\b");
        assert_eq!(splash_escape("line1\nline2"), "line1 line2");
        assert_eq!(splash_escape("tab\there"), "tab here");
        assert_eq!(splash_escape("norm\x01al"), "norm al");
    }

    #[test]
    fn location_with_injection_attempt_is_escaped_in_output() {
        let rendered = init_from(json!({
            "location": "Beijing\"; rm -rf /\"",
            "temp_c": 10,
            "condition": "sunny"
        }))
        .unwrap();
        let splash = rendered.render(AppLanguage::English).expect("render should succeed in this test");
        // The escaped sequence should appear verbatim.
        assert!(splash.contains("Beijing\\\"; rm -rf /\\\""));
    }

    #[test]
    fn render_output_obeys_canvas_eval_syntax_requirements() {
        let rendered = init_from(json!({
            "location": "Test",
            "temp_c": 20,
            "condition": "sunny"
        }))
        .unwrap();
        let splash = rendered.render(AppLanguage::English).expect("render should succeed in this test");

        // Must use runtime Splash grammar, not compile-time DSL commas.
        assert!(!splash.contains(','), "output contains commas: {splash}");
        // Must use draw_bg.border_radius, not the invalid draw_bg.radius alias.
        assert!(
            splash.contains("draw_bg.border_radius"),
            "missing draw_bg.border_radius"
        );
        assert!(
            !splash.contains("draw_bg.radius"),
            "output contains invalid draw_bg.radius: {splash}",
        );
        // Must NOT use ScrollYView in eval path.
        assert!(!splash.contains("ScrollYView"));
        // Must NOT rely on show_bg: true on pre-styled views.
        assert!(!splash.contains("show_bg: true"));
        // Must use explicit Inset{...} for padding.
        assert!(splash.contains("Inset{"));
        // Every whole-number float literal must use the trailing-dot form,
        // never the explicit-zero form like `8.0` or `16.0`.
        for forbidden in &[
            "8.0 ", "8.0,", "10.0 ", "10.0,", "11.0 ", "11.0,",
            "12.0 ", "12.0,", "16.0 ", "16.0,", "18.0 ", "18.0,",
            "20.0 ", "20.0,", "36.0 ", "36.0,", "44.0 ", "44.0,",
        ] {
            assert!(
                !splash.contains(forbidden),
                "output contains forbidden whole-number float form {forbidden:?}: {splash}",
            );
        }
    }

    #[test]
    fn labels_resolve_via_app_language() {
        let rendered = init_from(json!({
            "location": "Test",
            "temp_c": 20,
            "condition": "sunny",
            "humidity": 50
        }))
        .unwrap();
        let en = rendered.render(AppLanguage::English).expect("render should succeed in this test");
        let zh = rendered.render(AppLanguage::ChineseSimplified).expect("render should succeed in this test");
        assert!(en.contains("Humidity"));
        assert!(zh.contains("湿度"));
        assert!(!en.contains("湿度"));
        assert!(!zh.contains("Humidity"));
    }

    #[test]
    fn payload_language_overrides_app_language_for_guidance_card() {
        let rendered = init_from(json!({
            "location": "Paris",
            "language": "zh-CN",
            "temp_c": 17,
            "high_c": 19,
            "low_c": 9,
            "condition": "cloudy",
            "feels_like_c": 16,
            "humidity": 69,
            "wind_kph": 8,
            "periods": [
                { "slot": "morning", "temp_c": 9, "condition": "cloudy", "precipitation_probability": 5 },
                { "slot": "noon", "temp_c": 17, "condition": "cloudy", "precipitation_probability": 0 },
                { "slot": "night", "temp_c": 18, "condition": "cloudy", "precipitation_probability": 10 }
            ]
        }))
        .unwrap();

        let splash = rendered.render(AppLanguage::English).expect("render should succeed in this test");
        assert!(splash.contains("今天怎么穿"), "missing zh guidance header: {splash}");
        assert!(splash.contains("早上"), "missing zh morning label: {splash}");
        assert!(!splash.contains("Today's guidance"), "unexpected en guidance header: {splash}");
        assert!(!splash.contains("Morning"), "unexpected en morning label: {splash}");
    }

    #[test]
    fn rendered_weather_splash_eval_parses_in_makepad_vm() {
        let rendered = init_from(json!({
            "location": "Beijing",
            "temp_c": 16,
            "high_c": 24,
            "low_c": 12,
            "condition": "cloudy",
            "feels_like_c": 17,
            "humidity": 81,
            "wind_kph": 3,
            "uv_index_max": 6,
            "precipitation_probability_max": 10,
            "updated_at": "2026-04-15T10:52:09.501678+00:00",
            "periods": [
                { "slot": "morning", "temp_c": 13, "condition": "cloudy", "precipitation_probability": 10 },
                { "slot": "noon", "temp_c": 24, "condition": "sunny", "precipitation_probability": 0 },
                { "slot": "night", "temp_c": 14, "condition": "cloudy", "precipitation_probability": 5 }
            ]
        }))
        .unwrap();
        let splash = rendered.render(AppLanguage::English).expect("render should succeed in this test");

        let mut cx = makepad_widgets::Cx::new(Box::new(|_, _| {}));
        cx.with_vm(|vm| {
            makepad_widgets::script_mod(vm);

            let script_mod = makepad_widgets::makepad_platform::ScriptMod {
                cargo_manifest_path: String::new(),
                module_path: String::new(),
                file: "weather_card_test".to_string(),
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

            assert!(!value.is_err(), "Splash eval errored for weather card: {splash}");
            assert!(!value.is_nil(), "Splash eval returned nil for weather card: {splash}");

            let _view = makepad_widgets::View::script_from_value(vm, value);
        });
    }

    #[test]
    fn weather_guidance_template_asset_exists_with_visibility_slots() {
        let path = format!(
            "{}/src/home/app_registry/templates/weather_guidance/card_standard.splash",
            env!("CARGO_MANIFEST_DIR")
        );
        let template = std::fs::read_to_string(&path)
            .unwrap_or_else(|err| panic!("missing weather guidance template at {path}: {err}"));

        for expected in &[
            "visible: $state.updated.visible",
            "visible: $state.range.visible",
            "visible: $state.periods_section.visible",
            "visible: $state.period_1.visible",
            "visible: $state.chips_section.visible",
            "visible: $state.chip_1.visible",
        ] {
            assert!(
                template.contains(expected),
                "template missing required binding {expected:?}: {template}",
            );
        }
    }

    #[test]
    fn guidance_template_keeps_card_renderable_when_optional_metrics_are_missing() {
        let rendered = init_from(json!({
            "location": "Oslo",
            "temp_c": 11,
            "high_c": 13,
            "low_c": 7,
            "condition": "cloudy",
            "periods": [
                { "slot": "morning", "temp_c": 8, "condition": "cloudy", "precipitation_probability": 10 },
                { "slot": "noon", "temp_c": 13, "condition": "cloudy", "precipitation_probability": 15 }
            ]
        }))
        .unwrap();

        let splash = rendered.render(AppLanguage::English).expect("render should succeed in this test");
        assert!(splash.contains("Today's guidance"), "missing guidance header: {splash}");
        assert!(splash.contains("Oslo"), "missing location binding: {splash}");
        assert!(splash.contains("visible: false"), "missing hidden optional slots: {splash}");
        assert!(!splash.contains("$state."), "unbound template token leaked: {splash}");
    }

    #[test]
    fn v2_guidance_payload_renders_periods_and_blue_cloudy_theme() {
        let rendered = init_from(json!({
            "location": "Beijing",
            "temp_c": 16,
            "high_c": 24,
            "low_c": 12,
            "condition": "cloudy",
            "feels_like_c": 17,
            "humidity": 81,
            "wind_kph": 3,
            "uv_index_max": 6,
            "precipitation_probability_max": 10,
            "periods": [
                { "slot": "morning", "temp_c": 13, "condition": "cloudy", "precipitation_probability": 10 },
                { "slot": "noon", "temp_c": 24, "condition": "sunny", "precipitation_probability": 0 },
                { "slot": "night", "temp_c": 14, "condition": "cloudy", "precipitation_probability": 5 }
            ]
        }))
        .unwrap();

        let splash = rendered.render(AppLanguage::ChineseSimplified).expect("render should succeed in this test");
        assert!(splash.contains("今天怎么穿"), "missing guidance header: {splash}");
        assert!(splash.contains("适合通勤") || splash.contains("温差较大"), "missing practical chips: {splash}");
        assert!(splash.contains("早上"), "missing morning period: {splash}");
        assert!(splash.contains("中午"), "missing noon period: {splash}");
        assert!(splash.contains("晚上"), "missing night period: {splash}");
        assert!(splash.contains("#x6F8AA4"), "missing updated cloudy blue tone: {splash}");
    }
}
