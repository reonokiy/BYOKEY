//! `OpenAI` Codex CLI JWT OAuth authorization flow configuration.
//!
//! Implements the Authorization Code + PKCE (S256) flow extracted from the
//! Codex CLI binary. Callback port: 1455.

/// Local callback port for the OAuth redirect.
pub const CALLBACK_PORT: u16 = 1455;

/// `OpenAI` OAuth authorization endpoint.
pub const AUTH_URL: &str = "https://auth.openai.com/oauth/authorize";

/// OAuth scopes requested during authorization.
pub const SCOPES: &[&str] = &["openid", "email", "profile", "offline_access"];

const REDIRECT_URI_ENCODED: &str = "http%3A%2F%2Flocalhost%3A1455%2Fauth%2Fcallback";
const REDIRECT_URI: &str = "http://localhost:1455/auth/callback";

/// Build the authorization URL with PKCE parameters.
#[must_use]
pub fn build_auth_url(client_id: &str, code_challenge: &str, state: &str) -> String {
    let scope = SCOPES.join("+");
    format!(
        "{AUTH_URL}?client_id={client_id}&code_challenge={code_challenge}&code_challenge_method=S256&codex_cli_simplified_flow=true&id_token_add_organizations=true&prompt=login&redirect_uri={REDIRECT_URI_ENCODED}&response_type=code&scope={scope}&state={state}",
    )
}

/// Build the form-urlencoded parameters for the token exchange request.
#[must_use]
pub fn token_form_params<'a>(
    client_id: &'a str,
    code: &'a str,
    code_verifier: &'a str,
) -> [(&'static str, &'a str); 5] {
    [
        ("grant_type", "authorization_code"),
        ("client_id", client_id),
        ("code", code),
        ("redirect_uri", REDIRECT_URI),
        ("code_verifier", code_verifier),
    ]
}

// ── Device Auth (OpenAI private protocol) ────────────────────────────────────
//
// Codex CLI uses a non-standard device auth flow:
//   1. POST /api/accounts/deviceauth/usercode  → { device_auth_id, user_code, interval }
//   2. User visits /codex/device and enters user_code
//   3. Poll POST /api/accounts/deviceauth/token → { authorization_code, code_verifier, ... }
//   4. Standard OAuth code exchange with the returned authorization_code + code_verifier

const ISSUER: &str = "https://auth.openai.com";
const DEVICE_USERCODE_PATH: &str = "/api/accounts/deviceauth/usercode";
const DEVICE_TOKEN_PATH: &str = "/api/accounts/deviceauth/token";
const DEVICE_VERIFY_PATH: &str = "/codex/device";
const DEVICE_CALLBACK_PATH: &str = "/deviceauth/callback";

/// Run the OpenAI device auth flow and return a token.
pub async fn device_auth(
    http: &rquest::Client,
    creds: &OAuthCredentials,
) -> Result<OAuthToken> {
    use std::time::Duration;

    // Step 1: request user code
    let body = serde_json::json!({ "client_id": creds.client_id });
    let resp = http
        .post(format!("{ISSUER}{DEVICE_USERCODE_PATH}"))
        .json(&body)
        .send()
        .await?;
    let json: serde_json::Value = resp
        .json()
        .await
        .map_err(|e| ByokError::Auth(format!("failed to parse usercode response: {e}")))?;

    let device_auth_id = json
        .get("device_auth_id")
        .and_then(|v| v.as_str())
        .ok_or_else(|| ByokError::Auth("missing device_auth_id".into()))?
        .to_string();
    let user_code = json
        .get("user_code")
        .or_else(|| json.get("usercode"))
        .and_then(|v| v.as_str())
        .ok_or_else(|| ByokError::Auth("missing user_code".into()))?
        .to_string();
    // Codex API returns interval as a string; fall back to number or default.
    let interval: u64 = json
        .get("interval")
        .and_then(|v| {
            v.as_str()
                .and_then(|s| s.parse().ok())
                .or_else(|| v.as_u64())
        })
        .unwrap_or(5);

    // Step 2: show user code
    let verify_url = format!("{ISSUER}{DEVICE_VERIFY_PATH}");
    println!("\nVisit this URL and enter the code to log in:\n");
    println!("  URL:  {verify_url}");
    println!("  Code: {user_code}\n");

    // Step 3: poll for authorization_code
    let deadline = tokio::time::Instant::now() + Duration::from_secs(900); // 15 min
    let poll_url = format!("{ISSUER}{DEVICE_TOKEN_PATH}");
    let poll_body = serde_json::json!({
        "device_auth_id": device_auth_id,
        "user_code": user_code,
    });
    let (authorization_code, code_verifier) = loop {
        tokio::time::sleep(Duration::from_secs(interval)).await;
        if tokio::time::Instant::now() >= deadline {
            return Err(ByokError::Auth("device auth timed out after 15 minutes".into()));
        }

        let resp = http
            .post(&poll_url)
            .json(&poll_body)
            .send()
            .await?;

        let status = resp.status();
        if status.as_u16() == 403 || status.as_u16() == 404 {
            // Still pending
            continue;
        }
        if !status.is_success() {
            return Err(ByokError::Auth(format!(
                "device auth failed with status {status}"
            )));
        }

        let json: serde_json::Value = resp
            .json()
            .await
            .map_err(|e| ByokError::Auth(format!("failed to parse device token response: {e}")))?;
        let code = json
            .get("authorization_code")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ByokError::Auth("missing authorization_code".into()))?
            .to_string();
        let verifier = json
            .get("code_verifier")
            .and_then(|v| v.as_str())
            .ok_or_else(|| ByokError::Auth("missing code_verifier".into()))?
            .to_string();
        break (code, verifier);
    };

    // Step 4: exchange authorization_code for tokens
    const DEFAULT_TOKEN_URL: &str = "https://auth.openai.com/oauth/token";
    let token_url = creds.token_url.as_deref().unwrap_or(DEFAULT_TOKEN_URL);
    let redirect_uri = format!("{ISSUER}{DEVICE_CALLBACK_PATH}");
    let params = [
        ("grant_type", "authorization_code"),
        ("client_id", creds.client_id.as_str()),
        ("code", authorization_code.as_str()),
        ("redirect_uri", redirect_uri.as_str()),
        ("code_verifier", code_verifier.as_str()),
    ];
    let resp = http
        .post(token_url)
        .header("Content-Type", "application/x-www-form-urlencoded")
        .form(&params)
        .send()
        .await?;
    auth_code::send_and_parse_token(resp).await
}

// ── AuthCodeFlow implementation ───────────────────────────────────────────────

use async_trait::async_trait;
use byokey_types::{ByokError, OAuthToken, ProviderId, Result};

use crate::credentials::OAuthCredentials;
use crate::flow::auth_code::{self, AuthCodeFlow};

/// Codex auth-code provider.
pub struct Codex;

#[async_trait]
impl AuthCodeFlow for Codex {
    fn provider_id(&self) -> ProviderId {
        ProviderId::Codex
    }
    fn provider_name(&self) -> &'static str {
        "codex"
    }
    fn callback_port(&self) -> u16 {
        CALLBACK_PORT
    }

    fn build_auth_url(&self, client_id: &str, challenge: &str, state: &str) -> String {
        build_auth_url(client_id, challenge, state)
    }

    async fn exchange_code(
        &self,
        http: &rquest::Client,
        creds: &OAuthCredentials,
        code: &str,
        verifier: &str,
        _state: &str,
    ) -> Result<OAuthToken> {
        let token_url = creds
            .token_url
            .as_deref()
            .ok_or_else(|| ByokError::Auth("codex credentials missing token_url".into()))?;
        let params = token_form_params(&creds.client_id, code, verifier);
        let resp = http
            .post(token_url)
            .header("Accept", "application/json")
            .form(&params)
            .send()
            .await?;
        auth_code::send_and_parse_token(resp).await
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const TEST_CLIENT_ID: &str = "test-codex-client-id";

    #[test]
    fn test_auth_url_contains_client_id() {
        let url = build_auth_url(TEST_CLIENT_ID, "mychallenge", "mystate");
        assert!(url.contains(TEST_CLIENT_ID));
        assert!(url.contains("mychallenge"));
        assert!(url.contains("mystate"));
        assert!(url.contains(&CALLBACK_PORT.to_string()));
        assert!(url.contains("codex_cli_simplified_flow=true"));
        assert!(url.contains("S256"));
    }
}
