# Message Action Bar (Copy Button, Phase 1) Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a Cherry Studio-style always-visible action bar (single Copy button) below every text-like message in the Robrix2 timeline, on macOS and Android, per `specs/task-message-action-bar.spec.md`.

**Architecture:** A presentational `message_action_bar` View is appended to the `Message` and `CondensedMessage` DSL templates in `src/home/room_screen.rs`. Visibility is set unconditionally on every populate (via a new `show_action_bar` param on `Message::set_data`), so PortalList item recycling can never leak stale visibility. The button click re-uses the existing `MessageAction::CopyText(details)` action and handler; the handler gains a success toast (new i18n key).

**Tech Stack:** Makepad 2.0 `script_mod!` DSL, matrix-sdk timeline, existing `RBX_*` design tokens, `tr_key` i18n.

## Global Constraints

(From `specs/project.spec.md` + task spec — every task implicitly includes these.)

- Makepad **2.0** syntax only: `script_mod!`, `:=` for named children, `+:` to merge properties (NEVER bare `:` on `draw_bg`/`draw_icon`/`draw_text` of inherited widgets).
- Do NOT run `cargo fmt`. Do NOT add cargo dependencies. Do NOT use `.unwrap()` on user-facing paths.
- All colors/sizes via existing tokens: `(RBX_FG_TERTIARY)`, `(RBX_HIT_HOVER)`, `(RBX_HIT_DOWN)`, `(ICON_COPY)` — all already in DSL scope in `room_screen.rs` (see `src/shared/design_tokens.rs:71-72`, usage precedent `src/home/room_screen.rs:3423`). NO new hex colors.
- Do NOT modify: context-menu opening logic (`Hit::FingerDown` secondary / `Hit::FingerLongPress`), `src/home/new_message_context_menu.rs`, existing `content` children order (append only), `PortalList` structure.
- **Do NOT commit until the user has manually tested** (project rule). All "commit" happens once, at the end, after user sign-off. Commit message must NOT contain any `Co-Authored-By`/Claude attribution (commit-msg hook rejects it).
- Verify with `agent-spec` at the end (Task 5).

**Branch:** current branch `TigerInYourDream/message-design` in this worktree — already isolated; no new worktree needed.

---

### Task 1: i18n key for the "Copied" toast (test-first)

**Files:**
- Modify: `src/i18n.rs` (append a test in the existing `mod tests`, near line 137)
- Modify: `resources/i18n/en.json` (insert before the `room_screen.popup.message.copy_text_not_found` line, ~line 651)
- Modify: `resources/i18n/zh-CN.json` (insert before `room_screen.popup.message.copy_text_not_found`, ~line 649)

**Interfaces:**
- Produces: i18n key `"room_screen.popup.message.copied"` resolvable via `tr_key(lang, "room_screen.popup.message.copied")` in both locales. Task 4 depends on this key.

- [ ] **Step 1: Write the failing test**

In `src/i18n.rs`, inside `mod tests` (after `test_invitebot_i18n_keys_exist_in_all_locales`, which is the house pattern being mirrored), add:

```rust
    #[test]
    fn message_action_bar_i18n_keys_exist_in_all_locales() {
        for key in [
            "room_screen.popup.message.copied",
        ] {
            for language in AppLanguage::ALL {
                assert!(
                    dictionary(language).contains_key(key),
                    "missing i18n key {key:?} for language {language:?}",
                );
            }
        }
    }
```

- [ ] **Step 2: Run test to verify it fails**

Run: `cargo test message_action_bar_i18n_keys_exist_in_all_locales`
Expected: FAIL with `missing i18n key "room_screen.popup.message.copied" for language English`

- [ ] **Step 3: Add the keys to both locale files**

In `resources/i18n/en.json`, the file is a flat, alphabetically-ordered JSON object. Insert this line directly BEFORE the line containing `"room_screen.popup.message.copy_text_not_found"` (~line 651; `copied` sorts before `copy_`):

```json
  "room_screen.popup.message.copied": "Copied to clipboard.",
```

In `resources/i18n/zh-CN.json`, insert directly BEFORE its `"room_screen.popup.message.copy_text_not_found"` line (~line 649):

```json
  "room_screen.popup.message.copied": "已复制到剪贴板。",
```

- [ ] **Step 4: Run test to verify it passes**

Run: `cargo test message_action_bar_i18n_keys_exist_in_all_locales`
Expected: PASS (1 passed)

---

### Task 2: `message_action_bar` DSL in both message templates

**Files:**
- Modify: `src/home/room_screen.rs:2231` (Message template — after `thread_root_summary`)
- Modify: `src/home/room_screen.rs:~2419` (CondensedMessage template — after its `thread_root_summary`; line shifts by the Task-2 Message-template insertion)

**Interfaces:**
- Produces: widget path `content.message_action_bar` (a `View`, default `visible: false`) containing child button `copy_button`, reachable as `ids!(content.message_action_bar)` and `ids!(content.message_action_bar.copy_button)`. Tasks 3 and 4 depend on these exact names.

**Background for the implementer:** `CondensedMessage` (line 2238) re-declares `body :=` with `:=` — a FULL replacement of the subtree, not a merge — so the bar must be pasted into both templates. The ghost-button styling mirrors the proven `copy_room_id_button` at `room_screen.rs:3411-3426`. `RobrixNeutralIconButton` is defined in `src/shared/icon_button.rs:112` (base `RobrixIconButton` already has `border_radius: 4.0`, the `RBX_RADIUS_XS` value).

- [ ] **Step 1: Add the bar to the `Message` template**

In `src/home/room_screen.rs`, inside the `Message` template's `content := View { ... }`, insert immediately AFTER the line `thread_root_summary := mod.widgets.ThreadRootSummary {}` (line 2231) and before the closing brace of `content`:

```text
                message_action_bar := View {
                    visible: false
                    width: Fill,
                    height: Fit
                    flow: Right,
                    align: Align{y: 0.5}
                    margin: Inset{ top: 2.0 }

                    copy_button := RobrixNeutralIconButton {
                        width: Fit,
                        height: Fit,
                        padding: 6
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
                }
```

- [ ] **Step 2: Add the identical bar to the `CondensedMessage` template**

Inside the `CondensedMessage` template's `content := View { ... }`, insert the exact same `message_action_bar := View { ... }` block (verbatim copy of Step 1's block) immediately AFTER its `thread_root_summary := mod.widgets.ThreadRootSummary {}` line (was line 2419 pre-insertion) and before the closing brace of `content`.

- [ ] **Step 3: Verify it compiles**

Run: `cargo build`
Expected: success. (The bar is `visible: false` everywhere until Task 3, so `cargo run` at this point shows no visual change.)

---

### Task 3: Visibility wiring — show the bar only for text-like messages

**Files:**
- Modify: `src/home/room_screen.rs:11875` (the single `item.as_message().set_data(...)` call site in `populate_message_view`)
- Modify: `src/home/room_screen.rs:13832-13884` (`Message::set_data` + `MessageRef::set_data` — the only two definitions; grep `\.set_data(` confirms 11875 is the only external caller)

**Interfaces:**
- Consumes: `content.message_action_bar` widget path from Task 2.
- Produces: `Message::set_data(&mut self, cx: &mut Cx, details: MessageDetails, download_info: Option<DownloadableAttachment>, download_state: DownloadDisplayState, show_action_bar: bool)` — new trailing `bool` param, same on `MessageRef::set_data`. Task 4's click handler relies on visibility already being correct.

- [ ] **Step 1: Extend `Message::set_data` with the visibility parameter**

At `src/home/room_screen.rs:13833`, change the signature and add the visibility call. The function currently starts:

```rust
    fn set_data(
        &mut self,
        cx: &mut Cx,
        details: MessageDetails,
        download_info: Option<DownloadableAttachment>,
        download_state: DownloadDisplayState,
    ) {
        let prev_section_visible = self.download_info.is_some();
```

Change to (new param + one new call, placed right alongside the existing `download_section` visibility handling so the two per-populate visibility knobs live together):

```rust
    fn set_data(
        &mut self,
        cx: &mut Cx,
        details: MessageDetails,
        download_info: Option<DownloadableAttachment>,
        download_state: DownloadDisplayState,
        show_action_bar: bool,
    ) {
        let prev_section_visible = self.download_info.is_some();
```

Then, immediately after the existing lines

```rust
        self.view.view(cx, ids!(content.download_section))
            .set_visible(cx, section_visible);
```

add:

```rust
        self.view.view(cx, ids!(content.message_action_bar))
            .set_visible(cx, show_action_bar);
```

(Unconditional set on every populate — both `true` and `false` — is what makes PortalList item recycling safe: a recycled `Message` item previously showing the bar gets it switched off when re-used for a non-text message.)

- [ ] **Step 2: Update `MessageRef::set_data` forwarder**

At `src/home/room_screen.rs:13874` (post-edit line numbers shift), change:

```rust
impl MessageRef {
    fn set_data(
        &self,
        cx: &mut Cx,
        details: MessageDetails,
        download_info: Option<DownloadableAttachment>,
        download_state: DownloadDisplayState,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_data(cx, details, download_info, download_state);
    }
}
```

to:

```rust
impl MessageRef {
    fn set_data(
        &self,
        cx: &mut Cx,
        details: MessageDetails,
        download_info: Option<DownloadableAttachment>,
        download_state: DownloadDisplayState,
        show_action_bar: bool,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.set_data(cx, details, download_info, download_state, show_action_bar);
    }
}
```

- [ ] **Step 3: Compute the flag at the call site**

At `src/home/room_screen.rs:11875`, the current call:

```rust
    item.as_message().set_data(cx, message_details, download_info, download_state);
```

becomes:

```rust
    // The action bar (copy button) only applies to text-like messages;
    // media/sticker/redacted/UTD items keep it hidden.
    let show_action_bar = matches!(
        &msg_like_content.kind,
        MsgLikeKind::Message(msg) if matches!(
            msg.msgtype(),
            MessageType::Text(_) | MessageType::Notice(_) | MessageType::Emote(_)
        )
    );
    item.as_message().set_data(cx, message_details, download_info, download_state, show_action_bar);
```

`msg_like_content: &MsgLikeContent` is already a parameter of `populate_message_view` (line 11222), and `MsgLikeKind` / `MessageType` are already imported (used at lines 11275-11287). This single choke point covers every message kind: Text/Notice/Emote → `true`; Image/Video/Audio/File/Location/ServerNotice/unsupported → `false`; Redacted/UnableToDecrypt/Sticker/Poll (other `MsgLikeKind` variants) → `false`.

- [ ] **Step 4: Build and visually spot-check**

Run: `cargo build`
Expected: success, no warnings about unused `show_action_bar`.

Run: `cargo run` → open a room with text + image messages.
Expected: a small gray copy icon sits below every text message (including condensed ones); image/sticker messages show none. Scrolling up and down does not make bars appear on wrong items (recycling check).

---

### Task 4: Click dispatch + success toast

**Files:**
- Modify: `src/home/room_screen.rs:13789-13805` (the `Event::Actions` block inside `Message::handle_event` — where `download_button` clicks are already handled)
- Modify: `src/home/room_screen.rs:8986-9002` (the `MessageAction::CopyText` handler arm in RoomScreen)

**Interfaces:**
- Consumes: `ids!(content.message_action_bar.copy_button)` from Task 2; i18n key `room_screen.popup.message.copied` from Task 1.
- Produces: end-to-end copy flow. No new types or actions — reuses `MessageAction::CopyText(MessageDetails)`.

- [ ] **Step 1: Dispatch `CopyText` on button click**

In `Message::handle_event`, inside the existing `if let Event::Actions(actions) = event {` block (line 13789), AFTER the two download-button checks and BEFORE the `for action in actions {` loop, add:

```rust
            if self.view.button(cx, ids!(content.message_action_bar.copy_button)).clicked(actions) {
                cx.widget_action(
                    details.room_screen_widget_uid,
                    MessageAction::CopyText(details.clone()),
                );
            }
```

(`details` is already in scope — `let Some(details) = self.details.clone() else { return };` at line 13655. This is byte-for-byte the same dispatch the context menu performs in `new_message_context_menu.rs:446-452`, so the downstream handler needs no awareness of which entry point fired.)

- [ ] **Step 2: Add the success toast to the shared `CopyText` handler**

At `src/home/room_screen.rs:8986`, the current arm:

```rust
                MessageAction::CopyText(details) => {
                    let Some(tl) = self.tl_state.as_ref() else { return };
                    if let Some(event_tl_item) = Self::find_event_in_timeline(&tl.items, details, has_encryption_notice) {
                        cx.copy_to_clipboard(&plaintext_body_of_timeline_item(event_tl_item));
                    }
```

becomes:

```rust
                MessageAction::CopyText(details) => {
                    let Some(tl) = self.tl_state.as_ref() else { return };
                    if let Some(event_tl_item) = Self::find_event_in_timeline(&tl.items, details, has_encryption_notice) {
                        cx.copy_to_clipboard(&plaintext_body_of_timeline_item(event_tl_item));
                        enqueue_popup_notification(
                            tr_key(self.app_language, "room_screen.popup.message.copied"),
                            PopupKind::Success,
                            Some(2.0),
                        );
                    }
```

The `else` branch (error popup + `error!` log) stays untouched. `enqueue_popup_notification`, `tr_key`, and `PopupKind` are already imported in this file (used at lines 8963-8967 in the adjacent `Pin` arm). Because the toast lives in the shared handler, the right-click/long-press context menu's Copy Text gains the same confirmation for free (spec scenario `manual_test_context_menu_copy_regression`).

- [ ] **Step 3: Build and run the full test suite**

Run: `cargo build && cargo test`
Expected: build succeeds; all tests pass including `message_action_bar_i18n_keys_exist_in_all_locales`.

---

### Task 5: Verification against the spec, then user-testing gate

**Files:** none modified — verification only.

- [ ] **Step 1: agent-spec mechanical verification**

Run (repeated `--change` with relative paths — do NOT use `--change-scope`, it incorrectly absolutizes worktree paths):

```bash
agent-spec lint specs/task-message-action-bar.spec.md --min-score 0.7
agent-spec verify specs/task-message-action-bar.spec.md \
  --change src/home/room_screen.rs \
  --change src/i18n.rs \
  --change resources/i18n/en.json \
  --change resources/i18n/zh-CN.json \
  --change specs/task-message-action-bar.spec.md \
  --change docs/superpowers/plans/2026-07-10-message-action-bar.md
```

Expected: lint ≥ 0.7; verify reports Boundaries pass (all changed files within Allowed Changes). Manual-selector scenarios report `skip`/`uncertain` — that is expected for this project; they are discharged by Step 2.

- [ ] **Step 2: macOS manual test checklist (run `cargo run`)**

Walk the spec scenarios on desktop:

1. `manual_test_action_bar_visible_desktop` — gray copy icon below a text message, left-aligned with body text.
2. `manual_test_copy_click_desktop` — hover shows subtle tint; click → paste into another app reproduces the body; Success toast appears, auto-dismisses ≈2s.
3. `manual_test_condensed_message_bar` — send two quick consecutive messages; second (condensed) also has the bar.
4. `manual_test_multiline_copy` — 3-line message copies all lines with newlines.
5. `manual_test_image_message_no_bar` — image message: no bar.
6. `manual_test_redacted_message_no_bar` — delete a message: no bar on the redacted item.
7. `manual_test_context_menu_copy_regression` — right-click → Copy Text still works AND now shows the toast.
8. `manual_test_copy_event_not_found` — (best-effort) if reproducible, expect the existing Error popup; otherwise confirm by code inspection that the else-branch is untouched.
9. Visual noise check in a dense group-chat room; zh-CN language check for the toast (设置里切中文).

- [ ] **Step 3: Hand off to the user for testing (STOP — do not commit)**

Present: what changed, how to test on macOS (`cargo run`) and Android (user's usual `cargo makepad android run` flow), including `manual_test_copy_tap_android` and `manual_test_android_scroll_no_copy` (drag starting on the copy button must scroll, not copy).

**Wait for user confirmation.** Only after the user confirms both platforms:

- [ ] **Step 4: Single commit (post-approval only)**

```bash
git add src/home/room_screen.rs src/i18n.rs resources/i18n/en.json resources/i18n/zh-CN.json \
        specs/task-message-action-bar.spec.md docs/superpowers/plans/2026-07-10-message-action-bar.md
git commit -m "feat(timeline): message action bar with copy button below text messages

Cherry Studio-style always-visible per-message action bar (Phase 1: copy
only) on Message + CondensedMessage templates. Reuses the existing
MessageAction::CopyText pipeline and adds a success toast (new i18n key
room_screen.popup.message.copied) benefiting the context-menu path too."
```

(No Co-Authored-By lines — the commit-msg hook rejects Claude attribution.)

---

## Self-Review Notes

- **Spec coverage:** every Decision maps to a task — DSL bar (T2), both templates (T2), visibility per message type (T3), ghost styling/tokens (T2), click dispatch (T4), toast + i18n (T1+T4), no-new-deps (nothing added). All 11 scenarios are discharged in T5 Steps 1-3.
- **Recycling correctness:** visibility is re-set on EVERY populate through the single `set_data` choke point (including the cached-item path at 11875, which always runs) — the classic PortalList stale-state bug is structurally excluded.
- **Type consistency:** `show_action_bar: bool` is the trailing param in `Message::set_data`, `MessageRef::set_data`, and the single call site; widget paths `content.message_action_bar` / `content.message_action_bar.copy_button` are identical across T2/T3/T4.
