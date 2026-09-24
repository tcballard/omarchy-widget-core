# Desktop review harness

Run from the Core checkout on your Omarchy desktop. This development tool uses Python's standard library, is not installed with Core, and adds no application dependency or resident process. Core's supervisor/CLI is Rust; the manager and widgets use Quickshell/Qt, with a separate Wayland proxy per package. A small Rust executable does not imply a small complete running stack.

## Start an evidence folder

```bash
python3 tools/desktop-review.py --output "$HOME/widget-review" start
```

The folder must not already exist. `report.md` is the readable index, `checklist.md` describes every desktop check, and JSON events preserve commands, stdout/stderr, exit status, elapsed time and observations. Reuse the same folder for subsequent commands, including after reboot. Events append; recording a check again updates its displayed status while retaining its earlier observation. Do not run two collectors against the same folder concurrently.

Start records source revision/dirty status, OS, Omarchy, Hyprland, monitor layout, Quickshell, package versions and installed sizes, service identity, and SHA-256 file manifests of the installed payload and binaries. Use `start --core-dir /actual/install` for a nonstandard installation. Repeat `environment` after an update, passing the same override if used. Source revision and installed payload hashes are separate evidence: a checkout does not prove that its code is installed.

Files are created private to your user. Logs, monitor names, paths and your notes can contain personal information. Inspect before sharing; nothing is uploaded automatically. Journal capture is capped at the last 2,000 matching entries. Attach extra logs, screenshots or recordings in this folder and reference their filenames in observations.

## Run the automated suite

Prerequisites are the same as `.github/workflows/check.yml`: Rust 1.98.1 including rustfmt/clippy (pinned in rust-toolchain.toml and CI), Node, Python with PySide6 6.11.2 and Qt system libraries, bubblewrap, and a user namespace environment supporting the live sandbox probes. The sandbox capture command specifically uses `/usr/bin/python3`; install PySide6 there too. The harness does not install dependencies, fetch repositories, or alter your desktop installation.

```bash
python3 tools/desktop-review.py --output "$HOME/widget-review" suite \
  --worldclock /path/to/omarchy-widget-worldclock \
  --older-core /path/to/older-core/target/debug/omarchy-widget \
  --proxy "$PWD/target/wl-mitm-source/target/release/wl-mitm"
```

Use World Clock commit `e23549064d28d22d78c679d9d0b042f9532bafa1` and older Core commit `dc96dd4a4dc72603cabaae4e7151f7c15c47c662`, as pinned in CI. Build the older helper with its own `cargo build --locked`. Build the pinned proxy with `bash build-wayland-filter`. Record fixture revisions if you intentionally use different versions. Without these arguments, the affected cases remain **blocked**, never passed.

The suite builds/tests Rust in both feature configurations, checks formatting/clippy, builds the release binary for size, runs QML/integration/SDK/installer/keybinding/geometry tests, runs the manager ten times at each of four scales, live sandbox checks, supplied compatibility/World Clock fixtures and both Wayland feature cases. Each command has a 15-minute limit; failures do not stop later independent tests. Exit status is nonzero for any failure, timeout, blocked or unrun case. The destructive resource-pressure job is deliberately **not run** on your ordinary desktop: run the existing CI `resources` job in its disposable user and record its URL. Consequently the suite alone never claims complete acceptance.

The portable suite explicitly selects Qt's `offscreen` platform and a neutral
platform theme, overriding desktop environment variables such as
`QT_QPA_PLATFORM=wayland;xcb`. The manager test also enforces offscreen when run
directly: its synthetic clicks assume a fixed-size window outside compositor
focus and tiling rules. This is separate from live desktop acceptance. The
`quickshell_exit` check requires `qs` and runs the production idle-exit timer in
the actual Quickshell runtime with an isolated runtime directory.

## Measure the real stack

Wait for compilation, installation and startup to finish. Run measurements with no concurrent test suite or heavy unrelated work. Prepare each scenario yourself using the manager, then capture it. The collector does not add, hide, delete, restart or kill your widgets.

```bash
python3 tools/desktop-review.py --output "$HOME/widget-review" capture empty \
  --seconds 60 --note '0 packages, 0 instances; manager closed'
python3 tools/desktop-review.py --output "$HOME/widget-review" capture single \
  --seconds 60 --note '1 World Clock package, 1 visible instance; manager closed'
python3 tools/desktop-review.py --output "$HOME/widget-review" capture multi \
  --seconds 60 --note '3 packages, 6 visible instances; manager closed'
python3 tools/desktop-review.py --output "$HOME/widget-review" capture hidden \
  --seconds 600 --note '3 packages, 6 instances, all hidden'
```

Run each command only after preparing that scenario. Labels and notes are operator declarations, not an automatic assertion about instance counts. Repeat `custom` captures for editing, startup or other workloads.

Sampling includes the host and all matching island service cgroups, not just the Rust supervisor. Reports give per-unit CPU as a percentage of one core, mean and sampled-maximum cgroup memory, mean process PSS where readable, process counts, and raw per-process RSS/PSS/context-switch counters. Sum disjoint unit CPU or PSS values for the complete stack. `memory.current` includes cgroup-accounted cache; PSS apportions shared Qt pages; summing RSS double-counts shared pages. Tasks include threads and are distinct from process count. One-second process snapshots miss short-lived children, but cgroup CPU includes their work. Peaks between samples are not measured. Context switches are not wakeups; use a dedicated profiler for wakeups. The Python observer and systemctl queries add measurement overhead outside the measured service cgroups.

A missing user manager, missing host cgroup, changing unit set, or replaced/reset cgroup makes a capture **blocked**. Missing process details stay unavailable, never silently zero. Captures do not automatically pass checklist items: verify actual counts, rendering and timing, then record the observation. First-frame latency requires observing the rendered frame, not merely finding a running process.

## Record desktop outcomes

```bash
python3 tools/desktop-review.py --output "$HOME/widget-review" record settings pass \
  --note 'Save/Cancel/WM close clear edit; runner-kill recovery 3.1 s; see settings.webm'
python3 tools/desktop-review.py --output "$HOME/widget-review" record reveal not-applicable \
  --note 'Default build; reveal command confirmed disabled'
python3 tools/desktop-review.py --output "$HOME/widget-review" report
```

Use `fail` or `blocked` with the exact symptom when appropriate. Review every item in `checklist.md`: install; empty/single/multi/hidden performance; failing content; settings; passive-click keyboard focus; sustained-contention latency; arrange; topology; reserved space; workspaces; theme; optional reveal; migration/rollback; live weather; suspend/reboot; lifecycle/isolation; outside-author workflow; interrupted/full-disk persistence; disposable cgroup tests. The checks retain **not-run** until you explicitly record an outcome. Do destructive persistence experiments only in disposable storage.

## Reading the MB figures

`report.md` separates the Rust executable, proxy executable, total installed Core payload (including bundled proxy source), and checkout release binary. Individual binaries are already included in the installed total: do not add them again. Manifests record logical and allocated file bytes, skip symlinks and deduplicate hardlinks within each measured tree. MB is decimal; MiB is binary. Shared Qt/Quickshell dependency package sizes are recorded separately by pacman, not charged again as Rust binary size.

This is not a whole-system dependency closure or total user-data measurement. Widget packages, retained versions, installer backups, build caches and external shared dependencies are outside the Core payload total. Live memory is a separate measurement entirely. There is no agreed MB budget yet: establish these baselines before selecting a disk/PSS target or changing the rendering/process model. The existing 256 MiB per-package limit is a ceiling, not measured normal consumption.
