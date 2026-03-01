mod cli;
mod config;
mod install;
mod llm;
mod prompt;
mod safety;
mod ui;

use anyhow::{bail, Result};
use clap::Parser;
use std::io::Write;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

use cli::Cli;
use config::{Config, Engine};
use llm::{parse_response, LlmEngine};

fn start_spinner(msg: &str) -> Arc<AtomicBool> {
    let done = Arc::new(AtomicBool::new(false));
    let done_clone = done.clone();
    let msg = msg.to_string();

    std::thread::spawn(move || {
        let frames = ["⠋", "⠙", "⠹", "⠸", "⠼", "⠴", "⠦", "⠧", "⠇", "⠏"];
        let mut i = 0;
        while !done_clone.load(Ordering::Relaxed) {
            eprint!("\r{} {}  ", frames[i % frames.len()], msg);
            let _ = std::io::stderr().flush();
            std::thread::sleep(std::time::Duration::from_millis(80));
            i += 1;
        }
        // Clear the spinner line
        eprint!("\r\x1b[2K");
        let _ = std::io::stderr().flush();
    });

    done
}

fn stop_spinner(done: Arc<AtomicBool>) {
    done.store(true, Ordering::Relaxed);
    std::thread::sleep(std::time::Duration::from_millis(100));
}

fn resolve_engine(config: &Config) -> Result<Box<dyn LlmEngine>> {
    match config.engine {
        Engine::OpenAi => {
            let key = config
                .openai_api_key
                .clone()
                .ok_or_else(|| anyhow::anyhow!("OPENAI_API_KEY not set"))?;
            Ok(Box::new(llm::openai::OpenAiEngine::new(
                key,
                config.model.clone(),
            )))
        }
        Engine::Ollama => Ok(Box::new(llm::ollama::OllamaEngine::new(
            config.ollama_host.clone(),
            config.model.clone(),
        ))),
        Engine::Auto => {
            if let Some(key) = &config.openai_api_key {
                Ok(Box::new(llm::openai::OpenAiEngine::new(
                    key.clone(),
                    config.model.clone(),
                )))
            } else if llm::ollama::OllamaEngine::is_available(&config.ollama_host) {
                Ok(Box::new(llm::ollama::OllamaEngine::new(
                    config.ollama_host.clone(),
                    config.model.clone(),
                )))
            } else {
                bail!(
                    "No LLM engine available.\n\
                     Set OPENAI_API_KEY for OpenAI, or start Ollama at {}",
                    config.ollama_host
                )
            }
        }
    }
}

fn query_llm(engine: &dyn LlmEngine, query: &str) -> Result<llm::LlmResponse> {
    let sys = prompt::system_prompt();
    let user_msg = prompt::user_message(query);

    let raw = engine.generate(&sys, &user_msg)?;

    // First attempt to parse
    if let Ok(resp) = parse_response(&raw) {
        return Ok(resp);
    }

    // Retry once with corrective prompt
    let retry_msg = prompt::retry_message(&raw);
    let raw2 = engine.generate(&sys, &retry_msg)?;

    parse_response(&raw2).map_err(|e| {
        anyhow::anyhow!("Failed to parse LLM response after retry: {e}\nRaw response: {raw2}")
    })
}

fn run() -> Result<()> {
    let cli = Cli::parse();
    let config = Config::from_env()?.with_cli_overrides(&cli.engine, cli.force)?;

    // Handle subcommands
    if let Some(cmd) = &cli.command {
        match cmd {
            cli::Commands::Install => return install::install(None),
            cli::Commands::Uninstall => return install::uninstall(None),
            cli::Commands::Init { shell } => {
                let script = install::init_script(shell)?;
                print!("{}", script);
                return Ok(());
            }
        }
    }

    // Build query from args
    let query = if let Some(q) = &cli.query_str {
        q.clone()
    } else if !cli.query.is_empty() {
        cli.query.join(" ")
    } else if cli.shell_insert {
        // In shell-insert mode with no query, prompt interactively
        match ui::prompt_for_query()? {
            Some(q) => q,
            None => return Ok(()),
        }
    } else {
        bail!(
            "No query provided.\n\n\
             Usage: howto <natural language query>\n\
             Example: howto list all docker containers\n\n\
             Run 'howto --help' for all options."
        );
    };

    let engine = resolve_engine(&config)?;

    let resp = if cli.shell_insert {
        // No spinner inside shell widget — stderr output corrupts ZLE display
        query_llm(engine.as_ref(), &query)?
    } else {
        let spinner = start_spinner("Thinking...");
        let resp = query_llm(engine.as_ref(), &query);
        stop_spinner(spinner);
        resp?
    };

    // Compute our own risk classification
    let risk = safety::classify_risk(&resp.cmd);

    if cli.print_json {
        println!("{}", serde_json::to_string(&resp)?);
        return Ok(());
    }

    if cli.print_cmd {
        println!("{}", resp.cmd);
        return Ok(());
    }

    // Shell-insert mode (called by shell widget via Ctrl+G):
    // No interactive UI — just return the command on stdout.
    // The user controls execution because the command lands in their prompt.
    if cli.shell_insert {
        if risk == safety::Risk::Blocked && !config.allow_high {
            eprintln!("BLOCKED: this command has been classified as too dangerous.");
            eprintln!("Command: {}", resp.cmd);
            std::process::exit(1);
        }
        if risk == safety::Risk::High && !config.allow_high {
            eprintln!("WARNING: high-risk command. Use --force to allow.");
            eprintln!("Command: {}", resp.cmd);
            std::process::exit(1);
        }
        println!("{}", resp.cmd);
        return Ok(());
    }

    // Replace mode (direct CLI usage): skip interactive UI
    if config.mode == config::Mode::Replace {
        if risk == safety::Risk::Blocked && !config.allow_high {
            eprintln!("BLOCKED: this command has been classified as too dangerous.");
            eprintln!("Command: {}", resp.cmd);
            std::process::exit(1);
        }
        println!("{}", resp.cmd);
        return Ok(());
    }

    // Interactive mode (direct CLI usage): full TUI preview
    let result = ui::interactive_preview(&resp, risk, &config)?;
    if let Some(final_cmd) = result {
        println!("{}", final_cmd);
    }

    Ok(())
}

fn main() {
    if let Err(e) = run() {
        eprintln!("Error: {e:#}");
        std::process::exit(1);
    }
}
