use anyhow::{bail, Context, Result};
use serde_json::json;
use std::time::Duration;

use super::LlmEngine;

/// Preferred models in order. We pick the first one that's available locally.
const PREFERRED_MODELS: &[&str] = &[
    "qwen2.5-coder:1.5b",
    "qwen2.5-coder:0.5b",
    "qwen2.5-coder",
    "deepseek-coder:1.3b",
    "codellama:7b",
    "llama3.2:1b",
    "llama3.2",
    "mistral",
    "gemma2:2b",
];

pub struct OllamaEngine {
    host: String,
    model: String,
}

impl OllamaEngine {
    pub fn new(host: String, model: Option<String>) -> Self {
        let resolved = model.unwrap_or_else(|| {
            pick_model(&host).unwrap_or_else(|| "qwen2.5-coder:1.5b".to_string())
        });

        Self {
            host,
            model: resolved,
        }
    }

    /// Check if Ollama is reachable.
    pub fn is_available(host: &str) -> bool {
        let agent = ureq::AgentBuilder::new()
            .timeout(Duration::from_secs(2))
            .build();
        agent.get(host).call().is_ok()
    }
}

/// Pick the best model: currently loaded first, then preference list against available.
fn pick_model(host: &str) -> Option<String> {
    let agent = ureq::AgentBuilder::new()
        .timeout(Duration::from_secs(2))
        .build();

    // 1. Check currently loaded models (already warm — no cold-start)
    if let Some(loaded) = get_loaded_model(&agent, host) {
        return Some(loaded);
    }

    // 2. Check locally available models against our preference list
    let available = get_available_models(&agent, host)?;
    for preferred in PREFERRED_MODELS {
        for available_model in &available {
            if available_model == preferred
                || available_model.starts_with(&format!("{preferred}-"))
            {
                return Some(available_model.clone());
            }
        }
    }

    // 3. If none of our preferred models are available, use whatever is installed
    available.into_iter().next()
}

/// Get the first currently loaded/running model from Ollama.
fn get_loaded_model(agent: &ureq::Agent, host: &str) -> Option<String> {
    let json: serde_json::Value = agent
        .get(&format!("{host}/api/ps"))
        .call()
        .ok()?
        .into_json()
        .ok()?;

    json["models"]
        .as_array()?
        .first()?
        .get("name")?
        .as_str()
        .map(|s| s.to_string())
}

/// Get all locally available model names from Ollama.
fn get_available_models(agent: &ureq::Agent, host: &str) -> Option<Vec<String>> {
    let json: serde_json::Value = agent
        .get(&format!("{host}/api/tags"))
        .call()
        .ok()?
        .into_json()
        .ok()?;

    let models = json["models"].as_array()?;
    let names: Vec<String> = models
        .iter()
        .filter_map(|m| m["name"].as_str().map(|s| s.to_string()))
        .collect();

    if names.is_empty() {
        None
    } else {
        Some(names)
    }
}

impl LlmEngine for OllamaEngine {
    fn generate(&self, system: &str, user_msg: &str) -> Result<String> {
        let body = json!({
            "model": self.model,
            "messages": [
                {"role": "system", "content": system},
                {"role": "user", "content": user_msg}
            ],
            "stream": false,
            "options": {
                "temperature": 0.0
            }
        });

        let url = format!("{}/api/chat", self.host);
        let resp = ureq::post(&url).send_json(&body);

        match resp {
            Ok(r) => {
                let json: serde_json::Value =
                    r.into_json().context("Failed to parse Ollama response")?;
                let content = json["message"]["content"]
                    .as_str()
                    .context("No content in Ollama response")?
                    .to_string();
                Ok(content)
            }
            Err(ureq::Error::Status(code, r)) => {
                let text = r.into_string().unwrap_or_default();
                bail!("Ollama API error ({code}): {text}");
            }
            Err(e) => bail!("Failed to call Ollama API: {e}"),
        }
    }
}
