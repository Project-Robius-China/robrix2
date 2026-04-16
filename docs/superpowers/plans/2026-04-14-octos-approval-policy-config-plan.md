# Octos Approval Policy Configuration Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a native `approval_policy` config in Octos so approval-required tool calls are driven by first-class configuration instead of external hook scripts.

**Architecture:** Keep the existing approval-request protocol and runtime, but introduce a new config layer that decides when to enter that approval path. `tool_policy.deny` remains the hard-stop gate, `approval_policy` becomes the primary approval trigger for v1, and hooks remain available only for advanced deny/modify/logging cases.

**Tech Stack:** Rust, serde config deserialization, Octos CLI config/runtime, Octos agent execution path, Matrix approval-request protocol, `cargo test`, `cargo build`, `agent-spec`.

---

## File Map

- `specs/task-octos-approval-policy-config.spec.md`
  - Source-of-truth contract for config-driven approval rules.

- `../../octos/crates/octos-cli/src/config.rs`
  - Add config structs for `approval_policy` and validation hooks.

- `../../octos/crates/octos-cli/src/commands/chat.rs`
  - Pass native approval-policy config into the agent/session runtime.

- `../../octos/crates/octos-cli/src/commands/gateway/profile_factory.rs`
  - Ensure gateway-created child sessions inherit the resolved approval policy from profile config.

- `../../octos/crates/octos-cli/src/session_actor.rs`
  - No new protocol here if possible; only accept resolved pending approval drafts from the agent/runtime.

- `../../octos/crates/octos-agent/src/approval.rs`
  - Extend runtime approval types with config-driven rule matching and request shaping helpers.

- `../../octos/crates/octos-agent/src/agent/execution.rs`
  - Evaluate `tool_policy.deny` then `approval_policy` before tool execution.

- `../../octos/book/src/configuration.md`
- `../../octos/book/src/advanced.md`
  - Document the new `approval_policy` field and how it relates to hooks.

## Key Decisions To Preserve

- Do not move approval truth into Robrix.
- Do not require external Python/shell hooks for the default approval path.
- Keep v1 matching by tool name only.
- `tool_policy.deny` must override `approval_policy`.
- `approval_policy.rules` are first-match-wins.
- Empty `authorized_approvers` must fail closed at config validation time.
- Reuse the existing `org.octos.approval_request` / `approval_response` protocol; do not fork it.
- Do not add new cargo dependencies.

### Task 1: Add Config Types and Validation

**Files:**
- Modify: `../../octos/crates/octos-cli/src/config.rs`
- Test: `../../octos/crates/octos-cli/src/config.rs`

- [ ] **Step 1: Write failing config tests**

Add focused tests:
- `test_config_deserializes_approval_policy`
- `test_config_rejects_approval_policy_with_empty_authorized_approvers`
- `test_config_rejects_approval_policy_with_require_approval_false`
- `test_config_rejects_approval_policy_with_empty_tools`

- [ ] **Step 2: Run the new config tests and confirm they fail**

Run:

```bash
cargo test -p octos-cli approval_policy --quiet
```

Expected: FAIL because `approval_policy` does not exist in config yet.

- [ ] **Step 3: Add config structs**

In `config.rs`, add:
- `ApprovalPolicyConfig`
- `ApprovalRuleConfig`
- `ApprovalPolicyDefault`
- `ApprovalPolicyRiskLevel`
- `ApprovalPolicyTimeoutBehavior`

Use serde derives only; no new dependency.

- [ ] **Step 4: Add config validation helpers**

Validate:
- `rules[*].tools` non-empty
- `rules[*].authorized_approvers` non-empty
- `require_approval == true`
- `expires_in_secs > 0`
- `default == "allow"`

Return descriptive config-load errors.

- [ ] **Step 5: Re-run config tests**

Run:

```bash
cargo test -p octos-cli approval_policy --quiet
```

Expected: PASS.

### Task 2: Add Approval Policy Matching Runtime

**Files:**
- Modify: `../../octos/crates/octos-agent/src/approval.rs`
- Modify: `../../octos/crates/octos-agent/src/lib.rs`
- Test: `../../octos/crates/octos-agent/src/approval.rs`

- [ ] **Step 1: Write failing approval-policy matching tests**

Add focused tests:
- `test_approval_policy_first_match_wins`
- `test_approval_policy_non_matching_tool_returns_none`
- `test_approval_policy_generates_relative_expiry`
- `test_approval_policy_shell_call_emits_pending_approval`

- [ ] **Step 2: Run the new approval matching tests and confirm they fail**

Run:

```bash
cargo test -p octos-agent approval_policy --quiet
```

Expected: FAIL because rule matching helpers do not exist yet.

- [ ] **Step 3: Add runtime policy matcher**

In `approval.rs`, add a small matcher that:
- takes tool name + current time + config rules
- returns `None` for no match
- returns a `PendingApprovalDraft` or equivalent request spec when matched
- computes `expires_at = created_at + expires_in_secs`

- [ ] **Step 4: Keep digest generation in runtime**

Ensure `tool_args_digest` is still computed from actual tool args at runtime, not from config.

- [ ] **Step 5: Re-run approval matching tests**

Run:

```bash
cargo test -p octos-agent approval_policy --quiet
```

Expected: PASS.

### Task 3: Wire Approval Policy into Agent Execution

**Files:**
- Modify: `../../octos/crates/octos-agent/src/agent/execution.rs`
- Test: `../../octos/crates/octos-agent/src/agent/execution.rs`

- [ ] **Step 1: Write failing execution-order tests**

Add focused tests:
- `test_tool_policy_deny_overrides_approval_policy`
- `test_approval_policy_creates_pending_approval_before_tool_execution`
- `test_approval_policy_non_matching_tool_executes_normally`
- `test_approval_policy_does_not_require_hook_exit_3`

- [ ] **Step 2: Run the new execution tests and confirm they fail**

Run:

```bash
cargo test -p octos-agent execution approval_policy --quiet
```

Expected: FAIL because execution path does not consult native `approval_policy` yet.

- [ ] **Step 3: Evaluate deny before approval**

In `execution.rs`:
- keep existing `tool_policy.deny` behavior first
- if denied, stop immediately and do not create pending approval

- [ ] **Step 4: Evaluate approval policy before running the tool**

If a rule matches:
- create pending approval draft
- do not execute the tool
- return the approval-request outcome

If no rule matches:
- continue current execution path unchanged

- [ ] **Step 5: Keep hook compatibility narrow**

Do not require `before_tool_call` to emit approval JSON anymore for the common path.
Hooks may still:
- deny
- modify args
- log/observe

- [ ] **Step 6: Re-run execution tests**

Run:

```bash
cargo test -p octos-agent execution approval_policy --quiet
```

Expected: PASS.

### Task 4: Pass Approval Policy Through CLI/Gateway Config

**Files:**
- Modify: `../../octos/crates/octos-cli/src/commands/chat.rs`
- Modify: `../../octos/crates/octos-cli/src/commands/gateway/profile_factory.rs`
- Potentially modify: `../../octos/crates/octos-cli/src/session_actor.rs`
- Test: corresponding unit tests near config/runtime wiring

- [ ] **Step 1: Identify where agent/session runtime receives tool policy today**

Read the current config-to-runtime wiring in `chat.rs` and gateway profile factory.

- [ ] **Step 2: Add approval policy plumbing alongside tool policy**

Pass resolved `approval_policy` into the runtime/agent config using the same pattern as `tool_policy`.

- [ ] **Step 3: Add a gateway-focused failing test**

Add:
- `test_gateway_runtime_emits_matrix_approval_request_from_policy`

Use a minimal config/profile fixture that includes `approval_policy`.

- [ ] **Step 4: Run the gateway test and confirm it fails**

Run:

```bash
cargo test -p octos-cli gateway approval_policy --quiet
```

Expected: FAIL until runtime wiring is complete.

- [ ] **Step 5: Make the test pass**

Keep `session_actor.rs` protocol behavior unchanged if possible; it should just receive pending approvals from the agent path exactly as before.

- [ ] **Step 6: Re-run the gateway-focused tests**

Run:

```bash
cargo test -p octos-cli gateway approval_policy --quiet
```

Expected: PASS.

### Task 5: Document Config-Driven Approval

**Files:**
- Modify: `../../octos/book/src/configuration.md`
- Modify: `../../octos/book/src/advanced.md`

- [ ] **Step 1: Document the new config block in `configuration.md`**

Add:
- field description
- JSON example
- note that v1 matches by tool name only

- [ ] **Step 2: Update `advanced.md` to reposition hooks**

Clarify:
- hooks can still deny/modify/log
- native `approval_policy` is now the default approval mechanism
- exit code `3` remains available for advanced dynamic cases, but is no longer required for normal approval setup

- [ ] **Step 3: Sanity-check docs for consistency**

Search for conflicting statements about hooks being the only approval path.

Run:

```bash
rg -n "approval|before_tool_call|exit code 3" ../../octos/book/src -S
```

Expected: docs consistently describe native config-driven approval.

### Task 6: Full Verification

**Files:**
- No code changes; verification only

- [ ] **Step 1: Run focused Octos config tests**

```bash
cargo test -p octos-cli approval_policy --quiet
```

- [ ] **Step 2: Run focused Octos agent approval tests**

```bash
cargo test -p octos-agent approval --quiet
```

- [ ] **Step 3: Run full Matrix bus regression**

```bash
cargo test -p octos-bus --features matrix --quiet
```

- [ ] **Step 4: Build Octos**

```bash
cargo build
```

- [ ] **Step 5: User test checklist**

Verify manually:
- config with `approval_policy` and no hook still produces approval requests
- unauthorized local user sees disabled approval buttons in Robrix
- authorized user can approve/deny
- deny does not execute tool
- approve executes tool
- expired request emits timeout notice

