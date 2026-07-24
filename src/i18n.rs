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
    fn test_room_aliases_i18n_keys_exist_in_all_locales() {
        // Spec `specs/task-room-aliases.spec.md` Completion Criteria — the Room
        // Aliases section keys must resolve to a real translation (not the key
        // itself) in every locale.
        for key in [
            "room_settings.aliases.section_title",
            "room_settings.aliases.canonical_label",
            "room_settings.aliases.alt_label",
            "room_settings.aliases.add_placeholder",
            "room_settings.aliases.add_button",
            "room_settings.aliases.remove_button",
            "room_settings.aliases.set_canonical_button",
            "room_settings.aliases.invalid_format",
            "room_settings.aliases.publish_failed",
            "room_settings.aliases.readonly_hint",
            "room_settings.aliases.no_main_address",
            "room_settings.aliases.none_published",
            "room_settings.aliases.publishing",
            "room_settings.aliases.updating_main",
            "room_settings.aliases.removing",
            "room_settings.aliases.sign_in_required",
        ] {
            for language in AppLanguage::ALL {
                assert!(
                    dictionary(language).contains_key(key),
                    "missing i18n key {key:?} for language {language:?}",
                );
                assert_ne!(
                    tr_key(language, key),
                    key,
                    "i18n key {key:?} resolves to itself (no translation) for {language:?}",
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
}
