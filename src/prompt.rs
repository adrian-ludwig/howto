use std::process::Command;

/// Detect the current OS name and version.
fn detect_os() -> String {
    let name = std::env::consts::OS;
    let version = Command::new("uname")
        .arg("-r")
        .output()
        .ok()
        .and_then(|o| String::from_utf8(o.stdout).ok())
        .unwrap_or_default()
        .trim()
        .to_string();
    format!("{name} {version}").trim().to_string()
}

/// Detect the user's shell.
fn detect_shell() -> String {
    std::env::var("SHELL")
        .unwrap_or_else(|_| "unknown".to_string())
        .rsplit('/')
        .next()
        .unwrap_or("unknown")
        .to_string()
}

/// Build the system prompt for the LLM.
pub fn system_prompt() -> String {
    let os = detect_os();
    let shell = detect_shell();

    format!(
        r#"You are a shell command generator. The user describes a task in natural language. You respond with exactly one JSON object on a single line, no markdown, no explanation outside the JSON.

User's system: {os}, shell: {shell}

JSON format (no other output):
{{"cmd":"<command>","explain":"<short explanation>","risk":"<low|medium|high>","needs_sudo":<true|false>}}

Rules:
- "cmd" must be a single shell command (pipes allowed, but one logical line).
- Prefer the simplest, most portable command.
- Do not use aliases.
- "risk": "low" = read-only, "medium" = modifies state reversibly, "high" = destructive/irreversible.
- "needs_sudo": true only if the command requires root privileges.
- Return ONLY the JSON line. No markdown fences, no extra text."#
    )
}

/// Build the user message for the LLM.
pub fn user_message(query: &str) -> String {
    query.to_string()
}

/// Build a retry message when the LLM returns invalid JSON.
pub fn retry_message(raw: &str) -> String {
    format!(
        "Your previous response was not valid single-line JSON. Here is what you returned:\n{raw}\n\nReturn ONLY a valid JSON object on a single line, nothing else."
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_system_prompt_contains_os() {
        let prompt = system_prompt();
        assert!(prompt.contains("shell command generator"));
        let os = std::env::consts::OS;
        assert!(prompt.contains(os));
    }

    #[test]
    fn test_retry_message() {
        let msg = retry_message("bad output");
        assert!(msg.contains("bad output"));
        assert!(msg.contains("valid JSON"));
    }
}
