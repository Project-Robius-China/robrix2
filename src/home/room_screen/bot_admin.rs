//! Bot identity detection and app-service administration: which senders
//! count as bots, the cached per-room bot context, and the botfather
//! slash-command builders.

use super::*;

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.AppServicePanel = #(AppServicePanel::register_widget(vm)) {
        width: Fill
        height: Fit
        margin: Inset{left: 14, right: 54, top: 10, bottom: 16}
        flow: Down
        align: Align{x: 0.0, y: 0.0}
        spacing: 8

        sender_row := View {
            width: Fit
            height: Fit
            flow: Right
            spacing: 6

            sender_name := Label {
                width: Fit
                height: Fit
                draw_text +: {
                    text_style: USERNAME_TEXT_STYLE { font_size: 10.8 }
                    color: (COLOR_ACTIVE_PRIMARY)
                }
                text: ""
            }

            sender_tag := Label {
                width: Fit
                height: Fit
                draw_text +: {
                    text_style: REGULAR_TEXT { font_size: 9.5 }
                    color: #8A8A8A
                }
                text: ""
            }
        }

        bubble := RoundedView {
            width: 408
            height: Fit
            flow: Down
            spacing: 8
            padding: Inset{top: 14, right: 14, bottom: 12, left: 14}

            show_bg: true
            draw_bg +: {
                color: (COLOR_PRIMARY)
                border_radius: 0.0
                border_size: 1.0
                border_color: (COLOR_SECONDARY_DARKER)
            }

            header := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{y: 0.5}

                title := Label {
                    width: Fit
                    height: Fit
                    draw_text +: {
                        text_style: USERNAME_TEXT_STYLE { font_size: 11.2 }
                        color: #1F1F1F
                    }
                    text: ""
                }

                spacer := View {
                    width: Fill
                    height: Fit
                }

                dismiss_button := RobrixNeutralIconButton {
                    width: 28
                    height: 24
                    align: Align{x: 0.5, y: 0.5}
                    spacing: 0
                    padding: 0
                    draw_icon.svg: (ICON_CLOSE)
                    icon_walk: Walk{width: 12, height: 12}
                    text: ""
                }
            }

            subtitle := Label {
                width: Fill
                height: Fit
                draw_text +: {
                    text_style: REGULAR_TEXT { font_size: 10.5 }
                    color: (COLOR_TEXT)
                }
                text: ""
            }

            footer := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{x: 1.0, y: 0.5}

                timestamp := Label {
                    width: Fit
                    height: Fit
                    draw_text +: {
                        text_style: REGULAR_TEXT { font_size: 8.8 }
                        color: #9A9A9A
                    }
                    text: ""
                }
            }
        }

        keyboard := View {
            width: Fit
            height: Fit
            flow: Down
            spacing: 8

            first_row := View {
                width: Fit
                height: Fit
                flow: Right
                spacing: 8

                create_button := RobrixPositiveIconButton {
                    width: 156
                    height: 46
                    padding: 10
                    draw_icon.svg: (ICON_CHECKMARK)
                    icon_walk: Walk{width: 16, height: 16, margin: Inset{left: -2, right: -1}}
                    text: ""
                }

                list_button := RobrixNeutralIconButton {
                    width: 156
                    height: 46
                    padding: 10
                    draw_icon.svg: (ICON_SEARCH)
                    icon_walk: Walk{width: 14, height: 14, margin: Inset{left: -2, right: -1}}
                    text: ""
                }
            }

            second_row := View {
                width: Fit
                height: Fit
                flow: Right
                spacing: 8

                delete_button := RobrixNegativeIconButton {
                    width: 156
                    height: 46
                    padding: 10
                    draw_icon.svg: (ICON_CLOSE)
                    icon_walk: Walk{width: 14, height: 14, margin: Inset{left: -2, right: -1}}
                    text: ""
                }

                help_button := RobrixNeutralIconButton {
                    width: 156
                    height: 46
                    padding: 10
                    draw_icon.svg: (ICON_INFO)
                    icon_walk: Walk{width: 14, height: 14, margin: Inset{left: -2, right: -1}}
                    text: ""
                }
            }

            third_row := View {
                width: Fit
                height: Fit
                flow: Right
                spacing: 8

                view_bound_button := RobrixNeutralIconButton {
                    width: 156
                    height: 46
                    padding: 10
                    draw_icon.svg: (ICON_SEARCH)
                    icon_walk: Walk{width: 14, height: 14, margin: Inset{left: -2, right: -1}}
                    text: "View Bound Bots"
                }

                unbind_button := RobrixNeutralIconButton {
                    width: 156
                    height: 46
                    padding: 10
                    draw_icon.svg: (ICON_CLOSE)
                    icon_walk: Walk{width: 14, height: 14, margin: Inset{left: -2, right: -1}}
                    text: ""
                }
            }
        }
    }
}

pub(super) fn escape_slash_command_arg(value: &str) -> String {
    value.trim().replace('\\', "\\\\").replace('"', "\\\"")
}

pub(super) fn format_create_bot_command(
    username: &str,
    display_name: &str,
    system_prompt: Option<&str>,
) -> String {
    let mut command = format!("/createbot {} {}", username.trim(), display_name.trim());
    if let Some(system_prompt) = system_prompt
        .map(str::trim)
        .filter(|value| !value.is_empty())
    {
        command.push_str(" --prompt \"");
        command.push_str(&escape_slash_command_arg(system_prompt));
        command.push('"');
    }
    command
}

pub(super) fn format_delete_bot_command(matrix_user_id: &UserId) -> String {
    format!("/deletebot {matrix_user_id}")
}

pub(super) fn resolve_delete_bot_user_id(
    user_id_or_localpart: &str,
    current_user_id: Option<&UserId>,
    app_language: AppLanguage,
) -> Result<OwnedUserId, String> {
    let raw = user_id_or_localpart.trim();
    if raw.is_empty() {
        return Err(tr_key(app_language, "room_screen.bot.delete.error.empty_user_id").into());
    }

    if raw.starts_with('@') || raw.contains(':') {
        let full_user_id = if raw.starts_with('@') {
            raw.to_string()
        } else {
            format!("@{raw}")
        };
        return UserId::parse(&full_user_id)
            .map(|user_id| user_id.to_owned())
            .map_err(|_| tr_fmt(app_language, "room_screen.bot.delete.error.invalid_user_id", &[
                ("full_user_id", full_user_id.as_str()),
            ]));
    }

    let Some(current_user_id) = current_user_id else {
        return Err(
            tr_key(app_language, "room_screen.bot.delete.error.current_user_unavailable").into(),
        );
    };

    let full_user_id = format!("@{raw}:{}", current_user_id.server_name());
    UserId::parse(&full_user_id)
        .map(|user_id| user_id.to_owned())
        .map_err(|_| tr_fmt(app_language, "room_screen.bot.delete.error.invalid_user_id", &[
            ("full_user_id", full_user_id.as_str()),
        ]))
}

pub(super) fn detected_bot_binding_for_members(
    app_state: &AppState,
    room_id: &OwnedRoomId,
    members: &[RoomMember],
) -> Option<OwnedUserId> {
    if app_state.bot_settings.is_room_bound(room_id) {
        return None;
    }

    let own_user_id = current_user_id();
    let is_non_self = |room_member: &&RoomMember| {
        own_user_id
            .as_deref()
            .is_none_or(|own_user_id| room_member.user_id() != own_user_id)
    };

    if let Ok(configured_bot_user_id) = app_state
        .bot_settings
        .resolved_bot_user_id(current_user_id().as_deref())
    {
        if members
            .iter()
            .filter(is_non_self)
            .any(|room_member| room_member.user_id().as_str() == configured_bot_user_id.as_str())
        {
            return Some(configured_bot_user_id);
        }
    }

    let known_bot_user_ids = timeline_known_bot_user_ids(app_state);
    if let Some(bot_member) = members
        .iter()
        .filter(is_non_self)
        .filter(|room_member|
            known_bot_user_ids
                .iter()
                .any(|known_bot_user_id| known_bot_user_id.as_str() == room_member.user_id().as_str())
        )
        .min_by(|lhs, rhs| lhs.user_id().as_str().cmp(rhs.user_id().as_str()))
    {
        return Some(bot_member.user_id().to_owned());
    }

    let mut non_self_members = members.iter().filter(is_non_self);
    if let Some(dm_counterparty) = non_self_members.next()
        && non_self_members.next().is_none()
    {
        let localpart = dm_counterparty.user_id().localpart().to_ascii_lowercase();
        let localpart_likely_bot = localpart == "bot"
            || localpart == "botfather"
            || localpart.starts_with("bot_")
            || localpart.starts_with("bot-")
            || localpart.starts_with("bot.");
        let display_name_likely_bot = dm_counterparty
            .display_name()
            .is_some_and(|display_name| display_name.to_ascii_lowercase().contains("bot"));
        if localpart_likely_bot || display_name_likely_bot {
            return Some(dm_counterparty.user_id().to_owned());
        }
    }

    members
        .iter()
        .filter(is_non_self)
        .filter(|room_member| room_member.user_id().localpart().eq_ignore_ascii_case("botfather"))
        .min_by(|lhs, rhs| lhs.user_id().as_str().cmp(rhs.user_id().as_str()))
        .map(|room_member| room_member.user_id().to_owned())
}

pub(super) fn is_likely_bot_user_id(
    user_id: &UserId,
    resolved_parent_bot_user_id: Option<&UserId>,
) -> bool {
    if resolved_parent_bot_user_id.is_some_and(|parent| parent == user_id) {
        return true;
    }

    let localpart = user_id.localpart().to_ascii_lowercase();
    localpart == "bot"
        || localpart == "botfather"
        || localpart.starts_with("bot_")
        || localpart.starts_with("bot-")
        || localpart.starts_with("bot.")
        || localpart.ends_with("_bot")
        || (localpart.ends_with("bot") && localpart.len() > 3)
        || is_agent_chat_puppet_localpart(&localpart)
}

/// Agent-chat puppets an agent per Matrix account named
/// `<MATRIX_AGENT_PREFIX><team>_<role>` — the prefix defaults to `ac_` and the
/// roles come from the shared issue-workflow skill. None of those names contain
/// "bot", so without this they render as human messages and miss the bot card
/// (and its type badge / fold affordance) entirely.
pub(super) fn is_agent_chat_puppet_localpart(localpart: &str) -> bool {
    const AGENT_CHAT_PREFIX: &str = "ac_";
    const WORKFLOW_ROLE_SUFFIXES: &[&str] = &[
        "_coordinator",
        "_implementer",
        "_reviewer",
        "_final_reviewer",
    ];

    localpart.starts_with(AGENT_CHAT_PREFIX)
        || WORKFLOW_ROLE_SUFFIXES
            .iter()
            .any(|suffix| localpart.ends_with(suffix))
}

pub(crate) fn is_known_or_likely_bot(
    user_id: &UserId,
    resolved_parent_bot_user_id: Option<&UserId>,
    known_bot_user_ids: &[OwnedUserId],
) -> bool {
    known_bot_user_ids
        .iter()
        .any(|known_bot_user_id| known_bot_user_id.as_str() == user_id.as_str())
        || resolved_parent_bot_user_id.is_some_and(|parent| parent == user_id)
        || is_likely_bot_user_id(user_id, resolved_parent_bot_user_id)
}

pub(super) fn is_timeline_sender_bot(
    user_id: &UserId,
    resolved_parent_bot_user_id: Option<&UserId>,
    room_bot_user_ids: &[OwnedUserId],
    known_bot_user_ids: &[OwnedUserId],
) -> bool {
    room_bot_user_ids
        .iter()
        .any(|room_bot_user_id| room_bot_user_id.as_str() == user_id.as_str())
        || is_known_or_likely_bot(
            user_id,
            resolved_parent_bot_user_id,
            known_bot_user_ids,
        )
}

pub(super) fn collect_room_bot_user_ids(
    room_members: &[RoomMember],
    resolved_parent_bot_user_id: Option<&UserId>,
    known_bot_user_ids: &[OwnedUserId],
    persisted_room_bot_user_ids: &[OwnedUserId],
) -> Vec<OwnedUserId> {
    let own_user_id = current_user_id();
    let mut room_bot_user_ids = Vec::<OwnedUserId>::new();

    for persisted_room_bot_user_id in persisted_room_bot_user_ids {
        if room_bot_user_ids
            .iter()
            .all(|existing_user_id| existing_user_id.as_str() != persisted_room_bot_user_id.as_str())
        {
            room_bot_user_ids.push(persisted_room_bot_user_id.clone());
        }
    }

    for room_member in room_members.iter().filter(|room_member|
        own_user_id
            .as_deref()
            .is_none_or(|own_user_id| room_member.user_id() != own_user_id)
    ) {
        if is_known_or_likely_bot(
            room_member.user_id(),
            resolved_parent_bot_user_id,
            known_bot_user_ids,
        ) || is_likely_bot_member(room_member, resolved_parent_bot_user_id)
        {
            let user_id = room_member.user_id().to_owned();
            if room_bot_user_ids
                .iter()
                .all(|existing_user_id| existing_user_id.as_str() != user_id.as_str())
            {
                room_bot_user_ids.push(user_id);
            }
        }
    }

    room_bot_user_ids.sort_by(|lhs, rhs| lhs.as_str().cmp(rhs.as_str()));
    room_bot_user_ids
}

/// Returns the set of MXIDs the timeline should treat as bots: the union of the
/// app-service known-bot list (only when app-service is enabled) and every agent
/// registered in the global [`AgentRegistry`] (always, independent of app-service).
pub(super) fn timeline_known_bot_user_ids(app_state: &AppState) -> Vec<OwnedUserId> {
    let mut bot_user_ids = if app_state.bot_settings.enabled {
        app_state.bot_settings.known_bot_user_ids()
    } else {
        Vec::new()
    };
    for agent_user_id in app_state.agent_registry.agent_user_ids() {
        if bot_user_ids
            .iter()
            .all(|existing| existing.as_str() != agent_user_id.as_str())
        {
            bot_user_ids.push(agent_user_id);
        }
    }
    bot_user_ids
}

#[derive(Clone, Default)]
pub(super) struct TimelineBotContext {
    pub(super) app_service_enabled: bool,
    pub(super) app_service_room_bound: bool,
    pub(super) has_persisted_management_binding: bool,
    pub(super) bound_bot_user_id: Option<OwnedUserId>,
    pub(super) resolved_parent_bot_user_id: Option<OwnedUserId>,
    pub(super) persisted_bound_bot_user_ids: Vec<OwnedUserId>,
    pub(super) room_bot_user_ids: Vec<OwnedUserId>,
    pub(super) known_bot_user_ids: Vec<OwnedUserId>,
}

pub(super) struct CachedTimelineBotContext {
    pub(super) room_id: OwnedRoomId,
    pub(super) room_members: Option<Arc<Vec<RoomMember>>>,
    pub(super) app_service_enabled: bool,
    pub(super) room_is_bound: bool,
    pub(super) persisted_bound_bot_user_id: Option<OwnedUserId>,
    pub(super) persisted_bound_bot_user_ids: Vec<OwnedUserId>,
    pub(super) resolved_parent_bot_user_id: Option<OwnedUserId>,
    pub(super) known_bot_user_ids: Vec<OwnedUserId>,
    pub(super) value: TimelineBotContext,
}
impl CachedTimelineBotContext {
    fn has_same_members(&self, room_members: Option<&Arc<Vec<RoomMember>>>) -> bool {
        match (self.room_members.as_ref(), room_members) {
            (Some(cached), Some(current)) => Arc::ptr_eq(cached, current),
            (None, None) => true,
            _ => false,
        }
    }

    pub(super) fn matches(
        &self,
        room_id: &OwnedRoomId,
        room_members: Option<&Arc<Vec<RoomMember>>>,
        app_service_enabled: bool,
        room_is_bound: bool,
        persisted_bound_bot_user_id: Option<&OwnedUserId>,
        persisted_bound_bot_user_ids: &[OwnedUserId],
        resolved_parent_bot_user_id: Option<&OwnedUserId>,
        known_bot_user_ids: &[OwnedUserId],
    ) -> bool {
        self.room_id == *room_id
            && self.has_same_members(room_members)
            && self.app_service_enabled == app_service_enabled
            && self.room_is_bound == room_is_bound
            && self.persisted_bound_bot_user_id.as_ref() == persisted_bound_bot_user_id
            && self.persisted_bound_bot_user_ids.as_slice() == persisted_bound_bot_user_ids
            && self.resolved_parent_bot_user_id.as_ref() == resolved_parent_bot_user_id
            && self.known_bot_user_ids.as_slice() == known_bot_user_ids
    }
}

pub(super) fn compute_timeline_bot_context(
    app_state: Option<&AppState>,
    room_id: &OwnedRoomId,
    room_members: Option<&Arc<Vec<RoomMember>>>,
) -> (Option<OwnedUserId>, Vec<OwnedUserId>, Vec<OwnedUserId>) {
    app_state
        .map(|app_state| {
            let app_service_enabled = app_state.bot_settings.enabled;
            let persisted_room_bot_user_ids = if app_service_enabled {
                app_state.bot_settings.bound_bot_user_ids(room_id)
            } else {
                Vec::new()
            };
            let resolved_parent_bot_user_id = if app_service_enabled {
                app_state
                    .bot_settings
                    .resolved_bot_user_id(current_user_id().as_deref())
                    .ok()
            } else {
                None
            };
            // Union of the (app-service-gated) known-bot list and the global
            // AgentRegistry, so registry agents are recognized even when the
            // app-service integration is disabled.
            let known_bot_user_ids = timeline_known_bot_user_ids(app_state);
            let room_bot_user_ids = room_members
                .map(|members|
                    collect_room_bot_user_ids(
                        members.as_ref(),
                        resolved_parent_bot_user_id.as_deref(),
                        &known_bot_user_ids,
                        &persisted_room_bot_user_ids,
                    )
                )
                .unwrap_or(persisted_room_bot_user_ids);
            (
                resolved_parent_bot_user_id,
                room_bot_user_ids,
                known_bot_user_ids,
            )
        })
        .unwrap_or((None, Vec::new(), Vec::new()))
}

pub(super) fn is_likely_bot_member(
    room_member: &RoomMember,
    resolved_parent_bot_user_id: Option<&UserId>,
) -> bool {
    if is_likely_bot_user_id(room_member.user_id(), resolved_parent_bot_user_id) {
        return true;
    }

    room_member.display_name().is_some_and(|display_name| {
        let display_name = display_name.trim().to_ascii_lowercase();
        display_name == "bot"
            || display_name == "botfather"
            || display_name.starts_with("bot ")
            || display_name.ends_with(" bot")
            || display_name.contains(" bot ")
    })
}

pub(super) fn extract_bot_user_ids_from_listbots_reply(
    text: &str,
    default_server_name: Option<&OwnedServerName>,
) -> Vec<OwnedUserId> {
    let mut bot_user_ids = Vec::<OwnedUserId>::new();

    let mut push_bot = |bot_user_id: OwnedUserId| {
        if !bot_user_ids
            .iter()
            .any(|existing_bot_user_id| existing_bot_user_id.as_str() == bot_user_id.as_str())
        {
            bot_user_ids.push(bot_user_id);
        }
    };

    for token in text.split(|ch: char|
        !(ch.is_ascii_alphanumeric() || matches!(ch, '@' | ':' | '_' | '-' | '.'))
    ) {
        let token = token.trim();
        if token.is_empty() {
            continue;
        }

        if token.starts_with('@') && token.contains(':') {
            if let Ok(bot_user_id) = UserId::parse(token).map(|user_id| user_id.to_owned()) {
                if BotSettingsState::is_valid_known_bot_user_id(bot_user_id.as_ref()) {
                    push_bot(bot_user_id);
                }
            }
            continue;
        }

        if token.contains(':') && !token.starts_with('@') {
            let full_user_id = format!("@{token}");
            if let Ok(bot_user_id) = UserId::parse(&full_user_id).map(|user_id| user_id.to_owned()) {
                if BotSettingsState::is_valid_known_bot_user_id(bot_user_id.as_ref()) {
                    push_bot(bot_user_id);
                }
            }
            continue;
        }

        let localpart_lc = token.to_ascii_lowercase();
        let is_likely_bot_localpart = (
                localpart_lc == "bot"
                || localpart_lc.starts_with("bot_")
                || localpart_lc.starts_with("bot-")
                || localpart_lc.starts_with("bot.")
            )
            && localpart_lc != "bots"
            && localpart_lc != "botfather"
            && token
                .chars()
                .all(|ch| ch.is_ascii_alphanumeric() || ch == '_' || ch == '-' || ch == '.');
        if !is_likely_bot_localpart {
            continue;
        }

        let Some(default_server_name) = default_server_name else { continue };
        let full_user_id = format!("@{token}:{default_server_name}");
        if let Ok(bot_user_id) = UserId::parse(&full_user_id).map(|user_id| user_id.to_owned()) {
            push_bot(bot_user_id);
        }
    }

    bot_user_ids
}

#[derive(Clone, Default, Debug)]
pub enum AppServicePanelAction {
    Dismiss,
    OpenCreateBotModal,
    OpenDeleteBotModal,
    SendListBots,
    SendBotHelp,
    ShowBoundBots,
    Unbind,
    #[default]
    None,
}

impl ActionDefaultRef for AppServicePanelAction {
    fn default_ref() -> &'static Self {
        static DEFAULT: AppServicePanelAction = AppServicePanelAction::None;
        &DEFAULT
    }
}

#[derive(Script, ScriptHook, Widget)]
pub struct AppServicePanel {
    #[deref] view: View,
    #[rust] app_language: AppLanguage,
    #[rust] app_language_initialized: bool,
}

impl Widget for AppServicePanel {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if !self.app_language_initialized || self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }
        self.view.handle_event(cx, event, scope);

        let room_screen_props = scope
            .props
            .get::<RoomScreenProps>()
            .expect("BUG: RoomScreenProps should be available in Scope::props for AppServicePanel");
        self.view
            .button(cx, ids!(keyboard.third_row.view_bound_button))
            .set_visible(cx, room_screen_props.app_service_enabled);
        self.view
            .button(cx, ids!(keyboard.third_row.unbind_button))
            .set_visible(cx, room_screen_props.app_service_room_bound);

        if let Event::Actions(actions) = event {
            if self
                .view
                .button(cx, ids!(bubble.header.dismiss_button))
                .clicked(actions)
            {
                cx.widget_action(
                    room_screen_props.room_screen_widget_uid,
                    AppServicePanelAction::Dismiss,
                );
            }

            if self
                .view
                .button(cx, ids!(keyboard.first_row.create_button))
                .clicked(actions)
            {
                cx.widget_action(
                    room_screen_props.room_screen_widget_uid,
                    AppServicePanelAction::OpenCreateBotModal,
                );
            }

            if self
                .view
                .button(cx, ids!(keyboard.first_row.list_button))
                .clicked(actions)
            {
                cx.widget_action(
                    room_screen_props.room_screen_widget_uid,
                    AppServicePanelAction::SendListBots,
                );
            }

            if self
                .view
                .button(cx, ids!(keyboard.second_row.delete_button))
                .clicked(actions)
            {
                cx.widget_action(
                    room_screen_props.room_screen_widget_uid,
                    AppServicePanelAction::OpenDeleteBotModal,
                );
            }

            if self
                .view
                .button(cx, ids!(keyboard.second_row.help_button))
                .clicked(actions)
            {
                cx.widget_action(
                    room_screen_props.room_screen_widget_uid,
                    AppServicePanelAction::SendBotHelp,
                );
            }

            if self
                .view
                .button(cx, ids!(keyboard.third_row.view_bound_button))
                .clicked(actions)
            {
                cx.widget_action(
                    room_screen_props.room_screen_widget_uid,
                    AppServicePanelAction::ShowBoundBots,
                );
            }

            if self
                .view
                .button(cx, ids!(keyboard.third_row.unbind_button))
                .clicked(actions)
            {
                cx.widget_action(
                    room_screen_props.room_screen_widget_uid,
                    AppServicePanelAction::Unbind,
                );
            }
        }
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        let app_language = scope.data.get::<AppState>()
            .map(|app_state| app_state.app_language)
            .unwrap_or_default();
        if !self.app_language_initialized || self.app_language != app_language {
            self.set_app_language(cx, app_language);
        }
        self.view.draw_walk(cx, scope, walk)
    }
}

impl AppServicePanel {
    fn set_app_language(&mut self, cx: &mut Cx, app_language: AppLanguage) {
        self.app_language = app_language;
        self.app_language_initialized = true;
        self.view
            .label(cx, ids!(sender_row.sender_name))
            .set_text(cx, tr_key(self.app_language, "room_screen.app_service.sender_name"));
        self.view
            .label(cx, ids!(sender_row.sender_tag))
            .set_text(cx, tr_key(self.app_language, "room_screen.app_service.sender_tag"));
        self.view
            .label(cx, ids!(bubble.header.title))
            .set_text(cx, tr_key(self.app_language, "room_screen.app_service.title"));
        self.view
            .label(cx, ids!(bubble.subtitle))
            .set_text(cx, tr_key(self.app_language, "room_screen.app_service.subtitle"));
        self.view
            .label(cx, ids!(bubble.footer.timestamp))
            .set_text(cx, tr_key(self.app_language, "room_screen.app_service.timestamp_now"));
        self.view
            .button(cx, ids!(keyboard.first_row.create_button))
            .set_text(cx, tr_key(self.app_language, "room_screen.app_service.button.create_bot"));
        self.view
            .button(cx, ids!(keyboard.first_row.list_button))
            .set_text(cx, tr_key(self.app_language, "room_screen.app_service.button.list_bots"));
        self.view
            .button(cx, ids!(keyboard.second_row.delete_button))
            .set_text(cx, tr_key(self.app_language, "room_screen.app_service.button.delete_bot"));
        self.view
            .button(cx, ids!(keyboard.second_row.help_button))
            .set_text(cx, tr_key(self.app_language, "room_screen.app_service.button.bot_help"));
        self.view
            .button(cx, ids!(keyboard.third_row.view_bound_button))
            .set_text(cx, tr_key(self.app_language, "room_screen.app_service.button.bots"));
        self.view
            .button(cx, ids!(keyboard.third_row.unbind_button))
            .set_text(cx, tr_key(self.app_language, "room_screen.app_service.button.unbind"));
        self.view.redraw(cx);
    }
}


impl RoomScreen {
    pub(super) fn timeline_bot_context(
        &mut self,
        app_state: Option<&AppState>,
        room_id: &OwnedRoomId,
        room_members: Option<&Arc<Vec<RoomMember>>>,
    ) -> TimelineBotContext {
        let Some(app_state) = app_state else {
            return TimelineBotContext::default();
        };

        let app_service_enabled = app_state.bot_settings.enabled;
        let room_is_bound = app_state.bot_settings.is_room_bound(room_id);
        let persisted_bound_bot_user_id = if app_service_enabled {
            app_state.bot_settings.bound_bot_user_id(room_id).map(ToOwned::to_owned)
        } else {
            None
        };
        let persisted_bound_bot_user_ids = if app_service_enabled {
            app_state.bot_settings.bound_bot_user_ids(room_id)
        } else {
            Vec::new()
        };
        let resolved_parent_bot_user_id = if app_service_enabled {
            app_state
                .bot_settings
                .resolved_bot_user_id(current_user_id().as_deref())
                .ok()
        } else {
            None
        };
        let known_bot_user_ids = timeline_known_bot_user_ids(app_state);

        if let Some(cached) = self.timeline_bot_context_cache.as_ref()
            && cached.matches(
                room_id,
                room_members,
                app_service_enabled,
                room_is_bound,
                persisted_bound_bot_user_id.as_ref(),
                &persisted_bound_bot_user_ids,
                resolved_parent_bot_user_id.as_ref(),
                &known_bot_user_ids,
            )
        {
            return cached.value.clone();
        }
        if self.timeline_bot_context_cache.is_some() {
            self.invalidate_timeline_bot_context();
        }

        let has_persisted_management_binding = resolved_parent_bot_user_id
            .as_ref()
            .is_some_and(|resolved_parent_bot_user_id|
                persisted_bound_bot_user_ids
                    .iter()
                    .any(|bot_user_id| bot_user_id == resolved_parent_bot_user_id)
            );
        let room_bot_user_ids = room_members
            .map(|members|
                collect_room_bot_user_ids(
                    members.as_ref(),
                    resolved_parent_bot_user_id.as_deref(),
                    &known_bot_user_ids,
                    &persisted_bound_bot_user_ids,
                )
            )
            .unwrap_or_else(|| persisted_bound_bot_user_ids.clone());
        let detected_bound_bot_user_id = if app_service_enabled {
            room_members.and_then(|members|
                detected_bot_binding_for_members(
                    app_state,
                    room_id,
                    members.as_ref(),
                )
            )
        } else {
            None
        };
        let bound_bot_user_id = persisted_bound_bot_user_id
            .clone()
            .or(detected_bound_bot_user_id);
        let value = TimelineBotContext {
            app_service_enabled,
            app_service_room_bound: bound_bot_user_id.is_some(),
            has_persisted_management_binding,
            bound_bot_user_id,
            resolved_parent_bot_user_id: resolved_parent_bot_user_id.clone(),
            persisted_bound_bot_user_ids: persisted_bound_bot_user_ids.clone(),
            room_bot_user_ids,
            known_bot_user_ids: known_bot_user_ids.clone(),
        };
        self.timeline_bot_context_cache = Some(CachedTimelineBotContext {
            room_id: room_id.clone(),
            room_members: room_members.cloned(),
            app_service_enabled,
            room_is_bound,
            persisted_bound_bot_user_id,
            persisted_bound_bot_user_ids,
            resolved_parent_bot_user_id,
            known_bot_user_ids,
            value: value.clone(),
        });
        value
    }

    pub(super) fn invalidate_timeline_bot_context(&mut self) {
        self.timeline_bot_context_cache = None;
        if let Some(tl) = self.tl_state.as_mut() {
            tl.content_drawn_since_last_update.clear();
            tl.profile_drawn_since_last_update.clear();
        }
    }

    pub(super) fn discover_known_bot_user_ids_from_timeline_items(
        app_state: &AppState,
        timeline_items: &Vector<Arc<TimelineItem>>,
    ) -> Vec<OwnedUserId> {
        let Ok(parent_bot_user_id) = app_state
            .bot_settings
            .resolved_bot_user_id(current_user_id().as_deref())
        else {
            return Vec::new();
        };

        let default_server_name = current_user_id()
            .map(|user_id| user_id.server_name().to_owned());
        let mut discovered_bot_user_ids = Vec::<OwnedUserId>::new();
        let mut push_bot_user_id = |bot_user_id: OwnedUserId| {
            if bot_user_id.as_str() == parent_bot_user_id.as_str() {
                return;
            }
            if !discovered_bot_user_ids
                .iter()
                .any(|existing_bot_user_id| existing_bot_user_id.as_str() == bot_user_id.as_str())
            {
                discovered_bot_user_ids.push(bot_user_id);
            }
        };

        for item in timeline_items {
            let TimelineItemKind::Event(event_tl_item) = item.kind() else { continue };
            if event_tl_item.sender().as_str() != parent_bot_user_id.as_str() {
                continue;
            }
            let Some(message_text) = Self::extract_message_text(item) else { continue };
            for bot_user_id in extract_bot_user_ids_from_listbots_reply(
                &message_text,
                default_server_name.as_ref(),
            ) {
                push_bot_user_id(bot_user_id);
            }
        }

        discovered_bot_user_ids
    }

    pub(super) fn set_app_service_actions_visible(&mut self, cx: &mut Cx, visible: bool) {
        self.show_app_service_actions = visible;
        self.redraw(cx);
    }

    pub(super) fn toggle_app_service_actions(&mut self, cx: &mut Cx) {
        self.set_app_service_actions_visible(cx, !self.show_app_service_actions);
    }

    pub(super) fn close_create_bot_modal(&self, cx: &mut Cx) {
        self.view.modal(cx, ids!(create_bot_modal)).close(cx);
    }

    pub(super) fn close_delete_bot_modal(&self, cx: &mut Cx) {
        self.view.modal(cx, ids!(delete_bot_modal)).close(cx);
    }

    pub(super) fn open_create_bot_modal(&mut self, cx: &mut Cx) {
        let Some(room_name_id) = self.room_name_id.clone() else {
            return;
        };
        self.set_app_service_actions_visible(cx, false);
        self.view
            .create_bot_modal(cx, ids!(create_bot_modal_inner))
            .show(cx, room_name_id);
        self.view.modal(cx, ids!(create_bot_modal)).open(cx);
    }

    pub(super) fn open_delete_bot_modal(&mut self, cx: &mut Cx) {
        let Some(room_name_id) = self.room_name_id.clone() else {
            return;
        };
        self.set_app_service_actions_visible(cx, false);
        self.view
            .delete_bot_modal(cx, ids!(delete_bot_modal_inner))
            .show(cx, room_name_id);
        self.view.modal(cx, ids!(delete_bot_modal)).open(cx);
    }

    pub(super) fn reset_app_service_ui(&mut self, cx: &mut Cx) {
        self.set_app_service_actions_visible(cx, false);
        self.close_create_bot_modal(cx);
        self.close_delete_bot_modal(cx);
    }

    pub(super) fn resolved_app_service_bot_user_id(
        &self,
        app_state: &AppState,
        room_id: &OwnedRoomId,
    ) -> Option<OwnedUserId> {
        if let Some(bot_user_id) = app_state.bot_settings.bound_bot_user_id(room_id.as_ref()) {
            return Some(bot_user_id.to_owned());
        }

        self.tl_state
            .as_ref()
            .filter(|tl| tl.kind.room_id() == room_id)
            .and_then(|tl| tl.room_members.as_ref())
            .and_then(|members|
                detected_bot_binding_for_members(
                    app_state,
                    room_id,
                    members.as_ref(),
                )
            )
    }

    pub(super) fn is_app_service_room_bound(&self, app_state: &AppState, room_id: &OwnedRoomId) -> bool {
        self.resolved_app_service_bot_user_id(app_state, room_id).is_some()
    }

    pub(super) fn send_app_service_feedback_message(&self, message: impl Into<String>) {
        let Some(room_id) = self.room_id().cloned() else {
            return;
        };
        let message = format!("[App Service] {}", message.into());
        submit_async_request(MatrixRequest::SendMessage {
            timeline_kind: TimelineKind::MainRoom { room_id },
            message: RoomMessageEventContent::notice_plain(message),
            replied_to: None,
            target_user_id: None,
            explicit_room: false,
            broadcast_target_user_ids: None,
            #[cfg(feature = "tsp")]
            sign_with_tsp: false,
        });
    }

    pub(super) fn send_botfather_command(
        &mut self,
        cx: &mut Cx,
        app_state: &AppState,
        command: &str,
        success_message: String,
    ) -> bool {
        let Some(timeline_kind) = self.timeline_kind.clone() else {
            return false;
        };
        if timeline_kind.thread_root_event_id().is_some() {
            self.send_app_service_feedback_message(
                tr_key(self.app_language, "room_screen.popup.bot.main_timeline_only"),
            );
            return false;
        }

        let Some(room_id) = self.room_id().cloned() else {
            return false;
        };
        if !app_state.bot_settings.enabled {
            self.send_app_service_feedback_message(
                tr_key(self.app_language, "room_screen.popup.bot.enable_before_commands"),
            );
            return false;
        }
        let bound_bot_user_id = self.resolved_app_service_bot_user_id(app_state, &room_id);
        if bound_bot_user_id.is_none() {
            self.send_app_service_feedback_message(
                tr_key(self.app_language, "room_screen.popup.bot.bind_before_commands"),
            );
            return false;
        }

        submit_async_request(MatrixRequest::SendMessage {
            timeline_kind,
            message: RoomMessageEventContent::text_plain(command),
            replied_to: None,
            target_user_id: bound_bot_user_id,
            explicit_room: false,
            broadcast_target_user_ids: None,
            #[cfg(feature = "tsp")]
            sign_with_tsp: false,
        });

        self.send_app_service_feedback_message(success_message);
        self.set_app_service_actions_visible(cx, false);
        true
    }

    pub(super) fn send_create_bot_command(
        &mut self,
        cx: &mut Cx,
        app_state: &AppState,
        username: &str,
        display_name: &str,
        system_prompt: Option<&str>,
    ) {
        let Some(timeline_kind) = self.timeline_kind.clone() else {
            return;
        };
        if timeline_kind.thread_root_event_id().is_some() {
            self.send_app_service_feedback_message(
                tr_key(self.app_language, "room_screen.popup.bot.creation_main_timeline_only"),
            );
            return;
        }

        let Some(room_id) = self.room_id().cloned() else {
            return;
        };
        if !app_state.bot_settings.enabled {
            self.send_app_service_feedback_message(
                tr_key(self.app_language, "room_screen.popup.app_service.enable_before_create"),
            );
            return;
        }
        if !self.is_app_service_room_bound(app_state, &room_id) {
            self.send_app_service_feedback_message(
                tr_key(self.app_language, "room_screen.popup.app_service.bind_before_create"),
            );
            return;
        }

        let command = format_create_bot_command(username, display_name, system_prompt);
        if self.send_botfather_command(
            cx,
            app_state,
            &command,
            tr_fmt(self.app_language, "room_screen.popup.bot.sent_createbot", &[("username", username)]),
        ) {
            self.close_create_bot_modal(cx);
        }
    }

    pub(super) fn send_delete_bot_command(
        &mut self,
        cx: &mut Cx,
        app_state: &AppState,
        user_id_or_localpart: &str,
    ) {
        let matrix_user_id =
            match resolve_delete_bot_user_id(user_id_or_localpart, current_user_id().as_deref(), self.app_language) {
                Ok(user_id) => user_id,
                Err(error) => {
                    self.send_app_service_feedback_message(error);
                    return;
                }
            };

        let command = format_delete_bot_command(matrix_user_id.as_ref());
        if self.send_botfather_command(
            cx,
            app_state,
            &command,
            tr_fmt(self.app_language, "room_screen.popup.bot.sent_deletebot", &[("matrix_user_id", matrix_user_id.as_str())]),
        ) {
            self.close_delete_bot_modal(cx);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_bot_detection_configured_parent() {
        let user_id: OwnedUserId = "@octosbot:127.0.0.1:8128".try_into().unwrap();
        let resolved_parent_bot_user_id = Some(user_id.clone());
        let known_bot_user_ids = Vec::new();

        assert!(is_known_or_likely_bot(
            user_id.as_ref(),
            resolved_parent_bot_user_id.as_deref(),
            &known_bot_user_ids,
        ));
    }

    #[test]
    fn test_bot_detection_heuristic_fallback() {
        let user_id: OwnedUserId = "@myservice_bot:other.server".try_into().unwrap();
        let known_bot_user_ids = Vec::new();

        assert!(is_known_or_likely_bot(
            user_id.as_ref(),
            None,
            &known_bot_user_ids,
        ));
    }

    #[test]
    fn test_bot_detection_child_bot() {
        let user_id: OwnedUserId = "@octosbot_weather:127.0.0.1:8128".try_into().unwrap();
        let known_bot_user_ids = vec![user_id.clone()];

        assert!(is_known_or_likely_bot(
            user_id.as_ref(),
            None,
            &known_bot_user_ids,
        ));
    }

    #[test]
    fn test_bot_detection_rejects_normal_user() {
        let user_id: OwnedUserId = "@alice:127.0.0.1:8128".try_into().unwrap();
        let known_bot_user_ids = Vec::new();

        assert!(!is_known_or_likely_bot(
            user_id.as_ref(),
            None,
            &known_bot_user_ids,
        ));
    }

    #[test]
    fn test_timeline_bot_detection_uses_room_bot_user_ids() {
        let user_id: OwnedUserId = "@octosbot_bob:127.0.0.1:8128".try_into().unwrap();
        let room_bot_user_ids = vec![user_id.clone()];
        let known_bot_user_ids = Vec::new();

        assert!(is_timeline_sender_bot(
            user_id.as_ref(),
            None,
            &room_bot_user_ids,
            &known_bot_user_ids,
        ));
    }

    #[test]
    fn test_timeline_bot_context_cache_matches_its_full_identity_fingerprint() {
        let room_id: OwnedRoomId = "!room:example.org".try_into().unwrap();
        let known_bot_user_id: OwnedUserId = "@agent:example.org".try_into().unwrap();
        let cached = CachedTimelineBotContext {
            room_id: room_id.clone(),
            room_members: None,
            app_service_enabled: false,
            room_is_bound: false,
            persisted_bound_bot_user_id: None,
            persisted_bound_bot_user_ids: Vec::new(),
            resolved_parent_bot_user_id: None,
            known_bot_user_ids: vec![known_bot_user_id.clone()],
            value: TimelineBotContext::default(),
        };

        assert!(cached.matches(
            &room_id,
            None,
            false,
            false,
            None,
            &[],
            None,
            &[known_bot_user_id],
        ));
        assert!(!cached.matches(
            &room_id,
            None,
            false,
            false,
            None,
            &[],
            None,
            &[],
        ));
    }

    #[test]
    fn test_registry_agent_detected_as_bot_sender() {
        // An agent known only via the global AgentRegistry (not the app-service
        // known-bot list, and with a non-bot-like localpart) is still detected.
        let agent_id: OwnedUserId = "@agent:example.org".try_into().unwrap();
        let mut app_state = AppState::default();
        app_state
            .agent_registry
            .register(agent_id.clone(), crate::app::AgentEntry::default());

        let known_bot_user_ids = timeline_known_bot_user_ids(&app_state);
        assert!(is_known_or_likely_bot(agent_id.as_ref(), None, &known_bot_user_ids));
    }

    #[test]
    fn test_room_props_known_bot_user_ids_include_registry_agents() {
        let agent_id: OwnedUserId = "@agent:example.org".try_into().unwrap();
        let mut app_state = AppState::default();
        app_state
            .agent_registry
            .register(agent_id.clone(), crate::app::AgentEntry::default());

        let known_bot_user_ids = timeline_known_bot_user_ids(&app_state);

        assert!(known_bot_user_ids.iter().any(|id| id == &agent_id));
    }

    #[test]
    fn test_detected_bot_binding_uses_registry_augmented_known_bots() {
        let src = include_str!("bot_admin.rs");
        let fn_pos = src
            .find("fn detected_bot_binding_for_members")
            .expect("detected_bot_binding_for_members should exist");
        let fn_src = &src[fn_pos..src[fn_pos..].find("fn is_likely_bot_user_id")
            .map(|end| fn_pos + end)
            .unwrap_or(src.len())];

        assert!(
            fn_src.contains("timeline_known_bot_user_ids(app_state)"),
            "DM bot binding detection should include AgentRegistry agents such as OctosDirect",
        );
        assert!(
            !fn_src.contains("app_state.bot_settings.known_bot_user_ids()"),
            "DM bot binding detection must not read only raw AppService known-bots",
        );
    }

    #[test]
    fn test_non_agent_user_not_detected_as_bot() {
        // Empty registry, empty known-bot list, app-service disabled.
        let app_state = AppState::default();
        let human_id: OwnedUserId = "@human:example.org".try_into().unwrap();

        let known_bot_user_ids = timeline_known_bot_user_ids(&app_state);
        assert!(!is_known_or_likely_bot(human_id.as_ref(), None, &known_bot_user_ids));
    }

    #[test]
    fn test_empty_registry_and_no_known_bots_shows_no_bot_card() {
        let app_state = AppState::default();
        let known_bot_user_ids = timeline_known_bot_user_ids(&app_state);
        assert!(known_bot_user_ids.is_empty());

        let sender_id: OwnedUserId = "@someone:example.org".try_into().unwrap();
        let is_bot_sender = is_known_or_likely_bot(sender_id.as_ref(), None, &known_bot_user_ids);
        let render_state = compute_bot_timeline_render_state("hello", is_bot_sender);
        assert!(!render_state.show_card);
    }

    #[test]
    fn test_listbots_parser_ignores_octos_service_urls_and_ports() {
        let parsed = extract_bot_user_ids_from_listbots_reply(
            "Octos service: http://127.0.0.1:8787\nKnown bots: @octosbot:example.org",
            None,
        );

        assert_eq!(
            parsed,
            vec!["@octosbot:example.org".parse::<OwnedUserId>().unwrap()],
        );
        assert!(
            parsed.iter().all(|user_id| user_id.localpart() != ""),
            "service URL port fragments must not become Matrix user IDs",
        );
    }

    #[test]
    fn test_agent_chat_puppets_are_treated_as_bots() {
        // Real MXIDs from an agent-chat team: none contain "bot".
        for localpart in [
            "ac_tyrese_coordinator",
            "ac_tyrese_implementer",
            "ac_tyrese_reviewer",
            "ac_tyrese_final_reviewer",
            "wf_coordinator",
        ] {
            assert!(
                is_agent_chat_puppet_localpart(localpart),
                "{localpart} must be recognised as an agent-chat puppet"
            );
        }
        // Humans must not be swept up by the role-suffix rule.
        for localpart in ["tyreseluo", "alex", "haitang", "coordinator_notes"] {
            assert!(
                !is_agent_chat_puppet_localpart(localpart),
                "{localpart} must NOT be treated as a bot"
            );
        }
    }
}
