spec: task
name: "Agent-to-App Agent2View Runtime — scoped sessions, local reducers, and mission producer contract"
inherits: project
tags: [agent-to-app, agent2view, mission-room, runtime, octos, producer]
depends: [task-agent-to-app-system, task-agent-to-app-template-runtime]
estimate: 2d
---

## Intent

Add the first Robrix `agent2view` runtime layer on top of the existing static
`org.octos.app` renderer. Matrix event history remains authoritative, while
Robrix keeps scoped in-memory `AgentViewSession` state for message, room, and
account app instances. The first product target is an OctOS/OpenClaw mission
room: agents emit structured Matrix snapshots, and humans use existing OctOS
action responses for shared decisions.

## Decisions

- `AgentViewScopeKey` has exactly three identities: `message = room_id + event_id`,
  `room = room_id + app_id`, and `account = account_id + app_id`.
- `scope` defaults to `message`; `room` and `account` scopes require non-empty,
  trimmed `app_id`.
- `mission_room` must use `scope: "room"`; `mission_dashboard` must use
  `scope: "account"`.
- `RoomScreen` owns an `AgentViewRuntime` and binds successfully rendered
  `org.octos.app` envelopes into that runtime before showing the Splash card.
- Local reducer v1 accepts JSON Pointer `replace` and `remove`; `append` and
  richer list operations are explicitly unsupported.
- Local reducers mutate only Robrix view session state and set `dirty`; they do
  not rewrite Matrix events.
- Shared mission actions are transported by existing `org.octos.actions` /
  `org.octos.action_response`, not by hidden Splash-local state.
- OctOS/OpenClaw producers emit full mission snapshots as normal Matrix messages
  with useful `body` fallback text and original-content `org.octos.app`.

## Boundaries

### Allowed Changes
- src/home/app_registry/**
- src/home/room_screen.rs
- docs/design/agent-to-app-simplified-design.md
- docs/design/agent-mission-room-design.md
- specs/task-agent-to-app-agent2view-runtime.spec.md

### Forbidden
- Do not change the `org.octos.app` envelope field names.
- Do not add dynamic Matrix-supplied Splash templates.
- Do not make `m.replace` edits authoritative for app state.
- Do not add new Cargo dependencies.
- Do not run `cargo fmt` or `rustfmt`.

### Out of Scope
- Persistent account-level app state.
- Remote plugin or marketplace capability loading.
- Full in-card shared-action transport.
- LLM-generated templates or template repair loops.

## Acceptance Criteria

Scenario: Message scope remains isolated by event id
  Test: scope_key_uses_event_id_for_message_scope
  Given an app envelope omits `scope`
  When Robrix builds its `AgentViewScopeKey`
  Then the key is `Message`
  And the key contains `room_id + event_id`
  And no `app_id` is required

Scenario: Room scope requires a stable app id
  Test: parse_envelope_rejects_room_scope_without_app_id
  Given an app envelope declares `scope: "room"`
  When `app_id` is absent or blank
  Then Robrix rejects the envelope
  And the timeline falls back to Matrix `body`

Scenario: Mission room cannot be sent as message scope
  Test: parse_envelope_rejects_mission_room_without_room_scope
  Given an app envelope declares `type: "mission_room"`
  When the envelope omits the required room scope
  Then Robrix rejects the envelope
  And no shared mission session is created

Scenario: Account dashboard binds by current account and app id
  Test: agent_view_scope_key_for_account_scope_uses_current_account
  Given an app envelope declares `type: "mission_dashboard"`
  And the envelope declares `scope: "account"`
  And the envelope includes `app_id: "missions.global"`
  When RoomScreen computes the scope key
  Then the key is `Account`
  And the key contains `account_id + app_id`

Scenario: RoomScreen registers rendered apps in AgentViewRuntime
  Test: agent_view_scope_key_for_room_scope_uses_app_id
  Given a Matrix room id is available from the active `TimelineKind`
  Given a valid `mission_room` event renders through SplashHost
  When RoomScreen renders the message item
  Then it computes a room-scoped key from `room_id + app_id`
  And it binds an `AgentViewSession` before showing the Splash card

Scenario: Local reducer replace mutates session state
  Test: reducer_replace_updates_state_and_marks_dirty
  Given an existing `AgentViewSession`
  When the local reducer applies a JSON Pointer `replace`
  Then the session state changes
  And `dirty` is set to true
  And Matrix event content is not modified

Scenario: Local reducer remove can be a no-op
  Test: reducer_remove_absent_key_is_noop_without_dirty
  Given an existing `AgentViewSession`
  When the local reducer removes an absent JSON Pointer key
  Then the reducer returns no change
  And `dirty` remains false

Scenario: Unsupported append is rejected without mutation
  Test: reducer_append_is_rejected_without_mutating
  Given an existing `AgentViewSession`
  When the local reducer receives an `append` operation
  Then it returns `UpdateOpNotYetSupported`
  And the session state is unchanged
  And `dirty` remains false

Scenario: Mission room producer payload renders with action context
  Test: raw_matrix_mission_room_event_renders_to_splash
  Given an OctOS/OpenClaw producer emits a Matrix message with `type: "mission_room"`
  And the message includes `scope: "room"` and `app_id: "mission.main"`
  And the mission state includes a task priority and pending human action
  When Robrix renders the event
  Then the Splash output contains the mission title, task title, priority, and pending action text
  And no unresolved `$state.` binding remains

Scenario: Invalid mission state does not render Splash
  Test: mission_room_invalid_task_status_falls_back_to_body
  Level: unit
  Targets: app envelope validation, plain-text fallback boundary
  Given an OctOS/OpenClaw producer emits `mission_room` state with an unknown task status
  When Robrix validates the envelope
  Then rendering returns `None`
  And RoomScreen uses the plain-text message fallback path

## Out of Scope

- Synchronizing local reducer state back to Matrix automatically.
- Conflict resolution between multiple room-scoped mission snapshot events.
- Persisting `AgentViewRuntime` across app restarts.
- Adding clickable Splash-native mission controls.
