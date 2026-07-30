//! Token caching for VoIP authentication (OpenID and LiveKit JWT)
//!
//! This module provides caching mechanisms for OpenID tokens and LiveKit JWTs
//! to avoid unnecessary network requests. Tokens are cached with their expiration
//! times and validated before use.

use serde::{Deserialize, Serialize};
use matrix_sdk::ruma::OwnedRoomId;

/// Cached OpenID token with fetch timestamp for reuse.
/// This structure is serializable for persistence across app restarts.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedOpenIdToken {
    pub access_token: String,
    pub token_type: String,
    pub matrix_server_name: String,
    /// Unix timestamp (seconds) when the token was fetched
    pub fetched_at: u64,
    /// Expiry duration in seconds (copied from response for convenience)
    pub expires_in: u64,
}

impl CachedOpenIdToken {
    /// Create a new cached token with the current timestamp
    pub fn new(
        access_token: String,
        token_type: String,
        matrix_server_name: String,
        expires_in: u64,
    ) -> Self {
        let fetched_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);
        Self {
            access_token,
            token_type,
            matrix_server_name,
            fetched_at,
            expires_in,
        }
    }

    /// Check if the token is still valid with a safety margin (60 seconds)
    pub fn is_valid(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Token is valid if current time is less than (fetched_at + expires_in - margin)
        // Use a 60-second safety margin to avoid edge cases
        let safety_margin = 60;
        let expiry_time = self.fetched_at.saturating_add(self.expires_in);

        now < expiry_time.saturating_sub(safety_margin)
    }

    /// Get the remaining validity time in seconds
    pub fn remaining_seconds(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        let expiry_time = self.fetched_at.saturating_add(self.expires_in);
        expiry_time.saturating_sub(now)
    }
}

/// Cached LiveKit JWT with fetch timestamp for reuse.
/// JWT tokens typically expire after some time, we cache them to avoid unnecessary requests.
#[derive(Clone, Debug, Serialize, Deserialize)]
pub struct CachedLiveKitJwt {
    pub jwt: String,
    pub url: String,
    /// The room ID this JWT is valid for
    pub room_id: OwnedRoomId,
    /// Unix timestamp (seconds) when the JWT was fetched
    pub fetched_at: u64,
    /// Expiration timestamp extracted from JWT (seconds since epoch)
    pub expires_at: u64,
}

impl CachedLiveKitJwt {
    /// Create a new cached JWT with the current timestamp.
    /// Attempts to extract expiration from JWT payload, falls back to 1 hour default.
    pub fn new(jwt: String, url: String, room_id: OwnedRoomId) -> Self {
        let fetched_at = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Try to extract expiration from JWT
        let expires_at = Self::extract_jwt_expiration(&jwt)
            .unwrap_or(fetched_at + 3600); // Default to 1 hour if extraction fails

        Self {
            jwt,
            url,
            room_id,
            fetched_at,
            expires_at,
        }
    }

    /// Extract expiration timestamp from JWT payload.
    /// JWT format: header.payload.signature (all base64 encoded)
    fn extract_jwt_expiration(jwt: &str) -> Option<u64> {
        let parts: Vec<&str> = jwt.split('.').collect();
        if parts.len() != 3 {
            return None;
        }

        // Decode payload (second part) - JWT uses base64url encoding
        let payload_b64 = parts[1];
        // Add padding if needed
        let padding = (4 - payload_b64.len() % 4) % 4;
        let padded = format!("{}{}", payload_b64, "=".repeat(padding));
        // Replace URL-safe chars
        let standard_b64: String = padded
            .chars()
            .map(|c| match c {
                '-' => '+',
                '_' => '/',
                c => c,
            })
            .collect();

        // Decode base64
        let decoded = base64_decode(&standard_b64)?;
        let payload_str = String::from_utf8(decoded).ok()?;

        // Parse JSON and extract "exp" field
        // Simple parsing without full JSON library
        if let Some(exp_start) = payload_str.find("\"exp\":") {
            let after_exp = &payload_str[exp_start + 6..];
            let exp_str: String = after_exp
                .chars()
                .take_while(|c| c.is_ascii_digit())
                .collect();
            return exp_str.parse().ok();
        }

        None
    }

    /// Check if the JWT is still valid with a safety margin (60 seconds)
    pub fn is_valid(&self) -> bool {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        // Token is valid if current time is less than (expires_at - margin)
        let safety_margin = 60;
        now < self.expires_at.saturating_sub(safety_margin)
    }

    /// Check if this JWT is valid for the given room
    pub fn is_valid_for_room(&self, room_id: &OwnedRoomId) -> bool {
        self.is_valid() && &self.room_id == room_id
    }

    /// Get the remaining validity time in seconds
    pub fn remaining_seconds(&self) -> u64 {
        let now = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_secs())
            .unwrap_or(0);

        self.expires_at.saturating_sub(now)
    }
}

/// Simple base64 decoder (no external dependency)
fn base64_decode(input: &str) -> Option<Vec<u8>> {
    const CHARS: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";

    fn char_to_val(c: u8) -> Option<u8> {
        CHARS.iter().position(|&x| x == c).map(|p| p as u8)
    }

    let input: Vec<u8> = input.bytes().filter(|&b| b != b'=').collect();
    let mut output = Vec::new();

    for chunk in input.chunks(4) {
        let vals: Vec<u8> = chunk.iter().filter_map(|&c| char_to_val(c)).collect();
        if vals.len() < 2 {
            continue;
        }

        output.push((vals[0] << 2) | (vals[1] >> 4));
        if vals.len() > 2 {
            output.push((vals[1] << 4) | (vals[2] >> 2));
        }
        if vals.len() > 3 {
            output.push((vals[2] << 6) | vals[3]);
        }
    }

    Some(output)
}

/// Persistent VoIP token state, stored in AppState for persistence across app restarts.
#[derive(Clone, Debug, Default, Serialize, Deserialize)]
pub struct VoipTokenState {
    /// Cached OpenID token (valid for any room, tied to user session)
    pub cached_openid_token: Option<CachedOpenIdToken>,
    /// Cached LiveKit JWTs (per-room, since JWTs are room-specific)
    pub cached_livekit_jwts: Vec<CachedLiveKitJwt>,
}

impl VoipTokenState {
    /// Get a valid cached LiveKit JWT for the given room, if available
    pub fn get_valid_jwt_for_room(&self, room_id: &OwnedRoomId) -> Option<&CachedLiveKitJwt> {
        self.cached_livekit_jwts
            .iter()
            .find(|jwt| jwt.is_valid_for_room(room_id))
    }

    /// Store a new LiveKit JWT, replacing any existing one for the same room
    pub fn store_jwt(&mut self, jwt: CachedLiveKitJwt) {
        // Remove any existing JWT for this room
        self.cached_livekit_jwts
            .retain(|j| j.room_id != jwt.room_id);
        // Add the new JWT
        self.cached_livekit_jwts.push(jwt);
        // Clean up expired JWTs
        self.cleanup_expired();
    }

    /// Store a new OpenID token
    pub fn store_openid_token(&mut self, token: CachedOpenIdToken) {
        self.cached_openid_token = Some(token);
    }

    /// Get a valid cached OpenID token, if available
    pub fn get_valid_openid_token(&self) -> Option<&CachedOpenIdToken> {
        self.cached_openid_token
            .as_ref()
            .filter(|t| t.is_valid())
    }

    /// Clean up expired tokens
    pub fn cleanup_expired(&mut self) {
        // Remove expired JWTs
        self.cached_livekit_jwts.retain(|jwt| jwt.is_valid());
        // Clear expired OpenID token
        if let Some(ref token) = self.cached_openid_token {
            if !token.is_valid() {
                self.cached_openid_token = None;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_openid_token_validity() {
        let token = CachedOpenIdToken::new(
            "test_token".to_string(),
            "Bearer".to_string(),
            "matrix.org".to_string(),
            3600, // 1 hour
        );
        assert!(token.is_valid());
        assert!(token.remaining_seconds() > 3500);
    }

    #[test]
    fn test_jwt_expiration_extraction() {
        // This is a test JWT with exp claim
        // Header: {"alg":"HS256","typ":"JWT"}
        // Payload: {"exp":9999999999,"sub":"test"}
        // Note: This is not a real JWT, just for testing the extraction
        let test_jwt = "eyJhbGciOiJIUzI1NiIsInR5cCI6IkpXVCJ9.eyJleHAiOjk5OTk5OTk5OTksInN1YiI6InRlc3QifQ.signature";
        let exp = CachedLiveKitJwt::extract_jwt_expiration(test_jwt);
        assert_eq!(exp, Some(9999999999));
    }
}
