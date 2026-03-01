use anyhow::{bail, Context, Result};
use serde_json::json;

use super::LlmEngine;

pub struct OpenAiEngine {
    api_key: String,
    model: String,
}

impl OpenAiEngine {
    pub fn new(api_key: String, model: Option<String>) -> Self {
        Self {
            api_key,
            model: model.unwrap_or_else(|| "gpt-4o-mini".to_string()),
        }
    }
}

impl LlmEngine for OpenAiEngine {
    fn generate(&self, system: &str, user_msg: &str) -> Result<String> {
        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user_msg}
            ],
            "temperature": 0.0,
            "max_tokens": 256
        });

        let resp = ureq::post("https://api.openai.com/v1/chat/completions")
            .set("Authorization", &format!("Bearer {}", self.api_key))
            .send_json(&body);

        match resp {
            Ok(r) => {
                let json: serde_json::Value = r.into_json().context("Failed to parse OpenAI response")?;
                let content = json["choices"][0]["message"]["content"]
                    .as_str()
                    .context("No content in OpenAI response")?
                    .to_string();
                Ok(content)
            }
            Err(ureq::Error::Status(code, r)) => {
                let text = r.into_string().unwrap_or_default();
                bail!("OpenAI API error ({code}): {text}");
            }
            Err(e) => bail!("Failed to call OpenAI API: {e}"),
        }
    }
}
