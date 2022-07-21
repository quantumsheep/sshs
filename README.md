# sshs

<a href="https://repology.org/project/sshs/versions">
    <img src="https://repology.org/badge/vertical-allrepos/sshs.svg" alt="Packaging status" align="right">
</a>

Terminal user interface for SSH.  
It uses `~/.ssh/config` to list and connect to hosts.

<br>

[![example](https://i.imgur.com/iPmiEVU.gif)](https://asciinema.org/a/465800)

# Requirements
You need to have `ssh` installed and accessible from your terminal.

# How to install
## Homebrew
```shell
brew install sshs
```

## Chocolatey
Thanks to [Jakub Levý](https://github.com/jakublevy/chocopkgs/tree/master/sshs) for maintaining this package on Chocolatey.
```shell
choco install sshs
```

## Arch Linux
### Pacman
[Vote for the package to be added to the official repository](https://aur.archlinux.org/packages/sshs).  
In the meantime you can manually install it by cloning the repository and running `makepkg`:
```shell
pacman -S --needed git base-devel
git clone https://aur.archlinux.org/sshs.git
cd sshs
makepkg -si
```

### Yay
```shell
yay -Syua --needed --noconfirm sshs
```

## NixOS / Nix

### As a Flake
```shell
nix profile install 'github:quantumsheep/sshs'
```

### In your NixOS configuration
```nix
environment.systemPackages = with pkgs; [ sshs ];
```

### In your Home Manager configuration
```nix
home.packages = with pkgs; [ sshs ];
```

## From releases
Releases contains prebuilt binaries for Linux, macOS and Windows. You can download them at https://github.com/quantumsheep/sshs/releases.

## From sources
```bash
git clone https://github.com/quantumsheep/sshs.git
cd sshs
make
make install
```

# Troubleshooting
## [...]/.ssh/config: no such file or directory
- Check if you have `~/.ssh/config` file
- If you don't, create it with `touch ~/.ssh/config`

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

