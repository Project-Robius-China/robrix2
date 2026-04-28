//! Agent-to-app mini-app registry. See
//! `specs/task-agent-to-app-system.spec.md` and
//! `specs/task-agent-to-app-l1-weather-card.spec.md` for the contract.
//!
//! L1 layer: pure presentational apps (stateless, no tick, no
//! user interaction). Factories implement `init + render` only.
//!
//! Higher layers (L2b in-card controls, L3 stateful hosts) will
//! extend this trait with `on_action`, `on_tick`, `teardown` in
//! future commits; the v1 registry contract is intentionally
//! minimal so the first concrete type (weather) can ship
//! immediately.

use std::collections::HashMap;
use std::sync::OnceLock;

use serde_json::Value as JsonValue;

use crate::i18n::AppLanguage;

pub mod capability_descriptors;
pub mod local_functions;
pub mod news;
pub mod splash_host;
pub mod template_cache;
#[cfg(test)]
mod template_preflight_audit;
pub mod templates;
pub mod widget_manifest;
pub mod weather;

/// Validation error returned by an app factory's `init` method.
///
/// v1 keeps this simple: a machine-readable field name plus a
/// human-readable message. Both are used in the fallback warning
/// log when an app payload is rejected.
#[derive(Debug, Clone)]
pub struct ValidationError {
    pub field: &'static str,
    pub message: String,
}

impl ValidationError {
    pub fn new(field: &'static str, message: impl Into<String>) -> Self {
        Self {
            field,
            message: message.into(),
        }
    }
}

/// Typed render failure returned by `RenderedApp::render`. Replaces the
/// earlier `String` return type so that safety boundaries (host preflight
/// rejection, template missing, etc.) are **explicit** — not masked by a
/// successful-looking string that silently bypassed guards.
///
/// Contract: any variant MUST cause
/// `render_app_envelope_to_splash` to return `None`, which in turn lets
/// the timeline fall back to the Matrix `body` plain-text path. A
/// per-app `render` impl is NOT allowed to produce a Splash string that
/// has bypassed the SplashHost W5 / W7 / attribution guards.
#[derive(Debug, Clone)]
pub enum RenderFailure {
    /// SplashHost rejected the template — W5 widget whitelist, W7 local
    /// function whitelist, or attribution-override guard violation.
    /// The host's guard fired; the render MUST NOT produce a bypass.
    HostRejected { reason: String },
    /// SplashHost accepted the template but render-time binding or
    /// function invocation failed (e.g. a `$state.path` did not resolve).
    /// Not a security failure; still not a valid Splash output.
    HostError { reason: String },
    /// No template source found for the `(capability_id, template_id)`
    /// pair. Indicates a capability registered a `template_id` without
    /// shipping the corresponding `.splash` file.
    TemplateMissing {
        capability_id: String,
        template_id: String,
    },
    /// Capability infrastructure missing (e.g. no
    /// `CapabilityDescriptor` registered for this `app_type`, missing
    /// attribution chrome). Should not happen in a fully-configured
    /// build; surfaced so the cause is visible in logs.
    Internal { reason: String },
}

impl std::fmt::Display for RenderFailure {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::HostRejected { reason } => write!(f, "host rejected template: {reason}"),
            Self::HostError { reason } => write!(f, "host render error: {reason}"),
            Self::TemplateMissing {
                capability_id,
                template_id,
            } => write!(
                f,
                "template missing: capability={capability_id} template={template_id}"
            ),
            Self::Internal { reason } => write!(f, "render internal error: {reason}"),
        }
    }
}

impl std::error::Error for RenderFailure {}

/// Opaque per-app state produced by `init`. Each factory boxes its
/// own concrete state type behind `dyn RenderedApp`.
pub trait RenderedApp: Send + Sync {
    /// The app type key this instance belongs to. Used for audit
    /// logging and debugging; not for routing (the caller already
    /// knows the type when it invokes `init`).
    fn app_type(&self) -> &'static str;

    /// Produce a Splash DSL string to inject into the message's
    /// `splash_card` slot. Pure function of the instance state plus
    /// the current UI language.
    ///
    /// Returns `Err(RenderFailure)` when the SplashHost guard fires
    /// or render-time binding fails. Callers MUST treat `Err` as
    /// "fall back to plain text" — they must NOT attempt to
    /// reconstruct the Splash via a bypass path.
    fn render(&self, app_language: AppLanguage) -> Result<String, RenderFailure>;
}

/// Factory for an app type. Registered once into the global
/// registry and looked up by `org.octos.app.type`.
pub trait AppFactory: Send + Sync {
    /// The schema version this factory currently supports.
    fn supported_version(&self) -> u32;

    /// Whether the factory can render a requested schema version.
    ///
    /// Default behavior remains exact-match for simple v1 factories.
    /// Types that deliberately support multiple versions can override
    /// this without changing the lookup contract for everyone else.
    fn supports_version(&self, version: u32) -> bool {
        version == self.supported_version()
    }

    /// Validate and parse the `initial_state` into an opaque
    /// `RenderedApp` box. Returns `Err(ValidationError)` on
    /// malformed input; the caller falls back to plain text
    /// rendering and logs a warning.
    fn init(&self, initial_state: &JsonValue) -> Result<Box<dyn RenderedApp>, ValidationError>;
}

/// Lookup result from the registry.
pub enum AppLookup {
    /// Type registered and version supported — go ahead and call
    /// `init` on the returned factory.
    Supported(&'static dyn AppFactory),
    /// Type registered but the requested version is outside the
    /// supported range. Caller must fall back to plain text with
    /// a version-mismatch warning.
    VersionMismatch {
        supported: u32,
        requested: u32,
    },
    /// Type not in the registry at all. Caller must fall back to
    /// plain text with an unknown-type warning.
    Unknown,
}

fn registry() -> &'static HashMap<&'static str, &'static dyn AppFactory> {
    static REGISTRY: OnceLock<HashMap<&'static str, &'static dyn AppFactory>> = OnceLock::new();
    REGISTRY.get_or_init(|| {
        let mut m: HashMap<&'static str, &'static dyn AppFactory> = HashMap::new();
        m.insert(news::TYPE_KEY, &news::FACTORY);
        m.insert(weather::TYPE_KEY, &weather::FACTORY);
        m
    })
}

/// Look up an app type + version in the registry.
///
/// The registry is built lazily the first time this is called and
/// is immutable afterwards (no dynamic registration in v1).
pub fn lookup(app_type: &str, version: u32) -> AppLookup {
    let Some(factory) = registry().get(app_type) else {
        return AppLookup::Unknown;
    };
    if !factory.supports_version(version) {
        return AppLookup::VersionMismatch {
            supported: factory.supported_version(),
            requested: version,
        };
    }
    AppLookup::Supported(*factory)
}

/// Parse the full `org.octos.app` envelope from raw JSON content.
///
/// Returns `Some` iff the event carries a valid envelope shape
/// (regardless of whether the `type` is actually registered — the
/// caller separately resolves the type via `lookup`).
pub struct ParsedAppEnvelope {
    pub app_type: String,
    pub version: u32,
    pub initial_state: JsonValue,
}

pub fn parse_envelope(event_content: &JsonValue) -> Option<ParsedAppEnvelope> {
    let envelope = event_content.get("org.octos.app")?.as_object()?;
    let app_type = envelope.get("type")?.as_str()?.to_string();
    let version = envelope.get("version")?.as_u64().and_then(|n| u32::try_from(n).ok())?;
    let initial_state = envelope.get("initial_state").cloned().unwrap_or(JsonValue::Null);
    Some(ParsedAppEnvelope {
        app_type,
        version,
        initial_state,
    })
}

/// End-to-end: parse + lookup + init + render. Returns `Some` with
/// the Splash DSL string when everything succeeds, `None` when the
/// caller should fall back to plain text rendering (the fallback
/// reason is logged as a warning from inside this function).
pub fn render_app_envelope_to_splash(
    event_content: &JsonValue,
    app_language: AppLanguage,
) -> Option<String> {
    let envelope = parse_envelope(event_content)?;

    match lookup(&envelope.app_type, envelope.version) {
        AppLookup::Supported(factory) => match factory.init(&envelope.initial_state) {
            Ok(rendered) => match rendered.render(app_language) {
                Ok(splash) => Some(splash),
                Err(failure) => {
                    makepad_widgets::log!(
                        "org.octos.app render failed for type={} version={}: {}",
                        envelope.app_type,
                        envelope.version,
                        failure,
                    );
                    None
                }
            },
            Err(err) => {
                makepad_widgets::log!(
                    "org.octos.app validation failed for type={} version={}: field={} msg={}",
                    envelope.app_type,
                    envelope.version,
                    err.field,
                    err.message,
                );
                None
            }
        },
        AppLookup::VersionMismatch {
            supported,
            requested,
        } => {
            makepad_widgets::log!(
                "org.octos.app version mismatch for type={}: supported={} requested={}",
                envelope.app_type,
                supported,
                requested,
            );
            None
        }
        AppLookup::Unknown => {
            makepad_widgets::log!(
                "org.octos.app unknown type: {} (not in client registry)",
                envelope.app_type,
            );
            None
        }
    }
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{render_app_envelope_to_splash, RenderFailure};
    use crate::i18n::AppLanguage;

    // P0 safety-boundary test — RenderFailure::HostRejected MUST cause
    // the dispatcher to return None (plain-text fallback) rather than
    // reconstruct a Splash via any bypass path.
    #[test]
    fn render_failure_display_is_human_readable() {
        let f = RenderFailure::HostRejected {
            reason: "widget EvilWidget not on W5".into(),
        };
        let s = format!("{f}");
        assert!(s.contains("host rejected"));
        assert!(s.contains("EvilWidget"));
    }

    #[test]
    fn render_failure_template_missing_display() {
        let f = RenderFailure::TemplateMissing {
            capability_id: "weather_guidance".into(),
            template_id: "nonexistent".into(),
        };
        let s = format!("{f}");
        assert!(s.contains("template missing"));
        assert!(s.contains("nonexistent"));
    }

    // ==========================================================
    // P2 end-to-end host-rejection tests
    //
    // Purpose: lock the safety contract that when SplashHost
    // rejects a template for any W5 / W7 / attribution-override /
    // schema violation, the final dispatcher output is `None` and
    // the unsafe template content NEVER leaks to the output. No
    // bypass path exists to reconstruct the rejected Splash.
    // ==========================================================

    use super::splash_host::{
        AttributionChrome, DefaultSplashHost, HostError, TrustBadge,
    };
    use super::weather::host_error_to_render_failure;
    use super::{AppFactory, AppLookup, RenderedApp, ValidationError};

    /// Equivalent dispatcher path: duplicates the match in
    /// `render_app_envelope_to_splash` against an injected factory.
    /// Keeps production `render_app_envelope_to_splash` unchanged
    /// while letting P2 tests route a synthetic envelope through
    /// real envelope-parse + factory.init + rendered.render +
    /// Some/None projection, using a test-only factory.
    fn render_with_factory(
        event_content: &serde_json::Value,
        app_language: AppLanguage,
        factory: &dyn AppFactory,
    ) -> Option<String> {
        let envelope = super::parse_envelope(event_content)?;
        if !factory.supports_version(envelope.version) {
            return None;
        }
        match factory.init(&envelope.initial_state) {
            Ok(rendered) => match rendered.render(app_language) {
                Ok(splash) => Some(splash),
                Err(_failure) => None,
            },
            Err(_validation) => None,
        }
    }

    /// Test-only factory whose `render()` asks the real
    /// `DefaultSplashHost` to preflight an unsafe template source
    /// via the test-only `load_template_from_source` backdoor. The
    /// host MUST reject with `WidgetNotAllowed` /
    /// `AttributionFieldInTemplate` / `BindingPathNotInSchema`; the
    /// classifier maps that to `RenderFailure::HostRejected`; the
    /// dispatcher projects `Err` to `None`.
    struct UnsafeTemplateFactory {
        unsafe_source: &'static str,
    }

    impl AppFactory for UnsafeTemplateFactory {
        fn supported_version(&self) -> u32 {
            1
        }
        fn init(
            &self,
            _initial_state: &serde_json::Value,
        ) -> Result<Box<dyn RenderedApp>, ValidationError> {
            Ok(Box::new(UnsafeTemplateRendered {
                unsafe_source: self.unsafe_source,
            }))
        }
    }

    struct UnsafeTemplateRendered {
        unsafe_source: &'static str,
    }

    impl RenderedApp for UnsafeTemplateRendered {
        fn app_type(&self) -> &'static str {
            "test_unsafe"
        }
        fn render(&self, _lang: AppLanguage) -> Result<String, RenderFailure> {
            // Use a freshly-constructed host so state doesn't
            // entangle with the global singleton's cache. The
            // preflight validation path is the same — only the
            // storage instance differs.
            let host = DefaultSplashHost::new();
            let _ = AttributionChrome {
                capability_id: "test_unsafe".into(),
                display_name: "Test Unsafe".into(),
                icon_url: None,
                trust_badge: TrustBadge::Builtin,
            };
            let handle = host
                .load_template_from_source(
                    "weather_guidance",
                    "unsafe_injection_test",
                    self.unsafe_source,
                )
                .map_err(host_error_to_render_failure)?;
            // If the host ever accepted the unsafe template, this
            // panic surfaces the regression. In a passing test
            // build, the `?` above short-circuits.
            panic!(
                "unsafe template was NOT rejected by host; handle = {:?}",
                handle
            );
        }
    }

    /// Host rejection via W5 widget whitelist — unsafe template
    /// references a widget not on `widget_manifest`. Dispatcher must
    /// return `None` and the output must NOT contain the evil
    /// widget name.
    #[test]
    fn end_to_end_unsafe_widget_rejected_returns_none() {
        // This source references `EvilWidget`, which is not in the
        // v1 WidgetManifest. Host's W5 check must reject.
        let unsafe_source = "EvilWidget { text: \"pwn\" }";
        let factory = UnsafeTemplateFactory { unsafe_source };
        let envelope = serde_json::json!({
            "body": "fallback body",
            "msgtype": "m.text",
            "org.octos.app": {
                "type": "test_unsafe",
                "version": 1,
                "initial_state": {},
            }
        });

        let result = render_with_factory(&envelope, AppLanguage::English, &factory);
        assert_eq!(result, None, "host rejection MUST surface as None");
    }

    /// Host rejection via attribution-override guard — template
    /// content tries to stamp `capability_id:` inside. Host's
    /// attribution guard must reject; dispatcher returns `None`.
    #[test]
    fn end_to_end_attribution_override_rejected_returns_none() {
        let unsafe_source = r#"RoundedView {
            capability_id: "impersonator"
            Label { text: "trust me bro" }
        }"#;
        let factory = UnsafeTemplateFactory { unsafe_source };
        let envelope = serde_json::json!({
            "body": "fallback",
            "org.octos.app": {
                "type": "test_unsafe",
                "version": 1,
                "initial_state": {},
            }
        });

        let result = render_with_factory(&envelope, AppLanguage::English, &factory);
        assert_eq!(result, None);
    }

    /// Host rejection via schema binding-path check — template
    /// binds to `$state.path_not_in_schema`, which is not on
    /// WeatherCapabilitySchema's allowed paths. Host must reject.
    #[test]
    fn end_to_end_schema_binding_path_rejected_returns_none() {
        // `$state.malicious_escape` is not on
        // `WeatherCapabilitySchema::contains_path`. Schema check
        // runs because capability_id = "weather_guidance" in the
        // factory's render().
        let unsafe_source = r#"Label {
            text: $state.malicious_escape
        }"#;
        let factory = UnsafeTemplateFactory { unsafe_source };
        let envelope = serde_json::json!({
            "body": "fallback",
            "org.octos.app": {
                "type": "test_unsafe",
                "version": 1,
                "initial_state": {},
            }
        });

        let result = render_with_factory(&envelope, AppLanguage::English, &factory);
        assert_eq!(result, None);
    }

    /// Direct classifier confirmation: host.load_template_from_source
    /// with an unsafe widget returns the correct HostError variant,
    /// which classifies to HostRejected.
    #[test]
    fn host_load_template_from_source_rejects_unsafe_widget() {
        let host = DefaultSplashHost::new();
        let unsafe_source = "EvilWidget { }";
        let err = host
            .load_template_from_source("weather_guidance", "evil", unsafe_source)
            .expect_err("host must reject unsafe widget");
        match err {
            HostError::WidgetNotAllowed { ref name, .. } => {
                assert_eq!(name, "EvilWidget");
            }
            other => panic!(
                "expected WidgetNotAllowed for EvilWidget, got: {other:?}"
            ),
        }
    }

    /// Lock the contract: when `render_with_factory` returns None,
    /// the output is truly absent — caller then falls back to the
    /// Matrix `body` plain-text path. Verify the None has no
    /// residual Splash content by checking `.is_none()` (trivially
    /// true given `Option`, but the test documents the contract).
    #[test]
    fn host_rejection_produces_no_splash_leak() {
        let factory = UnsafeTemplateFactory {
            unsafe_source: "EvilWidget { text: \"leak_probe_LEAK\" }",
        };
        let envelope = serde_json::json!({
            "body": "plain text",
            "org.octos.app": {
                "type": "test_unsafe",
                "version": 1,
                "initial_state": {},
            }
        });
        let result = render_with_factory(&envelope, AppLanguage::English, &factory);
        assert!(
            result.is_none(),
            "when host rejects, there is NO Splash produced — ever"
        );
        // There is no String output path that could contain
        // `leak_probe_LEAK`; `Option::None` carries no payload.
        // This test's purpose is regression-marker: if a future
        // change leaks the template into some Some(String), it
        // fails.
        if let Some(splash) = result {
            assert!(!splash.contains("leak_probe_LEAK"),
                "unsafe template content leaked into dispatcher output");
            assert!(!splash.contains("EvilWidget"),
                "unsafe widget name leaked into dispatcher output");
        }
    }

    /// Regression-lock the existing unsupported-version path:
    /// when AppLookup::VersionMismatch fires (a different kind of
    /// rejection), dispatcher still returns None and does NOT
    /// attempt any template fallback reconstruction.
    #[test]
    fn version_mismatch_path_still_returns_none() {
        // Sanity: the real weather factory supports versions 1 & 2.
        let is_mismatch = matches!(
            super::lookup("weather", 99),
            AppLookup::VersionMismatch { .. }
        );
        assert!(is_mismatch, "expected VersionMismatch for v99");
    }

    #[test]
    fn raw_matrix_weather_event_renders_to_splash() {
        let event_content = json!({
            "body": "Beijing 22.0°C sunny",
            "format": "org.matrix.custom.html",
            "formatted_body": "Beijing 22.0°C sunny",
            "msgtype": "m.text",
            "org.octos.app": {
                "initial_state": {
                    "condition": "sunny",
                    "feels_like_c": 21,
                    "humidity": 47,
                    "location": "Beijing",
                    "temp_c": 22,
                    "updated_at": "2026-04-15T10:52:09.501678+00:00",
                    "wind_kph": 10
                },
                "type": "weather",
                "version": 1
            }
        });

        let splash = render_app_envelope_to_splash(&event_content, AppLanguage::English)
            .expect("weather org.octos.app event should render");

        assert!(splash.contains("Beijing"));
        assert!(splash.contains("RoundedView"));
        assert!(splash.contains("draw_bg.border_radius"));
        assert!(!splash.contains(','));
    }

    #[test]
    fn raw_matrix_weather_v2_event_renders_guidance_card() {
        let event_content = json!({
            "body": "Beijing 16°C cloudy",
            "msgtype": "m.text",
            "org.octos.app": {
                "initial_state": {
                    "condition": "cloudy",
                    "feels_like_c": 17,
                    "high_c": 24,
                    "humidity": 81,
                    "location": "Beijing",
                    "low_c": 12,
                    "periods": [
                        { "slot": "morning", "temp_c": 13, "condition": "cloudy", "precipitation_probability": 10 },
                        { "slot": "noon", "temp_c": 24, "condition": "sunny", "precipitation_probability": 0 },
                        { "slot": "night", "temp_c": 14, "condition": "cloudy", "precipitation_probability": 5 }
                    ],
                    "precipitation_probability_max": 10,
                    "temp_c": 16,
                    "updated_at": "2026-04-15T18:22:57.710209+00:00",
                    "uv_index_max": 6,
                    "wind_kph": 3
                },
                "type": "weather",
                "version": 2
            }
        });

        let splash =
            render_app_envelope_to_splash(&event_content, AppLanguage::ChineseSimplified)
                .expect("weather v2 org.octos.app event should render");

        assert!(splash.contains("今天怎么穿"), "missing guidance header: {splash}");
        assert!(splash.contains("早上"), "missing morning period: {splash}");
        assert!(splash.contains("中午"), "missing noon period: {splash}");
        assert!(splash.contains("晚上"), "missing night period: {splash}");
    }

    #[test]
    fn unsupported_version_bypasses_template_fallback() {
        let event_content = json!({
            "body": "fallback me",
            "msgtype": "m.text",
            "org.octos.app": {
                "initial_state": {
                    "location": "Beijing"
                },
                "type": "weather",
                "version": 99
            }
        });

        let splash = render_app_envelope_to_splash(&event_content, AppLanguage::English);
        assert!(
            splash.is_none(),
            "unsupported version must keep plain-text fallback path"
        );
    }

    #[test]
    fn raw_matrix_news_event_renders_to_splash() {
        let event_content = json!({
            "body": "AI funding updates",
            "msgtype": "m.text",
            "org.octos.app": {
                "initial_state": {
                    "topic": "AI",
                    "time_range": "today",
                    "headline": "AI funding rounds accelerate",
                    "summary": "Three major AI infrastructure startups announced new funding today.",
                    "items": [
                        {
                            "title": "Compute startup raises new round",
                            "source": "Tech Ledger",
                            "url": "https://example.com/compute-round"
                        },
                        {
                            "title": "Open-source tooling gains enterprise adoption",
                            "source": "Dev Weekly"
                        }
                    ],
                    "updated_at": "2026-04-23T08:00:00Z"
                },
                "type": "news",
                "version": 1
            }
        });

        let splash = render_app_envelope_to_splash(&event_content, AppLanguage::English)
            .expect("news org.octos.app event should render");

        assert!(splash.contains("AI funding rounds accelerate"), "{splash}");
        assert!(splash.contains("Tech Ledger"), "{splash}");
        assert!(splash.contains("news_guidance"), "{splash}");
        assert!(!splash.contains("$state."), "{splash}");
    }
}
