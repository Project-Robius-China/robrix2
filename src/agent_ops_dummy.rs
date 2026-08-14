//! An inert stand-in for the Agent Operations Panel.
//!
//! `HomeScreen`'s DSL declares an `agent_ops_page` unconditionally, so the
//! `AgentOpsPanel` name has to resolve in builds without the `agent_chat`
//! feature too. This provides that name and nothing else: the page exists but
//! is unreachable, because no navigation entry emits `GoToAgentOps` without the
//! feature, and `update_active_page_from_selection` normalizes any stale
//! `AgentOps` selection back to Home.
//!
//! Same shape as `tsp_dummy` — see that module for the original of this pattern.

use makepad_widgets::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.AgentOpsPanel = View {
        visible: false,
        width: Fill, height: Fill
    }
}

#[cfg(test)]
mod tests {
    #[test]
    fn test_panel_absent_without_agent_chat_feature() {
        assert!(!cfg!(feature = "agent_chat"));

        let settings = include_str!("settings/app_settings.rs");
        let emit_site = settings
            .find("NavigationBarAction::GoToAgentOps")
            .expect("feature build should retain the integration-status entry");
        let preceding = &settings[..emit_site];
        let guard = preceding
            .rfind("#[cfg(feature = \"agent_chat\")]")
            .expect("entry must be guarded by the agent_chat feature");
        assert!(
            !preceding[guard..].contains("#[cfg(not(feature = \"agent_chat\"))]"),
            "the entry must remain inside the feature-gated block",
        );
    }
}
