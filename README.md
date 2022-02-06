# sshs
Terminal user interface for SSH.  
It uses `~/.ssh/config` to list and connect to hosts.

[![example](https://i.imgur.com/iPmiEVU.gif)](https://asciinema.org/a/465800)

# Requirements
You need to have `ssh` installed and accessible from your terminal.

# How to install
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

## From releases
Releases contains prebuilt binaries for Linux, macOS and Windows. You can download them at https://github.com/quantumsheep/sshs/releases.

## From sources
```bash
git clone https://github.com/quantumsheep/sshs.git
cd sshs
make
make install
```
