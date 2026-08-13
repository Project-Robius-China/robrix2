spec: task
name: "Agent Operations Panel — Client Contract Gate"
inherits: project
tags: [feature, agent-chat, ops, security, contract]
estimate: 3d
---

## Intent

Robrix2 needs an Agent Operations Panel. agent-chat has supplied a development
manifest for `io.agentchat.agent_ops.v1`, but it is not released, is not bound
to an immutable source commit, and its canonical artifact bytes are not
vendored into Robrix2. The client runtime is also not implemented. Keep the
integration explicitly unavailable and fail closed while preserving a
separate, test-only R3 model experiment for historical design review.

## Decisions

### Current Gate Decisions

- The `agent_chat` feature may expose an integration-status screen, but it does
  not expose operational rows or controls while the contract gate is closed.
- Robrix2 does not collect, persist, log, or transmit the agent-chat Dashboard
  token, router bearer, bridge secret, or any equivalent backend credential.
- Robrix2 does not call the existing Dashboard `/api/router/*` endpoints. Those
  endpoints are not a supported Robrix2 client API.
- The canonical contract identifier is `io.agentchat.agent_ops.v1`. Its
  development manifest does not make it consumable. Proposal types and fixtures
  use the Robrix-owned `io.robrix.agent_ops.proposal.r3` schema, are compiled
  only in tests, and are never exported to or consumed by production runtime.
- Contract readiness requires a released manifest, a complete immutable source
  commit, the complete required artifact set, and a byte-for-byte SHA-256 match
  for every artifact compiled into Robrix2.
- Contract readiness does not mean connected. The operational runtime remains
  a separate future task covering bootstrap, authentication, transport, and
  all four views.

### Non-binding Future Contract Proposal

- The following decisions describe review material, not an accepted agent-chat
  contract and not current runtime behavior.
- The historical R3 experiment assumed Matrix as the authenticated control plane for
  session bootstrap, invalidation, revocation, and audit correlation, plus a
  co-resident desktop loopback scoped-HTTP data plane for snapshots and
  sensitive commands. A separately accepted agent-chat ADR must define exact
  owner/scope binding, device and E2EE evidence, sender-constrained proof of
  possession, endpoint identity, replay, expiry, and revocation.
- `snapshot_seq` is not mutation authorization. Actionable entities carry an
  entity version; resources carry `dirty_generation`; each available mutation
  is explicitly authorized by a backend-issued, short-lived, entity-bound
  action capability.
- Each projection is bound to one owner/project/agent scope and carries a
  projection id, stream epoch, auth-fence generation, and scope-local sequence.
  A global panel composes scopes without sharing their capabilities.
- The future projection is backend-owned. Robrix2 will not infer router state,
  actions, blocking chains, or approval eligibility from Matrix events.
- Approval remains exclusively in the existing owner-DM approval flow.
- The future projection carries `resource_id` for workspace actions and a full
  Matrix room/thread reference for navigation. It never carries absolute paths.
- `outcome_unknown` requires a complete begin-inspection then resolve sequence.
  Resolution has exactly three choices: `continue`, `accept_completed`, and
  `keep_blocked`; all require the inspection credential and operator note, and
  only `continue` accepts a required recovery instruction. The backend
  explicitly returns the allowed resolution subset; non-task dispatches only
  receive `continue`.
- Fixtures under `agent-ops-client-v1-proposal` are proposal-only. agent-chat is
  the canonical artifact owner and must publish a released digest manifest and
  the matching artifact bytes before the operational client can consume them.

## Boundaries

### Allowed Changes

- src/agent_ops/**
- src/agent_ops_dummy.rs
- src/app.rs
- src/home/home_screen.rs
- src/home/navigation_tab_bar.rs
- src/i18n.rs
- src/persistence/app_state.rs
- src/settings/app_settings.rs
- src/lib.rs
- resources/i18n/en.json
- resources/i18n/zh-CN.json
- docs/README.md
- docs/agent-system-planes.md
- docs/robrix-with-agentchat/README.md
- docs/robrix-with-agentchat/agent-ops-client-contract-v1-proposal.md
- specs/fixtures/agent-ops-client-v1/**
- specs/fixtures/agent-ops-client-v1-proposal/**
- specs/task-agent-ops-panel.spec.md

### Forbidden

- Do not embed or persist agent-chat backend credentials
- Do not call the local Dashboard router endpoints from Robrix2
- Do not present operational rows or mutations while the contract is not ready
- Do not add approve or deny controls
- Do not derive session, task, dispatch, lease, queue, or action state locally
- Do not expose absolute paths, secrets, or approval content
- Do not use Makepad 1.x syntax or hardcoded screen colors

## Completion Criteria

Scenario: The contract gate is fail closed
  Test: contract_gate_is_fail_closed
  Given the vendored agent-chat client contract is not released and verified
  When the Agent Operations integration is opened
  Then it reports that the contract is unavailable
  And it cannot become operational from local configuration

Scenario: Manifest claims cannot substitute for canonical bytes
  Test: manifest_only_release_cannot_open_without_compiled_artifacts
  Given a released manifest with a complete source commit and artifact list
  When no matching artifact bytes are compiled into Robrix2
  Then the contract remains unavailable

Scenario: Canonical artifacts are verified byte for byte
  Test: artifact_set_and_digest_mismatches_stay_closed
  Given a released manifest and its compiled artifact set
  When a path is absent or any artifact body is changed
  Then the contract remains unavailable with a named verification failure

Scenario: The panel has no Dashboard transport
  Test: panel_contains_no_backend_transport_or_credentials
  Given the contract gate is closed
  When Agent Operations and its settings request entry points are inspected
  Then they contain no HTTP request path or authorization header
  And it never polls the Dashboard

Scenario: Proposal types are test-only and not runtime integration types
  Test: proposal_model_is_not_wired_into_runtime_panel
  Given the local proposal model and fixtures compile only under cfg(test)
  When the contract-gated panel runtime is inspected
  Then it does not consume Snapshot, Invalidation, or command proposal types
  And no proposal fixture can make the panel operational

Scenario: Preferences collect no backend secret
  Test: settings_contains_no_agent_ops_secret_input
  Given a build with the agent_chat feature enabled
  When Preferences renders the integration section
  Then it contains no Dashboard token input
  And no Agent Operations credential is added to AppState

Scenario: Legacy prototype credentials are removed on restore
  Test: legacy_agent_ops_credentials_are_removed_from_disk_before_restore
  Given an old AppState containing the experimental Dashboard configuration
  When that state is loaded
  Then the obsolete agent_ops field is absent
  And the old token is removed from the persisted JSON before restore

Scenario: Credential cleanup failure stops restoration
  Test: credential_scrub_write_failure_aborts_restore
  Given persisted state contains an obsolete Agent Operations credential
  When sanitized state cannot be written back
  Then restoration returns an error instead of continuing with secret-bearing state

Scenario: Disabling agent-chat cannot restore the status page
  Test: closing_settings_never_returns_to_disabled_agent_ops
  Given Settings was opened from Agent Operations
  When agent-chat is disabled and Settings closes
  Then Home is selected instead of the disabled Agent Operations page

Scenario: Selecting a room always reveals the room workspace
  Test: opening_room_leaves_every_overlay_page_for_home
  Given Add Room, Settings, Directory, or Agent Operations is the active page
  When a room selection action is handled
  Then Home is selected before the desktop Dock handles the room
  And Home or a selected Space remains unchanged because it already shows the room workspace

Scenario: Contract status uses stable semantic presentation
  Test: contract_status_presentation_covers_tone_and_next_step
  Given every Agent Operations contract availability state
  When the panel presentation is derived
  Then ready is success and points to runtime construction
  And pending release states are warning
  And invalid or mismatched contract material is danger

Scenario: Agent Operations diagnostics are localized
  Test: agent_ops_status_and_detail_keys_exist_in_all_locales
  Given English and Simplified Chinese resources
  When every contract state and next-step message is resolved
  Then both locales provide native text without English fallback

Scenario: Proposal fixtures remain explicitly versioned
  Test: proposal_snapshot_fixture_matches_the_proposed_model
  Given the non-canonical R3 snapshot fixture
  When the fixture is parsed
  Then its schema is io.robrix.agent_ops.proposal.r3
  And it identifies its owner/project/agent scope and projection stream

Scenario: Snapshot actions bind entity-specific concurrency state
  Test: proposal_snapshot_fixture_matches_the_proposed_model
  Given an actionable dispatch and a dirty resource in the proposed snapshot
  When their available actions are parsed
  Then each action carries entity_version and an opaque expiring capability
  And the resource action also carries dirty_generation

Scenario: Outcome inspection issues a scoped resolution capability
  Test: proposal_inspection_fixture_carries_explicit_resolution_grants
  Given a successful outcome inspection response
  When the proposed fixture is parsed
  Then it carries inspection identity, expiry, and resource dirty_generation
  And it provides a dispatch-bound resolve_outcome capability with explicit allowed resolutions

Scenario: Outcome resolution covers all three choices
  Test: resolution_fixtures_cover_the_complete_outcome_closure
  Given an outcome_unknown dispatch has been inspected
  When the three proposed resolution fixtures are parsed
  Then continue includes a recovery instruction
  And accept_completed and keep_blocked cannot carry one

Scenario: Terminal resolution rejects recovery instructions on the wire
  Test: terminal_resolution_rejects_recovery_instruction
  Given an accept_completed request contains recovery_instruction
  When the strict proposed command model parses it
  Then the complete command is rejected

Scenario: Wrong schema is rejected during trusted-model construction
  Test: wrong_schema_is_rejected_during_deserialization
  Given a projection uses an unaccepted schema identifier
  When the proposed model parses it
  Then parsing fails before any caller can use the data

Scenario: Capability values are not exposed through Debug
  Test: capability_debug_output_is_redacted_and_empty_values_are_rejected
  Given a parsed action or inspection capability
  When the value is debug formatted
  Then the secret value is absent
  And a redacted marker is present

Scenario: All proposed presentation text rejects paths
  Test: complete_projection_rejects_embedded_filesystem_paths
  Given an absolute path embedded in any display string
  When the proposed model parses it
  Then the entire projection is rejected

Scenario: The panel has no second approval entry point
  Test: panel_contains_no_backend_transport_or_credentials
  Given the integration status screen
  When its controls are inspected
  Then no approve or deny operation exists

Scenario: The panel is absent without the agent_chat feature
  Test: test_panel_absent_without_agent_chat_feature
  Given a build with the agent_chat feature disabled
  When the app starts
  Then no Agent Operations entry appears in Preferences

## Out of Scope

- Consuming live snapshots or invalidations
- Sending Agent Operations commands
- Rendering Attention, Tasks, Queue, or Worktrees data
- Adding a new agent-chat authentication mechanism
- Changing agent-chat source code from this repository
