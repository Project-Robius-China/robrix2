# Agent Mission Room Design

Status: proposed product plan built on the simplified `agent2view` model.

This document defines the first higher-level product direction for Robrix2 as
an OpenClaw / OctOS-class agent client: a Matrix room can become a human-directed
multi-agent mission room. The core experience is not just chat, not just a
Kanban board, and not just an automation console. It is a room-scoped
`agent2view` app that gives humans a live mission-control surface over agent
planning, execution, review, and intervention.

Related design: [`agent-to-app-simplified-design.md`](agent-to-app-simplified-design.md).

## Product Thesis

Robrix2 should be the best client for agent collaboration because it can render
structured agent work as native room UI while preserving the Matrix room as the
audit trail.

The user should be able to:

- create or join a mission room.
- ask agents to pursue a goal.
- inspect the plan, task board, agent roster, blockers, and decisions.
- approve or redirect important steps.
- let agents execute autonomously within approved boundaries.
- step back into the conversation at any time and change direction.

The key is human-directed autonomy: agents may plan and execute, but shared
state changes that matter are visible and reviewable.

## Core Objects

```text
Room      = the collaboration space and audit log
Mission   = the shared goal being pursued
Plan      = the proposed decomposition of the goal
Task      = one unit of work owned by a human or agent
Agent     = a participant with role, status, and current assignment
Decision  = a human or agent decision recorded for later context
Blocker   = a reason progress is stopped
Action    = a human or agent intervention request
```

## App Scope

Agent Mission Room is a `room` scoped app:

```json
{
  "org.octos.app": {
    "type": "mission_room",
    "version": 1,
    "scope": "room",
    "app_id": "mission.main",
    "initial_state": {}
  }
}
```

Its instance key is:

```text
room_id + app_id
```

Multiple messages in the same room may render the same mission instance. This is
intentional: the room timeline remains the record, while the mission view is the
current projection of that shared state.

## Producer Contract

OpenClaw / OctOS producers should treat Robrix as a consumer of Matrix events,
not as a private RPC peer. A mission-state update is a normal room message whose
original event content contains the app envelope:

```json
{
  "body": "Mission update: plan is waiting for approval.",
  "msgtype": "m.text",
  "org.octos.app": {
    "type": "mission_room",
    "version": 1,
    "scope": "room",
    "app_id": "mission.main",
    "initial_state": {}
  },
  "org.octos.actions": [
    { "id": "approve_plan", "label": "Approve plan", "style": "primary" },
    { "id": "request_plan_changes", "label": "Request changes", "style": "secondary" }
  ]
}
```

Producer rules:

- Always include `scope: "room"` and a stable `app_id` for mission-room events.
- Use `mission.main` for the default room mission unless the room deliberately
  hosts multiple missions.
- Emit a full `initial_state` snapshot for every shared mission update. Robrix
  can keep local view state, but shared truth is the latest valid Matrix event.
- Keep `body` useful as the fallback and audit summary.
- Put shared human controls in `org.octos.actions`; the Splash mission card may
  display pending actions, but it is not the transport for shared approvals.
- Do not rely on `m.replace` edits to change mission app state. Robrix reads the
  app envelope from the original event content.
- Do not rely on `m.replace` edits to change mission action buttons. When an
  event carries a valid `org.octos.app` envelope, Robrix also reads
  `org.octos.actions` from original event content.

When a human clicks an OctOS action button, Robrix sends an
`org.octos.action_response` that targets the original producer. The producer is
responsible for validating the response, applying policy, and emitting a new
mission-room snapshot event.

For app-originated actions, Robrix includes source app metadata in the response:

```json
{
  "org.octos.action_response": {
    "action_id": "approve_plan",
    "source_event_id": "$mission123",
    "app": {
      "type": "mission_room",
      "version": 1,
      "scope": "room",
      "app_id": "mission.main"
    }
  }
}
```

The producer may still use `source_event_id` as the audit anchor, but the
embedded `app` object makes routing explicit when a room hosts multiple mission
apps.

For account-wide mission summaries, producers may emit:

```json
{
  "body": "Mission dashboard update.",
  "msgtype": "m.text",
  "org.octos.app": {
    "type": "mission_dashboard",
    "version": 1,
    "scope": "account",
    "app_id": "missions.global",
    "initial_state": {}
  }
}
```

The account dashboard is a global app instance keyed by `account_id + app_id`;
it should summarize mission rooms, not replace room-scoped mission truth.

## User Experience

The first screen is **Mission Control first**.

```text
Top        Mission goal, phase, progress, next human gate
Right      Agent roster and current activity
Middle     Kanban-style task board
Bottom L   Meeting / decision stream
Bottom R   Human controls and pending approvals
```

This layout keeps the human in command. The board shows execution state, but the
top-level mission status and pending human gate decide what needs attention.

## MVP Flow

The first version uses an approval-gated plan flow:

```text
Human creates Goal
  -> Planner agent proposes Plan + Tasks
  -> Mission app shows Pending Approval
  -> Human approves, edits, or rejects
  -> Executor agents claim approved tasks
  -> Board updates Planning / Doing / Review / Blocked / Done
  -> Reviewer agent or human reviews outputs
  -> Decisions and blockers are recorded in the room timeline
```

Agents can propose work, but planned tasks do not enter execution until a human
approves the plan or the relevant task subset.

## Mission State V1

The first state shape should stay explicit and small:

```json
{
  "goal": {
    "title": "Ship room-scoped agent2view runtime",
    "status": "planning"
  },
  "phase": "planning",
  "tasks": [
    {
      "id": "task-1",
      "title": "Define AgentViewSession state model",
      "status": "planning",
      "owner_agent": "planner",
      "priority": "high",
      "requires_human_approval": true
    }
  ],
  "agents": [
    {
      "id": "planner",
      "role": "planner",
      "status": "waiting_human",
      "current_task_id": "task-1"
    }
  ],
  "pending_human_actions": [
    {
      "id": "approve-plan-1",
      "kind": "approve_plan",
      "label": "Approve proposed plan"
    }
  ],
  "decisions": [],
  "blockers": []
}
```

Allowed status values:

```text
goal.status: planning | active | paused | completed
task.status: planning | approved | doing | review | blocked | done
agent.status: idle | working | blocked | waiting_human
```

The state should be optimized for a small static Splash template first. Do not
start with arbitrary nested boards, recursive subtasks, or rich per-agent logs.

## Human Actions

First-version human intervention actions:

```text
approve_plan
request_plan_changes
pause_mission
resume_mission
reassign_task
change_priority
mark_blocked
request_review
approve_result
```

Actions split into two categories:

### Local View Actions

These are immediate UI actions that do not become shared facts:

- expand/collapse task details.
- select an agent.
- filter board lanes.
- switch between board and meeting summary.

They may be handled by local `AgentViewSession` reducers.

### Shared Mission Actions

These change mission truth and must go through Matrix / OctOS action response:

- approving a plan.
- changing who owns a task.
- pausing or resuming the mission.
- marking a task blocked.
- approving a result.

Robrix2 should send an action response. The agent then emits a new event with
updated mission state. Robrix2 may optimistically update the local view, but the
event remains the authority.

## State Ownership

```text
Mission truth      = agent-produced Matrix events
Robrix view state  = room_id + app_id AgentViewSession
Widget state       = disposable; never authoritative
```

The view session exists so the app remains interactive across redraws and
`PortalList` virtualization. It must not silently rewrite Matrix history.

For V1:

- `message` scoped apps remain isolated by `room_id + event_id`.
- Mission Room uses `room` scope: `room_id + "mission.main"`.
- Account/global dashboards are deferred.

## Agent Roles

The UI should not assume a fixed set of agents, but the MVP should support these
roles:

```text
planner     decomposes goal into tasks
executor    performs assigned tasks
reviewer    checks outputs and risks
operator    coordinates tools, CI, deployments, or external systems
```

Agents are shown in the roster with:

- role.
- current status.
- current task.
- blocker or waiting reason.
- autonomy mode, if supplied by the backend.

## Autonomy Levels

Mission Room should make autonomy visible. A future state field may include:

```text
manual       agent proposes, human approves every action
supervised   agent executes approved task, asks at gates
autonomous   agent executes within declared policy
paused       no agent execution
```

For MVP, this can be rendered as text and controlled through `pause_mission` /
`resume_mission`. Fine-grained policy is future work.

## Rendering Strategy

The first implementation should use static templates:

```text
src/home/app_registry/templates/mission_room/mission_control.splash
```

The Rust side should mirror weather/news:

```text
src/home/app_registry/mission_room.rs
  - validate initial_state
  - normalize task/agent slots for the template
  - select mission_control template
  - render through SplashHost
```

The first template may cap visible items:

```text
tasks: first 6 or first N by lane
agents: first 4
pending_human_actions: first 3
decisions/blockers: compact summaries
```

This keeps the template simple and avoids needing virtualized dynamic lists
inside Splash in the first version.

## Phases

### Phase 1: Static Mission Room

Render a `mission_room` envelope as a room-scoped Mission Control card.

Scope:

- parse `scope: "room"` and `app_id`.
- render mission state with static Splash.
- no button actions yet.
- prove `room_id + app_id` identity.

### Phase 2: Human Approval Loop

Add shared actions for:

- `approve_plan`
- `request_plan_changes`
- `pause_mission`
- `resume_mission`

These actions send Matrix / OctOS action responses and wait for agent-produced
state updates.

### Phase 3: Task Board Runtime

Add task-level actions:

- `reassign_task`
- `change_priority`
- `mark_blocked`
- `request_review`
- `approve_result`

This phase makes the Kanban board operational rather than purely informative.

### Phase 4: Multi-Agent Orchestration View

Improve the agent roster:

- current task.
- dependency edges.
- waiting reason.
- autonomy mode.
- last heartbeat or last update.

### Phase 5: Account-Level Ops

Add an account/global dashboard that summarizes multiple mission rooms. This is
not required for the room-scoped MVP.

## Non-Goals

- No LLM-generated Splash template in the MVP.
- No arbitrary app plugin system.
- No account-wide persistence in Phase 1.
- No hidden local mutation of shared mission truth.
- No full project-management clone.
- No recursive subtask system until flat tasks prove insufficient.

## Success Criteria

The MVP is successful when:

- A room can display a `mission_room` app from an `org.octos.app` event.
- The app is keyed by `room_id + app_id`, not by a single message.
- The Mission Control view shows goal, phase, tasks, agents, blockers, and pending human actions.
- A human can approve or reject a proposed plan through an action response.
- The agent can emit a new event with updated mission state after human action.
- Robrix2 never treats local view state as shared truth without an agent-confirmed event.
