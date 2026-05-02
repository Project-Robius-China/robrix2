//! Compile-time aggregation point for `.splash` template assets.
//!
//! Templates are Robrix-side static resources (spec
//! `task-agent-to-app-splash-host-evolution.spec.md` §Template File 格式
//! 与位置, v1 addendum 2026-04-23). They are `include_str!`'d into the
//! binary so the SplashHost preflight (template-runtime spec Slice A)
//! can iterate them at `cargo test --lib` time and the `mod.rs`
//! dispatcher can hand the bytes to `SplashHost::load_template` at
//! runtime without touching the filesystem.
//!
//! Adding a new template:
//!   1. Drop `.splash` file under `templates/<capability_id>/`.
//!   2. Add a `pub const ... = include_str!(...)` entry here.
//!   3. Add a row to `ALL_TEMPLATES`.
//!   4. Register the capability + template_id pair in
//!      `capability_descriptors.rs` (for chrome) and
//!      `Capability::template_ids()` (OctOS side, once trait exists).

pub const WEATHER_CARD_STANDARD: &str =
    include_str!("templates/weather_guidance/card_standard.splash");

pub const MISSION_ROOM_CONTROL: &str =
    include_str!("templates/mission_room/mission_control.splash");

pub const NEWS_HEADLINES_CARD: &str =
    include_str!("templates/news_guidance/headlines_card.splash");

pub const NEWS_DIGEST_CARD: &str =
    include_str!("templates/news_guidance/digest_card.splash");

/// Flat table of every shipped template, keyed by `(capability_id,
/// template_id)`. This is the **single source of truth** — every
/// template lookup in the codebase (splash_host's production path,
/// preflight audit test, and test-only bypass helpers) MUST resolve
/// through this table rather than carrying its own `include_str!`.
/// Used by the template-runtime preflight audit test (plan Slice A
/// Step A.6) to iterate and validate every template at build time.
pub const ALL_TEMPLATES: &[(&str, &str, &str)] = &[
    ("mission_room", "mission_control", MISSION_ROOM_CONTROL),
    ("weather_guidance", "card_standard", WEATHER_CARD_STANDARD),
    ("news_guidance", "headlines_card", NEWS_HEADLINES_CARD),
    ("news_guidance", "digest_card", NEWS_DIGEST_CARD),
];

/// Resolve the static template source for `(capability_id, template_id)`.
/// Returns `None` when the pair is not registered in `ALL_TEMPLATES`.
///
/// This is the consolidated accessor per P1a — splash_host's production
/// `load_template_source`, the preflight audit test, and any test-only
/// bypass helpers all go through this single function. Keeping only one
/// `include_str!` site per template eliminates the "four-way drift"
/// failure mode (template edited in one place, silently stale in
/// others).
pub fn source_for(capability_id: &str, template_id: &str) -> Option<&'static str> {
    ALL_TEMPLATES
        .iter()
        .find(|(cap, tid, _)| *cap == capability_id && *tid == template_id)
        .map(|(_, _, src)| *src)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn weather_card_standard_is_non_empty() {
        assert!(!WEATHER_CARD_STANDARD.is_empty());
    }

    #[test]
    fn weather_card_standard_has_roundedview_root() {
        // Sanity marker: the template's top-level widget is a
        // RoundedView (the card container). This locks the file shape
        // against accidental deletion / truncation.
        let trimmed = WEATHER_CARD_STANDARD.trim_start();
        assert!(
            trimmed.starts_with("RoundedView {") || trimmed.starts_with("RoundedView{"),
            "weather card template must start with RoundedView root"
        );
    }

    #[test]
    fn weather_card_standard_binds_required_state_paths() {
        // The happy-path template depends on these `$state.*` paths.
        // If someone edits the template and drops one of these
        // bindings without updating the capability's build_state, the
        // failure would surface late at render time; this test surfaces
        // it at `cargo test --lib`.
        for marker in [
            "$state.location",
            "$state.hero.temp_text",
            "$state.hero.symbol",
            "$state.hero.bg_color",
            "$state.headline",
            "$state.summary",
            "$state.guidance_header",
            "$state.condition_summary",
        ] {
            assert!(
                WEATHER_CARD_STANDARD.contains(marker),
                "template missing required state binding: {marker}"
            );
        }
    }

    #[test]
    fn weather_card_standard_declares_visibility_gates_for_optionals() {
        // Optional fields (high/low range, updated_at, feels_like,
        // humidity, wind) are gated via `visible: $state.X.visible`
        // boolean bindings rather than unconditional rendering.
        // This keeps missing fields from producing empty widgets at
        // render time and preserves the fallback invariant.
        for marker in [
            "visible: $state.range.visible",
            "visible: $state.updated.visible",
            "visible: $state.feels_like.visible",
            "visible: $state.humidity.visible",
            "visible: $state.wind.visible",
            "visible: $state.periods_section.visible",
            "visible: $state.chips_section.visible",
        ] {
            assert!(
                WEATHER_CARD_STANDARD.contains(marker),
                "template missing visibility gate: {marker}"
            );
        }
    }

    #[test]
    fn weather_card_standard_does_not_reference_host_chrome_fields() {
        // Attribution chrome (capability_id / display_name / icon /
        // trust_badge) is host-owned per spec §6.1. Templates must
        // NOT redeclare these fields in content. This test is a
        // coarse grep; the structured Slice A preflight validator
        // replaces it with an AST check.
        for forbidden in [
            "capability_id:",
            "display_name:",
            "trust_badge:",
        ] {
            assert!(
                !WEATHER_CARD_STANDARD.contains(forbidden),
                "template must not redeclare host-chrome field: {forbidden}"
            );
        }
    }

    #[test]
    fn all_templates_table_contains_all_shipped_pairs() {
        let required = [
            ("weather_guidance", "card_standard"),
            ("mission_room", "mission_control"),
            ("news_guidance", "headlines_card"),
            ("news_guidance", "digest_card"),
        ];
        for (cap, tid) in required {
            let found = ALL_TEMPLATES
                .iter()
                .any(|(c, t, _)| *c == cap && *t == tid);
            assert!(found, "ALL_TEMPLATES must index {cap}/{tid}");
        }
    }

    #[test]
    fn all_templates_sources_match_individual_consts() {
        // Regression lock: editing the individual const MUST keep the
        // ALL_TEMPLATES row in sync. Compare by content (`==`) rather
        // than pointer identity — Rust `const` accesses don't always
        // dedupe static addresses.
        for (cap, tid, src) in ALL_TEMPLATES {
            match (*cap, *tid) {
                ("weather_guidance", "card_standard") => {
                    assert_eq!(*src, WEATHER_CARD_STANDARD);
                }
                ("mission_room", "mission_control") => {
                    assert_eq!(*src, MISSION_ROOM_CONTROL);
                }
                ("news_guidance", "headlines_card") => {
                    assert_eq!(*src, NEWS_HEADLINES_CARD);
                }
                ("news_guidance", "digest_card") => {
                    assert_eq!(*src, NEWS_DIGEST_CARD);
                }
                other => panic!("unknown template entry in ALL_TEMPLATES: {other:?}"),
            }
        }
    }

    #[test]
    fn source_for_known_pairs_returns_some() {
        assert!(source_for("weather_guidance", "card_standard").is_some());
        assert!(source_for("mission_room", "mission_control").is_some());
        assert!(source_for("news_guidance", "headlines_card").is_some());
        assert!(source_for("news_guidance", "digest_card").is_some());
    }

    #[test]
    fn source_for_unknown_pair_returns_none() {
        assert!(source_for("weather_guidance", "nonexistent").is_none());
        assert!(source_for("unknown_capability", "card_standard").is_none());
        assert!(source_for("", "").is_none());
    }

    #[test]
    fn source_for_matches_individual_const_contents() {
        // Use value equality: Rust `const` items of identical content
        // may have distinct static addresses at different access
        // sites, so `ptr::eq` is unreliable here. Content identity is
        // what the P1a single-source invariant actually cares about.
        assert_eq!(
            source_for("weather_guidance", "card_standard").unwrap(),
            WEATHER_CARD_STANDARD,
        );
        assert_eq!(
            source_for("mission_room", "mission_control").unwrap(),
            MISSION_ROOM_CONTROL,
        );
        assert_eq!(
            source_for("news_guidance", "headlines_card").unwrap(),
            NEWS_HEADLINES_CARD,
        );
        assert_eq!(
            source_for("news_guidance", "digest_card").unwrap(),
            NEWS_DIGEST_CARD,
        );
    }
}
