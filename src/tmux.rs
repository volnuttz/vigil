use anyhow::{anyhow, Result};
use crate::config::Config;
use crate::ssh;
use crate::util;

/// Build a tmux new-session command
pub fn build_session_command(config: &Config, session_name: &str) -> Vec<String> {
    let mut tmux_cmd: Vec<String> = vec![
        config.tmux_bin.clone(),
        "new-session".into(),
        "-A".into(),
        "-s".into(),
        session_name.to_string(),
    ];
    
    if !config.tmux_args.trim().is_empty() {
        if let Ok(mut extra) = shell_words::split(&config.tmux_args) {
            tmux_cmd.append(&mut extra);
        }
    }
    
    tmux_cmd
}

/// Build the full SSH command with embedded tmux session creation
pub fn build_attach_command(config: &Config, session_name: &str) -> Vec<String> {
    let tmux_cmd = build_session_command(config, session_name);
    let mut ssh_args = config.ssh_args.clone();
    
    // Ensure TTY allocation
    if !ssh_args.iter().any(|a| a == "-t" || a == "-tt") {
        ssh_args.insert(0, "-t".into());
    }
    
    config.debug_print(&format!("ssh args (pre-tmux): {:?}", ssh_args));
    config.debug_print(&format!("tmux argv: {:?}", tmux_cmd));
    
    ssh_args.extend(tmux_cmd);
    ssh_args
}

/// List all remote tmux sessions
pub fn list_remote_sessions(config: &Config) -> Result<Vec<String>> {
    let list_cmd = format!(
        "{} list-sessions -F {}",
        config.tmux_bin,
        util::shell_escape("#{session_name}")
    );

    match ssh::exec_remote_capture(config, &list_cmd) {
        Ok(output) => {
            let sessions: Vec<String> = output
                .lines()
                .map(|s| s.trim())
                .filter(|s| !s.is_empty())
                .map(|s| s.to_string())
                .collect();
            Ok(sessions)
        }
        Err(e) => {
            // Check if it's a "command not found" (127) error
            let stderr = format!("{}", e);
            if stderr.contains("127") || stderr.contains("not found") {
                eprintln!("[vigil] {}", util::tmux_install_hint());
                Err(anyhow!("remote tmux not found"))
            } else {
                // Non-zero from tmux when no server exists is fine; treat as no sessions
                Ok(Vec::new())
            }
        }
    }
}

/// Kill a remote tmux session
pub fn kill_remote_session(config: &Config, target: &str) -> Result<()> {
    let kill_cmd = format!(
        "{} kill-session -t {}",
        config.tmux_bin,
        util::shell_escape(target)
    );

    ssh::exec_remote_command(config, &kill_cmd)
}

/// Attach to a remote tmux session (creates if not exists)
pub fn attach_session(config: &Config, session_name: &str) -> Result<()> {
    let ssh_args = build_attach_command(config, session_name);
    
    let status = std::process::Command::new(&config.ssh_prog)
        .args(&ssh_args)
        .stdin(std::process::Stdio::inherit())
        .stdout(std::process::Stdio::inherit())
        .stderr(std::process::Stdio::inherit())
        .status()?;

    if !status.success() {
        if let Some(127) = status.code() {
            eprintln!("[vigil] {}", util::tmux_install_hint());
        }
        return Err(anyhow!("remote command exited with status: {}", status));
    }

    Ok(())
}
