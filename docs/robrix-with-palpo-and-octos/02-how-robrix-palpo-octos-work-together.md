# Architecture: How Robrix + Palpo + Octos Work Together

[中文版](02-how-robrix-palpo-octos-work-together-zh.md)

> **Goal:** After reading this guide, you will understand how the Matrix Application Service mechanism works, how Octos registers as an App Service on Palpo to receive and respond to messages, and how the complete message lifecycle flows from Robrix through Palpo to the AI bot and back.

This document explains the **mechanisms** behind the Robrix + Palpo + Octos system. If you want to deploy it, see [01-deploying-palpo-and-octos.md](01-deploying-palpo-and-octos.md). If you want to use it, see [03-using-robrix-with-palpo-and-octos.md](03-using-robrix-with-palpo-and-octos.md).

---

## Table of Contents

1. [Three Projects Overview](#1-three-projects-overview)
2. [Matrix Protocol Basics](#2-matrix-protocol-basics)
3. [Application Service Mechanism](#3-application-service-mechanism)
4. [Message Lifecycle](#4-message-lifecycle)
5. [Ports and Protocols](#5-ports-and-protocols)
6. [BotFather System](#6-botfather-system)
7. [Further Reading](#7-further-reading)

---

## 1. Three Projects Overview

| Project | Role | What it does |
|---------|------|--------------|
| [**Robrix**](https://github.com/Project-Robius-China/robrix2) | Matrix Client | A cross-platform Matrix chat client written in Rust using [Makepad](https://github.com/makepad/makepad/). Runs natively on macOS, Linux, Windows, Android, and iOS. This is the user-facing application -- where you read and send messages. |
| [**Palpo**](https://github.com/palpo-im/palpo) | Matrix Homeserver | A Rust-native Matrix homeserver. Stores user accounts, rooms, and messages in PostgreSQL. Routes events between clients (Robrix) and application services (Octos). Think of it as the central post office. |
| [**Octos**](https://github.com/octos-org/octos) | AI Bot (Appservice) | A Rust-native AI agent platform that runs as a [Matrix Application Service](https://spec.matrix.org/latest/application-service-api/). Receives messages from Palpo, forwards them to an LLM (DeepSeek, OpenAI, Anthropic, etc.), and posts the AI reply back into the room. |

Each project is independent and open-source. Together, they form a complete AI chat system where users interact with AI bots through a native chat interface, with all communication routed through a standards-compliant Matrix homeserver.

---

## 2. Matrix Protocol Basics

Before diving into the architecture, here are the Matrix protocol concepts you need to understand.

### Homeserver

A homeserver is the backbone of Matrix. It stores user accounts, room state, and message history. Every user belongs to exactly one homeserver -- for example, `@alice:example.com` belongs to the homeserver at `example.com`. In our system, Palpo is the homeserver.

### Room

A room is a shared conversation space. When you send a message, it is sent to a room, not directly to another user. All participants in the room see the message. Rooms can contain any mix of human users and bots.

### Event

Everything in Matrix is an **event**. A message is an event (`m.room.message`). Joining a room is an event (`m.room.member`). Changing a room's name is an event. Events are the fundamental unit of data -- they are immutable, ordered, and form the room's history.

### Client-Server API

This is how clients (like Robrix) communicate with their homeserver (Palpo). The Client-Server API is used for:

- Logging in and registering accounts
- Sending messages (`PUT /_matrix/client/v3/rooms/{roomId}/send/...`)
- Syncing room state and message history
- Managing rooms (creating, joining, inviting)

Robrix talks to Palpo exclusively through this API. Octos also uses it when sending bot replies back through Palpo.

### Server-Server API (Federation)

This is how homeservers talk to each other. If `@alice:server-a.com` sends a message in a room that `@bob:server-b.com` is in, the two homeservers communicate via federation to deliver the event. This is what makes Matrix a decentralized protocol. See [04-federation-with-palpo.md](04-federation-with-palpo.md) for details.

### Sliding Sync

Traditional Matrix sync downloads the entire room state on startup, which can be slow on mobile or constrained devices. **Sliding Sync** is an optimized sync mechanism (defined in the Matrix spec) that only sends the data the client currently needs -- like a sliding window over your room list. Robrix requires Sliding Sync support from the homeserver. Palpo supports it natively.

---

## 3. Application Service Mechanism

This section is the core of the architecture. Understanding the Application Service (appservice) mechanism is the key to understanding how Octos connects to Palpo.

### 3.1 What is a Matrix Application Service?

A Matrix Application Service is a special kind of program that has **elevated privileges** on a homeserver. Unlike a regular client that logs in with a username and password, an appservice:

- **Registers with the homeserver** via a YAML registration file (not through the Client-Server API)
- **Claims exclusive user namespaces** -- it owns a range of user IDs and can act as any of them
- **Receives pushed events** from the homeserver -- it does not need to poll or sync
- **Is not rate-limited** -- it can send messages at whatever speed it needs
- **Can create virtual users** dynamically, without going through the registration flow

This is the mechanism designed for bridges (connecting Matrix to Telegram, Slack, etc.) and bots. Octos uses it to run AI bots.

> Matrix spec reference: [Application Service API](https://spec.matrix.org/latest/application-service-api/)

### 3.2 Registration File: How Palpo Discovers Octos

On startup, Palpo reads all `.yaml` files from the directory specified by `appservice_registration_dir` in `palpo.toml`. Each file represents one registered appservice.

The registration file (`appservices/octos-registration.yaml`) contains:

```yaml
id: octos-matrix-appservice
url: "http://octos:8009"

as_token: "<your-as-token>"
hs_token: "<your-hs-token>"

sender_localpart: octosbot
rate_limited: false

namespaces:
  users:
    - exclusive: true
      regex: "@octosbot_.*:127\\.0\\.0\\.1:8128"
    - exclusive: true
      regex: "@octosbot:127\\.0\\.0\\.1:8128"
```

Here is what each field does:

| Field | Purpose |
|-------|---------|
| `id` | A unique name for this appservice. Palpo uses it to track event delivery. |
| `url` | The HTTP endpoint where Palpo sends events. This is Octos's address inside the Docker network. |
| `as_token` | The token Octos presents when calling Palpo's API. Proves "I am the registered appservice." |
| `hs_token` | The token Palpo presents when pushing events to Octos. Proves "I am the homeserver you registered with." |
| `sender_localpart` | The main bot's username. Combined with `server_name`, it becomes `@octosbot:127.0.0.1:8128`. |
| `namespaces.users` | Regex patterns for user IDs that this appservice exclusively owns. |

This is a **mutual trust relationship**: Octos authenticates to Palpo with `as_token`, and Palpo authenticates to Octos with `hs_token`. Both sides must have the same token pair, configured in two files: the registration YAML (for Palpo) and `botfather.json` (for Octos). If they do not match, nothing works. See the [Token Matching Checklist](01-deploying-palpo-and-octos.md#38-token-matching-checklist) in the deployment guide.

### 3.3 User Namespaces: Bot Identity

The `namespaces.users` section is how Palpo knows which user IDs belong to Octos. The regex patterns claim specific ranges:

- **`@octosbot:127.0.0.1:8128`** -- The main bot, also called the **BotFather**. This is the entry point for users.
- **`@octosbot_.*:127.0.0.1:8128`** -- Child bots created dynamically (e.g., `@octosbot_translator:127.0.0.1:8128`). The `.*` wildcard means Octos can create any user ID with the `octosbot_` prefix.

Setting `exclusive: true` means **no other entity can create or claim these user IDs**. If a regular user tries to register as `@octosbot:127.0.0.1:8128`, Palpo will reject the request.

This namespace mechanism is also how Palpo decides to notify Octos. When someone invites `@octosbot:127.0.0.1:8128` to a room, Palpo checks its registered appservices, finds that this user ID matches Octos's namespace, and pushes the invite event to Octos.

### 3.4 Event Push Flow

The appservice protocol is **push-based**, not pull-based. The appservice does not sync or poll -- the homeserver sends events to it.

When a message arrives in a room where an appservice user is present:

1. **Palpo checks its appservice registry.** It looks at which appservice users are members of the room. If `@octosbot:127.0.0.1:8128` is in the room, Palpo knows Octos needs to be notified.

2. **Palpo sends an HTTP PUT to Octos.** The request goes to `{url}/transactions/{txnId}` -- in our case, `http://octos:8009/transactions/{txnId}`. The body contains the event data (sender, room ID, message content, etc.), and Palpo includes `hs_token` for authentication.

3. **Octos processes the event.** It receives the event, identifies the room and sender, and decides how to respond. For an AI bot, this means calling the configured LLM.

4. **Octos sends its reply via Palpo's Client-Server API.** Octos does not have its own connection to Robrix. Instead, it acts as the bot user and sends a message through Palpo, just like any other client. It authenticates with `as_token`.

This push model is efficient: Octos does not waste resources polling, and events are delivered with minimal latency.

---

## 4. Message Lifecycle

Here is the complete journey of a message through the system, from the moment you type it in Robrix to the moment the AI bot's reply appears on your screen.

### Step-by-Step Data Flow

```
User types "Hello" in Robrix
         |
         v
+-----------------+
| 1. Robrix sends |  PUT /_matrix/client/v3/rooms/{roomId}/send/m.room.message
|    via CS API   |  -> http://127.0.0.1:8128 (Palpo)
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
| 3. Palpo pushes |  PUT /transactions/{txnId} -> http://octos:8009
|    to Octos     |  (Appservice API, internal Docker network)
+--------+--------+
         |
         v
+-----------------+
| 4. Octos calls  |  POST /v1/chat/completions -> DeepSeek API
|    the LLM      |  (or other configured provider)
+--------+--------+
         |
         v
+-----------------+
| 5. Octos sends  |  PUT /_matrix/client/v3/rooms/{roomId}/send/m.room.message
|    reply via    |  -> http://palpo:8008 (internal Docker network)
|    CS API       |  Auth: Bearer {as_token}
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

### What Happens at Each Step

**Step 1 -- Robrix sends the message.** When you hit send, Robrix makes an HTTP PUT request to Palpo's Client-Server API. The request includes the room ID, the event type (`m.room.message`), and the message content. Robrix connects to `http://127.0.0.1:8128`, which is Palpo's port exposed on the host machine.

**Step 2 -- Palpo stores the event.** Palpo receives the message, assigns it an event ID, and persists it to PostgreSQL. The room's state is updated to reflect the new message.

**Step 3 -- Palpo pushes the event to Octos.** Palpo checks its appservice registry and sees that `@octosbot:127.0.0.1:8128` is a member of this room. It sends the event to Octos's appservice endpoint (`http://octos:8009`) via an HTTP PUT to `/transactions/{txnId}`. This uses the internal Docker network -- no traffic leaves the host.

**Step 4 -- Octos calls the LLM.** Octos receives the event, extracts the message content, and calls the configured LLM provider (e.g., DeepSeek's `/v1/chat/completions` endpoint). It includes conversation history for context.

**Step 5 -- Octos sends the reply.** Once the LLM responds, Octos sends the reply back through Palpo's Client-Server API, acting as the bot user (`@octosbot:127.0.0.1:8128`). It authenticates with `as_token`. Note that Octos connects to Palpo at `http://palpo:8008` (Docker internal), not `127.0.0.1:8128` (host).

**Step 6 -- Palpo delivers the reply to Robrix.** Palpo stores the bot's reply event and includes it in Robrix's next Sliding Sync response. Robrix receives the event and displays the AI bot's message in the conversation.

### Architecture Diagram

```
+----------+                        +----------+                       +----------+         +-----+
|  Robrix  |  Client-Server API     |  Palpo   |  Appservice API       |  Octos   |  HTTPS  | LLM |
| (Client) | -------------------->  | (Server) | --------------------> |  (Bot)   | ------> |     |
|          | <--------------------  |          | <-------------------  |          | <------ |     |
+----------+   Sliding Sync        +----------+  Client-Server API    +----------+         +-----+
  Your machine                     Docker :8128      Docker :8009                          External
```

Key observations:

- **Robrix never talks directly to Octos.** All communication goes through Palpo. Robrix does not even know Octos exists -- it just sees bot users in rooms.
- **Two different paths, same API.** Both Robrix and Octos use the Client-Server API to talk to Palpo, but Octos authenticates with `as_token` (appservice credential) instead of a regular user session.
- **Internal vs. external traffic.** Robrix connects via the host port (8128). Palpo and Octos communicate on the Docker internal network (service names `palpo:8008` and `octos:8009`). Only the LLM API call goes to the internet.

---

## 5. Ports and Protocols

| Connection | Protocol | Default Port | Direction | Notes |
|-----------|----------|-------------|-----------|-------|
| Robrix -> Palpo | Client-Server API (Sliding Sync) | 8128 (host) -> 8008 (container) | Bidirectional | The only port Robrix needs. Exposed on the host machine. |
| Palpo -> Octos | Appservice API | 8009 (host) -> 8009 (container) | Palpo pushes events | Also exposed to host for debugging. Uses Docker service name `octos` internally. |
| Octos -> Palpo | Client-Server API | 8008 (internal Docker network) | Octos sends replies | Uses Docker service name `palpo`. Auth via `as_token`. |
| Octos Dashboard | HTTP | 8010 (host) -> 8080 (container) | Inbound | Optional admin UI for monitoring Octos. |
| Octos -> LLM | HTTPS | 443 (outbound) | Outbound | External API call to the LLM provider. |

**Why two different ports for Palpo (8008 vs. 8128)?** Inside the Docker network, Palpo listens on port 8008 (its container port). Docker maps host port 8128 to container port 8008. Octos, running in the same Docker network, connects directly to `palpo:8008`. Robrix, running on the host machine, connects to `127.0.0.1:8128`.

---

## 6. BotFather System

Octos implements a **BotFather** pattern for managing multiple AI bots through a single appservice.

### Parent and Child Bots

The **BotFather** is the main bot (`@octosbot:server_name`). It is the entry point -- users invite BotFather to a room to start interacting. But BotFather can also create **child bots**, each with a different personality and purpose.

```
BotFather (@octosbot:127.0.0.1:8128)
    |
    +-- Translator Bot (@octosbot_translator:127.0.0.1:8128)
    |       System prompt: "You are a translator. Translate all messages to English."
    |
    +-- Code Reviewer (@octosbot_reviewer:127.0.0.1:8128)
    |       System prompt: "You are a code reviewer. Review code for bugs and style."
    |
    +-- Writing Assistant (@octosbot_writer:127.0.0.1:8128)
            System prompt: "You are a writing assistant. Help improve clarity and tone."
```

### How Child Bots Work

Each child bot has its own:

- **Display name** -- A human-readable name shown in the chat (e.g., "Translator Bot")
- **System prompt** -- Instructions that define the bot's personality and behavior
- **User ID** -- Generated with the `octosbot_` prefix (e.g., `@octosbot_translator:127.0.0.1:8128`)

Child bots are created dynamically at runtime. They do not need separate registration files or separate processes. They all run within the single Octos appservice instance.

### Why This Works: The Namespace Connection

Remember the namespace regex in the registration file?

```yaml
namespaces:
  users:
    - exclusive: true
      regex: "@octosbot_.*:127\\.0\\.0\\.1:8128"
```

This wildcard pattern is what makes dynamic child bot creation possible. When Octos creates a new child bot like `@octosbot_translator:127.0.0.1:8128`, Palpo checks the registered namespaces, confirms that this user ID is within Octos's exclusive range, and allows it. No additional configuration is needed.

### Managing Bots from Robrix

Robrix has a built-in UI for creating and managing child bots through the BotFather system. From Robrix's **Bot Settings** panel, you can:

1. Enable appservice support and configure the BotFather user ID
2. Create new child bots with a custom username, display name, and system prompt
3. View and manage existing bots

For step-by-step instructions, see the [Bot Management](03-using-robrix-with-palpo-and-octos.md) section in the usage guide.

---

## 7. Further Reading

- **Matrix Application Service Spec:** [spec.matrix.org -- Application Service API](https://spec.matrix.org/latest/application-service-api/) -- The official protocol specification for appservices.
- **Octos Book:** [octos-org.github.io/octos](https://octos-org.github.io/octos/) -- Full documentation for Octos, including all 14 LLM providers, channels, skills, and memory.
- **Palpo GitHub:** [github.com/palpo-im/palpo](https://github.com/palpo-im/palpo) -- Palpo homeserver documentation and source.
- **Robrix GitHub:** [Project-Robius-China/robrix2](https://github.com/Project-Robius-China/robrix2) -- Robrix client source and feature tracker.
- **Matrix Spec (Client-Server API):** [spec.matrix.org -- Client-Server API](https://spec.matrix.org/latest/client-server-api/) -- The full Client-Server API specification, including Sliding Sync.
- **Deployment Guide:** [01-deploying-palpo-and-octos.md](01-deploying-palpo-and-octos.md) -- How to deploy and configure the system.
- **Usage Guide:** [03-using-robrix-with-palpo-and-octos.md](03-using-robrix-with-palpo-and-octos.md) -- How to use Robrix with AI bots, step by step.

---

*This document describes the architecture as of April 2026. For the latest updates, see the respective project repositories.*
