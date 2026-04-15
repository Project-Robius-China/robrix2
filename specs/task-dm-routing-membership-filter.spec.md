spec: task
name: "DM Routing Membership Filter — Skip Stale m.direct Mappings"
inherits: project
tags: [bugfix, dm, sliding-sync, membership]
estimate: 0.5d
---

## Intent

Fix the bug where leaving a DM with user X and then clicking X again in the People
tab silently navigates into the abandoned (left) room instead of prompting "Create
DM". The root cause is that
`MatrixRequest::OpenOrCreateDirectMessage` in `src/sliding_sync.rs` consults
`client.get_dm_room(user_id)`, which reads the Matrix `m.direct` account-data
mapping without checking local membership. Since leaving a DM does not remove its
entry from `m.direct`, the stale mapping points at the dead room and the user lands
in a room where every send is rejected with 403 M_FORBIDDEN.

## Constraints

- Keep the public shape of `MatrixRequest::OpenOrCreateDirectMessage` unchanged
  (same fields: `user_profile`, `allow_create`, `create_encrypted`)
- Keep the `DirectMessageRoomAction` enum variants unchanged
  (`FoundExisting`, `DidNotExist`, `NewlyCreated`, `FailedToCreate`)
- Do not modify the `direct_message_button` click logic in
  `src/profile/user_profile.rs`
- Do not modify `client.get_dm_room()` or any matrix-sdk behavior
- Do not introduce new dependencies
- Do not change any UI / Makepad DSL files

## Decisions

- Filter the result of `client.get_dm_room(user_id)` by local membership before
  emitting `FoundExisting`; the change is confined to the
  `MatrixRequest::OpenOrCreateDirectMessage` arm of `src/sliding_sync.rs`
- A DM room is treated as "active" (reusable) only when its `room.state()` is
  `RoomState::Joined`. All other states (`Left`, `Banned`, `Invited`, `Knocked`)
  are treated as "no existing DM"
- The membership check is a guard for the race between the leave HTTP response
  (200) and the local membership transition being applied by sliding sync; without
  it, a stale `Joined`-cached room can surface here during that window
- Upstream `client.get_dm_room()` already delegates to `joined_rooms()` which
  filters strictly by `RoomStateFilter::JOINED` (matrix-sdk
  `client/mod.rs:1288`), so in steady state only `Joined` rooms reach the filter;
  the predicate is still kept as explicit, testable defense in depth
- The fall-through is driven by `allow_create`: with `allow_create == false` the
  handler emits `DidNotExist` (opens the "Create DM" confirmation modal); with
  `allow_create == true` it proceeds to create a fresh DM
- The classification is implemented as a pure free function
  `fn is_active_dm_room_state(state: RoomState) -> bool` so it is unit-testable
  without spinning up a `Client`

## Boundaries

### Allowed Changes
- `src/sliding_sync.rs`
- `specs/task-dm-routing-membership-filter.spec.md`
- `issues/009-dm-routing-ignores-membership.md`

### Forbidden
- Do not modify `client.get_dm_room()`, `m.direct` account data, or matrix-sdk
- Do not modify any other `MatrixRequest::*` handler
- Do not modify `DirectMessageRoomAction` or its consumers in `src/app.rs`
- Do not modify the composer / send path / 403 error handling (separate follow-up)
- Do not modify dock tab cleanup on leave (separate, tracked as #8)
- Do not run `cargo fmt`
- Do not change Makepad DSL files

## Acceptance Criteria

Scenario: Leaving a DM and re-clicking the user opens the Create DM modal
  Test: manual_test_dm_routing_left_room_prompts_create
  Given the user has previously created a DM with peer X
  And the user has left that DM (room state is `Left`)
  When the user opens People, selects X, and clicks the direct-message button
  Then the client does not navigate to the abandoned room
  And the "Create New Direct Message" confirmation modal is shown
  And confirming the modal results in a fresh DM with peer X being created on the homeserver

Scenario: Active DM still resolves directly without a confirmation modal
  Test: manual_test_dm_routing_active_dm_navigates_directly
  Given the user has an active DM with peer Y (room state is `Joined`)
  When the user opens People, selects Y, and clicks the direct-message button
  Then the client navigates straight to the existing DM room
  And no "Create New Direct Message" confirmation modal is shown

Scenario: Banned-from DM is treated the same as a left DM
  Test: manual_test_dm_routing_banned_room_prompts_create
  Given the user was banned from a previous DM with peer Z (room state is `Banned`)
  When the user opens People, selects Z, and clicks the direct-message button
  Then the client does not navigate to the banned room
  And the "Create New Direct Message" confirmation modal is shown

Scenario: Classification predicate accepts Joined and rejects all other states
  Test: is_active_dm_room_state_only_joined_is_reusable
  Given the `is_active_dm_room_state` predicate
  When called with each `RoomState` variant
  Then it returns `true` for `RoomState::Joined`
  And it returns `false` for `RoomState::Invited`
  And it returns `false` for `RoomState::Left`
  And it returns `false` for `RoomState::Banned`
  And it returns `false` for `RoomState::Knocked`

Scenario: Sending in the freshly created DM succeeds with no 403
  Test: manual_test_dm_routing_new_dm_send_succeeds
  Given the user previously left a DM with peer X
  And the user has just confirmed "Create DM" in the modal for peer X again
  And the homeserver has created a new room and X has accepted the invite
  When the user types a message in the new DM and sends it
  Then no 403 M_FORBIDDEN error is shown
  And the homeserver records the message under the new room id, not the abandoned one

Scenario: When no DM mapping exists at all, behavior is unchanged
  Test: manual_test_dm_routing_no_existing_mapping_unchanged
  Given the user has never had a DM with peer V
  When the user opens People, selects V, and clicks the direct-message button
  Then the "Create New Direct Message" confirmation modal is shown
  And confirming the modal creates a new DM with V

Scenario: With allow_create true, a stale DM mapping triggers immediate creation without modal
  Test: manual_test_dm_routing_allow_create_skips_stale_room
  Level: manual
  Targets: src/sliding_sync.rs MatrixRequest::OpenOrCreateDirectMessage
  Given the user has previously left a DM with peer X (room state is `Left`)
  And a code path issues `OpenOrCreateDirectMessage` with `allow_create = true` for X
  When the request is processed
  Then the handler does not emit `FoundExisting` for the abandoned room
  And the handler proceeds to create a new DM with X
  And `DirectMessageRoomAction::NewlyCreated` is emitted on success

## Out of Scope

- Composer gating on membership inside an already-open left/banned room
- Auto-closing dock tabs on local `/leave` (issue #8)
- Auditing non-message send paths (reactions, edits, redacts, typing, receipts)
- Server-side cleanup of stale `m.direct` entries
- Any change to `client.get_dm_room()` upstream behavior
