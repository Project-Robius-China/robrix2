spec: task
name: "Agent-chat Owner-scoped Matrix UI Approval"
inherits: project
tags: [agent-chat, matrix, approval, security, ui]
estimate: 2d
---

## Intent

Render agent-chat's encrypted owner approval requests as native Robrix2 cards and
send button decisions as structured Matrix events. Robrix2 is a presentation
client only: agent-chat remains the sole authorization source and validates the
authenticated Matrix event sender, approval room, owner binding, digest,
expiration, and one-shot state before releasing a coding-agent operation.

## Decisions

- Consume `com.agentchat.approval.request.v1` only from the original Matrix event
  content; edits cannot replace security-sensitive request fields.
- Render `com.agentchat.approval.status.v1` as read-only fallback text with no
  action buttons.
- Render request actions from `com.agentchat.approval.actions` inside the shared
  warning-state approval card.
- Emit `com.agentchat.approval.verdict.v1` with `approve_once` or `deny`; never
  translate a text reply, slash command, or public-room control command into a
  verdict.
- Preserve `agent`, `project`, `project_room_id`, `request_id`, and
  `input_digest` exactly from request to verdict.
- Do not infer approval authority locally. A visible button is a UI affordance;
  agent-chat revalidates `event.sender` and all server-side bindings.
- Disable approval actions at `expires_at` and reject a stale click locally;
  schedule a redraw when the deadline arrives so a visible card changes state
  without unrelated UI activity. Agent-chat still performs the authoritative
  expiry check.
- Treat agent-chat verdicts as a distinct protocol path: do not add Octos
  routing metadata and do not invite the already-present bridge user based on a
  lazy-loaded membership cache.
- Explicitly query the bridge user's current Matrix device keys, then rotate the
  encrypted room's outbound Megolm session immediately before an agent-chat
  verdict. This re-shares a fresh key with the bridge's current device and
  prevents an approval click from becoming undecryptable when the bridge device
  was registered after an older room session was established.
- Keep the existing `org.octos.approval_request` protocol backward compatible.

## Boundaries

### Allowed Changes
- specs/task-agent-chat-owner-approval.spec.md
- src/shared/approval_card.rs
- src/shared/mod.rs
- src/home/room_screen.rs
- src/sliding_sync.rs
- resources/i18n/en.json
- resources/i18n/zh-CN.json

### Forbidden
- Do not add text-command approval paths
- Do not make Robrix2 an authorization source
- Do not trust display names or payload-declared approver identities
- Do not render buttons for public `com.agentchat.approval.status.v1` events
- Do not accept edited approval bindings from `m.replace` / `m.new_content`
- Do not add cargo dependencies

## Out of Scope

- Approval history browser
- Multi-party or threshold approval
- Runtime permission policy and Matrix owner binding, which remain in agent-chat

## Completion Criteria

Scenario: Public project-room status is read-only
  Test: test_agentchat_public_status_has_no_actions
  Given a timeline event has msgtype `com.agentchat.approval.status.v1`
  When Robrix2 renders the event
  Then it displays the fallback body
  And it renders no approval buttons

Scenario: Encrypted owner request renders native actions
  Test: test_parse_agentchat_owner_approval_request
  Given an original timeline event contains a valid `com.agentchat.approval.request.v1` payload
  When Robrix2 renders the event
  Then it displays a pending approval card
  And it displays exactly `approve_once` and `deny` actions

Scenario: Agent-chat approval events enter the Matrix UI timeline
  Test: agent_chat_approval_message_types_bypass_the_default_timeline_filter
  Given Matrix SDK's default timeline filter rejects custom room message types
  When an event uses an agent-chat request, status, or verdict msgtype
  Then Robrix2 adds that event to the timeline for rendering
  And unrelated custom message types remain filtered out

Scenario: Malformed request fails closed
  Test: test_malformed_agentchat_owner_approval_request_hides_buttons
  Given an agent-chat approval request omits a binding field or has an invalid digest
  When Robrix2 renders the event
  Then no approval buttons are rendered

Scenario: Expired approval actions are disabled
  Test: test_agentchat_approval_buttons_expire_at_deadline
  Given a valid agent-chat approval request reaches `expires_at`
  When Robrix2 renders or receives a click for that card
  Then it labels the request as expired
  And it emits no verdict event

Scenario: Visible approval cards redraw at their earliest deadline
  Test: test_approval_expiry_timer_uses_earliest_visible_deadline
  Given one or more visible pending approval cards
  When Robrix2 schedules the approval expiry timer
  Then it uses the earliest valid deadline
  And non-approval action buttons do not create an expiry timer

Scenario: Approval decision preserves the server binding
  Test: test_build_agentchat_approval_verdict
  Given a valid owner approval request
  When the user clicks Approve once
  Then Robrix2 sends `com.agentchat.approval.verdict.v1` to the same room
  And the verdict preserves agent, project, project room, request id, and input digest
  And the verdict action is `approve_once`

Scenario: Approval verdict refreshes encrypted device access
  Test: agent_chat_approval_verdict_rotates_the_outbound_room_key
  Given an encrypted owner approval room has an existing outbound Megolm session
  When Robrix2 sends an agent-chat approval verdict
  Then it explicitly queries the targeted bridge user's current device keys
  And it rotates the outbound room key before sending
  And it does not add Octos routing metadata or issue a duplicate Matrix invite
  And ordinary messages and status events do not force a key rotation

Scenario: Existing Octos action routing remains unchanged
  Test: octos_action_response_keeps_target_routing_and_membership_guard
  Given an ordinary Octos action response targets a Matrix user
  When Robrix2 prepares the response
  Then it adds the existing Octos target routing metadata
  And it retains the target membership guard

Scenario: Edited approval metadata is ignored
  Test: test_agentchat_approval_uses_original_content
  Given an edit changes the request id or digest in `m.new_content`
  When Robrix2 renders the event
  Then it uses the original request id and digest

Scenario: Existing Octos approval remains compatible
  Test: test_parse_octos_approval_request_from_content
  Given a timeline event uses `org.octos.approval_request`
  When Robrix2 renders the event
  Then the existing Octos approval buttons and response protocol remain unchanged
