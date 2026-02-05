/// Core configuration for vigil operations
#[derive(Debug, Clone)]
pub struct Config {
    pub session: String,
    pub tmux_bin: String,
    pub tmux_args: String,
    pub ssh_prog: String,
    pub ssh_args: Vec<String>,
    pub local_user: String,
    pub debug: bool,
}

impl Config {
    pub fn new(
        session: String,
        tmux_bin: String,
        tmux_args: String,
        ssh_prog: String,
        ssh_args: Vec<String>,
        local_user: String,
        debug: bool,
    ) -> Self {
        Config {
            session,
            tmux_bin,
            tmux_args,
            ssh_prog,
            ssh_args,
            local_user,
            debug,
        }
    }

    pub fn debug_print(&self, msg: &str) {
        if self.debug {
            eprintln!("[vigil] {}", msg);
        }
    }
}
