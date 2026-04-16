# Left Room Tabs Stay Open And Reopen On Restart

## Summary

When the user leaves a room, the corresponding desktop tab can remain open. On the next app restart, the stale tab is restored from persisted dock state and may show an infinite "Waiting for this room to be loaded from the homeserver" state.

This is especially confusing for DMs with duplicate display names, because a stale left room and a current joined room can appear as two tabs with the same label.

## Symptoms

- Leaving a room does not automatically close its open tab.
- Restarting the app restores the left room tab again.
- The stale tab may remain stuck in the room restore placeholder.
- Duplicate tabs can appear for the same display name if the user has both:
  - a current joined DM, and
  - an older DM with the same display name that was already left

## Root Cause

Two bugs combine:

1. `LeaveRoomResultAction::Left` was not wired into desktop tab cleanup.
   - The room could disappear from the rooms list eventually, but its open tab remained.

2. The persisted `SavedDockState` still retained the left room.
   - `MainDesktopUI::load_dock_state_from()` trusted `room_order` / `selected_room`
     from persisted state and recreated tabs without any stale-room pruning.

## Code References

- Leave result action:
  - `src/home/invite_screen.rs`
  - `src/join_leave_room_modal.rs`
  - `src/sliding_sync.rs`
- Dock persistence and restore:
  - `src/home/main_desktop_ui.rs`
  - `src/app.rs`

## Fixed Behavior

Leaving a room now:

- hides the room from the rooms list immediately
- removes the room from persisted dock state
- closes all open tabs for that room, including thread tabs

This fixes future occurrences of the bug.

## Remaining Note

Already-persisted stale tabs from older app runs may still exist until the user closes them once or the persisted state is otherwise cleaned up.
