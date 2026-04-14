# Issue #009: Mobile — App Service binding is lost after force-quit + relaunch

**Date:** 2026-04-14
**Severity:** High (blocks any practical mobile usage of the App Service / BotFather feature)
**Status:** Fix staged — pending Android manual verification
**Affected component:** `src/sliding_sync.rs` (`handle_load_app_state`), mobile platforms (verified Android, iOS likely same)

## Summary
On Android (and likely iOS), after the user fills the App Service settings (BotFather User ID, Octos Service URL) and clicks Save, the binding works during the current app session. However, once the user force-quits robrix2 and relaunches it, the App Service settings page comes up empty — both fields are blank and the Octos Service connection shows "Unreachable". The bot binding is not persisted across app restarts on mobile.

## Symptoms
- Open robrix2 on Android emulator or device
- Navigate to Settings → Labs → App Service
- Toggle "Enabled" on, fill BotFather User ID (e.g. `@octosbot:192.168.5.12:8128`) and Octos Service (e.g. `http://192.168.5.12:8010`), click **Save** — saves with success popup, "Check Now" shows Reachable ✅
- Force-quit robrix2 (swipe from recent apps / kill the process)
- Relaunch robrix2, go back to the same settings page
- Both fields are empty; Check Now shows Unreachable ❌

## Root Cause (hypothesis, needs verification)
`bot_settings.rs` currently calls `persist_bot_settings(app_state)` → `persistence::save_app_state(...)` after Save (line 331 / 427 / 578-581 in `src/settings/bot_settings.rs`). So the persistence CODE is wired. Candidate failure points that a fixer should audit:

1. **App-state hydrate on startup doesn't include App Service fields.** `load_app_state` may succeed but the App Service subsection is silently dropped (missing serde field, or a default that overwrites on deserialization).
2. **`app_data_dir()` on Android resolves to a path that doesn't survive app restart.** Android apps have multiple storage locations; cache and some internal dirs can be wiped by OS. If the persistence file ends up under `cacheDir` instead of `filesDir`, the OS can reclaim it at any time.
3. **App Service state is stored separately from the rest of `AppState` and only the main branch is loaded on startup.** Mobile code path may bypass the App Service load.
4. **Permission / path issue**: on Android, the app may succeed `save_app_state` to the Rust-side path but that path is inside a container the next process can't read (multi-process / scoped storage).

Desktop (macOS/Linux/Windows) likely works because `app_data_dir()` there resolves to a user-writable persistent location. Android/iOS have more constrained storage layers.

## Reproduction
1. Start local palpo + octos backend (see issue #005 and [clipboard pitfall doc from 2026-04-14] for correct setup)
2. Build & install robrix2 on Android emulator: `cargo makepad android run -p robrix --release`
3. Register/login; go to Settings → Labs → App Service
4. Enable and fill both fields; click Save
5. Verify "Check Now" reports Reachable
6. `adb shell am force-stop rs.robius.robrix` (or swipe-kill from recent apps)
7. Relaunch robrix2
8. Navigate back to App Service settings — observe empty fields

## Fix Applied

**Root cause confirmed**: `src/sliding_sync.rs::handle_load_app_state` gated the entire `RestoreAppStateFromPersistentState` dispatch behind a non-empty dock-state check:

```rust
if !app_state.saved_dock_state_home.open_rooms.is_empty()
    && !app_state.saved_dock_state_home.dock_items.is_empty()
{
    Cx::post_action(AppStateAction::RestoreAppStateFromPersistentState(Box::new(app_state)));
}
```

Mobile has no dock, so every relaunch silently dropped the loaded `bot_settings` (plus `app_language` and `translation` config). Desktop masked the bug because dock state is almost always non-empty after first run. The save path itself was always correct.

**Fix**: unconditionally dispatch `RestoreAppStateFromPersistentState` whenever `load_app_state` succeeds. The restore match arm in `src/app.rs:1071-1095` already performs a full `AppState` replacement and dispatches `LoadDockFromAppState` — empty-dock is safely handled downstream. Log and popup messages inside `handle_load_app_state` were also reworded away from "dock layout" language to reflect the broader scope.

**Regression guard**: `src/app.rs` unit test `test_app_state_roundtrip_preserves_bot_settings_with_empty_dock` pins the serde contract so any future `#[serde(skip)]` on `bot_settings` (or a breaking field rename) is caught at `cargo test` time instead of at Android runtime.

**Spec + Plan**:
- Contract: `specs/task-fix-mobile-appservice-persistence.spec.md` (agent-spec Task Contract, quality 93%, lifecycle 8/8 pass)
- Plan: `docs/superpowers/plans/2026-04-14-fix-mobile-appservice-persistence.md`

## Remaining Issues
1. Audit `persistence::save_app_state` and `load_app_state` for how `AppState.app_service` (or equivalent field) is (de)serialized
2. Verify `app_data_dir()` resolution on Android — must be a persistent location (filesDir equivalent), not cacheDir
3. Add an integration/smoke test that exercises Save → reload → assert fields restored (at minimum a desktop test covers the serde layer)
4. Check iOS after Android fix — same persistence abstraction likely means same fix covers both
5. Consider surfacing a tiny "Last saved: <timestamp>" label in the settings page so future regressions are user-visible

## Files Likely Involved
- `src/settings/bot_settings.rs` — Save path (calls `persist_bot_settings`)
- `src/persistence/*` — `save_app_state` / `load_app_state` implementation
- `src/app.rs` — where `load_app_state` is called on startup and fields are restored to `AppState`
- `src/sliding_sync.rs` — where `app_data_dir()` is resolved per platform

## Test Verification
| Before fix | After fix |
|---|---|
| Mobile: App Service binding cleared after force-quit + relaunch | Mobile: binding restored on relaunch; Check Now succeeds without re-entering fields |

## Related
- Blocking real-world mobile testing of PR [octos-org/octos#345](https://github.com/octos-org/octos/pull/345) (bidirectional Matrix media + bot routing) — every restart forces a re-bind, which makes iterative testing painful
