# Omarchy Widget Core

<img src="https://raw.githubusercontent.com/tcballard/omarchy-badges/75975e5b5bf75e7ede3764bcd2950046f7abfe2c/badges/v1/omarchy-app.svg" alt="Omarchy App" height="20">

A shared native desktop-widget host for Omarchy: fixed widget families, a snap grid, settings, and package management. Each widget lives in its own repository. Core runs their QML in a **separate Quickshell process**, supervised by a user service. An optional compatibility plugin forwards old shell commands; it loads no widget code.

**v0.0.2 is experimental.** The earlier 0.1.0/0.1.1 numbers were premature. This intentional version reset preserves settings; 0.1.0 is reserved for the first supported baseline. No installed Omarchy version has been verified for this new runtime on a live desktop yet. Target: Omarchy Quattro / Hyprland. The badge is a community identity label, not official approval.

## Install or upgrade Core

Requires Omarchy, Quickshell (`qs`), systemd user services, jq and Rust 1.89+. From this checkout:

```bash
bash install-local --update
omarchy restart shell
omarchy-widget manage
```

Use `bash install-local` for a first installation. The one-time shell restart replaces the old in-process service. Subsequent host restarts use `omarchy-widget restart`; they do not restart the Omarchy shell. The launcher is installed in `~/.local/bin`.

## Install and manage widgets

Use Core for package changes, including updates to already-installed widgets:

```bash
omarchy-widget install /path/to/widget
omarchy-widget add io.example.widget
omarchy-widget update /path/to/widget
omarchy-widget rollback io.example.widget
omarchy-widget arrange
omarchy-widget status
omarchy-widget logs
```

Do not use an old widget's `install-local --update` script to copy registry directories. Core now tracks complete package versions and switches the active version atomically. Updating or rolling back code preserves all instance settings. Code rollback cannot undo a widget-specific settings-schema migration.

To exercise all three sizes and the shared settings editor without another repository:

```bash
omarchy-widget install ./examples/notes
omarchy-widget add io.github.tcballard.core-example-notes
```

This is a development fixture, not a new product widget.

## Widget families

| Family | Grid cells | Logical dimensions |
|---|---|---|
| Small | 1 × 1 | 192 × 192 |
| Medium | 2 × 1 | 400 × 192 |
| Large | 2 × 2 | 400 × 400 |

The gap and snap increment are 16 logical pixels. Core applies theme scaling. Arrange shows grid dots; drag or use arrow keys to move, and cycle only the sizes the widget supports. Free resizing is intentionally absent. Overlap is currently allowed; snapping does not automatically pack or push other widgets.

API 1 packages remain loadable through a compatibility facade, but their arbitrary dimensions become fixed families: compact → small, standard → medium, wide → large. Existing World Clock layouts may need Large until its individual repository adopts the new family and settings-editor contract. Its local persistence debugging work has not been overwritten.

## State and recovery

Core uses `$XDG_DATA_HOME/omarchy/widgets` and `$XDG_STATE_HOME/omarchy/widgets`, with standard home-directory defaults. The first write migrates layout version 1 and saves `layout-v1.backup.json`. Cities, appearance and enabled state are preserved. Monitor positions are clamped for display; new placement writes snap to the grid.

Settings saves complete only after the helper commits state. A revision conflict retains the editor draft and reports an error. The writer uses a kernel file lock released on process exit, atomic replacement and filesystem sync. Package versions remain on disk for rollback; abandoned versions are not automatically garbage-collected in this release.

`omarchy-widget hide INSTANCE_ID` keeps settings. `remove PACKAGE_ID` unregisters the package and removes its instance settings; retained code is not securely erased. `duplicate INSTANCE_ID` creates independently configurable instances. `stop`, `start`, `restart` and `hide-all` control the shared host.

To uninstall the runtime while preserving widget data:

```bash
systemctl --user disable --now omarchy-widget-host.service
omarchy plugin disable io.github.tcballard.widget-core
```

Then remove the Core installation, launcher and service file if desired. Core's installer prints the backup location. Restore that copy to roll back Core itself. After layout migration, old Core needs the backed-up v1 layout; restoring that backup loses settings changes made since migration, so preserve the current layout first.

## Security and verification

The separate process provides crash separation from the shell. **It is not a security sandbox.** Installed QML is trusted code with your user account's access; widgets sharing the host are not isolated from one another. The [runtime decision](docs/runtime-and-security.md) describes the per-package sandbox architecture needed for untrusted marketplace widgets.

Portable verification runs Rust tests, rustfmt, Clippy, Qt component checks and a production QML settings/queue → real Rust CLI → disk/reopen integration test. Wayland, restart, theme switching, fractional scaling and multiple monitors still require the [desktop checks](docs/desktop-checks.md).

See the [widget contract](docs/widget-contract.md), [appearance contract](docs/widget-appearance.md), [review and assumptions](docs/assumptions.md), and [verification record](docs/verification.md).
