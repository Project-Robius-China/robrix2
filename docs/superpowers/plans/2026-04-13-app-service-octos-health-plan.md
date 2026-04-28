# App Service Octos Health Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Show an editable Octos Service URL plus a manual health-check status in `Settings > App Service`, defaulting to `http://127.0.0.1:8010`, without changing Matrix routing or bot behavior.

**Architecture:** Persist the Octos Service base URL in `BotSettingsState`, validate it before save/probe, and keep the health-check status as widget-local UI state in `BotSettings`. The probe sequence uses `{configured_url}/health` with fallback to `{configured_url}/api/status`, and the result only updates local settings UI state.

**Tech Stack:** Rust, Makepad 2.0 `script_mod!`, widget-local `cx.http_request` / `Event::NetworkResponses`, serde-persisted app state, existing `url` crate for URI validation.

---

### Task 1: Add persisted Octos service URL + pure validation helpers

**Files:**
- Modify: `src/app.rs`
- Test: `src/app.rs`

- [ ] **Step 1: Write the failing state test**

Add tests that construct default/custom `BotSettingsState` and assert:
- default service URL resolves to `http://127.0.0.1:8010`
- custom configured URL is preserved
- invalid URLs are rejected by validation helper
- health status defaults to `Unknown`
- checking flag defaults to false

- [ ] **Step 2: Run the test to verify it fails**

Run: `cargo test app_service_health_defaults`
Expected: FAIL because the health state and helper do not exist yet

- [ ] **Step 3: Add minimal state model**

In `src/app.rs`:
- add persisted `octos_service_url` field to `BotSettingsState`
- default it to `http://127.0.0.1:8010`
- add helper to resolve empty values back to the default
- add URI validation helper for `http` / `https`

- [ ] **Step 4: Run the test to verify it passes**

Run: `cargo test app_service_health_defaults`
Expected: PASS

### Task 2: Add manual health-check state machine

**Files:**
- Modify: `src/settings/bot_settings.rs`
- Test: `src/settings/bot_settings.rs`

- [ ] **Step 1: Write the failing behavior tests**

Add focused tests for:
- successful `{configured_url}/health` result maps to `Reachable`
- `{configured_url}/health` failure with `/api/status` success still maps to `Reachable`
- both probe failures map to `Unreachable`
- duplicate `Check Now` presses while checking do not enqueue overlapping work

- [ ] **Step 2: Run the tests to verify they fail**

Run:
```bash
cargo test app_service_health_check_
```
Expected: FAIL because the state machine still assumes a fixed URL

- [ ] **Step 3: Add request + action plumbing**

In `src/settings/bot_settings.rs`:
- keep `OctosHealthStatus` / probe stage as widget-local state
- make the probe sequence depend on the currently configured base URL
- keep `Checking` / duplicate-click suppression local to the widget
- treat any HTTP 200 as reachable
- treat timeout/connect/status failures on both endpoints as unreachable

- [ ] **Step 4: Run the tests to verify they pass**

Run:
```bash
cargo test app_service_health_check_
```
Expected: PASS

### Task 3: Render editable URL field, Save, and Check Now in BotSettings

**Files:**
- Modify: `src/settings/bot_settings.rs`
- Modify: `resources/i18n/en.json`
- Modify: `resources/i18n/zh-CN.json`
- Test: `src/settings/bot_settings.rs`

- [ ] **Step 1: Write the failing widget tests**

Add tests covering:
- card shows editable URL with local default and `Unknown`
- invalid URL is rejected before probing
- opening settings does not auto-start a probe
- pressing `Check Now` sets status to `Checking`
- button is disabled or ignored while already checking

- [ ] **Step 2: Run the tests to verify they fail**

Run:
```bash
cargo test app_service_health_ui_
```
Expected: FAIL because the UI row does not exist yet

- [ ] **Step 3: Implement the minimal UI**

In `src/settings/bot_settings.rs`:
- add a compact status block under the existing app service toggle
- show:
  - editable Octos Service input
  - `Save` button
  - status label
  - `Check Now` button
- validate the URL before save or probe
- persist valid URL changes to `BotSettingsState`
- use widget-local `cx.http_request` probing only when not already checking
- keep all text/theme consistent with existing settings card patterns

In i18n:
- add strings for service label, placeholder, save/check buttons, validation error, saved popup, and status labels

- [ ] **Step 4: Run the tests to verify they pass**

Run:
```bash
cargo test app_service_health_ui_
```
Expected: PASS

### Task 4: Regression verification

**Files:**
- Modify: `specs/task-app-service-octos-health.spec.md` only if implementation forced a wording correction

- [ ] **Step 1: Run targeted behavior tests**

Run:
```bash
cargo test app_service_health_defaults
cargo test app_service_health_check_
cargo test app_service_health_ui_
```
Expected: PASS

- [ ] **Step 2: Run full build**

Run:
```bash
cargo build
```
Expected: PASS

- [ ] **Step 3: Manual verification**

Check in app:
- `Settings > App Service` shows `Octos Service`
- the field defaults to `http://127.0.0.1:8010`
- editing and saving a valid remote URL persists it
- invalid URLs are rejected before probe
- initial status is `Unknown`
- opening settings alone does not trigger a check
- clicking `Check Now` moves to `Checking`
- with `{configured_url}` reachable, status becomes `Reachable`
- with `{configured_url}` unreachable, status becomes `Unreachable`
- App Service enable toggle and room bindings are unaffected
