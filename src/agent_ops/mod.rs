//! Agent Operations Panel client-contract gate.
//!
//! The operational UI is intentionally unavailable until agent-chat releases
//! canonical contract artifacts and a separate client runtime is implemented.
//! The test-only `model` module validates non-canonical design fixtures;
//! [`state`] makes the production gate explicit, while `panel` renders the
//! resulting status and performs no I/O.

#[cfg(test)]
mod model;
pub mod panel;
pub mod state;
mod sha256;

use makepad_widgets::Cx;

pub fn script_mod(vm: &mut makepad_widgets::ScriptVm) {
    panel::script_mod(vm);
}

/// Placeholder so `Cx` stays referenced if the widget layer is compiled out.
#[allow(dead_code)]
fn _assert_cx_in_scope(_cx: &mut Cx) {}
