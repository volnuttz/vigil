use anyhow::{anyhow, Context, Result};
use std::io::{self, Write};

/// Display a list of sessions and prompt user to select one
pub fn prompt_user_to_select_session(action: &str, sessions: &[String]) -> Result<String> {
    eprintln!("[vigil] Select a session to {}:", action);
    for (i, name) in sessions.iter().enumerate() {
        eprintln!("  {}. {}", i + 1, name);
    }
    eprint!("Enter number (or press Enter for 1): ");
    io::stderr().flush().ok();

    let mut input = String::new();
    io::stdin().read_line(&mut input).context("failed to read selection")?;
    let input = input.trim();
    let idx = if input.is_empty() { 1 } else { input.parse::<usize>().unwrap_or(0) };
    if idx == 0 || idx > sessions.len() {
        return Err(anyhow!("invalid selection"));
    }
    Ok(sessions[idx - 1].clone())
}

/// Print status message to stderr
pub fn status(msg: &str) {
    eprintln!("[vigil] {}", msg);
}

/// Print error message to stderr
pub fn error(msg: &str) {
    eprintln!("[vigil] ERROR: {}", msg);
}
