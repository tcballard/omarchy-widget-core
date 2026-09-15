# Desktop acceptance · v0.0.2

Not run in the remote development environment. Record `omarchy-version`, Quickshell/Qt versions, GPU, monitor layout/scales, theme, tested Core commit, exact steps and observations. Portable CI is not a substitute for these checks.

1. Upgrade an actual 0.1.1 installation with custom cities/appearance. Run the one-time shell restart. Confirm the backup, preserved settings and only one visible copy of each widget.
2. Confirm `systemctl --user status omarchy-widget-host` identifies a different process from the Omarchy shell. Kill only the widget service's main process; observe bounded restart and an unaffected bar/menu. Stop the service and confirm all widget helper children stop.
3. Install `examples/notes`, add it, edit via Core's settings panel, Save, restart the host and reopen. Cancel must not persist; stale revisions must produce an error and retain the draft. Test keyboard navigation and closing/reopening the editor. Clipboard and external IME access are deliberately unavailable under the current policy.
4. Install the existing World Clock. Verify its stored cities survive migration; use Large if its legacy layout needs room. Re-test its actual city editor, acknowledging that individual widget code is maintained separately.
5. Drag and use arrow keys in all three sizes. Confirm snap dots, monitor-local placement, no input interception outside cards, usable controls and no free resize. Test screen edges with top/bottom/side bars, unplug/replug, rotated displays and fractional scaling. Check whether compositor exclusion zones and Core clamping agree.
6. Switch between a light Windows Familiar theme and a dark theme while widgets remain open. Check text contrast, radius, rounded clipping, default padding and a `widgets.json` override. Verify changes to the selected theme refresh through broker snapshots within about one second and malformed theme data uses defaults.
7. Update a widget while its settings editor is open. Confirm code reload and error behaviour; do not silently discard a dirty editor as an acceptable result. Test valid rollback, invalid package rejection, interrupted install and full-disk failures.
8. Verify login/autostart, duplicate-start refusal, stop/start/restart/status/log commands, installer activation failure recovery and preservation of data when the runtime is disabled.
9. Confirm widgets remain behind fullscreen windows and explicit hiding suspends widget content. Automatic fullscreen detection is not available without the blocked Hyprland command socket. Measure idle CPU/wakeups with multiple widgets.

Any failure in installation, persistence, process separation or settings acknowledgement blocks calling this release ready for normal use. Run `omarchy-widget sandbox-check` before graphical checks. Verify networking is unavailable and the application renders with Qt software rendering. Portable tests cover package-scoped broker authority and actual Wayland proxy enforcement; this checklist covers their integration with the real desktop.

10. Enable two different packages. Confirm separate runner processes; stop one runner and verify the other remains visible while the failed runner restarts within its budget. Duplicate an instance and confirm instances of that package share one runner.
11. Open settings from the manager for an enabled and a hidden instance. Save, cancel, close using the window manager, and reopen. Confirm the editor does not reappear after restart following a successful close. Inspect logs for rejected legitimate layer-shell requests.
12. Stop Core and confirm manager, proxies, policy hooks and runner descendants are all gone. Update/rollback one package and verify its old generation no longer serves settings requests.
