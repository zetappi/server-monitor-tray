# Changelog

## Unreleased

- **Notification popups now auto-close after 15 seconds**. Both "site down" and "site back online" notifications use normal urgency with a 15s expire time; previously "site down" used critical urgency, which several notification daemons (including Cinnamon's) keep on screen indefinitely regardless of the expire time.
- **Fix**: opening Settings repeatedly no longer opens duplicate windows; the existing one is brought to the front instead (same fix already applied to the Site Status window).
- **Decoy sites / network outage detection**: configurable always-reachable hosts (`decoyN_url`, pinged via ICMP; default `1.1.1.1` and `8.8.8.8`) are checked whenever a monitored site fails all its retries. If no decoy responds either, it's treated as a general network outage — the tray icon blinks blue instead of red, no false "site down" notification is sent, and the site's last known status is preserved until the network check succeeds again. Editable from Settings.
- **Fix (false alarms)**: a single failed check no longer marks a site offline. Up to 3 attempts are made, 3 seconds apart, before a site is actually considered down — this absorbs brief timeouts/hiccups that aren't real outages.
- **Fix**: clicking the tray icon now reliably brings the "Site Status" window to the front (deiconify + present), even if it was minimized or behind other windows.
- **Desktop notification on status change**: a popup (via `notify-send`) is shown whenever a monitored site transitions online→offline (critical urgency) or offline→online (normal urgency). No notification is sent on startup or while the status stays unchanged. Requires `notify-send` (part of `libnotify-bin` on Debian/Ubuntu).
- **Fix**: clicking the tray icon repeatedly no longer opens duplicate "Site Status" windows; the existing window is brought to the front instead.
- **UI translated to English**: all visible interface text (window titles, buttons, menu entries, labels, the `.desktop` description) is now in English. Internal source code comments remain in Italian; the config file format is unchanged (keys were already in English: `url`, `type`, `interval_secs`, `timeout_secs`).
