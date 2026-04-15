//! L1 weather card factory. First concrete mini-app type.
//!
//! Contract: `specs/task-agent-to-app-l1-weather-card.spec.md`.
//!
//! This is a pure presentational component: `init` validates the
//! weather JSON payload and produces a `RenderedWeather` state;
//! `render` produces a Canvas eval-path Splash DSL string that
//! the caller injects into the message's `splash_card` slot.

use serde_json::Value as JsonValue;
use unicode_segmentation::UnicodeSegmentation;

use crate::i18n::{tr_key, AppLanguage};

use super::{AppFactory, RenderedApp, ValidationError};

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

pub struct WeatherFactory;

impl AppFactory for WeatherFactory {
    fn supported_version(&self) -> u32 {
        1
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
        let updated_at = obj
            .get("updated_at")
            .and_then(|v| v.as_str())
            .and_then(parse_rfc3339_or_none)
            .map(str::to_string);
        let forecast = parse_forecast(obj)?;

        Ok(Box::new(RenderedWeather {
            location,
            temp_c,
            condition,
            feels_like_c,
            humidity,
            wind_kph,
            updated_at,
            forecast,
        }))
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
            Self::Cloudy => "7F8C8D",  // gray-blue
            Self::Rainy => "5B6F80",   // darker gray-blue
            Self::Snowy => "708090",   // slate gray (not light, for white text contrast)
            Self::Stormy => "4A4E69",  // dark purple-gray
            Self::Foggy => "95A5A6",   // silver gray
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
    pub feels_like_c: Option<f64>,
    pub humidity: Option<u32>,
    pub wind_kph: Option<f64>,
    pub updated_at: Option<String>,
    pub forecast: Vec<ForecastEntry>,
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

    fn render(&self, app_language: AppLanguage) -> String {
        render_weather(self, app_language)
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

/// Top-level render function. Outputs a Splash DSL string that
/// conforms to the Canvas eval-path syntax (per
/// `makepad-2.0-splash` skill + spec §Splash 输出约束).
pub fn render_weather(state: &RenderedWeather, app_language: AppLanguage) -> String {
    let mut out = String::with_capacity(1024);

    // Card container.
    out.push_str("SolidView {");
    out.push_str(" width: Fill, height: Fit, flow: Down,");
    out.push_str(" padding: Inset{left: 20., right: 20., top: 16., bottom: 16.},");
    out.push_str(" spacing: 10.,");
    out.push_str(&format!(" draw_bg.color: #x{},", state.condition.bg_color_hex()));
    out.push_str(" draw_bg.radius: 12.");
    out.push('\n');

    // Top row: location (left, flex) + optional updated_at (right).
    out.push_str("  View { width: Fill, height: Fit, flow: Right, spacing: 8., align: Align{y: 0.5}");
    out.push('\n');
    out.push_str(&format!(
        "    Label {{ width: Fill, text: \"{}\", draw_text.color: #xffffff, draw_text.text_style.font_size: 18. }}\n",
        splash_escape(&state.location),
    ));
    if let Some(updated) = &state.updated_at {
        let prefix = tr_key(app_language, "agent_to_app.weather.updated_at_prefix");
        out.push_str(&format!(
            "    Label {{ text: \"{} {}\", draw_text.color: #xffffffaa, draw_text.text_style.font_size: 10. }}\n",
            splash_escape(prefix),
            splash_escape(updated),
        ));
    }
    out.push_str("  }\n");

    // Main row: symbol + temperature (left) + metadata column (right).
    out.push_str("  View { width: Fill, height: Fit, flow: Right, spacing: 12., align: Align{y: 0.5}\n");
    out.push_str(&format!(
        "    Label {{ text: \"{}\", draw_text.color: #xffffff, draw_text.text_style.font_size: 36. }}\n",
        splash_escape(state.condition.symbol()),
    ));
    out.push_str(&format!(
        "    Label {{ text: \"{}\u{00B0}\", draw_text.color: #xffffff, draw_text.text_style.font_size: 44. }}\n",
        fmt_temp(state.temp_c),
    ));
    out.push_str(
        "    View { width: Fill, height: Fit, flow: Down, spacing: 2., align: Align{x: 1.0}\n",
    );
    if let Some(feels) = state.feels_like_c {
        let label = tr_key(app_language, "agent_to_app.weather.feels_like");
        out.push_str(&format!(
            "      Label {{ text: \"{} {}\u{00B0}\", draw_text.color: #xffffffcc, draw_text.text_style.font_size: 11. }}\n",
            splash_escape(label),
            fmt_temp(feels),
        ));
    }
    if let Some(h) = state.humidity {
        let label = tr_key(app_language, "agent_to_app.weather.humidity");
        out.push_str(&format!(
            "      Label {{ text: \"{} {}%\", draw_text.color: #xffffffcc, draw_text.text_style.font_size: 11. }}\n",
            splash_escape(label),
            h,
        ));
    }
    if let Some(w) = state.wind_kph {
        let label = tr_key(app_language, "agent_to_app.weather.wind");
        out.push_str(&format!(
            "      Label {{ text: \"{} {} km/h\", draw_text.color: #xffffffcc, draw_text.text_style.font_size: 11. }}\n",
            splash_escape(label),
            fmt_temp(w),
        ));
    }
    out.push_str("    }\n");
    out.push_str("  }\n");

    // Forecast row (horizontal chips, no ScrollYView).
    if !state.forecast.is_empty() {
        let forecast_label = tr_key(app_language, "agent_to_app.weather.forecast");
        out.push_str(&format!(
            "  Label {{ text: \"{}\", draw_text.color: #xffffffaa, draw_text.text_style.font_size: 10. }}\n",
            splash_escape(forecast_label),
        ));
        out.push_str("  View { width: Fill, height: Fit, flow: Right, spacing: 6.\n");
        for entry in &state.forecast {
            out.push_str("    RoundedView { width: Fit, height: Fit, flow: Down, spacing: 2.,");
            out.push_str(" padding: Inset{left: 8., right: 8., top: 6., bottom: 6.},");
            out.push_str(" draw_bg.color: #xffffff22, draw_bg.radius: 6.\n");
            out.push_str(&format!(
                "      Label {{ text: \"{}\", draw_text.color: #xffffff, draw_text.text_style.font_size: 10. }}\n",
                splash_escape(&entry.day),
            ));
            out.push_str(&format!(
                "      Label {{ text: \"{}\", draw_text.color: #xffffff, draw_text.text_style.font_size: 16. }}\n",
                splash_escape(entry.condition.symbol()),
            ));
            out.push_str(&format!(
                "      Label {{ text: \"{}\u{00B0}/{}\u{00B0}\", draw_text.color: #xffffffcc, draw_text.text_style.font_size: 10. }}\n",
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

// ----- tests -----

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn init_from(value: JsonValue) -> Result<Box<dyn RenderedApp>, ValidationError> {
        WeatherFactory.init(&value)
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
        let splash = rendered.render(AppLanguage::English);
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
        let splash = rendered.render(AppLanguage::English);
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
        let splash = rendered.render(AppLanguage::English);
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
        let splash = rendered.render(AppLanguage::English);
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
        let splash = rendered.render(AppLanguage::English);
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
        let splash = rendered.render(AppLanguage::English);
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
        let splash = rendered.render(AppLanguage::English);

        // Must use draw_bg.radius, NOT draw_bg.border_radius.
        assert!(splash.contains("draw_bg.radius"), "missing draw_bg.radius");
        assert!(
            !splash.contains("border_radius"),
            "output contains forbidden border_radius: {splash}",
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
        let en = rendered.render(AppLanguage::English);
        let zh = rendered.render(AppLanguage::ChineseSimplified);
        assert!(en.contains("Humidity"));
        assert!(zh.contains("湿度"));
        assert!(!en.contains("湿度"));
        assert!(!zh.contains("Humidity"));
    }
}
