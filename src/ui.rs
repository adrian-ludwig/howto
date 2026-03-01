use anyhow::Result;
use crossterm::event::{self, Event, KeyCode, KeyEvent, KeyModifiers};
use crossterm::terminal;
use std::io::Write;

use crate::config::Config;
use crate::llm::LlmResponse;
use crate::safety::Risk;

/// Read a single key event from the terminal.
fn read_key() -> Result<KeyEvent> {
    loop {
        if let Event::Key(key) = event::read()? {
            return Ok(key);
        }
    }
}

/// Show the command preview and handle user interaction.
///
/// All UI output goes to stderr. Only the final command (if accepted) is
/// returned as `Some(cmd)` so the caller can print it to stdout.
pub fn interactive_preview(
    resp: &LlmResponse,
    risk: Risk,
    config: &Config,
) -> Result<Option<String>> {
    // Blocked commands are never allowed without --force
    if risk == Risk::Blocked && !config.allow_high {
        eprintln!("BLOCKED: this command has been classified as too dangerous.");
        eprintln!("Command: {}", resp.cmd);
        eprintln!("This command cannot be executed. Override with --force if you are sure.");
        return Ok(None);
    }

    // High-risk commands require --force
    if risk == Risk::High && !config.allow_high {
        show_preview(&resp.cmd, resp, risk);
        eprintln!("  High-risk command. Use --force to allow. Press any key to dismiss.");
        terminal::enable_raw_mode()?;
        let _ = read_key();
        terminal::disable_raw_mode()?;
        return Ok(None);
    }

    let needs_confirmation = risk >= Risk::Medium;
    let mut cmd = resp.cmd.clone();

    loop {
        show_preview(&cmd, resp, risk);

        if needs_confirmation {
            eprintln!("  Type EXECUTE to confirm   [e] edit   [Esc] cancel");
        } else {
            eprintln!("  [Enter] insert   [e] edit   [Esc] cancel");
        }

        if needs_confirmation {
            // For medium/high: need to read a full line ("EXECUTE"), so use line mode
            eprint!("\r\n  > ");
            std::io::stderr().flush()?;

            let mut input = String::new();
            let tty = std::fs::File::open("/dev/tty")?;
            let mut reader = std::io::BufReader::new(tty);
            std::io::BufRead::read_line(&mut reader, &mut input)?;
            let input = input.trim();

            match input {
                "EXECUTE" => return Ok(Some(cmd)),
                "e" => {
                    cmd = match edit_command(&cmd)? {
                        Some(edited) => edited,
                        None => return Ok(None),
                    };
                }
                "q" | "" => return Ok(None),
                _ => {
                    eprintln!("  Type EXECUTE to confirm, 'e' to edit, or 'q' to cancel.");
                }
            }
        } else {
            // For low risk: use raw mode for instant key response
            terminal::enable_raw_mode()?;
            let key = read_key();
            terminal::disable_raw_mode()?;
            eprintln!(); // newline after raw mode

            match key? {
                KeyEvent { code: KeyCode::Enter, .. } => {
                    return Ok(Some(cmd));
                }
                KeyEvent { code: KeyCode::Char('e'), modifiers, .. }
                    if !modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    cmd = match edit_command(&cmd)? {
                        Some(edited) => edited,
                        None => return Ok(None),
                    };
                }
                KeyEvent { code: KeyCode::Esc, .. }
                | KeyEvent { code: KeyCode::Char('q'), .. } => {
                    return Ok(None);
                }
                KeyEvent { code: KeyCode::Char('c'), modifiers, .. }
                    if modifiers.contains(KeyModifiers::CONTROL) =>
                {
                    return Ok(None);
                }
                _ => {
                    eprintln!("  Press Enter to insert, 'e' to edit, or Esc to cancel.");
                }
            }
        }
    }
}

fn show_preview(cmd: &str, resp: &LlmResponse, risk: Risk) {
    eprintln!();
    eprintln!("  Command: {}", cmd);
    eprintln!("  Explain: {}", resp.explain);
    eprintln!("  Risk:    {}", risk);
    if resp.needs_sudo {
        eprintln!("  Note:    requires sudo");
    }
    eprintln!();
}

/// Edit a command using crossterm raw mode key-by-key input.
fn edit_command(current: &str) -> Result<Option<String>> {
    let mut buf: Vec<char> = current.chars().collect();
    let mut cursor = buf.len();

    terminal::enable_raw_mode()?;
    // Show initial state
    redraw_edit(&buf, cursor);

    loop {
        let key = read_key()?;
        match key {
            KeyEvent { code: KeyCode::Enter, .. } => {
                terminal::disable_raw_mode()?;
                eprintln!();
                let result: String = buf.into_iter().collect();
                let trimmed = result.trim().to_string();
                return Ok(if trimmed.is_empty() { None } else { Some(trimmed) });
            }
            KeyEvent { code: KeyCode::Esc, .. } => {
                terminal::disable_raw_mode()?;
                eprintln!();
                return Ok(None);
            }
            KeyEvent { code: KeyCode::Char('c'), modifiers, .. }
                if modifiers.contains(KeyModifiers::CONTROL) =>
            {
                terminal::disable_raw_mode()?;
                eprintln!();
                return Ok(None);
            }
            KeyEvent { code: KeyCode::Char('a'), modifiers, .. }
                if modifiers.contains(KeyModifiers::CONTROL) =>
            {
                cursor = 0;
                redraw_edit(&buf, cursor);
            }
            KeyEvent { code: KeyCode::Char('e'), modifiers, .. }
                if modifiers.contains(KeyModifiers::CONTROL) =>
            {
                cursor = buf.len();
                redraw_edit(&buf, cursor);
            }
            KeyEvent { code: KeyCode::Char(c), .. } => {
                buf.insert(cursor, c);
                cursor += 1;
                redraw_edit(&buf, cursor);
            }
            KeyEvent { code: KeyCode::Backspace, .. } => {
                if cursor > 0 {
                    cursor -= 1;
                    buf.remove(cursor);
                    redraw_edit(&buf, cursor);
                }
            }
            KeyEvent { code: KeyCode::Delete, .. } => {
                if cursor < buf.len() {
                    buf.remove(cursor);
                    redraw_edit(&buf, cursor);
                }
            }
            KeyEvent { code: KeyCode::Left, .. } => {
                if cursor > 0 {
                    cursor -= 1;
                    redraw_edit(&buf, cursor);
                }
            }
            KeyEvent { code: KeyCode::Right, .. } => {
                if cursor < buf.len() {
                    cursor += 1;
                    redraw_edit(&buf, cursor);
                }
            }
            KeyEvent { code: KeyCode::Home, .. } => {
                cursor = 0;
                redraw_edit(&buf, cursor);
            }
            KeyEvent { code: KeyCode::End, .. } => {
                cursor = buf.len();
                redraw_edit(&buf, cursor);
            }
            _ => {}
        }
    }
}

fn redraw_edit(buf: &[char], cursor: usize) {
    let text: String = buf.iter().collect();
    let prompt = "  edit> ";
    // Clear line, print prompt + text, position cursor
    eprint!("\r\x1b[2K{}{}", prompt, text);
    // Move cursor to correct position
    let back = buf.len() - cursor;
    if back > 0 {
        eprint!("\x1b[{}D", back);
    }
    let _ = std::io::stderr().flush();
}

/// Prompt the user for a query when none was provided on the command line.
pub fn prompt_for_query() -> Result<Option<String>> {
    let mut buf: Vec<char> = Vec::new();
    let mut cursor = 0;

    eprint!("howto> ");
    let _ = std::io::stderr().flush();

    terminal::enable_raw_mode()?;

    loop {
        let key = read_key()?;
        match key {
            KeyEvent { code: KeyCode::Enter, .. } => {
                terminal::disable_raw_mode()?;
                eprintln!();
                let result: String = buf.into_iter().collect();
                let trimmed = result.trim().to_string();
                return Ok(if trimmed.is_empty() { None } else { Some(trimmed) });
            }
            KeyEvent { code: KeyCode::Esc, .. } => {
                terminal::disable_raw_mode()?;
                eprintln!();
                return Ok(None);
            }
            KeyEvent { code: KeyCode::Char('c'), modifiers, .. }
                if modifiers.contains(KeyModifiers::CONTROL) =>
            {
                terminal::disable_raw_mode()?;
                eprintln!();
                return Ok(None);
            }
            KeyEvent { code: KeyCode::Char(c), .. } => {
                buf.insert(cursor, c);
                cursor += 1;
                redraw_prompt(&buf, cursor);
            }
            KeyEvent { code: KeyCode::Backspace, .. } => {
                if cursor > 0 {
                    cursor -= 1;
                    buf.remove(cursor);
                    redraw_prompt(&buf, cursor);
                }
            }
            KeyEvent { code: KeyCode::Left, .. } => {
                if cursor > 0 {
                    cursor -= 1;
                    redraw_prompt(&buf, cursor);
                }
            }
            KeyEvent { code: KeyCode::Right, .. } => {
                if cursor < buf.len() {
                    cursor += 1;
                    redraw_prompt(&buf, cursor);
                }
            }
            _ => {}
        }
    }
}

fn redraw_prompt(buf: &[char], cursor: usize) {
    let text: String = buf.iter().collect();
    eprint!("\r\x1b[2Khowto> {}", text);
    let back = buf.len() - cursor;
    if back > 0 {
        eprint!("\x1b[{}D", back);
    }
    let _ = std::io::stderr().flush();
}
