# Deployment Guide: OpenClaw + Matrix

[中文版](01-deploying-openclaw-with-matrix-zh.md)

> **Goal:** After following this guide, you will have OpenClaw running as an AI agent connected to a Matrix homeserver. You can then use Robrix (or any Matrix client) to chat with OpenClaw-powered AI agents.

## What is OpenClaw?

[OpenClaw](https://github.com/openclaw/openclaw) is an open-source, self-hosted AI assistant platform (formerly MoltBot, renamed in early 2026). It supports multiple LLMs (DeepSeek, OpenAI, Anthropic, etc.) and connects to Matrix, Telegram, Discord, and other chat platforms via channel plugins. In this guide, OpenClaw logs in to the Matrix server as a **regular user** through its **Matrix channel plugin**, requiring no server-side configuration. This is simpler than Octos's Application Service approach -- see [Architecture Guide](03-how-robrix-and-openclaw-work-together.md) for details.

This guide walks you through deploying OpenClaw with Matrix step by step: from creating a Matrix bot account, to configuring the OpenClaw Matrix channel plugin, to verifying the connection end-to-end.

> **Just want to try it quickly?** Jump to [Quick Start](#2-quick-start).
>
> **Want to understand HOW OpenClaw connects to Matrix?** See [Architecture](03-how-robrix-and-openclaw-work-together.md) for the full explanation.

> **About OpenClaw:** OpenClaw is under rapid development and its CLI and plugin system have a number of bugs (e.g., the `channels add` wizard may crash). This guide documents a configuration approach we have **tested and verified** -- editing the config file directly, bypassing the unstable CLI wizards. If you encounter issues not covered here, consult the [OpenClaw official documentation](https://docs.openclaw.ai/) and [GitHub Issues](https://github.com/openclaw/openclaw/issues).

---

## Table of Contents

1. [Prerequisites](#1-prerequisites)
2. [Quick Start](#2-quick-start)
3. [Creating a Matrix Bot Account](#3-creating-a-matrix-bot-account)
4. [Installing OpenClaw and Initializing the Config Directory](#4-installing-openclaw-and-initializing-the-config-directory)
5. [Writing the Configuration File](#5-writing-the-configuration-file)
6. [Starting and Verifying](#6-starting-and-verifying)
7. [Troubleshooting](#7-troubleshooting)
8. [Production Configuration](#8-production-configuration)
9. [Further Reading](#9-further-reading)

---

## 1. Prerequisites

| Requirement                   | Notes                                                                                                                                           |
| ----------------------------- | ----------------------------------------------------------------------------------------------------------------------------------------------- |
| **Two Matrix accounts** | One for yourself, one for the OpenClaw bot                                                                                                      |
| **Node.js**             | v22.16+ or v24+ (recommended)                                                                                                                   |
| **LLM API Key**         | e.g.,[DeepSeek](https://platform.deepseek.com/) (free tier available), OpenAI, Anthropic, etc.                                                     |
| **Matrix homeserver**   | Local Palpo (recommended, see[Palpo Deployment Guide](../robrix-with-palpo-and-octos/01-deploying-palpo-and-octos.md)) or public server matrix.org |
| **Robrix**              | See[Getting Started with Robrix](../robrix/getting-started-with-robrix.md)                                                                         |

---

## 2. Quick Start

```
1. Register a Matrix bot account (remember the username and password)
2. Install OpenClaw → run openclaw config → edit ~/.openclaw/openclaw.json
3. Run openclaw gateway start
4. In Robrix, message the bot from another account
```

See below for detailed steps.

---

## 3. Creating a Matrix Bot Account

A bot account is just a **regular Matrix account**. OpenClaw logs in automatically using the username and password -- you do not need to manually obtain an Access Token.

| Server                        | How to register                                         | Notes                                                           |
| ----------------------------- | ------------------------------------------------------- | --------------------------------------------------------------- |
| **Local Palpo**         | Register in Robrix                                      | Connect to `http://127.0.0.1:8128` and register a new account |
| **matrix.org**          | Register in Robrix or[Element Web](https://app.element.io) | Public server, free, instant                                    |
| **Self-hosted Synapse** | Via Admin API or web registration                       | Recommended for production                                      |

When registering, remember:

- **Username** (e.g., `chalice`)
- **Password**

---

## 4. Installing OpenClaw and Initializing the Config Directory

### 4.1 Install

```bash
npm install -g openclaw@latest
openclaw --version    # verify installation
```

### 4.2 Initialize the config directory

```bash
openclaw config
```

> **This command will output an error -- that is expected and can be safely ignored.** The important thing is that it creates the config directory structure under `~/.openclaw/`. All subsequent configuration is done in this directory.

> **Why not use the `openclaw channels add` wizard?** OpenClaw v2026.4.7's CLI wizard has multiple bugs (Telegram plugin path error crashes the wizard, incomplete parameters, etc.). **Editing the config file directly is the only reliable approach.**

---

## 5. Writing the Configuration File

Edit `~/.openclaw/openclaw.json`. Two complete configurations are provided below for different scenarios.

### 5.1 Connecting to Local Palpo (Recommended)

```json
{
  "commands": {
    "native": "auto",
    "nativeSkills": "auto"
  },
  "models": {
    "providers": {
      "deepseek": {
        "baseUrl": "https://api.deepseek.com/v1",
        "apiKey": "sk-your-deepseek-key",
        "api": "openai-completions",
        "models": [
          {
            "id": "deepseek-chat",
            "name": "DeepSeek Chat",
            "contextWindow": 164000,
            "maxTokens": 8192
          }
        ]
      }
    }
  },
  "channels": {
    "matrix": {
      "enabled": true,
      "homeserver": "http://127.0.0.1:8128",
      "network": {
        "dangerouslyAllowPrivateNetwork": true
      },
      "userId": "@chalice:127.0.0.1:8128",
      "password": "your-password",
      "deviceName": "OpenClaw Bot",
      "encryption": true,
      "autoJoin": "always",
      "dm": {
        "policy": "open"
      }
    }
  },
  "plugins": {
    "entries": {
      "matrix": {
        "enabled": true
      }
    }
  },
  "gateway": {
    "mode": "local"
  }
}
```

### 5.2 Connecting to Public matrix.org

```json
{
  "commands": {
    "native": "auto",
    "nativeSkills": "auto"
  },
  "models": {
    "providers": {
      "deepseek": {
        "baseUrl": "https://api.deepseek.com/v1",
        "apiKey": "sk-your-deepseek-key",
        "api": "openai-completions",
        "models": [
          {
            "id": "deepseek-chat",
            "name": "DeepSeek Chat",
            "contextWindow": 164000,
            "maxTokens": 8192
          }
        ]
      }
    }
  },
  "channels": {
    "matrix": {
      "enabled": true,
      "homeserver": "https://matrix.org",
      "userId": "@your-bot:matrix.org",
      "password": "your-password",
      "deviceName": "OpenClaw Bot",
      "encryption": true,
      "autoJoin": "always",
      "dm": {
        "policy": "open"
      }
    }
  },
  "plugins": {
    "entries": {
      "matrix": {
        "enabled": true
      }
    }
  },
  "gateway": {
    "mode": "local"
  }
}
```

### 5.3 Configuration Details

#### `gateway` Configuration

| Field    | Value       | Key Notes                                                                                                                                                                                                                                                                                    |
| -------- | ----------- | -------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `mode` | `"local"` | **Required.** Without this field, gateway refuses to start with "missing gateway.mode" error. `"local"` means the OpenClaw gateway itself runs locally (listens on 127.0.0.1 only) -- this is unrelated to whether the LLM is remote. DeepSeek API calls still go over the internet. |

#### `models.providers` Configuration

| Field             | Value                             | Key Notes                                                                                                                                                                                                                                         |
| ----------------- | --------------------------------- | ------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `baseUrl`       | `"https://api.deepseek.com/v1"` | **Must include the `/v1` suffix.** DeepSeek uses an OpenAI-compatible API.                                                                                                                                                                |
| `apiKey`        | `"sk-xxx"`                      | **Write the key directly as plaintext.** Do not use `${ENV_VAR}` format -- macOS LaunchAgent services cannot read terminal environment variables. After writing, run `chmod 600 ~/.openclaw/openclaw.json` to protect file permissions. |
| `api`           | `"openai-completions"`          | **Not `type`.** Many online tutorials incorrectly use `"type"` -- the correct field name is `"api"`.                                                                                                                                  |
| `contextWindow` | `164000`                        | **Must be set high.** OpenClaw's system prompt alone takes 16K+ tokens; the default 4096 will cause errors. DeepSeek Chat supports 164K.                                                                                                    |
| `maxTokens`     | `8192`                          | Maximum tokens per reply.                                                                                                                                                                                                                         |

> **Note on `providers` format:** `providers` is an object (provider name as key), not an array. `models` inside a provider IS an array.

#### `channels.matrix` Configuration

| Field                                      | Value                         | Key Notes                                                                                                                                                                                                                                                                                     |
| ------------------------------------------ | ----------------------------- | --------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------------- |
| `enabled`                                | `true`                      | Enable the Matrix channel.                                                                                                                                                                                                                                                                    |
| `homeserver`                             | `"http://127.0.0.1:8128"`   | **Local Palpo must use `http`**, not `https` (Palpo has no TLS by default). matrix.org uses `https`.                                                                                                                                                                              |
| `network.dangerouslyAllowPrivateNetwork` | `true`                      | **Only needed for local/LAN deployments.** OpenClaw blocks private IPs (127.0.0.1, 10.x, 192.168.x) by default as an anti-SSRF security measure. Not needed when connecting to public servers like matrix.org.                                                                          |
| `userId`                                 | `"@chalice:127.0.0.1:8128"` | **Must be the full Matrix ID format** `@username:server`.                                                                                                                                                                                                                             |
| `password`                               | `"your-password"`           | Password authentication -- OpenClaw logs in automatically and caches the token at `~/.openclaw/credentials/matrix/`. Access Token authentication is also supported (replace `password` with `accessToken`) -- see [OpenClaw Matrix Plugin Docs](https://docs.openclaw.ai/channels/matrix). |
| `encryption`                             | `true`                      | **Strongly recommended.** Matrix DMs enable E2EE by default. Without this, the bot receives encrypted messages it cannot decrypt, resulting in "message sent but no reply".                                                                                                             |
| `autoJoin`                               | `"always"`                  | Accept all invites during testing. Change to `"allowlist"` in production.                                                                                                                                                                                                                   |
| `dm.policy`                              | `"open"`                    | Allow all DMs during testing. Change to `"allowlist"` in production.                                                                                                                                                                                                                        |

#### `plugins` Configuration

| Field                              | Value    | Key Notes                            |
| ---------------------------------- | -------- | ------------------------------------ |
| `plugins.entries.matrix.enabled` | `true` | Ensure the Matrix plugin is enabled. |

### 5.4 Local Palpo vs Public matrix.org Differences

| Setting                                    | Local Palpo                           | Public matrix.org                                          |
| ------------------------------------------ | ------------------------------------- | ---------------------------------------------------------- |
| `homeserver`                             | `http://127.0.0.1:8128`             | `https://matrix.org`                                     |
| `network.dangerouslyAllowPrivateNetwork` | **Required** `true`           | **Not needed** (remove the entire `network` block) |
| `userId` format                          | `@username:127.0.0.1:8128`          | `@username:matrix.org`                                   |
| TLS                                        | None (`http`)                       | Yes (`https`)                                            |
| Registration                               | Register in Robrix connected to Palpo | Register via Element Web or Robrix                         |

> **Switching from local Palpo to matrix.org:** This guide uses Palpo as the example, but the same configuration works on matrix.org or any standard Matrix server. You only need to change 3 things in `openclaw.json`:
>
> 1. `homeserver`: `http://127.0.0.1:8128` → `https://matrix.org`
> 2. `userId`: `@username:127.0.0.1:8128` → `@username:matrix.org`
> 3. Remove the entire `"network": { "dangerouslyAllowPrivateNetwork": true }` block (not needed for public servers)
>
> Everything else (LLM, encryption, autoJoin, etc.) **stays exactly the same**. After editing, run `openclaw gateway restart`.
>
> If you use another self-hosted Matrix server (e.g., Synapse, Dendrite), the same 3 changes apply -- just replace with your server's domain and protocol.

---

## 6. Starting and Verifying

### 6.1 Start the Gateway

```bash
openclaw gateway start
```

### 6.2 Check the Logs

```bash
tail -20 ~/.openclaw/logs/gateway.log
```

Confirm you see these key log lines:

```
[gateway] agent model: deepseek/deepseek-chat          ← LLM config is correct
[gateway] ready (6 plugins, 0.3s)                       ← Gateway is ready
[matrix] [default] starting provider (http://...)       ← Matrix connecting
matrix: logged in as @chalice:127.0.0.1:8128           ← Login successful
matrix: device is verified by its owner and ready for encrypted rooms  ← Encryption ready
```

> **Tip:** OpenClaw needs some time to complete the Matrix login and encryption device initialization after startup. If you only see `starting provider` but not `logged in` in the logs, wait a few seconds -- messages can only be received after login completes.

### 6.3 Test in Robrix

1. **Launch Robrix** and log in with your **personal account**
2. **Search for the bot**: Click the search icon, type the bot's Matrix ID (e.g., `@chalice:127.0.0.1:8128`), switch to the **People** tab (the bot is a regular user, so you must search under People)
3. **Start a DM**: Select the bot to enter a conversation
4. **Send a message** and wait patiently for a reply (LLM responses take a few seconds to tens of seconds)

<img src="../images/openclaw-bot-reply.png" width="600" alt="OpenClaw Bot (chalice) successfully replying in Robrix">

> **Important:** If you sent messages before OpenClaw's encryption device was created, those historical messages **can never be decrypted** (this is normal Matrix E2EE behavior). You must send a **new message** to trigger a reply.

---

## 7. Troubleshooting

| Symptom                                                       | Cause                                                    | Solution                                                                                                     |
| ------------------------------------------------------------- | -------------------------------------------------------- | ------------------------------------------------------------------------------------------------------------ |
| `channels add` wizard crashes with ENOENT                   | v2026.4.7 Telegram plugin path bug                       | Skip the wizard, edit `~/.openclaw/openclaw.json` directly                                                 |
| Gateway refuses to start: "missing gateway.mode"              | Config file missing `gateway` section                  | Add `"gateway": {"mode": "local"}`                                                                         |
| "Blocked hostname or private/internal/special-use IP address" | OpenClaw blocks private IPs by default                   | Add `"network": {"dangerouslyAllowPrivateNetwork": true}`                                                  |
| Matrix connection fails, keeps retrying                       | `homeserver` uses `https` but local Palpo has no TLS | Change to `http://127.0.0.1:8128`                                                                          |
| "Invalid input: expected record, received array"              | `providers` format is wrong                            | `providers` must be an object (key-value), not an array                                                    |
| "Unrecognized key: type"                                      | Wrong field name                                         | Use `"api"` instead of `"type"`                                                                          |
| "missing env var DEEPSEEK_API_KEY"                            | Environment variable not visible to LaunchAgent          | Write API key directly in the config file                                                                    |
| Message sent but bot does not reply (no error)                | DM is encrypted but OpenClaw has encryption disabled     | Add `"encryption": true`                                                                                   |
| "encrypted event received without encryption enabled"         | Same as above                                            | Add `"encryption": true`                                                                                   |
| "This message was sent before this device logged in"          | Historical messages cannot be decrypted                  | Normal behavior. Send a**new message**                                                                 |
| Bot replies are empty or error                                | LLM API key invalid or insufficient balance              | Check DeepSeek API key and account balance                                                                   |
| Robrix cannot find the bot                                    | Bot account not registered                               | Confirm the bot account exists (verify in Element Web)                                                       |
| Other OpenClaw issues                                         | —                                                       | Consult[OpenClaw docs](https://docs.openclaw.ai/) and [GitHub Issues](https://github.com/openclaw/openclaw/issues) |

> **Important note:** This guide only covers the configuration workflow we have tested and verified (OpenClaw v2026.4.7). OpenClaw is still under rapid development -- its CLI, plugin system, and gateway behavior may change in future versions. If you encounter OpenClaw issues not listed above (CLI errors, plugin loading failures, gateway behavior anomalies, etc.), these are OpenClaw-side issues. Please refer to:
>
> - [OpenClaw Official Documentation](https://docs.openclaw.ai/) -- latest configuration reference
> - [OpenClaw Matrix Channel Plugin Docs](https://docs.openclaw.ai/channels/matrix) -- Matrix plugin specifics
> - [OpenClaw GitHub Issues](https://github.com/openclaw/openclaw/issues) -- known issues and community discussions
>
> Robrix, as a standard Matrix client, communicates with OpenClaw through the Matrix protocol. The two are fully decoupled -- no special configuration is needed on the Robrix side.

---

## 8. Production Configuration

After testing, tighten permissions. Modify these fields in `channels.matrix`:

```json
{
  "autoJoin": "allowlist",
  "autoJoinAllowlist": ["!room-id:your-server"],
  "dm": {
    "policy": "allowlist",
    "allowFrom": ["@admin:your-server"],
    "sessionScope": "per-room"
  },
  "groupPolicy": "allowlist",
  "groupAllowFrom": ["@admin:your-server"],
  "groups": {
    "!room-id:your-server": {
      "requireMention": true
    }
  }
}
```

| Field              | Testing      | Production      | Purpose                                     |
| ------------------ | ------------ | --------------- | ------------------------------------------- |
| `autoJoin`       | `"always"` | `"allowlist"` | Only join allowlisted rooms                 |
| `dm.policy`      | `"open"`   | `"allowlist"` | Only accept DMs from allowlisted users      |
| `groupPolicy`    | —           | `"allowlist"` | Restrict who can trigger the bot in groups  |
| `requireMention` | —           | `true`        | In group chats, require @mention to respond |

---

## 9. Further Reading

- **OpenClaw Documentation:** [docs.openclaw.ai](https://docs.openclaw.ai/) -- full OpenClaw documentation.
- **OpenClaw Matrix Plugin:** [docs.openclaw.ai/channels/matrix](https://docs.openclaw.ai/channels/matrix) -- official Matrix channel plugin reference.
- **OpenClaw GitHub:** [github.com/openclaw/openclaw](https://github.com/openclaw/openclaw) -- source code, issues, and latest releases.
- **Palpo Deployment Guide:** [01-deploying-palpo-and-octos.md](../robrix-with-palpo-and-octos/01-deploying-palpo-and-octos.md) -- how to deploy a local Palpo homeserver.
- **Architecture Guide:** [03-how-robrix-and-openclaw-work-together.md](03-how-robrix-and-openclaw-work-together.md) -- how OpenClaw connects to Matrix, and comparison with the Octos AppService model.
- **Usage Guide:** [02-using-robrix-with-openclaw.md](02-using-robrix-with-openclaw.md) -- how to use Robrix to chat with OpenClaw agents.

---

*This guide is based on tested results from April 2026 (OpenClaw v2026.4.7 + Palpo). OpenClaw is under rapid development -- if you encounter issues, refer to the [official documentation](https://docs.openclaw.ai/) for the latest information.*
