# Philosophy and Overall Architecture

> **Scope**: This chapter lays out HAgency's four design principles and its three-layer architecture — every mechanism in the chapters that follow has a place on this map. Prerequisites: the Preface. If you are evaluating whether this system deserves your trust, start reading here.

## Humans Are Participants, Not Spectators

A typical "multi-agent development system" works like this: you submit a requirement, a swarm of agents churns away inside a black box, and eventually a result gets tossed back at you. The human is shut out of the process — you can't see what happened between the agents, you can't course-correct midway, and you certainly can't gate dangerous operations.

HAgency's design principles are the exact opposite:

1. **Shared space**: Humans and agents talk in the same Matrix room. Every dispatch, report, and debate between agents is a visible message in the room — the process is the record.
2. **Humans decide**: Directional decisions ("commit a checkpoint first, or keep writing?" "send a draft PR directly?") are escalated by the agents and made by you.
3. **Humans authorize**: An agent's dangerous operations (`gh` write operations, sandbox-escaping commands) trigger **Owner approval** — a card delivered to an encrypted DM that lets the agent proceed only after you click "Approve once". Approvals are single-use, time-limited, and fail-closed.
4. **Humans can intervene**: You can `@` any agent at any moment to interject, change the plan, or even take over the task — because everything happens in a chat room right in front of you.

Of these four, "humans authorize" is **enforced** by the approval protocol and cryptography (Chapter 5.4, Chapter 6); "shared space" and "humans can intervene" are guaranteed by the Matrix protocol itself; "humans decide" is upheld by workflow convention — Chapter 5 shows what each of the four looks like in daily use and how strongly each is guaranteed.

## Three-Layer Architecture

```mermaid
flowchart TB
    subgraph human["Human workbench"]
        R["Robrix2<br/>Matrix client (macOS / Windows / Linux)"]
    end

    subgraph matrix["Communication substrate (Matrix)"]
        P["Palpo homeserver<br/>or any Matrix server"]
        room["Project board room<br/>(humans + agent puppets, plaintext)"]
        appr["Approval DM Approval: agent<br/>(E2EE, human ↔ bridge)"]
        P --- room
        P --- appr
    end

    subgraph ac["Agent hub (agent-chat, local-first)"]
        BR["bridge-matrix.js<br/>Matrix ↔ backend bidirectional bridge"]
        BE["backend-v2.js :8090<br/>Authoritative store for messages / tasks / approvals"]
        MCP["mcp-server.js<br/>Messaging tools for each agent"]
        TMUX["tmux runtime<br/>Claude Code / Codex"]
        DASH["server.js :8084<br/>Local monitoring dashboard"]
        BR <--> BE
        BE <--> MCP
        MCP <--> TMUX
        BE --- DASH
    end

    R <-->|Client-Server API| P
    BR <-->|"Puppet accounts @ac_&lt;agent&gt;<br/>Bridge bot @agent-bridge-&lt;user&gt;"| P
```

A few key design choices, each with its own "why":

**Agents appear on Matrix as "puppet accounts".** Each agent maps to an `@ac_<name>:<server>` account with its own avatar and display name in the room — to a human, it is just another room member. The payoff is **protocol-level equality**: Matrix primitives like @mentions, read receipts, threads, and power levels apply to humans and agents alike, so there is no need to invent a second interaction model for agents.

**robrix→agent delivery is pure Matrix.** You mention `@wf_coordinator` in the room → Palpo → the bridge receives the event → converts it into an agent-chat notification → nudges Claude Code / Codex in tmux; the agent's reply travels back along the same path under its puppet identity. There is no private side channel anywhere in between, which means **any Matrix client can join the collaboration** — Robrix2 is simply the one with the best experience.

**Authoritative state lives in the agent-chat backend.** Who is an operator, which room is bound to which group, whether an approval has been consumed — all of it is decided by the backend's persistent store. **Robrix2 is only a client for display and initiating operations, never a source of authorization.** This boundary is the foundation of the entire security model (see Chapter 6).

**Approvals go through a dedicated encrypted DM.** Each agent has an `Approval: <agent>` E2EE room whose only members are you, the bridge bot, and that agent. Approval details (including the command preview) appear only there; everyone else in the project room sees nothing but a redacted waiting status. The visibility of sensitive information is squeezed to the minimum (see Chapter 5.4).
