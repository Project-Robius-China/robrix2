spec: task
name: "Octos Command Sync for Robrix BotFather Rooms"
inherits: project
tags: [bot, octos, slash-command, matrix-metadata]
---

## Intent

Align Robrix2's BotFather slash-command discovery and Matrix metadata with the current Octos App Service command protocol. This is a Robrix-only command-sync change: Robrix exposes commands that Octos already implements, and sends `/allbots` candidate targets as untrusted metadata for Octos to validate server-side.

## Decisions

- Command discovery is context-aware: `ManagementDm` offers BotFather management commands, while `ManagementRoom` additionally offers `/allbots`.
- `/schedule`, `/schedules`, and `/unschedule` are Octos baseline management commands and are available in both BotFather DM and management room contexts.
- `/allbots` is available only in management room context. It must not be discovered or classified as a management command in BotFather DM.
- Robrix derives `/allbots` `org.octos.broadcast_targets` only from persisted room bot bindings, never from detected room members or `room_bot_user_ids`.
- Robrix excludes the parent BotFather user ID from `/allbots` broadcast targets and sends only child bot candidates.
- Octos remains authoritative: `org.octos.broadcast_targets` is a candidate snapshot, not a permission decision.

## Boundaries

### Allowed Changes
- specs/task-octos-command-sync.spec.md
- src/shared/mentionable_text_input.rs
- src/room/room_input_bar.rs
- src/home/room_screen.rs
- src/sliding_sync.rs
- src/app.rs
- resources/i18n/en.json
- resources/i18n/zh-CN.json

### Forbidden
- Do not modify `~/Work/Projects/FW/octos` or any Octos source file.
- Do not cherry-pick or merge the old `feat/tg-bot-phase5-orchestration` branch wholesale.
- Do not restore automatic bot detection as persisted room binding.
- Do not expose child-bot commands such as `/new`, `/s`, or `/sessions`.
- Do not put Octos App Service baseline commands behind the `agent_chat` Cargo feature.
- Do not add new cargo dependencies.
- Do not run `cargo fmt` or `rustfmt`.

## Out of Scope

- Changing Octos server-side `/allbots` validation or fan-out behavior.
- UI for selecting child bot targets manually.
- Dynamic command discovery from Octos at runtime.
- Child-bot command catalogs.
- Automatic room binding or auto-persisting detected bots.

## Completion Criteria

Scenario: BotFather DM discovers schedule commands but not allbots
  Test: test_management_dm_command_catalog_excludes_allbots
  Given a BotFather direct-message context
  When Robrix builds slash-command discovery results
  Then `/schedule`, `/schedules`, and `/unschedule` are present
  And `/allbots` is absent

Scenario: Management room discovers allbots and schedule commands
  Test: test_management_room_command_catalog_includes_allbots
  Given a persisted BotFather management-room binding
  When Robrix builds slash-command discovery results
  Then `/schedule`, `/schedules`, `/unschedule`, and `/allbots` are present

Scenario: Typed allbots submit is classified only in management room context
  Test: test_allbots_classification_requires_management_room_context
  Given the submitted command text is "/allbots summarize"
  Given a BotFather direct-message context
  When the user submits that command text
  Then Robrix does not classify it as a management command
  Given a persisted BotFather management-room binding
  When the user submits that command text
  Then Robrix classifies it as a management command targeting BotFather

Scenario: Allbots metadata uses persisted child bindings only
  Test: test_allbots_broadcast_targets_use_persisted_child_bindings_only
  Given the submitted command text is "/allbots summarize"
  Given persisted bindings contain the parent BotFather and one child bot
  And `room_bot_user_ids` also contains a detected-only bot
  When the user submits that command text in a management room
  Then `broadcast_target_user_ids` contains the persisted child bot
  And it does not contain the parent BotFather
  And it does not contain the detected-only bot

Scenario: Allbots metadata is not produced outside management rooms
  Test: test_allbots_broadcast_targets_require_management_room_context
  Given the submitted command text is "/allbots summarize"
  Given a BotFather direct-message context
  When Robrix evaluates broadcast targets for that command text
  Then no `broadcast_target_user_ids` are produced
  Given a child-bot room context
  When Robrix evaluates broadcast targets for that command text
  Then no `broadcast_target_user_ids` are produced

Scenario: Sliding sync writes broadcast targets metadata
  Test: test_send_message_adds_octos_broadcast_targets
  Given a raw Matrix message content value
  And `broadcast_target_user_ids` contains child bot user IDs
  When Robrix adds Octos routing metadata
  Then the content includes `org.octos.broadcast_targets`
  And the field contains the child bot user IDs as strings
