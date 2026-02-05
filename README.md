# vigil

Persistent remote shell sessions over SSH and tmux

## Overview

`vigil` is a Rust CLI tool that makes it easy to manage persistent terminal sessions on remote hosts. It wraps SSH and tmux to provide a seamless experience for creating, attaching to, listing, and killing remote tmux sessions.

## Features

- **Persistent Sessions**: Create tmux sessions on remote hosts that persist across disconnections
- **Easy Attachment**: Quickly attach to existing remote sessions
- **Session Management**: List, select, and kill remote sessions interactively
- **SSH Flexibility**: Pass any SSH arguments and destination configuration
- **Customizable tmux**: Configure tmux binary location and session creation arguments
- **User-scoped Sessions**: Sessions are automatically scoped to the local user for organization

## Building

Requires Rust 1.70+

```bash
cargo build --release
```

The binary will be available at `target/release/vigil`

## Usage

### Create or attach to a default session

```bash
vigil user@example.com
```

### Create a session with a custom name

```bash
vigil --session mywork user@example.com
```

### List all sessions on a remote host

```bash
vigil --list user@example.com
```

### Attach to an existing session (interactive or by name)

```bash
# Interactive selection
vigil --attach user@example.com

# Attach to specific session
vigil --attach my-session user@example.com
```

### Kill a session

```bash
# Interactive selection
vigil --kill user@example.com

# Kill specific session
vigil --kill my-session user@example.com
```

### Custom tmux configuration

```bash
vigil --tmux=/usr/local/bin/tmux --tmuxargs="-u" user@example.com
```

## Options

| Option | Default | Description |
|--------|---------|-------------|
| `--session NAME` | `default` | Base tmux session name (suffixed with local user) |
| `--tmux PATH` | `tmux` | Path to tmux binary on remote host |
| `--tmuxargs ARGS` | (empty) | Extra arguments passed to `tmux new-session` |
| `--attach [NAME]` | - | Attach to a session (optionally by name) |
| `--select [NAME]` | - | Alias for `--attach` |
| `--kill [NAME]` | - | Kill a session (optionally by name) |
| `--list` | - | List all sessions and exit |

## Requirements

- SSH access to target host
- tmux installed on remote host
- Unix-like environment (Linux, macOS, BSD)