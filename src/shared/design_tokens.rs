//! Robrix2 design tokens — the semantic color / radius / type layer.
//!
//! This is the single source of truth for the "AI workspace" visual language
//! described in `docs/ui-visual-spec-zh.md`. It is **light-first**: the primary
//! content surfaces are light, with a small set of explicit *dark surface* tokens
//! for the desktop navigation rail and the mobile login screen. (A full dark theme
//! is intentionally out of scope for this round — see the spec, §11.)
//!
//! ## Naming
//! All tokens use the `RBX_` prefix and a `GROUP_ROLE[_STATE]` shape so they never
//! collide with the legacy `COLOR_*` / `SPACE_*` tokens in `styles.rs`:
//!
//! - `RBX_BG_*`      surfaces / backgrounds
//! - `RBX_FG_*`      text / foreground
//! - `RBX_ACCENT*`   the teal brand-accent (primary CTA, selection, links)
//! - `RBX_STROKE_*`  borders / dividers
//! - `RBX_<STATE>_FG` / `RBX_<STATE>_BG`  semantic state pairs (success/warning/danger/info/neutral)
//! - `RBX_NAV_*`     dark navigation-rail surfaces
//! - `RBX_LOGIN_*`   dark login surfaces
//! - `RBX_RADIUS_*`  corner radii
//! - `RBX_TEXT_*`    type-scale `TextStyle` presets
//!
//! Tokens are registered into the global `mod.widgets.*` namespace (so any other
//! `script_mod!` block can read them via `(RBX_TOKEN)` after `use mod.widgets.*`).
//! A curated subset is also exported as Rust `Vec4` consts for use in
//! `script_apply_eval!` / programmatic styling, mirroring the convention in
//! `styles.rs`.
//!
//! NOTE: inside the `script_mod!` block, only `//` comments are allowed — `///`
//! doc comments are parsed by Rust as `#[doc]` attributes and will not compile.

use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    // =========================================================================
    // 1. SURFACES — light-first backgrounds
    // =========================================================================
    // Page background. The calm cool-white the whole app sits on.
    mod.widgets.RBX_BG_CANVAS         = #xF7F9FC
    // Card / sheet / elevated surface.
    mod.widgets.RBX_BG_SURFACE        = #xFFFFFF
    // Subtle inset surface (grouped rows, secondary panels, table zebra).
    mod.widgets.RBX_BG_SURFACE_SUBTLE = #xF4F7FB
    // Sunken surface for light code/preview insets.
    mod.widgets.RBX_BG_SUNKEN         = #xEEF2F8
    // Hover wash over a surface (rows, list items).
    mod.widgets.RBX_BG_HOVER          = #xEFF4FB

    // =========================================================================
    // 2. FOREGROUND — text & icons on light surfaces
    // =========================================================================
    // Primary text (titles, body). Deep blue-grey, never pure black.
    mod.widgets.RBX_FG_PRIMARY    = #x16233B
    // Secondary text (subtitles, meta, helper).
    mod.widgets.RBX_FG_SECONDARY  = #x5A6B86
    // Tertiary text (timestamps, faint captions).
    mod.widgets.RBX_FG_TERTIARY   = #x8A98AE
    // Text/icon on top of an accent or dark fill.
    mod.widgets.RBX_FG_ON_ACCENT  = #xFFFFFF
    // Disabled text.
    mod.widgets.RBX_FG_DISABLED   = #xAEB7C6

    // =========================================================================
    // 3. ACCENT — the teal brand color (primary CTA, selection, focus)
    // =========================================================================
    // Primary accent (calm teal — the "Sign in securely" / primary button color).
    mod.widgets.RBX_ACCENT         = #x119FB3
    // Accent hover.
    mod.widgets.RBX_ACCENT_HOVER   = #x0E8C9E
    // Accent pressed.
    mod.widgets.RBX_ACCENT_PRESSED = #x0B7484
    // Soft accent tint (selected chip bg, highlighted row, focus ring fill).
    mod.widgets.RBX_ACCENT_SOFT    = #xE4F5F7
    // Hyperlink / inline-link color.
    mod.widgets.RBX_LINK           = #x1887C9

    // =========================================================================
    // 4. BRAND — logo colors. Use sparingly (brand entry points, app icon, the
    //    room-identity avatar). Do NOT flood functional UI with purple.
    // =========================================================================
    mod.widgets.RBX_BRAND_PURPLE  = #x572DCC
    mod.widgets.RBX_BRAND_CYAN    = #x05CDC7
    mod.widgets.RBX_BRAND_BLUE    = #x2D7CFF
    // Default room / space identity avatar fill (the teal "#" square).
    mod.widgets.RBX_IDENTITY_TEAL = #x14B8A6

    // =========================================================================
    // 5. STROKES & DIVIDERS
    // =========================================================================
    // Default card / control border (low contrast).
    mod.widgets.RBX_STROKE_SOFT   = #xE6EBF2
    // Stronger border (focused / emphasized control).
    mod.widgets.RBX_STROKE_STRONG = #xD5DEEA
    // Hairline divider between rows (alpha black).
    mod.widgets.RBX_DIVIDER       = #x00000010

    // =========================================================================
    // 6. SEMANTIC STATES — fg/bg pairs. One meaning = one color, always.
    //    success = Connected / Healthy / Enabled / Active / Synced
    //    warning = Approval required / Pending / Waiting
    //    danger  = Failed / Rejected / Error / risk action
    //    info    = capability / linked object / neutral metadata highlight
    //    neutral = Idle / secondary / disabled-ish
    // =========================================================================
    mod.widgets.RBX_SUCCESS_FG = #x1B8A4B
    mod.widgets.RBX_SUCCESS_BG = #xE8F6EE
    mod.widgets.RBX_WARNING_FG = #xC6790B
    mod.widgets.RBX_WARNING_BG = #xFBF1DD
    mod.widgets.RBX_DANGER_FG  = #xC5392F
    mod.widgets.RBX_DANGER_BG  = #xFBE9E7
    mod.widgets.RBX_INFO_FG    = #x1E6FBF
    mod.widgets.RBX_INFO_BG    = #xE7F0FB
    mod.widgets.RBX_NEUTRAL_FG = #x5A6B86
    mod.widgets.RBX_NEUTRAL_BG = #xEEF1F6

    // =========================================================================
    // 7. DARK SURFACES — desktop nav rail + mobile login (the only dark zones
    //    in this round). Kept explicit rather than as a full theme.
    // =========================================================================
    // Desktop left navigation rail background.
    mod.widgets.RBX_NAV_BG             = #x1A2336
    // Nav item idle foreground.
    mod.widgets.RBX_NAV_FG             = #xAEBAD0
    // Nav item active foreground.
    mod.widgets.RBX_NAV_FG_ACTIVE      = #xFFFFFF
    // Nav item active / selected pill background.
    mod.widgets.RBX_NAV_ITEM_ACTIVE_BG = #x2A3650
    // Nav rail hairline / section divider.
    mod.widgets.RBX_NAV_DIVIDER        = #x2C384F
    // Mobile login page background (deep navy).
    mod.widgets.RBX_LOGIN_BG           = #x0E1626
    // Mobile login field / card surface on the dark page.
    mod.widgets.RBX_LOGIN_SURFACE      = #x16213A

    // =========================================================================
    // 8. RADIUS scale — bigger & softer than the legacy RADIUS_* (4/6/8).
    //    Cards lean on radius + border, not heavy shadow.
    // =========================================================================
    mod.widgets.RBX_RADIUS_XS   = 6.0
    mod.widgets.RBX_RADIUS_SM   = 8.0
    mod.widgets.RBX_RADIUS_MD   = 12.0
    mod.widgets.RBX_RADIUS_LG   = 16.0
    mod.widgets.RBX_RADIUS_XL   = 20.0
    // Fully-rounded (pill) — use on badges / chips.
    mod.widgets.RBX_RADIUS_PILL = 100.0

    // =========================================================================
    // 9. SPACING — the 4px grid SPACE_XS..SPACE_XXL (4..24) from styles.rs still
    //    applies. These add the two larger section-level steps the new layouts use.
    // =========================================================================
    mod.widgets.RBX_SPACE_2XL = 32
    mod.widgets.RBX_SPACE_3XL = 40

    // =========================================================================
    // 10. TYPE SCALE — semantic TextStyle presets. Built on the app's real font
    //     providers (theme.font_regular / theme.font_bold). Sizes match Makepad's
    //     dense scale used elsewhere in robrix2 (8.5–18).
    // =========================================================================
    // Mobile/desktop page title (e.g. "Settings", room hero name).
    mod.widgets.RBX_TEXT_PAGE_TITLE    = theme.font_bold    { font_size: 17.0 }
    // Section / group title inside a page.
    mod.widgets.RBX_TEXT_SECTION_TITLE = theme.font_bold    { font_size: 13.0 }
    // Card title.
    mod.widgets.RBX_TEXT_CARD_TITLE    = theme.font_bold    { font_size: 12.0 }
    // Default body / list-row title.
    mod.widgets.RBX_TEXT_BODY          = theme.font_regular { font_size: 11.0 }
    // Emphasized body (selected value, key figure).
    mod.widgets.RBX_TEXT_BODY_STRONG   = theme.font_bold    { font_size: 11.0 }
    // Meta / caption / helper text.
    mod.widgets.RBX_TEXT_META          = theme.font_regular { font_size: 9.5  }
    // Badge / chip label.
    mod.widgets.RBX_TEXT_BADGE         = theme.font_bold    { font_size: 9.0  }
}

// =============================================================================
// Rust-side Vec4 mirror — for programmatic styling (script_apply_eval!, shaders,
// dynamic widgets). Only the most commonly-needed colors are mirrored; add more
// as call sites appear. Values are straight sRGB/255 (matching styles.rs).
// =============================================================================

/// #119FB3 — primary teal accent.
pub const RBX_ACCENT:         Vec4 = vec4(0.067, 0.624, 0.702, 1.0);
/// #0E8C9E — accent hover.
pub const RBX_ACCENT_HOVER:   Vec4 = vec4(0.055, 0.549, 0.620, 1.0);
/// #0B7484 — accent pressed.
pub const RBX_ACCENT_PRESSED: Vec4 = vec4(0.043, 0.455, 0.518, 1.0);
/// #FFFFFF — on-accent foreground.
pub const RBX_FG_ON_ACCENT:   Vec4 = vec4(1.0, 1.0, 1.0, 1.0);

/// #F7F9FC — page canvas.
pub const RBX_BG_CANVAS:      Vec4 = vec4(0.969, 0.976, 0.988, 1.0);
/// #FFFFFF — surface.
pub const RBX_BG_SURFACE:     Vec4 = vec4(1.0, 1.0, 1.0, 1.0);

/// #16233B — primary text.
pub const RBX_FG_PRIMARY:     Vec4 = vec4(0.086, 0.137, 0.231, 1.0);
/// #5A6B86 — secondary text.
pub const RBX_FG_SECONDARY:   Vec4 = vec4(0.353, 0.420, 0.525, 1.0);

/// #E6EBF2 — soft stroke.
pub const RBX_STROKE_SOFT:    Vec4 = vec4(0.902, 0.922, 0.949, 1.0);

/// #1B8A4B — success fg.
pub const RBX_SUCCESS_FG:     Vec4 = vec4(0.106, 0.541, 0.294, 1.0);
/// #E8F6EE — success bg.
pub const RBX_SUCCESS_BG:     Vec4 = vec4(0.910, 0.965, 0.933, 1.0);
/// #C6790B — warning fg.
pub const RBX_WARNING_FG:     Vec4 = vec4(0.776, 0.475, 0.043, 1.0);
/// #FBF1DD — warning bg.
pub const RBX_WARNING_BG:     Vec4 = vec4(0.984, 0.945, 0.867, 1.0);
/// #C5392F — danger fg.
pub const RBX_DANGER_FG:      Vec4 = vec4(0.773, 0.224, 0.184, 1.0);
/// #FBE9E7 — danger bg.
pub const RBX_DANGER_BG:      Vec4 = vec4(0.984, 0.914, 0.906, 1.0);
/// #1E6FBF — info fg.
pub const RBX_INFO_FG:        Vec4 = vec4(0.118, 0.435, 0.749, 1.0);
/// #E7F0FB — info bg.
pub const RBX_INFO_BG:        Vec4 = vec4(0.906, 0.941, 0.984, 1.0);

/// #1A2336 — dark nav rail background.
pub const RBX_NAV_BG:         Vec4 = vec4(0.102, 0.137, 0.212, 1.0);
/// #14B8A6 — room/space identity teal.
pub const RBX_IDENTITY_TEAL:  Vec4 = vec4(0.078, 0.722, 0.651, 1.0);
