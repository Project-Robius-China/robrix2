# Issue 002: Settings DropDown Arrow Visual Artifact

## Status: Open

## Symptom

The Settings → Preferences → Application Language dropdown displays a blue elliptical shape on the right side of the dropdown button. This is the dropdown arrow indicator area rendered by the DropDown shader, appearing as an oversized capsule shape.

## Root Cause

Makepad's `DropDown` widget has a hardcoded arrow drawing region in its `pixel: fn()` shader (in `widgets/src/drop_down.rs`, lines 113-200). The shader draws:
1. A background quad with `border_radius`
2. A separate arrow region on the right side with its own color (`arrow_color`)

When `border_radius` is set to `6.0` and the button width is `Fit`, the arrow region's capsule shape becomes visually prominent because the shader's arrow area calculation doesn't account for the smaller border radius.

## Affected Files

- `src/settings/settings_screen.rs` — the `language_dropdown` instance

## Investigation Notes

### Why styling the DropDown is difficult in Makepad

1. **uniform() vs plain value types**: The DropDown shader uses two types of color declarations:
   - `draw_text.color` = plain value (base color for `get_color` shader mix chain)
   - `draw_text.color_hover/focus/down` = `uniform()` (GPU uniforms)
   - All `draw_bg.*` colors = `uniform()`
   
2. Overriding `color` with `uniform()` breaks the shader's `get_color` function (the widget renders blank)
3. Overriding `uniform()` colors with plain values has no effect
4. Custom `set_type_default() do mod.widgets.DropDownFlat { ... }` types fail to render — the `popup_menu` field (stored as `ScriptValue`) loses its registration chain through `on_after_apply` → `PopupMenuGlobal` `ComponentMap`

### What was tried and failed

- Custom `SettingsLanguageDropdown` type via `set_type_default()` — rendered blank
- Plain value overrides on `draw_bg.color` — no effect (uniform not overridden)
- `uniform()` override on `draw_text.color` — broke get_color shader, blank text
- `DropDownFlat` with inline overrides — partially worked but arrow artifact remains

### Current workaround

Using `DropDown` (not `DropDownFlat`) with correct uniform/plain type matching for color overrides. The arrow visual artifact is accepted as a cosmetic issue.

## Potential Fix

Override the `pixel: fn()` shader on the `draw_bg` to customize the arrow drawing area. This requires rewriting ~90 lines of shader code from `drop_down.rs` and is high-effort for a cosmetic issue.

Alternative: Wait for Makepad upstream to expose arrow styling as configurable properties.

## Related

- Makepad source: `widgets/src/drop_down.rs` lines 113-200 (pixel shader)
- Makepad source: `widgets/src/popup_menu.rs` (PopupMenu rendering)
