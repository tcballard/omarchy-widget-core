# Verification record

Development environment: Linux container, Python 3.12, PySide6 6.11.2, offscreen Qt. Rust compilation runs in GitHub Actions because the container has no Rust toolchain. Exact input commits and results are recorded by the linked workflow runs.

## Reproduced

- Rust unit tests for validation, state reopen, stale settings rejection, version-1 migration, instance separation, update/rollback and grid enforcement; rustfmt and Clippy in branch CI.
- Native Qt manager/frame checks locally, including parsed standalone host/bridge QML. Theme/button fixtures are explicit; this is not an Omarchy desktop capture.

## In progress

- Production QML controller/queue/settings surface linked to a real Rust helper, exercising Save through disk and reopening. The first run exposed a Python QQuickItem wrapper import error; it is being rerun after correcting the test harness.
- Final installer and standalone runtime handoff checks.

## Not run

Real Wayland/Hyprland rendering, systemd session lifecycle, crash recovery on the XPS, graphics masking across drivers, fractional scaling, monitor hotplug and live theme transitions. See desktop-checks.md. No sandbox escape testing is claimed: this release is not sandboxed.

Upstream review: Omarchy source and Quickshell CLI/process documentation were inspected during development; the original widget repository was reviewed at `d99737791a5b983d6ba7dc6a9790e38ff4f8edfb`. No substantial source from that repository was copied.
