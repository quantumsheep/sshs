# sshs User Guide

## Introduction

sshs is a terminal user interface for SSH that provides a visual interface to manage and connect to SSH hosts defined in your `~/.ssh/config` file.

## Features

- Visual host browser with search and filtering
- Quick connection to any configured host
- Host configuration management
- Support for SSH groups
- Fuzzy search for large host lists
- Keyboard-driven navigation

## Installation

### macOS (Homebrew)

```bash
brew install sshs
```

### Linux

#### Arch Linux

```bash
pacman -S sshs
```

#### Alpine Linux

```bash
apk add sshs
```

#### NixOS / Nix

```bash
# As a profile
nix profile install github:quantumsheep/sshs

# Or in configuration
environment.systemPackages = [ pkgs.sshs ];
```

### Windows (Chocolatey)

```bash
choco install sshs
```

### From Source

```bash
cargo install --git https://github.com/quantumsheep/sshs
```

## Quick Start

### First Launch

```bash
sshs
```

sshs will automatically read your `~/.ssh/config` file and display all configured hosts.

### Basic Navigation

| Key | Action |
|-----|--------|
| `↑`/`↓` | Navigate hosts |
| `Enter` | Connect to selected host |
| `/` | Search/filter hosts |
| `Esc` | Clear search |
| `q` | Quit |
| `?` | Show help |

## SSH Configuration

### Basic Host Entry

Add hosts to `~/.ssh/config`:

```ssh
Host myserver
    HostName 192.168.1.100
    User admin
    Port 22
    IdentityFile ~/.ssh/id_ed25519
```

### Host Groups

Use wildcards for host groups:

```ssh
Host production-*
    User deploy
    IdentityFile ~/.ssh/prod_key

Host production-web
    HostName web.prod.example.com

Host production-db
    HostName db.prod.example.com
```

### Jump Hosts

Configure bastion/jump hosts:

```ssh
Host bastion
    HostName bastion.example.com
    User jump

Host internal-server
    HostName 10.0.0.50
    User admin
    ProxyJump bastion
```

### Host Aliases

Multiple aliases for the same host:

```ssh
Host prod web production
    HostName web.prod.example.com
    User deploy
```

## Advanced Usage

### Search and Filter

Press `/` to enter search mode, then type to filter hosts:

- Type `prod` to find all production hosts
- Type `db` to find database servers
- Search is case-insensitive and fuzzy

### Custom SSH Options

sshs respects all standard SSH options:

```ssh
Host myapp
    HostName app.example.com
    User appuser
    Port 2222
    IdentityFile ~/.ssh/app_key
    ForwardAgent yes
    LocalForward 8080 localhost:8080
    ServerAliveInterval 60
```

### Multiple Identity Files

```ssh
Host github-personal
    HostName github.com
    User git
    IdentityFile ~/.ssh/id_github_personal
    IdentitiesOnly yes

Host github-work
    HostName github.com
    User git
    IdentityFile ~/.ssh/id_github_work
    IdentitiesOnly yes
```

### Host-Specific Environment

```ssh
Host dev-server
    HostName dev.example.com
    User developer
    SetEnv RUST_BACKTRACE=1
    SetEnv NODE_ENV=development
```

## Tips and Tricks

### Organize with Comments

```ssh
# === Production Servers ===
Host prod-web
    HostName web.prod.example.com

Host prod-db
    HostName db.prod.example.com

# === Development Servers ===
Host dev-web
    HostName web.dev.example.com
```

### Include External Configs

Split configs into multiple files:

```ssh
# In ~/.ssh/config
Include ~/.ssh/config.d/*

# Create ~/.ssh/config.d/work.conf
# Create ~/.ssh/config.d/personal.conf
```

### Quick Connect from Terminal

While sshs is primarily a TUI, you can still use standard SSH:

```bash
# sshs helps you discover host names
ssh myserver

# Use tab completion with sshs knowledge
ssh <TAB>  # Shows hosts from config
```

## Troubleshooting

### Hosts Not Showing

1. Verify `~/.ssh/config` exists
2. Check file permissions: `chmod 600 ~/.ssh/config`
3. Ensure host entries use correct syntax
4. Run `sshs --config` to see loaded config path

### Connection Failures

sshs uses the system `ssh` command. Debug with:

```bash
# Verbose SSH output
ssh -v myhost

# Test key authentication
ssh -i ~/.ssh/mykey myhost
```

### Permission Denied

```bash
# Fix SSH directory permissions
chmod 700 ~/.ssh
chmod 600 ~/.ssh/config
chmod 600 ~/.ssh/id_*
```

## Configuration Reference

### sshs Settings

sshs itself has minimal configuration - it reads directly from `~/.ssh/config`.

Key bindings and display options may be configurable in future versions.

### SSH Config Options

Common options sshs supports:

| Option | Description |
|--------|-------------|
| `HostName` | Real hostname or IP |
| `User` | Username for connection |
| `Port` | SSH port (default: 22) |
| `IdentityFile` | Private key path |
| `ProxyJump` | Jump/bastion host |
| `ForwardAgent` | Agent forwarding |
| `LocalForward` | Port forwarding |
| `RemoteForward` | Remote port forwarding |
| `Compression` | Enable compression |
| `ServerAliveInterval` | Keepalive interval |

## Keyboard Shortcuts Reference

| Key | Action |
|-----|--------|
| `↑` `↓` | Navigate list |
| `Enter` | Connect |
| `/` | Search |
| `Esc` | Clear search/close help |
| `q` | Quit |
| `?` | Toggle help |
| `Home` | First host |
| `End` | Last host |
| `PgUp` | Page up |
| `PgDn` | Page down |

## Community

- **Repository**: https://github.com/quantumsheep/sshs
- **Issues**: https://github.com/quantumsheep/sshs/issues
- **Discussions**: https://github.com/quantumsheep/sshs/discussions

---

*Community contribution - unofficial user guide*
