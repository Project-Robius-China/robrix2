# TG Bot Approval Request Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a cross-repo approval-request flow where Octos emits approval-required bot messages, Robrix renders inline approval buttons, and Octos revalidates, audits, and executes only after a valid approval response.

**Architecture:** Keep the security boundary in Octos. Robrix only parses `org.octos.approval_request`, renders approval UI on top of the existing Phase 4c action-buttons path, and sends a one-shot targeted `org.octos.approval_response`. Octos owns pending approval storage, replay/expiry checks, approver revalidation, timeout notifications, and audit logging; the current `before_tool_call` hook boundary is the right interception point for approval-required tool calls.

**Tech Stack:** Rust, Makepad 2.0 `script_mod!`, Matrix custom event content fields, `matrix-sdk` send path in `sliding_sync.rs`, Octos agent lifecycle hooks (`before_tool_call`), Octos Matrix bus, `cargo test`, `cargo build`, `agent-spec`.

---

## File Map

- `specs/task-tg-bot-approval-request.spec.md`
  - Source-of-truth contract. Only update if implementation reveals a real wording bug.

- `src/home/room_screen.rs`
  - Parse `org.octos.approval_request` alongside existing `org.octos.actions`.
  - Render approval title/summary/expiry state using the current action-buttons container.
  - Disable approval buttons for unauthorized local users.
  - Build action-button context for approval responses.
  - Add Robrix-side unit tests for approval request parsing and UI state.

- `src/sliding_sync.rs`
  - Add a send-path helper for approval responses that sets:
    - one-shot `target_user_id = original_sender`
    - `org.octos.approval_response`
    - `m.in_reply_to` to the source event
  - Keep generic `org.octos.action_response` behavior intact.

- `resources/i18n/en.json`
- `resources/i18n/zh-CN.json`
  - Add fallback strings for disabled approval buttons, timeout state, and malformed-approval warnings if needed.

- `../../octos/crates/octos-agent/src/approval.rs` (new)
  - Own the runtime approval model:
    - `PendingApproval`
    - `ApprovalDecision`
    - validation helpers for expiry / replay / approver authority / digest
  - Provide a small in-memory pending store abstraction and audit event structs.

- `../../octos/crates/octos-agent/src/lib.rs`
  - Export the new approval module if needed by agent runtime tests.

- `../../octos/crates/octos-agent/src/agent/execution.rs`
  - Hook point for converting approval-required tool calls into pending approval requests instead of immediate execution.
  - Reuse existing `before_tool_call` lifecycle semantics; do not invent a parallel policy path.

- `../../octos/crates/octos-bus/src/matrix_channel.rs`
  - Emit Matrix approval-request messages.
  - Consume `org.octos.approval_response` messages from Matrix and forward them back into Octos approval handling.
  - Ensure expired/replayed/unauthorized responses do not execute tools.

- `../../octos/book/src/advanced.md`
  - Document approval request behavior as an extension of the hook / policy model.

## Key Implementation Decisions To Preserve

- Do not move permission truth into Robrix.
- Do not add slash-command approval UX like `/approve 123`.
- Do not let `authorized_approvers` from the message bypass Octos-side revalidation.
- Robrix must treat `tool_args_digest` as opaque request data: copy only, never recompute.
- Robrix must render approval requests from the original event content, not `m.replace` / `m.new_content`.
- Octos must derive approver identity from the Matrix event `sender`, never from payload fields.
- Octos must bind pending approvals to the originating `room_id`; wrong-room responses are invalid.
- Octos must reject approval requests with empty `authorized_approvers` instead of emitting unusable approval UI.
- Do not replace generic Phase 4c actions; approval requests are an extension, not a fork.
- Do not commit until user testing completes on both the Robrix UI side and the Octos execution/audit side.

---

### Task 1: Robrix Parses and Renders Approval Requests

**Files:**
- Modify: `src/home/room_screen.rs`
- Modify: `resources/i18n/en.json`
- Modify: `resources/i18n/zh-CN.json`
- Test: `src/home/room_screen.rs`

- [ ] **Step 1: Write failing parsing/render-state tests for approval requests**

Add focused tests near the existing `org.octos.actions` tests:
- `test_parse_octos_approval_request_from_content`
- `test_approval_buttons_disabled_for_unauthorized_user`
- `test_generic_actions_without_approval_request_remain_supported`
- `test_malformed_approval_request_hides_buttons`
- `test_approval_request_ignores_m_replace_edits`

Cover:
- valid `request_id`, `tool_args_digest`, `authorized_approvers`, `expires_at`
- missing required fields
- empty `authorized_approvers`
- local user present vs absent in `authorized_approvers`
- approval rendering sourced from the original event content only
- generic actions message still using the old path

- [ ] **Step 2: Run the new Robrix parsing tests and confirm they fail**

Run:

```bash
cargo test parse_octos_approval_request --quiet
cargo test approval_buttons_disabled --quiet
cargo test malformed_approval_request --quiet
```

Expected: FAIL because `room_screen.rs` currently only understands `org.octos.actions`.

- [ ] **Step 3: Add approval-request data structs and parser helpers**

In `src/home/room_screen.rs`:
- add a small `OctosApprovalRequest` struct
- add parser helpers that:
  - read `org.octos.approval_request`
  - read it from original event content rather than latest edit content
  - validate required fields
  - return `None` on malformed input
- add an authorization helper:
  - `local_user_can_approve(approval_request, current_user_id)`

- [ ] **Step 4: Extend the action-button render state with approval metadata**

Update the existing render-state computation so it can represent:
- generic actions
- approval request with enabled buttons
- approval request with disabled buttons
- malformed approval request with no buttons

Do not introduce a second button container. Keep approval on the current Phase 4c surface.

- [ ] **Step 5: Render approval title/summary state in the timeline**

In `populate_message_view()` and the action-button population helpers:
- surface approval title / summary above the buttons when approval metadata exists
- disable buttons when the local user is not in `authorized_approvers`
- leave generic action-button messages unchanged

- [ ] **Step 6: Re-run Robrix approval parsing/render tests**

Run:

```bash
cargo test parse_octos_approval_request --quiet
cargo test approval_buttons_disabled --quiet
cargo test generic_actions_without_approval_request --quiet
cargo test malformed_approval_request --quiet
```

Expected: PASS.

---

### Task 2: Robrix Sends Structured Approval Responses

**Files:**
- Modify: `src/home/room_screen.rs`
- Modify: `src/sliding_sync.rs`
- Test: `src/home/room_screen.rs`
- Test: `src/sliding_sync.rs`

- [ ] **Step 1: Write failing send-path tests for approval responses**

Add tests:
- `test_click_approve_builds_approval_response_payload`
- `test_click_deny_builds_approval_response_payload`
- `test_approval_response_routes_to_original_sender`
- `test_approval_response_copies_digest_without_recomputing`

Assert:
- `org.octos.approval_response.request_id`
- `decision = approve|deny`
- `tool_args_digest`
- `m.in_reply_to` points to the source event
- `target_user_id` equals the original bot sender

- [ ] **Step 2: Run the new send-path tests and confirm they fail**

Run:

```bash
cargo test approval_response --quiet
```

Expected: FAIL because button clicks currently only send generic `org.octos.action_response`.

- [ ] **Step 3: Add an approval-response request type separate from generic actions**

In `src/home/room_screen.rs`:
- add a dedicated approval-response request builder
- keep the current generic action-response builder untouched
- ensure approval clicks carry `request_id`, `decision`, and `tool_args_digest`
- ensure `tool_args_digest` is copied byte-for-byte from the request, with no client-side hashing or normalization

- [ ] **Step 4: Add a dedicated send helper in `sliding_sync.rs`**

Add the minimal send-path helper needed so approval clicks can:
- bypass input-bar reply/mention state
- set one-shot `target_user_id = original_sender`
- attach `org.octos.approval_response`
- attach `m.in_reply_to`

Do not merge this into unrelated generic send code unless it clearly reduces duplication.

- [ ] **Step 5: Wire approval button clicks to the new send helper**

In the room-screen action-button click handler:
- detect `approve` / `deny` on messages carrying approval metadata
- call the new approval send helper
- preserve current disable-on-click behavior
- keep generic action buttons on the old send path

- [ ] **Step 6: Re-run focused approval send tests**

Run:

```bash
cargo test approval_response --quiet
```

Expected: PASS.

---

### Task 3: Octos Converts Approval-Required Tool Calls into Pending Requests

**Files:**
- Create: `../../octos/crates/octos-agent/src/approval.rs`
- Modify: `../../octos/crates/octos-agent/src/lib.rs`
- Modify: `../../octos/crates/octos-agent/src/agent/execution.rs`
- Test: `../../octos/crates/octos-agent/src/approval.rs`
- Test: `../../octos/crates/octos-agent/src/agent/execution.rs`

- [ ] **Step 1: Write failing Octos unit tests for pending approval state**

Add focused tests such as:
- `test_create_pending_approval_with_digest_and_expiry`
- `test_pending_approval_rejects_duplicate_consume`
- `test_pending_approval_rejects_expired_request`
- `test_pending_approval_revalidates_authorized_approver`
- `test_pending_approval_rejects_empty_authorized_approvers`
- `test_pending_approval_rejects_wrong_room`

- [ ] **Step 2: Run the new Octos approval-state tests and confirm they fail**

Run from `../../octos`:

```bash
cargo test -p octos-agent approval --quiet
```

Expected: FAIL because approval runtime types and store do not exist yet.

- [ ] **Step 3: Create `approval.rs` with the minimal runtime model**

Implement:
- `PendingApproval`
- `ApprovalDecision`
- helper for `tool_args_digest`
- in-memory pending store keyed by `request_id`
- store original `room_id` alongside `request_id`
- helper methods:
  - `is_expired`
  - `consume_once`
  - `is_authorized_approver`

Keep the storage abstraction small; v1 does not need durable persistence.

- [ ] **Step 4: Hook approval-required tool calls in `agent/execution.rs`**

Use the existing `before_tool_call` / policy boundary:
- when policy says “approval required”, do not execute the tool
- if policy resolves to an empty approver set, deny immediately instead of creating a pending request
- create a pending approval entry instead
- return control to the transport layer with enough metadata to send a Matrix approval request

Do not invent a second policy engine alongside hooks/tool policy.

- [ ] **Step 5: Re-run Octos approval-state tests**

Run:

```bash
cargo test -p octos-agent approval --quiet
```

Expected: PASS.

---

### Task 4: Octos Emits Approval Requests and Consumes Approval Responses over Matrix

**Files:**
- Modify: `../../octos/crates/octos-bus/src/matrix_channel.rs`
- Test: `../../octos/crates/octos-bus/src/matrix_channel.rs`

- [ ] **Step 1: Write failing Matrix protocol tests**

Add tests covering:
- `test_matrix_approval_request_event_contains_protocol_fields`
- `test_matrix_approval_response_executes_once`
- `test_duplicate_approval_response_is_rejected`
- `test_expired_approval_response_notifies_without_execution`
- `test_approval_response_revalidated_against_current_policy`
- `test_approval_response_uses_matrix_sender_identity`
- `test_approval_response_wrong_room_rejected`

- [ ] **Step 2: Run the new Matrix approval tests and confirm they fail**

Run from `../../octos`:

```bash
cargo test -p octos-bus approval --features matrix --quiet
```

Expected: FAIL because Matrix channel currently has no approval protocol.

- [ ] **Step 3: Emit approval-request Matrix messages**

In `matrix_channel.rs`:
- add a helper to build `m.room.message` content with:
  - `org.octos.approval_request`
  - `org.octos.actions` for `approve` / `deny`
- include `request_id`, `tool_name`, `tool_args_digest`, `title`, `summary`,
  `risk_level`, `authorized_approvers`, `expires_at`, `on_timeout`

- [ ] **Step 4: Parse and validate `org.octos.approval_response`**

When Matrix messages arrive:
- detect `org.octos.approval_response`
- validate required fields
- look up pending request by `request_id`
- derive approver identity from Matrix event `sender`
- reject if expired / already consumed / unauthorized / digest mismatch / wrong room
- only execute the tool on a valid, first-time `approve`
- mark as terminal on `deny`

- [ ] **Step 5: Emit timeout and rejection notifications**

Add minimal follow-up Matrix messages for:
- timeout (`on_timeout = notify`)
- duplicate response rejected
- unauthorized response rejected

Keep wording simple; v1 is about safety and observability, not polished copy.

- [ ] **Step 6: Re-run Matrix approval tests**

Run:

```bash
cargo test -p octos-bus approval --features matrix --quiet
```

Expected: PASS.

---

### Task 5: Audit Logging and Documentation

**Files:**
- Modify: `../../octos/crates/octos-agent/src/approval.rs`
- Modify: `../../octos/book/src/advanced.md`
- Test: `../../octos/crates/octos-agent/src/approval.rs`

- [ ] **Step 1: Write failing audit-focused tests**

Add tests such as:
- `test_approval_request_creation_is_audited`
- `test_approval_terminal_decision_is_audited`

Assert that audit entries include:
- `request_id`
- `tool_name`
- `tool_args_digest`
- requester / approver
- execution outcome

- [ ] **Step 2: Run the audit tests and confirm they fail**

Run:

```bash
cargo test -p octos-agent approval_audit --quiet
```

Expected: FAIL because approval audit events are not yet emitted.

- [ ] **Step 3: Add minimal audit event recording**

In `approval.rs`:
- add structured audit event types
- record request creation
- record terminal decision / execution outcome

Prefer the smallest abstraction that can later be swapped to durable storage.

- [ ] **Step 4: Document the approval-request flow**

In `../../octos/book/src/advanced.md`:
- explain approval-required tool calls as an extension of `before_tool_call`
- document the Matrix fields at a high level
- state that Robrix is UI-only and Octos revalidates authority on receipt

- [ ] **Step 5: Re-run audit tests**

Run:

```bash
cargo test -p octos-agent approval_audit --quiet
```

Expected: PASS.

---

### Task 6: Cross-Repo Verification and User Test Checklist

**Files:**
- No new files unless the spec or docs need wording correction

- [ ] **Step 1: Run Robrix focused tests**

Run from `robrix2`:

```bash
cargo test approval_response --quiet
cargo test parse_octos_approval_request --quiet
cargo test approval_buttons_disabled --quiet
```

Expected: PASS.

- [ ] **Step 2: Run Octos focused tests**

Run from `../../octos`:

```bash
cargo test -p octos-agent approval --quiet
cargo test -p octos-bus approval --features matrix --quiet
```

Expected: PASS.

- [ ] **Step 3: Build both repos**

Run:

```bash
cargo build
```

from `robrix2`, then:

```bash
cargo build -p octos-agent -p octos-bus
```

from `../../octos`.

Expected: PASS.

- [ ] **Step 4: Manual end-to-end test with local Octos + Robrix**

Verify:
- Octos emits an approval request for a policy-gated action
- Robrix shows title, summary, and two buttons
- unauthorized local user sees disabled buttons
- edits to the approval request message do not alter approver UI
- authorized user can approve once
- duplicate clicks do not double-execute
- approval from the wrong room is rejected
- spoofed payload identity does not override Matrix sender identity
- expired request produces timeout notification instead of execution

- [ ] **Step 5: User testing checkpoint (required before any commit)**

Do not commit or open a PR until the user confirms:
- approval request UI looks correct
- approve/deny responses route to the correct bot
- denied / expired / duplicate flows are understandable
- Octos audit behavior matches expectation

---

## Notes for the Implementer

- Start with focused unit tests in both repos before touching behavior.
- Keep the first version narrow: `approve` / `deny`, `normal|critical`, `on_timeout = notify`.
- Reuse the existing Phase 4c action-button infrastructure in Robrix; do not create a second UI concept.
- If you need to choose between “clean abstraction” and “clear enforceable boundary”, prefer the latter. The important invariant is that approval authority never migrates into the client.
