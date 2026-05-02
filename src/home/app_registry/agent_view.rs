use std::collections::HashMap;

use serde_json::Value as JsonValue;

use super::splash_host::{remove_json_pointer, replace_json_pointer, HostError};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AgentViewScope {
    Message,
    Room,
    Account,
}

impl AgentViewScope {
    pub fn parse(raw: Option<&str>) -> Option<Self> {
        match raw.unwrap_or("message") {
            "message" => Some(Self::Message),
            "room" => Some(Self::Room),
            "account" => Some(Self::Account),
            _ => None,
        }
    }

    pub fn requires_app_id(self) -> bool {
        matches!(self, Self::Room | Self::Account)
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum AgentViewScopeKey {
    Message {
        room_id: String,
        event_id: String,
    },
    Room {
        room_id: String,
        app_id: String,
    },
    Account {
        account_id: String,
        app_id: String,
    },
}

impl AgentViewScopeKey {
    pub fn from_parts(
        scope: AgentViewScope,
        room_id: impl Into<String>,
        event_id: impl Into<String>,
        app_id: Option<&str>,
    ) -> Option<Self> {
        let room_id = room_id.into();
        match scope {
            AgentViewScope::Message => Some(Self::Message {
                room_id,
                event_id: event_id.into(),
            }),
            AgentViewScope::Room => Some(Self::Room {
                room_id,
                app_id: app_id?.to_string(),
            }),
            AgentViewScope::Account => None,
        }
    }

    pub fn from_account_parts(
        account_id: impl Into<String>,
        app_id: Option<&str>,
    ) -> Option<Self> {
        Some(Self::Account {
            account_id: account_id.into(),
            app_id: app_id?.to_string(),
        })
    }
}

#[derive(Debug, Clone)]
pub struct AgentViewSession {
    pub scope_key: AgentViewScopeKey,
    pub source_event_id: String,
    pub app_type: String,
    pub version: u32,
    pub template_id: String,
    pub state: JsonValue,
    pub dirty: bool,
}

#[derive(Debug, Clone)]
pub enum AgentViewLocalStateOp {
    Replace { value: JsonValue },
    Remove,
    Append { value: JsonValue },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AgentViewReducerError {
    SessionNotFound,
    Host(HostError),
}

impl From<HostError> for AgentViewReducerError {
    fn from(error: HostError) -> Self {
        Self::Host(error)
    }
}

#[derive(Debug, Default)]
pub struct AgentViewRuntime {
    sessions: HashMap<AgentViewScopeKey, AgentViewSession>,
}

impl AgentViewRuntime {
    pub fn get_or_create(
        &mut self,
        scope_key: AgentViewScopeKey,
        source_event_id: impl Into<String>,
        app_type: impl Into<String>,
        version: u32,
        template_id: impl Into<String>,
        initial_state: &JsonValue,
    ) -> &mut AgentViewSession {
        self.sessions
            .entry(scope_key.clone())
            .or_insert_with(|| AgentViewSession {
                scope_key,
                source_event_id: source_event_id.into(),
                app_type: app_type.into(),
                version,
                template_id: template_id.into(),
                state: initial_state.clone(),
                dirty: false,
            })
    }

    pub fn session(&self, scope_key: &AgentViewScopeKey) -> Option<&AgentViewSession> {
        self.sessions.get(scope_key)
    }

    pub fn reduce_local_state(
        &mut self,
        scope_key: &AgentViewScopeKey,
        path: &str,
        op: AgentViewLocalStateOp,
    ) -> Result<bool, AgentViewReducerError> {
        let session = self
            .sessions
            .get_mut(scope_key)
            .ok_or(AgentViewReducerError::SessionNotFound)?;
        let changed = match op {
            AgentViewLocalStateOp::Replace { value } => {
                replace_json_pointer(&mut session.state, path, &value)?
            }
            AgentViewLocalStateOp::Remove => remove_json_pointer(&mut session.state, path)?,
            AgentViewLocalStateOp::Append { value } => {
                let _ = value;
                return Err(AgentViewReducerError::Host(
                    HostError::UpdateOpNotYetSupported { op: "append".into() },
                ));
            }
        };
        if changed {
            session.dirty = true;
        }
        Ok(changed)
    }
}


#[cfg(test)]
mod tests {
    use serde_json::json;

    use super::{AgentViewLocalStateOp, AgentViewReducerError};
    use super::super::splash_host::HostError;
    use super::super::{
        parse_envelope, AgentViewRuntime, AgentViewScope, AgentViewScopeKey,
    };

    #[test]
    fn parse_envelope_defaults_to_message_scope_without_app_id() {
        let event = json!({
            "org.octos.app": {
                "type": "weather",
                "version": 2,
                "initial_state": {}
            }
        });

        let parsed = parse_envelope(&event).expect("envelope should parse");

        assert_eq!(parsed.scope, AgentViewScope::Message);
        assert_eq!(parsed.app_id, None);
    }

    #[test]
    fn parse_envelope_accepts_room_scope_with_app_id() {
        let event = json!({
            "org.octos.app": {
                "type": "mission_room",
                "version": 1,
                "scope": "room",
                "app_id": "mission.main",
                "initial_state": {}
            }
        });

        let parsed = parse_envelope(&event).expect("envelope should parse");

        assert_eq!(parsed.scope, AgentViewScope::Room);
        assert_eq!(parsed.app_id.as_deref(), Some("mission.main"));
    }

    #[test]
    fn parse_envelope_trims_room_scope_app_id() {
        let event = json!({
            "org.octos.app": {
                "type": "mission_room",
                "version": 1,
                "scope": "room",
                "app_id": "  mission.main  ",
                "initial_state": {}
            }
        });

        let parsed = parse_envelope(&event).expect("envelope should parse");

        assert_eq!(parsed.app_id.as_deref(), Some("mission.main"));
    }

    #[test]
    fn parse_envelope_rejects_room_scope_without_app_id() {
        let event = json!({
            "org.octos.app": {
                "type": "mission_room",
                "version": 1,
                "scope": "room",
                "initial_state": {}
            }
        });

        assert!(parse_envelope(&event).is_none());
    }

    #[test]
    fn parse_envelope_rejects_room_scope_with_blank_app_id() {
        let event = json!({
            "org.octos.app": {
                "type": "mission_room",
                "version": 1,
                "scope": "room",
                "app_id": "   ",
                "initial_state": {}
            }
        });

        assert!(parse_envelope(&event).is_none());
    }

    #[test]
    fn parse_envelope_rejects_mission_room_without_room_scope() {
        let event = json!({
            "org.octos.app": {
                "type": "mission_room",
                "version": 1,
                "app_id": "mission.main",
                "initial_state": {}
            }
        });

        assert!(parse_envelope(&event).is_none());
    }

    #[test]
    fn parse_envelope_rejects_mission_dashboard_without_account_scope() {
        let event = json!({
            "org.octos.app": {
                "type": "mission_dashboard",
                "version": 1,
                "scope": "room",
                "app_id": "missions.global",
                "initial_state": {}
            }
        });

        assert!(parse_envelope(&event).is_none());
    }

    #[test]
    fn scope_key_uses_event_id_for_message_scope() {
        let key = AgentViewScopeKey::from_parts(
            AgentViewScope::Message,
            "!room:example.org",
            "$event1",
            None,
        )
        .expect("message scope should not require app_id");

        assert_eq!(
            key,
            AgentViewScopeKey::Message {
                room_id: "!room:example.org".into(),
                event_id: "$event1".into(),
            }
        );
    }

    #[test]
    fn scope_key_uses_app_id_for_room_scope() {
        let key = AgentViewScopeKey::from_parts(
            AgentViewScope::Room,
            "!room:example.org",
            "$event1",
            Some("mission.main"),
        )
        .expect("room scope should use app_id");

        assert_eq!(
            key,
            AgentViewScopeKey::Room {
                room_id: "!room:example.org".into(),
                app_id: "mission.main".into(),
            }
        );
    }

    #[test]
    fn scope_key_uses_account_id_and_app_id_for_account_scope() {
        let key = AgentViewScopeKey::from_account_parts(
            "@alice:example.org",
            Some("missions.global"),
        )
        .expect("account scope should use app_id");

        assert_eq!(
            key,
            AgentViewScopeKey::Account {
                account_id: "@alice:example.org".into(),
                app_id: "missions.global".into(),
            }
        );
    }

    #[test]
    fn runtime_keeps_message_scoped_sessions_isolated() {
        let mut runtime = AgentViewRuntime::default();
        let first = json!({ "count": 1 });
        let second = json!({ "count": 9 });

        let first_key = AgentViewScopeKey::from_parts(
            AgentViewScope::Message,
            "!room:example.org",
            "$event1",
            None,
        )
        .unwrap();
        let second_key = AgentViewScopeKey::from_parts(
            AgentViewScope::Message,
            "!room:example.org",
            "$event2",
            None,
        )
        .unwrap();

        runtime.get_or_create(
            first_key.clone(),
            "$event1",
            "counter",
            1,
            "counter_card",
            &first,
        );
        runtime.get_or_create(
            second_key.clone(),
            "$event2",
            "counter",
            1,
            "counter_card",
            &second,
        );

        assert_eq!(runtime.session(&first_key).unwrap().state["count"], 1);
        assert_eq!(runtime.session(&second_key).unwrap().state["count"], 9);
    }

    #[test]
    fn runtime_reuses_room_scoped_session_by_app_id() {
        let mut runtime = AgentViewRuntime::default();
        let initial = json!({ "count": 1 });
        let ignored_later_initial = json!({ "count": 99 });
        let key = AgentViewScopeKey::from_parts(
            AgentViewScope::Room,
            "!room:example.org",
            "$event1",
            Some("counter.main"),
        )
        .unwrap();

        runtime.get_or_create(
            key.clone(),
            "$event1",
            "counter",
            1,
            "counter_card",
            &initial,
        );
        runtime.get_or_create(
            key.clone(),
            "$event2",
            "counter",
            1,
            "counter_card",
            &ignored_later_initial,
        );

        let session = runtime.session(&key).unwrap();
        assert_eq!(session.source_event_id, "$event1");
        assert_eq!(session.state["count"], 1);
    }

    #[test]
    fn runtime_reuses_account_scoped_session_by_account_and_app_id() {
        let mut runtime = AgentViewRuntime::default();
        let initial = json!({ "open_missions": 2 });
        let ignored_later_initial = json!({ "open_missions": 9 });
        let key = AgentViewScopeKey::from_account_parts(
            "@alice:example.org",
            Some("missions.global"),
        )
        .unwrap();

        runtime.get_or_create(
            key.clone(),
            "$event1",
            "mission_dashboard",
            1,
            "account_overview",
            &initial,
        );
        runtime.get_or_create(
            key.clone(),
            "$event2",
            "mission_dashboard",
            1,
            "account_overview",
            &ignored_later_initial,
        );

        let session = runtime.session(&key).unwrap();
        assert_eq!(session.source_event_id, "$event1");
        assert_eq!(session.state["open_missions"], 2);
    }

    #[test]
    fn reducer_replace_updates_state_and_marks_dirty() {
        let mut runtime = AgentViewRuntime::default();
        let initial = json!({ "filter": { "lane": "all" } });
        let key = AgentViewScopeKey::from_parts(
            AgentViewScope::Room,
            "!room:example.org",
            "$event1",
            Some("mission.main"),
        )
        .unwrap();

        runtime.get_or_create(
            key.clone(),
            "$event1",
            "mission_room",
            1,
            "mission_control",
            &initial,
        );
        let changed = runtime
            .reduce_local_state(
                &key,
                "/filter/lane",
                AgentViewLocalStateOp::Replace { value: json!("blocked") },
            )
            .expect("replace should succeed");

        let session = runtime.session(&key).unwrap();
        assert!(changed);
        assert!(session.dirty);
        assert_eq!(session.state["filter"]["lane"], "blocked");
    }

    #[test]
    fn reducer_remove_absent_key_is_noop_without_dirty() {
        let mut runtime = AgentViewRuntime::default();
        let initial = json!({ "filter": { "lane": "all" } });
        let key = AgentViewScopeKey::from_parts(
            AgentViewScope::Room,
            "!room:example.org",
            "$event1",
            Some("mission.main"),
        )
        .unwrap();

        runtime.get_or_create(
            key.clone(),
            "$event1",
            "mission_room",
            1,
            "mission_control",
            &initial,
        );
        let changed = runtime
            .reduce_local_state(&key, "/filter/sort", AgentViewLocalStateOp::Remove)
            .expect("remove absent key should be a no-op");

        let session = runtime.session(&key).unwrap();
        assert!(!changed);
        assert!(!session.dirty);
        assert_eq!(session.state, initial);
    }

    #[test]
    fn reducer_append_is_rejected_without_mutating() {
        let mut runtime = AgentViewRuntime::default();
        let initial = json!({ "items": [1] });
        let key = AgentViewScopeKey::from_parts(
            AgentViewScope::Message,
            "!room:example.org",
            "$event1",
            None,
        )
        .unwrap();

        runtime.get_or_create(
            key.clone(),
            "$event1",
            "counter",
            1,
            "counter_card",
            &initial,
        );
        let err = runtime
            .reduce_local_state(
                &key,
                "/items",
                AgentViewLocalStateOp::Append { value: json!(2) },
            )
            .expect_err("append is outside reducer v1");

        assert!(matches!(
            err,
            AgentViewReducerError::Host(HostError::UpdateOpNotYetSupported { .. })
        ));
        let session = runtime.session(&key).unwrap();
        assert!(!session.dirty);
        assert_eq!(session.state, initial);
    }

    #[test]
    fn reducer_unknown_session_is_rejected() {
        let mut runtime = AgentViewRuntime::default();
        let key = AgentViewScopeKey::from_parts(
            AgentViewScope::Message,
            "!room:example.org",
            "$event1",
            None,
        )
        .unwrap();

        let err = runtime
            .reduce_local_state(
                &key,
                "/count",
                AgentViewLocalStateOp::Replace { value: json!(2) },
            )
            .expect_err("missing session should not create state implicitly");

        assert_eq!(err, AgentViewReducerError::SessionNotFound);
    }
}
