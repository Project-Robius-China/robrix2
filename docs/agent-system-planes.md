# The three planes of the agent system

Written because the distinction is easy to lose, and losing it produces real
design errors. The first draft of `specs/task-agent-ops-panel.spec.md` put
approval buttons and worktree paths into a Matrix client screen; that was
caught by agent-chat's privacy requirement, not by good judgement. This
document exists so the next person does not have to rediscover the boundary the
same way.

Anyone about to extend the Agent Operations Panel, or to build a "see all the
agents" view, should read this first.

## Plane 1 — Matrix: the shared collaboration surface

agent-chat bridges each operator's local agents into Matrix as puppet accounts
(`@ac_<name>:…`). A room therefore contains agents belonging to several
different operators, alongside the humans.

**Robrix2 already renders this plane.** The room list, the member list, the
threads, the messages — that *is* the cross-operator view of who is present and
what is being discussed. It needs no additional panel, and it is governed by
Matrix's own access model: you see what the rooms you are in contain.

What lives here: conversation, task threads, agent presence, approval requests
addressed to their owner.

## Plane 2 — Router: one backend's private control plane

Each agent-chat backend owns the execution state of *its own* agents: topic
sessions, the dispatch ledger, workspace and resource leases, worktrees, task
bindings. See `agent-chat/docs/router-layer-design.md`.

This plane is inherently single-operator and privileged. It knows local
filesystem layout, which worktree holds uncommitted work, which dispatch ended
in `outcome_unknown` on this machine. One backend has no view of another
operator's backend, and should not.

What lives here: everything about *how* work executed, on *this* host.

## Plane 3 — Operations panel: a window onto plane 2

The Agent Operations Panel is an operator console for **one** backend. It is
not a fleet view and not a management surface for other people's agents.

If the canonical contract is released and the client runtime is built, its
authority must come from a scoped client session issued per client: separate
credentials, proof of possession, replay protection, an explicit
owner/DM/project/agent scope, and immediate revocation. The panel must read a
backend-generated projection; it must never derive execution state itself,
and the projection must be filtered before it leaves the backend — no absolute
paths, no approval content, nothing outside the granted scope.

Today Robrix2 establishes no such session, has no Agent Operations transport,
and renders only the fail-closed contract status screen. The development
manifest is a gate input, not evidence that authentication or runtime wiring is
available.

## Why plane 2 cannot simply be widened into a global view

The two planes have **opposite** disclosure properties:

| | Plane 1 (Matrix) | Plane 2 (Router) |
| --- | --- | --- |
| Audience | every member of the room | the operator of that one backend |
| Content | conversation and task threads | local execution internals |
| Access model | Matrix room membership | future scoped client session, per backend |
| Failure of confinement | a message reaches the wrong room | one machine's internals reach everyone |

Publishing plane 2 onto plane 1 as-is would expose each operator's local
execution state — paths, dirty workspaces, failure details — to every member of
a shared room. That is why any future router access requires a scoped client
session rather than something a Matrix client gains merely by being logged in.

## If a cross-operator fleet view is wanted, it is a fourth thing

"See every agent that everyone has connected" is a legitimate goal, but it is
not this panel with a wider scope. It inverts the direction of data flow:

- **The panel pulls**: a client authenticates to one backend and reads that
  backend's private projection.
- **A fleet view would push**: each backend publishes a deliberately public,
  privacy-filtered summary into Matrix — as state events in a shared room —
  and any client aggregates what it can already see.

The push design is the better fit for the goal, for three reasons. Each
operator decides what leaves their machine, because publishing is an explicit
act rather than a permission granted to a reader. Matrix's existing access
model does the authorization, so no cross-operator client credential has to be
invented. And it degrades correctly: an operator who publishes nothing is
simply absent from the view rather than a hole in someone's permission model.

Nothing in the current design implements this. It would need its own event
schema, its own decision record, and its own answer to "what is safe to publish
about a machine you do not control".

## Consuming agent-chat's client contract

agent-chat has supplied a development manifest for the canonical client
contract. Robrix2 vendors that manifest under
`specs/fixtures/agent-ops-client-v1/manifest.json`. Local, non-canonical design
fixtures live separately under
`specs/fixtures/agent-ops-client-v1-proposal/`; they can never satisfy the
runtime gate.

The manifest currently reads:

```json
{ "release_status": "development", "source_commit": null }
```

That development status is deliberate, but it does not prove that a consumable
contract or runtime exists. **Robrix2 must stay fail-closed against exactly
this condition** — a manifest whose `release_status` is not a released value,
or whose `source_commit` is null, describes a moving target. Consuming it would
mean building a client against artifacts that can change underneath it, which
is how a client and a server silently drift apart.

The order is therefore: agent-chat releases the complete canonical set and
binds it to a real commit → Robrix2 vendors the manifest and every artifact →
the build verifies their exact paths and bytes → the contract becomes ready →
the client runtime is built and validated. Not before.

### How the gate enforces this

Robrix2 vendors agent-chat's manifest at
`specs/fixtures/agent-ops-client-v1/manifest.json` and **compiles it in**
(`include_str!`). The gate in `src/agent_ops/state.rs` judges that embedded
copy together with the artifact bytes compiled into the same build. It reports
the contract ready only when all of these are true:

- the manifest names this contract and is marked `released`;
- `source_commit` is a complete 40- or 64-character Git object ID;
- the manifest contains the complete V1 artifact set with safe, unique paths
  and valid SHA-256 values;
- the compiled artifact set exactly matches the manifest and every byte digest
  matches.

Anything else is a distinct, named closed state with a localized explanation
shown in the panel.

Embedding rather than reading at runtime means the gate's answer is a fact
pinned by Robrix2's own commit: no filesystem lookup, no environment
dependence, and no network call (the panel is forbidden any transport, and
tests enforce that).

**To make the contract ready** once agent-chat has released and committed: copy
the released `manifest.json` and every listed canonical artifact, then register
each file with `include_bytes!` in `src/agent_ops/state.rs`. The reviewed diff
therefore contains both the producer binding and the exact consumer bytes.
Changing only `release_status`, a hash string, or a local flag cannot open the
gate.

Contract readiness is deliberately not called “connected”: it proves only that
Robrix2 has pinned the protocol material. Authentication, transport, snapshot
loading, mutations, and the four operational views are still separate runtime
work and must pass a real canary before the UI can claim connectivity.

## References

- `specs/task-agent-ops-panel.spec.md` — this repository's panel contract
- `agent-chat/docs/AGENT-OPS-CLIENT.md` — the client contract
- `agent-chat/knowledge/decisions/adr-012-scoped-agent-ops-client-access.md`
- `agent-chat/knowledge/requirements/req-agent-ops-client-access.md`
- `agent-chat/docs/router-layer-design.md` — plane 2's internals
- `agent-chat/knowledge/decisions/adr-011-backend-owned-ephemeral-runner-sessions.md`
