# SyncthingStatus

A small Rust app that shows the status of a local Syncthing instance as a system tray icon.
Works on **Windows** and **Linux** (Ubuntu and others).

## Features

- Color-coded status icon:
  - 🟢 green check – everything is up to date
  - 🔵 blue rotating arrows – syncing / scanning (animated)
  - 🟡 yellow pause – all folders paused
  - 🔴 red exclamation mark – folder error or system error
  - ⚪ gray cross – Syncthing is not responding
- Tooltip (Windows) / title (Linux) with version, connected devices, remaining data and download/upload speed
- Menu with per-folder status (completion in %, remaining data, paused/scanning/error)
- Menu actions: open the Web GUI, rescan all folders, refresh status now, quit
- Left-clicking the icon opens the Syncthing Web GUI (Windows)
- Address and API key are discovered automatically from Syncthing's `config.xml`
- Marks its own icon as "always visible" on the Windows 11 taskbar

## Installation

### Windows

```powershell
winget install ArveLomsland.SyncthingStatus
```

Or with Scoop:

```powershell
scoop bucket add syncthingstatus https://github.com/ArveLomsland/SyncthingStatus
scoop install syncthing-status
```

Or download `syncthing-status-<version>-setup.exe` from the
[releases page](https://github.com/ArveLomsland/SyncthingStatus/releases) and run it.
It installs per user (no administrator rights required), with optional autostart at login.

Silent install:

```powershell
.\syncthing-status-0.1.0-setup.exe /SILENT /TASKS=startup
```

Uninstall from Settings → Apps, or run `unins000.exe /SILENT` in the install directory
(`%LOCALAPPDATA%\Programs\SyncthingStatus`).

### Ubuntu / Debian

Add the package repository once, then install and update through `apt`:

```bash
sudo install -d -m 0755 /etc/apt/keyrings
curl -fsSL https://arvelomsland.github.io/SyncthingStatus/key.gpg \
  | sudo tee /etc/apt/keyrings/syncthingstatus.gpg > /dev/null
echo "deb [signed-by=/etc/apt/keyrings/syncthingstatus.gpg] \
https://arvelomsland.github.io/SyncthingStatus stable main" \
  | sudo tee /etc/apt/sources.list.d/syncthingstatus.list
sudo apt update
sudo apt install syncthing-status
```

Or install a downloaded package directly, without automatic updates:

```bash
sudo apt install ./syncthing-status_0.1.0_amd64.deb
```

The package installs `/usr/bin/syncthing-status`, a menu entry and autostart
(`/etc/xdg/autostart/syncthing-status.desktop`).

> **GNOME users:** GNOME (the Ubuntu default) has no built-in system tray.
> Install the **AppIndicator and KStatusNotifierItem Support** extension:
> ```bash
> sudo apt install gnome-shell-extension-appindicator
> ```
> Log out and back in, then enable it in the Extensions app. KDE, XFCE, Cinnamon
> and MATE work without any add-on.

## Building

### Windows

```powershell
cargo build --release
# or including the installer:
powershell -ExecutionPolicy Bypass -File packaging\windows\build-installer.ps1
```

The installer requires Inno Setup: `winget install JRSoftware.InnoSetup`

The default MSVC toolchain works if you have the Visual Studio "C++ build tools" installed.
Without them, use the GNU toolchain instead
(`rustup override set stable-x86_64-pc-windows-gnu`) together with MinGW from WinLibs;
the build script adds MinGW to `PATH` automatically. If you install the MSVC build tools
later, remove the override with `rustup override unset`.

### Ubuntu / Debian

```bash
sudo apt install build-essential pkg-config libgtk-3-dev libayatana-appindicator3-dev
cargo build --release
# or build a .deb package:
./packaging/linux/build-deb.sh
```

GitHub Actions (`.github/workflows/release.yml`) builds both the installer and the .deb,
and publishes them as release assets when you push a `v*` tag. It also submits the new
version to winget and Scoop, and `apt-repo.yml` rebuilds the APT repository.
See [docs/PUBLISHING.md](docs/PUBLISHING.md) for the one-time setup of each channel.

## Configuration

Everything is optional – the app normally reads Syncthing's own `config.xml`
(`%LOCALAPPDATA%\Syncthing\config.xml`, or `%APPDATA%\Syncthing\config.xml`,
`~/.local/state/syncthing/config.xml`, `~/.config/syncthing/config.xml`,
`~/Library/Application Support/Syncthing/config.xml`).

| Environment variable  | Description                     | Default           |
| --------------------- | ------------------------------- | ----------------- |
| `SYNCTHING_URL`       | Address of the GUI/API          | from `config.xml` |
| `SYNCTHING_APIKEY`    | API key                         | from `config.xml` |
| `SYNCTHING_POLL_SECS` | Seconds between status refreshes| `3`               |
| `SYNCTHING_HOME`      | Directory containing `config.xml` | OS default      |

Self-signed HTTPS certificates (Syncthing with `tls="true"`) are accepted.

## Command-line flags

```bash
syncthing-status --status        # fetch status once and print it to the console
syncthing-status --preview       # render the icons as ASCII art
syncthing-status --no-promote    # do not pin the icon to the taskbar (Windows 11)
```

## API endpoints used

`/rest/system/version`, `/rest/config/folders`, `/rest/config/devices`,
`/rest/system/status`, `/rest/db/status`, `/rest/system/connections`,
`/rest/system/error`, `/rest/db/scan` (POST).

## Project layout

| File                               | Contents                                          |
| ---------------------------------- | ------------------------------------------------- |
| `src/main.rs`                      | Tray icon, menu, event loop, background thread    |
| `src/syncthing.rs`                 | REST client and status model                      |
| `src/config.rs`                    | Discovery of address / API key                    |
| `src/icon.rs`                      | Icons generated in code (no image files)          |
| `src/platform.rs`                  | Win32 and GTK message loops, error dialog         |
| `src/win_promote.rs`               | Pins the icon to the taskbar (Windows 11)         |
| `packaging/windows/*.iss`, `*.ps1` | Inno Setup installer                              |
| `packaging/linux/*`                | .desktop files, icon and `.deb` packaging         |
| `packaging/winget/*`               | winget manifests for the first submission         |
| `bucket/*.json`                    | Scoop manifest (this repo doubles as a bucket)    |

## Platform status

| Platform | Status |
| -------- | ------ |
| Windows 10/11 | Built and tested (Syncthing v2.1.3) |
| Ubuntu/Debian | Code and packaging ready, but **untested** – no Linux machine available |
| macOS | Likely compiles, but no event loop is implemented |

## License

MIT
