use anyhow::{anyhow, Result};
use clap::Parser;
use crate::config::Config;
use crate::util;
use crate::ssh;

/// vigil: persistent remote shell sessions via SSH + tmux
#[derive(Parser, Debug)]
#[command(name = "vigil", version, about = "Persistent remote tmux sessions over SSH", trailing_var_arg = true)]
pub struct Cli {
    /// Base tmux session name (will be suffixed with local user)
    #[arg(long = "session", default_value = "default")]
    pub session: String,

    /// tmux binary on the remote host
    #[arg(long = "tmux", default_value = "tmux")]
    pub tmux_bin: String,

    /// Extra arguments passed to tmux new-session
    #[arg(long = "tmuxargs", default_value = "")]
    pub tmux_args: String,

    /// Attach to a session (optionally by name). Alias: --select
    #[arg(long = "attach", alias = "select", value_name = "NAME", num_args = 0..=1)]
    pub attach: Option<Option<String>>,

    /// Kill a session (optionally by name)
    #[arg(long = "kill", value_name = "NAME", num_args = 0..=1)]
    pub kill: Option<Option<String>>,

    /// List sessions on the remote host and exit
    #[arg(long = "list")]
    pub list: bool,

    /// SSH arguments and destination (e.g. user@host)
    #[arg(value_name = "SSH_ARGS", num_args = 0.., allow_hyphen_values = true)]
    pub ssh_args: Vec<String>,
}

impl Cli {
    /// Parse CLI arguments with fallback flag hoisting
    pub fn parse_with_fallback() -> Result<Self> {
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

    /// Convert CLI args to Config
    pub fn to_config(self) -> Result<Config> {
        // Check SSH is available
        if !util::check_ssh_available() {
            return Err(anyhow!("`ssh` not found in PATH"));
        }

        let local_user = util::get_local_username();
        let (ssh_prog, ssh_args) = ssh::infer_ssh_prog(&self.ssh_args)?;
        let debug = std::env::var_os("VIGIL_DEBUG").is_some();

        Ok(Config::new(
            self.session,
            self.tmux_bin,
            self.tmux_args,
            ssh_prog,
            ssh_args,
            local_user,
            debug,
        ))
    }
}
