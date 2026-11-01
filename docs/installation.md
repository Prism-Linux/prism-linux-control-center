# Installation

Prism Linux Control Center uses distribution packages that install the GUI, daemon, helper tools, systemd units, udev rules and desktop entry. Package install does not automatically start hardware control. On first launch, select either the per-user daemon or the system daemon; do not run both at once.

## Supported package targets

The v1.3.0-beta.1 package matrix is Ubuntu 24.04, Ubuntu 26.04 and Debian 13 on amd64. These are the only distributions for which Prism beta packages are prepared. The repository retains Arch and Fedora recipes for the wider project, but they are not release artifacts. Each Debian package is built and smoke-tested on its target distribution because WebKitGTK, FFmpeg and system libraries vary.

- Debian and Ubuntu: see [package verification and installation](debian-packages.md).
- Arch and Fedora: beta packages are not available; the recipes still track the upstream stable package channels.
- Immutable Fedora: the [Distrobox guide](distrobox.md) describes the upstream stable package channel, not this beta.
- From source: follow [Build from source](building-from-source.md).

## Runtime setup

The package installs user and system service definitions, but leaves both hardware daemon modes stopped on a fresh installation. Choose one mode using Installation Health in the GUI. The system mode is useful when hardware control must continue without a logged-in desktop. The session helper serves the active graphical session separately.

USB devices use the packaged udev rules. Some hardware, such as the NVIDIA GPU RGB controller, requires an adapter-specific permission rule. The installed package retains the existing `lianli` service account, binary names, service names and `/var/lib/lianli` configuration path for compatibility.

## Upgrade and configuration

Use your package manager to upgrade the existing package. Package scripts do not enable a daemon on first install; an existing selected service can be restarted during an upgrade. Existing configuration under `/var/lib/lianli` or the user's XDG configuration directory is retained. Back up settings from **Settings → Configuration** before changing service mode or moving between package families.

The beta uses `1.3.0~beta1` plus a distribution suffix in Debian package metadata. This sorts after `1.2.4` and before the stable `1.3.0` release. Beta updates use the prerelease channel; stable builds continue using GitHub's latest stable release endpoint.

## Removal

Remove the package with the distribution package manager (`apt remove lian-li-linux`, `dnf remove lian-li-linux`, or `pacman -R lianli-linux`). Purge package-manager configuration only when explicitly desired. The daemon's user configuration and saved hardware settings are user data; back them up and remove them manually only if you want a full reset. Removal does not uninstall optional EVDI kernel modules or drivers installed separately.
