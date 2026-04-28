spec: task
name: "Real-time Translation for Room Input Bar"
inherits: project
tags: [feature, translation, llm, ui]
estimate: 2d
---

## Intent

Add a real-time translation feature to the chat message input bar. Users can toggle "write-and-translate" mode, select a target language, type in their native language, and see the translation appear in a preview area above the input. Clicking "Apply" replaces the input text with the translation, ready to send.

## Context

The translation feature uses any OpenAI-compatible LLM API (local or cloud) to translate text in real-time as the user types. The system prompt is adapted from the makepad-voice-input (Vox) project's bilingual correction+translation prompt.

## Acceptance Criteria

### Scenario: Configure translation API
- **Given** the user opens Settings → Labs → Real-time Translation
- **When** they enable the toggle and enter API URL, API key, and model name
- **Then** clicking "Save" persists the configuration
- **And** clicking "Test Connection" validates the API returns a response

### Scenario: Activate translation mode
- **Given** translation is configured and enabled
- **When** the user clicks the translate button (文A icon) in the input bar
- **Then** a language selector popup appears with 17 supported languages
- **When** the user selects a target language (e.g., English)
- **Then** translation mode activates with a preview area showing the language badge

### Scenario: Real-time translation
- **Given** translation mode is active with target language "English"
- **When** the user types "你好" in the input bar and pauses for 500ms
- **Then** an LLM translation request is sent
- **And** the preview area shows "Hello" (or equivalent translation)

### Scenario: Apply translation
- **Given** the translation preview shows a result
- **When** the user clicks "Apply"
- **Then** the translated text replaces the input bar content
- **And** the user can send the translated message normally

### Scenario: Deactivate translation
- **Given** translation mode is active
- **When** the user clicks the translate button again or the close (X) button on the preview
- **Then** translation mode deactivates
- **And** the preview area disappears

### Scenario: Settings changes take effect immediately
- **Given** translation mode is active
- **When** the user changes the model in Settings → Labs → Translation and clicks Save
- **Then** subsequent translation requests use the new model

## Decisions

- LLM API: OpenAI-compatible `/v1/chat/completions` endpoint via Makepad's `cx.http_request()`
- System prompt: Adapted from makepad-voice-input's bilingual correction+translation prompt, supporting both same-language correction and cross-language translation
- Debounce: 500ms timeout via Makepad `Timer` API — restart on each text change
- Config storage: `TranslationConfig` in `AppState` (persisted per account) + global `Mutex<Option<TranslationConfig>>` for cross-widget access (because `scope.data` is not available during Timer/Network events)
- Config refresh: RoomInputBar reads global config on every `handle_event` call (Mutex lock is nanosecond-level)
- Language selector: Static DSL items in `overlay_wrapper` with `abs_pos` positioning via `draw_walk`
- Labels populated via Rust code in `populate_language_list()` (DSL dot-path overrides on deeply nested named children don't work reliably)
- Translation preview: RoundedView above input bar with language badge, preview text, Apply button, close button
- HTTP response handling: via `Event::NetworkResponses` / `NetworkResponse::HttpResponse` pattern

## Boundaries

### Allowed Changes
- `src/room/translation.rs` — NEW: translation service, config, LLM API, response parsing
- `src/settings/translation_settings.rs` — NEW: Settings UI for translation API config
- `src/room/room_input_bar.rs` — translate button, language popup, preview, debounce, HTTP handling
- `src/room/mod.rs` — register translation module
- `src/settings/mod.rs` — register translation_settings module
- `src/settings/settings_screen.rs` — add TranslationSettings to Labs tab
- `src/app.rs` — add TranslationConfig to AppState, init global config on restore
- `src/home/home_screen.rs` — pass translation config to settings populate
- `resources/icons/translate.svg` — translation icon
- `resources/i18n/en.json`, `zh-CN.json` — translation-related i18n keys

### Forbidden
- Do NOT add new cargo dependencies
- Do NOT modify the message sending pipeline (translation is applied before send, not during)
- Do NOT store translation state in Matrix room events
- Do NOT run `cargo fmt`

## Supported Languages

| Code | Name |
|------|------|
| en | English |
| zh | 简体中文 |
| zh-TW | 繁體中文 |
| ja | 日本語 |
| ko | 한국어 |
| es | Español |
| fr | Français |
| de | Deutsch |
| ru | Русский |
| pt | Português |
| ar | العربية |
| hi | हिन्दी |
| th | ไทย |
| vi | Tiếng Việt |
| id | Bahasa Indonesia |
| ms | Bahasa Melayu |
| tr | Türkçe |

## Known Issues

- Language selector popup positioning uses `abs_pos` in `draw_walk` — position is calculated from button rect, may shift if input bar layout changes
- Settings → Preferences language dropdown has arrow visual artifact (see issues/002)

## Completion Criteria

- [x] Translation service with OpenAI-compatible API
- [x] Settings UI with toggle, API config, Save, Test Connection
- [x] Translate button in input bar
- [x] Language selector popup with 17 languages
- [x] Real-time debounced translation
- [x] Translation preview with Apply/Close
- [x] Global config for cross-widget access
- [x] Config changes take effect immediately
- [x] `cargo build` passes
