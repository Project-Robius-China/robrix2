//! Makes a keyboard-focused `Button` activate on Space or Enter.
//!
//! Makepad's `Button` registers itself as a nav stop and plays its `focus.on`
//! animation from `Hit::KeyFocus`, so Tab both reaches it and lights its focus
//! ring — but `Button::handle_event` has no `Hit::KeyDown` branch, so the
//! button then ignores every key that follows. A ring that promises "this is
//! the control you're about to operate" and then does nothing is worse than no
//! ring at all: it strands anyone navigating without a pointer.
//!
//! ## Why this isn't a widget
//!
//! The obvious fix is to wrap `Button` in a Robrix widget that adds the missing
//! branch. That doesn't work here: `WidgetRef::button()` downcasts to the
//! concrete `Button` type, so a wrapper would turn every one of the ~130
//! `self.button(cx, ids!(…)).clicked(actions)` call sites into a silent no-op.
//!
//! So instead of changing what the buttons are, we deliver what the key press
//! should have produced. `ButtonRef::clicked()` matches purely on widget uid
//! (`actions.find_widget_action(self.widget_uid())`), so an externally-emitted
//! `ButtonAction::Clicked` is indistinguishable from one the button raised
//! itself, and every existing call site keeps working untouched.

use makepad_widgets::*;

/// Emits `ButtonAction::Clicked` for the focused `Button`, if Space or Enter
/// was just pressed and a `Button` currently holds keyboard focus.
///
/// Call once per event from the app root, before the UI tree handles the
/// event. Does nothing for any other key, or when focus is anywhere that is
/// not an enabled, visible button — typing a space into a text input walks the
/// tree, finds no button, and falls through untouched.
pub fn activate_focused_button(cx: &mut Cx, ui: &WidgetRef, event: &Event) {
    let Event::KeyDown(key) = event else { return };
    if !matches!(
        key.key_code,
        KeyCode::Space | KeyCode::ReturnKey | KeyCode::NumpadEnter,
    ) {
        return;
    }
    // Holding the key down should not fire repeatedly; a click doesn't.
    if key.is_repeat { return }

    let Some(uid) = find_focused_button(cx, ui) else { return };
    cx.widget_action(uid, ButtonAction::Clicked(key.modifiers));
}

/// Walks down from `ui` looking for an enabled, visible `Button` that holds
/// keyboard focus.
///
/// Only runs on Space/Enter, so the cost is bounded by how fast a person can
/// press a key rather than by the frame rate.
fn find_focused_button(cx: &Cx, ui: &WidgetRef) -> Option<WidgetUid> {
    let mut stack = vec![ui.clone()];
    while let Some(node) = stack.pop() {
        if node.skip_widget_tree_search() { continue }

        // `enabled` is the flag `Button::handle_event` itself gates clicks on,
        // and it is distinct from the animator's `disabled` state. Checking it
        // is what keeps a disabled button — which still registers a nav stop,
        // and so can still be Tabbed to — from being activated by a key when
        // it cannot be activated by a click.
        if let Some(button) = node.borrow::<Button>() {
            if button.enabled()
                && button.visible()
                && cx.has_key_focus(button.area())
            {
                return Some(button.widget_uid());
            }
            // A `Button` draws its own label and icon; nothing below it is a
            // separate focusable widget.
            continue;
        }

        node.children(&mut |_id, child| stack.push(child));
    }
    None
}
