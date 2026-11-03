# sshs

<a href="https://repology.org/project/sshs/versions">
    <img src="https://repology.org/badge/vertical-allrepos/sshs.svg" alt="Packaging status" align="right">
</a>

Terminal user interface for SSH.  
It uses `~/.ssh/config` to list and connect to hosts.

<br>
<br>

![example](.github/demo/demo.gif)

## Requirements

You need to have `ssh` installed and accessible from your terminal.

## How to install

### Homebrew

```shell
brew install sshs
```

### Chocolatey

Thanks to [Jakub Levý](https://github.com/jakublevy/chocopkgs/tree/master/sshs) for maintaining this package on Chocolatey.

```shell
choco install sshs
```

### Arch Linux

```shell
pacman -S sshs
```

### Alpine Linux

`sshs` is available in Alpine Linux [testing repository](https://pkgs.alpinelinux.org/package/edge/testing/x86_64/sshs).

```shell
apk add sshs
```

### Debian / Ubuntu

Download the `.deb` file for your architecture, then install it. Replace `amd64` with `arm64` on ARM machines.

```shell
curl -LO https://github.com/quantumsheep/sshs/releases/latest/download/sshs-linux-amd64.deb
sudo apt install ./sshs-linux-amd64.deb
```

### NetBSD

`sshs` is available on NetBSD from the [official repository](https://pkgsrc.se/security/sshs).

```shell
pkgin install sshs
```

### NixOS / Nix

#### As a Flake

```shell
nix profile install 'github:quantumsheep/sshs'
```

#### In your NixOS configuration

```nix
environment.systemPackages = with pkgs; [ sshs ];
```

#### In your Home Manager configuration

```nix
home.packages = with pkgs; [ sshs ];
```

### From releases

Releases contains prebuilt binaries for Linux, macOS and Windows. You can download them at <https://github.com/quantumsheep/sshs/releases>.

### From sources

Building sshs from sources requires [Rust](https://www.rust-lang.org/) compiler and [Cargo](https://doc.rust-lang.org/cargo/) to be installed. You can install them with [rustup](https://rustup.rs).

```bash
cargo install --git https://github.com/quantumsheep/sshs
```

Be sure to have `~/.cargo/bin` in your `PATH` environment variable.

You can also clone the repository and build it manually:

```bash
git clone https://github.com/quantumsheep/sshs.git
cd sshs
cargo build --release
```

The binary will be located at `./target/release/sshs` once the build is complete.

## Custom commands with templates

By default, selecting a host runs `ssh "{{{name}}}"`. The `--template` option replaces that command with your own [Handlebars](https://handlebarsjs.com/guide/) template, so sshs can launch any tool from the same host list.

These variables are available:

| Variable        | Value                                                    |
| --------------- | -------------------------------------------------------- |
| `name`          | First pattern of the `Host` line                         |
| `aliases`       | Other patterns of the `Host` line, joined with `, `      |
| `destination`   | `HostName`, or the host name when `HostName` is not set  |
| `user`          | `User`, empty when not set                               |
| `port`          | `Port`, empty when not set                               |
| `proxy_command` | `ProxyCommand`, empty when not set                       |

Use triple braces (`{{{name}}}`) so that values are not HTML-escaped, and `{{#if ...}}` for optional values. The rendered command is split like a shell would split it, then run directly, **without a shell**: quotes work, but pipes, `&&`, `$VAR` or `~` do not. Wrap the command in `sh -c '...'` if you need them.

`--on-session-start-template` and `--on-session-end-template` take the same variables and run before and after the main command.

### Examples

Mount a host with [SSHFS](https://github.com/libfuse/sshfs):

```bash
sshs --template 'sshfs "{{{name}}}":/path/to/remote /path/to/local'
```

Open a Remote Desktop (RDP) session with [FreeRDP](https://www.freerdp.com/), from a separate file of Windows hosts. sshs only reads the file to build the list, so it can use any extension and stay out of `~/.ssh/config`:

```nginx
# ~/.config/rdp-hosts
Host windows-server
  HostName 192.0.2.10
  User administrator@example.local
```

```bash
sshs --config ~/.config/rdp-hosts \
  --template 'xfreerdp /v:{{{destination}}}{{#if port}}:{{{port}}}{{/if}}{{#if user}} "/u:{{{user}}}"{{/if}} /dynamic-resolution +clipboard'
```

FreeRDP asks for the password when connecting. Some distributions (Arch Linux, Debian, Ubuntu) suffix the FreeRDP 3 binaries with `3`: use `xfreerdp3` there. On Wayland, `sdl-freerdp` (or `sdl-freerdp3`) takes the same options.

## Troubleshooting

### [...]/.ssh/config: no such file or directory

- Check if you have `~/.ssh/config` file
- If you don't, you can create it with `touch ~/.ssh/config`

If you want to use another SSH config file, you can use the `--config` option.

Here's a sample `~/.ssh/config` file:

```nginx
Host *
  AddKeysToAgent yes
  UseKeychain yes
  IdentityFile ~/.ssh/id_rsa

Host "My server"
  HostName server1.example.com
  User root
  Port 22

Host "Go through Proxy"
  HostName server2.example.com
  User someone
  Port 22
  ProxyCommand ssh -W %h:%p proxy.example.com
```

You can check the [OpenBSD `ssh_config` reference](https://man.openbsd.org/ssh_config.5) for more information on how to setup `~/.ssh/config`.
