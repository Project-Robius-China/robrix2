# FSF-0 Palpo profile

This directory owns the deterministic Palpo substrate used by the agent-chat
demo. Default tests are hermetic: they do not start Docker, Palpo, an agent, or
any remote service.

## Prepare configuration

```bash
cd roadmap/agentchat-demo/palpo
cp .env.example .env
chmod 600 .env
# Replace the six quoted placeholders. Generate each credential independently:
openssl rand -hex 32

set -a
. ./.env
set +a
node palpo-config.mjs "$(pwd)/.runtime/config"
```

For a team server, use the same files and change `PALPO_SERVER_NAME`,
`PALPO_PUBLIC_URL`, `PALPO_HOST_PORT`, and appservice URL in `.env`. Do not
commit `.env` or `.runtime/`.

## Start and stop

The Palpo source must exist at
`palpo-and-octos-deploy/repos/palpo`, matching the existing Dockerfile build
context.

```bash
docker compose --env-file .env --profile palpo-local up -d --build --wait
node bootstrap-accounts.mjs
node demo-doctor.mjs --json
docker compose --env-file .env --profile palpo-local down
```

Reset is deliberately separate from stop and requires explicit confirmation:

```bash
./demo-reset.sh --state-root "$(pwd)/.runtime" --confirm
# A reset removes rendered config and Palpo/Postgres data. Render again before restart:
node palpo-config.mjs "$(pwd)/.runtime/config"
docker compose --env-file .env --profile palpo-local up -d --build --wait
node bootstrap-accounts.mjs
node demo-doctor.mjs --json
```

Registration is protected by `PALPO_REGISTRATION_TOKEN`; unrestricted open
registration is disabled. The appservice sender is `_agentchat_appservice`,
separate from the password-login bot `agent-bridge`. On first bootstrap, the
admin account is registered through Matrix UIA and then promoted by one bounded
`users.is_admin` update through the local Compose Postgres service. Doctor logs
in with that account and verifies the role through Palpo's admin API.

## Contract tests

```bash
node --test tests/*.test.mjs
```

The five FSF acceptance selectors are present in `tests/real-e2e.test.mjs` and
are skipped by default. After reviewing the generated `.env`, run the isolated
real profile explicitly:

```bash
PALPO_REAL_E2E=1 node --test tests/real-e2e.test.mjs
```

This starts Docker and Palpo, but no Claude, Codex, agent-chat, or other agent
runtime.
