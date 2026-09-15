# Desktop acceptance · v0.0.2

Not run in the remote development environment. Record `omarchy-version`, Quickshell/Qt versions, GPU, monitor layout/scales, theme, tested Core commit, exact steps and observations. Portable CI is not a substitute for these checks.

1. Upgrade an actual 0.1.1 installation with custom cities/appearance. Run the one-time shell restart. Confirm the backup, preserved settings and only one visible copy of each widget.
2. Confirm `systemctl --user status omarchy-widget-host` identifies a different process from the Omarchy shell. Kill only the widget service's main process; observe bounded restart and an unaffected bar/menu. Stop the service and confirm all widget helper children stop.
3. Install `examples/notes`, add it, edit via Core's settings panel, Save, restart the host and reopen. Cancel must not persist; stale revisions must produce an error and retain the draft. Test keyboard navigation, paste and IME in the editor.
4. Install the existing World Clock. Verify its stored cities survive migration; use Large if its legacy layout needs room. Re-test its actual city editor, acknowledging that individual widget code is maintained separately.
5. Drag and use arrow keys in all three sizes. Confirm snap dots, monitor-local placement, no input interception outside cards, usable controls and no free resize. Test screen edges with top/bottom/side bars, unplug/replug, rotated displays and fractional scaling. Check whether compositor exclusion zones and Core clamping agree.
6. Switch between a light Windows Familiar theme and a dark theme while widgets remain open. Check text contrast, radius, rounded clipping, default padding and a `widgets.json` override. Verify refresh within five seconds; malformed theme data uses defaults.
7. Update a widget while its settings editor is open. Confirm code reload and error behaviour; do not silently discard a dirty editor as an acceptable result. Test valid rollback, invalid package rejection, interrupted install and full-disk failures.
8. Verify login/autostart, duplicate-start refusal, stop/start/restart/status/log commands, installer activation failure recovery and preservation of data when the runtime is disabled.
9. Confirm fullscreen handling and hidden widgets suspend work. Measure idle CPU/wakeups with multiple widgets.

Any failure in installation, persistence, process separation or settings acknowledgement blocks calling this release ready for normal use. Per-package security isolation is not claimed or tested by this checklist.
