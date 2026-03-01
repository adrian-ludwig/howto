use clap::{Parser, Subcommand};

/// Convert natural language to shell commands.
#[derive(Parser, Debug)]
#[command(
    name = "howto",
    version,
    about,
    after_long_help = "\
Environment variables:
  HOWTO_MODE        UX mode: \"interactive\" (default) or \"replace\"
  HOWTO_ENGINE      LLM engine: \"auto\" (default), \"openai\", or \"ollama\"
  HOWTO_MODEL       Override the model name for the selected engine
  OPENAI_API_KEY    API key for OpenAI (required if engine is openai)
  OLLAMA_HOST       Ollama server URL (default: http://127.0.0.1:11434)
  HOWTO_ALLOW_HIGH  Set to \"1\" to allow high-risk commands with --force"
)]
pub struct Cli {
    /// Natural language query (e.g. "list all docker containers")
    #[arg(trailing_var_arg = true)]
    pub query: Vec<String>,

    /// LLM engine to use
    #[arg(long, default_value = "auto")]
    pub engine: String,

    /// Output raw JSON response from the LLM
    #[arg(long)]
    pub print_json: bool,

    /// Output only the generated command
    #[arg(long)]
    pub print_cmd: bool,

    /// Shell insertion mode (used by shell widget, returns command only)
    #[arg(long)]
    pub shell_insert: bool,

    /// Query string for shell-insert mode
    #[arg(long)]
    pub query_str: Option<String>,

    /// Allow high-risk commands
    #[arg(long)]
    pub force: bool,

    #[command(subcommand)]
    pub command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
pub enum Commands {
    /// Install shell integration (adds to ~/.zshrc or ~/.bashrc)
    Install,
    /// Remove shell integration
    Uninstall,
    /// Print shell init script to stdout
    Init {
        /// Shell type: zsh or bash
        shell: String,
    },
}
