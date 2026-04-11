# Robrix Documentation

Welcome to the Robrix documentation. Choose a guide based on your use case.

---

## Robrix Only

For users who want to use Robrix as a standalone Matrix client, connecting to matrix.org or any existing homeserver:

| Guide | Goal |
|-------|------|
| [Getting Started with Robrix](robrix/getting-started-with-robrix.md) | **Install Robrix and start chatting.** Download or build Robrix, connect to a Matrix server, register an account, and join rooms. |

> Chinese: [Robrix 快速开始](robrix/getting-started-with-robrix-zh.md)

---

## Robrix + Palpo + Octos (AI Bot System)

For users who want to deploy a complete AI chat system — running your own Matrix homeserver with AI bot capabilities, then using Robrix to chat with AI bots:

| Guide | Goal |
|-------|------|
| [1. Deploying Palpo and Octos](robrix-with-palpo-and-octos/01-deploying-palpo-and-octos.md) | **Get Palpo homeserver and Octos AI bot running.** Clone, configure, and launch all backend services with Docker Compose so Robrix can connect to your own server. |
| [2. Using Robrix with Palpo and Octos](robrix-with-palpo-and-octos/02-using-robrix-with-palpo-and-octos.md) | **Use Robrix to chat with AI bots on your Palpo server.** Step-by-step with screenshots: log in, create rooms, invite bots, have conversations, and manage bots through the BotFather system. |
| [3. How Robrix, Palpo, and Octos Work Together](robrix-with-palpo-and-octos/03-how-robrix-palpo-octos-work-together.md) | **Understand the Application Service mechanism.** Learn how Octos registers as a Matrix App Service on Palpo, how messages flow from Robrix through Palpo to the AI bot, and how the BotFather system manages multiple bots. |
| [4. Federation with Palpo](robrix-with-palpo-and-octos/04-federation-with-palpo.md) | **Enable cross-server communication.** Configure Palpo for Matrix federation so users on different servers can chat with each other and access your AI bots. |

> Chinese:
> [1. 部署 Palpo 和 Octos](robrix-with-palpo-and-octos/01-deploying-palpo-and-octos-zh.md) ·
> [2. 在 Robrix 上使用 Palpo 和 Octos](robrix-with-palpo-and-octos/02-using-robrix-with-palpo-and-octos-zh.md) ·
> [3. Robrix、Palpo、Octos 协作原理](robrix-with-palpo-and-octos/03-how-robrix-palpo-octos-work-together-zh.md) ·
> [4. Palpo 联邦功能](robrix-with-palpo-and-octos/04-federation-with-palpo-zh.md)

---

## Robrix + OpenClaw (AI Agent Framework)

For users who want to connect OpenClaw AI agents to Matrix, then use Robrix to chat with them:

| Guide | Goal |
|-------|------|
| [1. Deploying OpenClaw with Matrix](robrix-with-openclaw/01-deploying-openclaw-with-matrix.md) | **Get OpenClaw connected to a Matrix homeserver.** Create a bot account, configure the Matrix channel plugin, and verify the connection so Robrix can chat with OpenClaw agents. |
| [2. Using Robrix with OpenClaw](robrix-with-openclaw/02-using-robrix-with-openclaw.md) | **Use Robrix to chat with OpenClaw agents.** Start conversations via DM or rooms, understand feature compatibility, and learn the differences from the Octos workflow. |
| [3. How Robrix and OpenClaw Work Together](robrix-with-openclaw/03-how-robrix-and-openclaw-work-together.md) | **Understand the client-based integration model.** Learn how OpenClaw connects to Matrix as a regular client (vs. Octos's Appservice model), how messages flow, and how E2EE works. |

> Chinese:
> [1. 部署 OpenClaw + Matrix](robrix-with-openclaw/01-deploying-openclaw-with-matrix-zh.md) ·
> [2. 在 Robrix 上使用 OpenClaw](robrix-with-openclaw/02-using-robrix-with-openclaw-zh.md) ·
> [3. Robrix 与 OpenClaw 协作原理](robrix-with-openclaw/03-how-robrix-and-openclaw-work-together-zh.md)

---

## Palpo and Octos Deployment Files

The [`palpo-and-octos-deploy/`](../palpo-and-octos-deploy/) directory (at the repository root) contains the runnable deployment files for Palpo and Octos, including Docker Compose and configuration templates:

```
palpo-and-octos-deploy/
├── compose.yml                  # Docker Compose orchestration
├── setup.sh                     # One-time setup script (clones source repos)
├── palpo.toml                   # Palpo homeserver config
├── .env.example                 # Environment variables template
├── appservices/
│   └── octos-registration.yaml  # Appservice registration (Palpo ↔ Octos)
├── config/
│   ├── botfather.json           # Bot profile and LLM settings
│   └── octos.json               # Octos global settings
├── repos/                       # Source repos (created by setup.sh, gitignored)
└── data/                        # Runtime data (created by Docker, gitignored)
```
