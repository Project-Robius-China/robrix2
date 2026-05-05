//! Widget manifest + W5 trust whitelist.
//!
//! Every widget name a template may reference must appear here with
//! `trust_level = Public`; anything else gets rejected by
//! `SplashHost::load_template` (plan Step 1.6).
//!
//! v1 manifest is hand-written; future dynamic widget packages will
//! extend this table through `register_widget(...)`-style APIs but that
//! path is out of scope for this spec (see design doc §10 Non-goals).
//!
//! Contract: see `specs/task-agent-to-app-splash-host-evolution.spec.md`
//! `§Widget Manifest 与 W5 trust whitelist` and design doc §6 row W5.

use std::collections::HashMap;
use std::sync::LazyLock;

/// Trust classification gate for templates. Only `Public` widgets are
/// reachable from a template file. `Internal` widgets exist for host
/// chrome / framework plumbing; `Sensitive` widgets (auth / payment /
/// secure input) are never reachable from any template regardless of
/// capability.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TrustLevel {
    Public,
    Internal,
    Sensitive,
}

impl TrustLevel {
    /// Whether templates may reference widgets at this trust level.
    /// Only `Public` returns `true` in v1.
    pub const fn is_template_reachable(self) -> bool {
        matches!(self, TrustLevel::Public)
    }
}

/// Typed prop schema entry. v1 keeps this enum small — enough to
/// validate the set of prop shapes weather + news templates actually
/// use. Expanded when a new widget's prop type is genuinely needed.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PropKind {
    /// Free-form string (usually state-bound via `$state.path`).
    Text,
    /// Number literal or number-binding.
    Number,
    /// Boolean literal or boolean-binding.
    Bool,
    /// Nested child widget reference (by name).
    Child,
    /// List of nested child widgets.
    ChildList,
    /// Color value (hex string, token reference, etc.).
    Color,
}

/// Single prop on a widget: its template-visible name, the accepted
/// value kind, and whether it is required.
#[derive(Debug, Clone, Copy)]
pub struct PropSpec {
    pub name: &'static str,
    pub kind: PropKind,
    pub required: bool,
}

/// One row of the widget manifest.
#[derive(Debug, Clone)]
pub struct WidgetDescriptor {
    /// Widget type name as it appears in `.splash` files
    /// (e.g. `"RoundedView"`, `"WeatherCard"`).
    pub name: &'static str,
    /// Trust gate; templates may only reference widgets where this is
    /// `TrustLevel::Public`.
    pub trust_level: TrustLevel,
    /// The prop schema templates must conform to when instantiating
    /// this widget. v1 validation is name + kind + required; finer
    /// semantic validation lands with preflight in the
    /// template-runtime spec.
    pub prop_schema: &'static [PropSpec],
    /// Logical module path (e.g. `"makepad_widgets::view::View"`).
    /// Used by future `import` generation; v1 is metadata only.
    pub module: &'static str,
}

/// Static v1 manifest. Populated with Makepad builtins today; Slice 1
/// Step 1.4 adds weather-specific entries (`TemperatureBar`,
/// `FocusTile`, `UvChip`), Slice 2 Step 2.1 adds news-specific entries
/// (`NewsTile`, `SourceChip`).
static WIDGET_MANIFEST: LazyLock<HashMap<&'static str, WidgetDescriptor>> =
    LazyLock::new(|| {
        let mut m: HashMap<&'static str, WidgetDescriptor> = HashMap::new();

        // Makepad builtins — all Public. Prop schemas are intentionally
        // minimal in v1; templates that use props not listed here will
        // fail preflight. Expand when needed, not speculatively.
        m.insert(
            "View",
            WidgetDescriptor {
                name: "View",
                trust_level: TrustLevel::Public,
                prop_schema: &[
                    PropSpec { name: "visible", kind: PropKind::Bool, required: false },
                    PropSpec { name: "width", kind: PropKind::Text, required: false },
                    PropSpec { name: "height", kind: PropKind::Text, required: false },
                    PropSpec { name: "flow", kind: PropKind::Text, required: false },
                    PropSpec { name: "padding", kind: PropKind::Text, required: false },
                    PropSpec { name: "spacing", kind: PropKind::Number, required: false },
                ],
                module: "makepad_widgets::view::View",
            },
        );
        m.insert(
            "RoundedView",
            WidgetDescriptor {
                name: "RoundedView",
                trust_level: TrustLevel::Public,
                prop_schema: &[
                    PropSpec { name: "visible", kind: PropKind::Bool, required: false },
                    PropSpec { name: "width", kind: PropKind::Text, required: false },
                    PropSpec { name: "height", kind: PropKind::Text, required: false },
                    PropSpec { name: "flow", kind: PropKind::Text, required: false },
                    PropSpec { name: "padding", kind: PropKind::Text, required: false },
                    PropSpec { name: "spacing", kind: PropKind::Number, required: false },
                    PropSpec { name: "draw_bg", kind: PropKind::Text, required: false },
                ],
                module: "makepad_widgets::rounded_view::RoundedView",
            },
        );
        m.insert(
            "Label",
            WidgetDescriptor {
                name: "Label",
                trust_level: TrustLevel::Public,
                prop_schema: &[
                    PropSpec { name: "visible", kind: PropKind::Bool, required: false },
                    PropSpec { name: "text", kind: PropKind::Text, required: false },
                    PropSpec { name: "draw_text", kind: PropKind::Text, required: false },
                ],
                module: "makepad_widgets::label::Label",
            },
        );
        m.insert(
            "Icon",
            WidgetDescriptor {
                name: "Icon",
                trust_level: TrustLevel::Public,
                prop_schema: &[
                    PropSpec { name: "icon", kind: PropKind::Text, required: true },
                    PropSpec { name: "width", kind: PropKind::Number, required: false },
                    PropSpec { name: "height", kind: PropKind::Number, required: false },
                ],
                module: "makepad_widgets::icon::Icon",
            },
        );
        m.insert(
            "Image",
            WidgetDescriptor {
                name: "Image",
                trust_level: TrustLevel::Public,
                prop_schema: &[
                    PropSpec { name: "source", kind: PropKind::Text, required: true },
                    PropSpec { name: "width", kind: PropKind::Number, required: false },
                    PropSpec { name: "height", kind: PropKind::Number, required: false },
                ],
                module: "makepad_widgets::image::Image",
            },
        );
        m.insert(
            "Button",
            WidgetDescriptor {
                name: "Button",
                trust_level: TrustLevel::Public,
                prop_schema: &[
                    PropSpec { name: "text", kind: PropKind::Text, required: false },
                    PropSpec { name: "draw_bg", kind: PropKind::Text, required: false },
                ],
                module: "makepad_widgets::button::Button",
            },
        );

        m
    });

/// Look up a widget by its template-visible name.
pub fn lookup(name: &str) -> Option<&'static WidgetDescriptor> {
    WIDGET_MANIFEST.get(name)
}

/// Whether a given widget name is reachable from a template (i.e.,
/// registered and marked `Public`). This is the single gate the
/// template preflight (Step 1.6) will call per widget reference.
pub fn is_template_reachable(name: &str) -> bool {
    lookup(name)
        .map(|d| d.trust_level.is_template_reachable())
        .unwrap_or(false)
}

/// Iterate all registered descriptors. Used by the build-time linter
/// to cross-check templates against the manifest; not for runtime
/// dispatch.
pub fn iter() -> impl Iterator<Item = &'static WidgetDescriptor> {
    WIDGET_MANIFEST.values()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn v1_manifest_contains_makepad_builtins() {
        for name in ["View", "RoundedView", "Label", "Icon", "Image", "Button"] {
            let d = lookup(name).unwrap_or_else(|| panic!("missing builtin widget: {name}"));
            assert_eq!(d.trust_level, TrustLevel::Public);
            assert_eq!(d.name, name);
        }
    }

    #[test]
    fn trust_level_only_public_is_template_reachable() {
        assert!(TrustLevel::Public.is_template_reachable());
        assert!(!TrustLevel::Internal.is_template_reachable());
        assert!(!TrustLevel::Sensitive.is_template_reachable());
    }

    #[test]
    fn is_template_reachable_rejects_unknown_widget() {
        assert!(!is_template_reachable("EvilWidget"));
    }

    #[test]
    fn is_template_reachable_accepts_public_builtin() {
        assert!(is_template_reachable("Label"));
        assert!(is_template_reachable("RoundedView"));
    }

    #[test]
    fn required_props_are_exposed_to_preflight() {
        // Icon + Image have required props; preflight (Step 1.6) will
        // read `required: true` entries to validate templates.
        let icon = lookup("Icon").unwrap();
        assert!(icon.prop_schema.iter().any(|p| p.name == "icon" && p.required));

        let image = lookup("Image").unwrap();
        assert!(image.prop_schema.iter().any(|p| p.name == "source" && p.required));
    }

    #[test]
    fn iter_returns_all_registered_descriptors() {
        let all: Vec<_> = iter().collect();
        // At least the 6 v1 builtins; later steps add more.
        assert!(all.len() >= 6, "expected >= 6 widgets, got {}", all.len());
    }

    #[test]
    fn widget_descriptor_module_path_present() {
        // Used by future import-generation; test locks the field is
        // populated for every v1 entry.
        for d in iter() {
            assert!(!d.module.is_empty(), "widget {} missing module path", d.name);
        }
    }
}
