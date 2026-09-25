# Omarchy Widget Core

<img src="https://raw.githubusercontent.com/tcballard/omarchy-badges/75975e5b5bf75e7ede3764bcd2950046f7abfe2c/badges/v1/omarchy-app.svg" alt="Omarchy App" height="20">

A shared native desktop-widget host for Omarchy: fixed widget families, a cell occupancy grid, settings, and package management. Each widget lives in its own repository. Core runs each active package in its **own sandboxed Quickshell process**, with a trusted manager and state broker supervised by a user service. An optional compatibility plugin forwards old shell commands; it loads no widget code.

**v0.0.2 is experimental.** The earlier 0.1.0/0.1.1 numbers were premature. This intentional version reset preserves settings; 0.1.0 is reserved for the first supported baseline. No installed Omarchy version has been verified for this new runtime on a live desktop yet. Target: Omarchy Quattro / Hyprland. The badge is a community identity label, not official approval.

## Install or upgrade Core

Requires Omarchy, Quickshell (`qs`), systemd 254+ user services with cgroup v2 CPU/memory/pids controls, jq, Bubblewrap (`bubblewrap`), Git and a current stable Rust toolchain (Core requires 1.89+). The installer also builds a pinned wl-mitm Wayland proxy. The installer checks namespace support and actual resource enforcement before replacing the existing installation. There is no unsandboxed fallback. From this checkout:

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
| Medium | 2 × 1 | (384 + gapX) × 192 |
| Large | 2 × 2 | (384 + gapX) × (384 + gapY) |

Core inherits global desktop gaps. Arrange shows cell outlines; drag or use arrow keys to move one cell, and cycle only the sizes the widget supports. Explicit moves and resizes reject collisions without pushing other widgets. Theme scale affects content rather than the cell grid.

API 1 packages remain loadable through a compatibility facade, but their arbitrary dimensions become fixed families: compact → small, standard → medium, wide → large. Existing World Clock layouts may need Large until its individual repository adopts the new family and settings-editor contract. Its local persistence debugging work has not been overwritten.

## State and recovery

Core uses `$XDG_DATA_HOME/omarchy/widgets` and `$XDG_STATE_HOME/omarchy/widgets`, with standard home-directory defaults. The first write migrates layout version 1 and saves `layout-v1.backup.json`. Cities, appearance and enabled state are preserved. Pixel-only positions migrate once to preferred monitor/cell coordinates when desktop geometry is available; temporary display fallbacks do not overwrite them.

Settings saves complete only after the helper commits state. A revision conflict retains the editor draft and reports an error. The writer uses a kernel file lock released on process exit, atomic replacement and filesystem sync. Package versions remain on disk for rollback; abandoned versions are not automatically garbage-collected in this release.

`omarchy-widget hide INSTANCE_ID` keeps settings and position. `remove-instance INSTANCE_ID`
deletes only that instance. `uninstall PACKAGE_ID keep` unregisters the package and
keeps its instances hidden for a later reinstall; `uninstall PACKAGE_ID delete`
also deletes its instances. Cached code versions remain on disk; uninstall does
not promise disk reclamation or secure erasure. The legacy `remove PACKAGE_ID`
command remains an alias for uninstall with settings deletion.

`create PACKAGE_ID small|medium|large` adds a fresh instance from defaults;
`duplicate INSTANCE_ID` copies an existing instance's settings into an independent
identity. `add INSTANCE_ID` shows a hidden instance. Legacy `add PACKAGE_ID` still
creates or shows that package's original placement. `stop`, `start`, `restart` and
`hide-all` control Core and its widget runners.

To uninstall the runtime while preserving widget data:

```bash
systemctl --user disable --now omarchy-widget-host.service
omarchy plugin disable io.github.tcballard.widget-core
```

Then remove the Core installation, launcher and service file if desired. Core's installer prints the backup location. Restore that copy to roll back Core itself. After layout migration, old Core needs the backed-up v1 layout; restoring that backup loses settings changes made since migration, so preserve the current layout first.

## Security and verification

Each package runs in a **mandatory Bubblewrap island** with read-only code, private temporary storage and no direct registry, home, network or session-bus access. A package-scoped broker mediates settings and layout changes. Its Wayland connection passes through a pinned allowlist proxy that blocks capture, clipboard and virtual-input protocols, and rejects overlay layers and exclusive keyboard capture. `omarchy-widget sandbox-check` tests namespace startup.

This development boundary is not independently audited. Each package has its own 256 MiB memory ceiling, 25% CPU quota and 64-task limit, including its proxy and descendants. Core has a separate budget. Allowed surface protocols do not enforce widget geometry against malicious clients. Network widgets need a future broker/permission model. Software rendering avoids exposing GPU devices. Theme values arrive through Core's broker. See the [runtime decision](docs/runtime-and-security.md) for the exact boundary and remaining limits.

Portable verification runs Rust tests, rustfmt, Clippy, Qt component checks and a production QML settings/queue → real Rust CLI → disk/reopen integration test. Wayland, restart, theme switching, fractional scaling and multiple monitors still require the [desktop checks](docs/desktop-checks.md).

See the [widget contract](docs/widget-contract.md), [appearance contract](docs/widget-appearance.md), [review and assumptions](docs/assumptions.md), and [verification record](docs/verification.md).

## Manager shortcut and workspace assignment (development)

**Super+Space → Widgets** opens the manager through the application launcher. The installer adds a `Widgets` desktop entry.

The installer attempts to add **Super+Alt+W → Desktop widgets** to the personal Hyprland bindings file. It supports Omarchy's Lua bindings and older `.conf` bindings, checks the active key map, preserves existing content with a backup and restores it if the binding does not activate. It never replaces a conflicting shortcut. If skipped, run `omarchy-widget bind-key` inside the desktop session or assign `omarchy-widget manage` yourself. Remove the single generated binding line (and its comment) to undo it, then reload Hyprland.

In the manager, set each instance's **Workspace** to `all` or a number from `1` to `9999`. Existing instances default to all workspaces. For example:

```bash
omarchy-widget workspace io.github.tcballard.worldclock 2
omarchy-widget workspace io.github.tcballard.worldclock all
```

The widget remains on its configured monitor and appears only while that monitor's active numbered workspace matches. It does not follow a workspace to a different monitor. Special workspace overlays leave the underlying numbered-workspace assignment unchanged. Duplicate instances inherit their initial assignment and can then be configured independently. Settings revisions and clock settings are unaffected.

Trusted Core queries Hyprland's local monitor IPC and supplies bounded monitor/workspace data, cell geometry and frame metrics through snapshots. The raw compositor socket is still absent from widget sandboxes. Visibility follows the normal approximately one-second refresh, with a short shared query cache. When desktop geometry is unavailable, all instances remain unplaced and the manager reports the problem. This is cooperative presentation, not a new security restriction on hostile Wayland clients.

This development branch remains **v0.0.2**. Real Hyprland shortcut activation, monitor hotplug and workspace switching need XPS acceptance. The network-widget direction is documented in [network-widgets.md](docs/network-widgets.md); network access is not enabled by this change.

To remove the launcher entry, delete `io.github.tcballard.widget-core.desktop` from `${XDG_DATA_HOME:-$HOME/.local/share}/applications`. This does not remove widgets or their settings.

## Building a widget

Follow the [widget authoring standard](docs/widget-authoring.md) alongside the API contract. It separates enforced package/runtime rules from the design, accessibility, lifecycle and verification conventions expected at review.

## Cell layout and inherited ricing (development)

Small, Medium and Large occupy 1×1, 2×1 and 2×2 cells. Core inherits global Hyprland gaps, border size and rounding; disabled widgets release their cells. Arrange mode previews footprints and rejects occupied drops. Different numbered workspaces can reuse cells; an all-workspace instance reserves them everywhere.

Preferred monitor/cell coordinates survive monitor removal and resolution/gap changes. Core temporarily finds another available slot or marks the instance unplaced, then restores the preference when available. See [layout/API details](docs/grid-layout.md) and [verification](docs/grid-verification.md). Core remains v0.0.2; real desktop acceptance is pending.


## Widgets manager (development)

Open `omarchy-widget manage` or **Super+Space → Widgets**. **Available** lists
installed widget types once each, with size choices, a preview and Add widget.
Add starts from defaults; it does not replace or copy an existing instance.
This is a local installed-package gallery, not an online store.

**Your widgets** lists individual instances, their monitor/workspace, size and
visibility. Configure, Arrange, Duplicate, Hide/Show and Remove act from here.
Remove asks before deleting one instance. Uninstall offers an explicit choice to
keep or delete all of a package's instance settings. Kept instances remain visible
in the manager as uninstalled and can be removed individually; reinstall leaves
them hidden until Show is chosen.

Packages can provide bounded static PNG previews for supported families. Without
an image, Core labels the grid footprint as a size preview. Third-party widget
QML is never loaded by the manager to generate a preview. Broken packages remain
listed with an error and an Uninstall action.

See [manager verification](docs/manager-verification.md) and the
[implementation stack](docs/implementation-stack.md). Portable Qt tests exercise
real mouse actions through the production controller and Rust registry. Live
Omarchy focus, keyboard navigation, scaling and monitor acceptance remain pending.
