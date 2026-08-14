use std::{collections::HashMap, sync::OnceLock};

use serde::{Deserialize, Serialize};

/// App UI language preference stored in persisted app state.
#[derive(Clone, Copy, Debug, Default, PartialEq, Eq, Serialize, Deserialize)]
pub enum AppLanguage {
    #[serde(rename = "en", alias = "English")]
    #[default]
    English,
    #[serde(rename = "zh-CN", alias = "ChineseSimplified")]
    ChineseSimplified,
}

impl AppLanguage {
    pub const ALL: [Self; 2] = [
        Self::English,
        Self::ChineseSimplified,
    ];

    pub fn code(self) -> &'static str {
        match self {
            Self::English => "en",
            Self::ChineseSimplified => "zh-CN",
        }
    }

    pub fn from_dropdown_index(index: usize) -> Self {
        Self::ALL
            .get(index)
            .copied()
            .unwrap_or(Self::English)
    }

    pub fn dropdown_index(self) -> usize {
        Self::ALL
            .iter()
            .position(|lang| *lang == self)
            .unwrap_or(0)
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum I18nKey {
    AllSettingsTitle,
    SettingsCategoryAccount,
    SettingsCategoryPreferences,
    SettingsCategoryLabs,
    SettingsCategoryContribute,
    LanguageTitle,
    ApplicationLanguageLabel,
    LanguageReloadHint,
    LanguageOptionEnglish,
    LanguageOptionChineseSimplified,
}

impl I18nKey {
    fn as_str(self) -> &'static str {
        match self {
            I18nKey::AllSettingsTitle => "settings.all_settings_title",
            I18nKey::SettingsCategoryAccount => "settings.category.account",
            I18nKey::SettingsCategoryPreferences => "settings.category.preferences",
            I18nKey::SettingsCategoryLabs => "settings.category.labs",
            I18nKey::SettingsCategoryContribute => "settings.category.contribute",
            I18nKey::LanguageTitle => "settings.preferences.language.title",
            I18nKey::ApplicationLanguageLabel => "settings.preferences.language.application_label",
            I18nKey::LanguageReloadHint => "settings.preferences.language.reload_hint",
            I18nKey::LanguageOptionEnglish => "language.option.english",
            I18nKey::LanguageOptionChineseSimplified => "language.option.chinese_simplified",
        }
    }
}

fn load_dictionary(language: AppLanguage) -> HashMap<String, String> {
    let json = match language {
        AppLanguage::English => include_str!("../resources/i18n/en.json"),
        AppLanguage::ChineseSimplified => include_str!("../resources/i18n/zh-CN.json"),
    };
    serde_json::from_str(json).unwrap_or_default()
}

fn dictionary(language: AppLanguage) -> &'static HashMap<String, String> {
    static EN_DICTIONARY: OnceLock<HashMap<String, String>> = OnceLock::new();
    static ZH_CN_DICTIONARY: OnceLock<HashMap<String, String>> = OnceLock::new();

    match language {
        AppLanguage::English => EN_DICTIONARY.get_or_init(|| load_dictionary(AppLanguage::English)),
        AppLanguage::ChineseSimplified => ZH_CN_DICTIONARY.get_or_init(|| load_dictionary(AppLanguage::ChineseSimplified)),
    }
}

pub fn tr_key(language: AppLanguage, key: &str) -> &str {
    dictionary(language)
        .get(key)
        .map(String::as_str)
        .or_else(|| dictionary(AppLanguage::English).get(key).map(String::as_str))
        .unwrap_or(key)
}

pub fn tr_fmt(language: AppLanguage, key: &str, vars: &[(&str, &str)]) -> String {
    let mut output = tr_key(language, key).to_string();
    for (name, value) in vars {
        output = output.replace(&format!("{{{name}}}"), value);
    }
    output
}

pub fn tr(language: AppLanguage, key: I18nKey) -> &'static str {
    tr_key(language, key.as_str())
}

pub fn language_dropdown_labels(language: AppLanguage) -> Vec<String> {
    vec![
        tr(language, I18nKey::LanguageOptionEnglish).to_string(),
        tr(language, I18nKey::LanguageOptionChineseSimplified).to_string(),
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_invitebot_i18n_keys_exist_in_all_locales() {
        for key in [
            "slash_command.invitebot.description",
            "slash_command.invitebot.empty_hint",
            "slash_command.invitebot.all_present_hint",
        ] {
            for language in AppLanguage::ALL {
                assert!(
                    dictionary(language).contains_key(key),
                    "missing i18n key {key:?} for language {language:?}",
                );
            }
        }
    }

    #[test]
    fn space_invite_i18n_keys_exist_in_all_locales() {
        for key in [
            "rooms_list_entry.invited.space.by_name_and_user",
            "rooms_list_entry.invited.space.by_user",
            "rooms_list_entry.invited.space.generic",
            "invite_screen.message.invited_by_space",
            "invite_screen.message.invited_generic_space",
            "invite_screen.button.join_space",
            "invite_screen.popup.joined_space_success",
            "join_leave_modal.title.confirm_accept_space_invite",
            "join_leave_modal.description.confirm_accept_space_invite",
            "join_leave_modal.title.confirm_reject_space_invite",
            "join_leave_modal.description.confirm_reject_space_invite",
            "join_leave_modal.popup.joined_space_success",
            "join_leave_modal.title.joined_space",
            "join_leave_modal.title.error_joining_space",
        ] {
            for language in AppLanguage::ALL {
                assert!(
                    dictionary(language).contains_key(key),
                    "missing i18n key {key:?} for language {language:?}",
                );
            }
        }
    }

    #[test]
    fn create_space_i18n_keys_exist_in_all_locales() {
        for key in [
            "add_menu.item.new_space",
            "add_room.create_space.help.default",
            "add_room.create_space.help.fixed_parent",
            "add_room.create_space.input.placeholder",
            "add_room.create_space.visibility.option.private",
            "add_room.create_space.visibility.option.public",
            "add_room.create_space.visibility.hint.private",
            "add_room.create_space.visibility.hint.public",
            "add_room.create_space.button.create",
            "add_room.create_space.modal.title",
            "add_room.create_space.modal.subtitle",
            "add_room.popup.created_space_success",
            "add_room.feedback.creating_space",
            "space_lobby.header.button.new_subspace",
        ] {
            for language in AppLanguage::ALL {
                assert!(
                    dictionary(language).contains_key(key),
                    "missing i18n key {key:?} for language {language:?}",
                );
            }
        }
    }

    #[test]
    fn space_membership_i18n_keys_exist_in_all_locales() {
        for key in [
            "space_lobby.header.button.add_existing_room",
            "space_lobby.item.button.remove_from_space",
            "space_lobby.item.button.cancel",
            "space_lobby.remove_from_space.confirm.title_room",
            "space_lobby.remove_from_space.confirm.title_space",
            "space_lobby.remove_from_space.confirm.body_room",
            "space_lobby.remove_from_space.confirm.body_space",
            "space_lobby.remove_from_space.confirm.accept",
            "space_lobby.popup.added_to_space",
            "space_lobby.popup.removed_from_space",
            "space_lobby.popup.add_to_space_failed",
            "space_lobby.popup.remove_from_space_failed",
            "add_existing_room.title",
            "add_existing_room.subtitle",
            "add_existing_room.filter.placeholder",
            "add_existing_room.status.no_rooms",
            "add_existing_room.status.no_matches",
        ] {
            for language in AppLanguage::ALL {
                assert!(
                    dictionary(language).contains_key(key),
                    "missing i18n key {key:?} for language {language:?}",
                );
            }
        }
    }

    #[test]
    fn message_action_bar_i18n_keys_exist_in_all_locales() {
        for key in [
            "room_screen.popup.message.copied",
            "room_screen.popup.message.copy_empty",
        ] {
            for language in AppLanguage::ALL {
                assert!(
                    dictionary(language).contains_key(key),
                    "missing i18n key {key:?} for language {language:?}",
                );
            }
        }
    }

    #[test]
    fn translation_i18n_keys_exist_for_settings_and_room_input() {
        assert_eq!(
            tr_key(AppLanguage::English, "settings.labs.translation.title"),
            "Real-time Translation",
        );
        assert_eq!(
            tr_key(AppLanguage::ChineseSimplified, "settings.labs.translation.title"),
            "实时翻译",
        );
        assert_eq!(
            tr_key(AppLanguage::English, "room_input_bar.translation.preview.idle"),
            "Start typing to translate...",
        );
        assert_eq!(
            tr_key(AppLanguage::ChineseSimplified, "room_input_bar.translation.preview.idle"),
            "开始输入即可翻译...",
        );
        assert_eq!(
            tr_key(AppLanguage::ChineseSimplified, "room_input_bar.input.placeholder"),
            "输入消息（支持 Markdown）...",
        );
    }

    #[test]
    fn agent_ops_status_and_detail_keys_exist_in_all_locales() {
        for suffix in [
            "status.no_manifest",
            "status.unreadable_manifest",
            "status.contract_mismatch",
            "status.not_released",
            "status.unbound_commit",
            "status.invalid_source_commit",
            "status.empty_artifacts",
            "status.invalid_artifact_manifest",
            "status.incomplete_artifact_manifest",
            "status.artifact_set_mismatch",
            "status.artifact_digest_mismatch",
            "status.contract_ready",
            "detail.no_manifest",
            "detail.unreadable_manifest",
            "detail.contract_mismatch",
            "detail.not_released",
            "detail.unbound_commit",
            "detail.invalid_source_commit",
            "detail.empty_artifacts",
            "detail.invalid_artifact_manifest",
            "detail.incomplete_artifact_manifest",
            "detail.artifact_set_mismatch",
            "detail.artifact_digest_mismatch",
            "detail.contract_ready",
            "status.next_step_release",
            "status.next_step_runtime",
        ] {
            let key = format!("agent_ops.{suffix}");
            for language in AppLanguage::ALL {
                assert!(
                    dictionary(language).contains_key(&key),
                    "missing i18n key {key:?} for language {language:?}",
                );
            }
            assert_ne!(
                dictionary(AppLanguage::English).get(&key),
                dictionary(AppLanguage::ChineseSimplified).get(&key),
                "Chinese Agent Operations text must not fall back to English for {key}",
            );
        }
    }
}
