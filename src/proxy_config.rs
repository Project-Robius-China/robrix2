use std::{io::ErrorKind, path::{Path, PathBuf}, sync::{Mutex, OnceLock}, time::Duration};

use makepad_widgets::{error, warning};
use matrix_sdk::reqwest::{Client, ClientBuilder, NoProxy, Proxy, tls};
use robius_proxy::ProxyConfig;
use serde::{Deserialize, Serialize};
use url::Url;

const POLICY_USER_AGENT: &str = concat!(
    "Robrix/", env!("CARGO_PKG_VERSION"), " (matrix-rust-sdk)"
);

use crate::app_data_dir;


const PROXY_STATE_FILE_NAME: &str = "proxy_state.json";
pub const DEFAULT_NO_PROXY_BYPASS: &[&str] = &[
    "localhost",
    "127.0.0.1",
    "::1",
];

static PROXY_ENV_LOCK: Mutex<()> = Mutex::new(());

// Holds the CLI `--proxy` value parsed once at startup so every code path
// (restore_session, downloads, SSO pre-build) can resolve the same override
// without re-parsing argv or threading the value through deep call chains.
static CLI_PROXY_OVERRIDE: OnceLock<Option<String>> = OnceLock::new();

pub fn set_cli_proxy_override(proxy_url: Option<&str>) {
    let _ = CLI_PROXY_OVERRIDE.set(normalize_proxy_url(proxy_url));
}

pub fn cli_proxy_override() -> Option<String> {
    CLI_PROXY_OVERRIDE.get().cloned().flatten()
}

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
        "http" | "https" => Ok(()),
        scheme => Err(format!(
            "Unsupported proxy URL scheme `{scheme}`. Use http or https."
        )),
    }
}

pub fn load_saved_proxy_url() -> Option<String> {
    load_saved_proxy_url_from_path(&proxy_state_file_path())
}

fn load_saved_proxy_url_from_path(state_path: &Path) -> Option<String> {
    let proxy_state_bytes = match std::fs::read(state_path) {
        Ok(bytes) => bytes,
        Err(e) if e.kind() == ErrorKind::NotFound => return None,
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
        .or_else(cli_proxy_override)
        .or_else(load_saved_proxy_url)
}

pub fn effective_proxy_url_from_saved_policy() -> Result<Option<String>, String> {
    let saved_proxy = load_saved_proxy_url();
    apply_proxy_to_process_env(saved_proxy.as_deref())?;
    Ok(saved_proxy)
}

pub fn save_proxy_url(proxy_url: Option<&str>) -> Result<Option<String>, String> {
    save_proxy_url_to_path(proxy_url, &proxy_state_file_path())
}

fn save_proxy_url_to_path(proxy_url: Option<&str>, state_path: &Path) -> Result<Option<String>, String> {
    let normalized_proxy_url = normalize_proxy_url(proxy_url);
    if let Some(proxy_url) = normalized_proxy_url.as_ref() {
        validate_proxy_url(proxy_url)?;
    }

    if let Some(parent_dir) = state_path.parent() {
        std::fs::create_dir_all(parent_dir)
            .map_err(|e| format!("Failed to create proxy state directory: {e}"))?;
    }

    let proxy_state = ProxyState {
        proxy_url: normalized_proxy_url.clone(),
    };
    let serialized_proxy_state = serde_json::to_vec(&proxy_state)
        .map_err(|e| format!("Failed to serialize proxy state: {e}"))?;

    // Hold the env lock across the file write + env apply so two concurrent
    // saves can never leave file and env disagreeing.
    let _env_guard = lock_proxy_env();
    std::fs::write(state_path, serialized_proxy_state)
        .map_err(|e| format!("Failed to write proxy state file {}: {e}", state_path.display()))?;
    apply_proxy_to_env_locked(normalized_proxy_url.as_deref())?;

    Ok(normalized_proxy_url)
}

fn build_env_proxy_config(proxy_url: &str) -> ProxyConfig {
    ProxyConfig::new()
        .direct(false)
        .manual_all(proxy_url)
        .manual_http(proxy_url)
        .manual_https(proxy_url)
        .bypass(DEFAULT_NO_PROXY_BYPASS.iter().copied())
}

fn lock_proxy_env() -> std::sync::MutexGuard<'static, ()> {
    // Recover from a prior panic-while-held: the env vars may be in an unknown
    // state, but every caller of this lock is about to overwrite them anyway.
    match PROXY_ENV_LOCK.lock() {
        Ok(guard) => guard,
        Err(poisoned) => {
            PROXY_ENV_LOCK.clear_poison();
            poisoned.into_inner()
        }
    }
}

fn apply_proxy_to_env_locked(proxy_url: Option<&str>) -> Result<(), String> {
    let normalized_proxy_url = normalize_proxy_url(proxy_url);
    match normalized_proxy_url.as_deref() {
        Some(proxy_url) => {
            validate_proxy_url(proxy_url)?;
            build_env_proxy_config(proxy_url)
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

pub fn apply_proxy_to_process_env(proxy_url: Option<&str>) -> Result<(), String> {
    let _env_guard = lock_proxy_env();
    apply_proxy_to_env_locked(proxy_url)
}

pub fn load_and_apply_saved_proxy_to_process_env() -> Option<String> {
    match effective_proxy_url_from_saved_policy() {
        Ok(saved_proxy) => saved_proxy,
        Err(e) => {
            error!("Failed to apply saved proxy to process env: {e}");
            load_saved_proxy_url()
        }
    }
}

pub fn build_reqwest_proxy(
    proxy_url: &str,
) -> anyhow::Result<Proxy> {
    validate_proxy_url(proxy_url)
        .map_err(|e| anyhow::anyhow!(e))?;
    let no_proxy = NoProxy::from_string(&DEFAULT_NO_PROXY_BYPASS.join(","));
    Ok(Proxy::all(proxy_url)?.no_proxy(no_proxy))
}

pub fn apply_policy_to_reqwest_builder(
    builder: ClientBuilder,
    proxy_url: Option<&str>,
) -> anyhow::Result<ClientBuilder> {
    match normalize_proxy_url(proxy_url) {
        Some(proxy_url) => Ok(builder.proxy(build_reqwest_proxy(&proxy_url)?)),
        None => Ok(builder.no_proxy()),
    }
}

pub fn build_policy_reqwest_client(
    proxy_url: Option<&str>,
    timeout: Option<Duration>,
) -> anyhow::Result<Client> {
    // Restore the security/operational defaults that matrix_sdk's HttpSettings
    // used to enforce before we switched ClientBuilder.proxy() → .http_client().
    let mut builder = Client::builder()
        .user_agent(POLICY_USER_AGENT)
        .min_tls_version(tls::Version::TLS_1_2);
    if let Some(timeout) = timeout {
        builder = builder.timeout(timeout);
    }
    let builder = apply_policy_to_reqwest_builder(builder, proxy_url)?;
    Ok(builder.build()?)
}

#[cfg(test)]
mod tests {
    use std::sync::Mutex;

    use super::*;

    const PROXY_ENV_VARS: &[&str] = &[
        "http_proxy",
        "HTTP_PROXY",
        "https_proxy",
        "HTTPS_PROXY",
        "all_proxy",
        "ALL_PROXY",
        "ftp_proxy",
        "FTP_PROXY",
        "no_proxy",
        "NO_PROXY",
    ];

    static TEST_PROXY_ENV_LOCK: Mutex<()> = Mutex::new(());

    fn with_proxy_env_lock(test: impl FnOnce()) {
        let _guard = TEST_PROXY_ENV_LOCK.lock().unwrap();
        let snapshot: Vec<(&str, Option<String>)> = PROXY_ENV_VARS
            .iter()
            .map(|key| (*key, std::env::var(key).ok()))
            .collect();

        clear_proxy_env();
        test();
        clear_proxy_env();

        for (key, value) in snapshot {
            if let Some(value) = value {
                set_env_var(key, &value);
            }
        }
    }

    fn clear_proxy_env() {
        for key in PROXY_ENV_VARS {
            remove_env_var(key);
        }
    }

    fn remove_env_var(key: &str) {
        // Tests serialize environment mutation with TEST_PROXY_ENV_LOCK.
        unsafe { std::env::remove_var(key); }
    }

    fn set_env_var(key: &str, value: &str) {
        // Tests serialize environment mutation with TEST_PROXY_ENV_LOCK.
        unsafe { std::env::set_var(key, value); }
    }

    fn proxy_state_test_path(name: &str) -> PathBuf {
        std::env::temp_dir()
            .join(format!("robrix_proxy_policy_test_{}_{}", name, std::process::id()))
            .join(PROXY_STATE_FILE_NAME)
    }

    fn assert_proxy_env_cleared() {
        for key in PROXY_ENV_VARS {
            assert!(
                std::env::var(key).is_err(),
                "{key} should be cleared but was {:?}",
                std::env::var(key).ok()
            );
        }
    }

    fn set_all_proxy_env(value: &str) {
        for key in PROXY_ENV_VARS {
            set_env_var(key, value);
        }
    }

    #[test]
    fn proxy_state_none_clears_inherited_env_proxy_vars() {
        with_proxy_env_lock(|| {
            set_all_proxy_env("http://127.0.0.1:9999");

            apply_proxy_to_process_env(None).unwrap();

            assert_proxy_env_cleared();
        });
    }

    #[test]
    fn save_proxy_url_none_clears_env_proxy_vars() {
        with_proxy_env_lock(|| {
            set_all_proxy_env("http://127.0.0.1:9999");
            let state_path = proxy_state_test_path("none");

            let saved = save_proxy_url_to_path(None, &state_path).unwrap();

            assert_eq!(saved, None);
            assert_eq!(load_saved_proxy_url_from_path(&state_path), None);
            assert_proxy_env_cleared();
            let _ = std::fs::remove_file(state_path);
        });
    }

    #[test]
    fn save_proxy_url_some_sets_proxy_env_and_bypass_rules() {
        with_proxy_env_lock(|| {
            let proxy = "http://127.0.0.1:7890";
            let state_path = proxy_state_test_path("some");

            let saved = save_proxy_url_to_path(Some(proxy), &state_path).unwrap();

            assert_eq!(saved.as_deref(), Some(proxy));
            assert_eq!(load_saved_proxy_url_from_path(&state_path).as_deref(), Some(proxy));
            assert_eq!(std::env::var("http_proxy").as_deref(), Ok(proxy));
            assert_eq!(std::env::var("https_proxy").as_deref(), Ok(proxy));
            assert_eq!(std::env::var("all_proxy").as_deref(), Ok(proxy));

            let no_proxy = std::env::var("NO_PROXY")
                .or_else(|_| std::env::var("no_proxy"))
                .unwrap();
            for expected in DEFAULT_NO_PROXY_BYPASS {
                assert!(
                    no_proxy.split(',').any(|value| value == *expected),
                    "NO_PROXY {no_proxy:?} should include {expected}"
                );
            }
            let _ = std::fs::remove_file(state_path);
        });
    }

    #[test]
    fn build_policy_reqwest_client_disables_system_proxy_when_proxy_is_none() {
        with_proxy_env_lock(|| {
            set_env_var("http_proxy", "http://127.0.0.1:9999");

            let client = build_policy_reqwest_client(None, None).unwrap();

            drop(client);
        });
    }

    #[test]
    fn build_policy_reqwest_client_attaches_no_proxy_bypass_for_local_addresses() {
        with_proxy_env_lock(|| {
            let proxy = build_reqwest_proxy("http://127.0.0.1:7890").unwrap();
            let proxy_debug = format!("{proxy:?}");

            for expected in DEFAULT_NO_PROXY_BYPASS {
                assert!(
                    proxy_debug.contains(expected),
                    "proxy debug {proxy_debug:?} should include bypass {expected}"
                );
            }
            for unexpected in ["192.168.0.0/16", "10.0.0.0/8", "172.16.0.0/12", "192.168.1.58"] {
                assert!(
                    !proxy_debug.contains(unexpected),
                    "proxy debug {proxy_debug:?} should not include implicit bypass {unexpected}"
                );
            }
        });
    }

    #[test]
    fn policy_user_agent_carries_robrix_identity_and_sdk_family() {
        assert!(
            POLICY_USER_AGENT.starts_with("Robrix/"),
            "expected UA to identify Robrix, got {POLICY_USER_AGENT:?}"
        );
        assert!(
            POLICY_USER_AGENT.contains("matrix-rust-sdk"),
            "expected UA to mark the SDK family for homeserver tooling, got {POLICY_USER_AGENT:?}"
        );
    }

    #[test]
    fn validate_proxy_url_rejects_socks_schemes() {
        for unsupported in ["socks5://127.0.0.1:1080", "socks5h://127.0.0.1:1080", "socks4://127.0.0.1:1080"] {
            let err = validate_proxy_url(unsupported)
                .expect_err("socks schemes should be rejected until reqwest is built with the socks feature");
            assert!(
                err.contains("Unsupported proxy URL scheme"),
                "expected scheme-rejection message, got {err:?}"
            );
        }
    }

    #[test]
    fn apply_proxy_to_process_env_recovers_from_poisoned_lock() {
        with_proxy_env_lock(|| {
            // Simulate a prior panic-while-held by poisoning the lock manually.
            let _ = std::panic::catch_unwind(|| {
                let _guard = PROXY_ENV_LOCK.lock().unwrap();
                panic!("poisoning PROXY_ENV_LOCK on purpose");
            });
            assert!(PROXY_ENV_LOCK.is_poisoned(), "lock should be poisoned for the test setup");

            // The next apply call must transparently recover and succeed.
            apply_proxy_to_process_env(Some("http://127.0.0.1:7890")).unwrap();
            assert_eq!(std::env::var("http_proxy").as_deref(), Ok("http://127.0.0.1:7890"));
            assert!(!PROXY_ENV_LOCK.is_poisoned(), "lock should be cleared after recovery");
        });
    }
}
