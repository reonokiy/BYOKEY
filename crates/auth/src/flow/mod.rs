//! Interactive login flow dispatcher for all supported providers.
//!
//! Delegates to [`auth_code::run`] or [`device_code::run`] via the
//! [`AuthCodeFlow`](auth_code::AuthCodeFlow) and
//! [`DeviceCodeFlow`](device_code::DeviceCodeFlow) traits.

pub mod auth_code;
pub mod device_code;

use byokey_types::{ByokError, OAuthToken, ProviderId, Result};

use crate::AuthManager;
use crate::provider::{amp, antigravity, claude, codex, copilot, gemini, iflow, kimi, qwen};

/// Options for the interactive login flow.
#[derive(Debug, Clone, Default)]
pub struct LoginOptions<'a> {
    /// Store the token under this account instead of the default.
    pub account: Option<&'a str>,
    /// Skip opening a browser; print the URL for manual use.
    /// For providers that also support device-code flow, this will use that flow instead.
    pub no_browser: bool,
}

/// Run the full interactive login flow for the given provider.
///
/// # Errors
///
/// Returns an error if the login flow fails for any reason (e.g., network error,
/// state mismatch, missing callback parameters, or token parse failure).
pub async fn login(
    provider: &ProviderId,
    auth: &AuthManager,
    opts: &LoginOptions<'_>,
) -> Result<()> {
    let http = rquest::Client::new();

    match provider {
        // Auth Code + PKCE
        ProviderId::Claude => auth_code::run(&claude::Claude, auth, &http, opts).await,
        ProviderId::Gemini => auth_code::run(&gemini::Gemini, auth, &http, opts).await,
        ProviderId::Antigravity => {
            auth_code::run(&antigravity::Antigravity, auth, &http, opts).await
        }
        ProviderId::IFlow => auth_code::run(&iflow::IFlow, auth, &http, opts).await,
        ProviderId::Amp => auth_code::run(&amp::Amp, auth, &http, opts).await,
        // Codex: --no-browser uses OpenAI's private device auth protocol
        ProviderId::Codex if opts.no_browser => {
            let creds = crate::credentials::fetch("codex", &http).await?;
            let tok = codex::device_auth(&http, &creds).await?;
            save_login_token(auth, provider, tok, opts.account).await?;
            tracing::info!(provider = %provider, "login successful");
            Ok(())
        }
        ProviderId::Codex => auth_code::run(&codex::Codex, auth, &http, opts).await,
        // Device Code
        ProviderId::Copilot => device_code::run(&copilot::Copilot, auth, &http, opts).await,
        ProviderId::Qwen => device_code::run(&qwen::Qwen::new(), auth, &http, opts).await,
        ProviderId::Kimi => device_code::run(&kimi::Kimi, auth, &http, opts).await,
        _ => Err(ByokError::Auth(format!("{provider} login not yet implemented"))),
    }
}

// ── Shared helpers ────────────────────────────────────────────────────────────

/// Save a token for a provider, routing to the named account if specified.
pub(crate) async fn save_login_token(
    auth: &AuthManager,
    provider: &ProviderId,
    token: OAuthToken,
    account: Option<&str>,
) -> Result<()> {
    if let Some(account_id) = account {
        auth.save_token_for(provider, account_id, None, token).await
    } else {
        auth.save_token(provider, token).await
    }
}

pub(crate) fn open_browser(url: &str, opts: &LoginOptions<'_>) {
    if opts.no_browser {
        println!("\nOpen this URL in your browser to log in:\n");
        println!("  {url}\n");
        return;
    }

    tracing::info!(url = %url, "opening browser for OAuth login");
    if let Err(e) = open::that(url) {
        tracing::warn!(error = %e, "failed to open browser automatically");
        println!("\nCould not open browser automatically. Open this URL manually:\n");
        println!("  {url}\n");
    }
}
