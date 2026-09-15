# Omarchy Widget Core

A shared native host for desktop widgets on Omarchy. Core provides the desktop surfaces, an installed-widget manager, placement controls and persistent settings. Each widget lives in its own repository and declares a separate `widget.json` contract.

**Experimental foundation — 0.1.0.** No individual widgets are bundled. This is a working starting point for trying the architecture on a real desktop, not an official Omarchy subsystem or marketplace.

## What is here

- Native Quickshell/QML host with one card-sized bottom-layer Wayland surface per added widget.
- Installed-widget manager: add, hide, refresh and enter arrangement mode.
- Drag or keyboard placement, supported-size cycling and monitor selection.
- Rust `omarchy-widget` CLI: validate, install local package snapshots, list, add, hide, configure, place and remove.
- Separate widget package manifest and versioned host context.
- Tests for registry operations and native Qt component rendering, plus GitHub Actions.

Core currently uses Quattro's service loader to start inside the existing shell process. Its root `manifest.json` is that small bootstrap adapter. Individual widgets use **`widget.json`**, install into a widget registry, and do not register as shell plugins. A future first-class Omarchy integration can replace the bootstrap while preserving the widget contract.

## Install Core

Requires an Omarchy **Quattro** checkout with its plugin/service API, Quickshell, Rust/Cargo, `jq` and GNU coreutils. This does not target older Omarchy shells.

```bash
git clone https://github.com/tcballard/omarchy-widget-core.git
cd omarchy-widget-core
bash install-local
omarchy-shell io.github.tcballard.widget-core manage
```

The installer builds the locked Rust dependencies, validates Core's bootstrap, copies the runtime into `~/.config/omarchy/plugins/io.github.tcballard.widget-core`, rescans and enables it. It refuses to replace an existing installation. Cloning or installing Core alone does not install any widgets.

The built CLI is `./target/release/omarchy-widget`. The installed copy is `~/.config/omarchy/plugins/io.github.tcballard.widget-core/bin/omarchy-widget`; use either path. The installer does not change your PATH.

## Use a widget package

Once an individual widget repo is available:

```bash
# Use a package checkout you have reviewed and trust.
./target/release/omarchy-widget validate /absolute/path/to/widget-repo
./target/release/omarchy-widget install /absolute/path/to/widget-repo
./target/release/omarchy-widget list
omarchy-shell io.github.tcballard.widget-core manage
```

Choose **Add** in the manager to load the widget. Installation only copies a snapshot; it does not run the widget. The manager refreshes when opened. After CLI changes, refresh an already-open host explicitly:

```bash
omarchy-shell io.github.tcballard.widget-core refresh
```

In **Arrange** mode, drag a card's header, cycle its supported sizes, move it between monitors or hide it. Focus the card and use arrow keys to move 20 logical pixels, Shift+arrow for 1 pixel, Escape to finish. Positions are clamped to the connected screen; a missing preferred monitor falls back to the first screen without erasing the preference.

Other host commands:

```bash
omarchy-shell io.github.tcballard.widget-core arrange
omarchy-shell io.github.tcballard.widget-core hide
omarchy-shell io.github.tcballard.widget-core show
omarchy-shell io.github.tcballard.widget-core status
```

`hide` suspends all widget views for the session. Per-widget Hide persists its placement and settings while removing it from the desktop.

## Storage and lifecycle

| Data | Default location |
| --- | --- |
| Installed package snapshots | `~/.local/share/omarchy/widgets/packages/<id>/` |
| Placement and settings | `~/.local/state/omarchy/widgets/layout.json` |
| Core bootstrap and binary | `~/.config/omarchy/plugins/io.github.tcballard.widget-core/` |

`XDG_DATA_HOME` and `XDG_STATE_HOME` override the widget registry roots; they must be absolute paths. The bootstrap follows the current Quattro plugin directory. There is one placement per widget ID in this first version. A World Clock is therefore one package and one card containing multiple timezone rows.

Use `omarchy-widget help` for the JSON command list. All CLI responses are JSON; failures exit nonzero. `configure ID JSON` replaces the settings object. `place ID JSON` requires `x`, `y`, `monitor` and `size`. `remove ID` deletes both the package and its saved settings; use `hide` to retain them.

To replace Core, disable `io.github.tcballard.widget-core` using `omarchy plugin disable`, move its installation directory aside, then rerun `bash install-local`. Widget packages and layout live separately and survive this. There is no automatic update or binary release channel yet.

If a process crashes during a registry write, a subsequent command may report a stale `.operation-lock`. Confirm no `omarchy-widget` process is running before removing that empty directory from the widget data root. Do not delete the layout to recover a lock. Unsupported or malformed layout files cause commands to fail and remain available for recovery.

## Build and check

```bash
cargo test --locked
cargo clippy --locked --all-targets -- -D warnings
python3 -m pip install 'PySide6==6.11.2'
python3 tests/qml_smoke.py
```

The Qt test parses the complete host and renders the production manager using explicit fixtures for Quattro's theme tokens and buttons. It checks empty/populated states and Escape handling. It does **not** run a Wayland compositor. Live Omarchy checks remain in [docs/desktop-checks.md](docs/desktop-checks.md).

## Package authors

Read [the widget contract](docs/widget-contract.md) and [the architecture assumptions](docs/assumptions.md). The contract is experimental and versioned; independent widgets should pin the Core version they test against.

QML runs in the shell process with the user's privileges. The scoped context is an API boundary, **not a security sandbox**. Add only trusted packages. Package validation checks structure, paths and size limits; it does not audit code or block a widget from importing process/network APIs.

Local snapshot installation is the only package transport implemented. Catalogue discovery, remote install/update, signed releases, authoring skills and official Omarchy integration are follow-on work after real widget testing.

MIT licensed.
