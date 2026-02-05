use anyhow::{anyhow, Context, Result};
use clap::Parser;
use std::env;
use std::process::{Command, Stdio};

/// vigil: persistent remote shell sessions via SSH + tmux
#[derive(Parser, Debug)]
#[command(name = "vigil", version, about = "Persistent remote tmux sessions over SSH", trailing_var_arg = true)]
struct Cli {
    /// Base tmux session name (will be suffixed with local user)
    #[arg(long = "session", default_value = "default")]
    session: String,

    /// tmux binary on the remote host
    #[arg(long = "tmux", default_value = "tmux")]
    tmux_bin: String,

    /// Extra arguments passed to tmux new-session
    #[arg(long = "tmuxargs", default_value = "")]
    tmux_args: String,

    /// Attach to a session (optionally by name). Alias: --select
    #[arg(long = "attach", alias = "select", value_name = "NAME", num_args = 0..=1)]
    attach: Option<Option<String>>,

    /// Kill a session (optionally by name)
    #[arg(long = "kill", value_name = "NAME", num_args = 0..=1)]
    kill: Option<Option<String>>,

    /// List sessions on the remote host and exit
    #[arg(long = "list")]
    list: bool,

    /// SSH arguments and destination (e.g. user@host)
    #[arg(value_name = "SSH_ARGS", num_args = 0.., allow_hyphen_values = true)]
    ssh_args: Vec<String>,
}

fn main() -> Result<()> {
    let parsed = parse_args()?;

    ensure_binaries_present()?;

    let local_user = get_local_username();

    // Infer ssh program/args first; we may need it for list/kill.
    let (ssh_prog, mut ssh_args) = infer_ssh_prog(&parsed.ssh_args)?;

    // List mode: print sessions and exit.
    if parsed.list {
        let sessions = list_remote_sessions(&ssh_prog, &parsed, &ssh_args)?;
        if sessions.is_empty() {
            eprintln!("[vigil] No tmux sessions found remotely.");
        } else {
            for s in sessions {
                println!("{}", s);
            }
        }
        return Ok(());
    }

    // Kill mode: kill a named session, or list/select if not provided, then exit.
    if let Some(kill_opt) = &parsed.kill {
        let target = match kill_opt {
            Some(name) => name.clone(),
            None => {
                let sessions = list_remote_sessions(&ssh_prog, &parsed, &ssh_args)?;
                if sessions.is_empty() {
                    eprintln!("[vigil] No tmux sessions found remotely to kill.");
                    return Ok(());
                }
                prompt_user_to_select_session("kill", &sessions)?
            }
        };
        kill_remote_session(&ssh_prog, &parsed, &ssh_args, &target)?;
        eprintln!("[vigil] Killed session '{}'.", target);
        return Ok(());
    }

    // Attach: explicit name wins; otherwise optionally interactive select.
    let chosen_session = if let Some(att_opt) = &parsed.attach {
        match att_opt {
            Some(name) => Some(name.clone()),
            None => match list_remote_sessions(&ssh_prog, &parsed, &ssh_args) {
                Ok(sessions) => {
                    if sessions.is_empty() {
                        eprintln!(
                            "[vigil] No tmux sessions found remotely; will create/attach to '{}'.",
                            format!("{}_{}", parsed.session, &local_user)
                        );
                        None
                    } else {
                        Some(prompt_user_to_select_session("attach", &sessions)? )
                    }
                }
                Err(e) => return Err(e),
            },
        }
    } else { None };

    let final_session = match chosen_session {
        Some(name) => name,
        None => format!("{}_{}", parsed.session, local_user),
    };

    // Compose remote tmux command: create if absent or attach if present
    let mut tmux_cmd: Vec<String> = vec![
        parsed.tmux_bin.clone(),
        "new-session".into(),
        "-A".into(),
        "-s".into(),
        final_session.clone(),
    ];
    if !parsed.tmux_args.trim().is_empty() {
        let mut extra = shell_words::split(&parsed.tmux_args)
            .map_err(|e| anyhow!("failed parsing --tmuxargs: {}", e))?;
        tmux_cmd.append(&mut extra);
    }

    if env::var_os("VIGIL_DEBUG").is_some() {
        eprintln!("[vigil] ssh prog: {}", ssh_prog);
        eprintln!("[vigil] ssh args (pre): {:?}", ssh_args);
        eprintln!("[vigil] tmux argv: {:?}", tmux_cmd);
    }
    ssh_args.extend(tmux_cmd);
    if env::var_os("VIGIL_DEBUG").is_some() {
        eprintln!("[vigil] ssh args (final): {:?}", ssh_args);
    }

    let status = Command::new(&ssh_prog)
        .args(&ssh_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to execute {}", ssh_prog))?;

    if !status.success() {
        if let Some(127) = status.code() {
            eprintln!(
                "[vigil] Remote reported 'command not found' — tmux may not be installed.\n  - Debian/Ubuntu: sudo apt-get install tmux\n  - RHEL/CentOS/Fedora: sudo yum install tmux (or dnf)\n  - macOS (Homebrew): brew install tmux"
            );
        }
        return Err(anyhow!("remote command exited with status: {}", status));
    }

    Ok(())
}

fn ensure_binaries_present() -> Result<()> {
    let status = Command::new("ssh")
        .arg("-V")
        .stdout(Stdio::null())
        .stderr(Stdio::null())
        .status();
    match status {
        Ok(_) => Ok(()),
        Err(_) => Err(anyhow!("`ssh` not found in PATH")),
    }
}

fn parse_args() -> Result<Cli> {
    let mut parsed = Cli::parse();

    // Be forgiving: if users place flags after the host (common habit), the
    // trailing var-arg will capture them. Scan ssh_args for our known flags and
    // hoist them into structured options, removing them from ssh_args.
    let mut i = 0;
    while i < parsed.ssh_args.len() {
        let tok = parsed.ssh_args[i].clone();
        if tok == "--list" && !parsed.list {
            parsed.list = true;
            parsed.ssh_args.remove(i);
            continue;
        }
        if (tok == "--attach" || tok == "--select") && parsed.attach.is_none() {
            parsed.ssh_args.remove(i);
            // Optional NAME follows if next token isn't a flag or host-like
            if i < parsed.ssh_args.len() {
                let next = &parsed.ssh_args[i];
                if !next.starts_with('-') && !next.contains('@') && !next.contains(':') {
                    let name = parsed.ssh_args.remove(i);
                    parsed.attach = Some(Some(name));
                } else {
                    parsed.attach = Some(None);
                }
            } else {
                parsed.attach = Some(None);
            }
            continue;
        }
        if tok == "--kill" && parsed.kill.is_none() {
            parsed.ssh_args.remove(i);
            if i < parsed.ssh_args.len() {
                let next = &parsed.ssh_args[i];
                if !next.starts_with('-') && !next.contains('@') && !next.contains(':') {
                    let name = parsed.ssh_args.remove(i);
                    parsed.kill = Some(Some(name));
                } else {
                    parsed.kill = Some(None);
                }
            } else {
                parsed.kill = Some(None);
            }
            continue;
        }
        i += 1;
    }

    // Ensure we allocate a TTY by default for attach/create operations.
    if !parsed.ssh_args.iter().any(|a| a == "-t" || a == "-tt") {
        parsed.ssh_args.insert(0, "-t".into());
    }

    Ok(parsed)
}

fn infer_ssh_prog(ssh_args: &[String]) -> Result<(String, Vec<String>)> {
    let prog = "ssh".to_string();
    Ok((prog, ssh_args.to_vec()))
}

fn shell_escape(s: &str) -> String {
    // Minimal shell escaping for tmux session names: wrap in single quotes and escape existing ones
    let escaped = s.replace('\'', "'\\''");
    format!("'{}'", escaped)
}

fn get_local_username() -> String {
    env::var("USER")
        .or_else(|_| env::var("LOGNAME"))
        .unwrap_or_else(|_| "user".to_string())
}

fn list_remote_sessions(ssh_prog: &str, parsed: &Cli, ssh_args: &[String]) -> Result<Vec<String>> {
    // Build a non-tty ssh invocation to list sessions only (names). We must
    // quote the tmux -F format because '#' starts a comment in the remote shell.
    let mut list_args: Vec<String> = ssh_args
        .iter()
        .filter(|a| a.as_str() != "-t" && a.as_str() != "-tt")
        .cloned()
        .collect();

    let remote_cmd = format!(
        "{} list-sessions -F {}",
        parsed.tmux_bin,
        shell_escape("#{session_name}")
    );
    list_args.push(remote_cmd);

    let output = Command::new(ssh_prog)
        .args(&list_args)
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .output()
        .with_context(|| format!("failed to execute {} for listing sessions", ssh_prog))?;

    if !output.status.success() {
        if let Some(127) = output.status.code() {
            eprintln!(
                "[vigil] tmux not found on remote host.\n  - Debian/Ubuntu: sudo apt-get install tmux\n  - RHEL/CentOS/Fedora: sudo yum install tmux (or dnf)\n  - macOS (Homebrew): brew install tmux"
            );
            return Err(anyhow!("remote tmux not found (exit 127)"));
        }
        // Non-zero from tmux when no server exists is fine; treat as no sessions
    }

    let stdout = String::from_utf8_lossy(&output.stdout);
    let sessions: Vec<String> = stdout
        .lines()
        .map(|s| s.trim())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .collect();
    Ok(sessions)
}

fn prompt_user_to_select_session(action: &str, sessions: &[String]) -> Result<String> {
    use std::io::{self, Write};
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

fn kill_remote_session(ssh_prog: &str, parsed: &Cli, ssh_args: &[String], target: &str) -> Result<()> {
    let mut kill_args: Vec<String> = ssh_args
        .iter()
        .filter(|a| a.as_str() != "-t" && a.as_str() != "-tt")
        .cloned()
        .collect();

    let remote_cmd = format!(
        "{} kill-session -t {}",
        parsed.tmux_bin,
        shell_escape(target)
    );
    kill_args.push(remote_cmd);

    let status = Command::new(ssh_prog)
        .args(&kill_args)
        .stdin(Stdio::inherit())
        .stdout(Stdio::inherit())
        .stderr(Stdio::inherit())
        .status()
        .with_context(|| format!("failed to execute {} for killing session", ssh_prog))?;

    if !status.success() {
        if let Some(127) = status.code() {
            eprintln!(
                "[vigil] tmux not found on remote host.\n  - Debian/Ubuntu: sudo apt-get install tmux\n  - RHEL/CentOS/Fedora: sudo yum install tmux (or dnf)\n  - macOS (Homebrew): brew install tmux"
            );
            return Err(anyhow!("remote tmux not found (exit 127)"));
        }
        return Err(anyhow!("failed to kill remote session '{}' (status: {:?})", target, status.code()));
    }
    Ok(())
}