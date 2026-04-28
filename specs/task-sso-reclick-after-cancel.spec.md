spec: task
name: "SSO Option Is Re-clickable After Cancel"
inherits: project
tags: [bugfix, login, sso, multi-account, ui]
estimate: 1d
---

## Intent

Fix issue #43 (https://github.com/Project-Robius-China/robrix2/issues/43): in add-account flow, after starting SSO once and cancelling, the SSO provider buttons can become non-clickable when returning to the sign-in screen. The expected behavior is that cancellation must always restore a retryable SSO state, so users can click an SSO provider again without restarting the app.

## Constraints

- Keep existing async request path: `submit_async_request(MatrixRequest::SpawnSSOServer { ... })`
- Keep duplicate-request guard while SSO is truly pending (`sso_pending` should still block repeated clicks during active flow)
- Preserve existing add-account navigation behavior (show/hide login screen semantics in `App`)
- Do not change login/signup semantics unrelated to SSO cancellation

## Decisions

- Keep SSO re-entry logic in existing login flow; do not redesign authentication architecture
- SSO button enabled/disabled UI must be driven by real SSO lifecycle state, not stale local state from prior attempts
- Cancellation paths (SSO modal cancel, add-account cancel, and return to login screen) must converge to a state where SSO is clickable again
- Preserve existing behavior that blocks duplicate SSO launches while a request is genuinely in flight
- `LoginAction::LoginFailure` emitted during add-account flow must not flip global `logged_in` state or hide the existing home/settings screen

## Boundaries

### Allowed Changes
- src/login/login_screen.rs
- src/sliding_sync.rs
- src/app.rs (only if needed to reset add-account/login transition state)

### Forbidden
- Do not change non-SSO login flows (password login/signup semantics)
- Do not add new dependencies
- Do not run `cargo fmt` or reformat unrelated code
- Do not change provider list/branding or add new SSO providers

## Acceptance Criteria

Scenario: Re-click SSO after cancelling an SSO attempt in add-account mode
  Test: manual_test_add_account_sso_retry_after_cancel
  Given the user is logged in and opens "Add another account"
  When the user clicks any SSO provider and then cancels the SSO flow
  Then returning to the add-account login screen shows SSO providers as enabled
  And clicking the same provider again starts a new SSO attempt

Scenario: Cancel add-account screen after SSO cancel, then re-open add-account
  Test: manual_test_add_account_cancel_then_reopen_sso_clickable
  Given an SSO attempt was cancelled during add-account flow
  When the user presses add-account cancel and later opens add-account again
  Then SSO providers are clickable on first try

Scenario: Cancel add-account after SSO cancel returns to non-blank settings/home UI
  Test: manual_test_add_account_cancel_returns_to_settings
  Given the user is logged in and enters add-account flow from settings
  And the user cancels an in-progress SSO flow
  When the user presses add-account cancel to go back
  Then the previous settings/home interface remains visible (not a blank page)
  And the session remains logged in

Scenario: Pending guard still blocks duplicate clicks only while truly pending
  Test: manual_test_sso_pending_guard_scope
  Given an SSO request is actively in flight
  When the user repeatedly clicks SSO provider buttons
  Then additional requests are ignored during pending
  And once pending ends (success, failure, or cancel), providers become clickable again

Scenario: UI affordance matches interactivity
  Test: manual_test_sso_button_visual_state_after_cancel
  Given an SSO flow has been cancelled
  When the user returns to the login screen
  Then SSO button cursor and visual mask indicate enabled/clickable state

Scenario: Regression guard for non-SSO login
  Test: manual_test_password_login_unchanged
  Given the login screen is shown
  When the user logs in with user ID and password
  Then password-based login behavior remains unchanged

## Out of Scope

- Changing SSO backend protocol, callback URL format, or browser-launch mechanism
- Adding telemetry/analytics for SSO cancellation
- UX redesign of login screen layout or modal copy
