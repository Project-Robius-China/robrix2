# Preface: What Is HAgency

> **Scope**: This chapter answers what HAgency is and why it exists, and lays out how to read the book. No prerequisites — every reader starts here.

**HAgency = Human + Agency.**

AI coding agents (Claude Code, Codex, ...) can already carry out a great deal of engineering work on their own. But most "multi-agent" products push the human to the margins: you become a button that fires off a prompt, and the rest of the process is neither visible nor open to intervention.

HAgency is after a different shape: **in a world of humans and agents, humans remain the agentic subject**. Humans and agents converse, divide up work, argue, and report in the same space; key decisions are made by humans, and high-risk operations require human authorization. Agent teams run autonomously — yet always transparently to humans, and open to intervention at any moment.

This is not a single product, but a collaboration system composed from three open-source projects:

| Project | Role |
|------|------|
| [Robrix2](https://github.com/Project-Robius-China/robrix2) | A native Matrix client written in Rust + Makepad — the human's workbench |
| [agent-chat](https://github.com/ZhangHanDong/agent-chat) | A local-first agent coordination system — the agents' dispatch hub and Matrix bridge |
| [Palpo](https://github.com/palpo-im/palpo) | A Matrix homeserver written in Rust — the neutral communication substrate |

The three speak the **Matrix protocol** as their common language. Choosing Matrix is no accident:

- **Open protocol**: anyone's agent-chat instance and any Matrix client can join the same space — the screenshots in this book include a real case of two people's agent teams collaborating in one room; the space runs on your own server and can further federate with the entire Matrix network;
- **End-to-end encryption**: the channel through which humans authorize agents (the approval DM) is protected by E2EE — not even the server can read the approval contents;
- **Neutral substrate**: humans participate through Robrix2 (or any Matrix client), agents participate through bridged puppet accounts — at the protocol level, the two sides are fully equal.

## Before You Read

### Prerequisites

- **Required**: basic command-line skills; everyday familiarity with Git and GitHub;
- **Nice to have**: experience with any Matrix client (Element, etc.); experience with Claude Code or Codex CLI;
- **Not required**: Rust or Makepad development experience (unless you want to modify Robrix2 itself); Matrix protocol internals.

### Recommended Reading Paths

**Path A: I want to get it running as fast as possible** (users)

> Preface → Chapter 4 Deployment Guide (pick one route) → Chapter 5 Team Collaboration in Practice (follow the screenshots)

**Path B: I want to understand why it can be trusted first** (evaluators / security-minded readers)

> Preface → Chapter 3 Concept and Architecture → Chapter 5.4 Owner Approval → Chapter 6 Security Model → then back to deployment

### Version and Evidence Baseline

This book was verified on **2026-07-24** against Robrix2 documentation commit `d4f5c4c8` and agent-chat mainline `ad45f67`. Project Board screenshots come from the not-yet-merged `feat/project-board` commit `3102a5f`, so Chapter 5.6 labels it as a preview rather than a shipped mainline feature. Palpo behavior is based on this repository's `palpo-and-octos-deploy/` artifact and the tested node; the published book should pin the exact Palpo commit or release.

The book uses four evidence labels:

| Label | Meaning |
|------|------|
| **Protocol-enforced** | backend/bridge validates and fails closed; Agent self-discipline cannot bypass it |
| **Current implementation** | present in the pinned commits, with stated preconditions |
| **Workflow convention** | required by a skill/prompt and may fail if the Agent, relay, or session stops |
| **Planned** | a design direction or partial foundation, not an end-to-end Robrix2 product path yet |

Every screenshot is from a real run. A screenshot proves what happened in that run; it does not by itself prove a universal, automatic product guarantee.

## How This Book Is Organized

**Part I** (Chapters 1–2) briefly introduces Robrix2 as a Matrix IM client in its own right — even without any agents attached, it is a complete, usable instant messenger.

**Part II** (Chapters 3–6) is the heart of the book: architecture, two deployment routes, six screenshot-driven scenarios, Agent pools and multi-user boundaries, troubleshooting, and the security model.
