use std::env;

/// Shell-escape a string for use in tmux commands
pub fn shell_escape(s: &str) -> String {
    // Minimal shell escaping for tmux session names: wrap in single quotes and escape existing ones
    let escaped = s.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

/// Get the local system username
pub fn get_local_username() -> String {
    env::var("USER")
        .or_else(|_| env::var("LOGNAME"))
        .unwrap_or_else(|_| "user".to_string())
}

/// Check if SSH binary is available in PATH
pub fn check_ssh_available() -> bool {
    use std::process::{Command, Stdio};
    
    Command::new("ssh")
        .arg("-V")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status()
        .is_ok()
}

const TMUX_INSTALL_MESSAGE: &str = 
    "tmux not found on remote host.\n  \
     - Debian/Ubuntu: sudo apt-get install tmux\n  \
     - RHEL/CentOS/Fedora: sudo yum install tmux (or dnf)\n  \
     - macOS (Homebrew): brew install tmux";

pub fn tmux_install_hint() -> &'static str {
    TMUX_INSTALL_MESSAGE
}
