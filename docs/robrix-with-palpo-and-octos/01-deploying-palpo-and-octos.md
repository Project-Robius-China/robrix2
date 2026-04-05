# Deployment Guide: Robrix + Palpo + Octos

[中文版](01-deploying-palpo-and-octos-zh.md)

> **Goal:** After following this guide, you will have Palpo (Matrix homeserver), Octos (AI bot), and PostgreSQL running via Docker Compose on your machine. Robrix will be able to connect to your Palpo server, and you can chat with the Octos AI bot.

This guide walks you through deploying the backend services step by step: from cloning the source code, to configuring each component, to verifying everything works end-to-end.

> **Just want to try it quickly?** Jump to [Quick Start](#2-quick-start) -- 5 steps to get running.
>
> **Want to understand WHY things are configured this way?** See [Architecture](02-how-robrix-palpo-octos-work-together.md) for the full explanation.

---

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Quick Start](#2-quick-start)
3. [Configuration Details](#3-configuration-details)
4. [End-to-End Verification](#4-end-to-end-verification)
5. [Troubleshooting](#5-troubleshooting)
6. [Further Reading](#6-further-reading)

---

## 1. Prerequisites

Before starting, make sure you have:

| Requirement | Version | Notes |
|-------------|---------|-------|
| **Docker** + **Docker Compose** | v2+ | `docker compose version` to check. Docker Desktop includes Compose v2. |
| **Git** | Any | For cloning source repos. |
| **An LLM API key** | -- | e.g., [DeepSeek](https://platform.deepseek.com/) (free tier available), OpenAI, Anthropic, etc. |
| **Robrix** | Latest | See [Getting Started with Robrix](../robrix/getting-started-with-robrix.md) for download or build instructions. |

> **Note:** Palpo and Octos are both built from source inside Docker. You do not need to install Rust or any other toolchain on your host machine.

---

## 2. Quick Start

Get everything running locally in 5 steps.

### Step 1: Clone the Repo

```bash
git clone https://github.com/Project-Robius-China/robrix2.git
cd robrix2/palpo-and-octos-deploy
```

### Step 2: Run the Setup Script

```bash
./setup.sh
```

This script:
- Clones the Palpo source repo into `repos/palpo/` (shallow clone from GitHub)
- Clones the Octos source repo into `repos/octos/` (shallow clone from GitHub)
- Creates your `.env` file from `.env.example`

Both services are built from source inside Docker to support all architectures (x86_64, ARM64/Apple Silicon, etc.).

> **Where do files go?** After running `setup.sh` and `docker compose up`, the `palpo-and-octos-deploy/` directory will contain:
> - `repos/` — Palpo and Octos source code (used by Docker to build images)
> - `data/` — runtime data (PostgreSQL database, Octos sessions, media files)
> - `.env` — your environment variables (API key, etc.)
>
> These directories are listed in `.gitignore` and will **not** be committed to the repository.

### Step 3: Set Your API Key

Edit `.env` and replace `your-api-key-here` with your actual API key:

```
DEEPSEEK_API_KEY=sk-xxxxxxxxxxxxxxxxxxxxxxxx
```

### Step 4: Start the Services

```bash
docker compose up -d
```

> **Important:** The first run builds both Palpo and Octos from source, which can take **10--30 minutes** depending on your machine and network speed. Palpo compiles its Rust codebase; Octos additionally downloads runtime tools (Node.js, Chromium) for its skill plugins. Subsequent runs use cached images and start in seconds.

Check that everything is running:

```bash
docker compose ps
```

You should see three services (`palpo_postgres`, `palpo`, `octos`) all in `running` state.

### Step 5: Connect with Robrix

1. **Open Robrix** (see [Getting Started with Robrix](../robrix/getting-started-with-robrix.md) if you don't have it yet)

2. **Set the homeserver**: In the login screen, enter `http://127.0.0.1:8128` in the **Homeserver URL** field

3. **Register a new account**: Enter a username and password, then click **Sign up**

4. **Talk to the AI bot**: After logging in, create a room and invite the bot:
   - Click the invite button in the room
   - Enter `@octosbot:127.0.0.1:8128`
   - Wait a moment for the bot to join the room (you should see a join event)
   - Send a message -- the AI bot should reply!

**That's it!** You now have a working Robrix + Palpo + Octos setup. Read on for configuration details, or jump to [Troubleshooting](#5-troubleshooting) if something isn't working.

---

## 3. Configuration Details

This section explains every configuration file in the `palpo-and-octos-deploy/` directory. You already have a working setup from the Quick Start -- come here when you want to customize.

> **Note:** To understand the architecture and WHY each component is configured this way, see [Architecture](02-how-robrix-palpo-octos-work-together.md).

### 3.1 Directory Layout

```
palpo-and-octos-deploy/
├── compose.yml                         # Docker Compose -- orchestrates all services
├── setup.sh                            # One-time setup script
├── .env.example                        # Environment variables template
├── palpo.toml                          # Palpo homeserver configuration
├── palpo.Dockerfile                    # Palpo Docker build (multi-stage, release)
├── appservices/
│   └── octos-registration.yaml         # Appservice registration (links Palpo <-> Octos)
├── config/
│   ├── botfather.json                  # Octos bot profile (LLM + Matrix channel config)
│   └── octos.json                      # Octos global settings
├── repos/                              # Source code (created by setup.sh, gitignored)
│   ├── palpo/                          # Palpo homeserver source
│   └── octos/                          # Octos bot source
├── data/                               # Persistent data (created at runtime, gitignored)
│   ├── pgsql/                          # PostgreSQL database files
│   ├── octos/                          # Octos runtime data
│   └── media/                          # Palpo media storage
```

### 3.2 Token Generation

The Appservice registration and the Octos bot profile share two secret tokens for mutual authentication. The example files come with pre-filled development tokens, but **you must generate new tokens for production**:

```bash
openssl rand -hex 32   # -> use as as_token
openssl rand -hex 32   # -> use as hs_token
```

These two values must be identical in `palpo-and-octos-deploy/appservices/octos-registration.yaml` and `palpo-and-octos-deploy/config/botfather.json`. If they don't match, the bot will not work. See [3.8 Token Matching Checklist](#38-token-matching-checklist).

### 3.3 Appservice Registration (`appservices/octos-registration.yaml`)

This file tells Palpo about Octos -- which user namespaces Octos manages and where to send events.

```yaml
id: octos-matrix-appservice
url: "http://octos:8009"

as_token: "d1f46062a08e4833b18286d95c5e09a5f3e4a1b2c3d4e5f6a7b8c9d0e1f2a3b4"
hs_token: "e2a57173b19f5944c29397ea6d6f1ab6a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9"

sender_localpart: octosbot
rate_limited: false

namespaces:
  users:
    - exclusive: true
      regex: "@octosbot_.*:127\\.0\\.0\\.1:8128"
    - exclusive: true
      regex: "@octosbot:127\\.0\\.0\\.1:8128"
  aliases: []
  rooms: []
```

| Field | Description |
|-------|-------------|
| `id` | A unique identifier for this appservice registration. |
| `url` | Where Palpo sends events. Uses the Docker service name `octos` (not `localhost`), because both containers share the same Docker network. |
| `as_token` | Token that Octos uses when calling Palpo's API. **Must match** `botfather.json`. |
| `hs_token` | Token that Palpo uses when pushing events to Octos. **Must match** `botfather.json`. |
| `sender_localpart` | The bot's Matrix local username. Becomes `@octosbot:127.0.0.1:8128`. |
| `rate_limited` | Set to `false` so the bot can respond without rate limits. |
| `namespaces.users` | Regex patterns for user IDs that this appservice owns. Include the bot itself (`@octosbot:...`) and any dynamically-created bot users (`@octosbot_*:...`). |

### 3.4 Palpo Configuration (`palpo.toml`)

```toml
server_name = "127.0.0.1:8128"

allow_registration = true
yes_i_am_very_very_sure_i_want_an_open_registration_server_prone_to_abuse = true
enable_admin_room = true

appservice_registration_dir = "/var/palpo/appservices"

# HTTP listener (Client-Server API)
[[listeners]]
address = "0.0.0.0:8008"

[logger]
format = "pretty"

[db]
url = "postgres://palpo:palpo_dev_password@palpo_postgres:5432/palpo"
pool_size = 10

[well_known]
server = "127.0.0.1:8128"
client = "http://127.0.0.1:8128"
```

| Field | Description |
|-------|-------------|
| `server_name` | The domain part of all Matrix IDs (e.g., `@user:127.0.0.1:8128`). |
| `allow_registration` | Whether new users can register. Set to `true` for Robrix users to create accounts. |
| `yes_i_am_very_very_sure_...` | Required safety confirmation when `allow_registration = true`. |
| `enable_admin_room` | Enables the server admin room for management. |
| `appservice_registration_dir` | Palpo loads all `.yaml` files from this directory on startup. This is how it discovers Octos. |
| `[[listeners]]` | Network listeners. Each entry defines an address Palpo listens on. |
| `[logger]` | Log format. `"pretty"` for development, `"json"` for production. |
| `[db]` | PostgreSQL connection. `palpo_postgres` is the Docker service name. The password must match `POSTGRES_PASSWORD` in `compose.yml`. |
| `[well_known]` | Used by clients for server discovery. Must match externally-reachable addresses. |

> **Note:** The `server_name` `"127.0.0.1:8128"` is for local development only. For production deployment, replace it with your actual domain name (e.g., `"chat.example.com"`). When you change `server_name`, you must also update it in `octos-registration.yaml` (the regex patterns) and `botfather.json` (`server_name` field).

> **Important:** In this local Docker setup, the Matrix identity is `127.0.0.1:8128`, so `server_name`, the appservice regex, and bot user IDs must all use `127.0.0.1:8128`. Only container-to-container traffic uses Docker service names like `palpo:8008` or `octos:8009`.

### 3.5 Octos Bot Profile (`config/botfather.json`)

This file defines the bot's identity, LLM provider, and Matrix channel configuration.

```json
{
  "id": "botfather",
  "name": "BotFather",
  "enabled": true,
  "config": {
    "provider": "deepseek",
    "model": "deepseek-chat",
    "api_key_env": "DEEPSEEK_API_KEY",
    "channels": [
      {
        "type": "matrix",
        "homeserver": "http://palpo:8008",
        "as_token": "d1f46062a08e4833b18286d95c5e09a5f3e4a1b2c3d4e5f6a7b8c9d0e1f2a3b4",
        "hs_token": "e2a57173b19f5944c29397ea6d6f1ab6a4b5c6d7e8f9a0b1c2d3e4f5a6b7c8d9",
        "server_name": "127.0.0.1:8128",
        "sender_localpart": "octosbot",
        "user_prefix": "octosbot_",
        "port": 8009,
        "allowed_senders": []
      }
    ],
    "gateway": {
      "max_history": 50,
      "queue_mode": "followup"
    }
  },
  "created_at": "2025-01-01T00:00:00Z",
  "updated_at": "2025-01-01T00:00:00Z"
}
```

> **Important:** The `created_at` and `updated_at` fields are **required** by Octos. If they are missing, Octos will skip this profile and the bot will never start.

**LLM Provider settings:**

| Field | Description |
|-------|-------------|
| `provider` | LLM provider name. Octos supports `deepseek`, `openai`, `anthropic`, and [more](https://octos-org.github.io/octos/). |
| `model` | Model identifier (e.g., `deepseek-chat`, `gpt-4o`, `claude-sonnet-4-20250514`). |
| `api_key_env` | Name of the environment variable holding your API key. |

**Matrix channel settings:**

| Field | Description |
|-------|-------------|
| `type` | Must be `"matrix"`. |
| `homeserver` | Palpo's internal URL. Uses Docker service name `palpo`, not `localhost`. |
| `as_token` / `hs_token` | Must match the appservice registration YAML. |
| `server_name` | The Matrix domain. Must match `server_name` in `palpo.toml`. |
| `sender_localpart` | Bot username. Must match the registration file. |
| `user_prefix` | Prefix for dynamically-created bot users (e.g., `octosbot_translator`). |
| `port` | Port Octos listens on for Appservice events from Palpo. |
| `allowed_senders` | Matrix user IDs allowed to talk to the bot. Empty `[]` = everyone. |

> **Important:** `homeserver` is the internal Docker URL Octos uses to call Palpo. `server_name` is the Matrix domain embedded in user IDs. They are related but not interchangeable. See [Architecture](02-how-robrix-palpo-octos-work-together.md) for why.

**Gateway settings:**

| Field | Description |
|-------|-------------|
| `max_history` | Maximum number of messages to include as context for the LLM. |
| `queue_mode` | How Octos handles incoming messages. `followup` queues new messages and processes them sequentially. |

**Switching LLM Provider (example: OpenAI instead of DeepSeek):**

1. In `botfather.json`, change: `"provider": "openai"`, `"model": "gpt-4o"`, `"api_key_env": "OPENAI_API_KEY"`
2. In `.env`, change: `OPENAI_API_KEY=sk-xxxxxxxx`
3. In `compose.yml`, add to the `octos` service's `environment`: `OPENAI_API_KEY: ${OPENAI_API_KEY}`

Octos supports 14+ providers — see [Octos Book](https://octos-org.github.io/octos/) for the full list.

### 3.6 Octos Global Settings (`config/octos.json`)

This file configures Octos's core runtime paths and logging.

```json
{
  "profiles_dir": "/root/.octos/profiles",
  "data_dir": "/root/.octos",
  "log_level": "debug"
}
```

| Field | Description |
|-------|-------------|
| `profiles_dir` | Directory where Octos loads bot profiles (like `botfather.json`). Mapped via Docker volume from `./config/`. |
| `data_dir` | Root directory for Octos runtime data (sessions, memory). Mapped from `./data/octos/`. |
| `log_level` | Octos log verbosity. Use `debug` for development, `info` for production. |

> **Note:** These are container-internal paths. The Docker volume mappings in `compose.yml` connect them to the host directories.

### 3.7 Docker Compose (`compose.yml`)

The provided `compose.yml` starts three services:

| Service | Image | Exposed Ports | Purpose |
|---------|-------|--------------|---------|
| `palpo_postgres` | `postgres:17` | *(none, internal only)* | Database for Palpo |
| `palpo` | Built from source | `8128:8008` | Matrix homeserver |
| `octos` | Built from source | `8009:8009`, `8010:8080` | AI bot appservice |

**Port mapping explanation:**

- `8128` -- Robrix connects here (Client-Server API)
- `8009` -- Palpo pushes events to Octos here (Appservice API, also exposed to host for debugging)
- `8010` -- Octos admin dashboard (optional, for monitoring)

**Persistent volumes:**

| Volume | Purpose |
|--------|---------|
| `./data/pgsql` | PostgreSQL data. Survives `docker compose down`. |
| `./data/octos` | Octos runtime data (sessions, memory). |
| `./data/media` | Media files uploaded through Matrix (images, files). |

**Environment variables (`.env`):**

| Variable | Required | Default | Description |
|----------|----------|---------|-------------|
| `DEEPSEEK_API_KEY` | **Yes** | -- | Your LLM API key |
| `DB_PASSWORD` | No | `palpo_dev_password` | PostgreSQL password |
| `RUST_LOG` | No | `octos=debug,info` | Log verbosity |

### 3.8 Token Matching Checklist

The most common configuration error is a token mismatch. These values **must be identical** across files:

| Value | In `octos-registration.yaml` | In `botfather.json` |
|-------|------------------------------|---------------------|
| `as_token` | `as_token: "d1f4..."` | `"as_token": "d1f4..."` |
| `hs_token` | `hs_token: "e2a5..."` | `"hs_token": "e2a5..."` |
| `sender_localpart` | `sender_localpart: octosbot` | `"sender_localpart": "octosbot"` |
| `server_name` | regex: `@octosbot:127\\.0\\.0\\.1:8128` | `"server_name": "127.0.0.1:8128"` |

If any of these don't match, the bot will not respond to messages. Double-check before filing a bug report!

---

## 4. End-to-End Verification

After setting up, run through this checklist to confirm everything works.

### Service Health Check

```bash
# Check all containers are running
docker compose ps

# Check Palpo logs for startup errors
docker compose logs palpo | tail -20

# Check Octos logs -- look for "appservice listening" or similar
docker compose logs octos | tail -20

# Verify Palpo is responding to the Matrix API
curl -s http://127.0.0.1:8128/_matrix/client/versions | head -5
```

### Client Connectivity Checklist

- [ ] Robrix can connect to `http://127.0.0.1:8128`
- [ ] You can register a new account
- [ ] After login, the room list loads (may be empty for a fresh account)
- [ ] You can create a new room

### Bot Interaction Checklist

- [ ] You can invite `@octosbot:127.0.0.1:8128` to a room
- [ ] The bot joins the room (check `docker compose logs octos` if it doesn't)
- [ ] Sending a message triggers a response from the bot
- [ ] The response content makes sense (confirms LLM connection works)

### Log Checking Order (Follow the Data Flow)

If something fails, check the logs in the order that data flows through the system:

```bash
# 1. Is Palpo receiving messages from Robrix?
docker compose logs palpo --since 1m

# 2. Is Palpo forwarding events to Octos?
docker compose logs palpo --since 1m | grep -i appservice

# 3. Is Octos receiving and processing events?
docker compose logs octos --since 1m

# 4. Is Octos successfully calling the LLM?
docker compose logs octos --since 1m | grep -i -E "deepseek|llm|provider"
```

---

## 5. Troubleshooting

### 5.1 Service Startup Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| `palpo_postgres` won't start | Port 5432 already in use, or corrupt data | Check `docker compose logs palpo_postgres`. Remove `data/pgsql/` to start fresh. |
| `palpo` build fails | Network issue or missing source | Ensure Docker can reach `github.com`. Check `docker compose logs palpo` for build errors. |
| `palpo` crashes on startup | Bad `palpo.toml` syntax or DB connection failure | Check logs. Ensure `palpo_postgres` is healthy first. Verify DB password matches. |
| `octos` build fails | Missing Dockerfile or network issue | Ensure Docker can reach `github.com`. Run `./setup.sh` to verify repos are cloned. |
| `octos` starts but logs show errors | Invalid `botfather.json` or missing API key | Check JSON syntax. Verify `DEEPSEEK_API_KEY` is set in `.env`. |

### 5.2 Robrix Connection Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| "Cannot connect to server" | Wrong homeserver URL or Palpo not running | Verify Palpo is running (`docker compose ps`). Confirm URL is `http://127.0.0.1:8128`. |
| Login succeeds but no rooms appear | Normal for a fresh account | Create a new room. Rooms appear as you join or create them. |
| Registration fails | `allow_registration = false` in `palpo.toml` | Check `palpo.toml`. Ensure `allow_registration = true`. |
| "Homeserver does not support Sliding Sync" | Palpo version too old | Rebuild Palpo: `docker compose build --no-cache palpo`. |
| Connection times out | Firewall blocking port 8128 | Check firewall rules. On macOS, allow incoming connections in System Settings. |

### 5.3 Bot Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| Bot does not respond to messages | Token mismatch between registration and profile | Verify the [Token Matching Checklist](#38-token-matching-checklist). |
| `Connection refused` in Palpo logs | Octos not running, or wrong `url` in registration YAML | Ensure Octos is running. The `url` must use Docker service name (`http://octos:8009`), not `localhost`. |
| `User ID not in namespace` | `sender_localpart` doesn't match `namespaces.users` regex | Update the regex in `octos-registration.yaml` to include the bot's full user ID pattern. |
| Bot joins room but gives empty replies | LLM API key invalid or quota exceeded | Check `docker compose logs octos` for API errors. Verify your API key and account balance. |
| Messages from some users are ignored | `allowed_senders` filtering in `botfather.json` | Set `allowed_senders` to `[]` to allow everyone, or add the user's Matrix ID. |
| Bot profile not loading | Missing `created_at` / `updated_at` in `botfather.json` | These fields are required. Add them as shown in section [3.5](#35-octos-bot-profile-configbotfatherjson). |

### 5.4 Useful Debug Commands

```bash
# View real-time logs for all services
docker compose logs -f

# View logs for a specific service
docker compose logs -f palpo
docker compose logs -f octos

# Restart a single service (e.g., after editing botfather.json)
docker compose restart octos

# Rebuild a single service (e.g., after updating source)
docker compose build --no-cache palpo
docker compose up -d palpo

# Check Palpo's Client-Server API
curl http://127.0.0.1:8128/_matrix/client/versions

# Full reset (WARNING: deletes all data including accounts and messages)
docker compose down -v
rm -rf data/
docker compose up -d
```

---

## 6. Further Reading

- **Octos Documentation (full):** [octos-org.github.io/octos](https://octos-org.github.io/octos/) -- covers all LLM providers, channels, skills, memory system, and advanced configuration.
- **Octos Matrix Appservice Guide:** [octos-org/octos#171](https://github.com/octos-org/octos/pull/171) -- the original Palpo + Octos integration guide this document builds upon.
- **Palpo:** [github.com/palpo-im/palpo](https://github.com/palpo-im/palpo) -- Palpo homeserver documentation.
- **Robrix:** [Project-Robius-China/robrix2](https://github.com/Project-Robius-China/robrix2) -- Robrix client, build instructions, and feature tracker.
- **Matrix Appservice Spec:** [spec.matrix.org -- Application Service API](https://spec.matrix.org/latest/application-service-api/) -- the Matrix protocol specification for application services.
- **Architecture Guide:** [02-how-robrix-palpo-octos-work-together.md](02-how-robrix-palpo-octos-work-together.md) -- how the Appservice mechanism works, message lifecycle, and BotFather system.

---

*This guide covers the deployment as of April 2026. For the latest updates, see the respective project repositories.*
