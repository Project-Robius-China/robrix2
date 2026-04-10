spec: task
name: "Room Info Modal Cancel Flow Consistency"
inherits: project
tags: [bugfix, room-info, modal, ui, navigation]
estimate: 1d
---

## Intent

Fix the Room Info panel interaction bug where opening `Report Room` or `Leave Room` shows a modal, but clicking `Cancel` first collapses the Room Info pane and only a second cancel closes the modal. Also fix modal lifetime so these dialogs do not remain visible after switching to a different room.
The issue is in the Room Screen overlay interaction between `RoomInfoSlidingPane`, `ReportRoomModal`, and the leave-room `NegativeConfirmationModal`. Current behavior indicates modal cancel/dismiss actions are not fully isolated from underlying pane interactions, causing UI state desynchronization (pane closes unexpectedly, modal survives room switch).

## Constraints

- Keep existing action entry points from Room Info (`ReportRoom`, `LeaveRoom`)
- Keep Matrix async request path unchanged:
  - `submit_async_request(MatrixRequest::ReportRoom { ... })`
  - `submit_async_request(MatrixRequest::LeaveRoom { ... })`
- Do not change unrelated Room Info actions (`Invite`, `People`, profile navigation)
- Do not redesign room navigation architecture

## Decisions

- Modal cancel must be single-step: one cancel action closes the active modal immediately
- Canceling or dismissing modal must not close the Room Info pane
- Modal visibility is scoped to the currently displayed room:
  - switching room closes any open report/leave modal
  - modal must not remain visible in the newly selected room
- Room switching while modal is open must not submit report/leave requests
- Existing submit semantics remain unchanged:
  - report submit sends `ReportRoom` and closes modal
  - leave confirm sends `LeaveRoom` and closes modal

## Boundaries

### Allowed Changes
- `src/home/room_screen.rs`
- `src/shared/confirmation_modal.rs` (only if needed for dismiss/cancel event handling)

### Forbidden
- Do not add new dependencies
- Do not change Matrix request payload format or backend behavior
- Do not change unrelated Room Info layout/content
- Do not run `cargo fmt` or reformat unrelated code

## Acceptance Criteria

Scenario: Cancel report modal in one click without collapsing Room Info pane
  Test: manual_test_room_info_report_cancel_single_step
  Given the user opens Room Info for a room
  And the user opens the `Report Room` modal
  When the user clicks `Cancel`
  Then the report modal closes immediately
  And the Room Info pane remains open

Scenario: Cancel leave modal in one click without collapsing Room Info pane
  Test: manual_test_room_info_leave_cancel_single_step
  Given the user opens Room Info for a room
  And the user opens the `Leave Room` confirm modal
  When the user clicks `Cancel`
  Then the leave modal closes immediately
  And the Room Info pane remains open

Scenario: Dismiss modal via backdrop or dismiss action does not close Room Info pane
  Test: manual_test_room_info_modal_dismiss_keeps_info_open
  Level: manual
  Targets: room_screen_modal_lifecycle
  Given a report or leave modal is open from Room Info
  When the modal is dismissed via non-submit close path
  Then the modal closes
  And the Room Info pane stays open

Scenario: Switching room closes modal and prevents cross-room modal leakage
  Test: manual_test_room_info_modal_closed_on_room_switch
  Given a report or leave modal is open in room A
  When the user switches to room B
  Then no report/leave modal is visible in room B
  And room B interaction is not blocked by stale modal overlay

Scenario: Returning to original room does not resurrect stale modal
  Test: manual_test_room_info_modal_not_restored_after_room_switch
  Given a modal was open in room A and the user switched to room B
  When the user switches back to room A
  Then the previous report/leave modal is not auto-reopened

Scenario: Submit behavior remains unchanged
  Test: manual_test_room_info_modal_submit_semantics_unchanged
  Given the user opens report/leave modal from Room Info
  When the user confirms the action
  Then report confirmation calls `submit_async_request(MatrixRequest::ReportRoom { ... })` exactly once
  And leave confirmation calls `submit_async_request(MatrixRequest::LeaveRoom { ... })` exactly once
  And the modal closes

Scenario: Room Info pane close behavior still works when no modal is active
  Test: manual_test_room_info_pane_close_regression_guard
  Given the Room Info pane is open and no modal is active
  When the user performs the pane close action
  Then the Room Info pane closes as before

Scenario: Report with empty reason does not submit and shows validation
  Test: manual_test_room_info_report_empty_reason_validation
  Given the user opens the `Report Room` modal
  And the reason input is empty
  When the user clicks the report/submit button
  Then no `submit_async_request(MatrixRequest::ReportRoom { ... })` request is sent
  And a validation error is shown in the modal
  And the modal remains open

## Out of Scope

- Changing copy/text/visual design of report or leave dialogs
- Adding telemetry for cancel/dismiss events
- Refactoring modal framework shared by unrelated screens
