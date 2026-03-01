use anyhow::{bail, Result};
use std::fs;
use std::path::PathBuf;

const START_MARKER: &str = "# >>> howto >>>";
const END_MARKER: &str = "# <<< howto <<<";

const ZSH_INIT: &str = r#"howto-widget() {
  local query="$BUFFER"
  [[ -z "$query" ]] && return

  local cmd
  cmd="$(howto --shell-insert --query-str "$query" 2>/dev/null)" || return

  BUFFER="$cmd"
  CURSOR=${#BUFFER}
  zle redisplay
}
zle -N howto-widget
bindkey '^G' howto-widget
"#;

const BASH_INIT: &str = r#"howto_widget() {
  local query="${READLINE_LINE}"
  [[ -z "$query" ]] && return

  local cmd
  cmd="$(howto --shell-insert --query-str "$query" 2>/dev/null)" || return

  READLINE_LINE="$cmd"
  READLINE_POINT="${#READLINE_LINE}"
}
bind -x '"\C-g":howto_widget'
"#;

/// Returns the path to the shell rc file for the given shell.
fn rc_path(shell: &str) -> Result<PathBuf> {
    let home = std::env::var("HOME")
        .map_err(|_| anyhow::anyhow!("HOME environment variable not set"))?;
    let filename = match shell {
        "zsh" => ".zshrc",
        "bash" => ".bashrc",
        _ => bail!("Unsupported shell: {shell}"),
    };
    Ok(PathBuf::from(home).join(filename))
}

/// Detects the current shell from the SHELL environment variable.
fn detect_shell() -> Result<String> {
    let shell_env = std::env::var("SHELL")
        .map_err(|_| anyhow::anyhow!("SHELL environment variable not set"))?;
    if shell_env.contains("zsh") {
        Ok("zsh".to_string())
    } else if shell_env.contains("bash") {
        Ok("bash".to_string())
    } else {
        bail!("Unsupported shell: {shell_env}")
    }
}

/// Installs the howto shell integration into the appropriate rc file.
pub fn install(shell_override: Option<&str>) -> Result<()> {
    let shell = match shell_override {
        Some(s) => s.to_string(),
        None => detect_shell()?,
    };
    let path = rc_path(&shell)?;

    if path.exists() {
        let content = fs::read_to_string(&path)?;
        if content.contains(START_MARKER) {
            eprintln!("howto shell integration is already installed in {}", path.display());
            return Ok(());
        }
    }

    let block = format!(
        "\n{}\neval \"$(howto init {})\"\n{}\n",
        START_MARKER, shell, END_MARKER
    );

    let mut file = fs::OpenOptions::new()
        .create(true)
        .append(true)
        .open(&path)?;
    use std::io::Write;
    file.write_all(block.as_bytes())?;

    eprintln!("howto shell integration installed in {}", path.display());
    eprintln!("Restart your shell or run: source {}", path.display());
    Ok(())
}

/// Removes the howto shell integration from the appropriate rc file.
pub fn uninstall(shell_override: Option<&str>) -> Result<()> {
    let shell = match shell_override {
        Some(s) => s.to_string(),
        None => detect_shell()?,
    };
    let path = rc_path(&shell)?;

    if !path.exists() {
        eprintln!("howto shell integration is not installed (file not found: {})", path.display());
        return Ok(());
    }

    let content = fs::read_to_string(&path)?;
    if !content.contains(START_MARKER) {
        eprintln!("howto shell integration is not installed in {}", path.display());
        return Ok(());
    }

    let mut result = Vec::new();
    let mut inside_block = false;
    for line in content.lines() {
        if line.trim() == START_MARKER {
            inside_block = true;
            continue;
        }
        if line.trim() == END_MARKER {
            inside_block = false;
            continue;
        }
        if !inside_block {
            result.push(line);
        }
    }

    let mut output = result.join("\n");
    // Preserve trailing newline if original had one
    if content.ends_with('\n') {
        output.push('\n');
    }

    fs::write(&path, output)?;
    eprintln!("howto shell integration removed from {}", path.display());
    Ok(())
}

/// Returns the shell init script for the given shell.
pub fn init_script(shell: &str) -> Result<String> {
    match shell {
        "zsh" => Ok(ZSH_INIT.to_string()),
        "bash" => Ok(BASH_INIT.to_string()),
        _ => bail!("Unsupported shell: {shell}. Supported shells: zsh, bash"),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::Write;

    #[test]
    fn test_block_content() {
        let shell = "zsh";
        let block = format!(
            "\n{}\neval \"$(howto init {})\"\n{}\n",
            START_MARKER, shell, END_MARKER
        );
        assert!(block.contains(START_MARKER));
        assert!(block.contains(END_MARKER));
        assert!(block.contains("eval \"$(howto init zsh)\""));
    }

    #[test]
    fn test_install_uninstall_roundtrip() {
        let dir = tempfile::tempdir().unwrap();
        let rc_file = dir.path().join(".zshrc");

        // Write some original content
        let original = "# existing config\nexport PATH=/usr/bin\n";
        fs::write(&rc_file, original).unwrap();

        // Simulate install by appending the block
        let block = format!(
            "\n{}\neval \"$(howto init zsh)\"\n{}\n",
            START_MARKER, END_MARKER
        );
        {
            let mut f = fs::OpenOptions::new().append(true).open(&rc_file).unwrap();
            f.write_all(block.as_bytes()).unwrap();
        }

        let after_install = fs::read_to_string(&rc_file).unwrap();
        assert!(after_install.contains(START_MARKER));
        assert!(after_install.contains(END_MARKER));

        // Simulate uninstall by removing the block
        let content = fs::read_to_string(&rc_file).unwrap();
        let mut result = Vec::new();
        let mut inside_block = false;
        for line in content.lines() {
            if line.trim() == START_MARKER {
                inside_block = true;
                continue;
            }
            if line.trim() == END_MARKER {
                inside_block = false;
                continue;
            }
            if !inside_block {
                result.push(line);
            }
        }
        let mut output = result.join("\n");
        if content.ends_with('\n') {
            output.push('\n');
        }
        fs::write(&rc_file, &output).unwrap();

        let after_uninstall = fs::read_to_string(&rc_file).unwrap();
        assert!(!after_uninstall.contains(START_MARKER));
        assert!(!after_uninstall.contains(END_MARKER));
        // Original content should be preserved
        assert!(after_uninstall.contains("# existing config"));
        assert!(after_uninstall.contains("export PATH=/usr/bin"));
    }

    #[test]
    fn test_init_zsh() {
        let script = init_script("zsh").unwrap();
        assert!(script.contains("howto-widget"));
        assert!(script.contains("bindkey"));
        assert!(script.contains("BUFFER"));
    }

    #[test]
    fn test_init_bash() {
        let script = init_script("bash").unwrap();
        assert!(script.contains("howto_widget"));
        assert!(script.contains("bind -x"));
        assert!(script.contains("READLINE_LINE"));
    }

    #[test]
    fn test_init_unsupported() {
        let result = init_script("fish");
        assert!(result.is_err());
    }
}
