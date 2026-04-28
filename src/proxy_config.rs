use std::path::PathBuf;

use makepad_widgets::{error, warning};
use robius_proxy::ProxyConfig;
use serde::{Deserialize, Serialize};
use url::Url;

use crate::app_data_dir;


const PROXY_STATE_FILE_NAME: &str = "proxy_state.json";

#[derive(Clone, Debug, Default, Serialize, Deserialize)]
#[serde(default)]
struct ProxyState {
    proxy_url: Option<String>,
}

fn proxy_state_file_path() -> PathBuf {
    app_data_dir().join(PROXY_STATE_FILE_NAME)
}

pub fn normalize_proxy_url(proxy_url: Option<&str>) -> Option<String> {
    proxy_url
        .map(str::trim)
        .filter(|value| !value.is_empty())
        .map(ToOwned::to_owned)
}

pub fn validate_proxy_url(proxy_url: &str) -> Result<(), String> {
    let proxy_url = proxy_url.trim();
    if proxy_url.is_empty() {
        return Ok(());
    }

    let parsed_url = Url::parse(proxy_url)
        .map_err(|e| format!("Invalid proxy URL: {e}"))?;

    match parsed_url.scheme() {
        "http" | "https" | "socks5" | "socks5h" => Ok(()),
        scheme => Err(format!(
            "Unsupported proxy URL scheme `{scheme}`. Use http, https, socks5, or socks5h."
        )),
    }
}

pub fn load_saved_proxy_url() -> Option<String> {
    let proxy_state_bytes = match std::fs::read(proxy_state_file_path()) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => return None,
        Err(e) => {
            warning!("Failed to read proxy state file: {e}");
            return None;
        }
    };

    let proxy_state: ProxyState = match serde_json::from_slice(&proxy_state_bytes) {
        Ok(state) => state,
        Err(e) => {
            warning!("Failed to parse proxy state file: {e}");
            return None;
        }
    };

    normalize_proxy_url(proxy_state.proxy_url.as_deref())
}

pub fn resolve_effective_proxy_url(proxy_override: Option<&str>) -> Option<String> {
    normalize_proxy_url(proxy_override)
        .or_else(load_saved_proxy_url)
}

pub fn save_proxy_url(proxy_url: Option<&str>) -> Result<Option<String>, String> {
    let normalized_proxy_url = normalize_proxy_url(proxy_url);
    if let Some(proxy_url) = normalized_proxy_url.as_ref() {
        validate_proxy_url(proxy_url)?;
    }

    let state_path = proxy_state_file_path();
    if let Some(parent_dir) = state_path.parent() {
        std::fs::create_dir_all(parent_dir)
            .map_err(|e| format!("Failed to create proxy state directory: {e}"))?;
    }

    let proxy_state = ProxyState {
        proxy_url: normalized_proxy_url.clone(),
    };
    let serialized_proxy_state = serde_json::to_vec(&proxy_state)
        .map_err(|e| format!("Failed to serialize proxy state: {e}"))?;
    std::fs::write(&state_path, serialized_proxy_state)
        .map_err(|e| format!("Failed to write proxy state file {}: {e}", state_path.display()))?;

    apply_proxy_to_process_env(normalized_proxy_url.as_deref())?;

    Ok(normalized_proxy_url)
}

fn build_env_proxy_config(proxy_url: &str) -> ProxyConfig {
    let mut config = ProxyConfig::new()
        .direct(false)
        .manual_all(proxy_url)
        .manual_http(proxy_url)
        .manual_https(proxy_url)
        .bypass(["localhost", "127.0.0.1", "::1"]);

    if proxy_url.to_ascii_lowercase().starts_with("socks") {
        config = config.manual_socks(proxy_url);
    }

    config
}

pub fn apply_proxy_to_process_env(proxy_url: Option<&str>) -> Result<(), String> {
    match normalize_proxy_url(proxy_url) {
        Some(proxy_url) => {
            validate_proxy_url(&proxy_url)?;
            build_env_proxy_config(&proxy_url)
                .apply_to_env()
                .map_err(|e| format!("Failed to apply proxy to process env: {e:?}"))?;
        }
        None => {
            ProxyConfig::clear_env()
                .map_err(|e| format!("Failed to clear proxy env vars: {e:?}"))?;
        }
    }

    Ok(())
}

pub fn load_and_apply_saved_proxy_to_process_env() -> Option<String> {
    let saved_proxy = load_saved_proxy_url();
    if let Some(proxy_url) = saved_proxy.as_deref() {
        if let Err(e) = apply_proxy_to_process_env(Some(proxy_url)) {
            error!("Failed to apply saved proxy to process env: {e}");
        }
    }
    saved_proxy
}
