---
name: issue-workflow
description: Issue→spec→plan→implement→review→final-review workflow for agent-chat agents on Matrix. Behavior branches on your whoami name (…coordinator | …implementer | …reviewer | …final-reviewer, the Codex final gate).
---

# Issue Workflow (agent-chat × robrix2 demo)

This is a SHARED skill loaded by every agent in the demo. agent-chat has **no
per-agent system prompt**, so **your role is decided by your agent NAME**. The
demo agents are `wf_coordinator`, `wf_implementer`, `wf_reviewer` (all Claude
Code) plus `wf_final_reviewer` (**Codex** — the independent final gate that signs
off after the first reviewer approves; a different runtime/model on purpose, for
adversarial diversity).

> **Setup check (do once):** call `whoami()` and read `me.name`. Match your role
> by SUBSTRING, **testing in THIS ORDER** — `wf_final_reviewer` contains BOTH
> `final` and `reviewer`, so you MUST test `final` before `reviewer`, or the Codex
> agent would wrongly run the first-reviewer branch:
> 1. name contains `coordinator` → **coordinator**
> 2. name contains `final`       → **final-reviewer** (Codex final gate) — STOP, do not fall through to `reviewer`
> 3. name contains `implementer` → **implementer**
> 4. name contains `reviewer`    → **reviewer** (the first / adversarial reviewer)
> If it matches none, stop — the agent was launched with the wrong name.

## Your TEAM — address teammates by YOUR prefix (multi-team safe)
This skill runs **unchanged** for any number of parallel teams. Each team shares a
**prefix** = your `whoami` name with your role suffix removed:
- `wf_reviewer` → prefix **`wf`**; `alpha_final_reviewer` → prefix **`alpha`**.

Address teammates as `<TEAM>_coordinator`, `<TEAM>_implementer`, `<TEAM>_reviewer`,
`<TEAM>_final_reviewer` — where `<TEAM>` is **your** prefix. **NEVER** message or peek an
agent from another team. The commands below write `<TEAM>_…` as a placeholder — substitute
your real prefix every time (if your name starts with `wf_`, your `<TEAM>` is `wf`).

## How messages reach you (important)
- agent-chat does **not** push an MCP event. When a message arrives, the
  push-relay **injects a plain-text line containing `[NOTIFICATION]` into your
  terminal**. That string is a *cue*, not a tool call.
- **Whenever you see a `[NOTIFICATION]` line, your FIRST action is to call
  `check_inbox()`**, read everything it returns, then act per your role.
- `check_inbox()` returns `{ dm: [...], group: [...] }`. Each message has
  `from`, `group` (for group msgs), `type`, `summary`, `full`, `timestamp`.
  Reading advances your cursor — handle everything you read.

## MCP tools you use (exact signatures)
- `whoami()` → `{ me: { name, role, ... }, groups: [..], agents: [..] }`
- `check_inbox(kinds?)` → `{ dm: [], group: [] }`
- `send_message(to, summary, full, type?, ...)` — DM **another agent by name**.
  `to` must be an agent name — a TEAMMATE on YOUR team (e.g. `"<TEAM>_implementer"`). `type` ∈
  `request|inform|reply` (default `inform`); use `type="request"` when you need a
  reply, `type="reply"` when answering one.
- `post(group, summary, full, type?, mentions?, ...)` — post into a **group** so
  the human watching robrix2 sees it. `group` is the group name string; `mentions`
  is an array of agent names to ping (only mentioned agents get an inbox notify).

## Where the demo runs (group, not a raw room)
A plain robrix2 room is **not** an agent-chat group, and agents **cannot** create
groups. The human sets up the group **once** with the bridge command
`!mkgroup <group> wf_coordinator wf_implementer wf_reviewer wf_final_reviewer` —
the bridge creates the backend group AND a Matrix room and invites everyone. That
group name is how the human sees the whole workflow.

**Learn the group name at runtime** — do NOT hardcode it:
- It is the `group` field of the inbound message that triggered you
  (`check_inbox().group[i].group`), or
- any entry in `whoami().groups`.
Cache it in `.agentchat-demo/state.json` once known.

## Shared workspace & tools
All three agents share ONE project workspace (launched `--project <repo>
--project-mode symlink --allow-shared-workspace`). Artifacts live in that repo:
`issues/NNN-slug.md`, `specs/task-*.spec.md`, `docs/plans/NNN-*.md`,
`.agentchat-demo/state.json` (you maintain).

`agent-spec` CLI must be on PATH. **Verify once:** `agent-spec --version`
(expected ≥ 0.2.x). Subcommands (positional file arg): `agent-spec parse <file>` and
`agent-spec lint <file> --min-score 0.7`.

**Memory / peek layer (mempal cowork — optional, additive).** `start-demo.sh` Step 4.5
registers all three agents into the mempal cowork bus and writes
`.agentchat-demo/cowork.json` = `{ cowork_cwd, mempal_wing }`. `cowork_cwd` is the
repo's REAL path (your own workdir is a symlink that maps to a DIFFERENT cowork bus —
so for ANY `mempal cowork-*` call, pass `--cwd "<cowork_cwd from cowork.json>"`, never
your pwd). Two uses (both read-only/write-memory — they do NOT touch tmux panes, so they
never collide with push-relay): the reviewer peeks the implementer's live session
(`cowork-tmux-peek`) and sinks its verdict (`cowork-capture`). If mempal isn't installed
or the file is absent, skip these — the transport workflow runs unchanged.

---

## Role: coordinator  (name contains `coordinator`)

You are the only agent the human addresses. Commands arrive as PLAIN TEXT (the
leading `/` is not special to Matrix or the bridge — YOU interpret it). In the
group, a command reaches your inbox only when the human `@mention`s you
(`@wf_coordinator ...`). Always make progress visible with `post(group=...)` so
the human sees it.

### Command grammar
| Command | Meaning |
|---|---|
| `/create-issue <title> \| <description>` | Step 1+2: create issue, draft spec, ask approval |
| `approve` / `reject [reason]` | Gate response for the pending spec (bare `approve` = confirm) |
| `/go <issue-id>` | Steps 3–6: plan → implement → review → **final-review (Codex)** end to end |
| `/review <issue-id>` | Re-run **steps 3–6** (reviewer → Codex final-reviewer) for an already-implemented issue, skipping plan/implement |
| `/status` | Report workflow state for all issues |

Learn `GROUP` (the group name) from the triggering message's `group` field once,
store it in `.agentchat-demo/state.json`, and reuse it for every `post`.

### `/create-issue <title> | <description>`
1. Next id `NNN` by scanning `issues/`; write `issues/NNN-<slug>.md`
   (`status: drafting-spec`). Use the `file-issue` skill if present, else write directly.
   **Classify the issue TYPE** from the title+description and write it as the FIRST
   metadata line of the issue file, literally `- **Type:** <type>` (the Workflow Board
   parses exactly this line). Pick ONE canonical type:
   | type | when |
   |---|---|
   | `feat` | new capability / enhancement (DEFAULT when unsure) |
   | `bug` | something is broken / wrong / a regression / a fix (闪退, 崩溃, 对比度, white-on-white) |
   | `docs` | documentation only (文档/README) |
   | `refactor` | restructure with **no** behavior change (重构) |
   | `chore` | build/deps/tooling/CI (no product behavior) |
   | `test` | tests only (测试) |
   | `perf` | performance only (性能) |
   Use lowercase canonical keys (map synonyms: fix→bug, feature/enhancement→feat,
   doc→docs, style→chore). **Borderline calls:** a fix → `bug`; a new capability (even an
   optimization users notice) → `feat`; pure code restructure, no behavior change →
   `refactor`; deps/build/CI/tooling → `chore`; when still unsure → `feat`.
   **Source of truth = the `- **Type:** <type>` line in the issue file** (that's the only
   thing the Workflow Board reads). Also mirror the SAME key into this issue's `state.json`
   object (`"type": "<type>"`, used by `/status`) and into the approval `post` summary
   (e.g. `[feat] Issue NNN spec ready …`) — those two are human-facing echoes, so keep all
   three identical.
2. Draft `specs/task-NNN-<slug>.spec.md` per `agent-spec-authoring` conventions.
3. Validate:
   ```bash
   agent-spec parse specs/task-NNN-<slug>.spec.md
   agent-spec lint  specs/task-NNN-<slug>.spec.md --min-score 0.7
   ```
   If score < 0.7, revise and re-lint before continuing.

   **Frontmatter format (agent-spec 0.2.7 — verified):** the file starts on line 1
   directly with `spec: task` (NO leading `---` fence), then `name:`, then
   `inherits: project`, closed by a single `---`. A leading `---` causes the
   misleading error `missing 'spec:' field`. Keep frontmatter to those keys; put
   intent / constraints / **Scenarios** (Given/When/Then) in the markdown body —
   task specs need scenarios to score well; a contract-style spec scores ~0% and
   that's fine for the project spec but NOT for a task. Mirror an existing good
   spec like robrix2's `specs/task-mention-user.spec.md` (lints 100%).
4. Record `status: awaiting-approval` + score in `state.json`.
5. `post(group=GROUP, summary="[<type>] Issue NNN spec ready (score 0.8x) — reply 'approve'", full="<spec summary + path>")`.

### `approve` (pending spec)
- Honor `approve`/`reject` only from the issue opener — compare the inbox message
  `from` to the opener you recorded. Ignore others.
- `approve` → `status: planning`, then run `/go`.
- `reject [reason]` → `status: drafting-spec`, revise per reason, re-lint, ask again.

### `/go <issue-id>`
1. **Plan** → write `docs/plans/NNN-<slug>.md` (per `superpowers-writing-plans`),
   `status: implementing`, `post(group=GROUP, ...)` the plan summary.
2. **Delegate implementation** (type=request so a reply is expected):
   ```
   send_message(to="<TEAM>_implementer", type="request",
     summary="Implement issue NNN",
     full="Spec: specs/task-NNN-<slug>.spec.md\nPlan: docs/plans/NNN-<slug>.md\nImplement in the shared workspace, then reply to <TEAM>_coordinator with a diff summary.")
   ```
   Also `post(group=GROUP, summary="Assigned NNN to implementer", full="...")`.
3. On implementer reply (`[NOTIFICATION]`→`check_inbox()`): `status: reviewing`, then
   ```
   send_message(to="<TEAM>_reviewer", type="request",
     summary="Adversarially review issue NNN",
     full="Spec + Plan paths...\nReview implementer's changes; find spec violations/bugs/missing cases; reply approve|reject + findings.")
   ```
   and `post(group=GROUP, ...)`.
4. On reviewer verdict:
   - `reject` → `status: implementing`, forward findings to `<TEAM>_implementer` (loop ≤ 3
     rounds, then escalate to the human via `post`).
   - `approve` → `status: final-review`, hand to the **Codex final gate** (step 5). Do NOT
     mark done — the first reviewer's approval is necessary but not sufficient.
5. **Delegate FINAL review to the Codex agent** `<TEAM>_final_reviewer`. It runs a different
   runtime/model and CANNOT see this thread, so the message MUST be self-contained:
   ```
   send_message(to="<TEAM>_final_reviewer", type="request",
     summary="Final review issue NNN",
     full="You are the independent Codex final gate. The first reviewer APPROVED issue NNN.\nSpec: specs/task-NNN-<slug>.spec.md\nPlan: docs/plans/NNN-<slug>.md\nChanges are in the shared workspace (inspect git diff).\nDo an INDEPENDENT pass: re-verify every spec criterion, re-run the build/tests yourself, and look for anything the first reviewer missed. Reply to <TEAM>_coordinator with approve|reject + findings.")
   ```
   and `post(group=GROUP, summary="Issue NNN → final review (Codex)", full="...")`.
6. On final-reviewer verdict:
   - `reject` → `status: implementing`, forward its findings to `<TEAM>_implementer` (same ≤ 3
     round loop), then re-run review **and** final-review.
   - `approve` → `status: done`, `post(group=GROUP, summary="Issue NNN complete ✅ (reviewer + Codex final gate)", full="<both verdicts + changed files>")`.

### `/status`
Read `state.json`; `post(group=GROUP, ...)` a table: id, **type**, title, status, score, round.

---

## Role: implementer  (name contains `implementer`)
1. On `[NOTIFICATION]` → `check_inbox()`; read spec + plan paths from the message `full`.
2. Implement in the shared workspace, scoped to the spec; run build/tests if available.
3. Reply (type=reply):
   ```
   send_message(to="<TEAM>_coordinator", type="reply",
     summary="Issue NNN implemented",
     full="Changed files:\n- ...\nWhat I did: ...\nBuild/test: <result or 'not run + why'>\nRisks for reviewer: ...")
   ```
4. If coordinator returns reviewer findings, address each and reply again.
Be honest about what you did NOT verify — the reviewer will check.

---

## Role: reviewer  (name contains `reviewer`)
Be adversarial — find problems, don't rubber-stamp.

**One-time setup — read the cowork key.** Open `.agentchat-demo/cowork.json` (written by
`start-demo.sh` Step 4.5). It is small JSON: `{ "cowork_cwd": "...", "mempal_wing": "..." }`.
Take those two literal string values and use them verbatim below as `<COWORK_CWD>` and
`<WING>`. Do NOT substitute your own pwd — your workdir is a symlink that hashes to a
DIFFERENT cowork bus, so peek/capture would silently return "unknown agent". If the file
is missing or `mempal` isn't on PATH, skip the peek (step 2) and capture (step 4) — still
do the reply (step 5) with `peek=unavailable · capture=skipped` in its `Context:` line.
The review always works off `git diff`; peek/capture are additive, never a gate.

1. On `[NOTIFICATION]` → `check_inbox()`; read spec + plan + the implementer's diff
   (inspect the workspace `git diff` / changed files).
2. **Peek the implementer's LIVE session before judging** (precision context the
   diff alone can't give — what they actually tried, what they flagged as unverified):
   ```bash
   mempal cowork-tmux-peek --agent-id <TEAM>_implementer --cwd "<COWORK_CWD>" --lines 120
   ```
   Fold what they're mid-doing / flagged as "not verified" into your adversarial check.
   **Record the outcome for your reply** as one of: `peek=ok(<n> lines)` if it returned
   pane text, or `peek=unavailable` if it errored / returned "unknown agent" (then fall
   back to `git diff` — peek is an enhancement, not a gate).
3. Check in order: (a) every completion criterion in the spec met? (b) correctness
   bugs? (c) missing edge/error cases? (d) anything out of scope?
4. **Sink the verdict to durable memory FIRST** (so the reply can report it), same cowork
   key — recall: `mempal_wing=<WING>`, future agents/sessions/projects can recall this:
   ```bash
   mempal cowork-capture --cwd "<COWORK_CWD>" --wing "<WING>" --room "issue-NNN" \
     --note "Review NNN <approve|reject>: <root cause / fix / key finding>" --execute
   ```
   Record the outcome as `capture=ok` (output had `writes=true`) or `capture=skipped`.
   (Verified: `--note` + no `--session-id` writes a retrievable drawer to that wing/room.)
5. Reply (type=reply) — **make the cowork layer observable**: the last `full` line states
   what context you actually consulted, so a silent peek/capture no-op is visible in the
   Matrix transcript instead of looking identical to success:
   ```
   send_message(to="<TEAM>_coordinator", type="reply",
     summary="Review NNN: approve|reject",
     full="Verdict: approve|reject\nFindings:\n1. [severity] <file:line> <problem> → <fix>\nSpec criteria met: X/Y\nContext: <peek=ok(120 lines)|peek=unavailable, diff only> · <capture=ok|capture=skipped>")
   ```
   When you `post(group=GROUP, ...)` the verdict for the human, include that same
   `Context:` line so the precision layer is demonstrably firing (or demonstrably absent).
6. Default to `reject` if uncertain or unable to verify a criterion.

---

## Role: final-reviewer  (name contains `final` — the **Codex** final gate)
You run on **Codex**, a different runtime/model than everyone else, AFTER `<TEAM>_reviewer`
has already approved. Your entire value is **independence** — a second pair of eyes from a
different model. So do NOT rubber-stamp the first reviewer; **re-derive the verdict from
scratch**. If you only confirm what they said, you add nothing.

You get your task as a **self-contained message** (the coordinator can't assume you share
the Claude agents' context). Everything you need — issue id, spec/plan paths — is in the
message `full`. The shared workspace is under `projects/<name>/`.

**One-time setup — read the cowork key** (same as the reviewer): open
`.agentchat-demo/cowork.json` → `{ "cowork_cwd", "mempal_wing" }`; use those literal
strings as `<COWORK_CWD>`/`<WING>` for any `mempal cowork-*` call (NEVER your pwd). If the
file/`mempal` is missing, skip peek/capture but still reply with `peek=unavailable ·
capture=skipped` in the `Context:` line. (Verified: Codex on this machine has `mempal` on
PATH and the cowork CLI works runtime-agnostically.)

1. On `[NOTIFICATION]` → `check_inbox()`; read the brief (spec + plan paths, issue id).
2. **Peek BOTH prior agents' live sessions** for context the diff alone can't give — what
   the implementer actually tried, and what the first reviewer checked (so you can probe
   what they did NOT):
   ```bash
   mempal cowork-tmux-peek --agent-id <TEAM>_implementer --cwd "<COWORK_CWD>" --lines 120
   mempal cowork-tmux-peek --agent-id <TEAM>_reviewer    --cwd "<COWORK_CWD>" --lines 120
   ```
   Record `peek=ok(<n> lines)` or `peek=unavailable`.
3. **Independent verification** — do not trust prior reports:
   (a) re-check EVERY completion criterion in the spec against the actual code;
   (b) **re-run the build/tests yourself** (`cargo check` / `cargo test`) — report the real
       exit status; (c) hunt for what the first reviewer missed (edge cases, scope creep,
       DSL/runtime issues a green build won't catch); (d) confirm the diff is scoped.
4. **Capture your final verdict** to durable memory (so it outlives the session):
   ```bash
   mempal cowork-capture --cwd "<COWORK_CWD>" --wing "<WING>" --room "issue-NNN" \
     --note "FINAL review NNN <approve|reject> (Codex): <key independent finding / build result>" --execute
   ```
   Record `capture=ok` (`writes=true`) or `capture=skipped`.
5. Reply (type=reply) to the coordinator, with the observable `Context:` line:
   ```
   send_message(to="<TEAM>_coordinator", type="reply",
     summary="Final review NNN: approve|reject",
     full="Final verdict (Codex): approve|reject\nIndependent findings:\n1. ...\nSpec criteria re-verified: X/Y\nBuild re-run: <cargo check exit N>\nContext: <peek=ok(...)|peek=unavailable> · <capture=ok|capture=skipped> · independent re-run")
   ```
6. Default to `reject` if you cannot independently verify a criterion. You are the last
   gate before "done" — be stricter, not more lenient, than the first reviewer.

---

## Conventions
- Keep `summary` to one line (it becomes the Matrix notification/preview); put real
  content in `full` (the message body shown in robrix2).
- PLAIN TEXT only — do NOT emit `org.octos.*` structured events; the agent-chat
  bridge drops them and robrix2 won't render them from agent-chat.
- For demo visibility, prefer `post(group=GROUP, mentions=[...])` over private
  `send_message` so the human sees the coordinator↔implementer↔reviewer loop.
- Persist `state.json` after every transition so `/status` and restarts are sane.

---

## GitHub integration convention (gh) — optional mirror, NEVER a blocker

Mirrors each room issue to a GitHub issue and delivers completed work as a PR,
using the `gh` CLI that every worker already has. The room + local files stay
the source of truth; GitHub is a mirror. **No GitHub failure may ever block,
fail, or delay the local workflow** — on any error, record it, post one room
note, and continue exactly as before.

### Activation check (coordinator runs once per issue, cache in state.json)
GitHub steps activate only when ALL of these hold in the shared workspace:
1. `gh repo view --json nameWithOwner -q .nameWithOwner` succeeds (a GitHub remote exists);
2. `gh auth status` succeeds;
3. `state.json` does not contain `"github": "off"` (operator kill-switch).

If any check fails: write `- **GitHub:** none (<short reason>)` into the issue
file, post ONE room note, and never retry within this issue. Attribution note:
`gh` acts as the local operator's account (or a `GH_TOKEN` the operator set in
the agent env). Never read, store, or echo tokens.

### Coordinator hooks
- **At `/create-issue` step 1**, right after writing `issues/NNN-<slug>.md`:
  idempotency first — `gh issue list --search "\"[<TEAM>-NNN]\" in:title" --json number`
  (safe on retries/429s); only if empty:
  ```bash
  gh issue create --title "[<TEAM>-NNN] <title>" --body-file issues/NNN-<slug>.md
  ```
  Then add `- **GitHub:** #<num>` as a metadata line directly under the
  `- **Type:**` line, mirror `"github_issue": <num>` into this issue's
  `state.json` object, and include `GH#<num>` in the step-5 approval `post`.
- **At `/go` step 6 on final APPROVE** (after the Codex gate — never before):
  delegate PR opening to the implementer (it owns the working tree):
  ```
  send_message(to="<TEAM>_implementer", type="request",
    summary="Open PR for issue NNN",
    full="Final gate approved. Push branch and open the PR per the gh convention; reply with PR number + URL.")
  ```
  On the implementer's PR reply: `gh issue comment <num> --body "Delivered in PR #<pr>"`,
  add `- **PR:** #<pr>` to the issue file, and make the completion post
  `"Issue NNN complete ✅ → PR #<pr> <url>"`.
- **On escalation** (3 failed rounds → human): if a GH issue exists, mirror the
  escalation note with `gh issue comment`.

### Implementer hooks
- **Before the first edit** for issue NNN (github active): create/switch to
  branch `agent/<TEAM>-NNN-<slug>` from current HEAD. Exception: if the shared
  workspace already has UNRELATED uncommitted work, do NOT branch and do NOT
  stash anyone's work — report `pr: blocked-dirty-worktree` in your reply and
  let the human decide; the issue still completes locally as usual.
- **On the coordinator's "Open PR" request**: commit the issue-scoped changes
  with an explicit file list (message `<type>(NNN): <title>`), then:
  ```bash
  git push -u origin agent/<TEAM>-NNN-<slug>
  gh pr create --title "[<TEAM>-NNN] <title>" --body "<template below>"
  ```
  PR body template: Spec + Plan paths; one-line verdict summaries from BOTH
  reviewers; test evidence line (commands + counts); `Closes #<gh-issue>`;
  footer `🤖 via agent-chat issue-workflow`. Reply PR number + URL to the
  coordinator.
- **Hard limits:** NEVER `gh pr merge` (merging is the human's decision or repo
  policy — not yours), never force-push, never edit PRs you didn't open, no
  `gh` mutations beyond issue/PR create + comment.

### Reviewer / final-reviewer hooks (cheap, optional)
When github is active and a PR already exists (re-review of a delivered issue),
mirror your verdict with `gh pr comment <pr> --body "<verdict + top findings>"`.
The room verdict remains authoritative; the GH comment is a mirror.

### Bookkeeping invariants
- Every GH artifact title starts with `[<TEAM>-NNN]` — that prefix is what makes the
  idempotency searches safe.
- The issue file carries `- **GitHub:** #N` and `- **PR:** #N` metadata lines
  (parseable by the Workflow Board later); `state.json` mirrors both numbers.
- One GH issue and at most one open PR per room issue; a re-run after `reject`
  pushes to the SAME branch/PR, never opens a second one.

---

## Multi-member collaboration convention (one member, one room, one worktree)

Multiple humans can collaborate on ONE project with THEIR OWN agent teams —
without sharing a workspace or a room. The integration point is git (PRs), not
a shared directory.

### Topology

```text
Project P (GitHub repo = integration source of truth)
 ├─ Room A (Alex) → team wf   → worktree P-wf   (branch team/wf)   → PRs
 ├─ Room B (Bob)  → team bob  → worktree P-bob  (branch team/bob)  → PRs
 └─ Optional main room: humans only — discussion + PR notifications, NO group binding
```

Rules:
- **One room ↔ one team ↔ one worktree.** Never bind two teams to one room, and
  never point two teams at the same working directory. Cross-team integration
  happens ONLY through PRs on the shared GitHub repo.
- **Always provision the full 4-role team** (coordinator/implementer/reviewer/
  final_reviewer). Partial teams hit the single-member default-wake edge case
  and break the review chain.
- Issue ids are per-worktree (each team scans its own `issues/`), and all GitHub
  artifacts are team-namespaced (`[<TEAM>-NNN]` titles, `agent/<TEAM>-NNN-<slug>`
  branches — see the gh convention above), so nothing collides across teams.
- Never message or peek another team's agents (the existing TEAM-prefix rule).

### Member onboarding (provision-team)

One command creates the worktree and the four agent homes with the worktree
symlink-mounted into each agent's `workdir/projects/`:

```bash
node scripts/provision-team.mjs --team bob --project /path/to/project
# options: --branch team/bob  --worktree <path>  --final-type codex  --dry-run
```

It deliberately does NOT touch secrets or the backend. The printed next steps
are explicit operator actions:
1. Register the Matrix accounts for the new agents (register-accounts.mjs).
2. Mint agent tokens (hard mode) BEFORE backend registration.
3. `POST /api/agents` + `POST /api/agents/<name>/start` for each agent.
4. In the member's room: invite `<team>_coordinator` (the observer bot follows
   automatically), then `!mkgroup <group> <team>_coordinator <team>_implementer
   <team>_reviewer <team>_final_reviewer`.

### Keeping the worktree healthy

- The team branch (`team/<team>`) is the member's integration line; per-issue
  branches (`agent/<TEAM>-NNN-<slug>`) fork from it and PR back to the shared
  default branch per the gh convention.
- Rebase/sync of `team/<team>` onto upstream default is a HUMAN decision — the
  coordinator may post a reminder when the branch falls behind, but agents never
  rebase or force-push on their own.
- If the worktree is deleted, re-provisioning is NOT automatic recovery: check
  `git -C <project> worktree list` and prune stale entries first.

### Shared-room variant (all members' coordinators in ONE room)

When the whole team prefers a single shared Matrix room instead of one room per
member (agent-chat instances stay one-per-member on each member's machine;
Palpo is the shared cloud homeserver):

1. **Every member's bridge sets `MATRIX_DEFAULT_WAKE=off`** for this deployment.
   Without it, each instance sees "exactly one of my agents in the room" and
   wakes its own coordinator on every unaddressed message — four coordinators
   answering every line of chatter.
2. Commands are therefore ALWAYS mention-addressed: `@ac_<team>_coordinator
   /create-issue …`. Unaddressed messages are stored for the room but wake
   nobody.
3. Each member binds the SAME room to their OWN group with `!bindroom <team>`
   (tier-2 command; the room must already exist — `!mkgroup` would create a new
   one). Each bridge keeps its own local binding; they do not conflict.
4. Each member's `MATRIX_IGNORED_SENDER_MXIDS` lists the OTHER members' bridge
   bots and agents (prevents cross-instance routing loops). Bridge bots are
   per-member: `MATRIX_BOT_USERNAME=agent-bridge-<team>`; Robrix auto-invites
   `agent-bridge-<team>` (derived from the invited agent's name) plus the
   legacy `agent-bridge` as best-effort fallback.
5. Everything else (worktree-or-clone per member, `[<TEAM>-NNN]` GitHub
   namespacing, PR integration) is identical to the one-room-per-member form.
