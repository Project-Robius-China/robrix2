# Federation (Production Deployment)

[中文版](05-federation-production-deployment-zh.md)

> ### ⚠️ Advanced Content / Not Required to Run Locally
>
> This document is **advanced material**. If you just want to run a working federation environment on your local machine (two Palpo nodes + one Octos bot), see [04-federation-with-palpo.md](04-federation-with-palpo.md) -- that document is **fully self-contained** and does not depend on anything here.
>
> The purpose of this document is to tell you, when you deploy to a **real server** for outside-world use, which bits must change from "local testing mode" to "production mode" and why.

> **Goal:** After following this guide, your Palpo server will be deployed on a real domain with Let's Encrypt TLS and a reverse proxy, capable of federating with public Matrix servers like `matrix.org`.

This document covers **production** Matrix federation deployment -- real domain, trusted TLS certificates, reverse proxy, DNS setup, and security hardening.

---

## 🔀 Local Testing vs. Production: Full Diff Table

This table is the **core value** of this document -- for every piece of the working local environment in Doc 04, it spells out what has to change in production.

| Aspect | Local testing (Doc 04) | Production (this doc) | Why |
|--------|------------------------|------------------------|-----|
| **Domain / hostname** | | | |
| `server_name` | `palpo-1:8448` / `palpo-2:8448` | `matrix.example.com` | Public servers must be DNS-resolvable |
| How servers find each other | Docker network DNS aliases | Real DNS A record + (optional) SRV record | Docker aliases only work inside containers |
| **TLS certificates** | | | |
| Cert type | Self-signed via `openssl req -x509` | Trusted CA (Let's Encrypt, etc.) | Remote servers reject self-signed certs |
| Cert validation | `allow_invalid_tls_certificates = true` | **Not set** or explicitly `false` | Production requires real validation |
| Cert management | Manual generation (one-time) | Caddy automatic / certbot periodic renewal | Let's Encrypt certs expire after 90 days |
| **Network ports** | | | |
| C-S API outward | `localhost:6001` / `localhost:6002` | `https://matrix.example.com` (443) | Public needs standard ports + HTTPS |
| Federation API outward | `localhost:6401` / `localhost:6402` | `matrix.example.com:8448` | Other servers reach via 8448 or 443 + well-known |
| Reverse proxy | Not used | Caddy / Nginx (strongly recommended) | Centralizes TLS, well-known, rate limits |
| TLS termination | Palpo itself (`[[listeners]] [listeners.tls]`) | Caddy / Nginx terminates; Palpo runs plain HTTP 8008 | Reverse proxy unifies cert management |
| **well-known configuration** | | | |
| `[well_known].server` | `localhost:6401` (convenient for host-side debugging) | `matrix.example.com:443` | Production advertises real endpoints |
| `[well_known].client` | `http://localhost:6001` | `https://matrix.example.com` | Production enforces HTTPS |
| well-known server | Served by Palpo built-in | Served directly by Caddy (or from Palpo backend) | Caddy is more flexible, independent of Palpo restarts |
| **Security-related** | | | |
| `allow_registration` | `true` (convenient for testing) | `false` (create accounts first, then lock) | Prevents account spam |
| `yes_i_am_very_very_sure...` | `true` | Remove or `false` | Production should never use unconditional registration |
| Database password | `palpo:palpo` (fixed weak password) | Strong random in `.env` | Prevents DB compromise on exposure |
| API keys (e.g., DeepSeek) | Written directly in `compose.yml` / `config.json` | `.env` environment variables, with `.gitignore` | Avoids accidental git check-in |
| Firewall | Doesn't matter (local) | Only open 443 / 8448 | Internal ports should not be public |
| **Logging and operations** | | | |
| Log format | `pretty` (human-readable) | `json` (machine-collectable) | Structured logs enable alerting |
| `RUST_LOG` | `debug` | `info` or `warn` | Lower I/O overhead in production |
| Data persistence | Docker volumes sufficient | Regular Postgres + media backups | Production data loss is catastrophic |
| **Federation access control** | | | |
| `[federation].enable` | `true` | `true` | Same |
| `[federation].allowed_servers` | Not set (wide open) | Optional allowlist for federation partners | Internal servers may only federate with specific peers |
| `[federation].denied_servers` | Not set | Optional blocklist for malicious servers | Used for spam/ban control |
| `trusted_servers` | Not set | `["matrix.org"]` | Production needs notary servers to help validate remote keys |
| **Bot (Octos) configuration** | | | |
| `botfather.json` `server_name` | `palpo-1:8448` | `matrix.example.com` | Production uses real domain |
| `botfather.json` `homeserver` | `http://palpo-1:8008` | `http://palpo:8008` (still Docker-internal name) | Bot connects to Palpo through Docker net, not public internet |
| AppService namespace regex | `@bot:palpo-1:8448` | `@octosbot:matrix\\.example\\.com` | Match real MXID format |
| `allowed_senders` | `[]` (wide open) | `[]` or explicit allowlist | Production may restrict who can use the bot |

> **Important principle:** In the table above, **only `server_name` and a few security-related fields MUST change.** Settings like `homeserver` pointing to a Docker-internal name (`http://palpo:8008`) stay the **same** in both local and production -- because Octos always connects to Palpo via the Docker network, never through the public internet.

---

## 📚 Scope of This Document

| Scenario | This document | Other documents |
|----------|---------------|-----------------|
| **Production federation** | ✅ This document | -- |
| **Local federation testing** (Docker DNS, self-signed certs) | ❌ | [04-federation-with-palpo.md](04-federation-with-palpo.md) |
| Single-node local deployment | ❌ | [01-deploying-palpo-and-octos.md](01-deploying-palpo-and-octos.md) |

> **Prerequisite:** It is recommended to finish the local dual-node federation test in [Document 04](04-federation-with-palpo.md) first. That way you will already understand concepts like `server_name`, `well-known`, and federation ports before deploying to production. This document assumes you have a server with administrator access and the ability to manage DNS for a real domain.

---

## Table of Contents

1. [Prerequisites for Production](#1-prerequisites-for-production)
2. [Overall Architecture](#2-overall-architecture)
3. [Domain and DNS Setup](#3-domain-and-dns-setup)
4. [Reverse Proxy (Caddy Example)](#4-reverse-proxy-caddy-example)
5. [Production `palpo.toml`](#5-production-palpotoml)
6. [Docker Compose Changes](#6-docker-compose-changes)
7. [AppService Registration Update](#7-appservice-registration-update)
8. [Launch and Verification](#8-launch-and-verification)
9. [Using Federation](#9-using-federation)
10. [Troubleshooting](#10-troubleshooting)
11. [Further Reading](#11-further-reading)

---

## 1. Prerequisites for Production

Production federation has stricter infrastructure requirements than local testing:

| Requirement | Local testing (Doc 04) | Production (this doc) |
|-------------|------------------------|------------------------|
| Domain name | Not needed (Docker DNS alias) | **Required** (e.g., `matrix.example.com`) |
| TLS certificate | Self-signed (`allow_invalid_tls_certificates = true`) | **Required** (trusted CA like Let's Encrypt) |
| Port 443 | Not needed | **Open** (Client-Server API) |
| Port 8448 | Docker-internal only | **Open** (Server-Server Federation API) |
| Reverse proxy | Not needed | **Recommended** (Caddy / Nginx) |
| DNS records | Not needed | A record required, SRV record optional |
| Public IP | Not needed | **Required** |

> **⚠️ Self-signed certificates will NOT work in production federation.** Other Matrix servers will reject the TLS connection and federation messages will fail to deliver. Use a trusted CA like [Let's Encrypt](https://letsencrypt.org/).

---

## 2. Overall Architecture

A typical production topology:

```
                  Internet
                      │
                      │ 443 / 8448
                      ▼
              ┌───────────────┐
              │  Caddy        │   ← Reverse proxy + auto TLS
              │  (host)       │
              └───────┬───────┘
                      │ localhost:8008
                      ▼
┌─────────── Docker network ─────────┐
│   ┌──────────────┐                 │
│   │ Palpo        │                 │
│   │ server_name: │                 │
│   │ matrix.      │                 │
│   │ example.com  │                 │
│   └──────┬───────┘                 │
│          │                         │
│          ▼ AppService              │
│   ┌──────────────┐    ┌─────────┐  │
│   │ Octos        │    │ Postgres│  │
│   └──────────────┘    └─────────┘  │
└─────────────────────────────────────┘
```

**Key design choices:**

1. Caddy listens on the public 443 port, handling TLS termination and automatic Let's Encrypt certificate renewal
2. Palpo only exposes plain HTTP 8008 internally and is proxied by Caddy
3. Clients (Robrix/Element) connect to `matrix.example.com` over HTTPS
4. Other federated servers connect to `matrix.example.com:8448` (or 443 + well-known delegation)

---

## 3. Domain and DNS Setup

### 3.1 Register a Domain

Register a domain (e.g., `example.com`) and allocate a subdomain for Matrix, such as `matrix.example.com`.

### 3.2 Create a DNS A Record

Point the subdomain to your server's public IP:

```
matrix.example.com.   IN  A   203.0.113.10
```

### 3.3 (Optional) Create an SRV Record

You need an SRV record if federation runs on a non-default port, or if you want federation for `example.com` to route to `matrix.example.com`:

```
_matrix-fed._tcp.example.com.   IN  SRV   10 0 8448 matrix.example.com.
```

> **When can I skip SRV?** If you serve `/.well-known/matrix/server` on `matrix.example.com:443`, Matrix clients discover the federation endpoint through well-known delegation without SRV. Most production deployments use this approach.

### 3.4 Production DNS Settings (palpo.toml)

When running inside a Docker network, use TCP for DNS resolution:

```toml
query_over_tcp_only = true       # UDP DNS in container networks can be unreliable
query_all_nameservers = true     # Query all configured DNS servers to avoid single point of failure
ip_lookup_strategy = 5           # 5 = try IPv4 first, then IPv6
```

---

## 4. Reverse Proxy (Caddy Example)

A reverse proxy is **strongly recommended** in production because:

1. **Automatic TLS** -- Caddy has built-in Let's Encrypt, handling certificate issuance and renewal automatically
2. **well-known endpoint management** -- Caddy responds directly, without depending on Palpo
3. **Traffic control** -- rate limiting, logging, WAF integration all hang off nicely

### 4.1 Caddyfile

```caddyfile
matrix.example.com {
    # Matrix client discovery endpoint
    handle /.well-known/matrix/client {
        header Access-Control-Allow-Origin "*"
        respond `{"m.homeserver":{"base_url":"https://matrix.example.com"}}`
    }

    # Matrix federation discovery endpoint
    handle /.well-known/matrix/server {
        respond `{"m.server":"matrix.example.com:443"}`
    }

    # Everything else: proxy to Palpo
    reverse_proxy localhost:8008
}

# If federation uses a non-443 port, add another block:
# matrix.example.com:8448 {
#     reverse_proxy localhost:8008
# }
```

### 4.2 Disable Palpo's Own TLS

Let Caddy exclusively handle TLS; Palpo runs plain HTTP internally:

```toml
# palpo.toml
[tls]
enable = false
```

### 4.3 Nginx Alternative

If you must use Nginx, manage certificates separately with `certbot`:

```bash
# Request certificate
sudo certbot --nginx -d matrix.example.com

# Cert paths
# /etc/letsencrypt/live/matrix.example.com/fullchain.pem
# /etc/letsencrypt/live/matrix.example.com/privkey.pem
```

Then write `ssl_certificate` / `ssl_certificate_key` in your Nginx config and proxy `/` to `http://127.0.0.1:8008`, with explicit `location` blocks for `/.well-known/matrix/server` and `/.well-known/matrix/client`.

---

## 5. Production `palpo.toml`

```toml
# ── Core configuration ─────────────────────────
# CHANGED: real domain
server_name = "matrix.example.com"

# CHANGED: disable open registration in production
# Create admin accounts first, then set to false
allow_registration = false

enable_admin_room = true
appservice_registration_dir = "/var/palpo/appservices"

# Do NOT enable self-signed cert bypass in production!
# allow_invalid_tls_certificates stays at default (false)

# ── Listeners ──────────────────────────────────
# Caddy proxies port 443 to here
[[listeners]]
address = "0.0.0.0:8008"

# ── Logging ────────────────────────────────────
[logger]
format = "json"         # CHANGED: JSON logs for production (easier to collect)
level = "info"

# ── Database ───────────────────────────────────
[db]
url = "postgres://palpo:<strong-password>@palpo_postgres:5432/palpo"
pool_size = 10

# ── well-known (Palpo serves it; can be omitted if Caddy handles it) ─
[well_known]
server = "matrix.example.com:443"
client = "https://matrix.example.com"

# ── Federation settings ────────────────────────
[federation]
enable = true
allow_inbound_profile_lookup = true    # Allow remote servers to query local user profiles
# Optional access control:
# allowed_servers = ["matrix.org", "*.trusted.com"]
# denied_servers = ["evil.com"]

# ── TLS (recommended to disable, let Caddy handle it) ─
[tls]
enable = false
# If not using a reverse proxy:
# enable = true
# cert = "/path/to/fullchain.pem"
# key = "/path/to/privkey.pem"
# dual_protocol = false

# ── Presence and typing (cross-federation indicators) ─
[presence]
allow_local = true
allow_incoming = true
allow_outgoing = true

[typing]
allow_incoming = true
allow_outgoing = true
federation_timeout = 30000

# ── Trusted servers (Perspectives key notaries) ─
trusted_servers = ["matrix.org"]

# ── DNS tuning (recommended inside containers) ─
query_over_tcp_only = true
query_all_nameservers = true
ip_lookup_strategy = 5
```

### 5.1 `[federation]` Field Reference

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable` | bool | `true` | Master switch for federation |
| `allow_loopback` | bool | `false` | Allow federation requests to self (dev only) |
| `allow_device_name` | bool | `false` | Expose device names via federation; disable for privacy |
| `allow_inbound_profile_lookup` | bool | `true` | Allow remote servers to query local user profiles |
| `allowed_servers` | list | none | Allowlist, supports wildcards like `*.example.com`; not set = allow all |
| `denied_servers` | list | `[]` | Blocklist, **takes precedence over** `allowed_servers` |

### 5.2 Trusted Servers (Perspectives Key Validation)

`trusted_servers` act as **notary servers** that help validate other servers' signing keys. This is the [Perspectives key validation](https://spec.matrix.org/latest/server-server-api/#querying-keys-through-another-server) mechanism.

```toml
trusted_servers = ["matrix.org"]
```

The most common choice is `matrix.org` since it is the central hub of the public federation.

---

## 6. Docker Compose Changes

> **Baseline note:** This section compares against the **single-node** deployment (`palpo-and-octos-deploy/compose.yml`, which uses `server_name = "127.0.0.1:8128"`), not the dual-node federation from Doc 04. Production topology is almost always a single homeserver with outward federation — structurally closer to single-node than to Doc 04's local-simulation topology. If you started from Doc 04, ignore the port/service-name specifics and focus on the **rightmost column** — production values apply identically.

Key differences versus the local `compose.yml`:

```yaml
services:
  palpo:
    # image / build section same as local
    ports:
      - "8008:8008"          # Caddy proxies here (no more 8128 exposure)
    volumes:
      - ./palpo.toml:/var/palpo/palpo.toml:ro
      - ./appservices:/var/palpo/appservices:ro
      - ./data/media:/var/palpo/media
      # If Palpo handles TLS directly (not recommended), mount certs:
      # - /etc/letsencrypt/live/matrix.example.com:/certs:ro
    restart: unless-stopped
    # ... rest same as local ...

  palpo_postgres:
    environment:
      POSTGRES_PASSWORD: ${DB_PASSWORD}   # CHANGED: read strong password from .env
      POSTGRES_USER: palpo
      POSTGRES_DB: palpo
    # ...

  octos:
    environment:
      DEEPSEEK_API_KEY: ${DEEPSEEK_API_KEY}
      RUST_LOG: octos=info                # CHANGED: info rather than debug
    # ...
```

Overall structure stays the same as the local deployment -- Postgres, Palpo, Octos. Key differences:

| Local | Production |
|-------|------------|
| `server_name = "127.0.0.1:8128"` | `server_name = "matrix.example.com"` |
| Port `8128:8008` | Port `8008:8008` (proxied by Caddy) |
| Palpo self-manages TLS (or none) | Caddy terminates TLS |
| `allow_registration = true` | `allow_registration = false` |
| `pretty` logs | `json` logs |
| API keys inline in yml | Read from `.env` |

---

## 7. AppService Registration Update

When switching from `127.0.0.1:8128` to a real domain, update the following files.

### 7.1 AppService namespace file

Different local setups use different filenames:

| Starting point | Path |
|---------------|------|
| Single-node (Doc 01) | `palpo-and-octos-deploy/appservices/octos-registration.yaml` |
| Dual-node federation (Doc 04) | `palpo-and-octos-deploy/federation/nodes/node1/appservices/octos.yaml` |

Whichever you use, update the namespace regex to your real domain:

```yaml
namespaces:
  users:
    - exclusive: true
      regex: "@octosbot_.*:matrix\\.example\\.com"    # CHANGED: real domain
    - exclusive: true
      regex: "@octosbot:matrix\\.example\\.com"       # CHANGED: real domain
```

### 7.2 `config/botfather.json`

```json
{
  "config": {
    "channels": [{
      "type": "matrix",
      "homeserver": "http://palpo:8008",
      "server_name": "matrix.example.com",
      "sender_localpart": "octosbot",
      ...
    }]
  }
}
```

> **Important:** The `homeserver` URL **stays** as the Docker-internal address (`http://palpo:8008`) because Octos connects to Palpo through Docker networking -- it doesn't need to go through the public internet. Only `server_name` changes to the real domain, since that is the outward Matrix identity.

---

## 8. Launch and Verification

### 8.1 Starting the Services

```bash
cd palpo-and-octos-deploy   # or your production deploy directory

# Set environment variables
cp .env.example .env
vim .env    # fill in DEEPSEEK_API_KEY, DB_PASSWORD, etc.

# Start
docker compose up -d

# Check status
docker compose ps
docker compose logs -f
```

### 8.2 Test well-known Endpoints

```bash
# Server discovery (used by other federated servers)
curl https://matrix.example.com/.well-known/matrix/server
# Expected: {"m.server":"matrix.example.com:443"}

# Client discovery (used by Robrix/Element)
curl https://matrix.example.com/.well-known/matrix/client
# Expected: {"m.homeserver":{"base_url":"https://matrix.example.com"}}
```

### 8.3 Matrix Federation Tester

Visit [https://federationtester.matrix.org](https://federationtester.matrix.org) and enter your domain (`matrix.example.com`). It checks:

- DNS resolution correctness
- TLS certificate trust chain
- well-known endpoint responses
- Server-Server API reachability
- Signing key validation

All checks must pass before federation is fully operational.

---

## 9. Using Federation

Once federation is up, you can:

### 9.1 Join Rooms on Other Servers

In Robrix:

1. Click the **＋** button in the left nav bar to open **Add/Explore Rooms and Spaces**
2. In the bottom **Join an existing room or space** section, enter a target room alias (`#general:matrix.org`), ID (`!...:matrix.org`), or a `matrix:` link, then click **Go**
3. Your server reaches `matrix.org` through federation and joins
4. Messages from all participating servers sync in real time

### 9.2 Invite Users from Other Servers

1. Open one of your rooms and use its invite action (right-click the room or open its info panel, depending on the Robrix version)
2. Enter the remote user's MXID: `@friend:other-server.com`
3. The invitation is sent via federation to the remote server
4. After they accept, they join your room

### 9.3 Cross-Federation AI Bot

With federation enabled, users from **other servers** can also interact with your Octos bot:

1. A user on `matrix.org` invites `@octosbot:matrix.example.com` to their room
2. The invitation is delivered to your server via federation
3. Octos accepts and joins the room
4. The bot responds to messages -- even though the room lives on a remote server

> **Note:** For this to work, `allowed_senders` in `botfather.json` must be an empty array `[]` (allow all users) or explicitly include the remote user's MXID (e.g., `@remoteuser:matrix.org`).

---

## 10. Troubleshooting

### 10.1 Common Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| Can't join rooms on other servers | Federation disabled or port blocked | Check `[federation] enable = true`; open firewall ports 443 and 8448 |
| "Unable to find signing key" | TLS or DNS issue | Certs must be from a trusted CA (not self-signed); verify DNS |
| well-known returns 404 | Reverse proxy not forwarding | Check Caddy/Nginx handles `/.well-known/matrix/*` |
| Remote users can't see local user profiles | profile lookup disabled | Set `allow_inbound_profile_lookup = true` |
| Timeout connecting to remote server | Outbound firewall or DNS problem | Try `query_over_tcp_only = true`; verify reachability on 8448 |
| Bot doesn't reply to federated users | `allowed_senders` filter | Set to `[]` or add remote MXID explicitly |
| Federation Tester reports TLS error | Incomplete cert chain or expired | Ensure `fullchain.pem` includes intermediate certs; check expiry |

### 10.2 Debug Commands

```bash
# Palpo federation logs
docker compose logs palpo | grep -i federation

# Can Palpo reach other servers?
docker compose exec palpo curl -sf https://matrix.org/.well-known/matrix/server

# Verify well-known externally
curl -sf https://matrix.example.com/.well-known/matrix/server
curl -sf https://matrix.example.com/.well-known/matrix/client

# Certificate expiry check
openssl s_client -connect matrix.example.com:443 \
  -servername matrix.example.com < /dev/null 2>/dev/null \
  | openssl x509 -noout -dates

# Test Server-Server API version
curl -sf https://matrix.example.com:8448/_matrix/federation/v1/version
```

### 10.3 Security Checklist

Before going live in production:

- [ ] `allow_registration = false` (or registration token configured)
- [ ] `yes_i_am_very_very_sure...` removed or set to false
- [ ] Database password is not the default value
- [ ] `.env` file is in `.gitignore` (not committed)
- [ ] `allow_invalid_tls_certificates` is unset or false
- [ ] TLS certificate is from a trusted CA (not self-signed)
- [ ] Caddy/Nginx enforces HTTPS redirect
- [ ] Firewall only opens 443 / 8448 (not 8008)
- [ ] Logs are in `json` format (for collection and alerts)
- [ ] Data volumes have a regular backup policy (especially Postgres)

---

## 11. Further Reading

- **Matrix Federation Spec:** [spec.matrix.org/latest/server-server-api](https://spec.matrix.org/latest/server-server-api/) -- Server-Server protocol specification
- **Matrix Federation Tester:** [federationtester.matrix.org](https://federationtester.matrix.org/) -- Online federation validator
- **Palpo GitHub:** [github.com/palpo-im/palpo](https://github.com/palpo-im/palpo) -- Palpo server source
- **Let's Encrypt:** [letsencrypt.org](https://letsencrypt.org/) -- Free, automated TLS certificates
- **Caddy:** [caddyserver.com](https://caddyserver.com/) -- Reverse proxy with built-in auto-HTTPS
- **Certbot:** [certbot.eff.org](https://certbot.eff.org/) -- Nginx + Let's Encrypt tool

---

*This guide covers production deployment as of April 2026. Specific configuration fields may change with upstream updates; refer to each project's repository for the latest details.*
