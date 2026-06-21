spec: task
name: "Require Cmd Enter To Send Messages"
inherits: project
tags: [input, keyboard, makepad, preferences]
---

## Intent

Robrix message composition must preserve the existing user preference for the send shortcut while defaulting new profiles to the safer primary-modifier path. Users can choose either `Enter` to send or platform primary modifier plus Enter (`Cmd+Enter` on macOS, `Ctrl+Enter` elsewhere), while popup selection still owns a bare Enter when a selectable row is focused.

## Decisions

- New app preferences default to primary-modifier send (`send_on_enter: false`).
- Message input submission respects `send_on_enter`: when true, bare Enter sends outside popup selection; when false, only platform primary modifier Enter sends.
- Platform primary submit modifier remains `Cmd+Enter` on macOS/iOS/tvOS and `Ctrl+Enter` on other targets.
- Slash-command pure-command selection may continue to emit the primary submit action directly.
- The settings screen must keep controlling `send_on_enter`; it must not force the toggle into one mode.

## Boundaries

### Allowed Changes
- src/shared/mentionable_text_input.rs
- src/settings/app_preferences.rs
- src/settings/app_settings.rs
- resources/i18n/en.json
- resources/i18n/zh-CN.json
- specs/task-primary-enter-send.spec.md

### Forbidden
- Do not run `cargo fmt` or `rustfmt`.
- Do not modify Matrix send routing, Octos service code, or appservice command classification.
- Do not add dependencies.

## Acceptance Criteria

Scenario: New app preferences default to primary-modifier send
  Test: default_send_shortcut_requires_primary_modifier
  Given Robrix constructs default app preferences
  When no persisted preference overrides are present
  Then `send_on_enter` is false

Scenario: Return-key submission respects send shortcut preference
  Test: return_key_submission_respects_send_on_enter_preference
  Given app preferences contain `send_on_enter: true`
  When the message input receives a bare Return keypress outside popup selection
  Then Robrix emits `TextInputAction::Returned`
  And given app preferences contain `send_on_enter: false`
  When the message input receives a bare Return keypress outside popup selection
  Then Robrix does not emit `TextInputAction::Returned`
  And when the message input receives primary-modifier Return
  Then Robrix emits `TextInputAction::Returned`

Scenario: Popup selection still owns bare Enter
  Test: return_key_submission_keeps_bare_enter_for_popup_selection
  Given an autocomplete popup is visible with a focused selectable item and `send_on_enter: true`
  When the message input receives a bare Return keypress
  Then Robrix does not force-submit the message before popup handling
  And when the message input receives primary-modifier Return
  Then Robrix force-submits the message

## Out of Scope

- Reworking multiline editing internals.
- Changing BotFather or Octos command routing.
- Removing persisted preference fields from existing app-state JSON.
