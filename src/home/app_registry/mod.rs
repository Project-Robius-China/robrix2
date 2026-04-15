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
    fn render(&self, app_language: AppLanguage) -> String;
}

/// Factory for an app type. Registered once into the global
/// registry and looked up by `org.octos.app.type`.
pub trait AppFactory: Send + Sync {
    /// The schema version this factory currently supports.
    fn supported_version(&self) -> u32;

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
    if version != factory.supported_version() {
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
            Ok(rendered) => Some(rendered.render(app_language)),
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
