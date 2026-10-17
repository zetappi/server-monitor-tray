# Changelog

## Unreleased

- **Desktop notification on status change**: a popup (via `notify-send`) is shown whenever a monitored site transitions online→offline (critical urgency) or offline→online (normal urgency). No notification is sent on startup or while the status stays unchanged. Requires `notify-send` (part of `libnotify-bin` on Debian/Ubuntu).
- **Fix**: clicking the tray icon repeatedly no longer opens duplicate "Site Status" windows; the existing window is brought to the front instead.
- **UI translated to English**: all visible interface text (window titles, buttons, menu entries, labels, the `.desktop` description) is now in English. Internal source code comments remain in Italian; the config file format is unchanged (keys were already in English: `url`, `type`, `interval_secs`, `timeout_secs`).
