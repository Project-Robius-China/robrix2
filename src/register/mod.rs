//! Account registration feature.
//!
//! Covers the dual-mode register flow (OIDC for MAS-delegated servers,
//! UIAA wizard for legacy servers). See `specs/task-register-flow.spec.md`.

use makepad_widgets::ScriptVm;

pub mod register_screen;
pub mod register_status_modal;
pub mod validation;

pub fn script_mod(vm: &mut ScriptVm) {
    register_status_modal::script_mod(vm);
    register_screen::script_mod(vm);
}
