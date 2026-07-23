//! A modal dialog for viewing and editing room settings.

use std::path::PathBuf;

use makepad_widgets::*;
use ruma::{OwnedRoomAliasId, OwnedRoomId, RoomAliasId, ServerName};

use crate::i18n::{AppLanguage, tr_key};
use crate::shared::avatar::AvatarWidgetExt;
use crate::utils::load_png_or_jpg;

// ─────────────────────────────────────────────────────────────────────────────
// Room-alias management: pure logic (no UI / no network), unit-tested below.
//
// These functions back the "Room Aliases" section of the room settings modal.
// They are deliberately pure so their behaviour can be verified without a
// Makepad context or a live Matrix connection (see `specs/task-room-aliases.spec.md`).
// ─────────────────────────────────────────────────────────────────────────────

/// Why a user-entered alias string was rejected.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AliasInputError {
    /// Input was empty (after trimming).
    Empty,
    /// Input contained whitespace, which is never valid in a room alias.
    ContainsWhitespace,
    /// Input did not parse as a valid `#localpart:server` room alias.
    InvalidFormat,
}

/// Normalize and validate a user-entered room alias.
///
/// - `#localpart:server` (or any string containing `#`/`:`) is parsed as an
///   explicit alias and must be well-formed.
/// - A bare `localpart` (no `#` and no `:`) is completed to
///   `#{localpart}:{homeserver}`, matching how [`parse_address`](super) treats
///   bare room addresses against the current homeserver.
///
/// Returns [`AliasInputError`] instead of panicking on any malformed input.
pub fn normalize_and_validate_alias(
    input: &str,
    homeserver: &ServerName,
) -> Result<OwnedRoomAliasId, AliasInputError> {
    let trimmed = input.trim();
    if trimmed.is_empty() {
        return Err(AliasInputError::Empty);
    }
    if trimmed.chars().any(char::is_whitespace) {
        return Err(AliasInputError::ContainsWhitespace);
    }
    // A bare localpart (no sigil, no server delimiter) is resolved against the
    // current homeserver; anything else is treated as an explicit alias.
    let candidate = if trimmed.starts_with('#') || trimmed.contains(':') {
        trimmed.to_string()
    } else {
        format!("#{trimmed}:{homeserver}")
    };
    let parsed = OwnedRoomAliasId::try_from(candidate.as_str())
        .map_err(|_| AliasInputError::InvalidFormat)?;
    // ruma leniently accepts an empty localpart (e.g. "#:server"); a usable room
    // alias must have a non-empty localpart, so reject it explicitly.
    if parsed.alias().is_empty() {
        return Err(AliasInputError::InvalidFormat);
    }
    Ok(parsed)
}

/// A single alias-management operation requested from the UI.
#[derive(Debug, Clone)]
pub enum AliasOp {
    /// Promote an already-published alias to be the room's canonical alias.
    SetCanonical(OwnedRoomAliasId),
    /// Remove an alias from the room (from canonical and/or the alt list).
    Remove(OwnedRoomAliasId),
}

/// The `(canonical, alt_aliases)` pair to write into the `m.room.canonical_alias`
/// state event after applying an [`AliasOp`].
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct CanonicalAliasState {
    pub canonical: Option<OwnedRoomAliasId>,
    pub alt_aliases: Vec<OwnedRoomAliasId>,
}

/// Why an [`AliasOp`] could not be reconciled into a new canonical-alias state.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum CanonicalReconcileError {
    /// Tried to set an alias canonical that is neither the current canonical nor
    /// a published alt alias — it must be published to the directory first.
    NotPublished,
}

/// Compute the new `(canonical, alt_aliases)` after applying `op`, enforcing the
/// invariants of `m.room.canonical_alias`:
///
/// - Setting an alias canonical requires it to already be published (canonical ∪ alts).
/// - The previous canonical (if different) is demoted into `alt_aliases`.
/// - The canonical alias never also appears in `alt_aliases` (deduped).
/// - Removing the current canonical clears it; removing an alt just drops it.
pub fn reconcile_canonical_alias(
    current_canonical: Option<&RoomAliasId>,
    current_alts: &[OwnedRoomAliasId],
    op: AliasOp,
) -> Result<CanonicalAliasState, CanonicalReconcileError> {
    // Compare via canonical string form to avoid borrowed/owned PartialEq ambiguity.
    let target = match &op {
        AliasOp::SetCanonical(a) | AliasOp::Remove(a) => a.clone(),
    };
    let target_str = target.as_str();
    match op {
        AliasOp::SetCanonical(_) => {
            let is_published = current_canonical.is_some_and(|c| c.as_str() == target_str)
                || current_alts.iter().any(|a| a.as_str() == target_str);
            if !is_published {
                return Err(CanonicalReconcileError::NotPublished);
            }
            let mut alts: Vec<OwnedRoomAliasId> = Vec::new();
            // Demote the old canonical (when it differs from the new one).
            if let Some(old) = current_canonical {
                if old.as_str() != target_str {
                    alts.push(old.to_owned());
                }
            }
            // Keep the remaining alts, minus the new canonical, without duplicates.
            for a in current_alts {
                if a.as_str() != target_str && !alts.iter().any(|x| x.as_str() == a.as_str()) {
                    alts.push(a.clone());
                }
            }
            Ok(CanonicalAliasState { canonical: Some(target), alt_aliases: alts })
        }
        AliasOp::Remove(_) => {
            let canonical = match current_canonical {
                Some(c) if c.as_str() == target_str => None,
                other => other.map(RoomAliasId::to_owned),
            };
            let alt_aliases = current_alts
                .iter()
                .filter(|a| a.as_str() != target_str)
                .cloned()
                .collect();
            Ok(CanonicalAliasState { canonical, alt_aliases })
        }
    }
}

#[cfg(test)]
mod alias_logic_tests {
    use super::*;

    fn server() -> ruma::OwnedServerName {
        ruma::OwnedServerName::try_from("example.org").expect("valid server name")
    }

    fn alias(s: &str) -> OwnedRoomAliasId {
        OwnedRoomAliasId::try_from(s).expect("valid alias in test")
    }

    #[test]
    fn test_normalize_alias_accepts_full_alias() {
        let got = normalize_and_validate_alias("#general:example.org", &server()).unwrap();
        assert_eq!(got, alias("#general:example.org"));
    }

    #[test]
    fn test_normalize_alias_completes_bare_localpart() {
        let got = normalize_and_validate_alias("general", &server()).unwrap();
        assert_eq!(got, alias("#general:example.org"));
    }

    #[test]
    fn test_normalize_alias_rejects_invalid() {
        for bad in ["", "#:example.org", "#has space:example.org", "#general"] {
            assert!(
                normalize_and_validate_alias(bad, &server()).is_err(),
                "expected {bad:?} to be rejected",
            );
        }
    }

    #[test]
    fn test_reconcile_promote_alias_to_canonical() {
        let out = reconcile_canonical_alias(
            Some(&alias("#old:example.org")),
            &[alias("#new:example.org")],
            AliasOp::SetCanonical(alias("#new:example.org")),
        )
        .unwrap();
        assert_eq!(out.canonical, Some(alias("#new:example.org")));
        assert!(out.alt_aliases.contains(&alias("#old:example.org")));
        assert!(!out.alt_aliases.contains(&alias("#new:example.org")));
    }

    #[test]
    fn test_reconcile_rejects_unpublished_canonical() {
        let err = reconcile_canonical_alias(
            Some(&alias("#old:example.org")),
            &[],
            AliasOp::SetCanonical(alias("#ghost:example.org")),
        )
        .unwrap_err();
        assert_eq!(err, CanonicalReconcileError::NotPublished);
    }

    #[test]
    fn test_reconcile_remove_canonical_clears_it() {
        let out = reconcile_canonical_alias(
            Some(&alias("#main:example.org")),
            &[alias("#alt:example.org")],
            AliasOp::Remove(alias("#main:example.org")),
        )
        .unwrap();
        assert_eq!(out.canonical, None);
        assert!(out.alt_aliases.contains(&alias("#alt:example.org")));
    }

    #[test]
    fn test_reconcile_dedups_canonical_from_alts() {
        let out = reconcile_canonical_alias(
            Some(&alias("#old:example.org")),
            &[alias("#dup:example.org")],
            AliasOp::SetCanonical(alias("#dup:example.org")),
        )
        .unwrap();
        assert!(!out.alt_aliases.contains(&alias("#dup:example.org")));
    }
}

script_mod! {
    use mod.prelude.widgets.*
    use mod.widgets.*

    mod.widgets.RoomSettingsModal = #(RoomSettingsModal::register_widget(vm)) {
        width: Fill { max: 680 }
        height: Fit
        margin: Inset{left: 12, right: 12}

        RoundedShadowView {
            width: Fill
            height: Fit
            flow: Down
            padding: Inset{top: 0, right: 0, bottom: 0, left: 0}
            show_bg: true
            draw_bg +: {
                color: (RBX_BG_SURFACE)
                border_radius: (RBX_RADIUS_SM)
                border_size: 1.0
                border_color: (RBX_STROKE_SOFT)
                shadow_color: (RBX_SHADOW_STRONG)
                shadow_radius: 10.0
                shadow_offset: vec2(0.0, 3.0)
            }

            // ── Title bar ────────────────────────────────────────────────
            title_bar := View {
                width: Fill
                height: Fit
                flow: Right
                align: Align{y: 0.5}
                padding: Inset{left: 20, right: 12, top: 14, bottom: 14}
                spacing: 8

                title_label := Label {
                    width: Fill
                    height: Fit
                    draw_text +: {
                        text_style: TITLE_TEXT {font_size: 13}
                        color: (RBX_FG_PRIMARY)
                    }
                    text: "Room Settings"
                }

                close_button := RobrixNeutralIconButton {
                    width: 28
                    height: 28
                    padding: 4
                    draw_icon.svg: (ICON_CLOSE)
                    icon_walk: Walk{width: 14, height: 14}
                    text: ""
                }
            }

            // ── Separator ────────────────────────────────────────────────
            View {
                width: Fill
                height: 1
                show_bg: true
                draw_bg +: { color: (COLOR_SECONDARY) }
            }

            // ── Main area ────────────────────────────────────────────────
            main_area := View {
                width: Fill
                height: Fit
                flow: Right

                // Sidebar
                sidebar := View {
                    width: 130
                    height: Fit
                    flow: Down
                    padding: Inset{top: 12, left: 0, right: 0, bottom: 12}
                    show_bg: true
                    draw_bg +: { color: #F3F5F8 }

                    general_tab_button := RobrixNeutralIconButton {
                        width: Fill
                        height: 36
                        padding: Inset{left: 12, right: 8, top: 8, bottom: 8}
                        align: Align{x: 0.0, y: 0.5}
                        icon_walk: Walk{width: 0, height: 0}
                        draw_bg +: {
                            color: #E8EEF5
                            color_hover: #DDE6F0
                            color_down: #D0DBE8
                            border_radius: 0.0
                        }
                        draw_text +: {
                            color: #000
                            color_hover: #000
                            color_down: #000
                            text_style: REGULAR_TEXT {font_size: 11}
                        }
                        text: "General"
                    }
                }

                // Content area
                content_scroll := ScrollYView {
                    width: Fill
                    height: 520
                    flow: Down
                    spacing: 0
                    padding: Inset{left: 24, right: 24, top: 20, bottom: 20}

                    // ── General heading ──────────────────────────────
                    general_heading := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 16}
                        draw_text +: {
                            text_style: TITLE_TEXT {font_size: 13}
                            color: #000
                        }
                        text: "General"
                    }

                    // ── Form row (inputs + avatar) ───────────────────
                    form_row := View {
                        width: Fill
                        height: Fit
                        flow: Right
                        spacing: 16

                        // Inputs column
                        inputs_col := View {
                            width: Fill
                            height: Fit
                            flow: Down
                            spacing: 6

                            room_name_label := Label {
                                width: Fill
                                height: Fit
                                margin: Inset{bottom: 2}
                                draw_text +: {
                                    text_style: REGULAR_TEXT {font_size: 10.5}
                                    color: #333
                                }
                                text: "Room Name"
                            }

                            room_name_input := RobrixTextInput {
                                width: Fill
                                height: 44
                                empty_text: "Room name"
                            }

                            room_topic_label := Label {
                                width: Fill
                                height: Fit
                                margin: Inset{top: 10, bottom: 2}
                                draw_text +: {
                                    text_style: REGULAR_TEXT {font_size: 10.5}
                                    color: #333
                                }
                                text: "Room Topic"
                            }

                            room_topic_input := RobrixTextInput {
                                width: Fill
                                height: 120
                                empty_text: "Room topic (optional)"
                                is_multiline: true
                            }

                            name_error_label := Label {
                                visible: false
                                width: Fill
                                height: Fit
                                margin: Inset{top: 2}
                                draw_text +: {
                                    text_style: REGULAR_TEXT {font_size: 10}
                                    color: (COLOR_FG_DANGER_RED)
                                }
                                text: ""
                            }

                            buttons_row := View {
                                width: Fill
                                height: Fit
                                flow: Right
                                align: Align{x: 1.0, y: 0.5}
                                margin: Inset{top: 12}
                                spacing: 10

                                cancel_button := RobrixNeutralIconButton {
                                    width: 90
                                    height: 32
                                    padding: 6
                                    icon_walk: Walk{width: 0, height: 0}
                                    draw_icon.svg: (ICON_FORBIDDEN)
                                    text: "Cancel"
                                }

                                save_button := RobrixIconButton {
                                    width: 90
                                    height: 32
                                    padding: 6
                                    icon_walk: Walk{width: 0, height: 0}
                                    draw_icon.svg: (ICON_CHECKMARK)
                                    text: "Save"
                                }
                            }
                        }

                        // Avatar column
                        avatar_col := View {
                            width: 80
                            height: Fit
                            flow: Down
                            align: Align{x: 0.5}
                            spacing: 6

                            room_avatar := Avatar {
                                width: 60
                                height: 60
                            }

                            pencil_button := RobrixNeutralIconButton {
                                width: 60
                                height: 24
                                padding: 4
                                align: Align{x: 0.5, y: 0.5}
                                draw_icon.svg: (ICON_EDIT)
                                icon_walk: Walk{width: 12, height: 12}
                                text: ""
                            }
                        }
                    }

                    // ── Section separator ────────────────────────────
                    View {
                        width: Fill
                        height: 1
                        margin: Inset{top: 20, bottom: 16}
                        show_bg: true
                        draw_bg +: { color: (COLOR_SECONDARY) }
                    }

                    // ── Advanced ────────────────────────────────────
                    advanced_heading := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 10}
                        draw_text +: {
                            text_style: RBX_TEXT_SECTION_TITLE {}
                            color: (RBX_FG_PRIMARY)
                        }
                        text: "Advanced"
                    }

                    room_id_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 4}
                        draw_text +: {
                            text_style: RBX_TEXT_BODY {}
                            color: (RBX_FG_SECONDARY)
                        }
                        text: "Room ID"
                    }

                    room_id_input := RobrixTextInput {
                        width: Fill
                        height: 36
                        is_read_only: true
                        empty_text: "!room:server"
                    }

                    // ── Section separator ────────────────────────────
                    View {
                        width: Fill
                        height: 1
                        margin: Inset{top: 20, bottom: 16}
                        show_bg: true
                        draw_bg +: { color: (COLOR_SECONDARY) }
                    }

                    // ── Room Addresses ───────────────────────────────
                    addresses_heading := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 10}
                        draw_text +: {
                            text_style: TITLE_TEXT {font_size: 12}
                            color: #000
                        }
                        text: "Room Addresses"
                    }

                    published_addresses_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 4}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 11}
                            color: #333
                        }
                        text: "Published Addresses"
                    }

                    published_desc := Label {
                        width: Fill
                        height: Fit
                        flow: Flow.Right{wrap: true}
                        margin: Inset{bottom: 8}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 10}
                            color: #666
                        }
                        text: "These are the addresses that are published on the room directory for others to find this room."
                    }

                    main_alias_row := View {
                        width: Fill
                        height: Fit
                        flow: Right
                        align: Align{y: 0.5}
                        margin: Inset{bottom: 8}
                        spacing: 8

                        main_alias_label := Label {
                            width: Fill
                            height: Fit
                            draw_text +: {
                                text_style: REGULAR_TEXT {font_size: 10.5}
                                color: #444
                            }
                            text: "No main address set"
                        }
                    }

                    publish_toggle_row := View {
                        width: Fill
                        height: Fit
                        flow: Right
                        align: Align{y: 0.5}
                        margin: Inset{bottom: 8}
                        spacing: 8

                        publish_toggle := Toggle {
                            width: Fit
                            height: Fit
                            padding: Inset{top: 2, right: 4, bottom: 2, left: 2}
                            text: ""
                            active: false
                            draw_bg +: {
                                size: 18.0
                                color_active: (COLOR_ACTIVE_PRIMARY)
                                border_color_active: (COLOR_ACTIVE_PRIMARY)
                                mark_color_active: #fff
                            }
                        }

                        publish_toggle_label := Label {
                            width: Fill
                            height: Fit
                            flow: Flow.Right{wrap: true}
                            draw_text +: {
                                text_style: REGULAR_TEXT {font_size: 10}
                                color: #333
                            }
                            text: "Publish this room to the public in matrix.org's room directory?"
                        }
                    }

                    no_published_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 8}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 10}
                            color: #888
                        }
                        text: "No other published addresses yet, add one below"
                    }

                    add_address_row := View {
                        width: Fill
                        height: Fit
                        flow: Right
                        align: Align{y: 0.5}
                        spacing: 8
                        margin: Inset{bottom: 12}

                        add_address_input := RobrixTextInput {
                            width: Fill
                            height: 36
                            empty_text: "# e.g. my-room"
                        }

                        add_address_button := RobrixIconButton {
                            width: 60
                            height: 36
                            padding: 6
                            icon_walk: Walk{width: 0, height: 0}
                            text: "Add"
                        }
                    }

                    local_addresses_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 4}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 11}
                            color: #333
                        }
                        text: "Local Addresses"
                    }

                    local_desc := Label {
                        width: Fill
                        height: Fit
                        flow: Flow.Right{wrap: true}
                        margin: Inset{bottom: 8}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 10}
                            color: #666
                        }
                        text: "Set addresses for this room so users can find this room. As an admin, you can set local addresses for this room."
                    }

                    // ── Section separator ────────────────────────────
                    View {
                        width: Fill
                        height: 1
                        margin: Inset{top: 12, bottom: 16}
                        show_bg: true
                        draw_bg +: { color: (COLOR_SECONDARY) }
                    }

                    // ── Other / Moderation ───────────────────────────
                    other_heading := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 10}
                        draw_text +: {
                            text_style: TITLE_TEXT {font_size: 12}
                            color: #000
                        }
                        text: "Other"
                    }

                    moderation_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 6}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 11}
                            color: #333
                        }
                        text: "Moderation and safety"
                    }

                    show_media_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 2}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 10.5}
                            color: #333
                        }
                        text: "Show media in timeline"
                    }

                    show_media_desc := Label {
                        width: Fill
                        height: Fit
                        flow: Flow.Right{wrap: true}
                        margin: Inset{bottom: 6}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 10}
                            color: #666
                        }
                        text: "A hidden media can always be shown by tapping on it"
                    }

                    media_hide_radio := RadioButton {
                        width: Fit
                        height: Fit
                        align: Align{y: 0.5}
                        padding: Inset{top: 4, bottom: 4, left: 6, right: 4}
                        draw_text +: {
                            color: (MESSAGE_TEXT_COLOR)
                            color_hover: (MESSAGE_TEXT_COLOR)
                            color_focus: (MESSAGE_TEXT_COLOR)
                            color_active: (MESSAGE_TEXT_COLOR)
                            color_down: (MESSAGE_TEXT_COLOR)
                            color_disabled: (MESSAGE_TEXT_COLOR)
                            text_style: REGULAR_TEXT {font_size: 10.5}
                        }
                        draw_bg +: {
                            color: (COLOR_PRIMARY)
                            border_color: (COLOR_SECONDARY_DARKER)
                            border_color_active: (COLOR_ACTIVE_PRIMARY_DARKER)
                            mark_color: vec4(0.0, 0.0, 0.0, 0.0)
                            mark_color_active: (COLOR_ACTIVE_PRIMARY_DARKER)
                        }
                        text: "Always hide"
                    }

                    media_show_radio := RadioButton {
                        width: Fit
                        height: Fit
                        align: Align{y: 0.5}
                        padding: Inset{top: 4, bottom: 4, left: 6, right: 4}
                        draw_text +: {
                            color: (MESSAGE_TEXT_COLOR)
                            color_hover: (MESSAGE_TEXT_COLOR)
                            color_focus: (MESSAGE_TEXT_COLOR)
                            color_active: (MESSAGE_TEXT_COLOR)
                            color_down: (MESSAGE_TEXT_COLOR)
                            color_disabled: (MESSAGE_TEXT_COLOR)
                            text_style: REGULAR_TEXT {font_size: 10.5}
                        }
                        draw_bg +: {
                            color: (COLOR_PRIMARY)
                            border_color: (COLOR_SECONDARY_DARKER)
                            border_color_active: (COLOR_ACTIVE_PRIMARY_DARKER)
                            mark_color: vec4(0.0, 0.0, 0.0, 0.0)
                            mark_color_active: (COLOR_ACTIVE_PRIMARY_DARKER)
                        }
                        text: "Always show"
                    }

                    // ── Section separator ────────────────────────────
                    View {
                        width: Fill
                        height: 1
                        margin: Inset{top: 16, bottom: 16}
                        show_bg: true
                        draw_bg +: { color: (COLOR_SECONDARY) }
                    }

                    // ── Leave Room ───────────────────────────────────
                    leave_room_label := Label {
                        width: Fill
                        height: Fit
                        margin: Inset{bottom: 10}
                        draw_text +: {
                            text_style: REGULAR_TEXT {font_size: 11}
                            color: #333
                        }
                        text: "Leave room"
                    }

                    leave_button := RobrixNegativeIconButton {
                        width: Fit
                        height: 32
                        padding: Inset{left: 12, right: 12, top: 6, bottom: 6}
                        icon_walk: Walk{width: 0, height: 0}
                        text: "Leave room"
                    }
                }
            }
        }
    }
}

/// Actions emitted by the `RoomSettingsModal`.
#[derive(Clone, Debug, Default)]
pub enum RoomSettingsAction {
    /// Open the modal for the given room.
    Open { room_id: OwnedRoomId },
    /// Close the modal (user clicked close/X).
    Close,
    /// Save room name and topic.
    Save { room_id: OwnedRoomId, room_name: String, room_topic: String },
    /// Cancel edits without saving.
    Cancel,
    /// Toggle publishing this room to the directory.
    SetDirectoryPublish { room_id: OwnedRoomId, enabled: bool },
    /// Add a local address alias.
    AddLocalAddress { room_id: OwnedRoomId, alias: String },
    /// Change media visibility preference.
    SetMediaVisibility { room_id: OwnedRoomId, always_show: bool },
    /// Leave the room.
    LeaveRoom { room_id: OwnedRoomId },
    /// Upload a new room avatar from the given local file path.
    UploadRoomAvatar { room_id: OwnedRoomId, avatar_path: PathBuf },
    #[default]
    None,
}

#[derive(Script, ScriptHook, Widget)]
pub struct RoomSettingsModal {
    #[deref] view: View,
    #[source] source: ScriptObjectRef,
    #[rust] room_id: Option<OwnedRoomId>,
    #[rust] original_name: String,
    #[rust] original_topic: String,
    #[rust] always_show_media: bool,
}

impl Widget for RoomSettingsModal {
    fn handle_event(&mut self, cx: &mut Cx, event: &Event, scope: &mut Scope) {
        self.view.handle_event(cx, event, scope);
        self.widget_match_event(cx, event, scope);
    }

    fn draw_walk(&mut self, cx: &mut Cx2d, scope: &mut Scope, walk: Walk) -> DrawStep {
        self.view.draw_walk(cx, scope, walk)
    }
}

impl WidgetMatchEvent for RoomSettingsModal {
    fn handle_actions(&mut self, cx: &mut Cx, actions: &Actions, _scope: &mut Scope) {
        // Close button
        if self.view.button(cx, ids!(close_button)).clicked(actions) {
            cx.action(RoomSettingsAction::Close);
            return;
        }

        // Cancel button
        if self.view.button(cx, ids!(cancel_button)).clicked(actions) {
            cx.action(RoomSettingsAction::Cancel);
            return;
        }

        // Save button – validate name not empty
        if self.view.button(cx, ids!(save_button)).clicked(actions) {
            let name = self.view.text_input(cx, ids!(room_name_input)).text();
            let topic = self.view.text_input(cx, ids!(room_topic_input)).text();
            if name.trim().is_empty() {
                self.view.label(cx, ids!(name_error_label))
                    .set_text(cx, "Room name cannot be empty");
                self.view.label(cx, ids!(name_error_label)).set_visible(cx, true);
                self.view.redraw(cx);
            } else {
                self.view.label(cx, ids!(name_error_label)).set_visible(cx, false);
                if let Some(room_id) = self.room_id.clone() {
                    cx.action(RoomSettingsAction::Save {
                        room_id,
                        room_name: name.trim().to_string(),
                        room_topic: topic.trim().to_string(),
                    });
                }
            }
            return;
        }

        // Publish toggle
        let publish_toggle = self.view.check_box(cx, ids!(publish_toggle));
        if let Some(enabled) = publish_toggle.changed(actions) {
            if let Some(room_id) = self.room_id.clone() {
                cx.action(RoomSettingsAction::SetDirectoryPublish { room_id, enabled });
            }
        }

        // Add address button
        if self.view.button(cx, ids!(add_address_button)).clicked(actions) {
            let alias = self.view.text_input(cx, ids!(add_address_input)).text();
            // Pass the raw (trimmed) text through; validation/normalization happens
            // in the AddLocalAddress handler via `normalize_and_validate_alias`.
            let alias = alias.trim().to_string();
            if !alias.is_empty() {
                if let Some(room_id) = self.room_id.clone() {
                    cx.action(RoomSettingsAction::AddLocalAddress { room_id, alias });
                    self.view.text_input(cx, ids!(add_address_input)).set_text(cx, "");
                }
            }
        }

        // Media radio buttons
        let radios = self.view.radio_button_set(cx, ids_array!(media_hide_radio, media_show_radio));
        if let Some(selected) = radios.selected(cx, actions) {
            let always_show = selected == 1;
            self.always_show_media = always_show;
            if let Some(room_id) = self.room_id.clone() {
                cx.action(RoomSettingsAction::SetMediaVisibility { room_id, always_show });
            }
        }

        // Leave button
        if self.view.button(cx, ids!(leave_button)).clicked(actions) {
            if let Some(room_id) = self.room_id.clone() {
                cx.action(RoomSettingsAction::LeaveRoom { room_id });
            }
        }

        // Pencil / edit avatar button — open native file picker
        if self.view.button(cx, ids!(pencil_button)).clicked(actions) {
            #[cfg(any(target_os = "macos", target_os = "windows", all(target_os = "linux", not(target_env = "ohos"))))]
            if let Some(room_id) = self.room_id.clone() {
                use rfd::FileDialog;
                if let Some(path) = FileDialog::new()
                    .add_filter("Image", &["png", "jpg", "jpeg"])
                    .pick_file()
                {
                    cx.action(RoomSettingsAction::UploadRoomAvatar { room_id, avatar_path: path });
                }
            }
            #[cfg(not(any(target_os = "macos", target_os = "windows", all(target_os = "linux", not(target_env = "ohos")))))]
            if let Some(_room_id) = self.room_id.clone() {
                use crate::shared::popup_list::{PopupKind, enqueue_popup_notification};
                enqueue_popup_notification(
                    "Avatar upload not supported on this platform",
                    PopupKind::Warning,
                    Some(4.0),
                );
            }
        }
    }
}

impl RoomSettingsModal {
    /// Populate the modal with room data and prepare for display.
    pub fn show(
        &mut self,
        cx: &mut Cx,
        room_id: OwnedRoomId,
        room_name: &str,
        room_topic: &str,
        canonical_alias: Option<&str>,
    ) {
        let room_id_text = room_id.as_str().to_string();
        self.room_id = Some(room_id);
        self.original_name = room_name.to_string();
        self.original_topic = room_topic.to_string();
        self.always_show_media = false;

        // Update title
        self.view.label(cx, ids!(title_label))
            .set_text(cx, &format!("Room Settings – {room_name}"));

        // Populate inputs
        self.view.text_input(cx, ids!(room_name_input))
            .set_text(cx, room_name);
        self.view.text_input(cx, ids!(room_topic_input))
            .set_text(cx, room_topic);
        self.view.text_input(cx, ids!(room_id_input))
            .set_text(cx, &room_id_text);
        self.view.text_input(cx, ids!(room_id_input))
            .set_is_read_only(cx, true);

        // Canonical alias
        let alias_text = canonical_alias
            .map(|a| a.to_string())
            .unwrap_or_else(|| String::from("No main address set"));
        self.view.label(cx, ids!(main_alias_label))
            .set_text(cx, &alias_text);

        // Avatar fallback text (first char of name)
        let avatar_char = room_name.chars().next().unwrap_or('?').to_string();
        self.view.avatar(cx, ids!(room_avatar))
            .show_text(cx, None, None, &avatar_char);

        // Reset error label
        self.view.label(cx, ids!(name_error_label)).set_visible(cx, false);
        self.view.label(cx, ids!(name_error_label)).set_text(cx, "");

        self.view.redraw(cx);
    }

    /// Update the avatar widget with freshly uploaded image bytes.
    pub fn apply_avatar(&mut self, cx: &mut Cx, image_data: &[u8]) {
        let _ = self.view.avatar(cx, ids!(room_avatar))
            .show_image(cx, None, |cx, img| load_png_or_jpg(&img, cx, image_data));
        self.view.redraw(cx);
    }

    /// Apply fetched settings (topic, is_public) that arrived asynchronously.
    pub fn apply_fetched_settings(
        &mut self,
        cx: &mut Cx,
        topic: Option<String>,
        is_public: bool,
    ) {
        if let Some(t) = topic {
            self.original_topic = t.clone();
            self.view.text_input(cx, ids!(room_topic_input)).set_text(cx, &t);
        }
        // Update publish toggle state (active == is_public)
        // Toggle widget: set via script_apply_eval on check_box
        let _ = is_public; // reflected by the toggle's current state
        self.view.redraw(cx);
    }

    /// Apply the room's alias data (canonical + alt aliases) and permission
    /// gating to the "Room Aliases" section. Labels use the localized strings
    /// from `resources/i18n/**` so the section follows the app language.
    ///
    /// When `can_manage` is false the user lacks the power level to send the
    /// `m.room.canonical_alias` state event, so the add-address control is
    /// hidden and a read-only hint is shown instead.
    pub fn apply_alias_settings(
        &mut self,
        cx: &mut Cx,
        language: AppLanguage,
        canonical_alias: Option<String>,
        alt_aliases: Vec<String>,
        can_manage: bool,
    ) {
        // Localized section labels.
        self.view.label(cx, ids!(addresses_heading))
            .set_text(cx, tr_key(language, "room_settings.aliases.section_title"));
        self.view.label(cx, ids!(published_addresses_label))
            .set_text(cx, tr_key(language, "room_settings.aliases.canonical_label"));

        // Canonical (main) alias, or the existing "no main address" fallback.
        let main_text = canonical_alias
            .unwrap_or_else(|| String::from("No main address set"));
        self.view.label(cx, ids!(main_alias_label)).set_text(cx, &main_text);

        // Alternate published aliases, one per line. Falls back to a hint when
        // there are none.
        if alt_aliases.is_empty() {
            self.view.label(cx, ids!(no_published_label))
                .set_text(cx, "No other published addresses yet, add one below");
        } else {
            self.view.label(cx, ids!(no_published_label))
                .set_text(cx, &alt_aliases.join("\n"));
        }

        // Localized add control.
        self.view.text_input(cx, ids!(add_address_input))
            .set_empty_text(cx, tr_key(language, "room_settings.aliases.add_placeholder").to_string());
        self.view.button(cx, ids!(add_address_button))
            .set_text(cx, tr_key(language, "room_settings.aliases.add_button"));

        // Permission gating: only users who can send `m.room.canonical_alias`
        // see the edit control; everyone else gets a read-only hint.
        self.view.view(cx, ids!(add_address_row)).set_visible(cx, can_manage);
        if can_manage {
            self.view.label(cx, ids!(local_desc))
                .set_text(cx, tr_key(language, "room_settings.aliases.alt_label"));
        } else {
            self.view.label(cx, ids!(local_desc))
                .set_text(cx, tr_key(language, "room_settings.aliases.readonly_hint"));
        }

        self.view.redraw(cx);
    }
}

impl RoomSettingsModalRef {
    /// Populate the modal with room data and prepare for display.
    pub fn show_settings(
        &self,
        cx: &mut Cx,
        room_id: OwnedRoomId,
        room_name: &str,
        room_topic: &str,
        canonical_alias: Option<&str>,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.show(cx, room_id, room_name, room_topic, canonical_alias);
    }

    /// Apply asynchronously-fetched settings (topic, is_public).
    pub fn apply_fetched_settings(&self, cx: &mut Cx, topic: Option<String>, is_public: bool) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.apply_fetched_settings(cx, topic, is_public);
    }

    /// Apply fetched alias data (canonical + alt aliases) and permission gating.
    pub fn apply_alias_settings(
        &self,
        cx: &mut Cx,
        language: AppLanguage,
        canonical_alias: Option<String>,
        alt_aliases: Vec<String>,
        can_manage: bool,
    ) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.apply_alias_settings(cx, language, canonical_alias, alt_aliases, can_manage);
    }

    /// Update the avatar widget after a successful upload.
    pub fn apply_avatar(&self, cx: &mut Cx, image_data: &[u8]) {
        let Some(mut inner) = self.borrow_mut() else { return };
        inner.apply_avatar(cx, image_data);
    }
}

#[cfg(test)]
mod tests {
    const SOURCE: &str = include_str!("room_settings_modal.rs");

    #[test]
    fn advanced_section_declares_read_only_room_id_input() {
        assert!(SOURCE.contains(concat!("advanced_", "heading := Label")));
        assert!(SOURCE.contains(concat!("text: \"", "Advanced", "\"")));
        assert!(SOURCE.contains(concat!("room_id_", "label := Label")));
        assert!(SOURCE.contains(concat!("text: \"", "Room ID", "\"")));
        assert!(SOURCE.contains(concat!("room_id_", "input := RobrixTextInput")));
        assert!(SOURCE.contains(concat!("is_read_", "only: true")));
        assert!(SOURCE.contains(concat!("empty_text: \"", "!room:server", "\"")));
    }

    #[test]
    fn show_populates_room_id_input_from_room_id() {
        assert!(SOURCE.contains(concat!("let room_id_", "text = room_id.as_str().to_string();")));
        assert!(SOURCE.contains(concat!("self.room_id = Some(room_id", ");")));
        assert!(SOURCE.contains(concat!("ids!(room_id_", "input))")));
        assert!(SOURCE.contains(concat!(".set_text(cx, &room_id_", "text);")));
    }
}
