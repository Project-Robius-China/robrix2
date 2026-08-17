# Deployment Guide

> **Scope**: This chapter helps you choose between the two deployment routes and gives you a pre-flight checklist. Prerequisites: Chapter 3 (you know what each of the three layers is).

A complete HAgency setup consists of three components, of which the **Matrix server** can be self-hosted or cloud-based:

| Component | Required | Where it runs |
|------|------|---------|
| Matrix homeserver (Palpo) | ✅ | Local Docker, **or** cloud (Meldry / matrix.org) |
| agent-chat | ✅ | The Agent machine: backend, dashboard, push relay, Matrix bridge, managed runtimes |
| Robrix2 | ✅ | Your desktop |

## Two Routes

- **[Local deployment](deploy-local.md)** — Palpo + agent-chat + Robrix2 all run on your own machine. Your data stays entirely in your hands; ideal for development, intranet teams, and privacy-sensitive scenarios.
- **[Cloud Matrix](deploy-cloud.md)** — Use a managed Palpo tenant created in one click on [Meldry](https://tenant.meldry.com/) (or the official matrix.org node) as your Matrix server, and run only agent-chat and Robrix2 locally. This spares you the operational cost of self-hosting a homeserver and naturally lets remote members join.

> On either route, agent-chat runs on **your own** machine — the coding agents need access to your code repositories and tmux, which is precisely what "local-first" means. The only thing that changes is where the Matrix server lives.

## Pre-Flight Checklist

| Item | Purpose | Notes |
|--------|------|------|
| Docker (local route only) | Runs Palpo + PostgreSQL | Not needed on the cloud route |
| Node.js 22+ and tmux | Runs agent-chat and managed runtimes | Linux has the supported installer; macOS currently uses a development run path |
| Rust toolchain | Builds Robrix2 | Install via `rustup` |
| Claude Code or Codex CLI | At least one coding runtime | Install both to try the heterogeneous final review in Chapter 5.5 |
| A code repository to collaborate on | What the agents work on | Any local Git repository |

## What You Get After Deployment

After the next chapter you will have a Matrix server, one managed Agent, an **unencrypted** project room bound to a group, and an approval room once its owner accepts the invitation. The four-role workflow and Project Board require the additional preparation described in their own chapters.

For failures, use [Operations Acceptance and Troubleshooting](operations.md). Common causes are missing required secrets, the wrong full MXID in the trust gate, no owner-originated Agent invite, an unmanaged runtime, or an approval invitation that has not been accepted.
