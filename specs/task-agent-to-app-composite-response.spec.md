spec: task
name: "Agent-to-App Composite Response — Card Summary + Same-Source Detail Body"
inherits: project
tags: [bot, agent-to-app, composite-response, matrix, robrix, octos]
depends: [task-agent-to-app-system, task-agent-to-app-producer-routing, task-agent-to-app-splash-host-evolution]
estimate: 2d
---

## Intent

Agent-to-app replies are not card-only replies. A useful app reply has two
surfaces from the same semantic result: a compact card summary for quick
scanning, and a detailed text body for explanation, accessibility, and clients
that cannot render apps. This spec defines the minimal composite response
contract without changing the `org.octos.app` envelope shape.

The first targets are weather and news. Weather cards summarize conditions and
guidance; their body expands clothing, commute, umbrella, UV, and time-of-day
details. News cards summarize the topic and headline list; their body expands
context, implications, and sources.

## Decisions

- Reuse the existing `DispatchDecision::AppReply { initial_state, body }` pair.
- Do not add `detail_text` or `style` as new top-level `org.octos.app` fields in
  this task.
- `initial_state` renders the app card summary.
- Matrix `body` is the same-source detail text, not a hidden fallback-only field.
- `initial_state` and `body` must be produced from one capability data fetch.
- App-capable Robrix clients render both surfaces: app card first, detail text
  bubble below it.
- Non-app-capable clients render only Matrix `body`.
- Resolver provider/schema failure is not a user-facing app failure; it falls
  through to the legacy LLM/tool path with structured logs.
- Detail bubble color should visually align with the card accent, but the first
  implementation may use a conservative capability-level accent mapping.

## Boundaries

### Allowed Changes
- `src/home/app_registry/**`
- `src/home/room_screen.rs`
- `src/home/timeline/**`
- `specs/task-agent-to-app-*.spec.md`
- OctOS capability producer code that builds `body` from capability data

### Forbidden
- Do not change the required `org.octos.app` envelope fields:
  `type`, `version`, `initial_state`.
- Do not generate detail text from a second independent LLM call when a
  capability app reply already has fetched deterministic data.
- Do not hide Matrix `body` for app-capable Robrix clients when an app card was
  rendered successfully.
- Do not show two contradictory facts between card and body.
- Do not run `cargo fmt`.

### Out of Scope
- Interactive app actions.
- Stateful mini-app ticks.
- Live news API integration or news factuality guarantees.
- Adding a new protocol version solely for `detail_text`.

## Acceptance Criteria

Scenario: Weather app reply renders card plus same-source detail body
  Test: test_weather_app_reply_renders_card_and_body
  Given an inbound Matrix event with `org.octos.app.type = "weather"`
  And the event has Matrix `body` containing clothing and commute guidance
  When Robrix renders the timeline item
  Then the weather card is visible
  And the body is rendered below the card as a detail bubble
  And the body is not hidden as accessibility-only fallback text

Scenario: News app reply renders card plus same-source detail body
  Test: test_news_app_reply_renders_card_and_body
  Given an inbound Matrix event with `org.octos.app.type = "news"`
  And the event has Matrix `body` containing headline explanation text
  When Robrix renders the timeline item
  Then the news card is visible
  And the body is rendered below the card as a detail bubble
  And the detail bubble uses a non-default visual treatment aligned with the
  news card accent

Scenario: Non-app-capable clients still get useful text
  Test: test_app_reply_body_remains_plain_text_fallback
  Given OctOS produces an app reply
  When the receiver ignores `org.octos.app`
  Then Matrix `body` alone contains the useful answer
  And the answer does not depend on reading `initial_state`

Scenario: Card and body use the same capability data
  Test: test_capability_app_reply_state_and_body_share_data_source
  Given a weather or news capability producer
  When it builds `initial_state` and `body`
  Then both are derived from the same fetched data object
  And no second independent LLM call generates the body facts

Scenario: Renderer failure falls back to body, not service unavailable
  Test: test_app_renderer_failure_shows_body_fallback
  Given an app envelope whose Splash template is rejected by the host
  And Matrix `body` contains a valid detail answer
  When Robrix renders the timeline item
  Then no unsafe Splash content is displayed
  And the body detail bubble remains visible to the user
  And no "服务暂不可用" text is synthesized by Robrix

Scenario: Resolver provider failure does not block legacy reply
  Test: should_retry_once_on_resolver_provider_error_then_fall_through_to_legacy
  Given the OctOS resolver provider fails twice
  When dispatcher handles the user turn
  Then it records `fallback_reason = resolver_provider_error`
  And it returns `FallThrough`
  And the legacy LLM/tool pipeline is allowed to answer the user

Scenario: Resolver schema failure does not block legacy reply
  Test: should_retry_once_on_invalid_json_then_fall_through_to_legacy
  Given the OctOS resolver returns invalid JSON twice
  When dispatcher handles the user turn
  Then it records `fallback_reason = resolver_schema_error`
  And it returns `FallThrough`
  And the legacy LLM/tool pipeline is allowed to answer the user
