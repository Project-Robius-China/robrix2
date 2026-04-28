# Architecture: How Robrix and OpenClaw Work Together

[中文版](03-how-robrix-and-openclaw-work-together-zh.md)

> **Goal:** After reading this document, you will understand how OpenClaw connects to Matrix as a regular client, how messages flow between Robrix, the Matrix homeserver, and the OpenClaw AI agent, and how this fundamentally differs from the Application Service model used by Octos.

This document explains the **mechanisms** behind the Robrix + OpenClaw system. If you want to deploy it, see [Deployment Guide](01-deploying-openclaw-with-matrix.md). If you want to use it, see [Usage Guide](02-using-robrix-with-openclaw.md).

---

## Table of Contents

1. [Two Projects Overview](#1-two-projects-overview)
2. [How OpenClaw Connects to Matrix](#2-how-openclaw-connects-to-matrix)
3. [Message Lifecycle](#3-message-lifecycle)
4. [Client Mode vs Application Service Mode](#4-client-mode-vs-application-service-mode)
5. [End-to-End Encryption (E2EE)](#5-end-to-end-encryption-e2ee)
6. [Further Reading](#6-further-reading)

---

## 1. Two Projects Overview

| Project | Role | What it does |
|---------|------|--------------|
| [**Robrix**](https://github.com/Project-Robius-China/robrix2) | Matrix Client | A cross-platform Matrix chat client built with Rust + [Makepad](https://github.com/makepad/makepad/). This is the user interface -- where you read and send messages. |
| [**OpenClaw**](https://github.com/openclaw/openclaw) | AI Agent Framework | An open-source AI assistant platform that connects to Matrix via its channel plugin as a **regular client**. Receives user messages, calls an LLM to generate replies, and sends them back to the room. |

Both projects are completely independent. OpenClaw is not designed specifically for Robrix or for Matrix -- it supports Telegram, Discord, Slack, and other channels. Matrix is just one of them.

---

## 2. How OpenClaw Connects to Matrix

OpenClaw connects to Matrix via the **Client-Server API**, the same way Robrix does -- it is simply a regular Matrix client.

### Connection Flow

```
1. On startup, OpenClaw calls POST /_matrix/client/v3/login with userId + password
2. Server returns an access_token, OpenClaw caches it at ~/.openclaw/credentials/
3. OpenClaw starts a Sliding Sync loop, continuously pulling new events
4. When an m.room.message event arrives, it extracts the content and calls the LLM
5. After the LLM responds, OpenClaw sends the reply via PUT /_matrix/client/v3/rooms/{roomId}/send/
```

### Key Characteristics

- **Authentication**: Username + password (same as any regular user)
- **Message retrieval**: Via Sync (actively pulls from server, not pushed by server)
- **Permission level**: Identical to a regular user (rate-limited, must be invited to join rooms)
- **Underlying SDK**: OpenClaw's Matrix plugin uses [matrix-js-sdk](https://github.com/matrix-org/matrix-js-sdk) (official JavaScript SDK)

---

## 3. Message Lifecycle

### Data Flow Diagram

```
User types "Hello" in Robrix
         |
         v
+-----------------+
| 1. Robrix sends |  PUT /_matrix/client/v3/rooms/{roomId}/send/m.room.message
|    via CS API   |  -> Palpo (http://127.0.0.1:8128)
+--------+--------+
         |
         v
+-----------------+
| 2. Palpo stores |  Event saved to PostgreSQL
|    the event    |  Room state updated
+--------+--------+
         |
         v
+-----------------+
| 3. OpenClaw     |  Receives new event via Sliding Sync
|    gets message |  (OpenClaw is a regular client, actively syncing)
+--------+--------+
         |
         v
+-----------------+
| 4. OpenClaw     |  POST https://api.deepseek.com/v1/chat/completions
|    calls LLM    |  With conversation history as context
+--------+--------+
         |
         v
+-----------------+
| 5. OpenClaw     |  PUT /_matrix/client/v3/rooms/{roomId}/send/m.room.message
|    sends reply  |  -> Palpo (http://127.0.0.1:8128)
|    via CS API   |  Auth: Bearer {access_token}
+--------+--------+
         |
         v
+-----------------+
| 6. Palpo stores |  Bot's reply event saved
|    & delivers   |  Sliding Sync pushes to Robrix
+--------+--------+
         |
         v
User sees AI reply in Robrix
```

### Architecture Diagram

```
+----------+                        +----------+                       +----------+         +-----+
|  Robrix  |  Client-Server API     |  Palpo   |  Client-Server API    | OpenClaw |  HTTPS  | LLM |
| (Client) | -------------------->  | (Server) | <------------------> | (AI Bot) | ------> |     |
|          | <--------------------  |          |   Sliding Sync        |          | <------ |     |
+----------+   Sliding Sync        +----------+                       +----------+         +-----+
  Your machine                     Docker :8128                        Your machine         External
```

**Key observations:**

- **Robrix and OpenClaw are equal peers to Palpo** -- both connect via the Client-Server API as regular clients.
- **OpenClaw requires no server-side configuration** -- no need to modify any Palpo config files (compare with Octos which requires AppService YAML registration).
- **Only the LLM call leaves local network** -- Robrix ↔ Palpo ↔ OpenClaw all stay on localhost; only the DeepSeek API call goes to the internet.

---

## 4. Client Mode vs Application Service Mode

The Robrix ecosystem offers two ways to integrate AI bots: OpenClaw's **client mode** and Octos's **Application Service mode**. Their core differences are:

### 4.1 Connection Mechanism Comparison

| | OpenClaw (Client Mode) | Octos (Application Service Mode) |
|---|---|---|
| **Connection** | Logs in with password, same as a regular user | Registered on the server via a YAML registration file |
| **Message retrieval** | **Sync pull** -- OpenClaw actively polls the server for new events | **Server push** -- Palpo actively pushes events to Octos's HTTP endpoint |
| **Authentication** | access_token (user-level) | as_token / hs_token (service-level, mutual authentication) |
| **Server-side config** | **None required** -- the bot is just a regular user | **Registration required** -- place YAML file in Palpo's `appservice_registration_dir` |
| **User namespaces** | Only one user ID | Can claim exclusive user namespaces, dynamically create child bots |
| **Rate limiting** | Subject to limits (same as regular users) | Exempt (`rate_limited: false`) |

### 4.2 Capability Comparison

| Capability | OpenClaw | Octos |
|------------|----------|-------|
| Basic conversation | Yes | Yes |
| Multiple LLM providers | Yes (14+) | Yes |
| E2EE encryption | Yes (Rust crypto SDK) | Not needed (AppService bypasses encryption) |
| Server-side administration | Not needed | Requires admin access to register AppService |
| Multi-channel (Telegram, Discord, etc.) | Yes | Matrix only |
| Homeserver requirements | Any standard Matrix server | Must support Application Service API |

### 4.3 Message Latency

| Stage | OpenClaw | Octos |
|-------|----------|-------|
| Message reaches bot | Sync interval (typically 1-5 seconds) | Instant push (< 100ms) |
| LLM response | Depends on LLM provider | Depends on LLM provider |
| Bot sends reply | Instant | Instant |

> OpenClaw uses Sliding Sync's long-polling mode, so actual latency is typically 1-2 seconds -- barely noticeable in a chat context.

### 4.4 Deployment Complexity

| | OpenClaw | Octos |
|---|---|---|
| Server side | No configuration needed | Requires registration YAML, token configuration |
| Client side | Install OpenClaw + edit one JSON file | Requires Docker Compose orchestrating three services |
| Token management | Password auto-login, token auto-cached | Must manually generate and synchronize as_token / hs_token |
| Architecture complexity | Simple (single process) | Complex (Palpo + Octos + PostgreSQL) |

### 4.5 When to Use Which?

| Scenario | Recommended |
|----------|-------------|
| Quick-test AI conversation | **OpenClaw** -- 5 minutes to configure, no server changes needed |
| Personal AI assistant | **OpenClaw** -- simple, flexible, multi-channel support |
| Team with multiple specialized bots | **Octos** -- BotFather can dynamically create child bots |
| Server-side administration needed | **Octos** -- AppService is registered and controlled by admins |
| High-concurrency messages | **Octos** -- server push + no rate limits |
| Cross-platform AI agent (Telegram/Discord simultaneously) | **OpenClaw** -- native multi-channel support |

---

## 5. End-to-End Encryption (E2EE)

### How OpenClaw Handles Encryption

OpenClaw's Matrix plugin uses matrix-js-sdk's **Rust crypto path**, implementing the Olm (one-to-one key exchange) and Megolm (group encryption) protocols.

When `"encryption": true` is configured:

1. **First login**: OpenClaw creates an encryption device and generates a cross-signing identity
2. **Auto-bootstrap**: Executes secret storage bootstrap; device is marked "verified by its owner"
3. **Receiving messages**: OpenClaw decrypts Megolm-encrypted messages
4. **Sending replies**: Replies are automatically encrypted

### Important Notes

- **Historical messages cannot be decrypted** -- Messages sent before OpenClaw's device was created did not have their Megolm session keys distributed to OpenClaw. They can never be decrypted.
- **vs Octos** -- Octos, as an AppService, receives **server-side decrypted plaintext events**. It does not need to handle E2EE at all. OpenClaw, as a client, must handle encryption itself.

---

## 6. Further Reading

- **OpenClaw Documentation:** [docs.openclaw.ai](https://docs.openclaw.ai/) -- full OpenClaw documentation.
- **OpenClaw Matrix Plugin:** [docs.openclaw.ai/channels/matrix](https://docs.openclaw.ai/channels/matrix) -- official Matrix channel plugin reference.
- **Matrix Client-Server API Spec:** [spec.matrix.org -- Client-Server API](https://spec.matrix.org/latest/client-server-api/) -- the protocol OpenClaw uses.
- **Matrix Application Service API Spec:** [spec.matrix.org -- Application Service API](https://spec.matrix.org/latest/application-service-api/) -- the protocol Octos uses.
- **Octos Architecture Guide:** [03-how-robrix-palpo-octos-work-together.md](../robrix-with-palpo-and-octos/03-how-robrix-palpo-octos-work-together.md) -- full explanation of the Octos AppService model.
- **Deployment Guide:** [01-deploying-openclaw-with-matrix.md](01-deploying-openclaw-with-matrix.md) -- how to deploy OpenClaw with Matrix.
- **Usage Guide:** [02-using-robrix-with-openclaw.md](02-using-robrix-with-openclaw.md) -- how to use Robrix to chat with OpenClaw agents.

---

*This document is based on tested results from April 2026. For the latest updates, see the respective project repositories.*
