use anyhow::{bail, Result};

#[derive(Debug, Clone, PartialEq)]
pub enum Mode {
    Interactive,
    Replace,
}

#[derive(Debug, Clone, PartialEq)]
pub enum Engine {
    Auto,
    OpenAi,
    Ollama,
}

#[derive(Debug, Clone)]
pub struct Config {
    pub mode: Mode,
    pub engine: Engine,
    pub model: Option<String>,
    pub openai_api_key: Option<String>,
    pub ollama_host: String,
    pub allow_high: bool,
}

impl Config {
    pub fn from_env() -> Result<Self> {
        let mode = match std::env::var("HOWTO_MODE").unwrap_or_default().as_str() {
            "replace" => Mode::Replace,
            "interactive" | "" => Mode::Interactive,
            other => bail!("Unknown HOWTO_MODE: {other}. Use 'interactive' or 'replace'."),
        };

        let engine = match std::env::var("HOWTO_ENGINE").unwrap_or_default().as_str() {
            "openai" => Engine::OpenAi,
            "ollama" => Engine::Ollama,
            "auto" | "" => Engine::Auto,
            other => bail!("Unknown HOWTO_ENGINE: {other}. Use 'auto', 'openai', or 'ollama'."),
        };

        Ok(Config {
            mode,
            engine,
            model: std::env::var("HOWTO_MODEL").ok().filter(|s| !s.is_empty()),
            openai_api_key: std::env::var("OPENAI_API_KEY").ok().filter(|s| !s.is_empty()),
            ollama_host: std::env::var("OLLAMA_HOST")
                .unwrap_or_else(|_| "http://127.0.0.1:11434".to_string()),
            allow_high: std::env::var("HOWTO_ALLOW_HIGH").unwrap_or_default() == "1",
        })
    }

    pub fn with_cli_overrides(mut self, engine_override: &str, force: bool) -> Result<Self> {
        if engine_override != "auto" {
            self.engine = match engine_override {
                "openai" => Engine::OpenAi,
                "ollama" => Engine::Ollama,
                other => bail!("Unknown engine: {other}"),
            };
        }
        if force {
            self.allow_high = true;
        }
        Ok(self)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        std::env::remove_var("HOWTO_MODE");
        std::env::remove_var("HOWTO_ENGINE");
        std::env::remove_var("HOWTO_MODEL");
        std::env::remove_var("OPENAI_API_KEY");
        std::env::remove_var("HOWTO_ALLOW_HIGH");

        let config = Config::from_env().unwrap();
        assert_eq!(config.mode, Mode::Interactive);
        assert_eq!(config.engine, Engine::Auto);
        assert!(config.model.is_none());
        assert!(!config.allow_high);
    }

    #[test]
    fn test_replace_mode() {
        std::env::set_var("HOWTO_MODE", "replace");
        let config = Config::from_env().unwrap();
        assert_eq!(config.mode, Mode::Replace);
        std::env::remove_var("HOWTO_MODE");
    }

    #[test]
    fn test_invalid_mode() {
        std::env::set_var("HOWTO_MODE", "bogus");
        let result = Config::from_env();
        assert!(result.is_err());
        std::env::remove_var("HOWTO_MODE");
    }

    #[test]
    fn test_cli_overrides() {
        std::env::remove_var("HOWTO_ENGINE");
        let config = Config::from_env().unwrap();
        let config = config.with_cli_overrides("ollama", true).unwrap();
        assert_eq!(config.engine, Engine::Ollama);
        assert!(config.allow_high);
    }
}
