# Shortcut and workspace development verification

Base: Core PR #2 at `6ca84013d0e2f28e54d6ddae34a1242d17551794`.
Omarchy binding reference: `86a2e5830eae4d660a66df8cf37f0a35bf4fe8a2` (Lua configuration). Hyprland's [IPC documentation](https://wiki.hypr.land/IPC/) confirms the local control socket and instance signature contract. Current development adds support for Lua and older conf binding files.

Environment: Ubuntu 24.04 container, Rust 1.98.1, PySide6 6.11.2, offscreen Qt. No live Hyprland session. `workspace-source-sha256.json` identifies the changed runtime and test inputs.

Reproduced locally:
- `cargo fmt --all -- --check`: pass.
- `cargo clippy --locked --all-targets -- -D warnings`: pass.
- `cargo build --locked`: pass.
- `cargo test --locked`: 20 passed; two failed because this container rejects UnixListener socket binding with EPERM. Affected: existing broker round-trip test and new monitor socket round-trip test. Neither is claimed as locally passed; both remain enabled in CI.
- `node tests/workspace.cjs`: pass, per-monitor matching, switching, legacy/all default and unavailable tracking.
- `python3 tests/keybinding.py`: pass, both file syntaxes, idempotence, collisions, inactive binding and reload failure rollback, spaced paths.
- `python3 tests/qml_smoke.py`: pass, native manager input/validation/signal and existing frame checks with explicit fixtures.
- `python3 tests/runtime_integration.py target/debug/omarchy-widget`: pass, real QML queue to Rust save/disk/reopen and stale-revision rejection.
- `python3 tests/installer.py`: pass, installer recovery and paths with spaces.
- `python3 tests/sandbox.py`: pass, static sandbox launch plan; no new live sandbox claim.
- `bash -n bind-key widget install-local`: pass.

Not run here: actual Hyprland shortcut activation, socket monitor query, workspace switching latency, monitor hotplug, fractional scaling, special-workspace overlays and full live widget rendering. This is not a release or acceptance sign-off. Existing isolation/resource CI evidence belongs to its original Core commit.

XPS checks: install this branch with `bash install-local --update`; open Super+Ctrl+Shift+W; assign a widget to 2; switch its monitor between 1 and 2; confirm only that widget hides/shows; restart Core and verify persistence; restore `all`. With two monitors, switching the other monitor must not affect visibility. Confirm a shortcut conflict leaves bindings untouched. A missing monitor snapshot should hide pinned widgets and show a manager error.
