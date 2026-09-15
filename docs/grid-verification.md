# Cell layout verification · Core v0.0.2

Implementation is on a draft branch stacked on the workspace/launcher work. No merge or desktop deployment is claimed.

## Portable evidence

- Rust 1.98.1: locked build, rustfmt and Clippy with warnings denied pass.
- 26 Rust tests pass, including gap/CSS parsing, old/current Hyprland response projections, asymmetric outer gaps, reserved panels, rotation/fractional scaling, no-gap geometry, workspace collision rules, full-grid unplaced state, hidden occupancy, resize rejection, monitor loss/return, gap-driven fallback/restoration and migration preserving settings.
- Registry integration checks duplicate allocation, rejected moves/workspace changes preserving exact on-disk bytes, and re-enabling without displacing an existing occupant. Existing lock/revision/persistence tests pass.
- Node tests cover the QML drag-preview geometry and workspace visibility. Rust remains authoritative when a preview becomes stale.
- PySide6 6.11.2 offscreen QML component smoke and production settings/queue → Rust CLI → disk/reopen integration pass. The headless place request is explicitly rejected because no desktop geometry is available, while settings saves continue and queued moves coalesce.
- qmlformat parses the complete Host.qml. Quickshell/Wayland window behavior is not exercised by this parser or the offscreen component tests.

Two existing UnixListener tests (`workspaces::tests::socket_round_trip` and `island::tests::broker_scopes_reads_writes_and_generations`) cannot bind sockets in this workspace: EPERM. They remain enabled in CI. Local Rust execution filters these two tests only. The pure projection tests exercise all five allowed compositor queries without exposing a runtime fixture/bypass option.

## Required XPS desktop acceptance

1. Compare widget separation with tiled windows under default, asymmetric and no-gap ricing. Confirm no-gap idle frames are square and borderless; allow unused right/bottom space from fixed-size cells.
2. Confirm bar/panel reserved space is applied exactly once at 100% and fractional scaling, including a rotated output.
3. Drag to occupied/out-of-bounds cells: red preview and message, old position retained. Resize into an occupant: fail without moving either widget. Verify arrow keys advance one cell.
4. Reuse cells on workspaces 1/2; an all-workspace widget must conflict with both. Verify the manager reports workspace-change errors.
5. Fill every slot, add another instance, confirm unplaced status/settings retention. Hide an occupant and verify recovery.
6. Remove/reconnect a preferred monitor, reduce/restore resolution, and change gaps while widgets exist. Verify deterministic temporary placement and restoration of the stored preference.
7. Reopen settings, save cities, restart Core, switch theme and validate migration of an existing pixel-only layout. Core stays v0.0.2.

Global Hyprland ricing is inherited. Per-window/workspace rule exceptions are deliberately outside this desktop-grid contract. Network widgets remain proposed, not implemented by this change.
