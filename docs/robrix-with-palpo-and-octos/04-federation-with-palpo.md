# Federation (Local Two-Node Testing)

[中文版](04-federation-with-palpo-zh.md)

> **Goal:** After following this guide, you will run two federated Palpo nodes on your local machine: node 1 hosts the Octos AI bot, and node 2 hosts a regular user account. You will then chat **across servers** from the node-2 user to the node-1 bot using Robrix -- all without a public domain or real TLS certificates.

---

## 🚀 Quick Start (5 commands)

This repository ships with a **ready-to-run federation config** under `palpo-and-octos-deploy/federation/`. You do not need to write any configuration files yourself.

### Prerequisites

- Docker + Docker Compose installed
- `repos/palpo` and `repos/octos` cloned as described in [01-deploying-palpo-and-octos.md](01-deploying-palpo-and-octos.md)
- If the single-node deployment is running, stop it first: `cd palpo-and-octos-deploy && docker compose down`

### Run

```bash
cd palpo-and-octos-deploy/federation

# 1. Generate self-signed certs for both nodes (one-time)
./gen-certs.sh

# 2. Set the API key
cp .env.example .env
$EDITOR .env         # fill in DEEPSEEK_API_KEY

# 3. Build and start all 5 services
docker compose up -d --build

# 4. Watch status (palpo-1 / palpo-2 should become healthy)
docker compose ps
```

### Service endpoints (referenced in the steps below)

Each palpo container exposes **two endpoints**: the Client-Server API (plain HTTP, used by Robrix) and the Federation API (TLS, used by the other palpo node). The table below lists URLs and Matrix identity strings side by side — they are **not the same thing**, and new users most often trip over this distinction.

| Service | Client API URL (Robrix "Homeserver" field) | server_name (used in MXIDs) | Federation API (node-to-node) | Purpose |
|---------|-------------------------------------------|----------------------------|------------------------------|---------|
| **palpo-1** | `http://localhost:6001` | `palpo-1:8448` | `https://localhost:6401` | Hosts the Octos bot, account `@bot:palpo-1:8448` |
| **palpo-2** | `http://localhost:6002` | `palpo-2:8448` | `https://localhost:6402` | Hosts the regular user, you'll register `@alice:palpo-2:8448` below |

> **Rule of thumb:** Robrix login / curl HTTP calls use **the URL on the left**; the `palpo-X:8448` that appears inside MXIDs is an **identity label**, not a URL — don't mix them up.

### Register a user on palpo-2 (required)

On a fresh environment palpo-2 has no users yet. Click **Sign up** on the Robrix login screen and register alice with:

| Field | Value |
|-------|-------|
| Username | `alice` |
| Password | `test1234` |
| **Homeserver** | `http://localhost:6002` |

For screenshots and the full registration flow, see [02-using-robrix-with-palpo-and-octos.md Section 3: Registration](02-using-robrix-with-palpo-and-octos.md#3-registration).

### Verify federation works (optional, via curl)

If you want to confirm the federation handshake before opening Robrix, log alice in and query the bot's profile. This triggers a cross-server call from palpo-2 to palpo-1:

```bash
# Log alice in and grab a token
TOKEN=$(curl -s -X POST http://localhost:6002/_matrix/client/v3/login \
  -H "Content-Type: application/json" \
  -d '{"type":"m.login.password","identifier":{"type":"m.id.user","user":"alice"},"password":"test1234"}' \
  | jq -r .access_token)

# Query the bot profile on palpo-1 via palpo-2 (triggers federation)
curl -s "http://localhost:6002/_matrix/client/v3/profile/@bot:palpo-1:8448" \
  -H "Authorization: Bearer $TOKEN"
# Non-404 response = federation works
```

### Chat with the bot in Robrix

Log in to Robrix using the alice account you registered above:

| Field | Value |
|-------|-------|
| Username | `@alice:palpo-2:8448` |
| Password | `test1234` |
| **Homeserver** | `http://localhost:6002` |

After logging in, click the **＋** button in the left nav bar to open **Add/Explore Rooms and Spaces**. **Matrix user-directory search is scoped to each homeserver**, so you *cannot* find the palpo-1 bot via the search box -- use the middle **Add a friend** panel instead:

![Robrix Add a friend panel](../images/robrix-add-friend.png)

1. In the **Add a friend** input, enter the bot's full MXID: `@bot:palpo-1:8448`
2. Click **Add friend** -- Robrix creates a DM room through the palpo-2 → palpo-1 federation channel
3. Open the new room, send `hello`, and wait for the bot to reply

> **Why must you go through "Add a friend" instead of searching?**
>
> Matrix's user-directory search (`/user_directory/search`) **only indexes users known to the local homeserver**. palpo-2 was just started and only knows alice -- it has **no record of any palpo-1 user**, so the search box will never find `@bot` no matter what you type.
>
> "Add a friend" directly invokes `/createRoom` + `invite`: when palpo-2 receives the invite request, it **actively issues a federation request** to palpo-1 to verify the MXID exists and accepts invitations -- this is the **only correct entry point** for creating a cross-federation DM.
>
> This constraint is not Robrix-specific -- Element, SchildiChat, and every other Matrix client work the same way: full MXID + invite, never search.

**A successful reply confirms all three legs work: federation handshake, AppService forwarding, and the Octos LLM backend.**

---

## 📚 Further Reading in This Document

The quick start above is enough to finish a test run. The rest of this document explains **why the config works** so you can adjust it or debug problems.

| What you want to do | Where to look |
|---------------------|---------------|
| Just run it, but something broke | [Section 8: Troubleshooting](#8-troubleshooting) |
| Understand the architecture | [Section 2: Architecture](#2-local-two-node-architecture) + [Section 7: Message Flow](#7-message-flow-explained) |
| Modify the config (different ports, bot name, etc.) | [Section 4: Configuration Details](#4-configuration-details) |
| Deploy to a real server | [05-federation-production-deployment.md](05-federation-production-deployment.md) (advanced) |
| Single-node (no federation) | [01-deploying-palpo-and-octos.md](01-deploying-palpo-and-octos.md) |
| Use Palpo + Octos in Robrix | [02-using-robrix-with-palpo-and-octos.md](02-using-robrix-with-palpo-and-octos.md) |

---

## Table of Contents (advanced reading)

1. [What is Matrix Federation?](#1-what-is-matrix-federation)
2. [Local Two-Node Architecture](#2-local-two-node-architecture)
3. [File Layout](#3-file-layout)
4. [Configuration Details](#4-configuration-details)
5. [Startup Details](#5-startup-details)
6. [Alternative: API-level testing (for CI / headless scripting)](#6-alternative-api-level-testing-for-ci--headless-scripting)
7. [Message Flow Explained](#7-message-flow-explained)
8. [Troubleshooting](#8-troubleshooting)
9. [Next Steps](#9-next-steps)

---

## 1. What is Matrix Federation?

Matrix is a **decentralized** communication protocol. Each organization can run its own homeserver, and federation enables users on different homeservers to communicate seamlessly -- much like email:

- `@alice:server-a.com` can chat with `@bob:server-b.com` directly
- Each server stores its own users' data independently
- Messages are replicated across all servers participating in a conversation
- If one server goes down, the others keep working

Matrix clients connect via two distinct APIs:

| API | Default port | Purpose |
|-----|--------------|---------|
| **Client-Server API (C-S)** | 443 (or 8008) | Client (Robrix, Element) talks to its own homeserver |
| **Server-Server API (Federation)** | 8448 | Two homeservers talk to each other |

The local deployment guide only uses the C-S API -- servers are isolated. **Federation requires opening port 8448 as well, with TLS encryption.**

---

## 2. Local Two-Node Architecture

This document uses the dual-node Docker environment under `palpo-and-octos-deploy/federation/`. Two Palpo nodes discover each other over an internal Docker network (`palpo-federation`), with no public domain needed.

```
┌──── Docker network "palpo-federation" ─────────────────┐
│                                                         │
│   ┌──────────────────────┐     ┌──────────────────────┐│
│   │  palpo-1             │     │  palpo-2             ││
│   │  server_name:        │     │  server_name:        ││
│   │    palpo-1:8448      │◄───►│    palpo-2:8448      ││
│   │                      │ fed │                      ││
│   │  8008 → host:6001    │8448 │  8008 → host:6002    ││
│   │  8448 → host:6401    │     │  8448 → host:6402    ││
│   │  (TLS self-signed)   │     │  (TLS self-signed)   ││
│   └──────────┬───────────┘     └──────────────────────┘│
│              │                                          │
│              │ AppService (HTTP transaction)           │
│              ▼                                          │
│   ┌──────────────────────┐                             │
│   │  octos               │                             │
│   │  bot MXID:           │                             │
│   │    @bot:palpo-1:8448 │                             │
│   │  listens on 8009     │                             │
│   └──────────────────────┘                             │
│                                                         │
│   (postgres databases omitted)                          │
└─────────────────────────────────────────────────────────┘

                   Robrix (on host)
                   ↓ connects to localhost:6002
                   login as @alice:palpo-2:8448
                   ↓ send DM to @bot:palpo-1:8448
                   (delivered cross-server via federation)
```

### Port Allocation

| Service | Container port | Host port | Purpose |
|---------|---------------|-----------|---------|
| palpo-1 | 8008 | 6001 | Client-Server API (Robrix / curl direct) |
| palpo-1 | 8448 | 6401 | Federation API (external debug observation) |
| palpo-2 | 8008 | 6002 | Client-Server API |
| palpo-2 | 8448 | 6402 | Federation API |
| octos | 8009 | 8009 | AppService transaction receiver |

Containers communicate with each other through Docker network aliases (`palpo-1`, `palpo-2`, `octos`), not through host-exposed ports.

---

## 3. File Layout

Full deployment directory layout (checked into git + runtime-generated artifacts):

```
palpo-and-octos-deploy/federation/
├── docker-compose.yml              # 5 services: 2 palpo + 2 postgres + octos
├── palpo.Dockerfile                # Alpine-based Palpo builder + runtime image
├── gen-certs.sh                    # Generates self-signed TLS certs into certs/
├── .env.example                    # Template: DEEPSEEK_API_KEY, RUST_LOG
├── .gitignore                      # Excludes certs/, data/, nodes/*/media/
├── README.md                       # Per-folder quick overview
├── config/
│   ├── botfather.json              # Octos bot profile (channel + LLM + admin_mode)
│   └── octos.json                  # Octos runtime: profiles_dir, data_dir, log_level
├── nodes/
│   ├── node1/
│   │   ├── palpo.toml              # server_name = "palpo-1:8448"
│   │   ├── appservices/
│   │   │   └── octos.yaml          # AppService registration (octos namespace)
│   │   └── media/                  # [runtime] uploaded media, persistent
│   └── node2/
│       ├── palpo.toml              # server_name = "palpo-2:8448"
│       └── media/                  # [runtime]
├── certs/                          # [runtime] generated by gen-certs.sh
│   ├── node1.crt / node1.key
│   └── node2.crt / node2.key
└── data/                           # [runtime] postgres + octos persistence
    ├── pg-1/                       # palpo-1's postgres data
    ├── pg-2/                       # palpo-2's postgres data
    └── octos/                      # Octos state (mounted as /root/.octos)
```

> `[runtime]` folders are ignored by `.gitignore` — they appear after you run `./gen-certs.sh` and `docker compose up`.

---

## 4. Configuration Details

> **Purpose of this section:** The files used by the quick start are already in `palpo-and-octos-deploy/federation/`. This section explains **what the key fields do**, so you know what to change when customizing, and what's likely to trip you up.

### 4.1 Self-Signed Certificates (what `./gen-certs.sh` does)

The `gen-certs.sh` script runs effectively these commands:

```bash
# Generate cert for palpo-1. CN must match the hostname part of server_name
openssl req -x509 -nodes -newkey rsa:2048 -days 365 \
  -keyout certs/node1.key -out certs/node1.crt \
  -subj "/CN=palpo-1" \
  -addext "subjectAltName=DNS:palpo-1"

# Same for palpo-2
openssl req -x509 -nodes -newkey rsa:2048 -days 365 \
  -keyout certs/node2.key -out certs/node2.crt \
  -subj "/CN=palpo-2" \
  -addext "subjectAltName=DNS:palpo-2"
```

Key point: **the CN and subjectAltName must match the hostname part of `server_name` in `palpo.toml`** (here `palpo-1` / `palpo-2`), otherwise the TLS handshake fails with a hostname mismatch.

### 4.2 `docker-compose.yml`

> 📁 **Actual file:** [`palpo-and-octos-deploy/federation/docker-compose.yml`](../../palpo-and-octos-deploy/federation/docker-compose.yml) -- the snippet below shows the key structure; see the file for the complete content.

```yaml
services:
  # ── Node 1: hosts Octos AppService ────────────────────
  palpo-1:
    build:
      context: ..                   # uses palpo-and-octos-deploy/repos/palpo
      dockerfile: federation/palpo.Dockerfile
    image: palpo-federation:local-dev
    container_name: palpo-1
    depends_on:
      palpo-pg-1: { condition: service_healthy }
    volumes:
      - ./nodes/node1/palpo.toml:/var/palpo/palpo.toml:ro
      - ./nodes/node1/media:/var/palpo/media
      - ./nodes/node1/appservices:/var/palpo/appservices:ro
      - ./certs/node1.crt:/var/palpo/certs/node1.crt:ro
      - ./certs/node1.key:/var/palpo/certs/node1.key:ro
    environment:
      PALPO_CONFIG: /var/palpo/palpo.toml
      RUST_LOG: palpo=debug,palpo_core=info
    ports:
      - "6001:8008"               # C-S API
      - "6401:8448"               # Federation API
    networks:
      federation: { aliases: [palpo-1] }

  palpo-pg-1:
    image: postgres:16-alpine
    container_name: palpo-pg-1
    environment:
      POSTGRES_DB: palpo_node_1
      POSTGRES_USER: palpo
      POSTGRES_PASSWORD: palpo
    volumes: [pg-1-data:/var/lib/postgresql/data]
    networks: [federation]
    healthcheck:
      test: [CMD-SHELL, pg_isready -U palpo]
      interval: 5s
      retries: 10

  # ── Node 2: regular user (alice registered here) ──────
  palpo-2:
    build:                            # same build spec as palpo-1; Docker layer cache
      context: ..                     # makes the second build a no-op
      dockerfile: federation/palpo.Dockerfile
    image: palpo-federation:local-dev # same image tag as palpo-1
    container_name: palpo-2
    depends_on:
      palpo-pg-2: { condition: service_healthy }
    volumes:
      - ./nodes/node2/palpo.toml:/var/palpo/palpo.toml:ro
      - ./nodes/node2/media:/var/palpo/media
      - ./certs/node2.crt:/var/palpo/certs/node2.crt:ro
      - ./certs/node2.key:/var/palpo/certs/node2.key:ro
    environment:
      PALPO_CONFIG: /var/palpo/palpo.toml
      RUST_LOG: palpo=debug,palpo_core=info
    ports:
      - "6002:8008"
      - "6402:8448"
    networks:
      federation: { aliases: [palpo-2] }

  palpo-pg-2:
    image: postgres:16-alpine
    container_name: palpo-pg-2
    environment:
      POSTGRES_DB: palpo_node_2
      POSTGRES_USER: palpo
      POSTGRES_PASSWORD: palpo
    volumes: [pg-2-data:/var/lib/postgresql/data]
    networks: [federation]
    healthcheck:
      test: [CMD-SHELL, pg_isready -U palpo]
      interval: 5s
      retries: 10

  # ── Octos AppService (only connects to palpo-1) ───────
  octos:
    build:
      context: ../repos/octos     # your local Octos source (cloned by parent ./setup.sh)
      dockerfile: Dockerfile
    image: octos-federation:local-dev
    container_name: octos
    depends_on: [palpo-1]
    volumes:
      - ./data/octos:/root/.octos                                         # persistent state
      - ./config/botfather.json:/root/.octos/profiles/botfather.json:ro    # bot profile loaded by Octos
      - ./config/octos.json:/config/octos.json:ro                          # runtime config (profiles_dir, etc.)
    environment:
      DEEPSEEK_API_KEY: ${DEEPSEEK_API_KEY}
      RUST_LOG: ${RUST_LOG:-octos=debug,info}
    command: ["serve", "--host", "0.0.0.0", "--port", "8080", "--config", "/config/octos.json"]
    ports:
      - "8009:8009"               # AppService transaction receiver
      - "8010:8080"               # Octos dashboard / admin API
    networks:
      federation: { aliases: [octos] }

networks:
  federation:
    name: palpo-federation

volumes:
  pg-1-data:
  pg-2-data:
```

> **On the Octos location:** Just like the single-node deployment (`palpo-and-octos-deploy/`), this setup runs Octos inside the docker network. The AppService URL uses the service name `http://octos:8009`. This is simpler than "Octos on host with `host.docker.internal`" and closer to the production pattern.

### 4.3 `nodes/node1/palpo.toml`

> 📁 **Actual file:** [`palpo-and-octos-deploy/federation/nodes/node1/palpo.toml`](../../palpo-and-octos-deploy/federation/nodes/node1/palpo.toml)

```toml
# ── palpo-1: use Docker network alias as server_name ──
server_name = "palpo-1:8448"

allow_registration = true
yes_i_am_very_very_sure_i_want_an_open_registration_server_prone_to_abuse = true
enable_admin_room = true

# ── Local testing: accept self-signed certs ──
allow_invalid_tls_certificates = true

appservice_registration_dir = "/var/palpo/appservices"

# Client-Server API (plain HTTP, for Robrix / curl)
[[listeners]]
address = "0.0.0.0:8008"

# Federation API (TLS, for palpo-2)
[[listeners]]
address = "0.0.0.0:8448"
[listeners.tls]
cert = "/var/palpo/certs/node1.crt"
key = "/var/palpo/certs/node1.key"

[logger]
format = "pretty"

[db]
url = "postgres://palpo:palpo@palpo-pg-1:5432/palpo_node_1"
pool_size = 10

# ── Enable federation ──
[federation]
enable = true

# well-known: for host-side clients (e.g., Robrix C-S connection)
[well_known]
server = "localhost:6401"
client = "http://localhost:6001"
```

### 4.4 `nodes/node2/palpo.toml`

> 📁 **Actual file:** [`palpo-and-octos-deploy/federation/nodes/node2/palpo.toml`](../../palpo-and-octos-deploy/federation/nodes/node2/palpo.toml)

Almost identical to node1 -- only `server_name`, ports, database, and cert paths change:

```toml
server_name = "palpo-2:8448"

allow_registration = true
yes_i_am_very_very_sure_i_want_an_open_registration_server_prone_to_abuse = true
enable_admin_room = true
allow_invalid_tls_certificates = true

[[listeners]]
address = "0.0.0.0:8008"

[[listeners]]
address = "0.0.0.0:8448"
[listeners.tls]
cert = "/var/palpo/certs/node2.crt"
key = "/var/palpo/certs/node2.key"

[logger]
format = "pretty"

[db]
url = "postgres://palpo:palpo@palpo-pg-2:5432/palpo_node_2"
pool_size = 10

[federation]
enable = true

[well_known]
server = "localhost:6402"
client = "http://localhost:6002"
```

> **Note:** node2 has **no** `appservice_registration_dir` because Octos is registered only on node1 in this test setup.

### 4.5 `nodes/node1/appservices/octos.yaml`

> 📁 **Actual file:** [`palpo-and-octos-deploy/federation/nodes/node1/appservices/octos.yaml`](../../palpo-and-octos-deploy/federation/nodes/node1/appservices/octos.yaml)

This is the AppService registration file on palpo-1, telling Palpo: "any message matching `@bot_*:palpo-1:8448` or `@bot:palpo-1:8448` should be forwarded to Octos".

```yaml
id: octos-matrix-appservice
url: "http://octos:8009"          # Docker network service name
as_token: "436682e5f10a0113775779eb8fedf702a095254a95e229c7d20f085b9082903b"
hs_token: "ef642609a1a5b2eda1486a6bada6411f4e861691a7500b10ff26b5b2e16573fd"
sender_localpart: bot
rate_limited: false
namespaces:
  users:
    - exclusive: true
      regex: "@bot:palpo-1:8448"
    - exclusive: true
      regex: "@bot_.*:palpo-1:8448"
  aliases: []
  rooms: []
```

> **Generate your own tokens:** The `as_token` / `hs_token` above are for demonstration only. For production, use `openssl rand -hex 32` to generate an independent random value per token. For local testing you can copy-paste the sample values as-is.

### 4.6 `config/botfather.json` and `config/octos.json`

> 📁 **Actual files:**
> - [`palpo-and-octos-deploy/federation/config/botfather.json`](../../palpo-and-octos-deploy/federation/config/botfather.json) -- Octos bot profile (LLM settings + Matrix channel that binds it to palpo-1)
> - [`palpo-and-octos-deploy/federation/config/octos.json`](../../palpo-and-octos-deploy/federation/config/octos.json) -- Octos runtime config (where to find profiles / data)

`botfather.json` is an **Octos bot profile**. Octos loads it from `profiles_dir` at startup and uses it to wire the bot to an LLM backend and to a Matrix homeserver:

```json
{
  "id": "botfather",
  "name": "BotFather",
  "enabled": true,
  "config": {
    "provider": "deepseek",
    "model": "deepseek-chat",
    "api_key_env": "DEEPSEEK_API_KEY",
    "admin_mode": true,
    "channels": [
      {
        "type": "matrix",
        "homeserver": "http://palpo-1:8008",
        "as_token": "436682e5f10a0113775779eb8fedf702a095254a95e229c7d20f085b9082903b",
        "hs_token": "ef642609a1a5b2eda1486a6bada6411f4e861691a7500b10ff26b5b2e16573fd",
        "server_name": "palpo-1:8448",
        "sender_localpart": "bot",
        "user_prefix": "bot_",
        "port": 8009,
        "allowed_senders": []
      }
    ],
    "gateway": {
      "max_history": 50,
      "queue_mode": "followup"
    }
  }
}
```

Key fields:

- **`provider` / `model` / `api_key_env`** -- LLM backend; swap for any other Octos-supported provider and update `.env` accordingly
- **`admin_mode: true`** -- unlocks Octos admin commands (matches #86)
- **`channels[0].homeserver` vs `channels[0].server_name`** -- two different concepts:
  - `homeserver = "http://palpo-1:8008"` -- HTTP URL Octos uses to call the Client-Server API
  - `server_name = "palpo-1:8448"` -- Matrix identity; must match palpo-1's `palpo.toml`
- **`as_token` / `hs_token`** -- must match `nodes/node1/appservices/octos.yaml` exactly, otherwise palpo-1 refuses the AppService connection
- **`allowed_senders: []`** -- empty means any user (including federated) can DM the bot
- **`gateway.queue_mode: "followup"`** -- how Octos queues concurrent conversations (`followup` keeps replies threaded per room)

`octos.json` is much simpler -- it just tells the Octos daemon where to look:

```json
{
  "profiles_dir": "/root/.octos/profiles",
  "data_dir": "/root/.octos",
  "log_level": "debug"
}
```

The compose file mounts `config/botfather.json` into `/root/.octos/profiles/`, so Octos discovers it automatically on startup.

---

## 5. Startup Details

> The Quick Start already covers the basic commands. This section adds what to watch for and common first-run observations.

### 5.1 Expected Startup Sequence

```
1. palpo-pg-1 / palpo-pg-2   start and pass pg_isready
2. palpo-1 / palpo-2         connect to postgres, listen on 8008 + 8448
3. palpo-1                   loads /var/palpo/appservices/octos.yaml
4. octos                     logs into palpo-1 as @bot:palpo-1:8448
```

### 5.2 Health Checks and Logs

```bash
# 5 container statuses
docker compose ps
# palpo-pg-1 / palpo-pg-2  → healthy
# palpo-1 / palpo-2         → healthy
# octos                     → running

# palpo-2 reaching palpo-1 (federation handshake)
docker compose logs palpo-2 | grep -i "palpo-1"

# Octos login success
docker compose logs octos | grep -i "bot\|logged in"
```

### 5.3 First-Time Build Duration

Both palpo and octos are compiled from source. The first `docker compose up -d --build` may take 5-10 minutes. Subsequent restarts are 1-2 seconds (unless source changes). Docker BuildKit caches Rust artifacts, so crates aren't recompiled every time.

### 5.4 Disk Footprint & Cleanup

Federation mode runs **two** palpo images, **two** postgres instances, and **one** octos, so the footprint is larger than single-node:

| Stage | Size |
|---|---|
| Images (steady, layers shared across the two palpo instances) | ~3 GB |
| Build cache (first build peak) | ~5 GB (reclaimable) |
| Runtime data (`data/node1` + `data/node2`) | ~50-100 MB per node |

Clean up when `docker system df` shows too much reclaimable cache:

```bash
docker builder prune -af            # drop build cache (safe)
docker compose down -v              # stop + wipe data volumes
docker system prune -af --volumes   # nuclear: everything Docker-related
```

See [01 §5.5 Cleaning up Docker Cache](01-deploying-palpo-and-octos.md#55-cleaning-up-docker-cache) for the full explanation of why cache grows and which command to pick.

---

## 6. Alternative: API-level testing (for CI / headless scripting)

> **This section mirrors the Quick Start via raw HTTP calls.** Use it when you want to script the workflow in CI, reproduce bugs from a terminal, or see exactly what Robrix is doing under the hood. If you just want to run through the demo interactively, the [Quick Start](#-quick-start-5-commands) covers the same ground via the Robrix UI and this section can be skipped.

### 6.1 Register alice on palpo-2 (curl)

Equivalent to clicking **Sign up** in Robrix — uses the unauthenticated `m.login.dummy` flow enabled by `allow_registration = true` in `palpo.toml`:

```bash
curl -X POST http://localhost:6002/_matrix/client/v3/register \
  -H "Content-Type: application/json" \
  -d '{
    "username": "alice",
    "password": "test1234",
    "auth": {"type": "m.login.dummy"}
  }'
```

Expected response:

```json
{
  "user_id": "@alice:palpo-2:8448",
  "access_token": "...",
  "home_server": "palpo-2:8448",
  ...
}
```

### 6.2 Verify federation between palpo-2 and palpo-1 (curl)

Query the bot's profile on palpo-1 through palpo-2 — this triggers a server-to-server handshake and proves the federation channel works before you even open a UI client:

```bash
# 1) Log in as alice to get a token
TOKEN=$(curl -s -X POST http://localhost:6002/_matrix/client/v3/login \
  -H "Content-Type: application/json" \
  -d '{
    "type":"m.login.password",
    "identifier":{"type":"m.id.user","user":"alice"},
    "password":"test1234"
  }' | jq -r .access_token)

# 2) Query the bot's profile on palpo-1 (triggers federation)
curl -s "http://localhost:6002/_matrix/client/v3/profile/@bot:palpo-1:8448" \
  -H "Authorization: Bearer $TOKEN"
```

**Expected result:** returns `{"displayname": "...", "avatar_url": "..."}` or an empty object `{}`. A `404` means federation is broken — see [Section 8](#8-troubleshooting).

### 6.3 Chat with the bot

Once 6.1 has registered alice and 6.2 has confirmed federation works, the DM creation + chat flow is identical to the Quick Start. Follow [Chat with the bot in Robrix](#chat-with-the-bot-in-robrix) for the UI walkthrough (Add a friend → `@bot:palpo-1:8448` → send `hello`).

If you need to automate the DM creation itself (for CI), use Matrix's [`POST /_matrix/client/v3/createRoom`](https://spec.matrix.org/latest/client-server-api/#post_matrixclientv3createroom) with alice's token from 6.1 — `invite: ["@bot:palpo-1:8448"]` and `is_direct: true` will trigger the same cross-federation lookup that Robrix's "Add a friend" button does.

---

## 7. Message Flow Explained

When alice sends `hello` to the bot, the message takes this path:

```
┌─────────────────┐
│ Robrix (host)   │
│ @alice:palpo-2  │
└────────┬────────┘
         │ PUT /_matrix/client/v3/rooms/{id}/send/m.room.message
         │ target http://localhost:6002
         ▼
┌─────────────────────────────────────────────────────┐
│ palpo-2 container                                   │
│ Sees event targeting @bot:palpo-1:8448              │
│ server_name part is "palpo-1:8448"                  │
│ Docker DNS resolves palpo-1 → container IP          │
└────────┬────────────────────────────────────────────┘
         │ PUT https://palpo-1:8448/_matrix/federation/v1/send/{txn}
         │ TLS (self-signed, allow_invalid=true skips validation)
         ▼
┌─────────────────────────────────────────────────────┐
│ palpo-1 container (8448 TLS listener)               │
│ Receives federation event                           │
│ Checks MXID against AppService namespaces           │
│   @bot:palpo-1:8448 matches octos.yaml regex        │
└────────┬────────────────────────────────────────────┘
         │ PUT http://octos:8009/_matrix/app/v1/transactions/{txn}
         │ Authorization: Bearer <hs_token>
         ▼
┌─────────────────────────────────────────────────────┐
│ octos container                                     │
│ Parses event, recognizes "hello"                    │
│ Calls DeepSeek API for a reply                      │
└────────┬────────────────────────────────────────────┘
         │ PUT http://palpo-1:8008/_matrix/client/v3/rooms/{id}/send/...
         │ Authorization: Bearer <as_token> (acting as bot)
         ▼
┌─────────────────────────────────────────────────────┐
│ palpo-1 → federation back to palpo-2 → alice sees   │
└─────────────────────────────────────────────────────┘
```

**Key observations:**

1. Robrix only knows about `localhost:6002` -- it is **unaware** of federation. Federation happens entirely inside palpo-2
2. The `palpo-2 → palpo-1` hop goes through TLS on port 8448, as required by the Matrix spec
3. `palpo-1 → octos` is AppService HTTP, with no federation concept -- to palpo-1, octos is just a local event handler
4. Octos replies through palpo-1's C-S API (using `as_token` to impersonate the bot), not through federation

---

## 8. Troubleshooting

### 8.1 Diagnostic Checklist

| Symptom | Likely cause | What to check |
|---------|--------------|---------------|
| `docker compose up` fails | Port conflict | `lsof -i :6001 :6002 :6401 :6402 :8009` |
| **Robrix register returns "Account Creation Failed" / request hangs on a fresh setup** | The target palpo container isn't actually serving (stuck in `Restarting (127)` or `Exited`) | `docker compose ps` — if palpo is not `healthy`, run `docker compose logs palpo-2 \| tail -30`. Common root causes: missing `libgcc` in the runtime image (Rust's stack-unwinding runtime needs `libgcc_s.so.1`; this is why `palpo.Dockerfile` apk-installs `libgcc` — don't remove it), wrong cert paths in `palpo.toml`, or postgres not yet healthy when palpo started |
| Step 6.2 profile query returns 404 | Federation broken | `docker compose logs palpo-2 \| grep -i "fed\|palpo-1"` |
| Bot receives message but doesn't reply | Octos → palpo-1 connection issue | `docker compose logs octos \| tail -50` |
| Robrix login: "invalid homeserver" | Homeserver URL wrong | Must be `http://localhost:6002`, not `palpo-2:8448` |
| "user not found" when creating DM | Federation profile lookup failed | Check palpo-2 logs for TLS handshake and cert validation |
| Message sent but never arrives | Federation async queue backoff | `docker compose logs palpo-2 \| grep -i "send_txn\|backoff"` |
| Spurious "Failed to join: it has already been joined" popup after creating a DM with the bot | Race condition between `/createRoom` and sliding sync | Benign — the DM is already created; just dismiss. Fixed by #83 (open). |

### 8.2 Common Debug Commands

```bash
# Follow all service logs
docker compose logs -f

# Only federation-related logs
docker compose logs palpo-1 palpo-2 | grep -i "federation"

# From inside palpo-1, test reaching palpo-2
docker compose exec palpo-1 curl -k https://palpo-2:8448/_matrix/federation/v1/version

# Check AppService registration on palpo-1
docker compose exec palpo-1 ls -la /var/palpo/appservices/

# Restart a service (without restarting the database)
docker compose restart palpo-1 octos

# Nuke everything (this wipes all user and room data!)
docker compose down -v
```

### 8.3 Verify Octos Is Registered

```bash
# palpo-1 should have loaded the AppService at startup
docker compose logs palpo-1 | grep -i "appservice\|octos"

# Octos should be able to use bot token against palpo-1
docker compose exec octos \
  curl -s -H "Authorization: Bearer 436682e5f10a0113775779eb8fedf702a095254a95e229c7d20f085b9082903b" \
  http://palpo-1:8008/_matrix/client/v3/account/whoami
# Expected: {"user_id":"@bot:palpo-1:8448",...}
```

---

## 9. Next Steps

- **Move to production:** This document uses Docker DNS aliases + self-signed certs, which only work on a single machine. For real deployment you need a real domain, Let's Encrypt certs, a reverse proxy, etc. → [05-federation-production-deployment.md](05-federation-production-deployment.md)
- **Federate with the public Matrix network:** Once production is set up, you can talk to `matrix.org` and other public servers. Invite your bot to public rooms, or let `matrix.org` users DM it.
- **Extend Octos capabilities:** The bot supports multiple LLM backends, custom commands, knowledge bases, etc. See the Octos project docs.

---

## Further Reading

- **Matrix Federation Spec:** [spec.matrix.org/latest/server-server-api](https://spec.matrix.org/latest/server-server-api/) -- Server-Server API protocol details
- **AppService Spec:** [spec.matrix.org/latest/application-service-api](https://spec.matrix.org/latest/application-service-api/) -- AppService communication protocol
- **Palpo GitHub:** [github.com/palpo-im/palpo](https://github.com/palpo-im/palpo) -- Palpo source and configuration reference
- **Matrix Federation Tester:** [federationtester.matrix.org](https://federationtester.matrix.org/) -- Online federation checker (public domains only)

---

*This guide is based on Palpo and Octos as of April 2026. Configuration files may change with upstream updates; refer to each project's repository for the latest details.*
