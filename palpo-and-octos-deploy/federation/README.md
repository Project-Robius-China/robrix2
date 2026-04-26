# Local Federation + Octos (Two-Node) — Ready-to-Run

[English](README.md) | [中文文档](../../docs/robrix-with-palpo-and-octos/04-federation-with-palpo-zh.md)

A **self-contained, copy-and-run** local Matrix federation setup:

- Two Palpo homeservers (`palpo-1` and `palpo-2`) that federate with each other
- Octos AI bot registered on `palpo-1` as an AppService (MXID `@bot:palpo-1:8448`)
- A user on `palpo-2` (`@alice:palpo-2:8448`) can chat with the bot across the federation
- All services run in Docker; no public domain or real TLS certificates required

## Prerequisites

- Docker + Docker Compose
- ~4 GB disk (for images and data)
- Palpo and Octos source already cloned to `../repos/palpo` and `../repos/octos` (same location the parent single-node setup uses)

```bash
# From the palpo-and-octos-deploy/ parent directory:
git clone https://github.com/palpo-im/palpo.git repos/palpo
git clone https://github.com/octos-org/octos.git repos/octos
```

## Quick Start

```bash
cd palpo-and-octos-deploy/federation

# 1. Generate self-signed certificates (one-time)
./gen-certs.sh

# 2. Set your DeepSeek API key
cp .env.example .env
$EDITOR .env        # fill in DEEPSEEK_API_KEY

# 3. Build and start everything
docker compose up -d --build

# 4. Watch the logs until all services are healthy
docker compose ps
docker compose logs -f
```

Expected final state:

| Container | Status |
|-----------|--------|
| `palpo-1` | healthy |
| `palpo-2` | healthy |
| `palpo-pg-1` | healthy |
| `palpo-pg-2` | healthy |
| `octos` | running |

## Testing the Federation

### Step 1: Register alice on palpo-2

```bash
curl -X POST http://localhost:6002/_matrix/client/v3/register \
  -H "Content-Type: application/json" \
  -d '{"username":"alice","password":"test1234","auth":{"type":"m.login.dummy"}}'
```

Expected: returns an `access_token`, with `user_id = "@alice:palpo-2:8448"`.

### Step 2: Verify federation works (without Robrix)

```bash
# Log in as alice
TOKEN=$(curl -s -X POST http://localhost:6002/_matrix/client/v3/login \
  -H "Content-Type: application/json" \
  -d '{"type":"m.login.password","identifier":{"type":"m.id.user","user":"alice"},"password":"test1234"}' \
  | jq -r .access_token)

# Query the bot on palpo-1 via palpo-2 (this triggers a federation call)
curl -s "http://localhost:6002/_matrix/client/v3/profile/@bot:palpo-1:8448" \
  -H "Authorization: Bearer $TOKEN"
```

Expected: returns `{"displayname": ...}` or an empty `{}`. A `404` means federation is not working — see Troubleshooting below.

### Step 3: Log in from Robrix

| Field | Value |
|-------|-------|
| Username | `@alice:palpo-2:8448` |
| Password | `test1234` |
| **Homeserver URL** | `http://localhost:6002` ← HTTP URL, not the MXID server part |

### Step 4: Chat with the bot

1. New Direct Message → `@bot:palpo-1:8448`
2. Send "hello"
3. The bot responds (through DeepSeek)

## Port Map

| Service | Container port | Host port | Purpose |
|---------|---------------|-----------|---------|
| palpo-1 | 8008 | 6001 | Client-Server API |
| palpo-1 | 8448 | 6401 | Federation API (TLS) |
| palpo-2 | 8008 | 6002 | Client-Server API |
| palpo-2 | 8448 | 6402 | Federation API (TLS) |
| octos | 8009 | 8009 | AppService transactions |
| octos | 8080 | 8010 | Octos dashboard/admin API |

## Directory Layout

```
federation/
├── README.md                           # This file
├── .env.example                        # Template for .env
├── .gitignore                          # Excludes certs/ and data/
├── docker-compose.yml                  # 5 services: 2 palpo + 2 postgres + octos
├── palpo.Dockerfile                    # Palpo build recipe
├── gen-certs.sh                        # One-shot cert generation
├── certs/                              # Self-signed TLS (gitignored)
│   ├── node1.crt  node1.key
│   └── node2.crt  node2.key
├── data/                               # Postgres + media volumes (gitignored)
├── nodes/
│   ├── node1/
│   │   ├── palpo.toml                  # server_name = "palpo-1:8448"
│   │   ├── appservices/
│   │   │   └── octos.yaml              # AppService registration for Octos
│   │   └── media/
│   └── node2/
│       ├── palpo.toml                  # server_name = "palpo-2:8448"
│       └── media/
└── config/
    ├── octos.json                      # Octos core config
    └── botfather.json                  # Matrix channel profile (targets palpo-1)
```

## Troubleshooting

| Symptom | Check |
|---------|-------|
| Ports 6001/6002/8009 already in use | Parent single-node setup running? `cd .. && docker compose down` |
| Step 2 returns 404 | `docker compose logs palpo-2 \| grep -i fed` |
| Bot receives but doesn't reply | `docker compose logs octos \| tail -50` |
| Robrix login fails | Make sure Homeserver URL is `http://localhost:6002`, not a MXID |
| TLS errors | `allow_invalid_tls_certificates = true` must be set in both palpo.toml files |

Clean restart (wipes user data):

```bash
docker compose down -v
rm -rf data/ certs/
./gen-certs.sh
docker compose up -d --build
```

## Differences vs the Parent Single-Node Setup

| | Single-node (parent dir) | This federation setup |
|-|--|--|
| Palpo nodes | 1 | 2 |
| server_name | `127.0.0.1:8128` | `palpo-1:8448` / `palpo-2:8448` |
| TLS | None | Self-signed on port 8448 |
| Federation | Off | On |
| Host ports | 8128 | 6001, 6002, 6401, 6402 |
| Octos | Registered on `127.0.0.1:8128` | Registered on `palpo-1:8448` only |

## Further Reading

- Full guide (Chinese): [04-federation-with-palpo-zh.md](../../docs/robrix-with-palpo-and-octos/04-federation-with-palpo-zh.md)
- Full guide (English): [04-federation-with-palpo.md](../../docs/robrix-with-palpo-and-octos/04-federation-with-palpo.md)
- Production deployment: [05-federation-production-deployment-zh.md](../../docs/robrix-with-palpo-and-octos/05-federation-production-deployment-zh.md)
