# Federation: Cross-Server Communication

[中文版](04-federation-with-palpo-zh.md)

> **Goal:** After following this guide, you will have Palpo configured for Matrix federation, enabling users on your server to communicate with users on other Matrix servers (like matrix.org), and allowing remote users to access your Octos AI bots.

This guide covers Matrix **federation** -- connecting your Palpo homeserver with other Matrix servers so users on different servers can communicate with each other.

> **Prerequisite:** You should already have a working local deployment. If not, see [01-deploying-palpo-and-octos.md](01-deploying-palpo-and-octos.md) first.

---

## Table of Contents

1. [What is Matrix Federation?](#1-what-is-matrix-federation)
2. [Prerequisites for Federation](#2-prerequisites-for-federation)
3. [Palpo Federation Configuration](#3-palpo-federation-configuration)
4. [Production Deployment](#4-production-deployment)
5. [Using Federation](#5-using-federation)
6. [Verification and Troubleshooting](#6-verification-and-troubleshooting)
7. [Further Reading](#7-further-reading)

---

## 1. What is Matrix Federation?

Matrix is a **decentralized** communication protocol. Each organization can run its own homeserver, and federation allows users on different homeservers to communicate seamlessly.

Think of it like email:

- `@alice:server-a.com` can chat with `@bob:server-b.com`
- Each server stores its own users' data
- Messages are replicated across all servers participating in a conversation
- No single point of control -- if one server goes down, others keep working

In the local deployment guide, everything runs on `127.0.0.1:8128` -- a single isolated server. Federation opens your server to the wider Matrix network.

```
  Server A                         Server B
┌──────────┐   Federation API    ┌──────────┐
│  Palpo   │ ◄────────────────►  │  Synapse  │
│  + Octos │   (port 8448)       │  or any   │
│  + Robrix│                     │  Matrix   │
└──────────┘                     └──────────┘
  @alice:server-a.com              @bob:server-b.com
       └─── can chat with ─────────────┘
```

---

## 2. Prerequisites for Federation

Federation has requirements beyond a local deployment:

| Requirement | Local Deployment | Federated Deployment |
|-------------|-----------------|---------------------|
| Domain name | Not needed (`127.0.0.1`) | Required (e.g., `matrix.example.com`) |
| TLS certificate | Not needed (HTTP) | Required (HTTPS, Let's Encrypt recommended) |
| Port 443 | Not needed | Open (Client-Server API) |
| Port 8448 | Not needed | Open (Server-Server Federation API) |
| Reverse proxy | Not needed | Recommended (Caddy or Nginx) |
| DNS records | Not needed | A record required, SRV record optional |

> **Self-signed certificates will NOT work** for federation. Other Matrix servers will refuse to connect. Use [Let's Encrypt](https://letsencrypt.org/) for free, trusted certificates.

---

## 3. Palpo Federation Configuration

### 3.1 Basic Settings (`palpo.toml`)

Here is a federation-ready `palpo.toml`. Changes from the local deployment are marked with comments:

```toml
# CHANGED: Use your real domain instead of 127.0.0.1:8128
server_name = "matrix.example.com"

# CHANGED: Disable open registration in production.
# Create accounts first, then set to false.
allow_registration = false

enable_admin_room = true
appservice_registration_dir = "/var/palpo/appservices"

[[listeners]]
address = "0.0.0.0:8008"

[logger]
format = "json"    # CHANGED: Use "json" for production

[db]
url = "postgres://palpo:YOUR_STRONG_PASSWORD@palpo_postgres:5432/palpo"
pool_size = 10

# CHANGED: Use real domain for discovery
[well_known]
server = "matrix.example.com:443"
client = "https://matrix.example.com"

# --- Federation settings (NEW) ---
[federation]
enable = true
allow_inbound_profile_lookup = true

# Optional: restrict federation to specific servers
# allowed_servers = ["matrix.org", "*.trusted.com"]
# denied_servers = ["evil.com"]

[tls]
enable = true
cert = "/path/to/fullchain.pem"
key = "/path/to/privkey.pem"

[presence]
allow_local = true
allow_incoming = true
allow_outgoing = true

[typing]
allow_incoming = true
allow_outgoing = true
federation_timeout = 30000

trusted_servers = ["matrix.org"]
```

### 3.2 Federation Settings Reference

The `[federation]` section controls how your server interacts with other Matrix servers:

| Field | Type | Default | Description |
|-------|------|---------|-------------|
| `enable` | bool | `true` | Master switch for federation. Set to `false` to run a completely isolated server. |
| `allow_loopback` | bool | `false` | Allow federation requests to self. For development only. |
| `allow_device_name` | bool | `false` | Expose device display names to federated users. Disabled for privacy. |
| `allow_inbound_profile_lookup` | bool | `true` | Allow remote servers to query local user profiles. Disabling hides display names from federated users. |
| `allowed_servers` | list | None | Allowlist: ONLY these servers can federate with yours. Supports wildcards (e.g., `*.trusted.com`). When unset, all servers are allowed. |
| `denied_servers` | list | `[]` | Denylist: block specific servers. **Takes precedence** over `allowed_servers`. Supports wildcards. |

### 3.3 Server Discovery (`[well_known]`)

The `[well_known]` section is **critical** for federation. It tells other homeservers how to find yours.

Palpo automatically serves these endpoints:

| Endpoint | Response | Used by |
|----------|----------|---------|
| `/.well-known/matrix/server` | `{"m.server": "matrix.example.com:443"}` | Other homeservers (federation) |
| `/.well-known/matrix/client` | `{"m.homeserver": {"base_url": "https://matrix.example.com"}}` | Matrix clients (Robrix, Element) |

If you are behind a reverse proxy, ensure these endpoints are forwarded correctly to Palpo. See [Section 4.2](#42-reverse-proxy-caddy-example) for proxy configuration.

### 3.4 TLS Configuration

```toml
[tls]
enable = true
cert = "/path/to/fullchain.pem"
key = "/path/to/privkey.pem"
dual_protocol = false    # Do NOT allow HTTP alongside HTTPS in production
```

If you use a reverse proxy that terminates TLS (recommended), you can leave `[tls]` disabled in Palpo and let the proxy handle certificates. See [Section 4.2](#42-reverse-proxy-caddy-example).

### 3.5 Presence and Typing (Federated Features)

These settings control real-time status indicators across federated servers:

```toml
[presence]
allow_local = true       # Local presence (your server only)
allow_incoming = true    # Receive presence updates from remote servers
allow_outgoing = true    # Send presence updates to remote servers

[typing]
allow_incoming = true    # Receive typing indicators from remote users
allow_outgoing = true    # Send typing indicators to remote users
federation_timeout = 30000   # Milliseconds
```

> **Note:** `allow_outgoing` under `[presence]` requires `allow_local` to be `true`.

### 3.6 Trusted Servers

```toml
trusted_servers = ["matrix.org"]
```

Trusted servers act as **notary servers** -- they help verify signing keys from other homeservers. This is part of the [Perspectives key verification](https://spec.matrix.org/latest/server-server-api/#querying-keys-through-another-server) mechanism. `matrix.org` is the most common choice.

### 3.7 DNS Configuration

These are top-level settings in `palpo.toml` (not inside any `[section]`):

```toml
# Add these at the top level of palpo.toml (alongside server_name, etc.)
query_over_tcp_only = true       # Use TCP for DNS (more reliable in containers)
query_all_nameservers = true     # Query all configured nameservers
ip_lookup_strategy = 5           # 5 = IPv4 first, then IPv6
```

> **Tip:** If running in Docker, `query_over_tcp_only = true` is recommended to avoid UDP DNS resolution issues in container networks.

---

## 4. Production Deployment

This section covers the infrastructure changes needed to go from local to federated deployment.

### 4.1 Domain and DNS Setup

1. **Register a domain** (e.g., `example.com`)

2. **Create a DNS A record** pointing to your server:
   ```
   matrix.example.com.  IN  A  203.0.113.10
   ```

3. **Optional: Create an SRV record** for federation on a non-standard port:
   ```
   _matrix-fed._tcp.example.com.  IN  SRV  10 0 8448 matrix.example.com.
   ```
   > The SRV record is not needed if you serve federation on port 443 and have a proper `/.well-known/matrix/server` response.

### 4.2 Reverse Proxy (Caddy Example)

Using a reverse proxy is recommended for production. Caddy automatically manages TLS certificates via Let's Encrypt.

```
matrix.example.com {
    # Well-known endpoints (federation discovery)
    handle /.well-known/matrix/server {
        respond `{"m.server":"matrix.example.com:443"}`
    }

    handle /.well-known/matrix/client {
        header Access-Control-Allow-Origin "*"
        respond `{"m.homeserver":{"base_url":"https://matrix.example.com"}}`
    }

    # Proxy everything else to Palpo
    reverse_proxy localhost:8008
}
```

With Caddy handling TLS, you can disable `[tls]` in `palpo.toml` and let Palpo listen on plain HTTP internally:

```toml
[tls]
enable = false    # Caddy terminates TLS
```

> **Nginx alternative:** If using Nginx, you need to manage Let's Encrypt certificates separately (e.g., with certbot) and configure `ssl_certificate` / `ssl_certificate_key` directives.

### 4.3 Updated Docker Compose

Key changes from the local `compose.yml`:

```yaml
services:
  palpo:
    # ... (same build section as local) ...
    ports:
      - "8008:8008"    # CHANGED: Caddy proxies to this port
      # No longer exposing 8128 directly
    volumes:
      - ./palpo.toml:/var/palpo/palpo.toml:ro
      - ./appservices:/var/palpo/appservices:ro
      - ./data/media:/var/palpo/media
      # ADDED: Mount TLS certs (only if Palpo handles TLS directly)
      # - /etc/letsencrypt/live/matrix.example.com:/certs:ro
    # ... rest same as local ...
```

The full Docker Compose structure remains the same as the local deployment -- PostgreSQL, Palpo, and Octos. The key differences are:

- `server_name` in `palpo.toml` uses your real domain
- Port mapping changes (Caddy on 443 proxies to Palpo on 8008)
- TLS handled by Caddy (or mounted certificates if Palpo handles TLS)
- `allow_registration = false` (create accounts first, then lock down)

### 4.4 Appservice Registration Updates

When switching from `127.0.0.1:8128` to a real domain, update these files:

**`appservices/octos-registration.yaml`** -- Update regex patterns:

```yaml
namespaces:
  users:
    - exclusive: true
      regex: "@octosbot_.*:matrix\\.example\\.com"    # CHANGED
    - exclusive: true
      regex: "@octosbot:matrix\\.example\\.com"       # CHANGED
```

**`config/botfather.json`** -- Update `server_name`:

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

> **Important:** The `homeserver` URL in `botfather.json` stays as the internal Docker address (`http://palpo:8008`). Only `server_name` changes to the real domain.

---

## 5. Using Federation

Once federation is configured, you can interact with users and rooms on other Matrix servers.

### 5.1 Join Rooms on Other Servers

From Robrix:

1. Click **Join Room**
2. Enter a room alias from another server, e.g., `#general:matrix.org`
3. Your server federates with `matrix.org` to join the room
4. Messages from users on all participating servers appear in real time

<!-- screenshot: federated-room.png -- Robrix showing a room from another server -->

### 5.2 Invite Users from Other Servers

1. Open a room on your server
2. Click **Invite**
3. Enter a user ID from another server: `@friend:other-server.com`
4. The invitation travels via federation to the remote server
5. When the remote user accepts, they join your room

### 5.3 Cross-Server AI Bot

With federation, users from **other servers** can also interact with your Octos bot:

1. A user on `matrix.org` invites `@octosbot:matrix.example.com` to their room
2. The invitation federates to your server
3. Octos accepts the invitation and joins the room
4. The bot responds to messages -- even though the room lives on a different server

> **Note:** For this to work, `allowed_senders` in `botfather.json` must either be empty `[]` (allow all users) or explicitly include the remote user's Matrix ID (e.g., `@remoteuser:matrix.org`).

---

## 6. Verification and Troubleshooting

### 6.1 Test Federation

**Check that well-known endpoints are accessible:**

```bash
# Server discovery (used by other homeservers)
curl https://matrix.example.com/.well-known/matrix/server

# Client discovery (used by Robrix/Element)
curl https://matrix.example.com/.well-known/matrix/client
```

**Use the Matrix Federation Tester:**

Visit [https://federationtester.matrix.org](https://federationtester.matrix.org) and enter your domain (e.g., `matrix.example.com`). It checks:

- DNS resolution
- TLS certificate validity
- Well-known endpoint responses
- Server-Server API reachability
- Signing key verification

### 6.2 Common Issues

| Symptom | Cause | Fix |
|---------|-------|-----|
| Cannot join rooms on other servers | Federation disabled or ports blocked | Check `[federation] enable = true` in `palpo.toml`. Ensure ports 443 and 8448 are open in your firewall. |
| "Unable to find signing key" | TLS or DNS misconfigured | Verify your TLS certificate is valid (not self-signed). Check that DNS resolves correctly. Run the Federation Tester. |
| Well-known returns 404 | Reverse proxy not forwarding | Check your Caddy/Nginx config forwards `/.well-known/matrix/*` to Palpo (or responds directly). |
| Remote users cannot see profiles | Profile lookup disabled | Set `allow_inbound_profile_lookup = true` in `[federation]`. |
| Connection timeouts to remote servers | Firewall or DNS issues | Check outbound connectivity. Try setting `query_over_tcp_only = true`. Verify your server can reach other Matrix servers on port 8448. |
| Bot does not respond to federated users | `allowed_senders` filtering | Set `allowed_senders` to `[]` in `botfather.json` to allow all users, or add the remote user's full Matrix ID. |

### 6.3 Debug Commands

```bash
# Check Palpo logs for federation activity
docker compose logs palpo | grep -i federation

# Check if Palpo can reach other servers
docker compose exec palpo curl -sf https://matrix.org/.well-known/matrix/server

# Verify your well-known endpoints externally
curl -sf https://matrix.example.com/.well-known/matrix/server
curl -sf https://matrix.example.com/.well-known/matrix/client

# Check TLS certificate
openssl s_client -connect matrix.example.com:443 -servername matrix.example.com < /dev/null 2>/dev/null | openssl x509 -noout -dates
```

---

## 7. Further Reading

- **Matrix Federation Specification:** [spec.matrix.org/latest/server-server-api](https://spec.matrix.org/latest/server-server-api/) -- the protocol specification for server-to-server communication.
- **Matrix Federation Tester:** [federationtester.matrix.org](https://federationtester.matrix.org/) -- online tool to verify your federation setup.
- **Palpo GitHub:** [github.com/palpo-im/palpo](https://github.com/palpo-im/palpo) -- Palpo homeserver source code and documentation.
- **Let's Encrypt:** [letsencrypt.org](https://letsencrypt.org/) -- free, automated TLS certificates.
- **Caddy:** [caddyserver.com](https://caddyserver.com/) -- reverse proxy with automatic HTTPS.

---

*This guide covers federation configuration as of April 2026. For the latest updates, see the respective project repositories.*
