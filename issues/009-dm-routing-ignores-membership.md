# DM Routing Ignores Membership

## Summary

After leaving a DM with user X, clicking X again in the People tab does **not** prompt
"Create DM" — the client navigates silently into the dead (left) DM room. Messages
typed there return `403 M_FORBIDDEN`. Palpo logs confirm: only a `leave` event exists
for that room, no new `join`.

This is the **primary** defect tracked under issue #98 (rewritten). The downstream
symptom (composer accepts input in the dead room and sends a 403) is a follow-up
defense-in-depth gap, not addressed by this fix.

## Symptoms

- Leave a DM with user X via room context menu or `/leave`.
- Open People, search X, click their row → click the direct-message button.
- Observed: client opens the old, empty, unjoined room. No "Create DM" confirmation
  modal appears.
- Type and send → red banner: `Failed to send message: ... 403 M_FORBIDDEN
  "sender's membership is not 'join'"`.
- Palpo event log: only the prior `leave`, no subsequent `join`.

## Root Cause

`MatrixRequest::OpenOrCreateDirectMessage` at `src/sliding_sync.rs:2082` resolves a
DM via `client.get_dm_room(user_id)`. That helper reads the Matrix `m.direct`
account-data mapping and **does not filter by local membership**. Per Matrix spec,
leaving a DM does not remove its entry from `m.direct`, so the mapping keeps
pointing at the abandoned room.

Flow (steps 4–6 are the bug):

1. User creates DM with @X → `m.direct[@X] = [!old]`.
2. User leaves `!old` → server accepts; `m.direct` is unchanged.
3. User clicks @X in People → `direct_message_button` (`profile/user_profile.rs:466`)
   fires `OpenOrCreateDirectMessage { allow_create: false }`.
4. `client.get_dm_room(@X)` returns `!old` (state = `Left`), not `None`.
5. Handler emits `DirectMessageRoomAction::FoundExisting` → `navigate_to_room(!old)`.
6. `DidNotExist` branch (which would open the "Create DM" confirmation modal) is
   never reached.
7. Any send in the dead room 403s.

## Code References

- `src/sliding_sync.rs:2079-2092` — `OpenOrCreateDirectMessage` handler.
- `src/profile/user_profile.rs:466-483` — `direct_message_button` click handler.
- `src/app.rs:1388-1442` — `DirectMessageRoomAction` dispatch.

## Fix Applied

Filter `client.get_dm_room()` by local membership via a pure predicate
`is_active_dm_room_state`. Treat a DM whose state is not `Joined` as "no existing
DM" so the handler falls through to `DidNotExist`, which opens the existing
"Create DM" confirmation modal. Confirming creates a fresh DM and invites the
peer, who (for appservice peers) auto-accepts.

```rust
fn is_active_dm_room_state(state: RoomState) -> bool {
    state == RoomState::Joined
}

let existing = client.get_dm_room(&user_profile.user_id)
    .filter(|r| is_active_dm_room_state(r.state()));
```

### Why `Joined` only

Upstream `client.get_dm_room()` delegates to `joined_rooms()`, which filters
strictly by `RoomStateFilter::JOINED` (matrix-sdk `client/mod.rs:1288`). In steady
state only `Joined` rooms reach the filter. The predicate still acts as:

1. A defense against the race between `leave()` returning 200 and the local
   membership transition being applied by sliding sync (during that window the
   cached state can still read `Joined`, but the check remains cheap and explicit).
2. A guard against any future relaxation of `get_dm_room()`'s upstream filter.

## Out of Scope (follow-ups)

- Composer gating on membership in any already-open left/banned room.
- Auto-closing dock tabs on local `/leave` (issue #8).
- Auditing non-message send paths (reactions/edits/redacts/typing/receipts).
- Cleaning stale `m.direct` entries server-side after a leave.

## Environment

- Branch cut from `main @ 55e39037`; reproduces on `main`.
- Homeserver: local palpo via testenv.
- Peer: appservice user (auto-accepts invites).
