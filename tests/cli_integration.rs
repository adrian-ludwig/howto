use std::process::Command;

fn howto_bin() -> Command {
    Command::new(env!("CARGO_BIN_EXE_howto"))
}

#[test]
fn test_help_shows_env_vars() {
    let output = howto_bin().arg("--help").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("HOWTO_MODE"), "Missing HOWTO_MODE in help");
    assert!(stdout.contains("HOWTO_ENGINE"), "Missing HOWTO_ENGINE in help");
    assert!(stdout.contains("OPENAI_API_KEY"), "Missing OPENAI_API_KEY in help");
    assert!(stdout.contains("OLLAMA_HOST"), "Missing OLLAMA_HOST in help");
    assert!(stdout.contains("HOWTO_ALLOW_HIGH"), "Missing HOWTO_ALLOW_HIGH in help");
    assert!(stdout.contains("HOWTO_MODEL"), "Missing HOWTO_MODEL in help");
}

#[test]
fn test_no_args_shows_usage() {
    let output = howto_bin().output().unwrap();
    let stderr = String::from_utf8(output.stderr).unwrap();
    assert!(!output.status.success());
    assert!(stderr.contains("No query provided") || stderr.contains("Usage"));
}

#[test]
fn test_version() {
    let output = howto_bin().arg("--version").output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(stdout.contains("howto"));
}

#[test]
fn test_init_zsh() {
    let output = howto_bin().args(["init", "zsh"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success());
    assert!(stdout.contains("howto-widget"));
    assert!(stdout.contains("bindkey"));
}

#[test]
fn test_init_bash() {
    let output = howto_bin().args(["init", "bash"]).output().unwrap();
    let stdout = String::from_utf8(output.stdout).unwrap();
    assert!(output.status.success());
    assert!(stdout.contains("howto_widget"));
    assert!(stdout.contains("bind -x"));
}

#[test]
fn test_init_unsupported_shell() {
    let output = howto_bin().args(["init", "fish"]).output().unwrap();
    assert!(!output.status.success());
}
