use anyhow::{anyhow, Context, Result};
use std::process::{Command, Stdio};
use crate::config::Config;
use crate::ui;

/// Infer SSH program and normalize arguments
pub fn infer_ssh_prog(ssh_args: &[String]) -> Result<(String, Vec<String>)> {
    let prog = "ssh".to_string();
    Ok((prog, ssh_args.to_vec()))
}

/// Execute a command over SSH on the remote host
pub fn exec_remote_command(
    config: &Config,
    command: &str,
) -> Result<()> {
    let mut ssh_args = config.ssh_args.clone();
    ssh_args.push(command.to_string());

    config.debug_print(&format!("ssh prog: {}", config.ssh_prog));
    config.debug_print(&format!("ssh args (final): {:?}", ssh_args));

    let status = Command::new(&config.ssh_prog)
        .args(&ssh_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to execute {}", config.ssh_prog))?;

    if !status.success() {
        if let Some(127) = status.code() {
            ui::error(
                "Remote reported 'command not found' — tmux may not be installed.\n  \
                 - Debian/Ubuntu: sudo apt-get install tmux\n  \
                 - RHEL/CentOS/Fedora: sudo yum install tmux (or dnf)\n  \
                 - macOS (Homebrew): brew install tmux"
            );
        }
        return Err(anyhow!("remote command exited with status: {}", status));
    }

    Ok(())
}

/// Execute SSH command and capture output
pub fn exec_remote_capture(
    config: &Config,
    command: &str,
) -> Result<String> {
    let mut ssh_args = config.ssh_args.clone();
    
    // Remove TTY flags for non-interactive commands
    ssh_args.retain(|a| a != "-t" && a != "-tt");
    
    ssh_args.push(command.to_string());

    config.debug_print(&format!("executing remote (capture): {}", command));

    let output = Command::new(&config.ssh_prog)
        .args(&ssh_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to execute {} for remote command", config.ssh_prog))?;

    Ok(String::from_utf8_lossy(&output.stdout).to_string())
}
