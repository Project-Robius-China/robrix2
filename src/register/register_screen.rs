//! RegisterScreen widget: homeserver picker + capability display.
//!
//! The full wizard body is added in later tasks; Task 1 only creates a stub
//! so the module compiles.

use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    pub RegisterScreen := {{RegisterScreen}} View {
        width: Fill,
        height: Fill,
        show_bg: true,
        draw_bg: { color: #x1F2124 }

        // TODO: Task 5 fills in this body.
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct RegisterScreen {
    #[deref] view: View,
}

impl Widget for RegisterScreen {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
    }
    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}
