# Issue 006: SSO provider list is hardcoded, causing GitHub login failures on many homeservers

**Date:** 2026-04-12
**Severity:** Major (SSO login is unreliable across homeservers)
**Status:** Open
**Affected components:** `src/login/login_screen.rs`, `src/sliding_sync.rs`, `src/persistence/matrix_state.rs`

## Summary

Robrix's SSO login UI is currently hardcoded to a fixed set of branded providers (`apple`, `facebook`, `github`, `gitlab`, `google`, `twitter`) and always maps them to `identity_provider_id = oidc-{brand}`.

This means "Login with GitHub" only works when the target Matrix homeserver exposes an SSO provider with the exact ID `oidc-github`. Many homeservers either:

- use a different provider ID,
- expose a different provider set entirely,
- or do not support SSO on the same homeserver URL the user is currently targeting.

The result is that GitHub login can fail even though the app supports multiple Matrix homeservers and multiple accounts.

## Symptoms

- Clicking the GitHub button fails on some homeservers even though SSO is configured server-side.
- The same Robrix build can succeed against one homeserver and fail against another.
- Users can conclude that Robrix "does not support multiple Matrix servers", even though normal login and multi-account state do support that.
- Leaving the homeserver field empty silently targets the default homeserver flow instead of the server the user actually intended.

## Evidence

### 1. The SSO buttons are hardcoded in the login screen

`src/login/login_screen.rs` defines a fixed provider grid:

- `apple_button`
- `facebook_button`
- `github_button`
- `gitlab_button`
- `google_button`
- `twitter_button`

Code reference:

- [src/login/login_screen.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/login/login_screen.rs:250)

### 2. GitHub always maps to `oidc-github`

On click, Robrix does not ask the homeserver which providers exist. It constructs the provider ID locally:

```rust
submit_async_request(MatrixRequest::SpawnSSOServer{
    identity_provider_id: format!("oidc-{}", brand),
    brand: brand.to_string(),
    homeserver_url: homeserver_input.text(),
    proxy,
});
```

Code reference:

- [src/login/login_screen.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/login/login_screen.rs:1209)

### 3. The worker uses that provider ID directly

The SSO path forwards the hardcoded provider ID into `matrix_auth().login_sso(...).identity_provider_id(...)` without dynamic discovery:

- [src/sliding_sync.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/sliding_sync.rs:5873)

### 4. The app does support multiple accounts and per-account homeservers

This is not a general "single server only" limitation:

- Account manager stores multiple accounts: [src/account_manager.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/account_manager.rs:39)
- Persisted sessions store a homeserver per account: [src/persistence/matrix_state.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/persistence/matrix_state.rs:21)
- Client building uses the provided homeserver or one inferred from the MXID: [src/sliding_sync.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/sliding_sync.rs:329)

## Root Cause

SSO provider selection is implemented as a static UI branding choice instead of a homeserver capability discovery flow.

The current model is:

1. show six preselected provider icons,
2. assume the provider ID is `oidc-{brand}`,
3. submit SSO against the chosen homeserver.

That is only valid for homeservers that deliberately match Robrix's assumptions.

## Why This Breaks Real Deployments

Matrix homeservers are allowed to expose different SSO providers and provider IDs.

Robrix currently does not:

- fetch login types from `/_matrix/client/v3/login`,
- inspect the server's actual SSO provider list,
- or adapt the UI to the returned providers.

So the GitHub button is not really "GitHub login support"; it is "try `oidc-github` and hope the server agrees".

## Current User Impact

- GitHub login can appear broken on custom homeservers.
- Users must guess whether the issue is:
  - the wrong homeserver,
  - the wrong provider ID,
  - or an actual SSO backend failure.
- The static UI can advertise providers that the server does not support at all.

## Recommended Fix

Replace the hardcoded provider button set with dynamic homeserver discovery:

1. Query the target homeserver's login capabilities from `/_matrix/client/v3/login`.
2. Extract the available SSO / identity providers.
3. Render the login buttons from the server response instead of a fixed local list.
4. Pass the real provider ID returned by the server into `identity_provider_id(...)`.
5. If the server exposes no SSO providers, hide the SSO section entirely instead of showing unusable buttons.

## Non-Goals

- Do not redesign password login.
- Do not remove multi-account support.
- Do not hardcode more provider names to chase server differences.

## Related Files

- [src/login/login_screen.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/login/login_screen.rs)
- [src/sliding_sync.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/sliding_sync.rs)
- [src/account_manager.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/account_manager.rs)
- [src/persistence/matrix_state.rs](/Users/zhangalex/Work/Projects/FW/robius/robrix2/src/persistence/matrix_state.rs)
- [specs/task-sso-reclick-after-cancel.spec.md](/Users/zhangalex/Work/Projects/FW/robius/robrix2/specs/task-sso-reclick-after-cancel.spec.md)
