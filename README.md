*[Versione italiana qui](README.it.md)*

# Server Monitor Tray

A lightweight, native Linux tray application, written in Rust, for keeping an eye on your websites and servers. It runs quietly in the background, checks each site on its own schedule via HTTP or ping, and turns the tray icon red — with a desktop popup — the moment something goes down, so you don't have to keep a browser tab open or wait for someone else to tell you your site is offline.

## Screenshots

| Online | Offline | Settings |
|---|---|---|
| ![Tray icon, online](docs/screenshots/tray-online.png) | ![Tray icon, offline](docs/screenshots/tray-offline.png) | ![Settings window](docs/screenshots/settings.png) |

## Features

- Monitors multiple sites/hosts in parallel, each with its own check interval
- Two check types per site: **HTTP** (GET request) or **Ping** (ICMP via the system `ping` command)
- Green icon when all sites are reachable, blinking red icon if at least one is offline
- Tooltip listing every site and its status
- Left-click on the icon: window with the detailed status of each site
- Right-click on the icon: menu with "Settings..." (add/remove sites, change check type and interval) and "Quit"
- Saving the settings automatically restarts the applet, applying the new configuration immediately

## Requirements

- Rust (2024 edition, recent stable toolchain)
- GTK3 (development libraries): on Debian/Ubuntu `sudo apt install libgtk-3-dev`
- A desktop with system tray support via StatusNotifierItem (e.g. GNOME with a tray extension, KDE, Cinnamon, XFCE)
- The system `ping` command available in `PATH` (for Ping-type checks)
- `notify-send` available in `PATH` (part of `libnotify-bin` on Debian/Ubuntu), for desktop popup notifications on status changes

## Installation (recommended)

The `install.sh` script builds the project and installs the applet as a proper system application:

```bash
./install.sh
```

What it does, specifically:

- Builds in release mode (`cargo build --release`)
- Copies the binary to `~/.local/bin/server-monitor-tray`
- Copies the icon to `~/.local/share/icons/hicolor/128x128/apps/`
- Creates a default config at `~/.config/server-monitor-tray/watchdog.conf` (only if it doesn't already exist — an existing config is never overwritten)
- Creates a `.desktop` file in `~/.local/share/applications/`, so the app shows up in the application menu with its own name and icon
- Copies the same `.desktop` file to `~/.config/autostart/`, so the app starts automatically on every login

After installation:

- Search for **"Server Monitor Tray"** in the application menu to launch it manually, or
- Log out/in (or reboot) to have it start via autostart, or
- Launch it right away with `~/.local/bin/server-monitor-tray ~/.config/server-monitor-tray/watchdog.conf`

To change the monitored sites later, use the "Settings..." menu from a right-click on the tray icon — no need to re-run `install.sh` unless you've updated the code.

### Updating after a code change

```bash
cargo build --release
cp target/release/watchdog ~/.local/bin/server-monitor-tray
```

(or simply re-run `./install.sh`, which will not touch your existing config)

### Uninstallation

```bash
rm ~/.local/bin/server-monitor-tray
rm ~/.local/share/icons/hicolor/128x128/apps/server-monitor-tray.png
rm ~/.local/share/applications/server-monitor-tray.desktop
rm ~/.config/autostart/server-monitor-tray.desktop
rm -r ~/.config/server-monitor-tray   # also removes your custom config
```

## Manual build (without installation)

```bash
cargo build --release
./target/release/watchdog [path/to/config.conf]
```

If not specified, `watchdog.conf` in the current directory is used. An example file is provided at [`watchdog.conf.example`](watchdog.conf.example) — copy it to `watchdog.conf` and customize it.

## Configuration file format

```ini
site1_url = https://example.com
site1_type = http
site1_interval_secs = 10

site2_url = example.com
site2_type = ping
site2_interval_secs = 15

decoy1_url = 1.1.1.1
decoy2_url = 8.8.8.8

timeout_secs = 5
```

- `siteN_url`: URL (for `http`) or host/IP (for `ping`) of site N. The `http(s)://` scheme is automatically stripped for ping checks.
- `siteN_type`: `http` or `ping`.
- `siteN_interval_secs`: interval in seconds between two checks of that site.
- `decoyN_url`: always-reachable host (pinged via ICMP) used as a network sanity check. If a monitored site fails all its retries, the decoy sites are pinged: if none of them responds either, it's treated as a general network outage (blinking blue icon) rather than that specific site being down. Defaults to `1.1.1.1` and `8.8.8.8` if omitted.
- `timeout_secs`: timeout (in seconds) applied to all checks, also changeable live from Settings.

### Tray icon states

| Icon | Meaning |
|---|---|
| Solid green | All monitored sites are reachable |
| Blinking red | At least one monitored site is down (and the network itself is up) |
| Blinking blue | No decoy site is reachable — likely a general network/internet outage, not a problem with your sites |

The file can also be edited by hand; the `N` numbers must be consecutive starting from 1.

## License

MIT
