# Mobile App Lifecycle Handling

Date: 2026-05-28
Status: Design for review
Scope: Robrix2 app lifecycle behavior (mobile + desktop), the Matrix sync service's coupling to that lifecycle, and the persistence write path it depends on.

## Goal

Bring robrix2's app lifecycle handling in line with platform expectations on every platform it ships to:

- save app state and stop the Matrix sync service when the app moves to background on mobile
- resume the Matrix sync service when the app returns to foreground on mobile
- save app state on the main window's close request before the window goes away on desktop
- save app state when the OS asks the app to quit (menu Quit, Cmd+Q, terminal Ctrl+C) before the app actually exits
- never write the same app-state JSON to disk twice in a row when nothing has changed
- give macOS the right application menu identity instead of the Makepad default

These behaviors are mandatory infrastructure for mobile robrix2 — without them, a backgrounded chat client loses unsaved state and keeps a Matrix sync alive in the OS background, wasting battery and network.

This design only covers the lifecycle infrastructure and the macOS menu identity that ships with it. It does not redesign settings, login, logout, or any business feature.

## Context

Robrix2 today has a single lifecycle code path: an inline `Event::Shutdown` block inside `App::handle_event` that saves the window geometry, saves the app state, and (under the `tsp` feature) saves the TSP wallet. Everything else about lifecycle is missing:

- on mobile, the OS sends `Pause` / `Background` / `Resume` / `Foreground` events; robrix2 ignores them
- the Matrix sync service stays running while the app is in background
- when the OS asks the app to quit (Cmd+Q, terminal SIGINT), robrix2 has no chance to react before exit
- the main window's red-X close has no dedicated save hook beyond whatever fires before `Shutdown`
- preference toggles save the full app state JSON on every change, even when the bytes are identical

In parallel, the Makepad runtime currently pinned in robrix2 (`kevinaboos/makepad` on the `cargo_makepad_ndk_fix` branch) predates a number of lifecycle-related Makepad APIs that are now standard: `Event::QuitRequested`, `Cx::request_quit`, the macOS bundle-identity environment variables, and a more comprehensive Android NDK build-tool setup. To get the lifecycle infrastructure, robrix2 needs to align its Makepad runtime with the version other parts of the Robius ecosystem already use.

The user's local divergences in the files this work touches are deliberate and must not be lost in the migration. They are listed in the dedicated section below.

## Design Direction

Three layers, each independent of the others, each committable on its own:

1. **Makepad runtime + build tool alignment.** Switch robrix2's runtime Makepad source to `makepad/dev`. Document that `cargo-makepad` (the CLI used for Android / iOS / packaging) should also be installed from `makepad/dev` — that branch's Android NDK and iOS tooling is more elaborate than the patch set that previously had to be carried as a fork.

2. **A small lifecycle subsystem inside the app.** Add an `AppLifecycle` struct on `App`, a single dispatch method `handle_lifecycle_event`, three private helpers (`persist_runtime_state`, `handle_shutdown`, and the fingerprint type that backs the dedup), and one new line at the end of `handle_event` to call into the dispatcher. Remove the inline `Event::Shutdown` block — its work moves into `handle_shutdown`.

3. **A sync-service state machine.** Replace the "call `sync_service.start()/stop()` directly" pattern with a desired/assumed running pair of atomic flags plus a `tokio::sync::Mutex` lifecycle lock. Lifecycle events set the desired state; an async worker reconciles assumed to desired. This makes background/foreground transitions safe against concurrent sync restarts and OS-level cancellations.

The persistence write path gets a small refactor so that the in-process lifecycle handler can serialize once, fingerprint the bytes, and skip the disk write when the bytes match the last save.

## Chosen Approach

### App lifecycle subsystem (`src/app.rs`)

A new private struct `AppLifecycle` is added next to `App`, holding four boolean/option fields: `is_foreground`, `is_active`, `last_app_state_save: Option<AppStateSaveFingerprint>`, and `shutdown_started`. The fingerprint type captures the user id plus a 64-bit hash plus the byte length of the serialized state, so that two consecutive writes that produce identical bytes can be detected without keeping the bytes around.

`App::handle_event` becomes mechanical: forward the event to the existing `MatchEvent` and `Widget::handle_event` paths, then call `self.handle_lifecycle_event(cx, event)`. The old inline `Event::Shutdown` block is removed in the same commit.

`handle_lifecycle_event` is a single `match event` with six arms:

- `QuitRequested(e)` — saves state via `persist_runtime_state("quit request")`
- `Pause` — flips `is_active` to false, saves state via `persist_runtime_state("pause")`
- `Resume` — flips `is_active` to true, requests sync to resume
- `Background` — flips `is_foreground` to false, saves state, requests sync to stop
- `Foreground` — flips `is_foreground` to true, requests sync to start
- `WindowCloseRequested(e)` — if the closing window is the main window, saves state
- `Shutdown` — calls `handle_shutdown`, which guards against double-fire, saves state, stops the sync service with a 3-second timeout, and (under the `tsp` feature) closes and serializes the TSP wallet with the same timeout.

`persist_runtime_state(reason: &'static str)` is the single chokepoint for "save now":
1. write the window geometry
2. if no logged-in user, return
3. serialize the current `AppState` to bytes
4. fingerprint those bytes against `lifecycle.last_app_state_save`
5. if identical, log and return
6. write the bytes to disk; on success, update `last_app_state_save`

This is what makes "Cmd+Q after Cmd+Q after Cmd+Q" not amplify disk I/O.

### Sync service state machine (`src/sliding_sync.rs`)

Three new module-level statics live alongside the existing `SYNC_SERVICE`:

- `SYNC_SERVICE_DESIRED_RUNNING: AtomicBool` — what the lifecycle says we want
- `SYNC_SERVICE_ASSUMED_RUNNING: AtomicBool` — what the worker last observed itself doing
- `SYNC_SERVICE_LIFECYCLE_LOCK: LazyLock<tokio::sync::Mutex<()>>` — guards reconciliation

Four new public-ish functions sit immediately after `get_sync_service`:

- `sync_service_desired_running() -> bool` — read the desired flag
- `set_sync_service_desired_running(bool, reason: &'static str)` — set the desired flag and spawn a reconciliation task on Tokio if the runtime is available
- `apply_sync_service_desired_state(reason: &'static str)` — the async reconciliation loop, started under the lifecycle lock, loops until `ASSUMED == DESIRED`
- `stop_sync_service_for_shutdown(timeout: Duration) -> Result<(), Elapsed>` — synchronous stop used by `handle_shutdown`, bounded by a timeout so the OS doesn't kill the process mid-flush

The initial sync startup inside `start_matrix_client_login_and_sync` is rewired: instead of calling `sync_service.start().await` directly and then placing the service into `SYNC_SERVICE`, it places the `Arc<SyncService>` first and then calls `apply_sync_service_desired_state("initial Matrix sync startup").await`. The same pattern applies at the account-switch point in the same function.

The sync-error restart branch in `handle_sync_service_state_subscriber` gains two guards:
- clear `SYNC_SERVICE_ASSUMED_RUNNING` on entry so the state machine knows the service is down
- short-circuit (`continue`) if the lifecycle has marked the service as not-desired, instead of restarting it against the user's intent
- route the restart through `apply_sync_service_desired_state("sync service error restart")` so the lifecycle lock is honored

Two existing cleanup points get the same `ASSUMED_RUNNING.store(false, Ordering::Release)` line:
- the login-loop fallthrough immediately after `SYNC_SERVICE.lock().unwrap().take()`
- the public async `clear_app_state` immediately after the same `take()`

The eight pre-existing `sync_service.start()/stop()` direct calls outside the lifecycle path are left alone. Migrating them is out of scope and would change semantics that have nothing to do with lifecycle.

### Persistence write layering (`src/persistence/app_state.rs`)

`save_app_state` is split into three layers without changing the existing public signature:

- `serialize_app_state(&AppState) -> anyhow::Result<Vec<u8>>` (new public) — pure serialization
- `save_app_state_bytes(&[u8], &UserId) -> anyhow::Result<()>` (new public) — disk write with `create_dir_all` for the user's state directory
- `save_app_state(AppState, OwnedUserId) -> anyhow::Result<()>` (unchanged signature) — delegates to the two helpers

The fingerprint dedup in `persist_runtime_state` needs the first two as separate functions. The third stays because robrix2 has six other call sites that save app state in response to user actions (preference toggles, etc.) and they should continue to compile and work unchanged.

`load_app_state` is **not** modified. Its signature, semantics, and the surrounding `should_restore_loaded_app_state` / `skip_app_state_restore_once` / `take_skip_app_state_restore_once` flow are robrix2-local behavior that must be preserved (see the no-regression section).

### App identity and menu (`.cargo/config.toml` + `script_mod!`)

Two small additions:

- `.cargo/config.toml`: add `MAKEPAD_BUNDLE_NAME = { value = "Robrix", force = true }` next to the existing `MAKEPAD_BUNDLE_IDENTIFIER`. Without this, the macOS application menu shows "MakepadStdinLoop" instead of "Robrix". The existing `MAKEPAD_BUNDLE_IDENTIFIER = "rs.robius.robrix"` line is **not** changed; the identifier stays robrix2-canonical.
- `script_mod!` in `src/app.rs`: add a `WindowMenu` block with a single "Quit Robrix" item bound to Cmd+Q. The Quit item dispatches via the runtime's `request_quit(QuitReason::App)`, which flows back through `handle_lifecycle_event`'s `QuitRequested` arm.

### Logout-error quit (`src/logout/logout_confirm_modal.rs`)

One line changes: when an unrecoverable logout error makes the modal request an immediate app restart, replace the direct `cx.quit()` with `cx.request_quit(QuitReason::App)`. This routes the quit through the lifecycle handler so the state save still happens before exit, matching the rest of the design.

## Robrix2-Local Behavior Preserved

Three categories of local divergence are explicitly preserved:

**Same field, different value.** robrix2's `.cargo/config.toml` already uses `rs.robius.robrix` (not `org.robius`) as the bundle identifier, with the `{ value = ..., force = true }` object syntax established earlier in the codebase. The new `MAKEPAD_BUNDLE_NAME` entry follows that same syntax convention. The identifier value itself is not changed.

**Same function, richer behavior.** robrix2's `should_restore_loaded_app_state` decides whether to emit a restore action based on a broad set of fields — dock state, bot settings, app language, and translation configuration. Replacing it with a narrower dock-only check would silently regress mobile users (where the dock is typically empty but bot/language/translation prefs still need restoring). This function is kept exactly as it is, and the surrounding `handle_load_app_state` body is not rewritten.

**Same data path, extra one-shot.** robrix2's persistence layer carries a `skip_app_state_restore_once` / `take_skip_app_state_restore_once` pair that the explicit-logout flow uses to suppress automatic restore on the next login. This pair has no upstream equivalent and is retained intact. By corollary, `load_app_state` keeps its `anyhow::Result<AppState>` signature — changing it to `Result<Option<AppState>>` would require rewiring the explicit-logout gate and is not done.

**Same action enum, wrapped payload.** robrix2 carries `RestoreAppStateFromPersistentState(Box<AppState>)` (boxed for action-queue size hygiene). The action enum is not touched by this work.

## Commit Plan

Seven commits, each independently buildable, each with a single concern. This shape was chosen so that the highest-risk commit (the Makepad runtime bump) can be reviewed and revert-tested on its own without dragging the rest of the work with it.

1. **`chore(makepad): align Makepad runtime with the Robius ecosystem`** — `Cargo.toml` swap + `cargo update -p makepad-widgets -p makepad-code-editor`. The commit message documents the matching `cargo install` command for the `cargo-makepad` CLI so anyone setting up a build environment gets a tool that matches the runtime.

2. **`chore(env): set MAKEPAD_BUNDLE_NAME for macOS app menu`** — single new entry in `.cargo/config.toml` using the `{ value, force }` syntax already established in that file.

3. **`feat(persistence): split save_app_state for write dedup`** — pure refactor; `serialize_app_state` + `save_app_state_bytes` become public; `save_app_state` is now a one-line delegation; six existing call sites are unaffected.

4. **`feat(sliding_sync): add sync service lifecycle state machine`** — three new statics + four new functions + rewire of the two initial-sync placements + two cleanup `ASSUMED_RUNNING.store(false)` additions + error-restart guards.

5. **`feat(app): add AppLifecycle and lifecycle event handler`** — adds the struct, the fingerprint type, the three new methods, removes the old inline `Event::Shutdown` block, appends the one-line dispatch in `handle_event`.

6. **`feat(menu): add macOS WindowMenu with Quit Robrix item`** — `WindowMenu` DSL block inside `script_mod!`.

7. **`fix(logout): use request_quit for graceful shutdown on unrecoverable logout error`** — one-line change in `logout_confirm_modal.rs`.

## Validation

Each commit must build under `cargo build` on desktop before moving on.

End-to-end manual smoke tests after commit 7, on macOS:

| Scenario | Expected |
|---|---|
| Launch the app | macOS app menu shows the "Robrix" submenu with a "Quit Robrix" item bound to Cmd+Q |
| Change a preference, then Cmd+Q | `latest_app_state.json` is written; subsequent identical Cmd+Q in the same session is logged as a dedup-skipped save |
| Close the main window via the red X | State is written; the sync service is stopped within the 3-second shutdown timeout |
| Hit Ctrl+C in the launching terminal | State is written before the process exits |
| App backgrounded via Cmd+H | Whatever Makepad emits is recorded in the PR description as observed behavior |
| Trigger the logout modal's unrecoverable-error restart path | App requests quit via the new routing, state save still happens |

Android and iOS smoke testing is the reviewer's responsibility and is acknowledged out of scope for the desktop-only validation gate.

## Risks

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| Makepad runtime bump exposes API drift in code outside this PR's footprint | Medium | High — full build break | Commit 1 is isolated; fix forward without entangling other commits; revert is a single-commit revert |
| `cargo-makepad` CLI behavior differs from the previously installed one for Android / iOS builds | Low | Medium | Verified that the aligned branch's Android/iOS tooling is more comprehensive than the prior fork's; team build documentation needs to call out the install command |
| Sync state machine and the eight pre-existing direct `sync_service.start()/stop()` calls flap the `ASSUMED_RUNNING` flag | Low | Low | Out-of-band direct calls are limited to flows (logout, account switch) that already serialize against the lifecycle path; observed flapping would be addressed as a follow-up |
| Removed inline `Event::Shutdown` block silently drops a save | Low | Medium | `handle_shutdown` invokes the same `persist_runtime_state` chokepoint plus the same TSP `block_on_async_with_timeout`; all helper functions (`block_on_async_with_timeout`, `persistent_state_dir`, `app_data_dir`) exist unchanged |
| NDK build tooling regression from leaving the previous fork | None | None | The aligned Makepad branch's NDK and iOS tool code path is a superset of the previous fork's, with richer `target_sdk_version` handling and updated NDK r28 defaults |

## Notes

- This work is the foundation for all subsequent mobile lifecycle behavior in robrix2. Once landed, additional mobile-specific hooks (e.g. notification handoff on background, network-class change on resume) can attach to the same lifecycle dispatch instead of inventing parallel mechanisms.
- The `cargo-makepad` CLI install command that matches the runtime in this PR is `cargo install --force --git https://github.com/makepad/makepad.git --branch dev cargo-makepad`. Build/CI documentation should reference this.
- No unit or integration tests are added. The lifecycle code path is exercised by real OS events, and the project's prevailing testing culture for these surfaces is manual smoke testing per the validation table above.
