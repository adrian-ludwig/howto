pub mod ollama;
pub mod openai;

use anyhow::Result;
use serde::{Deserialize, Serialize};

/// The structured response from the LLM.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct LlmResponse {
    pub cmd: String,
    pub explain: String,
    pub risk: String,
    pub needs_sudo: bool,
}

/// Trait for LLM engine adapters.
pub trait LlmEngine {
    fn generate(&self, system: &str, user_msg: &str) -> Result<String>;
}

/// Parse the LLM's raw text into a structured response.
pub fn parse_response(raw: &str) -> Result<LlmResponse> {
    let trimmed = raw.trim();
    let json_str = if trimmed.starts_with("```") {
        trimmed
            .trim_start_matches("```json")
            .trim_start_matches("```")
            .trim_end_matches("```")
            .trim()
    } else {
        trimmed
    };

    let resp: LlmResponse = serde_json::from_str(json_str)?;
    Ok(resp)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_valid_json() {
        let raw = r#"{"cmd":"ls -la","explain":"List files","risk":"low","needs_sudo":false}"#;
        let resp = parse_response(raw).unwrap();
        assert_eq!(resp.cmd, "ls -la");
        assert_eq!(resp.explain, "List files");
        assert_eq!(resp.risk, "low");
        assert!(!resp.needs_sudo);
    }

    #[test]
    fn test_parse_with_markdown_fences() {
        let raw = "```json\n{\"cmd\":\"ls\",\"explain\":\"list\",\"risk\":\"low\",\"needs_sudo\":false}\n```";
        let resp = parse_response(raw).unwrap();
        assert_eq!(resp.cmd, "ls");
    }

    #[test]
    fn test_parse_invalid_json() {
        let raw = "not json at all";
        assert!(parse_response(raw).is_err());
    }
}
