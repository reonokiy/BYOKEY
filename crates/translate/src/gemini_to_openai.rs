//! Translates Gemini API responses into OpenAI-compatible format.

use byokey_types::{ResponseTranslator, Result};
use serde_json::{Value, json};

/// Translator from Gemini response format to `OpenAI` chat completion format.
pub struct GeminiToOpenAI;

/// Maps a Gemini `finishReason` to an `OpenAI` `finish_reason`.
fn map_finish_reason(reason: Option<&str>, has_tool_calls: bool) -> &'static str {
    match reason {
        Some("MAX_TOKENS" | "max_tokens") => "length",
        _ if has_tool_calls => "tool_calls",
        _ => "stop",
    }
}

impl ResponseTranslator for GeminiToOpenAI {
    /// Translates a Gemini `generateContent` response into an `OpenAI` chat completion response.
    ///
    /// # Errors
    ///
    /// Returns an error if the response cannot be translated.
    fn translate_response(&self, res: Value) -> Result<Value> {
        let parts = res
            .pointer("/candidates/0/content/parts")
            .and_then(Value::as_array);

        let mut text_parts: Vec<&str> = Vec::new();
        let mut tool_calls: Vec<Value> = Vec::new();

        if let Some(parts) = parts {
            for (i, part) in parts.iter().enumerate() {
                if let Some(text) = part.get("text").and_then(Value::as_str) {
                    text_parts.push(text);
                }
                if let Some(fc) = part.get("functionCall") {
                    let name = fc.get("name").and_then(Value::as_str).unwrap_or("");
                    let args = fc.get("args").unwrap_or(&Value::Null);
                    let arguments = serde_json::to_string(args).unwrap_or_default();
                    tool_calls.push(json!({
                        "id": format!("call_{i}"),
                        "type": "function",
                        "function": {
                            "name": name,
                            "arguments": arguments,
                        }
                    }));
                }
            }
        }

        let has_tool_calls = !tool_calls.is_empty();

        let finish_reason = map_finish_reason(
            res.pointer("/candidates/0/finishReason")
                .and_then(Value::as_str),
            has_tool_calls,
        );

        let model = res
            .get("modelVersion")
            .and_then(Value::as_str)
            .unwrap_or("gemini")
            .to_string();

        let id = if model == "gemini" {
            let ts = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |d| d.as_millis());
            format!("chatcmpl-gemini-{ts}")
        } else {
            format!("chatcmpl-{model}")
        };

        let prompt_tokens = res
            .pointer("/usageMetadata/promptTokenCount")
            .and_then(Value::as_u64)
            .unwrap_or(0);
        let completion_tokens = res
            .pointer("/usageMetadata/candidatesTokenCount")
            .and_then(Value::as_u64)
            .unwrap_or(0);

        let content_value: Value = if text_parts.is_empty() {
            Value::Null
        } else {
            Value::String(text_parts.join(""))
        };

        let mut message = json!({
            "role": "assistant",
            "content": content_value,
        });

        if has_tool_calls {
            message["tool_calls"] = Value::Array(tool_calls);
        }

        Ok(json!({
            "id": id,
            "object": "chat.completion",
            "model": model,
            "choices": [{
                "index": 0,
                "message": message,
                "finish_reason": finish_reason
            }],
            "usage": {
                "prompt_tokens": prompt_tokens,
                "completion_tokens": completion_tokens,
                "total_tokens": prompt_tokens + completion_tokens
            }
        }))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn sample() -> Value {
        json!({
            "candidates": [{
                "content": { "parts": [{"text": "Hi there!"}], "role": "model" },
                "finishReason": "STOP"
            }],
            "usageMetadata": { "promptTokenCount": 8, "candidatesTokenCount": 4 }
        })
    }

    #[test]
    fn test_content_extraction() {
        let out = GeminiToOpenAI.translate_response(sample()).unwrap();
        assert_eq!(out["choices"][0]["message"]["content"], "Hi there!");
    }

    #[test]
    fn test_finish_reason_stop() {
        let out = GeminiToOpenAI.translate_response(sample()).unwrap();
        assert_eq!(out["choices"][0]["finish_reason"], "stop");
    }

    #[test]
    fn test_finish_reason_length() {
        let mut r = sample();
        r["candidates"][0]["finishReason"] = json!("MAX_TOKENS");
        let out = GeminiToOpenAI.translate_response(r).unwrap();
        assert_eq!(out["choices"][0]["finish_reason"], "length");
    }

    #[test]
    fn test_usage_mapping() {
        let out = GeminiToOpenAI.translate_response(sample()).unwrap();
        assert_eq!(out["usage"]["prompt_tokens"], 8);
        assert_eq!(out["usage"]["completion_tokens"], 4);
        assert_eq!(out["usage"]["total_tokens"], 12);
    }

    #[test]
    fn test_multi_part_text() {
        let res = json!({
            "candidates": [{
                "content": {
                    "parts": [
                        {"text": "Hello "},
                        {"text": "world!"}
                    ],
                    "role": "model"
                },
                "finishReason": "STOP"
            }],
            "usageMetadata": { "promptTokenCount": 5, "candidatesTokenCount": 3 }
        });
        let out = GeminiToOpenAI.translate_response(res).unwrap();
        assert_eq!(out["choices"][0]["message"]["content"], "Hello world!");
        assert_eq!(out["choices"][0]["finish_reason"], "stop");
    }

    #[test]
    fn test_function_call_response() {
        let res = json!({
            "candidates": [{
                "content": {
                    "parts": [{
                        "functionCall": {
                            "name": "get_weather",
                            "args": {"city": "Tokyo"}
                        }
                    }],
                    "role": "model"
                },
                "finishReason": "FUNCTION_CALL"
            }],
            "modelVersion": "gemini-2.0-flash",
            "usageMetadata": { "promptTokenCount": 10, "candidatesTokenCount": 8 }
        });
        let out = GeminiToOpenAI.translate_response(res).unwrap();
        let choice = &out["choices"][0];
        assert_eq!(choice["finish_reason"], "tool_calls");
        assert!(choice["message"]["content"].is_null());
        let tc = choice["message"]["tool_calls"].as_array().unwrap();
        assert_eq!(tc.len(), 1);
        assert_eq!(tc[0]["id"], "call_0");
        assert_eq!(tc[0]["type"], "function");
        assert_eq!(tc[0]["function"]["name"], "get_weather");
        let args: Value =
            serde_json::from_str(tc[0]["function"]["arguments"].as_str().unwrap()).unwrap();
        assert_eq!(args["city"], "Tokyo");
    }

    #[test]
    fn test_mixed_text_and_function_call() {
        let res = json!({
            "candidates": [{
                "content": {
                    "parts": [
                        {"text": "Let me check the weather."},
                        {
                            "functionCall": {
                                "name": "get_weather",
                                "args": {"city": "Berlin"}
                            }
                        }
                    ],
                    "role": "model"
                },
                "finishReason": "FUNCTION_CALL"
            }],
            "modelVersion": "gemini-2.0-flash",
            "usageMetadata": { "promptTokenCount": 12, "candidatesTokenCount": 10 }
        });
        let out = GeminiToOpenAI.translate_response(res).unwrap();
        let msg = &out["choices"][0]["message"];
        assert_eq!(msg["content"], "Let me check the weather.");
        let tc = msg["tool_calls"].as_array().unwrap();
        assert_eq!(tc.len(), 1);
        assert_eq!(tc[0]["function"]["name"], "get_weather");
        assert_eq!(out["choices"][0]["finish_reason"], "tool_calls");
    }

    #[test]
    fn test_empty_response() {
        let res = json!({
            "candidates": [{
                "content": { "parts": [], "role": "model" },
                "finishReason": "STOP"
            }],
            "usageMetadata": { "promptTokenCount": 0, "candidatesTokenCount": 0 }
        });
        let out = GeminiToOpenAI.translate_response(res).unwrap();
        assert!(out["choices"][0]["message"]["content"].is_null());
        assert_eq!(out["choices"][0]["finish_reason"], "stop");
        assert!(out["choices"][0]["message"].get("tool_calls").is_none());
    }

    #[test]
    fn test_model_version_extracted() {
        let mut r = sample();
        r["modelVersion"] = json!("gemini-2.0-flash");
        let out = GeminiToOpenAI.translate_response(r).unwrap();
        assert_eq!(out["model"], "gemini-2.0-flash");
        assert!(out["id"].as_str().unwrap().contains("gemini-2.0-flash"));
    }

    #[test]
    fn test_model_fallback() {
        let out = GeminiToOpenAI.translate_response(sample()).unwrap();
        assert_eq!(out["model"], "gemini");
        assert!(out["id"].as_str().unwrap().starts_with("chatcmpl-gemini-"));
    }

    #[test]
    fn test_multiple_function_calls() {
        let res = json!({
            "candidates": [{
                "content": {
                    "parts": [
                        {
                            "functionCall": {
                                "name": "get_weather",
                                "args": {"city": "Tokyo"}
                            }
                        },
                        {
                            "functionCall": {
                                "name": "get_time",
                                "args": {"timezone": "JST"}
                            }
                        }
                    ],
                    "role": "model"
                },
                "finishReason": "FUNCTION_CALL"
            }],
            "usageMetadata": { "promptTokenCount": 10, "candidatesTokenCount": 15 }
        });
        let out = GeminiToOpenAI.translate_response(res).unwrap();
        let tc = out["choices"][0]["message"]["tool_calls"]
            .as_array()
            .unwrap();
        assert_eq!(tc.len(), 2);
        assert_eq!(tc[0]["id"], "call_0");
        assert_eq!(tc[0]["function"]["name"], "get_weather");
        assert_eq!(tc[1]["id"], "call_1");
        assert_eq!(tc[1]["function"]["name"], "get_time");
    }
}
