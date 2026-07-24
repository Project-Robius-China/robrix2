# Preface: What Is HAgency

> **Scope**: This chapter answers "what HAgency is and why it exists," and lays out how to read this book. It has no prerequisites — every reader starts here.

**HAgency = Human + Agency.**

AI coding agents (Claude Code, Codex, ...) can already carry out a large share of engineering work on their own. But most "multi-agent" products push the human to the margins: you become a button that fires off a prompt, and the rest of the process is neither visible nor open to intervention.

HAgency aims for a different shape: **in a world where humans and agents coexist, the human remains a subject with agency**. Humans and agents converse, divide work, argue, and report in the same space; key decisions are made by the human, and high-risk operations are authorized by the human; the agent team runs autonomously, yet stays transparent to the human and open to intervention at any moment.

This is not a standalone product, but a collaboration system assembled from three open-source projects:

| Project | Role |
|------|------|
| [Robrix2](https://github.com/Project-Robius-China/robrix2) | A native Matrix client written in Rust + Makepad — the human's workbench |
| [agent-chat](https://github.com/ZhangHanDong/agent-chat) | A local-first agent coordination system — the agents' dispatch hub and Matrix bridge |
| [Palpo](https://github.com/palpo-im/palpo) | A Matrix homeserver written in Rust — the neutral communication substrate |

The three speak the **Matrix protocol** as their common language. Choosing Matrix is no accident:

- **Open protocol**: anyone's agent-chat instance and any Matrix client can join the same space — the screenshots in this book include a live example of two people's agent teams collaborating in the same room; the space runs on your own server and can further interoperate with the entire Matrix federation;
- **End-to-end encryption**: the human-to-agent authorization channel (the approval DM) is protected by E2EE — even the server cannot read approval contents;
- **Neutral substrate**: humans participate via Robrix2 (or any Matrix client), agents participate through bridged puppet accounts — the two sides are fully equal at the protocol level.

## Before You Read

### Prerequisites

- **Required**: basic command-line skills; everyday familiarity with Git and GitHub;
- **Nice to have**: experience with any Matrix client (Element, etc.); experience with Claude Code or Codex CLI;
- **Not required**: Rust or Makepad development experience (unless you want to modify Robrix2 itself); Matrix protocol internals.

### Recommended Reading Paths

**Path A: I want to get it running as fast as possible** (users)

> Preface → Chapter 4 deployment guide (pick one route) → Chapter 5 team collaboration in practice (follow the screenshots)

**Path B: I first want to understand why it deserves trust** (evaluators / security perspective)

> Preface → Chapter 3 philosophy and architecture → Chapter 5.4 Owner approval → Chapter 8 security model → then back to deployment

### Version and Evidence Baseline

This book was verified on **2026-07-24**, against Robrix2 docs commit `d4f5c4c8` and agent-chat mainline `ad45f67`. The Project Board screenshots come from commit `3102a5f` on the not-yet-merged `feat/project-board` branch, which is why Chapter 5.6 labels it a preview capability rather than a released mainline feature. Palpo's deployment behavior follows this repository's `palpo-and-octos-deploy/` and the actual test node; before the book's formal release, Palpo's exact commit or release version should also be pinned.

The book uses four evidence labels to avoid presenting behavior observed in one demo as a system guarantee:

| Label | Meaning |
|------|------|
| **Protocol-enforced** | The backend / bridge validates and fails closed; it cannot be bypassed by relying on agent self-discipline |
| **Current implementation** | Code and tests exist at the commits above, but explicit preconditions may apply |
| **Workflow convention** | Defined by skill / prompt; may break down when the agent, relay, or session is interrupted |
| **Planned** | A design direction or partial foundation exists, but the full Robrix2-to-backend product path is not yet in place |

This system is under active development; commands and interfaces will keep evolving. All screenshots in the book come from real running sessions; a screenshot proves "what happened in that run" — it does not, by itself, prove something has become a universal, automatic product capability.

## Structure of This Book

**Part One** (Chapters 1–2) briefly introduces Robrix2 as a Matrix IM client in its own right — even without connecting any agents, it is a complete, usable instant messaging tool.

**Part Two** (Chapters 3–8) is the heart of the book: philosophy and architecture, two deployment routes, six screenshot-driven collaboration scenarios (inviting agents in, the board room, Threads, approvals, the four-role workflow, and the Project Board), plus the agent pool, multi-user boundaries, troubleshooting, and the security model.
