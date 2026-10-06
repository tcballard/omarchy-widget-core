# Core-managed MyGo GitHub trial

This development branch adds a native MyGo renderer to Widget Core. Core still owns placement, occupancy, settings, permissions, public GitHub transport/cache, process limits and the Wayland filter. The MyGo process applies Core's resolved rectangles to GTK layer-shell surfaces and paints the calendar. There is no separate layout store or direct network client.

The trial has its own package ID, `io.github.tcballard.github-contributions-mygo`. Your existing GitHub widget and its settings stay available for comparison. This is an experimental renderer, not a released replacement. MyGo v0.2.11 is pinned; do not upgrade it without retesting pre-realization layer-shell attachment.

## Install for testing

Run on your Omarchy desktop. This builds and replaces the installed Core with the trial branch, using Core's existing transactional installer and backup. It does not merge either pull request.

```bash
sudo pacman -S --needed go gtk3 gtk-layer-shell

git clone --branch experiment/core-mygo-github --single-branch \
  https://github.com/tcballard/omarchy-widget-core.git omarchy-widget-core-mygo-trial
cd omarchy-widget-core-mygo-trial
OMARCHY_WIDGET_EXPERIMENTAL_MYGO=1 bash install-local --update

omarchy-widget install "$PWD/examples/mygo-github"
omarchy-widget github-permission io.github.tcballard.github-contributions-mygo allow
omarchy-widget add io.github.tcballard.github-contributions-mygo
omarchy-widget manage
```

For a first Core installation, omit `--update`. Existing Core prerequisites still apply: Rust, Quickshell, Bubblewrap, systemd user resource controls, Git, curl and jq. The MyGo build requires Go 1.27.1 or newer. The initial build downloads pinned Go dependencies and builds Core's pinned Wayland filter.

The CI artifact contains a Linux amd64 native host and synthetic previews for inspection. The host requires the Core sandbox and broker; do not launch it as a standalone app. The commands above build the complete installation from source.

## Test later

1. Compare the existing GitHub widget and **GitHub Contributions · MyGo trial**. Confirm the default `tcballard` calendar appears after granting access. Try another username using the gear or manager Settings, then restart Core and check it persists.
2. Choose **Arrange** in Widgets. Drag the native widget's **Move GitHub** handle and release to commit a snapped position. Focus that handle and use arrow keys for single-cell moves. Try occupied cells and screen edges: Core should reject the move and keep the previous placement. The trial displays a movement hint during the drag; the surface moves after Core acknowledges the drop.
3. Use **Size** to cycle small, medium and large, and **Screen** to request the next connector. Screen selects cell 0,0; an occupied target is rejected. The manager's placement recovery is also available. Choose **Done** or Escape to finish arranging.
4. Duplicate the trial widget in the manager. Change one username and check the other instance is independent. Hide/show one instance, toggle all widgets, and run `omarchy-widget restart`.
5. Assign a workspace in the manager. Change workspaces and reconnect an external monitor. Confirm the native widget follows Core's fallback and restoration behavior. Connector names must match exactly; an unknown connector hides the surface instead of guessing a monitor.
6. Switch Omarchy themes and test your normal display scale. The native view consumes Core's palette plus desktop radius/border. This pilot does not implement every optional QML frame appearance override.
7. Revoke GitHub access in the manager: the calendar should disappear and show the permission error. Re-enable it. Test offline refresh: Core retains stale data for the same account. Changing accounts must never show the previous account's graph.

Small shows the latest 13 weeks; medium shows the annual calendar; large splits it across two rows. Hover a cell for its date/count. Settings stay in Core's generated editor. Opening a public profile and keyboard navigation within the graph are not included in this pilot. No glass effect is enabled.

Record `omarchy-version`, display scale and the failing step if something behaves incorrectly. Diagnostics:

```bash
omarchy-widget status
omarchy-widget logs
journalctl --user -u 'omarchy-widget-*' --since '10 minutes ago' --no-pager
```

## Remove the trial / restore Core

Remove the separate test package while keeping its settings for a later attempt:

```bash
omarchy-widget uninstall io.github.tcballard.github-contributions-mygo keep
```

To return Core itself to the pre-trial version, from this checkout:

```bash
git switch --detach e1b8c1582d0a39cafe9a46add383b7651c6fa649
bash install-local --update
```

The regular GitHub package is untouched. Core's installer also prints a backup path and restores it automatically if activation fails. A later normal Core install omits the experimental native host unless the MyGo flag is supplied again.

## Verification and boundaries

Locally reproduced on the Linux build workspace with Go 1.27.1 and Rust 1.98.1: native build, Go race tests for view/actions/visibility/layer policy, Go vet/module verification, Core Rust build/clippy, parser regression and captured 367-day GitHub response, existing sandbox argv and transactional installer tests, and actual native headless PNG rendering at all three sizes. Synthetic previews are not Omarchy screenshots.

The local full Rust suite encountered four Unix-socket tests blocked by the workspace's `EPERM` restriction. The native broker socket tests likewise skip on that restriction; CI executes them on an ordinary runner. Local QML smoke testing requires PySide6, unavailable in this workspace. The existing Core CI runs these suites. The additional MyGo CI runs real GTK layer surfaces in an isolated Sway headless compositor, asserting the layer policy, resolved size/margins, duplicate creation, hiding and absence of an ordinary xdg toplevel. Check the PR's current CI result before installing.

Not observed here: your Omarchy/Hyprland session, real pointer/focus behavior, fractional scaling, monitor reconnects, or performance under production cgroup limits. The headless compositor test is a native surface test with a scoped broker fixture; production sandbox/resource enforcement remains covered separately by Core's existing suites. No performance improvement is claimed.

MyGo is MIT licensed; Go module dependencies and licenses are collected by `build-mygo`. GTK3 and gtk-layer-shell are dynamically loaded system libraries. The adapter uses `gdk_screen_get_monitor_plug_name` for connector identity and `gtk_layer_init_for_window` before realization. It refuses unsupported layer-shell or an unexpectedly realized MyGo window. MyGo's newer frameless-window implementation requires revisiting this hook.
