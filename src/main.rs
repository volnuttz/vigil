mod cli;
mod config;
mod ssh;
mod tmux;
mod ui;
mod util;

use anyhow::Result;

fn main() -> Result<()> {
    // Parse arguments with fallback flag hoisting
    let cli_args = cli::Cli::parse_with_fallback()?;
    
    // Extract mode flags before consuming cli_args
    let list_mode = cli_args.list;
    let kill_opt = cli_args.kill.clone();
    let attach_opt = cli_args.attach.clone();
    
    // Convert to config
    let config = cli_args.to_config()?;

    // Handle list mode: print sessions and exit
    if config.debug {
        ui::status("List mode enabled");
    }
    if config.debug || list_mode {
        match tmux::list_remote_sessions(&config) {
            Ok(sessions) => {
                if sessions.is_empty() {
                    ui::status("No tmux sessions found remotely.");
                } else {
                    for s in sessions {
                        println!("{}", s);
                    }
                }
            }
            Err(e) => {
                ui::error(&format!("Failed to list sessions: {}", e));
                return Err(e);
            }
        }
        if list_mode {
            return Ok(());
        }
    }

    // Handle kill mode: kill a named session or interactively select
    if let Some(kill_opt_val) = kill_opt {
        let target = match kill_opt_val {
            Some(name) => name,
            None => {
                match tmux::list_remote_sessions(&config) {
                    Ok(sessions) => {
                        if sessions.is_empty() {
                            ui::status("No tmux sessions found remotely to kill.");
                            return Ok(());
                        }
                        ui::prompt_user_to_select_session("kill", &sessions)?
                    }
                    Err(e) => {
                        ui::error(&format!("Failed to list sessions: {}", e));
                        return Err(e);
                    }
                }
            }
        };
        tmux::kill_remote_session(&config, &target)?;
        ui::status(&format!("Killed session '{}'.", target));
        return Ok(());
    }

    // Handle attach mode: attach to named, interactively selected, or default session
    let final_session_name = match attach_opt {
        Some(Some(name)) => {
            // Explicit session name provided
            name
        }
        Some(None) => {
            // Interactive selection
            match tmux::list_remote_sessions(&config) {
                Ok(sessions) => {
                    if sessions.is_empty() {
                        let default_name = format!("{}_{}", config.session, config.local_user);
                        ui::status(&format!(
                            "No tmux sessions found remotely; will create/attach to '{}'.",
                            default_name
                        ));
                        default_name
                    } else {
                        ui::prompt_user_to_select_session("attach", &sessions)?
                    }
                }
                Err(e) => {
                    ui::error(&format!("Failed to list sessions: {}", e));
                    return Err(e);
                }
            }
        }
        None => {
            // Default behavior: create/attach to user-scoped session
            format!("{}_{}", config.session, config.local_user)
        }
    };

    // Attach to the session
    tmux::attach_session(&config, &final_session_name)?;

    Ok(())
}