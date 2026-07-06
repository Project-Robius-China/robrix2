use std::io::Write;

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
    let state_dir = persistent_state_dir(user_id);
    std::fs::create_dir_all(&state_dir)?;
    let file = std::fs::File::create(state_dir.join(LATEST_APP_STATE_FILE_NAME))?;
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
    let file_bytes = match tokio::fs::read(&state_path).await {
        Ok(fb) => fb,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
            log!("No saved app state found, using default.");
            return Ok(AppState::default());
        }
        Err(e) => return Err(e.into())
    };
    let mut app_state = deserialize_app_state_or_recover(&file_bytes, &state_path);
    // Migration: upgraded users with a legacy known-bot list but no registry
    // get their bots seeded into the global AgentRegistry on load.
    app_state.seed_agent_registry_from_known_bots();
    Ok(app_state)
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
    use super::*;
    use crate::app::AgentEntry;

    #[tokio::test]
    async fn test_agent_registry_persists_per_account_no_cross_leak() {
        let alice: OwnedUserId = "@agent-registry-alice:example.org".try_into().unwrap();
        let bob: OwnedUserId = "@agent-registry-bob:example.org".try_into().unwrap();
        let _ = std::fs::remove_dir_all(persistent_state_dir(alice.as_ref()));
        let _ = std::fs::remove_dir_all(persistent_state_dir(bob.as_ref()));

        // Distinct accounts map to distinct storage paths, so one account's
        // saved state can never overwrite another's.
        assert_ne!(persistent_state_dir(&alice), persistent_state_dir(&bob));

        let mut alice_state = AppState::default();
        let agent: OwnedUserId = "@agent:example.org".try_into().unwrap();
        alice_state
            .agent_registry
            .register(agent.clone(), AgentEntry::default());
        let bob_state = AppState::default();

        save_app_state(alice_state, alice.clone()).unwrap();
        save_app_state(bob_state, bob.clone()).unwrap();

        let alice_loaded = load_app_state(alice.as_ref()).await.unwrap();
        let bob_loaded = load_app_state(bob.as_ref()).await.unwrap();

        assert!(alice_loaded.agent_registry.contains(agent.as_ref()));
        assert!(bob_loaded.agent_registry.is_empty());

        let _ = std::fs::remove_dir_all(persistent_state_dir(alice.as_ref()));
        let _ = std::fs::remove_dir_all(persistent_state_dir(bob.as_ref()));
    }

    #[tokio::test]
    async fn test_corrupt_registry_json_falls_back_to_default_state() {
        let user_id: OwnedUserId = "@agent-registry-corrupt:example.org".try_into().unwrap();
        let dir = persistent_state_dir(user_id.as_ref());
        let _ = std::fs::remove_dir_all(&dir);
        std::fs::create_dir_all(&dir).unwrap();
        let state_path = dir.join(LATEST_APP_STATE_FILE_NAME);
        let corrupt = br#"{"agent_registry": this is not valid json"#;
        std::fs::write(&state_path, corrupt).unwrap();

        let recovered = load_app_state(user_id.as_ref()).await.unwrap();

        // Falls back to a default AppState (empty registry) without panicking.
        assert_eq!(recovered.agent_registry.len(), 0);
        // The unreadable file is preserved as a backup.
        assert!(state_path.with_extension("json.bak").exists());

        let _ = std::fs::remove_dir_all(&dir);
    }
}
