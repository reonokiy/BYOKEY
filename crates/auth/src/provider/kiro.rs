//! Kiro Device Code + Authorization Code flow configuration.
//!
//! Auth endpoint: `prod.us-east-1.auth.desktop.kiro.dev`.
//! Callback port: 9876.

use crate::token::DeviceCodeResponse;
use byokey_types::Result;

pub const CALLBACK_PORT: u16 = 9876;
pub const AUTH_HOST: &str = "prod.us-east-1.auth.desktop.kiro.dev";

/// # Errors
///
/// Returns an error if the response is missing required fields (`device_code` or `user_code`).
pub fn parse_device_code_response(json: &serde_json::Value) -> Result<DeviceCodeResponse> {
    crate::token::parse_device_code_json(
        json,
        &crate::token::DeviceCodeParseConfig {
            verification_uri_fallback: Some(""),
            default_expires_in: 300,
        },
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    #[test]
    fn test_parse_device_code() {
        let resp = json!({
            "device_code": "dc123",
            "user_code": "ABCD-1234",
            "verification_uri": "https://kiro.dev/activate",
            "expires_in": 300,
            "interval": 5
        });
        let dc = parse_device_code_response(&resp).unwrap();
        assert_eq!(dc.device_code, "dc123");
        assert_eq!(dc.user_code, "ABCD-1234");
        assert_eq!(dc.expires_in, 300);
        assert_eq!(dc.interval, 5);
    }

    #[test]
    fn test_parse_device_code_missing_field() {
        assert!(parse_device_code_response(&json!({"user_code": "x"})).is_err());
    }
}
