# Desktop acceptance · v0.0.2

Use the [desktop review harness](test-harness.md) to run the portable suite, collect footprint/CPU/memory evidence and record each manual result in a resumable report. The harness adds no installed runtime dependency.

Not run in the remote development environment. Record `omarchy-version`, Quickshell/Qt versions, GPU, monitor layout/scales, theme, tested Core commit, exact steps and observations. Portable CI is not a substitute for these checks.

1. Upgrade an actual 0.1.1 installation with custom cities/appearance. Run the one-time shell restart. Confirm the backup, preserved settings and only one visible copy of each widget.
2. Confirm `systemctl --user status omarchy-widget-host` identifies a different process from the Omarchy shell. Kill only the widget service's main process; observe bounded restart and an unaffected bar/menu. Stop the service and confirm all widget helper children stop.
3. Install `examples/notes`, add it, edit via Core's settings panel, Save, restart the host and reopen. Cancel must not persist; stale revisions must produce an error and retain the draft. Test keyboard navigation and closing/reopening the editor. Clipboard and external IME access are deliberately unavailable under the current policy.
4. Update World Clock from API 2 to API 3. Verify its stored cities survive and re-test its actual city editor. API 1 packages are unsupported.
5. Drag and use arrow keys in all three sizes. Confirm snap dots, monitor-local placement, no input interception outside cards, usable controls and no free resize. Test screen edges with top/bottom/side bars, unplug/replug, rotated displays and fractional scaling. Check whether compositor exclusion zones and Core clamping agree.
6. Switch between a light Windows Familiar theme and a dark theme while widgets remain open. Check text contrast, radius, rounded clipping, default padding and a `widgets.json` override. Verify changes to the selected theme refresh through broker snapshots within about one second and malformed theme data uses defaults.
7. Update a widget while its settings editor is open. Confirm code reload and error behaviour; do not silently discard a dirty editor as an acceptable result. Test valid rollback, invalid package rejection, interrupted install and full-disk failures.
8. Verify login/autostart, duplicate-start refusal, stop/start/restart/status/log commands, installer activation failure recovery and preservation of data when the runtime is disabled.
9. Confirm widgets remain behind fullscreen windows and explicit hiding suspends widget content. Automatic fullscreen detection is not implemented; runners still have no Hyprland command socket. Measure idle CPU/wakeups with multiple widgets.

Any failure in installation, persistence, process separation or settings acknowledgement blocks calling this release ready for normal use. Run `omarchy-widget sandbox-check` before graphical checks. Verify networking is unavailable and the application renders with Qt software rendering. Portable tests cover package-scoped broker authority and actual Wayland proxy enforcement; this checklist covers their integration with the real desktop.

10. Enable two different packages. Confirm separate runner processes; stop one runner and verify the other remains visible while the failed runner restarts within its budget. Duplicate an instance and confirm instances of that package share one runner.
11. Open settings from the manager for an enabled and a hidden instance. Save, cancel, close using the window manager, and reopen. Confirm the editor does not reappear after restart following a successful close. Inspect logs for rejected legitimate layer-shell requests.
12. Stop Core and confirm manager, proxies, policy hooks and runner descendants are all gone. Update/rollback one package and verify its old generation no longer serves settings requests.

13. Inspect each `omarchy-widget-island-*.service`: `MemoryMax=268435456`, `MemorySwapMax=0`, `CPUQuotaPerSecUSec=250ms`, `TasksMax=64`, `OOMPolicy=kill`. Confirm its cgroup is a sibling of Core and includes both proxy and widget descendants. Measure normal startup, idle and edit usage for World Clock before treating these default budgets as tuned.
14. Confirm a package exhausting memory stops only that package, observes the bounded retry policy, and leaves the manager and another package usable. CPU pressure should throttle; task pressure should reject new tasks. These destructive workload fixtures run in disposable CI; do not deliberately exhaust your normal desktop to repeat them. Verify normal Core stop/restart cleans up all transient services on the XPS.

15. Invoke reveal separately from the manager. Check Escape, timeout, fullscreen,
    focus return, multi-output stacking and no application position changes.
    Lock during reveal; verify no widget content/input appears on the lock screen.
    Confirm elevated surfaces disappear when the lease ends, including a faulty
    client that refuses to demote. Record the compositor version and result.
16. Install `examples/weather`, grant its installed version permission, add two
    instances at the same coordinates and observe shared refresh. Revoke, update,
    disconnect networking, change workspace and resume. Confirm loading, stale
    last-known data, permission errors and bounded retries without raw renderer
    network access. This is the outstanding live provider test.
17. Create a widget using the SDK. Add two instances with different settings;
    update with a migration, roll back before edits, then verify rollback refuses
    to overwrite a later incompatible edit. Kill/restart only its package and
    verify the manager and another package remain usable. Finish settings before
    update; confirm an open editor blocks code replacement.

18. Click a passive clock while an application has keyboard focus. Widget surfaces request on-demand keyboard focus even outside arrange mode: record the focus transfer, how focus returns, gear Tab/Return and action-button behaviour. This is an unverified desktop behaviour change, not evidence that passive clicks preserve application focus.
19. Save under sustained registry contention. Record total latency, the retry notice, draft retention and absence of duplicate writes. The CLI retries pre-dispatch busy refusals for two seconds; the QML queue permits four more attempts with 0.2/0.4/0.8/1.0-second backoffs. Repeated two-second refusals therefore take about 12.4 seconds, plus dispatch overhead. Rate-limit refusals can terminate CLI attempts earlier. Unknown outcomes/timeouts are not replayed. Decide whether measured delay is acceptable before release.
20. Kill an editing runner, then hold the registry unavailable in a disposable fixture. Confirm Available reports recovery-blocked with the actual registry error and repair guidance, then automatically resumes recovery once the registry is usable. A completely unreadable layout may also prevent the manager from listing packages; inspect the supervisor journal in that case.

21. Idle resource regression: close Widgets and settings, confirm the manager Quickshell process exits while World Clock keeps ticking. Open/close Widgets three times; launch settings from the manager and confirm the request survives manager exit. Enter/leave arrange and (if enabled) reveal. Restart Core twice: exactly one active island remains per enabled package, with no previous supervisor generation revived. Repeat the 60-second resource capture under the same conditions as the September 17 baseline. Hide/show widgets and unplug/replug a monitor with no visible/screen binding-loop warnings.
