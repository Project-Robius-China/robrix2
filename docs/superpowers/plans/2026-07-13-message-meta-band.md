# Message Meta Band (V2 Layout, Phase 2) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: superpowers:executing-plans (inline). Per `specs/task-message-meta-band.spec.md`.

**Goal:** Merge copy button + bot metadata + read receipts into one meta band row; move the timestamp into the username row; restyle the bot badge to the accent pill.

**Architecture:** DSL restructure in `Message`/`CondensedMessage` templates + a data-flow change: `populate_bot_text_message_content` returns the band metadata string, `populate_message_view` threads it into `Message::set_data`, which is the single per-populate choke point that sets both `copy_button` visibility and `metadata_label` text (recycling-safe: always set, both ways).

**Tech Stack:** Makepad 2.0 script_mod DSL, existing RBX_* tokens.

## Global Constraints

Same as Phase 1 plan (Makepad 2.0 syntax, `+:` merges, no cargo fmt, no new deps, tokens only, no commit until user tests, no Claude attribution in commits).

---

### Task A: DSL restructure (both templates)

**Files:** Modify `src/home/room_screen.rs` (Message template ~1913-2260, CondensedMessage ~2265-2475)

Steps:
1. **Message template `username_view`**: after `bot_badge := RoundedView {...}` closing brace, add:
   ```text
                    timestamp := Timestamp {
                        margin: Inset{ left: 2 }
                    }
   ```
2. **Message template `profile`**: remove the `timestamp := Timestamp { margin: Inset{ top: 5.9 } }` child (CondensedMessage's gutter timestamp stays).
3. **bot_badge restyle** (Message template only — Condensed has no username row): `draw_bg +: { color: (COLOR_ACTIVE_PRIMARY) ... }` → `color: (RBX_ACCENT_SOFT)`; label `color: #fff` → `color: (RBX_ACCENT)`.
4. **Remove `bot_metadata_footer := View {...}`** (with `bot_provider_label`/`bot_footer_label`) from BOTH templates' `bot_message_card`.
5. **Meta band** — replace BOTH `message_action_bar` blocks: band `visible` default true (delete the `visible: false` line), children: `copy_button` (now `visible: false` by default on the button itself), new `metadata_label`, then `avatar_row`:
   ```text
                message_action_bar := View {
                    width: Fill,
                    height: Fit
                    flow: Right,
                    align: Align{y: 0.5}
                    margin: Inset{ top: 2.0 }
                    spacing: 6.0

                    copy_button := RobrixNeutralIconButton {
                        visible: false
                        width: Fit,
                        height: Fit,
                        padding: 4
                        margin: 0
                        spacing: 0
                        draw_bg +: {
                            color: #00000000
                            color_hover: (RBX_HIT_HOVER)
                            color_down: (RBX_HIT_DOWN)
                            border_size: 0.0
                        }
                        draw_icon +: { svg: (ICON_COPY), color: (RBX_FG_TERTIARY) }
                        icon_walk: Walk{width: 16, height: 16}
                        text: ""
                    }
                    metadata_label := Label {
                        width: Fill,
                        height: Fit
                        padding: 0
                        max_lines: 1
                        text_overflow: Ellipsis
                        draw_text +: {
                            text_style: mod.widgets.MESSAGE_TEXT_STYLE { font_size: 10.0 }
                            color: (mod.widgets.COLOR_BOT_FOOTER_TEXT)
                        }
                        text: ""
                    }
                    avatar_row := mod.widgets.AvatarRow {}
                }
   ```
6. **Reactions row**: remove `avatar_row := mod.widgets.AvatarRow {}` from the anonymous `View { reaction_list, avatar_row }` in BOTH templates (leave `reaction_list`).
7. `cargo build` → success.

### Task B: Rust wiring

**Files:** Modify `src/home/room_screen.rs`

1. `populate_bot_text_message_content` return type `bool` → `(bool, Option<String>)`:
   - non-card early path: `(populate_text_message_content(...), None)`
   - card path: delete the provider/footer label + footer-visibility populate block (12171-12194 region); compute instead:
     ```rust
     let band_metadata = if render_state.show_metadata_footer {
         match (render_state.provider.as_ref(), render_state.footer.as_ref()) {
             (Some(p), Some(f)) => Some(format!("{p} · {}", display_bot_footer_text(f))),
             (Some(p), None) => Some(p.clone()),
             (None, Some(f)) => Some(display_bot_footer_text(f).to_string()),
             (None, None) => None,
         }
     } else { None };
     ```
     and return `(drawn, band_metadata)` at the end (check `display_bot_footer_text` signature first).
2. Update its 3 call sites (streaming ~11321 discards metadata? NO — streaming also returns metadata; capture in the same local), Text ~11354, Notice ~11407: declare `let mut band_metadata: Option<String> = None;` before the big match in `populate_message_view`; each call site destructures `(drawn, meta)` and assigns `band_metadata = meta;`.
3. `Message::set_data` + `MessageRef::set_data`: rename `show_action_bar` → `show_copy_button`, add `band_metadata: Option<String>`; body replaces the band set_visible with:
   ```rust
   self.view.button(cx, ids!(content.message_action_bar.copy_button))
       .set_visible(cx, show_copy_button);
   self.view.label(cx, ids!(content.message_action_bar.metadata_label))
       .set_text(cx, band_metadata.as_deref().unwrap_or(""));
   ```
4. Call site (~11934): rename local to `show_copy_button`, pass `band_metadata`.
5. Timestamp populate (~12013): `ids!(profile.timestamp)` → `ids!(timestamp)` (name-only resolves both templates). Check surrounding guard: it must run for full messages too — verify placement.
6. `cargo build` → success.

### Task C: Tests

1. Read `test_bot_metadata_footer_renders_below_body` (~15179); rewrite to assert the new band metadata join behavior (or template placement if it was structural).
2. `cargo build && cargo test` → all pass.

### Task D: Verify + handoff

1. `agent-spec lint/verify` with `--change` set (room_screen.rs, both spec files, design html, this plan).
2. macOS visual pass per spec scenarios; STOP for user testing (mac + Android). No commit.

## Self-Review Notes

- Recycling safety: copy_button visibility AND metadata text are both set unconditionally in `set_data` every populate.
- Receipts preserved for image/sticker messages because the band is always visible and only its children toggle.
- `ids!(avatar_row)` / `ids!(reaction_list)` name-only lookups unaffected by the move.
- CondensedMessage: no username row → no timestamp move; gutter timestamp + `ids!(timestamp)` still resolves its `profile.timestamp`.
