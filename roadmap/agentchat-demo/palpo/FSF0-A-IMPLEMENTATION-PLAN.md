# FSF-0A Palpo Deployment Foundation Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Provide a deterministic local/team Palpo profile with generated
appservice configuration, idempotent account bootstrap, machine-readable doctor
checks, and a safe reset path.

**Architecture:** Keep all new deployment artifacts under
`roadmap/agentchat-demo/palpo/`. A Node configuration tool renders TOML/YAML from
validated environment values into an ignored runtime directory. Docker Compose
consumes only rendered files. Node's built-in test runner uses fake Matrix HTTP
servers and temporary directories, so default verification never starts Palpo,
Docker, Claude, or an agent runtime.

**Tech Stack:** Docker Compose, POSIX shell, Node.js ESM and `node:test`, Palpo
Matrix Client-Server API.

## Global Constraints

- Do not modify the existing Makepad UI files or run `cargo fmt`/`rustfmt`.
- Do not commit real tokens, passwords, access tokens, or signing keys.
- Local and team profiles use the same Compose file and rendered templates.
- Tests must not start Docker, Palpo, Claude, agent-chat, Matrix, or remote services.
- Real container execution remains an explicit operator action after tests pass.

---

### Task 1: Deterministic Configuration Renderer

**Files:**
- Create: `roadmap/agentchat-demo/palpo/.env.example`
- Create: `roadmap/agentchat-demo/palpo/templates/palpo.toml.tpl`
- Create: `roadmap/agentchat-demo/palpo/templates/appservice-agentchat.yaml.tpl`
- Create: `roadmap/agentchat-demo/palpo/palpo-config.mjs`
- Test: `roadmap/agentchat-demo/palpo/tests/palpo-config.test.mjs`

**Interfaces:**
- Consumes: validated environment values and an explicit output directory.
- Produces: `renderConfig({ env, outputDir })` and immutable rendered
`palpo.toml`, `appservice-agentchat.yaml`, and `deployment.json` files. Palpo
registration is token-gated; open dummy registration is forbidden.

- [x] **Step 1: Write renderer failure tests**

Test missing/placeholder secrets, invalid port/server names, unsafe YAML/TOML
characters, and token leakage in `deployment.json`.

- [x] **Step 2: Run tests and observe missing module failure**

Run: `node --test roadmap/agentchat-demo/palpo/tests/palpo-config.test.mjs`
Expected: FAIL because `palpo-config.mjs` does not exist.

- [x] **Step 3: Implement strict rendering**

Validate server name, public URL, port, database values, appservice URL, sender
localpart, and two non-placeholder secrets. Replace only closed template markers;
write files with mode `0600`; store only SHA-256 secret fingerprints in
`deployment.json`.

- [x] **Step 4: Run renderer tests**

Run: `node --test roadmap/agentchat-demo/palpo/tests/palpo-config.test.mjs`
Expected: PASS.

### Task 2: Parameterized Compose Profile

**Files:**
- Create: `roadmap/agentchat-demo/palpo/compose.yml`
- Create: `roadmap/agentchat-demo/palpo/README.md`
- Modify: `roadmap/agentchat-demo/.gitignore`
- Test: `roadmap/agentchat-demo/palpo/tests/compose-contract.test.mjs`

**Interfaces:**
- Consumes: rendered config directory and the existing
  `palpo-and-octos-deploy/palpo.Dockerfile` build context.
- Produces: `palpo-local` Compose profile with healthy Postgres and Palpo
  services, parameterized host port/data/config paths, and no embedded secret.

- [x] **Step 1: Write Compose contract test**

Verify the profile name, health dependencies, read-only config/appservice
mounts, parameterized host port, and absence of literal token/password values.

- [x] **Step 2: Run test and observe missing Compose failure**

Run: `node --test roadmap/agentchat-demo/palpo/tests/compose-contract.test.mjs`
Expected: FAIL because `compose.yml` does not exist.

- [x] **Step 3: Add Compose and operator documentation**

Use one profile for local and team-server settings. Document render, start,
doctor, stop, and reset commands without invoking agent runtimes.

- [x] **Step 4: Run Compose contract test**

Run: `node --test roadmap/agentchat-demo/palpo/tests/compose-contract.test.mjs`
Expected: PASS.

### Task 3: Idempotent Account Bootstrap

**Files:**
- Create: `roadmap/agentchat-demo/palpo/bootstrap-accounts.mjs`
- Test: `roadmap/agentchat-demo/palpo/tests/bootstrap-accounts.test.mjs`

**Interfaces:**
- Consumes: homeserver URL plus explicit account name/password list.
- Produces: `bootstrapAccounts({ homeserver, accounts, registrationToken,
  promoteAdmin, fetchImpl })` with one result per account and no printed
  password/access token. The CLI promotes the configured first admin with one
  bounded, idempotent update through the local Compose Postgres service.

- [x] **Step 1: Write fake-homeserver tests**

Cover token-gated first registration, second-run login without another account,
registration race, wrong existing password, admin promotion, and partial failure
continuation.

- [x] **Step 2: Run test and observe missing module failure**

Run: `node --test roadmap/agentchat-demo/palpo/tests/bootstrap-accounts.test.mjs`
Expected: FAIL because bootstrap module does not exist.

- [x] **Step 3: Implement Matrix dummy-registration/login flow**

Try password login first; register through `m.login.registration_token` only
when absent; retry login after an `M_USER_IN_USE` race; promote and verify the
configured admin; return redacted structured results.

- [x] **Step 4: Run bootstrap tests**

Run: `node --test roadmap/agentchat-demo/palpo/tests/bootstrap-accounts.test.mjs`
Expected: PASS.

### Task 4: Doctor and Safe Reset

**Files:**
- Create: `roadmap/agentchat-demo/palpo/demo-doctor.mjs`
- Create: `roadmap/agentchat-demo/palpo/demo-reset.sh`
- Test: `roadmap/agentchat-demo/palpo/tests/demo-doctor.test.mjs`
- Test: `roadmap/agentchat-demo/palpo/tests/reset-contract.test.mjs`

**Interfaces:**
- Consumes: rendered deployment manifest, homeserver URL, account credentials,
  and injected fetch/command functions.
- Produces: `runDoctor` JSON/text checks for homeserver, loaded appservice
  credential, admin role, and bot login with non-zero failure status, plus a
  reset command that requires an explicit state root plus confirmation flag.

- [x] **Step 1: Write doctor/reset negative tests**

Cover unreachable homeserver, appservice token fingerprint mismatch, missing bot
account, wrong bot password, healthy state, path traversal, root/empty reset
paths, and dry-run behavior.

- [x] **Step 2: Run tests and observe missing module/script failures**

Run: `node --test roadmap/agentchat-demo/palpo/tests/demo-doctor.test.mjs roadmap/agentchat-demo/palpo/tests/reset-contract.test.mjs`
Expected: FAIL.

- [x] **Step 3: Implement structured doctor and guarded reset**

Doctor reports exact failing dependency and emits JSON with no secrets. Reset
runs Compose down, validates the state root is below the demo directory, removes
only generated config/state, and is idempotent.

- [x] **Step 4: Run doctor/reset tests**

Run: `node --test roadmap/agentchat-demo/palpo/tests/demo-doctor.test.mjs roadmap/agentchat-demo/palpo/tests/reset-contract.test.mjs`
Expected: PASS.

### Task 5: FSF-0 A Contract Verification

**Files:**
- Modify: `roadmap/agentchat-demo/palpo/README.md`
- Modify: `roadmap/agentchat-demo/README.md`

**Interfaces:**
- Consumes: Tasks 1-4 outputs and OpenFab
  `specs/phase1/fsf0-a-palpo-deploy.spec.md`.
- Produces: one default test command and exact operator commands for the opt-in
  real profile.

- [x] **Step 1: Run all Node tests**

Run: `node --test roadmap/agentchat-demo/palpo/tests/*.test.mjs`
Expected: unit/contract tests PASS and five real acceptance selectors are
explicitly SKIPPED with no container started.

- [x] **Step 2: Validate shell and configuration artifacts**

Run: `bash -n roadmap/agentchat-demo/palpo/demo-reset.sh`
Expected: PASS.

Run: `docker compose -f roadmap/agentchat-demo/palpo/compose.yml config`
Expected: PASS when Docker Compose is installed; otherwise record the tool as an
operator preflight dependency without starting services.

- [x] **Step 3: Validate the source task contract**

Run: `agent-spec parse /Users/zhangalex/Work/Projects/FW/openfab/specs/phase1/fsf0-a-palpo-deploy.spec.md`

Run: `agent-spec lint /Users/zhangalex/Work/Projects/FW/openfab/specs/phase1/fsf0-a-palpo-deploy.spec.md --min-score 0.7`

Expected: parse succeeds and lint quality is 100%.

- [ ] **Step 4: Present for user testing**

Do not commit or open a PR. Report the exact render/start/doctor/reset commands
and wait for user confirmation as required by the Robrix2 repository rules.

The opt-in real acceptance command is:

`PALPO_REAL_E2E=1 node --test roadmap/agentchat-demo/palpo/tests/real-e2e.test.mjs`

It starts Docker/Palpo only, uses an isolated Compose project and runtime, and
does not start Claude, Codex, agent-chat, or another agent runtime.
