spec: task
name: "Telegram Bot UI Alignment — Phase 4b: /command@bot Explicit Addressing"
inherits: project
tags: [bot, ui, telegram-parity, slash-command, addressing]
depends: [task-tg-bot-menu-button]
estimate: 1d
---

## Intent

在多 bot 房间里，裸命令如 `/listbots` 是歧义的——谁应该响应？Telegram 通过
`/command@BotName` 语法让用户显式指定目标 bot。本任务在 Robrix2 的 slash
command 解析层加入对 `@bot` 后缀的支持，并把解析出的目标 bot 作为一次性
`target_user_id` 随消息发送，让 Octos gateway 的
`route_by_explicit_target` 正确路由，而不是走 `route_by_room` 的单 bot
fallback。

在**单 bot 房间**里，`@bot` 后缀是可选的；在**多 bot 房间**里，它是消歧
义的唯一可靠方式（除了 reply-to-bot）。本 spec 只做解析和路由，不做 bot
名字的自动补全 popup——那是后续增量。

## Decisions

- **语法**：`/command@localpart` 或 `/command@localpart:server.name`。
  解析器优先匹配完整 MXID 样式，其次匹配 localpart-only 简写。
- **解析位置**：新增 `parse_command_with_at_suffix(input: &str) ->
  Option<ParsedSlashCommand>`，放在 `mentionable_text_input.rs` 里现有
  slash command 工具函数旁边。签名：
  ```rust
  struct ParsedSlashCommand {
      command: String,       // "/listbots" (without the @bot suffix)
      target_localpart: Option<String>, // "octosbot_weather", None if no suffix
  }
  ```
- **消歧义**：解析时不立即解析到 `OwnedUserId`——解析器只返回 localpart，
  由调用方结合 `RoomScreenProps.known_bot_user_ids` 和
  `resolved_parent_bot_user_id` 映射到完整 MXID。
- **多 bot 房间解析失败**：如果 localpart 在 `known_bot_user_ids` 里找不
  到对应 bot（比如用户打错了），popup 提示"Bot '@xxx' not found in this
  room"并不发送消息。
- **单 bot 房间宽容**：如果房间只有一个已知 bot，且用户写的 `/cmd@other`
  指定的 bot 不在 room_bots 里，popup 提示并拒绝发送（**不**回退到单 bot）。
- **@bot 后缀自动补全**：本 spec 不实现。用户需要手动输入 `@` + bot 名字。
  Phase 4b+ 的增量任务可以加自动补全。
- **路由集成**：`routing_directives_for_message()` 或其调用方处理
  `ParsedSlashCommand::target_localpart` 后，在发送前把对应的
  `OwnedUserId` 作为 `target_user_id` 挂到消息上。这让 Octos 侧的
  `route_by_explicit_target`（优先级 1）捕获它，不依赖 room fallback。
- **与 mention 冲突**：解析器只在字符串以 `/` 开头**且** `@` 之前没有其
  他空白时才识别为 `/command@bot`。避免误把普通 `@mention` 当成命令。
- **消息 body 归一化（关键）**：发送时 body **必须被归一化**——`@bot` 后缀
  从 body 中剥离，只保留裸 `/command` 和参数。原因：现有 Octos BotFather 命令
  解析器按裸命令匹配（`/listbots`、`/bothelp` 等），不会自动忽略 Telegram
  式的 `@suffix`。如果 body 原样保留 `/listbots@octosbot_weather`，BotFather
  可能不识别命令。`target_user_id` 是机器可读的路由提示，body 是后端命令
  解析器看到的文本——两者分离。
  - 输入：`/listbots@octosbot_weather`
  - 发送 body：`/listbots`
  - 发送 target_user_id：`@octosbot_weather:127.0.0.1:8128`
  - 参数命令：`/createbot@octosbot foo` → body `/createbot foo`, target
    `@octosbot:...`
- **可见性恢复（UI）**：用户在输入框中看到的是完整的 `/command@bot`，这
  是 TG 风格的可见 addressing。归一化只发生在**发送路径**，不影响输入框
  显示。
- **Reply-to-bot 的交互**：如果用户同时在 reply-to-bot（`ReplyBot` 状态）
  又写了 `/command@other_bot`，**显式 `@other_bot` 胜出**——因为显式
  addressing 比 reply 语义更强。
- **ExplicitOverride 的交互**：本 spec **不**改变 `ExplicitOverride`
  enum 或其持久化——`/command@bot` 是一次性的 per-message routing，不是
  session-level override。
- **与 Phase 4a 的 precedence（关键）**：Phase 4a 定义了一组**已分类的
  BotFather 管理命令**（`/listbots`、`/bothelp`、`/createbot`、`/deletebot`
  等）。**Phase 4a 分类的命令优先级高于本 spec 的 bare-command 规则**：
  - 如果裸命令（无 `@bot` 后缀）在 `SLASH_COMMANDS` 里被分类为已知的
    BotFather 管理命令，则无论房间是 single-bot 还是 multi-bot，都按
    Phase 4a 的规则走——`target_user_id = parent_bot_user_id` +
    `explicit_room = false`。
  - 本 spec 的"multi-bot room 裸命令不 auto-resolve、继续 room-first"规
    则，**只对不在 `SLASH_COMMANDS` 里的未知命令（比如用户输错的
    `/foobar`）或用户自己构造的、没有分类的 slash 形式文本**生效。
  - 如果用户写 `/listbots@octosbot_weather`（显式 `@bot` 后缀），则按本
    spec 的 `/command@bot` 规则走，不触发 Phase 4a 的 classified-command
    路径——显式 addressing 胜出。

## Boundaries

### Allowed Changes
- src/shared/mentionable_text_input.rs
- src/room/room_input_bar.rs
- src/sliding_sync.rs (for passing target_user_id through send path, if needed)
- resources/i18n/en.json
- resources/i18n/zh-CN.json

### Forbidden
- 不要修改 Octos 后端（`route_by_explicit_target` 已经支持，无需新增逻辑）
- 不要修改 `ResolvedTarget` / `ExplicitOverride` enum 结构
- 不要实现 `@bot` 自动补全 popup（留给后续增量）
- 不要解析 display name 形式的 `@` 后缀（"@BotFather" 以显示名形式解析过
  于复杂，且容易 ambiguous）——只解析 localpart 和 MXID
- 不要新增 cargo 依赖
- 不要修改 Matrix event 的 custom fields 结构（复用 `org.octos.target_user_id`）

## Out of Scope

- `@bot` 后缀的输入自动补全
- 以 display name 形式指定 bot
- Bot 名字模糊匹配（fuzzy match）
- 通过 UI 菜单从候选 bot 列表里选择
- 多 `@` 后缀（一个消息同时寻址多个 bot）

## Completion Criteria

Scenario: Parse command with localpart-only suffix
  Test: test_parse_command_at_localpart
  Given the input "/listbots@octosbot_weather"
  When `parse_command_with_at_suffix` parses it
  Then the result is `Some(ParsedSlashCommand { command: "/listbots", target_localpart: Some("octosbot_weather") })`

Scenario: Parse command with full MXID suffix
  Test: test_parse_command_at_full_mxid
  Given the input "/listbots@octosbot:127.0.0.1:8128"
  When `parse_command_with_at_suffix` parses it
  Then the command field is "/listbots"
  And the `target_localpart` field is `Some("octosbot")` (extracted from the MXID)

Scenario: Parse bare command without suffix returns None target
  Test: test_parse_bare_command_no_target
  Given the input "/listbots"
  When `parse_command_with_at_suffix` parses it
  Then the command field is "/listbots"
  And the `target_localpart` field is `None`

Scenario: Parse command with trailing arguments preserves suffix
  Test: test_parse_command_at_with_args
  Given the input "/createbot@octosbot weather Weather Bot"
  When `parse_command_with_at_suffix` parses only the leading token
  Then the command field is "/createbot"
  And the `target_localpart` is `Some("octosbot")`
  And the arguments " weather Weather Bot" are preserved for later use

Scenario: Parser rejects bare mention "@user" that is not a command
  Test: test_parser_rejects_bare_mention
  Given the input "@octosbot hello"
  When `parse_command_with_at_suffix` inspects it
  Then the result is `None` (not a slash command)

Scenario: Parser rejects command with whitespace before @
  Test: test_parser_rejects_space_before_at
  Given the input "/listbots @octosbot"
  When `parse_command_with_at_suffix` inspects it
  Then the result is `Some(ParsedSlashCommand { command: "/listbots", target_localpart: None })`
  And the "@octosbot" portion is treated as part of arguments, not addressing

Scenario: Multi-bot room routes /command@known_bot to correct bot with normalized body
  Test: test_multi_bot_room_routes_to_specified_bot
  Given a room with `known_bot_user_ids = ["@octosbot_bob:127.0.0.1:8128", "@octosbot_weather:127.0.0.1:8128"]`
  And the user submits "/listbots@octosbot_weather"
  When the input bar builds the outgoing message
  Then the message `target_user_id` is `Some("@octosbot_weather:127.0.0.1:8128")`
  And the message body is "/listbots" (normalized, without the @bot suffix)
  And `explicit_room` is false

Scenario: Multi-bot room rejects /command@unknown_bot with popup
  Test: test_multi_bot_room_rejects_unknown_bot
  Given a room with `known_bot_user_ids = ["@octosbot_bob:127.0.0.1:8128"]`
  And the user submits "/listbots@octosbot_weather"
  When the input bar attempts to send the message
  Then the message send is rejected
  And a popup notification is shown with text "Bot '@octosbot_weather' not found in this room"
  And no message is sent to the room
  And the input text is preserved (not cleared)

Scenario: Single-bot room still honors explicit @bot suffix when matching
  Test: test_single_bot_room_honors_matching_suffix
  Given a room with `known_bot_user_ids = ["@octosbot:127.0.0.1:8128"]`
  And the user submits "/listbots@octosbot"
  When the input bar builds the outgoing message
  Then the message `target_user_id` is `Some("@octosbot:127.0.0.1:8128")`
  And the message body is "/listbots" (normalized)

Scenario: Single-bot room rejects wrong @bot suffix even though there is only one bot
  Test: test_single_bot_room_does_not_fallback_on_wrong_suffix
  Given a room with `known_bot_user_ids = ["@octosbot:127.0.0.1:8128"]`
  And the user submits "/listbots@other_bot"
  When the input bar attempts to send the message
  Then the message send is rejected
  And a popup notification is shown with text "Bot '@other_bot' not found in this room"
  And no fallback to the single known bot occurs

Scenario: Explicit @bot overrides reply-to-bot target
  Test: test_explicit_at_bot_overrides_reply_target
  Given the user is replying to a message from "@octosbot_bob:127.0.0.1:8128"
  And `known_bot_user_ids` contains both "@octosbot_bob" and "@octosbot_weather"
  When the user submits "/listbots@octosbot_weather"
  Then the outgoing `target_user_id` is `@octosbot_weather:127.0.0.1:8128` (NOT the reply target bob)
  And the Matrix `Reply` relation to bob's event is preserved on the message
  And `explicit_room` is false

Scenario: Unknown command without @ suffix in multi-bot room does NOT auto-resolve
  Test: test_bare_unknown_command_in_multi_bot_room_no_auto_target
  Given a room with `known_bot_user_ids = ["@octosbot_bob:127.0.0.1:8128", "@octosbot_weather:127.0.0.1:8128"]`
  And the user submits "/foobar" with no `@bot` suffix
  And "/foobar" is NOT in `SLASH_COMMANDS` (unclassified)
  When the input bar builds the outgoing message
  Then the message `target_user_id` is `None`
  And `explicit_room` is true (room-first behavior continues)
  And the message body is "/foobar"

Scenario: Classified BotFather command without @ suffix in multi-bot room DOES target parent bot
  Test: test_bare_classified_command_in_multi_bot_room_targets_parent
  Given a room with `known_bot_user_ids = ["@octosbot_bob:127.0.0.1:8128", "@octosbot:127.0.0.1:8128"]`
  And `@octosbot` is the parent/management bot
  And "/listbots" IS in `SLASH_COMMANDS` as a pure command (classified by Phase 4a)
  And the user submits "/listbots" with no `@bot` suffix
  When the input bar builds the outgoing message
  Then the message `target_user_id` is `"@octosbot:127.0.0.1:8128"` (the parent bot)
  And `explicit_room` is false
  And the message body is "/listbots"
  And this is the Phase 4a precedence — classified BotFather commands override
    the Phase 4b bare-command "no auto-resolve" rule

Scenario: @bot suffix in DM (single bound bot) routes to the bound bot
  Test: test_at_bot_suffix_in_dm_routes_to_bound_bot
  Given a DM with `bound_bot_user_id = "@octosbot:127.0.0.1:8128"`
  And `known_bot_user_ids = ["@octosbot:127.0.0.1:8128"]`
  And the user submits "/bothelp@octosbot"
  When the input bar builds the outgoing message
  Then the message `target_user_id` is `@octosbot:127.0.0.1:8128`
  And the message body is "/bothelp" (normalized)

Scenario: Parameterized command /command@bot args normalizes body to "/command args"
  Test: test_parameterized_command_at_bot_normalizes
  Given a room with `known_bot_user_ids = ["@octosbot:127.0.0.1:8128"]`
  And the user submits "/createbot@octosbot weather Weather Bot"
  When the input bar builds the outgoing message
  Then the message `target_user_id` is `@octosbot:127.0.0.1:8128`
  And the message body is "/createbot weather Weather Bot"
  And the body does NOT contain "@octosbot"

Scenario: Input box still shows the full @bot text while user is typing
  Test: test_input_box_preserves_at_bot_during_typing
  Given the user has typed "/listbots@octosbot" into the input
  When the input is rendered before sending
  Then the input widget displays the literal text "/listbots@octosbot"
  And the normalization only applies at send time (not during display)
