# Issue 012: Room aliases can be read but not managed from the client

**Date:** 2026-07-23
**Type:** Feature
**Status:** Open
**Driven by spec:** `specs/task-room-aliases.spec.md` (agent-spec, estimate 2d)
**Affected components:** `src/home/room_settings_modal.rs`, `src/sliding_sync.rs`, `src/app.rs`, `src/i18n.rs`, `resources/i18n/**`

## Summary

Robrix already **reads** room aliases end-to-end but offers **no way to manage them**.
`RoomsList` populates `canonical_alias: Option<OwnedRoomAliasId>` and
`alt_aliases: Vec<OwnedRoomAliasId>` from matrix-sdk (`room.canonical_alias()` /
`room.alt_aliases()`), and the Room Settings modal is even handed the canonical alias
today — but only as a read-only string. Users cannot publish a new alias, remove one,
or choose which alias is canonical.

This issue adds a **"Room Aliases" section to the Room Settings modal** so users with the
right power level can view and manage a room's aliases.

## Current behavior (evidence)

- `src/home/rooms_list.rs:307-309` / `:389-391` — `canonical_alias` + `alt_aliases` are
  already tracked in the room model.
- `src/sliding_sync.rs:7378-7384` / `:7487-7491` — both are read from
  `room.canonical_alias()` / `room.alt_aliases()`.
- `src/app.rs:1927-1931` — on `RoomSettingsAction::Open`, the canonical alias is fetched
  (`get_room_canonical_alias`) and passed into `show_settings(...)` as a plain string.
- `src/home/room_settings_modal.rs` — renders room settings but has **no alias
  add/remove/set-canonical controls**.
- Joining a room *by* alias already works (`src/home/add_room.rs::parse_address`); this
  issue is strictly about **managing** an existing room's aliases, not joining.

## Proposed scope

1. **View**: show canonical alias + all alt aliases in the Room Settings modal.
2. **Publish**: register a new alias (`#localpart:server`, or bare `localpart` resolved
   against the current homeserver) into the room directory.
3. **Remove**: unbind an alias from the room directory.
4. **Set canonical**: write the `m.room.canonical_alias` state event (alias + alt_aliases),
   or clear it.
5. **Permission-gated**: edit controls appear only when the user can send the
   `m.room.canonical_alias` state event; otherwise the section is read-only.
6. **Pure, unit-testable core**: alias normalization/validation
   (`normalize_and_validate_alias`) and canonical/alt reconciliation
   (`reconcile_canonical_alias`), plus i18n key presence — all covered by the spec's
   Completion Criteria scenarios.

## Out of scope

- Join-by-alias flow (already covered by `add_room.rs`).
- Space canonical alias management (`space_service_sync.rs`).
- Directory visibility / history-visibility toggles.
- Cross-homeserver alias migration or bulk import.

## Development contract

Development is driven by the agent-spec Task Contract at
`specs/task-room-aliases.spec.md`. Verify with:

```
agent-spec lint   specs/task-room-aliases.spec.md
agent-spec verify specs/task-room-aliases.spec.md
```

Acceptance is bound to pure-function unit tests (alias validation + canonical
reconciliation + i18n key presence); platform behavior (macOS/Android) is confirmed
manually in the implementation PR.
