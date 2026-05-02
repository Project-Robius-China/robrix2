//! Capability descriptor table — the **v1 chrome source** per spec
//! `§Attribution Chrome → 数据来源（v1 + future path）`.
//!
//! V1 design decision: capabilities that are **compile-time built-ins**
//! (weather, news, …) derive their `AttributionChrome` from this static
//! Robrix-side table, keyed by `app_type`. Dispatcher does NOT write
//! chrome into envelope metadata in v1 — that path is reserved for the
//! future third-party / orchestrator-gated capability spec.
//!
//! This module also exports the version constants needed by the future
//! `task-agent-to-app-template-runtime.spec.md` compatibility matrix:
//! `manifest_version`, `template_set_version` (per-capability), and
//! `HOST_VERSION` (per-Robrix-release). All start at 1 in v1; the
//! template-runtime spec defines upgrade rules.
//!
//! Contract: see `specs/task-agent-to-app-splash-host-evolution.spec.md`
//! `§Attribution Chrome` and the plan's Step 1.3a + Forward References.

use std::collections::HashMap;
use std::sync::LazyLock;

use super::splash_host::{AttributionChrome, TrustBadge};

/// Per-release host identity. Bumped when the host runtime contract
/// changes in a way templates / capabilities need to observe. V1 = 1.
/// The template-runtime spec wires this into the cache key.
pub const HOST_VERSION: u32 = 1;

/// One row of the capability descriptor table. Carries the 4 chrome
/// fields plus the 2 version fields needed by the template-runtime
/// spec. All fields are `'static`; the table is populated at startup
/// and never mutated.
#[derive(Debug, Clone)]
pub struct CapabilityDescriptor {
    /// Capability identifier from the OctOS side — matches
    /// `Capability::id()`. Stamped into `AttributionChrome` so the
    /// host chrome band shows who produced the reply.
    pub capability_id: &'static str,
    /// Human-facing name rendered in the host chrome band.
    pub display_name: &'static str,
    /// Optional icon resource path. `None` = fall back to generic
    /// capability icon.
    pub icon_url: Option<&'static str>,
    /// v1 always `Builtin`. `Verified` / `Unverified` reserved for the
    /// future envelope-chrome path.
    pub trust_badge: TrustBadge,
    /// Widget-manifest schema version this capability's templates were
    /// authored against. Bumped when the manifest adds / removes /
    /// retypes a widget used by this capability. v1 = 1.
    pub manifest_version: u32,
    /// Template-set version for this capability — bumped when any of
    /// its templates ship a breaking change (widget removed, required
    /// state field added, etc.). v1 = 1.
    pub template_set_version: u32,
    /// Optional backup template to try if the preferred template fails
    /// preflight or render binding. v1 weather has only one template,
    /// so this stays `None`.
    pub fallback_template_id: Option<&'static str>,
}

/// Static v1 capability descriptor table. Keyed by `app_type` (the
/// envelope's `type` field). Slice 2 Step 2.4 adds the `news` entry.
static CAPABILITY_DESCRIPTORS: LazyLock<HashMap<&'static str, CapabilityDescriptor>> =
    LazyLock::new(|| {
        let mut m: HashMap<&'static str, CapabilityDescriptor> = HashMap::new();

        m.insert(
            "weather",
            CapabilityDescriptor {
                capability_id: "weather_guidance",
                display_name: "Weather",
                // v1: no dedicated icon asset; host chrome falls back to
                // a generic capability icon. Swap for a real asset when
                // we decide on the chrome band visuals.
                icon_url: None,
                trust_badge: TrustBadge::Builtin,
                manifest_version: 1,
                template_set_version: 1,
                fallback_template_id: None,
            },
        );

        m.insert(
            "mission_room",
            CapabilityDescriptor {
                capability_id: "mission_room",
                display_name: "Mission Room",
                icon_url: None,
                trust_badge: TrustBadge::Builtin,
                manifest_version: 1,
                template_set_version: 1,
                fallback_template_id: None,
            },
        );

        m.insert(
            "news",
            CapabilityDescriptor {
                capability_id: "news_guidance",
                display_name: "News",
                icon_url: None,
                trust_badge: TrustBadge::Builtin,
                manifest_version: 1,
                template_set_version: 1,
                fallback_template_id: None,
            },
        );

        m
    });

/// Look up a capability descriptor by its envelope `app_type`. Returns
/// `None` for unknown types; callers decide whether to fall back
/// (per-app render() impls treat missing descriptor as a hard error
/// because they always know their own app_type).
pub fn lookup(app_type: &str) -> Option<&'static CapabilityDescriptor> {
    CAPABILITY_DESCRIPTORS.get(app_type)
}

/// Look up a capability descriptor and convert it to `AttributionChrome`
/// owned-string form for passing into `SplashHost::render_to_splash`.
/// This is the one-liner each `RenderedApp::render` impl will call.
pub fn chrome_for(app_type: &str) -> Option<AttributionChrome> {
    lookup(app_type).map(|d| AttributionChrome {
        capability_id: d.capability_id.to_string(),
        display_name: d.display_name.to_string(),
        icon_url: d.icon_url.map(|s| s.to_string()),
        trust_badge: d.trust_badge,
    })
}

/// Iterate all registered descriptors. Used by build-time audit tests
/// to cross-check that every registered consumer in `app_registry/mod.rs`
/// has a matching descriptor.
pub fn iter() -> impl Iterator<Item = (&'static &'static str, &'static CapabilityDescriptor)> {
    CAPABILITY_DESCRIPTORS.iter()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_descriptor_has_all_six_fields() {
        let d = lookup("weather").expect("weather descriptor exists");
        assert_eq!(d.capability_id, "weather_guidance");
        assert_eq!(d.display_name, "Weather");
        assert_eq!(d.icon_url, None);
        assert_eq!(d.trust_badge, TrustBadge::Builtin);
        assert_eq!(d.manifest_version, 1);
        assert_eq!(d.template_set_version, 1);
        assert_eq!(d.fallback_template_id, None);
    }

    #[test]
    fn news_descriptor_has_all_six_fields() {
        let d = lookup("news").expect("news descriptor exists");
        assert_eq!(d.capability_id, "news_guidance");
        assert_eq!(d.display_name, "News");
        assert_eq!(d.icon_url, None);
        assert_eq!(d.trust_badge, TrustBadge::Builtin);
        assert_eq!(d.manifest_version, 1);
        assert_eq!(d.template_set_version, 1);
        assert_eq!(d.fallback_template_id, None);
    }

    #[test]
    fn mission_room_descriptor_has_all_six_fields() {
        let d = lookup("mission_room").expect("mission_room descriptor exists");
        assert_eq!(d.capability_id, "mission_room");
        assert_eq!(d.display_name, "Mission Room");
        assert_eq!(d.icon_url, None);
        assert_eq!(d.trust_badge, TrustBadge::Builtin);
        assert_eq!(d.manifest_version, 1);
        assert_eq!(d.template_set_version, 1);
        assert_eq!(d.fallback_template_id, None);
    }

    #[test]
    fn host_version_starts_at_one() {
        // V1 locks HOST_VERSION = 1. Changing this requires the
        // template-runtime spec to define upgrade rules; not a silent
        // bump.
        assert_eq!(HOST_VERSION, 1);
    }

    #[test]
    fn lookup_unknown_app_type_returns_none() {
        assert!(lookup("fitness").is_none());
        assert!(lookup("").is_none());
    }

    #[test]
    fn chrome_for_weather_produces_owned_strings() {
        let chrome = chrome_for("weather").expect("weather chrome");
        assert_eq!(chrome.capability_id, "weather_guidance");
        assert_eq!(chrome.display_name, "Weather");
        assert!(chrome.icon_url.is_none());
        assert_eq!(chrome.trust_badge, TrustBadge::Builtin);
    }

    #[test]
    fn chrome_for_news_produces_owned_strings() {
        let chrome = chrome_for("news").expect("news chrome");
        assert_eq!(chrome.capability_id, "news_guidance");
        assert_eq!(chrome.display_name, "News");
        assert!(chrome.icon_url.is_none());
        assert_eq!(chrome.trust_badge, TrustBadge::Builtin);
    }

    #[test]
    fn chrome_for_unknown_returns_none() {
        assert!(chrome_for("fitness").is_none());
    }

    #[test]
    fn version_fields_preemptively_reserved_for_runtime_spec() {
        // The future `task-agent-to-app-template-runtime.spec.md`
        // will compose a cache key from (app_version, template_id,
        // template_hash, manifest_version, host_version). Two of those
        // ride on CapabilityDescriptor. This test locks that both
        // fields exist and start at 1, so the runtime spec can land
        // without retrofitting every capability.
        let d = lookup("weather").unwrap();
        assert!(d.manifest_version >= 1);
        assert!(d.template_set_version >= 1);
        assert!(d.fallback_template_id.is_none());
    }
}
