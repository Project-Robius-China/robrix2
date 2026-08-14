use std::io::Write;

use anyhow::Context as _;
use makepad_widgets::*;
use serde::{self, Deserialize, Serialize};
use matrix_sdk::ruma::{OwnedUserId, UserId};
use crate::{app::AppState, app_data_dir, persistence::persistent_state_dir};


const LATEST_APP_STATE_FILE_NAME: &str = "latest_app_state.json";
const SKIP_APP_STATE_RESTORE_ONCE_FILE_NAME: &str = "skip_app_state_restore_once";

const WINDOW_GEOM_STATE_FILE_NAME: &str = "window_geom_state.json";


/// Persistable state of the window's size, position, and fullscreen status.
#[derive(Default, Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct WindowGeomState {
    /// A tuple containing the window's width and height.
    pub inner_size: (f64, f64),
    /// A tuple containing the window's x and y position.
    pub position: (f64, f64),
    /// Maximise fullscreen if true.
    pub is_fullscreen: bool,
}


/// Save the current app state to persistent storage.
pub fn save_app_state(app_state: AppState, user_id: OwnedUserId) -> anyhow::Result<()> {
    let bytes = serialize_app_state(&app_state)?;
    save_app_state_bytes(&bytes, &user_id)
}

/// Serializes the current app state into the same format used by [`save_app_state`].
pub fn serialize_app_state(app_state: &AppState) -> anyhow::Result<Vec<u8>> {
    Ok(serde_json::to_vec(app_state)?)
}

/// Save pre-serialized app state bytes to persistent storage.
pub fn save_app_state_bytes(app_state_json: &[u8], user_id: &UserId) -> anyhow::Result<()> {
    let state_path = persistent_state_dir(user_id).join(LATEST_APP_STATE_FILE_NAME);
    save_app_state_bytes_to_path(app_state_json, &state_path)
}

fn save_app_state_bytes_to_path(
    app_state_json: &[u8],
    state_path: &std::path::Path,
) -> anyhow::Result<()> {
    if let Some(state_dir) = state_path.parent() {
        std::fs::create_dir_all(state_dir)?;
    }
    let file = std::fs::File::create(state_path)?;
    let mut writer = std::io::BufWriter::new(file);
    writer.write_all(app_state_json)?;
    writer.flush()?;
    log!("Successfully saved app state to persistent storage.");
    Ok(())
}

/// Marks that the next login for this user should skip automatic app-state restore once.
pub async fn skip_app_state_restore_once(user_id: &UserId) -> anyhow::Result<()> {
    let marker_path = persistent_state_dir(user_id).join(SKIP_APP_STATE_RESTORE_ONCE_FILE_NAME);
    if let Some(parent) = marker_path.parent() {
        tokio::fs::create_dir_all(parent).await?;
    }
    tokio::fs::write(marker_path, b"1").await?;
    Ok(())
}

/// Consumes the one-shot "skip automatic restore" marker for the given user, if present.
pub async fn take_skip_app_state_restore_once(user_id: &UserId) -> anyhow::Result<bool> {
    let marker_path = persistent_state_dir(user_id).join(SKIP_APP_STATE_RESTORE_ONCE_FILE_NAME);
    match tokio::fs::remove_file(marker_path).await {
        Ok(()) => Ok(true),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(false),
        Err(e) => Err(e.into()),
    }
}

/// Save the current state of the given window's geometry to persistent storage.
pub fn save_window_state(window_ref: WindowRef, cx: &Cx) -> anyhow::Result<()> {
    let inner_size = window_ref.get_inner_size(cx);
    let position = window_ref.get_position(cx);
    let window_geom = WindowGeomState {
        inner_size: (inner_size.x, inner_size.y),
        position: (position.x, position.y),
        is_fullscreen: window_ref.is_fullscreen(cx),
    };
    std::fs::write(
        app_data_dir().join(WINDOW_GEOM_STATE_FILE_NAME),
        serde_json::to_string(&window_geom)?,
    )?;
    log!("Successfully saved window geometry: {window_geom:?}");
    Ok(())
}

/// Loads the App state from persistent storage.
///
/// If the file doesn't exist or deserialization fails (e.g., due to incompatible format changes),
/// this function returns a default `AppState` and backs up the old file if it exists.
pub async fn load_app_state(user_id: &UserId) -> anyhow::Result<AppState> {
    let state_path = persistent_state_dir(user_id).join(LATEST_APP_STATE_FILE_NAME);
    load_app_state_from_path(&state_path).await
}

/// Production restore pipeline extracted around its resolved path so tests can
/// exercise file read, credential removal, deserialization, and migrations as
/// one effectful unit without writing to a real user's application directory.
async fn load_app_state_from_path(state_path: &std::path::Path) -> anyhow::Result<AppState> {
    let file_bytes = match tokio::fs::read(state_path).await {
        Ok(fb) => fb,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log!("No saved app state found, using default.");
            return Ok(AppState::default());
        }
        Err(e) => return Err(e.into())
    };
    let file_bytes = sanitize_persisted_agent_ops_credentials(state_path, file_bytes).await?;
    let mut app_state = deserialize_app_state_or_recover(&file_bytes, state_path);
    // Migration: upgraded users with a legacy known-bot list but no registry
    // get their bots seeded into the global AgentRegistry on load.
    app_state.seed_agent_registry_from_known_bots();
    Ok(app_state)
}

/// Removes the short-lived Agent Operations Dashboard prototype configuration.
///
/// That prototype stored a backend-wide credential under `agent_ops`. The
/// operational panel no longer supports that authentication model, so valid
/// legacy state is sanitized before it reaches `AppState` and before recovery
/// can create a backup. Unknown future `agent_ops` objects that do not contain
/// the prototype fields are left untouched.
#[derive(Debug, PartialEq, Eq)]
enum LegacyAgentOpsSanitization {
    Unchanged,
    Sanitized(Vec<u8>),
    UnreadableWithCredentials,
}

async fn sanitize_persisted_agent_ops_credentials(
    state_path: &std::path::Path,
    file_bytes: Vec<u8>,
) -> anyhow::Result<Vec<u8>> {
    match inspect_legacy_agent_ops_config(&file_bytes) {
        LegacyAgentOpsSanitization::Unchanged => Ok(file_bytes),
        LegacyAgentOpsSanitization::Sanitized(sanitized) => {
            tokio::fs::write(state_path, &sanitized)
                .await
                .with_context(|| {
                    format!(
                        "failed to remove obsolete Agent Operations credentials from {}",
                        state_path.display(),
                    )
                })?;
            log!("Removed obsolete Agent Operations credentials from persisted app state.");
            Ok(sanitized)
        }
        LegacyAgentOpsSanitization::UnreadableWithCredentials => {
            // Recovery normally preserves malformed state as a `.json.bak`.
            // A file containing the retired backend-wide credential is the one
            // exception: overwrite it before deserialization so no successful
            // restore can leave the credential in either the live file or a
            // recovery backup. If this write fails, propagate the error and do
            // not restore the account state.
            let sanitized = b"{}".to_vec();
            tokio::fs::write(state_path, &sanitized)
                .await
                .with_context(|| {
                    format!(
                        "failed to discard unreadable app state containing obsolete Agent Operations credentials at {}",
                        state_path.display(),
                    )
                })?;
            warning!(
                "Discarded unreadable app state containing obsolete Agent Operations credentials; no credential-bearing backup was created."
            );
            Ok(sanitized)
        }
    }
}

fn inspect_legacy_agent_ops_config(file_bytes: &[u8]) -> LegacyAgentOpsSanitization {
    let Ok(mut value) = serde_json::from_slice::<serde_json::Value>(file_bytes) else {
        return if contains_legacy_agent_ops_markers(file_bytes) {
            LegacyAgentOpsSanitization::UnreadableWithCredentials
        } else {
            LegacyAgentOpsSanitization::Unchanged
        };
    };
    let is_legacy = value
        .get("agent_ops")
        .and_then(serde_json::Value::as_object)
        .is_some_and(|config| {
            config.contains_key("base_url") || config.contains_key("bearer_token")
        });
    if !is_legacy {
        return LegacyAgentOpsSanitization::Unchanged;
    }
    let Some(object) = value.as_object_mut() else {
        return LegacyAgentOpsSanitization::Unchanged;
    };
    object.remove("agent_ops");
    match serde_json::to_vec(&value) {
        Ok(sanitized) => LegacyAgentOpsSanitization::Sanitized(sanitized),
        Err(_) => LegacyAgentOpsSanitization::UnreadableWithCredentials,
    }
}

fn contains_legacy_agent_ops_markers(file_bytes: &[u8]) -> bool {
    contains_bytes(file_bytes, br#""agent_ops""#)
        && (
            contains_bytes(file_bytes, br#""bearer_token""#)
                || contains_bytes(file_bytes, br#""base_url""#)
        )
}

fn contains_bytes(haystack: &[u8], needle: &[u8]) -> bool {
    haystack
        .windows(needle.len())
        .any(|window| window == needle)
}

/// Deserializes persisted app-state bytes, or — when the bytes are unreadable
/// (e.g. an incompatible format from a previous version) — backs up the file
/// and returns a default [`AppState`]. Never panics.
fn deserialize_app_state_or_recover(
    file_bytes: &[u8],
    state_path: &std::path::Path,
) -> AppState {
    match serde_json::from_slice(file_bytes) {
        Ok(app_state) => {
            log!("Successfully loaded app state from persistent storage.");
            app_state
        }
        Err(e) => {
            error!("Failed to deserialize app state: {e}. This may be due to an incompatible format from a previous version.");

            // Backup the old file to preserve user's data.
            let backup_path = state_path.with_extension("json.bak");
            if let Err(backup_err) = std::fs::rename(state_path, &backup_path) {
                error!("Failed to backup old app state file: {}", backup_err);
            } else {
                log!("Old app state backed up to: {:?}", backup_path);
            }

            log!("Using default app state. Your previous tabs and selections will be reset.");
            AppState::default()
        }
    }
}

/// Loads the window geometry's state from persistent storage.
pub fn load_window_state(window_ref: WindowRef, cx: &mut Cx) -> anyhow::Result<()> {
    let file = match std::fs::File::open(app_data_dir().join(WINDOW_GEOM_STATE_FILE_NAME)) {
        Ok(file) => file,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return Ok(()),
        Err(e) => return Err(e.into()),
    };
    let window_geom = serde_json::from_reader(file).map_err(|e| anyhow::anyhow!(e))?;
    log!("Restoring window geometry: {window_geom:?}");
    let WindowGeomState {
        inner_size,
        position,
        is_fullscreen,
    } = window_geom;
    window_ref.configure_window(
        cx,
        dvec2(inner_size.0, inner_size.1),
        dvec2(position.0, position.1),
        is_fullscreen,
        "Robrix".to_string(),
    );
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::atomic::{AtomicU64, Ordering};

    use super::*;
    use crate::app::AgentEntry;

    fn temporary_state_path(test_name: &str) -> std::path::PathBuf {
        static NEXT_ID: AtomicU64 = AtomicU64::new(0);
        std::env::temp_dir()
            .join(format!(
                "robrix-agent-ops-scrub-{}-{}-{test_name}",
                std::process::id(),
                NEXT_ID.fetch_add(1, Ordering::Relaxed),
            ))
            .join(LATEST_APP_STATE_FILE_NAME)
    }

    #[test]
    fn legacy_agent_ops_credentials_are_scrubbed_before_restore() {
        let mut stored = serde_json::to_value(AppState::default()).unwrap();
        stored["agent_ops"] = serde_json::json!({
            "base_url": "http://127.0.0.1:8084",
            "bearer_token": "legacy-prototype-secret",
        });
        let bytes = serde_json::to_vec(&stored).unwrap();

        let LegacyAgentOpsSanitization::Sanitized(sanitized) =
            inspect_legacy_agent_ops_config(&bytes)
        else {
            panic!("legacy state must be identified and sanitized");
        };
        let text = String::from_utf8(sanitized.clone()).unwrap();
        assert!(!text.contains("legacy-prototype-secret"));
        assert!(!text.contains("agent_ops"));
        serde_json::from_slice::<AppState>(&sanitized).unwrap();
    }

    #[test]
    fn future_agent_ops_state_without_legacy_fields_is_not_scrubbed() {
        let bytes = br#"{"agent_ops":{"contract":"future-version"}}"#;
        assert_eq!(
            inspect_legacy_agent_ops_config(bytes),
            LegacyAgentOpsSanitization::Unchanged,
        );
    }

    #[tokio::test]
    async fn corrupt_legacy_agent_ops_credentials_are_discarded_without_backup() {
        let state_path = temporary_state_path("corrupt-entry");
        let state_dir = state_path.parent().unwrap();
        std::fs::create_dir_all(&state_dir).unwrap();
        let corrupt = br#"{"agent_ops":{"base_url":"http://127.0.0.1:8084","bearer_token":"legacy-prototype-secret"},not-json"#;
        std::fs::write(&state_path, corrupt).unwrap();

        let restored = load_app_state_from_path(&state_path).await.unwrap();

        assert_eq!(
            serde_json::to_value(&restored).unwrap(),
            serde_json::to_value(AppState::default()).unwrap(),
        );
        let persisted = std::fs::read(&state_path).unwrap();
        assert_eq!(persisted, b"{}");
        assert!(!state_path.with_extension("json.bak").exists());
        std::fs::remove_dir_all(&state_dir).unwrap();
    }

    #[tokio::test]
    async fn legacy_agent_ops_credentials_are_removed_from_disk_before_restore() {
        let state_path = temporary_state_path("valid-entry");
        let state_dir = state_path.parent().unwrap();
        std::fs::create_dir_all(&state_dir).unwrap();
        let legacy = br#"{"agent_ops":{"base_url":"http://127.0.0.1:8084","bearer_token":"legacy-prototype-secret"},"app_prefs":{}}"#;
        std::fs::write(&state_path, legacy).unwrap();

        let restored = load_app_state_from_path(&state_path).await.unwrap();

        let persisted = std::fs::read(&state_path).unwrap();
        let persisted_state = serde_json::from_slice::<AppState>(&persisted).unwrap();
        assert_eq!(
            serde_json::to_value(&restored).unwrap(),
            serde_json::to_value(persisted_state).unwrap(),
        );
        assert!(!persisted.windows(b"legacy-prototype-secret".len())
            .any(|window| window == b"legacy-prototype-secret"));
        assert!(!persisted.windows(b"agent_ops".len())
            .any(|window| window == b"agent_ops"));
        assert!(!state_path.with_extension("json.bak").exists());
        std::fs::remove_dir_all(&state_dir).unwrap();
    }

    #[tokio::test]
    async fn credential_scrub_write_failure_aborts_restore() {
        let state_path = temporary_state_path("write-failure");
        std::fs::create_dir_all(&state_path).unwrap();
        let legacy = br#"{"agent_ops":{"bearer_token":"legacy-prototype-secret"}}"#;

        let result = sanitize_persisted_agent_ops_credentials(
            &state_path,
            legacy.to_vec(),
        )
        .await;

        assert!(result.is_err(), "restore must stop when credential removal cannot be persisted");
        std::fs::remove_dir_all(state_path.parent().unwrap()).unwrap();
    }

    #[tokio::test]
    async fn test_agent_registry_persists_per_account_no_cross_leak() {
        let alice: OwnedUserId = "@agent-registry-alice:example.org".try_into().unwrap();
        let bob: OwnedUserId = "@agent-registry-bob:example.org".try_into().unwrap();
        let alice_path = temporary_state_path("registry-alice");
        let bob_path = temporary_state_path("registry-bob");

        // Distinct accounts map to distinct storage paths, so one account's
        // saved state can never overwrite another's.
        assert_ne!(persistent_state_dir(&alice), persistent_state_dir(&bob));

        let mut alice_state = AppState::default();
        let agent: OwnedUserId = "@agent:example.org".try_into().unwrap();
        alice_state
            .agent_registry
            .register(agent.clone(), AgentEntry::default());
        let bob_state = AppState::default();

        save_app_state_bytes_to_path(
            &serialize_app_state(&alice_state).unwrap(),
            &alice_path,
        ).unwrap();
        save_app_state_bytes_to_path(
            &serialize_app_state(&bob_state).unwrap(),
            &bob_path,
        ).unwrap();

        let alice_loaded = load_app_state_from_path(&alice_path).await.unwrap();
        let bob_loaded = load_app_state_from_path(&bob_path).await.unwrap();

        assert!(alice_loaded.agent_registry.contains(agent.as_ref()));
        assert!(bob_loaded.agent_registry.is_empty());

        std::fs::remove_dir_all(alice_path.parent().unwrap()).unwrap();
        std::fs::remove_dir_all(bob_path.parent().unwrap()).unwrap();
    }

    #[tokio::test]
    async fn test_corrupt_registry_json_falls_back_to_default_state() {
        let state_path = temporary_state_path("corrupt-registry");
        let dir = state_path.parent().unwrap();
        std::fs::create_dir_all(&dir).unwrap();
        let corrupt = br#"{"agent_registry": this is not valid json"#;
        std::fs::write(&state_path, corrupt).unwrap();

        let recovered = load_app_state_from_path(&state_path).await.unwrap();

        // Falls back to a default AppState (empty registry) without panicking.
        assert_eq!(recovered.agent_registry.len(), 0);
        // The unreadable file is preserved as a backup.
        assert!(state_path.with_extension("json.bak").exists());

        std::fs::remove_dir_all(&dir).unwrap();
    }
}
