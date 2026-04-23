//! OIDC (MAS) login orchestration for existing Matrix accounts.
//!
//! This module owns the server-side OAuth 2.0 authorization-code flow that
//! lets robrix2 sign users into MAS-delegated homeservers (matrix.org,
//! alvin.meldry.com, any MSC2965 server). The UI-facing entry point is
//! `MatrixRequest::StartOidcLogin` in the Matrix worker; this module provides
//! the shared error taxonomy + user-facing message mapping that the worker
//! consumes.
//!
//! Error variants are added in lock-step with code paths that actually
//! construct them — we intentionally avoid "future-proofing" enums with
//! unconstructed arms (see feedback_no_allow_dead_code in agent memory).

/// Every terminal state an OIDC login can reach.
///
/// Variants are added as construction sites land. Today only the two states
/// reachable from the stub handler (and their tests) are defined.
#[derive(Debug)]
pub enum OidcLoginError {
    /// The flow was aborted before the homeserver issued tokens. Possible
    /// triggers: in-app Cancel button, browser `error=access_denied`,
    /// 3-minute total timeout. All collapse to the same user message
    /// because the corrective action is identical — click Continue again.
    Cancelled,

    /// Dynamic client registration (MSC2966) is unsupported by this server.
    /// Separated from generic errors so the message can tell the user the
    /// failure is server-side and won't be fixed by retrying.
    DynamicRegistrationNotSupported,
}

/// Translate an `OidcLoginError` into a sentence safe to show in the UI.
///
/// Pattern mirrors `map_register_error()` in sliding_sync.rs: technical
/// details go to `log!` / `error!` at the construction site; this function
/// is deliberately terse and friendly.
pub fn map_oidc_error(err: &OidcLoginError) -> String {
    match err {
        OidcLoginError::Cancelled => {
            "Sign-in was cancelled.".to_string()
        }
        OidcLoginError::DynamicRegistrationNotSupported => {
            "This server doesn't support third-party sign-in apps yet.".to_string()
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn map_oidc_error_handles_cancelled_login() {
        let msg = map_oidc_error(&OidcLoginError::Cancelled);
        assert!(msg.to_lowercase().contains("cancel"), "got: {msg}");
    }

    #[test]
    fn map_oidc_error_handles_missing_dynamic_registration() {
        let msg = map_oidc_error(&OidcLoginError::DynamicRegistrationNotSupported);
        assert!(msg.contains("third-party sign-in apps"), "got: {msg}");
    }
}
