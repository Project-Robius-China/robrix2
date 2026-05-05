spec: task
name: "Bot Markdown Diagram Modal Renderer"
inherits: project
tags: [bot, timeline, markdown, mermaid, diagram, makepad, modal]
depends: [task-tg-bot-timeline-cards]
estimate: 1d
---

## Intent

OctOS bot Markdown replies can now render code, tables, Mermaid, and diagram
blocks in the room timeline. The timeline renderer must stay stable while
scrolling, so interactive diagram behavior must not live inside virtualized
`PortalList` rows. This task adds a top-level diagram preview modal that opens
from static timeline previews and safely provides zoom, pan, reset, and Mermaid
animation outside the virtualized list.

## Decisions

- Timeline `mermaid_block` and `diagram_block` remain static previews.
- Timeline preview widgets must not schedule continuous `NextFrame` animation.
- Timeline preview widgets must not keep pan/zoom hit-test state for interaction.
- Clicking a static Mermaid preview emits a `RoomScreen`-handled action carrying
  the original Mermaid source.
- Clicking a static diagram preview emits a `RoomScreen`-handled action carrying
  the original diagram source.
- The interactive renderer lives in a top-level `Modal` under `RoomScreen`, not
  under a `PortalList` item.
- Mermaid modal rendering reuses `streaming-markdown-kit::render_mermaid_to_svg`
  and Makepad SVG drawing; it must not add a direct `rusty-mermaid` dependency.
- Diagram modal rendering reuses `makepad-diagram-kit::DiagramView`.
- The modal supports zoom, pan, and reset without affecting timeline scroll
  state.
- Closing the modal drops or clears the interactive source and stops modal-only
  animation.

## Boundaries

### Allowed Changes
- `src/home/room_screen.rs`
- `specs/task-bot-diagram-modal-renderer.spec.md`
- `issues/011-timeline-mermaid-drawlist-corruption.md`

### Forbidden
- Do not re-enable Mermaid pan/zoom hit testing inside timeline `PortalList`
  rows.
- Do not re-enable continuous Mermaid flow-dot `NextFrame` animation inside
  timeline `PortalList` rows.
- Do not add a direct `rusty-mermaid` dependency to Robrix.
- Do not replace the existing `streaming-markdown-kit` / `makepad-diagram-kit`
  rendering path.
- Do not change Matrix message sending, OctOS gateway behavior, or app-to-agent
  envelope production.
- Do not run `cargo fmt`.

### Out of Scope
- Editing Mermaid or diagram source in the modal.
- Exporting diagrams as SVG/PNG.
- Replacing the room timeline `PortalList`.
- General-purpose image viewer changes.
- New OctOS prompt or capability changes.

## Acceptance Criteria

Scenario: Mermaid preview opens interactive modal
  Test: test_mermaid_preview_action_opens_diagram_modal
  Given a bot-authored Markdown message with a fenced `mermaid` block
  And Robrix renders the block as a static timeline preview
  When the user clicks the Mermaid preview
  Then `RoomScreen` opens the diagram modal
  And the modal receives the original Mermaid source
  And the timeline preview remains static

Scenario: Diagram preview opens interactive modal
  Test: test_diagram_preview_action_opens_diagram_modal
  Given a bot-authored Markdown message with a fenced `diagram` block
  And Robrix renders the block as a static timeline preview
  When the user clicks the diagram preview
  Then `RoomScreen` opens the diagram modal
  And the modal receives the original diagram source
  And the modal uses `DiagramView` for rendering

Scenario: Timeline preview has no continuous animation
  Test: test_timeline_diagram_preview_does_not_request_next_frame
  Given a visible timeline item containing Mermaid content
  When the timeline item is populated and drawn
  Then the preview renderer does not schedule Mermaid flow-dot `NextFrame`
  And the preview renderer does not install pan/zoom hit-test behavior

Scenario: Modal Mermaid view supports pan zoom and reset
  Test: test_diagram_modal_mermaid_view_updates_pan_zoom_and_reset
  Given the diagram modal is open with Mermaid source
  When the user scrolls with the primary modifier over the modal diagram
  Then the modal Mermaid view updates zoom
  When the user drags the modal diagram
  Then the modal Mermaid view updates pan
  When the user double-clicks the modal diagram
  Then pan and zoom reset to defaults

Scenario: Modal closes cleanly
  Test: test_diagram_modal_close_clears_source_and_stops_animation
  Given the diagram modal is open with animated Mermaid content
  When the user clicks close, presses Escape, or dismisses the modal
  Then the modal closes
  And the stored diagram source is cleared
  And modal-only animation is no longer scheduled

Scenario: Empty or invalid source does not open modal content
  Test: test_diagram_modal_rejects_empty_or_invalid_source
  Given a timeline preview action with empty source
  When `RoomScreen` handles the action
  Then the modal does not show an interactive diagram
  And Robrix does not panic
  Given a Mermaid source that fails SVG rendering
  When the modal attempts to render it
  Then the modal shows a readable error state
  And Robrix does not panic

Scenario: Room scrolling remains stable after modal use
  Test: test_room_scroll_after_diagram_modal_does_not_reenable_timeline_animation
  Given a room containing bot messages with Mermaid and diagram blocks
  And the user opens and closes the diagram modal
  When the user scrolls the room timeline
  Then no timeline Mermaid preview schedules continuous animation
  And no stale preview interaction state is retained in the timeline row
