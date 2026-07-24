# Deployment Guide

> **Scope**: This chapter helps you choose between the two deployment routes and gives you a pre-flight checklist. Prerequisites: Chapter 3 (you know what each of the three layers is).

A complete HAgency setup consists of three components, of which the **Matrix server** can be self-hosted or cloud-based:

| Component | Required | Where it runs |
|------|------|---------|
| Matrix homeserver (Palpo) | ✅ | Local Docker, **or** cloud (Meldry / matrix.org) |
| agent-chat | ✅ | The machine that runs your agents (local-first by design) |
| Robrix2 | ✅ | Your desktop |

## Two Routes

- **[Local deployment](deploy-local.md)** — Palpo + agent-chat + Robrix2 all run on your own machine. Your data stays entirely in your hands; ideal for development, intranet teams, and privacy-sensitive scenarios.
- **[Cloud Matrix](deploy-cloud.md)** — Use a managed Palpo tenant created in one click on [Meldry](https://tenant.meldry.com/) (or the official matrix.org node) as your Matrix server, and run only agent-chat and Robrix2 locally. This spares you the operational cost of self-hosting a homeserver and naturally lets remote members join.

> On either route, agent-chat runs on **your own** machine — the coding agents need access to your code repositories and tmux, which is precisely what "local-first" means. The only thing that changes is where the Matrix server lives.

## Pre-Flight Checklist

| Item | Purpose | Notes |
|--------|------|------|
| Docker (local route only) | Runs Palpo + PostgreSQL | Not needed on the cloud route |
| Node.js 22+ and tmux | Runs agent-chat and the runtimes it manages | Check with `node -v` |
| Rust toolchain | Builds Robrix2 | Install via `rustup` |
| Claude Code or Codex CLI | At least one coding runtime | Install both to try the heterogeneous final review in Chapter 5.5 |
| A code repository to collaborate on | What the agents work on | Any local Git repository |

## What You Get After Deployment

After following the next chapter (or the one after), you will have: a Matrix server you can log in to; a coding agent running in tmux with a Matrix puppet identity; a project board room bound to an agent group; and a dedicated encrypted approval room for each agent. Every screenshot scenario in Chapter 5 can be reproduced step by step in this environment.

If you get stuck along the way, each of the two chapters ends with a "Troubleshooting" table — most failures cluster in three places: a wrong homeserver address, the bridge's trust gate not configured with your own account, and forgetting to create the group before `!bindroom`.
